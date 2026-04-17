use anyhow::Result;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

const BATCH_SIZE: usize = 64;

/// Supported embedding models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum EmbedModel {
    /// BAAI/bge-m3 (1024-dim, no prefixes)
    #[value(name = "bge-m3")]
    BgeM3,
    /// intfloat/multilingual-e5-small (384-dim, requires query/passage prefixes)
    #[value(name = "e5-small")]
    MultilingualE5Small,
}

impl EmbedModel {
    pub fn fastembed_model(&self) -> EmbeddingModel {
        match self {
            Self::BgeM3 => EmbeddingModel::BGEM3,
            Self::MultilingualE5Small => EmbeddingModel::MultilingualE5Small,
        }
    }

    pub fn dimension(&self) -> usize {
        match self {
            Self::BgeM3 => 1024,
            Self::MultilingualE5Small => 384,
        }
    }

    fn query_prefix(&self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "query: ",
        }
    }

    fn passage_prefix(&self) -> &'static str {
        match self {
            Self::BgeM3 => "",
            Self::MultilingualE5Small => "passage: ",
        }
    }
}

impl Default for EmbedModel {
    fn default() -> Self {
        Self::MultilingualE5Small
    }
}

pub struct Embedder {
    model: Mutex<TextEmbedding>,
    config: EmbedModel,
}

impl Embedder {
    pub fn new(config: EmbedModel) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(config.fastembed_model()).with_show_download_progress(true),
        )?;
        Ok(Self {
            model: Mutex::new(model),
            config,
        })
    }

    pub fn dimension(&self) -> usize {
        self.config.dimension()
    }

    /// Embed passages (applies passage prefix for models that need it).
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let prefix = self.config.passage_prefix();
        let mut model = self.model.lock().unwrap();
        if prefix.is_empty() {
            let embeddings = model.embed(texts, None)?;
            Ok(embeddings)
        } else {
            let prefixed: Vec<String> = texts.iter().map(|t| format!("{prefix}{t}")).collect();
            let refs: Vec<&str> = prefixed.iter().map(|s| s.as_str()).collect();
            let embeddings = model.embed(refs, None)?;
            Ok(embeddings)
        }
    }

    /// Embed a single query (applies query prefix for models that need it).
    pub fn embed_single(&self, text: &str) -> Result<Vec<f32>> {
        let prefix = self.config.query_prefix();
        let mut model = self.model.lock().unwrap();
        if prefix.is_empty() {
            let mut embeddings = model.embed(vec![text], None)?;
            Ok(embeddings.remove(0))
        } else {
            let prefixed = format!("{prefix}{text}");
            let mut embeddings = model.embed(vec![prefixed.as_str()], None)?;
            Ok(embeddings.remove(0))
        }
    }
}

/// Check that existing embeddings (if any) match the expected dimension.
/// Returns an error with instructions if there's a mismatch.
pub async fn check_embedding_dimension(db: &Surreal<Db>, expected_dim: usize) -> Result<()> {
    #[derive(Debug, SurrealValue)]
    struct EmbedProbe {
        embedding: Option<Vec<f32>>,
    }
    let mut res = db
        .query("SELECT embedding FROM hadith WHERE embedding IS NOT NONE LIMIT 1")
        .await?;
    let probes: Vec<EmbedProbe> = res.take(0)?;
    if let Some(probe) = probes.first() {
        if let Some(ref emb) = probe.embedding {
            if emb.len() != expected_dim {
                anyhow::bail!(
                    "Existing embeddings have dimension {} but selected model produces dimension {}.\n\
                     To switch models, clean your data directory and re-ingest:\n  \
                     rm -rf db_data\n  \
                     hadith ingest --embed-model <model> --file data/semantic_hadith.json\n  \
                     hadith ingest-quran --embed-model <model> --file data/quran.csv",
                    emb.len(),
                    expected_dim,
                );
            }
        }
    }
    Ok(())
}

/// Generate embeddings for all hadiths that don't have one yet.
pub async fn embed_all_hadiths(db: &Surreal<Db>, embedder: &Embedder) -> Result<()> {
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
                let text = match (h.text_ar.as_deref(), h.text_en.as_deref()) {
                    (Some(ar), Some(en)) => format!("{} {}", ar, en),
                    (Some(ar), None) => ar.to_string(),
                    (None, Some(en)) => en.to_string(),
                    (None, None) => String::new(),
                };
                format!("{} {}", narrator, text)
            })
            .collect();

        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        let embeddings = embedder.embed(&text_refs)?;

        let futs: Vec<_> = chunk
            .iter()
            .zip(embeddings.into_iter())
            .filter_map(|(hadith, embedding)| {
                hadith.id.as_ref().map(|id| {
                    db.query("UPDATE $id SET embedding = $embedding")
                        .bind(("id", id.clone()))
                        .bind(("embedding", embedding))
                })
            })
            .collect();

        for fut in futs {
            fut.await?;
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
