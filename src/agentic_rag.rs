use anyhow::Result;
use futures::stream::Stream;
use surrealdb::Surreal;

use crate::classify::QueryIntent;
use crate::db::Db;
use crate::embed::Embedder;
use crate::models::HadithSearchResult;
use crate::quran::models::AyahSearchResult;
use crate::rag::OllamaClient;
use crate::tools::{self, ApiNarratorSource};

/// Result of the agentic RAG pipeline.
pub enum AgenticResult {
    /// Structured DB query path — we have exact data, stream the answer.
    Structured {
        narrator_sources: Vec<ApiNarratorSource>,
        hadith_sources: Vec<HadithSearchResult>,
        byte_stream: Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + Unpin>,
    },
    /// Fallback to existing unified semantic RAG.
    Semantic {
        ayah_sources: Vec<AyahSearchResult>,
        hadith_sources: Vec<HadithSearchResult>,
        byte_stream: Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + Unpin>,
    },
}

const STRUCTURED_SYSTEM_PREFIX: &str = "\
You are a knowledgeable Islamic hadith scholar. Answer using ONLY the verified database results below.\n\
These numbers are exact counts from the database — do not estimate, round, or guess.\n\
Cite specific data points (names, numbers, generations) from the results.\n\
If the data doesn't answer the question, say so honestly.\n\n";

impl OllamaClient {
    /// Agentic RAG: classify intent, run structured queries or fall back to semantic RAG.
    pub async fn ask_agentic(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<AgenticResult> {
        // Phase 1: Classify
        let intent = match self.classify(question, model_override).await {
            Ok(intent) => intent,
            Err(e) => {
                tracing::warn!("Classification failed, falling back to semantic: {e}");
                QueryIntent::ContentQuery
            }
        };

        // ContentQuery → existing unified RAG
        if matches!(intent, QueryIntent::ContentQuery) {
            return self
                .fallback_semantic(db, embedder, question, model_override)
                .await;
        }

        // Phase 2: Execute structured query
        let tool_result = match &intent {
            QueryIntent::NarratorInfo { name } => {
                let Some(narrator) = tools::resolve_narrator(db, name).await? else {
                    tracing::info!("Narrator '{name}' not found, falling back to semantic");
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::narrator_info(db, &narrator).await?
            }
            QueryIntent::NarratorCount { name, book } => {
                let Some(narrator) = tools::resolve_narrator(db, name).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::count_hadiths(db, &narrator, book.as_deref()).await?
            }
            QueryIntent::NarratorTeachers { name } => {
                let Some(narrator) = tools::resolve_narrator(db, name).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::narrator_teachers(db, &narrator).await?
            }
            QueryIntent::NarratorStudents { name } => {
                let Some(narrator) = tools::resolve_narrator(db, name).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::narrator_students(db, &narrator).await?
            }
            QueryIntent::NarratorHadiths { name } => {
                let Some(narrator) = tools::resolve_narrator(db, name).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::narrator_hadiths(db, &narrator, 10).await?
            }
            QueryIntent::ChainBetween { name1, name2 } => {
                let Some(n1) = tools::resolve_narrator(db, name1).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                let Some(n2) = tools::resolve_narrator(db, name2).await? else {
                    return self
                        .fallback_semantic(db, embedder, question, model_override)
                        .await;
                };
                tools::chain_between(db, &n1, &n2).await?
            }
            QueryIntent::ContentQuery => unreachable!(),
        };

        // Phase 3: Stream LLM answer grounded in structured data
        let system_prompt = format!("{STRUCTURED_SYSTEM_PREFIX}{}", tool_result.context);
        let stream = self
            .chat_stream(&system_prompt, question, model_override)
            .await?;

        Ok(AgenticResult::Structured {
            narrator_sources: tool_result.narrator_sources,
            hadith_sources: tool_result.hadith_sources,
            byte_stream: Box::new(stream),
        })
    }

    /// Fallback to existing unified semantic RAG.
    async fn fallback_semantic(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<AgenticResult> {
        let (ayah_sources, hadith_sources, stream) = self
            .ask_unified(db, embedder, question, model_override)
            .await?;

        Ok(AgenticResult::Semantic {
            ayah_sources,
            hadith_sources,
            byte_stream: Box::new(stream),
        })
    }
}
