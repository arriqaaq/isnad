use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use tracing;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::ApiHadithSearchResult;
use crate::quran::models::ApiAyahSearchResult;

/// A single item in the unified search results — either a Quran ayah or a Hadith.
#[derive(Debug, Serialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum UnifiedSearchItem {
    Quran {
        #[serde(flatten)]
        ayah: ApiAyahSearchResult,
        unified_score: f64,
    },
    Hadith {
        #[serde(flatten)]
        hadith: ApiHadithSearchResult,
        unified_score: f64,
    },
}

/// Response from the unified search endpoint.
#[derive(Debug, Serialize)]
pub struct UnifiedSearchResponse {
    pub query: String,
    pub search_type: String,
    pub results: Vec<UnifiedSearchItem>,
    pub quran_count: usize,
    pub hadith_count: usize,
    pub page: usize,
    pub has_more: bool,
}

/// Reciprocal Rank Fusion score: 1 / (k + rank), with k = 60.
fn rrf_score(rank: usize) -> f64 {
    1.0 / (60.0 + rank as f64)
}

/// Search both Quran ayahs and Hadiths, then interleave via cross-source RRF with pagination.
pub async fn search_unified(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
    page: usize,
) -> Result<UnifiedSearchResponse> {
    // Fetch enough from each source to fill this page + detect has_more.
    // We need (page * limit) items total from the merged list, plus 1 to check has_more.
    let fetch_per_source = page * limit + 1;

    tracing::debug!(
        "unified search: query={query:?} limit={limit} page={page} fetch_per_source={fetch_per_source}"
    );

    // Run searches sequentially to avoid doubling HNSW stack usage on one worker thread
    let hadiths = crate::search::search_hadiths_hybrid(db, embedder, query, fetch_per_source, 0)
        .await
        .unwrap_or_default();

    let ayahs = crate::quran::search::search_ayahs_hybrid(db, embedder, query, fetch_per_source, 0)
        .await
        .unwrap_or_default();

    let quran_count = ayahs.len();
    let hadith_count = hadiths.len();

    // Cross-source RRF: assign ranks within each list, compute unified scores, merge
    let mut items: Vec<UnifiedSearchItem> = Vec::with_capacity(quran_count + hadith_count);

    for (rank, ayah) in ayahs.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Quran {
            ayah: ApiAyahSearchResult::from(ayah),
            unified_score: rrf_score(rank + 1),
        });
    }

    for (rank, hadith) in hadiths.into_iter().enumerate() {
        items.push(UnifiedSearchItem::Hadith {
            hadith: ApiHadithSearchResult::from(hadith),
            unified_score: rrf_score(rank + 1),
        });
    }

    // Sort by unified_score descending (interleaves the two sources)
    items.sort_by(|a, b| {
        let sa = match a {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        let sb = match b {
            UnifiedSearchItem::Quran { unified_score, .. } => *unified_score,
            UnifiedSearchItem::Hadith { unified_score, .. } => *unified_score,
        };
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Paginate: skip to the right page, take limit items
    let offset = (page - 1) * limit;
    let has_more = items.len() > offset + limit;
    let results: Vec<UnifiedSearchItem> = items.into_iter().skip(offset).take(limit).collect();

    Ok(UnifiedSearchResponse {
        query: query.to_string(),
        search_type: "hybrid".to_string(),
        results,
        quran_count,
        hadith_count,
        page,
        has_more,
    })
}
