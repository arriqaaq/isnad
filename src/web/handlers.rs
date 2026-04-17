use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use std::collections::HashSet;
use surrealdb::types::{RecordId, SurrealValue};

use crate::analysis;
use crate::models::{
    ApiBook, ApiHadith, ApiHadithFamily, ApiHadithSearchResult, ApiNarrator,
    ApiNarratorSearchResult, ApiNarratorWithCount, Book, GraphData, GraphEdge, GraphEdgeData,
    GraphNode, GraphNodeData, HADITH_FIELDS, Hadith, HadithFamily, Narrator, PaginatedResponse,
    StatsResponse, record_id_key_string, record_id_string,
};
use crate::rag::ChatChunk;

use super::AppState;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

// ── Query parameter types ──

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub page: Option<usize>,
}

#[derive(Deserialize)]
pub struct ListParams {
    pub book: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub q: Option<String>,
    pub generation: Option<String>,
}

#[derive(Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub model: Option<String>,
}

// ── API Handlers ──

pub async fn stats(State(state): State<AppState>) -> impl IntoResponse {
    let result = state
        .db
        .query(
            "SELECT count() AS c FROM hadith GROUP ALL; \
             SELECT count() AS c FROM narrator GROUP ALL; \
             SELECT count() AS c FROM book GROUP ALL;",
        )
        .await;

    let (hadith_count, narrator_count, book_count) = match result {
        Ok(mut res) => {
            let h: Option<CountResult> = res.take(0).unwrap_or(None);
            let n: Option<CountResult> = res.take(1).unwrap_or(None);
            let b: Option<CountResult> = res.take(2).unwrap_or(None);
            (
                h.map(|r| r.c).unwrap_or(0),
                n.map(|r| r.c).unwrap_or(0),
                b.map(|r| r.c).unwrap_or(0),
            )
        }
        Err(e) => {
            tracing::error!("Stats query failed: {e}");
            (0, 0, 0)
        }
    };

    Json(StatsResponse {
        hadith_count,
        narrator_count,
        book_count,
    })
}

pub async fn books(State(state): State<AppState>) -> impl IntoResponse {
    let books: Vec<Book> = match state
        .db
        .query("SELECT * FROM book ORDER BY book_number ASC")
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Books query failed: {e}");
            vec![]
        }
    };

    Json(books.into_iter().map(ApiBook::from).collect::<Vec<_>>())
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let search_type = params.search_type.unwrap_or_else(|| "hybrid".into());
    let limit = params.limit.unwrap_or(20);

    if query.is_empty() {
        return Json(serde_json::json!({
            "query": query,
            "search_type": search_type,
            "hadiths": [],
            "narrators": []
        }));
    }

    let hadith_results = match search_type.as_str() {
        "semantic" => {
            crate::search::search_hadiths_semantic(&state.db, &state.embedder, &query, limit)
                .await
                .unwrap_or_default()
        }
        "text" => crate::search::search_hadiths_text(&state.db, &query, limit, 0)
            .await
            .unwrap_or_default(),
        _ => {
            // Default: hybrid search (BM25 + vector via RRF)
            crate::search::search_hadiths_hybrid(&state.db, &state.embedder, &query, limit, 0)
                .await
                .unwrap_or_default()
        }
    };

    let narrator_results = crate::search::search_narrators(&state.db, &query, 10, 0)
        .await
        .unwrap_or_default();

    Json(serde_json::json!({
        "query": query,
        "search_type": search_type,
        "hadiths": hadith_results.into_iter().map(ApiHadithSearchResult::from).collect::<Vec<_>>(),
        "narrators": narrator_results.into_iter().map(ApiNarratorSearchResult::from).collect::<Vec<_>>()
    }))
}

pub async fn hadith_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    let query = if let Some(book_id) = params.book {
        format!(
            "SELECT {HADITH_FIELDS} FROM hadith WHERE book_id = {book_id} \
             ORDER BY hadith_number ASC LIMIT {limit} START {offset}"
        )
    } else {
        format!(
            "SELECT {HADITH_FIELDS} FROM hadith ORDER BY hadith_number ASC \
             LIMIT {limit} START {offset}"
        )
    };

    let hadiths: Vec<Hadith> = match state.db.query(&query).await {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Hadith list query failed: {e}");
            vec![]
        }
    };
    let has_more = hadiths.len() == limit;

    Json(PaginatedResponse {
        data: hadiths.into_iter().map(ApiHadith::from).collect(),
        page,
        has_more,
    })
}

pub async fn hadith_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let hrid = rid("hadith", &id);

    // Single multi-statement query instead of 4 sequential round trips
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid; \
             SELECT <-narrates<-narrator.* AS narrators FROM $rid; \
             SELECT in.id AS id, in.surah_number AS surah_number, in.ayah_number AS ayah_number, \
               in.text_ar AS text_ar, in.text_en AS text_en, in.tafsir_en AS tafsir_en \
               FROM references_hadith WHERE out = $rid ORDER BY surah_number, ayah_number; \
             SELECT ->similar_to->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid;"
        ))
        .bind(("rid", hrid))
        .await
        .map_err(|e| {
            tracing::error!("Hadith detail query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let hadith: Option<Hadith> = res.take(0).unwrap_or(None);
    let hadith = hadith.ok_or(StatusCode::NOT_FOUND)?;

    let narrators: Vec<Narrator> = {
        let result: Option<NarratorsResult> = res.take(1).unwrap_or(None);
        result.map(|r| r.narrators).unwrap_or_default()
    };

    let linked_ayahs: Vec<crate::quran::models::ApiAyah> = {
        let ayahs: Vec<crate::quran::models::Ayah> = res.take(2).unwrap_or_default();
        ayahs
            .into_iter()
            .map(crate::quran::models::ApiAyah::from)
            .collect()
    };

    let similar_hadiths: Vec<ApiHadith> = {
        #[derive(Debug, SurrealValue)]
        struct HadithsResult {
            hadiths: Vec<Hadith>,
        }
        let result: Option<HadithsResult> = res.take(3).unwrap_or(None);
        result
            .map(|r| r.hadiths.into_iter().map(ApiHadith::from).collect())
            .unwrap_or_default()
    };

    Ok(Json(serde_json::json!({
        "hadith": ApiHadith::from(hadith),
        "narrators": narrators.into_iter().map(ApiNarrator::from).collect::<Vec<_>>(),
        "linked_ayahs": linked_ayahs,
        "similar_hadiths": similar_hadiths
    })))
}

pub async fn narrator_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    // Build WHERE clauses dynamically
    let mut conditions: Vec<String> = Vec::new();
    if let Some(q) = &params.q {
        let _ = q; // used via bind
        conditions.push(
            "(string::lowercase(name_en) CONTAINS string::lowercase($q) OR name_ar CONTAINS $q)"
                .to_string(),
        );
    }
    if let Some(generation) = &params.generation {
        let _ = generation;
        conditions.push("generation = $generation".to_string());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let query_str = format!(
        "SELECT * FROM narrator {where_clause} ORDER BY hadith_count DESC LIMIT $limit START $offset"
    );

    let mut query = state.db.query(&query_str);
    if let Some(q) = &params.q {
        query = query.bind(("q", q.clone()));
    }
    if let Some(generation) = &params.generation {
        query = query.bind(("generation", generation.clone()));
    }
    query = query.bind(("limit", limit)).bind(("offset", offset));

    let narrators: Vec<NarratorWithCount> = match query.await {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Narrator list query failed: {e}");
            vec![]
        }
    };

    let has_more = narrators.len() == limit;
    let api_narrators: Vec<ApiNarratorWithCount> = narrators
        .into_iter()
        .map(|n| ApiNarratorWithCount {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
            bio: n.bio,
            kunya: n.kunya,
            death_year: n.death_year,
            hadith_count: n.hadith_count.unwrap_or(0),
        })
        .collect();

    Json(PaginatedResponse {
        data: api_narrators,
        page,
        has_more,
    })
}

pub async fn narrator_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let nrid = rid("narrator", &id);

    // Single multi-statement query instead of 4 sequential round trips
    let (narrator, hadiths, teachers, students) = match state
        .db
        .query(
            &format!(
                "SELECT * FROM $rid; \
                 SELECT ->narrates->hadith.{{{HADITH_FIELDS}}} AS hadiths FROM $rid; \
                 SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
                 SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;"
            ),
        )
        .bind(("rid", nrid))
        .await
    {
        Ok(mut res) => {
            let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
            let hadiths_result: Option<HadithsResult> = res.take(1).unwrap_or(None);
            let teachers_result: Option<TeachersResult> = res.take(2).unwrap_or(None);
            let students_result: Option<StudentsResult> = res.take(3).unwrap_or(None);
            (
                narrator,
                hadiths_result.map(|r| r.hadiths).unwrap_or_default(),
                teachers_result.map(|r| r.teachers).unwrap_or_default(),
                students_result.map(|r| r.students).unwrap_or_default(),
            )
        }
        Err(e) => {
            tracing::error!("Narrator detail query failed: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let narrator = narrator.ok_or(StatusCode::NOT_FOUND)?;

    // Deduplicate by narrator ID
    let dedup_narrators = |narrators: Vec<Narrator>| -> Vec<Narrator> {
        let mut seen = HashSet::new();
        narrators
            .into_iter()
            .filter(|n| {
                n.id.as_ref()
                    .map(|id| seen.insert(record_id_string(id)))
                    .unwrap_or(false)
            })
            .collect()
    };
    let teachers = dedup_narrators(teachers);
    let students = dedup_narrators(students);

    Ok(Json(serde_json::json!({
        "narrator": ApiNarrator::from(narrator),
        "hadiths": hadiths.into_iter().map(ApiHadith::from).collect::<Vec<_>>(),
        "teachers": teachers.into_iter().map(ApiNarrator::from).collect::<Vec<_>>(),
        "students": students.into_iter().map(ApiNarrator::from).collect::<Vec<_>>()
    })))
}

pub async fn chain_graph_data(
    State(state): State<AppState>,
    Path(hadith_id): Path<String>,
) -> impl IntoResponse {
    let hrid = rid("hadith", &hadith_id);

    let narrators = match state
        .db
        .query("SELECT <-narrates<-narrator.* AS narrators FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
    {
        Ok(mut r) => {
            let result: Option<NarratorsResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.narrators).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Chain narrators query failed: {e}");
            vec![]
        }
    };

    let edges: Vec<HeardFromEdge> = match state
        .db
        .query("SELECT in AS in_id, out AS out_id, chain_position FROM heard_from WHERE hadith_ref = $rid ORDER BY chain_position")
        .bind(("rid", hrid))
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Chain edges query failed: {e}");
            vec![]
        }
    };

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
        total_teachers: None,
        total_students: None,
    };

    for narrator in &narrators {
        if let Some(id) = &narrator.id {
            graph.nodes.push(GraphNode {
                data: GraphNodeData {
                    id: record_id_string(id),
                    label: narrator
                        .name_ar
                        .clone()
                        .unwrap_or_else(|| narrator.name_en.clone()),
                    label_en: narrator.name_en.clone(),
                    node_type: "narrator".into(),
                    generation: narrator.generation.clone(),
                },
            });
        }
    }

    for (i, edge) in edges.iter().enumerate() {
        graph.edges.push(GraphEdge {
            data: GraphEdgeData {
                id: format!("e{i}"),
                source: record_id_string(&edge.in_id),
                target: record_id_string(&edge.out_id),
                label: "heard from".into(),
                chain_position: edge.chain_position,
            },
        });
    }

    Json(graph)
}

pub async fn narrator_graph_data(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let nrid = rid("narrator", &id);

    let (narrator, teachers, students) = match state
        .db
        .query(
            "SELECT * FROM $rid; \
             SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $rid; \
             SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $rid;",
        )
        .bind(("rid", nrid))
        .await
    {
        Ok(mut res) => {
            let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
            let teachers_result: Option<TeachersResult> = res.take(1).unwrap_or(None);
            let students_result: Option<StudentsResult> = res.take(2).unwrap_or(None);
            (
                narrator,
                teachers_result.map(|r| r.teachers).unwrap_or_default(),
                students_result.map(|r| r.students).unwrap_or_default(),
            )
        }
        Err(e) => {
            tracing::error!("Narrator graph query failed: {e}");
            (None, vec![], vec![])
        }
    };

    // Deduplicate by narrator ID
    let dedup = |narrators: Vec<Narrator>| -> Vec<Narrator> {
        let mut seen = HashSet::new();
        narrators
            .into_iter()
            .filter(|n| {
                n.id.as_ref()
                    .map(|id| seen.insert(record_id_string(id)))
                    .unwrap_or(false)
            })
            .collect()
    };
    let teachers = dedup(teachers);
    let students = dedup(students);
    let total_teachers = teachers.len();
    let total_students = students.len();

    // Cap for graph rendering performance
    const MAX_GRAPH_NODES: usize = 25;
    let teachers: Vec<_> = teachers.into_iter().take(MAX_GRAPH_NODES).collect();
    let students: Vec<_> = students.into_iter().take(MAX_GRAPH_NODES).collect();

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
        total_teachers: Some(total_teachers),
        total_students: Some(total_students),
    };

    if let Some(narrator) = &narrator
        && let Some(nid) = &narrator.id
    {
        let nid_str = record_id_string(nid);
        graph.nodes.push(GraphNode {
            data: GraphNodeData {
                id: nid_str.clone(),
                label: narrator
                    .name_ar
                    .clone()
                    .unwrap_or_else(|| narrator.name_en.clone()),
                label_en: narrator.name_en.clone(),
                node_type: "center".into(),
                generation: narrator.generation.clone(),
            },
        });

        for (i, teacher) in teachers.iter().enumerate() {
            if let Some(tid) = &teacher.id {
                let tid_str = record_id_string(tid);
                graph.nodes.push(GraphNode {
                    data: GraphNodeData {
                        id: tid_str.clone(),
                        label: teacher
                            .name_ar
                            .clone()
                            .unwrap_or_else(|| teacher.name_en.clone()),
                        label_en: teacher.name_en.clone(),
                        node_type: "teacher".into(),
                        generation: teacher.generation.clone(),
                    },
                });
                graph.edges.push(GraphEdge {
                    data: GraphEdgeData {
                        id: format!("t{i}"),
                        source: nid_str.clone(),
                        target: tid_str,
                        label: "heard from".into(),
                        chain_position: None,
                    },
                });
            }
        }

        for (i, student) in students.iter().enumerate() {
            if let Some(sid) = &student.id {
                let sid_str = record_id_string(sid);
                graph.nodes.push(GraphNode {
                    data: GraphNodeData {
                        id: sid_str.clone(),
                        label: student
                            .name_ar
                            .clone()
                            .unwrap_or_else(|| student.name_en.clone()),
                        label_en: student.name_en.clone(),
                        node_type: "student".into(),
                        generation: student.generation.clone(),
                    },
                });
                graph.edges.push(GraphEdge {
                    data: GraphEdgeData {
                        id: format!("s{i}"),
                        source: sid_str,
                        target: nid_str.clone(),
                        label: "heard from".into(),
                        chain_position: None,
                    },
                });
            }
        }
    }

    Json(graph)
}

pub async fn ask(
    State(state): State<AppState>,
    Json(body): Json<AskRequest>,
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
        .ask(&state.db, &state.embedder, &question, model_name.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("RAG ask failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let source_hadiths: Vec<ApiHadithSearchResult> = sources
        .into_iter()
        .map(ApiHadithSearchResult::from)
        .collect();
    let sources_event = format!(
        "data: {}\n\n",
        serde_json::to_string(&serde_json::json!({ "sources": source_hadiths })).unwrap()
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
            Err(e) => {
                let err_event = format!(
                    "data: {}\n\n",
                    serde_json::to_string(&serde_json::json!({ "error": e.to_string() })).unwrap()
                );
                Ok(bytes::Bytes::from(err_event))
            }
        }));

    let body = Body::from_stream(sse_stream);

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

// ── Internal translation update endpoint ──

// ── Narrator update endpoint ──

#[derive(Deserialize)]
pub struct UpdateNarratorRequest {
    pub name_ar: Option<String>,
    pub name_en: Option<String>,
    pub gender: Option<String>,
    pub generation: Option<String>,
    pub bio: Option<String>,
    pub kunya: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub birth_year: Option<i64>,
    pub birth_calendar: Option<String>,
    pub death_year: Option<i64>,
    pub death_calendar: Option<String>,
    pub locations: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

pub async fn update_narrator(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateNarratorRequest>,
) -> impl IntoResponse {
    // Build a JSON object of all provided fields, then MERGE into the narrator
    let mut update = serde_json::Map::new();

    macro_rules! set_field {
        ($name:ident) => {
            if let Some(ref v) = body.$name {
                update.insert(stringify!($name).to_string(), serde_json::json!(v));
            }
        };
    }

    set_field!(name_ar);
    set_field!(name_en);
    set_field!(gender);
    set_field!(generation);
    set_field!(bio);
    set_field!(kunya);
    set_field!(aliases);
    set_field!(birth_year);
    set_field!(birth_calendar);
    set_field!(death_year);
    set_field!(death_calendar);
    set_field!(locations);
    set_field!(tags);
    if update.is_empty() {
        return StatusCode::BAD_REQUEST;
    }

    match state
        .db
        .query("UPDATE $rid MERGE $data")
        .bind(("rid", rid("narrator", &id)))
        .bind(("data", serde_json::Value::Object(update)))
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Narrator update failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// ── Internal translation update endpoint ──

#[derive(Deserialize)]
pub struct TranslateUpdate {
    pub table: String,
    pub id: String,
    pub field: String,
    pub value: String,
}

pub async fn update_translation(
    State(state): State<AppState>,
    Json(body): Json<TranslateUpdate>,
) -> impl IntoResponse {
    let table = &body.table;
    let field = &body.field;
    // Only allow updating specific fields on specific tables
    if !matches!(table.as_str(), "hadith" | "narrator")
        || !matches!(field.as_str(), "text_en" | "name_en")
    {
        return StatusCode::BAD_REQUEST;
    }

    let sql = format!("UPDATE $rid SET {field} = $value");
    match state
        .db
        .query(&sql)
        .bind(("rid", rid(table, &body.id)))
        .bind(("value", body.value.clone()))
        .await
    {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            tracing::error!("Translation update failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// ── Analysis endpoints ──

pub async fn family_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    let mut res = state
        .db
        .query(
            "SELECT * FROM hadith_family ORDER BY variant_count DESC \
             LIMIT $limit START $offset",
        )
        .bind(("limit", limit + 1))
        .bind(("offset", offset))
        .await
        .unwrap();
    let families: Vec<HadithFamily> = res.take(0).unwrap_or_default();

    let has_more = families.len() > limit;
    let data: Vec<ApiHadithFamily> = families
        .into_iter()
        .take(limit)
        .map(ApiHadithFamily::from)
        .collect();

    Json(PaginatedResponse {
        data,
        page,
        has_more,
    })
}

pub async fn family_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let fid = rid("hadith_family", &id);

    let mut res = state
        .db
        .query(format!(
            "SELECT * FROM $fid; \
             SELECT {HADITH_FIELDS} FROM hadith WHERE family_id = $fid ORDER BY hadith_number ASC;"
        ))
        .bind(("fid", fid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let family: Option<HadithFamily> =
        res.take(0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let family = family.ok_or(StatusCode::NOT_FOUND)?;
    let hadiths: Vec<Hadith> = res.take(1).unwrap_or_default();

    Ok(Json(serde_json::json!({
        "family": ApiHadithFamily::from(family),
        "hadiths": hadiths.into_iter().map(ApiHadith::from).collect::<Vec<_>>(),
    })))
}

pub async fn narrator_reliability(Path(id): Path<String>) -> impl IntoResponse {
    // Evidence/grading data removed — SemanticHadith grading was unreliable (see NOTES.md).
    // This endpoint kept for API compatibility; returns empty assessments.
    Json(serde_json::json!({
        "narrator_id": id,
        "assessments": [],
        "sources_count": 0,
    }))
}

// ── Mustalah API handlers ──

pub async fn mustalah_stats(State(state): State<AppState>) -> impl IntoResponse {
    let mut res = state
        .db
        .query(
            "SELECT count() AS c FROM hadith_family GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'mutawatir' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'mashhur' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'aziz' GROUP ALL;\
             SELECT count() AS c FROM isnad_analysis WHERE breadth_class = 'gharib' GROUP ALL",
        )
        .await
        .unwrap();

    let families: Option<CountResult> = res.take(0).unwrap_or(None);
    let analyzed: Option<CountResult> = res.take(1).unwrap_or(None);
    let mutawatir: Option<CountResult> = res.take(2).unwrap_or(None);
    let mashhur: Option<CountResult> = res.take(3).unwrap_or(None);
    let aziz: Option<CountResult> = res.take(4).unwrap_or(None);
    let gharib: Option<CountResult> = res.take(5).unwrap_or(None);

    Json(serde_json::json!({
        "family_count": families.map(|c| c.c).unwrap_or(0),
        "analyzed_count": analyzed.map(|c| c.c).unwrap_or(0),
        "mutawatir_count": mutawatir.map(|c| c.c).unwrap_or(0),
        "mashhur_count": mashhur.map(|c| c.c).unwrap_or(0),
        "aziz_count": aziz.map(|c| c.c).unwrap_or(0),
        "gharib_count": gharib.map(|c| c.c).unwrap_or(0),
    }))
}

pub async fn mustalah_family_analysis(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let fid = rid("hadith_family", &id);

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct IsnadRow {
        breadth_class: Option<String>,
        min_breadth: Option<i64>,
        bottleneck_tabaqah: Option<i64>,
        sahabi_count: Option<i64>,
        mutabaat_count: Option<i64>,
        shawahid_count: Option<i64>,
        chain_count: Option<i64>,
        ilal_flags: Option<Vec<String>>,
    }

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct ChainRow {
        variant: Option<RecordId>,
        continuity: Option<String>,
        narrator_count: Option<i64>,
        has_chronology_conflict: Option<bool>,
        narrator_ids: Option<Vec<String>>,
    }

    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct PivotRow {
        narrator: Option<RecordId>,
        bundle_coverage: Option<f64>,
        fan_out: Option<i64>,
        collector_diversity: Option<i64>,
        bypass_count: Option<i64>,
        is_bottleneck: Option<bool>,
    }

    let mut res = state
        .db
        .query(
            "SELECT * FROM isnad_analysis WHERE family = $fid LIMIT 1;\
             SELECT * FROM chain_assessment WHERE family = $fid;\
             SELECT * FROM narrator_pivot WHERE family = $fid ORDER BY bundle_coverage DESC;",
        )
        .bind(("fid", fid))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let isnad: Option<IsnadRow> = res.take(0).unwrap_or(None);
    let chains: Vec<ChainRow> = res.take(1).unwrap_or_default();
    let pivots: Vec<PivotRow> = res.take(2).unwrap_or_default();

    let chains_json: Vec<serde_json::Value> = chains
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "variant_id": c.variant.as_ref().map(record_id_key_string).unwrap_or_default(),
                "continuity": c.continuity,
                "narrator_count": c.narrator_count,
                "has_chronology_conflict": c.has_chronology_conflict,
                "narrator_ids": c.narrator_ids,
            })
        })
        .collect();

    let pivots_json: Vec<serde_json::Value> = pivots
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "narrator_id": p.narrator.as_ref().map(record_id_key_string).unwrap_or_default(),
                "bundle_coverage": p.bundle_coverage,
                "fan_out": p.fan_out,
                "collector_diversity": p.collector_diversity,
                "bypass_count": p.bypass_count,
                "is_bottleneck": p.is_bottleneck,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "analysis": isnad,
        "chains": chains_json,
        "pivots": pivots_json,
    })))
}

pub async fn narrator_isnad_role(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    #[derive(Debug, SurrealValue, serde::Serialize)]
    struct PivotInfo {
        family: Option<RecordId>,
        is_bottleneck: Option<bool>,
    }
    let mut res = state
        .db
        .query("SELECT family, is_bottleneck FROM narrator_pivot WHERE narrator = $nid")
        .bind(("nid", rid("narrator", &id)))
        .await
        .unwrap();
    let rows: Vec<PivotInfo> = res.take(0).unwrap_or_default();

    let pivot_count = rows.len();
    let bottleneck_count = rows
        .iter()
        .filter(|r| r.is_bottleneck == Some(true))
        .count();
    let families: Vec<String> = rows
        .iter()
        .filter_map(|r| r.family.as_ref().map(record_id_key_string))
        .collect();

    Json(serde_json::json!({
        "narrator_id": id,
        "pivot_family_count": pivot_count,
        "bottleneck_family_count": bottleneck_count,
        "families": families,
    }))
}

pub async fn matn_diff_handler(
    State(state): State<AppState>,
    Query(params): Query<DiffParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let a_id = params.a.ok_or(StatusCode::BAD_REQUEST)?;
    let b_id = params.b.ok_or(StatusCode::BAD_REQUEST)?;

    // Fetch both hadiths in a single multi-statement query
    let mut res = state
        .db
        .query(format!(
            "SELECT {HADITH_FIELDS} FROM $rid_a; SELECT {HADITH_FIELDS} FROM $rid_b;"
        ))
        .bind(("rid_a", rid("hadith", &a_id)))
        .bind(("rid_b", rid("hadith", &b_id)))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_a: Option<Hadith> = res.take(0).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_a = hadith_a.ok_or(StatusCode::NOT_FOUND)?;
    let hadith_b: Option<Hadith> = res.take(1).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let hadith_b = hadith_b.ok_or(StatusCode::NOT_FOUND)?;

    // Use Arabic text (or English fallback) for diffing
    let text_a = hadith_a
        .text_ar
        .as_deref()
        .or(hadith_a.text_en.as_deref())
        .unwrap_or("");
    let text_b = hadith_b
        .text_ar
        .as_deref()
        .or(hadith_b.text_en.as_deref())
        .unwrap_or("");

    let result = analysis::matn_diff::diff_matn(text_a, text_b, &a_id, &b_id);
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct DiffParams {
    pub a: Option<String>,
    pub b: Option<String>,
}

pub async fn export_family(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<ExportParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let result = analysis::export::fetch_family_analysis(&state.db, &id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let format = params.format.as_deref().unwrap_or("json");

    match format {
        "md" | "markdown" => {
            let md = analysis::export::export_markdown(&result);
            Ok(Response::builder()
                .header("Content-Type", "text/markdown")
                .header(
                    "Content-Disposition",
                    format!("attachment; filename=\"family_{id}.md\""),
                )
                .body(Body::from(md))
                .unwrap())
        }
        _ => {
            let bundle = analysis::export::ArtifactBundle::from(&result);
            let json = serde_json::to_string_pretty(&bundle).unwrap_or_default();
            Ok(Response::builder()
                .header("Content-Type", "application/json")
                .body(Body::from(json))
                .unwrap())
        }
    }
}

#[derive(Deserialize)]
pub struct ExportParams {
    pub format: Option<String>,
}

// ── Helper result types ──

#[derive(Debug, SurrealValue)]
struct CountResult {
    c: i64,
}

#[derive(Debug, SurrealValue)]
pub struct NarratorWithCount {
    pub id: Option<RecordId>,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub search_name: Option<String>,
    pub generation: Option<String>,
    pub bio: Option<String>,
    pub kunya: Option<String>,
    pub death_year: Option<i64>,
    pub hadith_count: Option<i64>,
}

#[derive(Debug, SurrealValue)]
struct NarratorsResult {
    narrators: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct HadithsResult {
    hadiths: Vec<Hadith>,
}

#[derive(Debug, SurrealValue)]
struct TeachersResult {
    teachers: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct StudentsResult {
    students: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct HeardFromEdge {
    in_id: RecordId,
    out_id: RecordId,
    chain_position: Option<i64>,
}

// ── Unified Quran & Sunnah endpoints ──

pub async fn unified_search(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let query = params.q.unwrap_or_default();
    let search_type = params.search_type.unwrap_or_else(|| "hybrid".into());
    let limit = params.limit.unwrap_or(20);
    let page = params.page.unwrap_or(1).max(1);

    if query.is_empty() {
        return Json(serde_json::json!({
            "query": query,
            "search_type": search_type,
            "results": [],
            "quran_count": 0,
            "hadith_count": 0,
            "page": page,
            "has_more": false
        }));
    }

    match crate::unified::search_unified(
        &state.db,
        &state.embedder,
        &query,
        &search_type,
        limit,
        page,
    )
    .await
    {
        Ok(response) => Json(serde_json::to_value(response).unwrap()),
        Err(e) => {
            tracing::error!("Unified search failed: {e}");
            Json(serde_json::json!({
                "query": query,
                "search_type": "hybrid",
                "results": [],
                "quran_count": 0,
                "hadith_count": 0
            }))
        }
    }
}

pub async fn unified_ask(
    State(state): State<AppState>,
    Json(body): Json<AskRequest>,
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

    // Use agentic RAG: classify intent, run structured queries or fallback to semantic
    let result = ollama
        .ask_agentic(&state.db, &state.embedder, &question, model_name.as_deref())
        .await
        .map_err(|e| {
            tracing::error!("Agentic RAG ask failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    use crate::agentic_rag::AgenticResult;
    use crate::quran::models::ApiAyahSearchResult;

    let (sources_event, byte_stream) = match result {
        AgenticResult::Structured {
            narrator_sources,
            hadith_sources,
            byte_stream,
        } => {
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({
                    "narrator_sources": narrator_sources,
                    "hadith_sources": hadith_api,
                }))
                .unwrap()
            );
            (event, byte_stream)
        }
        AgenticResult::Semantic {
            ayah_sources,
            hadith_sources,
            byte_stream,
        } => {
            let quran_api: Vec<ApiAyahSearchResult> = ayah_sources
                .into_iter()
                .map(ApiAyahSearchResult::from)
                .collect();
            let hadith_api: Vec<ApiHadithSearchResult> = hadith_sources
                .into_iter()
                .map(ApiHadithSearchResult::from)
                .collect();
            let event = format!(
                "data: {}\n\n",
                serde_json::to_string(&serde_json::json!({
                    "quran_sources": quran_api,
                    "hadith_sources": hadith_api,
                }))
                .unwrap()
            );
            (event, byte_stream)
        }
    };

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
            Err(e) => {
                let err_event = format!(
                    "data: {}\n\n",
                    serde_json::to_string(&serde_json::json!({ "error": e.to_string() })).unwrap()
                );
                Ok(bytes::Bytes::from(err_event))
            }
        }));

    let body = Body::from_stream(sse_stream);

    Ok(Response::builder()
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .body(body)
        .unwrap())
}

// ── Link Preview ──

#[derive(Deserialize)]
pub struct LinkPreviewParams {
    pub url: String,
}

pub async fn link_preview(
    State(state): State<AppState>,
    Query(params): Query<LinkPreviewParams>,
) -> Result<impl IntoResponse, StatusCode> {
    use crate::models::{ApiLinkPreview, LinkPreview};

    let url = params.url.trim().to_string();
    if url.is_empty() || (!url.starts_with("http://") && !url.starts_with("https://")) {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check cache first
    let mut res = state
        .db
        .query(
            "SELECT *, <string>fetched_at AS fetched_at FROM link_preview WHERE url = $url LIMIT 1",
        )
        .bind(("url", url.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cached: Option<LinkPreview> = res.take(0).unwrap_or(None);
    if let Some(lp) = cached {
        return Ok(Json(ApiLinkPreview::from(lp)));
    }

    // Fetch the URL
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (compatible; IlmBot/1.0; +https://ilm.app)",
        )
        .send()
        .await
        .map_err(|e| {
            tracing::warn!("Link preview fetch failed for {url}: {e}");
            StatusCode::BAD_GATEWAY
        })?;

    let html = resp.text().await.map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Extract OG tags via regex
    let extract = |pattern: &str| -> Option<String> {
        regex::Regex::new(pattern)
            .ok()
            .and_then(|re| re.captures(&html))
            .and_then(|caps| caps.get(1))
            .map(|m| html_escape_decode(m.as_str()))
    };

    // Handle both attribute orders: property before content AND content before property
    let extract_og = |prop: &str| -> Option<String> {
        let p1 = format!(r#"<meta[^>]+property="{prop}"[^>]+content="([^"]*)""#);
        let p2 = format!(r#"<meta[^>]+content="([^"]*)"[^>]+property="{prop}""#);
        extract(&p1).or_else(|| extract(&p2))
    };

    let og_title = extract_og("og:title");
    let og_desc = extract_og("og:description");
    let og_image = extract_og("og:image");
    let html_title = extract(r#"<title[^>]*>([^<]*)</title>"#);

    let title = og_title.or(html_title);
    let domain = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string());

    // Cache the result (delete old + create new to handle duplicates)
    let now = crate::web::note_handlers::now_iso();
    let _ = state
        .db
        .query(
            "DELETE link_preview WHERE url = $url; \
             CREATE link_preview CONTENT {
                url: $url, title: $title, description: $desc,
                image: $image, domain: $domain, fetched_at: $now
            }",
        )
        .bind(("url", url.clone()))
        .bind(("title", title.clone()))
        .bind(("desc", og_desc.clone()))
        .bind(("image", og_image.clone()))
        .bind(("domain", domain.clone()))
        .bind(("now", now))
        .await;

    Ok(Json(ApiLinkPreview {
        url,
        title,
        description: og_desc,
        image: og_image,
        domain,
    }))
}

fn html_escape_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
}
