//! `Id.`/`Ibid.` pin cites that point at a section (`§`) or paragraph (`¶`).
//!
//! eyecite 2.7.8 captures `Id. ¶ 4` but not `Id. § 5`: its tokenizer turns
//! the `§` into a section token that the pin cite scan stops at, and the `§`
//! then comes out as an unknown citation. citare goes further for `§` (#2):
//! the section becomes the id. citation's pin cite and no unknown citation
//! is emitted for it. The `¶` and `at <page>` cases match eyecite.

use citare_core::{get_citations, resolve_citations, Citation};
use rstest::rstest;

/// The resolution group of each citation, by position, or `None` when it is
/// unresolved. Groups are numbered in order of first appearance.
fn resolution_groups(citations: &[Citation]) -> Vec<Option<usize>> {
    let resolutions = resolve_citations(citations);
    let key = |c: &Citation| (c.type_name(), c.span());
    let mut seen: Vec<usize> = Vec::new();
    citations
        .iter()
        .map(|c| {
            let group = resolutions
                .values()
                .position(|members| members.iter().any(|m| key(m) == key(c)))?;
            Some(match seen.iter().position(|&g| g == group) {
                Some(label) => label,
                None => {
                    seen.push(group);
                    seen.len() - 1
                }
            })
        })
        .collect()
}

/// Find citations in `text` and return them with the id. citation's index.
fn find_id(text: &str) -> (Vec<Citation>, usize) {
    let citations = get_citations(text, false, "", &[]);
    let ids: Vec<usize> = citations
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Citation::Id(_)))
        .map(|(i, _)| i)
        .collect();
    assert_eq!(ids.len(), 1, "expected one id. citation in {text:?}");
    (citations, ids[0])
}

/// Assert the id. citation's span covers exactly `id_text`, its pin cite is
/// `pin`, its parenthetical is `paren`, and that it resolves to the citation
/// before it exactly when `resolves` is set.
fn check(text: &str, id_text: &str, pin: &str, paren: Option<&str>, resolves: bool) {
    let (citations, i) = find_id(text);
    let id = &citations[i];
    let start = text.find(id_text).expect("id_text not in text");
    assert_eq!(
        id.span(),
        (start, start + id_text.len()),
        "span in {text:?}"
    );
    assert_eq!(id.full_span(), id.span(), "full_span in {text:?}");
    let Citation::Id(data) = id else {
        unreachable!()
    };
    assert_eq!(
        data.metadata.pin_cite.as_deref(),
        Some(pin),
        "pin in {text:?}"
    );
    assert_eq!(
        data.metadata.parenthetical.as_deref(),
        paren,
        "paren in {text:?}"
    );

    // The section the pin cite took is not also reported on its own.
    assert!(
        !citations.iter().any(|c| matches!(c, Citation::Unknown(_))),
        "unexpected unknown citation in {text:?}: {:?}",
        citations.iter().map(|c| c.type_name()).collect::<Vec<_>>()
    );
    assert_eq!(citations.len(), 2, "citations in {text:?}");

    let groups = resolution_groups(&citations);
    assert_eq!(groups[0], Some(0), "antecedent resolution in {text:?}");
    let expected = if resolves { Some(0) } else { None };
    assert_eq!(groups[i], expected, "id. resolution in {text:?}");
}

/// Section pin cites after a statute: the id. citation resolves to it.
#[rstest]
// The example from #2.
#[case(
    "N.Y. Gen. Bus. Law § 394-ccc(1). Id. § 394-ccc(2)-(3).",
    "Id. § 394-ccc(2)-(3)",
    "§ 394-ccc(2)-(3)"
)]
#[case("See 42 U.S.C. § 1983. Id. § 1988.", "Id. § 1988", "§ 1988")]
#[case("See 42 U.S.C. § 1983. Id. §1988.", "Id. §1988", "§1988")]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 1983-1985.",
    "Id. §§ 1983-1985",
    "§§ 1983-1985"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. at § 1983(a).",
    "Id. at § 1983(a)",
    "at § 1983(a)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. at §§ 1983-1985.",
    "Id. at §§ 1983-1985",
    "at §§ 1983-1985"
)]
#[case(
    "See 42 U.S.C. § 1983. Ibid. § 1983(c).",
    "Ibid. § 1983(c)",
    "§ 1983(c)"
)]
#[case("See 42 U.S.C. § 1983. Id., § 1988.", "Id., § 1988", "§ 1988")]
#[case(
    "See 42 U.S.C. § 1983. Id. § 1983 et seq.",
    "Id. § 1983 et seq.",
    "§ 1983 et seq."
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 1983(a), (b).",
    "Id. § 1983(a), (b)",
    "§ 1983(a), (b)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 1983 provides a remedy.",
    "Id. § 1983",
    "§ 1983"
)]
fn section_pin_after_statute(#[case] text: &str, #[case] id_text: &str, #[case] pin: &str) {
    check(text, id_text, pin, None, true);
}

/// Statutes at Large and session laws have a page, but a section pin cite
/// into them is still valid: the page range check is for case reporters.
/// Corpus examples: `Pub.L.No.91-184, § 3(2), 83 Stat. 841 ... id. § 4(b)`
/// and `1980 Haw. Sess. Laws 366 ... Id. § 1, at 367`.
#[rstest]
#[case("See 83 Stat. 841. Id. § 4(b).", "Id. § 4(b)", "§ 4(b)")]
#[case("See 83 Stat. 841. Id. at § 3.", "Id. at § 3", "at § 3")]
#[case("1980 Haw. Sess. Laws 366. Id. § 1.", "Id. § 1", "§ 1")]
fn section_pin_after_paged_law(#[case] text: &str, #[case] id_text: &str, #[case] pin: &str) {
    check(text, id_text, pin, None, true);
}

/// A paragraph pin cite after a paged law citation keeps eyecite's rule: it
/// doesn't start with a page number, so the id. citation is unresolved.
#[test]
fn paragraph_pin_after_paged_law_is_unresolved() {
    check("See 83 Stat. 841. Id. ¶ 4.", "Id. ¶ 4", "¶ 4", None, false);
}

/// Each id. citation resolves to the one before it, so a section pin cite
/// that resolves keeps the chain going. In eyecite the `§` becomes an
/// unknown citation between the two, which resets the chain: the second
/// `Id.` there is unresolved.
#[test]
fn id_chain_continues_past_a_section_pin() {
    let text = "See 42 U.S.C. § 1983. Id. § 1988. Id. at § 1983(b).";
    let citations = get_citations(text, false, "", &[]);
    assert_eq!(citations.len(), 3, "citations in {text:?}");
    assert!(citations[1..].iter().all(|c| matches!(c, Citation::Id(_))));
    assert_eq!(
        resolution_groups(&citations),
        vec![Some(0), Some(0), Some(0)]
    );
}

#[test]
fn section_pin_with_parenthetical() {
    check(
        "See 42 U.S.C. § 1983. Id. § 1983 (explaining).",
        "Id. § 1983",
        "§ 1983",
        Some("explaining"),
        true,
    );
}

/// After a case, a pin cite that doesn't start with a page number can't be
/// checked against the case's pages, so eyecite's rule treats it as invalid
/// and leaves the id. citation unresolved. Sections follow the same rule.
#[rstest]
#[case("Foo v. Bar, 1 U.S. 1 (1999). Id. § 5.", "Id. § 5", "§ 5")]
#[case("Foo v. Bar, 1 U.S. 1 (1999). Id. §§ 5-7.", "Id. §§ 5-7", "§§ 5-7")]
#[case(
    "Foo v. Bar, 1 U.S. 1 (1999). Id. at § 12(b).",
    "Id. at § 12(b)",
    "at § 12(b)"
)]
#[case("Foo v. Bar, 1 U.S. 1 (1999). Ibid. § 3.", "Ibid. § 3", "§ 3")]
// eyecite parity: these already had their pin cite, and now also match
// eyecite's resolution (unresolved).
#[case("Foo v. Bar, 1 U.S. 1 (1999). Id. ¶ 4.", "Id. ¶ 4", "¶ 4")]
#[case("Foo v. Bar, 1 U.S. 1 (1999). Id. at ¶ 4.", "Id. at ¶ 4", "at ¶ 4")]
fn non_page_pin_after_case_is_unresolved(
    #[case] text: &str,
    #[case] id_text: &str,
    #[case] pin: &str,
) {
    check(text, id_text, pin, None, false);
}

#[rstest]
#[case("Foo v. Bar, 1 U.S. 1 (1999). Id. at 5.", "Id. at 5", "at 5")]
#[case("See 42 U.S.C. § 1983. Id. at 5.", "Id. at 5", "at 5")]
fn page_pin_resolves(#[case] text: &str, #[case] id_text: &str, #[case] pin: &str) {
    check(text, id_text, pin, None, true);
}

/// A `§` that doesn't directly follow the id. citation stays an unknown
/// citation, and the id. citation gets no pin cite.
#[test]
fn distant_section_is_not_a_pin() {
    let text = "See 42 U.S.C. § 1983. Id. The rule in § 5 applies.";
    let (citations, i) = find_id(text);
    let Citation::Id(data) = &citations[i] else {
        unreachable!()
    };
    assert_eq!(data.metadata.pin_cite, None);
    assert_eq!(citations[i].span().1 - citations[i].span().0, "Id.".len());
    assert!(matches!(citations.last(), Some(Citation::Unknown(_))));
}

/// A `§` with no section number after it isn't a pin cite either.
#[test]
fn bare_section_sign_is_not_a_pin() {
    let text = "See 42 U.S.C. § 1983. Id. § and more.";
    let (citations, i) = find_id(text);
    let Citation::Id(data) = &citations[i] else {
        unreachable!()
    };
    assert_eq!(data.metadata.pin_cite, None);
    assert!(matches!(citations.last(), Some(Citation::Unknown(_))));
}

/// Section pin cite forms from the #48 corpus study, where the pin cite
/// used to stop early or be missed. The section numbers and the text after
/// them come from the opinions; the antecedent is a stand-in statute.
#[rstest]
// Whitespace, including a line break or a non-breaking space, after the `§`.
#[case("See 42 U.S.C. § 1983. Id. §\n401.", "Id. §\n401", "§\n401")]
#[case(
    "See 42 U.S.C. § 1983. Id. §§\n\n157.167, 157.263.",
    "Id. §§\n\n157.167, 157.263",
    "§§\n\n157.167, 157.263"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §\u{a0}46.01.",
    "Id. §\u{a0}46.01",
    "§\u{a0}46.01"
)]
// Unicode hyphens (U+2010, U+2011) between the parts of a section number.
#[case(
    "See 42 U.S.C. § 1983. Id. § 76\u{2010}9\u{2010}101(1)(b).",
    "Id. § 76\u{2010}9\u{2010}101(1)(b)",
    "§ 76\u{2010}9\u{2010}101(1)(b)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 76\u{2011}5\u{2011}103.",
    "Id. § 76\u{2011}5\u{2011}103",
    "§ 76\u{2011}5\u{2011}103"
)]
// A letter after a dotted section number (Mich. Comp. Laws § 600.2912a).
#[case(
    "See 42 U.S.C. § 1983. Id. § 600.2912a.",
    "Id. § 600.2912a",
    "§ 600.2912a"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 404.510a; 404.512.",
    "Id. §§ 404.510a; 404.512",
    "§§ 404.510a; 404.512"
)]
// Subsections with a dot or a hyphen inside the parentheses.
#[case(
    "See 42 U.S.C. § 1983. Id. § 6105(a.1)(1).",
    "Id. § 6105(a.1)(1)",
    "§ 6105(a.1)(1)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 9799.14(c)(1.3), was required.",
    "Id. § 9799.14(c)(1.3)",
    "§ 9799.14(c)(1.3)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 16.012(d-1).",
    "Id. § 16.012(d-1)",
    "§ 16.012(d-1)"
)]
// California and New York subdivisions.
#[case(
    "See 42 U.S.C. § 1983. Id., § 5304, subd. (a).",
    "Id., § 5304, subd. (a)",
    "§ 5304, subd. (a)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 3, subd. 6, as amended.",
    "Id. § 3, subd. 6",
    "§ 3, subd. 6"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 1300.75.4.2, subd. (b)(1)(B), (2)(A).",
    "Id., § 1300.75.4.2, subd. (b)(1)(B), (2)(A)",
    "§ 1300.75.4.2, subd. (b)(1)(B), (2)(A)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., §§ 2962, subd. (d)(1), 2966, subd. (c), 2970; see generally.",
    "Id., §§ 2962, subd. (d)(1), 2966, subd. (c), 2970",
    "§§ 2962, subd. (d)(1), 2966, subd. (c), 2970"
)]
// Comments, application notes, illustrations and clauses.
#[case(
    "See 42 U.S.C. § 1983. Id. § 2B1.1, cmt. (n.2).",
    "Id. § 2B1.1, cmt. (n.2)",
    "§ 2B1.1, cmt. (n.2)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 2B1.1, cmt. (n.2(A)(i) and (ii)).",
    "Id. § 2B1.1, cmt. (n.2(A)(i) and (ii))",
    "§ 2B1.1, cmt. (n.2(A)(i) and (ii))"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 2L1.2 cmt. n. 1(B)(iii).",
    "Id. § 2L1.2 cmt. n. 1(B)(iii)",
    "§ 2L1.2 cmt. n. 1(B)(iii)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 2L1.2 application n. 5.",
    "Id. § 2L1.2 application n. 5",
    "§ 2L1.2 application n. 5"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 152 cmt. a.",
    "Id. § 152 cmt. a",
    "§ 152 cmt. a"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 381, comment a.",
    "Id. § 381, comment a",
    "§ 381, comment a"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 675, comment (d).",
    "Id. § 675, comment (d)",
    "§ 675, comment (d)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 458 Comment h.",
    "Id., § 458 Comment h",
    "§ 458 Comment h"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 199, illus. 1.",
    "Id., § 199, illus. 1",
    "§ 199, illus. 1"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 2, cl. 1.",
    "Id., § 2, cl. 1",
    "§ 2, cl. 1"
)]
// Bracketed subdivisions (treatises, New York codes).
#[case(
    "See 42 U.S.C. § 1983. Id. at § 41.02[4].",
    "Id. at § 41.02[4]",
    "at § 41.02[4]"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 261[1][a].",
    "Id., § 261[1][a]",
    "§ 261[1][a]"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 10-603 [d].",
    "Id. § 10-603 [d]",
    "§ 10-603 [d]"
)]
// `to` and `through` ranges.
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 216.103 to 216.111.",
    "Id. §§ 216.103 to 216.111",
    "§§ 216.103 to 216.111"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 58 through 61.",
    "Id. §§ 58 through 61",
    "§§ 58 through 61"
)]
// Lists that continue with a leading dot or hyphen, a semicolon or a `§`.
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 2007.023(b), .024(a).",
    "Id. §§ 2007.023(b), .024(a)",
    "§§ 2007.023(b), .024(a)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 8.01-.07.",
    "Id. §§ 8.01-.07",
    "§§ 8.01-.07"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 22-501, -3202, attempted robbery.",
    "Id. §§ 22-501, -3202",
    "§§ 22-501, -3202"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 23-602(A)(8); 23-603(A)(4).",
    "Id. §§ 23-602(A)(8); 23-603(A)(4)",
    "§§ 23-602(A)(8); 23-603(A)(4)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. at § 780.4, § 780.5.",
    "Id. at § 780.4, § 780.5",
    "at § 780.4, § 780.5"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., § 457 and § 459.",
    "Id., § 457 and § 459",
    "§ 457 and § 459"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 3553(a)(1), (3)-(7).",
    "Id. § 3553(a)(1), (3)-(7)",
    "§ 3553(a)(1), (3)-(7)"
)]
// Subsections after a space.
#[case(
    "See 42 U.S.C. § 1983. Id. § 1677(7)(C) (iii).",
    "Id. § 1677(7)(C) (iii)",
    "§ 1677(7)(C) (iii)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 780-113 (b)-(o).",
    "Id. § 780-113 (b)-(o)",
    "§ 780-113 (b)-(o)"
)]
// Roman numeral sections.
#[case(
    "See 42 U.S.C. § 1983. Id. at § IV(B)(6)(j).",
    "Id. at § IV(B)(6)(j)",
    "at § IV(B)(6)(j)"
)]
// A line break before the `§` after a lowercase `id.` or after `at`.
#[case(
    "See 42 U.S.C. § 1983; see id.\n§ 7407(d)(1).",
    "id.\n§ 7407(d)(1)",
    "§ 7407(d)(1)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. at\n\n§ 2149(c).",
    "Id. at\n\n§ 2149(c)",
    "at\n\n§ 2149(c)"
)]
fn section_pin_forms(#[case] text: &str, #[case] id_text: &str, #[case] pin: &str) {
    check(text, id_text, pin, None, true);
}

/// The id. citation's pin cite in `text`, which must hold one id. citation.
fn id_pin(text: &str) -> Option<String> {
    let (citations, i) = find_id(text);
    let Citation::Id(data) = &citations[i] else {
        unreachable!()
    };
    data.metadata.pin_cite.clone()
}

/// Where the section pin cite ends: the grammar above doesn't run on into
/// the next citation or into prose.
#[rstest]
#[case("See 42 U.S.C. § 1983. Id. § 5; 28 U.S.C. § 1331.", "§ 5")]
#[case("See 42 U.S.C. § 1983. Id. §§ 5; 1990 Wis. Laws 12.", "§§ 5")]
#[case("See 42 U.S.C. § 1983. Id. § 5 — the court held.", "§ 5")]
#[case("See 42 U.S.C. § 1983. Id. § 5 to the extent it applies.", "§ 5")]
#[case("See 42 U.S.C. § 1983. Id. § 5, comment by the drafters.", "§ 5")]
#[case("See 42 U.S.C. § 1983. Id. § 5 (West 1990).", "§ 5")]
#[case("See 42 U.S.C. § 1983. Id. § 5 (a) applies.", "§ 5 (a)")]
#[case(
    "See 42 U.S.C. § 1983. Id. § 157, comment (a), p. 373.",
    "§ 157, comment (a)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 28, Comment j, at 283.",
    "§ 28, Comment j"
)]
fn section_pin_stops(#[case] text: &str, #[case] pin: &str) {
    assert_eq!(id_pin(text).as_deref(), Some(pin), "pin in {text:?}");
}

/// A capitalized `Id.` that ends a line, with the `§` starting the next
/// one, is the end of a sentence in CAP's digests (`Id.\n§ 12 makes all
/// the trustees liable ...`), so it gets no pin cite and the `§` stays an
/// unknown citation.
#[rstest]
#[case("See 42 U.S.C. § 1983. Id.\n§ 12 makes all the trustees liable.")]
#[case("See 42 U.S.C. § 1983. Id.\n\n§ 6 of the act provides as follows.")]
#[case("See 42 U.S.C. § 1983. Ibid.\n§ 2 of the act of 1853 does not apply.")]
fn capitalized_id_before_a_line_break_is_not_a_pin(#[case] text: &str) {
    assert_eq!(id_pin(text), None, "pin in {text:?}");
    let citations = get_citations(text, false, "", &[]);
    assert!(matches!(citations.last(), Some(Citation::Unknown(_))));
}

/// More forms from the #48 study: OCR's spaced dashes, a CFR section's
/// hyphenated suffix after a subsection, dashes inside a subsection, and a
/// list that wraps onto the next line.
#[rstest]
#[case("See 42 U.S.C. § 1983. Id. § 14 — 501(b)(5).", "§ 14 — 501(b)(5)")]
#[case(
    "See 42 U.S.C. § 1983. Id. §§ 11, 11-a, 22\u{2014}23-c; see also.",
    "§§ 11, 11-a, 22\u{2014}23-c"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 3222.1(b)(10)(i\u{2014}iii).",
    "§ 3222.1(b)(10)(i\u{2014}iii)"
)]
#[case("See 42 U.S.C. § 1983. Id. § 1.411(d)-4, Q & A-2(b).", "§ 1.411(d)-4")]
#[case(
    "See 42 U.S.C. § 1983. Id. § 924(e) (2) (B) (i) \u{2014} (ii).",
    "§ 924(e) (2) (B) (i) \u{2014} (ii)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id. § 4513(a)(1)(A),\n(B)(i), (B)(v).",
    "§ 4513(a)(1)(A),\n(B)(i), (B)(v)"
)]
#[case(
    "See 42 U.S.C. § 1983. Id., §501 (b), 17 P.S. §211.501 (b).",
    "§501 (b)"
)]
fn more_section_pin_forms(#[case] text: &str, #[case] pin: &str) {
    assert_eq!(id_pin(text).as_deref(), Some(pin), "pin in {text:?}");
}
