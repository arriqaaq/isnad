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
        #[arg(long, default_value = "command-r7b-arabic")]
        translate_model: String,

        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,
    },
    /// Run analysis on ingested data (families, narrator enrichment)
    Analyze {
        /// Path to SurrealDB data directory
        #[arg(long, default_value = "db_data")]
        db_path: String,

        /// Compute hadith families from embedding similarity
        #[arg(long)]
        families: bool,

        /// Run mustalah al-hadith analysis on transmission chains
        #[arg(long)]
        mustalah: bool,

        /// Skip families larger than this (degenerate clusters hang analysis)
        #[arg(long, default_value = "500")]
        max_family_size: usize,
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
    let stack_size = 64 * 1024 * 1024; // 128 MB — SurrealDB recursive operations need deep stacks
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
            mustalah,
            max_family_size,
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

            if mustalah {
                println!("📖 Running mustalah al-hadith analysis...");
                use surrealdb::types::{RecordId, SurrealValue};

                #[derive(Debug, SurrealValue)]
                struct MFamilyRow {
                    id: Option<RecordId>,
                    variant_count: Option<i64>,
                }
                let mut res = db
                    .query(
                        "SELECT id, variant_count FROM hadith_family ORDER BY variant_count DESC",
                    )
                    .await?;
                let family_rows: Vec<MFamilyRow> = res.take(0)?;

                // Resume support: check already-analyzed families
                #[derive(Debug, SurrealValue)]
                struct MFamilyRef {
                    family: RecordId,
                }
                let mut done_res = db.query("SELECT family FROM isnad_analysis").await?;
                let done_rows: Vec<MFamilyRef> = done_res.take(0)?;
                let already_done: std::collections::HashSet<String> = done_rows
                    .iter()
                    .map(|r| hadith::models::record_id_key_string(&r.family))
                    .collect();
                let remaining = family_rows.len() - already_done.len().min(family_rows.len());
                if !already_done.is_empty() {
                    println!(
                        "   Resuming: {remaining} families remaining ({} already done)",
                        already_done.len()
                    );
                }

                let pb = indicatif::ProgressBar::new(remaining as u64);
                pb.set_style(
                    indicatif::ProgressStyle::default_bar()
                        .template("   {bar:40.green/black} {pos}/{len} families ({eta})")
                        .unwrap(),
                );

                let mut grade_counts: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                let mut analyzed = 0usize;

                for row in &family_rows {
                    let fid = row
                        .id
                        .as_ref()
                        .map(hadith::models::record_id_key_string)
                        .unwrap_or_default();
                    if fid.is_empty() || already_done.contains(&fid) {
                        continue;
                    }
                    let vcount = row.variant_count.unwrap_or(0) as usize;
                    if vcount > max_family_size {
                        pb.inc(1);
                        continue;
                    }

                    match analysis::mustalah::analyze_family_mustalah(&db, &fid).await? {
                        Some(result) => {
                            let grade_key = format!("{:?}", result.composite_grade).to_lowercase();
                            *grade_counts.entry(grade_key).or_default() += 1;
                            analysis::mustalah::store_mustalah_results(&db, &result).await?;
                            analyzed += 1;
                        }
                        None => {}
                    }
                    pb.inc(1);
                }
                pb.finish_and_clear();

                println!("   Mustalah analysis: {analyzed} families graded");
                for (grade, count) in &grade_counts {
                    println!("     {grade}: {count}");
                }
                tracing::info!("Mustalah analysis complete");
                did_something = true;
            }

            if !did_something {
                tracing::warn!("No analysis flags specified. Use --families or --mustalah.");
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
            quran::morphology::build_ayah_lemma_text(&db).await?;
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
            db::init_user_note_schema(&db).await?;
            db::init_link_preview_schema(&db).await?;
            quran::audio::init_reciters(&db).await?;
            web::serve(db, port, ollama_url, ollama_model).await?;
        }
    }

    Ok(())
}
