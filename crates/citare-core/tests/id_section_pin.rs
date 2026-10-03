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
