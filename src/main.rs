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

        /// Ollama model for translation
        #[arg(long, default_value = "llama3.2")]
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
    let data_dir = std::path::Path::new(target_path).parent().unwrap_or(std::path::Path::new("data"));
    std::fs::create_dir_all(data_dir)?;

    let zip_path = data_dir.join("sanadset.zip");

    // Download
    tracing::info!("Downloading Sanadset dataset from Mendeley Data...");
    let response = reqwest::get(SANADSET_ZIP_URL).await?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }
    let bytes = response.bytes().await?;
    tracing::info!("Downloaded {} MB", bytes.len() / 1_000_000);

    std::fs::write(&zip_path, &bytes)?;

    // Unzip
    tracing::info!("Extracting...");
    let file = std::fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();
        // Look for the sanadset CSV file inside the zip
        if name.ends_with(".csv") && (name.contains("sanadset") || name.contains("Sanadset") || name.contains("650")) {
            let mut out = std::fs::File::create(target_path)?;
            std::io::copy(&mut entry, &mut out)?;
            tracing::info!("Extracted {name} → {target_path}");
            found = true;
            break;
        }
    }

    if !found {
        // If no matching filename, extract the largest CSV
        let mut largest = (0, String::new());
        for i in 0..archive.len() {
            let entry = archive.by_index(i)?;
            if entry.name().ends_with(".csv") && entry.size() > largest.0 {
                largest = (entry.size() as u64, entry.name().to_string());
            }
        }
        if !largest.1.is_empty() {
            let mut entry = archive.by_name(&largest.1)?;
            let mut out = std::fs::File::create(target_path)?;
            std::io::copy(&mut entry, &mut out)?;
            tracing::info!("Extracted {} → {target_path}", largest.1);
        } else {
            anyhow::bail!("No CSV file found in the downloaded zip");
        }
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

            if translate {
                tracing::info!("Translating via Ollama ({translate_model})...");
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
