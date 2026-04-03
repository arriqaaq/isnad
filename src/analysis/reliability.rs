//! Three-layer reliability model for narrator assessment.
//!
//! Layers:
//! - Reported: Classical scholar assessments (full weight)
//! - Analytical: Derived from CL/PCL analysis (half weight)
//! - Derived: Weighted composite with contradiction detection

use std::collections::HashSet;

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

/// Map a reliability rating string to its prior probability.
pub fn rating_prior(rating: &str) -> f64 {
    match rating {
        "thiqah" => 0.75,
        "saduq" => 0.65,
        "majhul" => 0.50,
        "daif" => 0.35,
        "matruk" | "accused_fabrication" => 0.20,
        _ => 0.50,
    }
}

/// Contradiction cap applied when conflicting ratings detected.
const CONTRADICTION_CAP: f64 = 0.70;

/// Contradiction rating pairs (if both present, flag contradiction).
const CONTRADICTION_PAIRS: &[(&str, &str)] = &[
    ("thiqah", "daif"),
    ("thiqah", "matruk"),
    ("thiqah", "accused_fabrication"),
    ("matruk", "accused_fabrication"),
];

/// Evidence record from the database.
#[derive(Debug, SurrealValue, Serialize, Clone)]
pub struct Evidence {
    pub id: Option<RecordId>,
    pub narrator: Option<RecordId>,
    pub evidence_id: String,
    pub rating: String,
    pub rating_confidence: Option<f64>,
    pub scholar: Option<String>,
    pub work: Option<String>,
    pub citation_text: Option<String>,
    pub citation_span: Option<String>,
    pub dissent_notes: Option<String>,
    pub layer: String,
    pub source_collection: Option<String>,
    pub source_type: Option<String>,
    pub source_locator: Option<String>,
}

/// Derived assessment for a narrator.
#[derive(Debug, Serialize)]
pub struct DerivedAssessment {
    pub prior: f64,
    pub ratings: Vec<String>,
    pub conflicting: bool,
    pub derived_confidence: f64,
    pub sources_count: usize,
}

/// Compute the derived assessment for a narrator from their evidence records.
///
/// Algorithm (matching Riwaq reliability-layer.js):
/// - Reported layer: full weight
/// - Analytical layer: half weight
/// - Contradiction detection caps at 0.70
pub fn compute_derived(evidence: &[Evidence]) -> Option<DerivedAssessment> {
    if evidence.is_empty() {
        return None;
    }

    let mut weighted_sum = 0.0f64;
    let mut weight_total = 0.0f64;
    let mut ratings = Vec::new();

    for ev in evidence {
        let prior = rating_prior(&ev.rating);
        let weight = ev.rating_confidence.unwrap_or(0.5);

        if ev.layer == "analytical" {
            // Analytical layer is half-weighted
            weighted_sum += prior * weight * 0.5;
            weight_total += weight * 0.5;
        } else {
            // Reported layer (and any other) at full weight
            weighted_sum += prior * weight;
            weight_total += weight;
        }
        ratings.push(ev.rating.clone());
    }

    let derived_prior = if weight_total > 0.0 {
        weighted_sum / weight_total
    } else {
        0.50
    };

    // Detect contradictions
    let rating_set: HashSet<&str> = ratings.iter().map(|s| s.as_str()).collect();
    let has_contradiction = CONTRADICTION_PAIRS
        .iter()
        .any(|(a, b)| rating_set.contains(a) && rating_set.contains(b));

    let derived_confidence = if has_contradiction {
        derived_prior.min(CONTRADICTION_CAP)
    } else {
        derived_prior
    };

    // Round to 3 decimal places
    let derived_confidence = (derived_confidence * 1000.0).round() / 1000.0;
    let prior_rounded = (derived_prior * 1000.0).round() / 1000.0;

    let unique_ratings: Vec<String> = {
        let mut seen = HashSet::new();
        ratings
            .into_iter()
            .filter(|r| seen.insert(r.clone()))
            .collect()
    };

    Some(DerivedAssessment {
        prior: prior_rounded,
        ratings: unique_ratings,
        conflicting: has_contradiction,
        derived_confidence,
        sources_count: evidence.len(),
    })
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

/// Parameters for adding a reported evidence record.
pub struct ReportedEvidenceParams<'a> {
    pub narrator_id: &'a str,
    pub evidence_id: &'a str,
    pub rating: &'a str,
    pub scholar: Option<&'a str>,
    pub work: Option<&'a str>,
    pub citation_text: Option<&'a str>,
    pub source_collection: Option<&'a str>,
}

/// Add a reported evidence record for a narrator.
pub async fn add_reported_evidence(
    db: &Surreal<Db>,
    params: &ReportedEvidenceParams<'_>,
) -> Result<()> {
    let ReportedEvidenceParams {
        narrator_id,
        evidence_id,
        rating,
        scholar,
        work,
        citation_text,
        source_collection,
    } = params;
    let slug = format!("ev_{}_{}", narrator_id, evidence_id);
    db.query(
        "CREATE $rid CONTENT { \
            narrator: $nid, evidence_id: $eid, rating: $rating, \
            rating_confidence: 0.5, scholar: $scholar, work: $work, \
            citation_text: $citation, layer: 'reported', \
            source_collection: $scol, source_type: 'print', \
            source_locator: NONE, ingested_at: time::now() \
        }",
    )
    .bind(("rid", RecordId::new("evidence", slug.as_str())))
    .bind(("nid", RecordId::new("narrator", *narrator_id)))
    .bind(("eid", evidence_id.to_string()))
    .bind(("rating", rating.to_string()))
    .bind(("scholar", scholar.map(|s| s.to_string())))
    .bind(("work", work.map(|s| s.to_string())))
    .bind(("citation", citation_text.map(|s| s.to_string())))
    .bind(("scol", source_collection.map(|s| s.to_string())))
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_evidence(rating: &str, layer: &str, confidence: f64) -> Evidence {
        Evidence {
            id: None,
            narrator: None,
            evidence_id: "test".to_string(),
            rating: rating.to_string(),
            rating_confidence: Some(confidence),
            scholar: None,
            work: None,
            citation_text: None,
            citation_span: None,
            dissent_notes: None,
            layer: layer.to_string(),
            source_collection: None,
            source_type: None,
            source_locator: None,
        }
    }

    #[test]
    fn test_single_thiqah() {
        let ev = vec![make_evidence("thiqah", "reported", 0.5)];
        let result = compute_derived(&ev).unwrap();
        assert_eq!(result.prior, 0.75);
        assert_eq!(result.derived_confidence, 0.75);
        assert!(!result.conflicting);
    }

    #[test]
    fn test_analytical_half_weight() {
        let ev = vec![
            make_evidence("thiqah", "reported", 1.0),
            make_evidence("saduq", "analytical", 1.0),
        ];
        let result = compute_derived(&ev).unwrap();
        // reported: 0.75 * 1.0 = 0.75, weight 1.0
        // analytical: 0.65 * 1.0 * 0.5 = 0.325, weight 0.5
        // total: (0.75 + 0.325) / (1.0 + 0.5) = 1.075 / 1.5 = 0.717
        assert!((result.prior - 0.717).abs() < 0.001);
    }

    #[test]
    fn test_contradiction_cap() {
        let ev = vec![
            make_evidence("thiqah", "reported", 0.5),
            make_evidence("daif", "reported", 0.5),
        ];
        let result = compute_derived(&ev).unwrap();
        assert!(result.conflicting);
        assert!(result.derived_confidence <= 0.70);
    }

    #[test]
    fn test_empty_evidence() {
        let result = compute_derived(&[]);
        assert!(result.is_none());
    }
}
