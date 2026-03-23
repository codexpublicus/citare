use citare_core::{annotate_citations, get_citations, resolve_citations, CleanStep, UnbalancedTags};

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
            ((start, end), format!("<a id=\"{}\">", i), "</a>".to_string())
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
    assert!(annotated.contains("1 U.S. 1"), "missing first citation text");
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
            ((start, end), format!("<cite n=\"{}\">", i), "</cite>".to_string())
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
