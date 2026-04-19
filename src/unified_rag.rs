use anyhow::Result;
use futures::stream::Stream;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::{HadithSearchResult, record_id_string};
use crate::quran::models::AyahSearchResult;
use crate::quran::surah_name;
use crate::rag::OllamaClient;

pub(crate) const CONTEXT_AYAH_COUNT: usize = 4;
pub(crate) const CONTEXT_HADITH_COUNT: usize = 4;
const CONTEXT_HADITH_COUNT_SOLO: usize = 6;
const CONTEXT_AYAH_COUNT_SOLO: usize = 6;
const MAX_TAFSIR_CHARS: usize = 1000;
const MAX_TAFSIR_CHARS_SOLO: usize = 2000;

// ── Retrieval + context builders (shared between the three scoped ask methods) ──

/// Retrieve semantically-similar hadiths and format them as an LLM context block
/// headed by `## Relevant Hadiths:`. Batch-fetches isnad chains to avoid N+1.
/// Swallows retrieval errors (returns empty sources + header-only context).
async fn retrieve_and_build_hadith_context(
    db: &Surreal<Db>,
    embedder: &Embedder,
    question: &str,
    k: usize,
) -> (Vec<HadithSearchResult>, String) {
    let hadith_sources = crate::search::search_hadiths_semantic(db, embedder, question, k)
        .await
        .unwrap_or_default();

    #[derive(Debug, SurrealValue)]
    struct NarratesRow {
        hadith: surrealdb::types::RecordId,
        name_ar: Option<String>,
        name_en: String,
    }

    let hids: Vec<surrealdb::types::RecordId> =
        hadith_sources.iter().filter_map(|h| h.id.clone()).collect();

    let chain_map: std::collections::HashMap<String, Vec<String>> = if !hids.is_empty() {
        match db
            .query(
                "SELECT out AS hadith, in.name_ar AS name_ar, in.name_en AS name_en \
                 FROM narrates WHERE out IN $hids",
            )
            .bind(("hids", hids))
            .await
        {
            Ok(mut res) => {
                let rows: Vec<NarratesRow> = res.take(0).unwrap_or_default();
                let mut map: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for row in rows {
                    let hkey = record_id_string(&row.hadith);
                    let name = row.name_ar.unwrap_or(row.name_en);
                    map.entry(hkey).or_default().push(name);
                }
                map
            }
            Err(e) => {
                tracing::error!("Batch narrator chain query failed: {e}");
                std::collections::HashMap::new()
            }
        }
    } else {
        std::collections::HashMap::new()
    };

    let mut context = String::from("## Relevant Hadiths:\n\n");
    for h in &hadith_sources {
        let narrator = h.narrator_text.as_deref().unwrap_or("Unknown narrator");

        let chain_str =
            h.id.as_ref()
                .and_then(|hid| chain_map.get(&record_id_string(hid)))
                .filter(|names| !names.is_empty())
                .map(|names| format!("Chain of narration: {}", names.join(" → ")))
                .unwrap_or_default();

        context.push_str(&format!("Hadith #{} — {}\n", h.hadith_number, narrator));
        if !chain_str.is_empty() {
            context.push_str(&format!("{chain_str}\n"));
        }
        context.push_str(&format!(
            "{}\n\n",
            h.text_en.as_deref().or(h.text_ar.as_deref()).unwrap_or("")
        ));
    }

    (hadith_sources, context)
}

/// Retrieve semantically-similar ayahs and format them as an LLM context block
/// headed by `## Relevant Quranic Verses:`, with inline tafsir_en truncated to
/// `max_tafsir_chars`. Swallows retrieval errors.
async fn retrieve_and_build_ayah_context(
    db: &Surreal<Db>,
    embedder: &Embedder,
    question: &str,
    k: usize,
    max_tafsir_chars: usize,
) -> (Vec<AyahSearchResult>, String) {
    let ayah_sources = crate::quran::search::search_ayahs_semantic(db, embedder, question, k, 0)
        .await
        .unwrap_or_default();

    let mut context = String::from("## Relevant Quranic Verses:\n\n");
    for a in &ayah_sources {
        let name = surah_name(a.surah_number);
        let text_en = a.text_en.as_deref().unwrap_or("");

        context.push_str(&format!(
            "Surah {} ({}:{}): {}\nArabic: {}\n",
            name, a.surah_number, a.ayah_number, text_en, a.text_ar,
        ));

        if let Some(ref tafsir) = a.tafsir_en
            && !tafsir.is_empty()
        {
            let truncated = if tafsir.len() > max_tafsir_chars {
                &tafsir[..tafsir.floor_char_boundary(max_tafsir_chars)]
            } else {
                tafsir
            };
            context.push_str(&format!("Tafsir Ibn Kathir: {truncated}\n"));
        }
        context.push('\n');
    }

    (ayah_sources, context)
}

impl OllamaClient {
    /// Hadith-only semantic RAG: retrieve hadiths, ground the LLM in them.
    /// Used by `/api/ask` directly and by `ask_agentic` as the fallback for
    /// `ContentQuery` under `AskScope::Hadith`.
    pub async fn ask_hadith_only(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<(
        Vec<HadithSearchResult>,
        impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>,
    )> {
        let (hadith_sources, context) =
            retrieve_and_build_hadith_context(db, embedder, question, CONTEXT_HADITH_COUNT_SOLO)
                .await;

        let system_prompt = format!(
            "You are a knowledgeable Islamic scholar specializing in hadith.\n\
             Answer questions using ONLY the hadiths provided below as context.\n\
             Always cite the hadith number when referencing a hadith.\n\
             When relevant, mention the chain of narration (isnad) to support authenticity.\n\
             If the context doesn't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n{context}"
        );

        let stream = self
            .chat_stream(&system_prompt, question, model_override)
            .await?;
        Ok((hadith_sources, stream))
    }

    /// Quran-only semantic RAG: retrieve ayahs + their inline Ibn Kathir tafsir,
    /// ground the LLM in them. Used by `/api/quran/ask` and by `ask_agentic`
    /// under `AskScope::Quran` (which skips intent classification entirely).
    pub async fn ask_quran_only(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<(
        Vec<AyahSearchResult>,
        impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>,
    )> {
        let (ayah_sources, context) = retrieve_and_build_ayah_context(
            db,
            embedder,
            question,
            CONTEXT_AYAH_COUNT_SOLO,
            MAX_TAFSIR_CHARS_SOLO,
        )
        .await;

        let system_prompt = format!(
            "You are a knowledgeable Quran scholar. Answer the user's question using ONLY \
             the Quranic verses and their tafsir (commentary by Ibn Kathir) provided below.\n\
             Always cite verse references (surah:ayah) for every claim.\n\
             If the provided verses don't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n{context}"
        );

        let stream = self
            .chat_stream(&system_prompt, question, model_override)
            .await?;
        Ok((ayah_sources, stream))
    }

    /// Retrieve relevant ayahs and hadiths, then stream an LLM answer grounded in both.
    /// Used by `/api/unified/ask` via `ask_agentic` fallback under `AskScope::Both`.
    pub async fn ask_unified(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<(
        Vec<AyahSearchResult>,
        Vec<HadithSearchResult>,
        impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>,
    )> {
        let (ayah_sources, q_ctx) = retrieve_and_build_ayah_context(
            db,
            embedder,
            question,
            CONTEXT_AYAH_COUNT,
            MAX_TAFSIR_CHARS,
        )
        .await;
        let (hadith_sources, h_ctx) =
            retrieve_and_build_hadith_context(db, embedder, question, CONTEXT_HADITH_COUNT).await;

        let system_prompt = format!(
            "You are a knowledgeable Islamic scholar. Answer the user's question using ONLY \
             the Quranic verses and hadiths provided below as context.\n\
             When citing the Quran, always reference the surah and ayah number (e.g., 2:177).\n\
             When citing a hadith, always reference the hadith number.\n\
             When relevant, mention the chain of narration (isnad) to support hadith authenticity.\n\
             Draw from BOTH the Quran and the Sunnah (Prophetic tradition) when possible.\n\
             If the context doesn't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n{q_ctx}{h_ctx}"
        );

        let stream = self
            .chat_stream(&system_prompt, question, model_override)
            .await?;
        Ok((ayah_sources, hadith_sources, stream))
    }
}
