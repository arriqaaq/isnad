//! Mustalah al-Hadith analysis engine.
//!
//! Implements traditional Islamic hadith science methodology for analyzing
//! transmission chains.
//!
//! References: Dr. 'Imaad Jum'ah, *Mustalah al-Hadeeth Made Easy*;
//! at-Tahhaan, *Tayseer Mustalah al-Hadeeth*; as-Suyootee, *Tadreeb ar-Raawee*.

use std::collections::{HashMap, HashSet};

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

/// Individual chain grade (ref: pp.13, 17-18, 22, 29).
#[derive(Debug, Clone, Hash, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ChainGrade {
    /// Connected + all narrators 'adl and taamm ad-dabt + no defects
    Sahih,
    /// Connected + all narrators 'adl, lesser dabt (saduq) + no defects
    Hasan,
    /// Fails one or more conditions of hasan (poor memory, break, jahaalah)
    Daif,
    /// Narrator is matrook/munkar — gross errors, negligence, fisq
    DaifJiddan,
    /// Chain contains a known fabricator
    Mawdu,
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

/// Composite family grade with li-ghayrihi strengthening (ref: p.18).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositeGrade {
    Sahih,
    SahihLiGhayrihi,
    Hasan,
    HasanLiGhayrihi,
    Daif,
    DaifJiddan,
    Mawdu,
}

/// Corroboration strength.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorroborationStrength {
    Strong,
    Moderate,
    Weak,
    None,
}

/// Type of weakness — determines elevation eligibility.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeaknessType {
    /// Poor memory, chain break, unknown narrator — CAN be elevated
    Light,
    /// Fisq, kathib — CANNOT be elevated
    Severe,
}

// ── Result structs ──

/// Assessment of a single transmission chain (variant).
#[derive(Debug, Clone, Serialize)]
pub struct ChainAssessment {
    pub variant_id: String,
    pub continuity: ChainContinuity,
    pub chain_grade: ChainGrade,
    pub weakest_narrator_id: Option<String>,
    pub weakest_narrator_rating: Option<String>,
    pub weakest_narrator_prior: Option<f64>,
    pub narrator_count: usize,
    pub has_chronology_conflict: bool,
    pub has_majhul_narrator: bool,
    pub weakness_type: Option<WeaknessType>,
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
    pub reliability_rating: Option<String>,
    pub reliability_prior: Option<f64>,
    pub bypass_count: usize,
    pub is_bottleneck: bool,
}

/// Corroboration analysis (mutaba'at & shawahid).
#[derive(Debug, Clone, Serialize)]
pub struct CorroborationAnalysis {
    pub sahabi_count: usize,
    pub mutabaat_count: usize,
    pub shawahid_count: usize,
    pub reliable_mutabaat_count: usize,
    pub strength: CorroborationStrength,
}

/// Detected defect flags.
#[derive(Debug, Clone, Serialize)]
pub struct DefectFlags {
    pub has_chronology_conflict: bool,
    pub has_potential_idtirab: bool,
    pub matn_coherence: f64,
    pub flags: Vec<String>,
}

/// Complete mustalah analysis result for one hadith family.
#[derive(Debug, Clone, Serialize)]
pub struct FamilyMustalahResult {
    pub family_id: String,
    pub chains: Vec<ChainAssessment>,
    pub breadth: TransmissionBreadth,
    pub pivots: Vec<PivotNarrator>,
    pub corroboration: CorroborationAnalysis,
    pub defects: DefectFlags,
    pub best_chain_grade: ChainGrade,
    pub composite_grade: CompositeGrade,
}

// ── Constants ──

/// Minimum reliability prior for thiqah (fully retentive + upright).
const THIQAH_THRESHOLD: f64 = 0.75;
/// Minimum reliability prior for saduq (lesser dabt).
const SADUQ_THRESHOLD: f64 = 0.65;
/// Minimum reliability prior for "reliable" in corroboration assessment.
const RELIABLE_THRESHOLD: f64 = 0.65;
/// Below this = matruk/accused fabrication territory.
const SEVERE_WEAKNESS_THRESHOLD: f64 = 0.35;
/// Mutawatir minimum narrators at every tabaqah.
const MUTAWATIR_MIN: usize = 10;

// ══════════════════════════════════════════════════════════
// 1. Per-Chain Isnad Assessment
// ══════════════════════════════════════════════════════════

/// Assess a single chain (variant) for continuity and narrator quality.
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
        let is_mursal = chain.last().map_or(false, |root| {
            graph.nodes.get(root).map_or(false, |n| {
                n.generation == Some(2) && n.direct_teachers.is_empty()
            })
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
        let first_has_gap = chain.first().map_or(false, |first| {
            graph.nodes.get(first).and_then(|n| n.generation).is_none()
        });
        if first_has_gap {
            ChainContinuity::Muallaq
        } else {
            ChainContinuity::Munqati
        }
    } else {
        ChainContinuity::Muttasil
    };

    // Find weakest narrator
    let mut weakest_id: Option<String> = None;
    let mut weakest_prior: Option<f64> = None;
    let mut weakest_rating: Option<String> = None;
    let mut has_majhul = false;
    let mut weakness_type: Option<WeaknessType> = None;

    for nid in &chain {
        if let Some(node) = graph.nodes.get(nid) {
            let prior = node.reliability_prior.unwrap_or(0.50); // majhul default
            if node.reliability_rating.is_none() && node.reliability_prior.is_none() {
                has_majhul = true;
            }
            if weakest_prior.is_none() || prior < weakest_prior.unwrap() {
                weakest_prior = Some(prior);
                weakest_id = Some(nid.clone());
                weakest_rating = node.reliability_rating.clone();
            }
        }
    }

    // Determine chain grade based on continuity + weakest narrator
    let wp = weakest_prior.unwrap_or(0.50);
    let chain_grade = if wp <= 0.20 {
        // matruk / accused fabrication
        weakness_type = Some(WeaknessType::Severe);
        if weakest_rating.as_deref() == Some("accused_fabrication") {
            ChainGrade::Mawdu
        } else {
            ChainGrade::DaifJiddan
        }
    } else if wp < SEVERE_WEAKNESS_THRESHOLD {
        // munkar territory — fisq, gross errors
        weakness_type = Some(WeaknessType::Severe);
        ChainGrade::DaifJiddan
    } else if continuity != ChainContinuity::Muttasil {
        // Not connected = da'eef (but light weakness, can be elevated)
        weakness_type = Some(WeaknessType::Light);
        ChainGrade::Daif
    } else if wp < SADUQ_THRESHOLD || has_majhul {
        // Connected but narrator below saduq or unknown
        weakness_type = Some(WeaknessType::Light);
        ChainGrade::Daif
    } else if wp < THIQAH_THRESHOLD {
        // Connected, saduq level = hasan
        ChainGrade::Hasan
    } else {
        // Connected, all thiqah = sahih
        ChainGrade::Sahih
    };

    ChainAssessment {
        variant_id: variant_id.to_string(),
        continuity,
        chain_grade,
        weakest_narrator_id: weakest_id,
        weakest_narrator_rating: weakest_rating,
        weakest_narrator_prior: weakest_prior,
        narrator_count,
        has_chronology_conflict,
        has_majhul_narrator: has_majhul,
        weakness_type,
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
                    vmap.get(*v).map_or(false, |narrs| {
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
                reliability_rating: node.reliability_rating.clone(),
                reliability_prior: node.reliability_prior,
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
// 4. Corroboration (I'tibaar / Mutaabi' / Shaahid)
// ══════════════════════════════════════════════════════════

fn detect_corroboration(graph: &FamilyGraph) -> CorroborationAnalysis {
    // Group chains by their root narrator (Sahabi / source)
    let _roots = graph.root_narrators();
    let variant_ids: Vec<String> = graph.variant_ids.iter().cloned().collect();

    // Map each variant to its root narrator(s)
    let mut sahabi_variants: HashMap<String, Vec<String>> = HashMap::new();

    for vid in &variant_ids {
        let chain = graph.chain_for_variant(vid);
        // Root = last in chain (closest to source)
        if let Some(root) = chain.last() {
            sahabi_variants
                .entry(root.clone())
                .or_default()
                .push(vid.clone());
        }
    }

    let sahabi_count = sahabi_variants.len();

    // Mutaba'at: within same Sahabi, count divergent paths (>1 variant = has mutaba'at)
    let mut mutabaat_count = 0usize;
    let mut reliable_mutabaat_count = 0usize;

    for (_sahabi, variants) in &sahabi_variants {
        if variants.len() > 1 {
            // Each additional variant beyond the first is a mutaba'ah
            mutabaat_count += variants.len() - 1;

            // Check if corroborating chains are reliable
            for vid in variants.iter().skip(1) {
                let chain = graph.chain_for_variant(vid);
                let all_reliable = chain.iter().all(|nid| {
                    graph
                        .nodes
                        .get(nid)
                        .and_then(|n| n.reliability_prior)
                        .unwrap_or(0.50)
                        >= RELIABLE_THRESHOLD
                });
                if all_reliable {
                    reliable_mutabaat_count += 1;
                }
            }
        }
    }

    // Shawahid: different Sahabah narrating same meaning
    let shawahid_count = if sahabi_count > 1 {
        sahabi_count - 1
    } else {
        0
    };

    let strength = if reliable_mutabaat_count >= 3 || (shawahid_count >= 2 && mutabaat_count >= 1) {
        CorroborationStrength::Strong
    } else if mutabaat_count >= 1 || shawahid_count >= 1 {
        CorroborationStrength::Moderate
    } else if mutabaat_count > 0 {
        CorroborationStrength::Weak
    } else {
        CorroborationStrength::None
    };

    CorroborationAnalysis {
        sahabi_count,
        mutabaat_count,
        shawahid_count,
        reliable_mutabaat_count,
        strength,
    }
}

// ══════════════════════════════════════════════════════════
// 5. Composite Grade
// ══════════════════════════════════════════════════════════

fn compute_composite_grade(
    chains: &[ChainAssessment],
    corroboration: &CorroborationAnalysis,
) -> (ChainGrade, CompositeGrade) {
    if chains.is_empty() {
        return (ChainGrade::Daif, CompositeGrade::Daif);
    }

    // Find best chain grade
    let best = chains
        .iter()
        .map(|c| &c.chain_grade)
        .min() // Ord: Sahih < Hasan < Daif < DaifJiddan < Mawdu
        .cloned()
        .unwrap_or(ChainGrade::Daif);

    let has_strong_corroboration = corroboration.strength == CorroborationStrength::Strong
        || corroboration.strength == CorroborationStrength::Moderate;

    // Check if any chain's weakness is light (eligible for elevation)
    let has_elevatable_daif = chains.iter().any(|c| {
        c.chain_grade == ChainGrade::Daif && c.weakness_type.as_ref() == Some(&WeaknessType::Light)
    });

    let composite = match &best {
        ChainGrade::Sahih => CompositeGrade::Sahih,
        ChainGrade::Hasan => {
            if has_strong_corroboration {
                CompositeGrade::SahihLiGhayrihi
            } else {
                CompositeGrade::Hasan
            }
        }
        ChainGrade::Daif => {
            if has_elevatable_daif && has_strong_corroboration {
                // Da'eef with light weakness + corroboration = hasan li-ghayrihi (p.18)
                CompositeGrade::HasanLiGhayrihi
            } else {
                CompositeGrade::Daif
            }
        }
        ChainGrade::DaifJiddan => CompositeGrade::DaifJiddan,
        ChainGrade::Mawdu => CompositeGrade::Mawdu,
    };

    (best, composite)
}

// ══════════════════════════════════════════════════════════
// 6. Defect Detection
// ══════════════════════════════════════════════════════════

fn detect_defects(_graph: &FamilyGraph, chains: &[ChainAssessment]) -> DefectFlags {
    let mut flags: Vec<String> = Vec::new();
    let has_chronology_conflict = chains.iter().any(|c| c.has_chronology_conflict);

    if has_chronology_conflict {
        flags.push("Chronology conflict detected: student's generation predates teacher's".into());
    }

    // Check for potential idtirab: multiple chains of equal strength with conflicting matn
    // (simplified: if all chains are same grade but family has many variants, flag for review)
    let grades: HashSet<_> = chains.iter().map(|c| &c.chain_grade).collect();
    let has_potential_idtirab =
        grades.len() == 1 && chains.len() >= 3 && chains[0].chain_grade == ChainGrade::Hasan;

    if has_potential_idtirab {
        flags.push(
            "Potential idtirab: multiple equal-strength chains — verify matn consistency".into(),
        );
    }

    // Majhul narrators
    let majhul_count = chains.iter().filter(|c| c.has_majhul_narrator).count();
    if majhul_count > 0 {
        flags.push(format!(
            "{majhul_count} chain(s) contain unknown (majhul) narrator(s)"
        ));
    }

    // TODO: integrate matn_diff for actual coherence scoring
    let matn_coherence = 0.50; // placeholder until matn_diff integration

    DefectFlags {
        has_chronology_conflict,
        has_potential_idtirab,
        matn_coherence,
        flags,
    }
}

// ══════════════════════════════════════════════════════════
// Main orchestrator
// ══════════════════════════════════════════════════════════

/// Analyze a single hadith family using mustalah al-hadith methodology.
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

    // 1. Assess each chain
    let variant_ids: Vec<String> = graph.variant_ids.iter().cloned().collect();
    let chains: Vec<ChainAssessment> = variant_ids
        .iter()
        .map(|vid| assess_chain(&graph, vid))
        .collect();

    // 2. Transmission breadth
    let breadth = compute_breadth(&graph);

    // 3. Pivot narrators
    let pivots = identify_pivots(&mut graph);

    // 4. Corroboration
    let corroboration = detect_corroboration(&graph);

    // 5. Composite grade
    let (best_chain_grade, composite_grade) = compute_composite_grade(&chains, &corroboration);

    // 6. Defect detection
    let defects = detect_defects(&graph, &chains);

    Ok(Some(FamilyMustalahResult {
        family_id: family_id.to_string(),
        chains,
        breadth,
        pivots,
        corroboration,
        defects,
        best_chain_grade,
        composite_grade,
    }))
}

/// Store mustalah analysis results in the database.
pub async fn store_mustalah_results(db: &Surreal<Db>, result: &FamilyMustalahResult) -> Result<()> {
    let family_rid = RecordId::new("hadith_family", result.family_id.as_str());
    let slug = format!("isnad_{}", result.family_id);

    // Store family-level result
    db.query(
        "CREATE $rid CONTENT { \
            family: $family, \
            composite_grade: $composite_grade, \
            best_chain_grade: $best_chain_grade, \
            breadth_class: $breadth_class, \
            min_breadth: $min_breadth, \
            bottleneck_tabaqah: $bottleneck_tabaqah, \
            sahabi_count: $sahabi_count, \
            mutabaat_count: $mutabaat_count, \
            shawahid_count: $shawahid_count, \
            reliable_mutabaat_count: $reliable_mutabaat, \
            corroboration_strength: $corr_strength, \
            matn_coherence: $matn_coherence, \
            chain_count: $chain_count, \
            sahih_chain_count: $sahih_chains, \
            hasan_chain_count: $hasan_chains, \
            daif_chain_count: $daif_chains, \
            ilal_flags: $ilal_flags \
        }",
    )
    .bind(("rid", RecordId::new("isnad_analysis", slug.as_str())))
    .bind(("family", family_rid.clone()))
    .bind((
        "composite_grade",
        format!("{:?}", result.composite_grade).to_lowercase(),
    ))
    .bind((
        "best_chain_grade",
        format!("{:?}", result.best_chain_grade).to_lowercase(),
    ))
    .bind((
        "breadth_class",
        format!("{:?}", result.breadth.classification).to_lowercase(),
    ))
    .bind(("min_breadth", result.breadth.min_breadth as i64))
    .bind(("bottleneck_tabaqah", result.breadth.bottleneck_tabaqah))
    .bind(("sahabi_count", result.corroboration.sahabi_count as i64))
    .bind(("mutabaat_count", result.corroboration.mutabaat_count as i64))
    .bind(("shawahid_count", result.corroboration.shawahid_count as i64))
    .bind((
        "reliable_mutabaat",
        result.corroboration.reliable_mutabaat_count as i64,
    ))
    .bind((
        "corr_strength",
        format!("{:?}", result.corroboration.strength).to_lowercase(),
    ))
    .bind(("matn_coherence", result.defects.matn_coherence))
    .bind(("chain_count", result.chains.len() as i64))
    .bind((
        "sahih_chains",
        result
            .chains
            .iter()
            .filter(|c| c.chain_grade == ChainGrade::Sahih)
            .count() as i64,
    ))
    .bind((
        "hasan_chains",
        result
            .chains
            .iter()
            .filter(|c| c.chain_grade == ChainGrade::Hasan)
            .count() as i64,
    ))
    .bind((
        "daif_chains",
        result
            .chains
            .iter()
            .filter(|c| {
                matches!(
                    c.chain_grade,
                    ChainGrade::Daif | ChainGrade::DaifJiddan | ChainGrade::Mawdu
                )
            })
            .count() as i64,
    ))
    .bind(("ilal_flags", result.defects.flags.clone()))
    .await?;

    // Store per-chain assessments
    for (i, chain) in result.chains.iter().enumerate() {
        let chain_slug = format!("chain_{}_{}", result.family_id, i);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, \
                variant: $variant, \
                continuity: $continuity, \
                chain_grade: $chain_grade, \
                weakest_narrator: $weakest, \
                weakest_rating: $weakest_rating, \
                weakest_prior: $weakest_prior, \
                narrator_count: $narrator_count, \
                has_chronology_conflict: $chrono, \
                has_majhul: $majhul \
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
        .bind((
            "chain_grade",
            format!("{:?}", chain.chain_grade).to_lowercase(),
        ))
        .bind((
            "weakest",
            chain
                .weakest_narrator_id
                .as_ref()
                .map(|id| RecordId::new("narrator", id.as_str())),
        ))
        .bind(("weakest_rating", chain.weakest_narrator_rating.clone()))
        .bind(("weakest_prior", chain.weakest_narrator_prior))
        .bind(("narrator_count", chain.narrator_count as i64))
        .bind(("chrono", chain.has_chronology_conflict))
        .bind(("majhul", chain.has_majhul_narrator))
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
