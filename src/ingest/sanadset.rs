use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

pub fn make_progress(len: u64, prefix: &str) -> ProgressBar {
    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "   {{bar:40.green/black}} {{pos}}/{{len}} {prefix} ({{eta}})"
            ))
            .unwrap(),
    );
    pb
}

/// The six canonical hadith collections (Kutub al-Sittah).
const DEFAULT_BOOKS: &[&str] = &[
    "صحيح البخاري",
    "صحيح مسلم",
    "سنن أبي داود",
    "سنن النسائى الصغرى",
    "سنن الترمذي",
    "سنن ابن ماجه",
];

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Normalize an Arabic name into a slug for record IDs. Keeps diacritics.
/// Normalize an Arabic name into a slug for record IDs.
/// Uses normalize_arabic so that different diacritized forms of the same name
/// (e.g., أَبِي هُرَيْرَةَ and أبى هريرة) produce the same slug.
fn slug(name: &str) -> String {
    normalize_arabic(name).replace(' ', "_")
}

/// Slug with diacritics stripped — used ONLY for comparing names in compound isnad
/// duplicate detection. NOT for record IDs or display.
fn slug_bare(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| {
            let code = *c as u32;
            let is_diacritic = (0x0610..=0x061A).contains(&code)
                || (0x064B..=0x065F).contains(&code)
                || code == 0x0670
                || (0x06D6..=0x06DC).contains(&code)
                || (0x06DF..=0x06E8).contains(&code)
                || (0x06EA..=0x06ED).contains(&code);
            !is_diacritic && (c.is_alphanumeric() || *c == ' ')
        })
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join("_")
}

/// Normalize Arabic text for fuzzy matching: strip diacritics, normalize letter
/// variants (alef, taa marbuta, alef maqsura), normalize kunya grammatical case,
/// keep only Arabic letters + spaces.
pub fn normalize_arabic(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        let code = c as u32;
        // Skip diacritics (tashkeel)
        if (0x064B..=0x065F).contains(&code)
            || code == 0x0670
            || code == 0x0640 // tatweel
            || (0x0610..=0x061A).contains(&code)
        {
            continue;
        }
        // Normalize alef variants → bare alef
        if matches!(c, 'أ' | 'إ' | 'آ' | 'ٱ') {
            out.push('ا');
        // Normalize taa marbuta → haa
        } else if c == 'ة' {
            out.push('ه');
        // Normalize alef maqsura → yaa
        } else if c == 'ى' {
            out.push('ي');
        // Keep Arabic letters and spaces
        } else if (0x0620..=0x064A).contains(&code) || c == ' ' {
            out.push(c);
        }
    }
    // Collapse whitespace
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");

    // Normalize kunya grammatical case:
    // ابي X / ابى X → ابو X  (genitive/accusative → nominative for أبو)
    // امي X → ام X  (same for أم)
    normalize_kunya(&collapsed)
}

/// Normalize kunya grammatical case variants to nominative form.
/// In isnad chains, أبو becomes أبي/أبى after عن. This normalizes them back.
fn normalize_kunya(text: &str) -> String {
    let words: Vec<&str> = text.split(' ').collect();
    if words.is_empty() {
        return text.to_string();
    }

    let mut result = Vec::with_capacity(words.len());
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if i + 1 < words.len() {
            // ابي / ابى → ابو (when followed by another word, i.e. part of a kunya)
            if w == "ابي" || w == "ابى" {
                result.push("ابو");
                i += 1;
                continue;
            }
            // امي → ام
            if w == "امي" {
                result.push("ام");
                i += 1;
                continue;
            }
        }
        result.push(w);
        i += 1;
    }
    result.join(" ")
}

/// Book slug from Arabic book name.
fn book_slug(name: &str) -> String {
    format!("book_{}", slug(name))
}

/// Hadith slug from book name + hadith number (unique per book).
fn hadith_slug(book_name: &str, num: i64) -> String {
    format!("{}_{}", slug(book_name), num)
}

/// Parse the Sanad column: "['name1', 'name2', ...]" → Vec<String>
fn parse_sanad_list(sanad: &str) -> Vec<String> {
    let trimmed = sanad.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .map(|s| {
            s.trim()
                .trim_matches('\'')
                .trim_matches('"')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
}

/// Scan the CSV and return a sorted list of (book_name, hadith_count).
pub fn list_books(csv_path: &str) -> Result<Vec<(String, usize)>> {
    let mut books: BTreeMap<String, usize> = BTreeMap::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(csv_path)?;

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };
        let book = record.get(1).unwrap_or("").to_string();
        if !book.is_empty() {
            *books.entry(book).or_insert(0) += 1;
        }
    }

    let mut sorted: Vec<(String, usize)> = books.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(sorted)
}

/// Print the book list to stdout.
pub fn print_book_list(csv_path: &str) -> Result<()> {
    let books = list_books(csv_path)?;
    println!("{:>4}  {:>6}  {}", "#", "Count", "Book Name");
    println!("{}", "-".repeat(60));
    for (i, (name, count)) in books.iter().enumerate() {
        println!("{:>4}  {:>6}  {}", i + 1, count, name);
    }
    println!(
        "\nTotal: {} books, {} hadiths",
        books.len(),
        books.iter().map(|(_, c)| c).sum::<usize>()
    );
    println!("\nUse --books 1,2,3 to select specific books by number.");
    Ok(())
}

/// Resolve which books to ingest based on CLI flags.
pub fn resolve_books(
    csv_path: &str,
    book_indices: Option<&str>,
    all: bool,
) -> Result<HashSet<String>> {
    let all_books = list_books(csv_path)?;

    if all {
        return Ok(all_books.into_iter().map(|(name, _)| name).collect());
    }

    if let Some(indices_str) = book_indices {
        let mut selected = HashSet::new();
        for part in indices_str.split(',') {
            let idx: usize = part
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid book number: {part}"))?;
            if idx == 0 || idx > all_books.len() {
                anyhow::bail!("Book number {idx} out of range (1-{})", all_books.len());
            }
            selected.insert(all_books[idx - 1].0.clone());
        }
        return Ok(selected);
    }

    // Default: Kutub al-Sittah
    Ok(DEFAULT_BOOKS.iter().map(|s| s.to_string()).collect())
}

/// Main ingest function.
pub async fn ingest(
    db: &Surreal<Db>,
    csv_path: &str,
    selected_books: &HashSet<String>,
    limit_per_book: Option<usize>,
    resolver: Option<&super::name_resolver::NameResolver>,
) -> Result<()> {
    let path = Path::new(csv_path);
    if !path.exists() {
        anyhow::bail!("CSV file not found: {csv_path}");
    }

    if resolver.is_some() {
        println!(
            "📖 Ingesting from {} ({} books selected, with name resolution)",
            csv_path,
            selected_books.len()
        );
    } else {
        println!(
            "📖 Ingesting from {} ({} books selected)",
            csv_path,
            selected_books.len()
        );
    }

    // First pass: count how many hadiths we'll ingest (for progress bar)
    let total_expected = {
        let mut r = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(path)?;
        let mut counts: HashMap<String, usize> = HashMap::new();
        for rec in r.records().flatten() {
            let book = rec.get(1).unwrap_or("").to_string();
            if selected_books.contains(&book) {
                let c = counts.entry(book).or_insert(0);
                if let Some(limit) = limit_per_book {
                    if *c >= limit {
                        continue;
                    }
                }
                *c += 1;
            }
        }
        counts.values().sum::<usize>()
    };

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let mut book_counts: HashMap<String, usize> = HashMap::new();
    let mut narrators_created: HashSet<String> = HashSet::new();
    let mut books_created: HashSet<String> = HashSet::new();
    let mut hadith_count = 0;
    let mut heard_from_count = 0;
    let mut narrates_count = 0;

    let pb = make_progress(total_expected as u64, "hadiths ingested");

    for result in reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(_) => continue,
        };

        let book_name = record.get(1).unwrap_or("").to_string();
        if !selected_books.contains(&book_name) {
            continue;
        }

        // Check per-book limit
        let current_book_count = book_counts.entry(book_name.clone()).or_insert(0);
        if let Some(limit) = limit_per_book {
            if *current_book_count >= limit {
                continue;
            }
        }

        let num_str = record.get(2).unwrap_or("0");
        let hadith_num: i64 = num_str.parse().unwrap_or(0);
        if hadith_num == 0 {
            continue;
        }

        let raw_text = record.get(0).unwrap_or("");
        // Strip XML-style narrator tags from the full hadith text
        let arabic_text = raw_text
            .replace("<SANAD>", "")
            .replace("</SANAD>", "")
            .replace("<NAR>", "")
            .replace("</NAR>", "")
            .replace("<MATN>", "")
            .replace("</MATN>", "")
            .trim()
            .to_string();
        let matn = record.get(3).unwrap_or("").trim().to_string(); // Matn = actual hadith content
        let sanad_raw = record.get(4).unwrap_or(""); // Sanad = narrator chain
        let chain = parse_sanad_list(sanad_raw);

        let bslug = book_slug(&book_name);
        let hslug = hadith_slug(&book_name, hadith_num);

        // Create book if new
        if !books_created.contains(&book_name) {
            let book_num = books_created.len() as i64 + 1;
            db.query(
                "CREATE $rid CONTENT { book_number: $book_number, name_en: $name, name_ar: $name }",
            )
            .bind(("rid", rid("book", &bslug)))
            .bind(("book_number", book_num))
            .bind(("name", book_name.clone()))
            .await?;
            books_created.insert(book_name.clone());
        }

        // Narrator text = first narrator in chain (the primary narrator)
        let narrator_text = chain.last().cloned();

        // Create hadith — store both full text and matn (actual content without isnad)
        let matn_or_none: Option<String> = if matn.is_empty() { None } else { Some(matn) };
        db.query(
            "CREATE $rid CONTENT { \
                hadith_number: $hadith_number, \
                book_id: $book_id, \
                chapter_id: 0, \
                text_ar: $text_ar, \
                text_en: NONE, \
                matn: $matn, \
                narrator_text: $narrator_text, \
                grade: NONE, \
                book_name: $book_name, \
                embedding: NONE \
            }",
        )
        .bind(("rid", rid("hadith", &hslug)))
        .bind(("hadith_number", hadith_num))
        .bind(("book_id", books_created.len() as i64))
        .bind(("text_ar", arabic_text))
        .bind(("matn", matn_or_none))
        .bind(("narrator_text", narrator_text))
        .bind(("book_name", book_name.clone()))
        .await?;

        // Create belongs_to edge
        db.query("RELATE $from->belongs_to->$to")
            .bind(("from", rid("hadith", &hslug)))
            .bind(("to", rid("book", &bslug)))
            .await?;

        // Resolve narrator names and create narrator records + edges.
        //
        // When a NameResolver is available, we resolve each chain name to a
        // canonical ar_sanad_narrators ID. This deduplicates narrators that
        // appear under different spellings (e.g. أبي هريرة vs أبو هريرة)
        // and enables biographical enrichment later.

        // Build resolved slugs for this chain
        let resolved_chain: Vec<(String, String)> = if let Some(res) = resolver {
            // Use full chain resolution (multi-pass with context disambiguation)
            res.resolve_chain(&chain)
                .into_iter()
                .map(|(raw, resolution)| {
                    let resolved_slug = match resolution {
                        super::name_resolver::Resolution::Resolved(id) => {
                            // Use canonical narrator ID as slug
                            format!("arsanad_{id}")
                        }
                        _ => {
                            // Fallback to normalized slug for unresolved names
                            slug(&raw)
                        }
                    };
                    (raw, resolved_slug)
                })
                .collect()
        } else {
            chain.iter().map(|n| (n.clone(), slug(n))).collect()
        };

        for (name, nslug) in &resolved_chain {
            if nslug.is_empty() {
                continue;
            }
            // Skip relative references ("his father", "his grandfather") — not real narrator names
            let bare = normalize_arabic(name);
            if matches!(
                bare.as_str(),
                "ابي" | "ابيه" | "جده" | "عمه" | "امه" | "اخيه" | "ابيها"
            ) {
                continue;
            }
            if !narrators_created.contains(nslug) {
                // For resolved narrators, use canonical name from the resolver
                let display_name = if let Some(res) = resolver {
                    if let Some(id_str) = nslug.strip_prefix("arsanad_") {
                        if let Ok(id) = id_str.parse::<u32>() {
                            res.canonical_name(id).unwrap_or(name).to_string()
                        } else {
                            name.clone()
                        }
                    } else {
                        name.clone()
                    }
                } else {
                    name.clone()
                };

                db.query(
                    "CREATE $rid CONTENT { \
                        name_en: $name, \
                        name_ar: $name, \
                        search_name: $slug, \
                        gender: NONE, \
                        generation: NONE, \
                        bio: NONE \
                    }",
                )
                .bind(("rid", rid("narrator", nslug)))
                .bind(("name", display_name))
                .bind(("slug", nslug.clone()))
                .await
                .ok();
                narrators_created.insert(nslug.clone());
            }

            // narrates edge
            db.query("RELATE $from->narrates->$to")
                .bind(("from", rid("narrator", nslug)))
                .bind(("to", rid("hadith", &hslug)))
                .await
                .ok();
            narrates_count += 1;
        }

        // heard_from edges: chain[i] heard from chain[i+1]
        // For compound isnads (multiple chains merged by Sanadset), only create
        // edges where BOTH narrators are at their last (canonical) position.
        let slugs: Vec<&str> = resolved_chain.iter().map(|(_, s)| s.as_str()).collect();
        let mut last_pos: HashMap<&str, usize> = HashMap::new();
        for (i, s) in slugs.iter().enumerate() {
            last_pos.insert(s, i);
        }

        for i in 0..resolved_chain.len().saturating_sub(1) {
            let s1 = slugs[i];
            let s2 = slugs[i + 1];
            if s1.is_empty() || s2.is_empty() || s1 == s2 {
                continue;
            }
            // Only create edge if both narrators are at their canonical position
            if last_pos.get(s1) != Some(&i) {
                continue;
            }
            if last_pos.get(s2) != Some(&(i + 1)) {
                continue;
            }
            db.query("RELATE $from->heard_from->$to SET hadith_ref = $href")
                .bind(("from", rid("narrator", s1)))
                .bind(("to", rid("narrator", s2)))
                .bind(("href", rid("hadith", &hslug)))
                .await
                .ok();
            heard_from_count += 1;
        }

        *current_book_count += 1;
        hadith_count += 1;
        pb.inc(1);
    }

    pb.finish_with_message("done");
    println!(
        "   ✓ {} hadiths, {} books, {} narrators, {} chain edges",
        hadith_count,
        books_created.len(),
        narrators_created.len(),
        heard_from_count,
    );

    // Generate embeddings
    println!("🔢 Generating embeddings...");
    crate::embed::embed_all_hadiths(db).await?;

    Ok(())
}

// ── Human translations from sunnah.com (via HuggingFace) ──

const TRANSLATION_SOURCES: &[(&str, &str, &str)] = &[
    ("صحيح البخاري", "Sahih%20al-Bukhari.csv", "bukhari"),
    ("صحيح مسلم", "Sahih%20Muslim.csv", "muslim"),
    ("سنن أبي داود", "Sunan%20Abi%20Dawud.csv", "abudawud"),
    ("سنن النسائى الصغرى", "Sunan%20an-Nasa%27i.csv", "nasai"),
    ("سنن الترمذي", "Jami%60%20at-Tirmidhi.csv", "tirmidhi"),
    ("سنن ابن ماجه", "Sunan%20Ibn%20Majah.csv", "ibnmajah"),
];

const HF_BASE: &str = "https://huggingface.co/datasets/meeAtif/hadith_datasets/resolve/main/";

fn extract_narrator_en(english_text: &str) -> Option<String> {
    let text = english_text.trim();
    if let Some(rest) = text
        .strip_prefix("Narrated ")
        .or_else(|| text.strip_prefix("It is narrated on the authority of "))
    {
        if let Some(colon) = rest.find(':') {
            let name = rest[..colon].trim();
            if !name.is_empty() && name.len() < 100 {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn parse_hadith_num_from_ref(reference: &str) -> Option<i64> {
    // "https://sunnah.com/bukhari:1" → 1
    // "https://sunnah.com/muslim:8a" → 8
    reference
        .rsplit(':')
        .next()
        .and_then(|s| s.trim_end_matches(|c: char| c.is_alphabetic()).parse().ok())
}

pub async fn merge_human_translations(db: &Surreal<Db>) -> Result<()> {
    let client = Client::new();
    let translations_dir = std::path::Path::new("data/translations");
    std::fs::create_dir_all(translations_dir)?;

    let mut total_merged = 0;
    let mut narrator_names_found = 0;

    for (arabic_book, hf_file, short_name) in TRANSLATION_SOURCES {
        let csv_path = translations_dir.join(format!("{short_name}.csv"));

        // Download if not cached
        if !csv_path.exists() {
            let url = format!("{HF_BASE}{hf_file}");
            println!("   📥 Downloading {short_name}...");
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    let bytes = resp.bytes().await?;
                    std::fs::write(&csv_path, &bytes)?;
                    println!(" {} KB", bytes.len() / 1024);
                }
                Ok(resp) => {
                    println!(" failed (HTTP {})", resp.status());
                    continue;
                }
                Err(e) => {
                    println!(" failed ({e})");
                    continue;
                }
            }
        }

        // Parse CSV and build normalized_arabic → (english, narrator_en) map
        // We match by Arabic text similarity because Sanadset and sunnah.com use
        // different hadith numbering systems (global vs per-book).
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(&csv_path)?;

        struct Translation {
            normalized_arabic: String,
            english: String,
            narrator_en: Option<String>,
        }
        let mut translations: Vec<Translation> = Vec::new();
        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };
            let arabic = record.get(4).unwrap_or(""); // Arabic_Text column
            let english = record.get(5).unwrap_or("").to_string(); // English_Text
            if english.is_empty() || arabic.is_empty() {
                continue;
            }
            let narrator_en = extract_narrator_en(&english);
            translations.push(Translation {
                normalized_arabic: normalize_arabic(arabic),
                english,
                narrator_en,
            });
        }

        // Match translations to ingested hadiths by Arabic text similarity
        let mut merged = 0;

        #[derive(Debug, SurrealValue)]
        struct HadithForMatch {
            id: Option<RecordId>,
            hadith_number: i64,
            narrator_text: Option<String>,
            matn: Option<String>,
            text_ar: Option<String>,
        }

        let mut res = db
            .query(
                "SELECT id, hadith_number, narrator_text, matn, text_ar \
                 FROM hadith WHERE book_name = $book",
            )
            .bind(("book", arabic_book.to_string()))
            .await?;
        let hadiths: Vec<HadithForMatch> = res.take(0)?;

        let pb = make_progress(hadiths.len() as u64, short_name);
        for h in &hadiths {
            let Some(id) = &h.id else {
                pb.inc(1);
                continue;
            };

            // Normalize the matn (or text_ar) for matching
            let source_text = h.matn.as_deref().or(h.text_ar.as_deref()).unwrap_or("");
            let norm = normalize_arabic(source_text);

            // Use chars 20..80 as search key (skip common "رضي الله عنه" prefix)
            let chars: Vec<char> = norm.chars().collect();
            let key: String = if chars.len() > 80 {
                chars[20..80].iter().collect()
            } else if chars.len() > 30 {
                chars[10..].iter().collect()
            } else {
                norm.clone()
            };

            // Find the sunnah.com translation whose Arabic contains this key
            let matched = translations
                .iter()
                .find(|t| t.normalized_arabic.contains(&*key));

            if let Some(t) = matched {
                db.query("UPDATE $rid SET text_en = $en")
                    .bind(("rid", id.clone()))
                    .bind(("en", t.english.clone()))
                    .await
                    .ok();
                merged += 1;

                // Update narrator name_en if we found one
                if let Some(en_name) = &t.narrator_en {
                    if let Some(ar_name) = &h.narrator_text {
                        let nslug = slug(ar_name);
                        if !nslug.is_empty() {
                            db.query("UPDATE $rid SET name_en = $en")
                                .bind(("rid", rid("narrator", &nslug)))
                                .bind(("en", en_name.clone()))
                                .await
                                .ok();
                            narrator_names_found += 1;
                        }
                    }
                }
            }
            pb.inc(1);
        }

        pb.finish_and_clear();
        total_merged += merged;
        println!(
            "   ✓ {short_name}: {merged}/{} hadiths matched",
            hadiths.len()
        );
    }

    println!(
        "   ✓ Total: {total_merged} human translations, {narrator_names_found} narrator names"
    );
    Ok(())
}

// ── Translation via Ollama (fallback) ──

#[derive(Deserialize)]
struct OllamaGenResponse {
    response: String,
}

async fn ollama_generate(client: &Client, model: &str, prompt: &str) -> Result<String> {
    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        }))
        .send()
        .await?;
    if !res.status().is_success() {
        anyhow::bail!("Ollama error: {}", res.status());
    }
    let body: OllamaGenResponse = res.json().await?;
    Ok(body.response.trim().to_string())
}

fn fix_honorifics(text: &str) -> String {
    text.replace("PBUH", "ﷺ")
        .replace("pbuh", "ﷺ")
        .replace("peace be upon him", "ﷺ")
        .replace("Peace be upon him", "ﷺ")
        .replace("Peace Be Upon Him", "ﷺ")
}

#[derive(Debug, SurrealValue)]
struct IdAndArabic {
    id: Option<RecordId>,
    text_ar: Option<String>,
    matn: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct IdAndName {
    id: Option<RecordId>,
    name_ar: Option<String>,
}

pub async fn translate_all(db: &Surreal<Db>, model: &str) -> Result<()> {
    let client = Client::new();

    // Translate narrator names
    println!("🌐 Translating narrator names via Ollama ({model})...");
    let mut res = db
        .query("SELECT id, name_ar FROM narrator WHERE name_ar IS NOT NONE AND (name_en IS NONE OR name_en = name_ar)")
        .await?;
    let narrators: Vec<IdAndName> = res.take(0)?;

    // Batch narrators in groups of 20 for speed
    let total_batches = (narrators.len() + 19) / 20;
    println!(
        "   {} narrators in {} batches (may take a minute per batch)",
        narrators.len(),
        total_batches
    );
    let pb = make_progress(narrators.len() as u64, "narrators");
    for (batch_idx, chunk) in narrators.chunks(20).enumerate() {
        let names: Vec<(&RecordId, &str)> = chunk
            .iter()
            .filter_map(|n| Some((n.id.as_ref()?, n.name_ar.as_deref()?)))
            .collect();

        if names.is_empty() {
            pb.inc(chunk.len() as u64);
            continue;
        }

        let numbered: String = names
            .iter()
            .enumerate()
            .map(|(i, (_, ar))| format!("{}. {}", i + 1, ar))
            .collect::<Vec<_>>()
            .join("\n");

        let prompt = format!(
            "Transliterate these Arabic names to English. Output ONLY the English names, one per line, numbered to match. No explanations.\n\n{numbered}"
        );

        if let Ok(response) = ollama_generate(&client, model, &prompt).await {
            let lines: Vec<&str> = response
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();

            for (i, (id, _)) in names.iter().enumerate() {
                if let Some(line) = lines.get(i) {
                    // Strip numbering like "1. " or "1) "
                    let english = line.trim_start_matches(|c: char| {
                        c.is_ascii_digit() || c == '.' || c == ')' || c == ' '
                    });
                    let english = fix_honorifics(english.trim());
                    if !english.is_empty() && english.len() < 200 {
                        db.query("UPDATE $rid SET name_en = $v")
                            .bind(("rid", (*id).clone()))
                            .bind(("v", english))
                            .await
                            .ok();
                    }
                }
            }
        }
        pb.inc(chunk.len() as u64);
    }
    pb.finish_with_message("done");
    println!("   ✓ {} narrator names translated", narrators.len());

    // Translate hadith texts
    println!("🌐 Translating hadith texts via Ollama ({model})...");
    let mut res = db
        .query("SELECT id, text_ar, matn FROM hadith WHERE text_ar IS NOT NONE AND text_en IS NONE")
        .await?;
    let hadiths: Vec<IdAndArabic> = res.take(0)?;

    let pb = make_progress(hadiths.len() as u64, "hadiths");
    for h in &hadiths {
        if let Some(id) = &h.id {
            // Translate only the matn (actual hadith content, no isnad preamble)
            let arabic = h.matn.as_deref().or(h.text_ar.as_deref()).unwrap_or("");
            if arabic.is_empty() {
                pb.inc(1);
                continue;
            }
            let text = if arabic.len() > 3000 {
                &arabic[..3000]
            } else {
                arabic
            };
            let prompt = format!(
                "Translate this Islamic hadith from Arabic to English. \
                 Use the symbol ﷺ after mentions of the Prophet Muhammad. \
                 Never write PBUH or 'peace be upon him'. \
                 Output ONLY the English translation.\n\n{text}"
            );
            if let Ok(english) = ollama_generate(&client, model, &prompt).await {
                let english = fix_honorifics(&english);
                if !english.is_empty() {
                    db.query("UPDATE $rid SET text_en = $v")
                        .bind(("rid", id.clone()))
                        .bind(("v", english))
                        .await
                        .ok();
                }
            }
        }
        pb.inc(1);
    }
    pb.finish_with_message("done");
    println!("   ✓ {} hadiths translated", hadiths.len());
    println!("✅ Translation complete");
    Ok(())
}
