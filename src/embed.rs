use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

const BATCH_SIZE: usize = 64;

pub struct Embedder {
    model: Mutex<TextEmbedding>,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Small).with_show_download_progress(true),
        )?;
        Ok(Self {
            model: Mutex::new(model),
        })
    }

    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut model = self.model.lock().unwrap();
        let embeddings = model.embed(texts.to_vec(), None)?;
        Ok(embeddings)
    }

    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let mut model = self.model.lock().unwrap();
        let mut embeddings = model.embed(vec![text], None)?;
        Ok(embeddings.remove(0))
    }
}

/// Generate embeddings for all hadiths that don't have one yet.
pub async fn embed_all_hadiths(db: &Surreal<Db>) -> Result<()> {
    let embedder = Embedder::new()?;

    // Get hadiths without embeddings
    let mut response = db
        .query("SELECT id, hadith_number, text_ar, text_en, narrator_text FROM hadith WHERE embedding IS NONE")
        .await?;
    let hadiths: Vec<HadithForEmbed> = response.take(0)?;

    let total = hadiths.len();
    tracing::info!("Generating embeddings for {total} hadiths");

    for (batch_idx, chunk) in hadiths.chunks(BATCH_SIZE).enumerate() {
        let texts: Vec<String> = chunk
            .iter()
            .map(|h| {
                let narrator = h.narrator_text.as_deref().unwrap_or("");
                let text = h.text_en.as_deref().or(h.text_ar.as_deref()).unwrap_or("");
                format!("{} {}", narrator, text)
            })
            .collect();

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = embedder.embed(&text_refs)?;

        for (hadith, embedding) in chunk.iter().zip(embeddings.into_iter()) {
            if let Some(id) = &hadith.id {
                db.query("UPDATE $id SET embedding = $embedding")
                    .bind(("id", id.clone()))
                    .bind(("embedding", embedding))
                    .await?;
            }
        }

        let done = (batch_idx + 1) * BATCH_SIZE;
        tracing::info!("Embedded {}/{total} hadiths", done.min(total));
    }

    tracing::info!("Embedding generation complete");
    Ok(())
}

#[derive(Debug, SurrealValue)]
struct HadithForEmbed {
    id: Option<RecordId>,
    #[allow(dead_code)]
    hadith_number: i64,
    text_ar: Option<String>,
    text_en: Option<String>,
    narrator_text: Option<String>,
}
