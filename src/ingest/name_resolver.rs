//! Deterministic narrator name resolution.
//!
//! Loads ar_sanad_narrators.csv and builds a multi-strategy lookup index that
//! resolves any Arabic name form (with diacritics, grammatical inflection,
//! abbreviated kunyas, etc.) to a canonical narrator ID.
//!
//! Resolution strategies (tried in order):
//! 1. Exact match on normalized primary name
//! 2. Exact match on normalized namings/aliases
//! 3. Exact match on normalized kunia
//! 4. Chain-context disambiguation for ambiguous matches (using narrated_from/narrated_to)

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::Result;

use super::sanadset::normalize_arabic;

/// A single narrator record from ar_sanad_narrators.csv.
#[derive(Debug, Clone)]
pub struct NarratorRecord {
    pub id: u32,
    pub name: String,
    pub namings: Vec<String>,
    pub kunia: Option<String>,
    pub ibnhajar_rank: Option<String>,
    pub shuhra: Option<String>,
    pub laqab: Option<String>,
    pub death_year: Option<String>,
    pub birth_year: Option<String>,
    pub living_city: Option<String>,
    pub tabaqa: Option<String>,
    /// IDs of narrators this person narrated FROM (teachers)
    pub narrated_from: Vec<u32>,
    /// IDs of narrators this person narrated TO (students)
    pub narrated_to: Vec<u32>,
}

/// Result of resolving a name.
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Uniquely resolved to a single narrator.
    Resolved(u32),
    /// Multiple candidates — needs chain context to disambiguate.
    Ambiguous(Vec<u32>),
    /// No match found.
    Unresolved,
}

/// Pre-built index for fast narrator name resolution.
pub struct NameResolver {
    /// id → full narrator record
    pub narrators: HashMap<u32, NarratorRecord>,
    /// normalized form → set of narrator IDs (from primary name)
    name_index: HashMap<String, HashSet<u32>>,
    /// normalized form → set of narrator IDs (from namings/aliases)
    naming_index: HashMap<String, HashSet<u32>>,
    /// normalized form → set of narrator IDs (from kunia)
    kunia_index: HashMap<String, HashSet<u32>>,
    /// Unified index: any normalized form → set of narrator IDs
    unified_index: HashMap<String, HashSet<u32>>,
}

impl NameResolver {
    /// Load from ar_sanad_narrators.csv and build all indices.
    pub fn load(csv_path: &str) -> Result<Self> {
        let path = Path::new(csv_path);
        if !path.exists() {
            anyhow::bail!("Narrator CSV not found: {csv_path}");
        }

        let mut narrators = HashMap::new();
        let mut name_index: HashMap<String, HashSet<u32>> = HashMap::new();
        let mut naming_index: HashMap<String, HashSet<u32>> = HashMap::new();
        let mut kunia_index: HashMap<String, HashSet<u32>> = HashMap::new();
        let mut unified_index: HashMap<String, HashSet<u32>> = HashMap::new();

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .from_path(path)?;

        for result in reader.records() {
            let record = match result {
                Ok(r) => r,
                Err(_) => continue,
            };

            let get = |idx: usize| -> String { record.get(idx).unwrap_or("").trim().to_string() };
            let get_opt = |idx: usize| -> Option<String> {
                let v = get(idx);
                if v.is_empty() || v == "-" {
                    None
                } else {
                    Some(v)
                }
            };

            let id: u32 = match get(16).parse() {
                Ok(v) => v,
                Err(_) => continue,
            };

            let name = get(0);
            let namings = parse_python_list(&get(1));
            let kunia = get_opt(8);
            let narrated_from = parse_id_list(&get(17));
            let narrated_to = parse_id_list(&get(18));

            let rec = NarratorRecord {
                id,
                name: name.clone(),
                namings: namings.clone(),
                kunia: kunia.clone(),
                ibnhajar_rank: get_opt(2),
                shuhra: get_opt(3),
                laqab: get_opt(4),
                death_year: get_opt(9),
                birth_year: get_opt(13),
                living_city: get_opt(12),
                tabaqa: get_opt(10),
                narrated_from,
                narrated_to,
            };

            // Index primary name
            let norm_name = normalize_arabic(&name);
            if !norm_name.is_empty() {
                name_index.entry(norm_name.clone()).or_default().insert(id);
                unified_index.entry(norm_name).or_default().insert(id);
            }

            // Index all namings/aliases
            for alias in &rec.namings {
                let norm = normalize_arabic(alias);
                if !norm.is_empty() {
                    naming_index.entry(norm.clone()).or_default().insert(id);
                    unified_index.entry(norm).or_default().insert(id);
                }
            }

            // Index kunia
            if let Some(ref k) = rec.kunia {
                let norm = normalize_arabic(k);
                if !norm.is_empty() {
                    kunia_index.entry(norm.clone()).or_default().insert(id);
                    unified_index.entry(norm).or_default().insert(id);
                }
            }

            narrators.insert(id, rec);
        }

        tracing::info!(
            "NameResolver loaded: {} narrators, {} name forms, {} naming forms, {} kunia forms, {} unified forms",
            narrators.len(),
            name_index.len(),
            naming_index.len(),
            kunia_index.len(),
            unified_index.len(),
        );

        Ok(Self {
            narrators,
            name_index,
            naming_index,
            kunia_index,
            unified_index,
        })
    }

    /// Resolve a single name (already normalized) without chain context.
    /// Returns Resolved if unique, Ambiguous if multiple, Unresolved if none.
    pub fn resolve_name(&self, normalized: &str) -> Resolution {
        if normalized.is_empty() {
            return Resolution::Unresolved;
        }

        // 1. Exact match on primary name
        if let Some(ids) = self.name_index.get(normalized) {
            if ids.len() == 1 {
                return Resolution::Resolved(*ids.iter().next().unwrap());
            }
        }

        // 2. Exact match on namings/aliases
        if let Some(ids) = self.naming_index.get(normalized) {
            if ids.len() == 1 {
                return Resolution::Resolved(*ids.iter().next().unwrap());
            }
        }

        // 3. Exact match on kunia
        if let Some(ids) = self.kunia_index.get(normalized) {
            if ids.len() == 1 {
                return Resolution::Resolved(*ids.iter().next().unwrap());
            }
        }

        // 4. Check unified (any match) — might be ambiguous
        if let Some(ids) = self.unified_index.get(normalized) {
            if ids.len() == 1 {
                return Resolution::Resolved(*ids.iter().next().unwrap());
            }
            return Resolution::Ambiguous(ids.iter().copied().collect());
        }

        Resolution::Unresolved
    }

    /// Resolve a name using chain context for disambiguation.
    ///
    /// `neighbors` contains the resolved IDs of adjacent narrators in the isnad chain.
    /// For a name at position i in [A, B, C], neighbors = resolved IDs of A (student) and C (teacher).
    /// We disambiguate by checking narrated_from/narrated_to relationships.
    pub fn resolve_with_context(
        &self,
        normalized: &str,
        prev_id: Option<u32>,
        next_id: Option<u32>,
    ) -> Resolution {
        let base = self.resolve_name(normalized);

        match base {
            Resolution::Ambiguous(ref candidates) => {
                let filtered = self.disambiguate(candidates, prev_id, next_id);
                if filtered.len() == 1 {
                    Resolution::Resolved(filtered[0])
                } else if filtered.is_empty() {
                    // Context didn't help — return original ambiguous set
                    base
                } else {
                    Resolution::Ambiguous(filtered)
                }
            }
            Resolution::Unresolved => {
                // Try partial/substring matching as last resort
                self.resolve_partial(normalized, prev_id, next_id)
            }
            resolved => resolved,
        }
    }

    /// Disambiguate candidates using narrated_from/narrated_to relationships.
    fn disambiguate(
        &self,
        candidates: &[u32],
        prev_id: Option<u32>,
        next_id: Option<u32>,
    ) -> Vec<u32> {
        if prev_id.is_none() && next_id.is_none() {
            return candidates.to_vec();
        }

        let mut scored: Vec<(u32, u32)> = candidates
            .iter()
            .map(|&cid| {
                let mut score = 0u32;
                if let Some(rec) = self.narrators.get(&cid) {
                    // prev_id is the narrator BEFORE this one in chain (student / receiver)
                    // In isnad: prev heard from this, so this person's narrated_to should contain prev
                    if let Some(prev) = prev_id {
                        if rec.narrated_to.contains(&prev) {
                            score += 2;
                        }
                    }
                    // next_id is the narrator AFTER this one in chain (teacher / source)
                    // This person heard from next, so narrated_from should contain next
                    if let Some(next) = next_id {
                        if rec.narrated_from.contains(&next) {
                            score += 2;
                        }
                    }
                }
                (cid, score)
            })
            .collect();

        // Keep only candidates with the highest score (> 0)
        let max_score = scored.iter().map(|(_, s)| *s).max().unwrap_or(0);
        if max_score == 0 {
            return candidates.to_vec();
        }
        scored
            .into_iter()
            .filter(|(_, s)| *s == max_score)
            .map(|(id, _)| id)
            .collect()
    }

    /// Partial/substring matching as last resort for completely unresolved names.
    /// Checks if the query is a prefix of or contained in any known form.
    fn resolve_partial(
        &self,
        normalized: &str,
        prev_id: Option<u32>,
        next_id: Option<u32>,
    ) -> Resolution {
        if normalized.len() < 4 {
            return Resolution::Unresolved;
        }

        let mut candidates: HashSet<u32> = HashSet::new();

        // Check if any known form starts with the query (short name matching long)
        // or if the query starts with any known form (long name matching short)
        for (form, ids) in &self.unified_index {
            if form.starts_with(normalized) || normalized.starts_with(form.as_str()) {
                candidates.extend(ids);
            }
        }

        if candidates.is_empty() {
            return Resolution::Unresolved;
        }

        if candidates.len() == 1 {
            return Resolution::Resolved(*candidates.iter().next().unwrap());
        }

        // Try disambiguation with chain context
        let cands: Vec<u32> = candidates.into_iter().collect();
        let filtered = self.disambiguate(&cands, prev_id, next_id);
        if filtered.len() == 1 {
            Resolution::Resolved(filtered[0])
        } else {
            Resolution::Ambiguous(filtered)
        }
    }

    /// Resolve an entire isnad chain with multi-pass disambiguation.
    ///
    /// Pass 1: Resolve all names without context.
    /// Pass 2: Use resolved neighbors to disambiguate remaining ambiguous names.
    /// Pass 3: Repeat pass 2 (newly resolved names may help remaining ambiguous ones).
    pub fn resolve_chain(&self, raw_names: &[String]) -> Vec<(String, Resolution)> {
        let normalized: Vec<String> = raw_names.iter().map(|n| normalize_arabic(n)).collect();

        // Pass 1: resolve without context
        let mut results: Vec<Resolution> =
            normalized.iter().map(|n| self.resolve_name(n)).collect();

        // Pass 2 & 3: context-based disambiguation (two rounds)
        for _ in 0..2 {
            let mut changed = false;
            for i in 0..results.len() {
                if matches!(results[i], Resolution::Resolved(_)) {
                    continue;
                }

                let prev_id = if i > 0 {
                    match &results[i - 1] {
                        Resolution::Resolved(id) => Some(*id),
                        _ => None,
                    }
                } else {
                    None
                };

                let next_id = if i + 1 < results.len() {
                    match &results[i + 1] {
                        Resolution::Resolved(id) => Some(*id),
                        _ => None,
                    }
                } else {
                    None
                };

                let new_result = self.resolve_with_context(&normalized[i], prev_id, next_id);
                if matches!(new_result, Resolution::Resolved(_))
                    && !matches!(results[i], Resolution::Resolved(_))
                {
                    results[i] = new_result;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }

        raw_names.iter().cloned().zip(results).collect()
    }

    /// Get the canonical display name for a resolved narrator ID.
    pub fn canonical_name(&self, id: u32) -> Option<&str> {
        self.narrators.get(&id).map(|r| r.name.as_str())
    }

    /// Get a short display name: prefer kunia or shuhra if available, else primary name.
    pub fn display_name(&self, id: u32) -> Option<&str> {
        self.narrators.get(&id).map(|r| {
            r.shuhra
                .as_deref()
                .or(r.kunia.as_deref())
                .unwrap_or(&r.name)
        })
    }
}

/// Parse a Python-style list string like "['a', 'b', 'c']" into Vec<String>.
fn parse_python_list(s: &str) -> Vec<String> {
    let trimmed = s.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .map(|item| {
            item.trim()
                .trim_matches('\'')
                .trim_matches('"')
                .trim()
                .to_string()
        })
        .filter(|item| !item.is_empty())
        .collect()
}

/// Parse an ID list like "[1, 2, 3]" into Vec<u32>.
fn parse_id_list(s: &str) -> Vec<u32> {
    let trimmed = s.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return vec![];
    }
    let inner = &trimmed[1..trimmed.len() - 1];
    inner
        .split(',')
        .filter_map(|item| item.trim().parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_kunya_in_names() {
        // أبي should become أبو when followed by another word
        assert_eq!(normalize_arabic("أَبِي هُرَيْرَةَ"), "ابو هريره");
        assert_eq!(normalize_arabic("أبى صالحٍ"), "ابو صالح");
        assert_eq!(normalize_arabic("أَبِي سَلَمَةَ"), "ابو سلمه");
        // أبو should stay أبو
        assert_eq!(normalize_arabic("أبو هريرة"), "ابو هريره");
        // Standalone ابي (relative reference) should NOT become ابو
        assert_eq!(normalize_arabic("ابي"), "ابي");
    }
}
