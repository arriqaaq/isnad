use anyhow::Result;
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use surrealdb::Surreal;
use surrealdb::types::SurrealValue;

use crate::db::Db;
use crate::embed::Embedder;
use crate::models::HadithSearchResult;
use crate::models::record_id_string;
use crate::search::search_hadiths_semantic;

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.2";
const CONTEXT_HADITH_COUNT: usize = 6;

#[derive(Clone)]
pub struct OllamaClient {
    pub http: Client,
    pub base_url: String,
    pub model: String,
}

#[derive(Serialize)]
pub(crate) struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Serialize, Clone)]
pub(crate) struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct ChatChunk {
    pub message: Option<ChatChunkMessage>,
    pub done: bool,
}

#[derive(Deserialize)]
pub struct ChatChunkMessage {
    pub content: String,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
    }

    /// Non-streaming Ollama call with JSON format. Used for classification.
    pub async fn chat_json(
        &self,
        system: &str,
        user: &str,
        model_override: Option<&str>,
    ) -> Result<serde_json::Value> {
        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        let request = ChatRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: user.to_string(),
                },
            ],
            stream: false,
            format: Some("json".to_string()),
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
            anyhow::bail!("Ollama chat_json error {status}: {body}");
        }

        let body: serde_json::Value = response.json().await?;
        let content = body["message"]["content"].as_str().unwrap_or("{}");
        let parsed: serde_json::Value =
            serde_json::from_str(content).unwrap_or(serde_json::json!({"intent": "content"}));
        Ok(parsed)
    }

    /// Streaming Ollama call with a pre-built system prompt. Used by agentic RAG.
    pub async fn chat_stream(
        &self,
        system_prompt: &str,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>> {
        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        let request = ChatRequest {
            model,
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system_prompt.to_string(),
                },
                ChatMessage {
                    role: "user".into(),
                    content: question.to_string(),
                },
            ],
            stream: true,
            format: None,
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

        Ok(response.bytes_stream())
    }

    /// Retrieve relevant hadiths and stream an LLM answer.
    /// Returns (sources, response_stream) where the stream is fully owned (no borrows).
    pub async fn ask(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<(
        Vec<HadithSearchResult>,
        impl Stream<Item = Result<bytes::Bytes, reqwest::Error>> + use<>,
    )> {
        // 1. Retrieve relevant hadiths via semantic search
        let sources = search_hadiths_semantic(db, embedder, question, CONTEXT_HADITH_COUNT).await?;

        // 2. Batch-fetch narrator chains for all source hadiths (avoids N+1)
        let hids: Vec<surrealdb::types::RecordId> =
            sources.iter().filter_map(|h| h.id.clone()).collect();

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

        // 3. Build context from retrieved hadiths, enriched with narrator chains
        let mut context = String::new();
        for h in &sources {
            let narrator = h.narrator_text.as_deref().unwrap_or("Unknown narrator");

            let chain_str =
                h.id.as_ref()
                    .and_then(|hid| chain_map.get(&record_id_string(hid)))
                    .filter(|names| !names.is_empty())
                    .map(|names| format!("Chain of narration: {}", names.join(" → ")))
                    .unwrap_or_default();

            context.push_str(&format!("Hadith #{} — {}\n", h.hadith_number, narrator,));
            if !chain_str.is_empty() {
                context.push_str(&format!("{chain_str}\n"));
            }
            context.push_str(&format!(
                "{}\n\n",
                h.text_en.as_deref().or(h.text_ar.as_deref()).unwrap_or("")
            ));
        }

        let system_prompt = format!(
            "You are a knowledgeable Islamic scholar assistant specializing in Sahih al-Bukhari.\n\
             Answer questions using ONLY the hadiths provided below as context.\n\
             Always cite the hadith number when referencing a hadith.\n\
             When relevant, mention the chain of narration (isnad) to support authenticity.\n\
             If the context doesn't contain relevant information, say so honestly.\n\
             Be concise and accurate.\n\n\
             ## Relevant Hadiths:\n\n{context}"
        );

        let model = model_override
            .map(|s| s.to_string())
            .unwrap_or_else(|| self.model.clone());

        // 3. Call Ollama chat API with streaming
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
            format: None,
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

        // Return the byte stream — it's fully owned, no borrows captured
        Ok((sources, response.bytes_stream()))
    }
}
