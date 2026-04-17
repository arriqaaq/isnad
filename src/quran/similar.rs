//! Ingest mutashabihat (shared phrases) and similar ayahs from QUL JSON files.

use std::collections::HashMap;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::ingest::batch::{Batch, batch_size_from_env};

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Convert QUL verse_key "2:255" to our record key "2_255".
fn verse_key_to_record_key(vk: &str) -> String {
    vk.replace(':', "_")
}

/// Validate that a record key looks like `<i64>_<i64>` so the RELATE
/// targets can't punch a hole in a transaction (similar_to / shares_phrase
/// have strict typed schemas — a single bad ref aborts the whole batch).
fn valid_ayah_key(rk: &str) -> bool {
    let mut parts = rk.split('_');
    let s = parts.next().and_then(|s| s.parse::<i64>().ok());
    let a = parts.next().and_then(|a| a.parse::<i64>().ok());
    s.is_some() && a.is_some() && parts.next().is_none()
}

/// Ingest shared phrases (mutashabihat) and similar ayahs from QUL JSON files.
pub async fn ingest_similar(db: &Surreal<Db>, qul_dir: &str) -> Result<()> {
    // Clear existing data for re-runnability
    println!("🗑  Clearing existing similar/phrase data...");
    db.query("DELETE shares_phrase; DELETE similar_to; DELETE quran_phrase")
        .await?
        .check()?;

    ingest_phrases(db, qul_dir).await?;
    ingest_similar_ayahs(db, qul_dir).await?;
    Ok(())
}

async fn ingest_phrases(db: &Surreal<Db>, qul_dir: &str) -> Result<()> {
    let path = format!("{qul_dir}/phrases.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            println!("   Skipping phrases.json (not found at {path})");
            return Ok(());
        }
    };

    let data: HashMap<String, serde_json::Value> = serde_json::from_str(&content)?;
    println!("   Found {} shared phrases", data.len());

    let pb = indicatif::ProgressBar::new(data.len() as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} phrases ({eta})")
            .unwrap(),
    );

    // Materialise into a Vec so we can chunk it deterministically.
    let entries: Vec<(&String, &serde_json::Value)> = data.iter().collect();
    // Phrases bring 1 CREATE + N RELATEs each (typically a few RELATEs); 100
    // phrases per batch keeps the SurrealQL size sane.
    let batch_size = batch_size_from_env(100);

    for chunk in entries.chunks(batch_size) {
        let mut b = Batch::new();
        for (phrase_id, entry) in chunk {
            let occurrence = entry["count"].as_i64().unwrap_or(0);
            let verses_count = entry["ayahs"].as_i64().unwrap_or(0);
            let chapters_count = entry["surahs"].as_i64().unwrap_or(0);

            // Try to get text from source ayah
            let source_key = entry["source"]["key"].as_str().unwrap_or("");
            let source_from = entry["source"]["from"].as_i64().unwrap_or(0);
            let source_to = entry["source"]["to"].as_i64().unwrap_or(0);

            // Build a text placeholder from the source ayah words. This SELECT
            // stays per-phrase (small + indexed); only writes are batched.
            let text_ar = if !source_key.is_empty() {
                let rk = verse_key_to_record_key(source_key);
                let words_text = get_phrase_text(db, &rk, source_from, source_to).await;
                words_text.unwrap_or_else(|| format!("phrase_{phrase_id}"))
            } else {
                format!("phrase_{phrase_id}")
            };

            // Create phrase node first so the RELATEs below resolve within
            // the same transaction.
            let p_phrase_rid = b.param(rid("quran_phrase", phrase_id));
            let p_text_ar = b.param(text_ar);
            let p_occurrence = b.param(occurrence);
            let p_verses_count = b.param(verses_count);
            let p_chapters_count = b.param(chapters_count);
            b.push(format!(
                "CREATE {p_phrase_rid} CONTENT {{ \
                 text_ar: {p_text_ar}, text_ar_simple: NONE, \
                 occurrence: {p_occurrence}, verses_count: {p_verses_count}, \
                 chapters_count: {p_chapters_count} }}"
            ));

            // Reuse the phrase param for every RELATE in this iteration.
            if let Some(ayah_map) = entry["ayah"].as_object() {
                for (vk, ranges) in ayah_map {
                    let ayah_rk = verse_key_to_record_key(vk);
                    // shares_phrase is RELATION IN ayah OUT quran_phrase — skip
                    // junk keys instead of poisoning the whole batch.
                    if !valid_ayah_key(&ayah_rk) {
                        continue;
                    }
                    if let Some(range_arr) = ranges.as_array() {
                        for range in range_arr {
                            if let Some(pair) = range.as_array() {
                                let from = pair.first().and_then(|v| v.as_i64()).unwrap_or(0);
                                let to = pair.get(1).and_then(|v| v.as_i64()).unwrap_or(0);

                                let p_in = b.param(rid("ayah", &ayah_rk));
                                let p_from = b.param(from);
                                let p_to = b.param(to);
                                b.push(format!(
                                    "RELATE {p_in} -> shares_phrase -> {p_phrase_rid} \
                                     SET word_from = {p_from}, word_to = {p_to}"
                                ));
                            }
                        }
                    }
                }
            }
        }
        b.commit(db).await?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("done");
    println!("   {} shared phrases ingested", data.len());
    Ok(())
}

/// Try to extract phrase text by reading word records from the source ayah.
/// Falls back to extracting from the ayah's full text_ar by word position if quran_word table is empty.
async fn get_phrase_text(db: &Surreal<Db>, ayah_rk: &str, from: i64, to: i64) -> Option<String> {
    let parts: Vec<&str> = ayah_rk.split('_').collect();
    if parts.len() != 2 {
        return None;
    }
    let surah: i64 = parts[0].parse().ok()?;
    let ayah: i64 = parts[1].parse().ok()?;

    // Try quran_word table first (most accurate)
    #[derive(Debug, SurrealValue)]
    struct WordText {
        text_ar: String,
    }

    let mut res = db
        .query(
            "SELECT text_ar FROM quran_word WHERE surah_number = $s AND ayah_number = $a \
             AND word_position >= $from AND word_position <= $to \
             ORDER BY word_position",
        )
        .bind(("s", surah))
        .bind(("a", ayah))
        .bind(("from", from))
        .bind(("to", to))
        .await
        .ok()?;

    let words: Vec<WordText> = res.take(0).ok()?;
    if !words.is_empty() {
        let text: Vec<&str> = words.iter().map(|w| w.text_ar.as_str()).collect();
        return Some(text.join(" "));
    }

    // Fallback: extract word range from the ayah's full Arabic text
    #[derive(Debug, SurrealValue)]
    struct AyahText {
        text_ar: String,
    }
    let mut res = db
        .query("SELECT text_ar FROM ayah WHERE surah_number = $s AND ayah_number = $a")
        .bind(("s", surah))
        .bind(("a", ayah))
        .await
        .ok()?;
    let ayahs: Vec<AyahText> = res.take(0).ok()?;
    let full_text = &ayahs.first()?.text_ar;
    let all_words: Vec<&str> = full_text.split_whitespace().collect();
    let start = (from as usize).saturating_sub(1);
    let end = (to as usize).min(all_words.len());
    if start < end {
        Some(all_words[start..end].join(" "))
    } else {
        None
    }
}

async fn ingest_similar_ayahs(db: &Surreal<Db>, qul_dir: &str) -> Result<()> {
    // QUL exports as "matching-ayah.json" (not "similar_ayahs.json")
    let path = format!("{qul_dir}/matching-ayah.json");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            println!("   Skipping similar_ayahs.json (not found at {path})");
            return Ok(());
        }
    };

    let data: HashMap<String, Vec<serde_json::Value>> = serde_json::from_str(&content)?;

    // Count total edges
    let total: usize = data.values().map(|v| v.len()).sum();
    println!(
        "   Found {} similar ayah pairs across {} source ayahs",
        total,
        data.len()
    );

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} similar pairs ({eta})")
            .unwrap(),
    );

    // Flatten (from_rk, m) pairs and drop ones with unparseable keys up-front;
    // similar_to is RELATION IN ayah OUT ayah, so a single bad RELATE in a
    // batch would roll back the whole transaction.
    let mut pairs: Vec<(String, String, &serde_json::Value)> = Vec::with_capacity(total);
    let mut skipped = 0usize;
    for (vk, matches) in &data {
        let from_rk = verse_key_to_record_key(vk);
        if !valid_ayah_key(&from_rk) {
            skipped += matches.len();
            continue;
        }
        for m in matches {
            let matched_key = m["matched_ayah_key"].as_str().unwrap_or("");
            if matched_key.is_empty() {
                skipped += 1;
                continue;
            }
            let to_rk = verse_key_to_record_key(matched_key);
            if !valid_ayah_key(&to_rk) {
                skipped += 1;
                continue;
            }
            pairs.push((from_rk.clone(), to_rk, m));
        }
    }
    if skipped > 0 {
        println!("   ⚠ skipped {skipped} pairs with invalid ayah keys");
    }

    // Tiny RELATE statements — bigger chunks than the default work well.
    let batch_size = batch_size_from_env(500);
    for chunk in pairs.chunks(batch_size) {
        let mut b = Batch::new();
        for (from_rk, to_rk, m) in chunk {
            let score = m["score"].as_i64().unwrap_or(0);
            let coverage = m["coverage"].as_i64().unwrap_or(0);
            let positions_json = m.get("match_words").map(|v| v.to_string());

            let p_in = b.param(rid("ayah", from_rk));
            let p_out = b.param(rid("ayah", to_rk));
            let p_score = b.param(score);
            let p_coverage = b.param(coverage);
            let p_positions = b.param(positions_json);
            b.push(format!(
                "RELATE {p_in} -> similar_to -> {p_out} \
                 SET score = {p_score}, coverage = {p_coverage}, matched_positions = {p_positions}"
            ));
        }
        b.commit(db).await?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("done");
    println!("   {} similar ayah edges ingested", pairs.len());
    Ok(())
}
