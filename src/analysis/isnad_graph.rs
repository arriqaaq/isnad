//! Isnad graph infrastructure for hadith transmission chain analysis.
//!
//! Provides the core graph structures and traversal methods used by
//! both the mustalah analysis engine and any structural analysis.

use std::collections::{HashMap, HashSet};

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;

// ── Internal DB row types ──

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
    reliability_rating: Option<String>,
}

// ── Graph structures ──

pub struct NarratorNode {
    pub variants: HashSet<String>, // hadith keys containing this narrator
    pub direct_students: HashSet<String>, // narrators who heard from this one
    pub direct_teachers: HashSet<String>, // narrators this one heard from
    pub has_bio: bool,
    pub generation: Option<i64>, // parsed tabaqah number (1=sahabi, 2=tabi'i, ...)
    pub reliability_prior: Option<f64>,
    pub reliability_rating: Option<String>, // e.g. "thiqah", "saduq", "daif"
}

pub struct Edge {
    pub chronology_conflict: bool,
}

pub struct FamilyGraph {
    pub nodes: HashMap<String, NarratorNode>,
    pub edges: HashMap<String, Vec<Edge>>, // key: "student->teacher"
    pub variant_ids: HashSet<String>,
    ancestors_cache: HashMap<String, HashSet<String>>,
    descendants_cache: HashMap<String, HashSet<String>>,
    variant_narrator_map: Option<HashMap<String, HashSet<String>>>,
}

#[derive(Clone, Copy)]
pub enum Direction {
    Teachers,
    Students,
}

// ── Helpers ──

/// Normalize a value to [0, 1] given a min/max range.
pub fn norm(val: f64, min: f64, max: f64) -> f64 {
    ((val - min) / (max - min)).clamp(0.0, 1.0)
}

/// Parse a generation string to an integer tabaqah number.
///
/// Handles both numeric strings ("1", "2", "10") from the ingested data
/// and text-based strings ("sahabi", "tabi'i") from older data formats.
pub fn parse_generation(generation: &str) -> Option<i64> {
    // First try direct numeric parse (e.g. "1", "2", "10", "6?")
    let trimmed = generation.trim().trim_end_matches('?');
    if let Ok(n) = trimmed.parse::<i64>() {
        return Some(n);
    }

    // Fall back to text-based parsing
    let lower = generation.to_lowercase();
    if lower.contains("sahab") {
        Some(1)
    } else if lower.contains("tabi") && lower.contains("tabi") {
        // tabi' al-tabi'in
        Some(3)
    } else if lower.contains("tabi") {
        Some(2)
    } else {
        None
    }
}

// ── FamilyGraph implementation ──

impl FamilyGraph {
    pub fn new(
        nodes: HashMap<String, NarratorNode>,
        edges: HashMap<String, Vec<Edge>>,
        variant_ids: HashSet<String>,
    ) -> Self {
        Self {
            nodes,
            edges,
            variant_ids,
            ancestors_cache: HashMap::new(),
            descendants_cache: HashMap::new(),
            variant_narrator_map: None,
        }
    }

    /// Find terminal narrators (leaves with no students) reachable from `start` via students.
    pub fn reachable_terminals(&self, start: &str) -> HashSet<String> {
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

    /// Compute reachable set in a given direction (teachers = upstream, students = downstream).
    pub fn reachable_set(&self, start: &str, dir: Direction) -> HashSet<String> {
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

    /// Cached version of reachable_set.
    pub fn reachable_set_cached(&mut self, nid: &str, dir: Direction) -> HashSet<String> {
        let cache = match dir {
            Direction::Teachers => &self.ancestors_cache,
            Direction::Students => &self.descendants_cache,
        };
        if let Some(cached) = cache.get(nid) {
            return cached.clone();
        }
        let result = self.reachable_set(nid, dir);
        let cache = match dir {
            Direction::Teachers => &mut self.ancestors_cache,
            Direction::Students => &mut self.descendants_cache,
        };
        cache.insert(nid.to_string(), result.clone());
        result
    }

    /// Build the variant→narrator reverse map (lazy, built once).
    pub fn ensure_variant_narrator_map(&mut self) {
        if self.variant_narrator_map.is_some() {
            return;
        }
        let mut map: HashMap<String, HashSet<String>> = HashMap::new();
        for (nid, node) in &self.nodes {
            for vid in &node.variants {
                map.entry(vid.clone()).or_default().insert(nid.clone());
            }
        }
        self.variant_narrator_map = Some(map);
    }

    /// Get the variant→narrator map (must call ensure_variant_narrator_map first).
    pub fn variant_narrator_map(&self) -> Option<&HashMap<String, HashSet<String>>> {
        self.variant_narrator_map.as_ref()
    }

    /// Compute pre-single-strand ratio: fraction of upstream (teacher-side) hops
    /// that are single-strand (teacher has exactly 1 student).
    pub fn compute_pre_single_strand(&self, nid: &str) -> f64 {
        let mut total_hops = 0usize;
        let mut single_strand = 0usize;

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
    pub fn compute_bypass_ratio(&mut self, nid: &str) -> f64 {
        let node = match self.nodes.get(nid) {
            Some(n) => n,
            None => return 0.0,
        };

        let total = self.variant_ids.len();
        if total == 0 {
            return 0.0;
        }

        let missing: Vec<String> = self
            .variant_ids
            .iter()
            .filter(|v| !node.variants.contains(*v))
            .cloned()
            .collect();

        let ancestors = self.reachable_set_cached(nid, Direction::Teachers);
        let descendants = self.reachable_set_cached(nid, Direction::Students);

        let variant_narrators = self.variant_narrator_map.as_ref().unwrap();

        let mut bypass_count = 0;
        for variant in &missing {
            if let Some(narrators) = variant_narrators.get(variant) {
                let has_ancestor = narrators.iter().any(|n| ancestors.contains(n));
                let has_descendant = narrators.iter().any(|n| descendants.contains(n));
                if has_ancestor && has_descendant {
                    bypass_count += 1;
                }
            }
        }

        bypass_count as f64 / total as f64
    }

    /// Compute chronology conflict ratio for a narrator's incident edges.
    pub fn compute_chronology_ratio(&self, nid: &str) -> f64 {
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

    /// Compute provenance completeness: fraction of chain narrators with biographical data.
    pub fn compute_provenance(&mut self, nid: &str) -> f64 {
        if self.nodes.get(nid).is_none() {
            return 1.0;
        }

        let chain_narrators: HashSet<String> = {
            let mut all = self.reachable_set_cached(nid, Direction::Teachers);
            all.extend(self.reachable_set_cached(nid, Direction::Students));
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

    /// Get narrators at a specific tabaqah (generation layer).
    pub fn narrators_at_tabaqah(&self, tabaqah: i64) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.generation == Some(tabaqah))
            .map(|(nid, _)| nid.as_str())
            .collect()
    }

    /// Get all distinct tabaqat (generation layers) present in the graph.
    pub fn tabaqat(&self) -> Vec<i64> {
        let mut tabs: Vec<i64> = self
            .nodes
            .values()
            .filter_map(|n| n.generation)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        tabs.sort();
        tabs
    }

    /// Get root narrators (those with no teachers in the graph — typically Sahabah or source).
    pub fn root_narrators(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.direct_teachers.is_empty())
            .map(|(nid, _)| nid.as_str())
            .collect()
    }

    /// Get leaf narrators (those with no students — typically collectors).
    pub fn leaf_narrators(&self) -> Vec<&str> {
        self.nodes
            .iter()
            .filter(|(_, node)| node.direct_students.is_empty())
            .map(|(nid, _)| nid.as_str())
            .collect()
    }

    /// Get the ordered chain of narrators for a specific variant (hadith).
    /// Returns narrators ordered from collector (leaf) to source (root).
    /// Uses the variant_narrator_map + heard_from edges to reconstruct order.
    pub fn chain_for_variant(&self, variant_id: &str) -> Vec<String> {
        let variant_narrators = match self.variant_narrator_map.as_ref() {
            Some(map) => match map.get(variant_id) {
                Some(narrators) => narrators,
                None => return vec![],
            },
            None => return vec![],
        };

        if variant_narrators.is_empty() {
            return vec![];
        }

        // Find leaf narrators in this variant (no students within the variant)
        let leaves: Vec<&String> = variant_narrators
            .iter()
            .filter(|nid| {
                self.nodes
                    .get(*nid)
                    .map(|n| {
                        n.direct_students
                            .iter()
                            .all(|s| !variant_narrators.contains(s))
                    })
                    .unwrap_or(true)
            })
            .collect();

        // Walk from leaf toward root via teachers within this variant
        let start = match leaves.first() {
            Some(l) => (*l).clone(),
            None => return variant_narrators.iter().cloned().collect(),
        };

        let mut chain = vec![start.clone()];
        let mut current = start;
        let mut visited = HashSet::new();
        visited.insert(current.clone());

        loop {
            let teachers: Vec<String> = self
                .nodes
                .get(&current)
                .map(|n| {
                    n.direct_teachers
                        .iter()
                        .filter(|t| variant_narrators.contains(*t) && !visited.contains(*t))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default();

            if let Some(next) = teachers.first() {
                chain.push(next.clone());
                visited.insert(next.clone());
                current = next.clone();
            } else {
                break;
            }
        }

        chain
    }
}

// ── Graph building from database ──

/// Build a FamilyGraph from database data for a given family.
/// Returns None if the family has fewer than 2 hadiths.
pub async fn build_family_graph(db: &Surreal<Db>, family_id: &str) -> Result<Option<FamilyGraph>> {
    let family_rid = RecordId::new("hadith_family", family_id);

    // 1. Get all hadiths in this family
    let t = std::time::Instant::now();
    let mut res = db
        .query("SELECT id FROM hadith WHERE family_id = $fid")
        .bind(("fid", family_rid.clone()))
        .await?;

    #[derive(Debug, SurrealValue)]
    struct IdOnly {
        id: Option<RecordId>,
    }
    let hadith_ids: Vec<IdOnly> = res.take(0)?;
    eprintln!(
        "     [db] {family_id} q1 hadiths: {:?}, {} rows",
        t.elapsed(),
        hadith_ids.len()
    );

    if hadith_ids.len() < 2 {
        return Ok(None);
    }

    let hadith_keys: Vec<String> = hadith_ids
        .iter()
        .filter_map(|h| h.id.as_ref().map(crate::models::record_id_key_string))
        .collect();

    // 2. Fetch all heard_from edges for these hadiths
    let t = std::time::Instant::now();
    let hids: Vec<RecordId> = hadith_ids.iter().filter_map(|h| h.id.clone()).collect();
    let mut res = db
        .query(
            "SELECT in AS student, out AS teacher, hadith_ref \
             FROM heard_from WHERE hadith_ref IN $hids",
        )
        .bind(("hids", hids.clone()))
        .await?;
    let edges: Vec<HeardFromRow> = res.take(0)?;
    eprintln!(
        "     [db] {family_id} q2 heard_from: {:?}, {} rows",
        t.elapsed(),
        edges.len()
    );

    // 3. Fetch narrates edges
    let t = std::time::Instant::now();
    let mut res = db
        .query(
            "SELECT in AS narrator, out AS hadith \
             FROM narrates WHERE out IN $hids",
        )
        .bind(("hids", hids))
        .await?;
    let narrates: Vec<NarratesRow> = res.take(0)?;
    eprintln!(
        "     [db] {family_id} q3 narrates: {:?}, {} rows",
        t.elapsed(),
        narrates.len()
    );

    // 4. Fetch narrator bio data
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

    let t = std::time::Instant::now();
    let mut bio_map: HashMap<String, NarratorBio> = HashMap::new();
    let narrator_rids: Vec<RecordId> = narrator_ids
        .iter()
        .map(|nid| RecordId::new("narrator", nid.as_str()))
        .collect();
    let narrator_count = narrator_rids.len();
    eprintln!("     [db] {family_id} q4 narrator bios: {narrator_count} IDs to fetch");
    // Chunk large IN queries to avoid SurrealDB performance cliff
    let mut bios: Vec<NarratorBio> = Vec::new();
    for chunk in narrator_rids.chunks(200) {
        let mut res = db
            .query(
                "SELECT id, bio, generation, reliability_prior, reliability_rating \
                 FROM narrator WHERE id IN $ids",
            )
            .bind(("ids", chunk.to_vec()))
            .await?;
        let chunk_bios: Vec<NarratorBio> = res.take(0)?;
        bios.extend(chunk_bios);
    }
    eprintln!(
        "     [db] {family_id} q4 narrator bios: {:?}, {} rows",
        t.elapsed(),
        bios.len()
    );
    for bio in bios {
        if let Some(ref id) = bio.id {
            let key = crate::models::record_id_key_string(id);
            bio_map.insert(key, bio);
        }
    }

    // 5. Build the family graph
    let mut graph = FamilyGraph::new(
        HashMap::new(),
        HashMap::new(),
        hadith_keys.iter().cloned().collect(),
    );

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
                reliability_rating: bio.and_then(|b| b.reliability_rating.clone()),
            }
        });
        node.variants.insert(hkey);
    }

    // Add edges
    for e in &edges {
        let student_key = crate::models::record_id_key_string(&e.student);
        let teacher_key = crate::models::record_id_key_string(&e.teacher);
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
            chronology_conflict,
        });
    }

    Ok(Some(graph))
}
