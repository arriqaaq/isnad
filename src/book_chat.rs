//! Book chat module — PageIndex-style agentic retrieval over Turath books.
//!
//! At startup, loads tree structures (built offline by PageIndex from markdown)
//! from disk. At query time:
//!   1. Two-phase navigation: pick chapter → pick section (~1K tokens each)
//!   2. Reads the text content from the tree nodes for those sections
//!   3. Sends the text + question to Ollama → streams the answer

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::rag::OllamaClient;

/// Truncate a string at a char boundary, not in the middle of a multi-byte character.
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

// ── Data structures ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BookTree {
    pub book_id: u64,
    pub name_en: String,
    pub name_ar: String,
    pub structure: serde_json::Value,
    pub line_count: usize,
    pub md_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionRange {
    pub start_line: u64,
    pub end_line: u64,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SectionContent {
    pub line: u64,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BookSource {
    pub line: u64,
    pub title: String,
}

// ── Navigation cache ────────────────────────────────────────────────────────

const CACHE_TTL_SECS: u64 = 600; // 10 minutes
const CACHE_MAX_ENTRIES: usize = 100;

pub struct NavCache {
    entries: Mutex<HashMap<(u64, String), (Instant, Vec<SectionRange>)>>,
}

impl NavCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn get(&self, book_id: u64, question: &str) -> Option<Vec<SectionRange>> {
        let entries = self.entries.lock().ok()?;
        let key = (book_id, question.to_string());
        if let Some((instant, ranges)) = entries.get(&key) {
            if instant.elapsed().as_secs() < CACHE_TTL_SECS {
                return Some(ranges.clone());
            }
        }
        None
    }

    pub fn put(&self, book_id: u64, question: &str, ranges: Vec<SectionRange>) {
        if let Ok(mut entries) = self.entries.lock() {
            // Evict expired entries if at capacity
            if entries.len() >= CACHE_MAX_ENTRIES {
                entries.retain(|_, (instant, _)| instant.elapsed().as_secs() < CACHE_TTL_SECS);
            }
            // If still at capacity, clear oldest half
            if entries.len() >= CACHE_MAX_ENTRIES {
                let mut by_age: Vec<_> = entries.keys().cloned().collect();
                by_age
                    .sort_by_key(|k| entries.get(k).map(|(i, _)| i.elapsed()).unwrap_or_default());
                // Remove oldest half
                for key in by_age.iter().rev().take(CACHE_MAX_ENTRIES / 2) {
                    entries.remove(key);
                }
            }
            entries.insert((book_id, question.to_string()), (Instant::now(), ranges));
        }
    }
}

// ── Book map JSON (written by scripts/index_books.py) ───────────────────────

#[derive(Debug, Deserialize)]
struct BookMapEntry {
    name_en: String,
    #[serde(default)]
    name_ar: String,
    #[serde(default)]
    line_count: usize,
    #[serde(default)]
    md_path: String,
}

// ── Loading ─────────────────────────────────────────────────────────────────

/// Load all book trees from the PageIndex workspace directory.
pub fn load_book_trees(workspace_dir: &Path) -> Result<HashMap<u64, BookTree>> {
    let book_map_path = workspace_dir.join("book_map.json");
    if !book_map_path.exists() {
        anyhow::bail!(
            "book_map.json not found in {}. Run: python3 scripts/index_books.py",
            workspace_dir.display()
        );
    }

    let raw = std::fs::read_to_string(&book_map_path)
        .with_context(|| format!("reading {}", book_map_path.display()))?;
    let book_map: HashMap<String, BookMapEntry> =
        serde_json::from_str(&raw).context("parsing book_map.json")?;

    let mut trees = HashMap::new();

    for (book_id_str, entry) in &book_map {
        let book_id: u64 = book_id_str
            .parse()
            .with_context(|| format!("invalid book_id in book_map.json: {book_id_str}"))?;

        let tree_path = workspace_dir.join(format!("{book_id_str}.json"));
        if !tree_path.exists() {
            tracing::warn!(
                "Tree file {}.json not found for book {}, skipping",
                book_id_str,
                entry.name_en
            );
            continue;
        }

        let tree_raw = std::fs::read_to_string(&tree_path)
            .with_context(|| format!("reading {}", tree_path.display()))?;
        let tree_doc: serde_json::Value = serde_json::from_str(&tree_raw)
            .with_context(|| format!("parsing {}", tree_path.display()))?;

        let structure = tree_doc
            .get("structure")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]));

        let line_count = tree_doc
            .get("line_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(entry.line_count as u64) as usize;

        let md_path = PathBuf::from(&entry.md_path);

        tracing::info!(
            "Loaded book {} ({}) — {} lines",
            book_id,
            entry.name_en,
            line_count,
        );

        trees.insert(
            book_id,
            BookTree {
                book_id,
                name_en: entry.name_en.clone(),
                name_ar: entry.name_ar.clone(),
                structure,
                line_count,
                md_path,
            },
        );
    }

    Ok(trees)
}

// ── Tree helpers ────────────────────────────────────────────────────────────

/// A chapter heading: its line number, title, and the sub-tree node.
struct ChapterInfo {
    line_num: u64,
    title: String,
    node: serde_json::Value,
}

/// Extract level-1 headings (direct children of the root node).
fn get_level1_chapters(structure: &serde_json::Value) -> Vec<ChapterInfo> {
    // The tree structure is: [root_node] where root_node has "nodes": [chapter1, chapter2, ...]
    let root = match structure.as_array().and_then(|arr| arr.first()) {
        Some(r) => r,
        None => return Vec::new(),
    };

    let children = match root.get("nodes").and_then(|v| v.as_array()) {
        Some(c) => c,
        None => return Vec::new(),
    };

    children
        .iter()
        .map(|node| ChapterInfo {
            line_num: node.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0),
            title: node
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            node: node.clone(),
        })
        .collect()
}

/// Extract summary from a node (prefix_summary for branches, summary for leaves).
fn node_summary(node: &serde_json::Value) -> Option<String> {
    node.get("prefix_summary")
        .or_else(|| node.get("summary"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| truncate_str(s, 250).to_string())
}

/// Format level-1 chapters as a compact TOC for the LLM, including summaries when available.
fn format_chapters_toc(chapters: &[ChapterInfo]) -> String {
    let mut out = String::new();
    for ch in chapters {
        out.push_str(&format!("{} [line {}]\n", ch.title, ch.line_num));
        if let Some(summary) = node_summary(&ch.node) {
            out.push_str(&format!("  → {summary}\n"));
        }
    }
    out
}

/// Format a single chapter's sub-headings for the LLM.
fn format_chapter_subtree(chapter: &ChapterInfo) -> String {
    let mut out = String::new();
    if let Some(children) = chapter.node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            format_node_compact(child, 0, &mut out);
        }
    }
    out
}

fn format_node_compact(node: &serde_json::Value, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let title = node["title"].as_str().unwrap_or("(untitled)");
    let line_num = node.get("line_num").and_then(|v| v.as_u64());

    if let Some(ln) = line_num {
        out.push_str(&format!("{indent}{title} [line {ln}]\n"));
    } else {
        out.push_str(&format!("{indent}{title}\n"));
    }

    // Include summary if present (short, to keep token budget)
    if let Some(summary) = node_summary(node) {
        out.push_str(&format!("{indent}  → {summary}\n"));
    }

    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            format_node_compact(child, depth + 1, out);
        }
    }
}

/// Parse LLM response array of line numbers, handling many formats:
/// - `[123, 456]` (numbers)
/// - `["123", "456"]` (strings)
/// - `"123, 456"` (comma-separated string)
/// - `[{"line_num": 123}, {"line_num": 456}]` (objects)
fn parse_line_numbers(v: &serde_json::Value) -> Vec<u64> {
    // Try comma-separated string first
    if let Some(s) = v.as_str() {
        return s
            .split(',')
            .filter_map(|p| p.trim().parse::<u64>().ok())
            .collect();
    }

    let arr = match v.as_array() {
        Some(a) => a,
        None => return Vec::new(),
    };

    arr.iter()
        .filter_map(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
                .or_else(|| v.get("line_num").and_then(|ln| ln.as_u64()))
                .or_else(|| v.get("line").and_then(|ln| ln.as_u64()))
        })
        .collect()
}

// ── Two-phase navigation ────────────────────────────────────────────────────

/// Two-phase navigation: first pick chapters (~820 tokens), then pick sections
/// within the selected chapters (~100-5K tokens each).
pub async fn navigate_two_phase(
    ollama: &OllamaClient,
    book: &BookTree,
    question: &str,
) -> Result<Vec<SectionRange>> {
    let chapters = get_level1_chapters(&book.structure);
    if chapters.is_empty() {
        anyhow::bail!("No chapters found in book tree");
    }

    // Phase 1: Pick relevant chapters
    let chapters_toc = format_chapters_toc(&chapters);
    let valid_chapter_lines: std::collections::HashSet<u64> =
        chapters.iter().map(|c| c.line_num).collect();

    let phase1_system = format!(
        "You are navigating the chapters of \"{book_name}\".\n\
         Given the user's question, identify which chapters are most likely to contain the answer.\n\
         Each chapter may include a summary (→) describing its content.\n\
         Return JSON only: {{\"chapters\": [line_num1, line_num2, ...]}}\n\
         Rules:\n\
         - Select 1-3 chapters (at least 1 unless nothing remotely relates)\n\
         - Use the line numbers shown in brackets (numbers only, not strings)\n\
         - Match on topic/meaning, not exact wording — questions may use different terms\n\n\
         Chapters:\n{chapters_toc}",
        book_name = book.name_en,
    );

    let phase1_result = ollama
        .chat_json(&phase1_system, question, None)
        .await
        .context("Phase 1 (chapter selection) failed")?;

    let mut selected_lines: Vec<u64> = parse_line_numbers(
        phase1_result
            .get("chapters")
            .unwrap_or(&serde_json::Value::Null),
    );
    selected_lines.truncate(3);

    // Validate line numbers exist in the tree
    selected_lines.retain(|ln| {
        if valid_chapter_lines.contains(ln) {
            true
        } else {
            tracing::warn!("LLM returned non-existent chapter line {ln}, dropping");
            false
        }
    });

    // Retry with a broader prompt if Phase 1 returned nothing
    if selected_lines.is_empty() {
        tracing::warn!("Phase 1 returned no valid chapters, retrying with broader prompt");
        let retry_system = format!(
            "You are navigating the chapters of \"{book_name}\".\n\
             The user's question may use different terminology than chapter titles.\n\
             Look for chapters whose topic could RELATE to the question, even loosely.\n\
             If truly nothing matches, pick the 2 most foundational/introductory chapters.\n\
             Return JSON only: {{\"chapters\": [line_num1, line_num2]}}\n\
             Use ONLY the line numbers shown in brackets.\n\n\
             Chapters:\n{chapters_toc}",
            book_name = book.name_en,
        );
        if let Ok(retry_result) = ollama.chat_json(&retry_system, question, None).await {
            selected_lines = parse_line_numbers(
                retry_result
                    .get("chapters")
                    .unwrap_or(&serde_json::Value::Null),
            );
            selected_lines.truncate(3);
            selected_lines.retain(|ln| valid_chapter_lines.contains(ln));
        }
    }

    // Final fallback: pick first 2 chapters
    if selected_lines.is_empty() {
        tracing::warn!("All LLM navigation attempts failed, falling back to first 2 chapters");
        selected_lines = chapters.iter().take(2).map(|c| c.line_num).collect();
    }

    // Phase 2: For each selected chapter, pick specific sections
    let mut all_ranges: Vec<SectionRange> = Vec::new();

    for &chapter_line in &selected_lines {
        let chapter = match chapters.iter().find(|c| c.line_num == chapter_line) {
            Some(c) => c,
            None => continue,
        };

        let subtree = format_chapter_subtree(chapter);
        if subtree.is_empty() {
            // Chapter has no sub-headings; use the chapter itself
            all_ranges.push(SectionRange {
                start_line: chapter.line_num,
                end_line: chapter.line_num + 500,
                reason: chapter.title.clone(),
            });
            continue;
        }

        // Collect valid line numbers in this subtree for validation
        let mut valid_section_lines: std::collections::HashSet<u64> =
            std::collections::HashSet::new();
        if let Some(children) = chapter.node.get("nodes").and_then(|v| v.as_array()) {
            for child in children {
                collect_line_nums(child, &mut valid_section_lines);
            }
        }

        let phase2_system = format!(
            "You are reading the sections within chapter \"{chapter_title}\" of \"{book_name}\".\n\
             Given the user's question, identify the most relevant sections.\n\
             Return JSON only: {{\"sections\": [{{\"start_line\": N, \"end_line\": N, \"reason\": \"...\"}}]}}\n\
             Rules:\n\
             - Select 1-3 sections (at least 1 if anything looks related)\n\
             - Use the line numbers shown in brackets (numbers, not strings)\n\
             - For end_line, use the start_line of the NEXT section, or add 200 if it's the last\n\n\
             Sections:\n{subtree}",
            chapter_title = chapter.title,
            book_name = book.name_en,
        );

        let mut chapter_added = 0;
        match ollama.chat_json(&phase2_system, question, None).await {
            Ok(result) => {
                if let Some(sections) = result.get("sections").and_then(|v| v.as_array()) {
                    for section in sections.iter().take(3) {
                        if let Ok(mut range) =
                            serde_json::from_value::<SectionRange>(section.clone())
                        {
                            // Validate start_line exists
                            if valid_section_lines.contains(&range.start_line) {
                                // Ensure end_line > start_line
                                if range.end_line <= range.start_line {
                                    range.end_line = range.start_line + 200;
                                }
                                all_ranges.push(range);
                                chapter_added += 1;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Phase 2 failed for chapter {}: {e}", chapter.title);
            }
        }

        // Fallback: if phase 2 gave nothing for this chapter, use the chapter itself
        if chapter_added == 0 {
            tracing::warn!(
                "Phase 2 yielded no valid sections for chapter {}, using whole chapter",
                chapter.title
            );
            all_ranges.push(SectionRange {
                start_line: chapter.line_num,
                end_line: chapter.line_num + 500,
                reason: format!("Fallback: entire chapter '{}'", chapter.title),
            });
        }

        if all_ranges.len() >= 5 {
            break;
        }
    }

    all_ranges.truncate(5);
    Ok(all_ranges)
}

// ── Section text fetching ───────────────────────────────────────────────────

/// Collect all line_num values in a subtree (for validation).
fn collect_line_nums(node: &serde_json::Value, out: &mut std::collections::HashSet<u64>) {
    if let Some(ln) = node.get("line_num").and_then(|v| v.as_u64()) {
        out.insert(ln);
    }
    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            collect_line_nums(child, out);
        }
    }
}

/// Fetch section text from the tree's embedded text content.
/// If a range returns no content, expands search by ±200 lines.
pub fn fetch_sections(book: &BookTree, ranges: &[SectionRange]) -> Result<Vec<SectionContent>> {
    let mut results = Vec::new();

    for range in ranges {
        let mut found = Vec::new();
        collect_sections_in_range(
            &book.structure,
            range.start_line,
            range.end_line,
            &mut found,
        );

        // Fallback: expand range by ±200 lines if empty
        if found.is_empty() {
            let expanded_start = range.start_line.saturating_sub(200);
            let expanded_end = range.end_line + 200;
            tracing::warn!(
                "No content in range [{}-{}], expanding to [{}-{}]",
                range.start_line,
                range.end_line,
                expanded_start,
                expanded_end
            );
            collect_sections_in_range(&book.structure, expanded_start, expanded_end, &mut found);
        }

        results.extend(found);
    }

    // Deduplicate by line number (ranges may overlap after expansion)
    let mut seen = std::collections::HashSet::new();
    results.retain(|s| seen.insert(s.line));

    Ok(results)
}

fn collect_sections_in_range(
    node: &serde_json::Value,
    start: u64,
    end: u64,
    results: &mut Vec<SectionContent>,
) {
    if let Some(arr) = node.as_array() {
        for child in arr {
            collect_sections_in_range(child, start, end, results);
        }
        return;
    }

    let line_num = node.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);

    if line_num >= start && line_num <= end {
        let title = node
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let text = node
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if !text.is_empty() {
            results.push(SectionContent {
                line: line_num,
                title,
                text,
            });
        }
    }

    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        for child in children {
            collect_sections_in_range(child, start, end, results);
        }
    }
}

// ── Answer generation context builder ───────────────────────────────────────

/// Build the system prompt with section excerpts for the answer generation step.
pub fn build_answer_prompt(book_name: &str, sections: &[SectionContent]) -> String {
    let mut context = String::new();
    for s in sections {
        context.push_str(&format!(
            "--- [{title}] (line {line}) ---\n{text}\n\n",
            title = s.title,
            line = s.line,
            text = s.text
        ));
    }

    // Cap context at ~25K bytes (find valid char boundary)
    if context.len() > 25_000 {
        let safe = truncate_str(&context, 25_000).len();
        context.truncate(safe);
        context.push_str("\n... (content truncated)\n");
    }

    format!(
        "You are a knowledgeable Islamic scholar answering questions about \"{book_name}\".\n\
         Use ONLY the section excerpts provided below as context.\n\
         Always cite your sources by mentioning the section title when referencing specific content.\n\
         If the excerpts don't contain relevant information, say so honestly.\n\
         Respond in the same language as the user's question.\n\
         Be concise and accurate.\n\n\
         ## Section Excerpts:\n\n{context}"
    )
}

// ── Build sources from section ranges and tree structure ────────────────────

/// Convert section ranges into source citations.
pub fn build_sources(book: &BookTree, ranges: &[SectionRange]) -> Vec<BookSource> {
    let mut sources = Vec::new();

    for range in ranges {
        if let Some(title) = find_title_at_line(&book.structure, range.start_line) {
            sources.push(BookSource {
                line: range.start_line,
                title,
            });
        }
    }

    sources
}

fn find_title_at_line(node: &serde_json::Value, target_line: u64) -> Option<String> {
    if let Some(arr) = node.as_array() {
        for child in arr {
            if let Some(t) = find_title_at_line(child, target_line) {
                return Some(t);
            }
        }
        return None;
    }

    let line_num = node.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);
    if line_num == target_line {
        return node
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    if let Some(children) = node.get("nodes").and_then(|v| v.as_array()) {
        let mut best: Option<String> = None;
        for child in children {
            if let Some(t) = find_title_at_line(child, target_line) {
                return Some(t);
            }
            let child_line = child.get("line_num").and_then(|v| v.as_u64()).unwrap_or(0);
            if child_line <= target_line {
                best = child
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
            }
        }
        if best.is_some() {
            return best;
        }
    }

    None
}
