use serde::Serialize;
use surrealdb::types::{RecordId, RecordIdKey, SurrealValue};

/// Format a RecordId's key as a string (for URLs, graph IDs, etc.)
pub fn record_id_key_string(id: &RecordId) -> String {
    match &id.key {
        RecordIdKey::String(s) => s.clone(),
        RecordIdKey::Number(n) => n.to_string(),
        _ => format!("{:?}", id.key),
    }
}

/// Format a full RecordId as "table:key" string
pub fn record_id_string(id: &RecordId) -> String {
    format!("{}:{}", id.table.as_str(), record_id_key_string(id))
}

// ── Database record types ──

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Narrator {
    pub id: Option<RecordId>,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub search_name: Option<String>,
    pub gender: Option<String>,
    pub generation: Option<String>,
    pub bio: Option<String>,
    // Biographical fields
    pub kunya: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub birth_year: Option<i64>,
    pub birth_calendar: Option<String>,
    pub death_year: Option<i64>,
    pub death_calendar: Option<String>,
    pub locations: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    // Reliability fields
    pub reliability_rating: Option<String>,
    pub reliability_prior: Option<f64>,
    pub reliability_source: Option<String>,
}

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Hadith {
    pub id: Option<RecordId>,
    pub hadith_number: i64,
    pub book_id: i64,
    pub chapter_id: i64,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
    pub narrator_text: Option<String>,
    pub grade: Option<String>,
    pub book_name: Option<String>,
    pub matn: Option<String>,
}

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Book {
    pub id: Option<RecordId>,
    pub book_number: i64,
    pub name_en: String,
    pub name_ar: Option<String>,
}

// ── Analysis types ──

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct HadithFamily {
    pub id: Option<RecordId>,
    pub family_label: Option<String>,
    pub variant_count: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiHadithFamily {
    pub id: String,
    pub family_label: Option<String>,
    pub variant_count: Option<i64>,
}

impl From<HadithFamily> for ApiHadithFamily {
    fn from(f: HadithFamily) -> Self {
        Self {
            id: f.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            family_label: f.family_label,
            variant_count: f.variant_count,
        }
    }
}

// ── Search result types ──

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct HadithSearchResult {
    pub id: Option<RecordId>,
    pub hadith_number: i64,
    pub book_id: i64,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
    pub narrator_text: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct NarratorSearchResult {
    pub id: Option<RecordId>,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
    pub hadith_count: Option<i64>,
}

// ── Graph data for Cytoscape.js ──

#[derive(Debug, Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Serialize)]
pub struct GraphNode {
    pub data: GraphNodeData,
}

#[derive(Debug, Serialize)]
pub struct GraphNodeData {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub generation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub data: GraphEdgeData,
}

#[derive(Debug, Serialize)]
pub struct GraphEdgeData {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
}

// ── API response types (RecordId flattened to String) ──

#[derive(Debug, Serialize)]
pub struct ApiNarrator {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
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
    pub reliability_rating: Option<String>,
    pub reliability_prior: Option<f64>,
    pub reliability_source: Option<String>,
}

impl From<Narrator> for ApiNarrator {
    fn from(n: Narrator) -> Self {
        Self {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            gender: n.gender,
            generation: n.generation,
            bio: n.bio,
            kunya: n.kunya,
            aliases: n.aliases,
            birth_year: n.birth_year,
            birth_calendar: n.birth_calendar,
            death_year: n.death_year,
            death_calendar: n.death_calendar,
            locations: n.locations,
            tags: n.tags,
            reliability_rating: n.reliability_rating,
            reliability_prior: n.reliability_prior,
            reliability_source: n.reliability_source,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiHadith {
    pub id: String,
    pub hadith_number: i64,
    pub book_id: i64,
    pub chapter_id: i64,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
    pub narrator_text: Option<String>,
    pub grade: Option<String>,
    pub book_name: Option<String>,
    pub matn: Option<String>,
}

impl From<Hadith> for ApiHadith {
    fn from(h: Hadith) -> Self {
        Self {
            id: h.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            hadith_number: h.hadith_number,
            book_id: h.book_id,
            chapter_id: h.chapter_id,
            text_ar: h.text_ar,
            text_en: h.text_en,
            narrator_text: h.narrator_text,
            grade: h.grade,
            book_name: h.book_name,
            matn: h.matn,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiBook {
    pub id: String,
    pub book_number: i64,
    pub name_en: String,
    pub name_ar: Option<String>,
}

impl From<Book> for ApiBook {
    fn from(b: Book) -> Self {
        Self {
            id: b.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            book_number: b.book_number,
            name_en: b.name_en,
            name_ar: b.name_ar,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiNarratorWithCount {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
    pub bio: Option<String>,
    pub hadith_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiHadithSearchResult {
    pub id: String,
    pub hadith_number: i64,
    pub book_id: i64,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
    pub narrator_text: Option<String>,
    pub score: Option<f64>,
}

impl From<HadithSearchResult> for ApiHadithSearchResult {
    fn from(h: HadithSearchResult) -> Self {
        Self {
            id: h.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            hadith_number: h.hadith_number,
            book_id: h.book_id,
            text_ar: h.text_ar,
            text_en: h.text_en,
            narrator_text: h.narrator_text,
            score: h.score,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiNarratorSearchResult {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
    pub hadith_count: Option<i64>,
}

impl From<NarratorSearchResult> for ApiNarratorSearchResult {
    fn from(n: NarratorSearchResult) -> Self {
        Self {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
            hadith_count: n.hadith_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: usize,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub hadith_count: i64,
    pub narrator_count: i64,
    pub book_count: i64,
}
