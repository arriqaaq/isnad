//! Hadith type classification (marfu', mawquf, mursal, qudsi, maqtu').
//!
//! Classifies hadiths based on:
//! - Matn text analysis for Prophet/divine speech references
//! - Last narrator's generation (Companion vs Tabi'i)
//!
//! Must run AFTER narrator_bio enrichment (which populates the generation field).

use anyhow::Result;
use surrealdb::Surreal;
use surrealdb::types::{RecordId, SurrealValue};

use crate::db::Db;
use crate::ingest::sanadset::normalize_arabic;
use crate::models::HadithType;

/// Check if normalized text contains a reference to the Prophet Muhammad.
///
/// Matches common Arabic phrases (after diacritic removal) but NOT bare "محمد"
/// since that's an extremely common narrator name.
fn has_prophet_reference(normalized: &str) -> bool {
    const PATTERNS: &[&str] = &[
        "عن النبي",
        "قال النبي",
        "ان النبي",
        "عن رسول الله",
        "قال رسول الله",
        "نهى رسول الله",
        "امر رسول الله",
        "سمعت رسول الله",
        "صلى الله عليه وسلم",
        "عليه الصلاه والسلام",
        "عنه عليه السلام",
    ];
    PATTERNS.iter().any(|p| normalized.contains(p))
}

/// Check if normalized matn text indicates a Qudsi (divine) hadith.
fn is_qudsi_text(normalized: &str) -> bool {
    const PATTERNS: &[&str] = &[
        "قال الله",
        "يقول الله",
        "ان الله قال",
        "ان الله يقول",
        "قال الله تعالى",
        "قال الله تبارك",
        "فيما يرويه عن ربه",
        "فيما رواه عن ربه",
        "يرويه عن ربه",
    ];
    PATTERNS.iter().any(|p| normalized.contains(p))
}

#[derive(Debug, SurrealValue)]
struct HadithForClassify {
    id: Option<RecordId>,
    matn: Option<String>,
    text_ar: Option<String>,
    narrator_text: Option<String>,
}

#[derive(Debug, SurrealValue)]
struct NarratorGeneration {
    generation: Option<String>,
}

/// Classify all hadiths that don't yet have a hadith_type.
/// Returns the number of hadiths classified.
pub async fn classify_hadith_types(db: &Surreal<Db>) -> Result<usize> {
    // Fetch all unclassified hadiths
    let mut res = db
        .query("SELECT id, matn, text_ar, narrator_text FROM hadith WHERE hadith_type IS NONE")
        .await?;
    let hadiths: Vec<HadithForClassify> = res.take(0)?;

    let total = hadiths.len();
    if total == 0 {
        println!("   No unclassified hadiths found.");
        return Ok(0);
    }

    println!("   Classifying {total} hadiths...");
    let mut classified = 0;

    for h in &hadiths {
        let Some(id) = &h.id else { continue };

        // Normalize the matn/text for pattern matching
        let text_source = h.matn.as_deref().or(h.text_ar.as_deref()).unwrap_or("");
        let normalized_text = normalize_arabic(text_source);
        let normalized_narrator = h
            .narrator_text
            .as_deref()
            .map(normalize_arabic)
            .unwrap_or_default();

        // Step 1: Check for Qudsi (divine speech in matn)
        if is_qudsi_text(&normalized_text) {
            update_type(db, id, HadithType::Qudsi).await;
            classified += 1;
            continue;
        }

        // Step 2: Check for Prophet reference in text or narrator_text
        let has_prophet_ref =
            has_prophet_reference(&normalized_text) || has_prophet_reference(&normalized_narrator);

        // Step 3: Get the last narrator's generation (highest chain_position = closest to Prophet)
        let last_gen = get_last_narrator_generation(db, id).await;
        let gen_lower = last_gen.as_deref().unwrap_or("").to_lowercase();
        let is_companion = gen_lower.contains("sahab") || gen_lower.contains("صحاب");
        let is_tabii = gen_lower.contains("tabi") || gen_lower.contains("تابع");

        // Step 4: Classify
        let hadith_type = if has_prophet_ref {
            if is_tabii && !is_companion {
                // Tabi'i claiming from Prophet without Companion = mursal
                Some(HadithType::Mursal)
            } else {
                // Companion narrating from Prophet, or generation unknown but Prophet referenced
                Some(HadithType::Marfu)
            }
        } else if is_companion {
            Some(HadithType::Mawquf)
        } else if is_tabii {
            Some(HadithType::Maqtu)
        } else {
            None // Insufficient data
        };

        if let Some(ht) = hadith_type {
            update_type(db, id, ht).await;
            classified += 1;
        }
    }

    Ok(classified)
}

async fn get_last_narrator_generation(db: &Surreal<Db>, hadith_id: &RecordId) -> Option<String> {
    let mut res = db
        .query(
            "SELECT <-narrates<-narrator.generation AS generation \
             FROM $rid \
             ORDER BY chain_position DESC \
             LIMIT 1",
        )
        .bind(("rid", hadith_id.clone()))
        .await
        .ok()?;

    let result: Option<NarratorGeneration> = res.take(0).ok()?;
    result.and_then(|r| r.generation)
}

async fn update_type(db: &Surreal<Db>, hadith_id: &RecordId, hadith_type: HadithType) {
    db.query("UPDATE $rid SET hadith_type = $ht")
        .bind(("rid", hadith_id.clone()))
        .bind(("ht", hadith_type.as_str()))
        .await
        .ok();
}
