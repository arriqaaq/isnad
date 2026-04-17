//! Hadith ingestion from pre-processed SemanticHadith KG V2 JSON.
//!
//! Source: <https://github.com/A-Kamran/SemanticHadith-V2>
//!
//! This module ingests hadiths with fully identified narrator chains,
//! narrator bios, topics, similarity links, and chapter data — all from
//! a single JSON file extracted from the SemanticHadith KG TTL.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use anyhow::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use crate::db::Db;
use crate::ingest::batch::{Batch, batch_size_from_env};
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
    // NOTE: ibn_hajar_rank and reliability_grade exist in source JSON but are
    // intentionally NOT ingested — data quality is unreliable (see NOTES.md).
    // Narrator grading should come from Tahdhib al-Tahdhib via Turath instead.
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

/// Parse a narrator's generation string ("1", "10", "6?") to an integer.
fn parse_gen_num(narrator: Option<&Narrator>) -> Option<u32> {
    narrator?
        .generation
        .as_deref()?
        .trim_end_matches('?')
        .parse()
        .ok()
}

/// Split a concatenated tahwil chain into separate sub-chains.
///
/// Tahwil hadiths have multiple parallel isnads concatenated into one flat array.
/// We detect boundaries where the generation jumps UP by ≥ 2 (chronologically
/// impossible in a single valid isnad).
fn split_tahwil_chain(chain: &[String], narrators: &HashMap<String, Narrator>) -> Vec<Vec<String>> {
    if chain.is_empty() {
        return vec![];
    }
    let mut sub_chains: Vec<Vec<String>> = Vec::new();
    let mut current = vec![chain[0].clone()];
    for i in 1..chain.len() {
        let g_prev = parse_gen_num(narrators.get(chain[i - 1].as_str()));
        let g_curr = parse_gen_num(narrators.get(chain[i].as_str()));
        if let (Some(gp), Some(gc)) = (g_prev, g_curr) {
            if gc >= gp + 2 {
                sub_chains.push(current);
                current = vec![chain[i].clone()];
                continue;
            }
        }
        current.push(chain[i].clone());
    }
    sub_chains.push(current);
    sub_chains
}

/// Check whether a heard_from edge should be skipped.
fn should_skip_heard_from(g1: Option<u32>, g2: Option<u32>) -> bool {
    if let (Some(g1), Some(g2)) = (g1, g2) {
        // Gen gap >= 2: tahwil / parallel narrator boundary
        if g2 >= g1 + 2 {
            return true;
        }
        // Sahabi terminus: gen 1 narrators never hear from later generations
        if g1 == 1 && g2 > 1 {
            return true;
        }
    }
    false
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
    embedder: &crate::embed::Embedder,
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

    // Fixed book order for deterministic collection_id assignment
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
            "CREATE $rid CONTENT { collection_id: $collection_id, name_en: $name, name_ar: $name }",
        )
        .bind(("rid", rid("collection", &bslug)))
        .bind(("collection_id", book_num))
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
                tags: $tags, \
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

    // ── Create hadiths + edges (batched for performance) ────────────────────

    println!("📜 Ingesting {total_expected} hadiths...");
    let pb = make_progress(total_expected as u64, "hadiths ingested");

    let mut ingested_counts: HashMap<String, usize> = HashMap::new();
    let mut hadith_count = 0;
    let mut heard_from_count = 0;

    // Collect hadiths to process, then batch them
    let mut pending: Vec<(&String, &Hadith)> = Vec::new();
    for (_hid, hadith) in &all_hadiths {
        let count = ingested_counts.entry(hadith.book.clone()).or_insert(0);
        if let Some(limit) = limit_per_book {
            if *count >= limit {
                continue;
            }
        }
        if hadith.ref_no.map_or(true, |n| n <= 0) {
            continue;
        }
        *count += 1;
        pending.push((_hid, hadith));
    }

    // Reset counts for actual processing
    ingested_counts.clear();

    // Pre-build set of hadith slugs we will actually insert (across all sub-chains
    // for tahwil hadiths). Used to filter out similar_to edges pointing to
    // hadiths outside this ingest scope (e.g. truncated by --limit), so the
    // batched transaction doesn't waste work on dangling refs.
    let mut pending_slugs: HashSet<String> = HashSet::with_capacity(pending.len());
    for (_hid, hadith) in &pending {
        let ref_no = hadith.ref_no.unwrap();
        let base = hadith_slug(&hadith.book, ref_no);
        let subs = split_tahwil_chain(&hadith.chain, &data.narrators);
        if subs.len() > 1 {
            for vi in 0..subs.len() {
                pending_slugs.insert(format!("{}_v{}", base, vi + 1));
            }
        } else {
            pending_slugs.insert(base);
        }
    }

    // Tunable: 1 fsync per ~200 hadiths instead of 1 per statement (~15× per hadith).
    // Override via env for experimentation: INGEST_BATCH=500 (shared across all ingest paths).
    let batch_size = batch_size_from_env(200);

    for batch in pending.chunks(batch_size) {
        let mut b = Batch::new();

        for (_hid, hadith) in batch {
            let ref_no = hadith.ref_no.unwrap();
            let bslug = book_slug(&hadith.book);
            let base_hslug = hadith_slug(&hadith.book, ref_no);
            let text_ar = hadith.text_ar.as_deref().unwrap_or("");
            let text_en = hadith.text_en.as_deref().map(fix_english_text);
            let sub_chains = split_tahwil_chain(&hadith.chain, &data.narrators);
            let is_tahwil = sub_chains.len() > 1;
            let collection_id = *book_num_map.get(&hadith.book).unwrap_or(&0);
            let book_name = hadith.book_name.clone();
            let hadith_type = hadith
                .hadith_type
                .as_deref()
                .map(map_hadith_type)
                .map(String::from);
            let topics = hadith.topics.clone().unwrap_or_default();
            let quran_verses = hadith.quran_verses.clone().unwrap_or_default();
            let chapter_name = hadith.chapter.clone();

            for (vi, sub_chain) in sub_chains.iter().enumerate() {
                let hslug = if is_tahwil {
                    format!("{}_v{}", base_hslug, vi + 1)
                } else {
                    base_hslug.clone()
                };

                let narrator_text = sub_chain.last().and_then(|hn| {
                    data.narrators.get(hn.as_str()).and_then(|n| {
                        n.popular_name
                            .as_deref()
                            .or(n.name.as_deref())
                            .map(|s| s.to_string())
                    })
                });

                let p_rid = b.param(rid("hadith", &hslug));
                let p_hn = b.param(ref_no);
                let p_cid = b.param(collection_id);
                let p_tar = b.param(text_ar.to_string());
                let p_ten = b.param(text_en.clone());
                let p_nt = b.param(narrator_text);
                let p_bn = b.param(book_name.clone());
                let p_ht = b.param(hadith_type.clone());
                let p_top = b.param(topics.clone());
                let p_qv = b.param(quran_verses.clone());
                let p_cn = b.param(chapter_name.clone());
                let p_coll = b.param(rid("collection", &bslug));

                let mut stmt = String::new();
                let _ = write!(
                    stmt,
                    "CREATE {p_rid} CONTENT {{ \
                        hadith_number: {p_hn}, \
                        collection_id: {p_cid}, \
                        chapter_id: 0, \
                        text_ar: {p_tar}, \
                        text_en: {p_ten}, \
                        matn: NONE, \
                        narrator_text: {p_nt}, \
                        grade: NONE, \
                        book_name: {p_bn}, \
                        embedding: NONE, \
                        hadith_type: {p_ht}, \
                        topics: {p_top}, \
                        quran_verses: {p_qv}, \
                        chapter_name: {p_cn} \
                    }}"
                );
                b.push(&stmt);
                b.push(format!("RELATE {p_rid}->belongs_to->{p_coll}"));

                // narrates: skip if narrator wasn't created (not in source data),
                // otherwise the strict typed RELATION would reject the whole batch.
                for (pos, hn_id) in sub_chain.iter().enumerate() {
                    if !data.narrators.contains_key(hn_id) {
                        continue;
                    }
                    let nslug = narrator_slug(hn_id);
                    let p_from = b.param(rid("narrator", &nslug));
                    let p_pos = b.param(pos as i64);
                    b.push(format!(
                        "RELATE {p_from}->narrates->{p_rid} SET chain_position = {p_pos}"
                    ));
                }

                // heard_from: same skip rule for missing narrators on either side.
                for i in 0..sub_chain.len().saturating_sub(1) {
                    let n1 = &sub_chain[i];
                    let n2 = &sub_chain[i + 1];
                    if !data.narrators.contains_key(n1) || !data.narrators.contains_key(n2) {
                        continue;
                    }
                    let s1 = narrator_slug(n1);
                    let s2 = narrator_slug(n2);
                    if s1 == s2 {
                        continue;
                    }
                    let g1 = parse_gen_num(data.narrators.get(n1.as_str()));
                    let g2 = parse_gen_num(data.narrators.get(n2.as_str()));
                    if should_skip_heard_from(g1, g2) {
                        continue;
                    }
                    let p_from = b.param(rid("narrator", &s1));
                    let p_to = b.param(rid("narrator", &s2));
                    let p_href = b.param(rid("hadith", &hslug));
                    let p_pos = b.param(i as i64);
                    b.push(format!(
                        "RELATE {p_from}->heard_from->{p_to} SET hadith_ref = {p_href}, chain_position = {p_pos}"
                    ));
                    heard_from_count += 1;
                }
            }

            // similar_to edges (anchored on the first sub-chain variant).
            let first_hslug = if is_tahwil {
                format!("{}_v1", base_hslug)
            } else {
                base_hslug.clone()
            };
            let p_first = b.param(rid("hadith", &first_hslug));

            let emit_similar = |b: &mut Batch, sim_ids: &[String], strength: &str| {
                for sim_hid in sim_ids {
                    let Some(sim_ref) = parse_hadith_ref(sim_hid) else {
                        continue;
                    };
                    // Skip dangling refs to hadiths not in this ingest scope.
                    // Tahwil targets are checked by base form (sim_ref has no _vN
                    // suffix); accept if any variant or the base exists.
                    let exists = pending_slugs.contains(&sim_ref)
                        || pending_slugs.contains(&format!("{sim_ref}_v1"));
                    if !exists {
                        continue;
                    }
                    let p_to = b.param(rid("hadith", &sim_ref));
                    let p_str = b.param(strength.to_string());
                    b.push(format!(
                        "RELATE {p_first}->similar_to->{p_to} SET strength = {p_str}"
                    ));
                }
            };
            emit_similar(
                &mut b,
                hadith.similar.as_deref().unwrap_or_default(),
                "similar",
            );
            emit_similar(
                &mut b,
                hadith.strongly_similar.as_deref().unwrap_or_default(),
                "strong",
            );

            hadith_count += 1;
        }

        b.commit(db).await?;
        pb.inc(batch.len() as u64);
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
    crate::embed::embed_all_hadiths(db, embedder).await?;

    // Backfill narrator hadith counts
    println!("📊 Computing narrator hadith counts...");
    crate::db::backfill_narrator_hadith_counts(db).await?;

    Ok(())
}
