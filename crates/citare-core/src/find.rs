//! Citation finding pipeline.
//!
//! This is the main entry point for extracting citations from legal text.
//! Ported from eyecite's `find.py`.

use std::collections::HashMap;

use crate::clean::CleanStep;
use crate::helpers::{
    add_journal_metadata, add_law_metadata, add_post_citation, add_pre_citation, add_pre_cite_year,
    disambiguate_reporters, extract_pin_cite, extract_section_pin_cite, filter_citations,
    find_case_name, find_case_name_in_html, find_case_name_in_html_short, find_case_name_short,
    guess_court, guess_edition, joins_nameless_parallel_cites, joins_parallel_cites,
    start_of_citation,
};
use crate::models::citation::{
    normalize_blank_page, CaseMetadata, Citation, CitationCommon, FullCaseCitationData,
    FullCaseMetadata, FullJournalCitationData, FullLawCitationData, IdCitationData, IdMetadata,
    JournalMetadata, LawMetadata, ResourceCitationCommon, ShortCaseCitationData, ShortCaseMetadata,
    SupraCitationData, SupraMetadata, UnknownCitationData,
};
use crate::models::document::Document;
use crate::models::edition::ReporterSource;
use crate::models::token::{Token, TokenOrStr, Tokens};
use crate::regexes::{SUPRA_ANTECEDENT_GROUPS, SUPRA_ANTECEDENT_REGEX};
use crate::tokenizer::{Tokenize, DEFAULT_TOKENIZER};

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Extract citations from legal text.
///
/// This is the main workhorse function. Given a string of text (e.g., a
/// judicial opinion or other legal document), return a list of `Citation`
/// objects representing the citations found in the document.
///
/// # Arguments
///
/// * `plain_text` - The text to parse. You may wish to use `clean_text`
///   to pre-process your text before passing it here.
/// * `remove_ambiguous` - Whether to remove citations that might refer to
///   more than one reporter and can't be narrowed down by date.
/// * `markup_text` - If the source text has markup (XML or HTML), pass it
///   here instead of `plain_text`. The clean steps (which must include
///   `Html`; it is added if missing) turn it into the plain text that
///   citations are found in, and their spans index that plain text.
/// * `clean_steps` - Cleanup steps to apply before tokenization.
pub fn get_citations(
    plain_text: &str,
    remove_ambiguous: bool,
    markup_text: &str,
    clean_steps: &[CleanStep],
) -> Vec<Citation> {
    let mut document = Document::new(plain_text, markup_text, clean_steps);
    get_citations_from_document(&mut document, remove_ambiguous)
}

/// Extract citations from a [`Document`] built with [`Document::new`].
///
/// [`get_citations`] is this plus `Document::new`. Use it when you need the
/// document afterwards: its `plain_text` is the cleaned text the citation
/// spans index, and for markup input `plain_to_markup` maps those spans
/// onto the markup.
pub fn get_citations_from_document(
    document: &mut Document,
    remove_ambiguous: bool,
) -> Vec<Citation> {
    get_citations_from_document_with(document, remove_ambiguous, &*DEFAULT_TOKENIZER)
}

/// The options of [`get_citations_with`]: eyecite's `get_citations`
/// keyword arguments.
///
/// Build one with [`FindOptions::new`] (or `default()`) and the builder
/// methods. New options may be added in minor releases, so the struct
/// can't be built with a literal outside citare.
///
/// ```rust
/// use citare_core::{get_citations_with, CleanStep, FindOptions};
///
/// let steps = [CleanStep::Html, CleanStep::AllWhitespace];
/// let options = FindOptions::new()
///     .markup_text("<p>See <i>Foo v. Bar</i>, 1 U.S. 1 (1999).</p>")
///     .clean_steps(&steps);
/// let citations = get_citations_with("", &options);
/// assert_eq!(citations[0].matched_text(), "1 U.S. 1");
/// ```
#[derive(Clone, Copy, Default)]
#[non_exhaustive]
pub struct FindOptions<'a> {
    /// Drop citations that might refer to more than one reporter and can't
    /// be narrowed down by date.
    pub remove_ambiguous: bool,
    /// HTML or XML to find citations in, instead of the plain text (see
    /// [`get_citations`]).
    pub markup_text: &'a str,
    /// Cleanup steps applied before tokenization.
    pub clean_steps: &'a [CleanStep],
    /// The tokenizer to use; `None` is the shared [`DEFAULT_TOKENIZER`].
    pub tokenizer: Option<&'a dyn Tokenize>,
}

impl<'a> FindOptions<'a> {
    /// The defaults: plain text, no cleaning, ambiguous citations kept, the
    /// default tokenizer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set [`remove_ambiguous`](Self::remove_ambiguous).
    pub fn remove_ambiguous(mut self, remove_ambiguous: bool) -> Self {
        self.remove_ambiguous = remove_ambiguous;
        self
    }

    /// Set [`markup_text`](Self::markup_text).
    pub fn markup_text(mut self, markup_text: &'a str) -> Self {
        self.markup_text = markup_text;
        self
    }

    /// Set [`clean_steps`](Self::clean_steps).
    pub fn clean_steps(mut self, clean_steps: &'a [CleanStep]) -> Self {
        self.clean_steps = clean_steps;
        self
    }

    /// Set [`tokenizer`](Self::tokenizer).
    pub fn tokenizer(mut self, tokenizer: &'a dyn Tokenize) -> Self {
        self.tokenizer = Some(tokenizer);
        self
    }
}

impl std::fmt::Debug for FindOptions<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FindOptions")
            .field("remove_ambiguous", &self.remove_ambiguous)
            .field("markup_text", &self.markup_text)
            .field("clean_steps", &self.clean_steps.len())
            .field("tokenizer", &self.tokenizer.map(|_| "custom"))
            .finish()
    }
}

/// Extract citations from legal text with [`FindOptions`].
///
/// eyecite's `get_citations` with all of its arguments, including a custom
/// `tokenizer`. [`get_citations`] is this with the default tokenizer.
pub fn get_citations_with(plain_text: &str, options: &FindOptions<'_>) -> Vec<Citation> {
    let mut document = Document::new(plain_text, options.markup_text, options.clean_steps);
    let tokenizer = options.tokenizer.unwrap_or(&*DEFAULT_TOKENIZER);
    get_citations_from_document_with(&mut document, options.remove_ambiguous, tokenizer)
}

/// [`get_citations_from_document`] with a custom tokenizer.
pub fn get_citations_from_document_with(
    document: &mut Document,
    remove_ambiguous: bool,
    tokenizer: &dyn Tokenize,
) -> Vec<Citation> {
    // 1. Tokenize
    document.tokenize_with(tokenizer);

    // 2. For each citation token, extract citation
    let mut citations: Vec<Citation> = Vec::new();

    // We need to collect citation_tokens info before processing since we
    // need mutable access to words during processing but the token info
    // is derived from that same data.
    let citation_token_info: Vec<(usize, Token)> = document.citation_tokens.clone();

    // Section tokens taken as an id. citation's pin cite, which must not
    // also become unknown citations.
    let mut section_pins: Vec<usize> = Vec::new();

    // Positions in `citations` of full case citations to look for
    // references to.
    let mut reference_sources: Vec<usize> = Vec::new();

    for (i, token) in &citation_token_info {
        let i = *i;
        match token {
            // CASE 1: Citation token (reporter, law journal, or law)
            Token::Citation(ct) => {
                if ct.short {
                    let citation = extract_shortform_citation(document, i);
                    citations.push(citation);
                } else {
                    let mut citation = extract_full_citation(document, i);

                    // Check for parallel citations
                    if !citations.is_empty() {
                        if let (Citation::FullCase(_), Some(preceding @ Citation::FullCase(_))) =
                            (&citation, citations.last_mut())
                        {
                            handle_parallel_citation(&mut citation, preceding, &document.words);
                        }
                    }

                    // Reference citations that follow a full case
                    // citation, by its plaintiff or defendant, go just
                    // before it. They are found once every full citation
                    // is known, so the text is searched for all names in
                    // one pass (see `ReferenceFinder`).
                    if matches!(citation, Citation::FullCase(_)) {
                        reference_sources.push(citations.len());
                    }

                    citations.push(citation);
                }
            }

            // CASE 2: Id/Ibid citation
            Token::Id(_) => {
                let (citation, section_indexes) = extract_id_citation(&document.words, i);
                section_pins.extend(section_indexes);
                citations.push(citation);
            }

            // CASE 3: Supra citation
            Token::Supra(_) => {
                let citation = extract_supra_citation(&document.words, i);
                citations.push(citation);
            }

            // CASE 4: Section marker -> Unknown citation, unless an id.
            // citation took it as its pin cite
            Token::Section(_) if section_pins.contains(&i) => continue,
            Token::Section(_) => {
                let common = CitationCommon {
                    token: token.clone(),
                    index: i,
                    span_start: None,
                    span_end: None,
                    full_span_start: None,
                    full_span_end: None,
                    groups: token.groups().clone(),
                };
                citations.push(Citation::Unknown(UnknownCitationData { common }));
            }

            // CASE 5: Not a citation
            _ => continue,
        }
    }

    let citations = insert_reference_citations(citations, &reference_sources, document);

    // 3. Filter and sort
    let mut citations = filter_citations(citations);

    if remove_ambiguous {
        citations = disambiguate_reporters(citations);
    }

    citations
}

// ---------------------------------------------------------------------------
// Extract full citation
// ---------------------------------------------------------------------------

fn extract_full_citation(document: &Document, index: usize) -> Citation {
    let token = match &document.words[index] {
        TokenOrStr::Token(t) => t.clone(),
        _ => unreachable!("Expected token at citation index"),
    };

    let ct = match &token {
        Token::Citation(ct) => ct,
        _ => unreachable!("Expected citation token"),
    };

    // Determine citation source type
    let editions = if !ct.exact_editions.is_empty() {
        &ct.exact_editions
    } else {
        &ct.variation_editions
    };

    let cite_sources: std::collections::HashSet<ReporterSource> =
        editions.iter().map(|e| e.reporter.source).collect();

    let mut groups = token.groups().clone();
    normalize_blank_page(&mut groups);
    let all_editions: Vec<_> = ct
        .exact_editions
        .iter()
        .chain(ct.variation_editions.iter())
        .cloned()
        .collect();

    if cite_sources.contains(&ReporterSource::Laws) {
        // Full law citation
        let mut data = FullLawCitationData {
            resource: ResourceCitationCommon {
                common: CitationCommon {
                    token: token.clone(),
                    index,
                    span_start: None,
                    span_end: None,
                    full_span_start: None,
                    full_span_end: None,
                    groups,
                },
                exact_editions: ct.exact_editions.clone(),
                variation_editions: ct.variation_editions.clone(),
                all_editions,
                edition_guess: None,
                year: None,
            },
            metadata: LawMetadata::default(),
        };

        add_law_metadata(&mut data, &document.words);

        // Guess edition
        data.resource.edition_guess = guess_edition(
            &data.resource.exact_editions,
            &data.resource.variation_editions,
            data.resource.year,
        );

        Citation::FullLaw(data)
    } else if cite_sources.contains(&ReporterSource::Journals) {
        // Full journal citation
        let mut data = FullJournalCitationData {
            resource: ResourceCitationCommon {
                common: CitationCommon {
                    token: token.clone(),
                    index,
                    span_start: None,
                    span_end: None,
                    full_span_start: None,
                    full_span_end: None,
                    groups,
                },
                exact_editions: ct.exact_editions.clone(),
                variation_editions: ct.variation_editions.clone(),
                all_editions,
                edition_guess: None,
                year: None,
            },
            metadata: JournalMetadata::default(),
        };

        add_journal_metadata(&mut data, &document.words);

        data.resource.edition_guess = guess_edition(
            &data.resource.exact_editions,
            &data.resource.variation_editions,
            data.resource.year,
        );

        Citation::FullJournal(data)
    } else {
        // Full case citation (default, includes reporters)
        let mut data = FullCaseCitationData {
            resource: ResourceCitationCommon {
                common: CitationCommon {
                    token: token.clone(),
                    index,
                    span_start: None,
                    span_end: None,
                    full_span_start: None,
                    full_span_end: None,
                    groups,
                },
                exact_editions: ct.exact_editions.clone(),
                variation_editions: ct.variation_editions.clone(),
                all_editions: all_editions.clone(),
                edition_guess: None,
                year: None,
            },
            metadata: FullCaseMetadata::default(),
        };

        // Add post-citation metadata (court, year, pin_cite, etc.)
        add_post_citation(&mut data, &document.words);

        // Find case name (plaintiff/defendant), from emphasis tags first
        // when the document came from markup
        find_case_name_in_html(&mut data, document);
        if data.metadata.defendant.is_none() {
            find_case_name(&mut data, &document.words, false);
        } else {
            add_pre_cite_year(&mut data, &document.words);
        }

        // Add pre-citation metadata (antecedent before repeated full cite)
        add_pre_citation(&mut data, &document.words);

        // Guess court from reporter
        data.metadata.case.court =
            guess_court(&data.metadata.case.court, &data.resource.all_editions);

        // Guess edition
        data.resource.edition_guess = guess_edition(
            &data.resource.exact_editions,
            &data.resource.variation_editions,
            data.resource.year,
        );

        Citation::FullCase(data)
    }
}

// ---------------------------------------------------------------------------
// Extract shortform citation
// ---------------------------------------------------------------------------

fn extract_shortform_citation(document: &Document, index: usize) -> Citation {
    let token = match &document.words[index] {
        TokenOrStr::Token(t) => t.clone(),
        _ => unreachable!("Expected token at citation index"),
    };

    let ct = match &token {
        Token::Citation(ct) => ct,
        _ => unreachable!("Expected citation token"),
    };

    // The pin cite is matched against the original page text, before a
    // blank page is normalized away (as eyecite does).
    let page_prefix = token
        .groups()
        .get("page")
        .and_then(|v| v.as_deref())
        .unwrap_or("");
    let (pin_cite, span_end, parenthetical) = extract_pin_cite(&document.words, index, page_prefix);

    let mut groups = token.groups().clone();
    normalize_blank_page(&mut groups);

    let span_end_val = span_end.unwrap_or(0);
    let all_editions: Vec<_> = ct
        .exact_editions
        .iter()
        .chain(ct.variation_editions.iter())
        .cloned()
        .collect();

    let mut data = ShortCaseCitationData {
        resource: ResourceCitationCommon {
            common: CitationCommon {
                token: token.clone(),
                index,
                span_start: None,
                span_end: if span_end_val > 0 {
                    Some(span_end_val)
                } else {
                    None
                },
                full_span_start: None,
                full_span_end: Some(span_end_val.max(token.end())),
                groups,
            },
            exact_editions: ct.exact_editions.clone(),
            variation_editions: ct.variation_editions.clone(),
            all_editions: all_editions.clone(),
            edition_guess: None,
            year: None,
        },
        metadata: ShortCaseMetadata {
            case: CaseMetadata {
                pin_cite,
                parenthetical,
                ..Default::default()
            },
            antecedent_guess: None,
        },
    };

    // Find case name (antecedent), from emphasis tags first when the
    // document came from markup
    find_case_name_in_html_short(&mut data, document);
    if data.metadata.antecedent_guess.is_none() {
        find_case_name_short(&mut data, &document.words);
    }

    // Guess edition
    data.resource.edition_guess = guess_edition(
        &data.resource.exact_editions,
        &data.resource.variation_editions,
        data.resource.year,
    );

    // Guess court
    data.metadata.case.court = guess_court(&data.metadata.case.court, &data.resource.all_editions);

    Citation::ShortCase(data)
}

// ---------------------------------------------------------------------------
// Extract supra citation
// ---------------------------------------------------------------------------

fn extract_supra_citation(words: &Tokens, index: usize) -> Citation {
    let token = match &words[index] {
        TokenOrStr::Token(t) => t.clone(),
        _ => unreachable!("Expected token at supra index"),
    };

    let (pin_cite, span_end, parenthetical) = extract_pin_cite(words, index, "");

    let mut antecedent_guess = None;
    let mut volume = None;
    let mut antecedent_length = 0;

    let m = crate::helpers::match_on_tokens(
        words,
        index as i64 - 1,
        SUPRA_ANTECEDENT_REGEX,
        "",
        true,  // strings_only
        false, // backward
    );

    if let Some(m) = m {
        // Merge branch groups
        let mut merged = HashMap::new();
        for &(logical_name, branch_names) in SUPRA_ANTECEDENT_GROUPS {
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

        antecedent_guess = merged.get("antecedent").and_then(|v| v.clone());
        volume = merged.get("volume").and_then(|v| v.clone());
        antecedent_length = m.end - m.start;
    }

    let common = CitationCommon {
        token: token.clone(),
        index,
        span_start: None,
        span_end,
        full_span_start: Some(token.start().saturating_sub(antecedent_length)),
        full_span_end: Some(span_end.unwrap_or(token.end())),
        groups: token.groups().clone(),
    };

    Citation::Supra(SupraCitationData {
        common,
        metadata: SupraMetadata {
            antecedent_guess,
            pin_cite,
            parenthetical,
            volume,
            ..Default::default()
        },
    })
}

// ---------------------------------------------------------------------------
// Extract id citation
// ---------------------------------------------------------------------------

/// Extract an id. citation, and the indexes of the `§` tokens its pin cite
/// took, if it took any.
fn extract_id_citation(words: &Tokens, index: usize) -> (Citation, Vec<usize>) {
    let token = match &words[index] {
        TokenOrStr::Token(t) => t.clone(),
        _ => unreachable!("Expected token at id index"),
    };

    let (mut pin_cite, mut span_end, mut parenthetical) = extract_pin_cite(words, index, "");
    let mut section_indexes = Vec::new();
    if pin_cite.is_none() {
        if let Some(section) = extract_section_pin_cite(words, index) {
            pin_cite = section.pin_cite;
            span_end = Some(section.span_end);
            parenthetical = section.parenthetical;
            section_indexes = section.section_indexes;
        }
    }

    let common = CitationCommon {
        token: token.clone(),
        index,
        span_start: None,
        span_end,
        full_span_start: None,
        full_span_end: None,
        groups: token.groups().clone(),
    };

    let citation = Citation::Id(IdCitationData {
        common,
        metadata: IdMetadata {
            pin_cite,
            parenthetical,
            ..Default::default()
        },
    });
    (citation, section_indexes)
}

// ---------------------------------------------------------------------------
// Handle parallel citations
// ---------------------------------------------------------------------------

/// Give a full case citation the case name and year of the one before it
/// when they are parallel citations of one decision: eyecite's rule, the
/// same full span start, or (#66) nothing but pin cites and punctuation
/// between them (see [`joins_parallel_cites`]). The second case is for a
/// citation whose own search for a case name ended elsewhere, such as past
/// the 28 words it looks back.
///
/// Two citations that found no case name join when only commas, pin cites
/// and punctuation stand between them (see
/// [`joins_nameless_parallel_cites`]): the run starts at the first
/// citation's own text, which becomes the full span start of every
/// citation in it (#78).
fn handle_parallel_citation(current: &mut Citation, preceding: &mut Citation, words: &Tokens) {
    if let (Citation::FullCase(ref mut cur_data), Citation::FullCase(ref mut pre_data)) =
        (current, preceding)
    {
        // A run of citations without a case name (#78), which may not cross
        // a semicolon. Its full span starts at its first citation.
        let pre_common = &pre_data.resource.common;
        let pre_span_start = pre_common
            .span_start
            .unwrap_or_else(|| pre_common.token.start());
        let nameless = cur_data.resource.common.full_span_start.is_none()
            && pre_data.metadata.plaintiff.is_none()
            && pre_data.metadata.defendant.is_none()
            && pre_common
                .full_span_start
                .is_none_or(|start| start_of_citation(words, pre_common.index, start));
        let (pre_index, cur_index) = (
            pre_data.resource.common.index,
            cur_data.resource.common.index,
        );
        let joins = if nameless {
            joins_nameless_parallel_cites(words, pre_index, cur_index)
        } else {
            joins_parallel_cites(words, pre_index, cur_index)
        };
        let pre_common = &mut pre_data.resource.common;
        if joins
            && pre_common.full_span_start.is_none()
            && cur_data.resource.common.full_span_start.is_none()
        {
            pre_common.full_span_start = Some(pre_span_start);
        }
        let pre_start = pre_common.full_span_start;
        let same_start = cur_data.resource.common.full_span_start == pre_start;
        let adjacent = pre_start.is_some() && joins;
        if same_start || adjacent {
            cur_data.resource.common.full_span_start = pre_start;
            cur_data.metadata.defendant = pre_data.metadata.defendant.clone();
            cur_data.metadata.plaintiff = pre_data.metadata.plaintiff.clone();
            cur_data.metadata.case.year = pre_data.metadata.case.year.clone();
            cur_data.resource.year = pre_data.resource.year;
        }
    }
}

// ---------------------------------------------------------------------------
// Reference citations
// ---------------------------------------------------------------------------

/// Party names of a full case citation usable to find references, keyed by
/// `ReferenceCitation.name_fields` in eyecite order.
fn reference_names(data: &FullCaseCitationData) -> Vec<(&'static str, &str)> {
    let m = &data.metadata;
    [
        ("plaintiff", &m.plaintiff),
        ("defendant", &m.defendant),
        ("resolved_case_name_short", &m.resolved_case_name_short),
        ("resolved_case_name", &m.resolved_case_name),
    ]
    .into_iter()
    .filter_map(|(key, value)| {
        value
            .as_deref()
            .filter(|v| crate::utils::is_valid_name(v))
            .map(|v| (key, v))
    })
    .collect()
}

/// Build a `ReferenceCitation` over `plain_start..plain_end`, taking its
/// metadata from the named groups that matched (eyecite's
/// `match.groupdict()`).
fn make_reference(
    data: String,
    span: (usize, usize),
    full_span: (usize, usize),
    group: impl Fn(&str) -> Option<String>,
) -> Citation {
    let common = CitationCommon {
        token: Token::CaseReference(crate::models::token::TokenData {
            data,
            start: span.0,
            end: span.1,
            groups: HashMap::new(),
        }),
        index: 0,
        span_start: Some(span.0),
        span_end: Some(span.1),
        full_span_start: Some(full_span.0),
        full_span_end: Some(full_span.1),
        groups: HashMap::new(),
    };
    Citation::Reference(crate::models::citation::ReferenceCitationData {
        common,
        metadata: crate::models::citation::ReferenceMetadata {
            parenthetical: None,
            pin_cite: group("pin_cite"),
            pin_cite_span_start: None,
            pin_cite_span_end: None,
            plaintiff: group("plaintiff"),
            defendant: group("defendant"),
            resolved_case_name_short: group("resolved_case_name_short"),
            resolved_case_name: group("resolved_case_name"),
        },
    })
}

/// Put each full case citation's reference citations just before it in
/// `citations`, where eyecite's loop adds them. `sources` are the positions
/// of the full case citations, in increasing order.
fn insert_reference_citations(
    citations: Vec<Citation>,
    sources: &[usize],
    document: &Document,
) -> Vec<Citation> {
    if sources.is_empty() {
        return citations;
    }
    let mut finder = ReferenceFinder::new(document, sources.iter().map(|&i| &citations[i]));
    let mut found: Vec<Vec<Citation>> = sources
        .iter()
        .map(|&i| finder.references(&citations[i]))
        .collect();
    if found.iter().all(Vec::is_empty) {
        return citations;
    }
    let mut out = Vec::with_capacity(citations.len() + found.iter().map(Vec::len).sum::<usize>());
    let mut next = sources.iter().zip(found.iter_mut()).peekable();
    for (i, citation) in citations.into_iter().enumerate() {
        if let Some((_, references)) = next.next_if(|(&source, _)| source == i) {
            out.append(references);
        }
        out.push(citation);
    }
    out
}

/// Finds the reference citations that follow full case citations.
///
/// eyecite searches the whole text after each full citation for its party
/// names, which costs the text length times the number of citations (#44).
/// A reference match starts with one of the names, so this finds where every
/// name occurs in one Aho-Corasick pass, and tries the reference pattern
/// only there, over a window that holds any match that can start there.
/// For markup, the pattern starts with an `<em>` or `<i>` tag, so it is
/// tried only where one opens. The matches are the ones eyecite finds.
struct ReferenceFinder<'d> {
    document: &'d Document,
    /// Start offsets in the plain text of each name, in increasing order.
    occurrences: HashMap<String, Vec<usize>>,
    /// Name-pincite patterns by pattern text; `None` if it didn't compile.
    pin_cite_regexes: HashMap<String, Option<fancy_regex::Regex>>,
    /// Emphasis patterns, anchored, by pattern text.
    markup_regexes: HashMap<String, Option<regex::Regex>>,
    /// Offsets in the markup where an `<em>` or `<i>` tag opens, in order.
    emphasis_starts: Vec<usize>,
}

impl<'d> ReferenceFinder<'d> {
    fn new<'c>(document: &'d Document, sources: impl Iterator<Item = &'c Citation>) -> Self {
        let plain_text = document.plain_text.as_str();
        let mut names: Vec<&str> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for citation in sources {
            let Citation::FullCase(data) = citation else {
                continue;
            };
            if citation.span().1 >= plain_text.len() {
                continue;
            }
            for (_, name) in reference_names(data) {
                if seen.insert(name) {
                    names.push(name);
                }
            }
        }
        let mut occurrences: HashMap<String, Vec<usize>> = HashMap::new();
        if !names.is_empty() {
            let ac = aho_corasick::AhoCorasick::builder()
                .match_kind(aho_corasick::MatchKind::Standard)
                .build(&names)
                .expect("names make a valid automaton");
            let mut starts: Vec<Vec<usize>> = vec![Vec::new(); names.len()];
            // Overlapping matches come in order of their end, and every
            // match of one name has the same length, so each list is sorted.
            for m in ac.find_overlapping_iter(plain_text) {
                starts[m.pattern().as_usize()].push(m.start());
            }
            occurrences = names.iter().map(|n| n.to_string()).zip(starts).collect();
        }

        let mut emphasis_starts = Vec::new();
        if let Some(markup) = document.markup_text.as_deref() {
            for tag in ["<em>", "<i>"] {
                emphasis_starts.extend(markup.match_indices(tag).map(|(i, _)| i));
            }
            emphasis_starts.sort_unstable();
        }

        Self {
            document,
            occurrences,
            pin_cite_regexes: HashMap::new(),
            markup_regexes: HashMap::new(),
            emphasis_starts,
        }
    }

    /// Reference citations of `citation`, from the plain text and, when the
    /// document came from markup, from emphasis tags.
    fn references(&mut self, citation: &Citation) -> Vec<Citation> {
        let Citation::FullCase(data) = citation else {
            return Vec::new();
        };
        if self.document.plain_text.len() <= citation.span().1 {
            return Vec::new();
        }
        let mut references = self.pincited_references(citation, data);
        if self.document.markup_text.is_some() {
            references.extend(self.markup_references(citation, data));
        }
        references
    }

    /// References with the name-pincite pattern ("Foo at 123").
    fn pincited_references(
        &mut self,
        citation: &Citation,
        data: &FullCaseCitationData,
    ) -> Vec<Citation> {
        let plain_text = self.document.plain_text.as_str();
        let (_, span_end) = citation.span();
        let names = reference_names(data);

        // A match starts with one of the names (escaped and matched
        // case-sensitively) at or after the end of the citation.
        let mut candidates: Vec<usize> = names
            .iter()
            .filter_map(|(_, name)| self.occurrences.get(*name))
            .flat_map(|starts| {
                let first = starts.partition_point(|&s| s < span_end);
                starts[first..].iter().copied()
            })
            .collect();
        if candidates.is_empty() {
            return Vec::new();
        }
        candidates.sort_unstable();
        candidates.dedup();

        let regexes: Vec<String> = names
            .iter()
            .map(|(key, value)| format!("(?P<{key}>{})", regex::escape(value)))
            .collect();
        let pin_cite_re = crate::regexes::reference_pin_cite_re(
            &regexes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
        );
        let re = self
            .pin_cite_regexes
            .entry(pin_cite_re)
            .or_insert_with_key(|pattern| fancy_regex::Regex::new(pattern).ok());
        let Some(re) = re.as_ref() else {
            return Vec::new();
        };

        // eyecite matches in the text after the citation, so a word
        // boundary at its very start sees no character before it.
        let remaining_text = &plain_text[span_end..];
        let mut references = Vec::new();
        let mut pos = span_end;
        for start in candidates {
            if start < pos {
                continue;
            }
            let window_end = names
                .iter()
                .filter(|(_, name)| plain_text[start..].starts_with(name))
                .map(|(_, name)| pin_cite_window_end(plain_text, start + name.len()))
                .max()
                .unwrap_or(start);
            let haystack = &remaining_text[..window_end - span_end];
            let caps = match re.captures_from_pos(haystack, start - span_end) {
                Ok(Some(caps)) => caps,
                _ => continue,
            };
            let m = caps.get(0).expect("group 0 is always present");
            if m.start() != start - span_end {
                continue;
            }
            let span = (span_end + m.start(), span_end + m.end());
            references.push(make_reference(m.as_str().to_string(), span, span, |name| {
                caps.name(name).map(|g| g.as_str().to_string())
            }));
            pos = span.1;
        }
        references
    }

    /// References in `<em>`/`<i>` tags holding one of the citation's party
    /// names (eyecite `find_reference_citations_from_markup`). Spans are
    /// mapped back to the plain text, like every other citation's.
    fn markup_references(
        &mut self,
        citation: &Citation,
        data: &FullCaseCitationData,
    ) -> Vec<Citation> {
        let document = self.document;
        let (Some(to_markup), Some(to_plain), Some(markup)) = (
            document.plain_to_markup.as_ref(),
            document.markup_to_plain.as_ref(),
            document.markup_text.as_deref(),
        ) else {
            return Vec::new();
        };

        let regexes: Vec<String> = reference_names(data)
            .into_iter()
            .map(|(key, value)| {
                let value = value
                    .split_whitespace()
                    .map(regex::escape)
                    .collect::<Vec<_>>()
                    .join(r"\s+");
                format!("(?P<{key}>{value})")
            })
            .collect();
        if regexes.is_empty() {
            return Vec::new();
        }

        // Include punctuation and spaces around the party name so the match
        // is contiguous with the style tags. Every match starts at an
        // emphasis tag, so it is anchored and tried at each one in turn.
        let pattern = format!(r"^<(?:em|i)>\s*({})[:;.,\s]*</(?:em|i)>", regexes.join("|"));
        let re = self
            .markup_regexes
            .entry(pattern)
            .or_insert_with_key(|pattern| regex::Regex::new(pattern).ok());
        let Some(re) = re.as_ref() else {
            return Vec::new();
        };

        let start_in_markup = to_markup.update(citation.span().0, true);
        if markup.get(start_in_markup..).is_none() {
            return Vec::new();
        }

        let first = self
            .emphasis_starts
            .partition_point(|&s| s < start_in_markup);
        let mut references = Vec::new();
        let mut pos = start_in_markup;
        for &tag_start in &self.emphasis_starts[first..] {
            if tag_start < pos {
                continue;
            }
            let Some(caps) = re.captures(&markup[tag_start..]) else {
                continue;
            };
            let whole = caps.get(0).unwrap();
            let name = caps.get(1).unwrap();
            pos = tag_start + whole.end();
            let full_start = to_plain.update(tag_start + whole.start(), false);
            let full_end = to_plain.update(tag_start + whole.end(), true);
            let start = to_plain.update(tag_start + name.start(), false);
            let end = to_plain.update(tag_start + name.end(), true);

            // `<i>Foo</i> v. <i>Bar, supra</i>`: <i>Foo</i> is not a reference.
            let after = document.plain_text.get(full_end..).unwrap_or("");
            if re_after_markup_reference().is_match(after) {
                continue;
            }

            let text = document
                .plain_text
                .get(start..end)
                .unwrap_or("")
                .to_string();
            references.push(make_reference(
                text,
                (start, end),
                (full_start, full_end),
                |group| caps.name(group).map(|g| g.as_str().to_string()),
            ));
        }
        references
    }
}

/// End of the text a name-pincite match can see when its name ends at
/// `after_name`: past the first character that no part of the pattern after
/// the name can match (whitespace, ", at", "note", "n.", "fn.", "¶", "§",
/// "*", "pg.", "pp.", digits, ":" and "-"). The pattern's lookahead reads
/// that character, so it is included; with no such character, the window
/// runs to the end of the text, where the lookahead's `$` matches.
fn pin_cite_window_end(text: &str, after_name: usize) -> usize {
    let can_continue =
        |c: char| c.is_whitespace() || c.is_numeric() || ",.:-&*\u{B6}\u{A7}atnoefpg".contains(c);
    text[after_name..]
        .char_indices()
        .find(|&(_, c)| !can_continue(c))
        .map_or(text.len(), |(i, c)| after_name + i + c.len_utf8())
}

fn re_after_markup_reference() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^\s*(v[.s]|supra)\s").unwrap())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_citations_basic() {
        let citations = get_citations("1 U.S. 1", false, "", &[]);
        assert!(
            !citations.is_empty(),
            "Expected at least one citation for '1 U.S. 1'"
        );
        assert!(
            matches!(&citations[0], Citation::FullCase(_)),
            "Expected FullCase citation, got {:?}",
            citations[0].type_name()
        );
    }

    #[test]
    fn test_get_citations_with_metadata() {
        let citations = get_citations("Foo v. Bar, 1 U.S. 1 (1999)", false, "", &[]);
        assert!(!citations.is_empty(), "Expected at least one citation");
        if let Citation::FullCase(ref data) = citations[0] {
            assert_eq!(
                data.metadata.case.year.as_deref(),
                Some("1999"),
                "Expected year 1999"
            );
        } else {
            panic!("Expected FullCase citation");
        }
    }

    #[test]
    fn test_get_citations_year_parsing() {
        let citations = get_citations("1 U.S. 1 (2020)", false, "", &[]);
        assert!(!citations.is_empty());
        if let Citation::FullCase(ref data) = citations[0] {
            assert_eq!(data.resource.year, Some(2020));
        }
    }

    #[test]
    fn test_id_citation() {
        let citations = get_citations("1 U.S. 1. Id. at 2.", false, "", &[]);
        let has_id = citations.iter().any(|c| matches!(c, Citation::Id(_)));
        assert!(has_id, "Expected an Id citation in '1 U.S. 1. Id. at 2.'");
    }

    #[test]
    fn test_supra_citation() {
        let citations = get_citations("1 U.S. 1. Foo, supra, at 2.", false, "", &[]);
        let has_supra = citations.iter().any(|c| matches!(c, Citation::Supra(_)));
        assert!(
            has_supra,
            "Expected a Supra citation. Found: {:?}",
            citations.iter().map(|c| c.type_name()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_short_citation() {
        let citations = get_citations(
            "Foo v. Bar, 1 U.S. 1 (1999). Bar, 1 U.S., at 2.",
            false,
            "",
            &[],
        );
        let has_short = citations
            .iter()
            .any(|c| matches!(c, Citation::ShortCase(_)));
        assert!(
            has_short,
            "Expected a ShortCase citation. Found: {:?}",
            citations.iter().map(|c| c.type_name()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_get_citations_empty_input() {
        let citations = get_citations("", false, "", &[]);
        assert!(citations.is_empty());
    }

    #[test]
    fn test_get_citations_no_citations() {
        let citations = get_citations("This is just regular text.", false, "", &[]);
        assert!(citations.is_empty());
    }

    #[test]
    fn test_get_citations_multiple() {
        let citations = get_citations("See 1 U.S. 1 (1999); 2 U.S. 2 (2000)", false, "", &[]);
        let full_cases: Vec<_> = citations
            .iter()
            .filter(|c| matches!(c, Citation::FullCase(_)))
            .collect();
        assert!(
            full_cases.len() >= 2,
            "Expected at least 2 full case citations, got {}",
            full_cases.len()
        );
    }

    #[test]
    fn test_get_citations_court() {
        let citations = get_citations("1 U.S. 1 (4th Cir. 1999)", false, "", &[]);
        if let Some(Citation::FullCase(ref data)) = citations.first() {
            // Court should be set
            if let Some(court) = &data.metadata.case.court {
                // Just verify it's a string
                assert!(!court.is_empty());
            }
        }
    }

    #[test]
    fn test_get_citations_with_clean_steps() {
        let citations = get_citations("1  U.S.  1", false, "", &[CleanStep::InlineWhitespace]);
        assert!(
            !citations.is_empty(),
            "Expected citation after cleaning whitespace"
        );
    }

    #[test]
    fn test_remove_ambiguous() {
        // With remove_ambiguous=true, citations with multiple possible reporters
        // that can't be narrowed should be removed
        let with = get_citations("1 U.S. 1 (1999)", true, "", &[]);
        let without = get_citations("1 U.S. 1 (1999)", false, "", &[]);
        // Both should find the citation (U.S. is unambiguous)
        assert!(!with.is_empty());
        assert!(!without.is_empty());
    }

    #[test]
    fn test_citation_spans() {
        let text = "See 1 U.S. 1 (1999)";
        let citations = get_citations(text, false, "", &[]);
        if let Some(c) = citations.first() {
            let (start, end) = c.span();
            assert!(end > start, "span end should be > start");
            assert!(end <= text.len(), "span end should be <= text length");
        }
    }

    // -----------------------------------------------------------------------
    // Id. citation pin cite tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_id_pin_cite_at_page() {
        let cites = get_citations("1 U.S. 1 (1999). Id. at 5.", false, "", &[]);
        let id = cites.iter().find(|c| matches!(c, Citation::Id(_))).unwrap();
        if let Citation::Id(d) = id {
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at 5"));
        }
    }

    #[test]
    fn test_id_pin_cite_at_range() {
        let cites = get_citations("1 U.S. 1 (1999). Id., at 123-24.", false, "", &[]);
        let id = cites.iter().find(|c| matches!(c, Citation::Id(_))).unwrap();
        if let Citation::Id(d) = id {
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at 123-24"));
        }
    }

    #[test]
    fn test_id_pin_cite_star_pagination() {
        let cites = get_citations("1 U.S. 1 (1999). Id. at *5.", false, "", &[]);
        let id = cites.iter().find(|c| matches!(c, Citation::Id(_))).unwrap();
        if let Citation::Id(d) = id {
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at *5"));
        }
    }

    #[test]
    fn test_id_no_pin_cite() {
        let cites = get_citations("1 U.S. 1 (1999). Id.", false, "", &[]);
        let id = cites.iter().find(|c| matches!(c, Citation::Id(_))).unwrap();
        if let Citation::Id(d) = id {
            assert_eq!(d.metadata.pin_cite, None);
        }
    }

    #[test]
    fn test_id_pin_cite_multiple_pages() {
        let cites = get_citations("1 U.S. 1 (1999). Id. at 5, 7.", false, "", &[]);
        let id = cites.iter().find(|c| matches!(c, Citation::Id(_))).unwrap();
        if let Citation::Id(d) = id {
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at 5, 7"));
        }
    }

    // -----------------------------------------------------------------------
    // Supra citation pin cite tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_supra_pin_cite() {
        let cites = get_citations(
            "Foo v. Bar, 1 U.S. 1 (1999). Foo, supra, at 240.",
            false,
            "",
            &[],
        );
        let supra = cites
            .iter()
            .find(|c| matches!(c, Citation::Supra(_)))
            .unwrap();
        if let Citation::Supra(d) = supra {
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at 240"));
            assert_eq!(d.metadata.antecedent_guess.as_deref(), Some("Foo"));
        }
    }

    #[test]
    fn test_supra_with_volume() {
        let cites = get_citations(
            "Foo v. Bar, 1 U.S. 1 (1999). Foo, 515 supra, at 240.",
            false,
            "",
            &[],
        );
        let supra = cites
            .iter()
            .find(|c| matches!(c, Citation::Supra(_)))
            .unwrap();
        if let Citation::Supra(d) = supra {
            assert_eq!(d.metadata.volume.as_deref(), Some("515"));
        }
    }

    // -----------------------------------------------------------------------
    // Short case citation pin cite tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_short_case_pin_cite() {
        let cites = get_citations(
            "Foo v. Bar, 1 U.S. 1 (1999). Bar, 1 U.S., at 2.",
            false,
            "",
            &[],
        );
        let short = cites
            .iter()
            .find(|c| matches!(c, Citation::ShortCase(_)))
            .unwrap();
        if let Citation::ShortCase(d) = short {
            assert_eq!(d.metadata.case.pin_cite.as_deref(), Some("2"));
        }
    }

    // -----------------------------------------------------------------------
    // Full case citation metadata tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_case_court_parsing() {
        let cites = get_citations(
            "Foo v. Bar, 1 U.S. 12, 347-348 (4th Cir. 1982)",
            false,
            "",
            &[],
        );
        if let Citation::FullCase(d) = &cites[0] {
            assert_eq!(d.metadata.case.court.as_deref(), Some("ca4"));
            assert_eq!(d.metadata.case.pin_cite.as_deref(), Some("347-348"));
            assert_eq!(d.metadata.case.year.as_deref(), Some("1982"));
        }
    }

    #[test]
    fn test_full_case_with_month_day() {
        let cites = get_citations(
            "Foo v. Bar, 164 A.3d 1189 (Pa. Feb. 9, 2017)",
            false,
            "",
            &[],
        );
        if let Citation::FullCase(d) = &cites[0] {
            assert_eq!(d.metadata.case.month.as_deref(), Some("Feb."));
            assert_eq!(d.metadata.case.day.as_deref(), Some("9"));
        }
    }

    #[test]
    fn test_full_case_parenthetical() {
        let cites = get_citations(
            "Foo v. Bar, 1 U.S. 1 (1999) (overruling something)",
            false,
            "",
            &[],
        );
        if let Citation::FullCase(d) = &cites[0] {
            assert_eq!(
                d.metadata.case.parenthetical.as_deref(),
                Some("overruling something")
            );
        }
    }

    #[test]
    fn test_plaintiff_defendant() {
        let cites = get_citations("Lissner v. Test, 1 U.S. 1 (1982)", false, "", &[]);
        if let Citation::FullCase(d) = &cites[0] {
            assert_eq!(d.metadata.plaintiff.as_deref(), Some("Lissner"));
            assert_eq!(d.metadata.defendant.as_deref(), Some("Test"));
        }
    }

    // -----------------------------------------------------------------------
    // Law citation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_law_citation_basic() {
        let cites = get_citations("Mass. Gen. Laws ch. 1, § 2", false, "", &[]);
        assert!(cites.iter().any(|c| matches!(c, Citation::FullLaw(_))));
    }

    #[test]
    fn test_usc_citation() {
        let cites = get_citations("42 U.S.C. § 1983", false, "", &[]);
        assert!(cites.iter().any(|c| matches!(c, Citation::FullLaw(_))));
    }

    // -----------------------------------------------------------------------
    // Journal citation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_journal_citation() {
        let cites = get_citations("1 Minn. L. Rev. 1", false, "", &[]);
        assert!(cites.iter().any(|c| matches!(c, Citation::FullJournal(_))));
    }

    #[test]
    fn test_journal_with_year() {
        let cites = get_citations("1 Minn. L. Rev. 1 (2012)", false, "", &[]);
        let j = cites
            .iter()
            .find(|c| matches!(c, Citation::FullJournal(_)))
            .unwrap();
        if let Citation::FullJournal(d) = j {
            assert_eq!(d.resource.year, Some(2012));
        }
    }

    // -----------------------------------------------------------------------
    // Edge case tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_false_positive_on_address() {
        // "111 S.W. 12th St." should NOT be a citation.
        // eyecite behavior: finds no citation here because "12th" is not a
        // valid page number (ordinal suffix prevents matching).
        let cites = get_citations("111 S.W. 12th St. is an address", false, "", &[]);
        assert!(cites.is_empty() || !cites.iter().any(|c| c.matched_text().contains("S.W.")));
    }

    fn only_full_case(cites: &[Citation]) -> &FullCaseCitationData {
        assert_eq!(cites.len(), 1, "expected one citation, got {:?}", cites);
        match &cites[0] {
            Citation::FullCase(d) => d,
            other => panic!("expected FullCase, got {:?}", other),
        }
    }

    // Ported from eyecite tests/test_FindTest.py: a page made only of
    // underscores is a known-missing page and is normalized to None.
    #[test]
    fn test_blank_page_is_none() {
        let cites = get_citations("1 U.S. ___", false, "", &[]);
        let d = only_full_case(&cites);
        let g = &d.resource.common.groups;
        assert_eq!(g.get("volume").cloned().flatten().as_deref(), Some("1"));
        assert_eq!(
            g.get("reporter").cloned().flatten().as_deref(),
            Some("U.S.")
        );
        assert_eq!(g.get("page").cloned().flatten(), None);
        // The matched text still carries the original underscores.
        assert_eq!(cites[0].matched_text(), "1 U.S. ___");
    }

    #[test]
    fn test_blank_page_followed_by_comma() {
        // cf. eyecite#137
        let cites = get_citations("1 U. S. ___,", false, "", &[]);
        let d = only_full_case(&cites);
        let g = &d.resource.common.groups;
        assert_eq!(
            g.get("reporter").cloned().flatten().as_deref(),
            Some("U. S.")
        );
        assert_eq!(g.get("page").cloned().flatten(), None);
    }

    #[test]
    fn test_blank_page_with_pin_cite() {
        let cites = get_citations(
            "\n 576 U.S. ___, 13",
            false,
            "",
            &[CleanStep::AllWhitespace],
        );
        let d = only_full_case(&cites);
        let g = &d.resource.common.groups;
        assert_eq!(g.get("volume").cloned().flatten().as_deref(), Some("576"));
        assert_eq!(g.get("page").cloned().flatten(), None);
        assert_eq!(d.metadata.case.pin_cite.as_deref(), Some("13"));
        assert_eq!(d.metadata.case.court.as_deref(), Some("scotus"));
    }

    #[test]
    fn test_placeholder_citation() {
        // "585 U.S. ___" uses the placeholder page token; it still produces a
        // FullCase citation, with the page group normalized to None.
        let cites = get_citations("585 U.S. ___ (2018)", false, "", &[]);
        let d = only_full_case(&cites);
        assert_eq!(
            d.resource.common.groups.get("page").cloned().flatten(),
            None
        );
        assert_eq!(d.metadata.case.year.as_deref(), Some("2018"));
    }

    fn references(cites: &[Citation]) -> Vec<&crate::models::citation::ReferenceCitationData> {
        cites
            .iter()
            .filter_map(|c| match c {
                Citation::Reference(d) => Some(d),
                _ => None,
            })
            .collect()
    }

    // Ported from eyecite tests/test_FindTest.py: a reference citation
    // carries its pin cite and only the party name that matched.
    #[test]
    fn test_reference_citation_plaintiff_and_pin_cite() {
        let text = "Foo v. Bar 1 U.S. 12, 347-348. something something, In Foo at 62, we see that";
        let cites = get_citations(text, false, "", &[]);
        assert_eq!(cites.len(), 2, "{:?}", cites);
        let refs = references(&cites);
        assert_eq!(refs.len(), 1);
        let r = refs[0];
        assert_eq!(cites[1].matched_text(), "Foo at 62");
        assert_eq!(r.metadata.plaintiff.as_deref(), Some("Foo"));
        assert_eq!(r.metadata.defendant, None);
        assert_eq!(r.metadata.pin_cite.as_deref(), Some("at 62"));
    }

    #[test]
    fn test_reference_citation_skips_disallowed_defendant() {
        let text = "Foo v. United States 1 U.S. 12, 347-348. something something ... \
                    the United States at 1776 we see that and Foo at 62";
        let cites = get_citations(text, false, "", &[]);
        let refs = references(&cites);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].metadata.plaintiff.as_deref(), Some("Foo"));
        assert_eq!(refs[0].metadata.defendant, None);
        assert_eq!(refs[0].metadata.pin_cite.as_deref(), Some("at 62"));
    }

    #[test]
    fn test_reference_citation_must_follow_full_cite() {
        let text = "In Foo at 62 we see that, Foo v. Bar 1 U.S. 12, 347-348. something something,";
        let cites = get_citations(text, false, "", &[]);
        assert!(references(&cites).is_empty());
    }

    #[test]
    fn test_reference_citation_against_defendant() {
        let text =
            "In re Foo 1 Mass. 12, 347-348. something something, in Foo at 62, we see that, ";
        let cites = get_citations(text, false, "", &[]);
        let refs = references(&cites);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].metadata.plaintiff, None);
        assert_eq!(refs[0].metadata.defendant.as_deref(), Some("Foo"));
        assert_eq!(refs[0].metadata.pin_cite.as_deref(), Some("at 62"));
    }

    #[test]
    fn test_reference_citation_after_id() {
        let text = "we said in Morton v. Mancari, 417 U. S. 535, 552 (1974) \u{201c}Literally every \
                    piece ....\u{201d}. \u{201c}asisovereign tribal entities . . . .\u{201d} Id. In \
                    Mancari at 665";
        let cites = get_citations(text, false, "", &[]);
        let refs = references(&cites);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].metadata.plaintiff, None);
        assert_eq!(refs[0].metadata.defendant.as_deref(), Some("Mancari"));
        assert_eq!(refs[0].metadata.pin_cite.as_deref(), Some("at 665"));
    }

    #[test]
    fn test_reference_citation_amick() {
        let text = "General Casualty cites as compelling Amick v. Liberty Mut. Ins. Co., 455 A.2d \
                    793 (R.I. 1983). In that case ... Stats, do. See Amick at 795";
        let cites = get_citations(text, false, "", &[]);
        let refs = references(&cites);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].metadata.plaintiff.as_deref(), Some("Amick"));
        assert_eq!(refs[0].metadata.defendant, None);
        assert_eq!(refs[0].metadata.pin_cite.as_deref(), Some("at 795"));
    }

    // Ported from eyecite test_reference_extraction_using_resolved_names:
    // resolved case names are also used to find references.
    #[test]
    fn test_reference_extraction_using_resolved_names() {
        let text = "See, e.g., State v. W1ngler, 135 A. 2d 468 (1957);\n\
                    [State v. Wingler at 175, citing, Minnesota ex rel.]";
        let mut cites = get_citations(text, false, "", &[]);
        assert!(references(&cites).is_empty());
        if let Citation::FullCase(d) = &mut cites[0] {
            d.metadata.resolved_case_name = Some("State v. Wingler".to_string());
        } else {
            panic!("expected FullCase, got {:?}", cites[0]);
        }
        let document = Document::new(text, "", &[]);
        let refs =
            ReferenceFinder::new(&document, std::iter::once(&cites[0])).references(&cites[0]);
        assert_eq!(refs.len(), 1, "{:?}", refs);
        assert_eq!(refs[0].matched_text(), "State v. Wingler at 175");
        if let Citation::Reference(r) = &refs[0] {
            assert_eq!(
                r.metadata.resolved_case_name.as_deref(),
                Some("State v. Wingler")
            );
            assert_eq!(r.metadata.plaintiff, None);
            assert_eq!(r.metadata.pin_cite.as_deref(), Some("at 175"));
        }
    }

    // Ported from eyecite: a "v." at the very start of the input leaves no
    // case name, so the party falls through to antecedent_guess.
    #[test]
    fn test_missing_plaintiff_at_start_of_input() {
        let cites = get_citations("v. Bar, 1 U.S. 1", false, "", &[]);
        assert_eq!(cites.len(), 1);
        match &cites[0] {
            Citation::FullCase(d) => {
                assert_eq!(d.metadata.antecedent_guess.as_deref(), Some("Bar"));
                assert_eq!(d.metadata.plaintiff, None);
                assert_eq!(d.metadata.defendant, None);
            }
            other => panic!("expected FullCase, got {:?}", other),
        }
    }

    #[test]
    fn test_parallel_citations() {
        let cites = get_citations("Foo v. Bar, 1 U.S. 12, 1 S. Ct. 2 (1982)", false, "", &[]);
        let full_cases: Vec<_> = cites
            .iter()
            .filter(|c| matches!(c, Citation::FullCase(_)))
            .collect();
        assert_eq!(full_cases.len(), 2, "Should find 2 parallel citations");
    }

    #[test]
    fn test_harvard_slug_in_citation() {
        let cites = get_citations("1 U.S. 1", false, "", &[]);
        if let Some(rc) = cites[0].resource_common() {
            let slug = rc
                .all_editions
                .iter()
                .find_map(|e| e.harvard_slug.as_deref());
            assert_eq!(slug, Some("us"));
        }
    }

    // -----------------------------------------------------------------------
    // Markup input (eyecite test_find_citations html cases)
    // -----------------------------------------------------------------------

    fn markup_cites(markup: &str) -> Vec<Citation> {
        get_citations(
            "",
            false,
            markup,
            &[CleanStep::Html, CleanStep::InlineWhitespace],
        )
    }

    #[test]
    fn test_markup_only_finds_full_citation() {
        let cites = markup_cites("<p>See 254 <i>U.S.</i> 189.</p>");
        assert_eq!(cites.len(), 1);
        assert!(matches!(cites[0], Citation::FullCase(_)));
        // Without the `html` step, it is added rather than failing
        // (eyecite#328).
        let cites = get_citations("", false, "<p>See 254 U.S. 189.</p>", &[]);
        assert_eq!(cites.len(), 1);
    }

    #[test]
    fn test_markup_ibid() {
        let cites = markup_cites("<p>before asdf. <i>Ibid.</i></p> <p>foo bar lorem</p>");
        assert_eq!(cites.len(), 1);
        assert!(matches!(cites[0], Citation::Id(_)));
    }

    #[test]
    fn test_markup_id_pin_cite() {
        for markup in [
            "<p>before asdf. <i>Id.,</i> at 123.</p> <p>foo bar</p>",
            "<p>before asdf. <i>Id.,</i> at <b>123.</b></p> <p>foo bar</p>",
        ] {
            let cites = markup_cites(markup);
            assert_eq!(cites.len(), 1, "{markup}");
            let Citation::Id(d) = &cites[0] else {
                panic!("expected Id for {markup}");
            };
            assert_eq!(d.metadata.pin_cite.as_deref(), Some("at 123"), "{markup}");
        }
    }

    #[test]
    fn test_markup_plain_case_name() {
        let cites = markup_cites(
            "<p>State v. Luna-Benitez (S53965). Alternative writ issued, dismissed, 342 Or 255</p>",
        );
        assert_eq!(cites.len(), 1);
        assert!(matches!(cites[0], Citation::FullCase(_)));
    }

    /// Years of the full case citations in `cites`, keyed by matched text.
    fn case_years(cites: &[Citation]) -> Vec<(String, Option<i32>, Option<String>)> {
        cites
            .iter()
            .filter_map(|c| match c {
                Citation::FullCase(d) => Some((
                    c.matched_text().to_string(),
                    d.resource.year,
                    d.metadata.case.year.clone(),
                )),
                _ => None,
            })
            .collect()
    }

    /// An Ohio-style year between the case name and the volume is the
    /// citation's own, ahead of a year in a later parenthetical (#85).
    /// eyecite reads "(1972)," the same way; it matches the year at the
    /// start of the word.
    #[test]
    fn test_year_before_volume_plain() {
        let text = "In re Murray (1990), 52 Ohio St.3d 155, 157; Stanley v. Illinois (1972), \
                    405 U.S. 645, 651. Id., (1990), 52 Ohio St.3d 155, 157.";
        let years = case_years(&get_citations(text, false, "", &[]));
        assert_eq!(
            years[0],
            ("52 Ohio St.3d 155".into(), Some(1990), Some("1990".into()))
        );
        assert_eq!(
            years[1],
            ("405 U.S. 645".into(), Some(1972), Some("1972".into()))
        );

        let cites = get_citations("Foo v. Bar (1972), 1 U.S. 1 (1999).", false, "", &[]);
        let Citation::FullCase(d) = &cites[0] else {
            panic!("expected FullCase");
        };
        assert_eq!(d.resource.year, Some(1972));
        assert_eq!(d.metadata.defendant.as_deref(), Some("Bar"));

        // citation-bench find/ohio_state_variation/0, from CourtListener
        // opinion 1612618: the year isn't part of the defendant.
        let text = "See Troop A Riding Academy v. Miller (1934), 127 Ohio 545, 189 N.E. 650.";
        for c in get_citations(text, false, "", &[]) {
            let Citation::FullCase(d) = &c else {
                panic!("expected FullCase");
            };
            assert_eq!(d.resource.year, Some(1934), "{}", c.matched_text());
            assert_eq!(d.metadata.defendant.as_deref(), Some("Miller"));
            assert_eq!(
                d.metadata.plaintiff.as_deref(),
                Some("Troop A Riding Academy")
            );
        }

        // A year that ends an earlier sentence or name isn't the next
        // citation's.
        for text in [
            "Dr. Miles (1911). United States v. Arnold, Schwinn & Co., 388 U.S. 365 (1967).",
            "Dr. Miles (1911), and United States v. Arnold, 388 U.S. 365 (1967).",
        ] {
            let years = case_years(&get_citations(text, false, "", &[]));
            assert_eq!(years[0].1, Some(1967), "{text}");
        }
    }

    /// The same in markup, where the case name comes from emphasis tags and
    /// eyecite doesn't look for a year before the volume (#85).
    #[test]
    fn test_year_before_volume_markup() {
        let markup = "<p><em>In re Murray</em> (1990), 52 Ohio St.3d 155, 157; \
                      <em>Stanley v. Illinois</em>\n(1972), 405 U.S. 645, 651.</p>\
                      <p>4 <em>Id.,</em> (1990), 52 Ohio St.3d 155, 157, citing, \
                      <em>Santosky v. Kramer</em>\n(1982), 455 U.S. 745.</p>";
        let years = case_years(&markup_cites(markup));
        let year_of = |text: &str| {
            years
                .iter()
                .find(|(t, _, _)| t == text)
                .map(|(_, y, _)| *y)
                .unwrap()
        };
        assert_eq!(year_of("405 U.S. 645"), Some(1972));
        assert_eq!(year_of("455 U.S. 745"), Some(1982));
        assert_eq!(years[0].1, Some(1990));

        // A parallel citation after the year has it too.
        let cites = markup_cites(
            "<p><em>Stanley v. Illinois</em> (1972), 405 U.S. 645, 92 S.Ct. 1208, 651. \
             <em>Id.,</em> (1990), 52 Ohio St.3d 155.</p>",
        );
        let years = case_years(&cites);
        assert_eq!(years[0].1, Some(1972), "{years:?}");
        assert_eq!(years[1].1, Some(1972), "{years:?}");

        // A year that ends an earlier sentence or name isn't the next
        // citation's.
        let cites = markup_cites(
            "<p>Standard Oil Co. v. United States, 221 U.S. 1 (1911); <em>Dr. Miles</em> \
             (1911), (1912). <em>United States v. Arnold, Schwinn &amp; Co.</em>, 388 U.S. \
             365 (1967).</p>",
        );
        let years = case_years(&cites);
        assert_eq!(years[1].1, Some(1967), "{years:?}");

        // A year that closes an earlier citation isn't the next one's.
        let cites = markup_cites("<p><em>Foo v. Bar</em>, 1 U.S. 1 (1950), 2 F.2d 3.</p>");
        let years = case_years(&cites);
        assert_eq!(years[1].1, None, "{years:?}");
    }

    /// A sentence that ends in a parenthetical bounds the case name of the
    /// citation after it (#95).
    #[test]
    fn test_case_name_stops_at_sentence_end() {
        for (text, plaintiff, defendant, name_start) in [
            (
                "Dr. Miles (1911). United States v. Arnold, Schwinn & Co., 388 U.S. 365 (1967).",
                Some("United States"),
                "Arnold, Schwinn & Co.",
                18,
            ),
            (
                "as in Dr. Miles (5th Cir.). United States v. Arnold, 388 U.S. 365 (1967).",
                Some("United States"),
                "Arnold",
                28,
            ),
            (
                "Dr. Miles (1911)?\u{201D} United States v. Arnold, 388 U.S. 365 (1967).",
                Some("United States"),
                "Arnold",
                21,
            ),
            (
                "Dr. Miles (1911). Arnold, Schwinn & Co., 388 U.S. 365 (1967).",
                None,
                "Arnold, Schwinn & Co.",
                18,
            ),
        ] {
            let cites = get_citations(text, false, "", &[]);
            let Citation::FullCase(d) = &cites[0] else {
                panic!("expected FullCase: {cites:?}");
            };
            assert_eq!(
                (
                    d.metadata.plaintiff.as_deref(),
                    d.metadata.defendant.as_deref(),
                    d.resource.year,
                ),
                (plaintiff, Some(defendant), Some(1967)),
                "{text}"
            );
            assert_eq!(cites[0].full_span().0, name_start, "{text}");
        }

        // A footnote number after the sentence isn't a case name.
        let text = "33 L.Ed.2d 346 (1972). [2] 298 A.2d 761 (Del. 1972).";
        let cites = get_citations(text, false, "", &[]);
        let Citation::FullCase(d) = &cites[1] else {
            panic!("expected FullCase: {cites:?}");
        };
        assert_eq!(d.metadata.defendant, None);
        assert_eq!(cites[1].full_span(), (27, 51));

        // Nor does such a citation take the year of the one before it, while
        // the citations of a run after it share the run's year.
        let text = "33 L.Ed.2d 346 (1972). [2] 298 A.2d 761 (Del. 1972). [3] 282 N.C. 431, \
                    194 S.E.2d 19 (1973). [4] ___ U.S. ___, 112 S.Ct. 773, 116 L.Ed.2d 903 \
                    (1992).";
        let years = case_years(&get_citations(text, false, "", &[]));
        let years: Vec<_> = years.iter().map(|(t, y, _)| (t.as_str(), *y)).collect();
        assert_eq!(
            years,
            [
                ("33 L.Ed.2d 346", Some(1972)),
                ("298 A.2d 761", Some(1972)),
                ("282 N.C. 431", Some(1973)),
                ("194 S.E.2d 19", Some(1973)),
                ("112 S.Ct. 773", Some(1992)),
                ("116 L.Ed.2d 903", Some(1992)),
            ]
        );
    }

    #[test]
    fn test_ambiguous_flag() {
        let cites = get_citations("1 U.S. 1", false, "", &[]);
        assert!(!cites[0].is_ambiguous(), "U.S. should not be ambiguous");
    }

    /// Port of eyecite's `test_relaxed_reporter_whitespace` (eyecite #305):
    /// reporters printed with spaced-out abbreviations still match, while
    /// periods stay mandatory.
    #[test]
    fn test_relaxed_reporter_whitespace() {
        fn group<'a>(c: &'a Citation, name: &str) -> Option<&'a str> {
            c.resource_common()?.common.groups.get(name)?.as_deref()
        }

        // (text, type, reporter, reporter_found, [(group, value)])
        type Case<'a> = (&'a str, &'a str, &'a str, &'a str, &'a [(&'a str, &'a str)]);
        let cases: &[Case] = &[
            (
                "123 N. Y. S. 2d 456",
                "FullCaseCitation",
                "N.Y.S.2d",
                "N. Y. S. 2d",
                &[("volume", "123"), ("page", "456")],
            ),
            (
                "260 F. R. D. 38",
                "FullCaseCitation",
                "F.R.D.",
                "F. R. D.",
                &[("volume", "260"), ("page", "38")],
            ),
            (
                "8 N. Y. 3d 204",
                "FullCaseCitation",
                "N.Y.3d",
                "N. Y. 3d",
                &[("volume", "8"), ("page", "204")],
            ),
            // Partially spaced (space only after the final period)
            (
                "799 N.Y.S. 2d 795",
                "FullCaseCitation",
                "N.Y.S.2d",
                "N.Y.S. 2d",
                &[("volume", "799"), ("page", "795")],
            ),
            // Multi-word reporter with spaced tokens
            (
                "31 Ohio C. C. 270",
                "FullCaseCitation",
                "Ohio C.C.",
                "Ohio C. C.",
                &[("volume", "31"), ("page", "270")],
            ),
            // Compact forms still match unchanged
            (
                "799 N.Y.S.2d 795",
                "FullCaseCitation",
                "N.Y.S.2d",
                "N.Y.S.2d",
                &[("volume", "799"), ("page", "795")],
            ),
            // Short cite through the short_cite_re path
            (
                "See Adarand, 799 N. Y. S. 2d, at 797",
                "ShortCaseCitation",
                "N.Y.S.2d",
                "N. Y. S. 2d",
                &[("volume", "799"), ("page", "797")],
            ),
            // Laws and journals go through the same chokepoint
            (
                "18 U. S. C. \u{00a7} 4241",
                "FullLawCitation",
                "U.S.C.",
                "U. S. C.",
                &[("title", "18"), ("section", "4241")],
            ),
            (
                "1 F. B. I. S. 1",
                "FullJournalCitation",
                "F.B.I.S.",
                "F. B. I. S.",
                &[],
            ),
        ];

        for &(text, type_name, reporter, found, groups) in cases {
            let cites = get_citations(text, false, "", &[]);
            assert_eq!(cites.len(), 1, "{text:?}: {cites:?}");
            let c = &cites[0];
            assert_eq!(c.type_name(), type_name, "{text:?}");
            assert_eq!(
                c.corrected_reporter().as_deref(),
                Some(reporter),
                "{text:?}"
            );
            assert_eq!(group(c, "reporter"), Some(found), "{text:?}");
            for &(name, value) in groups {
                assert_eq!(group(c, name), Some(value), "{text:?} group {name}");
            }
        }

        let cites = get_citations("See Adarand, 799 N. Y. S. 2d, at 797", false, "", &[]);
        match &cites[0] {
            Citation::ShortCase(d) => {
                assert_eq!(d.metadata.antecedent_guess.as_deref(), Some("Adarand"))
            }
            other => panic!("expected short case, got {other:?}"),
        }

        // Periods stay mandatory.
        assert!(get_citations("123 N Y S 2d 456", false, "", &[]).is_empty());
    }

    /// Port of eyecite's `test_no_duplicate_editions` (eyecite #317): the
    /// same edition is never listed twice in `all_editions`, but genuine
    /// ambiguity between distinct editions is preserved.
    #[test]
    fn test_no_duplicate_editions() {
        for text in [
            "238 F.3d 273", // whitespace-only variation
            "410 U.S. 113",
            "123 F.2d 456",
            "5 Cal. 4th 1",
            "410 S.W.2d 1", // custom regex without $edition
        ] {
            let cites = get_citations(text, false, "", &[]);
            let rc = cites[0].resource_common().unwrap();
            let names: Vec<_> = rc.all_editions.iter().map(|e| &e.short_name).collect();
            assert_eq!(
                rc.all_editions.len(),
                1,
                "{text:?} has duplicate editions: {names:?}"
            );
        }

        // "B.R." maps to two distinct editions (Bankruptcy Reporter and
        // Baltimore City Reports); de-duplication must keep both.
        let cites = get_citations("1 B.R. 1", false, "", &[]);
        let rc = cites[0].resource_common().unwrap();
        assert_eq!(rc.all_editions.len(), 2);
        assert_ne!(rc.all_editions[0], rc.all_editions[1]);
    }
}
