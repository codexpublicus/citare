//! The case name before a short case citation (#27, items 2 and 3): with a
//! "v.", eyecite 2.7.8 gives the defendant as `antecedent_guess` and the
//! plaintiff as `plaintiff`, from the same backward scan as a full
//! citation's. citare joined both sides ("Walters Walters") and kept no
//! plaintiff. The inputs are from CourtListener opinion 4606371 in the
//! one-percent sample, and the expected values are eyecite 2.7.8's.

use citare_core::{get_citations, Citation};

/// A short case citation's antecedent guess, plaintiff and full span.
type ShortName = (Option<String>, Option<String>, (usize, usize));

fn short_names(text: &str) -> Vec<ShortName> {
    get_citations(text, false, "", &[])
        .iter()
        .filter_map(|c| match c {
            Citation::ShortCase(d) => Some((
                d.metadata.antecedent_guess.clone(),
                d.metadata.plaintiff.clone(),
                c.full_span(),
            )),
            _ => None,
        })
        .collect()
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

#[test]
fn antecedent_is_the_defendant() {
    assert_eq!(
        short_names(
            "See Marks v. Marks, 342 S.E.2d at 861; Walters v. Walters, 298 S.E.2d at 342."
        ),
        [
            (s("Marks"), s("Marks"), (4, 37)),
            (s("Walters"), s("Walters"), (39, 76)),
        ]
    );
    assert_eq!(
        short_names("In State v. Smith, 298 S.E.2d at 342."),
        [(s("Smith"), s("State"), (0, 36))]
    );
}

// The full span starts at the case name, not at the sentence before it.
#[test]
fn case_name_stops_at_the_sentence_before() {
    assert_eq!(
        short_names("of North Carolina. Marks v. Marks, 342 S.E.2d at 861."),
        [(s("Marks"), s("Marks"), (19, 52))]
    );
}

#[test]
fn no_plaintiff_without_a_v() {
    assert_eq!(
        short_names("Foo, 298 S.E.2d at 342."),
        [(s("Foo"), None, (0, 22))]
    );
}

#[test]
fn plaintiff_in_metadata_fields() {
    let cites = get_citations("Walters v. Walters, 298 S.E.2d at 342.", false, "", &[]);
    let fields = cites[0].metadata_fields();
    assert!(
        fields.contains(&("plaintiff", Some("Walters"))),
        "{fields:?}"
    );
    assert!(fields.contains(&("antecedent_guess", Some("Walters"))));
}
