use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::ingest::sanadset::make_progress;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

// ── Quran.com API types ──

const API_BASE: &str = "https://quran.com/api/proxy/content/api/qdc";

#[derive(Debug, Deserialize)]
struct ByAyahResponse {
    #[serde(default)]
    hadith_references: Vec<HadithRef>,
}

#[derive(Debug, Deserialize, serde::Serialize, Clone)]
struct HadithRef {
    collection: String,
    hadith_number: String,
    #[serde(default)]
    english_urn: Option<i64>,
    #[serde(default)]
    arabic_urn: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct HadithsPageResponse {
    #[serde(default)]
    hadiths: Vec<HadithFullEntry>,
    #[serde(default)]
    has_more: bool,
}

#[derive(Debug, Deserialize)]
struct HadithFullEntry {
    collection: String,
    #[serde(rename = "hadithNumber")]
    hadith_number: String,
    #[serde(default)]
    hadith: Vec<HadithLangEntry>,
}

#[derive(Debug, Deserialize)]
struct HadithLangEntry {
    lang: String,
    body: String,
}

// ── Collection name mapping ──

fn collection_to_arabic(collection: &str) -> Option<&'static str> {
    match collection {
        "bukhari" => Some("صحيح البخاري"),
        "muslim" => Some("صحيح مسلم"),
        "abudawud" => Some("سنن أبي داود"),
        "nasai" => Some("سنن النسائى الصغرى"),
        "tirmidhi" => Some("جامع الترمذي"),
        "ibnmajah" => Some("سنن ابن ماجه"),
        _ => None,
    }
}

/// Parse the numeric portion of a hadith number (e.g., "3017 a" → 3017, "774b" → 774).
fn parse_hadith_num(s: &str) -> Option<i64> {
    let numeric: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    numeric.parse().ok()
}

/// Strip HTML tags from a string.
fn strip_html(s: &str) -> String {
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
    out
}

fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .default_headers({
            let mut h = reqwest::header::HeaderMap::new();
            h.insert("Referer", "https://quran.com/".parse().unwrap());
            h
        })
        .build()?)
}

// ── Surah metadata (ayah counts) ──

fn surah_ayah_counts() -> Vec<i64> {
    vec![
        7, 286, 200, 176, 120, 165, 206, 75, 129, 109, 123, 111, 43, 52, 99, 128, 111, 110, 98,
        135, 112, 78, 118, 64, 77, 227, 93, 88, 69, 60, 34, 30, 73, 54, 45, 83, 182, 88, 75, 85,
        54, 53, 89, 59, 37, 35, 38, 29, 18, 45, 60, 49, 62, 55, 78, 96, 29, 22, 24, 13, 14, 11, 11,
        18, 12, 12, 30, 52, 52, 44, 28, 28, 20, 56, 40, 31, 50, 40, 46, 42, 29, 19, 36, 25, 22, 17,
        19, 26, 30, 20, 15, 21, 11, 8, 8, 19, 5, 8, 8, 11, 11, 8, 3, 9, 5, 4, 7, 3, 6, 3, 5, 4, 5,
        6,
    ]
}

// ── Cache helpers ──

fn cache_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("data/quran_hadith_refs_cache")
}

fn ensure_cache_dir() -> Result<()> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("refs"))?;
    Ok(())
}

// ── Main ingestion function ──

pub async fn ingest_hadith_refs(db: &Surreal<Db>) -> Result<()> {
    ensure_cache_dir()?;
    let client = build_client()?;
    let ayah_counts = surah_ayah_counts();

    // Step 1: Discover which verses have hadiths
    println!("🔍 Step 1: Discovering verses with hadith references...");
    let verses_with_hadiths = discover_verses(&client, &ayah_counts).await?;
    println!(
        "   ✓ Found {} verses with hadith references",
        verses_with_hadiths.len()
    );

    // Step 2: Fetch reference metadata for each verse
    println!("📥 Step 2: Fetching reference metadata...");
    let all_refs = fetch_all_refs(&client, &verses_with_hadiths).await?;
    let total_refs: usize = all_refs.values().map(|v| v.len()).sum();
    println!(
        "   ✓ Fetched {} references across {} verses",
        total_refs,
        all_refs.len()
    );

    // Step 3: Resolve to local hadith records & create edges
    println!("🔗 Step 3: Resolving references to local hadith records...");
    let (matched, unmatched) = resolve_and_create_edges(db, &all_refs).await?;
    println!(
        "   ✓ Created {} edges, {} unmatched",
        matched,
        unmatched.len()
    );

    // Step 4: Fallback text matching for unmatched refs
    if !unmatched.is_empty() {
        println!(
            "🔎 Step 4: Fallback text matching for {} unmatched refs...",
            unmatched.len()
        );
        let fallback_matched = fallback_text_match(db, &client, &unmatched).await?;
        println!(
            "   ✓ Resolved {} additional refs via text matching",
            fallback_matched
        );

        let still_unresolved = unmatched.len() - fallback_matched;
        if still_unresolved > 0 {
            println!("   ⚠ {} refs could not be resolved", still_unresolved);
        }
    } else {
        println!("🔎 Step 4: No unmatched refs — skipping fallback.");
    }

    // Step 5: Summary
    let mut res = db
        .query("SELECT count() FROM references_hadith GROUP ALL")
        .await?;
    let total_edges: Option<CountResult> = res.take(0).unwrap_or(None);
    let edge_count = total_edges.map(|c| c.count).unwrap_or(0);
    println!("\n✅ Done! {} total ayah→hadith edges created.", edge_count);

    Ok(())
}

#[derive(Debug, SurrealValue)]
struct CountResult {
    count: i64,
}

// ── Step 1: Discover verses with hadiths ──

async fn discover_verses(
    client: &reqwest::Client,
    ayah_counts: &[i64],
) -> Result<Vec<(i64, i64, i64)>> {
    // Returns: Vec<(surah, ayah, count)>
    let cache_path = cache_dir().join("counts.json");

    // Try cache
    if cache_path.exists() {
        let data = std::fs::read_to_string(&cache_path)?;
        let cached: Vec<(i64, i64, i64)> = serde_json::from_str(&data)?;
        if !cached.is_empty() {
            println!("   (using cached counts)");
            return Ok(cached);
        }
    }

    let mut result = Vec::new();
    let pb = make_progress(114, "surahs scanned");

    for (i, &count) in ayah_counts.iter().enumerate() {
        let surah = (i + 1) as i64;
        let url = format!(
            "{API_BASE}/hadith_references/count_within_range?from={surah}:1&to={surah}:{count}&language=en"
        );

        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    if let Ok(counts) = serde_json::from_str::<HashMap<String, i64>>(&text) {
                        for (verse_key, cnt) in counts {
                            if let Some((s, a)) = parse_verse_key(&verse_key) {
                                result.push((s, a, cnt));
                            }
                        }
                    }
                }
            }
            Ok(resp) => {
                tracing::warn!("Count request for surah {surah} returned {}", resp.status());
            }
            Err(e) => {
                tracing::warn!("Count request for surah {surah} failed: {e}");
            }
        }

        pb.inc(1);
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    pb.finish_with_message("done");

    // Cache results
    let json = serde_json::to_string(&result)?;
    std::fs::write(&cache_path, json)?;

    Ok(result)
}

fn parse_verse_key(key: &str) -> Option<(i64, i64)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() == 2 {
        let s = parts[0].parse().ok()?;
        let a = parts[1].parse().ok()?;
        Some((s, a))
    } else {
        None
    }
}

// ── Step 2: Fetch references for each verse ──

async fn fetch_all_refs(
    client: &reqwest::Client,
    verses: &[(i64, i64, i64)],
) -> Result<HashMap<(i64, i64), Vec<HadithRef>>> {
    let mut all_refs: HashMap<(i64, i64), Vec<HadithRef>> = HashMap::new();
    let pb = make_progress(verses.len() as u64, "verse references fetched");

    for &(surah, ayah, _count) in verses {
        let cache_path = cache_dir().join(format!("refs/{}_{}.json", surah, ayah));

        let refs = if cache_path.exists() {
            let data = std::fs::read_to_string(&cache_path)?;
            serde_json::from_str::<Vec<HadithRef>>(&data).unwrap_or_default()
        } else {
            let url = format!("{API_BASE}/hadith_references/by_ayah/{surah}:{ayah}?language=en");

            let refs = match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(body) = resp.json::<ByAyahResponse>().await {
                        body.hadith_references
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            };

            // Cache
            let json = serde_json::to_string(&refs)?;
            std::fs::write(&cache_path, json)?;

            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            refs
        };

        if !refs.is_empty() {
            all_refs.insert((surah, ayah), refs);
        }
        pb.inc(1);
    }

    pb.finish_with_message("done");
    Ok(all_refs)
}

// ── Step 3: Resolve references and create edges ──

struct UnmatchedRef {
    surah: i64,
    ayah: i64,
    collection: String,
    hadith_number: String,
}

async fn resolve_and_create_edges(
    db: &Surreal<Db>,
    all_refs: &HashMap<(i64, i64), Vec<HadithRef>>,
) -> Result<(usize, Vec<UnmatchedRef>)> {
    let total_refs: usize = all_refs.values().map(|v| v.len()).sum();
    let pb = make_progress(total_refs as u64, "references resolved");

    let mut matched_count = 0usize;
    let mut unmatched = Vec::new();

    for (&(surah, ayah), refs) in all_refs {
        let ayah_rid = rid("ayah", &format!("{surah}_{ayah}"));

        for href in refs {
            let arabic_book = match collection_to_arabic(&href.collection) {
                Some(name) => name,
                None => {
                    pb.inc(1);
                    continue;
                }
            };

            let num = match parse_hadith_num(&href.hadith_number) {
                Some(n) => n,
                None => {
                    pb.inc(1);
                    continue;
                }
            };

            // Query local DB for matching hadith
            let mut res = db
                .query("SELECT id FROM hadith WHERE book_name = $book AND hadith_number = $num LIMIT 1")
                .bind(("book", arabic_book.to_string()))
                .bind(("num", num))
                .await?;

            #[derive(Debug, SurrealValue)]
            struct IdOnly {
                id: Option<RecordId>,
            }

            let found: Vec<IdOnly> = res.take(0)?;

            if let Some(row) = found.first() {
                if let Some(ref hadith_id) = row.id {
                    // Create RELATE edge
                    db.query(
                        "RELATE $from->references_hadith->$to SET \
                         collection = $collection, \
                         hadith_number = $hadith_number, \
                         source = 'qurancom'",
                    )
                    .bind(("from", ayah_rid.clone()))
                    .bind(("to", hadith_id.clone()))
                    .bind(("collection", href.collection.clone()))
                    .bind(("hadith_number", href.hadith_number.clone()))
                    .await?;

                    matched_count += 1;
                } else {
                    unmatched.push(UnmatchedRef {
                        surah,
                        ayah,
                        collection: href.collection.clone(),
                        hadith_number: href.hadith_number.clone(),
                    });
                }
            } else {
                unmatched.push(UnmatchedRef {
                    surah,
                    ayah,
                    collection: href.collection.clone(),
                    hadith_number: href.hadith_number.clone(),
                });
            }

            pb.inc(1);
        }
    }

    pb.finish_with_message("done");
    Ok((matched_count, unmatched))
}

// ── Step 4: Fallback text matching ──

async fn fallback_text_match(
    db: &Surreal<Db>,
    client: &reqwest::Client,
    unmatched: &[UnmatchedRef],
) -> Result<usize> {
    // Group unmatched by verse to minimize API calls
    let mut by_verse: HashMap<(i64, i64), Vec<&UnmatchedRef>> = HashMap::new();
    for u in unmatched {
        by_verse.entry((u.surah, u.ayah)).or_default().push(u);
    }

    let pb = make_progress(unmatched.len() as u64, "unmatched refs resolved");
    let mut resolved = 0usize;

    for (&(surah, ayah), refs) in &by_verse {
        // Fetch hadith texts from Quran.com (paginate from page 2)
        let mut hadith_texts: Vec<(String, String, String)> = Vec::new(); // (collection, number, arabic_text)
        let mut page = 2;

        loop {
            let url = format!(
                "{API_BASE}/hadith_references/by_ayah/{surah}:{ayah}/hadiths?language=en&page={page}&limit=4"
            );

            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(text) = resp.text().await {
                        // Parse with lenient JSON (control chars in hadith HTML)
                        if let Ok(data) = serde_json::from_str::<HadithsPageResponse>(&text) {
                            for h in &data.hadiths {
                                if let Some(ar) = h.hadith.iter().find(|l| l.lang == "ar") {
                                    let clean = strip_html(&ar.body);
                                    hadith_texts.push((
                                        h.collection.clone(),
                                        h.hadith_number.clone(),
                                        clean,
                                    ));
                                }
                            }
                            if !data.has_more {
                                break;
                            }
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                _ => break,
            }
            page += 1;
            if page > 20 {
                break; // Safety limit
            }
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }

        // For each unmatched ref, try to find a text match
        let ayah_rid = rid("ayah", &format!("{surah}_{ayah}"));

        for uref in refs {
            // Find the Arabic text for this specific unmatched ref
            let arabic_text = hadith_texts
                .iter()
                .find(|(c, n, _)| c == &uref.collection && n == &uref.hadith_number)
                .map(|(_, _, t)| t.as_str());

            if let Some(text) = arabic_text {
                // Extract a meaningful text fragment (skip short text)
                let fragment: String = text
                    .chars()
                    .skip(20) // Skip initial narrator chain words
                    .take(80)
                    .collect();

                if fragment.len() < 20 {
                    pb.inc(1);
                    continue;
                }

                let arabic_book = match collection_to_arabic(&uref.collection) {
                    Some(name) => name,
                    None => {
                        pb.inc(1);
                        continue;
                    }
                };

                // Search local DB by text fragment
                let mut res = db
                    .query(
                        "SELECT id FROM hadith WHERE book_name = $book AND text_ar CONTAINS $fragment LIMIT 1",
                    )
                    .bind(("book", arabic_book.to_string()))
                    .bind(("fragment", fragment))
                    .await?;

                #[derive(Debug, SurrealValue)]
                struct IdOnly {
                    id: Option<RecordId>,
                }

                let found: Vec<IdOnly> = res.take(0)?;

                if let Some(row) = found.first() {
                    if let Some(ref hadith_id) = row.id {
                        db.query(
                            "RELATE $from->references_hadith->$to SET \
                             collection = $collection, \
                             hadith_number = $hadith_number, \
                             source = 'qurancom_text_match'",
                        )
                        .bind(("from", ayah_rid.clone()))
                        .bind(("to", hadith_id.clone()))
                        .bind(("collection", uref.collection.clone()))
                        .bind(("hadith_number", uref.hadith_number.clone()))
                        .await?;

                        resolved += 1;
                    }
                } else {
                    tracing::warn!(
                        "Could not resolve: {}:{} → {} #{}",
                        surah,
                        ayah,
                        uref.collection,
                        uref.hadith_number
                    );
                }
            } else {
                tracing::warn!(
                    "No Arabic text available for fallback: {}:{} → {} #{}",
                    surah,
                    ayah,
                    uref.collection,
                    uref.hadith_number
                );
            }

            pb.inc(1);
        }

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    pb.finish_with_message("done");
    Ok(resolved)
}

// ── Query functions (used by API handlers) ──

/// Get hadiths linked to an ayah via references_hadith edges.
pub async fn get_curated_hadiths(
    db: &Surreal<Db>,
    surah: i64,
    ayah: i64,
) -> Result<Vec<crate::models::Hadith>> {
    let ayah_key = format!("{surah}_{ayah}");

    // SurrealDB graph traversal: record->edge->target returns array of target records
    // We query the edge table directly and fetch the linked hadith records
    let mut res = db
        .query("SELECT out.* FROM references_hadith WHERE in = $ayah_id")
        .bind(("ayah_id", rid("ayah", &ayah_key)))
        .await?;

    #[derive(Debug, SurrealValue)]
    struct OutRow {
        out: Option<crate::models::Hadith>,
    }

    let rows: Vec<OutRow> = res.take(0)?;
    let hadiths: Vec<crate::models::Hadith> = rows.into_iter().filter_map(|r| r.out).collect();

    Ok(hadiths)
}

/// Get hadith reference counts per ayah in a surah.
pub async fn get_hadith_counts(db: &Surreal<Db>, surah: i64) -> Result<HashMap<i64, i64>> {
    // Query the edge table directly — group by source ayah to get counts
    // The `in` field of references_hadith is the ayah record ID (e.g., ayah:5_3)
    let mut res = db
        .query(
            "SELECT in.ayah_number AS ayah_number, count() AS count \
             FROM references_hadith \
             WHERE in.surah_number = $surah \
             GROUP BY ayah_number",
        )
        .bind(("surah", surah))
        .await?;

    #[derive(Debug, SurrealValue)]
    struct AyahCount {
        ayah_number: Option<i64>,
        count: i64,
    }

    let rows: Vec<AyahCount> = res.take(0)?;
    let mut counts = HashMap::new();
    for row in rows {
        if let Some(ayah_num) = row.ayah_number {
            counts.insert(ayah_num, row.count);
        }
    }

    Ok(counts)
}

/// Find semantically related hadiths for an ayah using embedding similarity.
pub async fn find_semantic_hadiths(
    db: &Surreal<Db>,
    surah: i64,
    ayah: i64,
    limit: usize,
) -> Result<Vec<crate::models::HadithSearchResult>> {
    let ayah_key = format!("{surah}_{ayah}");

    // First get the ayah's embedding
    #[derive(Debug, SurrealValue)]
    struct EmbeddingRow {
        embedding: Option<Vec<f32>>,
    }

    let mut res = db
        .query("SELECT embedding FROM $ayah_id")
        .bind(("ayah_id", rid("ayah", &ayah_key)))
        .await?;
    let rows: Vec<EmbeddingRow> = res.take(0)?;

    let embedding = match rows.first().and_then(|r| r.embedding.as_ref()) {
        Some(e) => e.clone(),
        None => return Ok(Vec::new()),
    };

    // HNSW similarity search against hadith embeddings
    let sql = format!(
        "SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score FROM hadith \
         WHERE embedding <|{limit},40|> $query_vec \
         ORDER BY score DESC"
    );

    let mut res = db.query(&sql).bind(("query_vec", embedding)).await?;
    let results: Vec<crate::models::HadithSearchResult> = res.take(0)?;

    Ok(results)
}
