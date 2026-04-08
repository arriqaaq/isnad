//! Juynboll Falsifiability Analysis.
//!
//! Four algorithmic tests that evaluate whether the structural predictions of
//! Juynboll's CL-as-fabricator thesis hold against empirical transmission data.
//!
//! Tests:
//! 1. Reliable Bypass Analysis — do reliable independent paths bypass the CL?
//! 2. Multiple Independent CLs — do independent convergence points exist?
//! 3. Cross-Family CL Frequency — are prolific CLs classically reliable?
//! 4. Pre-CL Chain Diversity — are upstream chains diverse and reliable?

use std::collections::HashSet;

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

use super::cl_pcl::{Direction, FamilyGraph};

/// Minimum reliability prior to count a narrator as "reliable" (saduq threshold).
const RELIABLE_THRESHOLD: f64 = 0.65;

// ── Result types ──

/// Detail of a single bypass variant for a specific CL.
#[derive(Debug, Clone, Serialize)]
pub struct BypassDetail {
    pub variant_id: String,
    pub bypass_narrators: Vec<String>,
    pub min_reliability: Option<f64>,
    pub avg_reliability: Option<f64>,
    pub all_reliable: bool,
}

/// Reliable bypass analysis result for one CL.
#[derive(Debug, Clone, Serialize)]
pub struct ReliableBypassResult {
    pub cl_narrator_id: String,
    pub total_variants: usize,
    pub bypass_count: usize,
    pub reliable_bypass_count: usize,
    pub reliable_bypass_ratio: f64,
    pub details: Vec<BypassDetail>,
}

/// A pair of CLs checked for independence.
#[derive(Debug, Clone, Serialize)]
pub struct IndependentClPair {
    pub cl_a: String,
    pub cl_b: String,
    pub independent: bool,
}

/// Independent CL analysis result for a family.
#[derive(Debug, Clone, Serialize)]
pub struct IndependentClResult {
    pub family_id: String,
    pub cl_count: usize,
    pub pairs_checked: usize,
    pub independent_pairs: usize,
    pub pairs: Vec<IndependentClPair>,
}

/// Pre-CL chain diversity result for one CL.
#[derive(Debug, Clone, Serialize)]
pub struct PreClDiversityResult {
    pub cl_narrator_id: String,
    pub pre_single_strand_ratio: f64,
    pub upstream_narrator_count: usize,
    pub upstream_reliable_count: usize,
    pub upstream_reliable_ratio: f64,
    pub upstream_branching_points: usize,
    pub upstream_with_bio_count: usize,
}

/// Per-family Juynboll analysis result.
#[derive(Debug, Clone, Serialize)]
pub struct FamilyJuynbollResult {
    pub family_id: String,
    pub reliable_bypasses: Vec<ReliableBypassResult>,
    pub independent_cls: Option<IndependentClResult>,
    pub pre_cl_diversity: Vec<PreClDiversityResult>,
}

/// Cross-family narrator summary (computed corpus-wide).
#[derive(Debug, Clone, Serialize)]
pub struct CrossFamilyNarrator {
    pub narrator_id: String,
    pub cl_family_count: usize,
    pub reliability_prior: Option<f64>,
    pub reliability_rating: Option<String>,
}

/// Corpus-level summary.
#[derive(Debug, Serialize)]
pub struct CorpusJuynbollSummary {
    pub families_analyzed: usize,
    pub families_with_reliable_bypass: usize,
    pub families_with_independent_cls: usize,
    pub cross_family_narrators: Vec<CrossFamilyNarrator>,
}

// ── Test 1: Reliable Bypass Analysis ──

/// Compute reliable bypass analysis for a single CL in a family graph.
///
/// A bypass variant is one that contains both ancestors and descendants of the CL
/// but does not contain the CL itself. A bypass is "reliable" if all narrators in
/// the bypass path with known reliability have prior >= 0.65 (saduq or better).
pub(crate) fn compute_reliable_bypass(
    graph: &mut FamilyGraph,
    cl_id: &str,
) -> ReliableBypassResult {
    let total_variants = graph.variant_ids.len();

    // Extract missing variants before mutable borrows
    let missing: Vec<String> = match graph.nodes.get(cl_id) {
        Some(node) => graph
            .variant_ids
            .iter()
            .filter(|v| !node.variants.contains(*v))
            .cloned()
            .collect(),
        None => {
            return ReliableBypassResult {
                cl_narrator_id: cl_id.to_string(),
                total_variants,
                bypass_count: 0,
                reliable_bypass_count: 0,
                reliable_bypass_ratio: 0.0,
                details: vec![],
            };
        }
    };

    let ancestors = graph.reachable_set_cached(cl_id, Direction::Teachers);
    let descendants = graph.reachable_set_cached(cl_id, Direction::Students);

    let mut details = Vec::new();

    for variant in &missing {
        // Check if this variant has both an ancestor and a descendant of the CL
        let has_ancestor = graph
            .nodes
            .iter()
            .any(|(nid, n)| n.variants.contains(variant) && ancestors.contains(nid));
        let has_descendant = graph
            .nodes
            .iter()
            .any(|(nid, n)| n.variants.contains(variant) && descendants.contains(nid));

        if !has_ancestor || !has_descendant {
            continue;
        }

        // This is a bypass variant — collect narrators and their reliability
        let bypass_narrators: Vec<String> = graph
            .nodes
            .iter()
            .filter(|(nid, n)| n.variants.contains(variant) && *nid != cl_id)
            .map(|(nid, _)| nid.clone())
            .collect();

        let reliabilities: Vec<f64> = bypass_narrators
            .iter()
            .filter_map(|nid| graph.nodes.get(nid).and_then(|n| n.reliability_prior))
            .collect();

        let min_reliability = reliabilities.iter().copied().reduce(f64::min);
        let avg_reliability = if reliabilities.is_empty() {
            None
        } else {
            Some(reliabilities.iter().sum::<f64>() / reliabilities.len() as f64)
        };

        // "all_reliable" requires:
        // 1. At least one narrator has known reliability
        // 2. ALL narrators with known reliability have prior >= threshold
        let has_any_known = !reliabilities.is_empty();
        let all_known_reliable = reliabilities.iter().all(|r| *r >= RELIABLE_THRESHOLD);
        let all_reliable = has_any_known && all_known_reliable;

        details.push(BypassDetail {
            variant_id: variant.clone(),
            bypass_narrators,
            min_reliability,
            avg_reliability,
            all_reliable,
        });
    }

    let bypass_count = details.len();
    let reliable_bypass_count = details.iter().filter(|d| d.all_reliable).count();
    let reliable_bypass_ratio = if total_variants > 0 {
        reliable_bypass_count as f64 / total_variants as f64
    } else {
        0.0
    };

    ReliableBypassResult {
        cl_narrator_id: cl_id.to_string(),
        total_variants,
        bypass_count,
        reliable_bypass_count,
        reliable_bypass_ratio,
        details,
    }
}

// ── Test 2: Multiple Independent CLs ──

/// Check whether CLs in a family are independent (no ancestor-descendant relationship).
pub(crate) fn compute_independent_cls(
    graph: &mut FamilyGraph,
    cl_ids: &[String],
    family_id: &str,
) -> IndependentClResult {
    let mut pairs = Vec::new();
    let mut independent_count = 0;

    for i in 0..cl_ids.len() {
        for j in (i + 1)..cl_ids.len() {
            let cl_a = &cl_ids[i];
            let cl_b = &cl_ids[j];

            let ancestors_a = graph.reachable_set_cached(cl_a, Direction::Teachers);
            let descendants_a = graph.reachable_set_cached(cl_a, Direction::Students);

            let independent = !ancestors_a.contains(cl_b) && !descendants_a.contains(cl_b);
            if independent {
                independent_count += 1;
            }

            pairs.push(IndependentClPair {
                cl_a: cl_a.clone(),
                cl_b: cl_b.clone(),
                independent,
            });
        }
    }

    IndependentClResult {
        family_id: family_id.to_string(),
        cl_count: cl_ids.len(),
        pairs_checked: pairs.len(),
        independent_pairs: independent_count,
        pairs,
    }
}

// ── Test 4: Pre-CL Chain Diversity ──

/// Compute pre-CL chain diversity metrics by walking upstream from the CL.
pub(crate) fn compute_pre_cl_diversity(graph: &FamilyGraph, cl_id: &str) -> PreClDiversityResult {
    let mut visited = HashSet::new();
    let mut stack: Vec<String> = graph
        .nodes
        .get(cl_id)
        .map(|n| n.direct_teachers.iter().cloned().collect())
        .unwrap_or_default();

    let mut total_hops = 0usize;
    let mut single_strand = 0usize;
    let mut reliable_count = 0usize;
    let mut branching_points = 0usize;
    let mut with_bio_count = 0usize;

    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        total_hops += 1;

        if let Some(node) = graph.nodes.get(&current) {
            // Single-strand check (same as compute_pre_single_strand)
            if node.direct_students.len() == 1 {
                single_strand += 1;
            }
            // Branching point: narrator has > 1 student in the upstream chain
            if node.direct_students.len() > 1 {
                branching_points += 1;
            }
            // Reliability check
            if let Some(prior) = node.reliability_prior
                && prior >= RELIABLE_THRESHOLD
            {
                reliable_count += 1;
            }
            // Bio check
            if node.has_bio {
                with_bio_count += 1;
            }
            // Continue upstream
            for teacher in &node.direct_teachers {
                stack.push(teacher.clone());
            }
        }
    }

    let pre_single_strand_ratio = if total_hops == 0 {
        0.0
    } else {
        single_strand as f64 / total_hops as f64
    };

    let upstream_reliable_ratio = if total_hops == 0 {
        0.0
    } else {
        reliable_count as f64 / total_hops as f64
    };

    PreClDiversityResult {
        cl_narrator_id: cl_id.to_string(),
        pre_single_strand_ratio,
        upstream_narrator_count: total_hops,
        upstream_reliable_count: reliable_count,
        upstream_reliable_ratio,
        upstream_branching_points: branching_points,
        upstream_with_bio_count: with_bio_count,
    }
}

// ── Orchestrator ──

/// Run all per-family Juynboll falsifiability tests.
pub(crate) fn analyze_family_juynboll(
    graph: &mut FamilyGraph,
    cl_ids: &[String],
    family_id: &str,
) -> FamilyJuynbollResult {
    let reliable_bypasses: Vec<ReliableBypassResult> = cl_ids
        .iter()
        .map(|cl_id| compute_reliable_bypass(graph, cl_id))
        .collect();

    let independent_cls = if cl_ids.len() >= 2 {
        Some(compute_independent_cls(graph, cl_ids, family_id))
    } else {
        None
    };

    let pre_cl_diversity: Vec<PreClDiversityResult> = cl_ids
        .iter()
        .map(|cl_id| compute_pre_cl_diversity(graph, cl_id))
        .collect();

    FamilyJuynbollResult {
        family_id: family_id.to_string(),
        reliable_bypasses,
        independent_cls,
        pre_cl_diversity,
    }
}

// ── Test 3: Cross-Family CL Frequency (corpus-level) ──

/// Compute corpus-level cross-family CL frequency summary.
pub async fn compute_cross_family_summary(db: &Surreal<Db>) -> Result<CorpusJuynbollSummary> {
    // Count families analyzed (those with cl_analysis records)
    #[derive(Debug, SurrealValue)]
    struct CountResult {
        c: i64,
    }

    let mut res = db
        .query("SELECT family, count() AS c FROM cl_analysis GROUP BY family")
        .await?;
    #[derive(Debug, SurrealValue)]
    struct FamilyCount {
        c: i64,
    }
    let family_groups: Vec<FamilyCount> = res.take(0)?;
    let families_analyzed = family_groups.len() as i64;

    // Count families with reliable bypass
    let mut res = db
        .query(
            "SELECT count() AS c FROM juynboll_analysis WHERE has_reliable_bypass = true GROUP ALL",
        )
        .await?;
    let families_with_reliable_bypass: i64 = res
        .take::<Option<CountResult>>(0)?
        .map(|c| c.c)
        .unwrap_or(0);

    // Count families with independent CLs
    let mut res = db
        .query(
            "SELECT count() AS c FROM juynboll_analysis WHERE has_independent_cls = true GROUP ALL",
        )
        .await?;
    let families_with_independent_cls: i64 = res
        .take::<Option<CountResult>>(0)?
        .map(|c| c.c)
        .unwrap_or(0);

    // Cross-family CL frequency: narrators who are CL in multiple families
    #[derive(Debug, SurrealValue)]
    struct ClNarratorRow {
        narrator: RecordId,
        cnt: i64,
    }

    let mut res = db
        .query(
            "SELECT narrator, count() AS cnt \
             FROM cl_analysis WHERE candidate_type = 'CL' \
             GROUP BY narrator ORDER BY cnt DESC LIMIT 50",
        )
        .await?;
    let cl_narrators: Vec<ClNarratorRow> = res.take(0)?;

    #[derive(Debug, SurrealValue)]
    struct NarratorInfo {
        reliability_prior: Option<f64>,
        reliability_rating: Option<String>,
    }

    let mut cross_family_narrators = Vec::new();
    for row in &cl_narrators {
        let nid = crate::models::record_id_key_string(&row.narrator);
        let mut res = db
            .query("SELECT reliability_prior, reliability_rating FROM $rid")
            .bind(("rid", row.narrator.clone()))
            .await?;
        let info: Option<NarratorInfo> = res.take(0)?;

        cross_family_narrators.push(CrossFamilyNarrator {
            narrator_id: nid,
            cl_family_count: row.cnt as usize,
            reliability_prior: info.as_ref().and_then(|i| i.reliability_prior),
            reliability_rating: info.and_then(|i| i.reliability_rating),
        });
    }

    Ok(CorpusJuynbollSummary {
        families_analyzed: families_analyzed as usize,
        families_with_reliable_bypass: families_with_reliable_bypass as usize,
        families_with_independent_cls: families_with_independent_cls as usize,
        cross_family_narrators,
    })
}

// ── Storage ──

/// Store per-family Juynboll analysis results in the database.
pub async fn store_juynboll_results(db: &Surreal<Db>, result: &FamilyJuynbollResult) -> Result<()> {
    let has_reliable_bypass = result
        .reliable_bypasses
        .iter()
        .any(|r| r.reliable_bypass_count > 0);
    let reliable_bypass_count: usize = result
        .reliable_bypasses
        .iter()
        .map(|r| r.reliable_bypass_count)
        .sum();
    let max_reliable_bypass_ratio = result
        .reliable_bypasses
        .iter()
        .map(|r| r.reliable_bypass_ratio)
        .reduce(f64::max)
        .unwrap_or(0.0);
    let has_independent_cls = result
        .independent_cls
        .as_ref()
        .map(|r| r.independent_pairs > 0)
        .unwrap_or(false);
    let independent_cl_pairs = result
        .independent_cls
        .as_ref()
        .map(|r| r.independent_pairs)
        .unwrap_or(0);
    let cl_count = result.reliable_bypasses.len();
    let upstream_reliable_ratio = result
        .pre_cl_diversity
        .iter()
        .map(|d| d.upstream_reliable_ratio)
        .reduce(f64::max)
        .unwrap_or(0.0);
    let upstream_branching_points: usize = result
        .pre_cl_diversity
        .iter()
        .map(|d| d.upstream_branching_points)
        .sum();

    db.query(
        "CREATE $rid CONTENT { \
            family: $family, \
            has_reliable_bypass: $hrb, \
            reliable_bypass_count: $rbc, \
            max_reliable_bypass_ratio: $mrbr, \
            has_independent_cls: $hic, \
            independent_cl_pairs: $icp, \
            cl_count: $clc, \
            upstream_reliable_ratio: $urr, \
            upstream_branching_points: $ubp \
        }",
    )
    .bind((
        "rid",
        RecordId::new("juynboll_analysis", result.family_id.as_str()),
    ))
    .bind((
        "family",
        RecordId::new("hadith_family", result.family_id.as_str()),
    ))
    .bind(("hrb", has_reliable_bypass))
    .bind(("rbc", reliable_bypass_count as i64))
    .bind(("mrbr", max_reliable_bypass_ratio))
    .bind(("hic", has_independent_cls))
    .bind(("icp", independent_cl_pairs as i64))
    .bind(("clc", cl_count as i64))
    .bind(("urr", upstream_reliable_ratio))
    .bind(("ubp", upstream_branching_points as i64))
    .await?;

    Ok(())
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::super::cl_pcl::{Edge, FamilyGraph, NarratorNode};
    use super::*;
    use std::collections::HashMap;

    /// Build a test graph with the given narrators, edges, and variant assignments.
    fn build_test_graph(
        narrators: Vec<(&str, Option<f64>, bool)>, // (id, reliability_prior, has_bio)
        edges: Vec<(&str, &str, &str)>,            // (student, teacher, variant)
        variant_assignments: Vec<(&str, Vec<&str>)>, // (narrator, [variants])
        variant_ids: Vec<&str>,
    ) -> FamilyGraph {
        let mut nodes = HashMap::new();
        for (id, reliability, has_bio) in &narrators {
            nodes.insert(
                id.to_string(),
                NarratorNode {
                    variants: HashSet::new(),
                    direct_students: HashSet::new(),
                    direct_teachers: HashSet::new(),
                    has_bio: *has_bio,
                    generation: None,
                    reliability_prior: *reliability,
                },
            );
        }

        // Assign variants to narrators
        for (nid, variants) in &variant_assignments {
            if let Some(node) = nodes.get_mut(*nid) {
                for v in variants {
                    node.variants.insert(v.to_string());
                }
            }
        }

        let mut graph_edges: HashMap<String, Vec<Edge>> = HashMap::new();

        // Build adjacency + edges
        for (student, teacher, _) in &edges {
            if let Some(node) = nodes.get_mut(*student) {
                node.direct_teachers.insert(teacher.to_string());
            }
            if let Some(node) = nodes.get_mut(*teacher) {
                node.direct_students.insert(student.to_string());
            }
            let key = format!("{}->{}", student, teacher);
            graph_edges.entry(key).or_default().push(Edge {
                chronology_conflict: false,
            });
        }

        FamilyGraph::new(
            nodes,
            graph_edges,
            variant_ids.iter().map(|s| s.to_string()).collect(),
        )
    }

    // ── Test 1: Reliable Bypass ──

    // A bypass variant must contain BOTH a descendant AND an ancestor of the CL,
    // but NOT the CL itself. The variant takes an alternative path.
    // E.g.: v4: student_1 -> bypass_n -> teacher_a (bypasses CL)
    //   student_1 is a descendant of CL, teacher_a is an ancestor.

    #[test]
    fn test_reliable_bypass_detected() {
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("teacher_a", Some(0.75), true),
                ("student_1", Some(0.65), true),
                ("student_2", Some(0.65), true),
                ("student_3", Some(0.65), true),
                ("bypass_n", Some(0.75), true),
            ],
            vec![
                ("cl", "teacher_a", "v1"),
                ("student_1", "cl", "v1"),
                ("student_2", "cl", "v2"),
                ("student_3", "cl", "v3"),
                ("student_1", "bypass_n", "v4"),
                ("bypass_n", "teacher_a", "v4"),
            ],
            vec![
                ("cl", vec!["v1", "v2", "v3"]),
                ("teacher_a", vec!["v1", "v4"]),
                ("student_1", vec!["v1", "v4"]),
                ("student_2", vec!["v2"]),
                ("student_3", vec!["v3"]),
                ("bypass_n", vec!["v4"]),
            ],
            vec!["v1", "v2", "v3", "v4"],
        );

        let result = compute_reliable_bypass(&mut graph, "cl");
        assert_eq!(result.bypass_count, 1);
        assert_eq!(result.reliable_bypass_count, 1);
        assert!(result.details[0].all_reliable);
    }

    #[test]
    fn test_unreliable_bypass_not_counted() {
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("teacher_a", Some(0.75), true),
                ("student_1", Some(0.65), true),
                ("student_2", Some(0.65), true),
                ("student_3", Some(0.65), true),
                ("bypass_n", Some(0.20), true), // matruk
            ],
            vec![
                ("cl", "teacher_a", "v1"),
                ("student_1", "cl", "v1"),
                ("student_2", "cl", "v2"),
                ("student_3", "cl", "v3"),
                ("student_1", "bypass_n", "v4"),
                ("bypass_n", "teacher_a", "v4"),
            ],
            vec![
                ("cl", vec!["v1", "v2", "v3"]),
                ("teacher_a", vec!["v1", "v4"]),
                ("student_1", vec!["v1", "v4"]),
                ("student_2", vec!["v2"]),
                ("student_3", vec!["v3"]),
                ("bypass_n", vec!["v4"]),
            ],
            vec!["v1", "v2", "v3", "v4"],
        );

        let result = compute_reliable_bypass(&mut graph, "cl");
        assert_eq!(result.bypass_count, 1);
        assert_eq!(result.reliable_bypass_count, 0);
        assert!(!result.details[0].all_reliable);
    }

    #[test]
    fn test_bypass_unknown_reliability() {
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("teacher_a", Some(0.75), true),
                ("student_1", Some(0.65), true),
                ("student_2", Some(0.65), true),
                ("student_3", Some(0.65), true),
                ("bypass_n", None, false), // unknown reliability
            ],
            vec![
                ("cl", "teacher_a", "v1"),
                ("student_1", "cl", "v1"),
                ("student_2", "cl", "v2"),
                ("student_3", "cl", "v3"),
                ("student_1", "bypass_n", "v4"),
                ("bypass_n", "teacher_a", "v4"),
            ],
            vec![
                ("cl", vec!["v1", "v2", "v3"]),
                ("teacher_a", vec!["v1", "v4"]),
                ("student_1", vec!["v1", "v4"]),
                ("student_2", vec!["v2"]),
                ("student_3", vec!["v3"]),
                ("bypass_n", vec!["v4"]),
            ],
            vec!["v1", "v2", "v3", "v4"],
        );

        let result = compute_reliable_bypass(&mut graph, "cl");
        assert_eq!(result.bypass_count, 1);
        // bypass_n has None, but student_1 (0.65) and teacher_a (0.75) are known+reliable.
        // Unknown narrators don't disqualify — only known unreliable ones do.
        assert_eq!(result.reliable_bypass_count, 1);
    }

    #[test]
    fn test_bypass_mixed_reliability() {
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("teacher_a", Some(0.75), true),
                ("student_1", Some(0.65), true),
                ("student_2", Some(0.65), true),
                ("student_3", Some(0.65), true),
                ("bypass_n", Some(0.50), true), // majhul — below 0.65 threshold
            ],
            vec![
                ("cl", "teacher_a", "v1"),
                ("student_1", "cl", "v1"),
                ("student_2", "cl", "v2"),
                ("student_3", "cl", "v3"),
                ("student_1", "bypass_n", "v4"),
                ("bypass_n", "teacher_a", "v4"),
            ],
            vec![
                ("cl", vec!["v1", "v2", "v3"]),
                ("teacher_a", vec!["v1", "v4"]),
                ("student_1", vec!["v1", "v4"]),
                ("student_2", vec!["v2"]),
                ("student_3", vec!["v3"]),
                ("bypass_n", vec!["v4"]),
            ],
            vec!["v1", "v2", "v3", "v4"],
        );

        let result = compute_reliable_bypass(&mut graph, "cl");
        assert_eq!(result.bypass_count, 1);
        assert_eq!(result.reliable_bypass_count, 0);
        assert!(!result.details[0].all_reliable);
    }

    #[test]
    fn test_no_bypass_variants() {
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("teacher_a", Some(0.75), true),
                ("student_1", Some(0.65), true),
                ("student_2", Some(0.65), true),
            ],
            vec![
                ("cl", "teacher_a", "v1"),
                ("student_1", "cl", "v1"),
                ("student_2", "cl", "v2"),
                ("cl", "teacher_a", "v2"),
            ],
            vec![
                ("cl", vec!["v1", "v2"]),
                ("teacher_a", vec!["v1", "v2"]),
                ("student_1", vec!["v1"]),
                ("student_2", vec!["v2"]),
            ],
            vec!["v1", "v2"],
        );

        let result = compute_reliable_bypass(&mut graph, "cl");
        assert_eq!(result.bypass_count, 0);
        assert_eq!(result.reliable_bypass_count, 0);
    }

    // ── Test 2: Independent CLs ──

    #[test]
    fn test_independent_cls_detected() {
        // Two CLs on separate branches — no ancestor/descendant relationship
        let mut graph = build_test_graph(
            vec![
                ("source", Some(0.75), true),
                ("cl_a", Some(0.75), true),
                ("cl_b", Some(0.75), true),
                ("s1", Some(0.65), true),
                ("s2", Some(0.65), true),
                ("s3", Some(0.65), true),
                ("s4", Some(0.65), true),
            ],
            vec![
                ("cl_a", "source", "v1"),
                ("s1", "cl_a", "v1"),
                ("s2", "cl_a", "v2"),
                ("cl_b", "source", "v3"),
                ("s3", "cl_b", "v3"),
                ("s4", "cl_b", "v4"),
            ],
            vec![
                ("source", vec!["v1", "v3"]),
                ("cl_a", vec!["v1", "v2"]),
                ("cl_b", vec!["v3", "v4"]),
                ("s1", vec!["v1"]),
                ("s2", vec!["v2"]),
                ("s3", vec!["v3"]),
                ("s4", vec!["v4"]),
            ],
            vec!["v1", "v2", "v3", "v4"],
        );

        let result = compute_independent_cls(
            &mut graph,
            &["cl_a".to_string(), "cl_b".to_string()],
            "fam1",
        );
        assert_eq!(result.pairs_checked, 1);
        assert_eq!(result.independent_pairs, 1);
        assert!(result.pairs[0].independent);
    }

    #[test]
    fn test_dependent_cls_not_independent() {
        // cl_b is a student of cl_a — they are in ancestor-descendant relationship
        let mut graph = build_test_graph(
            vec![
                ("source", Some(0.75), true),
                ("cl_a", Some(0.75), true),
                ("cl_b", Some(0.75), true),
                ("s1", Some(0.65), true),
                ("s2", Some(0.65), true),
            ],
            vec![
                ("cl_a", "source", "v1"),
                ("cl_b", "cl_a", "v1"),
                ("s1", "cl_b", "v1"),
                ("s2", "cl_b", "v2"),
            ],
            vec![
                ("source", vec!["v1"]),
                ("cl_a", vec!["v1"]),
                ("cl_b", vec!["v1", "v2"]),
                ("s1", vec!["v1"]),
                ("s2", vec!["v2"]),
            ],
            vec!["v1", "v2"],
        );

        let result = compute_independent_cls(
            &mut graph,
            &["cl_a".to_string(), "cl_b".to_string()],
            "fam1",
        );
        assert_eq!(result.pairs_checked, 1);
        assert_eq!(result.independent_pairs, 0);
        assert!(!result.pairs[0].independent);
    }

    #[test]
    fn test_single_cl_no_pairs() {
        let mut graph = build_test_graph(
            vec![("cl", Some(0.75), true)],
            vec![],
            vec![("cl", vec!["v1"])],
            vec!["v1"],
        );

        let result = compute_independent_cls(&mut graph, &["cl".to_string()], "fam1");
        assert_eq!(result.pairs_checked, 0);
        assert_eq!(result.independent_pairs, 0);
    }

    // ── Test 4: Pre-CL Diversity ──

    #[test]
    fn test_pre_cl_diversity_all_reliable() {
        // CL with upstream narrators all having high reliability
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("t1", Some(0.75), true), // thiqah
                ("t2", Some(0.65), true), // saduq
            ],
            vec![("cl", "t1", "v1"), ("t1", "t2", "v1")],
            vec![("cl", vec!["v1"]), ("t1", vec!["v1"]), ("t2", vec!["v1"])],
            vec!["v1"],
        );

        let result = compute_pre_cl_diversity(&graph, "cl");
        assert_eq!(result.upstream_narrator_count, 2);
        assert_eq!(result.upstream_reliable_count, 2);
        assert!((result.upstream_reliable_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_pre_cl_diversity_branching() {
        // CL with upstream narrators that have multiple students (branching)
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("t1", Some(0.75), true),
                ("t2", Some(0.65), true),
                ("other_student", Some(0.65), true), // gives t1 two students
            ],
            vec![
                ("cl", "t1", "v1"),
                ("t1", "t2", "v1"),
                ("other_student", "t1", "v2"),
            ],
            vec![
                ("cl", vec!["v1"]),
                ("t1", vec!["v1", "v2"]),
                ("t2", vec!["v1"]),
                ("other_student", vec!["v2"]),
            ],
            vec!["v1", "v2"],
        );

        let result = compute_pre_cl_diversity(&graph, "cl");
        assert_eq!(result.upstream_narrator_count, 2); // t1 and t2
        assert!(result.upstream_branching_points > 0); // t1 has 2 students
    }

    #[test]
    fn test_pre_cl_diversity_single_strand() {
        // CL with linear upstream chain (all single-student)
        let mut graph = build_test_graph(
            vec![
                ("cl", Some(0.75), true),
                ("t1", Some(0.75), true),
                ("t2", Some(0.65), true),
            ],
            vec![("cl", "t1", "v1"), ("t1", "t2", "v1")],
            vec![("cl", vec!["v1"]), ("t1", vec!["v1"]), ("t2", vec!["v1"])],
            vec!["v1"],
        );

        let result = compute_pre_cl_diversity(&graph, "cl");
        // t1 has 1 student (cl), t2 has 1 student (t1) — all single-strand
        assert_eq!(result.upstream_branching_points, 0);
        assert!((result.pre_single_strand_ratio - 1.0).abs() < f64::EPSILON);
    }
}
