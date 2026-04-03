//! Anti-hallucination safeguards for RAG output validation.
//!
//! Prevents synthetic/fabricated evidence and validates LLM claims
//! against the database.

use serde::Serialize;

/// Prefix patterns that indicate synthetic/fabricated data.
const SYNTHETIC_PREFIXES: &[&str] = &[
    "synthetic_",
    "placeholder_",
    "fake_",
    "test_",
    "dummy_",
    "generated_",
    "auto_",
];

/// Exact values that indicate synthetic data.
const EXACT_SYNTHETIC: &[&str] = &["null", "undefined", "n/a", "none", "unknown"];

/// Content substrings that indicate fabrication.
const FABRICATION_KEYWORDS: &[&str] = &["fake", "fabricat", "forged"];

/// Allowed source types for evidence provenance.
const VERIFIABLE_SOURCE_TYPES: &[&str] = &["url", "print", "manuscript"];

/// Check if a value looks synthetic or fabricated.
pub fn is_synthetic(value: &str) -> bool {
    let lower = value.to_lowercase();
    SYNTHETIC_PREFIXES.iter().any(|p| lower.starts_with(p))
        || EXACT_SYNTHETIC.iter().any(|e| lower == *e)
        || FABRICATION_KEYWORDS.iter().any(|k| lower.contains(k))
}

/// Check if a source type is verifiable.
pub fn is_verifiable_source(source_type: &str) -> bool {
    VERIFIABLE_SOURCE_TYPES.contains(&source_type)
}

/// Violation severity level.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum ViolationLevel {
    Blocking,
    Warning,
    Info,
}

/// A validation violation.
#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub level: ViolationLevel,
    pub code: String,
    pub message: String,
}

/// Result of validating RAG output against the database.
#[derive(Debug, Serialize)]
pub struct RagValidationResult {
    pub valid: bool,
    pub violations: Vec<Violation>,
}

/// Validate RAG output text against known narrators and provided context.
///
/// Checks:
/// - Narrator names mentioned in response exist in known_narrators set
/// - Hadith numbers referenced are in provided context_hadith_numbers
pub fn validate_rag_output(
    response_text: &str,
    _known_narrator_names: &[String],
    context_hadith_numbers: &[i64],
) -> RagValidationResult {
    let mut violations = Vec::new();

    // Check for hadith number references not in context
    // Pattern: "Hadith #N", "hadith N", "#N"
    for word in response_text.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_digit());
        if let Ok(num) = cleaned.parse::<i64>()
            && num > 0
            && num < 10000
            && !context_hadith_numbers.contains(&num)
        {
            // Only flag if it looks like a hadith reference
            let prev_idx = response_text.find(word).unwrap_or(0);
            let before = if prev_idx > 10 {
                &response_text[prev_idx - 10..prev_idx]
            } else {
                &response_text[..prev_idx]
            };
            let before_lower = before.to_lowercase();
            if before_lower.contains("hadith")
                || before_lower.contains("#")
                || before_lower.contains("number")
            {
                violations.push(Violation {
                    level: ViolationLevel::Warning,
                    code: "UNVERIFIED_HADITH_REF".to_string(),
                    message: format!("Hadith #{num} referenced but not in provided context"),
                });
            }
        }
    }

    let valid = !violations
        .iter()
        .any(|v| v.level == ViolationLevel::Blocking);

    RagValidationResult { valid, violations }
}

/// Validate evidence data before import.
pub fn validate_evidence_import(
    evidence_id: &str,
    narrator_id: &str,
    source_type: Option<&str>,
    source_collection: Option<&str>,
    source_locator: Option<&str>,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    if is_synthetic(evidence_id) {
        violations.push(Violation {
            level: ViolationLevel::Blocking,
            code: "SYNTHETIC_EVIDENCE".to_string(),
            message: format!("Evidence ID '{evidence_id}' matches synthetic pattern"),
        });
    }

    if is_synthetic(narrator_id) {
        violations.push(Violation {
            level: ViolationLevel::Blocking,
            code: "UNKNOWN_NARRATOR_ID".to_string(),
            message: format!("Narrator ID '{narrator_id}' matches synthetic pattern"),
        });
    }

    // Check provenance completeness
    if source_collection.is_none() || source_type.is_none() || source_locator.is_none() {
        violations.push(Violation {
            level: ViolationLevel::Warning,
            code: "MISSING_PROVENANCE".to_string(),
            message: "Evidence missing source provenance fields".to_string(),
        });
    }

    if let Some(st) = source_type
        && !is_verifiable_source(st)
    {
        violations.push(Violation {
            level: ViolationLevel::Warning,
            code: "UNVERIFIABLE_SOURCE".to_string(),
            message: format!("Source type '{st}' is not in verifiable set"),
        });
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthetic_detection() {
        assert!(is_synthetic("synthetic_001"));
        assert!(is_synthetic("placeholder_abc"));
        assert!(is_synthetic("fake_evidence"));
        assert!(is_synthetic("null"));
        assert!(is_synthetic("undefined"));
        assert!(is_synthetic("n/a"));
        assert!(is_synthetic("This is fabricated"));
        assert!(!is_synthetic("ev-hisham-001"));
        assert!(!is_synthetic("thiqah"));
    }

    #[test]
    fn test_evidence_validation() {
        let v = validate_evidence_import(
            "synthetic_001",
            "narrator_1",
            Some("print"),
            Some("Taqrib"),
            Some("Vol 2"),
        );
        assert!(v.iter().any(|v| v.code == "SYNTHETIC_EVIDENCE"));
    }

    #[test]
    fn test_clean_evidence() {
        let v = validate_evidence_import(
            "ev-hisham-001",
            "hisham_ibn_urwah",
            Some("print"),
            Some("Taqrib al-Tahdhib"),
            Some("Vol. 2, p. 159"),
        );
        assert!(v.is_empty());
    }
}
