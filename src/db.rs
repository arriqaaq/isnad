use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::RocksDb;

pub type Db = surrealdb::engine::local::Db;

pub async fn connect(path: &str) -> Result<Surreal<Db>> {
    let db = Surreal::new::<RocksDb>(path).await?;
    db.use_ns("hadith_app").use_db("sahih_bukhari").await?;
    Ok(db)
}

pub async fn init_schema(db: &Surreal<Db>) -> Result<()> {
    // Execute schema statements individually to get better error reporting
    for (i, stmt) in SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|mut r| r.check()) {
            tracing::error!(
                "Schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Database schema initialized");
    Ok(())
}

const SCHEMA: &str = r#"
-- === NODES ===

DEFINE TABLE IF NOT EXISTS narrator SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name_ar      ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS name_en      ON narrator TYPE string;
DEFINE FIELD IF NOT EXISTS search_name  ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS gender       ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS generation   ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS bio          ON narrator TYPE option<string>;
DEFINE INDEX IF NOT EXISTS narrator_name ON TABLE narrator FIELDS name_en;

DEFINE TABLE IF NOT EXISTS hadith SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS hadith_number ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS book_id       ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS chapter_id    ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS text_ar       ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS text_en       ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS narrator_text ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS grade         ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS book_name    ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS embedding     ON hadith TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS hadith_vec    ON TABLE hadith FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
DEFINE INDEX IF NOT EXISTS hadith_book   ON TABLE hadith FIELDS book_id;

DEFINE TABLE IF NOT EXISTS book SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS book_number ON book TYPE int;
DEFINE FIELD IF NOT EXISTS name_en     ON book TYPE string;
DEFINE FIELD IF NOT EXISTS name_ar     ON book TYPE option<string>;

-- === EDGES (graph relations) ===

-- "Narrator B heard_from Narrator A" (student -> teacher, toward the Prophet)
DEFINE TABLE IF NOT EXISTS heard_from TYPE RELATION FROM narrator TO narrator;
DEFINE FIELD IF NOT EXISTS hadith_ref ON heard_from TYPE option<record<hadith>>;

-- "Narrator narrates Hadith" (narrator closest to Bukhari -> hadith)
DEFINE TABLE IF NOT EXISTS narrates TYPE RELATION FROM narrator TO hadith;
DEFINE FIELD IF NOT EXISTS chain_position ON narrates TYPE option<int>;

-- "Hadith belongs_to Book"
DEFINE TABLE IF NOT EXISTS belongs_to TYPE RELATION FROM hadith TO book;
"#;
