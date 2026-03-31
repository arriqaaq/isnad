//! Narrator biographical data ingestion from the AR-Sanad dataset.
//!
//! Source: https://github.com/somaia02/Narrator-Disambiguation
//! Contains 18,298 narrators with Ibn Hajar reliability ranks from Taqrib al-Tahdhib.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::analysis::reliability;
use crate::db::Db;
use crate::ingest::sanadset::{make_progress, normalize_arabic};

const AR_SANAD_URL: &str =
    "https://raw.githubusercontent.com/somaia02/Narrator-Disambiguation/main/Narrators%20data.csv";

/// Statistics from narrator bio ingestion.
pub struct BioIngestionStats {
    pub total_ar_sanad: usize,
    pub total_with_rank: usize,
    pub matched_exact: usize,
    pub matched_substring: usize,
    pub skipped_ambiguous: usize,
    pub skipped_no_rank: usize,
    pub unmatched: usize,
    pub narrators_updated: usize,
    pub evidence_created: usize,
}

#[derive(Debug, SurrealValue)]
struct NarratorForMatch {
    id: Option<RecordId>,
    name_ar: Option<String>,
    search_name: Option<String>,
}

/// Map Ibn Hajar's Arabic rank phrases to our reliability rating system.
///
/// The AR-Sanad dataset contains 1,348 unique rank phrases from Taqrib al-Tahdhib.
/// We map the most common to our 6-level system. Unknown phrases map to None.
fn map_ibn_hajar_rank(rank: &str) -> Option<&'static str> {
    let trimmed = rank.trim().trim_end_matches('.');

    // Companions
    if trimmed.contains("صحابي") || trimmed.contains("صحابية") || trimmed.contains("الصحاب")
    {
        return Some("thiqah"); // Companions are trustworthy by default
    }

    // Thiqah variants (trustworthy)
    if trimmed.starts_with("ثقة")
        || trimmed == "ثبت حافظ"
        || trimmed.contains("متقن")
        || trimmed.contains("إمام")
        || trimmed.starts_with("الحافظ")
        || trimmed.contains("أحد الأئمة")
        || trimmed.contains("الفقيه")
    {
        return Some("thiqah");
    }

    // Saduq variants (truthful)
    if trimmed.starts_with("صدوق")
        || trimmed == "لا بأس به"
        || trimmed.starts_with("مقبول")
        || trimmed.starts_with("مقبولة")
    {
        return Some("saduq");
    }

    // Majhul variants (unknown)
    if trimmed.starts_with("مجهول")
        || trimmed.starts_with("مستور")
        || trimmed.contains("لا يعرف")
        || trimmed.contains("مبهم")
    {
        return Some("majhul");
    }

    // Daif variants (weak)
    if trimmed.starts_with("ضعيف")
        || trimmed.starts_with("لين")
        || trimmed.contains("تكلموا فيه")
        || trimmed.contains("فيه نظر")
    {
        return Some("daif");
    }

    // Matruk variants (abandoned)
    if trimmed.starts_with("متروك")
        || trimmed.contains("كذبه")
        || trimmed.contains("تكذيبه")
        || trimmed.contains("وضاع")
    {
        return Some("matruk");
    }

    None // Unrecognized rank
}

/// Parse a Hijri death year from AR-Sanad format like "345 هـ" or "231 هـ ، أو 232 هـ".
/// Returns the first year found.
fn parse_hijri_year(text: &str) -> Option<i64> {
    let trimmed = text.trim();
    if trimmed == "-" || trimmed.is_empty() {
        return None;
    }
    // Extract first number from the string
    let num_str: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string();
    num_str.parse().ok()
}

/// Download the AR-Sanad CSV if it doesn't exist at the given path.
async fn ensure_csv(csv_path: &str) -> Result<()> {
    if Path::new(csv_path).exists() {
        return Ok(());
    }

    let parent = Path::new(csv_path).parent().unwrap_or(Path::new("data"));
    std::fs::create_dir_all(parent)?;

    println!("Downloading AR-Sanad narrator dataset from GitHub...");
    let response = reqwest::get(AR_SANAD_URL).await?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed: HTTP {}", response.status());
    }
    let bytes = response.bytes().await?;
    std::fs::write(csv_path, &bytes)?;
    println!("   Downloaded {} KB -> {csv_path}", bytes.len() / 1024);
    Ok(())
}

/// Ingest narrator biographical data from the AR-Sanad dataset.
///
/// Matches AR-Sanad narrators to existing DB narrators via normalized Arabic name
/// fuzzy matching, then updates matched narrators with reliability ratings,
/// birth/death dates, and creates evidence records.
pub async fn ingest_narrator_bios(
    db: &Surreal<Db>,
    csv_path: &str,
    resolver: Option<&super::name_resolver::NameResolver>,
) -> Result<BioIngestionStats> {
    // Auto-download if missing
    ensure_csv(csv_path).await?;

    tracing::info!("Loading AR-Sanad narrator bios from {csv_path}...");

    // 1. Fetch all existing narrators
    let mut res = db
        .query("SELECT id, name_ar, search_name FROM narrator")
        .await?;
    let db_narrators: Vec<NarratorForMatch> = res.take(0)?;
    tracing::info!("Found {} narrators in database", db_narrators.len());

    // 2. Build matching indices
    let mut match_index: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut narrators_created_keys: HashSet<String> = HashSet::new();
    for n in &db_narrators {
        if let (Some(id), Some(name_ar)) = (&n.id, &n.name_ar) {
            let key = crate::models::record_id_key_string(id);
            narrators_created_keys.insert(key.clone());
            let normalized = normalize_arabic(name_ar);
            if !normalized.is_empty() {
                match_index
                    .entry(normalized)
                    .or_default()
                    .push((key, name_ar.clone()));
            }
        }
    }

    // 3. Parse AR-Sanad CSV
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)?;

    let headers = reader.headers()?.clone();
    let records: Vec<csv::StringRecord> = reader.records().filter_map(|r| r.ok()).collect();
    let total = records.len();
    tracing::info!("Loaded {total} AR-Sanad narrator records");

    // Column indices
    let col = |name: &str| -> Option<usize> { headers.iter().position(|h| h == name) };
    let i_name = col("name");
    let i_rank = col("Ibnhajar_rank");
    let i_shuhra = col("shuhra");
    let i_kunia = col("kunia");
    let i_death = col("death_year");
    let i_birth = col("birth_year");
    let i_city = col("living_city");
    let i_tabaqa = col("tabaqa");
    let i_laqab = col("laqab");

    let mut stats = BioIngestionStats {
        total_ar_sanad: total,
        total_with_rank: 0,
        matched_exact: 0,
        matched_substring: 0,
        skipped_ambiguous: 0,
        skipped_no_rank: 0,
        unmatched: 0,
        narrators_updated: 0,
        evidence_created: 0,
    };

    let pb = make_progress(total as u64, "enriching narrators");

    // 4. Match and update
    for record in &records {
        pb.inc(1);

        let get = |idx: Option<usize>| -> &str {
            idx.and_then(|i| record.get(i))
                .map(|s| s.trim())
                .filter(|s| *s != "-" && !s.is_empty())
                .unwrap_or("")
        };

        let full_name = get(i_name);
        let rank_text = get(i_rank);
        let shuhra = get(i_shuhra);
        let kunia = get(i_kunia);
        let death_year_raw = get(i_death);
        let birth_year_raw = get(i_birth);
        let city = get(i_city);
        let tabaqa = get(i_tabaqa);
        let laqab = get(i_laqab);

        // Skip if no rank
        if rank_text.is_empty() {
            stats.skipped_no_rank += 1;
            continue;
        }
        stats.total_with_rank += 1;

        let rating = match map_ibn_hajar_rank(rank_text) {
            Some(r) => r,
            None => {
                // Try simpler matching for edge cases
                if rank_text.contains("ثقة") {
                    "thiqah"
                } else if rank_text.contains("صدوق") || rank_text.contains("مقبول") {
                    "saduq"
                } else if rank_text.contains("ضعيف") || rank_text.contains("لين") {
                    "daif"
                } else if rank_text.contains("متروك") || rank_text.contains("كذب") {
                    "matruk"
                } else {
                    "majhul" // Default unknown for unrecognizable ranks
                }
            }
        };

        // Try to match against DB narrators.
        //
        // Strategy 1: If narrators were ingested with NameResolver, they have
        // keys like "arsanad_{id}". We can get the AR-Sanad ID directly from
        // the CSV's "id" column and check if that narrator exists in the DB.
        let ar_sanad_id_str = record
            .get(col("id").unwrap_or(16))
            .unwrap_or("")
            .trim()
            .to_string();

        let mut matched_key: Option<String> = None;

        // Direct match via arsanad_ key (when resolver was used during ingestion)
        if !ar_sanad_id_str.is_empty() && ar_sanad_id_str != "-" {
            let direct_key = format!("arsanad_{ar_sanad_id_str}");
            if narrators_created_keys.contains(&direct_key) {
                matched_key = Some(direct_key);
                stats.matched_exact += 1;
            }
        }

        // Strategy 2: Fallback to fuzzy name matching (for legacy ingestion without resolver)
        if matched_key.is_none() {
            let shuhra_norm = normalize_arabic(if !shuhra.is_empty() {
                shuhra
            } else {
                full_name
            });
            let full_norm = normalize_arabic(full_name);

            // Exact match on shuhra
            if let Some(matches) = match_index.get(&shuhra_norm) {
                if matches.len() == 1 {
                    matched_key = Some(matches[0].0.clone());
                    stats.matched_exact += 1;
                } else if matches.len() > 1 {
                    stats.skipped_ambiguous += 1;
                    continue;
                }
            }

            // Substring match: DB narrator name found within AR-Sanad full name
            if matched_key.is_none() && !full_norm.is_empty() {
                let mut substring_matches = Vec::new();
                for (norm_key, entries) in &match_index {
                    if norm_key.len() >= 3 && full_norm.contains(norm_key.as_str()) {
                        for (key, _) in entries {
                            substring_matches.push(key.clone());
                        }
                    }
                }
                if substring_matches.len() == 1 {
                    matched_key = Some(substring_matches[0].clone());
                    stats.matched_substring += 1;
                } else if substring_matches.len() > 1 {
                    stats.skipped_ambiguous += 1;
                    continue;
                }
            }
        }

        let narrator_key = match matched_key {
            Some(k) => k,
            None => {
                stats.unmatched += 1;
                continue;
            }
        };

        // Parse dates
        let death_year = parse_hijri_year(death_year_raw);
        let birth_year = parse_hijri_year(birth_year_raw);
        let prior = reliability::rating_prior(rating);

        // Build update object
        let mut update = serde_json::Map::new();
        update.insert("reliability_rating".to_string(), serde_json::json!(rating));
        update.insert("reliability_prior".to_string(), serde_json::json!(prior));
        update.insert(
            "reliability_source".to_string(),
            serde_json::json!("Taqrib al-Tahdhib (AR-Sanad)"),
        );

        if let Some(dy) = death_year {
            update.insert("death_year".to_string(), serde_json::json!(dy));
            update.insert("death_calendar".to_string(), serde_json::json!("hijri"));
        }
        if let Some(by) = birth_year {
            update.insert("birth_year".to_string(), serde_json::json!(by));
            update.insert("birth_calendar".to_string(), serde_json::json!("hijri"));
        }
        if !kunia.is_empty() {
            update.insert("kunya".to_string(), serde_json::json!(kunia));
        }
        if !city.is_empty() {
            update.insert("locations".to_string(), serde_json::json!(vec![city]));
        }
        if !tabaqa.is_empty() {
            update.insert("generation".to_string(), serde_json::json!(tabaqa));
        }

        // Build tags from laqab + rank
        let mut tags = Vec::new();
        if !laqab.is_empty() {
            tags.push(laqab.to_string());
        }
        tags.push(rating.to_string());
        update.insert("tags".to_string(), serde_json::json!(tags));

        // UPDATE narrator
        db.query("UPDATE $rid MERGE $data")
            .bind(("rid", RecordId::new("narrator", narrator_key.as_str())))
            .bind(("data", serde_json::Value::Object(update)))
            .await
            .ok();
        stats.narrators_updated += 1;

        // Create evidence record
        let ev_id = format!("taqrib_{}", narrator_key);
        reliability::add_reported_evidence(
            db,
            &narrator_key,
            &ev_id,
            rating,
            Some("Ibn Hajar al-Asqalani"),
            Some("Taqrib al-Tahdhib"),
            Some(rank_text),
            Some("AR-Sanad Dataset"),
        )
        .await
        .ok();
        stats.evidence_created += 1;
    }

    pb.finish_and_clear();

    tracing::info!(
        "AR-Sanad ingestion complete: {} total, {} with rank, {} exact matches, {} substring matches, \
         {} ambiguous (skipped), {} unmatched, {} updated, {} evidence records",
        stats.total_ar_sanad,
        stats.total_with_rank,
        stats.matched_exact,
        stats.matched_substring,
        stats.skipped_ambiguous,
        stats.unmatched,
        stats.narrators_updated,
        stats.evidence_created,
    );

    Ok(stats)
}
