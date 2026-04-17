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
pub struct BooksConfigResponse {
    pub books: Vec<BookConfigEntry>,
    pub tafsir_book_id: Option<u64>,
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
            let mut tafsir_book_id: Option<u64> = None;
            let books: Vec<BookConfigEntry> = rows
                .into_iter()
                .map(|b| {
                    let bid = b.book_id as u64;
                    let bt = b.book_type.as_deref();
                    if bt == Some("tafsir") {
                        tafsir_book_id = Some(bid);
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

            Json(BooksConfigResponse {
                books,
                tafsir_book_id,
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

pub async fn surah_tafsir_pages(
    State(state): State<AppState>,
    Path(surah_number): Path<u64>,
) -> impl IntoResponse {
    let result: Result<Vec<MappingRow>, _> = state
        .db
        .query(
            "SELECT ayah, page_index, heading FROM tafsir_ayah_map \
             WHERE surah = $surah ORDER BY ayah ASC",
        )
        .bind(("surah", surah_number as i64))
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
            tracing::error!("Failed to get tafsir pages for surah {surah_number}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get tafsir pages",
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
                    nav_cache.put(book.book_id, &question, r.clone());
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
