//! Tokenizer: extract citation tokens from legal text.
//!
//! Ported from eyecite's `tokenizers.py`. Provides both a simple [`Tokenizer`]
//! and a performance-optimised [`AhocorasickTokenizer`] that pre-filters
//! extractors using Aho-Corasick string matching.

use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use aho_corasick::AhoCorasick;
use regex::Regex;

use crate::models::edition::{Edition, Reporter, ReporterSource};
use crate::models::token::{CitationTokenData, Token, TokenData, TokenOrStr, Tokens};

// ---------------------------------------------------------------------------
// Token type and extractor extra data
// ---------------------------------------------------------------------------

/// The kind of token an extractor produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    Citation,
    Id,
    Supra,
    StopWord,
    Section,
    Paragraph,
    Placeholder,
}

/// Extra data attached to citation extractors (edition lists, short flag).
#[derive(Debug, Clone, Default)]
pub struct ExtractorExtra {
    pub exact_editions: Vec<Edition>,
    pub variation_editions: Vec<Edition>,
    pub short: bool,
}

// ---------------------------------------------------------------------------
// TokenExtractor
// ---------------------------------------------------------------------------

/// An extractor applies a compiled regex to text and produces [`Token`]s.
///
/// Regex compilation is **lazy** — the regex string is stored at construction
/// time but only compiled to a [`Regex`] on first use. This means creating
/// 10K+ extractors is near-instant; only the ~50-100 extractors actually
/// selected by the Aho-Corasick pre-filter pay the compilation cost.
pub struct TokenExtractor {
    /// The original regex string.
    pub regex: String,
    /// Lazily compiled regex.
    compiled_regex: OnceLock<Option<Regex>>,
    /// What kind of token this extractor produces.
    pub token_type: TokenType,
    /// Extra data for citation tokens.
    pub extra: ExtractorExtra,
    /// Strings a text must contain for this extractor to possibly match.
    pub strings: Vec<String>,
    /// Whether this extractor is case-insensitive.
    pub case_insensitive: bool,
}

impl std::fmt::Debug for TokenExtractor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenExtractor")
            .field("regex", &self.regex)
            .field("token_type", &self.token_type)
            .field("strings", &self.strings)
            .field("case_insensitive", &self.case_insensitive)
            .finish()
    }
}

impl TokenExtractor {
    /// Get the compiled regex, compiling on first access.
    fn compiled_regex(&self) -> Option<&Regex> {
        self.compiled_regex
            .get_or_init(|| match Regex::new(&self.regex) {
                Ok(r) => Some(r),
                Err(e) => {
                    log::warn!("Failed to compile regex: {}", e);
                    None
                }
            })
            .as_ref()
    }

    /// Find all non-overlapping matches of this extractor's regex in `text`.
    pub fn find_iter<'a>(
        &'a self,
        text: &'a str,
    ) -> Box<dyn Iterator<Item = regex::Match<'a>> + 'a> {
        match self.compiled_regex() {
            Some(re) => Box::new(re.find_iter(text)),
            None => Box::new(std::iter::empty()),
        }
    }

    /// Create a token from a regex match.
    ///
    /// Re-runs the regex with captures to extract named groups.
    /// Uses capture group 1 (the inner capture from `nonalphanum_boundaries_re`)
    /// for the token's data span, falling back to the full match if no group 1.
    pub fn make_token(&self, m: regex::Match, text: &str) -> Option<Token> {
        let re = self.compiled_regex()?;
        let match_str = &text[m.start()..m.end()];
        let caps = re.captures(match_str)?;

        // Use group 1 (inner capture) if present, else full match
        let (inner_start, inner_end, data) = if let Some(inner) = caps.get(1) {
            (
                m.start() + inner.start(),
                m.start() + inner.end(),
                inner.as_str().to_string(),
            )
        } else {
            (m.start(), m.end(), match_str.to_string())
        };

        // Skip empty tokens (can occur from corrupted/complex PDF data)
        if data.is_empty() {
            return None;
        }

        // Extract named groups
        let mut groups = HashMap::new();
        for name in re.capture_names().flatten() {
            let value = caps.name(name).map(|m| m.as_str().to_string());
            groups.insert(name.to_string(), value);
        }

        let token_data = TokenData {
            data,
            start: inner_start,
            end: inner_end,
            groups,
        };

        match self.token_type {
            TokenType::Citation => Some(Token::Citation(CitationTokenData {
                token_data,
                exact_editions: self.extra.exact_editions.clone(),
                variation_editions: self.extra.variation_editions.clone(),
                short: self.extra.short,
            })),
            TokenType::Id => Some(Token::Id(token_data)),
            TokenType::Supra => Some(Token::Supra(token_data)),
            TokenType::StopWord => Some(Token::StopWord(token_data)),
            TokenType::Section => Some(Token::Section(token_data)),
            TokenType::Paragraph => Some(Token::Paragraph(token_data)),
            TokenType::Placeholder => Some(Token::Placeholder(token_data)),
        }
    }
}

// ---------------------------------------------------------------------------
// Nominative reporters
// ---------------------------------------------------------------------------

/// Reporter short names considered "nominative" — matches are deprioritised
/// when they overlap with other citation matches.
pub const NOMINATIVE_REPORTER_NAMES: &[&str] = &[
    "Thompson", "Cooke", "Holmes", "Olcott", "Chase", "Gilmer", "Bee", "Deady", "Taney",
];

/// Returns `true` if `token` is a citation from a nominative reporter.
pub fn token_is_from_nominative_reporter(token: &Token) -> bool {
    if let Token::Citation(ct) = token {
        let name = if !ct.exact_editions.is_empty() {
            &ct.exact_editions[0].reporter.short_name
        } else if !ct.variation_editions.is_empty() {
            &ct.variation_editions[0].reporter.short_name
        } else {
            return false;
        };
        NOMINATIVE_REPORTER_NAMES.contains(&name.as_str())
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Build-time generated extractor data
// ---------------------------------------------------------------------------

include!(concat!(env!("OUT_DIR"), "/generated_extractors.rs"));

// ---------------------------------------------------------------------------
// Extractor builder (uses build-time generated data)
// ---------------------------------------------------------------------------

/// Global lazily-initialised extractors.
pub static EXTRACTORS: LazyLock<Vec<TokenExtractor>> = LazyLock::new(build_extractors);

/// Global lazily-initialised tokenizer shared by every `get_citations` call.
///
/// Building the extractors and Aho-Corasick automata is far more expensive
/// than tokenizing a typical document, and each extractor compiles its regex
/// on first use, so sharing one instance keeps both costs to once per process.
pub static DEFAULT_TOKENIZER: LazyLock<AhocorasickTokenizer> =
    LazyLock::new(AhocorasickTokenizer::new);

/// Global lazily-initialised editions lookup (reporter string -> editions).
pub static EDITIONS_LOOKUP: LazyLock<HashMap<String, Vec<Edition>>> =
    LazyLock::new(build_editions_lookup);

/// Convert a generated edition to a runtime Edition.
fn edition_from_generated(gen: &GeneratedEdition) -> Edition {
    let source = match gen.reporter_source {
        0 => ReporterSource::Reporters,
        1 => ReporterSource::Laws,
        2 => ReporterSource::Journals,
        _ => ReporterSource::Reporters,
    };
    Edition {
        reporter: Reporter::new(
            gen.reporter_short_name.to_string(),
            gen.reporter_name.to_string(),
            gen.reporter_cite_type.to_string(),
            source,
        ),
        short_name: gen.short_name.to_string(),
        start: gen.start.map(|s| s.to_string()),
        end: gen.end.map(|s| s.to_string()),
        harvard_slug: gen.harvard_slug.map(|s| s.to_string()),
    }
}

/// Convert a token type byte to a TokenType enum.
fn token_type_from_u8(v: u8) -> TokenType {
    match v {
        0 => TokenType::Citation,
        1 => TokenType::Id,
        2 => TokenType::Supra,
        3 => TokenType::StopWord,
        4 => TokenType::Section,
        5 => TokenType::Paragraph,
        6 => TokenType::Placeholder,
        _ => TokenType::Citation,
    }
}

/// Build all extractors from the build-time generated data.
///
/// Reconstructs the full set of extractors by substituting reporter strings
/// into deduplicated regex templates.  This produces the same extractors as
/// the old flat `GENERATED_EXTRACTORS` array, but the generated source file
/// is ~60% smaller because each template is stored only once.
///
/// Regex compilation is still deferred — each extractor compiles its regex
/// lazily on first use.
pub fn build_extractors() -> Vec<TokenExtractor> {
    let mut extractors =
        Vec::with_capacity(GENERATED_REPORTER_MAP.len() + GENERATED_ONEOFF_EXTRACTORS.len());

    // Citation extractors: combine templates with reporter strings
    for entry in GENERATED_REPORTER_MAP {
        let template = GENERATED_TEMPLATES[entry.template_idx];
        // Substitute the reporter placeholder with the actual reporter string
        let regex = template.replace(REPORTER_PLACEHOLDER, entry.reporter_string);

        let editions: Vec<Edition> = entry
            .edition_indices
            .iter()
            .map(|&i| edition_from_generated(&GENERATED_EDITIONS[i]))
            .collect();
        let variations: Vec<Edition> = entry
            .variation_indices
            .iter()
            .map(|&i| edition_from_generated(&GENERATED_EDITIONS[i]))
            .collect();

        extractors.push(TokenExtractor {
            regex,
            compiled_regex: OnceLock::new(),
            token_type: TokenType::Citation,
            extra: ExtractorExtra {
                exact_editions: editions,
                variation_editions: variations,
                short: entry.short,
            },
            strings: entry.strings.iter().map(|s| s.to_string()).collect(),
            case_insensitive: false,
        });
    }

    // One-off extractors (Id, Supra, StopWord, etc.)
    for gen in GENERATED_ONEOFF_EXTRACTORS {
        let token_type = token_type_from_u8(gen.token_type);

        extractors.push(TokenExtractor {
            regex: gen.regex.to_string(),
            compiled_regex: OnceLock::new(),
            token_type,
            extra: ExtractorExtra::default(),
            strings: gen.strings.iter().map(|s| s.to_string()).collect(),
            case_insensitive: gen.case_insensitive,
        });
    }

    extractors
}

/// Build the editions lookup table from the build-time generated data.
fn build_editions_lookup() -> HashMap<String, Vec<Edition>> {
    let mut lookup = HashMap::with_capacity(GENERATED_EDITIONS_LOOKUP.len());
    for &(key, indices) in GENERATED_EDITIONS_LOOKUP {
        let editions: Vec<Edition> = indices
            .iter()
            .map(|&i| edition_from_generated(&GENERATED_EDITIONS[i]))
            .collect();
        lookup.insert(key.to_string(), editions);
    }
    lookup
}

// ---------------------------------------------------------------------------
// Tokenizer (base)
// ---------------------------------------------------------------------------

/// A tokenizer applies a set of [`TokenExtractor`]s to text and returns
/// structured tokens.
pub struct Tokenizer {
    extractors: Vec<TokenExtractor>,
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Tokenizer {
    /// Create a new `Tokenizer` with the default global extractors.
    pub fn new() -> Self {
        Self {
            extractors: build_extractors(),
        }
    }

    /// Tokenize `text` and return (all_tokens, citation_tokens_with_positions).
    pub fn tokenize(&self, text: &str) -> (Tokens, Vec<(usize, Token)>) {
        tokenize_impl(&self.extract_tokens(text), text)
    }

    fn get_extractors(&self, _text: &str) -> &[TokenExtractor] {
        &self.extractors
    }

    fn extract_tokens(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        for extractor in self.get_extractors(text) {
            for m in extractor.find_iter(text) {
                if let Some(token) = extractor.make_token(m, text) {
                    tokens.push(token);
                }
            }
        }
        tokens
    }
}

// ---------------------------------------------------------------------------
// AhocorasickTokenizer
// ---------------------------------------------------------------------------

/// Remove all whitespace from `text` (eyecite's `_strip_ws`), so the
/// prefilter stays a superset of the whitespace-relaxed reporter regexes.
fn strip_ws(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// A performance-optimised tokenizer that uses Aho-Corasick pre-filtering
/// to only run extractors whose required strings appear in the text.
pub struct AhocorasickTokenizer {
    extractors: Vec<TokenExtractor>,
    /// Indices of extractors that have no required strings (always run).
    unfiltered_indices: Vec<usize>,
    /// Case-sensitive Aho-Corasick automaton.
    case_sensitive_ac: Option<AhoCorasick>,
    /// For each pattern in the case-sensitive AC: which extractor indices it maps to.
    case_sensitive_map: Vec<Vec<usize>>,
    /// Case-insensitive Aho-Corasick automaton.
    case_insensitive_ac: Option<AhoCorasick>,
    /// For each pattern in the case-insensitive AC: which extractor indices it maps to.
    case_insensitive_map: Vec<Vec<usize>>,
}

impl Default for AhocorasickTokenizer {
    fn default() -> Self {
        Self::new()
    }
}

impl AhocorasickTokenizer {
    /// Create a new `AhocorasickTokenizer` with the default global extractors.
    pub fn new() -> Self {
        let extractors = build_extractors();
        Self::from_extractors(extractors)
    }

    /// Build from a list of extractors (useful for testing).
    fn from_extractors(extractors: Vec<TokenExtractor>) -> Self {
        let mut unfiltered_indices = Vec::new();
        let mut cs_grouped: HashMap<String, Vec<usize>> = HashMap::new();
        let mut ci_grouped: HashMap<String, Vec<usize>> = HashMap::new();

        for (idx, ext) in extractors.iter().enumerate() {
            if ext.strings.is_empty() {
                unfiltered_indices.push(idx);
            } else if ext.case_insensitive {
                for s in &ext.strings {
                    ci_grouped
                        .entry(strip_ws(s).to_lowercase())
                        .or_default()
                        .push(idx);
                }
            } else {
                for s in &ext.strings {
                    cs_grouped.entry(strip_ws(s)).or_default().push(idx);
                }
            }
        }

        // Build case-sensitive AC
        let (case_sensitive_ac, case_sensitive_map) = if cs_grouped.is_empty() {
            (None, Vec::new())
        } else {
            let patterns: Vec<String> = cs_grouped.keys().cloned().collect();
            let map: Vec<Vec<usize>> = patterns.iter().map(|p| cs_grouped[p].clone()).collect();
            let ac = aho_corasick::AhoCorasick::builder()
                .match_kind(aho_corasick::MatchKind::Standard)
                .build(&patterns)
                .expect("Failed to build case-sensitive AhoCorasick");
            (Some(ac), map)
        };

        // Build case-insensitive AC
        let (case_insensitive_ac, case_insensitive_map) = if ci_grouped.is_empty() {
            (None, Vec::new())
        } else {
            let patterns: Vec<String> = ci_grouped.keys().cloned().collect();
            let map: Vec<Vec<usize>> = patterns.iter().map(|p| ci_grouped[p].clone()).collect();
            let ac = aho_corasick::AhoCorasick::builder()
                .match_kind(aho_corasick::MatchKind::Standard)
                .build(&patterns)
                .expect("Failed to build case-insensitive AhoCorasick");
            (Some(ac), map)
        };

        Self {
            extractors,
            unfiltered_indices,
            case_sensitive_ac,
            case_sensitive_map,
            case_insensitive_ac,
            case_insensitive_map,
        }
    }

    /// Tokenize `text` and return (all_tokens, citation_tokens_with_positions).
    pub fn tokenize(&self, text: &str) -> (Tokens, Vec<(usize, Token)>) {
        let tokens = self.extract_tokens(text);
        tokenize_impl(&tokens, text)
    }

    /// Get the set of extractors that could match `text`.
    ///
    /// Uses overlapping iteration to find all patterns, even when shorter
    /// patterns (e.g., "A.") overlap with longer ones (e.g., "A.2d").
    ///
    /// Reporter regexes allow flexible whitespace between abbreviation tokens
    /// (eyecite #305), so the prefilter strings and the text are both matched
    /// with whitespace removed: `N. Y. S. 2d` must still select the
    /// `N.Y.S.2d` extractor. Match spans still come from running the selected
    /// extractors over the original text.
    fn get_extractors(&self, text: &str) -> Vec<&TokenExtractor> {
        let mut result: HashSet<usize> = self.unfiltered_indices.iter().copied().collect();
        let stripped = strip_ws(text);
        let text = stripped.as_str();

        if let Some(ref ac) = self.case_sensitive_ac {
            let mut state = aho_corasick::automaton::OverlappingState::start();
            loop {
                ac.find_overlapping(text, &mut state);
                let mat = match state.get_match() {
                    Some(m) => m,
                    None => break,
                };
                for &idx in &self.case_sensitive_map[mat.pattern().as_usize()] {
                    result.insert(idx);
                }
            }
        }

        if let Some(ref ac) = self.case_insensitive_ac {
            let lower = text.to_lowercase();
            let mut state = aho_corasick::automaton::OverlappingState::start();
            loop {
                ac.find_overlapping(&lower, &mut state);
                let mat = match state.get_match() {
                    Some(m) => m,
                    None => break,
                };
                for &idx in &self.case_insensitive_map[mat.pattern().as_usize()] {
                    result.insert(idx);
                }
            }
        }

        result.iter().map(|&i| &self.extractors[i]).collect()
    }

    fn extract_tokens(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        for extractor in self.get_extractors(text) {
            for m in extractor.find_iter(text) {
                if let Some(token) = extractor.make_token(m, text) {
                    tokens.push(token);
                }
            }
        }
        tokens
    }

    /// Debug: return the number of extractors selected for text.
    pub fn extractor_count_for_text(&self, text: &str) -> usize {
        self.get_extractors(text).len()
    }
}

// ---------------------------------------------------------------------------
// Shared tokenize implementation
// ---------------------------------------------------------------------------

/// Core tokenization logic shared by `Tokenizer` and `AhocorasickTokenizer`.
///
/// Takes a list of raw tokens, sorts them, resolves overlaps, fills gaps
/// with plain text, and returns the final token list plus citation positions.
fn tokenize_impl(raw_tokens: &[Token], text: &str) -> (Tokens, Vec<(usize, Token)>) {
    // Sort by (start ascending, end descending)
    let mut sorted: Vec<&Token> = raw_tokens.iter().collect();
    sorted.sort_by(|a, b| {
        a.start()
            .cmp(&b.start())
            .then_with(|| b.end().cmp(&a.end()))
    });

    let mut citation_tokens: Vec<(usize, Token)> = Vec::new();
    let mut all_tokens: Tokens = Vec::new();
    let mut last_token: Option<Token> = None;
    let mut offset: usize = 0;

    for token in sorted {
        if let Some(ref mut lt) = last_token {
            // Attempt to merge identical matches from different extractors
            if lt.merge(token) {
                // Keep the emitted token in sync with the merged one: both
                // `citation_tokens` and `all_tokens` hold clones, and
                // citation extraction reads editions from `all_tokens`.
                if let Some((idx, ct)) = citation_tokens.last_mut() {
                    ct.merge(token);
                    all_tokens[*idx] = TokenOrStr::Token(ct.clone());
                }
                continue;
            }
        }

        if offset > token.start() {
            // Overlapping match
            if let Some(ref lt) = last_token {
                if matches!(token, Token::Citation(_)) && token_is_from_nominative_reporter(lt) {
                    // Prefer non-nominative: replace the last token
                    citation_tokens.pop();
                    all_tokens.pop();
                    // Recalculate offset — we need to re-emit text before this token
                    // The offset before the popped token was the start of the popped token
                    offset = lt.start();
                } else {
                    // Skip this overlapping token
                    continue;
                }
            } else {
                continue;
            }
        }

        // Emit plain text before this token
        if offset < token.start() {
            append_text(&mut all_tokens, &text[offset..token.start()]);
        }

        // Emit the token
        citation_tokens.push((all_tokens.len(), token.clone()));
        all_tokens.push(TokenOrStr::Token(token.clone()));
        offset = token.end();
        last_token = Some(token.clone());
    }

    // Emit trailing plain text
    if offset < text.len() {
        append_text(&mut all_tokens, &text[offset..]);
    }

    (all_tokens, citation_tokens)
}

/// Split text into words (separated by spaces), treating whitespace as
/// tokens too, and append to the token list.
///
/// This replicates Python's `Tokenizer.append_text` exactly:
/// ```python
/// for part in text.split(" "):
///     if part:
///         tokens.extend((part, " "))
///     else:
///         tokens.append(" ")
/// tokens.pop()
/// ```
pub fn append_text(tokens: &mut Tokens, text: &str) {
    if text.is_empty() {
        return;
    }
    for part in text.split(' ') {
        if !part.is_empty() {
            tokens.push(TokenOrStr::Str(part.to_string()));
            tokens.push(TokenOrStr::Str(" ".to_string()));
        } else {
            tokens.push(TokenOrStr::Str(" ".to_string()));
        }
    }
    tokens.pop(); // Remove final extra space
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extractor_count() {
        let extractors = build_extractors();
        // Should be > 2000 extractors (reporters + laws + journals + one-offs)
        assert!(
            extractors.len() > 2000,
            "Expected > 2000 extractors, got {}",
            extractors.len()
        );
    }

    #[test]
    fn test_append_text_basic() {
        let mut tokens = Vec::new();
        append_text(&mut tokens, "foo bar baz");
        let strs: Vec<&str> = tokens
            .iter()
            .map(|t| match t {
                TokenOrStr::Str(s) => s.as_str(),
                TokenOrStr::Token(_) => "<token>",
            })
            .collect();
        assert_eq!(strs, vec!["foo", " ", "bar", " ", "baz"]);
    }

    #[test]
    fn test_append_text_double_space() {
        let mut tokens = Vec::new();
        append_text(&mut tokens, "foo  bar");
        let strs: Vec<&str> = tokens
            .iter()
            .map(|t| match t {
                TokenOrStr::Str(s) => s.as_str(),
                TokenOrStr::Token(_) => "<token>",
            })
            .collect();
        assert_eq!(strs, vec!["foo", " ", " ", "bar"]);
    }

    #[test]
    fn test_append_text_empty() {
        let mut tokens = Vec::new();
        append_text(&mut tokens, "");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_append_text_single_word() {
        let mut tokens = Vec::new();
        append_text(&mut tokens, "hello");
        let strs: Vec<&str> = tokens
            .iter()
            .map(|t| match t {
                TokenOrStr::Str(s) => s.as_str(),
                TokenOrStr::Token(_) => "<token>",
            })
            .collect();
        assert_eq!(strs, vec!["hello"]);
    }

    /// Port of eyecite's updated `test_extractor_filter` (eyecite #305): the
    /// prefilter matches whitespace-insensitively, so the compact `U.S.` in
    /// the text also selects the extractors for spaced variations like
    /// `U. S.`, and spaced text selects the compact reporter's extractor.
    /// citare emits one extractor per variation, so variations are checked
    /// individually rather than as one combined string set.
    #[test]
    fn test_extractor_filter() {
        let tokenizer = AhocorasickTokenizer::new();
        let strings_for = |text: &str| -> HashSet<Vec<String>> {
            tokenizer
                .get_extractors(text)
                .into_iter()
                .filter(|e| !e.strings.is_empty())
                .map(|e| {
                    let mut s = e.strings.clone();
                    s.sort();
                    s
                })
                .collect()
        };

        let selected = strings_for("See foo, 123 U.S. 456. Id.");
        let mut stop_words: Vec<String> = crate::regexes::STOP_WORDS
            .iter()
            .map(|s| s.to_string())
            .collect();
        stop_words.sort();
        assert!(selected.contains(&stop_words));
        assert!(selected.contains(&vec!["U.S.".to_string()]));
        assert!(selected.contains(&vec!["U. S.".to_string()]));
        assert!(selected.contains(&vec!["ibid.".to_string(), "id.".to_string()]));

        let selected = strings_for("123 N. Y. S. 2d 456");
        assert!(selected.contains(&vec!["N.Y.S.2d".to_string()]));
    }

    /// Two extractors matching the same span merge into one token, and the
    /// merged editions must reach `all_tokens` (which citation extraction
    /// reads), not just the `citation_tokens` list.
    #[test]
    fn test_merged_token_reaches_all_tokens() {
        let make = |short_name: &str| {
            let edition = Edition {
                reporter: Reporter::new(
                    short_name.to_string(),
                    short_name.to_string(),
                    "state".to_string(),
                    ReporterSource::Reporters,
                ),
                short_name: short_name.to_string(),
                start: None,
                end: None,
                harvard_slug: None,
            };
            Token::Citation(CitationTokenData {
                token_data: TokenData {
                    data: "1 P.R. 1".to_string(),
                    start: 0,
                    end: 8,
                    groups: HashMap::new(),
                },
                exact_editions: vec![],
                variation_editions: vec![edition],
                short: false,
            })
        };
        let raw = vec![make("P."), make("Pen. & W.")];
        let (all_tokens, citation_tokens) = tokenize_impl(&raw, "1 P.R. 1");
        assert_eq!(citation_tokens.len(), 1);
        let idx = citation_tokens[0].0;
        match &all_tokens[idx] {
            TokenOrStr::Token(Token::Citation(ct)) => {
                assert_eq!(ct.variation_editions.len(), 2);
            }
            other => panic!("expected citation token, got {other:?}"),
        }
    }

    /// Port of eyecite's updated `test_tokenize`: "U. S." is both a
    /// registered variation and, with relaxed whitespace, an exact match for
    /// "U.S.". The overlapping matches merge into one token that keeps the
    /// edition as an exact match only (eyecite #317).
    #[test]
    fn test_spaced_variation_is_exact_once() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_tokens, citation_tokens) =
            tokenizer.tokenize("See Roe v. Wade, 410 U. S. 113 (1973)");
        let ct = citation_tokens
            .iter()
            .find_map(|(_, t)| match t {
                Token::Citation(ct) => Some(ct),
                _ => None,
            })
            .expect("citation token");
        let exact: Vec<_> = ct
            .exact_editions
            .iter()
            .map(|e| e.short_name.as_str())
            .collect();
        assert_eq!(exact, vec!["U.S."]);
        assert!(ct.variation_editions.is_empty());
    }

    #[test]
    fn test_basic_tokenize() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_tokens, citation_tokens) = tokenizer.tokenize("1 U.S. 1");
        assert!(
            !citation_tokens.is_empty(),
            "Should find at least one citation token"
        );
        let (_, ref first_ct) = citation_tokens[0];
        assert_eq!(first_ct.data(), "1 U.S. 1");
    }

    #[test]
    fn test_tokenize_with_surrounding_text() {
        let tokenizer = AhocorasickTokenizer::new();
        let (all_tokens, citation_tokens) = tokenizer.tokenize("foo 1 U.S. 1 bar");
        assert_eq!(
            citation_tokens.len(),
            1,
            "Expected 1 citation, got {}",
            citation_tokens.len()
        );
        // The all_tokens list should contain text before, the citation, and text after
        assert!(all_tokens.len() > 1);
    }

    #[test]
    fn test_tokenize_produces_text_tokens() {
        let tokenizer = AhocorasickTokenizer::new();
        let (all_tokens, _) = tokenizer.tokenize("hello world");
        // Should produce ["hello", " ", "world"]
        let strs: Vec<&str> = all_tokens
            .iter()
            .filter_map(|t| match t {
                TokenOrStr::Str(s) => Some(s.as_str()),
                TokenOrStr::Token(_) => None,
            })
            .collect();
        assert_eq!(strs, vec!["hello", " ", "world"]);
    }

    #[test]
    fn test_nominative_reporter_detection() {
        let td = TokenData {
            data: "1 Holmes 1".to_string(),
            start: 0,
            end: 10,
            groups: HashMap::new(),
        };
        let reporter = Reporter::new(
            "Holmes".to_string(),
            "Holmes Reports".to_string(),
            "federal".to_string(),
            ReporterSource::Reporters,
        );
        let edition = Edition {
            reporter,
            short_name: "Holmes".to_string(),
            start: None,
            end: None,
            harvard_slug: None,
        };
        let token = Token::Citation(CitationTokenData {
            token_data: td,
            exact_editions: vec![edition],
            variation_editions: vec![],
            short: false,
        });
        assert!(token_is_from_nominative_reporter(&token));
    }

    #[test]
    fn test_non_nominative_reporter() {
        let td = TokenData {
            data: "1 U.S. 1".to_string(),
            start: 0,
            end: 8,
            groups: HashMap::new(),
        };
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
        let token = Token::Citation(CitationTokenData {
            token_data: td,
            exact_editions: vec![edition],
            variation_editions: vec![],
            short: false,
        });
        assert!(!token_is_from_nominative_reporter(&token));
    }

    #[test]
    fn test_non_citation_not_nominative() {
        let td = TokenData {
            data: "id.".to_string(),
            start: 0,
            end: 3,
            groups: HashMap::new(),
        };
        assert!(!token_is_from_nominative_reporter(&Token::Id(td)));
    }

    #[test]
    fn test_base_tokenizer_matches_ahocorasick() {
        // Both tokenizers should find the same citation in simple text
        let base = Tokenizer::new();
        let ac = AhocorasickTokenizer::new();
        let text = "1 U.S. 1";
        let (_, base_cts) = base.tokenize(text);
        let (_, ac_cts) = ac.tokenize(text);
        assert_eq!(base_cts.len(), ac_cts.len());
        assert_eq!(base_cts[0].1.data(), ac_cts[0].1.data());
    }

    #[test]
    fn test_id_token_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("foo id. bar");
        let id_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Id(_)))
            .collect();
        assert!(
            !id_tokens.is_empty(),
            "Should find an Id token in 'foo id. bar'"
        );
    }

    #[test]
    fn test_supra_token_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("foo supra bar");
        let supra_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Supra(_)))
            .collect();
        assert!(
            !supra_tokens.is_empty(),
            "Should find a Supra token in 'foo supra bar'"
        );
    }

    #[test]
    fn test_section_token_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("42\u{00A7}1983 is important");
        let section_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Section(_)))
            .collect();
        assert!(!section_tokens.is_empty(), "Should find a Section token");
    }

    #[test]
    fn test_placeholder_token_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("___ U.S. ___");
        let placeholder_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Placeholder(_)))
            .collect();
        assert!(
            !placeholder_tokens.is_empty(),
            "Should find a Placeholder token in '___ U.S. ___'"
        );
    }

    #[test]
    fn test_stop_word_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("Smith v Jones");
        let stop_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::StopWord(_)))
            .collect();
        assert!(
            !stop_tokens.is_empty(),
            "Should find a StopWord token for 'v'"
        );
    }

    #[test]
    fn test_paragraph_token_extraction() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("foo\nbar");
        let para_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Paragraph(_)))
            .collect();
        assert!(
            !para_tokens.is_empty(),
            "Should find a Paragraph token for newline"
        );
    }

    #[test]
    fn test_multiple_citations() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("1 U.S. 1 and 2 U.S. 2");
        let cite_tokens: Vec<_> = citation_tokens
            .iter()
            .filter(|(_, t)| matches!(t, Token::Citation(_)))
            .collect();
        assert!(
            cite_tokens.len() >= 2,
            "Should find at least 2 citation tokens, got {}",
            cite_tokens.len()
        );
    }

    #[test]
    fn test_citation_groups() {
        let tokenizer = AhocorasickTokenizer::new();
        let (_, citation_tokens) = tokenizer.tokenize("1 U.S. 1");
        let (_, ref ct) = citation_tokens[0];
        let groups = ct.groups();
        // Should have volume and page groups at minimum
        assert!(
            groups.contains_key("volume") || groups.contains_key("page"),
            "Citation should have named capture groups, got: {:?}",
            groups.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_editions_lookup() {
        let lookup = &*EDITIONS_LOOKUP;
        assert!(
            lookup.contains_key("U.S."),
            "EDITIONS_LOOKUP should contain 'U.S.'"
        );
        assert!(
            !lookup["U.S."].is_empty(),
            "U.S. should have at least one edition"
        );
    }

    #[test]
    fn test_harvard_slug_us() {
        let lookup = &*EDITIONS_LOOKUP;
        let editions = lookup
            .get("U.S.")
            .expect("U.S. should be in EDITIONS_LOOKUP");
        let slug = editions.iter().find_map(|e| e.harvard_slug.as_deref());
        assert_eq!(
            slug,
            Some("us"),
            "U.S. editions should have harvard_slug = Some(\"us\")"
        );
    }

    #[test]
    fn test_harvard_slug_a2d() {
        let lookup = &*EDITIONS_LOOKUP;
        let editions = lookup
            .get("A.2d")
            .expect("A.2d should be in EDITIONS_LOOKUP");
        let slug = editions.iter().find_map(|e| e.harvard_slug.as_deref());
        assert_eq!(
            slug,
            Some("a2d"),
            "A.2d editions should have harvard_slug = Some(\"a2d\")"
        );
    }

    #[test]
    fn test_harvard_slug_none_for_unmatched() {
        // "A." (Atlantic Reporter 1st series abbreviated) is not in the
        // Harvard CAP data, so its editions should have harvard_slug = None.
        let lookup = &*EDITIONS_LOOKUP;
        if let Some(editions) = lookup.get("A.") {
            for e in editions {
                assert!(
                    e.harvard_slug.is_none(),
                    "A. should not have a harvard_slug, got {:?}",
                    e.harvard_slug
                );
            }
        }
        // Also verify at the raw generated data level
        let unmatched = GENERATED_EDITIONS
            .iter()
            .filter(|e| e.short_name == "A." && e.harvard_slug.is_some())
            .count();
        assert_eq!(
            unmatched, 0,
            "A. should have no Harvard slug in generated data"
        );
    }

    #[test]
    fn test_harvard_slug_in_generated_editions() {
        // Verify the generated data array contains expected slugs
        let us_edition = GENERATED_EDITIONS
            .iter()
            .find(|e| e.short_name == "U.S." && e.harvard_slug == Some("us"));
        assert!(
            us_edition.is_some(),
            "GENERATED_EDITIONS should contain U.S. with harvard_slug = Some(\"us\")"
        );
        let a2d_edition = GENERATED_EDITIONS
            .iter()
            .find(|e| e.short_name == "A.2d" && e.harvard_slug == Some("a2d"));
        assert!(
            a2d_edition.is_some(),
            "GENERATED_EDITIONS should contain A.2d with harvard_slug = Some(\"a2d\")"
        );
    }
}
