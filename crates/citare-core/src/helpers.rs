//! Helper functions for the citation finding pipeline.
//!
//! Ported from eyecite's `helpers.py`. Provides token scanning, metadata
//! extraction, court lookup, overlap filtering, and case name extraction.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use fancy_regex::Regex as FancyRegex;
use log::warn;

use crate::models::citation::{
    Citation, CitationCommon, FullCaseCitationData, FullJournalCitationData, FullLawCitationData,
    ShortCaseCitationData,
};
use crate::models::document::Document;
use crate::models::token::{Token, TokenOrStr, Tokens};
use crate::regexes::{
    POST_FULL_CITATION_GROUPS, POST_FULL_CITATION_REGEX, POST_JOURNAL_CITATION_REGEX,
    POST_LAW_CITATION_REGEX, POST_SHORT_CITATION_REGEX, PRE_FULL_CITATION_REGEX, STOP_WORD_REGEX,
    YEAR_REGEX,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Median case name length in the CL db.
const BACKWARD_SEEK: usize = 28;

/// Maximum characters to scan using `match_on_tokens`.
const MAX_MATCH_CHARS: usize = 300;

/// Highest valid year is current year + 1.
const HIGHEST_VALID_YEAR: i32 = 2027;

// ---------------------------------------------------------------------------
// Regex singletons (avoid compiling in loops)
// ---------------------------------------------------------------------------

fn re_named_groups() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\(\?P<([^>]+)>").unwrap())
}

fn re_capitalized_word() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\b([A-Z][a-zA-Z0-9]*)\b.*").unwrap())
}

fn re_leading_articles() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^(of|the|an|and)\s+").unwrap())
}

fn re_leading_articles_ci() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)^(of|the|an|and)\b").unwrap())
}

fn re_trailing_number() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\b\d+\b$").unwrap())
}

fn re_year_paren() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^\(\d{4}\)$").unwrap())
}

fn re_v_split() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\s+v\.?\s+").unwrap())
}

fn re_lowercase_word() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"\b[a-z]\w*\b").unwrap())
}

fn re_stop_words_ci() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(&format!("(?i){}", *STOP_WORD_REGEX)).unwrap())
}

fn re_year() -> &'static FancyRegex {
    static RE: OnceLock<FancyRegex> = OnceLock::new();
    RE.get_or_init(|| FancyRegex::new(&YEAR_REGEX).unwrap())
}

fn re_year_only() -> &'static FancyRegex {
    static RE: OnceLock<FancyRegex> = OnceLock::new();
    RE.get_or_init(|| FancyRegex::new(&format!("^(?:{})$", *YEAR_REGEX)).unwrap())
}

fn re_leading_in() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)^In\s+").unwrap())
}

// ---------------------------------------------------------------------------
// Courts data
// ---------------------------------------------------------------------------

/// Generated at build time from courts-db by `build.rs`.
mod generated_courts {
    include!(concat!(env!("OUT_DIR"), "/generated_courts.rs"));
}
use generated_courts::GENERATED_COURT_KEYS;

/// Lowercase alphanumeric form of a court citation string, used to match
/// citation parentheticals against courts-db. `build.rs` applies the same
/// reduction to every court's `citation_string`.
fn normalize_court_string(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

// ---------------------------------------------------------------------------
// Anchored pattern cache
// ---------------------------------------------------------------------------

/// A `match_on_tokens` pattern compiled once: the anchored regex plus the
/// named groups found in the unanchored source.
struct AnchoredPattern {
    re: FancyRegex,
    names: Vec<String>,
}

/// Cache of compiled anchored patterns, keyed by `(pattern, forward)`.
///
/// Compiling the metadata regexes dominated per-call cost, and every
/// in-crate caller passes one of a handful of static patterns.
type PatternCache = HashMap<(String, bool), Option<Arc<AnchoredPattern>>>;
static ANCHORED_PATTERNS: LazyLock<RwLock<PatternCache>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Upper bound on cached patterns, so callers passing many distinct
/// patterns can't grow the cache without limit.
const MAX_CACHED_PATTERNS: usize = 64;

fn anchored_pattern(regex_str: &str, forward: bool) -> Option<Arc<AnchoredPattern>> {
    let key = (regex_str.to_string(), forward);
    if let Some(hit) = ANCHORED_PATTERNS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(&key)
    {
        return hit.clone();
    }

    let anchored = if forward {
        format!("^(?:{})", regex_str)
    } else {
        format!("(?:{})$", regex_str)
    };
    // fancy_regex doesn't expose group names in source order, so collect
    // every (?P<name>...) in the regex string instead.
    let compiled = FancyRegex::new(&anchored).ok().map(|re| {
        let names = re_named_groups()
            .captures_iter(regex_str)
            .map(|cap| cap[1].to_string())
            .collect();
        Arc::new(AnchoredPattern { re, names })
    });

    let mut cache = ANCHORED_PATTERNS.write().unwrap_or_else(|e| e.into_inner());
    if cache.len() < MAX_CACHED_PATTERNS {
        cache.insert(key, compiled.clone());
    }
    compiled
}

// ---------------------------------------------------------------------------
// MatchResult
// ---------------------------------------------------------------------------

/// Result of a `match_on_tokens` call.
///
/// Provides access to named capture groups and the matched span within the
/// built text buffer.
pub struct MatchResult {
    /// Named capture groups (logical name -> matched text).
    pub groups: HashMap<String, Option<String>>,
    /// Start offset within the built text.
    pub start: usize,
    /// End offset within the built text.
    pub end: usize,
}

impl MatchResult {
    /// Get a named group value, returning `None` if the group doesn't exist
    /// or wasn't matched.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.groups.get(name).and_then(|v| v.as_deref())
    }
}

// ---------------------------------------------------------------------------
// token_to_str
// ---------------------------------------------------------------------------

/// Convert a `TokenOrStr` to its string representation.
fn token_to_str(token: &TokenOrStr) -> String {
    match token {
        TokenOrStr::Str(s) => s.clone(),
        TokenOrStr::Token(t) => t.data().to_string(),
    }
}

// ---------------------------------------------------------------------------
// match_on_tokens
// ---------------------------------------------------------------------------

/// Where to cut text scanned by [`match_on_tokens`] once it reaches
/// `MAX_MATCH_CHARS`, or `None` while it is shorter.
///
/// eyecite caps the text at that many code points (`len(text)`,
/// `text[:MAX_MATCH_CHARS]` forward, `text[-MAX_MATCH_CHARS:]` backward), so
/// the cut is the byte offset of a character boundary: the end of the text
/// to keep when scanning forward, the start of it when scanning backward.
fn match_cap(text: &str, forward: bool) -> Option<usize> {
    // A string has at least as many bytes as characters.
    if text.len() < MAX_MATCH_CHARS {
        return None;
    }
    if forward {
        match text.char_indices().nth(MAX_MATCH_CHARS) {
            Some((cut, _)) => Some(cut),
            None => (text.chars().count() == MAX_MATCH_CHARS).then_some(text.len()),
        }
    } else {
        text.char_indices()
            .rev()
            .nth(MAX_MATCH_CHARS - 1)
            .map(|(cut, _)| cut)
    }
}

/// Scan forward or backward from `start_index`, building a text buffer from
/// consecutive tokens, then match `regex_str` against the buffer.
///
/// - If `forward` is true, anchors the regex at the start (`^`).
/// - If `forward` is false, anchors at the end (`$`).
/// - `prefix` is prepended (forward) or appended (backward) before scanning.
/// - `strings_only`: stop at any non-string token (except paragraph tokens,
///   which always stop scanning).
pub fn match_on_tokens(
    words: &Tokens,
    start_index: i64,
    regex_str: &str,
    prefix: &str,
    strings_only: bool,
    forward: bool,
) -> Option<MatchResult> {
    let mut text = prefix.to_string();

    if forward {
        let start = (start_index.max(0) as usize).min(words.len());
        for token in &words[start..] {
            // Stop at paragraph tokens
            if let TokenOrStr::Token(Token::Paragraph(_)) = token {
                break;
            }
            // Stop at non-string tokens if strings_only
            if strings_only {
                if let TokenOrStr::Token(_) = token {
                    break;
                }
            }
            text.push_str(&token_to_str(token));
            if let Some(cut) = match_cap(&text, true) {
                text.truncate(cut);
                break;
            }
        }
    } else {
        let start = if start_index >= 0 {
            (start_index as usize).min(words.len().saturating_sub(1))
        } else {
            return None;
        };
        for i in (0..=start).rev() {
            let token = &words[i];
            if let TokenOrStr::Token(Token::Paragraph(_)) = token {
                break;
            }
            if strings_only {
                if let TokenOrStr::Token(_) = token {
                    break;
                }
            }
            text = format!("{}{}", token_to_str(token), text);
            if let Some(cut) = match_cap(&text, false) {
                text.replace_range(..cut, "");
                break;
            }
        }
    }

    // Match using fancy_regex (supports lookahead)
    let pattern = anchored_pattern(regex_str, forward)?;
    let captures = pattern.re.captures(&text).ok()??;

    // Extract all named groups
    let mut groups = HashMap::new();
    for name in &pattern.names {
        let value = captures.name(name).map(|m| m.as_str().to_string());
        groups.insert(name.clone(), value);
    }

    let overall = captures.get(0)?;

    Some(MatchResult {
        groups,
        start: overall.start(),
        end: overall.end(),
    })
}

// ---------------------------------------------------------------------------
// get_year
// ---------------------------------------------------------------------------

/// Parse a year string and return it if within a reasonable range.
pub fn get_year(word: &str) -> Option<i32> {
    let year: i32 = word.parse().ok()?;
    if !(1600..=HIGHEST_VALID_YEAR).contains(&year) {
        return None;
    }
    Some(year)
}

// ---------------------------------------------------------------------------
// get_court_by_paren
// ---------------------------------------------------------------------------

/// Map a citation-parenthetical court string (e.g. "2d Cir") to a court id.
///
/// Does not handle SCOTUS (which lacks parentheticals).
pub fn get_court_by_paren(paren_string: &str) -> Option<String> {
    let court_str = normalize_court_string(paren_string);

    if court_str.is_empty() {
        return None;
    }

    let mut fallback: Option<&str> = None;
    for &(s, id) in GENERATED_COURT_KEYS {
        if s == court_str {
            return Some(id.to_string());
        }
        if s.starts_with(&court_str) {
            fallback = Some(id);
        }
    }
    fallback.map(str::to_string)
}

// ---------------------------------------------------------------------------
// clean_pin_cite
// ---------------------------------------------------------------------------

/// Strip leading/trailing commas and spaces from a pin cite.
pub fn clean_pin_cite(pin_cite: Option<&str>) -> Option<String> {
    pin_cite
        .map(|s| s.trim_matches(&[',', ' '][..]).to_string())
        .filter(|s| !s.is_empty())
}

// ---------------------------------------------------------------------------
// process_parenthetical
// ---------------------------------------------------------------------------

/// Process a matched parenthetical, trimming nested unbalanced parens
/// and filtering year-only parentheticals.
pub fn process_parenthetical(paren: Option<&str>) -> Option<String> {
    let paren = paren?;
    if paren.is_empty() {
        return None;
    }

    let mut balance: i32 = 0;
    for (i, ch) in paren.char_indices() {
        match ch {
            '(' => balance += 1,
            ')' => balance -= 1,
            _ => {}
        }
        if balance < 0 {
            let trimmed = &paren[..i];
            if trimmed.is_empty() {
                return None;
            }
            return Some(trimmed.to_string());
        }
    }

    // Filter year-only parentheticals
    if re_year().is_match(paren).unwrap_or(false) {
        // Check if the whole string is just a year
        if re_year_only().is_match(paren).unwrap_or(false) {
            return None;
        }
    }

    if paren.is_empty() {
        None
    } else {
        Some(paren.to_string())
    }
}

// ---------------------------------------------------------------------------
// extract_pin_cite
// ---------------------------------------------------------------------------

/// Extract a pin cite from the tokens following a citation token.
///
/// Returns `(pin_cite, span_end, parenthetical)`.
pub fn extract_pin_cite(
    words: &Tokens,
    index: usize,
    prefix: &str,
) -> (Option<String>, Option<usize>, Option<String>) {
    // Get the end position of the token at `index`
    let from_token_end = match &words[index] {
        TokenOrStr::Token(t) => t.end(),
        TokenOrStr::Str(s) => {
            // This shouldn't normally happen for citation tokens
            // but handle it gracefully
            warn!("extract_pin_cite called on string token");
            s.len()
        }
    };

    let m = match_on_tokens(
        words,
        (index + 1) as i64,
        &POST_SHORT_CITATION_REGEX,
        prefix,
        true, // strings_only
        true, // forward
    );

    if let Some(m) = m {
        let pin_cite_raw = m.get("pin_cite");
        let pin_cite = clean_pin_cite(pin_cite_raw);
        let extra_chars = if let Some(pc) = pin_cite_raw {
            pc.trim_end_matches(&[',', ' '][..]).len()
        } else {
            0
        };
        let parenthetical = process_parenthetical(m.get("parenthetical"));
        let span_end = from_token_end + extra_chars - prefix.len();
        (pin_cite, Some(span_end), parenthetical)
    } else {
        (None, None, None)
    }
}

// ---------------------------------------------------------------------------
// add_post_citation (for full case citations)
// ---------------------------------------------------------------------------

/// Add metadata to a full case citation from the text following it.
///
/// Parses court, year, pin_cite, parenthetical, etc.
pub fn add_post_citation(citation: &mut FullCaseCitationData, words: &Tokens) {
    let index = citation.resource.common.index;
    let span_end = citation
        .resource
        .common
        .span_end
        .unwrap_or_else(|| citation.resource.common.token.end());

    let m = match_on_tokens(
        words,
        (index + 1) as i64,
        &POST_FULL_CITATION_REGEX,
        "",
        false, // not strings_only
        true,  // forward
    );

    let m = match m {
        Some(m) => m,
        None => return,
    };

    // Merge branch groups for pin_cite, month, year
    // We need to re-run the regex with fancy_regex captures to get branch groups
    // Since match_on_tokens already extracted groups, but we need the branch-merged version
    // We'll reparse the captured groups through POST_FULL_CITATION_GROUPS
    let mut merged = HashMap::new();
    for &(logical_name, branch_names) in POST_FULL_CITATION_GROUPS {
        let value = branch_names
            .iter()
            .filter_map(|name| {
                m.groups
                    .get(*name)
                    .and_then(|v| v.as_deref())
                    .map(|s| s.to_string())
            })
            .next();
        merged.insert(logical_name.to_string(), value);
    }

    // Also include non-branched groups
    for (k, v) in &m.groups {
        if !merged.contains_key(k) {
            merged.insert(k.clone(), v.clone());
        }
    }

    citation.resource.common.full_span_end = Some(span_end + m.end);

    let pin_cite_raw = merged.get("pin_cite").and_then(|v| v.as_deref());
    citation.metadata.case.pin_cite = clean_pin_cite(pin_cite_raw);
    if let Some(pc) = pin_cite_raw {
        citation.metadata.case.pin_cite_span_end = Some(span_end + pc.len());
    }

    let extra = merged
        .get("extra")
        .and_then(|v| v.as_deref())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    citation.metadata.extra = extra;

    let raw_paren = merged.get("parenthetical").and_then(|v| v.as_deref());
    let processed_paren = process_parenthetical(raw_paren);

    // Adjust full_span_end if parenthetical was trimmed
    if let (Some(fse), Some(raw), Some(ref proc)) = (
        citation.resource.common.full_span_end,
        raw_paren,
        &processed_paren,
    ) {
        if raw.len() > proc.len() {
            let offset = raw.len() - proc.len();
            citation.resource.common.full_span_end = Some(fse - offset);
        }
    }
    citation.metadata.case.parenthetical = processed_paren;

    citation.metadata.case.year = merged.get("year").and_then(|v| v.clone());
    citation.metadata.case.month = merged.get("month").and_then(|v| v.clone());
    citation.metadata.case.day = merged.get("day").and_then(|v| v.clone());

    if let Some(year_str) = merged.get("year").and_then(|v| v.as_deref()) {
        citation.resource.year = get_year(year_str);
    }

    if let Some(court_str) = merged.get("court").and_then(|v| v.as_deref()) {
        if !court_str.is_empty() {
            citation.metadata.case.court = get_court_by_paren(court_str);
        }
    }
}

// ---------------------------------------------------------------------------
// add_law_metadata
// ---------------------------------------------------------------------------

/// Add metadata to a full law citation.
pub fn add_law_metadata(citation: &mut FullLawCitationData, words: &Tokens) {
    let index = citation.resource.common.index;
    let span_end = citation
        .resource
        .common
        .span_end
        .unwrap_or_else(|| citation.resource.common.token.end());

    let m = match_on_tokens(
        words,
        (index + 1) as i64,
        &POST_LAW_CITATION_REGEX,
        "",
        true, // strings_only
        true, // forward
    );

    let m = match m {
        Some(m) => m,
        None => return,
    };

    citation.resource.common.full_span_end = Some(span_end + m.end);
    citation.metadata.pin_cite = clean_pin_cite(m.get("pin_cite"));
    citation.metadata.publisher = m.get("publisher").map(|s| s.to_string());
    citation.metadata.day = m.get("day").map(|s| s.to_string());
    citation.metadata.month = m.get("month").map(|s| s.to_string());
    citation.metadata.parenthetical = process_parenthetical(m.get("parenthetical"));
    citation.metadata.year = m.get("year").map(|s| s.to_string());

    if let Some(year_str) = m.get("year") {
        citation.resource.year = get_year(year_str);
    }
}

// ---------------------------------------------------------------------------
// add_journal_metadata
// ---------------------------------------------------------------------------

/// Add metadata to a full journal citation.
pub fn add_journal_metadata(citation: &mut FullJournalCitationData, words: &Tokens) {
    let index = citation.resource.common.index;
    let span_end = citation
        .resource
        .common
        .span_end
        .unwrap_or_else(|| citation.resource.common.token.end());

    let m = match_on_tokens(
        words,
        (index + 1) as i64,
        &POST_JOURNAL_CITATION_REGEX,
        "",
        true, // strings_only
        true, // forward
    );

    let m = match m {
        Some(m) => m,
        None => return,
    };

    citation.resource.common.full_span_end = Some(span_end + m.end);
    citation.metadata.pin_cite = clean_pin_cite(m.get("pin_cite"));
    citation.metadata.parenthetical = process_parenthetical(m.get("parenthetical"));
    citation.metadata.year = m.get("year").map(|s| s.to_string());

    if let Some(year_str) = m.get("year") {
        citation.resource.year = get_year(year_str);
    }
}

// ---------------------------------------------------------------------------
// add_pre_citation
// ---------------------------------------------------------------------------

/// Scan backwards to find a (PartyName - Pincite) component before a full case
/// citation. Only runs if plaintiff/defendant haven't already been found.
pub fn add_pre_citation(citation: &mut FullCaseCitationData, words: &Tokens) {
    if citation.metadata.plaintiff.is_some() || citation.metadata.defendant.is_some() {
        return;
    }

    let index = citation.resource.common.index;
    let span_start = citation
        .resource
        .common
        .span_start
        .unwrap_or_else(|| citation.resource.common.token.start());

    let m = match_on_tokens(
        words,
        index as i64 - 1,
        &PRE_FULL_CITATION_REGEX,
        "",
        true,  // strings_only
        false, // backward
    );

    let m = match m {
        Some(m) => m,
        None => return,
    };

    if m.get("pin_cite").is_some() {
        let match_len = m.end - m.start;
        citation.metadata.case.pin_cite_span_start = Some(span_start.saturating_sub(match_len));
    }

    citation.metadata.case.pin_cite = clean_pin_cite(m.get("pin_cite"));
    citation.metadata.antecedent_guess = m.get("antecedent").map(|s| s.to_string());

    let match_length = m.end - m.start;
    citation.resource.common.full_span_start = Some(span_start.saturating_sub(match_length));
}

// ---------------------------------------------------------------------------
// find_case_name (backward scan for plaintiff/defendant)
// ---------------------------------------------------------------------------

/// Find the case name by scanning backward from the citation token.
///
/// Updates `citation.metadata.plaintiff`, `citation.metadata.defendant`,
/// and `citation.full_span_start`.
pub fn find_case_name(citation: &mut FullCaseCitationData, words: &Tokens, short: bool) {
    let mut state = initialize_search_state(citation);
    scan_for_case_boundaries(words, citation, &mut state);

    // eyecite only processes a truthy (non-empty) candidate.
    if state
        .candidate_case_name
        .as_deref()
        .is_some_and(|c| !c.is_empty())
    {
        process_case_name(citation, words, &state, short);
    }
}

/// Find case name for a short case citation.
pub fn find_case_name_short(citation: &mut ShortCaseCitationData, words: &Tokens) {
    let index = citation.resource.common.index;
    let span_start = citation
        .resource
        .common
        .span_start
        .unwrap_or_else(|| citation.resource.common.token.start());
    let back_seek = index.saturating_sub(BACKWARD_SEEK);

    let mut state = SearchState {
        offset: 0,
        v_token_found: false,
        start_index: None,
        candidate_case_name: None,
        pre_cite_year: None,
        title_starting_index: if index > 0 { index - 1 } else { 0 },
        case_name_length: 0,
        plaintiff_length: 0,
    };

    // Scan backward
    if index > 0 {
        let scan_start = index - 1;
        for i in (back_seek..=scan_start).rev() {
            let word = &words[i];
            let word_str = token_to_str(word);
            state.offset += word_str.len();

            if word_str == "," {
                continue;
            }
            state.case_name_length += 1;

            if state.v_token_found && !word_str.trim().is_empty() {
                state.plaintiff_length += 1;
            }

            // Handle citation tokens
            if let TokenOrStr::Token(Token::Citation(_)) = word {
                if i > 0 {
                    state.title_starting_index = i - 1;
                }
                continue;
            }

            // Terminal punctuation
            if word_str.ends_with(';') || word_str.ends_with('\u{201D}') || word_str.ends_with('"')
            {
                state.start_index = Some(i + 2);
                state.candidate_case_name =
                    Some(extract_text(words, i + 2, state.title_starting_index));
                break;
            }

            // "v" stop word
            if is_v_token(word) {
                state.v_token_found = true;
                state.start_index = Some(v_token_start_index(words, i));
                state.candidate_case_name = Some(extract_text(
                    words,
                    state.start_index.unwrap(),
                    state.title_starting_index,
                ));
                continue;
            }

            // Stop word (non-v)
            if let TokenOrStr::Token(Token::StopWord(_)) = word {
                state.start_index = Some(i + 2);
                state.candidate_case_name =
                    Some(extract_text(words, i + 2, state.title_starting_index));
                break;
            }

            // Lowercase without v token
            if !state.v_token_found
                && !word_str.is_empty()
                && word_str
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_lowercase() && c.is_alphabetic())
                && !["of", "the", "an", "and"].contains(&word_str.as_str())
            {
                if let TokenOrStr::Token(Token::Supra(_)) = word {
                    if i > 0 {
                        state.title_starting_index = i - 1;
                    }
                    continue;
                }
                state.start_index = Some(i + 2);
                let text = extract_text(words, i + 2, state.title_starting_index);
                // Try to find a capitalized word
                let cap_re = re_capitalized_word();
                if let Some(m) = cap_re.find(&text) {
                    state.candidate_case_name = Some(text[m.start()..].to_string());
                } else {
                    state.candidate_case_name = None;
                }
                break;
            }

            // Lowercase after v token
            if state.v_token_found
                && !word_str.is_empty()
                && word_str.chars().next().is_some_and(|c| !c.is_uppercase())
                && !word_str.trim().is_empty()
                && !["of", "the", "an", "and"].contains(&word_str.as_str())
            {
                state.start_index = Some(i + 2);
                let mut text = extract_text(words, i + 2, state.title_starting_index);
                let prefix_re = re_leading_articles();
                text = prefix_re.replace(&text, "").to_string();
                state.candidate_case_name = Some(text);
                break;
            }

            if i == 0 {
                state.candidate_case_name =
                    Some(extract_text(words, 0, state.title_starting_index));
                state.start_index = Some(0);
            }
        }
    }

    // Process antecedent_guess
    if let Some(ref candidate) = state.candidate_case_name {
        let clean = strip_stop_words(candidate);
        if !clean.is_empty() {
            citation.metadata.antecedent_guess = Some(clean);
            if let Some(start_idx) = state.start_index {
                let offset = case_name_offset(words, start_idx, index);
                let span_start_val = span_start.saturating_sub(offset);
                citation.resource.common.full_span_start = Some(span_start_val);
            }
        }
    }

    if let Some(ref year) = state.pre_cite_year {
        citation.metadata.case.year = Some(year.clone());
        if let Ok(y) = year.parse::<i32>() {
            citation.resource.year = Some(y);
        }
    }
}

struct SearchState {
    offset: usize,
    v_token_found: bool,
    start_index: Option<usize>,
    candidate_case_name: Option<String>,
    pre_cite_year: Option<String>,
    title_starting_index: usize,
    case_name_length: usize,
    plaintiff_length: usize,
}

fn initialize_search_state(citation: &FullCaseCitationData) -> SearchState {
    let index = citation.resource.common.index;
    SearchState {
        offset: 0,
        v_token_found: false,
        start_index: None,
        candidate_case_name: None,
        pre_cite_year: None,
        title_starting_index: if index > 0 { index - 1 } else { 0 },
        case_name_length: 0,
        plaintiff_length: 0,
    }
}

fn scan_for_case_boundaries(
    words: &Tokens,
    citation: &FullCaseCitationData,
    state: &mut SearchState,
) {
    let index = citation.resource.common.index;
    if index == 0 {
        return;
    }
    let back_seek = index.saturating_sub(BACKWARD_SEEK);
    let year_paren_re = re_year_paren();

    for i in (back_seek..index).rev() {
        let word = &words[i];
        let word_str = token_to_str(word);
        state.offset += word_str.len();

        if word_str == "," {
            continue;
        }
        state.case_name_length += 1;

        if state.v_token_found && !word_str.trim().is_empty() {
            state.plaintiff_length += 1;
        }

        // Handle citation tokens
        if let TokenOrStr::Token(Token::Citation(_)) = word {
            if i > 0 {
                state.title_starting_index = i - 1;
            }
            continue;
        }

        // Terminal punctuation
        if word_str.ends_with(';') || word_str.ends_with('\u{201D}') || word_str.ends_with('"') {
            state.start_index = Some(i + 2);
            state.candidate_case_name =
                Some(extract_text(words, i + 2, state.title_starting_index));
            return;
        }

        // Year before citation e.g. (2004)
        if year_paren_re.is_match(&word_str) {
            if i > 0 {
                state.title_starting_index = i - 1;
            }
            state.pre_cite_year = Some(word_str[1..5].to_string());
            continue;
        }

        // Opening paren after sufficient words
        if word_str.starts_with('(') && state.case_name_length > 3 {
            let start = if word_str == "("
                || word_str
                    .chars()
                    .nth(1)
                    .is_some_and(|c| c.is_alphabetic() && c.is_lowercase())
            {
                i + 2
            } else {
                i
            };
            state.start_index = Some(start);
            state.candidate_case_name =
                Some(extract_text(words, start, state.title_starting_index));
            return;
        }

        // Lowercase after v token
        if state.v_token_found
            && !word_str.is_empty()
            && word_str.chars().next().is_some_and(|c| !c.is_uppercase())
            && !word_str.trim().is_empty()
            && !["of", "the", "an", "and"].contains(&word_str.as_str())
        {
            state.start_index = Some(i + 2);
            let text = extract_text(words, i + 2, state.title_starting_index);
            state.candidate_case_name = Some(re_leading_articles().replace(&text, "").to_string());
            return;
        }

        // Placeholder citations
        if let TokenOrStr::Token(Token::Citation(_) | Token::Placeholder(_)) = word {
            if i > 0 {
                state.title_starting_index = i - 1;
            }
            continue;
        }

        // "v" stop word
        if is_v_token(word) {
            state.v_token_found = true;
            let start = v_token_start_index(words, i);
            state.start_index = Some(start);
            state.candidate_case_name =
                Some(extract_text(words, start, state.title_starting_index));
            continue;
        }

        // Capitalized abbreviation after v token or other stop word
        if (is_capitalized_abbreviation(&word_str, state.v_token_found, state.plaintiff_length))
            || matches!(word, TokenOrStr::Token(Token::StopWord(_)))
        {
            state.start_index = Some(i + 2);
            state.candidate_case_name =
                Some(extract_text(words, i + 2, state.title_starting_index));
            return;
        }

        // Lowercase without v token
        if !state.v_token_found
            && !word_str.is_empty()
            && word_str
                .chars()
                .next()
                .is_some_and(|c| !c.is_uppercase() && c.is_alphabetic())
            && !word_str.trim().is_empty()
            && !["of", "the", "an", "and", "ex", "rel."].contains(&word_str.as_str())
        {
            if let TokenOrStr::Token(Token::Supra(_)) = word {
                if i > 0 {
                    state.title_starting_index = i - 1;
                }
                continue;
            }
            state.start_index = Some(i + 2);
            let text = extract_text(words, i + 2, state.title_starting_index);
            if let Some(m) = re_capitalized_word().find(&text) {
                state.candidate_case_name = Some(text[m.start()..].to_string());
            } else {
                state.candidate_case_name = None;
            }
            return;
        }

        // Start of text
        if i == 0 {
            let mut text = extract_text(words, 0, state.title_starting_index);
            let prefix_re = re_leading_articles_ci();
            text = prefix_re.replace(&text, "").to_string();

            // Drop if ends in numbers
            let trailing_num_re = re_trailing_number();
            if trailing_num_re.is_match(&text) {
                state.candidate_case_name = None;
            } else {
                state.candidate_case_name = Some(text);
            }
            state.start_index = Some(0);
        }
    }
}

fn process_case_name(
    citation: &mut FullCaseCitationData,
    words: &Tokens,
    state: &SearchState,
    short: bool,
) {
    let candidate = match &state.candidate_case_name {
        Some(c) => c.clone(),
        None => return,
    };

    let index = citation.resource.common.index;
    let span_start = citation
        .resource
        .common
        .span_start
        .unwrap_or_else(|| citation.resource.common.token.start());

    if state.v_token_found {
        let v_re = re_v_split();
        let splits: Vec<&str> = v_re.splitn(&candidate, 2).collect();
        let (plaintiff, defendant) = if splits.len() == 2 {
            (splits[0].to_string(), splits[1].to_string())
        } else {
            (String::new(), splits[0].to_string())
        };

        let plaintiff = plaintiff.trim_matches(&[' ', ',', '('][..]).to_string();
        // Remove lowercase words from plaintiff
        let lc_re = re_lowercase_word();
        let clean_plaintiff = lc_re.replace_all(&plaintiff, "").to_string();
        let plaintiff = strip_stop_words(&clean_plaintiff);
        if !short {
            citation.metadata.plaintiff = Some(plaintiff).filter(|s| !s.is_empty());
        }

        let defendant = strip_stop_words(&defendant);
        if !short {
            citation.metadata.defendant = Some(defendant.clone()).filter(|s| !s.is_empty());
        } else {
            citation.metadata.antecedent_guess =
                Some(strip_stop_words(&candidate)).filter(|s| !s.is_empty());
        }
    } else {
        let defendant = strip_stop_words(&candidate);
        if !short {
            citation.metadata.defendant = Some(defendant.clone()).filter(|s| !s.is_empty());
        } else {
            citation.metadata.antecedent_guess = Some(defendant.clone()).filter(|s| !s.is_empty());
        }
    }

    // Calculate full_span_start
    if let Some(start_idx) = state.start_index {
        let clean = if state.v_token_found || !short {
            state.candidate_case_name.as_deref().unwrap_or("")
        } else {
            ""
        };
        if !clean.is_empty() || state.start_index.is_some() {
            let offset = case_name_offset(words, start_idx, index);
            citation.resource.common.full_span_start = Some(span_start.saturating_sub(offset));
        }
    }

    if let Some(ref year) = state.pre_cite_year {
        citation.metadata.case.year = Some(year.clone());
        if let Ok(y) = year.parse::<i32>() {
            citation.resource.year = Some(y);
        }
    }
}

// ---------------------------------------------------------------------------
// Small helper functions
// ---------------------------------------------------------------------------

/// Start index of the case name when a "v" token sits at word `i`.
///
/// eyecite uses `i - 2` as a Python slice start, so a "v" at the first or
/// second word wraps around to the end of the word list and yields an empty
/// candidate (e.g. `v. Bar, 1 U.S. 1` gets no case name, and `Bar` falls
/// through to `antecedent_guess`). Mirror that wrap-around.
fn v_token_start_index(words: &Tokens, i: usize) -> usize {
    if i >= 2 {
        i - 2
    } else {
        (words.len() + i).saturating_sub(2)
    }
}

fn extract_text(words: &Tokens, start: usize, end: usize) -> String {
    let start = start.min(words.len());
    let end = end.min(words.len());
    if start > end {
        return String::new();
    }
    words[start..end]
        .iter()
        .map(token_to_str)
        .collect::<String>()
}

/// How far, in bytes, a case name starting at word `start` begins before
/// the citation at word `index`.
///
/// eyecite steps back `len("".join(words[start:index - 1])) + 1` code points
/// from the citation: the words before the separator, plus one for the
/// separator, assuming it is a single character. Counted in bytes, that
/// "+ 1" lands inside a no-break space or an em dash glued to the
/// citation, so step back the same number of characters over the text
/// before the citation (`words[..index]`) instead.
fn case_name_offset(words: &Tokens, start: usize, index: usize) -> usize {
    let index = index.min(words.len());
    let end = index.saturating_sub(1);
    let chars: usize = words[start.min(end)..end]
        .iter()
        .map(|w| token_to_str(w).chars().count())
        .sum::<usize>()
        + 1;
    let mut remaining = chars;
    let mut bytes = 0;
    for word in words[..index].iter().rev() {
        let word = token_to_str(word);
        for ch in word.chars().rev() {
            if remaining == 0 {
                return bytes;
            }
            remaining -= 1;
            bytes += ch.len_utf8();
        }
    }
    // Ran past the start of the text; the caller saturates at 0.
    bytes + remaining
}

fn text_len_between(words: &Tokens, start: usize, end: usize) -> usize {
    let start = start.min(words.len());
    let end = end.min(words.len());
    if start > end {
        return 0;
    }
    words[start..end]
        .iter()
        .map(|w| token_to_str(w).len())
        .sum()
}

fn is_v_token(word: &TokenOrStr) -> bool {
    if let TokenOrStr::Token(Token::StopWord(td)) = word {
        td.groups.get("stop_word").and_then(|v| v.as_deref()) == Some("v")
    } else {
        false
    }
}

fn is_capitalized_abbreviation(
    word_str: &str,
    v_token_found: bool,
    plaintiff_length: usize,
) -> bool {
    v_token_found
        && !word_str.is_empty()
        && word_str.chars().next().is_some_and(|c| c.is_uppercase())
        && word_str.len() > 4
        && word_str.ends_with('.')
        && plaintiff_length > 1
}

/// Strip legal stop words from text.
fn strip_stop_words(text: &str) -> String {
    let stop_re = re_stop_words_ci();
    let cleaned = stop_re.replace_all(text, " ").to_string();
    let in_re = re_leading_in();
    let mut text = in_re.replace(&cleaned, "").to_string().trim().to_string();
    text = text
        .trim_start_matches('(')
        .trim_end_matches(')')
        .to_string();
    if text.contains(';') {
        if let Some(part) = text.split(';').nth(1) {
            text = part.to_string();
        }
    }
    let text = text.trim_matches(&[',', ' '][..]).to_string();
    let cleaned = stop_re.replace_all(&text, "").to_string();
    cleaned.trim_matches(&[',', ' '][..]).trim().to_string()
}

// ---------------------------------------------------------------------------
// Markup-aware case names (eyecite `find_case_name_in_html` and friends)
// ---------------------------------------------------------------------------

fn re_versus_split() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)\s+vs?\.?\s+").unwrap())
}

/// `str(word).strip(", ") == ""`
fn is_whitespace_word(word: &TokenOrStr) -> bool {
    token_to_str(word)
        .trim_matches(|c| c == ',' || c == ' ')
        .is_empty()
}

/// Truncate `text` like Python's `text[: -drop]`, staying on a char
/// boundary.
fn drop_last_bytes(text: &str, drop: usize) -> String {
    let mut keep = text.len().saturating_sub(drop);
    while !text.is_char_boundary(keep) {
        keep -= 1;
    }
    text[..keep].to_string()
}

/// The single emphasis tag containing plain-text `position`, if exactly one
/// does (eyecite `find_html_tags_at_position`).
fn find_html_tag_at_position(document: &Document, position: usize) -> Option<(usize, usize)> {
    let to_markup = document.plain_to_markup.as_ref()?;
    let markup_loc = to_markup.update(position, true);
    let mut tags = document
        .emphasis_tags
        .iter()
        .filter(|(_, start, end)| *start <= markup_loc && markup_loc < *end);
    let (_, start, end) = tags.next()?;
    if tags.next().is_some() {
        return None;
    }
    Some((*start, *end))
}

/// Map an emphasis tag's markup span to plain text, returning the plain
/// text it covers and its plain span (eyecite
/// `convert_html_to_plain_text_and_loc`).
fn convert_html_to_plain_text_and_loc(
    document: &Document,
    tag: (usize, usize),
) -> (String, usize, usize) {
    let Some(to_plain) = document.markup_to_plain.as_ref() else {
        return (String::new(), 0, 0);
    };
    let start = to_plain.update(tag.0, true);
    let end = to_plain.update(tag.1, true);
    let text = document
        .plain_text
        .get(start..end)
        .unwrap_or("")
        .to_string();
    (text, start, end)
}

fn common_span(common: &CitationCommon) -> (usize, usize) {
    (
        common.span_start.unwrap_or_else(|| common.token.start()),
        common.span_end.unwrap_or_else(|| common.token.end()),
    )
}

/// Python's `text.strip().strip(",")` plus an optional `.strip("(")`.
fn clean_party(text: &str, strip_paren: bool) -> String {
    let text = text.trim().trim_matches(',');
    if strip_paren {
        text.trim_matches('(').to_string()
    } else {
        text.to_string()
    }
}

/// Set plaintiff and defendant from one or two emphasis tags.
fn set_parties_from_html(
    citation: &mut FullCaseCitationData,
    document: &Document,
    plaintiff_tag: (usize, usize),
    defendant_tag: (usize, usize),
) {
    let (plaintiff, defendant, mut start) = if plaintiff_tag == defendant_tag {
        // Both parties in one element: split on "v." / "vs."
        let (case_name, start, _) = convert_html_to_plain_text_and_loc(document, plaintiff_tag);
        let mut parts = re_versus_split().splitn(&case_name, 2);
        let first = parts.next().unwrap_or("").to_string();
        match parts.next() {
            Some(second) => (first, second.to_string(), start),
            None => (String::new(), case_name.clone(), start),
        }
    } else {
        let (plaintiff, start, _) = convert_html_to_plain_text_and_loc(document, plaintiff_tag);
        let (defendant, _, _) = convert_html_to_plain_text_and_loc(document, defendant_tag);
        (plaintiff, defendant, start)
    };

    let clean_plaintiff = strip_stop_words(&plaintiff);
    citation.metadata.plaintiff = Some(clean_party(&clean_plaintiff, true));
    citation.metadata.defendant = Some(clean_party(&strip_stop_words(&defendant), false));

    // eyecite shifts the start by the number of characters stripping removed
    // (`len(plaintiff) - len(clean_plaintiff)`); shift by as many
    // characters, not bytes, or a trimmed no-break space moves it into the
    // middle of a character.
    let removed = plaintiff
        .chars()
        .count()
        .saturating_sub(clean_plaintiff.chars().count());
    if removed > 0 && start <= document.plain_text.len() {
        start = crate::utils::advance_chars(&document.plain_text, start, removed);
    }
    citation.resource.common.full_span_start = Some(start);
}

/// Extract plaintiff and defendant around the "v." token at `index`.
fn extract_parties_from_versus(
    citation: &mut FullCaseCitationData,
    document: &Document,
    index: usize,
) {
    let words = &document.words;
    let versus_start = match &words[index] {
        TokenOrStr::Token(t) => t.start(),
        TokenOrStr::Str(_) => return,
    };
    // Python's words[index - 2:index] is empty when index < 2.
    let left_shift = if index >= 2 {
        text_len_between(words, index - 2, index)
    } else {
        0
    };
    let right_shift = text_len_between(words, index, index + 2);
    let Some(plaintiff_pos) = versus_start.checked_sub(left_shift) else {
        return;
    };
    let defendant_pos = versus_start + right_shift;

    let (Some(plaintiff_tag), Some(defendant_tag)) = (
        find_html_tag_at_position(document, plaintiff_pos),
        find_html_tag_at_position(document, defendant_pos),
    ) else {
        return;
    };
    set_parties_from_html(citation, document, plaintiff_tag, defendant_tag);
}

/// Extract the defendant after a non-"v." stop word at `index` (e.g.
/// "In re <i>Foo</i>").
fn extract_defendant_after_stopword(
    citation: &mut FullCaseCitationData,
    document: &Document,
    index: usize,
) {
    let words = &document.words;
    let word_start = match &words[index] {
        TokenOrStr::Token(t) => t.start(),
        TokenOrStr::Str(_) => return,
    };
    let mut shift = 3;
    while index + shift < words.len()
        && matches!(&words[index + shift], TokenOrStr::Str(s) if s == " ")
    {
        shift += 1;
    }
    let right_offset = text_len_between(words, index, index + shift);
    let Some(loc) = (word_start + right_offset).checked_sub(1) else {
        return;
    };

    let Some(tag) = find_html_tag_at_position(document, loc) else {
        return;
    };
    let (mut defendant, start, end) = convert_html_to_plain_text_and_loc(document, tag);
    let (cite_start, _) = common_span(&citation.resource.common);
    if end > cite_start {
        defendant = drop_last_bytes(&defendant, end - cite_start);
    }
    citation.metadata.defendant = Some(
        strip_stop_words(&defendant)
            .trim_matches(|c| c == ',' || c == ' ')
            .to_string(),
    );
    citation.resource.common.full_span_start = Some(start);
}

/// Find plaintiff/defendant of a full case citation from emphasis tags in
/// the document's markup (eyecite `find_case_name_in_html`).
///
/// Does nothing unless the document was built from markup.
pub fn find_case_name_in_html(citation: &mut FullCaseCitationData, document: &Document) {
    if document.plain_to_markup.is_none() {
        return;
    }
    let words = &document.words;
    let index = citation.resource.common.index;
    let back_seek = index.saturating_sub(BACKWARD_SEEK);
    for i in (back_seek..index).rev() {
        let word = &words[i];
        if is_whitespace_word(word) {
            continue;
        }
        if is_v_token(word) {
            extract_parties_from_versus(citation, document, i);
            return;
        }
        if let TokenOrStr::Token(Token::StopWord(_)) = word {
            extract_defendant_after_stopword(citation, document, i);
            return;
        }
    }
}

/// Find the antecedent of a short case citation from emphasis tags in the
/// document's markup (eyecite `find_case_name_in_html(short=True)`).
///
/// Does nothing unless the document was built from markup.
pub fn find_case_name_in_html_short(citation: &mut ShortCaseCitationData, document: &Document) {
    if document.plain_to_markup.is_none() {
        return;
    }
    let words = &document.words;
    let index = citation.resource.common.index;
    let back_seek = index.saturating_sub(BACKWARD_SEEK);
    // The first non-blank word before the citation decides.
    let Some(i) = (back_seek..index)
        .rev()
        .find(|&i| !is_whitespace_word(&words[i]))
    else {
        return;
    };

    let offset = text_len_between(words, i, index);
    let Some(loc) = citation.resource.common.token.start().checked_sub(offset) else {
        return;
    };
    let Some(tag) = find_html_tag_at_position(document, loc) else {
        return;
    };
    let (mut antecedent, start, end) = convert_html_to_plain_text_and_loc(document, tag);
    let (cite_start, _) = common_span(&citation.resource.common);
    if end > cite_start {
        antecedent = drop_last_bytes(&antecedent, end - cite_start);
    }
    citation.metadata.antecedent_guess = Some(strip_stop_words(&antecedent));
    citation.resource.common.full_span_start = Some(start);
}

// ---------------------------------------------------------------------------
// guess_edition
// ---------------------------------------------------------------------------

/// Set `edition_guess` on a resource citation by narrowing down editions
/// using the year if available.
pub fn guess_edition(
    exact_editions: &[crate::models::edition::Edition],
    variation_editions: &[crate::models::edition::Edition],
    year: Option<i32>,
) -> Option<crate::models::edition::Edition> {
    let editions = if !exact_editions.is_empty() {
        exact_editions
    } else if !variation_editions.is_empty() {
        variation_editions
    } else {
        return None;
    };

    let mut candidates: Vec<&crate::models::edition::Edition> = editions.iter().collect();

    // Narrow by year
    if let Some(y) = year {
        if candidates.len() > 1 {
            let narrowed: Vec<&crate::models::edition::Edition> = candidates
                .iter()
                .filter(|e| e.includes_year(y))
                .copied()
                .collect();
            if !narrowed.is_empty() {
                candidates = narrowed;
            }
        }
    }

    if candidates.len() == 1 {
        Some(candidates[0].clone())
    } else {
        None
    }
}

/// Set court to "scotus" if the reporter is a SCOTUS reporter and no court
/// has been set yet.
pub fn guess_court(
    court: &Option<String>,
    all_editions: &[crate::models::edition::Edition],
) -> Option<String> {
    if court.is_some() {
        return court.clone();
    }
    if all_editions.iter().any(|e| e.reporter.is_scotus) {
        Some("scotus".to_string())
    } else {
        court.clone()
    }
}

// ---------------------------------------------------------------------------
// filter_citations
// ---------------------------------------------------------------------------

/// Check if two spans overlap.
fn overlapping_spans(span1: (usize, usize), span2: (usize, usize)) -> bool {
    let (s1, e1) = span1;
    let (s2, e2) = span2;
    s1.max(s2) < e1.min(e2)
}

/// Filter and order citations, removing overlaps.
///
/// Deduplicates by span, sorts by full_span, and resolves overlaps
/// (preferring non-reference citations over reference citations,
/// and short-case over supra when overlapping).
pub fn filter_citations(citations: Vec<Citation>) -> Vec<Citation> {
    if citations.is_empty() {
        return citations;
    }

    // Dedup by span
    let mut seen: HashMap<(usize, usize), usize> = HashMap::new();
    let mut deduped = Vec::new();
    for c in citations {
        let span = c.span();
        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(span) {
            e.insert(deduped.len());
            deduped.push(c);
        }
    }

    // Sort by full_span
    deduped.sort_by_key(|c| c.full_span());

    let mut filtered: Vec<Citation> = Vec::new();
    filtered.push(deduped.remove(0));

    for citation in deduped {
        let last = filtered.last().unwrap();
        let is_overlapping = overlapping_spans(citation.full_span(), last.full_span());

        if is_overlapping {
            // Prefer non-reference over reference
            if matches!(last, Citation::Reference(_)) {
                filtered.pop();
                filtered.push(citation);
                continue;
            }
            if matches!(citation, Citation::Reference(_)) {
                continue;
            }

            // Skip supra when overlapping with short case
            if matches!(citation, Citation::Supra(_)) && matches!(last, Citation::ShortCase(_)) {
                continue;
            }

            // Keep citation if it appears in the parenthetical
            let paren = match last {
                Citation::FullCase(d) => d.metadata.case.parenthetical.as_deref(),
                Citation::ShortCase(d) => d.metadata.case.parenthetical.as_deref(),
                Citation::FullLaw(d) => d.metadata.parenthetical.as_deref(),
                Citation::FullJournal(d) => d.metadata.parenthetical.as_deref(),
                Citation::Supra(d) => d.metadata.parenthetical.as_deref(),
                Citation::Id(d) => d.metadata.parenthetical.as_deref(),
                Citation::Reference(d) => d.metadata.parenthetical.as_deref(),
                Citation::Unknown(_) => None,
            };
            if let Some(p) = paren {
                if p.contains(citation.matched_text()) {
                    filtered.push(citation);
                    continue;
                }
            }

            // Known overlap case: parallel full citations
            if !matches!(
                (&citation, last),
                (Citation::FullCase(_), Citation::FullCase(_))
            ) {
                warn!(
                    "Unknown overlap case. Last: {:?}, Current: {:?}",
                    last.type_name(),
                    citation.type_name()
                );
            }
        }

        filtered.push(citation);
    }

    filtered
}

// ---------------------------------------------------------------------------
// disambiguate_reporters
// ---------------------------------------------------------------------------

/// Remove citations where there are multiple possible reporters and none
/// could be narrowed down.
pub fn disambiguate_reporters(citations: Vec<Citation>) -> Vec<Citation> {
    citations
        .into_iter()
        .filter(|c| {
            if let Some(rc) = c.resource_common() {
                rc.edition_guess.is_some()
            } else {
                true
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_year_valid() {
        assert_eq!(get_year("1999"), Some(1999));
        assert_eq!(get_year("2024"), Some(2024));
    }

    #[test]
    fn test_get_year_invalid() {
        assert_eq!(get_year("1500"), None);
        assert_eq!(get_year("3000"), None);
        assert_eq!(get_year("abc"), None);
    }

    #[test]
    fn test_get_court_by_paren_exact() {
        // "D. Pa." should match "pennsylvaniad"
        let result = get_court_by_paren("D. Pa.");
        assert!(result.is_some(), "Expected to find a court for 'D. Pa.'");
    }

    #[test]
    fn test_get_court_by_paren_empty() {
        assert_eq!(get_court_by_paren(""), None);
    }

    #[test]
    fn test_generated_court_keys_match_courts_db() {
        let json = include_str!("../../../data/courts-db/courts_db/data/courts.json");
        let courts = citare_data_build::courts::parse_courts(json).unwrap();
        let expected: Vec<(String, String)> = courts
            .iter()
            .map(|c| (normalize_court_string(&c.citation_string), c.id.clone()))
            .collect();
        let generated: Vec<(String, String)> = GENERATED_COURT_KEYS
            .iter()
            .map(|&(k, id)| (k.to_string(), id.to_string()))
            .collect();
        assert_eq!(generated, expected);
    }

    #[test]
    fn test_clean_pin_cite() {
        assert_eq!(
            clean_pin_cite(Some(", at 240,")),
            Some("at 240".to_string())
        );
        assert_eq!(clean_pin_cite(Some("  ")), None);
        assert_eq!(clean_pin_cite(None), None);
    }

    #[test]
    fn test_process_parenthetical_basic() {
        assert_eq!(
            process_parenthetical(Some("overruling Foo")),
            Some("overruling Foo".to_string())
        );
    }

    #[test]
    fn test_process_parenthetical_unbalanced() {
        // "something) (other)" should trim at the first unbalanced )
        assert_eq!(
            process_parenthetical(Some("something) (other)")),
            Some("something".to_string())
        );
    }

    #[test]
    fn test_process_parenthetical_year_only() {
        assert_eq!(process_parenthetical(Some("2012")), None);
    }

    #[test]
    fn test_process_parenthetical_none() {
        assert_eq!(process_parenthetical(None), None);
    }

    #[test]
    fn test_process_parenthetical_empty() {
        assert_eq!(process_parenthetical(Some("")), None);
    }

    #[test]
    fn test_strip_stop_words() {
        let result = strip_stop_words("See Adarand");
        assert!(
            result.contains("Adarand"),
            "Expected 'Adarand' in '{}'",
            result
        );
    }

    #[test]
    fn test_guess_edition_single() {
        use crate::models::edition::{Edition, Reporter, ReporterSource};
        let reporter = Reporter::new(
            "U.S.".to_string(),
            "United States Reports".to_string(),
            "federal".to_string(),
            ReporterSource::Reporters,
        );
        let edition = Edition {
            reporter,
            short_name: "U.S.".to_string(),
            start: None,
            end: None,
            harvard_slug: None,
        };
        let result = guess_edition(std::slice::from_ref(&edition), &[], None);
        assert_eq!(result, Some(edition));
    }

    #[test]
    fn test_guess_edition_multiple_no_year() {
        use crate::models::edition::{Edition, Reporter, ReporterSource};
        let reporter = Reporter::new(
            "S.W.".to_string(),
            "Southwest Reporter".to_string(),
            "state".to_string(),
            ReporterSource::Reporters,
        );
        let e1 = Edition {
            reporter: reporter.clone(),
            short_name: "S.W.".to_string(),
            start: Some("1886-01-01".to_string()),
            end: Some("1999-12-31".to_string()),
            harvard_slug: None,
        };
        let e2 = Edition {
            reporter,
            short_name: "S.W.2d".to_string(),
            start: Some("2000-01-01".to_string()),
            end: None,
            harvard_slug: None,
        };
        // Without year, can't narrow down
        assert_eq!(guess_edition(&[e1, e2], &[], None), None);
    }

    #[test]
    fn test_overlapping_spans() {
        assert!(overlapping_spans((0, 10), (5, 15)));
        assert!(!overlapping_spans((0, 5), (5, 10)));
        assert!(!overlapping_spans((0, 5), (10, 15)));
    }

    #[test]
    fn test_match_on_tokens_forward() {
        let words: Tokens = vec![
            TokenOrStr::Str("hello".to_string()),
            TokenOrStr::Str(" ".to_string()),
            TokenOrStr::Str("world".to_string()),
        ];
        let m = match_on_tokens(&words, 0, r"hello\s+world", "", false, true);
        assert!(m.is_some());
    }

    #[test]
    fn test_match_on_tokens_backward() {
        let words: Tokens = vec![
            TokenOrStr::Str("hello".to_string()),
            TokenOrStr::Str(" ".to_string()),
            TokenOrStr::Str("world".to_string()),
        ];
        let m = match_on_tokens(&words, 2, r"hello\s+world", "", false, false);
        assert!(m.is_some());
    }
}
