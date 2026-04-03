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
        let embeddings = model.embed(texts, None)?;
        Ok(embeddings)
    }

    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        // E5 models require "query: " prefix for search queries to separate
        // the query embedding space from the passage embedding space.
        let prefixed = format!("query: {text}");
        let mut model = self.model.lock().unwrap();
        let mut embeddings = model.embed(vec![&prefixed], None)?;
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

    let pb = indicatif::ProgressBar::new(total as u64);
    pb.set_style(
        indicatif::ProgressStyle::default_bar()
            .template("   {bar:40.green/black} {pos}/{len} embeddings ({eta})")
            .unwrap(),
    );

    for chunk in hadiths.chunks(BATCH_SIZE) {
        let texts: Vec<String> = chunk
            .iter()
            .map(|h| {
                let narrator = h.narrator_text.as_deref().unwrap_or("");
                let text = h.text_en.as_deref().or(h.text_ar.as_deref()).unwrap_or("");
                // E5 models require "passage: " prefix for document embeddings
                format!("passage: {} {}", narrator, text)
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

        pb.inc(chunk.len() as u64);
    }

    pb.finish_with_message("done");
    println!("   ✓ {} embeddings generated", total);
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
