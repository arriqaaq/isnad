use anyhow::Result;
use futures::stream::Stream;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::{HadithSearchResult, record_id_string};
use crate::quran::models::AyahSearchResult;
use crate::quran::surah_name;
use crate::rag::OllamaClient;

const CONTEXT_AYAH_COUNT: usize = 4;
const CONTEXT_HADITH_COUNT: usize = 4;
const MAX_TAFSIR_CHARS: usize = 1000;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Serialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

impl OllamaClient {
    /// Retrieve relevant ayahs and hadiths, then stream an LLM answer grounded in both.
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
        // 1. Retrieve from both sources sequentially (HNSW needs deep stack)
        let ayah_sources =
            crate::quran::search::search_ayahs_semantic(db, embedder, question, CONTEXT_AYAH_COUNT, 0)
                .await
                .unwrap_or_default();
        let hadith_sources =
            crate::search::search_hadiths_semantic(db, embedder, question, CONTEXT_HADITH_COUNT)
                .await
                .unwrap_or_default();

        // 2. Build Quran context
        let mut context = String::new();
        context.push_str("## Relevant Quranic Verses:\n\n");
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
                let truncated = if tafsir.len() > MAX_TAFSIR_CHARS {
                    &tafsir[..tafsir.floor_char_boundary(MAX_TAFSIR_CHARS)]
                } else {
                    tafsir
                };
                context.push_str(&format!("Tafsir Ibn Kathir: {truncated}\n"));
            }
            context.push('\n');
        }

        // 3. Batch-fetch narrator chains for all hadith sources (avoids N+1)
        let hids: Vec<surrealdb::types::RecordId> =
            hadith_sources.iter().filter_map(|h| h.id.clone()).collect();

        #[derive(Debug, SurrealValue)]
        struct NarratesRow {
            hadith: surrealdb::types::RecordId,
            name_ar: Option<String>,
            name_en: String,
        }

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

        // Build Hadith context with narrator chains
        context.push_str("## Relevant Hadiths:\n\n");
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

        // 4. System prompt emphasizing dual sourcing
        let system_prompt = format!(
            "You are a knowledgeable Islamic scholar. Answer the user's question using ONLY \
             the Quranic verses and hadiths provided below as context.\n\
             When citing the Quran, always reference the surah and ayah number (e.g., 2:177).\n\
             When citing a hadith, always reference the hadith number.\n\
             When relevant, mention the chain of narration (isnad) to support hadith authenticity.\n\
             Draw from BOTH the Quran and the Sunnah (Prophetic tradition) when possible.\n\
             If the context doesn't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n\
             {context}"
        );

        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        // 5. Call Ollama chat API with streaming
        let request = ChatRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt,
                },
                ChatMessage {
                    role: "user".into(),
                    content: question.to_string(),
                },
            ],
            stream: true,
        };

        let response = self
            .http
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error {status}: {body}");
        }

        Ok((ayah_sources, hadith_sources, response.bytes_stream()))
    }
}
