//! Word-level matn (hadith text body) diffing using Longest Common Subsequence.

use serde::Serialize;

/// Maximum grid cells for LCS computation (safety guard).
const MAX_GRID_CELLS: usize = 120_000;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum DiffKind {
    Unchanged,
    Added,
    Missing,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffSegment {
    pub text: String,
    pub kind: DiffKind,
}

#[derive(Debug, Serialize)]
pub struct MatnDiffResult {
    pub hadith_a: String,
    pub hadith_b: String,
    pub segments_a: Vec<DiffSegment>,
    pub segments_b: Vec<DiffSegment>,
    pub similarity_ratio: f64,
}

/// Compute word-level diff between two hadith texts using LCS.
pub fn diff_matn(text_a: &str, text_b: &str, id_a: &str, id_b: &str) -> MatnDiffResult {
    let words_a: Vec<&str> = text_a.split_whitespace().collect();
    let words_b: Vec<&str> = text_b.split_whitespace().collect();

    let n = words_a.len();
    let m = words_b.len();

    // Safety guard
    if n * m > MAX_GRID_CELLS {
        return MatnDiffResult {
            hadith_a: id_a.to_string(),
            hadith_b: id_b.to_string(),
            segments_a: vec![DiffSegment {
                text: text_a.to_string(),
                kind: DiffKind::Missing,
            }],
            segments_b: vec![DiffSegment {
                text: text_b.to_string(),
                kind: DiffKind::Added,
            }],
            similarity_ratio: 0.0,
        };
    }

    // Standard LCS DP
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in 1..=n {
        for j in 1..=m {
            if words_a[i - 1] == words_b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let lcs_len = dp[n][m];

    // Backtrack to produce diff segments
    let mut seg_a = Vec::new();
    let mut seg_b = Vec::new();
    let mut i = n;
    let mut j = m;

    // Collect operations in reverse
    enum Op {
        Match(usize, usize),
        DeleteA(usize),
        InsertB(usize),
    }
    let mut ops = Vec::new();

    while i > 0 && j > 0 {
        if words_a[i - 1] == words_b[j - 1] {
            ops.push(Op::Match(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] >= dp[i][j - 1] {
            ops.push(Op::DeleteA(i - 1));
            i -= 1;
        } else {
            ops.push(Op::InsertB(j - 1));
            j -= 1;
        }
    }
    while i > 0 {
        ops.push(Op::DeleteA(i - 1));
        i -= 1;
    }
    while j > 0 {
        ops.push(Op::InsertB(j - 1));
        j -= 1;
    }

    ops.reverse();

    // Build segments by merging consecutive same-kind words
    let mut current_a_words: Vec<&str> = Vec::new();
    let mut current_a_kind = DiffKind::Unchanged;
    let mut current_b_words: Vec<&str> = Vec::new();
    let mut current_b_kind = DiffKind::Unchanged;

    let flush_a = |words: &mut Vec<&str>, kind: &DiffKind, out: &mut Vec<DiffSegment>| {
        if !words.is_empty() {
            out.push(DiffSegment {
                text: words.join(" "),
                kind: kind.clone(),
            });
            words.clear();
        }
    };

    let flush_b = |words: &mut Vec<&str>, kind: &DiffKind, out: &mut Vec<DiffSegment>| {
        if !words.is_empty() {
            out.push(DiffSegment {
                text: words.join(" "),
                kind: kind.clone(),
            });
            words.clear();
        }
    };

    for op in &ops {
        match op {
            Op::Match(ai, bi) => {
                if current_a_kind != DiffKind::Unchanged {
                    flush_a(&mut current_a_words, &current_a_kind, &mut seg_a);
                }
                if current_b_kind != DiffKind::Unchanged {
                    flush_b(&mut current_b_words, &current_b_kind, &mut seg_b);
                }
                current_a_kind = DiffKind::Unchanged;
                current_b_kind = DiffKind::Unchanged;
                current_a_words.push(words_a[*ai]);
                current_b_words.push(words_b[*bi]);
            }
            Op::DeleteA(ai) => {
                if current_a_kind != DiffKind::Missing {
                    flush_a(&mut current_a_words, &current_a_kind, &mut seg_a);
                    current_a_kind = DiffKind::Missing;
                }
                current_a_words.push(words_a[*ai]);
            }
            Op::InsertB(bi) => {
                if current_b_kind != DiffKind::Added {
                    flush_b(&mut current_b_words, &current_b_kind, &mut seg_b);
                    current_b_kind = DiffKind::Added;
                }
                current_b_words.push(words_b[*bi]);
            }
        }
    }

    // Flush remaining
    if !current_a_words.is_empty() {
        seg_a.push(DiffSegment {
            text: current_a_words.join(" "),
            kind: current_a_kind,
        });
    }
    if !current_b_words.is_empty() {
        seg_b.push(DiffSegment {
            text: current_b_words.join(" "),
            kind: current_b_kind,
        });
    }

    let total_words = n + m;
    let similarity = if total_words > 0 {
        (2 * lcs_len) as f64 / total_words as f64
    } else {
        1.0
    };

    MatnDiffResult {
        hadith_a: id_a.to_string(),
        hadith_b: id_b.to_string(),
        segments_a: seg_a,
        segments_b: seg_b,
        similarity_ratio: similarity,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_texts() {
        let result = diff_matn("hello world", "hello world", "a", "b");
        assert_eq!(result.similarity_ratio, 1.0);
        assert_eq!(result.segments_a.len(), 1);
        assert_eq!(result.segments_a[0].kind, DiffKind::Unchanged);
    }

    #[test]
    fn test_completely_different() {
        let result = diff_matn("foo bar", "baz qux", "a", "b");
        assert_eq!(result.similarity_ratio, 0.0);
    }

    #[test]
    fn test_partial_overlap() {
        let result = diff_matn(
            "the prophet said pray five times",
            "the prophet said fast during ramadan",
            "a",
            "b",
        );
        assert!(result.similarity_ratio > 0.0);
        assert!(result.similarity_ratio < 1.0);
    }
}
