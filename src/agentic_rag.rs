// ── Agentic RAG: Two-Phase Classify-Then-Execute Pipeline ──
//
// ## Why do we need LLMs at all if RAG just queries a database?
//
// In this system the LLM serves two roles:
//
// 1. **Intent classification** — parsing "how many hadiths did Abu Huraira narrate?"
//    into `NarratorCount { name: "Abu Huraira" }`. This *could* be replaced with
//    regex/keyword rules, but those break on Arabic, synonyms, and varied phrasing.
//    "كم حديثاً رواه أبو هريرة" and "total hadith reported by abu hurayrah" are the
//    same intent — only a language model handles that robustly.
//
// 2. **Natural language generation** — turning `{count: 5374, name: "Abu Hurayrah"}`
//    into a readable answer. This *could* be replaced with templates, but the LLM
//    adapts its response to the specific question asked.
//
// For structured queries (counts, relationships), the LLM is mostly a translator.
// The database does the real work. But LLMs are irreplaceable for:
//
// - **Unstructured reasoning** — "Compare the themes of patience in Surah Al-Baqarah
//   with the hadiths about patience" — no SQL query can do this. You need something
//   that understands language.
// - **Synthesis across sources** — combining 4 hadiths + 3 ayahs into a coherent
//   answer that explains relationships between them.
// - **Multilingual understanding** — parsing Arabic and English variants of the same
//   question as identical intent.
// - **Ambiguity resolution** — "What did the Prophet say about dogs?" requires
//   understanding context, not keyword matching.
// - **Generating new text** — code, articles, translations, summaries — things that
//   don't exist in any database.
//
// People train models because most real-world knowledge isn't in a clean database
// with `hadith_count` pre-computed. It's in millions of books, papers, conversations.
// The LLM compresses that into something queryable. This project is special because
// we *did* the hard work of structuring the data into a graph — most domains don't
// have that luxury.
//
// | Data type                              | Best tool      | LLM needed?               |
// |----------------------------------------|----------------|---------------------------|
// | Structured facts (counts, relations)   | Database query | No — LLM just formats     |
// | Unstructured text understanding        | LLM            | Yes                       |
// | Reasoning across multiple sources      | LLM            | Yes                       |
// | Creative generation                    | LLM            | Yes                       |
//
// ## Example flow: "How many hadiths did Abu Huraira narrate?"
//
// ```text
// ┌─────────────────────────────────────────────────────────────────────┐
// │ Step 1: Frontend                                                   │
// │   POST /api/unified/ask { question: "How many hadiths did Abu..." }│
// │                                                                    │
// │ Step 2: Handler (handlers.rs)                                      │
// │   → calls ollama.ask_agentic(db, embedder, question)               │
// │                                                                    │
// │ Step 3: Phase 1 — Classification (this file, ~500ms)               │
// │   Non-streaming Ollama call with format: "json"                    │
// │   System: "Classify this question. Output JSON."                   │
// │   User:   "How many hadiths did Abu Huraira narrate?"              │
// │   LLM returns: {"intent":"narrator_count","name":"Abu Huraira"}    │
// │   → Parsed into QueryIntent::NarratorCount { name, book: None }    │
// │                                                                    │
// │ Step 4: Phase 2 — Structured DB Query (tools.rs)                   │
// │   a) resolve_narrator("Abu Huraira")                               │
// │      SELECT * FROM narrator WHERE name_en CONTAINS 'abu huraira'   │
// │        OR name_ar CONTAINS ... OR kunya CONTAINS ...               │
// │      ORDER BY hadith_count DESC LIMIT 5                            │
// │      → Returns the real Abu Hurayrah record (id, hadith_count=5374)│
// │                                                                    │
// │   b) count_hadiths(narrator, book=None)                            │
// │      Uses pre-computed narrator.hadith_count = 5374                │
// │      No query needed — O(1) field read                             │
// │                                                                    │
// │   c) Builds context string:                                        │
// │      "## Narrator Hadith Count                                     │
// │       Narrator: أبو هريرة (Abu Huraira)                            │
// │       Generation (Tabaqah): 1                                      │
// │       Total hadiths narrated: 5374"                                │
// │                                                                    │
// │ Step 5: Phase 3 — LLM Answer (streaming)                          │
// │   System: "Answer using ONLY the verified database results below.  │
// │            These numbers are exact — do not guess. {context}"      │
// │   User:   "How many hadiths did Abu Huraira narrate?"              │
// │   LLM streams: "Abu Hurayrah narrated 5,374 hadiths..."           │
// │   → Grounded in exact data, not hallucination                      │
// │                                                                    │
// │ Step 6: SSE back to frontend                                       │
// │   Event 1: { narrator_sources: [{id, name, hadith_count, ...}] }   │
// │   Event 2+: { text: "Abu Hurayrah..." }                           │
// │   Final: { done: true }                                            │
// └─────────────────────────────────────────────────────────────────────┘
// ```
//
// For content questions ("What does Islam say about patience?"), the classifier
// returns ContentQuery and we fall back to the existing semantic RAG pipeline
// (vector search → retrieve ayahs + hadiths → LLM synthesizes an answer).

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
    ///
    /// See the module-level comment for a full walkthrough of the Abu Huraira example.
    pub async fn ask_agentic(
        &self,
        db: &Surreal<Db>,
        embedder: &Embedder,
        question: &str,
        model_override: Option<&str>,
    ) -> Result<AgenticResult> {
        // Phase 1: Classify the user's question into a structured intent.
        // e.g. "How many hadiths did Abu Huraira narrate?"
        //    → NarratorCount { name: "Abu Huraira", book: None }
        // This is a non-streaming Ollama call with format:"json" (~500ms).
        let intent = match self.classify(question, model_override).await {
            Ok(intent) => intent,
            Err(e) => {
                tracing::warn!("Classification failed, falling back to semantic: {e}");
                QueryIntent::ContentQuery
            }
        };

        // ContentQuery → fall back to existing semantic vector search RAG.
        // e.g. "What does Islam say about patience?" — no structured query can answer this,
        // so we retrieve semantically similar ayahs + hadiths and let the LLM synthesize.
        if matches!(intent, QueryIntent::ContentQuery) {
            return self
                .fallback_semantic(db, embedder, question, model_override)
                .await;
        }

        // Phase 2: Execute structured DB queries based on the classified intent.
        // For NarratorCount("Abu Huraira"):
        //   a) resolve_narrator("Abu Huraira") → finds the narrator record via fuzzy name matching
        //   b) count_hadiths(narrator) → reads pre-computed hadith_count (5374) — O(1), no scan
        //   c) Builds a context string with the exact data for the LLM
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

        // Phase 3: Stream the LLM answer, grounded in exact database results.
        // The system prompt contains the structured context (e.g. "Total hadiths: 5374")
        // and instructs the LLM to cite exact numbers — no estimating or guessing.
        // The LLM's only job here is to format the data into natural language.
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
