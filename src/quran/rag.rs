use anyhow::Result;
use futures::stream::Stream;
use surrealdb::Surreal;

use crate::db::Db;
use crate::embed::Embedder;
use crate::rag::OllamaClient;

use super::models::AyahSearchResult;
use super::search::search_ayahs_semantic;
use super::surah_name;

const CONTEXT_AYAH_COUNT: usize = 6;
const MAX_TAFSIR_CHARS: usize = 2000;

impl OllamaClient {
    /// Retrieve relevant ayahs and stream an LLM answer about the Quran.
    pub async fn ask_quran(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<(
        Vec<AyahSearchResult>,
        impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>,
    )> {
        // 1. Retrieve relevant ayahs via semantic search
        let sources = search_ayahs_semantic(db, embedder, question, CONTEXT_AYAH_COUNT, 0).await?;

        // 2. Build context from retrieved ayahs with tafsir
        let mut context = String::new();
        for a in &sources {
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

        let system_prompt = format!(
            "You are a knowledgeable Quran scholar. Answer the user's question using ONLY \
             the provided Quranic verses and their tafsir (commentary by Ibn Kathir).\n\
             Always cite verse references (surah:ayah) for every claim.\n\
             If the provided verses don't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n\
             ## Relevant Quranic Verses:\n\n{context}"
        );

        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        // 3. Call Ollama chat API with streaming
        use serde::Serialize;

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

        Ok((sources, response.bytes_stream()))
    }
}
