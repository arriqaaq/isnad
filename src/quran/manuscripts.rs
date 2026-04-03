use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use quick_xml::Reader;
use quick_xml::events::Event;
use surrealdb::Surreal;
use surrealdb::types::RecordId;

use crate::db::Db;

fn rid(table: &str, key: &str) -> RecordId {
    RecordId::new(table, key)
}

/// Ingest manuscript descriptions and variant readings from Corpus Coranicum TEI XML.
pub async fn ingest_manuscripts(db: &Surreal<Db>, tei_dir: &str) -> Result<()> {
    let manuscripts_dir = format!("{tei_dir}/data/quran_manuscripts");
    let variants_dir = format!("{tei_dir}/data/quran_variants");

    // Phase 1: Parse manuscripts
    let ms_path = std::path::Path::new(&manuscripts_dir);
    if ms_path.is_dir() {
        let xml_files = collect_xml_files(ms_path)?;
        if !xml_files.is_empty() {
            println!("Parsing {} manuscript XML files...", xml_files.len());
            let pb = ProgressBar::new(xml_files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("   {bar:40.cyan/blue} {pos}/{len} manuscripts ({eta})")
                    .unwrap(),
            );

            let mut total = 0usize;
            for path in &xml_files {
                match parse_manuscript_file(path) {
                    Ok(manuscripts) => {
                        for ms in &manuscripts {
                            let key = slug(&ms.name);
                            if key.is_empty() {
                                continue;
                            }
                            let sql = r#"
                                INSERT INTO manuscript {
                                    id: $id,
                                    name: $name,
                                    repository: $repository,
                                    location: $location,
                                    date_range: $date_range,
                                    material: $material,
                                    script_type: NONE,
                                    description: NONE,
                                    source_url: NONE,
                                    surah_start: NONE,
                                    surah_end: NONE,
                                    ayah_start: NONE,
                                    ayah_end: NONE
                                } ON DUPLICATE KEY UPDATE
                                    repository = $repository,
                                    location = $location,
                                    date_range = $date_range,
                                    material = $material
                            "#;
                            if let Err(e) = db
                                .query(sql)
                                .bind(("id", rid("manuscript", &key)))
                                .bind(("name", ms.name.clone()))
                                .bind(("repository", ms.repository.clone()))
                                .bind(("location", ms.location.clone()))
                                .bind(("date_range", ms.date_range.clone()))
                                .bind(("material", ms.material.clone()))
                                .await
                                .and_then(|r| r.check())
                            {
                                tracing::warn!("Failed to insert manuscript '{}': {e}", ms.name);
                            } else {
                                total += 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse {}: {e}", path.display());
                    }
                }
                pb.inc(1);
            }
            pb.finish_with_message("done");
            println!("   Inserted {total} manuscripts");
        }
    } else {
        tracing::info!("No manuscripts directory found at {manuscripts_dir}, skipping");
    }

    // Phase 2: Parse variant readings
    let var_path = std::path::Path::new(&variants_dir);
    if var_path.is_dir() {
        let xml_files = collect_xml_files(var_path)?;
        if !xml_files.is_empty() {
            println!("Parsing {} variant reading XML files...", xml_files.len());
            let pb = ProgressBar::new(xml_files.len() as u64);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("   {bar:40.cyan/blue} {pos}/{len} variant files ({eta})")
                    .unwrap(),
            );

            let mut total = 0usize;
            let mut variant_id = 0u64;
            for path in &xml_files {
                match parse_variant_file(path) {
                    Ok(variants) => {
                        for v in &variants {
                            variant_id += 1;
                            let key = format!("v{variant_id}");
                            let sql = r#"
                                INSERT INTO variant_reading {
                                    id: $id,
                                    surah_number: $surah,
                                    ayah_number: $ayah,
                                    word_position: $word_pos,
                                    reader_name: $reader,
                                    reading_ar: $reading,
                                    standard_ar: $standard,
                                    source: $source,
                                    manuscript_id: NONE
                                }
                            "#;
                            if let Err(e) = db
                                .query(sql)
                                .bind(("id", rid("variant_reading", &key)))
                                .bind(("surah", v.surah_number))
                                .bind(("ayah", v.ayah_number))
                                .bind(("word_pos", v.word_position))
                                .bind(("reader", v.reader_name.clone()))
                                .bind(("reading", v.reading_ar.clone()))
                                .bind(("standard", v.standard_ar.clone()))
                                .bind(("source", v.source.clone()))
                                .await
                                .and_then(|r| r.check())
                            {
                                tracing::warn!("Failed to insert variant: {e}");
                            } else {
                                total += 1;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse variants {}: {e}", path.display());
                    }
                }
                pb.inc(1);
            }
            pb.finish_with_message("done");
            println!("   Inserted {total} variant readings");
        }
    } else {
        tracing::info!("No variants directory found at {variants_dir}, skipping");
    }

    Ok(())
}

// ── Internal types ──

struct ParsedManuscript {
    name: String,
    repository: Option<String>,
    location: Option<String>,
    date_range: Option<String>,
    material: Option<String>,
}

struct ParsedVariant {
    surah_number: i64,
    ayah_number: i64,
    word_position: Option<i64>,
    reader_name: String,
    reading_ar: String,
    standard_ar: Option<String>,
    source: Option<String>,
}

// ── Helpers ──

fn collect_xml_files(dir: &std::path::Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "xml") {
                files.push(path);
            } else if path.is_dir() {
                // Recurse one level
                for sub in std::fs::read_dir(&path)? {
                    let sub = sub?;
                    let sp = sub.path();
                    if sp.extension().is_some_and(|e| e == "xml") {
                        files.push(sp);
                    }
                }
            }
        }
    }
    files.sort();
    Ok(files)
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Extract text content from all child events until a matching end tag.
fn read_text_until_end(reader: &mut Reader<&[u8]>, end_tag: &[u8]) -> String {
    let mut buf = Vec::new();
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Text(e)) => {
                if let Ok(t) = e.unescape() {
                    text.push_str(&t);
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == end_tag => break,
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    text.trim().to_string()
}

/// Parse a single manuscript TEI XML file.
fn parse_manuscript_file(path: &std::path::Path) -> Result<Vec<ParsedManuscript>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut manuscripts = Vec::new();
    let mut buf = Vec::new();

    // State for current msDesc being parsed
    let mut in_ms_desc = false;
    let mut in_ms_identifier = false;
    let mut in_history = false;
    let mut in_phys_desc = false;

    let mut name = String::new();
    let mut repository: Option<String> = None;
    let mut location: Option<String> = None;
    let mut date_range: Option<String> = None;
    let mut material: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_slice() {
                    b"msDesc" => {
                        in_ms_desc = true;
                        name.clear();
                        repository = None;
                        location = None;
                        date_range = None;
                        material = None;
                    }
                    b"msIdentifier" if in_ms_desc => {
                        in_ms_identifier = true;
                    }
                    b"idno" if in_ms_identifier => {
                        let t = read_text_until_end(&mut reader, b"idno");
                        if !t.is_empty() && name.is_empty() {
                            name = t;
                        }
                    }
                    b"repository" if in_ms_identifier => {
                        let t = read_text_until_end(&mut reader, b"repository");
                        if !t.is_empty() {
                            repository = Some(t);
                        }
                    }
                    b"settlement" if in_ms_identifier => {
                        let t = read_text_until_end(&mut reader, b"settlement");
                        if !t.is_empty() {
                            location = Some(t);
                        }
                    }
                    b"history" if in_ms_desc => {
                        in_history = true;
                    }
                    b"origDate" if in_history => {
                        let t = read_text_until_end(&mut reader, b"origDate");
                        if !t.is_empty() {
                            date_range = Some(t);
                        }
                    }
                    b"physDesc" if in_ms_desc => {
                        in_phys_desc = true;
                    }
                    b"material" if in_phys_desc => {
                        let t = read_text_until_end(&mut reader, b"material");
                        if !t.is_empty() {
                            material = Some(t);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_slice() {
                    b"msDesc" => {
                        if !name.is_empty() {
                            manuscripts.push(ParsedManuscript {
                                name: name.clone(),
                                repository: repository.clone(),
                                location: location.clone(),
                                date_range: date_range.clone(),
                                material: material.clone(),
                            });
                        }
                        in_ms_desc = false;
                        in_ms_identifier = false;
                        in_history = false;
                        in_phys_desc = false;
                    }
                    b"msIdentifier" => in_ms_identifier = false,
                    b"history" => in_history = false,
                    b"physDesc" => in_phys_desc = false,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parse error in {}: {e}", path.display());
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    // If no msDesc found, try to extract a manuscript name from the filename
    if manuscripts.is_empty() {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !stem.is_empty() {
            manuscripts.push(ParsedManuscript {
                name: stem,
                repository: None,
                location: None,
                date_range: None,
                material: None,
            });
        }
    }

    Ok(manuscripts)
}

/// Parse a variant readings TEI XML file.
/// The exact structure varies, so we look for common patterns:
/// - `<rdg>` or `<reading>` elements with `wit` or `resp` attributes
/// - `<app>` elements that contain `<lem>` (standard) and `<rdg>` (variant)
/// - Surah/ayah references from `<ref>` or attributes like `n="2:255"`
/// Parse the allvariants.xml TEI file from Corpus Coranicum.
///
/// Structure:
/// ```xml
/// <item xml:id="variant_28">
///   <persName key="variantreader_12">Reader Name</persName>
///   <title key="variantsource_X">Source</title>
///   <ab>
///     <w n="020:002:001">word text</w>
///     <w n="020:002:002">word text</w>
///   </ab>
/// </item>
/// ```
///
/// The `n` attribute on `<w>` gives surah:ayah:word (zero-padded 3 digits).
fn parse_variant_file(path: &std::path::Path) -> Result<Vec<ParsedVariant>> {
    let content = std::fs::read_to_string(path)?;
    let mut reader = Reader::from_str(&content);
    reader.config_mut().trim_text(true);

    let mut variants = Vec::new();
    let mut buf = Vec::new();

    let source = path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    // State machine for parsing <item> elements
    let mut in_item = false;
    let mut reader_name = String::new();
    let mut word_texts: Vec<String> = Vec::new();
    let mut first_surah: Option<i64> = None;
    let mut first_ayah: Option<i64> = None;
    let mut first_word: Option<i64> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let local = local_name(e.name().as_ref());
                match local.as_slice() {
                    b"item" => {
                        in_item = true;
                        reader_name.clear();
                        word_texts.clear();
                        first_surah = None;
                        first_ayah = None;
                        first_word = None;
                    }
                    b"persName" if in_item => {
                        let t = read_text_until_end(&mut reader, b"persName");
                        if !t.is_empty() {
                            reader_name = t;
                        }
                    }
                    b"w" if in_item => {
                        // Extract n="SSS:AAA:WWW" attribute
                        for attr in e.attributes().flatten() {
                            let key = String::from_utf8_lossy(attr.key.as_ref());
                            if key == "n" {
                                let val = String::from_utf8_lossy(&attr.value);
                                let parts: Vec<&str> = val.split(':').collect();
                                if parts.len() == 3 {
                                    if let (Ok(s), Ok(a), Ok(w)) = (
                                        parts[0].parse::<i64>(),
                                        parts[1].parse::<i64>(),
                                        parts[2].parse::<i64>(),
                                    ) {
                                        if first_surah.is_none() {
                                            first_surah = Some(s);
                                            first_ayah = Some(a);
                                            first_word = Some(w);
                                        }
                                    }
                                }
                            }
                        }
                        let t = read_text_until_end(&mut reader, b"w");
                        if !t.is_empty() {
                            word_texts.push(t);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                let local = local_name(e.name().as_ref());
                if local.as_slice() == b"item" && in_item {
                    // Emit variant if we have valid data
                    if !word_texts.is_empty() && first_surah.is_some() && first_ayah.is_some() {
                        variants.push(ParsedVariant {
                            surah_number: first_surah.unwrap(),
                            ayah_number: first_ayah.unwrap(),
                            word_position: first_word,
                            reader_name: if reader_name.is_empty() {
                                "Unknown".to_string()
                            } else {
                                reader_name.clone()
                            },
                            reading_ar: word_texts.join(" "),
                            standard_ar: None,
                            source: source.clone(),
                        });
                    }
                    in_item = false;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parse error in {}: {e}", path.display());
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(variants)
}

/// Strip namespace prefix from XML tag name (e.g., "tei:msDesc" -> "msDesc").
/// Returns owned Vec to avoid borrow lifetime issues with quick-xml events.
fn local_name(name: &[u8]) -> Vec<u8> {
    if let Some(pos) = name.iter().position(|&b| b == b':') {
        name[pos + 1..].to_vec()
    } else {
        name.to_vec()
    }
}

/// Try to parse surah:ayah from a string like "Q2:255", "2:255", "002_255", "s2.v255"
fn parse_verse_ref(s: &str) -> Option<(i64, i64)> {
    // Pattern: "Q2:255" or "2:255"
    let cleaned = s.trim_start_matches('Q').trim_start_matches('q');
    if let Some((a, b)) = cleaned.split_once(':') {
        let surah = a.trim().parse::<i64>().ok()?;
        let ayah = b.trim().parse::<i64>().ok()?;
        if (1..=114).contains(&surah) && ayah > 0 {
            return Some((surah, ayah));
        }
    }
    // Pattern: "002_255"
    if let Some((a, b)) = s.split_once('_') {
        let surah = a.trim().parse::<i64>().ok()?;
        let ayah = b.trim().parse::<i64>().ok()?;
        if (1..=114).contains(&surah) && ayah > 0 {
            return Some((surah, ayah));
        }
    }
    None
}

/// Try to parse surah/ayah from filename stem (e.g., "002_255" or "s002a255")
fn parse_surah_ayah_from_stem(stem: &str) -> Option<(i64, i64)> {
    parse_verse_ref(stem)
}
