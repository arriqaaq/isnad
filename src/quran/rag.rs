use anyhow::Result;
use futures::stream::Stream;
use surrealdb::Surreal;

use crate::db::Db;
use crate::embed::Embedder;
use crate::rag::OllamaClient;

use super::models::AyahSearchResult;
use super::search::search_ayahs_semantic;

const CONTEXT_AYAH_COUNT: usize = 6;
const MAX_TAFSIR_CHARS: usize = 2000;

/// Surah transliteration names indexed by number.
fn surah_name(n: i64) -> &'static str {
    const NAMES: &[&str] = &[
        "", "Al-Fatihah", "Al-Baqarah", "Ali 'Imran", "An-Nisa", "Al-Ma'idah",
        "Al-An'am", "Al-A'raf", "Al-Anfal", "At-Tawbah", "Yunus",
        "Hud", "Yusuf", "Ar-Ra'd", "Ibrahim", "Al-Hijr",
        "An-Nahl", "Al-Isra", "Al-Kahf", "Maryam", "Taha",
        "Al-Anbya", "Al-Hajj", "Al-Mu'minun", "An-Nur", "Al-Furqan",
        "Ash-Shu'ara", "An-Naml", "Al-Qasas", "Al-'Ankabut", "Ar-Rum",
        "Luqman", "As-Sajdah", "Al-Ahzab", "Saba", "Fatir",
        "Ya-Sin", "As-Saffat", "Sad", "Az-Zumar", "Ghafir",
        "Fussilat", "Ash-Shuraa", "Az-Zukhruf", "Ad-Dukhan", "Al-Jathiyah",
        "Al-Ahqaf", "Muhammad", "Al-Fath", "Al-Hujurat", "Qaf",
        "Adh-Dhariyat", "At-Tur", "An-Najm", "Al-Qamar", "Ar-Rahman",
        "Al-Waqi'ah", "Al-Hadid", "Al-Mujadila", "Al-Hashr", "Al-Mumtahanah",
        "As-Saf", "Al-Jumu'ah", "Al-Munafiqun", "At-Taghabun", "At-Talaq",
        "At-Tahrim", "Al-Mulk", "Al-Qalam", "Al-Haqqah", "Al-Ma'arij",
        "Nuh", "Al-Jinn", "Al-Muzzammil", "Al-Muddaththir", "Al-Qiyamah",
        "Al-Insan", "Al-Mursalat", "An-Naba", "An-Nazi'at", "'Abasa",
        "At-Takwir", "Al-Infitar", "Al-Mutaffifin", "Al-Inshiqaq", "Al-Buruj",
        "At-Tariq", "Al-A'la", "Al-Ghashiyah", "Al-Fajr", "Al-Balad",
        "Ash-Shams", "Al-Layl", "Ad-Duhaa", "Ash-Sharh", "At-Tin",
        "Al-'Alaq", "Al-Qadr", "Al-Bayyinah", "Az-Zalzalah", "Al-'Adiyat",
        "Al-Qari'ah", "At-Takathur", "Al-'Asr", "Al-Humazah", "Al-Fil",
        "Quraysh", "Al-Ma'un", "Al-Kawthar", "Al-Kafirun", "An-Nasr",
        "Al-Masad", "Al-Ikhlas", "Al-Falaq", "An-Nas",
    ];
    NAMES.get(n as usize).copied().unwrap_or("Unknown")
}

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
        let sources = search_ayahs_semantic(db, embedder, question, CONTEXT_AYAH_COUNT).await?;

        // 2. Build context from retrieved ayahs with tafsir
        let mut context = String::new();
        for a in &sources {
            let name = surah_name(a.surah_number);
            let text_en = a.text_en.as_deref().unwrap_or("");

            context.push_str(&format!(
                "Surah {} ({}:{}): {}\nArabic: {}\n",
                name, a.surah_number, a.ayah_number, text_en, a.text_ar,
            ));

            if let Some(ref tafsir) = a.tafsir_en {
                if !tafsir.is_empty() {
                    let truncated = if tafsir.len() > MAX_TAFSIR_CHARS {
                        &tafsir[..tafsir.floor_char_boundary(MAX_TAFSIR_CHARS)]
                    } else {
                        tafsir
                    };
                    context.push_str(&format!("Tafsir Ibn Kathir: {truncated}\n"));
                }
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
