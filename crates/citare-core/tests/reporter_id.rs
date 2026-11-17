//! `69 id. 76`: "id." standing for the reporter of the citation before it
//! (#129). Nineteenth-century and early twentieth-century opinions cite a
//! run of cases in one reporter as `Langdon v. The Mayor, 92 N.Y. 427;
//! Thomas v. Thomas, 18 id. 481`. eyecite reads the `id.` as an id.
//! citation, which resolves to whatever came before it; citare reads a
//! volume, `id.` and a page as a full case citation in the reporter of the
//! case citation before it.

use citare_core::{get_citations, resolve_citations, Citation};

fn find(text: &str) -> Vec<Citation> {
    get_citations(text, false, "", &[])
}

/// The full case citations in `text`, as (volume, normalized reporter,
/// page, plaintiff, matched text).
fn cases(text: &str) -> Vec<(String, String, String, Option<String>, String)> {
    find(text)
        .into_iter()
        .filter_map(|c| match &c {
            Citation::FullCase(d) => {
                let g = &d.resource.common.groups;
                Some((
                    g["volume"].clone().unwrap_or_default(),
                    c.corrected_reporter().unwrap_or_default(),
                    g["page"].clone().unwrap_or_default(),
                    d.metadata.plaintiff.clone(),
                    c.matched_text().to_string(),
                ))
            }
            _ => None,
        })
        .collect()
}

fn ids(text: &str) -> usize {
    find(text)
        .iter()
        .filter(|c| matches!(c, Citation::Id(_)))
        .count()
}

#[test]
fn reporter_id_is_a_full_citation_in_the_reporter_before_it() {
    let text = "Langdon v. The Mayor, 92 N.Y. 427; Thomas v. Thomas, 18 id. 481.";
    let found = cases(text);
    assert_eq!(found.len(), 2, "{found:?}");
    let (vol, rep, page, plaintiff, matched) = &found[1];
    assert_eq!(
        (vol.as_str(), rep.as_str(), page.as_str()),
        ("18", "N.Y.", "481")
    );
    assert_eq!(plaintiff.as_deref(), Some("Thomas"));
    assert_eq!(matched, "18 id. 481");
    assert_eq!(ids(text), 0);
}

/// The #129 example's form: `Fink v. Disbrow, 69 id. 76` after an Illinois
/// citation is 69 Ill. 76.
#[test]
fn paddock_v_palmer_form() {
    let text = "See Bowen v. Hastings, 47 Ill. 192; Fink v. Disbrow, 69 id. 76, \
                and the cases there cited.";
    let found = cases(text);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[1].0, "69");
    assert_eq!(found[1].1, "Ill.");
    assert_eq!(found[1].2, "76");
    assert_eq!(found[1].3.as_deref(), Some("Fink"));
    assert_eq!(ids(text), 0);
}

/// In parentheses, with a capital `Id.`, and after a short cite.
#[test]
fn variants() {
    let text = "(Smith v. Jones, 5 N.Y. 1; Genet v. City of Brooklyn (99 Id. 296).)";
    let found = cases(text);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[1].0, "99");
    assert_eq!(found[1].1, "N.Y.");
    assert_eq!(found[1].2, "296");
    let text = "Smith, 5 N.Y. at 3; Brown v. Green, 12 id. 323.";
    let found = cases(text);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].1, "N.Y.");
    assert_eq!(found[0].2, "323");
}

/// `Delaney v. McCormack, id. 174`: "id." standing for the volume and
/// reporter of the case citation before it, after a case name.
#[test]
fn same_volume_after_a_case_name() {
    let text = "Smith v. Edwards, 88 N.Y. 92; Delaney v. McCormack, id. 174; \
                Matter of Crossman, Id. 503.";
    let found = cases(text);
    assert_eq!(found.len(), 3, "{found:?}");
    for (case, page, plaintiff) in [(&found[1], "174", "Delaney"), (&found[2], "503", "")] {
        assert_eq!((case.0.as_str(), case.1.as_str()), ("88", "N.Y."));
        assert_eq!(case.2, page);
        if !plaintiff.is_empty() {
            assert_eq!(case.3.as_deref(), Some(plaintiff));
        }
    }
    assert_eq!(found[1].4, "id. 174");
    let text = "In re Darling, 6 N.Y.S. 191; In re Bennett, Id. 199.";
    let found = cases(text);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!((found[1].0.as_str(), found[1].2.as_str()), ("6", "199"));
    assert_eq!(ids(text), 0);
    // Both forms in a row.
    let text = "Smith v. Edwards, 88 N.Y. 92; Jones v. Brown, 90 id. 5; Delaney v. Roe, id. 174.";
    let found = cases(text);
    assert_eq!(found.len(), 3, "{found:?}");
    assert_eq!((found[2].0.as_str(), found[2].2.as_str()), ("90", "174"));
}

/// After a parallel group, "id." is the group's first reporter.
#[test]
fn first_reporter_of_a_parallel_group() {
    let text = "Bennett v. Kidd, 74 W. Va. 569, 82 S.E. 614; Cook v. Raleigh Lumber Co., \
                Id. 503, 82 S.E. 327.";
    let found = cases(text);
    assert_eq!(found.len(), 4, "{found:?}");
    assert_eq!(
        (
            found[2].0.as_str(),
            found[2].1.as_str(),
            found[2].2.as_str()
        ),
        ("74", "W. Va.", "503")
    );
}

/// The case name has to be in the same clause: not after a `;` with no
/// case name in between, as in a chemistry journal's `ibid. 72, 280`.
#[test]
fn case_name_in_the_same_clause() {
    let text = "Smith v. Jones, 50 Ill. 100. See Kondo, J. Pharm. Soc. Japan 48, 1156 (1928); \
                Tomita, Kusuda, ibid. 72, 280 (1952).";
    assert_eq!(cases(text).len(), 1);
}

/// After a citation the tokenizer doesn't recognize, "id." stands for that
/// reporter, which is unknown: no citation, and no id. citation either.
#[test]
fn after_an_unrecognized_citation() {
    let text = "Smith v. Jones, 5 Hill, 10. Biggs v. Lawrence, (3 Durnford & East, 454,) \
                and Waymell v. Reed, (5 Id. 599,) were cases.";
    assert_eq!(cases(text).len(), 1);
    let text = "Smith v. Jones, 1 Denio, 10. Spaulding v. Spaulding, 3 How. Pr. R. 297. \
                Daws v. Green, Id. 377.";
    assert!(cases(text).iter().all(|c| c.1 != "Denio" || c.2 != "377"));
    assert_eq!(ids(text), 0);
}

/// A party named Thompson reads as the nominative reporter `Thompson` with
/// no volume; it isn't what a later `id.` stands for.
#[test]
fn not_after_a_citation_with_no_volume() {
    let text = "Markham v. Jaudon, 41 N.Y. 235; Capron v. Thompson, 86 id. 418; \
                Taussig v. Hart, 58 id. 429.";
    let found = cases(text);
    let last = found.last().unwrap();
    assert_eq!(
        (last.0.as_str(), last.1.as_str(), last.2.as_str()),
        ("58", "N.Y.", "429")
    );
}

/// An id. citation after the reporter id. refers to it.
#[test]
fn id_after_reporter_id_resolves_to_it() {
    let text = "Smith v. Jones, 50 Ill. 100; Fink v. Disbrow, 69 id. 76. Id. at 80.";
    let citations = find(text);
    let resolutions = resolve_citations(&citations);
    let id = citations
        .iter()
        .find(|c| matches!(c, Citation::Id(_)))
        .expect("an id. citation");
    let (resource, _) = resolutions
        .iter()
        .find(|(_, members)| members.iter().any(|m| m.span() == id.span()))
        .expect("the id. citation resolves");
    let full = resolutions[resource]
        .iter()
        .find(|m| matches!(m, Citation::FullCase(_)))
        .unwrap();
    assert_eq!(full.matched_text(), "69 id. 76");
}

/// Left as id. citations: a real `Id. at`, a volume and page with no case
/// citation before them, and a reporter id. after a statute, whose `id.` is
/// the statute's code.
#[test]
fn not_a_reporter_id() {
    assert_eq!(ids("Smith v. Jones, 50 Ill. 100. Id. at 102."), 1);
    assert_eq!(ids("As held in 92 id. 427, the rule applies."), 1);
    // `Id. 174` with no case name before it is the citation's id. citation.
    assert_eq!(ids("Smith v. Jones, 50 Ill. 100. Id. 102."), 1);
    assert_eq!(ids("Smith v. Jones, 50 Ill. 100, and see, id. 102."), 1);
    let text = "See 42 U.S.C. § 1983; 2 id. 5.";
    assert_eq!(cases(text).len(), 0);
}
