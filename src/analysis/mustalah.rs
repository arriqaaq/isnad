//! Mustalah al-Hadith structural analysis engine.
//!
//! Analyzes transmission chain structure (continuity, breadth, corroboration)
//! without computing algorithmic grades. Narrator assessments come from
//! scholarly sources stored in the `evidence` table.
//!
//! References: at-Tahhaan, *Tayseer Mustalah al-Hadeeth*; as-Suyootee,
//! *Tadreeb ar-Raawee*.

use std::collections::HashMap;

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use super::isnad_graph::{self, Direction, FamilyGraph};
use crate::db::Db;

// ── Enums ──

/// Chain continuity classification (ref: pp.21-27, 43).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChainContinuity {
    /// Connected — every narrator heard directly from the prior (p.43)
    Muttasil,
    /// Broken — one narrator missing in the middle (p.25)
    Munqati,
    /// Tabi'i reports directly from Prophet, no Sahabi (p.24)
    Mursal,
    /// Missing narrator(s) at beginning of isnaad (p.23)
    Muallaq,
    /// Two or more consecutive narrators missing (p.25)
    Mudal,
}

/// Transmission breadth classification (ref: pp.9-12).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BreadthClass {
    /// Large number at all levels, inconceivable collusion (p.9)
    Mutawatir,
    /// Three or more at each tabaqah, below mutawatir (p.10)
    Mashhur,
    /// At least two at every level (p.11)
    Aziz,
    /// Only one narrator at some level (p.12)
    Gharib,
}

// ── Constants ──

/// Mutawatir minimum narrators at every tabaqah.
const MUTAWATIR_MIN: usize = 10;

// ── Result structs ──

/// Assessment of a single transmission chain (variant) — structural facts only.
#[derive(Debug, Clone, Serialize)]
pub struct ChainAssessment {
    pub variant_id: String,
    pub continuity: ChainContinuity,
    pub narrator_count: usize,
    pub has_chronology_conflict: bool,
    /// Narrator IDs in chain order (student → teacher → ... → source).
    pub narrator_ids: Vec<String>,
}

/// Transmission breadth analysis.
#[derive(Debug, Clone, Serialize)]
pub struct TransmissionBreadth {
    pub classification: BreadthClass,
    /// (tabaqah_number, narrator_count) pairs
    pub breadth_per_tabaqah: Vec<(i64, usize)>,
    pub min_breadth: usize,
    pub bottleneck_tabaqah: Option<i64>,
    /// For gharib: which narrator is the fard at the bottleneck
    pub fard_narrator: Option<String>,
}

/// Pivot narrator (madar al-isnad) info.
#[derive(Debug, Clone, Serialize)]
pub struct PivotNarrator {
    pub narrator_id: String,
    pub bundle_coverage: f64,
    pub fan_out: usize,
    pub collector_diversity: usize,
    pub bypass_count: usize,
    pub is_bottleneck: bool,
}

/// Corroboration analysis (mutaba'at & shawahid) — counts only.
#[derive(Debug, Clone, Serialize)]
pub struct CorroborationAnalysis {
    pub sahabi_count: usize,
    pub mutabaat_count: usize,
    pub shawahid_count: usize,
}

/// Detected defect flags.
#[derive(Debug, Clone, Serialize)]
pub struct DefectFlags {
    pub has_chronology_conflict: bool,
    pub flags: Vec<String>,
}

/// Complete mustalah structural analysis result for one hadith family.
#[derive(Debug, Clone, Serialize)]
pub struct FamilyMustalahResult {
    pub family_id: String,
    pub chains: Vec<ChainAssessment>,
    pub breadth: TransmissionBreadth,
    pub pivots: Vec<PivotNarrator>,
    pub corroboration: CorroborationAnalysis,
    pub defects: DefectFlags,
}

// ══════════════════════════════════════════════════════════
// 1. Per-Chain Structural Assessment
// ══════════════════════════════════════════════════════════

/// Assess a single chain (variant) for continuity and structure.
fn assess_chain(graph: &FamilyGraph, variant_id: &str) -> ChainAssessment {
    let chain = graph.chain_for_variant(variant_id);
    let narrator_count = chain.len();

    // Detect continuity issues
    let mut has_chronology_conflict = false;
    let mut generation_gaps = 0usize;
    let mut consecutive_gaps = 0usize;
    let mut max_consecutive_gap = 0usize;

    for i in 0..chain.len().saturating_sub(1) {
        let edge_key = format!("{}->{}", chain[i], chain[i + 1]);
        // Check if edge exists (connected)
        if let Some(edges) = graph.edges.get(&edge_key) {
            for e in edges {
                if e.chronology_conflict {
                    has_chronology_conflict = true;
                }
            }
            consecutive_gaps = 0;
        } else {
            // No edge = potential break. Check generation gap.
            let g1 = graph.nodes.get(&chain[i]).and_then(|n| n.generation);
            let g2 = graph.nodes.get(&chain[i + 1]).and_then(|n| n.generation);
            if let (Some(a), Some(b)) = (g1, g2) {
                let gap = (b - a).unsigned_abs() as usize;
                if gap > 1 {
                    generation_gaps += gap - 1;
                    consecutive_gaps += gap - 1;
                    max_consecutive_gap = max_consecutive_gap.max(consecutive_gaps);
                } else {
                    consecutive_gaps = 0;
                }
            } else {
                consecutive_gaps = 0;
            }
        }
    }

    // Classify continuity
    let continuity = if generation_gaps == 0 && max_consecutive_gap == 0 {
        // Check for mursal: root narrator is Tabi'i (gen 2) with no teacher
        let is_mursal = chain.last().is_some_and(|root| {
            graph
                .nodes
                .get(root)
                .is_some_and(|n| n.generation == Some(2) && n.direct_teachers.is_empty())
        });
        if is_mursal {
            ChainContinuity::Mursal
        } else {
            ChainContinuity::Muttasil
        }
    } else if max_consecutive_gap >= 2 {
        ChainContinuity::Mudal
    } else if generation_gaps > 0 {
        // Check if gap is at beginning (mu'allaq) or middle (munqati')
        let first_has_gap = chain
            .first()
            .is_some_and(|first| graph.nodes.get(first).and_then(|n| n.generation).is_none());
        if first_has_gap {
            ChainContinuity::Muallaq
        } else {
            ChainContinuity::Munqati
        }
    } else {
        ChainContinuity::Muttasil
    };

    ChainAssessment {
        variant_id: variant_id.to_string(),
        continuity,
        narrator_count,
        has_chronology_conflict,
        narrator_ids: chain,
    }
}

// ══════════════════════════════════════════════════════════
// 2. Transmission Breadth
// ══════════════════════════════════════════════════════════

fn compute_breadth(graph: &FamilyGraph) -> TransmissionBreadth {
    let tabaqat = graph.tabaqat();
    if tabaqat.is_empty() {
        return TransmissionBreadth {
            classification: BreadthClass::Gharib,
            breadth_per_tabaqah: vec![],
            min_breadth: 0,
            bottleneck_tabaqah: None,
            fard_narrator: None,
        };
    }

    let mut breadth_per_tabaqah: Vec<(i64, usize)> = Vec::new();
    let mut min_breadth = usize::MAX;
    let mut bottleneck_tabaqah = None;
    let mut fard_narrator = None;

    for &tab in &tabaqat {
        let narrators = graph.narrators_at_tabaqah(tab);
        let count = narrators.len();
        breadth_per_tabaqah.push((tab, count));
        if count < min_breadth {
            min_breadth = count;
            bottleneck_tabaqah = Some(tab);
            if count == 1 {
                fard_narrator = narrators.first().map(|s| s.to_string());
            } else {
                fard_narrator = None;
            }
        }
    }

    if min_breadth == usize::MAX {
        min_breadth = 0;
    }

    let classification = if min_breadth >= MUTAWATIR_MIN {
        BreadthClass::Mutawatir
    } else if min_breadth >= 3 {
        BreadthClass::Mashhur
    } else if min_breadth >= 2 {
        BreadthClass::Aziz
    } else {
        BreadthClass::Gharib
    };

    TransmissionBreadth {
        classification,
        breadth_per_tabaqah,
        min_breadth,
        bottleneck_tabaqah,
        fard_narrator,
    }
}

// ══════════════════════════════════════════════════════════
// 3. Pivot Narrators (Madar al-Isnad)
// ══════════════════════════════════════════════════════════

fn identify_pivots(graph: &mut FamilyGraph) -> Vec<PivotNarrator> {
    let total_variants = graph.variant_ids.len();
    if total_variants == 0 {
        return vec![];
    }

    graph.ensure_variant_narrator_map();

    let nids: Vec<String> = graph.nodes.keys().cloned().collect();
    let mut pivots: Vec<PivotNarrator> = Vec::new();

    for nid in &nids {
        let node = &graph.nodes[nid];
        let fan_out = node.direct_students.len();
        let bundle_coverage = node.variants.len() as f64 / total_variants as f64;
        let collector_diversity = graph.reachable_terminals(nid).len();
        let bypass_count = {
            let missing: Vec<String> = graph
                .variant_ids
                .iter()
                .filter(|v| !node.variants.contains(*v))
                .cloned()
                .collect();
            let ancestors = graph.reachable_set(nid, Direction::Teachers);
            let descendants = graph.reachable_set(nid, Direction::Students);
            let vmap = graph.variant_narrator_map().unwrap();
            missing
                .iter()
                .filter(|v| {
                    vmap.get(*v).is_some_and(|narrs| {
                        narrs.iter().any(|n| ancestors.contains(n))
                            && narrs.iter().any(|n| descendants.contains(n))
                    })
                })
                .count()
        };

        // Only include narrators with meaningful coverage
        if bundle_coverage >= 0.20 || fan_out >= 2 {
            let is_bottleneck = bundle_coverage >= 0.95;
            pivots.push(PivotNarrator {
                narrator_id: nid.clone(),
                bundle_coverage,
                fan_out,
                collector_diversity,
                bypass_count,
                is_bottleneck,
            });
        }
    }

    // Sort by coverage descending
    pivots.sort_by(|a, b| {
        b.bundle_coverage
            .partial_cmp(&a.bundle_coverage)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.fan_out.cmp(&a.fan_out))
    });

    pivots.truncate(10); // Top 10 pivots
    pivots
}

// ══════════════════════════════════════════════════════════
// 4. Corroboration Counts (I'tibaar / Mutaabi' / Shaahid)
// ══════════════════════════════════════════════════════════

fn detect_corroboration(graph: &FamilyGraph) -> CorroborationAnalysis {
    let variant_ids: Vec<String> = graph.variant_ids.iter().cloned().collect();

    // Map each variant to its root narrator (Sahabi / source)
    let mut sahabi_variants: HashMap<String, Vec<String>> = HashMap::new();

    for vid in &variant_ids {
        let chain = graph.chain_for_variant(vid);
        if let Some(root) = chain.last() {
            sahabi_variants
                .entry(root.clone())
                .or_default()
                .push(vid.clone());
        }
    }

    let sahabi_count = sahabi_variants.len();

    // Mutaba'at: within same Sahabi, count divergent paths
    let mut mutabaat_count = 0usize;
    for variants in sahabi_variants.values() {
        if variants.len() > 1 {
            mutabaat_count += variants.len() - 1;
        }
    }

    // Shawahid: different Sahabah narrating same meaning
    let shawahid_count = sahabi_count.saturating_sub(1);

    CorroborationAnalysis {
        sahabi_count,
        mutabaat_count,
        shawahid_count,
    }
}

// ══════════════════════════════════════════════════════════
// 5. Defect Detection
// ══════════════════════════════════════════════════════════

fn detect_defects(_graph: &FamilyGraph, chains: &[ChainAssessment]) -> DefectFlags {
    let mut flags: Vec<String> = Vec::new();
    let has_chronology_conflict = chains.iter().any(|c| c.has_chronology_conflict);

    if has_chronology_conflict {
        flags.push("Chronology conflict detected: student's generation predates teacher's".into());
    }

    DefectFlags {
        has_chronology_conflict,
        flags,
    }
}

// ══════════════════════════════════════════════════════════
// Main orchestrator
// ══════════════════════════════════════════════════════════

/// Analyze a single hadith family — structural analysis only, no computed grades.
pub async fn analyze_family_mustalah(
    db: &Surreal<Db>,
    family_id: &str,
) -> Result<Option<FamilyMustalahResult>> {
    let mut graph = match isnad_graph::build_family_graph(db, family_id).await? {
        Some(g) => g,
        None => return Ok(None),
    };

    // Ensure variant-narrator map is built
    graph.ensure_variant_narrator_map();

    // 1. Assess each chain (structural: continuity + narrator list)
    let variant_ids: Vec<String> = graph.variant_ids.iter().cloned().collect();
    let chains: Vec<ChainAssessment> = variant_ids
        .iter()
        .map(|vid| assess_chain(&graph, vid))
        .collect();

    // 2. Transmission breadth
    let breadth = compute_breadth(&graph);

    // 3. Pivot narrators
    let pivots = identify_pivots(&mut graph);

    // 4. Corroboration counts
    let corroboration = detect_corroboration(&graph);

    // 5. Defect detection
    let defects = detect_defects(&graph, &chains);

    Ok(Some(FamilyMustalahResult {
        family_id: family_id.to_string(),
        chains,
        breadth,
        pivots,
        corroboration,
        defects,
    }))
}

/// Store mustalah analysis results in the database.
pub async fn store_mustalah_results(db: &Surreal<Db>, result: &FamilyMustalahResult) -> Result<()> {
    let family_rid = RecordId::new("hadith_family", result.family_id.as_str());
    let slug = format!("isnad_{}", result.family_id);

    // Store family-level structural result
    db.query(
        "CREATE $rid CONTENT { \
            family: $family, \
            breadth_class: $breadth_class, \
            min_breadth: $min_breadth, \
            bottleneck_tabaqah: $bottleneck_tabaqah, \
            sahabi_count: $sahabi_count, \
            mutabaat_count: $mutabaat_count, \
            shawahid_count: $shawahid_count, \
            chain_count: $chain_count, \
            ilal_flags: $ilal_flags \
        }",
    )
    .bind(("rid", RecordId::new("isnad_analysis", slug.as_str())))
    .bind(("family", family_rid.clone()))
    .bind((
        "breadth_class",
        format!("{:?}", result.breadth.classification).to_lowercase(),
    ))
    .bind(("min_breadth", result.breadth.min_breadth as i64))
    .bind(("bottleneck_tabaqah", result.breadth.bottleneck_tabaqah))
    .bind(("sahabi_count", result.corroboration.sahabi_count as i64))
    .bind(("mutabaat_count", result.corroboration.mutabaat_count as i64))
    .bind(("shawahid_count", result.corroboration.shawahid_count as i64))
    .bind(("chain_count", result.chains.len() as i64))
    .bind(("ilal_flags", result.defects.flags.clone()))
    .await?;

    // Store per-chain structural assessments
    for (i, chain) in result.chains.iter().enumerate() {
        let chain_slug = format!("chain_{}_{}", result.family_id, i);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, \
                variant: $variant, \
                continuity: $continuity, \
                narrator_count: $narrator_count, \
                has_chronology_conflict: $chrono, \
                narrator_ids: $narrator_ids \
            }",
        )
        .bind((
            "rid",
            RecordId::new("chain_assessment", chain_slug.as_str()),
        ))
        .bind(("family", family_rid.clone()))
        .bind((
            "variant",
            RecordId::new("hadith", chain.variant_id.as_str()),
        ))
        .bind((
            "continuity",
            format!("{:?}", chain.continuity).to_lowercase(),
        ))
        .bind(("narrator_count", chain.narrator_count as i64))
        .bind(("chrono", chain.has_chronology_conflict))
        .bind(("narrator_ids", chain.narrator_ids.clone()))
        .await?;
    }

    // Store pivot narrators
    for pivot in &result.pivots {
        let pivot_slug = format!("pivot_{}_{}", result.family_id, pivot.narrator_id);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, \
                narrator: $narrator, \
                bundle_coverage: $coverage, \
                fan_out: $fan_out, \
                collector_diversity: $diversity, \
                bypass_count: $bypass, \
                is_bottleneck: $bottleneck \
            }",
        )
        .bind(("rid", RecordId::new("narrator_pivot", pivot_slug.as_str())))
        .bind(("family", family_rid.clone()))
        .bind((
            "narrator",
            RecordId::new("narrator", pivot.narrator_id.as_str()),
        ))
        .bind(("coverage", pivot.bundle_coverage))
        .bind(("fan_out", pivot.fan_out as i64))
        .bind(("diversity", pivot.collector_diversity as i64))
        .bind(("bypass", pivot.bypass_count as i64))
        .bind(("bottleneck", pivot.is_bottleneck))
        .await?;
    }

    Ok(())
}
