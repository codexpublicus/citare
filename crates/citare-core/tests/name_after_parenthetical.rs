//! A case name after an earlier citation's parenthetical and a comma (#27,
//! item 4): "People v. Taylor, 219 Ill. App. 3d 47 (1991), and People v.
//! Ceasar, 231 Ill. App. 3d 54 (1992)" (CourtListener opinion 4665783 in the
//! one-percent sample). The backward scan for Ceasar's plaintiff stopped at
//! the "(" of "(1991)," and took "1991), and People"; eyecite 2.7.8 gives
//! Ceasar's citation the plaintiff "" and the defendant "Taylor".

use citare_core::{get_citations, Citation};

/// Each full case citation's (matched text, plaintiff, defendant).
fn parties(text: &str) -> Vec<(String, Option<String>, Option<String>)> {
    get_citations(text, false, "", &[])
        .iter()
        .filter_map(|c| match c {
            Citation::FullCase(d) => Some((
                c.matched_text().to_string(),
                d.metadata.plaintiff.clone(),
                d.metadata.defendant.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn named(cite: &str, plaintiff: &str, defendant: &str) -> (String, Option<String>, Option<String>) {
    (
        cite.to_string(),
        Some(plaintiff.to_string()),
        Some(defendant.to_string()),
    )
}

#[test]
fn name_starts_after_the_parenthetical_before_it() {
    let taylor = named("219 Ill. App. 3d 47", "People", "Taylor");
    let ceasar = named("231 Ill. App. 3d 54", "People", "Ceasar");
    assert_eq!(
        parties(
            "People v. Taylor, 219 Ill. App. 3d 47 (1991), and People v. Ceasar, 231 Ill. App. \
             3d 54 (1992)."
        ),
        [taylor.clone(), ceasar.clone()]
    );
    // The same across a paragraph break, as the opinion's text has it.
    assert_eq!(
        parties(
            "People v. Taylor, 219 Ill. App. 3d 47 (1991), and\n\nPeople v. Ceasar, 231 Ill. \
             App. 3d 54 (1992)."
        ),
        [taylor, ceasar]
    );
}

#[test]
fn name_after_a_pin_cite_parenthetical_or_court() {
    assert_eq!(
        parties(
            "Jackson v. Virginia, 443 U.S. 307, 315-16 (1979), and People v. Siguenza-Brito, 235 \
             Ill. 2d 213, 224 (2009)."
        )[1],
        named("235 Ill. 2d 213", "People", "Siguenza-Brito")
    );
    assert_eq!(
        parties(
            "Smith v. Jones, 1 F.3d 1 (7th Cir. 1993), and United States v. Brown, 2 F.3d 2 (7th \
             Cir. 1993)."
        )[1],
        named("2 F.3d 2", "United States", "Brown")
    );
    assert_eq!(
        parties("Smith v. Jones, 1 F.3d 1 (7th Cir. 1993); see also Doe v. Roe, 2 F.3d 2 (1994).")
            [1],
        named("2 F.3d 2", "Doe", "Roe")
    );
}

// A year between the name and the volume is still the citation's own.
#[test]
fn year_before_the_volume_is_unchanged() {
    assert_eq!(
        parties("Stanley v. Illinois (1972), 405 U.S. 645, 651."),
        [named("405 U.S. 645", "Stanley", "Illinois")]
    );
}
