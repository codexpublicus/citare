use citare_core::{
    annotate_citations, get_citations, resolve_citations, CleanStep, UnbalancedTags,
};

#[test]
fn test_full_pipeline() {
    // Two distinct full-case citations plus an Id. back-reference.
    let text = "Foo v. Bar, 1 U.S. 1 (1999). Baz v. Qux, 2 U.S. 100 (2000). Id. at 102.";

    // Step 1: Find citations
    let citations = get_citations(text, false, "", &[]);
    assert!(
        citations.len() >= 3,
        "Expected 3+ citations (2 full + 1 id), got {}: {:?}",
        citations.len(),
        citations.iter().map(|c| c.type_name()).collect::<Vec<_>>(),
    );

    // Step 2: Resolve citations
    let resolutions = resolve_citations(&citations);
    assert!(!resolutions.is_empty());

    // Step 3: Annotate
    let mut annotations: Vec<((usize, usize), String, String)> = citations
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let (start, end) = c.span();
            (
                (start, end),
                format!("<a id=\"{}\">", i),
                "</a>".to_string(),
            )
        })
        .collect();
    let annotated = annotate_citations(
        text,
        &mut annotations,
        "",
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert!(annotated.contains("<a id="), "missing opening tag");
    assert!(annotated.contains("</a>"), "missing closing tag");
    assert!(
        annotated.contains("1 U.S. 1"),
        "missing first citation text"
    );
}

#[test]
fn test_pipeline_with_cleaning() {
    let text = "1  U.S.  1"; // double spaces
    let citations = get_citations(text, false, "", &[CleanStep::AllWhitespace]);
    assert!(
        !citations.is_empty(),
        "Expected citation after cleaning whitespace"
    );
}

#[test]
fn test_pipeline_id_resolves_to_full_cite() {
    let text = "1 U.S. 1 (1999). Id. at 2.";
    let citations = get_citations(text, false, "", &[]);
    let resolutions = resolve_citations(&citations);
    // Both the full and id. citation resolve to the same resource.
    assert_eq!(resolutions.len(), 1, "Expected one unique resource");
    let total: usize = resolutions.values().map(|v| v.len()).sum();
    assert_eq!(total, 2, "Expected 2 citations resolved");
}

#[test]
fn test_pipeline_annotation_order_independent() {
    // Annotations provided in reverse order must still produce correct output.
    let text = "1 U.S. 1 and 2 U.S. 2";
    let citations = get_citations(text, false, "", &[]);
    // We expect at least two full-case citations.
    let full: Vec<_> = citations
        .iter()
        .filter(|c| matches!(c, citare_core::Citation::FullCase(_)))
        .collect();
    assert!(full.len() >= 2, "Expected >= 2 full case citations");

    // Build annotations in reverse order.
    let mut annotations: Vec<((usize, usize), String, String)> = full
        .iter()
        .rev()
        .enumerate()
        .map(|(i, c)| {
            let (start, end) = c.span();
            (
                (start, end),
                format!("<cite n=\"{}\">", i),
                "</cite>".to_string(),
            )
        })
        .collect();

    let annotated = annotate_citations(
        text,
        &mut annotations,
        "",
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert!(annotated.contains("<cite"), "missing opening cite tag");
    assert!(annotated.contains("</cite>"), "missing closing cite tag");
}

#[test]
fn test_pipeline_empty_text() {
    let citations = get_citations("", false, "", &[]);
    assert!(citations.is_empty());
    let resolutions = resolve_citations(&citations);
    assert!(resolutions.is_empty());
}

#[test]
fn test_pipeline_no_citations() {
    let text = "This is ordinary prose with no legal citations.";
    let citations = get_citations(text, false, "", &[]);
    assert!(citations.is_empty());
}

#[test]
fn test_harvard_slug_us_via_pipeline() {
    // "1 U.S. 1" should yield a FullCaseCitation whose edition_guess (or
    // all_editions) carries harvard_slug = Some("us").
    let citations = get_citations("1 U.S. 1", false, "", &[]);
    let full: Vec<_> = citations
        .iter()
        .filter(|c| matches!(c, citare_core::Citation::FullCase(_)))
        .collect();
    assert!(!full.is_empty(), "Expected at least one FullCaseCitation");

    let slug = full[0]
        .resource_common()
        .and_then(|r| r.edition_guess.as_ref())
        .and_then(|e| e.harvard_slug.as_deref())
        .or_else(|| {
            full[0]
                .resource_common()
                .and_then(|r| r.all_editions.first())
                .and_then(|e| e.harvard_slug.as_deref())
        });

    assert_eq!(
        slug,
        Some("us"),
        "U.S. citation should have harvard_slug = Some(\"us\")"
    );
}

#[test]
fn test_harvard_slug_a2d_via_pipeline() {
    let citations = get_citations("1 A.2d 1", false, "", &[]);
    let full: Vec<_> = citations
        .iter()
        .filter(|c| matches!(c, citare_core::Citation::FullCase(_)))
        .collect();
    assert!(
        !full.is_empty(),
        "Expected at least one FullCaseCitation for A.2d"
    );

    let slug = full[0]
        .resource_common()
        .and_then(|r| r.edition_guess.as_ref())
        .and_then(|e| e.harvard_slug.as_deref())
        .or_else(|| {
            full[0]
                .resource_common()
                .and_then(|r| r.all_editions.first())
                .and_then(|e| e.harvard_slug.as_deref())
        });

    assert_eq!(
        slug,
        Some("a2d"),
        "A.2d citation should have harvard_slug = Some(\"a2d\")"
    );
}

// Ported from eyecite tests/test_AnnotateTest.py: the annotator still
// renders the original underscores even though `page` is None.
#[test]
fn test_annotate_blank_page() {
    let text = "foo 1 U.S. ____ bar";
    let citations = get_citations(text, false, "", &[]);
    assert_eq!(citations.len(), 1);
    assert_eq!(
        citations[0].common().groups.get("page").cloned().flatten(),
        None
    );
    let mut annotations: Vec<((usize, usize), String, String)> = citations
        .iter()
        .enumerate()
        .map(|(i, c)| (c.span(), format!("<{}>", i), format!("</{}>", i)))
        .collect();
    let annotated = annotate_citations(
        text,
        &mut annotations,
        "",
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert_eq!(annotated, "foo <0>1 U.S. ____</0> bar");
}

/// The CAP slug for the first full case citation in `text`, picked the way
/// the CLI and WASM bindings pick it.
fn harvard_slug_of(text: &str) -> Option<String> {
    let citations = get_citations(text, false, "", &[]);
    let rc = citations
        .iter()
        .find(|c| matches!(c, citare_core::Citation::FullCase(_)))
        .unwrap_or_else(|| panic!("no full case citation in {text:?}"))
        .resource_common()?;
    rc.edition_guess
        .as_ref()
        .and_then(|e| e.harvard_slug.clone())
        .or_else(|| rc.all_editions.first().and_then(|e| e.harvard_slug.clone()))
}

// Every spelling of a reporter that has a CAP slug gets that slug (#29):
// spaced forms, reporters-db variations, and editions whose CAP short_name
// is a reporters-db variation rather than the edition name (CAP's "So.2d",
// "N.Y. 2d", "Misc.2d" against reporters-db's "So. 2d", "N.Y.2d", "Misc. 2d").
#[rstest::rstest]
#[case("1 U. S. 1", "us")]
#[case("1 U.S. 1", "us")]
#[case("1 US 1", "us")]
#[case("1 F. 2d 1", "f2d")]
#[case("1 F 2d 1", "f2d")]
#[case("1 S. Ct. 1", "s-ct")]
#[case("1 S.Ct. 1", "s-ct")]
#[case("1 Sup.Ct. 1", "s-ct")]
#[case("12 N. Y. S. 34", "nys")]
#[case("1 So. 2d 1", "so2d")]
#[case("1 So.2d 1", "so2d")]
#[case("1 So. 3d 1", "so3d")]
#[case("1 N.Y.2d 1", "ny-2d")]
#[case("1 N. Y. 2d 1", "ny-2d")]
#[case("1 NY2d 1", "ny-2d")]
#[case("1 Misc. 2d 1", "misc2d")]
#[case("1 Misc.2d 1", "misc2d")]
#[case("1 Misc 3d 1", "misc3d")]
fn test_harvard_slug_for_every_spelling(#[case] text: &str, #[case] slug: &str) {
    assert_eq!(harvard_slug_of(text).as_deref(), Some(slug), "{text}");
}

// CAP has no N.Y.S.2d volumes (static.case.law/nys2d does not exist), so no
// spelling of it may claim a slug; a guessed "nys2d" would be a dead link.
#[rstest::rstest]
#[case("12 N.Y.S.2d 34")]
#[case("12 N. Y. S. 2d 34")]
#[case("12 NYS2d 34")]
fn test_harvard_slug_none_for_nys2d(#[case] text: &str) {
    assert_eq!(harvard_slug_of(text), None, "{text}");
}

// CAP's "P.R." (Puerto Rico Reports) is a reporters-db variation of several
// editions, Pacific Reporter among them, so it must not leak onto any of them.
#[test]
fn test_harvard_slug_ambiguous_variation_not_assigned() {
    assert_ne!(harvard_slug_of("1 P. 1").as_deref(), Some("pr"));
}
