use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use surrealdb::types::SurrealValue;

use super::AppState;

// ── Response types ──

#[derive(Debug, Serialize)]
pub struct TurathBookSummary {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
}

#[derive(Debug, Serialize)]
pub struct TurathBookDetail {
    pub book_id: u64,
    pub name_ar: String,
    pub name_en: String,
    pub author_ar: String,
    pub total_pages: u64,
    pub headings: Vec<TurathHeading>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TurathHeading {
    pub title: String,
    pub level: u32,
    pub page_index: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TurathPage {
    pub page_index: u64,
    pub text: String,
    pub vol: String,
    pub page_num: u64,
}

#[derive(Debug, Serialize)]
pub struct TurathPagesResponse {
    pub pages: Vec<TurathPage>,
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
    pub sharh_book_id: u64,
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

// ── DB row types ──

#[derive(Debug, SurrealValue)]
struct SharhRow {
    hadith_number: i64,
    sharh_book_id: i64,
    page_index: i64,
}

#[derive(Debug, SurrealValue)]
struct NarratorBookRow {
    turath_book_id: i64,
    page_index: i64,
    entry_num: Option<i64>,
    book_name: String,
}

#[derive(Debug, Serialize)]
pub struct NarratorBookRef {
    pub turath_book_id: u64,
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

pub async fn list_books(State(state): State<AppState>) -> impl IntoResponse {
    let result: Result<Vec<BookRow>, _> = state
        .db
        .query("SELECT book_id, name_ar, name_en, author_ar, total_pages FROM turath_book")
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(books) => {
            let summaries: Vec<TurathBookSummary> = books
                .into_iter()
                .map(|b| TurathBookSummary {
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
            tracing::error!("Failed to list turath books: {e}");
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
        .query("SELECT * FROM turath_book WHERE book_id = $bid LIMIT 1")
        .bind(("bid", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(Some(book)) => {
            let headings: Vec<TurathHeading> = book
                .headings
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();

            Json(TurathBookDetail {
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
            tracing::error!("Failed to get turath book {book_id}: {e}");
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
            "SELECT page_index, text, vol, page_num FROM turath_page \
             WHERE book_id = $bid AND page_index >= $start AND page_index < $end \
             ORDER BY page_index ASC",
        )
        .bind(("bid", book_id as i64))
        .bind(("start", start as i64))
        .bind(("end", (start + size) as i64))
        .await;

    let total_result: Result<Option<CountResult>, _> = state
        .db
        .query("SELECT count() AS c FROM turath_page WHERE book_id = $bid GROUP ALL")
        .bind(("bid", book_id as i64))
        .await
        .and_then(|mut r| r.take(0));

    let total = total_result.ok().flatten().map(|c| c.c as u64).unwrap_or(0);

    match result {
        Ok(mut res) => {
            let pages: Vec<PageRow> = res.take(0).unwrap_or_default();
            let response = TurathPagesResponse {
                pages: pages
                    .into_iter()
                    .map(|p| TurathPage {
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

/// Batch lookup: hadith numbers → sharh page references.
/// GET /api/hadiths/sharh-pages?book=1&numbers=1,2,3,4,5
pub async fn hadith_sharh_pages(
    State(state): State<AppState>,
    Query(params): Query<SharhBatchParams>,
) -> impl IntoResponse {
    let book_id = params.book.unwrap_or(1); // default to Bukhari

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
        "SELECT hadith_number, sharh_book_id, page_index FROM hadith_sharh_map \
         WHERE book_id = {book_id} AND hadith_number IN [{nums_str}]"
    );

    let result: Result<Vec<SharhRow>, _> = state.db.query(&sql).await.and_then(|mut r| r.take(0));

    // Get sharh book name for display
    let book_name = match state
        .db
        .query("SELECT name_en FROM turath_book WHERE book_id IN (SELECT VALUE sharh_book_id FROM hadith_sharh_map WHERE book_id = $bid LIMIT 1) LIMIT 1")
        .bind(("bid", book_id as i64))
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
                        sharh_book_id: row.sharh_book_id as u64,
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
    let result: Result<Vec<NarratorBookRow>, _> = state
        .db
        .query(
            "SELECT turath_book_id, page_index, entry_num, book_name \
             FROM narrator_book_map WHERE narrator_id = $nid",
        )
        .bind(("nid", narrator_id.clone()))
        .await
        .and_then(|mut r| r.take(0));

    match result {
        Ok(rows) => {
            let refs: Vec<NarratorBookRef> = rows
                .into_iter()
                .map(|r| NarratorBookRef {
                    turath_book_id: r.turath_book_id as u64,
                    page_index: r.page_index as u64,
                    entry_num: r.entry_num.map(|n| n as u64),
                    book_name: r.book_name,
                })
                .collect();
            Json(refs).into_response()
        }
        Err(e) => {
            tracing::error!("Failed to get narrator books for {narrator_id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to get narrator books",
            )
                .into_response()
        }
    }
}
