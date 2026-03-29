use anyhow::Result;
use surrealdb::Surreal;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::{HadithSearchResult, NarratorSearchResult};

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
