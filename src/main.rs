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
    /// Ingest hadith data from SemanticHadith KG (6 major books with identified narrators)
    Ingest {
        /// Path to SemanticHadith JSON data file
        #[arg(long, default_value = "data/semantic_hadith.json")]
        file: String,

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
            limit,
            translate,
            translate_model,
            db_path,
        } => {
            if !std::path::Path::new(&file).exists() {
                anyhow::bail!(
                    "SemanticHadith JSON not found at {file}\n\
                     Run: make semantic-download && make semantic-extract"
                );
            }

            let db = db::connect(&db_path).await?;
            db::init_schema(&db).await?;
            // Define fulltext indexes BEFORE ingesting data — on an empty table
            // this is instant, and subsequent inserts incrementally update the
            // index. This avoids the "memtable history insufficient" error that
            // occurs when building a fulltext index over thousands of rows.
            db::init_fulltext_indexes(&db).await?;
            ingest::semantic::ingest(&db, &file, limit).await?;

            // Merge human English translations from sunnah.com (better quality)
            println!("🌐 Merging human English translations from sunnah.com...");
            ingest::sanadset::merge_human_translations(&db).await?;

            if translate {
                // Fill gaps with Ollama for hadiths/narrators still missing English
                println!("🤖 Filling translation gaps via Ollama ({translate_model})...");
                ingest::sanadset::translate_all(&db, &translate_model).await?;
            }

            tracing::info!("Ingestion complete");
        }
        Commands::Analyze {
            db_path,
            families,
            juynboll,
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

            if !did_something {
                tracing::warn!("No analysis flags specified. Use --families or --juynboll.");
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
            db::init_tafsir_chunk_schema(&db).await?;
            // Define fulltext indexes BEFORE ingesting data — on an empty table
            // this is instant, and subsequent inserts incrementally update the
            // index. This avoids the "memtable history insufficient" error that
            // occurs when building a fulltext index over thousands of rows in a
            // single long-running transaction after ingestion.
            db::init_quran_fulltext_indexes(&db).await?;
            quran::ingest::ingest(&db, &file).await?;

            // Chunk and embed tafsir texts
            println!("📝 Chunking and embedding tafsir texts...");
            let embedder = embed::Embedder::new()?;
            quran::ingest::embed_tafsir_chunks(&db, &embedder).await?;

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
            db::init_tafsir_chunk_schema(&db).await?;
            db::init_reciter_schema(&db).await?;
            quran::audio::init_reciters(&db).await?;
            web::serve(db, port, ollama_url, ollama_model).await?;
        }
    }

    Ok(())
}
