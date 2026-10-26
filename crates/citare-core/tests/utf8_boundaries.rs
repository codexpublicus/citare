//! Multibyte characters next to citations must never panic or produce spans
//! that split a character.
//!
//! Spans are UTF-8 byte offsets, but eyecite computes them in code points,
//! and several of its offsets are "length of these words, plus one" or "the
//! first N characters". Converted to bytes naively, each of those lands inside a
//! multibyte character as soon as one sits at the wrong place. These tests
//! put one at every place.

use citare_core::{
    annotate_citations, clean_text, get_citations, resolve_citations, Citation, CitationSpans,
    CleanStep, UnbalancedTags,
};

/// Characters seen next to citations in real opinions, of every UTF-8 width.
const MULTIBYTE: &[char] = &['é', '§', '\u{a0}', '’', '—', '🎉'];

/// Plain-text documents covering every citation type and the metadata
/// scanned before and after a citation.
const PLAIN: &[&str] = &[
    "Foo v. Bar, 547 So.2d 1034, 1036 (Fla. 1989) (holding that x). Id. at 1037. \
     Foo, 547 So.2d at 1038. Foo, supra, at 5. Bar at 1039.",
    "See 42 U.S.C. § 1983 (2000); 1 Harv. L. Rev. 100, 105 (1990). In re Smith, 410 U.S. 113 (1973).",
    "if not for counsel's errors. People v. Chapple , 291 Ill. App. 3d 574, 585 (1997), \
     quoting Strickland , 466 U.S. at 695.",
    "instituted. Story's Confl. of Laws, §§ 558, 576.—13 Pet. 312.—9 Plow. 407.",
    "Lissner, 1 U.S. 1, at 2. Lissner at 3.",
];

/// Markup documents, so the emphasis-tag code paths run too.
const MARKUP: &[&str] = &[
    "<p><i>Foo</i> v. <i>Bar</i>, 547 So.2d 1034, 1036 (Fla. 1989). <i>Id.</i> at 1037. \
     <i>Foo</i>, 547 So.2d at 1038. <i>Foo</i> at 1039.</p>",
    "<p>See <i>In re Smith</i>, 410 U.S. 113 (1973); <em>Smith</em>, supra, at 5.</p>",
];

fn assert_boundaries(text: &str, citations: &[Citation], context: &str) {
    for c in citations {
        let spans = CitationSpans::from_citation(c);
        let offsets = [
            Some(spans.span.0),
            Some(spans.span.1),
            Some(spans.full_span.0),
            Some(spans.full_span.1),
            spans.pin_cite_span_start,
            spans.pin_cite_span_end,
        ];
        for offset in offsets.into_iter().flatten() {
            assert!(
                offset <= text.len() && text.is_char_boundary(offset),
                "{} offset {offset} is not a char boundary in {text:?} ({context})",
                c.type_name(),
            );
        }
    }
}

fn markers(citations: &[Citation]) -> Vec<((usize, usize), String, String)> {
    citations
        .iter()
        .flat_map(|c| [c.span(), c.full_span()])
        .map(|span| (span, "<a>".to_string(), "</a>".to_string()))
        .collect()
}

/// Run find, resolve and annotate over plain text.
fn exercise_plain(text: &str, context: &str) {
    let citations = get_citations(text, false, "", &[]);
    assert_boundaries(text, &citations, context);
    resolve_citations(&citations);
    annotate_citations(
        text,
        &mut markers(&citations),
        "",
        UnbalancedTags::Unchecked,
        None,
        None,
    );
}

/// Run find, resolve and annotate (in every unbalanced-tag mode) over
/// markup.
fn exercise_markup(markup: &str, context: &str) {
    let steps = [CleanStep::Html, CleanStep::InlineWhitespace];
    let citations = get_citations("", false, markup, &steps);
    let plain = clean_text(markup, &steps).unwrap();
    assert_boundaries(&plain, &citations, context);
    resolve_citations(&citations);
    for mode in [
        UnbalancedTags::Unchecked,
        UnbalancedTags::Skip,
        UnbalancedTags::Wrap,
    ] {
        annotate_citations(&plain, &mut markers(&citations), markup, mode, None, None);
    }
}

/// `text` with `ch` inserted at every character position, and with each
/// space in turn replaced by `ch`.
fn variants(text: &str, ch: char) -> Vec<String> {
    let mut out = Vec::new();
    for (i, c) in text.char_indices().chain([(text.len(), ' ')]) {
        out.push(format!("{}{ch}{}", &text[..i], &text[i..]));
        if c == ' ' && i < text.len() {
            out.push(format!("{}{ch}{}", &text[..i], &text[i + 1..]));
        }
    }
    out
}

/// Run `f` on every input, and fail with the inputs that panicked.
fn run_all(inputs: impl IntoIterator<Item = String>, f: impl Fn(&str, &str)) {
    let mut failures = Vec::new();
    let mut total = 0;
    for input in inputs {
        total += 1;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&input, "")));
        if let Err(e) = result {
            let msg = e
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            failures.push(format!("{input:?}: {msg}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {total} inputs failed; first few:\n{}",
        failures.len(),
        failures[..failures.len().min(8)].join("\n"),
    );
}

#[test]
fn multibyte_anywhere_in_plain_text() {
    let inputs = PLAIN
        .iter()
        .flat_map(|text| MULTIBYTE.iter().flat_map(|&ch| variants(text, ch)));
    run_all(inputs, exercise_plain);
}

/// Inserting inside a tag name makes a different (still parseable)
/// document; it must not panic either.
#[test]
fn multibyte_anywhere_in_markup() {
    let inputs = MARKUP
        .iter()
        .flat_map(|text| MULTIBYTE.iter().flat_map(|&ch| variants(text, ch)));
    run_all(inputs, exercise_markup);
}

/// The text scanned after a citation is capped at 300 characters. A
/// multibyte character straddling byte 300 used to panic in
/// `String::truncate` (CourtListener opinions, 15 of 779 in the 1% sample).
#[test]
fn forward_scan_cap_inside_multibyte_char() {
    for n in 280..=310 {
        for &ch in MULTIBYTE {
            let text = format!("Foo v. Bar, 547 So.2d 1034 {}{ch}s house.", "x".repeat(n));
            exercise_plain(&text, &format!("{n} {ch:?}"));
        }
    }
}

/// The reported repro, verbatim.
#[test]
fn forward_scan_cap_reported_repro() {
    for n in [297, 298] {
        let text = format!("Foo v. Bar, 547 So.2d 1034 {}’s house.", "x".repeat(n));
        let citations = get_citations(&text, false, "", &[]);
        assert_eq!(citations.len(), 1);
        assert_eq!(citations[0].span(), (12, 26));
    }
}

/// The backward scan (pre-citation pin cites, supra antecedents) has the
/// same cap from the other end.
#[test]
fn backward_scan_cap_inside_multibyte_char() {
    for n in 280..=310 {
        for &ch in MULTIBYTE {
            let text = format!("{ch}{} 547 So.2d 1034. Foo, supra, at 2.", "x".repeat(n));
            exercise_plain(&text, &format!("{n} {ch:?}"));
            let text = format!("{ch}{}, supra, at 2.", "x".repeat(n));
            exercise_plain(&text, &format!("{n} {ch:?}"));
        }
    }
}

/// The cap counts characters, as eyecite's `len(text)` does, not bytes: a
/// court and year 290 characters (but more than 300 bytes) after the
/// citation are still found.
#[test]
fn forward_scan_cap_counts_characters() {
    let text = format!("Foo v. Bar, 1 U.S. 1 {}(1999)", "é ".repeat(140));
    let citations = get_citations(&text, false, "", &[]);
    assert_eq!(citations.len(), 1);
    let Citation::FullCase(data) = &citations[0] else {
        panic!("expected a full case citation");
    };
    assert_eq!(data.metadata.case.year.as_deref(), Some("1999"));
}

/// CourtListener 3137924: a no-break space between the case name and the
/// citation put `full_span` one byte into it. eyecite 2.7.8 starts the full
/// span at the no-break space before "People".
#[test]
fn case_name_before_no_break_space() {
    let text = "if not for counsel's errors. \u{a0} People v. Chapple , \u{a0}291 Ill. App. 3d \
                574, 585 (1997), quoting Strickland , 466 U.S. at 695.";
    let citations = get_citations(text, false, "", &[]);
    assert_boundaries(text, &citations, "3137924");
    let (start, end) = citations[0].full_span();
    assert_eq!(
        &text[start..end],
        "\u{a0} People v. Chapple , \u{a0}291 Ill. App. 3d 574, 585 (1997)"
    );
}

/// CourtListener 7034132: an em dash glued to the citation put `full_span`
/// inside it.
#[test]
fn case_name_before_em_dash() {
    let text = "instituted.” Story’s Confl. of Laws, §§ 558, 576.—13 Pet. 312.—9 Plow. 407.";
    let citations = get_citations(text, false, "", &[]);
    assert_boundaries(text, &citations, "7034132");
    // eyecite 2.7.8 starts it inside "Story’s", one character after the
    // last word it counted.
    let case = citations
        .iter()
        .find(|c| c.type_name() == "FullCaseCitation")
        .unwrap();
    let (start, end) = case.full_span();
    assert_eq!(
        &text[start..end],
        "y’s Confl. of Laws, §§ 558, 576.—13 Pet. 312"
    );
}

/// Annotating markup maps plain-text byte offsets into the markup; the map
/// must count bytes too, or every multibyte character before a citation
/// shifts the annotation.
#[test]
fn annotate_markup_after_multibyte_text() {
    let markup = "<p>Café “quoted” 🎉 ““““““ <i>Foo</i> v. Bar, 1 U.S. 1 (1999). <b>x</b> y</p>";
    let steps = [CleanStep::Html];
    let citations = get_citations("", false, markup, &steps);
    let plain = clean_text(markup, &steps).unwrap();
    let mut markers: Vec<_> = citations
        .iter()
        .map(|c| (c.span(), "[".to_string(), "]".to_string()))
        .collect();
    let out = annotate_citations(
        &plain,
        &mut markers,
        markup,
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert!(out.contains("[1 U.S. 1]"), "{out}");
}

/// Stripping a trailing no-break space from an emphasized plaintiff shifts
/// the full span start by one character (eyecite 2.7.8), not two bytes.
#[test]
fn markup_plaintiff_shift_counts_characters() {
    let markup = "<p><i>€xyz\u{a0}</i> v. <i>Bar</i>, 1 U.S. 1 (1999)</p>";
    let steps = [CleanStep::Html, CleanStep::InlineWhitespace];
    let citations = get_citations("", false, markup, &steps);
    let plain = clean_text(markup, &steps).unwrap();
    assert_boundaries(&plain, &citations, "plaintiff shift");
    let (start, end) = citations[0].full_span();
    assert_eq!(&plain[start..end], "xyz\u{a0} v. Bar, 1 U.S. 1 (1999)");
}

/// A parenthesized word opening with a lowercase non-ASCII letter ends the
/// case name after it, as eyecite's `word[1].islower()` does.
#[test]
fn lowercase_non_ascii_after_paren() {
    let text = "the court in Foo Bar Baz Qux (étude) v. Smith, 1 U.S. 1 (1999)";
    let citations = get_citations(text, false, "", &[]);
    let (start, end) = citations[0].full_span();
    assert_eq!(&text[start..end], "v. Smith, 1 U.S. 1 (1999)");
}

/// `annotate_citations` takes caller-supplied byte offsets (the WASM
/// binding passes them straight through from JavaScript). Offsets inside a
/// character, past the end or reversed are skipped rather than panicking.
#[test]
fn annotate_skips_offsets_that_split_characters() {
    let text = "é 1 U.S. 1 🎉";
    let mut markers: Vec<_> = [(1, 5), (3, 11), (12, 13), (5, 3), (3, 100), (3, 11)]
        .into_iter()
        .map(|span| (span, "[".to_string(), "]".to_string()))
        .collect();
    let out = annotate_citations(
        text,
        &mut markers,
        "",
        UnbalancedTags::Unchecked,
        None,
        None,
    );
    assert_eq!(out, "é [1 U.S. 1] 🎉");
}
