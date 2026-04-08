//! Export pipeline for hadith family analysis reports.
//!
//! Generates Markdown and JSON artifacts from mustalah analysis results.

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;

use super::mustalah::FamilyMustalahResult;
use crate::db::Db;

/// JSON artifact bundle for a family analysis.
#[derive(Debug, Serialize)]
pub struct ArtifactBundle {
    pub family_id: String,
    pub composite_grade: String,
    pub best_chain_grade: String,
    pub breadth_class: String,
    pub chain_count: usize,
}

impl From<&FamilyMustalahResult> for ArtifactBundle {
    fn from(r: &FamilyMustalahResult) -> Self {
        Self {
            family_id: r.family_id.clone(),
            composite_grade: format!("{:?}", r.composite_grade),
            best_chain_grade: format!("{:?}", r.best_chain_grade),
            breadth_class: format!("{:?}", r.breadth.classification),
            chain_count: r.chains.len(),
        }
    }
}

/// Generate a Markdown report for a family analysis.
pub fn export_markdown(result: &FamilyMustalahResult) -> String {
    let mut md = String::new();

    md.push_str(&format!(
        "# Hadith Family Analysis: {}\n\n",
        result.family_id
    ));
    md.push_str(&format!(
        "**Composite Grade:** {:?}\n",
        result.composite_grade
    ));
    md.push_str(&format!(
        "**Best Chain Grade:** {:?}\n",
        result.best_chain_grade
    ));
    md.push_str(&format!(
        "**Breadth:** {:?} (min {})\n",
        result.breadth.classification, result.breadth.min_breadth
    ));
    md.push_str(&format!("**Chains:** {}\n\n", result.chains.len()));

    // Chain assessments table
    md.push_str("## Chain Assessments\n\n");
    md.push_str("| Variant | Continuity | Grade | Weakest | Narrators |\n");
    md.push_str("|---------|-----------|-------|---------|----------|\n");

    for c in &result.chains {
        md.push_str(&format!(
            "| {} | {:?} | {:?} | {} | {} |\n",
            c.variant_id,
            c.continuity,
            c.chain_grade,
            c.weakest_narrator_rating.as_deref().unwrap_or("—"),
            c.narrator_count,
        ));
    }
    md.push('\n');

    // Corroboration
    md.push_str("## Corroboration\n\n");
    md.push_str(&format!(
        "- Sahabah: {}\n- Mutaba'at: {} ({} reliable)\n- Shawahid: {}\n- Strength: {:?}\n\n",
        result.corroboration.sahabi_count,
        result.corroboration.mutabaat_count,
        result.corroboration.reliable_mutabaat_count,
        result.corroboration.shawahid_count,
        result.corroboration.strength,
    ));

    // Pivots
    if !result.pivots.is_empty() {
        md.push_str("## Pivot Narrators\n\n");
        md.push_str("| Narrator | Coverage | Fan-out | Bottleneck |\n");
        md.push_str("|----------|----------|---------|------------|\n");
        for p in &result.pivots {
            md.push_str(&format!(
                "| {} | {:.2} | {} | {} |\n",
                p.narrator_id,
                p.bundle_coverage,
                p.fan_out,
                if p.is_bottleneck { "yes" } else { "" },
            ));
        }
        md.push('\n');
    }

    // Defects
    if !result.defects.flags.is_empty() {
        md.push_str("## Defect Flags\n\n");
        for flag in &result.defects.flags {
            md.push_str(&format!("- {}\n", flag));
        }
    }

    md
}

/// Fetch family analysis results from the database for export.
pub async fn fetch_family_analysis(
    db: &Surreal<Db>,
    family_id: &str,
) -> Result<Option<FamilyMustalahResult>> {
    // Use the mustalah analysis engine to re-analyze from the graph
    super::mustalah::analyze_family_mustalah(db, family_id).await
}
