use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::types::SurrealValue;

use super::AppState;
use crate::book_chat;
use crate::rag::ChatChunk;

// ── Response types ──

#[derive(Debug, Serialize)]
pub struct BookSummary {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
}

#[derive(Debug, Serialize)]
pub struct BookDetail {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
    pub headings: Vec<BookHeading>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookHeading {
    pub title: String,
    pub level: u32,
    pub page_index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookPage {
    pub page_index: u64,
    pub text: String,
    pub vol: String,
    pub page_num: u64,
}

#[derive(Debug, Serialize)]
pub struct BookPagesResponse {
    pub pages: Vec<BookPage>,
    pub total: u64,
    pub start: u64,
    pub size: u64,
}

#[derive(Debug, Serialize)]
pub struct TafsirPageRef {
    pub page_index: u64,
    pub heading: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TafsirSurahMappings {
    pub mappings: HashMap<String, TafsirPageRef>,
}

// ── Query params ──

#[derive(Deserialize)]
pub struct PaginationParams {
    pub start: Option<u64>,
    pub size: Option<u64>,
}

// ── Sharh response types ──

#[derive(Debug, Serialize)]
pub struct SharhPageRef {
    pub book_id: u64,
    pub page_index: u64,
    pub book_name: String,
}

#[derive(Debug, Serialize)]
pub struct SharhBatchResponse {
    pub mappings: HashMap<String, SharhPageRef>,
}

#[derive(Deserialize)]
pub struct SharhBatchParams {
    pub book: Option<u64>,
    pub numbers: Option<String>, // comma-separated hadith numbers
}

// ── Books config response ──

#[derive(Debug, Serialize)]
pub struct BookConfigEntry {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub category: Option<String>,
    pub book_type: Option<String>,
    pub chat_enabled: bool,
    pub default_questions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TafsirBookEntry {
    pub book_id: u64,
    pub slug: String,
    pub name_ar: String,
    pub name_en: String,
    pub is_default: bool,
}

#[derive(Debug, Serialize)]
pub struct BooksConfigResponse {
    pub books: Vec<BookConfigEntry>,
    pub tafsir_books: Vec<TafsirBookEntry>,
}

/// Ibn Kathir is the historical default — appears first and is selected by the UI
/// when no explicit tafsir is chosen.
const DEFAULT_TAFSIR_BOOK_ID: u64 = 23604;

fn slugify_tafsir(name_en: &str) -> String {
    let cleaned: String = name_en
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    cleaned
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn default_questions_for_type(book_type: Option<&str>) -> Vec<String> {
    match book_type {
        Some("tafsir") => vec![
            "ما تفسير بسم الله الرحمن الرحيم؟".into(),
            "ما تفسير آية الكرسي؟".into(),
        ],
        Some("sharh") => vec![
            "ما هو بدء الوحي؟".into(),
            "ما هي المواضيع التي يتناولها هذا الكتاب؟".into(),
        ],
        Some("collection") => vec![
            "ما هي أبواب هذا الكتاب؟".into(),
            "ما أول حديث في هذا الكتاب؟".into(),
        ],
        Some("biography") => vec![
            "من هم أشهر الرواة في هذا الكتاب؟".into(),
            "ما ترجمة أبي هريرة؟".into(),
        ],
        _ => vec!["ما هي المواضيع التي يتناولها هذا الكتاب؟".into()],
    }
}

// ── DB row types ──

#[derive(Debug, SurrealValue)]
struct ConfigBookRow {
    book_id: i64,
    name_ar: String,
    name_en: String,
    category: Option<String>,
    book_type: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct SharhRow {
    hadith_number: i64,
    book_id: i64,
    page_index: i64,
}

#[derive(Debug, SurrealValue)]
struct NarratorBookRow {
    book_id: i64,
    page_index: i64,
    entry_num: Option<i64>,
    book_name: String,
}

#[derive(Debug, Serialize)]
pub struct NarratorBookRef {
    pub book_id: u64,
    pub page_index: u64,
    pub entry_num: Option<u64>,
    pub book_name: String,
}

#[derive(Debug, SurrealValue)]
struct NameRow {
    name_en: String,
}

#[derive(Debug, SurrealValue)]
struct BookRow {
    book_id: i64,
    name_ar: String,
    name_en: String,
    author_ar: String,
    total_pages: i64,
    headings: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct PageRow {
    page_index: i64,
    text: String,
    vol: String,
    page_num: i64,
}

#[derive(Debug, SurrealValue)]
struct MappingRow {
    ayah: i64,
    page_index: i64,
    heading: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct CountResult {
    c: i64,
}

// ── Handlers ──

/// GET /api/books/config
/// Returns book metadata with categories, types, chat status, and default questions.
pub async fn books_config(State(state): State<AppState>) -> impl IntoResponse {
    let result: Result<Vec<ConfigBookRow>, _> = state
        .db
        .query("SELECT book_id, name_ar, name_en, category, book_type FROM book")
        .await
        .and_then(|mut r| r.take(0));

    let chat_book_ids: std::collections::HashSet<u64> = state
        .book_trees
        .as_ref()
        .map(|trees| trees.keys().copied().collect())
        .unwrap_or_default();

    match result {
        Ok(rows) => {
            let mut tafsir_books: Vec<TafsirBookEntry> = Vec::new();
            let books: Vec<BookConfigEntry> = rows
                .into_iter()
                .map(|b| {
                    let bid = b.book_id as u64;
                    let bt = b.book_type.as_deref();
                    if bt == Some("tafsir") {
                        tafsir_books.push(TafsirBookEntry {
                            book_id: bid,
                            slug: slugify_tafsir(&b.name_en),
                            name_ar: b.name_ar.clone(),
                            name_en: b.name_en.clone(),
                            is_default: bid == DEFAULT_TAFSIR_BOOK_ID,
                        });
                    }
                    BookConfigEntry {
                        book_id: bid,
                        name_ar: b.name_ar,
                        name_en: b.name_en,
                        category: b.category,
                        book_type: b.book_type.clone(),
                        chat_enabled: chat_book_ids.contains(&bid),
                        default_questions: default_questions_for_type(bt),
                    }
                })
                .collect();

            // Default first, then stable by book_id.
            tafsir_books.sort_by_key(|t| (!t.is_default, t.book_id));

            Json(BooksConfigResponse {
                books,
                tafsir_books,
            })
            .into_response()
        }
        Err(e) => {
            tracing::error!("Failed to fetch books config: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch config").into_response()
        }
    }
}

pub async fn list_books(State(state): State<AppState>) -> impl IntoResponse {
    let result: Result<Vec<BookRow>, _> = state
        .db
        .query("SELECT book_id, name_ar, name_en, author_ar, total_pages FROM book")
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(books) => {
            let summaries: Vec<BookSummary> = books
                .into_iter()
                .map(|b| BookSummary {
                    book_id: b.book_id as u64,
                    name_ar: b.name_ar,
                    name_en: b.name_en,
                    author_ar: b.author_ar,
                    total_pages: b.total_pages as u64,
                })
                .collect();
            Json(summaries).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to list books: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to list books").into_response()
        }
    }
}

pub async fn get_book(
    State(state): State<AppState>,
    Path(book_id): Path<u64>,
) -> impl IntoResponse {
    let result: Result<Option<BookRow>, _> = state
        .db
        .query("SELECT * FROM book WHERE book_id = $bid LIMIT 1")
        .bind(("bid", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(Some(book)) => {
            let headings: Vec<BookHeading> = book
                .headings
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            Json(BookDetail {
                book_id: book.book_id as u64,
                name_ar: book.name_ar,
                name_en: book.name_en,
                author_ar: book.author_ar,
                total_pages: book.total_pages as u64,
                headings,
            })
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "Book not found").into_response(),
        Err(e) => {
            tracing::error!("Failed to get book {book_id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get book").into_response()
        }
    }
}

pub async fn get_pages(
    State(state): State<AppState>,
    Path(book_id): Path<u64>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    let start = params.start.unwrap_or(0);
    let size = params.size.unwrap_or(20).min(100);

    let result = state
        .db
        .query(
            "SELECT page_index, text, vol, page_num FROM book_page \
             WHERE book_id = $bid AND page_index >= $start AND page_index < $end \
             ORDER BY page_index ASC",
        )
        .bind(("bid", book_id as i64))
        .bind(("start", start as i64))
        .bind(("end", (start + size) as i64))
        .await;

    let total_result: Result<Option<CountResult>, _> = state
        .db
        .query("SELECT count() AS c FROM book_page WHERE book_id = $bid GROUP ALL")
        .bind(("bid", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    let total = total_result.ok().flatten().map(|c| c.c as u64).unwrap_or(0);

    match result {
        Ok(mut res) => {
            let pages: Vec<PageRow> = res.take(0).unwrap_or_default();
            let response = BookPagesResponse {
                pages: pages
                    .into_iter()
                    .map(|p| BookPage {
                        page_index: p.page_index as u64,
                        text: p.text,
                        vol: p.vol,
                        page_num: p.page_num as u64,
                    })
                    .collect(),
                total,
                start,
                size,
            };
            Json(response).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get pages for book {book_id}: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get pages").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct TafsirQuery {
    pub book_id: Option<u64>,
}

pub async fn surah_tafsir_pages(
    State(state): State<AppState>,
    Path(surah_number): Path<u64>,
    Query(q): Query<TafsirQuery>,
) -> impl IntoResponse {
    let book_id = q.book_id.unwrap_or(DEFAULT_TAFSIR_BOOK_ID);
    let result: Result<Vec<MappingRow>, _> = state
        .db
        .query(
            "SELECT ayah, page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $surah AND book_id = $book ORDER BY ayah ASC",
        )
        .bind(("surah", surah_number as i64))
        .bind(("book", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(rows) => {
            let mut mappings = HashMap::new();
            for row in rows {
                mappings.insert(
                    row.ayah.to_string(),
                    TafsirPageRef {
                        page_index: row.page_index as u64,
                        heading: row.heading,
                    },
                );
            }
            Json(TafsirSurahMappings { mappings }).into_response()
        }
        Err(e) => {
            tracing::error!(
                "Failed to get tafsir pages for surah {surah_number} book {book_id}: {e}"
            );
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get tafsir pages",
            )
                .into_response()
        }
    }
}

#[derive(Debug, SurrealValue)]
struct AyahTafsirMapping {
    page_index: i64,
    heading: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct AyahTafsirPage {
    text: String,
    vol: String,
    page_num: i64,
}

#[derive(Debug, Serialize)]
pub struct AyahTafsirResponse {
    pub book_id: u64,
    pub surah: u64,
    pub ayah: u64,
    pub page_index: u64,
    pub vol: String,
    pub page_num: u64,
    pub heading: Option<String>,
    pub text: String,
}

/// GET /api/quran/ayah/{surah}/{ayah}/tafsir?book_id=<id>
/// Returns the tafsir body for one ayah from one book in a single response.
/// The `(surah, ayah)` anchor stays constant while the UI switches `book_id`.
pub async fn ayah_tafsir(
    State(state): State<AppState>,
    Path((surah, ayah)): Path<(u64, u64)>,
    Query(q): Query<TafsirQuery>,
) -> impl IntoResponse {
    let book_id = q.book_id.unwrap_or(DEFAULT_TAFSIR_BOOK_ID);

    let mapping: Result<Option<AyahTafsirMapping>, _> = state
        .db
        .query(
            "SELECT page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $s AND ayah = $a AND book_id = $b LIMIT 1",
        )
        .bind(("s", surah as i64))
        .bind(("a", ayah as i64))
        .bind(("b", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    let mapping = match mapping {
        Ok(Some(m)) => m,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "No tafsir mapping for this ayah").into_response();
        }
        Err(e) => {
            tracing::error!(
                "Failed to lookup tafsir mapping for {surah}:{ayah} book {book_id}: {e}"
            );
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to lookup tafsir").into_response();
        }
    };

    let page: Result<Option<AyahTafsirPage>, _> = state
        .db
        .query(
            "SELECT text, vol, page_num FROM book_page \
             WHERE book_id = $b AND page_index = $p LIMIT 1",
        )
        .bind(("b", book_id as i64))
        .bind(("p", mapping.page_index))
        .await
        .and_then(|mut r| r.take(0));

    match page {
        Ok(Some(p)) => Json(AyahTafsirResponse {
            book_id,
            surah,
            ayah,
            page_index: mapping.page_index as u64,
            vol: p.vol,
            page_num: p.page_num as u64,
            heading: mapping.heading,
            text: p.text,
        })
        .into_response(),
        Ok(None) => {
            tracing::warn!(
                "tafsir mapping for {surah}:{ayah} book {book_id} points at missing page {}",
                mapping.page_index
            );
            (StatusCode::NOT_FOUND, "Tafsir page missing").into_response()
        }
        Err(e) => {
            tracing::error!("Failed to load tafsir page for {surah}:{ayah} book {book_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to load tafsir page",
            )
                .into_response()
        }
    }
}

/// Batch lookup: hadith numbers -> sharh page references.
/// GET /api/hadiths/sharh-pages?book=1&numbers=1,2,3,4,5
pub async fn hadith_sharh_pages(
    State(state): State<AppState>,
    Query(params): Query<SharhBatchParams>,
) -> impl IntoResponse {
    let collection_id = params.book.unwrap_or(1); // default to Bukhari

    let numbers: Vec<i64> = params
        .numbers
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    if numbers.is_empty() {
        return Json(SharhBatchResponse {
            mappings: HashMap::new(),
        })
        .into_response();
    }

    // Build SQL with IN clause
    let nums_str = numbers
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let sql = format!(
        "SELECT hadith_number, book_id, page_index FROM hadith_sharh_map \
         WHERE collection_id = {collection_id} AND hadith_number IN [{nums_str}]"
    );

    let result: Result<Vec<SharhRow>, _> = state.db.query(&sql).await.and_then(|mut r| r.take(0));

    // Get sharh book name for display
    let book_name = match state
        .db
        .query("SELECT name_en FROM book WHERE book_id IN (SELECT VALUE book_id FROM hadith_sharh_map WHERE collection_id = $bid LIMIT 1) LIMIT 1")
        .bind(("bid", collection_id as i64))
        .await
    {
        Ok(mut r) => {
            let row: Option<NameRow> = r.take(0).unwrap_or(None);
            row.map(|r| r.name_en).unwrap_or_else(|| "Fath al-Bari".to_string())
        }
        Err(_) => "Fath al-Bari".to_string(),
    };

    match result {
        Ok(rows) => {
            let mut mappings = HashMap::new();
            for row in rows {
                mappings.insert(
                    row.hadith_number.to_string(),
                    SharhPageRef {
                        book_id: row.book_id as u64,
                        page_index: row.page_index as u64,
                        book_name: book_name.clone(),
                    },
                );
            }
            Json(SharhBatchResponse { mappings }).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get sharh pages: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get sharh pages",
            )
                .into_response()
        }
    }
}

/// Get all book references for a narrator.
/// GET /api/narrators/:id/books
pub async fn narrator_books(
    State(state): State<AppState>,
    Path(narrator_id): Path<String>,
) -> impl IntoResponse {
    // Strip "narrator:" prefix if present (URL params include the table prefix)
    let clean_id = narrator_id
        .strip_prefix("narrator:")
        .unwrap_or(&narrator_id)
        .to_string();

    let result: Result<Vec<NarratorBookRow>, _> = state
        .db
        .query(
            "SELECT book_id, page_index, entry_num, book_name \
             FROM narrator_book_map WHERE narrator_id = $nid",
        )
        .bind(("nid", clean_id.clone()))
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(rows) => {
            let refs: Vec<NarratorBookRef> = rows
                .into_iter()
                .map(|r| NarratorBookRef {
                    book_id: r.book_id as u64,
                    page_index: r.page_index as u64,
                    entry_num: r.entry_num.map(|n| n as u64),
                    book_name: r.book_name,
                })
                .collect();
            Json(refs).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get narrator books for {clean_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get narrator books",
            )
                .into_response()
        }
    }
}

// ── Book Chat (PageIndex-style agentic retrieval) ──

#[derive(Debug, Deserialize)]
pub struct BookChatRequest {
    pub question: String,
}

/// POST /api/books/{book_id}/chat
///
/// Streams an SSE response immediately — the user sees "navigating" status
/// while the LLM processes. Uses two-phase navigation and caching.
pub async fn book_chat(
    State(state): State<AppState>,
    Path(book_id): Path<u64>,
    Json(body): Json<BookChatRequest>,
) -> Result<Response, StatusCode> {
    let question = body.question.trim().to_string();
    if question.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let ollama = state.ollama.as_ref().ok_or_else(|| {
        tracing::error!("Ollama client not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let book_trees = state.book_trees.as_ref().ok_or_else(|| {
        tracing::error!("Book trees not loaded (--pageindex-dir not set)");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let book = book_trees.get(&book_id).ok_or_else(|| {
        tracing::error!("Book {book_id} not found in PageIndex data");
        StatusCode::NOT_FOUND
    })?;

    let ollama = ollama.clone();
    let book = book.clone();
    let nav_cache = state.nav_cache.clone();

    // Build the SSE stream — work happens inside, so status events are sent immediately
    let sse_stream = async_stream::stream! {
        use futures::StreamExt;

        // Step 1: Navigate (two-phase, with cache)
        yield Ok::<_, std::io::Error>(
            bytes::Bytes::from("data: {\"status\":\"navigating\"}\n\n")
        );

        // Check cache first
        let ranges = if let Some(cached) = nav_cache.get(book.book_id, &question) {
            tracing::info!("Nav cache hit for book {} q={}", book.book_id, &question[..question.len().min(40)]);
            cached
        } else {
            match book_chat::navigate_two_phase(&ollama, &book, &question).await {
                Ok(r) => {
                    // Only cache non-empty results
                    if !r.is_empty() {
                        nav_cache.put(book.book_id, &question, r.clone());
                    }
                    r
                }
                Err(e) => {
                    tracing::error!("navigate_two_phase failed: {e}");
                    yield Ok(bytes::Bytes::from(format!(
                        "data: {}\n\n",
                        serde_json::json!({"error": format!("Navigation failed: {e}")})
                    )));
                    return;
                }
            }
        };

        // Step 2: Fetch sections
        yield Ok(bytes::Bytes::from(format!(
            "data: {}\n\n",
            serde_json::json!({"status": "reading", "sections": ranges})
        )));

        let sections = match book_chat::fetch_sections(&book, &ranges) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("fetch_sections failed: {e}");
                Vec::new()
            }
        };

        // If no sections could be retrieved, surface this to the user
        if sections.is_empty() {
            tracing::warn!("No sections found for question: {}", &question[..question.len().min(80)]);
            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "status": "no_relevant_sections",
                    "message": "Could not find relevant sections in this book for your question. Try rephrasing in Arabic or asking about a more specific topic."
                })
            )));
        }

        let sources = book_chat::build_sources(&book, &ranges);
        yield Ok(bytes::Bytes::from(format!(
            "data: {}\n\n",
            serde_json::to_string(&serde_json::json!({"sources": sources})).unwrap()
        )));

        // Step 3: Stream answer
        let answer_prompt = book_chat::build_answer_prompt(&book.name_en, &sections);

        let byte_stream = match ollama
            .chat_stream(&answer_prompt, &question, None)
            .await
        {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("answer stream failed: {e}");
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({"error": format!("Answer generation failed: {e}")})
                )));
                return;
            }
        };

        let mut byte_stream = std::pin::pin!(byte_stream);
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(raw) => {
                    let mut sse = String::new();
                    for line in raw.split(|&b| b == b'\n') {
                        if line.is_empty() {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_slice::<ChatChunk>(line) {
                            if let Some(msg) = parsed.message
                                && !msg.content.is_empty()
                            {
                                sse.push_str(&format!(
                                    "data: {}\n\n",
                                    serde_json::to_string(
                                        &serde_json::json!({"text": msg.content})
                                    )
                                    .unwrap()
                                ));
                            }
                            if parsed.done {
                                sse.push_str("data: {\"done\":true}\n\n");
                            }
                        }
                    }
                    if !sse.is_empty() {
                        yield Ok(bytes::Bytes::from(sse));
                    }
                }
                Err(e) => {
                    yield Ok(bytes::Bytes::from(format!(
                        "data: {}\n\n",
                        serde_json::json!({"error": e.to_string()})
                    )));
                }
            }
        }
    };

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::from_stream(sse_stream))
        .unwrap())
}

// ── Multi-tafsir feature (/tafsir page) ───────────────────────────────────

#[derive(Debug, SurrealValue)]
struct AllMappingRow {
    book_id: i64,
    page_index: i64,
    heading: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct TafsirBookMetaRow {
    book_id: i64,
    name_en: String,
    name_ar: String,
}

#[derive(Debug, SurrealValue)]
struct AyahTafsirEnRow {
    tafsir_en: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AllTafsirsEntry {
    pub book_id: u64,
    pub name_en: String,
    pub name_ar: String,
    pub is_default: bool,
    pub page_index: u64,
    pub vol: String,
    pub page_num: u64,
    pub heading: Option<String>,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct InlineEnglishTafsir {
    pub body: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AllTafsirsResponse {
    pub surah: u64,
    pub ayah: u64,
    pub entries: Vec<AllTafsirsEntry>,
    pub english: InlineEnglishTafsir,
}

/// GET /api/tafsir/ayah/{surah}/{ayah}/all
/// Returns every available tafsir for this ayah in one response:
///   - One Arabic entry per ingested book (book_type="tafsir") with page_index + full page text.
///   - The inline English Ibn Kathir from ayah.tafsir_en, if present.
pub async fn ayah_tafsirs_all(
    State(state): State<AppState>,
    Path((surah, ayah)): Path<(u64, u64)>,
) -> impl IntoResponse {
    // 1. Per-book mappings for this ayah.
    let mappings: Result<Vec<AllMappingRow>, _> = state
        .db
        .query(
            "SELECT book_id, page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $s AND ayah = $a",
        )
        .bind(("s", surah as i64))
        .bind(("a", ayah as i64))
        .await
        .and_then(|mut r| r.take(0));

    let mappings = match mappings {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Failed to fetch tafsir mappings for {surah}:{ayah}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch tafsir mappings",
            )
                .into_response();
        }
    };

    // 2. Tafsir book metadata (book_id → name_en/name_ar). Filters out non-tafsir
    //    rows accidentally present in tafsir_ayah_map (defensive).
    let books: Result<Vec<TafsirBookMetaRow>, _> = state
        .db
        .query(r#"SELECT book_id, name_en, name_ar FROM book WHERE book_type = "tafsir""#)
        .await
        .and_then(|mut r| r.take(0));

    let book_meta: std::collections::HashMap<i64, (String, String)> = match books {
        Ok(rows) => rows
            .into_iter()
            .map(|b| (b.book_id, (b.name_en, b.name_ar)))
            .collect(),
        Err(e) => {
            tracing::error!("Failed to fetch tafsir book metadata: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to fetch book metadata",
            )
                .into_response();
        }
    };

    // 3. Fetch page text per mapping. Sequential is fine — 2-5 books, each
    //    lookup is indexed and fast.
    let mut entries: Vec<AllTafsirsEntry> = Vec::with_capacity(mappings.len());
    for m in mappings {
        let Some((name_en, name_ar)) = book_meta.get(&m.book_id).cloned() else {
            // A book referenced in the map but missing from the registry (or not
            // of type "tafsir"). Skip — don't leak non-tafsir bodies here.
            continue;
        };

        let page: Result<Option<AyahTafsirPage>, _> = state
            .db
            .query(
                "SELECT text, vol, page_num FROM book_page \
                 WHERE book_id = $b AND page_index = $p LIMIT 1",
            )
            .bind(("b", m.book_id))
            .bind(("p", m.page_index))
            .await
            .and_then(|mut r| r.take(0));

        let Ok(Some(p)) = page else {
            tracing::warn!(
                "Missing book_page for book {} page_index {} (ayah {}:{})",
                m.book_id,
                m.page_index,
                surah,
                ayah
            );
            continue;
        };

        entries.push(AllTafsirsEntry {
            book_id: m.book_id as u64,
            name_en,
            name_ar,
            is_default: (m.book_id as u64) == DEFAULT_TAFSIR_BOOK_ID,
            page_index: m.page_index as u64,
            vol: p.vol,
            page_num: p.page_num as u64,
            heading: m.heading,
            text: p.text,
        });
    }

    // Sort: default first (Ibn Kathir), then by book_id.
    entries.sort_by_key(|e| (!e.is_default, e.book_id));

    // 4. Inline English Ibn Kathir from ayah.tafsir_en.
    let ayah_en: Result<Option<AyahTafsirEnRow>, _> = state
        .db
        .query(
            "SELECT tafsir_en FROM ayah \
             WHERE surah_number = $s AND ayah_number = $a LIMIT 1",
        )
        .bind(("s", surah as i64))
        .bind(("a", ayah as i64))
        .await
        .and_then(|mut r| r.take(0));

    let english = InlineEnglishTafsir {
        body: ayah_en.ok().flatten().and_then(|r| r.tafsir_en),
    };

    Json(AllTafsirsResponse {
        surah,
        ayah,
        entries,
        english,
    })
    .into_response()
}

/// Resolve the set of tafsir book_ids that have BOTH a `book_type="tafsir"`
/// row in the DB AND a loaded PageIndex BookTree. Used as the corpus for
/// cross-book Ask AI.
async fn tafsir_book_ids(state: &AppState) -> Vec<u64> {
    let Some(trees) = state.book_trees.as_ref() else {
        return Vec::new();
    };
    let tafsir_rows: Result<Vec<TafsirBookMetaRow>, _> = state
        .db
        .query(r#"SELECT book_id, name_en, name_ar FROM book WHERE book_type = "tafsir""#)
        .await
        .and_then(|mut r| r.take(0));
    let Ok(rows) = tafsir_rows else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|r| r.book_id as u64)
        .filter(|id| trees.contains_key(id))
        .collect()
}

// ── Request shape ─────────────────────────────────────────────────────────
//
// `BookChatRequest` (defined above for /api/books/{id}/chat) is reused here
// because clients on the old path send `{ question }`. For the new
// verse-aware shortcut we add `TafsirAskRequest` with an optional `verse`.
// Axum deserializes whichever body the client sends; missing `verse` falls
// through to the navigation path.

#[derive(Debug, Deserialize)]
pub struct TafsirAskRequest {
    pub question: String,
    /// Optional verse anchor. When present, the handler **skips PageIndex LLM
    /// navigation entirely** and fetches exact tafsir pages for this
    /// `(surah, ayah)` from the precomputed `tafsir_ayah_map` index.
    /// The primary call site is the `/tafsir` page, which always has a
    /// verse context.
    pub verse: Option<TafsirVerseAnchor>,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct TafsirVerseAnchor {
    pub surah: u64,
    pub ayah: u64,
}

// DB row shapes for the verse-aware shortcut.
#[derive(Debug, SurrealValue)]
struct MappingPageRow {
    book_id: i64,
    page_index: i64,
    heading: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct NextPageRow {
    page_index: i64,
}

#[derive(Debug, SurrealValue)]
struct BookPageFullRow {
    page_index: i64,
    text: String,
    vol: String,
    page_num: i64,
}

/// Hard upper bound on pages fetched per book for a single verse. Guards
/// against runaway ranges at end-of-book or when the ayah→page index is
/// missing a "next ayah" boundary. 20 pages × 2–3 books ≈ 60 pages; each
/// page is ~1–3 KB, so worst-case context stays under 180 KB before the
/// 25 KB cap in `build_multi_book_prompt` truncates it.
const MAX_VERSE_WINDOW_PAGES: i64 = 20;

/// POST /api/tafsir/ask
///
/// Cross-tafsir Q&A over every loaded `book_type="tafsir"` book. Streams
/// status events and a synthesized answer via SSE. Accepts either:
///
/// ```json
/// { "question": "..." }
/// { "question": "...", "verse": { "surah": 2, "ayah": 255 } }
/// ```
///
/// ────────────────────────────────────────────────────────────────────────
/// # Two retrieval paths, one synthesis
/// ────────────────────────────────────────────────────────────────────────
///
/// ## (A) Verse-aware shortcut — used when `verse` is present
///
/// Motivation: `/tafsir?verse=S:A` always knows which ayah the user is on.
/// Running an LLM (`navigate_two_phase`) twice per book to rediscover what
/// the `tafsir_ayah_map` index already records is pure waste — and the
/// weaker the nav model, the worse it performs (Phase 2 falls back to
/// "whole chapter" for vague selections, introducing noise).
///
/// Data flow:
/// 1. Read one `tafsir_ayah_map` row per tafsir book for this `(surah,
///    ayah)`. Each row yields the **starting** page_index of that book's
///    commentary on the verse.
/// 2. For each book, resolve the **ayah-boundary window**:
///    `[page_index, next_ayah_page_index)` where `next_ayah_page_index` is
///    the smallest `page_index` strictly greater than the current one across
///    ALL mappings for this book (any surah, any ayah). This correctly
///    captures multi-page commentaries (Tabari's Throne-Verse tafsir spans
///    ~5–15 pages) and self-terminates at the next ayah boundary even when
///    that's the first ayah of the next surah. Capped at
///    `MAX_VERSE_WINDOW_PAGES` to guard against degenerate ranges.
/// 3. Fetch all `book_page` rows in the window (one query per book).
/// 4. Flow directly into the shared synthesis step (below).
///
/// The inline English Ibn Kathir (`ayah.tafsir_en`, a QUL-sourced HTML
/// blob on the `ayah` table) is **deliberately excluded** from the
/// synthesis corpus here. It isn't a paginated turath book — treating
/// it as one confuses the "sources" list with two different storage
/// models. Users still see it in the accordion UI via `ayah_tafsirs_all`.
///
/// Latency: ~5–10 ms per DB query × N tafsir books, then one `chat_json`
/// call **per book in parallel** (each sees only its own pages, so the
/// prompts are 5–15 KB instead of one 30–60 KB combined prompt). On local
/// CPU models per-book calls run ~30–90 s and overlap if
/// `OLLAMA_NUM_PARALLEL > 1`; total wall time ≈ the slowest book. No LLM
/// nav cost on top.
///
/// ## (B) PageIndex nav fan-out — used when `verse` is absent
///
/// For general, non-verse-anchored questions ("compare the scholars on
/// destiny"). Runs [`book_chat::navigate_two_phase`] in parallel against
/// every loaded tafsir `BookTree`, each with a 180 s timeout. Timed-out
/// or empty-result books are dropped and reported as `book_skipped` SSE
/// events, not treated as hard failures. Results feed the same synthesis
/// step.
///
/// This path is expensive on weak models (2 LLM calls × N books, often
/// serialized by Ollama unless `OLLAMA_NUM_PARALLEL` is raised).
///
/// ## Shared synthesis
///
/// Both paths produce `books_sections: Vec<(book_name, Vec<SectionContent>)>`
/// and the English inline body. [`book_chat::build_multi_book_prompt`]
/// assembles them into a single prompt with per-book delimiters and a
/// shared 25 KB cap. The answer is streamed from Ollama as SSE `text`
/// events, terminated by `{"done": true}`.
///
/// ────────────────────────────────────────────────────────────────────────
/// # SSE event contract
/// ────────────────────────────────────────────────────────────────────────
///
/// Shared across both paths (client does not need to know which):
/// - `{"status": "loading_verse", "verse": {"surah": S, "ayah": A}}` (shortcut)
/// - `{"status": "navigating", "books": [book_id, ...]}` (nav)
/// - `{"status": "reading", "books": [{book_id, name_en, sections}, ...]}`
/// - `{"status": "book_skipped", "book_id": X, "reason": "..."}`
/// - `{"sources": [{book_id, book_name_en, book_name_ar, line, title}, ...]}`
///
/// Verse-aware extractive path only:
/// - `{"status": "extracting", "books": N}` (fan-out starting)
/// - `{"status": "book_extracted", "book_id": X, "book_name_en": "...",
///    "entries": K, "dropped": M, "error": null|"..."}` (one per book as it finishes)
/// - `{"result": {"overview": ..., "entries": [...], "dropped": N}}` (merged, terminal)
/// - `{"status": "no_valid_extraction", "available_pages": [...]}` (fallback, terminal)
///
/// Nav fallback path only:
/// - `{"text": "..."}` (repeated; LLM token chunks)
///
/// Terminal for both:
/// - `{"done": true}` | `{"error": "..."}`
pub async fn tafsir_ask(
    State(state): State<AppState>,
    Json(body): Json<TafsirAskRequest>,
) -> Result<Response, StatusCode> {
    let question = body.question.trim().to_string();
    if question.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let verse_anchor = body.verse;

    let ollama = state.ollama.as_ref().ok_or_else(|| {
        tracing::error!("Ollama client not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    // `book_trees` is required for the navigation fallback path. The
    // verse-aware shortcut could in principle run without PageIndex (it
    // only needs `tafsir_ayah_map` + `book_page`), but we still gate on
    // `book_trees` here so both branches share the same prerequisite and
    // there's one consistent "is the tafsir corpus ready?" check upstream.
    let book_trees = state.book_trees.as_ref().ok_or_else(|| {
        tracing::error!("Book trees not loaded (--pageindex-dir not set)");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let ids = tafsir_book_ids(&state).await;
    if ids.is_empty() {
        tracing::warn!("No tafsir BookTrees loaded — /api/tafsir/ask unavailable");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let books: Vec<crate::book_chat::BookTree> = ids
        .iter()
        .filter_map(|id| book_trees.get(id).cloned())
        .collect();

    let ollama = ollama.clone();
    let nav_cache = state.nav_cache.clone();
    let state_for_shortcut = state.clone();

    let sse_stream = async_stream::stream! {
        use futures::StreamExt;
        use std::time::Duration;

        // ─────────────────────────────────────────────────────────────────
        // Phase 1: Retrieval
        //
        // Produces `books_sections: Vec<(book_name, Vec<SectionContent>)>`
        // via one of two paths depending on whether a verse anchor was
        // provided. Both paths emit SSE status/source events along the way
        // so the client can surface progress during multi-second waits.
        // ─────────────────────────────────────────────────────────────────

        let books_sections: Vec<(String, Vec<crate::book_chat::SectionContent>)>;

        if let Some(verse) = verse_anchor {
            // ═══════════════════════════════════════════════════════════
            // PATH (A) — VERSE-AWARE SHORTCUT
            //
            // The client tells us "the user is asking about ayah S:A", so
            // we sidestep the LLM navigation entirely and read the exact
            // tafsir pages from `tafsir_ayah_map` (a precomputed per-ayah
            // index populated at ingestion time with 100% coverage).
            //
            // Per book:
            //   (1) SELECT page_index FROM tafsir_ayah_map WHERE book = B
            //                                              AND surah = S
            //                                              AND ayah  = A
            //     — the starting page of commentary for this ayah.
            //
            //   (2) SELECT MIN(page_index) FROM tafsir_ayah_map
            //         WHERE book = B AND page_index > start_page
            //     — the smallest page_index greater than `start_page` across
            //       all mappings for this book. That's the boundary where
            //       the next ayah's commentary begins, regardless of whether
            //       it's the next ayah in this surah or the first ayah of
            //       the next surah. Capped at start_page + MAX_VERSE_WINDOW_PAGES
            //       to bound worst-case context size.
            //
            //   (3) SELECT * FROM book_page WHERE book_id = B
            //                               AND page_index ∈ [start, end)
            //     — the actual commentary text, one row per page, in order.
            //
            // Why boundary-aware and not a fixed window: in Ibn Kathir one
            // ayah ≈ one page, but in Tabari a single ayah's commentary
            // regularly spans 5–15 pages. A fixed 3-page window would miss
            // most of Tabari's substance on long ayahs; the boundary scan
            // adapts automatically.
            // ═══════════════════════════════════════════════════════════

            yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "status": "loading_verse",
                    "verse": {"surah": verse.surah, "ayah": verse.ayah}
                })
            )));

            // Step 1: one query fetches every tafsir book's starting page
            //         for this ayah. Joined with the loaded BookTree set so
            //         we only consider books that are actually available.
            let mappings: Vec<MappingPageRow> = match state_for_shortcut
                .db
                .query(
                    "SELECT book_id, page_index, heading FROM tafsir_ayah_map \
                     WHERE surah = $s AND ayah = $a",
                )
                .bind(("s", verse.surah as i64))
                .bind(("a", verse.ayah as i64))
                .await
                .and_then(|mut r| r.take(0))
            {
                Ok(rows) => rows,
                Err(e) => {
                    tracing::error!("verse-aware: tafsir_ayah_map lookup failed: {e}");
                    yield Ok(bytes::Bytes::from(format!(
                        "data: {}\n\n",
                        serde_json::json!({"error": format!("Mapping lookup failed: {e}")})
                    )));
                    return;
                }
            };

            // Book metadata map for {book_id → (name_en, name_ar)}. Fetched
            // once and reused — avoids N round-trips when we have N books.
            let book_meta: std::collections::HashMap<i64, (String, String)> = books
                .iter()
                .map(|b| (b.book_id as i64, (b.name_en.clone(), b.name_ar.clone())))
                .collect();

            let loaded_ids: std::collections::HashSet<i64> =
                books.iter().map(|b| b.book_id as i64).collect();

            // (book_id, name_en, name_ar, section_contents, verse_heading)
            type SectionsByBookRow = (
                u64,
                String,
                String,
                Vec<crate::book_chat::SectionContent>,
                Option<String>,
            );
            let mut sections_by_book: Vec<SectionsByBookRow> = Vec::new();
            let mut skipped: Vec<(u64, String)> = Vec::new();

            // Step 2 + 3, per book: find the ayah-boundary, fetch the window.
            for m in &mappings {
                if !loaded_ids.contains(&m.book_id) {
                    // Book has a mapping row but no loaded BookTree. Skip —
                    // not strictly required by the shortcut, but keeps the
                    // corpus consistent with the nav path.
                    continue;
                }
                let Some((name_en, name_ar)) = book_meta.get(&m.book_id).cloned() else {
                    continue;
                };

                // Next-ayah boundary (by page_index, any surah/ayah).
                let boundary: Option<NextPageRow> = state_for_shortcut
                    .db
                    .query(
                        "SELECT page_index FROM tafsir_ayah_map \
                         WHERE book_id = $b AND page_index > $p \
                         ORDER BY page_index ASC LIMIT 1",
                    )
                    .bind(("b", m.book_id))
                    .bind(("p", m.page_index))
                    .await
                    .and_then(|mut r| r.take(0))
                    .ok()
                    .flatten();

                let hard_cap = m.page_index + MAX_VERSE_WINDOW_PAGES;
                let end_page = match boundary {
                    Some(n) => n.page_index.min(hard_cap),
                    // Last ayah in book — scan up to the cap.
                    None => hard_cap,
                };

                if end_page <= m.page_index {
                    skipped.push((m.book_id as u64, "empty page window".into()));
                    continue;
                }

                let pages: Vec<BookPageFullRow> = match state_for_shortcut
                    .db
                    .query(
                        "SELECT page_index, text, vol, page_num FROM book_page \
                         WHERE book_id = $b AND page_index >= $from AND page_index < $to \
                         ORDER BY page_index ASC",
                    )
                    .bind(("b", m.book_id))
                    .bind(("from", m.page_index))
                    .bind(("to", end_page))
                    .await
                    .and_then(|mut r| r.take(0))
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        skipped.push((m.book_id as u64, format!("page fetch failed: {e}")));
                        continue;
                    }
                };

                if pages.is_empty() {
                    skipped.push((m.book_id as u64, "no pages in window".into()));
                    continue;
                }

                // Map each page into a SectionContent so the shared synthesis
                // prompt treats it uniformly with nav results. `line` carries
                // the page_index (useful later for citation linking), `title`
                // holds the human-readable Vol/Page for inline citations.
                let sections: Vec<crate::book_chat::SectionContent> = pages
                    .iter()
                    .map(|p| crate::book_chat::SectionContent {
                        line: p.page_index as u64,
                        title: format!("Vol {} · Page {}", p.vol, p.page_num),
                        text: p.text.clone(),
                    })
                    .collect();

                let _ = name_ar; // reserved for future citation rendering

                sections_by_book.push((
                    m.book_id as u64,
                    name_en,
                    name_ar.clone(),
                    sections,
                    m.heading.clone(),
                ));
            }

            // Note: the inline English `ayah.tafsir_en` is intentionally NOT
            // included in the synthesis context here. It isn't a paginated
            // turath book — it's a QUL-sourced HTML blob on the `ayah` table.
            // Mixing it into the "sources" list would conflate two different
            // storage models (turath page ranges vs. per-ayah HTML). The
            // accordion endpoint (`ayah_tafsirs_all`) still surfaces it
            // separately for the reader UI; the Ask AI corpus stays pure
            // Arabic-turath.

            // Surface skipped books so the UI can inform the user.
            for (bid, reason) in &skipped {
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({"status": "book_skipped", "book_id": bid, "reason": reason})
                )));
            }

            if sections_by_book.is_empty() {
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({
                        "status": "no_relevant_sections",
                        "message": format!(
                            "No tafsir entries found for {}:{}. Is this verse ingested?",
                            verse.surah, verse.ayah
                        )
                    })
                )));
                return;
            }

            // Sources event — one entry per page fetched, grouped by book.
            let sources: Vec<serde_json::Value> = sections_by_book
                .iter()
                .flat_map(|(book_id, name_en, name_ar, sections, _)| {
                    let bid = *book_id;
                    let ne = name_en.clone();
                    let na = name_ar.clone();
                    sections.iter().map(move |s| serde_json::json!({
                        "book_id": bid,
                        "book_name_en": ne,
                        "book_name_ar": na,
                        "line": s.line,
                        "title": s.title,
                    })).collect::<Vec<_>>()
                })
                .collect();

            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({"sources": sources})
            )));

            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "status": "reading",
                    "books": sections_by_book.iter().map(|(bid, ne, _, sections, _)| {
                        serde_json::json!({
                            "book_id": bid,
                            "name_en": ne,
                            "sections": sections.len(),
                        })
                    }).collect::<Vec<_>>()
                })
            )));

            // ───────────────────────────────────────────────────────────
            // Extractive synthesis (verse-aware path) — per-book parallel
            //
            // One `chat_json` call per book, fanned out concurrently. Each
            // call sees only that book's own pages (smaller prompt → much
            // faster completion on local CPU models) and a single-element
            // allow-list containing just its own book_id. Results are
            // validated server-side as they arrive and merged.
            //
            // Why not one big call with every book's pages:
            //   - A single 30-page Arabic prompt on `command-r7b-arabic`
            //     takes 2–5 min; the user sees one "extracting" spinner
            //     the whole time.
            //   - Per-book calls are ~5–15 KB each → 30–60 s each. With
            //     OLLAMA_NUM_PARALLEL > 1, wall time ≈ slowest book.
            //   - Streamed `book_extracted` events give the user visible
            //     progress as each book finishes.
            //
            // Anti-hallucination guarantees are unchanged: every entry's
            // arabic_quote is still verified as a verbatim substring of
            // the page it cites, and the allow-list (per book) rejects
            // any cross-book fabrication.
            // ───────────────────────────────────────────────────────────

            let extract_books: Vec<(u64, String, Vec<crate::book_chat::SectionContent>)> =
                sections_by_book
                    .iter()
                    .map(|(id, name_en, _, sections, _)| {
                        (*id, name_en.clone(), sections.clone())
                    })
                    .collect();

            // Build the available_pages payload once — used by the
            // no_valid_extraction fallback if validation merges to zero.
            let available_pages: Vec<serde_json::Value> = sections_by_book
                .iter()
                .flat_map(|(bid, ne, na, secs, _)| {
                    let b = *bid;
                    let ne = ne.clone();
                    let na = na.clone();
                    secs.iter()
                        .map(move |s| {
                            serde_json::json!({
                                "book_id": b,
                                "book_name_en": ne,
                                "book_name_ar": na,
                                "page_index": s.line,
                                "title": s.title,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect();

            let total_books = extract_books.len();
            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({"status": "extracting", "books": total_books})
            )));

            let verse_tup = (verse.surah, verse.ayah);
            let mut in_flight: futures::stream::FuturesUnordered<_> = extract_books
                .into_iter()
                .map(|(bid, name_en, sections)| {
                    let ollama = ollama.clone();
                    let question = question.clone();
                    async move {
                        let single_book = vec![(bid, name_en.clone(), sections.clone())];
                        let prompt = crate::book_chat::build_tafsir_extract_prompt(
                            verse_tup,
                            &single_book,
                        );
                        let allowed: std::collections::HashSet<u64> =
                            [bid].into_iter().collect();
                        let page_texts: std::collections::HashMap<(u64, u64), String> =
                            sections
                                .iter()
                                .map(|s| ((bid, s.line), s.text.clone()))
                                .collect();

                        // Per-book timeout: a single book's prompt is far
                        // smaller than the previous all-in-one, so 180 s
                        // is generous. Timeouts drop to zero-entry rather
                        // than fail the whole request — other books may
                        // still produce useful extracts.
                        let res = tokio::time::timeout(
                            Duration::from_secs(180),
                            ollama.chat_json(&prompt, &question, None),
                        )
                        .await;

                        let (validated, err): (
                            crate::book_chat::ValidatedExtract,
                            Option<String>,
                        ) = match res {
                            Ok(Ok(raw)) => (
                                crate::book_chat::validate_extract_result(
                                    raw, &allowed, &page_texts,
                                ),
                                None,
                            ),
                            Ok(Err(e)) => {
                                tracing::warn!(
                                    "tafsir extract (book {bid} '{name_en}'): chat_json failed: {e}"
                                );
                                (
                                    crate::book_chat::ValidatedExtract {
                                        overview: None,
                                        entries: Vec::new(),
                                        dropped: 0,
                                    },
                                    Some(format!("chat_json failed: {e}")),
                                )
                            }
                            Err(_) => {
                                tracing::warn!(
                                    "tafsir extract (book {bid} '{name_en}'): timeout after 180s"
                                );
                                (
                                    crate::book_chat::ValidatedExtract {
                                        overview: None,
                                        entries: Vec::new(),
                                        dropped: 0,
                                    },
                                    Some("timeout".to_string()),
                                )
                            }
                        };

                        (bid, name_en, validated, err)
                    }
                })
                .collect();

            let mut merged_overview: Option<String> = None;
            let mut merged_entries: Vec<crate::book_chat::ValidatedEntry> = Vec::new();
            let mut merged_dropped: usize = 0;

            while let Some((bid, name_en, validated, err)) = in_flight.next().await {
                // First non-empty overview wins; the UI shows one framing
                // sentence, not one per book.
                if merged_overview.is_none()
                    && validated.overview.as_deref().is_some_and(|s| !s.trim().is_empty())
                {
                    merged_overview = validated.overview.clone();
                }
                let n_entries = validated.entries.len();
                let n_dropped = validated.dropped;
                merged_entries.extend(validated.entries);
                merged_dropped += n_dropped;

                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({
                        "status": "book_extracted",
                        "book_id": bid,
                        "book_name_en": name_en,
                        "entries": n_entries,
                        "dropped": n_dropped,
                        "error": err,
                    })
                )));
            }

            if merged_entries.is_empty() {
                tracing::warn!(
                    "tafsir_ask extractive: no valid entries across all {} books ({} dropped)",
                    total_books,
                    merged_dropped
                );
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({
                        "status": "no_valid_extraction",
                        "dropped": merged_dropped,
                        "available_pages": available_pages,
                    })
                )));
            } else {
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({"result": {
                        "overview": merged_overview,
                        "entries": merged_entries,
                        "dropped": merged_dropped,
                    }})
                )));
            }

            yield Ok(bytes::Bytes::from("data: {\"done\":true}\n\n"));
            return;
        } else {
            // ═══════════════════════════════════════════════════════════
            // PATH (B) — PAGEINDEX NAVIGATION FAN-OUT
            //
            // No verse anchor → classic agentic retrieval. For each loaded
            // tafsir BookTree, run the two-phase LLM navigation
            // ([`book_chat::navigate_two_phase`]) in parallel, with a
            // per-book 180 s timeout. Books that time out or return empty
            // ranges are dropped and surfaced as `book_skipped` SSE events.
            //
            // This path is expensive on weak models (2 LLM calls × N books,
            // typically serialized by Ollama unless OLLAMA_NUM_PARALLEL > 1),
            // which is why the verse-aware shortcut exists.
            // ═══════════════════════════════════════════════════════════

            let book_ids: Vec<u64> = books.iter().map(|b| b.book_id).collect();
            yield Ok::<_, std::io::Error>(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({"status": "navigating", "books": book_ids})
            )));

            let nav_futures = books.iter().map(|book| {
                let ollama = ollama.clone();
                let nav_cache = nav_cache.clone();
                let question = question.clone();
                let book = book.clone();
                async move {
                    if let Some(cached) = nav_cache.get(book.book_id, &question) {
                        return (book, Ok(cached));
                    }
                    let fut = book_chat::navigate_two_phase(&ollama, &book, &question);
                    // Generous per-book timeout: local Ollama often serializes,
                    // so the second book sees (first_latency + its_own_latency).
                    // The single-book book_chat handler has no timeout — we
                    // cap here only so a stuck book can't pin the whole request.
                    match tokio::time::timeout(Duration::from_secs(180), fut).await {
                        Ok(Ok(r)) => {
                            if !r.is_empty() {
                                nav_cache.put(book.book_id, &question, r.clone());
                            }
                            (book, Ok(r))
                        }
                        Ok(Err(e)) => (book, Err(format!("navigate failed: {e}"))),
                        Err(_) => (book, Err("timeout".to_string())),
                    }
                }
            });
            let nav_results = futures::future::join_all(nav_futures).await;

            let mut per_book: Vec<(
                crate::book_chat::BookTree,
                Vec<crate::book_chat::SectionRange>,
                Vec<crate::book_chat::SectionContent>,
            )> = Vec::new();
            let mut failed: Vec<(u64, String)> = Vec::new();

            for (book, nav) in nav_results {
                match nav {
                    Err(e) => {
                        failed.push((book.book_id, e));
                        continue;
                    }
                    Ok(ranges) if ranges.is_empty() => {
                        failed.push((book.book_id, "no relevant sections".to_string()));
                        continue;
                    }
                    Ok(ranges) => match book_chat::fetch_sections(&book, &ranges) {
                        Ok(sections) if !sections.is_empty() => {
                            per_book.push((book, ranges, sections));
                        }
                        Ok(_) => failed.push((book.book_id, "no sections fetched".to_string())),
                        Err(e) => failed.push((book.book_id, format!("fetch_sections failed: {e}"))),
                    },
                }
            }

            for (bid, reason) in &failed {
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({"status": "book_skipped", "book_id": bid, "reason": reason})
                )));
            }

            if per_book.is_empty() {
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({
                        "status": "no_relevant_sections",
                        "message": "No tafsir returned usable sections for this question."
                    })
                )));
                return;
            }

            let sources: Vec<serde_json::Value> = per_book
                .iter()
                .flat_map(|(book, ranges, _)| {
                    let book_id = book.book_id;
                    let name_en = book.name_en.clone();
                    let name_ar = book.name_ar.clone();
                    book_chat::build_sources(book, ranges)
                        .into_iter()
                        .map(move |s| serde_json::json!({
                            "book_id": book_id,
                            "book_name_en": name_en,
                            "book_name_ar": name_ar,
                            "line": s.line,
                            "title": s.title,
                        }))
                })
                .collect();

            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({"sources": sources})
            )));

            yield Ok(bytes::Bytes::from(format!(
                "data: {}\n\n",
                serde_json::json!({
                    "status": "reading",
                    "books": per_book.iter().map(|(b, _, s)| serde_json::json!({
                        "book_id": b.book_id,
                        "name_en": b.name_en,
                        "sections": s.len(),
                    })).collect::<Vec<_>>()
                })
            )));

            books_sections = per_book
                .into_iter()
                .map(|(b, _, s)| (b.name_en, s))
                .collect();
        }

        // ─────────────────────────────────────────────────────────────────
        // Phase 2: Synthesis (shared across both paths)
        //
        // `build_multi_book_prompt` concatenates each book's sections with
        // `=== Book Name ===` delimiters and applies a 25 KB shared cap.
        // The synthesis system prompt instructs the model to attribute every
        // claim to its book, flag agreements/disagreements, and answer in
        // the user's language.
        // ─────────────────────────────────────────────────────────────────

        let prompt = book_chat::build_multi_book_prompt(&books_sections);

        let byte_stream = match ollama.chat_stream(&prompt, &question, None).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("tafsir_ask answer stream failed: {e}");
                yield Ok(bytes::Bytes::from(format!(
                    "data: {}\n\n",
                    serde_json::json!({"error": format!("Answer generation failed: {e}")})
                )));
                return;
            }
        };

        let mut byte_stream = std::pin::pin!(byte_stream);
        while let Some(chunk) = byte_stream.next().await {
            match chunk {
                Ok(raw) => {
                    let mut sse = String::new();
                    for line in raw.split(|&b| b == b'\n') {
                        if line.is_empty() { continue; }
                        if let Ok(parsed) = serde_json::from_slice::<ChatChunk>(line) {
                            if let Some(msg) = parsed.message
                                && !msg.content.is_empty()
                            {
                                sse.push_str(&format!(
                                    "data: {}\n\n",
                                    serde_json::to_string(
                                        &serde_json::json!({"text": msg.content})
                                    ).unwrap()
                                ));
                            }
                            if parsed.done {
                                sse.push_str("data: {\"done\":true}\n\n");
                            }
                        }
                    }
                    if !sse.is_empty() {
                        yield Ok(bytes::Bytes::from(sse));
                    }
                }
                Err(e) => {
                    yield Ok(bytes::Bytes::from(format!(
                        "data: {}\n\n",
                        serde_json::json!({"error": e.to_string()})
                    )));
                }
            }
        }
    };

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(Body::from_stream(sse_stream))
        .unwrap())
}
