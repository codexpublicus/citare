//! The section number of a full statute citation (#48). reporters-db's
//! `law_section` (`\d+(?:[\-.:]\d+){,3}`) has no letter suffix and only an
//! ASCII hyphen, so eyecite 2.7.8 cuts `Mich. Comp. Laws § 600.2912a` to
//! `§ 600` and `Utah Code Ann. § 76‐9‐101` (U+2010) to `§ 76`. citare
//! overrides the section grammar in `build.rs`.

use citare_core::{get_citations, Citation};
use rstest::rstest;

/// The one full law citation in `text`: its matched text and section.
fn law(text: &str) -> (String, String) {
    let citations = get_citations(text, false, "", &[]);
    let laws: Vec<_> = citations
        .iter()
        .filter(|c| matches!(c, Citation::FullLaw(_)))
        .collect();
    assert_eq!(laws.len(), 1, "expected one law citation in {text:?}");
    let c = laws[0];
    let (start, end) = c.span();
    let section = c
        .common()
        .groups
        .get("section")
        .cloned()
        .flatten()
        .expect("section group");
    (text[start..end].to_string(), section)
}

#[rstest]
// A letter after a dotted section number.
#[case("See Mich. Comp. Laws § 600.2912a.", "600.2912a")]
#[case("See Mich. Comp. Laws § 600.2912a(1).", "600.2912a")]
#[case("See Mich. Comp. Laws § 600.2912b and more.", "600.2912b")]
// A letter after the section number, then more parts. eyecite finds no
// law citation in `42 U.S.C. § 2000e-5`.
#[case("See 42 U.S.C. § 2000e-5(f).", "2000e-5")]
#[case("See 17 C.F.R. § 240.10b-5.", "240.10b-5")]
#[case("See Mich. Comp. Laws § 750.520b.", "750.520b")]
// Unicode hyphens.
#[case(
    "See Utah Code Ann. § 76\u{2010}9\u{2010}101.",
    "76\u{2010}9\u{2010}101"
)]
#[case(
    "See Utah Code Ann. § 76\u{2011}5\u{2011}103 (2008).",
    "76\u{2011}5\u{2011}103"
)]
// A letter part after a hyphen (New York).
#[case("N.Y. Gen. Bus. Law § 394-ccc(1).", "394-ccc")]
// Unchanged.
#[case("See Utah Code Ann. § 76-9-101.", "76-9-101")]
#[case("See Cal. Penal Code § 1170.12.", "1170.12")]
#[case("See Tex. Fam. Code Ann. § 161.001(b)(1)(D).", "161.001")]
#[case("See Mich. Comp. Laws § 600.2912. The court", "600.2912")]
fn law_section(#[case] text: &str, #[case] section: &str) {
    let (matched, got) = law(text);
    assert_eq!(got, section, "section in {text:?} (matched {matched:?})");
}
