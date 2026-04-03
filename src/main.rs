use hadith::{analysis, db, embed, ingest, quran, web};

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
    /// Run analysis on ingested data (families, CL/PCL, narrator enrichment)
    Analyze {
        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Compute hadith families from embedding similarity
        #[arg(long)]
        families: bool,

        /// Run Juynboll falsifiability analysis on CL candidates
        #[arg(long)]
        juynboll: bool,

        /// Enrich narrators with biographical data from AR-Sanad dataset (auto-downloads if missing)
        #[arg(long, default_value = "data/ar_sanad_narrators.csv")]
        narrator_bio: Option<String>,
    },
    /// Ingest Quran data (Arabic + English + Tafsir Ibn Kathir)
    IngestQuran {
        /// Path to Quran CSV file (from scripts/prepare_quran_data.py)
        #[arg(long, default_value = "data/quran.csv")]
        file: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest Quran→Hadith reference mappings from Quran.com
    IngestQuranHadithRefs {
        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest word morphology data (corpus.quran.com + QUL translations)
    IngestMorphology {
        /// Path to quran-morphology.txt
        #[arg(long, default_value = "data/quran-morphology.txt")]
        file: String,

        /// Directory with QUL JSON files (colored-english-wbw-translation.json, etc.)
        #[arg(long, default_value = "qul")]
        qul_dir: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest shared phrases (mutashabihat) and similar ayahs from QUL JSON
    IngestQuranSimilar {
        /// Directory with QUL JSON files (phrases.json, matching-ayah.json)
        #[arg(long, default_value = "qul")]
        qul_dir: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Ingest manuscript descriptions and variant readings from Corpus Coranicum TEI XML
    IngestManuscripts {
        /// Path to cloned corpus-coranicum-tei repository
        #[arg(long, default_value = "data/corpus-coranicum-tei")]
        tei_dir: String,

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

fn main() -> Result<()> {
    let stack_size = 128 * 1024 * 1024; // 128 MB — SurrealDB recursive operations need deep stacks
    eprintln!("Starting tokio runtime with {stack_size} byte worker thread stacks");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(stack_size)
        .build()?
        .block_on(async_main())
}

async fn async_main() -> Result<()> {
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

            // Create BM25 full-text indexes after data is loaded
            db::init_fulltext_indexes(&db).await?;

            tracing::info!("Ingestion complete");
        }
        Commands::Analyze {
            db_path,
            families,
            juynboll,
            narrator_bio,
        } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;

            let mut did_something = false;

            if families {
                let embedder = embed::Embedder::new()?;
                let count = analysis::family::compute_families(&db, &embedder).await?;
                tracing::info!("Created {count} hadith families");
                did_something = true;
            }

            if juynboll {
                println!("🔬 Running CL/PCL + Juynboll falsifiability analysis...");
                use surrealdb::types::{RecordId, SurrealValue};

                // Get all families
                #[derive(Debug, SurrealValue)]
                struct IdOnly {
                    id: Option<RecordId>,
                }
                let mut res = db
                    .query(
                        "SELECT id, variant_count FROM hadith_family ORDER BY variant_count DESC",
                    )
                    .await?;
                let family_rows: Vec<IdOnly> = res.take(0)?;

                let mut cl_families = 0usize;
                let mut juynboll_families = 0usize;
                for row in &family_rows {
                    let fid = row
                        .id
                        .as_ref()
                        .map(hadith::models::record_id_key_string)
                        .unwrap_or_default();
                    if fid.is_empty() {
                        continue;
                    }
                    let result =
                        analysis::cl_pcl::analyze_family(&db, &fid, "structural_only").await?;

                    // Store CL/PCL results
                    if !result.candidates.is_empty() {
                        analysis::cl_pcl::store_results(&db, &result).await?;
                        cl_families += 1;
                    }

                    // Store Juynboll results
                    if let Some(ref j) = result.juynboll {
                        analysis::juynboll::store_juynboll_results(&db, j).await?;
                        juynboll_families += 1;
                    }
                }

                println!(
                    "   CL/PCL analysis: {} families with candidates",
                    cl_families
                );

                // Print corpus summary
                let summary = analysis::juynboll::compute_cross_family_summary(&db).await?;
                println!("   Juynboll analysis: {} families", juynboll_families);
                println!(
                    "   Families with reliable bypass: {}",
                    summary.families_with_reliable_bypass
                );
                println!(
                    "   Families with independent CLs: {}",
                    summary.families_with_independent_cls
                );
                if !summary.cross_family_narrators.is_empty() {
                    println!(
                        "   Cross-family CL narrators (top): {}",
                        summary
                            .cross_family_narrators
                            .iter()
                            .take(5)
                            .map(|n| {
                                format!(
                                    "{} ({}x, {:?})",
                                    n.narrator_id, n.cl_family_count, n.reliability_rating
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                tracing::info!("Analysis complete");
                did_something = true;
            }

            if let Some(bio_path) = narrator_bio {
                println!("📚 Enriching narrators with AR-Sanad biographical data...");
                ingest::narrator_bio::ingest_narrator_bios(&db, &bio_path).await?;
                did_something = true;
            }

            if !did_something {
                tracing::warn!(
                    "No analysis flags specified. Use --families, --juynboll, or --narrator-bio."
                );
            }
        }
        Commands::IngestQuran { file, db_path } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "Quran CSV not found at {file}. Run: python scripts/prepare_quran_data.py"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            db::init_quran_schema(&db).await?;
            quran::ingest::ingest(&db, &file).await?;
            db::init_quran_fulltext_indexes(&db).await?;

            tracing::info!("Quran ingestion complete");
        }
        Commands::IngestQuranHadithRefs { db_path } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            db::init_quran_schema(&db).await?;
            quran::hadith_refs::ingest_hadith_refs(&db).await?;
            tracing::info!("Quran-Hadith reference ingestion complete");
        }
        Commands::IngestMorphology {
            file,
            qul_dir,
            db_path,
        } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "Morphology data not found at {file}. Download from: https://github.com/mustafa0x/quran-morphology"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_quran_word_schema(&db).await?;
            quran::morphology::ingest_morphology(&db, &file, &qul_dir).await?;
            tracing::info!("Morphology ingestion complete");
        }
        Commands::IngestManuscripts { tei_dir, db_path } => {
            if !std::path::Path::new(&tei_dir).is_dir() {
                anyhow::bail!(
                    "TEI directory not found at {tei_dir}. Clone from: https://github.com/telota/corpus-coranicum-tei"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_manuscript_schema(&db).await?;
            quran::manuscripts::ingest_manuscripts(&db, &tei_dir).await?;
            tracing::info!("Manuscript ingestion complete");
        }
        Commands::IngestQuranSimilar { qul_dir, db_path } => {
            let db = db::connect(&db_path).await?;
            db::init_quran_schema(&db).await?;
            db::init_quran_word_schema(&db).await?;
            db::init_quran_similar_schema(&db).await?;
            println!("Ingesting shared phrases and similar ayahs from {qul_dir}...");
            quran::similar::ingest_similar(&db, &qul_dir).await?;
            tracing::info!("Quran similar/mutashabihat ingestion complete");
        }
        Commands::Serve {
            port,
            db_path,
            ollama_url,
            ollama_model,
        } => {
            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            db::init_quran_schema(&db).await?;
            db::init_quran_word_schema(&db).await?;
            db::init_quran_similar_schema(&db).await?;
            db::init_reciter_schema(&db).await?;
            db::init_manuscript_schema(&db).await?;
            quran::audio::init_reciters(&db).await?;
            db::init_fulltext_indexes(&db).await?;
            db::backfill_narrator_hadith_counts(&db).await?;
            // Quran fulltext indexes are created during ingest-quran, not here.
            // Creating them on an empty table with option<string> fields causes errors.
            web::serve(db, port, ollama_url, ollama_model).await?;
        }
    }

    Ok(())
}
