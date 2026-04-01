use std::collections::{HashMap, HashSet};

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::embed::Embedder;

/// Similarity threshold for clustering hadiths into families.
const COSINE_THRESHOLD: f64 = 0.85;

/// Minimum shared narrators to confirm a family match.
const MIN_SHARED_NARRATORS: usize = 1;

// ── Internal types ──

#[derive(Debug, SurrealValue)]
struct HadithForClustering {
    id: Option<RecordId>,
    hadith_number: i64,
    book_name: Option<String>,
    narrator_text: Option<String>,
    embedding: Option<Vec<f64>>,
}

#[derive(Debug, SurrealValue)]
struct NarratorsOfHadith {
    narrators: Vec<RecordId>,
}

// ── Union-Find ──

struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}

/// Cluster hadiths into families based on embedding similarity + narrator overlap.
///
/// Algorithm:
/// 1. Fetch all hadiths with embeddings
/// 2. For each hadith, find k-nearest neighbors via HNSW
/// 3. Filter by cosine >= COSINE_THRESHOLD
/// 4. Confirm via shared narrators (>= MIN_SHARED_NARRATORS)
/// 5. Union-Find to merge into families
/// 6. Create hadith_family records, update hadith.family_id
pub async fn compute_families(db: &Surreal<Db>, _embedder: &Embedder) -> Result<usize> {
    tracing::info!("Computing hadith families...");

    // 1. Fetch all hadiths with embeddings
    let mut res = db
        .query(
            "SELECT id, hadith_number, book_name, narrator_text, embedding \
             FROM hadith WHERE embedding IS NOT NONE",
        )
        .await?;
    let hadiths: Vec<HadithForClustering> = res.take(0)?;
    let count = hadiths.len();
    tracing::info!("Found {count} hadiths with embeddings");

    if count == 0 {
        return Ok(0);
    }

    // Build index: hadith_key -> index
    let mut key_to_idx: HashMap<String, usize> = HashMap::new();
    for (i, h) in hadiths.iter().enumerate() {
        if let Some(ref id) = h.id {
            let key = crate::models::record_id_key_string(id);
            key_to_idx.insert(key, i);
        }
    }

    // 2. Build narrator sets per hadith (for overlap check)
    tracing::info!("Fetching narrator sets...");
    let mut narrator_sets: Vec<HashSet<String>> = vec![HashSet::new(); count];
    for (i, h) in hadiths.iter().enumerate() {
        if let Some(ref id) = h.id {
            let mut nr = db
                .query("SELECT <-narrates<-narrator.id AS narrators FROM $rid")
                .bind(("rid", id.clone()))
                .await?;
            let result: Option<NarratorsOfHadith> = nr.take(0)?;
            if let Some(r) = result {
                for nid in &r.narrators {
                    narrator_sets[i].insert(crate::models::record_id_key_string(nid));
                }
            }
        }
    }

    // 3. For each hadith, find neighbors via embedding similarity
    tracing::info!("Computing pairwise similarities...");
    let mut uf = UnionFind::new(count);
    let mut pairs_found = 0u64;

    for (i, h) in hadiths.iter().enumerate() {
        let embedding = match &h.embedding {
            Some(e) if !e.is_empty() => e,
            _ => continue,
        };

        // Convert to f32 for the HNSW query
        let query_vec: Vec<f32> = embedding.iter().map(|&v| v as f32).collect();

        let sql = format!(
            "SELECT id, vector::similarity::cosine(embedding, $qv) AS score \
             FROM hadith WHERE embedding <|10,40|> $qv ORDER BY score DESC"
        );
        let mut nr = db.query(&sql).bind(("qv", query_vec)).await?;

        #[derive(Debug, SurrealValue)]
        struct SimResult {
            id: Option<RecordId>,
            score: Option<f64>,
        }

        let neighbors: Vec<SimResult> = nr.take(0)?;

        for neighbor in &neighbors {
            let score = neighbor.score.unwrap_or(0.0);
            if score < COSINE_THRESHOLD {
                continue;
            }

            let neighbor_key = match &neighbor.id {
                Some(id) => crate::models::record_id_key_string(id),
                None => continue,
            };

            let j = match key_to_idx.get(&neighbor_key) {
                Some(&idx) => idx,
                None => continue,
            };

            if i == j {
                continue;
            }

            // Check narrator overlap
            let shared = narrator_sets[i].intersection(&narrator_sets[j]).count();
            if shared >= MIN_SHARED_NARRATORS {
                uf.union(i, j);
                pairs_found += 1;
            }
        }

        if (i + 1) % 500 == 0 {
            tracing::info!("Processed {}/{count} hadiths...", i + 1);
        }
    }

    tracing::info!("Found {pairs_found} similar pairs");

    // 4. Group by root and create families
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..count {
        let root = uf.find(i);
        groups.entry(root).or_default().push(i);
    }

    // Only create families for groups with 2+ hadiths
    let families: Vec<Vec<usize>> = groups.into_values().filter(|g| g.len() >= 2).collect();

    tracing::info!(
        "Creating {} families ({} hadiths in families, {} singletons)",
        families.len(),
        families.iter().map(|f| f.len()).sum::<usize>(),
        count - families.iter().map(|f| f.len()).sum::<usize>(),
    );

    // 5. Create family records and update hadiths
    for (fi, members) in families.iter().enumerate() {
        // Use first member's narrator_text as label
        let label = hadiths[members[0]]
            .narrator_text
            .clone()
            .unwrap_or_else(|| format!("Family {}", fi + 1));

        let family_slug = format!("family_{fi}");

        db.query("CREATE $rid CONTENT { family_label: $label, variant_count: $count }")
            .bind(("rid", RecordId::new("hadith_family", family_slug.as_str())))
            .bind(("label", label))
            .bind(("count", members.len() as i64))
            .await?;

        // Update each hadith's family_id
        for &mi in members {
            if let Some(ref hid) = hadiths[mi].id {
                db.query("UPDATE $rid SET family_id = $fid")
                    .bind(("rid", hid.clone()))
                    .bind(("fid", RecordId::new("hadith_family", family_slug.as_str())))
                    .await?;
            }
        }
    }

    tracing::info!(
        "Family computation complete: {} families created",
        families.len()
    );
    Ok(families.len())
}
