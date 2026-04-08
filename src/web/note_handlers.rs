use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use serde::Deserialize;
use surrealdb::types::{RecordId, SurrealValue};

use crate::models::{ApiUserNote, NoteRef, PaginatedResponse, UserNote};

use super::AppState;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

fn now_iso() -> String {
    // Generate ISO 8601 timestamp without chrono crate
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Convert to UTC date-time components
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let mins = (time_secs % 3600) / 60;
    let s = time_secs % 60;
    // Days since epoch to Y-M-D (simplified leap year calc)
    let mut y = 1970i64;
    let mut remaining_days = days as i64;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for md in month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{hours:02}:{mins:02}:{s:02}Z",
        m + 1,
        remaining_days + 1
    )
}

fn extract_device_id(headers: &HeaderMap) -> Result<String, StatusCode> {
    headers
        .get("X-Device-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or(StatusCode::BAD_REQUEST)
}

// ── Query parameter types ──

#[derive(Deserialize)]
pub struct NoteListParams {
    pub ref_type: Option<String>,
    pub ref_id: Option<String>,
    pub tag: Option<String>,
    pub color: Option<String>,
    pub q: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct BulkRefsParams {
    pub ref_type: String,
    pub ref_ids: String, // comma-separated
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub ref_type: String,
    pub ref_id: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub refs: Option<Vec<NoteRef>>,
}

#[derive(Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub color: Option<String>,
    pub tags: Option<Vec<String>>,
    pub refs: Option<Vec<NoteRef>>,
}

#[derive(Deserialize)]
pub struct UpdateNoteRefsRequest {
    pub action: String, // "add" | "remove"
    #[serde(rename = "ref")]
    pub note_ref: NoteRef,
}

#[derive(Deserialize)]
pub struct UpdateRefAnnotationRequest {
    pub annotation: String,
}

// ── Handlers ──

pub async fn list_notes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<NoteListParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20);
    let offset = (page - 1) * limit;

    let mut conditions = vec!["device_id = $did".to_string()];
    if let Some(ref rt) = params.ref_type {
        let _ = rt;
        conditions.push("ref_type = $ref_type".to_string());
    }
    if let Some(ref ri) = params.ref_id {
        let _ = ri;
        conditions.push("ref_id = $ref_id".to_string());
    }
    if let Some(ref color) = params.color {
        let _ = color;
        conditions.push("color = $color".to_string());
    }
    if let Some(ref q) = params.q {
        let _ = q;
        conditions.push(
            "(string::lowercase(content) CONTAINS string::lowercase($q) OR string::lowercase(title) CONTAINS string::lowercase($q))"
                .to_string(),
        );
    }
    if let Some(ref tag) = params.tag {
        let _ = tag;
        conditions.push("$tag IN tags".to_string());
    }

    let where_clause = format!("WHERE {}", conditions.join(" AND "));
    // Cast created_at/updated_at to string to handle legacy datetime values
    let query_str = format!(
        "SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at \
         FROM user_note {where_clause} ORDER BY updated_at DESC LIMIT {limit} START {offset}"
    );

    let mut query = state.db.query(&query_str).bind(("did", did));
    if let Some(ref rt) = params.ref_type {
        query = query.bind(("ref_type", rt.clone()));
    }
    if let Some(ref ri) = params.ref_id {
        query = query.bind(("ref_id", ri.clone()));
    }
    if let Some(ref color) = params.color {
        query = query.bind(("color", color.clone()));
    }
    if let Some(ref q) = params.q {
        query = query.bind(("q", q.clone()));
    }
    if let Some(ref tag) = params.tag {
        query = query.bind(("tag", tag.clone()));
    }

    let notes: Vec<UserNote> = match query.await {
        Ok(mut r) => match r.take::<Vec<UserNote>>(0) {
            Ok(notes) => notes,
            Err(e) => {
                tracing::error!("Note list deserialization failed: {e}");
                vec![]
            }
        },
        Err(e) => {
            tracing::error!("Note list query failed: {e}");
            vec![]
        }
    };
    let has_more = notes.len() == limit;

    Ok(Json(PaginatedResponse {
        data: notes.into_iter().map(ApiUserNote::from).collect(),
        page,
        has_more,
    }))
}

pub async fn get_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    let mut res = state
        .db
        .query("SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at FROM $rid WHERE device_id = $did")
        .bind(("rid", rid("user_note", &id)))
        .bind(("did", did))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ApiUserNote::from(note)))
}

pub async fn create_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateNoteRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;
    let content = body.content.unwrap_or_default();
    let color = body.color.unwrap_or_else(|| "yellow".to_string());
    let refs_json = body
        .refs
        .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| "[]".to_string()));

    let now = now_iso();

    let mut res = state
        .db
        .query(
            "CREATE user_note CONTENT {
                device_id: $did,
                ref_type: $ref_type,
                ref_id: $ref_id,
                title: $title,
                content: $content,
                color: $color,
                tags: $tags,
                refs: $refs,
                created_at: $now,
                updated_at: $now
            }",
        )
        .bind(("did", did))
        .bind(("ref_type", body.ref_type))
        .bind(("ref_id", body.ref_id))
        .bind(("title", body.title))
        .bind(("content", content))
        .bind(("color", color))
        .bind(("tags", body.tags))
        .bind(("refs", refs_json))
        .bind(("now", now))
        .await
        .map_err(|e| {
            tracing::error!("Create note failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let note: Option<UserNote> = res.take(0).map_err(|e| {
        tracing::error!("Create note deserialization failed: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let note = note.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiUserNote::from(note)))
}

pub async fn update_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    let mut update = serde_json::Map::new();
    if let Some(ref title) = body.title {
        update.insert("title".to_string(), serde_json::json!(title));
    }
    if let Some(ref content) = body.content {
        update.insert("content".to_string(), serde_json::json!(content));
    }
    if let Some(ref color) = body.color {
        update.insert("color".to_string(), serde_json::json!(color));
    }
    if let Some(ref tags) = body.tags {
        update.insert("tags".to_string(), serde_json::json!(tags));
    }
    if let Some(ref refs) = body.refs {
        let refs_json = serde_json::to_string(refs).unwrap_or_else(|_| "[]".to_string());
        update.insert("refs".to_string(), serde_json::json!(refs_json));
    }

    if update.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // MERGE and SET cannot be combined in SurrealDB — do MERGE first, then SET updated_at
    state
        .db
        .query("UPDATE $rid MERGE $data WHERE device_id = $did")
        .bind(("rid", rid("user_note", &id)))
        .bind(("data", serde_json::Value::Object(update)))
        .bind(("did", did.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Update note MERGE failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut res = state
        .db
        .query("UPDATE $rid SET updated_at = $updated_at WHERE device_id = $did RETURN AFTER")
        .bind(("rid", rid("user_note", &id)))
        .bind(("did", did))
        .bind(("updated_at", now_iso()))
        .await
        .map_err(|e| {
            tracing::error!("Update note SET failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(ApiUserNote::from(note)))
}

pub async fn delete_note(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    // Verify ownership before deleting
    let mut res = state
        .db
        .query("SELECT device_id FROM $rid")
        .bind(("rid", rid("user_note", &id)))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    #[derive(Debug, SurrealValue)]
    struct DeviceRow {
        device_id: String,
    }
    let row: Option<DeviceRow> = res.take(0).unwrap_or(None);
    let row = row.ok_or(StatusCode::NOT_FOUND)?;
    if row.device_id != did {
        return Err(StatusCode::FORBIDDEN);
    }

    state
        .db
        .query("DELETE $rid")
        .bind(("rid", rid("user_note", &id)))
        .await
        .map_err(|e| {
            tracing::error!("Delete note failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn bulk_note_refs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(params): Query<BulkRefsParams>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;
    let ref_ids: Vec<String> = params
        .ref_ids
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if ref_ids.is_empty() {
        return Ok(Json(serde_json::json!({})));
    }

    #[derive(Debug, SurrealValue)]
    struct RefCount {
        ref_id: Option<String>,
        color: String,
        count: i64,
    }

    let mut res = state
        .db
        .query(
            "SELECT ref_id, color, count() AS count FROM user_note \
             WHERE device_id = $did AND ref_type = $ref_type AND ref_id IN $ref_ids \
             GROUP BY ref_id, color",
        )
        .bind(("did", did.clone()))
        .bind(("ref_type", params.ref_type.clone()))
        .bind(("ref_ids", ref_ids.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Bulk note refs query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let counts: Vec<RefCount> = res.take(0).unwrap_or_default();

    // Also check which ref_ids appear in topic notes' refs JSON
    // For simplicity, we build a map from the anchored notes query
    let mut result = serde_json::Map::new();
    for rc in counts {
        if let Some(ref_id) = rc.ref_id {
            result.insert(
                ref_id,
                serde_json::json!({ "color": rc.color, "count": rc.count }),
            );
        }
    }

    Ok(Json(serde_json::Value::Object(result)))
}

pub async fn list_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    let mut res = state
        .db
        .query("SELECT tags FROM user_note WHERE device_id = $did AND tags IS NOT NONE")
        .bind(("did", did))
        .await
        .map_err(|e| {
            tracing::error!("List tags query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    #[derive(Debug, SurrealValue)]
    struct TagRow {
        tags: Option<Vec<String>>,
    }

    let rows: Vec<TagRow> = res.take(0).unwrap_or_default();
    let mut all_tags: Vec<String> = rows
        .into_iter()
        .flat_map(|r| r.tags.unwrap_or_default())
        .collect();
    all_tags.sort();
    all_tags.dedup();

    Ok(Json(all_tags))
}

pub async fn export_notes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    let notes: Vec<UserNote> = match state
        .db
        .query("SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at FROM user_note WHERE device_id = $did ORDER BY updated_at DESC")
        .bind(("did", did))
        .await
    {
        Ok(mut r) => r.take(0).unwrap_or_default(),
        Err(e) => {
            tracing::error!("Export notes query failed: {e}");
            vec![]
        }
    };

    Ok(Json(
        notes.into_iter().map(ApiUserNote::from).collect::<Vec<_>>(),
    ))
}

pub async fn update_note_refs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<UpdateNoteRefsRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    // Fetch current note
    let mut res = state
        .db
        .query("SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at FROM $rid WHERE device_id = $did")
        .bind(("rid", rid("user_note", &id)))
        .bind(("did", did.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::NOT_FOUND)?;

    let mut refs: Vec<NoteRef> = note
        .refs
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    match body.action.as_str() {
        "add" => {
            // Don't add duplicates
            let exists = refs
                .iter()
                .any(|r| r.ref_type == body.note_ref.ref_type && r.ref_id == body.note_ref.ref_id);
            if !exists {
                refs.push(body.note_ref);
            }
        }
        "remove" => {
            refs.retain(|r| {
                !(r.ref_type == body.note_ref.ref_type && r.ref_id == body.note_ref.ref_id)
            });
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }

    let refs_json = serde_json::to_string(&refs).unwrap_or_else(|_| "[]".to_string());

    let mut res = state
        .db
        .query("UPDATE $rid SET refs = $refs, updated_at = $updated_at WHERE device_id = $did RETURN AFTER")
        .bind(("rid", rid("user_note", &id)))
        .bind(("refs", refs_json))
        .bind(("did", did))
        .bind(("updated_at", now_iso()))
        .await
        .map_err(|e| {
            tracing::error!("Update note refs failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiUserNote::from(note)))
}

pub async fn update_ref_annotation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((id, idx)): Path<(String, usize)>,
    Json(body): Json<UpdateRefAnnotationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let did = extract_device_id(&headers)?;

    let mut res = state
        .db
        .query("SELECT *, <string>created_at AS created_at, <string>updated_at AS updated_at FROM $rid WHERE device_id = $did")
        .bind(("rid", rid("user_note", &id)))
        .bind(("did", did.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::NOT_FOUND)?;

    let mut refs: Vec<NoteRef> = note
        .refs
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    if idx >= refs.len() {
        return Err(StatusCode::BAD_REQUEST);
    }

    refs[idx].annotation = Some(body.annotation);
    let refs_json = serde_json::to_string(&refs).unwrap_or_else(|_| "[]".to_string());

    let mut res = state
        .db
        .query("UPDATE $rid SET refs = $refs, updated_at = $updated_at WHERE device_id = $did RETURN AFTER")
        .bind(("rid", rid("user_note", &id)))
        .bind(("refs", refs_json))
        .bind(("did", did))
        .bind(("updated_at", now_iso()))
        .await
        .map_err(|e| {
            tracing::error!("Update ref annotation failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let note: Option<UserNote> = res.take(0).unwrap_or(None);
    let note = note.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ApiUserNote::from(note)))
}
