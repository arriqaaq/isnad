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
    http: Client,
    pub base_url: String,
    pub model: String,
}

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

#[derive(Deserialize)]
pub struct ChatChunk {
    pub message: Option<ChatChunkMessage>,
    pub done: bool,
}

#[derive(Deserialize)]
pub struct ChatChunkMessage {
    pub content: String,
}

/// Narrator info returned from chain query.
#[derive(Debug, SurrealValue)]
struct ChainNarrator {
    name_ar: Option<String>,
    name_en: String,
}

/// Result of the narrator chain query for a hadith.
#[derive(Debug, SurrealValue)]
struct ChainResult {
    narrators: Vec<ChainNarrator>,
}

impl OllamaClient {
    pub fn new(base_url: Option<String>, model: Option<String>) -> Self {
        Self {
            http: Client::new(),
            base_url: base_url.unwrap_or_else(|| DEFAULT_OLLAMA_URL.to_string()),
            model: model.unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        }
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

        // 2. Build context from retrieved hadiths, enriched with narrator chains (GraphRAG)
        let mut context = String::new();
        for h in &sources {
            let narrator = h.narrator_text.as_deref().unwrap_or("Unknown narrator");

            // Fetch the narrator chain (isnad) for this hadith via graph traversal
            let chain_str = if let Some(ref hid) = h.id {
                let rid_str = record_id_string(hid);
                let chain_result: Result<Option<ChainResult>, _> = db
                    .query(
                        "SELECT <-narrates<-narrator.{name_ar, name_en} AS narrators FROM type::thing($rid)",
                    )
                    .bind(("rid", rid_str))
                    .await
                    .and_then(|mut r| r.take(0));

                match chain_result {
                    Ok(Some(cr)) if !cr.narrators.is_empty() => {
                        let names: Vec<String> = cr
                            .narrators
                            .iter()
                            .map(|n| n.name_ar.clone().unwrap_or_else(|| n.name_en.clone()))
                            .collect();
                        format!("Chain of narration: {}", names.join(" → "))
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            };

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
