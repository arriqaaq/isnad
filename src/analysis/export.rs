//! Export pipeline for hadith family analysis reports.
//!
//! Generates Markdown and JSON artifacts from CL/PCL analysis results.

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use super::cl_pcl::{Candidate, FamilyAnalysisResult};
use crate::db::Db;

/// JSON artifact bundle for a family analysis.
#[derive(Debug, Serialize)]
pub struct ArtifactBundle {
    pub family_id: String,
    pub family_status: String,
    pub profile: String,
    pub candidate_count: usize,
    pub candidates: Vec<Candidate>,
}

impl From<&FamilyAnalysisResult> for ArtifactBundle {
    fn from(r: &FamilyAnalysisResult) -> Self {
        Self {
            family_id: r.family_id.clone(),
            family_status: r.family_status.clone(),
            profile: r.profile.clone(),
            candidate_count: r.candidates.len(),
            candidates: r.candidates.clone(),
        }
    }
}

/// Generate a Markdown report for a family analysis.
pub fn export_markdown(result: &FamilyAnalysisResult) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# Hadith Family Analysis: {}\n\n",
        result.family_id
    ));
    md.push_str(&format!("**Status:** {}\n", result.family_status));
    md.push_str(&format!("**Profile:** {}\n", result.profile));
    md.push_str(&format!("**Candidates:** {}\n\n", result.candidates.len()));

    if result.candidates.is_empty() {
        md.push_str("No CL/PCL candidates identified.\n");
        return md;
    }

    // Candidates table
    md.push_str("## Candidates\n\n");
    md.push_str(
        "| Rank | Narrator | Type | Outcome | Confidence | Fan-out | Coverage | Diversity |\n",
    );
    md.push_str(
        "|------|----------|------|---------|------------|---------|----------|-----------|\n",
    );

    for c in &result.candidates {
        let conf_display = format!("{:.4}", c.final_confidence);
        let outcome_icon = match c.outcome.as_str() {
            "supported" => "Supported",
            "contested" => "Contested",
            "uncertain" => "Uncertain",
            "likely_weak_in_context" => "Likely Weak",
            _ => &c.outcome,
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:.2} | {} |\n",
            c.rank,
            c.narrator_id,
            c.candidate_type,
            outcome_icon,
            conf_display,
            c.features.fan_out,
            c.features.bundle_coverage,
            c.features.collector_diversity,
        ));
    }
    md.push('\n');

    // Detailed scores
    md.push_str("## Detailed Scores\n\n");
    for c in &result.candidates {
        md.push_str(&format!(
            "### #{} — {} ({})\n\n",
            c.rank, c.narrator_id, c.candidate_type
        ));
        md.push_str(&format!(
            "- **Structural Score:** {:.4}\n",
            c.structural_score
        ));
        md.push_str(&format!(
            "- **Final Confidence:** {:.4}\n",
            c.final_confidence
        ));
        md.push_str(&format!("- **Outcome:** {}\n", c.outcome));
        if c.contradiction_cap_active {
            md.push_str("- **Contradiction Cap:** Active (capped at 0.70)\n");
        }
        md.push_str(&format!(
            "- Pre-single-strand ratio: {:.4}\n",
            c.features.pre_single_strand_ratio
        ));
        md.push_str(&format!(
            "- Bundle coverage: {:.4}\n",
            c.features.bundle_coverage
        ));
        md.push_str(&format!(
            "- Collector diversity: {}\n",
            c.features.collector_diversity
        ));
        md.push_str(&format!("- Fan-out: {}\n", c.features.fan_out));
        md.push_str(&format!("- Bypass ratio: {:.4}\n", c.features.bypass_ratio));
        md.push_str(&format!(
            "- Chronology conflict ratio: {:.4}\n",
            c.features.chronology_conflict_ratio
        ));
        md.push_str(&format!(
            "- Matn coherence: {:.4}\n",
            c.features.matn_coherence
        ));
        md.push_str(&format!(
            "- Provenance completeness: {:.4}\n\n",
            c.features.provenance_completeness
        ));
    }

    md
}

/// Fetch family analysis results from the database for export.
pub async fn fetch_family_analysis(
    db: &Surreal<Db>,
    family_id: &str,
) -> Result<Option<FamilyAnalysisResult>> {
    #[derive(Debug, SurrealValue)]
    struct ClRow {
        narrator: Option<RecordId>,
        candidate_type: String,
        pcl_mode: Option<String>,
        fan_out: i64,
        bundle_coverage: f64,
        collector_diversity: i64,
        pre_single_strand_ratio: f64,
        bypass_ratio: f64,
        chronology_conflict_ratio: f64,
        matn_coherence: f64,
        provenance_completeness: f64,
        structural_score: f64,
        reliability_prior: Option<f64>,
        final_confidence: f64,
        outcome: String,
        contradiction_cap_active: bool,
        profile: String,
        family_status: String,
        rank: i64,
    }

    let mut res = db
        .query("SELECT * FROM cl_analysis WHERE family = $fid ORDER BY rank ASC")
        .bind(("fid", RecordId::new("hadith_family", family_id)))
        .await?;
    let rows: Vec<ClRow> = res.take(0)?;

    if rows.is_empty() {
        return Ok(None);
    }

    let profile = rows[0].profile.clone();
    let family_status = rows[0].family_status.clone();

    let candidates = rows
        .into_iter()
        .map(|r| Candidate {
            narrator_id: r
                .narrator
                .as_ref()
                .map(crate::models::record_id_key_string)
                .unwrap_or_default(),
            candidate_type: r.candidate_type,
            pcl_mode: r.pcl_mode,
            structural_score: r.structural_score,
            final_confidence: r.final_confidence,
            outcome: r.outcome,
            contradiction_cap_active: r.contradiction_cap_active,
            profile: r.profile,
            features: super::cl_pcl::FeatureVector {
                fan_out: r.fan_out as usize,
                bundle_coverage: r.bundle_coverage,
                collector_diversity: r.collector_diversity as usize,
                pre_single_strand_ratio: r.pre_single_strand_ratio,
                bypass_ratio: r.bypass_ratio,
                chronology_conflict_ratio: r.chronology_conflict_ratio,
                matn_coherence: r.matn_coherence,
                provenance_completeness: r.provenance_completeness,
            },
            rank: r.rank as usize,
            family_status: r.family_status,
        })
        .collect();

    Ok(Some(FamilyAnalysisResult {
        family_id: family_id.to_string(),
        family_status,
        profile,
        candidates,
    }))
}
