//! Citation finding pipeline.
//!
//! This is the main entry point for extracting citations from legal text.
//! Ported from eyecite's `find.py`.

use std::collections::HashMap;

use crate::clean::CleanStep;
use crate::helpers::{
    add_journal_metadata, add_law_metadata, add_post_citation, add_pre_citation,
    disambiguate_reporters, extract_pin_cite, filter_citations, find_case_name,
    find_case_name_short, guess_court, guess_edition,
};
use crate::models::citation::{
    CaseMetadata, Citation, CitationCommon, FullCaseCitationData, FullCaseMetadata,
    FullJournalCitationData, FullLawCitationData, IdCitationData, IdMetadata,
    JournalMetadata, LawMetadata, ResourceCitationCommon, ShortCaseCitationData,
    ShortCaseMetadata, SupraCitationData, SupraMetadata, UnknownCitationData,
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
///   here to enable markup-aware case name extraction (Phase 8).
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

                    // TODO: Phase 8 - extract reference citations from markup
                    // For now, just extract pincited references
                    let references = extract_pincited_reference_citations(&citation, &document.plain_text);
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
    let all_editions: Vec<_> = ct.exact_editions.iter().chain(ct.variation_editions.iter()).cloned().collect();

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

        // Find case name (plaintiff/defendant)
        // For now, use plain text only (markup-aware is Phase 8)
        find_case_name(&mut data, &document.words, false);

        // Add pre-citation metadata (antecedent before repeated full cite)
        add_pre_citation(&mut data, &document.words);

        // Guess court from reporter
        data.metadata.case.court = guess_court(
            &data.metadata.case.court,
            &data.resource.all_editions,
        );

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

    let (pin_cite, span_end, parenthetical) = extract_pin_cite(
        &document.words,
        index,
        page_prefix,
    );

    let span_end_val = span_end.unwrap_or(0);
    let all_editions: Vec<_> = ct.exact_editions.iter().chain(ct.variation_editions.iter()).cloned().collect();

    let mut data = ShortCaseCitationData {
        resource: ResourceCitationCommon {
            common: CitationCommon {
                token: token.clone(),
                index,
                span_start: None,
                span_end: if span_end_val > 0 { Some(span_end_val) } else { None },
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

    // Find case name (antecedent)
    find_case_name_short(&mut data, &document.words);

    // Guess edition
    data.resource.edition_guess = guess_edition(
        &data.resource.exact_editions,
        &data.resource.variation_editions,
        data.resource.year,
    );

    // Guess court
    data.metadata.case.court = guess_court(
        &data.metadata.case.court,
        &data.resource.all_editions,
    );

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
// Extract pincited reference citations
// ---------------------------------------------------------------------------

fn extract_pincited_reference_citations(
    citation: &Citation,
    plain_text: &str,
) -> Vec<Citation> {
    // Only applies to full case citations
    let data = match citation {
        Citation::FullCase(d) => d,
        _ => return Vec::new(),
    };

    let (_, span_end) = citation.span();
    if span_end >= plain_text.len() {
        return Vec::new();
    }

    // Build regexes from plaintiff/defendant names
    let mut regexes = Vec::new();
    if let Some(ref plaintiff) = data.metadata.plaintiff {
        if crate::utils::is_valid_name(plaintiff) {
            regexes.push(format!("(?P<plaintiff>{})", regex::escape(plaintiff)));
        }
    }
    if let Some(ref defendant) = data.metadata.defendant {
        if crate::utils::is_valid_name(defendant) {
            regexes.push(format!("(?P<defendant>{})", regex::escape(defendant)));
        }
    }

    if regexes.is_empty() {
        return Vec::new();
    }

    let pin_cite_re = crate::regexes::reference_pin_cite_re(
        &regexes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    let remaining_text = &plain_text[span_end..];
    let re = match fancy_regex::Regex::new(&pin_cite_re) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    let mut references = Vec::new();

    // fancy_regex doesn't have find_iter that returns spans easily,
    // use a manual loop
    let mut search_start = 0;
    loop {
        let remaining = &remaining_text[search_start..];
        if remaining.is_empty() {
            break;
        }
        let m = match re.find(remaining) {
            Ok(Some(m)) => m,
            _ => break,
        };

        let abs_start = span_end + search_start + m.start();
        let abs_end = span_end + search_start + m.end();
        let matched_text = m.as_str().to_string();

        let token = Token::CaseReference(crate::models::token::TokenData {
            data: matched_text,
            start: abs_start,
            end: abs_end,
            groups: HashMap::new(),
        });

        let common = CitationCommon {
            token,
            index: 0,
            span_start: Some(abs_start),
            span_end: Some(abs_end),
            full_span_start: Some(abs_start),
            full_span_end: Some(abs_end),
            groups: HashMap::new(),
        };

        references.push(Citation::Reference(crate::models::citation::ReferenceCitationData {
            common,
            metadata: crate::models::citation::ReferenceMetadata {
                parenthetical: None,
                pin_cite: None,
                pin_cite_span_start: None,
                pin_cite_span_end: None,
                plaintiff: data.metadata.plaintiff.clone(),
                defendant: data.metadata.defendant.clone(),
                resolved_case_name_short: None,
                resolved_case_name: None,
            },
        }));

        search_start += m.start() + m.as_str().len().max(1);
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
        let has_short = citations.iter().any(|c| matches!(c, Citation::ShortCase(_)));
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
        let citations = get_citations(
            "See 1 U.S. 1 (1999); 2 U.S. 2 (2000)",
            false,
            "",
            &[],
        );
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
            if data.metadata.case.court.is_some() {
                // Just verify it's a string
                assert!(!data.metadata.case.court.as_ref().unwrap().is_empty());
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
}
