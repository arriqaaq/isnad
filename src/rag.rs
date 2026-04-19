use anyhow::Result;
use futures::stream::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_MODEL: &str = "llama3.2";

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
}
