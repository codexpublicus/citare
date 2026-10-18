//! The eyecite-shaped entry points (#25): `get_citations_with` and its
//! tokenizer option, `resolve_citations_with` and its resolvers, and
//! `dump_citations`.

use std::collections::HashMap;

use citare_core::tokenizer::{build_extractors, TokenType};
use citare_core::{
    dump_citations, get_citations, get_citations_with, resolve_citations, resolve_citations_with,
    AhocorasickTokenizer, Citation, CleanStep, DefaultResolver, FindOptions, ResolveContext,
    Resolver, Tokenizer,
};

const TEXT: &str = "Foo v. Bar, 1 U.S. 1, 3 (1999). Id. at 2. See Baz v. Qux, 2 U.S. 50 \
                    (2000). Foo, 1 U.S., at 4. Baz, supra, at 52.";

fn summary(citations: &[Citation]) -> Vec<(&'static str, (usize, usize))> {
    citations
        .iter()
        .map(|c| (c.type_name(), c.span()))
        .collect()
}

// ---------------------------------------------------------------------------
// get_citations_with
// ---------------------------------------------------------------------------

#[test]
fn default_options_match_get_citations() {
    assert_eq!(
        summary(&get_citations_with(TEXT, &FindOptions::new())),
        summary(&get_citations(TEXT, false, "", &[]))
    );
}

#[test]
fn options_pass_markup_clean_steps_and_remove_ambiguous() {
    let markup = "<p>In <i>Foo v. Bar</i>, 1 U.S. 1 (1999), the court held. \
                  Later, in <i>Foo</i>, it was settled.</p>";
    let steps = [CleanStep::Html, CleanStep::AllWhitespace];
    let with = get_citations_with(
        "",
        &FindOptions::new().markup_text(markup).clean_steps(&steps),
    );
    let steps = [CleanStep::Html, CleanStep::AllWhitespace];
    let plain = get_citations("", false, markup, &steps);
    assert_eq!(summary(&with), summary(&plain));
    assert_eq!(
        with.iter().map(Citation::type_name).collect::<Vec<_>>(),
        ["FullCaseCitation", "ReferenceCitation"]
    );

    // "1 So. 1" is ambiguous between editions of the Southern Reporter
    // without a year to tell them apart.
    let text = "1 So. 1";
    let kept = get_citations_with(text, &FindOptions::new());
    let removed = get_citations_with(text, &FindOptions::new().remove_ambiguous(true));
    assert_eq!(
        summary(&removed),
        summary(&get_citations(text, true, "", &[]))
    );
    assert!(kept.len() >= removed.len());
}

/// eyecite's `test_custom_tokenizer`: copies of the extractors whose
/// regexes take a comma wherever they took a period.
#[test]
fn custom_tokenizer_with_edited_extractors() {
    let extractors: Vec<_> = build_extractors()
        .iter()
        // Every extractor would compile 10,000 regexes; the U.S. ones and
        // the one-offs are enough here.
        .filter(|e| e.token_type != TokenType::Citation || e.strings.iter().any(|s| s == "U.S."))
        .map(|e| {
            let mut e = e.clone();
            e.regex = e.regex.replace(r"\.", "[.,]");
            e
        })
        .collect();
    assert!(extractors
        .iter()
        .any(|e| e.token_type == TokenType::Citation));
    let tokenizer = Tokenizer::from_extractors(extractors);

    let text = "1 U,S, 1";
    assert!(get_citations(text, false, "", &[]).is_empty());
    let citations = get_citations_with(text, &FindOptions::new().tokenizer(&tokenizer));
    assert_eq!(citations.len(), 1);
    let groups = &citations[0].common().groups;
    assert_eq!(groups["reporter"].as_deref(), Some("U,S,"));
    assert_eq!(groups["volume"].as_deref(), Some("1"));
    assert_eq!(citations[0].corrected_reporter().as_deref(), Some("U.S."));
}

/// A caller's own extractor added to the default ones, through the
/// Aho-Corasick prefilter.
#[test]
fn custom_extractor_added_to_the_default_tokenizer() {
    let mut extractors = build_extractors();
    let us = extractors
        .iter()
        .find(|e| e.token_type == TokenType::Citation && e.strings.iter().any(|s| s == "U.S."))
        .expect("a U.S. extractor")
        .clone();
    extractors.push(citare_core::TokenExtractor::new(
        r"((?P<volume>\d+)\s+(?P<reporter>Supreme Ct\. Rep\.)\s+(?P<page>\d+))",
        TokenType::Citation,
        us.extra.clone(),
        vec!["Supreme Ct.".to_string()],
        false,
    ));
    let tokenizer = AhocorasickTokenizer::from_extractors(extractors);
    assert!(tokenizer.extractors().len() > 1);

    let text = "Foo v. Bar, 5 Supreme Ct. Rep. 7 (1999).";
    let citations = get_citations_with(text, &FindOptions::new().tokenizer(&tokenizer));
    assert_eq!(citations.len(), 1, "{citations:?}");
    assert_eq!(citations[0].matched_text(), "5 Supreme Ct. Rep. 7");
    let Citation::FullCase(case) = &citations[0] else {
        panic!("expected a full case citation");
    };
    assert_eq!(case.metadata.plaintiff.as_deref(), Some("Foo"));
    assert_eq!(case.metadata.defendant.as_deref(), Some("Bar"));

    // The default tokenizer doesn't know the reporter.
    assert!(get_citations(text, false, "", &[]).is_empty());
}

// ---------------------------------------------------------------------------
// resolve_citations_with
// ---------------------------------------------------------------------------

/// Resource groups as (resource, matched texts), in document order.
fn grouped<R: std::fmt::Debug>(groups: HashMap<R, Vec<Citation>>) -> Vec<(String, Vec<String>)> {
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, cites)| cites[0].span());
    groups
        .into_iter()
        .map(|(r, cites)| {
            (
                format!("{r:?}"),
                cites.iter().map(|c| c.matched_text().to_string()).collect(),
            )
        })
        .collect()
}

#[test]
fn default_resolver_matches_resolve_citations() {
    let citations = get_citations(TEXT, false, "", &[]);
    let by_default = resolve_citations(&citations);
    let with = resolve_citations_with(&citations, &mut DefaultResolver::default());
    // Id. citations compare by identity, so compare the groups' texts.
    assert_eq!(grouped(by_default), grouped(with.clone()));
    assert_eq!(with.len(), 2);
}

/// Groups citations by a caller's ids, leaving the short forms to citare's
/// rules.
struct ById {
    ids: HashMap<&'static str, u32>,
    full_calls: usize,
}

impl Resolver for ById {
    type Resource = u32;

    fn resolve_full(&mut self, citation: &Citation) -> Option<u32> {
        self.full_calls += 1;
        self.ids.get(citation.matched_text()).copied()
    }
}

#[test]
fn own_resources_get_short_forms_by_default() {
    let citations = get_citations(TEXT, false, "", &[]);
    let mut resolver = ById {
        ids: HashMap::from([("1 U.S. 1", 7), ("2 U.S. 50", 9)]),
        full_calls: 0,
    };
    let groups = grouped(resolve_citations_with(&citations, &mut resolver));
    assert_eq!(resolver.full_calls, 2);
    assert_eq!(
        groups,
        [
            (
                "7".into(),
                vec!["1 U.S. 1".into(), "Id.".into(), "1 U.S., at 4".into()]
            ),
            ("9".into(), vec!["2 U.S. 50".into(), "supra,".into()]),
        ]
    );
}

#[test]
fn unresolved_full_citation_is_dropped_and_blocks_its_short_forms() {
    let citations = get_citations(TEXT, false, "", &[]);
    // Only Baz v. Qux is known.
    let mut resolver = ById {
        ids: HashMap::from([("2 U.S. 50", 9)]),
        full_calls: 0,
    };
    let groups = grouped(resolve_citations_with(&citations, &mut resolver));
    assert_eq!(
        groups,
        [("9".into(), vec!["2 U.S. 50".into(), "supra,".into()])]
    );

    // Two full citations in one volume, the second unresolved: the short
    // form matches both, so it is ambiguous, not the first's.
    let text = "Foo v. Bar, 1 U.S. 1 (1999). Foo v. Baz, 1 U.S. 50 (1999). \
                Foo, 1 U.S., at 52.";
    let citations = get_citations(text, false, "", &[]);
    let mut resolver = ById {
        ids: HashMap::from([("1 U.S. 1", 7)]),
        full_calls: 0,
    };
    let groups = grouped(resolve_citations_with(&citations, &mut resolver));
    assert_eq!(groups, [("7".into(), vec!["1 U.S. 1".into()])]);
}

/// Overrides every short form, checking what the context holds.
struct Overrides {
    seen_full: Vec<usize>,
}

impl Resolver for Overrides {
    type Resource = String;

    fn resolve_full(&mut self, citation: &Citation) -> Option<String> {
        Some(citation.matched_text().to_string())
    }

    fn resolve_shortcase(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, String>,
    ) -> Option<String> {
        self.seen_full.push(context.full_citations().count());
        // Post-filter citare's rule.
        context
            .resolve_shortcase(citation)
            .map(|r| format!("{r} (short)"))
    }

    fn resolve_supra(&mut self, _: &Citation, _: &ResolveContext<'_, String>) -> Option<String> {
        None
    }

    fn resolve_id(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, String>,
    ) -> Option<String> {
        assert_eq!(
            context.last_resolution().map(String::as_str),
            Some("1 U.S. 1")
        );
        assert_eq!(context.resolutions().len(), 1);
        // The built-in rules don't cross kinds.
        assert_eq!(context.resolve_supra(citation), None);
        context.resolve_id(citation)
    }
}

#[test]
fn overrides_see_the_context_and_can_call_the_built_in_rules() {
    let citations = get_citations(TEXT, false, "", &[]);
    let mut resolver = Overrides { seen_full: vec![] };
    let groups = grouped(resolve_citations_with(&citations, &mut resolver));
    assert_eq!(resolver.seen_full, [2]);
    assert_eq!(
        groups,
        [
            ("\"1 U.S. 1\"".into(), vec!["1 U.S. 1".into(), "Id.".into()]),
            ("\"2 U.S. 50\"".into(), vec!["2 U.S. 50".into()]),
            ("\"1 U.S. 1 (short)\"".into(), vec!["1 U.S., at 4".into()]),
        ]
    );
}

#[test]
fn resolver_trait_objects_work() {
    let citations = get_citations(TEXT, false, "", &[]);
    let mut resolver = DefaultResolver::default();
    let dynamic: &mut dyn Resolver<Resource = citare_core::Resource> = &mut resolver;
    assert_eq!(resolve_citations_with(&citations, dynamic).len(), 2);
}

// ---------------------------------------------------------------------------
// dump_citations
// ---------------------------------------------------------------------------

#[test]
fn dump_lists_each_citation_with_context() {
    let text = "Intro line.\nblah. Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3.\nNext line.";
    let citations = get_citations(text, false, "", &[]);
    let dump = dump_citations(&citations, text, 30, false);
    let lines: Vec<&str> = dump.lines().collect();
    assert_eq!(
        lines[0],
        "FullCaseCitation: blah. Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3."
    );
    for expected in [
        "  * groups",
        "    * reporter=\"U.S.\"",
        "  * metadata",
        "    * pin_cite=\"2\"",
        "    * plaintiff=\"Foo\"",
        "  * year=1999",
        "  * corrected_reporter=\"U.S.\"",
    ] {
        assert!(lines.contains(&expected), "{expected:?} in\n{dump}");
    }
    // Unset metadata is left out.
    assert!(!dump.contains("month="));
    // Context stops at the line break.
    let id = lines
        .iter()
        .find(|l| l.starts_with("IdCitation:"))
        .expect("id line");
    assert!(id.ends_with("Id. at 3."), "{id}");

    let colored = dump_citations(&citations, text, 5, true);
    assert!(colored.starts_with("\x1b[32mFullCaseCitation:\x1b[0m Bar, \x1b[94m\x1b[1m1 U.S. 1"));
    assert_eq!(dump_citations(&[], text, 30, false), "");
}
