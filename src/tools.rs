use std::collections::{HashSet, VecDeque};

use anyhow::Result;
use serde::Serialize;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::models::{Hadith, HadithSearchResult, Narrator, record_id_key_string, record_id_string};

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
        // Book-specific count via graph traversal
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
        (row.map(|r| r.count).unwrap_or(0), Some(book_name))
    } else {
        // Use pre-computed hadith_count
        (narrator.hadith_count.unwrap_or(0), None)
    };

    let mut context = format!("## Narrator Hadith Count\n\n");
    context.push_str(&format!("Narrator: {} ({})\n", name, narrator.name_en));
    if let Some(generation) = &narrator.generation {
        context.push_str(&format!("Generation (Tabaqah): {generation}\n"));
    }
    if let Some(book_label) = book_label {
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

#[derive(Debug, SurrealValue)]
struct HadithsResult {
    hadiths: Vec<Hadith>,
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

    let sql = format!(
        "SELECT ->narrates->hadith.{{{}}} AS hadiths FROM $nid",
        crate::models::HADITH_SEARCH_FIELDS
    );

    let mut res = db.query(&sql).bind(("nid", nid.clone())).await?;

    let result: Option<HadithsResult> = res.take(0).unwrap_or(None);
    let hadiths = result.map(|r| r.hadiths).unwrap_or_default();
    let total = hadiths.len();
    let sample: Vec<&Hadith> = hadiths.iter().take(limit).collect();

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

    let hadith_sources: Vec<HadithSearchResult> = sample
        .iter()
        .map(|h| HadithSearchResult {
            id: h.id.clone(),
            hadith_number: h.hadith_number,
            book_id: h.book_id,
            text_ar: h.text_ar.clone(),
            text_en: h.text_en.clone(),
            narrator_text: h.narrator_text.clone(),
            score: None,
        })
        .collect();

    let source = narrator_to_source(narrator, vec![], vec![]);
    Ok(ToolOutput {
        context,
        narrator_sources: vec![source],
        hadith_sources,
    })
}

/// Find a transmission path between two narrators via BFS over heard_from edges.
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

    // BFS from narrator1, expanding both directions of heard_from
    let max_depth = 6;
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<(RecordId, Vec<(String, String)>)> = VecDeque::new();
    // (current_node, path_of (id_str, name) pairs)

    visited.insert(id1_str.clone());
    queue.push_back((id1.clone(), vec![(id1_str.clone(), name1.to_string())]));

    let mut found_path: Option<Vec<(String, String)>> = None;

    #[derive(Debug, SurrealValue)]
    struct Neighbor {
        id: Option<RecordId>,
        name_ar: Option<String>,
        name_en: String,
    }

    while let Some((current, path)) = queue.pop_front() {
        if path.len() > max_depth {
            break;
        }

        // Expand both directions: teachers and students
        let mut res = db
            .query(
                "SELECT array::distinct(array::flatten([\
                    ->heard_from->narrator.{id, name_ar, name_en}, \
                    <-heard_from<-narrator.{id, name_ar, name_en}\
                ])) AS neighbors FROM $nid",
            )
            .bind(("nid", current))
            .await?;

        #[derive(Debug, SurrealValue)]
        struct NeighborsResult {
            neighbors: Vec<Neighbor>,
        }

        let result: Option<NeighborsResult> = res.take(0).unwrap_or(None);
        let neighbors = result.map(|r| r.neighbors).unwrap_or_default();

        for n in neighbors {
            let Some(ref nid) = n.id else { continue };
            let nid_str = record_id_string(nid);
            if visited.contains(&nid_str) {
                continue;
            }
            visited.insert(nid_str.clone());

            let nname = n.name_ar.as_deref().unwrap_or(&n.name_en).to_string();
            let mut new_path = path.clone();
            new_path.push((nid_str.clone(), nname));

            if nid_str == id2_str {
                found_path = Some(new_path);
                break;
            }

            queue.push_back((nid.clone(), new_path));
        }

        if found_path.is_some() {
            break;
        }
    }

    let mut context = format!("## Transmission Chain: {} ↔ {}\n\n", name1, name2);

    if let Some(ref path) = found_path {
        context.push_str(&format!("Found a path of {} steps:\n\n", path.len() - 1));
        let names: Vec<&str> = path.iter().map(|(_, name)| name.as_str()).collect();
        context.push_str(&format!("{}\n", names.join(" → ")));
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
