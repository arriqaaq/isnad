use std::collections::HashMap;

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;

use super::models::{AYAH_SEARCH_FIELDS, AyahSearchResult};

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
        .query(format!(
            "SELECT {AYAH_SEARCH_FIELDS}, 0.0 AS score FROM ayah WHERE id IN $ids"
        ))
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
///
/// Single-step KNN with cosine similarity scoring. Uses explicit field selection
/// (not `SELECT *`) to avoid pulling the embedding column through the result set.
pub async fn search_ayahs_semantic(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<AyahSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    let sql = format!(
        "SELECT {AYAH_SEARCH_FIELDS}, vector::similarity::cosine(embedding, $query_vec) AS score \
         FROM ayah WHERE embedding <|{limit},80|> $query_vec \
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
        .query(format!(
            "SELECT {AYAH_SEARCH_FIELDS}, 0.0 AS score FROM ayah WHERE id IN $ids"
        ))
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

/// BM25 full-text search within Tafsir Ibn Kathir text.
async fn search_ayahs_tafsir_bm25(
    db: &Surreal<Db>,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    // TODO(surrealdb#7199): use bind variables instead of inline literals
    let safe_q = escape_surql(query);
    let sql = format!(
        "SELECT {AYAH_SEARCH_FIELDS}, search::score(1) AS score FROM ayah \
         WHERE tafsir_en @1@ '{safe_q}' \
         ORDER BY score DESC \
         LIMIT {limit} START {offset}"
    );
    let mut response = db.query(&sql).await?;
    let results: Vec<AyahSearchResult> = response.take(0)?;
    Ok(results)
}

/// Intermediate row returned by the tafsir chunk HNSW vector search.
#[derive(Debug, SurrealValue)]
struct TafsirChunkHit {
    ayah_id: Option<surrealdb::types::RecordId>,
    score: Option<f64>,
}

/// Semantic search over tafsir_chunk embeddings, deduplicated to ayah level.
async fn search_ayahs_tafsir_semantic(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
) -> Result<Vec<AyahSearchResult>> {
    let query_vec = embedder.embed_single(query)?;

    // Fetch extra chunks to account for multiple chunks mapping to the same ayah
    let chunk_limit = limit * 3;
    let sql = format!(
        "SELECT ayah_id, vector::similarity::cosine(embedding, $query_vec) AS score \
         FROM tafsir_chunk \
         WHERE embedding <|{chunk_limit},80|> $query_vec \
         ORDER BY score DESC"
    );
    let mut response = db.query(&sql).bind(("query_vec", query_vec)).await?;
    let hits: Vec<TafsirChunkHit> = response.take(0)?;

    if hits.is_empty() {
        return Ok(vec![]);
    }

    // Deduplicate: keep the MAX score per ayah_id
    let mut best: HashMap<String, (surrealdb::types::RecordId, f64)> = HashMap::new();
    for h in &hits {
        if let (Some(aid), Some(sc)) = (&h.ayah_id, h.score) {
            let key = crate::models::record_id_string(aid);
            best.entry(key)
                .and_modify(|(_, prev)| {
                    if sc > *prev {
                        *prev = sc;
                    }
                })
                .or_insert_with(|| (aid.clone(), sc));
        }
    }

    // Sort by score descending, take top `limit`
    let mut sorted: Vec<(String, surrealdb::types::RecordId, f64)> = best
        .into_iter()
        .map(|(k, (rid, sc))| (k, rid, sc))
        .collect();
    sorted.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    sorted.truncate(limit);

    let ids: Vec<surrealdb::types::RecordId> =
        sorted.iter().map(|(_, rid, _)| rid.clone()).collect();
    let score_map: HashMap<String, f64> =
        sorted.iter().map(|(k, _, sc)| (k.clone(), *sc)).collect();

    let mut fetch_resp = db
        .query(format!(
            "SELECT {AYAH_SEARCH_FIELDS}, 0.0 AS score FROM ayah WHERE id IN $ids"
        ))
        .bind(("ids", ids))
        .await?;
    let mut results: Vec<AyahSearchResult> = fetch_resp.take(0)?;

    // Re-attach semantic scores
    for a in &mut results {
        if let Some(ref aid) = a.id {
            let key = crate::models::record_id_string(aid);
            a.score = score_map.get(&key).copied();
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

/// Reciprocal Rank Fusion over multiple ranked lists.
///
/// Each list contains `(id_string, score)` pairs in rank order.
/// Returns fused `(id_string, rrf_score)` sorted descending, truncated to `limit`.
fn rrf_fuse(lists: &[&[(String, f64)]], k: f64, limit: usize) -> Vec<(String, f64)> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    for list in lists {
        for (rank, (id, _score)) in list.iter().enumerate() {
            *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (k + (rank as f64 + 1.0));
        }
    }
    let mut fused: Vec<(String, f64)> = scores.into_iter().collect();
    fused.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    fused.truncate(limit);
    fused
}

/// Hybrid tafsir search combining BM25 full-text and vector similarity via RRF.
pub async fn search_ayahs_tafsir(
    db: &Surreal<Db>,
    embedder: &Embedder,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<Vec<AyahSearchResult>> {
    // Run BM25 and semantic searches
    let bm25_results = search_ayahs_tafsir_bm25(db, query, limit, offset).await?;
    let semantic_results = search_ayahs_tafsir_semantic(db, embedder, query, limit).await?;

    // Convert to (id_string, score) lists for RRF
    let bm25_pairs: Vec<(String, f64)> = bm25_results
        .iter()
        .filter_map(|a| {
            let id = crate::models::record_id_string(a.id.as_ref()?);
            Some((id, a.score.unwrap_or(0.0)))
        })
        .collect();
    let semantic_pairs: Vec<(String, f64)> = semantic_results
        .iter()
        .filter_map(|a| {
            let id = crate::models::record_id_string(a.id.as_ref()?);
            Some((id, a.score.unwrap_or(0.0)))
        })
        .collect();

    let fused = rrf_fuse(&[&bm25_pairs, &semantic_pairs], 60.0, limit);

    if fused.is_empty() {
        return Ok(vec![]);
    }

    // Build RecordIds from the fused id strings (format: "ayah:KEY")
    let ids: Vec<surrealdb::types::RecordId> = fused
        .iter()
        .filter_map(|(id_str, _)| {
            let (table, key) = id_str.split_once(':')?;
            Some(surrealdb::types::RecordId::new(table, key))
        })
        .collect();

    #[allow(clippy::mutable_key_type)]
    let score_map: HashMap<String, f64> = fused.into_iter().collect();

    let mut fetch_resp = db
        .query(format!(
            "SELECT {AYAH_SEARCH_FIELDS}, 0.0 AS score FROM ayah WHERE id IN $ids"
        ))
        .bind(("ids", ids))
        .await?;
    let mut results: Vec<AyahSearchResult> = fetch_resp.take(0)?;

    // Attach fused scores
    for a in &mut results {
        if let Some(ref aid) = a.id {
            let key = crate::models::record_id_string(aid);
            a.score = score_map.get(&key).copied();
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
