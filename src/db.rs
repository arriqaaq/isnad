use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::SurrealKv;

pub type Db = surrealdb::engine::local::Db;

pub async fn connect(path: &str) -> Result<Surreal<Db>> {
    let db = Surreal::new::<SurrealKv>(path).await?;
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
-- Biographical fields
DEFINE FIELD IF NOT EXISTS kunya          ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS aliases        ON narrator TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS birth_year     ON narrator TYPE option<int>;
DEFINE FIELD IF NOT EXISTS birth_calendar ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS death_year     ON narrator TYPE option<int>;
DEFINE FIELD IF NOT EXISTS death_calendar ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS locations      ON narrator TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS tags           ON narrator TYPE option<array<string>>;
-- Reliability fields
DEFINE FIELD IF NOT EXISTS reliability_rating ON narrator TYPE option<string>;
DEFINE FIELD IF NOT EXISTS reliability_prior  ON narrator TYPE option<float>;
DEFINE FIELD IF NOT EXISTS reliability_source ON narrator TYPE option<string>;
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
DEFINE FIELD IF NOT EXISTS matn         ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS embedding     ON hadith TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS hadith_vec    ON TABLE hadith FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
DEFINE INDEX IF NOT EXISTS hadith_book   ON TABLE hadith FIELDS book_id;

-- BM25 full-text search analyzers (indexes created after ingest via init_fulltext_indexes)
DEFINE ANALYZER IF NOT EXISTS en_analyzer TOKENIZERS blank, class FILTERS lowercase, snowball(english);
DEFINE ANALYZER IF NOT EXISTS ar_analyzer TOKENIZERS blank, class;

DEFINE TABLE IF NOT EXISTS book SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS book_number ON book TYPE int;
DEFINE FIELD IF NOT EXISTS name_en     ON book TYPE string;
DEFINE FIELD IF NOT EXISTS name_ar     ON book TYPE option<string>;

-- === ANALYSIS TABLES ===

DEFINE TABLE IF NOT EXISTS hadith_family SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family_label ON hadith_family TYPE option<string>;
DEFINE FIELD IF NOT EXISTS variant_count ON hadith_family TYPE option<int>;

DEFINE FIELD IF NOT EXISTS family_id ON hadith TYPE option<record<hadith_family>>;
DEFINE INDEX IF NOT EXISTS hadith_family_idx ON TABLE hadith FIELDS family_id;

DEFINE TABLE IF NOT EXISTS cl_analysis SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family ON cl_analysis TYPE record<hadith_family>;
DEFINE FIELD IF NOT EXISTS narrator ON cl_analysis TYPE record<narrator>;
DEFINE FIELD IF NOT EXISTS candidate_type ON cl_analysis TYPE string;
DEFINE FIELD IF NOT EXISTS pcl_mode ON cl_analysis TYPE option<string>;
DEFINE FIELD IF NOT EXISTS fan_out ON cl_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS bundle_coverage ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS collector_diversity ON cl_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS pre_single_strand_ratio ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS bypass_ratio ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS chronology_conflict_ratio ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS matn_coherence ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS provenance_completeness ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS structural_score ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS reliability_prior ON cl_analysis TYPE option<float>;
DEFINE FIELD IF NOT EXISTS final_confidence ON cl_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS outcome ON cl_analysis TYPE string;
DEFINE FIELD IF NOT EXISTS contradiction_cap_active ON cl_analysis TYPE bool;
DEFINE FIELD IF NOT EXISTS profile ON cl_analysis TYPE string;
DEFINE FIELD IF NOT EXISTS family_status ON cl_analysis TYPE string;
DEFINE FIELD IF NOT EXISTS rank ON cl_analysis TYPE int;
DEFINE INDEX IF NOT EXISTS cl_family_idx ON TABLE cl_analysis FIELDS family;
DEFINE INDEX IF NOT EXISTS cl_narrator_idx ON TABLE cl_analysis FIELDS narrator;

DEFINE TABLE IF NOT EXISTS juynboll_analysis SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family ON juynboll_analysis TYPE record<hadith_family>;
DEFINE FIELD IF NOT EXISTS has_reliable_bypass ON juynboll_analysis TYPE bool;
DEFINE FIELD IF NOT EXISTS reliable_bypass_count ON juynboll_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS max_reliable_bypass_ratio ON juynboll_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS has_independent_cls ON juynboll_analysis TYPE bool;
DEFINE FIELD IF NOT EXISTS independent_cl_pairs ON juynboll_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS cl_count ON juynboll_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS upstream_reliable_ratio ON juynboll_analysis TYPE float;
DEFINE FIELD IF NOT EXISTS upstream_branching_points ON juynboll_analysis TYPE int;
DEFINE INDEX IF NOT EXISTS juynboll_family_idx ON TABLE juynboll_analysis FIELDS family UNIQUE;

DEFINE TABLE IF NOT EXISTS evidence SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS narrator ON evidence TYPE record<narrator>;
DEFINE FIELD IF NOT EXISTS evidence_id ON evidence TYPE string;
DEFINE FIELD IF NOT EXISTS rating ON evidence TYPE string;
DEFINE FIELD IF NOT EXISTS rating_confidence ON evidence TYPE option<float>;
DEFINE FIELD IF NOT EXISTS scholar ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS work ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS citation_text ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS citation_span ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS dissent_notes ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS layer ON evidence TYPE string;
DEFINE FIELD IF NOT EXISTS source_collection ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source_type ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source_locator ON evidence TYPE option<string>;
DEFINE FIELD IF NOT EXISTS ingested_at ON evidence TYPE option<datetime>;
DEFINE INDEX IF NOT EXISTS evidence_narrator_idx ON TABLE evidence FIELDS narrator;

DEFINE TABLE IF NOT EXISTS narrator_alias SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS canonical ON narrator_alias TYPE record<narrator>;
DEFINE FIELD IF NOT EXISTS alias_name ON narrator_alias TYPE string;
DEFINE FIELD IF NOT EXISTS alias_type ON narrator_alias TYPE string;
DEFINE INDEX IF NOT EXISTS alias_name_idx ON TABLE narrator_alias FIELDS alias_name;

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

/// Create BM25 full-text search indexes on hadith text fields.
/// Called separately from init_schema because FULLTEXT indexes on SCHEMAFULL
/// tables with option<string> fields block inserts when the value is NONE.
/// Must be called after data is ingested (or at serve time).
pub async fn init_fulltext_indexes(db: &Surreal<Db>) -> Result<()> {
    let stmts = [
        "DEFINE INDEX IF NOT EXISTS hadith_text_en_search ON TABLE hadith FIELDS text_en FULLTEXT ANALYZER en_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS hadith_text_ar_search ON TABLE hadith FIELDS text_ar FULLTEXT ANALYZER ar_analyzer BM25 HIGHLIGHTS",
    ];
    for stmt in stmts {
        if let Err(e) = db.query(stmt).await.and_then(|mut r| r.check()) {
            tracing::warn!("Fulltext index creation failed (non-fatal): {e}");
        }
    }
    tracing::info!("Full-text search indexes initialized");
    Ok(())
}
