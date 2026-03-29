use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::{HadithSearchResult, NarratorSearchResult};

/// Result from search::rrf() — contains id and fused score.
#[derive(Debug, SurrealValue)]
struct RrfResult {
    id: Option<surrealdb::types::RecordId>,
    rrf_score: Option<f64>,
}

/// Bilingual text search for hadiths (searches both Arabic and English).
pub async fn search_hadiths_text(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<HadithSearchResult>> {
    let mut response = db
        .query(
            "SELECT *, 0.0 AS score FROM hadith \
             WHERE string::lowercase(text_en) CONTAINS string::lowercase($query) \
                OR text_ar CONTAINS $query \
             LIMIT $limit START $offset",
        )
        .bind(("query", query.to_string()))
        .bind(("limit", limit))
        .bind(("offset", offset))
        .await?;
    let results: Vec<HadithSearchResult> = response.take(0)?;
    Ok(results)
}

/// Semantic search for hadiths using vector similarity.
pub async fn search_hadiths_semantic(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<HadithSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    let sql = format!(
        "SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score FROM hadith \
         WHERE embedding <|{limit},40|> $query_vec \
         ORDER BY score DESC"
    );
    let mut response = db.query(&sql).bind(("query_vec", query_vec)).await?;
    let results: Vec<HadithSearchResult> = response.take(0)?;
    Ok(results)
}

/// Hybrid search combining BM25 full-text and vector similarity via Reciprocal Rank Fusion.
///
/// Runs vector search and BM25 full-text search as separate queries, then fuses
/// results using `search::rrf()` (Reciprocal Rank Fusion, k=60). Finally fetches
/// full hadith records for the top-ranked IDs.
pub async fn search_hadiths_hybrid(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
    _offset: usize,
) -> Result<Vec<HadithSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    // 1. Vector search  2. BM25 full-text search  3. Fuse with RRF
    let sql = format!(
        "LET $vs = SELECT id, vector::distance::knn() AS distance \
             FROM hadith WHERE embedding <|{limit},40|> $query_vec; \
         LET $ft = SELECT id, search::score(1) AS ft_score \
             FROM hadith WHERE text_en @1@ $query \
             ORDER BY ft_score DESC LIMIT {limit}; \
         RETURN search::rrf([$vs, $ft], {limit}, 60)"
    );

    let mut response = db
        .query(&sql)
        .bind(("query_vec", query_vec))
        .bind(("query", query.to_string()))
        .await?;

    // search::rrf returns Vec<{id, rrf_score}> — extract IDs and scores
    let fused: Vec<RrfResult> = response.take(2)?;
    if fused.is_empty() {
        return Ok(vec![]);
    }

    // Fetch full hadith records for the fused IDs
    let ids: Vec<surrealdb::types::RecordId> = fused.iter().filter_map(|r| r.id.clone()).collect();

    let mut fetch_response = db
        .query("SELECT *, 0.0 AS score FROM hadith WHERE id IN $ids")
        .bind(("ids", ids))
        .await?;
    let mut hadiths: Vec<HadithSearchResult> = fetch_response.take(0)?;

    // Attach RRF scores and sort by score descending
    for h in &mut hadiths {
        if let Some(ref hid) = h.id {
            h.score = fused
                .iter()
                .find(|r| r.id.as_ref() == Some(hid))
                .and_then(|r| r.rrf_score);
        }
    }
    hadiths.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(hadiths)
}

/// Bilingual search for narrators (searches both Arabic and English names).
pub async fn search_narrators(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<NarratorSearchResult>> {
    let mut response = db
        .query(
            "SELECT *, \
             count(->narrates->hadith) AS hadith_count \
             FROM narrator \
             WHERE string::lowercase(name_en) CONTAINS string::lowercase($query) \
                OR name_ar CONTAINS $query \
             LIMIT $limit START $offset",
        )
        .bind(("query", query.to_string()))
        .bind(("limit", limit))
        .bind(("offset", offset))
        .await?;
    let results: Vec<NarratorSearchResult> = response.take(0)?;
    Ok(results)
}
