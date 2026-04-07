use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;

use crate::models::{ApiHadith, ApiHadithSearchResult};
use crate::quran::models::{
    ApiAyah, ApiAyahSearchResult, ApiPhraseWithAyahs, ApiQuranWord, ApiReciter, ApiSimilarAyah,
    ApiSurah, Ayah, AyahSimilarResponse, QuranPhrase, QuranSearchResponse, QuranStatsResponse,
    QuranWord, Reciter, RootSearchResponse, Surah, SurahDetailResponse,
};
use crate::rag::ChatChunk;

use super::AppState;

// ── Query parameter types ──

#[derive(Deserialize)]
pub struct AyahHadithParams {
    pub include_semantic: Option<bool>,
    pub semantic_limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct QuranSearchParams {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct QuranBrowseParams {
    pub surah: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct QuranAskRequest {
    pub question: String,
    pub model: Option<String>,
}

// ── Handlers ──

pub async fn quran_stats(State(state): State<AppState>) -> impl IntoResponse {
    let mut res = state
        .db
        .query(
            "SELECT count() FROM surah GROUP ALL; \
             SELECT count() FROM ayah GROUP ALL",
        )
        .await
        .unwrap();
    let surah_count: Option<CountResult> = res.take(0).unwrap_or(None);
    let ayah_count: Option<CountResult> = res.take(1).unwrap_or(None);

    Json(QuranStatsResponse {
        surah_count: surah_count.map(|c| c.count).unwrap_or(0),
        ayah_count: ayah_count.map(|c| c.count).unwrap_or(0),
    })
}

use surrealdb::types::SurrealValue;

#[derive(Debug, SurrealValue)]
struct CountResult {
    count: i64,
}

pub async fn reciters(State(state): State<AppState>) -> impl IntoResponse {
    let mut res = state
        .db
        .query("SELECT * FROM reciter ORDER BY name_en ASC")
        .await
        .unwrap();
    let reciters: Vec<Reciter> = res.take(0).unwrap_or_default();
    let api_reciters: Vec<ApiReciter> = reciters.into_iter().map(ApiReciter::from).collect();
    Json(api_reciters)
}

pub async fn surah_list(State(state): State<AppState>) -> impl IntoResponse {
    let mut res = state
        .db
        .query("SELECT * FROM surah ORDER BY surah_number ASC")
        .await
        .unwrap();
    let surahs: Vec<Surah> = res.take(0).unwrap_or_default();
    let api_surahs: Vec<ApiSurah> = surahs.into_iter().map(ApiSurah::from).collect();
    Json(api_surahs)
}

pub async fn surah_detail(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<SurahDetailResponse>, StatusCode> {
    // Get surah
    let mut res = state
        .db
        .query("SELECT * FROM surah WHERE surah_number = $num LIMIT 1")
        .bind(("num", number))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let surah: Option<Surah> = res.take(0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let surah = surah.ok_or(StatusCode::NOT_FOUND)?;

    // Get ayahs for this surah
    let mut res2 = state
        .db
        .query("SELECT * FROM ayah WHERE surah_number = $num ORDER BY ayah_number ASC")
        .bind(("num", number))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let ayahs: Vec<Ayah> = res2
        .take(0)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SurahDetailResponse {
        surah: ApiSurah::from(surah),
        ayahs: ayahs.into_iter().map(ApiAyah::from).collect(),
    }))
}

pub async fn quran_search(
    State(state): State<AppState>,
    Query(params): Query<QuranSearchParams>,
) -> Result<Json<QuranSearchResponse>, StatusCode> {
    let query = params.q.unwrap_or_default();
    if query.trim().is_empty() {
        return Ok(Json(QuranSearchResponse {
            query,
            search_type: "text".into(),
            ayahs: vec![],
        }));
    }

    let limit = params.limit.unwrap_or(20);
    let search_type = params.search_type.as_deref().unwrap_or("text");

    let results = match search_type {
        "semantic" => {
            crate::quran::search::search_ayahs_semantic(&state.db, &state.embedder, &query, limit)
                .await
        }
        "hybrid" => {
            crate::quran::search::search_ayahs_hybrid(&state.db, &state.embedder, &query, limit, 0)
                .await
        }
        "tafsir" => {
            crate::quran::search::search_ayahs_tafsir(&state.db, &state.embedder, &query, limit, 0)
                .await
        }
        _ => crate::quran::search::search_ayahs_text(&state.db, &query, limit, 0).await,
    };

    let ayahs = results.map_err(|e| {
        tracing::error!("Quran search failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(QuranSearchResponse {
        query,
        search_type: search_type.to_string(),
        ayahs: ayahs.into_iter().map(ApiAyahSearchResult::from).collect(),
    }))
}

pub async fn ayah_browse(
    State(state): State<AppState>,
    Query(params): Query<QuranBrowseParams>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(50);
    let page = params.page.unwrap_or(1);
    let offset = (page - 1) * limit;

    let (sql, needs_surah) = if let Some(surah) = params.surah {
        (
            format!(
                "SELECT * FROM ayah WHERE surah_number = $surah \
                 ORDER BY ayah_number ASC LIMIT {limit} START {offset}"
            ),
            Some(surah),
        )
    } else {
        (
            format!(
                "SELECT * FROM ayah ORDER BY surah_number ASC, ayah_number ASC \
                 LIMIT {limit} START {offset}"
            ),
            None,
        )
    };

    let mut query = state.db.query(&sql);
    if let Some(surah) = needs_surah {
        query = query.bind(("surah", surah));
    }
    let mut res = query.await.unwrap();
    let ayahs: Vec<Ayah> = res.take(0).unwrap_or_default();
    let has_more = ayahs.len() == limit;

    Json(crate::models::PaginatedResponse {
        data: ayahs.into_iter().map(ApiAyah::from).collect::<Vec<_>>(),
        page,
        has_more,
    })
}

pub async fn ask_quran(
    State(state): State<AppState>,
    Json(body): Json<QuranAskRequest>,
) -> Result<Response, StatusCode> {
    let question = body.question.trim().to_string();
    if question.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let ollama = state.ollama.as_ref().ok_or_else(|| {
        tracing::error!("Ollama client not configured");
        StatusCode::SERVICE_UNAVAILABLE
    })?;

    let model_name = body.model.clone();
    let (sources, byte_stream) = ollama
        .ask_quran(&state.db, &state.embedder, &question, model_name.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Quran RAG ask failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let source_ayahs: Vec<ApiAyahSearchResult> =
        sources.into_iter().map(ApiAyahSearchResult::from).collect();
    let sources_event = format!(
        "data: {}\n\n",
        serde_json::to_string(&serde_json::json!({ "sources": source_ayahs })).unwrap()
    );

    let sse_stream =
        futures::stream::once(
            async move { Ok::<_, std::io::Error>(bytes::Bytes::from(sources_event)) },
        )
        .chain(byte_stream.map(|chunk| match chunk {
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
                                serde_json::to_string(&serde_json::json!({ "text": msg.content }))
                                    .unwrap()
                            ));
                        }
                        if parsed.done {
                            sse.push_str("data: {\"done\":true}\n\n");
                        }
                    }
                }
                Ok(bytes::Bytes::from(sse))
            }
            Err(e) => Err(std::io::Error::other(e)),
        }));

    Ok(Response::builder()
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(Body::from_stream(sse_stream))
        .unwrap())
}

// ── Ayah-Hadith reference handlers ──

#[derive(serde::Serialize)]
pub struct AyahHadithResponse {
    pub curated: Vec<ApiHadith>,
    pub related: Option<Vec<ApiHadithSearchResult>>,
}

pub async fn ayah_hadiths(
    State(state): State<AppState>,
    Path(ayah_key): Path<String>,
    Query(params): Query<AyahHadithParams>,
) -> Result<Json<AyahHadithResponse>, StatusCode> {
    // Parse "surah:ayah" from path
    let (surah, ayah) = parse_ayah_key(&ayah_key).ok_or(StatusCode::BAD_REQUEST)?;

    // Get curated hadiths via relation edges
    let curated = crate::quran::hadith_refs::get_curated_hadiths(&state.db, surah, ayah)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get curated hadiths: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Optionally get semantic results
    let related = if params.include_semantic.unwrap_or(false) {
        let limit = params.semantic_limit.unwrap_or(5);
        let results =
            crate::quran::hadith_refs::find_semantic_hadiths(&state.db, surah, ayah, limit)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to get semantic hadiths: {e}");
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
        Some(
            results
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect(),
        )
    } else {
        None
    };

    Ok(Json(AyahHadithResponse {
        curated: curated.into_iter().map(ApiHadith::from).collect(),
        related,
    }))
}

pub async fn surah_hadith_counts(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<std::collections::HashMap<String, i64>>, StatusCode> {
    let counts = crate::quran::hadith_refs::get_hadith_counts(&state.db, number)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get hadith counts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert i64 keys to String keys for JSON
    let string_counts: std::collections::HashMap<String, i64> = counts
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    Ok(Json(string_counts))
}

/// Returns { ayah_number: count } for ayahs in this surah that have similar_to or shares_phrase edges.
pub async fn surah_similar_counts(
    State(state): State<AppState>,
    Path(number): Path<i64>,
) -> Result<Json<std::collections::HashMap<String, i64>>, StatusCode> {
    #[derive(Debug, surrealdb::types::SurrealValue)]
    struct AyahCount {
        ayah_number: i64,
        count: i64,
    }

    // Use subquery to get ayah IDs first (uses ayah_surah_idx), then filter edges
    // by indexed `in` field — avoids dereferencing in.surah_number on every edge row.
    let mut res = state
        .db
        .query(
            "LET $ayah_ids = (SELECT id FROM ayah WHERE surah_number = $s); \
             SELECT in.ayah_number AS ayah_number, count() AS count \
             FROM similar_to WHERE in IN $ayah_ids \
             GROUP BY ayah_number; \
             SELECT in.ayah_number AS ayah_number, count() AS count \
             FROM shares_phrase WHERE in IN $ayah_ids \
             GROUP BY ayah_number",
        )
        .bind(("s", number))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let similar: Vec<AyahCount> = res.take(1).unwrap_or_default();
    let phrases: Vec<AyahCount> = res.take(2).unwrap_or_default();

    // Merge both counts
    let mut counts = std::collections::HashMap::new();
    for ac in similar.iter().chain(phrases.iter()) {
        *counts.entry(ac.ayah_number.to_string()).or_insert(0) += ac.count;
    }

    Ok(Json(counts))
}

fn parse_ayah_key(key: &str) -> Option<(i64, i64)> {
    let parts: Vec<&str> = key.split(':').collect();
    if parts.len() == 2 {
        let s = parts[0].parse().ok()?;
        let a = parts[1].parse().ok()?;
        Some((s, a))
    } else {
        None
    }
}

// ── Word Morphology Handlers ──

pub async fn ayah_words(
    State(state): State<AppState>,
    Path(ayah_key): Path<String>,
) -> Result<Json<Vec<ApiQuranWord>>, (StatusCode, String)> {
    let (surah, ayah) = parse_ayah_key(&ayah_key).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid ayah key format. Use surah:ayah (e.g., 1:1)".to_string(),
        )
    })?;

    let mut res = state
        .db
        .query(
            "SELECT * FROM quran_word WHERE surah_number = $s AND ayah_number = $a ORDER BY word_position",
        )
        .bind(("s", surah))
        .bind(("a", ayah))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let words: Vec<QuranWord> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(words.into_iter().map(ApiQuranWord::from).collect()))
}

pub async fn root_search(
    State(state): State<AppState>,
    Path(root): Path<String>,
) -> Result<Json<RootSearchResponse>, (StatusCode, String)> {
    let mut res = state
        .db
        .query(
            "SELECT * FROM quran_word WHERE root = $root ORDER BY surah_number, ayah_number, word_position",
        )
        .bind(("root", root.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let words: Vec<QuranWord> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Count unique ayahs
    let ayah_count = words
        .iter()
        .map(|w| (w.surah_number, w.ayah_number))
        .collect::<std::collections::HashSet<_>>()
        .len();

    Ok(Json(RootSearchResponse {
        root,
        occurrences: words.into_iter().map(ApiQuranWord::from).collect(),
        ayah_count,
    }))
}

// ── Similar Ayahs / Mutashabihat Handlers ──

use crate::models::record_id_key_string;
use surrealdb::types::RecordId;

#[derive(Debug, SurrealValue)]
struct SimilarEdgeRow {
    score: i64,
    coverage: i64,
    matched_positions: Option<String>,
    surah_number: i64,
    ayah_number: i64,
    text_ar: Option<String>,
    text_en: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct PhraseEdgeRow {
    id: Option<RecordId>,
    text_ar: String,
    occurrence: i64,
    verses_count: i64,
    chapters_count: i64,
}

#[derive(Debug, SurrealValue)]
struct AyahKeyRow {
    surah_number: i64,
    ayah_number: i64,
}

pub async fn ayah_similar(
    State(state): State<AppState>,
    Path(ayah_key): Path<String>,
) -> Result<Json<AyahSimilarResponse>, (StatusCode, String)> {
    let (surah, ayah) = parse_ayah_key(&ayah_key).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "Invalid ayah key format. Use surah:ayah (e.g., 2:255)".to_string(),
        )
    })?;

    let ayah_id = RecordId::new("ayah", format!("{surah}_{ayah}"));

    // 1. Query similar ayahs
    let mut res = state
        .db
        .query(
            "SELECT score, coverage, matched_positions, \
             out.surah_number AS surah_number, out.ayah_number AS ayah_number, \
             out.text_ar AS text_ar, out.text_en AS text_en \
             FROM similar_to WHERE in = $ayah_id ORDER BY score DESC LIMIT 20",
        )
        .bind(("ayah_id", ayah_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let similar_rows: Vec<SimilarEdgeRow> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let similar: Vec<ApiSimilarAyah> = similar_rows
        .into_iter()
        .map(|r| ApiSimilarAyah {
            ayah_key: format!("{}:{}", r.surah_number, r.ayah_number),
            score: r.score,
            coverage: r.coverage,
            matched_positions: r
                .matched_positions
                .and_then(|s| serde_json::from_str(&s).ok()),
            text_ar: r.text_ar,
            text_en: r.text_en,
        })
        .collect();

    // 2. Query shared phrases for this ayah
    let mut res2 = state
        .db
        .query(
            "SELECT out.id AS id, out.text_ar AS text_ar, out.occurrence AS occurrence, \
             out.verses_count AS verses_count, out.chapters_count AS chapters_count \
             FROM shares_phrase WHERE in = $ayah_id",
        )
        .bind(("ayah_id", ayah_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let phrase_rows: Vec<PhraseEdgeRow> = res2
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Batch-fetch all ayahs sharing any of these phrases (single query instead of N+1)
    let phrase_ids: Vec<RecordId> = phrase_rows.iter().filter_map(|p| p.id.clone()).collect();

    let mut phrase_ayah_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    if !phrase_ids.is_empty() {
        #[derive(Debug, SurrealValue)]
        struct PhraseAyahRow {
            phrase_id: Option<RecordId>,
            surah_number: i64,
            ayah_number: i64,
        }

        let mut res3 = state
            .db
            .query(
                "SELECT out AS phrase_id, in.surah_number AS surah_number, in.ayah_number AS ayah_number \
                 FROM shares_phrase WHERE out IN $phrase_ids AND in != $ayah_id",
            )
            .bind(("phrase_ids", phrase_ids))
            .bind(("ayah_id", ayah_id.clone()))
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let rows: Vec<PhraseAyahRow> = res3
            .take(0)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        for row in rows {
            if let Some(ref pid) = row.phrase_id {
                let key = record_id_key_string(pid);
                phrase_ayah_map
                    .entry(key)
                    .or_default()
                    .push(format!("{}:{}", row.surah_number, row.ayah_number));
            }
        }
    }

    let phrases: Vec<ApiPhraseWithAyahs> = phrase_rows
        .into_iter()
        .filter_map(|p| {
            let pid = p.id.as_ref()?;
            let pid_str = record_id_key_string(pid);
            Some(ApiPhraseWithAyahs {
                id: pid_str.clone(),
                text_ar: p.text_ar,
                occurrence: p.occurrence,
                ayah_keys: phrase_ayah_map.remove(&pid_str).unwrap_or_default(),
            })
        })
        .collect();

    Ok(Json(AyahSimilarResponse { similar, phrases }))
}

pub async fn phrase_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ApiPhraseWithAyahs>, (StatusCode, String)> {
    let phrase_id = RecordId::new("quran_phrase", id.clone());

    // Get phrase record
    let mut res = state
        .db
        .query("SELECT * FROM $phrase_id")
        .bind(("phrase_id", phrase_id.clone()))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let phrase: Option<QuranPhrase> = res
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let phrase = phrase.ok_or_else(|| (StatusCode::NOT_FOUND, format!("Phrase {id} not found")))?;

    // Get all ayahs sharing this phrase
    let mut res2 = state
        .db
        .query(
            "SELECT in.surah_number AS surah_number, in.ayah_number AS ayah_number \
             FROM shares_phrase WHERE out = $phrase_id",
        )
        .bind(("phrase_id", phrase_id))
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ayah_rows: Vec<AyahKeyRow> = res2
        .take(0)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let ayah_keys: Vec<String> = ayah_rows
        .into_iter()
        .map(|r| format!("{}:{}", r.surah_number, r.ayah_number))
        .collect();

    Ok(Json(ApiPhraseWithAyahs {
        id: phrase.id.as_ref().map(record_id_key_string).unwrap_or(id),
        text_ar: phrase.text_ar,
        occurrence: phrase.occurrence,
        ayah_keys,
    }))
}
