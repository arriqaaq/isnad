//! Narrator evidence model for scholarly assessments (jarh wa ta'dil).
//!
//! Stores and retrieves assessments from classical scholars — no algorithmic
//! scoring or numerical thresholds. Evidence records come from scholarly
//! sources registered in the `scholarly_source` table.

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

/// Evidence record from the database.
#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Evidence {
    pub id: Option<RecordId>,
    pub narrator: Option<RecordId>,
    pub evidence_id: String,
    pub rating: Option<String>,
    pub scholar: Option<String>,
    pub work: Option<String>,
    pub citation_text: Option<String>,
    pub layer: String,
    pub source: Option<RecordId>,
    pub source_locator: Option<String>,
}

/// Fetch all evidence records for a narrator from the database.
pub async fn get_narrator_evidence(db: &Surreal<Db>, narrator_id: &str) -> Result<Vec<Evidence>> {
    let mut res = db
        .query("SELECT * FROM evidence WHERE narrator = $nid")
        .bind(("nid", RecordId::new("narrator", narrator_id)))
        .await?;
    let evidence: Vec<Evidence> = res.take(0)?;
    Ok(evidence)
}

/// Parameters for adding an evidence record from a scholarly source.
pub struct EvidenceParams<'a> {
    pub narrator_id: &'a str,
    pub evidence_id: &'a str,
    pub rating: Option<&'a str>,
    pub scholar: &'a str,
    pub work: &'a str,
    pub citation_text: &'a str,
    pub source_key: &'a str,
    pub source_locator: Option<&'a str>,
}

/// Add an evidence record for a narrator.
pub async fn add_evidence(db: &Surreal<Db>, params: &EvidenceParams<'_>) -> Result<()> {
    let slug = format!("ev_{}_{}", params.narrator_id, params.evidence_id);
    db.query(
        "CREATE $rid CONTENT { \
            narrator: $nid, evidence_id: $eid, rating: $rating, \
            scholar: $scholar, work: $work, \
            citation_text: $citation, layer: 'reported', \
            source: $source, source_locator: $locator, \
            ingested_at: time::now() \
        }",
    )
    .bind(("rid", RecordId::new("evidence", slug.as_str())))
    .bind(("nid", RecordId::new("narrator", params.narrator_id)))
    .bind(("eid", params.evidence_id.to_string()))
    .bind(("rating", params.rating.map(|s| s.to_string())))
    .bind(("scholar", params.scholar.to_string()))
    .bind(("work", params.work.to_string()))
    .bind(("citation", params.citation_text.to_string()))
    .bind((
        "source",
        RecordId::new("scholarly_source", params.source_key),
    ))
    .bind(("locator", params.source_locator.map(|s| s.to_string())))
    .await?;
    Ok(())
}
