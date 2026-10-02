//! Span conversion on real citations in non-ASCII text.

use citare_core::{get_citations, Citation, CitationSpans, OffsetMap, OffsetUnit};

/// Slice `text` by code point offsets, as Python and eyecite do.
fn slice_chars(text: &str, (start, end): (usize, usize)) -> String {
    text.chars().skip(start).take(end - start).collect()
}

/// Slice `text` by UTF-16 offsets, as JavaScript's `String.prototype.slice`
/// does.
fn slice_utf16(text: &str, (start, end): (usize, usize)) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    String::from_utf16(&units[start..end]).expect("span splits a surrogate pair")
}

const TEXT: &str = "“Smart” quotes 🎉 — see Café v. Bär, 410 U.S. 113, 115 (1973) \
                    (résumé 🎉). Id. at 116. See also 42 U.S.C. § 1983.";

fn find<'a>(citations: &'a [Citation], type_name: &str) -> &'a Citation {
    citations
        .iter()
        .find(|c| c.type_name() == type_name)
        .unwrap_or_else(|| panic!("no {type_name}"))
}

#[test]
fn every_span_slices_the_same_text_in_every_unit() {
    let citations = get_citations(TEXT, false, "", &[]);
    assert!(citations.len() >= 3, "expected case, id and law citations");
    let map = OffsetMap::new(TEXT);

    for c in &citations {
        let bytes = CitationSpans::from_citation(c);
        let chars = map.citation_spans(c, OffsetUnit::Chars).unwrap();
        let utf16 = map.citation_spans(c, OffsetUnit::Utf16).unwrap();

        for (b, ch, u) in [
            (bytes.span, chars.span, utf16.span),
            (bytes.full_span, chars.full_span, utf16.full_span),
        ] {
            let expected = &TEXT[b.0..b.1];
            assert_eq!(slice_chars(TEXT, ch), expected, "{}", c.type_name());
            assert_eq!(slice_utf16(TEXT, u), expected, "{}", c.type_name());
        }
        for (b, ch, u) in [
            (
                bytes.pin_cite_span_start,
                chars.pin_cite_span_start,
                utf16.pin_cite_span_start,
            ),
            (
                bytes.pin_cite_span_end,
                chars.pin_cite_span_end,
                utf16.pin_cite_span_end,
            ),
        ] {
            assert_eq!(b.is_some(), ch.is_some());
            assert_eq!(b.is_some(), u.is_some());
            if let (Some(b), Some(ch), Some(u)) = (b, ch, u) {
                assert_eq!(ch, TEXT[..b].chars().count());
                assert_eq!(u, TEXT[..b].encode_utf16().count());
            }
        }
    }
}

#[test]
fn case_citation_offsets_shift_by_unit() {
    let citations = get_citations(TEXT, false, "", &[]);
    let case = find(&citations, "FullCaseCitation");
    let (start, end) = case.span();
    assert_eq!(&TEXT[start..end], "410 U.S. 113");

    let map = OffsetMap::new(TEXT);
    // The quotes, dash, é and ä are each one code point and one UTF-16
    // unit; the emoji before the citation is one code point but a
    // surrogate pair, so all three units differ.
    let chars = map.to_chars(start).unwrap();
    assert_ne!(chars, start);
    assert_eq!(chars, TEXT[..start].chars().count());
    assert_eq!(map.to_utf16(start).unwrap(), chars + 1);
    assert_eq!(map.to_chars(end).unwrap() - chars, 12);

    // The pin cite ", 115" follows the citation; citare records its end.
    let pin_end = case.pin_cite_span().1.expect("pin cite end");
    assert_eq!(&TEXT[end..pin_end], ", 115");
    let utf16 = map.citation_spans(case, OffsetUnit::Utf16).unwrap();
    let pin = (utf16.span.1, utf16.pin_cite_span_end.unwrap());
    assert_eq!(slice_utf16(TEXT, pin), ", 115");

    // full_span starts at the case name, which itself contains é and ä.
    let (fs, fe) = case.full_span();
    assert!(TEXT[fs..fe].starts_with("Café v. Bär"));
    let chars = map.citation_spans(case, OffsetUnit::Chars).unwrap();
    assert_eq!(slice_chars(TEXT, chars.full_span), &TEXT[fs..fe]);
}

#[test]
fn section_sign_inside_a_citation() {
    let citations = get_citations(TEXT, false, "", &[]);
    let law = find(&citations, "FullLawCitation");
    let map = OffsetMap::new(TEXT);
    let (s, e) = map.citation_spans(law, OffsetUnit::Utf16).unwrap().span;
    assert_eq!(slice_utf16(TEXT, (s, e)), "42 U.S.C. § 1983");
    // § is two bytes but one UTF-16 unit, so the span shrinks by one.
    let (bs, be) = law.span();
    assert_eq!(be - bs, e - s + 1);
}

#[test]
fn bytes_unit_is_identity() {
    let citations = get_citations(TEXT, false, "", &[]);
    let map = OffsetMap::new(TEXT);
    for c in &citations {
        assert_eq!(
            map.citation_spans(c, OffsetUnit::Bytes).unwrap(),
            CitationSpans::from_citation(c)
        );
    }
}
