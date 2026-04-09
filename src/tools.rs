use std::collections::HashSet;

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::models::{HadithSearchResult, Narrator, record_id_key_string, record_id_string};

/// Output from a structured tool execution, ready for both SSE and LLM context.
pub struct ToolOutput {
    /// Formatted text for the LLM system prompt.
    pub context: String,
    /// Narrator sources for the SSE sources event.
    pub narrator_sources: Vec<ApiNarratorSource>,
    /// Hadith sources for the SSE sources event.
    pub hadith_sources: Vec<HadithSearchResult>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ApiNarratorSource {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
    pub hadith_count: Option<i64>,
    pub reliability_rating: Option<String>,
    pub ibn_hajar_rank: Option<String>,
    pub kunya: Option<String>,
    pub bio: Option<String>,
    pub death_year: Option<i64>,
    pub teachers: Vec<NarratorBrief>,
    pub students: Vec<NarratorBrief>,
}

#[derive(Debug, Serialize, Clone)]
pub struct NarratorBrief {
    pub id: String,
    pub name_ar: Option<String>,
    pub name_en: String,
    pub generation: Option<String>,
}

impl From<Narrator> for NarratorBrief {
    fn from(n: Narrator) -> Self {
        Self {
            id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
            name_ar: n.name_ar,
            name_en: n.name_en,
            generation: n.generation,
        }
    }
}

// ── Narrator Resolution ──

/// Fuzzy-resolve a narrator name to the best-matching narrator record.
pub async fn resolve_narrator(db: &Surreal<Db>, name: &str) -> Result<Option<Narrator>> {
    let lower = name.to_lowercase();
    let slug = crate::quran::ingest::strip_arabic_diacritics(name);

    // Multi-signal search: name_en, name_ar, kunya, aliases, search_name slug
    let sql = "\
        SELECT * FROM narrator WHERE \
            string::lowercase(name_en) CONTAINS string::lowercase($q) \
            OR name_ar CONTAINS $q \
            OR kunya CONTAINS $q \
            OR search_name CONTAINS $slug \
            OR $q INSIDE aliases \
        ORDER BY hadith_count DESC \
        LIMIT 5";

    let mut res = db
        .query(sql)
        .bind(("q", lower))
        .bind(("slug", slug))
        .await?;

    let matches: Vec<Narrator> = res.take(0).unwrap_or_default();
    Ok(matches.into_iter().next())
}

// ── Tool Functions ──

/// Count hadiths narrated by a narrator, optionally filtered by book name.
pub async fn count_hadiths(
    db: &Surreal<Db>,
    narrator: &Narrator,
    book: Option<&str>,
) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let (count, book_label) = if let Some(book_name) = book {
        // Resolve book name → book_id first, then use indexed int comparison.
        // hadith.book_id has hadith_book index; narrates.in has narrates_in_idx.
        #[derive(Debug, SurrealValue)]
        struct BookRow {
            book_number: i64,
            name_en: String,
        }
        let mut book_res = db
            .query(
                "SELECT book_number, name_en FROM book \
                 WHERE string::lowercase(name_en) CONTAINS string::lowercase($name) \
                 LIMIT 1",
            )
            .bind(("name", book_name.to_string()))
            .await?;
        let resolved_book: Option<BookRow> = book_res.take(0).unwrap_or(None);

        if let Some(bk) = resolved_book {
            #[derive(Debug, SurrealValue)]
            struct CountRow {
                count: i64,
            }
            // Use book_id (int) comparison — out.book_id uses hadith_book index
            let sql = "SELECT count() AS count FROM narrates \
                       WHERE in = $nid AND out.book_id = $book_id \
                       GROUP ALL";
            let mut res = db
                .query(sql)
                .bind(("nid", nid.clone()))
                .bind(("book_id", bk.book_number))
                .await?;
            let row: Option<CountRow> = res.take(0).unwrap_or(None);
            (row.map(|r| r.count).unwrap_or(0), Some(bk.name_en))
        } else {
            // Book not found — fall back to string match on book_name
            #[derive(Debug, SurrealValue)]
            struct CountRow {
                count: i64,
            }
            let sql = "SELECT count() AS count FROM narrates \
                       WHERE in = $nid AND out.book_name CONTAINS $book \
                       GROUP ALL";
            let mut res = db
                .query(sql)
                .bind(("nid", nid.clone()))
                .bind(("book", book_name.to_string()))
                .await?;
            let row: Option<CountRow> = res.take(0).unwrap_or(None);
            (
                row.map(|r| r.count).unwrap_or(0),
                Some(book_name.to_string()),
            )
        }
    } else {
        // Use pre-computed hadith_count
        (narrator.hadith_count.unwrap_or(0), None)
    };

    let mut context = format!("## Narrator Hadith Count\n\n");
    context.push_str(&format!("Narrator: {} ({})\n", name, narrator.name_en));
    if let Some(generation) = &narrator.generation {
        context.push_str(&format!("Generation (Tabaqah): {generation}\n"));
    }
    if let Some(ref book_label) = book_label {
        context.push_str(&format!("Hadiths narrated in {book_label}: {count}\n"));
    } else {
        context.push_str(&format!("Total hadiths narrated: {count}\n"));
    }

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get full narrator bio + reliability info.
pub async fn narrator_info(db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    // Fetch evidence records for reliability assessments
    #[derive(Debug, SurrealValue)]
    struct Evidence {
        rating: Option<String>,
        scholar: Option<String>,
        work: Option<String>,
    }

    let mut res = db
        .query("SELECT rating, scholar, work FROM evidence WHERE narrator = $nid")
        .bind(("nid", nid.clone()))
        .await?;
    let evidence: Vec<Evidence> = res.take(0).unwrap_or_default();

    let mut context = format!("## Narrator Information\n\n");
    context.push_str(&format!(
        "Name (Arabic): {}\n",
        narrator.name_ar.as_deref().unwrap_or("N/A")
    ));
    context.push_str(&format!("Name (English): {}\n", narrator.name_en));
    if let Some(kunya) = &narrator.kunya {
        context.push_str(&format!("Kunya: {kunya}\n"));
    }
    if let Some(generation) = &narrator.generation {
        context.push_str(&format!("Generation (Tabaqah): {generation}\n"));
    }
    if let Some(death) = narrator.death_year {
        context.push_str(&format!("Death year: {death} AH\n"));
    }
    if let Some(bio) = &narrator.bio {
        context.push_str(&format!("Biography: {bio}\n"));
    }
    if let Some(rating) = &narrator.reliability_rating {
        context.push_str(&format!("Reliability rating: {rating}\n"));
    }
    if let Some(rank) = &narrator.ibn_hajar_rank {
        context.push_str(&format!("Ibn Hajar rank: {rank}\n"));
    }
    if let Some(count) = narrator.hadith_count {
        context.push_str(&format!("Total hadiths narrated: {count}\n"));
    }
    if let Some(aliases) = &narrator.aliases {
        if !aliases.is_empty() {
            context.push_str(&format!("Also known as: {}\n", aliases.join(", ")));
        }
    }
    if !evidence.is_empty() {
        context.push_str("\nScholarly assessments:\n");
        for e in &evidence {
            let scholar = e.scholar.as_deref().unwrap_or("Unknown");
            let rating = e.rating.as_deref().unwrap_or("N/A");
            let work = e.work.as_deref().unwrap_or("");
            if work.is_empty() {
                context.push_str(&format!("- {scholar}: {rating}\n"));
            } else {
                context.push_str(&format!("- {scholar} ({work}): {rating}\n"));
            }
        }
    }

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

// ── Helper types for graph queries ──

#[derive(Debug, SurrealValue)]
struct TeachersResult {
    teachers: Vec<Narrator>,
}

#[derive(Debug, SurrealValue)]
struct StudentsResult {
    students: Vec<Narrator>,
}

/// Get narrator's teachers (who they heard from).
pub async fn narrator_teachers(db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let mut res = db
        .query(
            "SELECT array::distinct(array::filter(->heard_from->narrator.*, |$v| $v IS NOT NONE)) AS teachers FROM $nid",
        )
        .bind(("nid", nid.clone()))
        .await?;

    let result: Option<TeachersResult> = res.take(0).unwrap_or(None);
    let teachers = result.map(|r| r.teachers).unwrap_or_default();

    let mut context = format!("## Teachers of {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "{name} had {} known teacher(s):\n\n",
        teachers.len()
    ));
    for t in &teachers {
        let tname = t.name_ar.as_deref().unwrap_or(&t.name_en);
        context.push_str(&format!("- {} ({})", tname, t.name_en));
        if let Some(generation) = &t.generation {
            context.push_str(&format!(", generation {generation}"));
        }
        context.push('\n');
    }

    let teacher_briefs: Vec<NarratorBrief> =
        teachers.iter().cloned().map(NarratorBrief::from).collect();
    let source = narrator_to_source(narrator, teacher_briefs, vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get narrator's students (who heard from them).
pub async fn narrator_students(db: &Surreal<Db>, narrator: &Narrator) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);

    let mut res = db
        .query(
            "SELECT array::distinct(array::filter(<-heard_from<-narrator.*, |$v| $v IS NOT NONE)) AS students FROM $nid",
        )
        .bind(("nid", nid.clone()))
        .await?;

    let result: Option<StudentsResult> = res.take(0).unwrap_or(None);
    let students = result.map(|r| r.students).unwrap_or_default();

    let mut context = format!("## Students of {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "{name} had {} known student(s):\n\n",
        students.len()
    ));
    for s in &students {
        let sname = s.name_ar.as_deref().unwrap_or(&s.name_en);
        context.push_str(&format!("- {} ({})", sname, s.name_en));
        if let Some(generation) = &s.generation {
            context.push_str(&format!(", generation {generation}"));
        }
        context.push('\n');
    }

    let student_briefs: Vec<NarratorBrief> =
        students.iter().cloned().map(NarratorBrief::from).collect();
    let source = narrator_to_source(narrator, vec![], student_briefs);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: vec![],
    })
}

/// Get sample hadiths narrated by a narrator.
pub async fn narrator_hadiths(
    db: &Surreal<Db>,
    narrator: &Narrator,
    limit: usize,
) -> Result<ToolOutput> {
    let nid = narrator.id.as_ref().unwrap();
    let name = narrator.name_ar.as_deref().unwrap_or(&narrator.name_en);
    let total = narrator.hadith_count.unwrap_or(0);

    // Query narrates edges directly with LIMIT — avoids fetching all hadiths into memory.
    // Uses narrates_in_idx on `in`.
    let sql = format!(
        "SELECT out.id AS id, out.hadith_number AS hadith_number, out.book_id AS book_id, \
         out.text_ar AS text_ar, out.text_en AS text_en, out.narrator_text AS narrator_text \
         FROM narrates WHERE in = $nid LIMIT {limit}"
    );

    let mut res = db.query(&sql).bind(("nid", nid.clone())).await?;
    let sample: Vec<HadithSearchResult> = res.take(0).unwrap_or_default();

    let mut context = format!("## Hadiths narrated by {name} ({})\n\n", narrator.name_en);
    context.push_str(&format!(
        "Showing {} of {} total hadiths:\n\n",
        sample.len(),
        total
    ));
    for h in &sample {
        context.push_str(&format!("Hadith #{}\n", h.hadith_number));
        if let Some(text) = h.text_en.as_deref().or(h.text_ar.as_deref()) {
            let truncated = if text.len() > 300 {
                &text[..text.floor_char_boundary(300)]
            } else {
                text
            };
            context.push_str(&format!("{truncated}\n\n"));
        }
    }

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources: sample,
    })
}

/// Find a transmission path between two narrators via batched BFS over heard_from edges.
/// Uses 2 queries per depth level (one for each edge direction) instead of 1 per node.
pub async fn chain_between(
    db: &Surreal<Db>,
    narrator1: &Narrator,
    narrator2: &Narrator,
) -> Result<ToolOutput> {
    let id1 = narrator1.id.as_ref().unwrap();
    let id2 = narrator2.id.as_ref().unwrap();
    let name1 = narrator1.name_ar.as_deref().unwrap_or(&narrator1.name_en);
    let name2 = narrator2.name_ar.as_deref().unwrap_or(&narrator2.name_en);

    let id1_str = record_id_string(id1);
    let id2_str = record_id_string(id2);

    // Batched BFS: query ALL frontier nodes at once per depth level.
    // 2 queries per level (outward + inward edges) instead of 1 query per node.
    let max_depth = 6;
    let mut visited: HashSet<String> = HashSet::new();
    // Map from node id_str -> (parent_id_str, node_name) for path reconstruction
    let mut parent: std::collections::HashMap<String, (String, String)> =
        std::collections::HashMap::new();

    visited.insert(id1_str.clone());
    parent.insert(id1_str.clone(), ("".to_string(), name1.to_string()));

    let mut frontier: Vec<RecordId> = vec![id1.clone()];
    let mut found = false;

    // Edge row: one side is a frontier node, the other is the neighbor
    #[derive(Debug, SurrealValue)]
    struct EdgeRow {
        from_id: RecordId,
        to_id: RecordId,
        to_name_ar: Option<String>,
        to_name_en: String,
    }

    for _depth in 0..max_depth {
        if frontier.is_empty() {
            break;
        }

        // Batch query: outward edges (frontier -> heard_from -> narrator)
        // Uses heard_from_out_idx on `out` (frontier nodes are `out` in heard_from)
        // heard_from: in=student, out=teacher. "out heard_from in" = "out's teacher is in"
        // Wait — re-reading schema: RELATION FROM narrator TO narrator
        // RELATE $from->heard_from->$to means $from heard from $to
        // So `out` = the person being heard from (teacher), `in` = the one who heard (student)
        // ->heard_from-> traverses outward: gets teachers
        // <-heard_from<- traverses inward: gets students
        //
        // For edges where frontier nodes are the `in` side (student): teachers
        let mut res = db
            .query(
                "SELECT in AS from_id, out AS to_id, out.name_ar AS to_name_ar, out.name_en AS to_name_en \
                 FROM heard_from WHERE in IN $frontier",
            )
            .bind(("frontier", frontier.clone()))
            .await?;
        let outward: Vec<EdgeRow> = res.take(0).unwrap_or_default();

        // For edges where frontier nodes are the `out` side (teacher): students
        let mut res2 = db
            .query(
                "SELECT out AS from_id, in AS to_id, in.name_ar AS to_name_ar, in.name_en AS to_name_en \
                 FROM heard_from WHERE out IN $frontier",
            )
            .bind(("frontier", frontier.clone()))
            .await?;
        let inward: Vec<EdgeRow> = res2.take(0).unwrap_or_default();

        let mut next_frontier: Vec<RecordId> = Vec::new();

        for edge in outward.iter().chain(inward.iter()) {
            let from_str = record_id_string(&edge.from_id);
            let to_str = record_id_string(&edge.to_id);

            if visited.contains(&to_str) {
                continue;
            }
            visited.insert(to_str.clone());

            let to_name = edge
                .to_name_ar
                .as_deref()
                .unwrap_or(&edge.to_name_en)
                .to_string();
            parent.insert(to_str.clone(), (from_str, to_name));

            if to_str == id2_str {
                found = true;
                break;
            }
            next_frontier.push(edge.to_id.clone());
        }

        if found {
            break;
        }
        frontier = next_frontier;
    }

    // Reconstruct path from parent map
    let found_path = if found {
        let mut path = Vec::new();
        let mut current = id2_str.clone();
        while !current.is_empty() {
            if let Some((prev, name)) = parent.get(&current) {
                path.push(name.clone());
                current = prev.clone();
            } else {
                break;
            }
        }
        path.reverse();
        Some(path)
    } else {
        None
    };

    let mut context = format!("## Transmission Chain: {} ↔ {}\n\n", name1, name2);

    if let Some(ref path) = found_path {
        context.push_str(&format!("Found a path of {} steps:\n\n", path.len() - 1));
        context.push_str(&format!("{}\n", path.join(" → ")));
    } else {
        context.push_str(&format!(
            "No transmission path found between {name1} and {name2} within {max_depth} steps.\n"
        ));
    }

    let src1 = narrator_to_source(narrator1, vec![], vec![]);
    let src2 = narrator_to_source(narrator2, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![src1, src2],
        hadith_sources: vec![],
    })
}

// ── Helpers ──

fn narrator_to_source(
    n: &Narrator,
    teachers: Vec<NarratorBrief>,
    students: Vec<NarratorBrief>,
) -> ApiNarratorSource {
    ApiNarratorSource {
        id: n.id.as_ref().map(record_id_key_string).unwrap_or_default(),
        name_ar: n.name_ar.clone(),
        name_en: n.name_en.clone(),
        generation: n.generation.clone(),
        hadith_count: n.hadith_count,
        reliability_rating: n.reliability_rating.clone(),
        ibn_hajar_rank: n.ibn_hajar_rank.clone(),
        kunya: n.kunya.clone(),
        bio: n.bio.clone(),
        death_year: n.death_year,
        teachers,
        students,
    }
}
