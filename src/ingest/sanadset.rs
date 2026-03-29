use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

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

/// Normalize an Arabic name into a slug for record IDs.
fn slug(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    cleaned.split_whitespace().collect::<Vec<_>>().join("_")
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
) -> Result<()> {
    let path = Path::new(csv_path);
    if !path.exists() {
        anyhow::bail!("CSV file not found: {csv_path}");
    }

    tracing::info!(
        "Ingesting from {} ({} books selected)",
        csv_path,
        selected_books.len()
    );

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

        let arabic_text = record.get(0).unwrap_or("").to_string();
        let sanad_raw = record.get(3).unwrap_or("");
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

        // Create hadith
        db.query(
            "CREATE $rid CONTENT { \
                hadith_number: $hadith_number, \
                book_id: $book_id, \
                chapter_id: 0, \
                text_ar: $text_ar, \
                text_en: NONE, \
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
        .bind(("narrator_text", narrator_text))
        .bind(("book_name", book_name.clone()))
        .await?;

        // Create belongs_to edge
        db.query("RELATE $from->belongs_to->$to")
            .bind(("from", rid("hadith", &hslug)))
            .bind(("to", rid("book", &bslug)))
            .await?;

        // Create narrators and chain edges
        for name in &chain {
            let nslug = slug(name);
            if nslug.is_empty() {
                continue;
            }
            if !narrators_created.contains(&nslug) {
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
                .bind(("rid", rid("narrator", &nslug)))
                .bind(("name", name.clone()))
                .bind(("slug", nslug.clone()))
                .await
                .ok();
                narrators_created.insert(nslug.clone());
            }

            // narrates edge
            db.query("RELATE $from->narrates->$to")
                .bind(("from", rid("narrator", &nslug)))
                .bind(("to", rid("hadith", &hslug)))
                .await
                .ok();
            narrates_count += 1;
        }

        // heard_from edges: chain[i] heard from chain[i+1]
        for i in 0..chain.len().saturating_sub(1) {
            let student = slug(&chain[i]);
            let teacher = slug(&chain[i + 1]);
            if student.is_empty() || teacher.is_empty() {
                continue;
            }
            db.query("RELATE $from->heard_from->$to SET hadith_ref = $href")
                .bind(("from", rid("narrator", &student)))
                .bind(("to", rid("narrator", &teacher)))
                .bind(("href", rid("hadith", &hslug)))
                .await
                .ok();
            heard_from_count += 1;
        }

        *current_book_count += 1;
        hadith_count += 1;
        if hadith_count % 500 == 0 {
            tracing::info!("Inserted {hadith_count} hadiths...");
        }
    }

    tracing::info!(
        "Inserted {} hadiths, {} books, {} narrators, {} heard_from edges, {} narrates edges",
        hadith_count,
        books_created.len(),
        narrators_created.len(),
        heard_from_count,
        narrates_count,
    );

    // Generate embeddings
    tracing::info!("Generating embeddings (this may take a while)...");
    crate::embed::embed_all_hadiths(db).await?;

    Ok(())
}

// ── Translation via Ollama ──

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
}

#[derive(Debug, SurrealValue)]
struct IdAndName {
    id: Option<RecordId>,
    name_ar: Option<String>,
}

pub async fn translate_all(db: &Surreal<Db>, model: &str) -> Result<()> {
    let client = Client::new();

    // Translate narrator names
    tracing::info!("Translating narrator names via Ollama...");
    let mut res = db
        .query("SELECT id, name_ar FROM narrator WHERE name_ar IS NOT NONE")
        .await?;
    let narrators: Vec<IdAndName> = res.take(0)?;
    tracing::info!("Translating {} narrator names", narrators.len());

    for (i, n) in narrators.iter().enumerate() {
        if let (Some(id), Some(arabic)) = (&n.id, &n.name_ar) {
            let prompt = format!(
                "Transliterate this Arabic name to English. Output ONLY the English name, nothing else.\n\n{arabic}"
            );
            match ollama_generate(&client, model, &prompt).await {
                Ok(english) => {
                    let english = fix_honorifics(&english);
                    if !english.is_empty() && english.len() < 200 {
                        db.query("UPDATE $rid SET name_en = $v")
                            .bind(("rid", id.clone()))
                            .bind(("v", english))
                            .await
                            .ok();
                    }
                }
                Err(e) => tracing::warn!("Narrator {i} translation failed: {e}"),
            }
            if (i + 1) % 50 == 0 {
                tracing::info!("Translated {}/{} narrator names", i + 1, narrators.len());
            }
        }
    }

    // Translate hadith texts
    tracing::info!("Translating hadith texts via Ollama...");
    let mut res = db
        .query("SELECT id, text_ar FROM hadith WHERE text_ar IS NOT NONE")
        .await?;
    let hadiths: Vec<IdAndArabic> = res.take(0)?;
    tracing::info!("Translating {} hadiths", hadiths.len());

    for (i, h) in hadiths.iter().enumerate() {
        if let (Some(id), Some(arabic)) = (&h.id, &h.text_ar) {
            let text = if arabic.len() > 3000 {
                &arabic[..3000]
            } else {
                arabic.as_str()
            };
            let prompt = format!(
                "Translate this Islamic hadith from Arabic to English. \
                 Use the symbol ﷺ after mentions of the Prophet Muhammad. \
                 Never write PBUH or 'peace be upon him'. \
                 Output ONLY the English translation.\n\n{text}"
            );
            match ollama_generate(&client, model, &prompt).await {
                Ok(english) => {
                    let english = fix_honorifics(&english);
                    if !english.is_empty() {
                        db.query("UPDATE $rid SET text_en = $v")
                            .bind(("rid", id.clone()))
                            .bind(("v", english))
                            .await
                            .ok();
                    }
                }
                Err(e) => tracing::warn!("Hadith {i} translation failed: {e}"),
            }
            if (i + 1) % 10 == 0 {
                tracing::info!("Translated {}/{} hadiths", i + 1, hadiths.len());
            }
        }
    }

    tracing::info!("Translation complete");
    Ok(())
}
