//! Hadith ingestion from pre-processed SemanticHadith KG V2 JSON.
//!
//! Source: <https://github.com/A-Kamran/SemanticHadith-V2>
//!
//! This module ingests hadiths with fully identified narrator chains,
//! narrator bios, topics, similarity links, and chapter data — all from
//! a single JSON file extracted from the SemanticHadith KG TTL.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use crate::db::Db;
use crate::ingest::sanadset::make_progress;

// ── JSON schema (matches scripts/build_semantic_data.py output) ─────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticData {
    pub narrators: HashMap<String, Narrator>,
    pub hadiths: HashMap<String, Hadith>,
    pub book_names: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Narrator {
    pub popular_name: Option<String>,
    pub name: Option<String>,
    pub teknonym: Option<String>,
    pub generation: Option<String>,
    pub lineage: Option<String>,
    pub residence: Option<String>,
    pub death_year: Option<String>,
    pub birth_year: Option<String>,
    pub title: Option<String>,
    pub office: Option<String>,
    pub attribute: Option<String>,
    pub narrator_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hadith {
    pub book: String,
    pub book_name: String,
    pub ref_no: Option<i64>,
    pub chain: Vec<String>,
    #[serde(rename = "type")]
    pub hadith_type: Option<String>,
    pub chapter: Option<String>,
    pub chapter_preface: Option<String>,
    pub topics: Option<Vec<String>>,
    pub mentions: Option<Vec<String>>,
    pub quran_verses: Option<Vec<String>>,
    pub similar: Option<Vec<String>>,
    pub strongly_similar: Option<Vec<String>>,
    pub see_also: Option<Vec<String>>,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
}

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Slug for a book: e.g. "book_sb"
fn book_slug(prefix: &str) -> String {
    format!("book_{}", prefix.to_lowercase())
}

/// Slug for a hadith: e.g. "sb_1"
fn hadith_slug(prefix: &str, num: i64) -> String {
    format!("{}_{}", prefix.to_lowercase(), num)
}

/// Slug for a narrator: e.g. "hn_04698"
fn narrator_slug(hn_id: &str) -> String {
    // HN04698 → hn_04698
    format!("hn_{}", hn_id.strip_prefix("HN").unwrap_or(hn_id))
}

/// Parse a Hijri year string like "57" or "116" to i64.
fn parse_year(s: &str) -> Option<i64> {
    s.trim().parse().ok()
}

/// Map SemanticHadith type names to proper mustalah al-hadith terminology.
fn map_hadith_type(raw: &str) -> &str {
    match raw {
        "elevated" => "مرفوع",
        "stopped" => "موقوف",
        "sacred" => "قدسي",
        "severed" => "مقطوع",
        other => other,
    }
}

/// Fix English hadith text: normalize honorifics to use ﷺ symbol.
fn fix_english_text(text: &str) -> String {
    text.replace("Allah's Apostle", "Allah's Messenger (ﷺ)")
        .replace("Allah's Messenger", "Allah's Messenger (ﷺ)")
        // Avoid double-applying
        .replace("(ﷺ) (ﷺ)", "(ﷺ)")
        .replace("PBUH", "ﷺ")
        .replace("pbuh", "ﷺ")
        .replace("peace be upon him", "ﷺ")
        .replace("Peace be upon him", "ﷺ")
}

/// Convert a SemanticHadith hadith ID like "SB-HD0001" to our slug format "sb_1".
fn parse_hadith_ref(sem_id: &str) -> Option<String> {
    // SB-HD0001 → sb_1
    let parts: Vec<&str> = sem_id.split("-HD").collect();
    if parts.len() == 2 {
        let prefix = parts[0].to_lowercase();
        let num: i64 = parts[1].parse().ok()?;
        Some(format!("{}_{}", prefix, num))
    } else {
        None
    }
}

/// Main ingestion from SemanticHadith JSON.
pub async fn ingest(
    db: &Surreal<Db>,
    json_path: &str,
    limit_per_book: Option<usize>,
) -> Result<()> {
    let path = Path::new(json_path);
    if !path.exists() {
        anyhow::bail!(
            "SemanticHadith JSON not found: {json_path}\n\
             Run: make semantic-download && make semantic-extract"
        );
    }

    println!("📖 Loading SemanticHadith data from {json_path}...");
    let raw = std::fs::read_to_string(path)?;
    let data: SemanticData = serde_json::from_str(&raw)?;
    println!(
        "   {} narrators, {} hadiths across {} books",
        data.narrators.len(),
        data.hadiths.len(),
        data.book_names.len(),
    );

    // ── Count hadiths per book (for progress bar + limit) ──────────────────
    // Sort first so limit applies deterministically to lowest-numbered hadiths

    let mut all_hadiths: Vec<(&String, &Hadith)> = data.hadiths.iter().collect();
    all_hadiths.sort_by(|(a_id, a), (b_id, b)| {
        a.book
            .cmp(&b.book)
            .then(a.ref_no.cmp(&b.ref_no))
            .then(a_id.cmp(b_id))
    });

    let mut book_counts: HashMap<String, usize> = HashMap::new();
    let mut total_expected = 0;
    for (_hid, h) in &all_hadiths {
        let count = book_counts.entry(h.book.clone()).or_insert(0);
        if let Some(limit) = limit_per_book {
            if *count >= limit {
                continue;
            }
        }
        *count += 1;
        total_expected += 1;
    }

    // ── Create books ───────────────────────────────────────────────────────

    // Fixed book order for deterministic book_id assignment
    const BOOK_ORDER: &[(&str, &str)] = &[
        ("SB", "صحيح البخاري"),
        ("SM", "صحيح مسلم"),
        ("SD", "سنن أبي داود"),
        ("JT", "جامع الترمذي"),
        ("SN", "سنن النسائى الصغرى"),
        ("IM", "سنن ابن ماجه"),
    ];

    let mut books_created: HashSet<String> = HashSet::new();
    let mut book_num_map: HashMap<String, i64> = HashMap::new();
    for (i, (prefix, arabic_name)) in BOOK_ORDER.iter().enumerate() {
        let bslug = book_slug(prefix);
        let book_num = (i + 1) as i64;
        db.query(
            "CREATE $rid CONTENT { book_number: $book_number, name_en: $name, name_ar: $name }",
        )
        .bind(("rid", rid("book", &bslug)))
        .bind(("book_number", book_num))
        .bind(("name", arabic_name.to_string()))
        .await?;
        book_num_map.insert(prefix.to_string(), book_num);
        books_created.insert(prefix.to_string());
    }

    // ── Create narrators (all at once, with bio) ───────────────────────────

    println!(
        "👤 Creating {} narrators with bio data...",
        data.narrators.len()
    );
    let nar_pb = make_progress(data.narrators.len() as u64, "narrators");

    // Sort narrators by ID for deterministic insertion order
    let mut sorted_narrators: Vec<(&String, &Narrator)> = data.narrators.iter().collect();
    sorted_narrators.sort_by_key(|(id, _)| *id);

    for (hn_id, nar) in &sorted_narrators {
        let nslug = narrator_slug(hn_id);
        let display_name = nar
            .popular_name
            .as_deref()
            .or(nar.name.as_deref())
            .unwrap_or(hn_id);

        let kunya = nar.teknonym.as_deref();
        let generation = nar.generation.as_deref();
        let death_year = nar.death_year.as_deref().and_then(parse_year);
        let birth_year = nar.birth_year.as_deref().and_then(parse_year);

        // Build tags from lineage + title + attribute
        let mut tags: Vec<String> = Vec::new();
        if let Some(ref lineage) = nar.lineage {
            tags.push(lineage.clone());
        }
        if let Some(ref title) = nar.title {
            tags.push(title.clone());
        }
        if let Some(ref attr) = nar.attribute {
            tags.push(attr.clone());
        }

        // Build locations from residence
        let locations: Vec<String> = nar
            .residence
            .as_deref()
            .filter(|r| !r.starts_with("http"))
            .map(|r| vec![r.to_string()])
            .unwrap_or_default();

        // Build aliases
        let mut aliases: Vec<String> = Vec::new();
        if let (Some(pop), Some(full)) = (&nar.popular_name, &nar.name) {
            if pop != full {
                aliases.push(full.clone());
            }
        }

        db.query(
            "CREATE $rid CONTENT { \
                name_en: $name, \
                name_ar: $name, \
                search_name: $slug, \
                gender: NONE, \
                generation: $generation, \
                bio: NONE, \
                kunya: $kunya, \
                aliases: $aliases, \
                birth_year: $birth_year, \
                birth_calendar: $birth_cal, \
                death_year: $death_year, \
                death_calendar: $death_cal, \
                locations: $locations, \
                tags: $tags \
            }",
        )
        .bind(("rid", rid("narrator", &nslug)))
        .bind(("name", display_name.to_string()))
        .bind(("slug", nslug.clone()))
        .bind(("generation", generation.map(|s| s.to_string())))
        .bind(("kunya", kunya.map(|s| s.to_string())))
        .bind(("aliases", aliases))
        .bind(("birth_year", birth_year))
        .bind(("birth_cal", birth_year.map(|_| "hijri".to_string())))
        .bind(("death_year", death_year))
        .bind(("death_cal", death_year.map(|_| "hijri".to_string())))
        .bind(("locations", locations))
        .bind(("tags", tags))
        .await
        .ok();

        nar_pb.inc(1);
    }
    nar_pb.finish_with_message("done");

    // ── Create hadiths + edges ─────────────────────────────────────────────

    println!("📜 Ingesting {total_expected} hadiths...");
    let pb = make_progress(total_expected as u64, "hadiths ingested");

    let mut ingested_counts: HashMap<String, usize> = HashMap::new();
    let mut hadith_count = 0;
    let mut heard_from_count = 0;

    for (_hid, hadith) in &all_hadiths {
        // Check per-book limit
        let count = ingested_counts.entry(hadith.book.clone()).or_insert(0);
        if let Some(limit) = limit_per_book {
            if *count >= limit {
                continue;
            }
        }

        let ref_no = match hadith.ref_no {
            Some(n) if n > 0 => n,
            _ => continue,
        };

        let bslug = book_slug(&hadith.book);
        let hslug = hadith_slug(&hadith.book, ref_no);

        // Narrator text = last narrator in chain (the sahabi/companion)
        let narrator_text = hadith.chain.last().and_then(|hn| {
            data.narrators.get(hn.as_str()).and_then(|n| {
                n.popular_name
                    .as_deref()
                    .or(n.name.as_deref())
                    .map(|s| s.to_string())
            })
        });

        let text_ar = hadith.text_ar.as_deref().unwrap_or("");
        let text_en = hadith.text_en.as_deref().map(fix_english_text);

        // Create hadith record
        db.query(
            "CREATE $rid CONTENT { \
                hadith_number: $hadith_number, \
                book_id: $book_id, \
                chapter_id: 0, \
                text_ar: $text_ar, \
                text_en: $text_en, \
                matn: NONE, \
                narrator_text: $narrator_text, \
                grade: NONE, \
                book_name: $book_name, \
                embedding: NONE, \
                hadith_type: $hadith_type, \
                topics: $topics, \
                quran_verses: $quran_verses, \
                chapter_name: $chapter_name \
            }",
        )
        .bind(("rid", rid("hadith", &hslug)))
        .bind(("hadith_number", ref_no))
        .bind(("book_id", *book_num_map.get(&hadith.book).unwrap_or(&0)))
        .bind(("text_ar", text_ar.to_string()))
        .bind(("text_en", text_en))
        .bind(("narrator_text", narrator_text))
        .bind(("book_name", hadith.book_name.clone()))
        .bind((
            "hadith_type",
            hadith
                .hadith_type
                .as_deref()
                .map(map_hadith_type)
                .map(String::from),
        ))
        .bind(("topics", hadith.topics.clone().unwrap_or_default()))
        .bind((
            "quran_verses",
            hadith.quran_verses.clone().unwrap_or_default(),
        ))
        .bind(("chapter_name", hadith.chapter.clone()))
        .await?;

        // belongs_to edge
        db.query("RELATE $from->belongs_to->$to")
            .bind(("from", rid("hadith", &hslug)))
            .bind(("to", rid("book", &bslug)))
            .await?;

        // narrates edges (narrator → hadith with chain position)
        for (pos, hn_id) in hadith.chain.iter().enumerate() {
            let nslug = narrator_slug(hn_id);
            db.query("RELATE $from->narrates->$to SET chain_position = $pos")
                .bind(("from", rid("narrator", &nslug)))
                .bind(("to", rid("hadith", &hslug)))
                .bind(("pos", pos as i64))
                .await
                .ok();
        }

        // heard_from edges: chain[i] heard from chain[i+1]
        for i in 0..hadith.chain.len().saturating_sub(1) {
            let s1 = narrator_slug(&hadith.chain[i]);
            let s2 = narrator_slug(&hadith.chain[i + 1]);
            if s1 == s2 {
                continue;
            }
            db.query("RELATE $from->heard_from->$to SET hadith_ref = $href, chain_position = $pos")
                .bind(("from", rid("narrator", &s1)))
                .bind(("to", rid("narrator", &s2)))
                .bind(("href", rid("hadith", &hslug)))
                .bind(("pos", i as i64))
                .await
                .ok();
            heard_from_count += 1;
        }

        // similar_to edges
        for sim_hid in hadith.similar.as_deref().unwrap_or_default() {
            if let Some(sim_ref) = parse_hadith_ref(sim_hid) {
                db.query("RELATE $from->similar_to->$to SET strength = 'similar'")
                    .bind(("from", rid("hadith", &hslug)))
                    .bind(("to", rid("hadith", &sim_ref)))
                    .await
                    .ok();
            }
        }
        for sim_hid in hadith.strongly_similar.as_deref().unwrap_or_default() {
            if let Some(sim_ref) = parse_hadith_ref(sim_hid) {
                db.query("RELATE $from->similar_to->$to SET strength = 'strong'")
                    .bind(("from", rid("hadith", &hslug)))
                    .bind(("to", rid("hadith", &sim_ref)))
                    .await
                    .ok();
            }
        }

        *count += 1;
        hadith_count += 1;
        pb.inc(1);
    }

    pb.finish_with_message("done");
    println!(
        "   ✓ {} hadiths, {} books, {} narrators, {} chain edges",
        hadith_count,
        books_created.len(),
        data.narrators.len(),
        heard_from_count,
    );

    // Generate embeddings
    println!("🔢 Generating embeddings...");
    crate::embed::embed_all_hadiths(db).await?;

    // Backfill narrator hadith counts
    println!("📊 Computing narrator hadith counts...");
    crate::db::backfill_narrator_hadith_counts(db).await?;

    Ok(())
}
