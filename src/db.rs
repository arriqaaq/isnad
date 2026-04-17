use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::engine::local::SurrealKv;

pub type Db = surrealdb::engine::local::Db;

pub async fn connect(path: &str) -> Result<Surreal<Db>> {
    let db = Surreal::new::<SurrealKv>(path).await?;
    db.use_ns("hadith_app").use_db("sahih_bukhari").await?;
    Ok(db)
}

pub async fn init_schema(db: &Surreal<Db>, embed_dim: usize) -> Result<()> {
    let schema = SCHEMA.replace("DIMENSION 1024", &format!("DIMENSION {embed_dim}"));
    // Execute schema statements individually to get better error reporting
    for (i, stmt) in schema
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
-- Pre-computed count of hadiths narrated (see backfill_narrator_hadith_counts)
DEFINE FIELD IF NOT EXISTS hadith_count       ON narrator TYPE option<int>;
DEFINE INDEX IF NOT EXISTS narrator_name ON TABLE narrator FIELDS name_en;

DEFINE TABLE IF NOT EXISTS hadith SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS hadith_number ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS collection_id ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS chapter_id    ON hadith TYPE int;
DEFINE FIELD IF NOT EXISTS text_ar       ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS text_en       ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS narrator_text ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS grade         ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS book_name    ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS matn         ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS hadith_type   ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS topics        ON hadith TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS quran_verses  ON hadith TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS chapter_name  ON hadith TYPE option<string>;
DEFINE FIELD IF NOT EXISTS embedding     ON hadith TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS hadith_vec    ON TABLE hadith FIELDS embedding HNSW DIMENSION 1024 DIST COSINE;
DEFINE INDEX IF NOT EXISTS hadith_collection ON TABLE hadith FIELDS collection_id;

DEFINE TABLE IF NOT EXISTS collection SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS collection_id ON collection TYPE int;
DEFINE FIELD IF NOT EXISTS name_en       ON collection TYPE string;
DEFINE FIELD IF NOT EXISTS name_ar       ON collection TYPE option<string>;
DEFINE INDEX IF NOT EXISTS collection_id_idx ON collection FIELDS collection_id UNIQUE;

-- === ANALYSIS TABLES ===

DEFINE TABLE IF NOT EXISTS hadith_family SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family_label ON hadith_family TYPE option<string>;
DEFINE FIELD IF NOT EXISTS variant_count ON hadith_family TYPE option<int>;

DEFINE FIELD IF NOT EXISTS family_id ON hadith TYPE option<record<hadith_family>>;
DEFINE INDEX IF NOT EXISTS hadith_family_idx ON TABLE hadith FIELDS family_id;


-- === MUSTALAH ANALYSIS TABLES (structural facts only, no computed grades) ===

DEFINE TABLE IF NOT EXISTS isnad_analysis SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family ON isnad_analysis TYPE record<hadith_family>;
DEFINE FIELD IF NOT EXISTS breadth_class ON isnad_analysis TYPE string;
DEFINE FIELD IF NOT EXISTS min_breadth ON isnad_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS bottleneck_tabaqah ON isnad_analysis TYPE option<int>;
DEFINE FIELD IF NOT EXISTS sahabi_count ON isnad_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS mutabaat_count ON isnad_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS shawahid_count ON isnad_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS chain_count ON isnad_analysis TYPE int;
DEFINE FIELD IF NOT EXISTS ilal_flags ON isnad_analysis TYPE option<array<string>>;
DEFINE INDEX IF NOT EXISTS isnad_family_idx ON TABLE isnad_analysis FIELDS family UNIQUE;

DEFINE TABLE IF NOT EXISTS chain_assessment SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family ON chain_assessment TYPE record<hadith_family>;
DEFINE FIELD IF NOT EXISTS variant ON chain_assessment TYPE record<hadith>;
DEFINE FIELD IF NOT EXISTS continuity ON chain_assessment TYPE string;
DEFINE FIELD IF NOT EXISTS narrator_count ON chain_assessment TYPE int;
DEFINE FIELD IF NOT EXISTS has_chronology_conflict ON chain_assessment TYPE bool;
DEFINE FIELD IF NOT EXISTS narrator_ids ON chain_assessment TYPE option<array<string>>;
DEFINE INDEX IF NOT EXISTS chain_family_idx ON TABLE chain_assessment FIELDS family;

DEFINE TABLE IF NOT EXISTS narrator_pivot SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS family ON narrator_pivot TYPE record<hadith_family>;
DEFINE FIELD IF NOT EXISTS narrator ON narrator_pivot TYPE record<narrator>;
DEFINE FIELD IF NOT EXISTS bundle_coverage ON narrator_pivot TYPE float;
DEFINE FIELD IF NOT EXISTS fan_out ON narrator_pivot TYPE int;
DEFINE FIELD IF NOT EXISTS collector_diversity ON narrator_pivot TYPE int;
DEFINE FIELD IF NOT EXISTS bypass_count ON narrator_pivot TYPE int;
DEFINE FIELD IF NOT EXISTS is_bottleneck ON narrator_pivot TYPE bool;
DEFINE INDEX IF NOT EXISTS pivot_family_idx ON TABLE narrator_pivot FIELDS family;
DEFINE INDEX IF NOT EXISTS pivot_narrator_idx ON TABLE narrator_pivot FIELDS narrator;

-- === EDGES (graph relations) ===

-- "Narrator B heard_from Narrator A" (student -> teacher, toward the Prophet)
DEFINE TABLE IF NOT EXISTS heard_from TYPE RELATION FROM narrator TO narrator;
DEFINE FIELD IF NOT EXISTS hadith_ref ON heard_from TYPE option<record<hadith>>;
DEFINE FIELD IF NOT EXISTS chain_position ON heard_from TYPE option<int>;
DEFINE INDEX IF NOT EXISTS heard_from_in_idx ON TABLE heard_from FIELDS in;
DEFINE INDEX IF NOT EXISTS heard_from_out_idx ON TABLE heard_from FIELDS out;
DEFINE INDEX IF NOT EXISTS heard_from_hadith_ref_idx ON TABLE heard_from FIELDS hadith_ref;

-- "Narrator narrates Hadith" (narrator closest to Bukhari -> hadith)
DEFINE TABLE IF NOT EXISTS narrates TYPE RELATION FROM narrator TO hadith;
DEFINE FIELD IF NOT EXISTS chain_position ON narrates TYPE option<int>;
DEFINE INDEX IF NOT EXISTS narrates_in_idx ON TABLE narrates FIELDS in;
DEFINE INDEX IF NOT EXISTS narrates_out_idx ON TABLE narrates FIELDS out;

-- "Hadith belongs_to Collection"
DEFINE TABLE IF NOT EXISTS belongs_to TYPE RELATION FROM hadith TO collection;
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
DEFINE FIELD IF NOT EXISTS text_ar_lemma   ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS text_en         ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS tafsir_en       ON ayah TYPE option<string>;
DEFINE FIELD IF NOT EXISTS juz             ON ayah TYPE option<int>;
DEFINE FIELD IF NOT EXISTS hizb            ON ayah TYPE option<int>;

DEFINE FIELD IF NOT EXISTS embedding       ON ayah TYPE option<array<float>>;
DEFINE INDEX IF NOT EXISTS ayah_vec        ON TABLE ayah FIELDS embedding HNSW DIMENSION 1024 DIST COSINE;
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

pub async fn init_quran_schema(db: &Surreal<Db>, embed_dim: usize) -> Result<()> {
    let schema = QURAN_SCHEMA.replace("DIMENSION 1024", &format!("DIMENSION {embed_dim}"));
    for (i, stmt) in schema
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
    // Define analyzers + fulltext indexes. Call this BEFORE data ingestion so
    // the table is empty (instant) and each subsequent insert incrementally
    // updates the index, avoiding long-running transactions that hit
    // "memtable history insufficient" errors.
    let stmts = [
        "DEFINE ANALYZER en_analyzer TOKENIZERS blank,class FILTERS lowercase,snowball(english)",
        "DEFINE ANALYZER ar_analyzer TOKENIZERS blank,class",
        "DEFINE INDEX IF NOT EXISTS ayah_text_en_search ON TABLE ayah FIELDS text_en FULLTEXT ANALYZER en_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS ayah_text_ar_search ON TABLE ayah FIELDS text_ar_simple FULLTEXT ANALYZER ar_analyzer BM25 HIGHLIGHTS",
        "DEFINE INDEX IF NOT EXISTS ayah_text_ar_lemma_search ON TABLE ayah FIELDS text_ar_lemma FULLTEXT ANALYZER ar_analyzer BM25 HIGHLIGHTS",
        // ayah_tafsir_en_search index dropped — tafsir search retired.
        // Existing deployments keep the index on disk; run
        //   REMOVE INDEX ayah_tafsir_en_search ON TABLE ayah;
        // to reclaim space.
    ];
    for (i, stmt) in stmts.iter().enumerate() {
        if let Err(e) = db.query(*stmt).await.and_then(|r| r.check()) {
            let msg = e.to_string();
            if msg.contains("already exists") {
                continue;
            }
            tracing::error!("Quran fulltext index {i} failed: {e}");
            return Err(e.into());
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

// ── User Notes Schema ──

const USER_NOTE_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS user_note SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS device_id   ON user_note TYPE string;
DEFINE FIELD IF NOT EXISTS ref_type    ON user_note TYPE string;
DEFINE FIELD IF NOT EXISTS ref_id      ON user_note TYPE option<string>;
DEFINE FIELD IF NOT EXISTS title       ON user_note TYPE option<string>;
DEFINE FIELD IF NOT EXISTS content     ON user_note TYPE string;
DEFINE FIELD IF NOT EXISTS color       ON user_note TYPE string;
DEFINE FIELD IF NOT EXISTS tags        ON user_note TYPE option<array<string>>;
DEFINE FIELD IF NOT EXISTS refs        ON user_note TYPE option<string>;
DEFINE FIELD IF NOT EXISTS created_at  ON user_note TYPE option<string>;
DEFINE FIELD IF NOT EXISTS updated_at  ON user_note TYPE option<string>;
DEFINE INDEX IF NOT EXISTS note_device_ref ON TABLE user_note FIELDS device_id, ref_type, ref_id
"#;

// ── Link Preview Cache Schema ──

const LINK_PREVIEW_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS link_preview SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS url         ON link_preview TYPE string;
DEFINE FIELD IF NOT EXISTS title       ON link_preview TYPE option<string>;
DEFINE FIELD IF NOT EXISTS description ON link_preview TYPE option<string>;
DEFINE FIELD IF NOT EXISTS image       ON link_preview TYPE option<string>;
DEFINE FIELD IF NOT EXISTS domain      ON link_preview TYPE option<string>;
DEFINE FIELD IF NOT EXISTS fetched_at  ON link_preview TYPE option<string>;
DEFINE INDEX IF NOT EXISTS lp_url ON TABLE link_preview FIELDS url UNIQUE
"#;

pub async fn init_user_note_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in USER_NOTE_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "User note schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("User note schema initialized");
    Ok(())
}

pub async fn init_link_preview_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in LINK_PREVIEW_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Link preview schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Link preview schema initialized");
    Ok(())
}

// ── Book Schema (source-agnostic: turath, shamela, openiti, etc.) ──

const BOOK_SCHEMA: &str = r#"
DEFINE TABLE IF NOT EXISTS book SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS book_id     ON book TYPE int;
DEFINE FIELD IF NOT EXISTS name_ar     ON book TYPE string;
DEFINE FIELD IF NOT EXISTS name_en     ON book TYPE string;
DEFINE FIELD IF NOT EXISTS author_ar   ON book TYPE string;
DEFINE FIELD IF NOT EXISTS total_pages ON book TYPE int;
DEFINE FIELD IF NOT EXISTS headings    ON book TYPE option<string>;
DEFINE FIELD IF NOT EXISTS category    ON book TYPE option<string>;
DEFINE FIELD IF NOT EXISTS book_type   ON book TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source      ON book TYPE option<string>;
DEFINE FIELD IF NOT EXISTS source_id   ON book TYPE option<string>;
DEFINE FIELD IF NOT EXISTS tags        ON book TYPE option<array<string>>;
DEFINE INDEX IF NOT EXISTS book_id_idx ON book FIELDS book_id UNIQUE;

DEFINE TABLE IF NOT EXISTS book_page SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS book_id     ON book_page TYPE int;
DEFINE FIELD IF NOT EXISTS page_index  ON book_page TYPE int;
DEFINE FIELD IF NOT EXISTS text        ON book_page TYPE string;
DEFINE FIELD IF NOT EXISTS vol         ON book_page TYPE string;
DEFINE FIELD IF NOT EXISTS page_num    ON book_page TYPE int;
DEFINE INDEX IF NOT EXISTS book_page_lookup ON book_page FIELDS book_id, page_index UNIQUE;

DEFINE TABLE IF NOT EXISTS tafsir_ayah_map SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS surah       ON tafsir_ayah_map TYPE int;
DEFINE FIELD IF NOT EXISTS ayah        ON tafsir_ayah_map TYPE int;
DEFINE FIELD IF NOT EXISTS book_id     ON tafsir_ayah_map TYPE int;
DEFINE FIELD IF NOT EXISTS page_index  ON tafsir_ayah_map TYPE int;
DEFINE FIELD IF NOT EXISTS heading     ON tafsir_ayah_map TYPE option<string>;
DEFINE INDEX IF NOT EXISTS tafsir_ayah_lookup ON tafsir_ayah_map FIELDS surah, ayah UNIQUE;

DEFINE TABLE IF NOT EXISTS hadith_sharh_map SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS hadith_number ON hadith_sharh_map TYPE int;
DEFINE FIELD IF NOT EXISTS collection_id ON hadith_sharh_map TYPE int;
DEFINE FIELD IF NOT EXISTS book_id       ON hadith_sharh_map TYPE int;
DEFINE FIELD IF NOT EXISTS page_index    ON hadith_sharh_map TYPE int;
DEFINE FIELD IF NOT EXISTS context       ON hadith_sharh_map TYPE option<string>;
DEFINE INDEX IF NOT EXISTS hadith_sharh_lookup ON hadith_sharh_map FIELDS hadith_number, collection_id UNIQUE;

DEFINE TABLE IF NOT EXISTS narrator_book_map SCHEMAFULL;
DEFINE FIELD IF NOT EXISTS narrator_id ON narrator_book_map TYPE string;
DEFINE FIELD IF NOT EXISTS book_id     ON narrator_book_map TYPE int;
DEFINE FIELD IF NOT EXISTS page_index  ON narrator_book_map TYPE int;
DEFINE FIELD IF NOT EXISTS entry_num   ON narrator_book_map TYPE option<int>;
DEFINE FIELD IF NOT EXISTS book_name   ON narrator_book_map TYPE string;
DEFINE INDEX IF NOT EXISTS narrator_book_lookup ON narrator_book_map FIELDS narrator_id, book_id UNIQUE
"#;

pub async fn init_book_schema(db: &Surreal<Db>) -> Result<()> {
    for (i, stmt) in BOOK_SCHEMA
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .enumerate()
    {
        let sql = format!("{stmt};");
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!(
                "Book schema statement {i} failed: {e}\n  SQL: {}",
                stmt.chars().take(120).collect::<String>()
            );
            return Err(e.into());
        }
    }
    tracing::info!("Book schema initialized");
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
