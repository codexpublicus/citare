//! Tokenizer: extract citation tokens from legal text.
//!
//! Ported from eyecite's `tokenizers.py`. Provides both a simple [`Tokenizer`]
//! and a performance-optimised [`AhocorasickTokenizer`] that pre-filters
//! extractors using Aho-Corasick string matching.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use aho_corasick::AhoCorasick;
use regex::Regex;

use crate::models::edition::{Edition, Reporter, ReporterSource};
use crate::models::token::{CitationTokenData, Token, TokenData, TokenOrStr, Tokens};
use crate::regexes::{
    nonalphanum_boundaries_re, short_cite_re, ID_REGEX, PAGE_NUMBER_REGEX, PARAGRAPH_REGEX,
    PLACEHOLDER_CITATIONS, SECTION_REGEX, STOP_WORD_REGEX, STOP_WORDS,
    SUPRA_REGEX,
};

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
pub struct TokenExtractor {
    /// The original regex string (before compilation).
    pub regex: String,
    /// The compiled regex (standard `regex` crate).
    pub compiled_regex: Regex,
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
    /// Create a token from a regex match.
    ///
    /// Re-runs the regex with captures to extract named groups.
    /// Uses capture group 1 (the inner capture from `nonalphanum_boundaries_re`)
    /// for the token's data span, falling back to the full match if no group 1.
    pub fn make_token(&self, m: regex::Match, text: &str) -> Option<Token> {
        let match_str = &text[m.start()..m.end()];
        let caps = self.compiled_regex.captures(match_str)?;

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
        for name in self.compiled_regex.capture_names().flatten() {
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
    "Thompson", "Cooke", "Holmes", "Olcott", "Chase", "Gilmer", "Bee",
    "Deady", "Taney",
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
// Extractor builder
// ---------------------------------------------------------------------------

/// Embedded JSON data files.
const REPORTERS_JSON: &str =
    include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
const LAWS_JSON: &str =
    include_str!("../../../data/reporters-db/reporters_db/data/laws.json");
const JOURNALS_JSON: &str =
    include_str!("../../../data/reporters-db/reporters_db/data/journals.json");
const REGEXES_JSON: &str =
    include_str!("../../../data/reporters-db/reporters_db/data/regexes.json");

/// Global lazily-initialised extractors.
pub static EXTRACTORS: LazyLock<Vec<TokenExtractor>> = LazyLock::new(build_extractors);

/// Global lazily-initialised editions lookup (reporter string -> editions).
pub static EDITIONS_LOOKUP: LazyLock<HashMap<String, Vec<Edition>>> =
    LazyLock::new(|| build_extractors_and_lookup().1);

/// Internal data structure for accumulating editions by regex.
#[derive(Debug, Clone, Default)]
struct EditionsByRegexEntry {
    editions: Vec<Edition>,
    variations: Vec<Edition>,
    strings: HashSet<String>,
    short: bool,
}

/// Build all extractors. This is the main entry point called once by `LazyLock`.
pub fn build_extractors() -> Vec<TokenExtractor> {
    build_extractors_and_lookup().0
}

/// Build extractors and the editions lookup table together.
fn build_extractors_and_lookup() -> (Vec<TokenExtractor>, HashMap<String, Vec<Edition>>) {
    // 1. Load and process regex variables with overrides
    let mut raw: serde_json::Value =
        serde_json::from_str(REGEXES_JSON).expect("Failed to parse regexes.json");

    // Apply the Python overrides before processing:
    //   raw_regex_variables["full_cite"][""] = "$volume $reporter,? $page"
    //   raw_regex_variables["page"][""] = rf"(?P<page>{PAGE_NUMBER_REGEX})"
    if let Some(obj) = raw.as_object_mut() {
        if let Some(full_cite) = obj.get_mut("full_cite") {
            if let Some(fc_obj) = full_cite.as_object_mut() {
                fc_obj.insert(
                    String::new(),
                    serde_json::Value::String("$volume $reporter,? $page".to_string()),
                );
            }
        }
        if let Some(page) = obj.get_mut("page") {
            if let Some(page_obj) = page.as_object_mut() {
                page_obj.insert(
                    String::new(),
                    serde_json::Value::String(format!(
                        "(?P<page>{})",
                        &*PAGE_NUMBER_REGEX
                    )),
                );
            }
        }
    }

    let regex_variables = citare_data_build::template::process_variables(&raw);

    // 2. Parse data files
    let reporters =
        citare_data_build::reporters::parse_reporters(REPORTERS_JSON)
            .expect("Failed to parse reporters.json");
    let laws = citare_data_build::reporters::parse_laws(LAWS_JSON)
        .expect("Failed to parse laws.json");
    let journals =
        citare_data_build::reporters::parse_journals(JOURNALS_JSON)
            .expect("Failed to parse journals.json");

    // 3. Build editions_by_regex and editions_lookup
    let mut editions_by_regex: HashMap<String, EditionsByRegexEntry> = HashMap::new();
    let mut editions_lookup: HashMap<String, Vec<Edition>> = HashMap::new();

    // Helper: substitute $edition in a template with escaped edition names
    fn substitute_edition(template: &str, edition_names: &[&str]) -> String {
        let edition = edition_names
            .iter()
            .map(|e| regex::escape(e))
            .collect::<Vec<_>>()
            .join("|");
        citare_data_build::template::safe_substitute(
            template,
            &HashMap::from([("edition".to_string(), edition)]),
        )
    }

    // Helper: add a regex for an edition
    let add_regex = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                     editions_lookup: &mut HashMap<String, Vec<Edition>>,
                     kind: &str,
                     reporter_strings: &[&str],
                     edition: &Edition,
                     regex: &str| {
        for &reporter in reporter_strings {
            editions_lookup
                .entry(reporter.to_string())
                .or_default()
                .push(edition.clone());
        }

        let entry = editions_by_regex.entry(regex.to_string()).or_default();
        match kind {
            "editions" => entry.editions.push(edition.clone()),
            "variations" => entry.variations.push(edition.clone()),
            _ => {}
        }

        // Check if the escaped first reporter string appears in the regex
        let have_strings = regex.contains(&regex::escape(reporter_strings[0]));
        if have_strings {
            for &s in reporter_strings {
                entry.strings.insert(s.to_string());
            }
        }

        // Add short cite variant
        let short_regex = short_cite_re(regex);
        if short_regex != *regex {
            let short_entry = editions_by_regex
                .entry(short_regex.clone())
                .or_default();
            match kind {
                "editions" => short_entry.editions.push(edition.clone()),
                "variations" => short_entry.variations.push(edition.clone()),
                _ => {}
            }
            short_entry.short = true;
            if have_strings {
                for &s in reporter_strings {
                    short_entry.strings.insert(s.to_string());
                }
            }
        }
    };

    // Helper: expand regex templates for an edition.
    //
    // Mirrors Python's `substitute_editions()` which returns a SEPARATE regex
    // for each edition string (the canonical name + each variation that maps
    // to this edition). This is critical: each variation gets its own compiled
    // regex and its own TokenExtractor so the AC automaton can pre-filter by
    // the variation string.
    let add_regexes = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                       editions_lookup: &mut HashMap<String, Vec<Edition>>,
                       regex_templates: &[String],
                       edition_name: &str,
                       edition: &Edition,
                       variation_names: &[String],
                       regex_variables: &HashMap<String, String>| {
        for template in regex_templates {
            let resolved = citare_data_build::template::recursive_substitute(
                template,
                regex_variables,
                100,
            )
            .unwrap_or_else(|e| {
                log::warn!("Failed to resolve template '{}': {}", template, e);
                template.clone()
            });

            // Fix Python-ism: {,N} → {0,N} (Rust regex requires explicit lower bound)
            let resolved = resolved.replace("{,", "{0,");

            // If the template doesn't contain $edition, there's only one regex
            if !resolved.contains("$edition") && !resolved.contains("${edition}") {
                add_regex(
                    editions_by_regex,
                    editions_lookup,
                    "editions",
                    &[edition_name],
                    edition,
                    &resolved,
                );
                continue;
            }

            // Generate separate regex for the canonical edition name
            let regex = substitute_edition(&resolved, &[edition_name]);
            add_regex(
                editions_by_regex,
                editions_lookup,
                "editions",
                &[edition_name],
                edition,
                &regex,
            );

            // Generate a SEPARATE regex for EACH variation (not combined)
            for var_name in variation_names {
                let var_regex = substitute_edition(&resolved, &[var_name.as_str()]);
                add_regex(
                    editions_by_regex,
                    editions_lookup,
                    "variations",
                    &[var_name.as_str()],
                    edition,
                    &var_regex,
                );
            }
        }
    };

    // --- Add reporters.json ---
    for (source_key, source_cluster) in &reporters {
        for source in source_cluster {
            let reporter_obj = Reporter::new(
                source_key.clone(),
                source.name.clone(),
                source.cite_type.clone(),
                ReporterSource::Reporters,
            );

            for (edition_name, edition_data) in &source.editions {
                let edition = Edition {
                    reporter: reporter_obj.clone(),
                    short_name: edition_name.clone(),
                    start: edition_data.start.clone(),
                    end: edition_data.end.clone(),
                };

                let regex_templates: Vec<String> = edition_data
                    .regexes
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .cloned()
                    .unwrap_or_else(|| vec!["$full_cite".to_string()]);

                // Find variations that map to this edition
                let edition_variations: Vec<String> = source
                    .variations
                    .iter()
                    .filter(|(_, v)| v.as_str() == edition_name)
                    .map(|(k, _)| k.clone())
                    .collect();

                add_regexes(
                    &mut editions_by_regex,
                    &mut editions_lookup,
                    &regex_templates,
                    edition_name,
                    &edition,
                    &edition_variations,
                    &regex_variables,
                );
            }
        }
    }

    // --- Add laws.json ---
    for (source_key, source_cluster) in &laws {
        for source in source_cluster {
            let reporter_obj = Reporter::new(
                source_key.clone(),
                source.name.clone(),
                source.cite_type.clone(),
                ReporterSource::Laws,
            );

            let edition = Edition {
                reporter: reporter_obj,
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
            };

            let regex_templates: Vec<String> = if source.regexes.is_empty() {
                vec!["$full_cite".to_string()]
            } else {
                // Replace "§ " with "§§? ?" to handle citations to multiple sections
                source
                    .regexes
                    .iter()
                    .map(|r| r.replace("§ ", "§§? ?"))
                    .collect()
            };

            let variation_names: Vec<String> = source.variations.clone();

            add_regexes(
                &mut editions_by_regex,
                &mut editions_lookup,
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &regex_variables,
            );
        }
    }

    // --- Add journals.json ---
    for (source_key, source_cluster) in &journals {
        for source in source_cluster {
            let reporter_obj = Reporter::new(
                source_key.clone(),
                source.name.clone(),
                source.cite_type.clone(),
                ReporterSource::Journals,
            );

            let edition = Edition {
                reporter: reporter_obj,
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
            };

            let regex_templates: Vec<String> = if source.regexes.is_empty() {
                vec!["$full_cite".to_string()]
            } else {
                source.regexes.clone()
            };

            let variation_names: Vec<String> = source.variations.clone();

            add_regexes(
                &mut editions_by_regex,
                &mut editions_lookup,
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &regex_variables,
            );
        }
    }

    // 4. Convert editions_by_regex into TokenExtractors
    let mut extractors: Vec<TokenExtractor> = Vec::new();

    for (regex, cluster) in &editions_by_regex {
        let wrapped = nonalphanum_boundaries_re(regex);
        match Regex::new(&wrapped) {
            Ok(compiled) => {
                extractors.push(TokenExtractor {
                    regex: wrapped,
                    compiled_regex: compiled,
                    token_type: TokenType::Citation,
                    extra: ExtractorExtra {
                        exact_editions: cluster.editions.clone(),
                        variation_editions: cluster.variations.clone(),
                        short: cluster.short,
                    },
                    strings: cluster.strings.iter().cloned().collect(),
                    case_insensitive: false,
                });
            }
            Err(e) => {
                log::warn!(
                    "Skipping invalid citation regex: {} (error: {})",
                    &wrapped[..wrapped.len().min(80)],
                    e
                );
            }
        }
    }

    // 5. Add one-off extractors
    let oneoff_specs: Vec<(String, TokenType, bool, Vec<String>)> = vec![
        // Id
        (
            ID_REGEX.clone(),
            TokenType::Id,
            true,
            vec!["id.".to_string(), "ibid.".to_string()],
        ),
        // Supra
        (
            SUPRA_REGEX.clone(),
            TokenType::Supra,
            true,
            vec!["supra".to_string()],
        ),
        // Paragraph
        (PARAGRAPH_REGEX.to_string(), TokenType::Paragraph, false, vec![]),
        // Stop words
        (
            STOP_WORD_REGEX.clone(),
            TokenType::StopWord,
            true,
            STOP_WORDS.iter().map(|s| s.to_string()).collect(),
        ),
        // Placeholder citations
        (
            PLACEHOLDER_CITATIONS.clone(),
            TokenType::Placeholder,
            true,
            vec![],
        ),
        // Section symbols
        (
            SECTION_REGEX.to_string(),
            TokenType::Section,
            false,
            vec!["\u{00A7}".to_string()],
        ),
    ];

    for (regex_str, token_type, case_insensitive, strings) in oneoff_specs {
        let regex_with_flags = if case_insensitive {
            format!("(?i){}", regex_str)
        } else {
            regex_str
        };
        match Regex::new(&regex_with_flags) {
            Ok(compiled) => {
                extractors.push(TokenExtractor {
                    regex: regex_with_flags,
                    compiled_regex: compiled,
                    token_type,
                    extra: ExtractorExtra::default(),
                    strings,
                    case_insensitive,
                });
            }
            Err(e) => {
                log::warn!(
                    "Skipping invalid one-off regex for {:?}: {}",
                    token_type,
                    e
                );
            }
        }
    }

    (extractors, editions_lookup)
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
            for m in extractor.compiled_regex.find_iter(text) {
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
                        .entry(s.to_lowercase())
                        .or_default()
                        .push(idx);
                }
            } else {
                for s in &ext.strings {
                    cs_grouped.entry(s.clone()).or_default().push(idx);
                }
            }
        }

        // Build case-sensitive AC
        let (case_sensitive_ac, case_sensitive_map) = if cs_grouped.is_empty() {
            (None, Vec::new())
        } else {
            let patterns: Vec<String> = cs_grouped.keys().cloned().collect();
            let map: Vec<Vec<usize>> = patterns
                .iter()
                .map(|p| cs_grouped[p].clone())
                .collect();
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
            let map: Vec<Vec<usize>> = patterns
                .iter()
                .map(|p| ci_grouped[p].clone())
                .collect();
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
    fn get_extractors(&self, text: &str) -> Vec<&TokenExtractor> {
        let mut result: HashSet<usize> =
            self.unfiltered_indices.iter().copied().collect();

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
            for m in extractor.compiled_regex.find_iter(text) {
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
                // Also update the citation_tokens entry if it was a citation
                if let Some((_, ref mut ct)) = citation_tokens.last_mut() {
                    ct.merge(token);
                }
                continue;
            }
        }

        if offset > token.start() {
            // Overlapping match
            if let Some(ref lt) = last_token {
                if matches!(token, Token::Citation(_))
                    && token_is_from_nominative_reporter(lt)
                {
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
        assert!(
            !section_tokens.is_empty(),
            "Should find a Section token"
        );
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
}
