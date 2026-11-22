//! Subsequent history (#114): a full case citation with no case name of its
//! own that follows another's pin cites, parentheticals and a history phrase
//! (`cert. denied`, `aff'd`, ...) is a later decision in the same case. It
//! keeps no plaintiff or defendant of its own; `history_plaintiff` and
//! `history_defendant` give the parties of the case it continues.

use citare_core::{get_citations, Citation, CleanStep};

/// Each full case citation's matched text, its own parties and the parties
/// of the case it is subsequent history of.
type Names = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn names_of(cites: Vec<Citation>) -> Vec<Names> {
    cites
        .into_iter()
        .filter_map(|c| match c {
            Citation::FullCase(ref d) => Some((
                c.matched_text().to_string(),
                d.metadata.plaintiff.clone(),
                d.metadata.defendant.clone(),
                d.metadata.history_plaintiff.clone(),
                d.metadata.history_defendant.clone(),
            )),
            _ => None,
        })
        .collect()
}

fn names(text: &str) -> Vec<Names> {
    names_of(get_citations(text, false, "", &[]))
}

fn s(v: &str) -> Option<String> {
    Some(v.to_string())
}

/// The history parties the citation `at` was given, and that it has no
/// parties of its own.
fn history(found: &[Names], at: usize) -> (Option<String>, Option<String>) {
    let (text, plaintiff, defendant, hp, hd) = &found[at];
    assert!(
        plaintiff.is_none() && defendant.is_none(),
        "{text} has parties of its own: {found:?}"
    );
    (hp.clone(), hd.clone())
}

#[test]
fn cert_denied_takes_the_case_name_it_follows() {
    let found =
        names("Daye v. Bounds, 509 F.2d 66 (4th Cir.), cert. denied, 421 U.S. 1002 (1975).");
    assert_eq!(found[1].0, "421 U.S. 1002");
    assert_eq!(history(&found, 1), (s("Daye"), s("Bounds")));
    // The case's own citation keeps its parties and has no history.
    assert_eq!(
        found[0],
        ("509 F.2d 66".into(), s("Daye"), s("Bounds"), None, None)
    );
}

#[test]
fn a_parallel_run_in_history_takes_the_name() {
    let found = names(
        "Segreto v. Bristol, 71 Conn.App. 844, 848, 804 A.2d 928, cert. denied, 261 Conn. 941, \
         808 A.2d 1132 (2002).",
    );
    assert_eq!(found[2].0, "261 Conn. 941");
    assert_eq!(history(&found, 2), (s("Segreto"), s("Bristol")));
    assert_eq!(history(&found, 3), (s("Segreto"), s("Bristol")));
    let found = names(
        "Flaksa v. Little River Marine Construction Co., 389 F.2d 885 (5th Cir.), cert. den., \
         392 U.S. 928, 88 S.Ct. 2287, 20 L.Ed.2d 1387 (1968).",
    );
    for at in 1..4 {
        assert_eq!(
            history(&found, at),
            (s("Flaksa"), s("Little River Marine Construction Co.")),
            "{found:?}"
        );
    }
}

#[test]
fn other_history_phrases() {
    // A second history after another, `aff'd after remand`, N.J.'s `certif.
    // denied`, `reh. denied`, a bracketed note, and `cert, denied`.
    for (text, at, plaintiff, defendant) in [
        (
            "State v. Webb, 238 Conn. 389, 468 n.56, 680 A.2d 147 (1996), aff'd after remand, 252 \
             Conn. 128, 750 A.2d 448, cert. denied, 531 U.S. 835, 121 S. Ct. 93, 148 L. Ed. 2d 53 \
             (2000).",
            4,
            "State",
            "Webb",
        ),
        (
            "Hantman v. Township of Randolph, 58 N.J.Super. 127, 155 A.2d 554 (App.Div.1959), \
             certif. denied, 31 N.J. 550.",
            2,
            "Hantman",
            "Township of Randolph",
        ),
        (
            "United States v. Mendenhall, 446 U.S. 544, 553-54, 100 S. Ct. 1870, 64 L. Ed. 2d 497, \
             reh. denied, 448 U.S. 908.",
            3,
            "United States",
            "Mendenhall",
        ),
        (
            "United States v. Rosenwasser, 550 F.2d 806, 814 (2d Cir.) [Gurfein, J., dissenting], \
             cert. denied, 434 U.S. 825.",
            1,
            "United States",
            "Rosenwasser",
        ),
        (
            "Exxon Corp. v. FTC, 589 F.2d 582, 587 (D.C.Cir.1978), cert, denied, 441 U.S. 943.",
            1,
            "Exxon Corp.",
            "FTC",
        ),
        (
            "Roe v. Doe, 3 F.3d 4, 5 (1st Cir. 1993) (holding that the rule applies), rev'd, 500 \
             U.S. 1 (1994).",
            1,
            "Roe",
            "Doe",
        ),
    ] {
        let found = names(text);
        assert_eq!(
            history(&found, at),
            (s(plaintiff), s(defendant)),
            "{text}: {found:?}"
        );
    }
}

#[test]
fn no_history_name_without_a_history_phrase() {
    // A citation after a new sentence, history under another name (`aff'd
    // sub nom.`), and a nameless citation after a plain comma.
    for text in [
        "Roe v. Wade, 410 U.S. 113 (1973). See 261 Conn. 941.",
        "Roe v. Wade, 410 U.S. 113 (1973), aff'd sub nom. Doe v. Bolton, 261 Conn. 941.",
        "Roe v. Wade, 410 U.S. 113 (1973); 261 Conn. 941.",
    ] {
        let found = names(text);
        assert!(
            found.iter().all(|n| n.3.is_none() && n.4.is_none()),
            "{text}: {found:?}"
        );
    }
}

#[test]
fn history_in_markup() {
    let found = names_of(get_citations(
        "",
        false,
        "<p><i>Daye v. Bounds,</i> 509 F.2d 66 (4th Cir.), <i>cert. denied,</i> 421 U.S. 1002 \
         (1975).</p>",
        &[CleanStep::Html, CleanStep::InlineWhitespace],
    ));
    assert_eq!(found[1].0, "421 U.S. 1002");
    assert_eq!(history(&found, 1), (s("Daye"), s("Bounds")));
}

#[test]
fn history_in_metadata_fields() {
    let cites = get_citations(
        "Daye v. Bounds, 509 F.2d 66 (4th Cir.), cert. denied, 421 U.S. 1002 (1975).",
        false,
        "",
        &[],
    );
    let fields = cites[1].metadata_fields();
    assert!(fields.contains(&("history_plaintiff", Some("Daye"))));
    assert!(fields.contains(&("history_defendant", Some("Bounds"))));
}
