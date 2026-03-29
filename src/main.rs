mod db;
mod embed;
mod ingest;
mod models;
mod rag;
mod search;
mod web;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "hadith",
    about = "Hadith Explorer — browse and search Islamic hadith collections"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest hadith data from a Sanadset CSV file
    Ingest {
        /// Path to Sanadset CSV file
        #[arg(long, default_value = "data/sanadset.csv")]
        file: String,

        /// List all available books and exit
        #[arg(long)]
        list_books: bool,

        /// Comma-separated book numbers to load (from --list-books output)
        #[arg(long)]
        books: Option<String>,

        /// Load all books (not just the default 6)
        #[arg(long)]
        all: bool,

        /// Max hadiths per book (for testing). Omit to load all.
        #[arg(long)]
        limit: Option<usize>,

        /// Translate hadiths and narrators to English using Ollama
        #[arg(long)]
        translate: bool,

        /// Ollama model for fallback translation
        #[arg(long, default_value = "qwen3:8b")]
        translate_model: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Start the web server
    Serve {
        /// Port to listen on
        #[arg(long, default_value_t = 3000)]
        port: u16,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Ollama API base URL
        #[arg(long, env = "OLLAMA_URL")]
        ollama_url: Option<String>,

        /// Ollama model name
        #[arg(long, env = "OLLAMA_MODEL")]
        ollama_model: Option<String>,
    },
}

const SANADSET_ZIP_URL: &str = "https://data.mendeley.com/public-api/zip/5xth87zwb5/download/5";

async fn download_sanadset(target_path: &str) -> Result<()> {
    use futures::StreamExt;
    use indicatif::{ProgressBar, ProgressStyle};
    use std::io::Write;

    let data_dir = std::path::Path::new(target_path)
        .parent()
        .unwrap_or(std::path::Path::new("data"));
    std::fs::create_dir_all(data_dir)?;

    let zip_path = data_dir.join("sanadset.zip");

    // Download with progress bar
    println!("📥 Downloading Sanadset dataset from Mendeley Data...");
    let response = reqwest::get(SANADSET_ZIP_URL).await?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }

    let total = response.content_length().unwrap_or(0);
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta})")
            .unwrap(),
    );

    let mut file = std::fs::File::create(&zip_path)?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pb.inc(chunk.len() as u64);
        file.write_all(&chunk)?;
    }
    pb.finish_with_message("downloaded");
    println!("   ✓ Downloaded {} MB", total / 1_000_000);

    // Unzip — find the largest CSV (that's the sanadset data)
    println!("📦 Extracting...");
    let file = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    // List all CSVs and pick the largest one
    let mut largest_idx: Option<usize> = None;
    let mut largest_size: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        let size = entry.size();
        if name.ends_with(".csv") {
            println!("   found: {} ({} MB)", name, size / 1_000_000);
            if size > largest_size {
                largest_size = size;
                largest_idx = Some(i);
            }
        }
    }

    if let Some(idx) = largest_idx {
        let mut entry = archive.by_index(idx)?;
        let name = entry.name().to_string();
        let mut out = std::fs::File::create(target_path)?;
        std::io::copy(&mut entry, &mut out)?;
        println!(
            "   ✓ Extracted {name} ({} MB) → {target_path}",
            largest_size / 1_000_000
        );
    } else {
        anyhow::bail!("No CSV file found in the downloaded zip");
    }

    // Cleanup zip
    std::fs::remove_file(&zip_path).ok();
    tracing::info!("Dataset ready at {target_path}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hadith=info".into()),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest {
            file,
            list_books,
            books,
            all,
            limit,
            translate,
            translate_model,
            db_path,
        } => {
            // Auto-download sanadset if file doesn't exist
            let file = if !std::path::Path::new(&file).exists() {
                tracing::info!("Dataset not found at {file}, downloading...");
                download_sanadset(&file).await?;
                file
            } else {
                file
            };

            if list_books {
                ingest::sanadset::print_book_list(&file)?;
                return Ok(());
            }

            let selected = ingest::sanadset::resolve_books(&file, books.as_deref(), all)?;
            tracing::info!("Selected {} books for ingestion", selected.len());

            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            ingest::sanadset::ingest(&db, &file, &selected, limit).await?;

            // Always merge human translations from sunnah.com (free, instant)
            println!("🌐 Merging human English translations from sunnah.com...");
            ingest::sanadset::merge_human_translations(&db).await?;

            if translate {
                // Fill gaps with Ollama for hadiths/narrators still missing English
                println!("🤖 Filling translation gaps via Ollama ({translate_model})...");
                ingest::sanadset::translate_all(&db, &translate_model).await?;
            }

            tracing::info!("Ingestion complete");
        }
        Commands::Serve {
            port,
            db_path,
            ollama_url,
            ollama_model,
        } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            web::serve(db, port, ollama_url, ollama_model).await?;
        }
    }

    Ok(())
}
