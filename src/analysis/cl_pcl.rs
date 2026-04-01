//! CL/PCL (Common Link / Partial Common Link) analysis engine.
//!
//! Implements Juynboll's methodology for identifying key transmission narrators
//! in hadith isnad chains using graph-theoretic analysis.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

// ── Constants (matching Riwaq clpcl-analyzer.js) ──

const DEFAULT_MATN_COHERENCE: f64 = 0.50;
const CONTRADICTION_CAP: f64 = 0.70;
const SUPPORTED_THRESHOLD: f64 = 0.75;
const CONTESTED_LOW: f64 = 0.55;
const UNCERTAIN_LOW: f64 = 0.35;

// CL candidate thresholds
const CL_MIN_FAN_OUT: usize = 3;
const CL_MIN_BUNDLE_COVERAGE: f64 = 0.35;
const CL_MIN_COLLECTOR_DIVERSITY: usize = 3;

// PCL candidate thresholds
const PCL_MIN_FAN_OUT: usize = 2;
const PCL_MIN_BUNDLE_COVERAGE: f64 = 0.20;

// ── Internal types ──

#[derive(Debug, SurrealValue)]
struct HeardFromRow {
    student: RecordId,
    teacher: RecordId,
    hadith_ref: Option<RecordId>,
}

#[derive(Debug, SurrealValue)]
struct NarratesRow {
    narrator: RecordId,
    hadith: RecordId,
}

#[derive(Debug, SurrealValue)]
struct NarratorBio {
    id: Option<RecordId>,
    bio: Option<String>,
    generation: Option<String>,
    reliability_prior: Option<f64>,
}

/// Per-narrator feature vector.
#[derive(Debug, Clone, Serialize)]
pub struct FeatureVector {
    pub fan_out: usize,
    pub bundle_coverage: f64,
    pub collector_diversity: usize,
    pub pre_single_strand_ratio: f64,
    pub bypass_ratio: f64,
    pub chronology_conflict_ratio: f64,
    pub matn_coherence: f64,
    pub provenance_completeness: f64,
}

/// A CL/PCL candidate with scores.
#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub narrator_id: String,
    pub candidate_type: String, // "CL" or "PCL"
    pub pcl_mode: Option<String>,
    pub structural_score: f64,
    pub final_confidence: f64,
    pub outcome: String,
    pub contradiction_cap_active: bool,
    pub profile: String,
    pub features: FeatureVector,
    pub rank: usize,
    pub family_status: String,
}

/// Result of analyzing one hadith family.
#[derive(Debug, Serialize)]
pub struct FamilyAnalysisResult {
    pub family_id: String,
    pub family_status: String, // cl_detected, pcl_only, insufficient_data
    pub profile: String,
    pub candidates: Vec<Candidate>,
    pub juynboll: Option<super::juynboll::FamilyJuynbollResult>,
}

// ── Graph structures ──

pub(crate) struct NarratorNode {
    pub(crate) variants: HashSet<String>, // hadith keys containing this narrator
    pub(crate) direct_students: HashSet<String>, // narrators who heard from this one
    pub(crate) direct_teachers: HashSet<String>, // narrators this one heard from
    pub(crate) has_bio: bool,
    pub(crate) generation: Option<i64>, // parsed generation for chronology
    pub(crate) reliability_prior: Option<f64>,
}

pub(crate) struct Edge {
    pub(crate) variant_id: String,
    pub(crate) chronology_conflict: bool,
}

pub(crate) struct FamilyGraph {
    pub(crate) nodes: HashMap<String, NarratorNode>,
    pub(crate) edges: HashMap<String, Vec<Edge>>, // key: "student->teacher"
    pub(crate) variant_ids: HashSet<String>,
}

fn norm(val: f64, min: f64, max: f64) -> f64 {
    ((val - min) / (max - min)).clamp(0.0, 1.0)
}

/// Parse a generation string to a rough numeric ordering.
fn parse_generation(generation: &str) -> Option<i64> {
    let lower = generation.to_lowercase();
    if lower.contains("sahab") {
        Some(1)
    } else if lower.contains("tabi") && lower.contains("tabi") {
        Some(3)
    }
    // tabi' al-tabi'in
    else if lower.contains("tabi") {
        Some(2)
    } else {
        None
    }
}

// ── Core analysis ──

impl FamilyGraph {
    fn compute_features(&self) -> HashMap<String, FeatureVector> {
        let total_variants = self.variant_ids.len();
        if total_variants == 0 {
            return HashMap::new();
        }

        let mut features = HashMap::new();

        for (nid, node) in &self.nodes {
            // 1. Fan-out
            let fan_out = node.direct_students.len();

            // 2. Bundle coverage
            let bundle_coverage = node.variants.len() as f64 / total_variants as f64;

            // 3. Collector diversity: distinct terminal narrators reachable downstream
            let collectors = self.reachable_terminals(nid);
            let collector_diversity = collectors.len();

            // 4. Pre-single-strand ratio
            let pre_single_strand_ratio = self.compute_pre_single_strand(nid);

            // 5. Bypass ratio
            let bypass_ratio = self.compute_bypass_ratio(nid);

            // 6. Chronology conflict ratio
            let chronology_conflict_ratio = self.compute_chronology_ratio(nid);

            // 7. Matn coherence (default)
            let matn_coherence = DEFAULT_MATN_COHERENCE;

            // 8. Provenance completeness
            let provenance_completeness = self.compute_provenance(nid);

            features.insert(
                nid.clone(),
                FeatureVector {
                    fan_out,
                    bundle_coverage,
                    collector_diversity,
                    pre_single_strand_ratio,
                    bypass_ratio,
                    chronology_conflict_ratio,
                    matn_coherence,
                    provenance_completeness,
                },
            );
        }

        features
    }

    /// Find terminal narrators (leaves with no students) reachable from `start` via students.
    pub(crate) fn reachable_terminals(&self, start: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut stack = vec![start.to_string()];
        let mut terminals = HashSet::new();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get(&current) {
                if node.direct_students.is_empty() && current != start {
                    terminals.insert(current);
                } else {
                    for student in &node.direct_students {
                        stack.push(student.clone());
                    }
                }
            }
        }
        terminals
    }

    /// Compute pre-single-strand ratio: fraction of upstream (teacher-side) hops
    /// that are single-strand (teacher has exactly 1 student).
    fn compute_pre_single_strand(&self, nid: &str) -> f64 {
        let mut total_hops = 0usize;
        let mut single_strand = 0usize;

        // Walk upstream (toward Prophet) via teachers
        let mut visited = HashSet::new();
        let mut stack: Vec<String> = self
            .nodes
            .get(nid)
            .map(|n| n.direct_teachers.iter().cloned().collect())
            .unwrap_or_default();

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            total_hops += 1;
            if let Some(node) = self.nodes.get(&current) {
                if node.direct_students.len() == 1 {
                    single_strand += 1;
                }
                for teacher in &node.direct_teachers {
                    stack.push(teacher.clone());
                }
            }
        }

        if total_hops == 0 {
            0.0
        } else {
            single_strand as f64 / total_hops as f64
        }
    }

    /// Compute bypass ratio: fraction of variants that bypass this narrator
    /// (have both ancestors and descendants but don't include this narrator).
    fn compute_bypass_ratio(&self, nid: &str) -> f64 {
        let node = match self.nodes.get(nid) {
            Some(n) => n,
            None => return 0.0,
        };

        let total = self.variant_ids.len();
        if total == 0 {
            return 0.0;
        }

        // Variants NOT containing this narrator
        let missing: Vec<&String> = self
            .variant_ids
            .iter()
            .filter(|v| !node.variants.contains(*v))
            .collect();

        let ancestors = self.reachable_set(nid, Direction::Teachers);
        let descendants = self.reachable_set(nid, Direction::Students);

        let mut bypass_count = 0;
        for variant in &missing {
            // Check if any narrator in this variant is an ancestor AND any is a descendant
            let has_ancestor = self
                .nodes
                .values()
                .any(|n| n.variants.contains(*variant) && ancestors.contains(&self.node_key(n)));
            let has_descendant = self
                .nodes
                .values()
                .any(|n| n.variants.contains(*variant) && descendants.contains(&self.node_key(n)));
            if has_ancestor && has_descendant {
                bypass_count += 1;
            }
        }

        bypass_count as f64 / total as f64
    }

    pub(crate) fn node_key(&self, _node: &NarratorNode) -> String {
        // Find the key for this node (reverse lookup)
        for (k, v) in &self.nodes {
            if std::ptr::eq(v, _node) {
                return k.clone();
            }
        }
        String::new()
    }

    pub(crate) fn reachable_set(&self, start: &str, dir: Direction) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut stack = vec![start.to_string()];
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get(&current) {
                let neighbors = match dir {
                    Direction::Teachers => &node.direct_teachers,
                    Direction::Students => &node.direct_students,
                };
                for n in neighbors {
                    stack.push(n.clone());
                }
            }
        }
        visited.remove(start);
        visited
    }

    fn compute_chronology_ratio(&self, nid: &str) -> f64 {
        let mut total_edges = 0usize;
        let mut conflict_edges = 0usize;

        for (edge_key, edges) in &self.edges {
            let parts: Vec<&str> = edge_key.split("->").collect();
            if parts.len() != 2 {
                continue;
            }
            if parts[0] != nid && parts[1] != nid {
                continue;
            }
            for edge in edges {
                total_edges += 1;
                if edge.chronology_conflict {
                    conflict_edges += 1;
                }
            }
        }

        if total_edges == 0 {
            0.0
        } else {
            conflict_edges as f64 / total_edges as f64
        }
    }

    fn compute_provenance(&self, nid: &str) -> f64 {
        // Fraction of narrators in this narrator's chains that have biographical data
        let _node = match self.nodes.get(nid) {
            Some(n) => n,
            None => return 1.0,
        };

        let chain_narrators: HashSet<String> = {
            let mut all = self.reachable_set(nid, Direction::Teachers);
            all.extend(self.reachable_set(nid, Direction::Students));
            all.insert(nid.to_string());
            all
        };

        if chain_narrators.is_empty() {
            return 1.0;
        }

        let with_bio = chain_narrators
            .iter()
            .filter(|n| self.nodes.get(*n).map(|node| node.has_bio).unwrap_or(false))
            .count();

        with_bio as f64 / chain_narrators.len() as f64
    }
}

pub(crate) enum Direction {
    Teachers,
    Students,
}

/// Compute structural score from features (exact Riwaq formula).
fn compute_structural_score(f: &FeatureVector) -> f64 {
    // S1 = pre_single_strand_ratio (weight 0.30)
    let s1 = f.pre_single_strand_ratio;
    // S2 = bundle_coverage (weight 0.25)
    let s2 = f.bundle_coverage;
    // S3 = collector_diversity normalized to [2, 8] (weight 0.15)
    let s3 = norm(f.collector_diversity as f64, 2.0, 8.0);
    // S4 = fan_out normalized to [3, 8] (weight 0.20)
    let s4 = norm(f.fan_out as f64, 3.0, 8.0);
    // S5 = matn_coherence (weight 0.10)
    let s5 = f.matn_coherence;

    // Penalties
    let p1 = f.bypass_ratio;
    let p2 = f.chronology_conflict_ratio;
    let p3 = 1.0 - f.provenance_completeness;

    let raw = 0.30 * s1 + 0.25 * s2 + 0.15 * s3 + 0.20 * s4 + 0.10 * s5
        - 0.20 * p1
        - 0.10 * p2
        - 0.05 * p3;

    raw.clamp(0.0, 1.0)
}

fn map_outcome(confidence: f64, contradiction_active: bool) -> &'static str {
    if !contradiction_active && confidence >= SUPPORTED_THRESHOLD {
        "supported"
    } else if confidence >= CONTESTED_LOW {
        "contested"
    } else if confidence >= UNCERTAIN_LOW {
        "uncertain"
    } else {
        "likely_weak_in_context"
    }
}

/// Build a FamilyGraph from database data for a given family.
/// Returns None if the family has fewer than 2 hadiths.
pub(crate) async fn build_family_graph(
    db: &Surreal<Db>,
    family_id: &str,
) -> Result<Option<FamilyGraph>> {
    let family_rid = RecordId::new("hadith_family", family_id);

    // 1. Get all hadiths in this family
    let mut res = db
        .query("SELECT id FROM hadith WHERE family_id = $fid")
        .bind(("fid", family_rid.clone()))
        .await?;

    #[derive(Debug, SurrealValue)]
    struct IdOnly {
        id: Option<RecordId>,
    }
    let hadith_ids: Vec<IdOnly> = res.take(0)?;

    if hadith_ids.len() < 2 {
        return Ok(None);
    }

    let hadith_keys: Vec<String> = hadith_ids
        .iter()
        .filter_map(|h| h.id.as_ref().map(crate::models::record_id_key_string))
        .collect();

    // 2. Fetch all heard_from edges for these hadiths
    let mut res = db
        .query(
            "SELECT in AS student, out AS teacher, hadith_ref \
             FROM heard_from WHERE hadith_ref IN $hids",
        )
        .bind((
            "hids",
            hadith_ids
                .iter()
                .filter_map(|h| h.id.clone())
                .collect::<Vec<RecordId>>(),
        ))
        .await?;
    let edges: Vec<HeardFromRow> = res.take(0)?;

    // 3. Fetch narrates edges to know which narrators appear in which hadiths
    let mut res = db
        .query(
            "SELECT in AS narrator, out AS hadith \
             FROM narrates WHERE out IN $hids",
        )
        .bind((
            "hids",
            hadith_ids
                .iter()
                .filter_map(|h| h.id.clone())
                .collect::<Vec<RecordId>>(),
        ))
        .await?;
    let narrates: Vec<NarratesRow> = res.take(0)?;

    // 4. Fetch narrator bio data for provenance + reliability
    let narrator_ids: HashSet<String> = {
        let mut ids = HashSet::new();
        for e in &edges {
            ids.insert(crate::models::record_id_key_string(&e.student));
            ids.insert(crate::models::record_id_key_string(&e.teacher));
        }
        for n in &narrates {
            ids.insert(crate::models::record_id_key_string(&n.narrator));
        }
        ids
    };

    let mut bio_map: HashMap<String, NarratorBio> = HashMap::new();
    for nid in &narrator_ids {
        let mut res = db
            .query("SELECT id, bio, generation, reliability_prior FROM $rid")
            .bind(("rid", RecordId::new("narrator", nid.as_str())))
            .await?;
        let bio: Option<NarratorBio> = res.take(0)?;
        if let Some(b) = bio {
            bio_map.insert(nid.clone(), b);
        }
    }

    // 5. Build the family graph
    let mut graph = FamilyGraph {
        nodes: HashMap::new(),
        edges: HashMap::new(),
        variant_ids: hadith_keys.iter().cloned().collect(),
    };

    // Initialize nodes from narrates edges
    for nr in &narrates {
        let nkey = crate::models::record_id_key_string(&nr.narrator);
        let hkey = crate::models::record_id_key_string(&nr.hadith);
        let node = graph.nodes.entry(nkey.clone()).or_insert_with(|| {
            let bio = bio_map.get(&nkey);
            NarratorNode {
                variants: HashSet::new(),
                direct_students: HashSet::new(),
                direct_teachers: HashSet::new(),
                has_bio: bio.map(|b| b.bio.is_some()).unwrap_or(false),
                generation: bio
                    .and_then(|b| b.generation.as_deref())
                    .and_then(parse_generation),
                reliability_prior: bio.and_then(|b| b.reliability_prior),
            }
        });
        node.variants.insert(hkey);
    }

    // Add edges
    for e in &edges {
        let student_key = crate::models::record_id_key_string(&e.student);
        let teacher_key = crate::models::record_id_key_string(&e.teacher);
        let variant_key = e
            .hadith_ref
            .as_ref()
            .map(crate::models::record_id_key_string)
            .unwrap_or_default();

        // Check chronology conflict
        let student_gen = graph.nodes.get(&student_key).and_then(|n| n.generation);
        let teacher_gen = graph.nodes.get(&teacher_key).and_then(|n| n.generation);
        let chronology_conflict = match (student_gen, teacher_gen) {
            (Some(sg), Some(tg)) => sg < tg, // student generation before teacher = conflict
            _ => false,
        };

        // Update adjacency
        if let Some(node) = graph.nodes.get_mut(&student_key) {
            node.direct_teachers.insert(teacher_key.clone());
        }
        if let Some(node) = graph.nodes.get_mut(&teacher_key) {
            node.direct_students.insert(student_key.clone());
        }

        let edge_key = format!("{}->{}", student_key, teacher_key);
        graph.edges.entry(edge_key).or_default().push(Edge {
            variant_id: variant_key,
            chronology_conflict,
        });
    }

    Ok(Some(graph))
}

/// Analyze a single hadith family for CL/PCL candidates.
pub async fn analyze_family(
    db: &Surreal<Db>,
    family_id: &str,
    profile: &str, // "structural_only" or "reliability_weighted"
) -> Result<FamilyAnalysisResult> {
    let graph = match build_family_graph(db, family_id).await? {
        Some(g) => g,
        None => {
            return Ok(FamilyAnalysisResult {
                family_id: family_id.to_string(),
                family_status: "insufficient_data".to_string(),
                profile: profile.to_string(),
                candidates: vec![],
                juynboll: None,
            });
        }
    };

    // 6. Compute features
    let features = graph.compute_features();

    // 7. Generate candidates
    let mut cl_candidates: Vec<String> = Vec::new();
    let mut pcl_candidates: Vec<(String, String)> = Vec::new(); // (id, mode)

    for (nid, f) in &features {
        if f.fan_out >= CL_MIN_FAN_OUT
            && f.bundle_coverage >= CL_MIN_BUNDLE_COVERAGE
            && f.collector_diversity >= CL_MIN_COLLECTOR_DIVERSITY
        {
            cl_candidates.push(nid.clone());
        }
    }

    let cl_set: HashSet<&String> = cl_candidates.iter().collect();
    for (nid, f) in &features {
        if cl_set.contains(nid) {
            continue;
        }
        if f.fan_out >= PCL_MIN_FAN_OUT && f.bundle_coverage >= PCL_MIN_BUNDLE_COVERAGE {
            // Determine mode
            let ancestors = graph.reachable_set(nid, Direction::Teachers);
            let is_cl_anchored = ancestors.iter().any(|a| cl_set.contains(a));
            let mode = if is_cl_anchored {
                "cl_anchored"
            } else if f.collector_diversity >= 2 {
                "fallback"
            } else {
                continue;
            };
            pcl_candidates.push((nid.clone(), mode.to_string()));
        }
    }

    // 8. Score and classify
    let mut candidates: Vec<Candidate> = Vec::new();

    for nid in &cl_candidates {
        let f = &features[nid];
        let structural = compute_structural_score(f);
        let reliability_prior = graph.nodes.get(nid).and_then(|n| n.reliability_prior);

        let (final_conf, contradiction_active) = compute_final_confidence(
            structural,
            reliability_prior,
            profile,
            false, // TODO: detect contradictions from reliability layer
        );

        candidates.push(Candidate {
            narrator_id: nid.clone(),
            candidate_type: "CL".to_string(),
            pcl_mode: None,
            structural_score: structural,
            final_confidence: final_conf,
            outcome: map_outcome(final_conf, contradiction_active).to_string(),
            contradiction_cap_active: contradiction_active,
            profile: profile.to_string(),
            features: f.clone(),
            rank: 0,
            family_status: String::new(),
        });
    }

    for (nid, mode) in &pcl_candidates {
        let f = &features[nid];
        let structural = compute_structural_score(f);
        let reliability_prior = graph.nodes.get(nid).and_then(|n| n.reliability_prior);

        let (final_conf, contradiction_active) =
            compute_final_confidence(structural, reliability_prior, profile, false);

        candidates.push(Candidate {
            narrator_id: nid.clone(),
            candidate_type: "PCL".to_string(),
            pcl_mode: Some(mode.clone()),
            structural_score: structural,
            final_confidence: final_conf,
            outcome: map_outcome(final_conf, contradiction_active).to_string(),
            contradiction_cap_active: contradiction_active,
            profile: profile.to_string(),
            features: f.clone(),
            rank: 0,
            family_status: String::new(),
        });
    }

    // 9. Deterministic ranking (5-level tiebreaker)
    candidates.sort_by(|a, b| {
        b.final_confidence
            .partial_cmp(&a.final_confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                b.features
                    .bundle_coverage
                    .partial_cmp(&a.features.bundle_coverage)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(b.features.fan_out.cmp(&a.features.fan_out))
            .then(
                a.features
                    .bypass_ratio
                    .partial_cmp(&b.features.bypass_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(a.narrator_id.cmp(&b.narrator_id))
    });

    // Assign ranks
    for (i, c) in candidates.iter_mut().enumerate() {
        c.rank = i + 1;
    }

    let family_status = if !cl_candidates.is_empty() {
        "cl_detected"
    } else if !pcl_candidates.is_empty() {
        "pcl_only"
    } else {
        "insufficient_data"
    };

    for c in &mut candidates {
        c.family_status = family_status.to_string();
    }

    // 10. Juynboll falsifiability analysis (runs on families with CLs)
    let juynboll = if !cl_candidates.is_empty() {
        Some(super::juynboll::analyze_family_juynboll(
            &graph,
            &cl_candidates,
            family_id,
        ))
    } else {
        None
    };

    Ok(FamilyAnalysisResult {
        family_id: family_id.to_string(),
        family_status: family_status.to_string(),
        profile: profile.to_string(),
        candidates,
        juynboll,
    })
}

fn compute_final_confidence(
    structural: f64,
    reliability_prior: Option<f64>,
    profile: &str,
    has_contradiction: bool,
) -> (f64, bool) {
    let mut confidence = if profile == "reliability_weighted" {
        let prior = reliability_prior.unwrap_or(0.50);
        (0.65 * structural + 0.35 * prior).clamp(0.0, 1.0)
    } else {
        structural
    };

    let contradiction_active = if has_contradiction {
        confidence = confidence.min(CONTRADICTION_CAP);
        true
    } else {
        false
    };

    (confidence, contradiction_active)
}

/// Store analysis results in the cl_analysis table.
pub async fn store_results(db: &Surreal<Db>, result: &FamilyAnalysisResult) -> Result<()> {
    for c in &result.candidates {
        let slug = format!("{}_{}", result.family_id, c.narrator_id);
        db.query(
            "CREATE $rid CONTENT { \
                family: $family, narrator: $narrator, \
                candidate_type: $ctype, pcl_mode: $pcl_mode, \
                fan_out: $fan_out, bundle_coverage: $coverage, \
                collector_diversity: $diversity, \
                pre_single_strand_ratio: $pssr, \
                bypass_ratio: $bypass, \
                chronology_conflict_ratio: $chrono, \
                matn_coherence: $matn, \
                provenance_completeness: $prov, \
                structural_score: $structural, \
                reliability_prior: $rprior, \
                final_confidence: $confidence, \
                outcome: $outcome, \
                contradiction_cap_active: $cap, \
                profile: $profile, \
                family_status: $fstatus, \
                rank: $rank \
            }",
        )
        .bind(("rid", RecordId::new("cl_analysis", slug.as_str())))
        .bind((
            "family",
            RecordId::new("hadith_family", result.family_id.as_str()),
        ))
        .bind((
            "narrator",
            RecordId::new("narrator", c.narrator_id.as_str()),
        ))
        .bind(("ctype", c.candidate_type.clone()))
        .bind(("pcl_mode", c.pcl_mode.clone()))
        .bind(("fan_out", c.features.fan_out as i64))
        .bind(("coverage", c.features.bundle_coverage))
        .bind(("diversity", c.features.collector_diversity as i64))
        .bind(("pssr", c.features.pre_single_strand_ratio))
        .bind(("bypass", c.features.bypass_ratio))
        .bind(("chrono", c.features.chronology_conflict_ratio))
        .bind(("matn", c.features.matn_coherence))
        .bind(("prov", c.features.provenance_completeness))
        .bind(("structural", c.structural_score))
        .bind(("rprior", c.features.provenance_completeness)) // placeholder
        .bind(("confidence", c.final_confidence))
        .bind(("outcome", c.outcome.clone()))
        .bind(("cap", c.contradiction_cap_active))
        .bind(("profile", c.profile.clone()))
        .bind(("fstatus", c.family_status.clone()))
        .bind(("rank", c.rank as i64))
        .await?;
    }
    Ok(())
}
