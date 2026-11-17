//! Tokenizer: extract citation tokens from legal text.
//!
//! The counterpart of eyecite's `tokenizers.py`. Provides both a simple [`Tokenizer`]
//! and a performance-optimised [`AhocorasickTokenizer`] that pre-filters
//! extractors using Aho-Corasick string matching.

use std::collections::{BTreeMap, HashMap};
use std::sync::{LazyLock, OnceLock};

use aho_corasick::AhoCorasick;
use regex::Regex;

use crate::models::edition::{Edition, ReporterSource};
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

impl Clone for TokenExtractor {
    /// Copy the extractor without its compiled regex, so a copy whose
    /// `regex` is then changed compiles the new pattern (eyecite's
    /// `copy(extractor)` plus `del e._compiled_regex`).
    fn clone(&self) -> Self {
        TokenExtractor {
            regex: self.regex.clone(),
            compiled_regex: OnceLock::new(),
            token_type: self.token_type,
            extra: self.extra.clone(),
            strings: self.strings.clone(),
            case_insensitive: self.case_insensitive,
        }
    }
}

impl TokenExtractor {
    /// Create an extractor.
    ///
    /// `regex` is compiled on first use (a pattern that fails to compile
    /// logs a warning and matches nothing). Capture group 1, if any, is the
    /// token's text, and named groups (`volume`, `reporter`, `page`, ...)
    /// become the token's groups. `strings` are the literal strings a text
    /// must contain for [`AhocorasickTokenizer`] to run the extractor
    /// (whitespace is ignored when matching them); with none, it always
    /// runs. A [`TokenType::Citation`] extractor's `extra` lists the
    /// editions its citations can be in.
    pub fn new(
        regex: impl Into<String>,
        token_type: TokenType,
        extra: ExtractorExtra,
        strings: Vec<String>,
        case_insensitive: bool,
    ) -> Self {
        TokenExtractor {
            regex: regex.into(),
            compiled_regex: OnceLock::new(),
            token_type,
            extra,
            strings,
            case_insensitive,
        }
    }

    /// Whether every edition this extractor finds is a law (laws.json).
    fn is_law(&self) -> bool {
        let mut eds = self
            .extra
            .exact_editions
            .iter()
            .chain(&self.extra.variation_editions)
            .peekable();
        eds.peek().is_some() && eds.all(|e| e.reporter.source == ReporterSource::Laws)
    }

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

        let mut token_data = token_data;
        if self.token_type == TokenType::Citation && self.is_law() {
            // A reporter regex ends in `\s*` (whitespace is relaxed after its
            // periods), so a statute regex with optional space after the
            // reporter (`G. L. c. 6`, `12 O.S. 2011`) can capture it.
            if let Some(Some(r)) = token_data.groups.get_mut("reporter") {
                let trimmed = r.trim_end().len();
                r.truncate(trimmed);
            }
        }

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
// Extractor builder (uses the data bundle, see `crate::data`)
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

/// Global lazily-initialised editions lookup (reporter string -> editions),
/// derived from the reporter map. Only tests use it.
pub static EDITIONS_LOOKUP: LazyLock<HashMap<String, Vec<Edition>>> =
    LazyLock::new(build_editions_lookup);

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

/// Build all extractors from the loaded data bundle.
///
/// Reconstructs the full set of extractors by substituting reporter strings
/// into deduplicated regex templates. Regex compilation is still deferred:
/// each extractor compiles its regex lazily on first use.
pub fn build_extractors() -> Vec<TokenExtractor> {
    let data = crate::data::get();
    let bundle = &data.bundle;
    let mut extractors = Vec::with_capacity(bundle.reporter_map.len() + bundle.oneoffs.len());

    // Citation extractors: combine templates with reporter strings
    for entry in &bundle.reporter_map {
        let template = &bundle.templates[entry.template_idx as usize];
        // Substitute the reporter placeholder with the actual reporter string
        let regex = template.replace(crate::data::REPORTER_PLACEHOLDER, &entry.reporter_string);

        let editions: Vec<Edition> = entry
            .edition_indices
            .iter()
            .map(|&i| data.editions[i as usize].clone())
            .collect();
        let variations: Vec<Edition> = entry
            .variation_indices
            .iter()
            .map(|&i| data.editions[i as usize].clone())
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
    for gen in &bundle.oneoffs {
        extractors.push(TokenExtractor {
            regex: gen.regex.to_string(),
            compiled_regex: OnceLock::new(),
            token_type: token_type_from_u8(gen.token_type),
            extra: ExtractorExtra::default(),
            strings: gen.strings.iter().map(|s| s.to_string()).collect(),
            case_insensitive: gen.case_insensitive,
        });
    }

    extractors
}

/// Build the editions lookup from the loaded data bundle: each prefilter
/// string mapped to the editions (exact, then variations) of every citation
/// extractor that lists it, in registry order and without repeats.
fn build_editions_lookup() -> HashMap<String, Vec<Edition>> {
    let data = crate::data::get();
    let mut lookup: HashMap<String, Vec<Edition>> = HashMap::new();
    for entry in &data.bundle.reporter_map {
        for s in &entry.strings {
            let editions = lookup.entry(s.to_string()).or_default();
            for &i in entry.edition_indices.iter().chain(&entry.variation_indices) {
                let e = &data.editions[i as usize];
                if !editions.contains(e) {
                    editions.push(e.clone());
                }
            }
        }
    }
    lookup
}

// ---------------------------------------------------------------------------
// Tokenize trait
// ---------------------------------------------------------------------------

/// Something that turns text into tokens for the citation finder: eyecite's
/// `Tokenizer` interface.
///
/// [`Tokenizer`] and [`AhocorasickTokenizer`] implement it. Pass one to
/// [`crate::find::FindOptions::tokenizer`] to find citations with your own
/// extractors (see [`build_extractors`] and [`TokenExtractor::new`]).
pub trait Tokenize {
    /// Tokenize `text`: every token and the text between tokens, in order,
    /// and the citation tokens (citation, id, supra and section) with their
    /// index in the first list.
    fn tokenize(&self, text: &str) -> (Tokens, Vec<(usize, Token)>);
}

impl Tokenize for Tokenizer {
    fn tokenize(&self, text: &str) -> (Tokens, Vec<(usize, Token)>) {
        Tokenizer::tokenize(self, text)
    }
}

impl Tokenize for AhocorasickTokenizer {
    fn tokenize(&self, text: &str) -> (Tokens, Vec<(usize, Token)>) {
        AhocorasickTokenizer::tokenize(self, text)
    }
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

    /// Create a `Tokenizer` that runs `extractors`, every one on every text.
    pub fn from_extractors(extractors: Vec<TokenExtractor>) -> Self {
        Self { extractors }
    }

    /// The extractors this tokenizer runs.
    pub fn extractors(&self) -> &[TokenExtractor] {
        &self.extractors
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

    /// Create an `AhocorasickTokenizer` that runs `extractors`, each only on
    /// texts that contain one of its `strings`.
    pub fn from_extractors(extractors: Vec<TokenExtractor>) -> Self {
        let mut unfiltered_indices = Vec::new();
        // Ordered maps, so the automata's pattern ids are the same in every
        // process.
        let mut cs_grouped: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut ci_grouped: BTreeMap<String, Vec<usize>> = BTreeMap::new();

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
            let (patterns, map): (Vec<String>, Vec<Vec<usize>>) = cs_grouped.into_iter().unzip();
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
            let (patterns, map): (Vec<String>, Vec<Vec<usize>>) = ci_grouped.into_iter().unzip();
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
    ///
    /// The extractors come back in registry order, the order the base
    /// [`Tokenizer`] runs them in. Tokens with equal spans keep that order
    /// through the stable sort in `tokenize_impl`, so it decides which token
    /// wins an overlap and the order of merged editions. A hash set here
    /// made that order vary per process (#22).
    fn get_extractors(&self, text: &str) -> Vec<&TokenExtractor> {
        let mut selected = vec![false; self.extractors.len()];
        for &idx in &self.unfiltered_indices {
            selected[idx] = true;
        }
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
                    selected[idx] = true;
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
                    selected[idx] = true;
                }
            }
        }

        self.extractors
            .iter()
            .zip(selected)
            .filter_map(|(ext, keep)| keep.then_some(ext))
            .collect()
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

    /// The extractors this tokenizer can run.
    pub fn extractors(&self) -> &[TokenExtractor] {
        &self.extractors
    }

    /// Debug: return the number of extractors selected for text.
    pub fn extractor_count_for_text(&self, text: &str) -> usize {
        self.get_extractors(text).len()
    }
}

// ---------------------------------------------------------------------------
// Shared tokenize implementation
// ---------------------------------------------------------------------------

/// The length of a law citation token's `reporter` group, or 0 for any other
/// token.
///
/// Two laws.json regexes can match the same text with different groups:
/// `N.Y. Gen. Bus. Law § 394` is reporter `N.Y.` with subject `Gen. Bus.
/// Law` under reporters-db's `N.Y.` entry, and reporter `N.Y. Gen. Bus. Law`
/// with subject `Gen. Bus.` under its `N.Y. Law` entry. Only the first token
/// of a span survives, and the extractor order that decides it is arbitrary
/// (eyecite's varies from run to run), so the longer reporter, the more
/// specific entry, goes first (#49).
fn law_reporter_len(token: &Token) -> usize {
    match token {
        Token::Citation(ct)
            if ct
                .exact_editions
                .iter()
                .chain(&ct.variation_editions)
                .next()
                .is_some()
                && ct
                    .exact_editions
                    .iter()
                    .chain(&ct.variation_editions)
                    .all(|e| e.reporter.source == ReporterSource::Laws) =>
        {
            ct.token_data
                .groups
                .get("reporter")
                .and_then(|r| r.as_deref())
                .map_or(0, str::len)
        }
        _ => 0,
    }
}

/// Core tokenization logic shared by `Tokenizer` and `AhocorasickTokenizer`.
///
/// Takes a list of raw tokens, sorts them, resolves overlaps, fills gaps
/// with plain text, and returns the final token list plus citation positions.
fn tokenize_impl(raw_tokens: &[Token], text: &str) -> (Tokens, Vec<(usize, Token)>) {
    // Sort by (start ascending, end descending), then put the law token with
    // the longest reporter first among those with the same span.
    let mut sorted: Vec<&Token> = raw_tokens.iter().collect();
    sorted.sort_by(|a, b| {
        a.start()
            .cmp(&b.start())
            .then_with(|| b.end().cmp(&a.end()))
            .then_with(|| law_reporter_len(b).cmp(&law_reporter_len(a)))
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

    reporter_ids(all_tokens, citation_tokens)
}

/// `69 id. 76`: "id." standing for the reporter of the case citation before
/// it (#129). Older opinions cite a run of cases in one reporter as `Langdon
/// v. The Mayor, 92 N.Y. 427; Thomas v. Thomas, 18 id. 481`, and a second
/// case in the same volume as `Smith v. Edwards, 88 id. 92; Delaney v.
/// McCormack, id. 174`. eyecite takes each `id.` for an id. citation. Here a
/// volume, `id.` and a page become one full citation token with the editions
/// and reporter of the citation token before it, so the rest of the pipeline
/// reads `18 N.Y. 481`; and `id.` and a page after a case name (`Delaney v.
/// McCormack, id. 174`) become one with that citation's volume too, `88
/// N.Y. 174`.
///
/// It applies only when the citation token before the `id.` is a case
/// reporter's (a full or short case citation). After a statute or a journal
/// the `id.` stands for that, and stays an id. citation, as it does with no
/// citation before it. Without a volume, it applies only after a case name
/// (`v.`, `In re`, `Matter of`, `ex rel.`), so `Id. 174` after a citation is
/// still that citation's id. citation.
fn reporter_ids(
    all_tokens: Tokens,
    citation_tokens: Vec<(usize, Token)>,
) -> (Tokens, Vec<(usize, Token)>) {
    fn word(tokens: &Tokens, i: usize) -> Option<&str> {
        match tokens.get(i)? {
            TokenOrStr::Str(s) => Some(s),
            TokenOrStr::Token(_) => None,
        }
    }
    fn is_case(ct: &CitationTokenData) -> bool {
        let mut eds = ct
            .exact_editions
            .iter()
            .chain(&ct.variation_editions)
            .peekable();
        eds.peek().is_some() && eds.all(|e| e.reporter.source == ReporterSource::Reporters)
    }
    // A volume: up to four digits, after an optional opening bracket.
    fn volume(w: &str) -> Option<(&str, &str)> {
        let digits = w.trim_start_matches(['(', '[']);
        let ok = (1..=4).contains(&digits.len()) && digits.bytes().all(|b| b.is_ascii_digit());
        ok.then(|| (&w[..w.len() - digits.len()], digits))
    }
    // A page: up to five digits at the start of the word.
    fn page(w: &str) -> Option<(&str, &str)> {
        let n = w.bytes().take_while(u8::is_ascii_digit).count();
        let next = w[n..].chars().next();
        ((1..=5).contains(&n) && !next.is_some_and(char::is_alphanumeric)).then(|| w.split_at(n))
    }
    // A case name in the clause before word `to`: `v.`, `In re`, `Matter
    // of`, `ex rel.` within the 15 words before it, back to `from` (the
    // citation before it), a `;` or a closing parenthesis. `v.`, `vs.` and
    // `re` are stop word tokens.
    fn case_name_before(tokens: &Tokens, from: usize, to: usize) -> bool {
        for j in (from.max(to.saturating_sub(30))..to).rev() {
            let w = match tokens.get(j) {
                Some(TokenOrStr::Str(s)) => s.as_str(),
                Some(TokenOrStr::Token(Token::StopWord(t))) => t.data.as_str(),
                _ => return false,
            };
            let key = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_ascii_lowercase();
            // `In re` is one stop word token.
            let last = key.rsplit(' ').next().unwrap_or_default();
            if matches!(last, "v" | "vs" | "re" | "matter" | "rel") {
                return true;
            }
            if j + 2 < to && (w.ends_with(';') || w.ends_with(')') || w.ends_with(":")) {
                return false;
            }
        }
        false
    }
    // Pin cites and commas between the citations of a parallel group.
    fn only_pins(tokens: &Tokens, from: usize, to: usize) -> bool {
        (from..to).all(|j| match tokens.get(j) {
            Some(TokenOrStr::Str(s)) => s
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, ' ' | ',' | '-' | '\u{2013}')),
            _ => false,
        })
    }
    // A citation the tokenizer didn't recognize between two word indexes:
    // a bare volume and a capitalized word after it (`3 Durnford & East,
    // 454`, `3 How. Pr. R. 297`). The `id.` after it stands for that
    // reporter, not the recognized one before it.
    fn unrecognized_cite_between(tokens: &Tokens, from: usize, to: usize) -> bool {
        (from..to.saturating_sub(2)).any(|j| {
            word(tokens, j).and_then(volume).is_some()
                && word(tokens, j + 1) == Some(" ")
                && word(tokens, j + 2)
                    .is_some_and(|w| w.chars().next().is_some_and(char::is_uppercase))
        })
    }
    fn group(ct: &CitationTokenData, name: &str) -> Option<String> {
        ct.token_data.groups.get(name).cloned().flatten()
    }

    if !citation_tokens
        .iter()
        .any(|(_, t)| matches!(t, Token::Id(_)))
    {
        return (all_tokens, citation_tokens);
    }

    // The id. tokens that become citations, by the index of their first
    // word: the index of their last word, the id. token's index, the new
    // token, and the text left over before and after it.
    struct Merge {
        last: usize,
        id: usize,
        token: Token,
        before: String,
        after: String,
    }
    let mut merges: BTreeMap<usize, Merge> = BTreeMap::new();
    let mut drops: std::collections::HashSet<usize> = std::collections::HashSet::new();
    // The citation token before each id. token: a merged one counts, so
    // `90 id. 5; Delaney v. Roe, id. 174` is 90 N.Y. 174.
    let mut previous: Option<(usize, CitationTokenData)> = None;
    for (i, token) in &citation_tokens {
        let id = match token {
            // A citation with no volume (a nominative reporter's,
            // `Thompson, 86`, which is as often a party's name) can't be
            // what a volume and `id.` stand for.
            Token::Citation(ct) if group(ct, "volume").is_none_or(|v| v.is_empty()) => continue,
            Token::Citation(ct) => {
                // In a parallel group, "id." stands for the first
                // reporter: `74 W. Va. 569, 82 S. E. 614; Cook v. Raleigh
                // Lumber Co., Id. 503, 82 S. E. 327` is 74 W. Va. 503.
                let parallel = previous.as_ref().is_some_and(|(p, prev)| {
                    is_case(prev) && is_case(ct) && only_pins(&all_tokens, p + 1, *i)
                });
                if parallel {
                    let (_, first) = previous.take().unwrap();
                    previous = Some((*i, first));
                } else {
                    previous = Some((*i, ct.clone()));
                }
                continue;
            }
            Token::Id(id) => id,
            _ => continue,
        };
        let i = *i;
        let Some((prev_index, prev)) = previous.as_ref().filter(|(_, ct)| is_case(ct)) else {
            continue;
        };
        let prev_index = *prev_index;
        if word(&all_tokens, i + 1) != Some(" ") || i < 2 || word(&all_tokens, i - 1) != Some(" ") {
            continue;
        }
        let Some((pg, after)) = word(&all_tokens, i + 2).and_then(page) else {
            continue;
        };
        let before_word = word(&all_tokens, i - 2);
        // After a citation the tokenizer didn't recognize, the reporter the
        // `id.` stands for is unknown. It still isn't an id. citation, so it
        // goes back to plain text rather than resolve to the wrong case.
        if unrecognized_cite_between(&all_tokens, prev_index + 1, i - 1) {
            let shape = before_word.and_then(volume).is_some()
                || (before_word.is_some_and(|w| w.ends_with(','))
                    && case_name_before(&all_tokens, prev_index + 1, i - 2));
            if shape {
                drops.insert(i);
            }
            continue;
        }
        let (first, start, vol, before) = match before_word.and_then(volume) {
            Some((before, vol)) => (i - 2, id.start - 1 - vol.len(), vol.to_string(), before),
            None if before_word.is_some_and(|w| w.ends_with(','))
                && case_name_before(&all_tokens, prev_index + 1, i - 2) =>
            {
                let Some(vol) = group(prev, "volume") else {
                    continue;
                };
                (i, id.start, vol, "")
            }
            None => continue,
        };
        let data = if first == i {
            format!("{} {pg}", id.data)
        } else {
            format!("{vol} {} {pg}", id.data)
        };
        let groups = HashMap::from([
            ("volume".to_string(), Some(vol)),
            ("reporter".to_string(), group(prev, "reporter")),
            ("page".to_string(), Some(pg.to_string())),
        ]);
        let ct = CitationTokenData {
            token_data: TokenData {
                data,
                start,
                end: id.end + 1 + pg.len(),
                groups,
            },
            exact_editions: prev.exact_editions.clone(),
            variation_editions: prev.variation_editions.clone(),
            short: false,
        };
        let token = Token::Citation(ct.clone());
        merges.insert(
            first,
            Merge {
                last: i + 2,
                id: i,
                token,
                before: before.to_string(),
                after: after.to_string(),
            },
        );
        previous = Some((i, ct));
    }
    if merges.is_empty() && drops.is_empty() {
        return (all_tokens, citation_tokens);
    }

    // Rebuild the word list with each merge's words replaced by its token,
    // and renumber the citation tokens.
    let mut words: Tokens = Vec::with_capacity(all_tokens.len());
    let mut new_index: HashMap<usize, usize> = HashMap::new();
    let mut tokens_by_id: HashMap<usize, Token> = HashMap::new();
    let mut old = all_tokens.into_iter().enumerate();
    while let Some((j, t)) = old.next() {
        let Some(m) = merges.remove(&j) else {
            match t {
                TokenOrStr::Token(Token::Id(id)) if drops.contains(&j) => {
                    words.push(TokenOrStr::Str(id.data));
                }
                t => {
                    new_index.insert(j, words.len());
                    words.push(t);
                }
            }
            continue;
        };
        if m.last > j {
            old.nth(m.last - j - 1);
        }
        if !m.before.is_empty() {
            words.push(TokenOrStr::Str(m.before));
        }
        new_index.insert(m.id, words.len());
        words.push(TokenOrStr::Token(m.token.clone()));
        tokens_by_id.insert(m.id, m.token);
        if !m.after.is_empty() {
            words.push(TokenOrStr::Str(m.after));
        }
    }
    let citations = citation_tokens
        .into_iter()
        .filter(|(i, _)| !drops.contains(i))
        .map(|(i, t)| (new_index[&i], tokens_by_id.remove(&i).unwrap_or(t)))
        .collect();
    (words, citations)
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
    use crate::models::edition::Reporter;
    use std::collections::HashSet;

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

    /// Mirrors eyecite's updated `test_extractor_filter` (eyecite #305): the
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

    /// The prefilter returns the selected extractors in registry order, the
    /// order the base `Tokenizer` (and eyecite's) runs them in, so tokens
    /// with equal spans reach the merge and overlap steps in the same order
    /// on every run and in every process (#22).
    #[test]
    fn test_extractor_order_is_registry_order() {
        let text = "See foo, 123 U.S. 456; 5 F.3d 17; 2 Mass. 3; 1 Johns. 2; \
                    100 N. Y. S. 2d 4; 22 U.S.C. § 1983. Id. at 5; supra.";
        let mut first: Option<Vec<usize>> = None;
        for _ in 0..4 {
            let tokenizer = AhocorasickTokenizer::new();
            let indices: Vec<usize> = tokenizer
                .get_extractors(text)
                .into_iter()
                .map(|e| {
                    tokenizer
                        .extractors
                        .iter()
                        .position(|x| std::ptr::eq(x, e))
                        .expect("selected extractor is in the registry")
                })
                .collect();
            assert!(indices.len() > 10, "too few extractors selected");
            assert!(
                indices.windows(2).all(|w| w[0] < w[1]),
                "extractors not in registry order: {indices:?}"
            );
            match &first {
                None => first = Some(indices),
                Some(f) => assert_eq!(f, &indices),
            }
        }
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
                slug: None,
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

    /// Mirrors eyecite's updated `test_tokenize`: "U. S." is both a
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
            slug: None,
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
            slug: None,
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
        let unmatched = crate::data::get()
            .bundle
            .editions
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
        let editions = &crate::data::get().bundle.editions;
        let us_edition = editions
            .iter()
            .find(|e| e.short_name == "U.S." && e.harvard_slug.as_deref() == Some("us"));
        assert!(
            us_edition.is_some(),
            "the data bundle should contain U.S. with harvard_slug = Some(\"us\")"
        );
        let a2d_edition = editions
            .iter()
            .find(|e| e.short_name == "A.2d" && e.harvard_slug.as_deref() == Some("a2d"));
        assert!(
            a2d_edition.is_some(),
            "the data bundle should contain A.2d with harvard_slug = Some(\"a2d\")"
        );
    }
}
