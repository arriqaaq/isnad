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

// ── Word Morphology ──

#[derive(Debug, SurrealValue, Clone)]
pub struct QuranWord {
    pub id: Option<RecordId>,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub word_position: i64,
    pub text_ar: String,
    pub text_ar_simple: Option<String>,
    pub translation: Option<String>,
    pub transliteration: Option<String>,
    pub pos: String,
    pub root: Option<String>,
    pub lemma: Option<String>,
    pub features: Option<serde_json::Value>,
    pub segments: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiQuranWord {
    pub id: String,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub word_position: i64,
    pub text_ar: String,
    pub pos: String,
    pub root: Option<String>,
    pub lemma: Option<String>,
    pub translation: Option<String>,
    pub transliteration: Option<String>,
    pub features: Option<serde_json::Value>,
    pub segments: Option<serde_json::Value>,
}

impl From<QuranWord> for ApiQuranWord {
    fn from(w: QuranWord) -> Self {
        Self {
            id: w.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            surah_number: w.surah_number,
            ayah_number: w.ayah_number,
            word_position: w.word_position,
            text_ar: w.text_ar,
            pos: w.pos,
            root: w.root,
            lemma: w.lemma,
            translation: w.translation,
            transliteration: w.transliteration,
            features: w.features,
            segments: w.segments.and_then(|s| serde_json::from_str(&s).ok()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RootSearchResponse {
    pub root: String,
    pub occurrences: Vec<ApiQuranWord>,
    pub ayah_count: usize,
}

// ── Reciter ──

#[derive(Debug, SurrealValue, Clone)]
pub struct Reciter {
    pub id: Option<RecordId>,
    pub name_en: String,
    pub name_ar: Option<String>,
    pub style: Option<String>,
    pub folder_name: String,
    pub bitrate: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiReciter {
    pub id: String,
    pub name_en: String,
    pub name_ar: Option<String>,
    pub style: Option<String>,
    pub folder_name: String,
    pub bitrate: Option<String>,
}

impl From<Reciter> for ApiReciter {
    fn from(r: Reciter) -> Self {
        Self {
            id: r.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_en: r.name_en,
            name_ar: r.name_ar,
            style: r.style,
            folder_name: r.folder_name,
            bitrate: r.bitrate,
        }
    }
}

// ── Manuscript ──

#[derive(Debug, SurrealValue, Clone)]
pub struct Manuscript {
    pub id: Option<RecordId>,
    pub name: String,
    pub repository: Option<String>,
    pub location: Option<String>,
    pub date_range: Option<String>,
    pub material: Option<String>,
    pub script_type: Option<String>,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub surah_start: Option<i64>,
    pub surah_end: Option<i64>,
    pub ayah_start: Option<i64>,
    pub ayah_end: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ApiManuscript {
    pub id: String,
    pub name: String,
    pub repository: Option<String>,
    pub location: Option<String>,
    pub date_range: Option<String>,
    pub material: Option<String>,
    pub script_type: Option<String>,
    pub description: Option<String>,
}

impl From<Manuscript> for ApiManuscript {
    fn from(m: Manuscript) -> Self {
        Self {
            id: m.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name: m.name,
            repository: m.repository,
            location: m.location,
            date_range: m.date_range,
            material: m.material,
            script_type: m.script_type,
            description: m.description,
        }
    }
}

// ── Variant Reading ──

#[derive(Debug, SurrealValue, Clone)]
pub struct VariantReading {
    pub id: Option<RecordId>,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub word_position: Option<i64>,
    pub reader_name: String,
    pub reading_ar: String,
    pub standard_ar: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiVariantReading {
    pub id: String,
    pub surah_number: i64,
    pub ayah_number: i64,
    pub reader_name: String,
    pub reading_ar: String,
    pub standard_ar: Option<String>,
    pub source: Option<String>,
}

impl From<VariantReading> for ApiVariantReading {
    fn from(v: VariantReading) -> Self {
        Self {
            id: v.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            surah_number: v.surah_number,
            ayah_number: v.ayah_number,
            reader_name: v.reader_name,
            reading_ar: v.reading_ar,
            standard_ar: v.standard_ar,
            source: v.source,
        }
    }
}

// ── Similar Ayahs / Mutashabihat ──

#[derive(Debug, SurrealValue, Clone)]
pub struct QuranPhrase {
    pub id: Option<RecordId>,
    pub text_ar: String,
    pub text_ar_simple: Option<String>,
    pub occurrence: i64,
    pub verses_count: i64,
    pub chapters_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ApiQuranPhrase {
    pub id: String,
    pub text_ar: String,
    pub occurrence: i64,
    pub verses_count: i64,
    pub chapters_count: i64,
}

impl From<QuranPhrase> for ApiQuranPhrase {
    fn from(p: QuranPhrase) -> Self {
        Self {
            id: p.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            text_ar: p.text_ar,
            occurrence: p.occurrence,
            verses_count: p.verses_count,
            chapters_count: p.chapters_count,
        }
    }
}

#[derive(Debug, SurrealValue, Clone)]
pub struct SimilarAyahEdge {
    pub id: Option<RecordId>,
    pub score: i64,
    pub coverage: i64,
    pub matched_positions: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiSimilarAyah {
    pub ayah_key: String,
    pub score: i64,
    pub coverage: i64,
    pub matched_positions: Option<serde_json::Value>,
    pub text_ar: Option<String>,
    pub text_en: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AyahSimilarResponse {
    pub similar: Vec<ApiSimilarAyah>,
    pub phrases: Vec<ApiPhraseWithAyahs>,
}

#[derive(Debug, Serialize)]
pub struct ApiPhraseWithAyahs {
    pub id: String,
    pub text_ar: String,
    pub occurrence: i64,
    pub ayah_keys: Vec<String>,
}
