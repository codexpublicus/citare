//! `annotate_citations` must place its wrappers exactly around each
//! citation, in plain text and in markup, whatever multibyte characters sit
//! before, inside or after the citation (#21).
//!
//! Citation spans are UTF-8 byte offsets into the plain text. In markup
//! they go through the plain -> markup span map, which must count the same
//! unit and be the same map `Document::plain_to_markup` holds, or the
//! wrappers drift away from the per-citation `markup` spans citare reports.
//! The corpus is generated deterministically from small pieces, in small
//! documents and in documents on both sides of the 16 KB size at which the
//! span map's diff switches to anchored pieces.

use citare_core::{
    annotate_citations, get_citations_from_document, CleanStep, Document, UnbalancedTags,
};

/// Multibyte markers, so a byte/character mix-up in the output shows too.
const OPEN: &str = "⟦";
const CLOSE: &str = "⟧";

/// Text placed before the citation: accented names, curly quotes, an
/// emoji, CJK.
const BEFORE: &[&str] = &[
    "",
    "Señor Bär said “see” ",
    "東京地裁 🎉 noted: ",
    "Peña’s brief — ",
];

/// Citations, some with multibyte characters inside their spans.
const CITATIONS: &[&str] = &[
    "Bär v. Peña, 410 U.S. 113, 115 (1973)",
    "42 U.S.C. § 1983",
    "Müller v. Øster, 547 So.2d 1034 (Fla. 1989)",
    "410 U.S. 113",
];

/// Text placed after the citation.
const AFTER: &[&str] = &["", " (citing “Peña”).", " 🎉 中文の判決.", "; Bär’s reply."];

/// Ways to set a citation in markup.
fn markup_styles(citation: &str) -> Vec<String> {
    let mut out = vec![citation.to_string()];
    if let Some((name, rest)) = citation.split_once(", ") {
        out.push(format!("<i>{name}</i>, {rest}"));
        out.push(format!(
            "<em class=\"case-name\">{name}</em>, <b>{rest}</b>"
        ));
        out.push(format!(
            "<a href=\"/opinion/peña\" title=\"Bär\">{name}, {rest}</a>"
        ));
    } else {
        out.push(format!("<b>{citation}</b>"));
        out.push(format!("<span title=\"§ Peña\">{citation}</span>"));
    }
    out
}

/// One filler paragraph, distinct per `i`, full of multibyte characters.
fn filler(i: usize, markup: bool) -> String {
    let body = format!(
        "Paragraph {i} (ref{i}x): “Bär” and Peña met in 東京 on day {i} 🎉; the court’s note {i} \
         cites nothing at all. "
    );
    if markup {
        format!("<p class=\"para-{i}\"><i>Note {i}</i> {body}</p>\n")
    } else {
        body
    }
}

/// `count` filler paragraphs.
fn padding(start: usize, count: usize, markup: bool) -> String {
    (start..start + count).map(|i| filler(i, markup)).collect()
}

/// Paragraph counts for documents well under, just under and over the
/// 16 KB anchored-diff threshold (plain and markup together).
const PAD: &[usize] = &[0, 30, 120];

/// Padded markup documents are slow to build in a debug test run, so they
/// take every `PADDED_STRIDE`th combination of pieces. It is coprime with
/// the number of separators, so every separator still meets every size.
const PADDED_STRIDE: usize = 5;

fn plain_corpus() -> Vec<String> {
    let mut out = Vec::new();
    for &pad in PAD {
        for before in BEFORE {
            for citation in CITATIONS {
                for after in AFTER {
                    out.push(format!(
                        "{}{before}{citation}{after} {}",
                        padding(0, pad / 2, false),
                        padding(pad / 2, pad - pad / 2, false),
                    ));
                }
            }
        }
    }
    out
}

/// Whitespace and tags set around a citation in markup. Cleaning collapses
/// these, so the plain -> markup diff has a choice of where to put them.
const SEPARATORS: &[(&str, &str)] = &[
    ("", ""),
    ("  ", "  "),
    ("\n ", " \n"),
    ("<br> ", "  <b> </b>"),
];

fn markup_corpus() -> Vec<String> {
    let mut out = Vec::new();
    let mut n = 0;
    for &pad in PAD {
        for before in BEFORE {
            for citation in CITATIONS {
                for styled in markup_styles(citation) {
                    for after in AFTER {
                        n += 1;
                        if pad > 0 && n % PADDED_STRIDE != 0 {
                            continue;
                        }
                        let (lead, trail) = SEPARATORS[n % SEPARATORS.len()];
                        out.push(format!(
                            "<html><body>\n{}<p>{before}{lead}{styled}{trail}{after}</p>\n{}\
                             </body></html>",
                            padding(0, pad / 2, true),
                            padding(pad / 2, pad - pad / 2, true),
                        ));
                    }
                }
            }
        }
    }
    out
}

/// `text` with every annotation marker removed.
fn unmark(text: &str) -> String {
    text.replace(OPEN, "").replace(CLOSE, "")
}

/// The text between each pair of markers, in order.
fn wrapped(text: &str) -> Vec<&str> {
    text.split(OPEN)
        .skip(1)
        .map(|piece| piece.split(CLOSE).next().unwrap())
        .collect()
}

/// Annotate one span per call, so overlap clipping never applies.
fn annotate_one(
    plain: &str,
    span: (usize, usize),
    source: &str,
    updater: Option<&citare_core::SpanUpdater>,
) -> String {
    let mut annotations = vec![(span, OPEN.to_string(), CLOSE.to_string())];
    annotate_citations(
        plain,
        &mut annotations,
        source,
        UnbalancedTags::Unchecked,
        None,
        updater,
    )
}

/// Run `check` on every input; fail listing the inputs that panicked or
/// failed.
fn run_all(inputs: Vec<String>, check: impl Fn(&str) -> Result<(), String>) {
    let total = inputs.len();
    let mut failures = Vec::new();
    for input in &inputs {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(input)));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => failures.push(msg),
            Err(e) => {
                let msg = e
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default();
                failures.push(format!("panic: {msg}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {total} inputs failed; first: {}",
        failures.len(),
        failures
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n---\n"),
    );
}

#[test]
fn plain_wrappers_surround_citation_text() {
    run_all(plain_corpus(), |text| {
        let mut document = Document::new(text, "", &[]);
        let citations = get_citations_from_document(&mut document, false);
        if citations.is_empty() {
            return Err(format!("no citations in {text:?}"));
        }
        for citation in &citations {
            for span in [citation.span(), citation.full_span()] {
                let out = annotate_one(text, span, "", None);
                if unmark(&out) != text {
                    return Err(format!("text changed for {span:?} in {text:?}: {out:?}"));
                }
                let inner = wrapped(&out);
                if inner != [&text[span.0..span.1]] {
                    return Err(format!("wrapped {inner:?} for {span:?} in {text:?}"));
                }
            }
        }
        Ok(())
    });
}

#[test]
fn markup_wrappers_match_markup_spans() {
    let steps = [CleanStep::Html, CleanStep::InlineWhitespace];
    run_all(markup_corpus(), |markup| {
        let mut document = Document::new("", markup, &steps);
        let citations = get_citations_from_document(&mut document, false);
        if citations.is_empty() {
            return Err(format!("no citations in {markup:?}"));
        }
        let plain = &document.plain_text;
        let to_markup = document.plain_to_markup.as_ref().unwrap();
        for citation in &citations {
            for span in [citation.span(), citation.full_span()] {
                // Where citare reports this span in the markup.
                let start = to_markup.update(span.0, true);
                let end = to_markup.update(span.1, false);
                let expected = &markup[start..end];

                for updater in [None, Some(to_markup)] {
                    let out = annotate_one(plain, span, markup, updater);
                    if unmark(&out) != markup {
                        return Err(format!(
                            "markup changed for {span:?} (updater {}) in {markup:?}: {out:?}",
                            updater.is_some(),
                        ));
                    }
                    let inner = wrapped(&out);
                    if inner != [expected] {
                        return Err(format!(
                            "wrapped {inner:?}, expected {expected:?} for plain {:?} \
                             (updater {}) in {markup:?}",
                            &plain[span.0..span.1],
                            updater.is_some(),
                        ));
                    }
                }
            }
            // The citation itself carries no tags in this corpus, so its
            // wrapped markup is exactly its plain text.
            let span = citation.span();
            let out = annotate_one(plain, span, markup, None);
            if wrapped(&out) != [&plain[span.0..span.1]] {
                return Err(format!(
                    "citation {:?} wrapped as {:?} in {markup:?}",
                    &plain[span.0..span.1],
                    wrapped(&out),
                ));
            }
        }
        Ok(())
    });
}

/// All citations of a document annotated in one call, as the CLI does.
#[test]
fn markup_annotates_every_citation_in_one_pass() {
    let steps = [CleanStep::Html, CleanStep::InlineWhitespace];
    let markup = format!(
        "<html><body>\n{}<p>“Bär” relied on <i>Bär v. Peña</i>, 410 U.S. 113, 115 (1973) 🎉, \
         and 東京 cases; see 42 U.S.C. § 1983; <em>Müller v. Øster</em>, 547 So.2d 1034 \
         (Fla. 1989) (Peña’s view).</p>\n{}</body></html>",
        padding(0, 60, true),
        padding(60, 60, true),
    );
    let mut document = Document::new("", &markup, &steps);
    let citations = get_citations_from_document(&mut document, false);
    assert_eq!(citations.len(), 3);
    let plain = &document.plain_text;
    let mut annotations: Vec<_> = citations
        .iter()
        .map(|c| (c.span(), OPEN.to_string(), CLOSE.to_string()))
        .collect();
    let out = annotate_citations(
        plain,
        &mut annotations,
        &markup,
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert_eq!(unmark(&out), markup);
    assert_eq!(
        wrapped(&out),
        ["410 U.S. 113", "42 U.S.C. § 1983", "547 So.2d 1034"]
    );
}

/// Where eyecite 2.7.8 puts the wrappers (all but one case, noted below) around each citation's
/// `full_span` (`annotate_citations(plain_text, [(span, "[", "]")],
/// markup)` after `get_citations(markup_text=..., clean_steps=["html",
/// "inline_whitespace"])`). Whitespace that cleaning collapses next to a
/// citation lands inside or outside the wrapper depending on the diff, and
/// eyecite's diff has no cleanup pass.
#[test]
fn markup_whitespace_matches_eyecite() {
    let steps = [CleanStep::Html, CleanStep::InlineWhitespace];
    let cases = [
        (
            "  \n<b>” 42 U.S.C. § 1983  <i> v.  v. a🎉",
            "  \n<b>” [42 U.S.C. § 1983 ] <i> v.  v. a🎉",
        ),
        (
            "<p></b>”<b></i>,  42 U.S.C. § 1983  <b> \n“, ",
            "<p></b>”<b></i>,  [42 U.S.C. § 1983  <b> ]\n“, ",
        ),
        // citare's html step runs the closing </i> into the quote after it, where
        // eyecite adds a space (docs/divergences.md#html-inline-join); the case name
        // then starts after the opening quotes, and so does the wrapper. eyecite gives
        // " [“<i>“aPeña</p> Bär v. Peña, 410 U.S. 113 (1973)]</i>“, “  a".
        (
            " “<i>“aPeña</p> Bär v. Peña, 410 U.S. 113 (1973)</i>“, “  a",
            " “<i>“[aPeña</p> Bär v. Peña, 410 U.S. 113 (1973)]</i>“, “  a",
        ),
        (
            "<i>”</p>, Bär 42 U.S.C. § 1983  \n<i>",
            "<i>”</p>, Bär [42 U.S.C. § 1983  ]\n<i>",
        ),
        (
            "“\n   Bär v. Peña, 410 U.S. 113 (1973)<p>Peña",
            "“\n[   Bär v. Peña, 410 U.S. 113 (1973)]<p>Peña",
        ),
        ("\n 42 U.S.C. § 1983  \n", "\n [42 U.S.C. § 1983  ]\n"),
        (
            "</b>🎉🎉<i>,  42 U.S.C. § 1983  “ v. ",
            "</b>🎉🎉<i>,  [42 U.S.C. § 1983  ]“ v. ",
        ),
    ];
    for (markup, expected) in cases {
        let mut document = Document::new("", markup, &steps);
        let citations = get_citations_from_document(&mut document, false);
        assert_eq!(citations.len(), 1, "{markup:?}");
        let mut annotations = vec![(citations[0].full_span(), "[".to_string(), "]".to_string())];
        let out = annotate_citations(
            &document.plain_text,
            &mut annotations,
            markup,
            UnbalancedTags::Unchecked,
            None,
            None,
        );
        assert_eq!(out, expected, "{markup:?}");
    }
}
