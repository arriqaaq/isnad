use std::collections::HashMap;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;

use super::models::AyahSearchResult;

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

/// Bilingual text search for ayahs using BM25 full-text indexes.
///
/// Runs English and Arabic BM25 searches separately and fuses via RRF,
/// since `search::score()` only returns scores for a single MATCHES clause.
pub async fn search_ayahs_text(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    _offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    let normalized_ar = crate::quran::ingest::strip_arabic_diacritics(query);
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);
    let safe_ar = escape_surql(&normalized_ar);

    let sql = format!(
        "LET $en = SELECT id, search::score(1) AS ft_score \
             FROM ayah WHERE text_en @1@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         LET $ar = SELECT id, search::score(2) AS ft_score \
             FROM ayah WHERE text_ar_simple @2@ '{safe_ar}' \
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
        .query("SELECT *, 0.0 AS score FROM ayah WHERE id IN $ids")
        .bind(("ids", ids))
        .await?;
    let mut results: Vec<AyahSearchResult> = fetch_resp.take(0)?;

    // Attach RRF scores via HashMap
    #[allow(clippy::mutable_key_type)]
    let score_map: HashMap<_, _> = fused
        .into_iter()
        .filter_map(|r| Some((r.id?, r.rrf_score?)))
        .collect();
    for a in &mut results {
        if let Some(ref aid) = a.id {
            a.score = score_map.get(aid).copied();
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

/// Semantic search for ayahs using vector similarity.
pub async fn search_ayahs_semantic(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<AyahSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    let sql = format!(
        "SELECT *, vector::similarity::cosine(embedding, $query_vec) AS score FROM ayah \
         WHERE embedding <|{limit},80|> $query_vec \
         ORDER BY score DESC"
    );
    let mut response = db.query(&sql).bind(("query_vec", query_vec)).await?;
    let results: Vec<AyahSearchResult> = response.take(0)?;
    Ok(results)
}

/// Hybrid search combining BM25 full-text and vector similarity via Reciprocal Rank Fusion.
pub async fn search_ayahs_hybrid(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
    _offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    let query_vec = embedder.embed_single(query)?;
    let normalized_ar = crate::quran::ingest::strip_arabic_diacritics(query);
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);
    let safe_ar = escape_surql(&normalized_ar);

    let sql = format!(
        "LET $vs = SELECT id, vector::distance::knn() AS distance \
             FROM ayah WHERE embedding <|{limit},80|> $query_vec; \
         LET $ft = SELECT id, search::score(1) AS ft_score \
             FROM ayah WHERE text_en @1@ '{safe_q}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         LET $ar = SELECT id, search::score(2) AS ft_score \
             FROM ayah WHERE text_ar_simple @2@ '{safe_ar}' \
             ORDER BY ft_score DESC LIMIT {limit}; \
         RETURN search::rrf([$vs, $ft, $ar], {limit}, 60)"
    );

    let mut response = db.query(&sql).bind(("query_vec", query_vec)).await?;

    let fused: Vec<RrfResult> = response.take(3)?;
    if fused.is_empty() {
        return Ok(vec![]);
    }

    let ids: Vec<surrealdb::types::RecordId> = fused.iter().filter_map(|r| r.id.clone()).collect();

    let mut fetch_response = db
        .query("SELECT *, 0.0 AS score FROM ayah WHERE id IN $ids")
        .bind(("ids", ids))
        .await?;
    let mut ayahs: Vec<AyahSearchResult> = fetch_response.take(0)?;

    // Attach RRF scores via HashMap
    #[allow(clippy::mutable_key_type)]
    let score_map: HashMap<_, _> = fused
        .into_iter()
        .filter_map(|r| Some((r.id?, r.rrf_score?)))
        .collect();
    for a in &mut ayahs {
        if let Some(ref aid) = a.id {
            a.score = score_map.get(aid).copied();
        }
    }
    ayahs.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(ayahs)
}

/// Search within Tafsir Ibn Kathir text using BM25 full-text search.
pub async fn search_ayahs_tafsir(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);
    let sql = format!(
        "SELECT *, search::score(1) AS score FROM ayah \
         WHERE tafsir_en @1@ '{safe_q}' \
         ORDER BY score DESC \
         LIMIT {limit} START {offset}"
    );
    let mut response = db.query(&sql).await?;
    let results: Vec<AyahSearchResult> = response.take(0)?;
    Ok(results)
}
