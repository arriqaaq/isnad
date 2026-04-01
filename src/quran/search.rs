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

/// Bilingual text search for ayahs (searches both Arabic and English).
pub async fn search_ayahs_text(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    let normalized_ar = crate::quran::ingest::strip_arabic_diacritics(query);
    let mut response = db
        .query(
            "SELECT *, 0.0 AS score FROM ayah \
             WHERE string::lowercase(text_en) CONTAINS string::lowercase($query) \
                OR text_ar_simple CONTAINS $normalized_ar \
             LIMIT $limit START $offset",
        )
        .bind(("query", query.to_string()))
        .bind(("normalized_ar", normalized_ar))
        .bind(("limit", limit))
        .bind(("offset", offset))
        .await?;
    let results: Vec<AyahSearchResult> = response.take(0)?;
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
         WHERE embedding <|{limit},40|> $query_vec \
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

    let sql = format!(
        "LET $vs = SELECT id, vector::distance::knn() AS distance \
             FROM ayah WHERE embedding <|{limit},40|> $query_vec; \
         LET $ft = SELECT id, search::score(1) AS ft_score \
             FROM ayah WHERE text_en @1@ $query \
             ORDER BY ft_score DESC LIMIT {limit}; \
         LET $ar = SELECT id, search::score(2) AS ft_score \
             FROM ayah WHERE text_ar_simple @2@ $normalized_ar \
             ORDER BY ft_score DESC LIMIT {limit}; \
         RETURN search::rrf([$vs, $ft, $ar], {limit}, 60)"
    );

    let mut response = db
        .query(&sql)
        .bind(("query_vec", query_vec))
        .bind(("query", query.to_string()))
        .bind(("normalized_ar", normalized_ar))
        .await?;

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

    for a in &mut ayahs {
        if let Some(ref aid) = a.id {
            a.score = fused
                .iter()
                .find(|r| r.id.as_ref() == Some(aid))
                .and_then(|r| r.rrf_score);
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
    let mut response = db
        .query(
            "SELECT *, search::score(1) AS score FROM ayah \
             WHERE tafsir_en @1@ $query \
             ORDER BY score DESC \
             LIMIT $limit START $offset",
        )
        .bind(("query", query.to_string()))
        .bind(("limit", limit))
        .bind(("offset", offset))
        .await?;
    let results: Vec<AyahSearchResult> = response.take(0)?;
    Ok(results)
}
