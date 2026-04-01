use serde::Serialize;
use surrealdb::types::{RecordId, SurrealValue};

use crate::models::record_id_key_string;

// ── Database record types ──

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Surah {
    pub id: Option<RecordId>,
    pub surah_number: i64,
    pub name_ar: String,
    pub name_en: String,
    pub name_translit: String,
    pub revelation_type: String,
    pub ayah_count: i64,
}

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Ayah {
    pub id: Option<RecordId>,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub text_ar: String,
    pub text_ar_simple: Option<String>,
    pub text_en: Option<String>,
    pub tafsir_en: Option<String>,
    pub text_ar_tajweed: Option<String>,
    pub juz: Option<i64>,
    pub hizb: Option<i64>,
}

#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct AyahSearchResult {
    pub id: Option<RecordId>,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub text_ar: String,
    pub text_en: Option<String>,
    pub tafsir_en: Option<String>,
    pub text_ar_tajweed: Option<String>,
    pub score: Option<f64>,
}

// ── API response types (RecordId flattened to String) ──

#[derive(Debug, Serialize)]
pub struct ApiSurah {
    pub id: String,
    pub surah_number: i64,
    pub name_ar: String,
    pub name_en: String,
    pub name_translit: String,
    pub revelation_type: String,
    pub ayah_count: i64,
}

impl From<Surah> for ApiSurah {
    fn from(s: Surah) -> Self {
        Self {
            id: s.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            surah_number: s.surah_number,
            name_ar: s.name_ar,
            name_en: s.name_en,
            name_translit: s.name_translit,
            revelation_type: s.revelation_type,
            ayah_count: s.ayah_count,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiAyah {
    pub id: String,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub text_ar: String,
    pub text_en: Option<String>,
    pub tafsir_en: Option<String>,
    pub text_ar_tajweed: Option<String>,
}

impl From<Ayah> for ApiAyah {
    fn from(a: Ayah) -> Self {
        Self {
            id: a.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            surah_number: a.surah_number,
            ayah_number: a.ayah_number,
            text_ar: a.text_ar,
            text_en: a.text_en,
            tafsir_en: a.tafsir_en,
            text_ar_tajweed: a.text_ar_tajweed,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApiAyahSearchResult {
    pub id: String,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub text_ar: String,
    pub text_en: Option<String>,
    pub tafsir_en: Option<String>,
    pub text_ar_tajweed: Option<String>,
    pub score: Option<f64>,
}

impl From<AyahSearchResult> for ApiAyahSearchResult {
    fn from(a: AyahSearchResult) -> Self {
        Self {
            id: a.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            surah_number: a.surah_number,
            ayah_number: a.ayah_number,
            text_ar: a.text_ar,
            text_en: a.text_en,
            tafsir_en: a.tafsir_en,
            text_ar_tajweed: a.text_ar_tajweed,
            score: a.score,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct QuranSearchResponse {
    pub query: String,
    pub search_type: String,
    pub ayahs: Vec<ApiAyahSearchResult>,
}

#[derive(Debug, Serialize)]
pub struct QuranStatsResponse {
    pub surah_count: i64,
    pub ayah_count: i64,
}

#[derive(Debug, Serialize)]
pub struct SurahDetailResponse {
    pub surah: ApiSurah,
    pub ayahs: Vec<ApiAyah>,
}
