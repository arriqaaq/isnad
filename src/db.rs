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
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
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
-- Pre-computed count of hadiths narrated (see backfill_narrator_hadith_counts)
DEFINE FIELD IF NOT EXISTS hadith_count       ON narrator TYPE option<int>;
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
DEFINE INDEX IF NOT EXISTS heard_from_in_idx ON TABLE heard_from FIELDS in;
DEFINE INDEX IF NOT EXISTS heard_from_out_idx ON TABLE heard_from FIELDS out;
DEFINE INDEX IF NOT EXISTS heard_from_hadith_ref_idx ON TABLE heard_from FIELDS hadith_ref;

-- "Narrator narrates Hadith" (narrator closest to Bukhari -> hadith)
DEFINE TABLE IF NOT EXISTS narrates TYPE RELATION FROM narrator TO hadith;
DEFINE FIELD IF NOT EXISTS chain_position ON narrates TYPE option<int>;
DEFINE INDEX IF NOT EXISTS narrates_in_idx ON TABLE narrates FIELDS in;
DEFINE INDEX IF NOT EXISTS narrates_out_idx ON TABLE narrates FIELDS out;

-- "Hadith belongs_to Book"
DEFINE TABLE IF NOT EXISTS belongs_to TYPE RELATION FROM hadith TO book;
DEFINE INDEX IF NOT EXISTS belongs_to_in_idx ON TABLE belongs_to FIELDS in;
"#;

// ── Quran Schema ──

const QURAN_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS surah SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS surah_number    ON surah TYPE int;
DEFINE FIELD IF NOT EXISTS name_ar         ON surah TYPE string;
DEFINE FIELD IF NOT EXISTS name_en         ON surah TYPE string;
DEFINE FIELD IF NOT EXISTS name_translit   ON surah TYPE string;
DEFINE FIELD IF NOT EXISTS revelation_type ON surah TYPE string;
DEFINE FIELD IF NOT EXISTS ayah_count      ON surah TYPE int;
DEFINE INDEX IF NOT EXISTS surah_number_idx ON TABLE surah FIELDS surah_number UNIQUE;

DEFINE TABLE IF NOT EXISTS ayah SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS surah_number    ON ayah TYPE int;
DEFINE FIELD IF NOT EXISTS ayah_number     ON ayah TYPE int;
DEFINE FIELD IF NOT EXISTS text_ar         ON ayah TYPE string;
DEFINE FIELD IF NOT EXISTS text_ar_simple  ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS text_en         ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS tafsir_en       ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS juz             ON ayah TYPE option<int>;
DEFINE FIELD IF NOT EXISTS hizb            ON ayah TYPE option<int>;

DEFINE FIELD IF NOT EXISTS embedding       ON ayah TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS ayah_vec        ON TABLE ayah FIELDS embedding HNSW DIMENSION 384 DIST COSINE;
DEFINE INDEX IF NOT EXISTS ayah_surah_idx  ON TABLE ayah FIELDS surah_number;
DEFINE INDEX IF NOT EXISTS ayah_composite  ON TABLE ayah FIELDS surah_number, ayah_number UNIQUE;

-- Edge: ayah → hadith (curated mapping from Quran.com)
DEFINE TABLE IF NOT EXISTS references_hadith SCHEMAFULL TYPE RELATION IN ayah OUT hadith;
DEFINE FIELD IF NOT EXISTS collection     ON references_hadith TYPE string;
DEFINE FIELD IF NOT EXISTS hadith_number  ON references_hadith TYPE string;
DEFINE FIELD IF NOT EXISTS source         ON references_hadith TYPE string DEFAULT 'qurancom';
DEFINE INDEX IF NOT EXISTS refs_hadith_in_idx ON TABLE references_hadith FIELDS in;
DEFINE INDEX IF NOT EXISTS refs_hadith_out_idx ON TABLE references_hadith FIELDS out
"#;

// ── Quran Word Morphology Schema ──

const QURAN_WORD_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS quran_word SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS surah_number    ON quran_word TYPE int;
DEFINE FIELD IF NOT EXISTS ayah_number     ON quran_word TYPE int;
DEFINE FIELD IF NOT EXISTS word_position   ON quran_word TYPE int;
DEFINE FIELD IF NOT EXISTS text_ar         ON quran_word TYPE string;
DEFINE FIELD IF NOT EXISTS text_ar_simple  ON quran_word TYPE option<string>;
DEFINE FIELD IF NOT EXISTS translation     ON quran_word TYPE option<string>;
DEFINE FIELD IF NOT EXISTS transliteration ON quran_word TYPE option<string>;
DEFINE FIELD IF NOT EXISTS pos             ON quran_word TYPE string;
DEFINE FIELD IF NOT EXISTS root            ON quran_word TYPE option<string>;
DEFINE FIELD IF NOT EXISTS lemma           ON quran_word TYPE option<string>;
REMOVE FIELD IF EXISTS features ON quran_word;
DEFINE FIELD features ON quran_word TYPE option<object> FLEXIBLE;
REMOVE FIELD IF EXISTS segments ON quran_word;
DEFINE FIELD segments ON quran_word TYPE option<string>;
DEFINE INDEX IF NOT EXISTS quran_word_ayah_idx ON TABLE quran_word FIELDS surah_number, ayah_number;
DEFINE INDEX IF NOT EXISTS quran_word_root_idx ON TABLE quran_word FIELDS root;
DEFINE INDEX IF NOT EXISTS quran_word_pos_idx ON TABLE quran_word FIELDS pos
"#;

// ── Reciter Schema ──

const RECITER_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS reciter SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS name_en      ON reciter TYPE string;
DEFINE FIELD IF NOT EXISTS name_ar      ON reciter TYPE option<string>;
DEFINE FIELD IF NOT EXISTS style        ON reciter TYPE option<string>;
DEFINE FIELD IF NOT EXISTS folder_name  ON reciter TYPE string;
DEFINE FIELD IF NOT EXISTS bitrate      ON reciter TYPE option<string>
"#;

pub async fn init_reciter_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in RECITER_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Reciter schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Reciter schema initialized");
    Ok(())
}

pub async fn init_quran_word_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in QURAN_WORD_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Quran word schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Quran word schema initialized");
    Ok(())
}

// ── Quran Similar / Mutashabihat Schema ──

const QURAN_SIMILAR_SCHEMA: &str = r#"
-- Shared phrases (mutashabihat hub nodes)
DEFINE TABLE IF NOT EXISTS quran_phrase SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS text_ar        ON quran_phrase TYPE string;
DEFINE FIELD IF NOT EXISTS text_ar_simple  ON quran_phrase TYPE option<string>;
DEFINE FIELD IF NOT EXISTS occurrence     ON quran_phrase TYPE int;
DEFINE FIELD IF NOT EXISTS verses_count   ON quran_phrase TYPE int;
DEFINE FIELD IF NOT EXISTS chapters_count ON quran_phrase TYPE int;

-- Edge: ayah -> shares_phrase -> quran_phrase
DEFINE TABLE IF NOT EXISTS shares_phrase SCHEMAFULL TYPE RELATION IN ayah OUT quran_phrase;
DEFINE FIELD IF NOT EXISTS word_from       ON shares_phrase TYPE int;
DEFINE FIELD IF NOT EXISTS word_to         ON shares_phrase TYPE int;
DEFINE FIELD IF NOT EXISTS matched_count   ON shares_phrase TYPE option<int>;
DEFINE INDEX IF NOT EXISTS shares_phrase_in_idx ON TABLE shares_phrase FIELDS in;
DEFINE INDEX IF NOT EXISTS shares_phrase_out_idx ON TABLE shares_phrase FIELDS out;

-- Edge: ayah -> similar_to -> ayah
DEFINE TABLE IF NOT EXISTS similar_to SCHEMAFULL TYPE RELATION IN ayah OUT ayah;
DEFINE FIELD IF NOT EXISTS score           ON similar_to TYPE int;
DEFINE FIELD IF NOT EXISTS coverage        ON similar_to TYPE int;
DEFINE FIELD IF NOT EXISTS matched_positions ON similar_to TYPE option<string>;
DEFINE INDEX IF NOT EXISTS similar_to_in_idx ON TABLE similar_to FIELDS in
"#;

pub async fn init_quran_similar_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in QURAN_SIMILAR_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Quran similar schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Quran similar/mutashabihat schema initialized");
    Ok(())
}

pub async fn init_quran_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in QURAN_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Quran schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Quran database schema initialized");
    Ok(())
}

pub async fn init_quran_fulltext_indexes(db: &Surreal<Db>) -> Result<()> {
    let stmts = [
        "DEFINE ANALYZER en_analyzer TOKENIZERS blank,class FILTERS lowercase,snowball(english)",
        "DEFINE ANALYZER ar_analyzer TOKENIZERS blank,class",
        "DEFINE INDEX IF NOT EXISTS ayah_text_en_search ON TABLE ayah FIELDS text_en FULLTEXT ANALYZER en_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS ayah_text_ar_search ON TABLE ayah FIELDS text_ar_simple FULLTEXT ANALYZER ar_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS ayah_tafsir_en_search ON TABLE ayah FIELDS tafsir_en FULLTEXT ANALYZER en_analyzer BM25 HIGHLIGHTS",
    ];
    for (i, stmt) in stmts.iter().enumerate() {
        let mut attempts = 0;
        loop {
            match db.query(*stmt).await.and_then(|r| r.check()) {
                Ok(_) => break,
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("already exists") {
                        break;
                    }
                    attempts += 1;
                    if attempts >= 5 {
                        tracing::error!("Quran fulltext index {i} failed after {attempts} attempts: {e}");
                        return Err(e.into());
                    }
                    let delay = 500 * attempts;
                    tracing::warn!("Quran fulltext index {i} retry {attempts}/5 (waiting {delay}ms): {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                }
            }
        }
        // Pause between FULLTEXT index definitions — SurrealDB builds indexes in the
        // background and concurrent builds on the same table cause memtable conflicts.
        if i >= 2 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
    }
    tracing::info!("Quran full-text search indexes initialized");
    Ok(())
}

/// Create BM25 full-text search indexes on hadith text fields.
/// Called separately from init_schema because FULLTEXT indexes on SCHEMAFULL
/// tables with option<string> fields block inserts when the value is NONE.
/// Must be called after data is ingested (or at serve time).
pub async fn init_fulltext_indexes(db: &Surreal<Db>) -> Result<()> {
    let stmts = [
        // Analyzers first (no IF NOT EXISTS support — re-define is idempotent)
        "DEFINE ANALYZER en_analyzer TOKENIZERS blank,class FILTERS lowercase,snowball(english)",
        "DEFINE ANALYZER ar_analyzer TOKENIZERS blank,class",
        // Then indexes — sequential to avoid concurrent creation issues
        "DEFINE INDEX IF NOT EXISTS hadith_text_en_search ON TABLE hadith FIELDS text_en FULLTEXT ANALYZER en_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS hadith_text_ar_search ON TABLE hadith FIELDS text_ar FULLTEXT ANALYZER ar_analyzer BM25 HIGHLIGHTS",
    ];
    for (i, stmt) in stmts.iter().enumerate() {
        if let Err(e) = db.query(*stmt).await.and_then(|r| r.check()) {
            let msg = e.to_string();
            if msg.contains("already exists") {
                continue;
            }
            tracing::error!("Fulltext index {i} failed: {e}");
            return Err(e.into());
        }
        // Pause between FULLTEXT index definitions to avoid memtable conflicts
        if i >= 2 {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }
    tracing::info!("Full-text search indexes initialized");
    Ok(())
}

/// Pre-compute hadith_count on every narrator record.
///
/// This avoids expensive `count(->narrates->hadith)` graph traversals on every
/// narrator list/search request. Call at startup after schema init.
pub async fn backfill_narrator_hadith_counts(db: &Surreal<Db>) -> Result<()> {
    db.query("UPDATE narrator SET hadith_count = count(->narrates->hadith)")
        .await?
        .check()?;
    tracing::info!("Backfilled narrator hadith_count");
    Ok(())
}
