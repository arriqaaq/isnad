use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;

// ── JSON file structures ──

#[derive(Debug, Deserialize)]
struct PageMeta {
    #[allow(dead_code)]
    page_id: Option<u32>,
    page: Option<u32>,
    vol: Option<String>,
    #[allow(dead_code)]
    headings: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawPage {
    page_id: u32,
    meta: serde_json::Value,
    text: String,
}

#[derive(Debug, Deserialize)]
struct HeadingsFile {
    meta: serde_json::Value,
    indexes: IndexesData,
}

#[derive(Debug, Deserialize)]
struct IndexesData {
    headings: Vec<RawHeading>,
    #[allow(dead_code)]
    volumes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawHeading {
    title: String,
    level: u32,
    page: u32,
}

#[derive(Debug, Deserialize)]
struct TafsirMappingEntry {
    page_index: u32,
    heading: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SharhMappingEntry {
    page_index: u32,
    #[allow(dead_code)]
    context: Option<String>,
}

fn parse_meta(raw: &serde_json::Value) -> PageMeta {
    match raw {
        serde_json::Value::String(s) => serde_json::from_str(s).unwrap_or(PageMeta {
            page_id: None,
            page: None,
            vol: None,
            headings: None,
        }),
        serde_json::Value::Object(_) => serde_json::from_value(raw.clone()).unwrap_or(PageMeta {
            page_id: None,
            page: None,
            vol: None,
            headings: None,
        }),
        _ => PageMeta {
            page_id: None,
            page: None,
            vol: None,
            headings: None,
        },
    }
}

/// Ingest a turath book (pages + headings) into SurrealDB.
/// Works for any book — just pass the right book_id, name, and author.
pub async fn ingest_book(
    db: &Surreal<Db>,
    pages_file: &str,
    headings_file: &str,
    book_id: u32,
    name_ar: &str,
    name_en: &str,
    author_ar: &str,
    category: Option<&str>,
    book_type: Option<&str>,
) -> Result<()> {
    crate::db::init_turath_schema(db).await?;

    // Check if this specific book is already ingested
    let count: Option<CountResult> = db
        .query("SELECT count() AS c FROM turath_page WHERE book_id = $bid GROUP ALL")
        .bind(("bid", book_id as i64))
        .await?
        .take(0)?;
    if count.map(|c| c.c as u64).unwrap_or(0) > 0 {
        tracing::info!("Turath book {book_id} already ingested, skipping");
        return Ok(());
    }

    // 1. Load headings
    tracing::info!("Loading headings from {headings_file}...");
    let headings_raw = std::fs::read_to_string(headings_file)?;
    let headings_data: HeadingsFile = serde_json::from_str(&headings_raw)?;

    let headings_json: Vec<serde_json::Value> = headings_data
        .indexes
        .headings
        .iter()
        .map(|h| {
            serde_json::json!({
                "title": h.title,
                "level": h.level,
                "page_index": h.page
            })
        })
        .collect();
    let headings_str = serde_json::to_string(&headings_json)?;

    // 2. Load pages
    tracing::info!("Loading pages from {pages_file}...");
    let pages_raw = std::fs::read_to_string(pages_file)?;
    let pages: Vec<RawPage> = serde_json::from_str(&pages_raw)?;
    let total_pages = pages.len() as u32;
    tracing::info!("Loaded {total_pages} pages for book {book_id}");

    // Insert book record
    let escaped_name_ar = name_ar.replace('\'', "\\'");
    let escaped_name_en = name_en.replace('\'', "\\'");
    let escaped_author = author_ar.replace('\'', "\\'");
    let escaped_headings = headings_str.replace('\'', "\\'");
    let cat_clause = category
        .map(|c| format!(", category = '{c}'"))
        .unwrap_or_default();
    let type_clause = book_type
        .map(|t| format!(", book_type = '{t}'"))
        .unwrap_or_default();
    db.query(&format!(
        "CREATE turath_book SET book_id = {book_id}, name_ar = '{escaped_name_ar}', \
         name_en = '{escaped_name_en}', author_ar = '{escaped_author}', \
         total_pages = {total_pages}, headings = '{escaped_headings}'{cat_clause}{type_clause}"
    ))
    .await?
    .check()?;
    tracing::info!("Inserted turath_book record for {name_en} (book_id={book_id})");

    // 3. Insert pages in batches
    let batch_size = 100;
    let bar = indicatif::ProgressBar::new(total_pages as u64);
    bar.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40} {pos}/{len} pages")?,
    );

    for chunk in pages.chunks(batch_size) {
        let mut sql = String::new();
        for (i, page) in chunk.iter().enumerate() {
            let meta = parse_meta(&page.meta);
            let vol = meta.vol.unwrap_or_else(|| "1".to_string());
            let page_num = meta.page.unwrap_or(0);
            let page_index = page.page_id - 1;

            let escaped_text = page.text.replace('\\', "\\\\").replace('\'', "\\'");
            let escaped_vol = vol.replace('\'', "\\'");

            sql.push_str(&format!(
                "CREATE turath_page SET book_id = {book_id}, page_index = {page_index}, \
                 text = '{escaped_text}', vol = '{escaped_vol}', page_num = {page_num};\n"
            ));

            if i % 10 == 9 || i == chunk.len() - 1 {
                if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
                    tracing::error!("Failed to insert page batch: {e}");
                    for stmt in sql.split(";\n").filter(|s| !s.is_empty()) {
                        let _ = db.query(stmt).await;
                    }
                }
                sql.clear();
            }
        }
        bar.inc(chunk.len() as u64);
    }
    bar.finish();
    tracing::info!("Inserted {total_pages} turath pages for book {book_id}");

    Ok(())
}

/// Ingest tafsir ayah→page mapping (for Tafsir Ibn Kathir).
pub async fn ingest_tafsir_mapping(
    db: &Surreal<Db>,
    mapping_file: &str,
    book_id: u32,
) -> Result<()> {
    // Check if already ingested
    let count: Option<CountResult> = db
        .query("SELECT count() AS c FROM tafsir_ayah_map WHERE book_id = $bid GROUP ALL")
        .bind(("bid", book_id as i64))
        .await?
        .take(0)?;
    if count.map(|c| c.c as u64).unwrap_or(0) > 0 {
        tracing::info!("Tafsir ayah mapping for book {book_id} already ingested, skipping");
        return Ok(());
    }

    tracing::info!("Loading tafsir verse mapping from {mapping_file}...");
    let mapping_raw = std::fs::read_to_string(mapping_file)?;
    let mapping: HashMap<String, TafsirMappingEntry> = serde_json::from_str(&mapping_raw)?;
    tracing::info!("Loaded {} ayah mappings", mapping.len());

    let mut sql = String::new();
    let mut inserted = 0;
    for (key, entry) in &mapping {
        let parts: Vec<&str> = key.split(':').collect();
        if parts.len() != 2 {
            continue;
        }
        let surah: u32 = parts[0].parse().unwrap_or(0);
        let ayah: u32 = parts[1].parse().unwrap_or(0);
        if surah == 0 || ayah == 0 {
            continue;
        }

        let heading_sql = match &entry.heading {
            Some(h) => format!("'{}'", h.replace('\'', "\\'")),
            None => "NONE".to_string(),
        };

        sql.push_str(&format!(
            "CREATE tafsir_ayah_map SET surah = {surah}, ayah = {ayah}, \
             book_id = {book_id}, page_index = {}, heading = {heading_sql};\n",
            entry.page_index
        ));
        inserted += 1;

        if inserted % 200 == 0 {
            if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
                tracing::error!("Failed to insert tafsir mapping batch: {e}");
            }
            sql.clear();
        }
    }
    if !sql.is_empty() {
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!("Failed to insert final tafsir mapping batch: {e}");
        }
    }
    tracing::info!("Inserted {inserted} tafsir ayah mappings for book {book_id}");
    Ok(())
}

/// Ingest hadith→sharh page mapping (e.g. Bukhari hadith numbers → Fath al-Bari pages).
pub async fn ingest_hadith_sharh_mapping(
    db: &Surreal<Db>,
    mapping_file: &str,
    collection_book_id: u32, // e.g. 1 for Bukhari in our DB
    sharh_book_id: u32,      // e.g. 1673 for Fath al-Bari on turath
) -> Result<()> {
    // Check if already ingested
    let count: Option<CountResult> = db
        .query(
            "SELECT count() AS c FROM hadith_sharh_map \
             WHERE book_id = $bid AND sharh_book_id = $sbid GROUP ALL",
        )
        .bind(("bid", collection_book_id as i64))
        .bind(("sbid", sharh_book_id as i64))
        .await?
        .take(0)?;
    if count.map(|c| c.c as u64).unwrap_or(0) > 0 {
        tracing::info!(
            "Hadith sharh mapping for book_id={collection_book_id} → sharh={sharh_book_id} already ingested"
        );
        return Ok(());
    }

    tracing::info!("Loading hadith sharh mapping from {mapping_file}...");
    let mapping_raw = std::fs::read_to_string(mapping_file)?;
    let mapping: HashMap<String, SharhMappingEntry> = serde_json::from_str(&mapping_raw)?;
    tracing::info!("Loaded {} hadith→sharh mappings", mapping.len());

    let mut sql = String::new();
    let mut inserted = 0;
    for (key, entry) in &mapping {
        let hadith_number: u32 = match key.parse() {
            Ok(n) => n,
            Err(_) => continue,
        };

        sql.push_str(&format!(
            "CREATE hadith_sharh_map SET hadith_number = {hadith_number}, \
             book_id = {collection_book_id}, sharh_book_id = {sharh_book_id}, \
             page_index = {};\n",
            entry.page_index
        ));
        inserted += 1;

        if inserted % 200 == 0 {
            if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
                tracing::error!("Failed to insert hadith sharh mapping batch: {e}");
            }
            sql.clear();
        }
    }
    if !sql.is_empty() {
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!("Failed to insert final hadith sharh mapping batch: {e}");
        }
    }
    tracing::info!(
        "Inserted {inserted} hadith sharh mappings (book_id={collection_book_id} → sharh={sharh_book_id})"
    );
    Ok(())
}

/// Ingest narrator→book page mapping (e.g. narrators → Tahdhib al-Tahdhib pages).
pub async fn ingest_narrator_book_mapping(
    db: &Surreal<Db>,
    mapping_file: &str,
    turath_book_id: u32,
    book_name: &str,
) -> Result<()> {
    // Check if already ingested
    let count: Option<CountResult> = db
        .query(
            "SELECT count() AS c FROM narrator_book_map \
             WHERE turath_book_id = $bid GROUP ALL",
        )
        .bind(("bid", turath_book_id as i64))
        .await?
        .take(0)?;
    if count.map(|c| c.c as u64).unwrap_or(0) > 0 {
        tracing::info!(
            "Narrator book mapping for turath_book_id={turath_book_id} already ingested"
        );
        return Ok(());
    }

    tracing::info!("Loading narrator book mapping from {mapping_file}...");
    let mapping_raw = std::fs::read_to_string(mapping_file)?;
    let mapping: HashMap<String, NarratorBookEntry> = serde_json::from_str(&mapping_raw)?;
    tracing::info!("Loaded {} narrator→book mappings", mapping.len());

    let escaped_book_name = book_name.replace('\'', "\\'");
    let mut sql = String::new();
    let mut inserted = 0;
    for (narrator_id, entry) in &mapping {
        let escaped_nid = narrator_id.replace('\'', "\\'");
        let entry_num_sql = match entry.entry_num {
            Some(n) => n.to_string(),
            None => "NONE".to_string(),
        };

        sql.push_str(&format!(
            "CREATE narrator_book_map SET narrator_id = '{escaped_nid}', \
             turath_book_id = {turath_book_id}, page_index = {}, \
             entry_num = {entry_num_sql}, book_name = '{escaped_book_name}';\n",
            entry.page_index
        ));
        inserted += 1;

        if inserted % 200 == 0 {
            if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
                tracing::error!("Failed to insert narrator book mapping batch: {e}");
            }
            sql.clear();
        }
    }
    if !sql.is_empty() {
        if let Err(e) = db.query(&sql).await.and_then(|r| r.check()) {
            tracing::error!("Failed to insert final narrator book mapping batch: {e}");
        }
    }
    tracing::info!("Inserted {inserted} narrator→book mappings for {book_name}");
    Ok(())
}

#[derive(Debug, Deserialize)]
struct NarratorBookEntry {
    page_index: u32,
    entry_num: Option<u32>,
    #[allow(dead_code)]
    book_name: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct CountResult {
    c: i64,
}
