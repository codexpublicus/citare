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
    normalize_blank_page, CaseMetadata, Citation, CitationCommon, FullCaseCitationData,
    FullCaseMetadata, FullJournalCitationData, FullLawCitationData, IdCitationData, IdMetadata,
    JournalMetadata, LawMetadata, ResourceCitationCommon, ShortCaseCitationData, ShortCaseMetadata,
    SupraCitationData, SupraMetadata, UnknownCitationData,
};
use crate::models::document::Document;
use crate::models::edition::ReporterSource;
use crate::models::token::{Token, TokenOrStr, Tokens};
use crate::regexes::{SUPRA_ANTECEDENT_GROUPS, SUPRA_ANTECEDENT_REGEX};
use crate::tokenizer::DEFAULT_TOKENIZER;

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
    document.tokenize(&DEFAULT_TOKENIZER);

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
                    let references =
                        extract_pincited_reference_citations(&citation, &document.plain_text);
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

        // Find case name (plaintiff/defendant)
        // For now, use plain text only (markup-aware is Phase 8)
        find_case_name(&mut data, &document.words, false);

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

    // Find case name (antecedent)
    find_case_name_short(&mut data, &document.words);

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
// Extract pincited reference citations
// ---------------------------------------------------------------------------

fn extract_pincited_reference_citations(citation: &Citation, plain_text: &str) -> Vec<Citation> {
    // Only applies to full case citations
    let data = match citation {
        Citation::FullCase(d) => d,
        _ => return Vec::new(),
    };

    let (_, span_end) = citation.span();
    if span_end >= plain_text.len() {
        return Vec::new();
    }

    // Build one named alternative per usable name, in eyecite's
    // `ReferenceCitation.name_fields` order.
    let name_fields: [(&str, &Option<String>); 4] = [
        ("plaintiff", &data.metadata.plaintiff),
        ("defendant", &data.metadata.defendant),
        (
            "resolved_case_name_short",
            &data.metadata.resolved_case_name_short,
        ),
        ("resolved_case_name", &data.metadata.resolved_case_name),
    ];
    let usable: Vec<(&str, &str)> = name_fields
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_deref()
                .filter(|v| crate::utils::is_valid_name(v))
                .map(|v| (*key, v))
        })
        .collect();
    let regexes: Vec<String> = usable
        .iter()
        .map(|(key, v)| format!("(?P<{}>{})", key, regex::escape(v)))
        .collect();
    let names: Vec<&str> = usable.iter().map(|(_, v)| *v).collect();

    if regexes.is_empty() {
        return Vec::new();
    }

    // A reference match must contain one of the names verbatim (they are
    // escaped and matched case-sensitively). Skip compiling the
    // per-citation regex when none appears after the citation, which is
    // the common case.
    let remaining_text = &plain_text[span_end..];
    if !names.iter().any(|name| remaining_text.contains(name)) {
        return Vec::new();
    }

    let pin_cite_re = crate::regexes::reference_pin_cite_re(
        &regexes.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
    );

    let re = match fancy_regex::Regex::new(&pin_cite_re) {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    let mut references = Vec::new();

    // Like eyecite's `match.groupdict()`, the metadata holds only the groups
    // that took part in the match: the matched name and the pin cite.
    let mut pos = 0;
    while pos <= remaining_text.len() {
        let caps = match re.captures_from_pos(remaining_text, pos) {
            Ok(Some(caps)) => caps,
            _ => break,
        };
        let m = caps.get(0).expect("group 0 is always present");
        let group = |name: &str| caps.name(name).map(|g| g.as_str().to_string());

        let abs_start = span_end + m.start();
        let abs_end = span_end + m.end();

        let token = Token::CaseReference(crate::models::token::TokenData {
            data: m.as_str().to_string(),
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

        references.push(Citation::Reference(
            crate::models::citation::ReferenceCitationData {
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
            },
        ));

        pos = if m.end() > m.start() {
            m.end()
        } else {
            // Step past an empty match without splitting a UTF-8 character.
            match remaining_text[m.end()..].chars().next() {
                Some(c) => m.end() + c.len_utf8(),
                None => break,
            }
        };
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
        let refs = extract_pincited_reference_citations(&cites[0], text);
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
