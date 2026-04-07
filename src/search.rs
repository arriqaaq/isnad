use std::collections::HashMap;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::{HADITH_SEARCH_FIELDS, HadithSearchResult, NarratorSearchResult};

/// Result from search::rrf() — contains id and fused score.
#[derive(Debug, SurrealValue)]
struct RrfResult {
    id: Option<surrealdb::types::RecordId>,
    rrf_score: Option<f64>,
}

/// Escape a user query for safe inlining in SurrealQL string literals.
///
/// WORKAROUND: SurrealDB BM25 `@N@` operator silently returns 0 results when
/// the right-hand operand is a bind variable ($param). We must inline the search
/// text as a string literal. Remove this and use bind variables once fixed.
/// Tracking: https://github.com/surrealdb/surrealdb/issues/7199
/// Grep for "surrealdb#7199" to find all affected call sites.
fn escape_surql(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

/// Bilingual text search for hadiths using BM25 full-text indexes.
///
/// Runs English and Arabic BM25 searches separately and fuses via RRF,
/// since `search::score()` only returns scores for a single MATCHES clause.
pub async fn search_hadiths_text(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    _offset: usize,
) -> Result<Vec<HadithSearchResult>> {
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);
    let sql = format!(
        "LET $en = SELECT id, search::score(1) AS ft_score \
             FROM hadith WHERE text_en @1@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         LET $ar = SELECT id, search::score(2) AS ft_score \
             FROM hadith WHERE text_ar @2@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         RETURN search::rrf([$en, $ar], {limit}, 60)"
    );

    let mut response = db.query(&sql).await?;
    let fused: Vec<RrfResult> = response.take(2)?;
    if fused.is_empty() {
        return Ok(vec![]);
    }

    let ids: Vec<surrealdb::types::RecordId> = fused.iter().filter_map(|r| r.id.clone()).collect();

    let mut fetch_resp = db
        .query(format!(
            "SELECT {HADITH_SEARCH_FIELDS}, 0.0 AS score FROM hadith WHERE id IN $ids"
        ))
        .bind(("ids", ids))
        .await?;
    let mut results: Vec<HadithSearchResult> = fetch_resp.take(0)?;

    // Attach RRF scores via HashMap (O(n) instead of O(n*m))
    #[allow(clippy::mutable_key_type)]
    let score_map: HashMap<_, _> = fused
        .into_iter()
        .filter_map(|r| Some((r.id?, r.rrf_score?)))
        .collect();
    for h in &mut results {
        if let Some(ref hid) = h.id {
            h.score = score_map.get(hid).copied();
        }
    }
    results.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(results)
}

/// Semantic search for hadiths using vector similarity.
///
/// Single-step KNN with cosine similarity scoring. Uses explicit field selection
/// (not `SELECT *`) to avoid pulling the embedding column through the result set.
pub async fn search_hadiths_semantic(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<HadithSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    let sql = format!(
        "SELECT {HADITH_SEARCH_FIELDS}, vector::similarity::cosine(embedding, $query_vec) AS score \
         FROM hadith WHERE embedding <|{limit},80|> $query_vec \
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
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);

    // 1. Vector search  2. BM25 full-text search (en + ar)  3. Fuse with RRF
    let sql = format!(
        "LET $vs = SELECT id, vector::distance::knn() AS distance \
             FROM hadith WHERE embedding <|{limit},80|> $query_vec; \
         LET $ft_en = SELECT id, search::score(1) AS ft_score \
             FROM hadith WHERE text_en @1@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         LET $ft_ar = SELECT id, search::score(2) AS ft_score \
             FROM hadith WHERE text_ar @2@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         RETURN search::rrf([$vs, $ft_en, $ft_ar], {limit}, 60)"
    );

    let mut response = db.query(&sql).bind(("query_vec", query_vec)).await?;

    // search::rrf returns Vec<{id, rrf_score}> — extract IDs and scores
    let fused: Vec<RrfResult> = response.take(3)?;
    if fused.is_empty() {
        return Ok(vec![]);
    }

    // Fetch full hadith records for the fused IDs
    let ids: Vec<surrealdb::types::RecordId> = fused.iter().filter_map(|r| r.id.clone()).collect();

    let mut fetch_response = db
        .query(format!(
            "SELECT {HADITH_SEARCH_FIELDS}, 0.0 AS score FROM hadith WHERE id IN $ids"
        ))
        .bind(("ids", ids))
        .await?;
    let mut hadiths: Vec<HadithSearchResult> = fetch_response.take(0)?;

    // Attach RRF scores via HashMap (O(n) instead of O(n*m))
    #[allow(clippy::mutable_key_type)]
    let score_map: HashMap<_, _> = fused
        .into_iter()
        .filter_map(|r| Some((r.id?, r.rrf_score?)))
        .collect();
    for h in &mut hadiths {
        if let Some(ref hid) = h.id {
            h.score = score_map.get(hid).copied();
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
/// Uses CONTAINS (not BM25) since narrator table is small (~1000 rows).
/// hadith_count is read from pre-computed field (see db::backfill_narrator_hadith_counts).
pub async fn search_narrators(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<NarratorSearchResult>> {
    let mut response = db
        .query(
            "SELECT * \
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
