//! Quran word morphology ingest from corpus.quran.com + QUL word-by-word data.
//!
//! Data sources:
//! - `quran-morphology.txt` from mustafa0x/quran-morphology (corpus.quran.com v0.4 fork)
//! - QUL word translation JSON: `{ "1:1:1": "In (the) name", ... }`
//! - QUL transliteration JSON: `{ "1:1": "Bismillahi r-rahmani r-raheem", ... }`

use std::collections::HashMap;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use crate::db::Db;
use crate::ingest::batch::{Batch, batch_size_from_env};
use crate::quran::ingest::strip_arabic_diacritics;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Strip HTML tags from a string, e.g. "<span class='n'>name</span>" → "name"
fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            out.push(c);
        }
    }
    // Collapse multiple spaces
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A single morpheme segment parsed from one line of quran-morphology.txt.
struct RawSegment {
    text_ar: String,
    pos: String,
    features: HashMap<String, String>,
    is_prefix: bool,
    is_suffix: bool,
}

/// A complete word (all segments grouped) ready for DB insertion.
struct ParsedWord {
    surah: i64,
    ayah: i64,
    word_pos: i64,
    text_ar: String,
    pos: String,
    root: Option<String>,
    lemma: Option<String>,
    features: serde_json::Value,
    segments: Vec<serde_json::Value>,
}

/// Parse the feature string like "ROOT:سمو|LEM:اسْم|M|GEN" into a map.
fn parse_features(tag_str: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for part in tag_str.split('|') {
        if let Some((key, val)) = part.split_once(':') {
            map.insert(key.to_string(), val.to_string());
        } else {
            // Standalone tags: M, F, S, P, D, NOM, GEN, ACC, ADJ, PN, PRON, etc.
            match part {
                "M" => {
                    map.insert("gender".into(), "M".into());
                }
                "F" => {
                    map.insert("gender".into(), "F".into());
                }
                "MS" => {
                    map.insert("gender".into(), "M".into());
                    map.insert("number".into(), "S".into());
                }
                "FS" => {
                    map.insert("gender".into(), "F".into());
                    map.insert("number".into(), "S".into());
                }
                "MD" => {
                    map.insert("gender".into(), "M".into());
                    map.insert("number".into(), "D".into());
                }
                "FD" => {
                    map.insert("gender".into(), "F".into());
                    map.insert("number".into(), "D".into());
                }
                "MP" => {
                    map.insert("gender".into(), "M".into());
                    map.insert("number".into(), "P".into());
                }
                "FP" => {
                    map.insert("gender".into(), "F".into());
                    map.insert("number".into(), "P".into());
                }
                "S" if !map.contains_key("number") => {
                    map.insert("number".into(), "S".into());
                }
                "P" if !map.contains_key("number") => {
                    map.insert("number".into(), "P".into());
                }
                "D" if !map.contains_key("number") => {
                    map.insert("number".into(), "D".into());
                }
                "NOM" => {
                    map.insert("case".into(), "NOM".into());
                }
                "GEN" => {
                    map.insert("case".into(), "GEN".into());
                }
                "ACC" => {
                    map.insert("case".into(), "ACC".into());
                }
                "PERF" => {
                    map.insert("aspect".into(), "PERF".into());
                }
                "IMPF" => {
                    map.insert("aspect".into(), "IMPF".into());
                }
                "IMPV" => {
                    map.insert("aspect".into(), "IMPV".into());
                }
                "PASS" => {
                    map.insert("voice".into(), "PASS".into());
                }
                "ACT_PCPL" => {
                    map.insert("derivation".into(), "ACT_PCPL".into());
                }
                "PASS_PCPL" => {
                    map.insert("derivation".into(), "PASS_PCPL".into());
                }
                "ADJ" => {
                    map.insert("type".into(), "ADJ".into());
                }
                "PN" => {
                    map.insert("type".into(), "PN".into());
                }
                "PRON" => {
                    map.insert("type".into(), "PRON".into());
                }
                "REL" => {
                    map.insert("type".into(), "REL".into());
                }
                "DEM" => {
                    map.insert("type".into(), "DEM".into());
                }
                "CONJ" => {
                    map.insert("type".into(), "CONJ".into());
                }
                "DET" => {
                    map.insert("type".into(), "DET".into());
                }
                "PREF" => {
                    map.insert("affix".into(), "PREF".into());
                }
                "SUFF" => {
                    map.insert("affix".into(), "SUFF".into());
                }
                "NV" => {
                    map.insert("type".into(), "NV".into());
                }
                // Person tags like "1P", "2MS", "3FS", "1S", "2MP", etc.
                s if s.len() >= 2 && s.starts_with(|c: char| c.is_ascii_digit()) => {
                    let person = &s[0..1];
                    map.insert("person".into(), person.to_string());
                    if s.len() >= 3 {
                        map.insert("gender".into(), s[1..2].to_string());
                        map.insert("number".into(), s[2..3].to_string());
                    } else if s.len() == 2 {
                        map.insert("number".into(), s[1..2].to_string());
                    }
                }
                _ => {
                    map.insert(part.to_string(), "true".into());
                }
            }
        }
    }
    map
}

/// Parse the morphology file into grouped words.
fn parse_morphology_file(path: &str) -> Result<Vec<ParsedWord>> {
    let content = std::fs::read_to_string(path)?;

    // Group segments by (surah, ayah, word_pos)
    let mut word_segments: HashMap<(i64, i64, i64), Vec<RawSegment>> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }

        let loc_parts: Vec<&str> = parts[0].split(':').collect();
        if loc_parts.len() < 4 {
            continue;
        }

        let surah: i64 = loc_parts[0].parse()?;
        let ayah: i64 = loc_parts[1].parse()?;
        let word_pos: i64 = loc_parts[2].parse()?;

        let text_ar = parts[1].to_string();
        let pos = parts[2].to_string();
        let features = parse_features(parts[3]);

        let is_prefix = features.get("affix").is_some_and(|v| v == "PREF");
        let is_suffix = features.get("affix").is_some_and(|v| v == "SUFF");

        word_segments
            .entry((surah, ayah, word_pos))
            .or_default()
            .push(RawSegment {
                text_ar,
                pos,
                features,
                is_prefix,
                is_suffix,
            });
    }

    // Build ParsedWord from grouped segments
    let mut words: Vec<ParsedWord> = Vec::with_capacity(word_segments.len());

    for ((surah, ayah, word_pos), segments) in &word_segments {
        // Concatenate all segment texts to form the full word
        let text_ar: String = segments.iter().map(|s| s.text_ar.as_str()).collect();

        // Find the "main" segment (not prefix/suffix) for the word-level POS, root, lemma
        let main_seg = segments
            .iter()
            .find(|s| !s.is_prefix && !s.is_suffix)
            .unwrap_or(&segments[0]);

        let pos = main_seg.pos.clone();
        let root = main_seg.features.get("ROOT").cloned();
        let lemma = main_seg.features.get("LEM").cloned();

        // Build features object from the main segment
        let mut feat_map = serde_json::Map::new();
        for (k, v) in &main_seg.features {
            match k.as_str() {
                "ROOT" | "LEM" | "affix" => {} // stored in dedicated fields
                _ => {
                    feat_map.insert(k.clone(), serde_json::Value::String(v.clone()));
                }
            }
        }

        // Build segments array
        let seg_array: Vec<serde_json::Value> = segments
            .iter()
            .map(|s| {
                let mut m = serde_json::Map::new();
                m.insert("pos".into(), serde_json::Value::String(s.pos.clone()));
                m.insert("text".into(), serde_json::Value::String(s.text_ar.clone()));
                if s.is_prefix {
                    m.insert("affix".into(), serde_json::Value::String("PREF".into()));
                }
                if s.is_suffix {
                    m.insert("affix".into(), serde_json::Value::String("SUFF".into()));
                }
                if let Some(root) = s.features.get("ROOT") {
                    m.insert("root".into(), serde_json::Value::String(root.clone()));
                }
                serde_json::Value::Object(m)
            })
            .collect();

        words.push(ParsedWord {
            surah: *surah,
            ayah: *ayah,
            word_pos: *word_pos,
            text_ar,
            pos,
            root,
            lemma,
            features: serde_json::Value::Object(feat_map),
            segments: seg_array,
        });
    }

    // Sort by surah, ayah, word_pos for deterministic output
    words.sort_by_key(|w| (w.surah, w.ayah, w.word_pos));

    Ok(words)
}

/// Ingest morphology data + optional QUL word translations/transliteration.
pub async fn ingest_morphology(db: &Surreal<Db>, morph_path: &str, qul_dir: &str) -> Result<()> {
    // 0. Clear any existing word data (re-runnable)
    println!("🗑  Clearing existing quran_word records...");
    db.query("DELETE quran_word").await?.check()?;

    // 1. Parse morphology file
    println!("📖 Parsing morphology data from {morph_path}...");
    let words = parse_morphology_file(morph_path)?;
    println!("   Found {} unique words", words.len());

    // 2. Load QUL word translations if available
    // Try colored version first (in data/ root), then plain version in qul_dir
    let word_translations =
        load_qul_json(&format!("{qul_dir}/colored-english-wbw-translation.json"));
    if word_translations.is_some() {
        println!("   ✓ Loaded QUL word translations");
    }

    // 3. Load QUL transliteration if available
    let transliterations = load_qul_json(&format!("{qul_dir}/transliteration.json"));
    if transliterations.is_some() {
        println!("   ✓ Loaded QUL transliterations");
    }

    // 4. Insert words into SurrealDB
    println!("📝 Inserting words into database...");
    let total = words.len();
    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} words ({eta})")
            .unwrap(),
    );

    let batch_size = batch_size_from_env(200);
    for chunk in words.chunks(batch_size) {
        let mut b = Batch::new();
        for word in chunk {
            let key = format!("{}_{}_{}", word.surah, word.ayah, word.word_pos);
            let text_ar_simple = strip_arabic_diacritics(&word.text_ar);

            // Look up QUL word translation: key format "surah:ayah:word"
            // Value may contain HTML spans like "<span class='n'>name</span>" — strip them
            let translation = word_translations.as_ref().and_then(|wt| {
                let k = format!("{}:{}:{}", word.surah, word.ayah, word.word_pos);
                wt.get(&k).and_then(|v| v.as_str()).map(strip_html_tags)
            });

            // Look up QUL transliteration: key format "surah:ayah" (ayah-level, not word)
            // We store the full ayah transliteration on word_pos=1 only to avoid duplication
            let transliteration = if word.word_pos == 1 {
                transliterations.as_ref().and_then(|tl| {
                    let k = format!("{}:{}", word.surah, word.ayah);
                    tl.get(&k).and_then(|v| v.as_str()).map(|s| s.to_string())
                })
            } else {
                None
            };

            let segments_json = serde_json::to_string(&word.segments)?;

            let p_rid = b.param(rid("quran_word", &key));
            let p_surah = b.param(word.surah);
            let p_ayah = b.param(word.ayah);
            let p_word_pos = b.param(word.word_pos);
            let p_text_ar = b.param(word.text_ar.clone());
            let p_text_ar_simple = b.param(text_ar_simple);
            let p_translation = b.param(translation);
            let p_transliteration = b.param(transliteration);
            let p_pos = b.param(word.pos.clone());
            let p_root = b.param(word.root.clone());
            let p_lemma = b.param(word.lemma.clone());
            let p_features = b.param(word.features.clone());
            let p_segments = b.param(segments_json);

            b.push(format!(
                "CREATE {p_rid} CONTENT {{ \
                 surah_number: {p_surah}, ayah_number: {p_ayah}, word_position: {p_word_pos}, \
                 text_ar: {p_text_ar}, text_ar_simple: {p_text_ar_simple}, \
                 translation: {p_translation}, transliteration: {p_transliteration}, \
                 pos: {p_pos}, root: {p_root}, lemma: {p_lemma}, \
                 features: {p_features}, segments: {p_segments} }}"
            ));
        }
        b.commit(db).await?;
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("done");
    println!("   ✓ {} words ingested", total);

    Ok(())
}

/// Build a lemma-based Arabic text field for each ayah.
///
/// Aggregates the lemma of every word in the ayah (from quran_word),
/// strips diacritics for consistent BM25 matching, and stores the
/// result in `ayah.text_ar_lemma`. This enables Arabic search to match
/// inflected forms (e.g., searching عيد matches the verse containing عيدا).
pub async fn build_ayah_lemma_text(db: &Surreal<Db>) -> Result<()> {
    use surrealdb::types::SurrealValue;

    #[derive(Debug, SurrealValue)]
    struct WordLemma {
        surah_number: i64,
        ayah_number: i64,
        word_position: i64,
        lemma: Option<String>,
    }

    println!("📝 Building ayah lemma text from morphology data...");

    let mut res = db
        .query("SELECT surah_number, ayah_number, word_position, lemma FROM quran_word ORDER BY surah_number, ayah_number, word_position")
        .await?;
    let words: Vec<WordLemma> = res.take(0)?;

    // Group lemmas by (surah, ayah)
    let mut ayah_lemmas: HashMap<(i64, i64), Vec<String>> = HashMap::new();
    for w in &words {
        if let Some(ref lemma) = w.lemma {
            let normalized = strip_arabic_diacritics(lemma);
            if !normalized.is_empty() {
                ayah_lemmas
                    .entry((w.surah_number, w.ayah_number))
                    .or_default()
                    .push(normalized);
            }
        }
    }

    let total = ayah_lemmas.len();
    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.cyan/blue} {pos}/{len} ayahs ({eta})")
            .unwrap(),
    );

    let lemma_entries: Vec<((i64, i64), Vec<String>)> = ayah_lemmas.into_iter().collect();
    let batch_size = batch_size_from_env(200);
    for chunk in lemma_entries.chunks(batch_size) {
        let mut b = Batch::new();
        for ((surah, ayah), lemmas) in chunk {
            let lemma_text = lemmas.join(" ");
            let key = format!("{}_{}", surah, ayah);
            let p_rid = b.param(rid("ayah", &key));
            let p_lemma_text = b.param(lemma_text);
            b.push(format!("UPDATE {p_rid} SET text_ar_lemma = {p_lemma_text}"));
        }
        b.commit(db).await?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    println!("   ✓ {} ayahs updated with lemma text", total);
    Ok(())
}

/// Load a QUL JSON file as a serde_json::Value (HashMap-like).
fn load_qul_json(path: &str) -> Option<serde_json::Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}
