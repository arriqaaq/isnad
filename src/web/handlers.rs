use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use futures::StreamExt;
use serde::Deserialize;
use surrealdb::types::{RecordId, SurrealValue};

use crate::models::{
    ApiBook, ApiHadith, ApiHadithSearchResult, ApiNarrator, ApiNarratorSearchResult,
    ApiNarratorWithCount, Book, GraphData, GraphEdge, GraphEdgeData, GraphNode, GraphNodeData,
    Hadith, Narrator, PaginatedResponse, StatsResponse, record_id_key_string, record_id_string,
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
}

#[derive(Deserialize)]
pub struct ListParams {
    pub book: Option<i64>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
    pub q: Option<String>,
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
            "SELECT * FROM hadith WHERE book_id = {book_id} \
             ORDER BY hadith_number ASC LIMIT {limit} START {offset}"
        )
    } else {
        format!(
            "SELECT * FROM hadith ORDER BY hadith_number ASC \
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

    let mut res = state
        .db
        .query("SELECT * FROM $rid")
        .bind(("rid", hrid.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Hadith detail query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let hadith: Option<Hadith> = res.take(0).unwrap_or(None);
    let hadith = hadith.ok_or(StatusCode::NOT_FOUND)?;

    let narrators = match state
        .db
        .query("SELECT <-narrates<-narrator.* AS narrators FROM $rid")
        .bind(("rid", hrid))
        .await
    {
        Ok(mut r) => {
            let result: Option<NarratorsResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.narrators).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Narrator query for hadith failed: {e}");
            vec![]
        }
    };

    Ok(Json(serde_json::json!({
        "hadith": ApiHadith::from(hadith),
        "narrators": narrators.into_iter().map(ApiNarrator::from).collect::<Vec<_>>()
    })))
}

pub async fn narrator_list(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50);
    let offset = (page - 1) * limit;

    let narrators: Vec<NarratorWithCount> = if let Some(q) = &params.q {
        match state
            .db
            .query(
                "SELECT *, count(->narrates->hadith) AS hadith_count FROM narrator \
                 WHERE string::lowercase(name_en) CONTAINS string::lowercase($q) \
                    OR name_ar CONTAINS $q \
                 ORDER BY hadith_count DESC LIMIT $limit START $offset",
            )
            .bind(("q", q.clone()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
        {
            Ok(mut r) => r.take(0).unwrap_or_default(),
            Err(e) => {
                tracing::error!("Narrator search query failed: {e}");
                vec![]
            }
        }
    } else {
        match state
            .db
            .query(
                "SELECT *, count(->narrates->hadith) AS hadith_count FROM narrator \
                 ORDER BY hadith_count DESC LIMIT $limit START $offset",
            )
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
        {
            Ok(mut r) => r.take(0).unwrap_or_default(),
            Err(e) => {
                tracing::error!("Narrator list query failed: {e}");
                vec![]
            }
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

    let mut res = state
        .db
        .query("SELECT * FROM $rid")
        .bind(("rid", nrid.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Narrator detail query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let narrator: Option<Narrator> = res.take(0).unwrap_or(None);
    let narrator = narrator.ok_or(StatusCode::NOT_FOUND)?;

    let hadiths = match state
        .db
        .query("SELECT ->narrates->hadith.* AS hadiths FROM $rid")
        .bind(("rid", nrid.clone()))
        .await
    {
        Ok(mut r) => {
            let result: Option<HadithsResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.hadiths).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Hadiths for narrator query failed: {e}");
            vec![]
        }
    };

    let teachers = match state
        .db
        .query("SELECT ->heard_from->narrator.* AS teachers FROM $rid")
        .bind(("rid", nrid.clone()))
        .await
    {
        Ok(mut r) => {
            let result: Option<TeachersResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.teachers).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Teachers query failed: {e}");
            vec![]
        }
    };

    let students = match state
        .db
        .query("SELECT <-heard_from<-narrator.* AS students FROM $rid")
        .bind(("rid", nrid))
        .await
    {
        Ok(mut r) => {
            let result: Option<StudentsResult> = r.take(0).unwrap_or(None);
            result.map(|r| r.students).unwrap_or_default()
        }
        Err(e) => {
            tracing::error!("Students query failed: {e}");
            vec![]
        }
    };

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
        .query("SELECT in AS in_id, out AS out_id FROM heard_from WHERE hadith_ref = $rid")
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
    };

    for narrator in &narrators {
        if let Some(id) = &narrator.id {
            graph.nodes.push(GraphNode {
                data: GraphNodeData {
                    id: record_id_string(id),
                    label: narrator.name_en.clone(),
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
             SELECT ->heard_from->narrator.* AS teachers FROM $rid; \
             SELECT <-heard_from<-narrator.* AS students FROM $rid;",
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

    let mut graph = GraphData {
        nodes: Vec::new(),
        edges: Vec::new(),
    };

    if let Some(narrator) = &narrator {
        if let Some(nid) = &narrator.id {
            let nid_str = record_id_string(nid);
            graph.nodes.push(GraphNode {
                data: GraphNodeData {
                    id: nid_str.clone(),
                    label: narrator.name_en.clone(),
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
                            label: teacher.name_en.clone(),
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
                            label: student.name_en.clone(),
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
                        },
                    });
                }
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
                        if let Some(msg) = parsed.message {
                            if !msg.content.is_empty() {
                                sse.push_str(&format!(
                                    "data: {}\n\n",
                                    serde_json::to_string(
                                        &serde_json::json!({ "text": msg.content })
                                    )
                                    .unwrap()
                                ));
                            }
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
}
