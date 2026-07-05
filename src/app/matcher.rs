//! Greedy fuzzy-matching engine with scoring, inspired by fzf-style algorithms.
//!
//! Query characters must appear in order in the target (case-insensitive),
//! but not necessarily consecutively.  Results carry a relevance score so
//! they can be ranked: the best match is the one with the highest score.
//!
//! Whitespace in the query is treated as a term separator — every term
//! must match independently.  This lets users type `src main` to find
//! `src/main.rs`.
//!
//! # Hot path
//!
//! Build a single [`FuzzyMatcher`] per keystroke and reuse it across all
//! candidate targets.  The matcher owns the prepared pattern and reusable
//! scratch buffers, so after the first warm-up call no per-candidate heap
//! allocations are needed.
//!
//! # Scoring
//!
//! The greedy scanner awards bonuses for:
//!
//! * **Consecutive** characters (adjacent in both query and target).
//! * **Word boundaries** (after `/`, `_`, `-`, `.`, space, or a
//!   lowercase→uppercase camelCase transition).
//! * **Path-segment start** (beginning of a directory or file name).
//! * **Basename start** (beginning of the final path component).
//!
//! And penalises gaps between matched characters.

use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Score constants
// ---------------------------------------------------------------------------

mod score {
    /// Bonus for adjacent matched characters.
    pub const CONSECUTIVE: i32 = 15;
    /// Bonus for matching after a word separator or camelCase transition.
    pub const WORD_BOUNDARY: i32 = 30;
    /// Bonus for matching at position 0 of the target.
    pub const START_OF_STRING: i32 = 40;
    /// Bonus for a lowercase→uppercase transition (camelCase).
    pub const CAMEL_CASE: i32 = 20;
    /// Penalty per unmatched character between two matched positions.
    pub const GAP_PENALTY: i32 = -3;
    /// Extra penalty for the first unmatched character of a gap.
    pub const GAP_START_PENALTY: i32 = -5;
    /// Bonus when the query exactly matches the entire target.
    pub const EXACT_MATCH: i32 = 100;
    /// Bonus when the query matches the basename up to the extension dot
    /// or a word separator (e.g. "main" in "main.rs").
    pub const EXACT_BASENAME_MATCH: i32 = 80;
    /// Bonus for all query characters appearing consecutively in the target.
    pub const CONTIGUOUS_SUBSTRING: i32 = 50;
    /// Bonus when a contiguous match starts at the basename (filename)
    /// — i.e. right after the last `/`.
    pub const BASENAME_PREFIX: i32 = 50;
    /// Smaller bonus when a contiguous match starts at an interior
    /// path segment (after a `/` that is not the one before the basename).
    pub const PATH_SEGMENT_PREFIX: i32 = 30;
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The result of a fuzzy match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyMatch {
    /// Whether all query characters (terms) matched.
    pub matched: bool,
    /// Relevance score.  Higher is better.  Meaningful only when `matched` is
    /// `true`.
    pub score: i32,
    /// Indices (in byte/char offset — see notes) of the matched positions
    /// within the target string.  These are **character** indices
    /// (`.char_indices()` offsets), not byte offsets.
    pub match_positions: Vec<usize>,
}

impl FuzzyMatch {
    pub fn no_match() -> Self {
        Self {
            matched: false,
            score: 0,
            match_positions: Vec::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Prepared pattern (query split into terms, pre-lowercased)
// ---------------------------------------------------------------------------

/// A query pre-processed for repeated matching.
///
/// Terms are separated by whitespace.  Every term must match for the overall
/// query to match.
#[derive(Debug, Clone)]
pub struct FuzzyPattern {
    terms: Vec<PreparedTerm>,
}

/// A single whitespace-separated term from the query.
#[derive(Debug, Clone)]
struct PreparedTerm {
    /// Lowercased characters of the term.
    lower_chars: Vec<char>,
    /// ASCII fast-path bytes — `Some` iff the term is entirely ASCII.
    ascii_lower: Option<Vec<u8>>,
}

impl PreparedTerm {
    fn new(term: &str) -> Self {
        let lower = term.to_lowercase();
        let lower_chars: Vec<char> = lower.chars().collect();
        let ascii_lower = if lower.is_ascii() {
            Some(lower.into_bytes())
        } else {
            None
        };
        Self {
            lower_chars,
            ascii_lower,
        }
    }
}

impl FuzzyPattern {
    /// Prepare a query for repeated fuzzy matching.
    ///
    /// Whitespace-split terms: `"src main rs"` produces three terms
    /// (`"src"`, `"main"`, `"rs"`) which must all match independently.
    pub fn new(query: &str) -> Self {
        let terms: Vec<PreparedTerm> = query
            .split_whitespace()
            .map(PreparedTerm::new)
            .collect();
        Self { terms }
    }

    /// Returns `true` when the query is empty or whitespace-only.
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Reusable matcher (amortises scratch allocation)
// ---------------------------------------------------------------------------

/// A reusable fuzzy matcher that amortises both query preparation and
/// target-side scratch allocation.
///
/// Build one per search (per keystroke) and call [`match_target`](Self::match_target)
/// for each candidate.
#[derive(Debug, Clone)]
pub struct FuzzyMatcher {
    pattern: FuzzyPattern,
    /// Scratch buffer: chars of the current target (reused across calls).
    target_chars: Vec<char>,
    /// Scratch buffer: lowercased chars of the current target.
    target_lower: Vec<char>,
}

impl FuzzyMatcher {
    /// Create a new matcher for `query`.
    ///
    /// Scratch buffers start empty and grow lazily on the first
    /// successful match.
    pub fn new(query: &str) -> Self {
        Self {
            pattern: FuzzyPattern::new(query),
            target_chars: Vec::new(),
            target_lower: Vec::new(),
        }
    }

    /// Returns `true` when the query is empty / whitespace-only.
    pub fn is_empty(&self) -> bool {
        self.pattern.is_empty()
    }

    /// Match `target` against this matcher's query.
    ///
    /// Reuses internal scratch buffers — no per-call allocation after the
    /// first warm-up.
    pub fn match_target(&mut self, target: &str) -> FuzzyMatch {
        if self.pattern.is_empty() {
            return FuzzyMatch {
                matched: true,
                score: 0,
                match_positions: Vec::new(),
            };
        }

        // Fast rejection: every term must pass the subsequence gate.
        for term in &self.pattern.terms {
            if !is_subsequence(term, target) {
                return FuzzyMatch::no_match();
            }
        }

        // Refill scratch buffers in place (grows if needed, reuses otherwise).
        refill_target(&mut self.target_chars, &mut self.target_lower, target);

        if self.pattern.terms.len() == 1 {
            score_term(
                &self.pattern.terms[0],
                &self.target_chars,
                &self.target_lower,
            )
        } else {
            score_multi_term(&self.pattern.terms, &self.target_chars, &self.target_lower)
        }
    }
}

// ---------------------------------------------------------------------------
// Fast subsequence gate (no allocation)
// ---------------------------------------------------------------------------

/// Boolean check: does every character of `term` appear in order in `target`?
///
/// This is a fast, non-allocating pre-check.  It uses an ASCII byte-level
/// fast path when both term and target are ASCII.
fn is_subsequence(term: &PreparedTerm, target: &str) -> bool {
    if let Some(ref ascii_q) = term.ascii_lower {
        if target.is_ascii() {
            return is_subsequence_ascii(ascii_q, target.as_bytes());
        }
    }
    is_subsequence_chars(&term.lower_chars, target)
}

fn is_subsequence_ascii(q: &[u8], t: &[u8]) -> bool {
    if q.is_empty() {
        return true;
    }
    let mut qi = 0;
    for &b in t {
        if b.to_ascii_lowercase() == q[qi] {
            qi += 1;
            if qi == q.len() {
                return true;
            }
        }
    }
    false
}

fn is_subsequence_chars(q: &[char], target: &str) -> bool {
    if q.is_empty() {
        return true;
    }
    let mut qi = 0;
    for lc in target.chars().flat_map(|c| c.to_lowercase()) {
        if lc == q[qi] {
            qi += 1;
            if qi == q.len() {
                return true;
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Scratch-buffer helpers
// ---------------------------------------------------------------------------

/// Fill `chars` and `lower` with the raw and lowercased characters of
/// `target`.  Grows the buffers if needed, reuses capacity otherwise.
fn refill_target(chars: &mut Vec<char>, lower: &mut Vec<char>, target: &str) {
    chars.clear();
    lower.clear();
    chars.reserve(target.len());
    lower.reserve(target.len());
    for c in target.chars() {
        chars.push(c);
        for lc in c.to_lowercase() {
            lower.push(lc);
        }
    }
}

// ---------------------------------------------------------------------------
// Greedy per-term scorer
// ---------------------------------------------------------------------------

/// Check whether `query[qi..]` can be found as a subsequence of
/// `target_lower[ti..]`.
///
/// Pure boolean — no allocation, no scoring.  Used to reject unpromising
/// first-character starting positions.
fn can_match_rest(query: &[char], target_lower: &[char], mut ti: usize) -> bool {
    let mut qi = 0;
    while qi < query.len() && ti < target_lower.len() {
        if target_lower[ti] == query[qi] {
            qi += 1;
        }
        ti += 1;
    }
    qi == query.len()
}

/// Score the first matched character at position `ti` in `target_chars`.
fn score_first_char(ti: usize, target_chars: &[char]) -> i32 {
    if ti == 0 {
        return score::START_OF_STRING;
    }
    if ti < target_chars.len() {
        let prev = target_chars[ti - 1];
        if prev == '/' || prev == '_' || prev == '-' || prev == '.' || prev == ' ' {
            return score::WORD_BOUNDARY;
        }
        if prev.is_lowercase() && target_chars[ti].is_uppercase() {
            return score::CAMEL_CASE;
        }
    }
    0
}

/// Score a single term using a smart greedy scan.
///
/// For the first character we evaluate *all* viable starting positions and
/// pick the one that produces the highest initial bonus (word boundary,
/// start-of-string, etc.) while still allowing the rest of the query to
/// match.  Subsequent characters use a simple left-to-right greedy scan.
fn score_term(
    term: &PreparedTerm,
    target_chars: &[char],
    target_lower: &[char],
) -> FuzzyMatch {
    let query = &term.lower_chars;
    if query.is_empty() {
        return FuzzyMatch {
            matched: true,
            score: 0,
            match_positions: Vec::new(),
        };
    }

    // ------------------------------------------------------------------
    // 1.  Pick the best starting position for the first query char.
    // ------------------------------------------------------------------
    let first_char = query[0];
    let rest = &query[1..];
    let mut best_first_ti: Option<usize> = None;
    let mut best_first_score: i32 = i32::MIN;

    for candidate in 0..target_lower.len() {
        if target_lower[candidate] != first_char {
            continue;
        }
        // If there are remaining chars, verify they can still match.
        if !rest.is_empty() && !can_match_rest(rest, target_lower, candidate + 1) {
            continue;
        }
        let initial_score = score_first_char(candidate, target_chars);
        if initial_score > best_first_score {
            best_first_score = initial_score;
            best_first_ti = Some(candidate);
        }
    }

    let first_ti = match best_first_ti {
        Some(ti) => ti,
        None => return FuzzyMatch::no_match(),
    };

    // ------------------------------------------------------------------
    // 2.  Greedy scan for the remaining characters.
    // ------------------------------------------------------------------
    let mut positions = Vec::with_capacity(query.len());
    let mut match_score: i32 = best_first_score;
    let mut ti = first_ti;
    let mut prev_match_pos: Option<usize> = None;

    for (qi, &qc) in query.iter().enumerate() {
        // For qi == 0 we already know the position.
        if qi == 0 {
            positions.push(ti);
            prev_match_pos = Some(ti);
            ti += 1;
            continue;
        }

        // Advance through target until we find this query char.
        while ti < target_lower.len() && target_lower[ti] != qc {
            ti += 1;
        }
        if ti >= target_lower.len() {
            return FuzzyMatch::no_match();
        }

        // Score the transition.
        if let Some(prev_pos) = prev_match_pos {
            if ti == prev_pos + 1 {
                match_score += score::CONSECUTIVE;
            } else {
                let gap = ti - prev_pos - 1;
                match_score += score::GAP_START_PENALTY;
                match_score += score::GAP_PENALTY * (gap as i32 - 1).max(0);
            }
        }

        positions.push(ti);
        prev_match_pos = Some(ti);
        ti += 1;
    }

    // ------------------------------------------------------------------
    // 3.  Post-match bonuses (contiguous, exact, path-segment prefix).
    // ------------------------------------------------------------------
    let query_len = query.len();
    let contiguous = query_len == 1 || positions.windows(2).all(|w| w[1] == w[0] + 1);

    if contiguous {
        match_score += score::CONTIGUOUS_SUBSTRING;

        // Exact match: query == entire target.
        if query_len == target_lower.len() {
            match_score += score::EXACT_MATCH;
            return FuzzyMatch {
                matched: true,
                score: match_score,
                match_positions: positions,
            };
        }

        // Exact basename match: query is a prefix and the next character
        // is a file-extension dot, word separator, etc.
        let at_start = positions[0] == 0;
        if at_start && query_len < target_chars.len() {
            let next = target_chars[query_len];
            if next == '.' || next == '-' || next == '_' || next == ' ' {
                match_score += score::EXACT_BASENAME_MATCH;
                return FuzzyMatch {
                    matched: true,
                    score: match_score,
                    match_positions: positions,
                };
            }
        }

        // Path-segment prefix bonus.
        let start = positions[0];
        let starts_segment = start == 0 || (start > 0 && target_chars[start - 1] == '/');
        if starts_segment {
            let last_slash = target_chars.iter().rposition(|&c| c == '/');
            let basename_start = last_slash.map(|i| i + 1).unwrap_or(0);
            if start == basename_start {
                match_score += score::BASENAME_PREFIX;
            } else {
                match_score += score::PATH_SEGMENT_PREFIX;
            }
        }
    }

    FuzzyMatch {
        matched: true,
        score: match_score,
        match_positions: positions,
    }
}

// ---------------------------------------------------------------------------
// Multi-term scorer
// ---------------------------------------------------------------------------

/// Score a multi-term pattern.  Every term must match; scores are summed.
fn score_multi_term(
    terms: &[PreparedTerm],
    target_chars: &[char],
    target_lower: &[char],
) -> FuzzyMatch {
    let mut total_score: i32 = 0;
    let mut all_positions = Vec::new();

    for term in terms {
        let result = score_term(term, target_chars, target_lower);
        if !result.matched {
            return FuzzyMatch::no_match();
        }
        total_score += result.score;
        all_positions.extend(result.match_positions);
    }

    // Sort and deduplicate positions (overlapping terms).
    all_positions.sort_unstable();
    all_positions.dedup();

    FuzzyMatch {
        matched: true,
        score: total_score,
        match_positions: all_positions,
    }
}

// ---------------------------------------------------------------------------
// Convenience free functions (for one-shot use)
// ---------------------------------------------------------------------------

/// One-shot fuzzy match.  Builds a matcher internally.
///
/// For repeated matching, prefer [`FuzzyMatcher`].
pub fn fuzzy_match(query: &str, target: &str) -> FuzzyMatch {
    let mut matcher = FuzzyMatcher::new(query);
    matcher.match_target(target)
}

/// Filter and rank a list of items by fuzzy relevance.
///
/// Returns `Vec<(index, FuzzyMatch)>` sorted by score descending (best first).
/// Non-matching items are excluded.
pub fn fuzzy_filter<T, F>(query: &str, items: &[T], get_text: F) -> Vec<(usize, FuzzyMatch)>
where
    F: Fn(&T) -> &str,
{
    let mut matcher = FuzzyMatcher::new(query);
    let mut results: Vec<(usize, FuzzyMatch)> = items
        .iter()
        .enumerate()
        .map(|(idx, item)| (idx, matcher.match_target(get_text(item))))
        .filter(|(_, m)| m.matched)
        .collect();

    results.sort_by_key(|(_, m)| std::cmp::Reverse(m.score));
    results
}

// ---------------------------------------------------------------------------
// Convenience: score a file path (match against the relative path)
// ---------------------------------------------------------------------------

/// Result of matching a file path.
#[derive(Debug, Clone)]
pub struct FuzzyFileResult {
    /// Path relative to the project root (used for matching/display).
    pub relative_path: String,
    /// Absolute (or original) file path.
    pub full_path: PathBuf,
    /// Relevance score.
    pub score: i32,
    /// Character indices of matched positions in `relative_path`.
    pub match_positions: Vec<usize>,
}

/// Score all files in `entries` against `query` and return ranked results.
///
/// `entries` is a slice of `(relative_path, full_path)` pairs.
pub fn score_files(
    query: &str,
    entries: &[(String, PathBuf)],
    limit: usize,
) -> Vec<FuzzyFileResult> {
    if query.is_empty() || entries.is_empty() {
        return Vec::new();
    }

    let mut matcher = FuzzyMatcher::new(query);
    let mut results: Vec<FuzzyFileResult> = entries
        .iter()
        .filter_map(|(rel, full)| {
            let m = matcher.match_target(rel);
            if m.matched {
                Some(FuzzyFileResult {
                    relative_path: rel.to_string(),
                    full_path: full.clone(),
                    score: m.score,
                    match_positions: m.match_positions,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by score descending.
    results.sort_by_key(|r| std::cmp::Reverse(r.score));
    results.truncate(limit);
    results
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -- Empty / trivial ----------------------------------------------------

    #[test]
    fn empty_query_matches_everything() {
        let m = fuzzy_match("", "anything");
        assert!(m.matched);
    }

    #[test]
    fn empty_target_no_match() {
        let m = fuzzy_match("a", "");
        assert!(!m.matched);
    }

    // -- Basic behaviour ----------------------------------------------------

    #[test]
    fn exact_match() {
        let m = fuzzy_match("save", "save");
        assert!(m.matched);
        assert!(m.score > 0);
    }

    #[test]
    fn case_insensitive() {
        let m = fuzzy_match("SAVE", "save file");
        assert!(m.matched);
        let m = fuzzy_match("save", "SAVE FILE");
        assert!(m.matched);
    }

    #[test]
    fn substring_match() {
        let m = fuzzy_match("file", "Save File");
        assert!(m.matched);
    }

    #[test]
    fn sparse_match() {
        let m = fuzzy_match("sf", "Save File");
        assert!(m.matched);
        assert_eq!(m.match_positions.len(), 2);
    }

    #[test]
    fn no_match() {
        let m = fuzzy_match("xyz", "Save File");
        assert!(!m.matched);
    }

    #[test]
    fn query_longer_than_target() {
        let m = fuzzy_match("verylongquery", "short");
        assert!(!m.matched);
    }

    #[test]
    fn out_of_order_no_match() {
        // Characters must appear in order.
        let m = fuzzy_match("fs", "Save File");
        assert!(!m.matched);
    }

    // -- Ranking / scoring --------------------------------------------------

    #[test]
    fn consecutive_scores_higher_than_sparse() {
        let c = fuzzy_match("ab", "xabc");
        let s = fuzzy_match("ab", "xaxb");
        assert!(c.matched && s.matched);
        assert!(c.score > s.score, "consecutive {} > sparse {}", c.score, s.score);
    }

    #[test]
    fn word_boundary_scores_higher() {
        let b = fuzzy_match("sf", "Save File");
        let m = fuzzy_match("af", "Save File");
        assert!(b.matched && m.matched);
        assert!(b.score > m.score, "boundary {} > middle {}", b.score, m.score);
    }

    #[test]
    fn start_of_string_scores_higher() {
        let s = fuzzy_match("s", "Save File");
        let m = fuzzy_match("a", "Save File");
        assert!(s.matched && m.matched);
        assert!(s.score > m.score, "start {} > middle {}", s.score, m.score);
    }

    #[test]
    fn camel_case_boundary() {
        let m = fuzzy_match("sf", "saveFile");
        assert!(m.matched);
        assert!(m.score > 0);
    }

    #[test]
    fn exact_match_scores_highest() {
        let exact = fuzzy_match("main", "main");
        let longer = fuzzy_match("main", "main.rs");
        assert!(exact.matched && longer.matched);
        assert!(exact.score > longer.score, "exact {} > longer {}", exact.score, longer.score);
    }

    // -- Path-segment prefix ------------------------------------------------

    #[test]
    fn basename_prefix_beats_intra_segment() {
        // "ts" should rank tsconfig.json above pkg.ts
        let prefix = fuzzy_match("ts", "crates/tsconfig.json");
        let intra = fuzzy_match("ts", "crates/pkg.ts");
        assert!(prefix.matched && intra.matched);
        assert!(prefix.score > intra.score,
            "tsconfig.json ({}) > pkg.ts ({})", prefix.score, intra.score);
    }

    #[test]
    fn directory_segment_prefix_beats_mid_segment() {
        let dir = fuzzy_match("ts", "crates/ts-parser/src/lib.rs");
        let intra = fuzzy_match("ts", "crates/pkg.ts");
        assert!(dir.matched && intra.matched);
        assert!(dir.score > intra.score,
            "ts-parser ({}) > pkg.ts ({})", dir.score, intra.score);
    }

    #[test]
    fn basename_outranks_directory_prefix() {
        let base = fuzzy_match("ts", "crates/tsconfig.json");
        let dir = fuzzy_match("ts", "crates/ts-parser/src/lib.rs");
        assert!(base.matched && dir.matched);
        assert!(base.score > dir.score,
            "basename tsconfig.json ({}) > dir ts-parser ({})", base.score, dir.score);
    }

    // -- Multi-term ---------------------------------------------------------

    #[test]
    fn multi_term_all_must_match() {
        let m = fuzzy_match("src main rs", "src/main.rs");
        assert!(m.matched);
        let m = fuzzy_match("src xyz", "src/main.rs");
        assert!(!m.matched);
    }

    #[test]
    fn multi_term_scores_combined() {
        let m = fuzzy_match("save file", "Save File");
        assert!(m.matched);
        assert!(m.score > 0);
    }

    #[test]
    fn leading_trailing_spaces_ignored() {
        let m = fuzzy_match("  save  ", "Save File");
        assert!(m.matched);
    }

    #[test]
    fn multiple_spaces_between_terms() {
        let m = fuzzy_match("save   file", "Save File");
        assert!(m.matched);
    }

    // -- Real-world file paths ----------------------------------------------

    #[test]
    fn real_world_file_patterns() {
        assert!(fuzzy_match("main", "src/main.rs").matched);
        assert!(fuzzy_match("mod", "src/input/mod.rs").matched);
        assert!(fuzzy_match("fuzz", "src/app/fuzzy.rs").matched);
        assert!(fuzzy_match("app app", "src/app/app.rs").matched);
    }

    // -- FuzzyMatcher reuse -------------------------------------------------

    #[test]
    fn matcher_reuse_produces_same_results() {
        let mut matcher = FuzzyMatcher::new("main");
        let targets = [
            "src/main.rs",
            "src/app/main.rs",
            "README.md",
            "no_match_here",
        ];
        for &t in &targets {
            let oneshot = fuzzy_match("main", t);
            let reused = matcher.match_target(t);
            assert_eq!(oneshot, reused, "mismatch for target {:?}", t);
        }
    }

    // -- score_files helper -------------------------------------------------

    #[test]
    fn score_files_returns_ranked_results() {
        let entries = vec![
            ("src/main.rs".to_string(), PathBuf::from("src/main.rs")),
            ("README.md".to_string(), PathBuf::from("README.md")),
            ("src/app/fuzzy.rs".to_string(), PathBuf::from("src/app/fuzzy.rs")),
        ];
        let results = score_files("main", &entries, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relative_path, "src/main.rs");
    }

    #[test]
    fn score_files_respects_limit() {
        let entries = vec![
            ("src/main.rs".to_string(), PathBuf::from("src/main.rs")),
            ("src/app.rs".to_string(), PathBuf::from("src/app.rs")),
        ];
        let results = score_files("src", &entries, 1);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn score_files_empty_query_returns_empty() {
        let entries = vec![("a".to_string(), PathBuf::from("a"))];
        assert!(score_files("", &entries, 10).is_empty());
    }
}
