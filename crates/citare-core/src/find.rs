//! Citation finding pipeline.
//!
//! This is the main entry point for extracting citations from legal text.
//! Ported from eyecite's `find.py`.

use std::collections::HashMap;

use crate::clean::CleanStep;
use crate::helpers::{
    add_journal_metadata, add_law_metadata, add_post_citation, add_pre_citation,
    disambiguate_reporters, extract_pin_cite, filter_citations, find_case_name,
    find_case_name_in_html, find_case_name_in_html_short, find_case_name_short, guess_court,
    guess_edition,
};
use crate::models::citation::{
    CaseMetadata, Citation, CitationCommon, FullCaseCitationData, FullCaseMetadata,
    FullJournalCitationData, FullLawCitationData, IdCitationData, IdMetadata, JournalMetadata,
    LawMetadata, ResourceCitationCommon, ShortCaseCitationData, ShortCaseMetadata,
    SupraCitationData, SupraMetadata, UnknownCitationData,
};
use crate::models::document::Document;
use crate::models::edition::ReporterSource;
use crate::models::token::{Token, TokenOrStr, Tokens};
use crate::regexes::{SUPRA_ANTECEDENT_GROUPS, SUPRA_ANTECEDENT_REGEX};
use crate::tokenizer::AhocorasickTokenizer;

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
    // 1. Create Document
    let mut document = Document::new(plain_text, markup_text, clean_steps);

    // 2. Tokenize
    let tokenizer = AhocorasickTokenizer::new();
    document.tokenize(&tokenizer);

    // 3. For each citation token, extract citation
    let mut citations: Vec<Citation> = Vec::new();

    // We need to collect citation_tokens info before processing since we
    // need mutable access to words during processing but the token info
    // is derived from that same data.
    let citation_token_info: Vec<(usize, Token)> = document.citation_tokens.clone();

    for (i, token) in &citation_token_info {
        let i = *i;
        match token {
            // CASE 1: Citation token (reporter, law journal, or law)
            Token::Citation(ct) => {
                if ct.short {
                    let citation = extract_shortform_citation(&document, i);
                    citations.push(citation);
                } else {
                    let mut citation = extract_full_citation(&document, i);

                    // Check for parallel citations
                    if !citations.is_empty() {
                        if let (Citation::FullCase(_), Some(Citation::FullCase(_))) =
                            (&citation, citations.last())
                        {
                            handle_parallel_citation(&mut citation, citations.last().unwrap());
                        }
                    }

                    // Check for reference citations that follow a full
                    // citation, using the plaintiff or defendant
                    let references = extract_reference_citations(&citation, &document);
                    citations.extend(references);

                    citations.push(citation);
                }
            }

            // CASE 2: Id/Ibid citation
            Token::Id(_) => {
                let citation = extract_id_citation(&document.words, i);
                citations.push(citation);
            }

            // CASE 3: Supra citation
            Token::Supra(_) => {
                let citation = extract_supra_citation(&document.words, i);
                citations.push(citation);
            }

            // CASE 4: Section marker -> Unknown citation
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

    // 4. Filter and sort
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

    let groups = token.groups().clone();
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

    let groups = token.groups().clone();
    let page_prefix = groups.get("page").and_then(|v| v.as_deref()).unwrap_or("");

    let (pin_cite, span_end, parenthetical) = extract_pin_cite(&document.words, index, page_prefix);

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

fn extract_id_citation(words: &Tokens, index: usize) -> Citation {
    let token = match &words[index] {
        TokenOrStr::Token(t) => t.clone(),
        _ => unreachable!("Expected token at id index"),
    };

    let (pin_cite, span_end, parenthetical) = extract_pin_cite(words, index, "");

    let common = CitationCommon {
        token: token.clone(),
        index,
        span_start: None,
        span_end,
        full_span_start: None,
        full_span_end: None,
        groups: token.groups().clone(),
    };

    Citation::Id(IdCitationData {
        common,
        metadata: IdMetadata {
            pin_cite,
            parenthetical,
            ..Default::default()
        },
    })
}

// ---------------------------------------------------------------------------
// Handle parallel citations
// ---------------------------------------------------------------------------

fn handle_parallel_citation(current: &mut Citation, preceding: &Citation) {
    if let (Citation::FullCase(ref mut cur_data), Citation::FullCase(ref pre_data)) =
        (current, preceding)
    {
        if cur_data.resource.common.full_span_start == pre_data.resource.common.full_span_start {
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

/// Extract reference citations that follow a full case citation, from the
/// plain text and, when the document came from markup, from emphasis tags.
fn extract_reference_citations(citation: &Citation, document: &Document) -> Vec<Citation> {
    let Citation::FullCase(data) = citation else {
        return Vec::new();
    };
    if document.plain_text.len() <= citation.span().1 {
        return Vec::new();
    }
    let mut references = extract_pincited_reference_citations(citation, &document.plain_text);
    if document.markup_text.is_some() {
        references.extend(find_reference_citations_from_markup(
            document, citation, data,
        ));
    }
    references
}

/// Extract reference citations with the name-pincite pattern
/// ("Foo at 123").
fn extract_pincited_reference_citations(citation: &Citation, plain_text: &str) -> Vec<Citation> {
    let Citation::FullCase(data) = citation else {
        return Vec::new();
    };

    let (_, span_end) = citation.span();
    if span_end >= plain_text.len() {
        return Vec::new();
    }

    let regexes: Vec<String> = reference_names(data)
        .into_iter()
        .map(|(key, value)| format!("(?P<{key}>{})", regex::escape(value)))
        .collect();
    if regexes.is_empty() {
        return Vec::new();
    }

    let pin_cite_re = crate::regexes::reference_pin_cite_re(
        &regexes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );
    let Ok(re) = fancy_regex::Regex::new(&pin_cite_re) else {
        return Vec::new();
    };

    let remaining_text = &plain_text[span_end..];
    re.captures_iter(remaining_text)
        .filter_map(Result::ok)
        .map(|caps| {
            let m = caps.get(0).unwrap();
            let span = (span_end + m.start(), span_end + m.end());
            make_reference(m.as_str().to_string(), span, span, |name| {
                caps.name(name).map(|g| g.as_str().to_string())
            })
        })
        .collect()
}

fn re_after_markup_reference() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^\s*(v[.s]|supra)\s").unwrap())
}

/// Use `<em>`/`<i>` style tags and the citation's party names to find
/// reference citations in the markup (eyecite
/// `find_reference_citations_from_markup`). Spans are mapped back to the
/// plain text, like every other citation's.
fn find_reference_citations_from_markup(
    document: &Document,
    citation: &Citation,
    data: &FullCaseCitationData,
) -> Vec<Citation> {
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

    // Include punctuation and spaces around the party name so the match is
    // contiguous with the style tags.
    let pattern = format!(r"<(?:em|i)>\s*({})[:;.,\s]*</(?:em|i)>", regexes.join("|"));
    let Ok(re) = regex::Regex::new(&pattern) else {
        return Vec::new();
    };

    let start_in_markup = to_markup.update(citation.span().0, true);
    let Some(haystack) = markup.get(start_in_markup..) else {
        return Vec::new();
    };

    let mut references = Vec::new();
    for caps in re.captures_iter(haystack) {
        let whole = caps.get(0).unwrap();
        let name = caps.get(1).unwrap();
        let full_start = to_plain.update(start_in_markup + whole.start(), false);
        let full_end = to_plain.update(start_in_markup + whole.end(), true);
        let start = to_plain.update(start_in_markup + name.start(), false);
        let end = to_plain.update(start_in_markup + name.end(), true);

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

    #[test]
    fn test_placeholder_citation() {
        // "585 U.S. ___" uses the placeholder page token; it should still
        // produce a FullCase citation — the page group is "___" not None.
        let cites = get_citations("585 U.S. ___ (2018)", false, "", &[]);
        assert!(
            !cites.is_empty(),
            "Expected a citation for placeholder page"
        );
        if let Citation::FullCase(d) = &cites[0] {
            // Placeholder page is captured as "___", not absent
            let page = d
                .resource
                .common
                .groups
                .get("page")
                .and_then(|v| v.as_deref());
            assert_eq!(page, Some("___"), "Placeholder page should be '___'");
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

    #[test]
    fn test_ambiguous_flag() {
        let cites = get_citations("1 U.S. 1", false, "", &[]);
        assert!(!cites[0].is_ambiguous(), "U.S. should not be ambiguous");
    }
}
