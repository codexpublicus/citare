//! Parallel citation groups (#66): adjacent full case citations that `find`
//! gives one case name and full span start, the way it marks a decision cited
//! in more than one reporter. A group must never take in a citation of a
//! different decision.
//!
//! Each input comes from an opinion in the CourtListener ten-percent sample.

use citare_core::{get_citations, Citation, CleanStep};

/// The parallel groups in `cites`, as matched texts: runs of full case
/// citations, in output order, that share a full span start (the pairs
/// `find` copies the case name across and `resolve` treats as one decision).
fn groups(cites: &[Citation]) -> Vec<Vec<String>> {
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut last_start = None;
    for c in cites {
        let Citation::FullCase(d) = c else {
            last_start = None;
            continue;
        };
        let start = d.resource.common.full_span_start;
        if start.is_some() && start == last_start {
            groups
                .last_mut()
                .unwrap()
                .push(c.matched_text().to_string());
        } else {
            groups.push(vec![c.matched_text().to_string()]);
        }
        last_start = start;
    }
    groups.retain(|g| g.len() > 1);
    groups
}

fn markup(text: &str) -> Vec<Vec<String>> {
    groups(&get_citations(
        "",
        false,
        text,
        &[CleanStep::Html, CleanStep::InlineWhitespace],
    ))
}

fn plain(text: &str) -> Vec<Vec<String>> {
    groups(&get_citations(text, false, "", &[]))
}

/// Compare groups, each as a set: output order follows the full spans, which
/// put a bracketed parallel cite before the one it follows.
fn expect(mut groups: Vec<Vec<String>>, expected: &[&[&str]]) {
    let mut expected: Vec<Vec<String>> = expected
        .iter()
        .map(|g| g.iter().map(|s| s.to_string()).collect())
        .collect();
    for g in groups.iter_mut().chain(expected.iter_mut()) {
        g.sort();
    }
    assert_eq!(groups, expected);
}

/// The defendant `find` gives the full case citation `matched`.
fn defendant(cites: &[Citation], matched: &str) -> Option<String> {
    cites.iter().find_map(|c| match c {
        Citation::FullCase(d) if c.matched_text() == matched => d.metadata.defendant.clone(),
        _ => None,
    })
}

// --- Two decisions that must not be grouped ---------------------------------

// courtlistener-10pct/4795483: the second case is cited by a short name, and
// the markup scan for "v." went back past the first citation to its name.
#[test]
fn case_after_semicolon_with_short_name_starts_a_group() {
    let text = "<p>(citing <i>Sutton v. Elrod,</i> 724 So.2d 551 (Ala.Civ.App.1998); \
                <i>Beasley,</i> 564 So.2d 959 (Ala.Civ.App.1990)).</p>";
    expect(markup(text), &[]);
    let cites = get_citations(
        "",
        false,
        text,
        &[CleanStep::Html, CleanStep::InlineWhitespace],
    );
    assert_ne!(defendant(&cites, "564 So.2d 959").as_deref(), Some("Elrod"));
}

// courtlistener-10pct/5732968: "Matter of" names a new case without "v.".
#[test]
fn matter_of_after_semicolon_starts_a_group() {
    let text = "<p>(<i>see Matter of Jamal B. v Danielle C.,</i> 38 AD3d 908 [2007]; \
                <i>Matter of Marie H,</i> 25 AD3d 704 [2006]).</p>";
    expect(markup(text), &[]);
}

// courtlistener-10pct/5189464: affirmed by another court, a second decision.
#[test]
fn subsequent_history_is_not_parallel() {
    expect(
        markup("<p>In the case of <i>Zink v. McManus</i> (49 Hun, 583; affd., 121 N. Y. 259) both the owner and occupant.</p>"),
        &[],
    );
    expect(
        markup("<p>(<i>City of New York v. Trustees,</i> 85 App. Div. 355, 361; affd. on opinion below, 180 N. Y. 527.)</p>"),
        &[],
    );
}

// courtlistener-10pct/4664021: two decisions in one case.
#[test]
fn citations_joined_by_and_are_not_parallel() {
    expect(
        markup("<p>See <i>Commonwealth v. Hart,</i> 470 Mass. 837 and 472 Mass. 448 (2015).</p>"),
        &[],
    );
}

// A citation after a year that closes the one before it is another decision.
#[test]
fn citation_after_closing_year_is_not_parallel() {
    expect(
        markup("<p>See <i>Smith v. Jones,</i> 209 P. 1102 (1922), 214 P. 146.</p>"),
        &[],
    );
}

// --- Parallel citations that must stay grouped ------------------------------

#[test]
fn federal_parallel_cites_with_pins_stay_grouped() {
    let text = "Celotex Corp. v. Catrett, 477 U.S. 317, 322-23, 106 S.Ct. 2548, \
                91 L.Ed.2d 265 (1986).";
    let both = [&["477 U.S. 317", "106 S.Ct. 2548", "91 L.Ed.2d 265"][..]];
    expect(plain(text), &both);
    expect(
        markup(&format!(
            "<p>{}</p>",
            text.replace(
                "Celotex Corp. v. Catrett,",
                "<i>Celotex Corp. v. Catrett,</i>"
            )
        )),
        &both,
    );
}

// The issue's list: two decisions, each cited in parallel, form two groups.
#[test]
fn citing_list_forms_one_group_per_case() {
    let text = "(citing <i>Celotex Corp. v. Catrett,</i> 477 U.S. 317, 322, 106 S.Ct. 2548; \
                <i>Anderson v. Liberty Lobby, Inc.,</i> 477 U.S. 242, 248, 106 S.Ct. 2505)";
    expect(
        markup(&format!("<p>{text}</p>")),
        &[
            &["477 U.S. 317", "106 S.Ct. 2548"],
            &["477 U.S. 242", "106 S.Ct. 2505"],
        ],
    );
}

// courtlistener-10pct/2011712: Michigan separates parallel cites with a semicolon.
#[test]
fn michigan_semicolon_parallel_stays_grouped() {
    expect(
        markup("<p><i>People v Smith,</i> 392 Mich 30, 35; 219 NW2d 41 (1974).</p>"),
        &[&["392 Mich 30", "219 NW2d 41"]],
    );
}

// courtlistener-10pct/4697148: Georgia puts the parallel cites in parentheses.
#[test]
fn georgia_parenthesized_parallel_stays_grouped() {
    expect(
        markup(
            "<p><i>Jackson v. Virginia,</i> 443 U.S. 307 (99 SCt 2781, 61 LE2d 560) (1979).</p>",
        ),
        &[&["443 U.S. 307", "99 SCt 2781"]],
    );
    expect(
        markup("<p><i>Smith v. State,</i> 252 Ga. 7 (1) (310 SE2d 528) (1984).</p>"),
        &[&["252 Ga. 7", "310 SE2d 528"]],
    );
}

// courtlistener-10pct/2175059: California brackets them.
#[test]
fn california_bracketed_parallel_stays_grouped() {
    expect(
        markup("<p>(see <i>Hutnick v. United States Fidelity &amp; Guaranty Co.</i> (1988) 47 Cal.3d 456, 462 [253 Cal. Rptr. 236, 763 P.2d 1326].)</p>"),
        &[&["47 Cal.3d 456", "253 Cal. Rptr. 236", "763 P.2d 1326"]],
    );
}

// courtlistener-10pct/4655164 and 3713718: paragraph pins, and an Ohio Bar
// Reports cite citare doesn't recognize, between parallel cites.
#[test]
fn paragraph_pins_and_unknown_reporters_stay_grouped() {
    expect(
        markup("<p><i>State v. Kelley,</i> 2006 MT 84, ¶ 19, 332 Mont. 27, 134 P.3d 64.</p>"),
        &[&["2006 MT 84", "332 Mont. 27", "134 P.3d 64"]],
    );
    expect(
        markup("<p><i>Wing v. Anchor Media, Ltd.,</i> 2 Ohio App.3d 335, 2 OBR 380, 442 N.E.2d 101 (1982).</p>"),
        &[&["2 Ohio App.3d 335", "442 N.E.2d 101"]],
    );
}

// Pin cites that lost their parallel group in a first version of #66, from
// courtlistener-10pct/1337419, 1446766, 1876512, 1421553, 2119525, 732313.
#[test]
fn pin_cite_forms_stay_grouped() {
    for (text, group) in [
        (
            "<p><i>Oglethorpe Power Corp. v. Sheriff,</i> 210 Ga. App. 299, 301(4)(b), 436 S.E.2d 14 (1993).</p>",
            &["210 Ga. App. 299", "436 S.E.2d 14"][..],
        ),
        (
            "<p><i>Ankenbrandt v. Richards,</i> 504 U.S. 689, 703 and n. 6, 112 S.Ct. 2206, 2214-15 n. 6, 119 L.Ed.2d 468 (1992).</p>",
            &["504 U.S. 689", "112 S.Ct. 2206", "119 L.Ed.2d 468"][..],
        ),
        (
            "<p><i>Tilghman v. Proctor,</i> 125 U.S. 136-160, 8 S. Ct. 894, 31 L.Ed. 664.</p>",
            &["125 U.S. 136", "8 S. Ct. 894", "31 L.Ed. 664"][..],
        ),
        (
            "<p><i>State Farm Mut. Auto. Ins. Co. v. Campbell,</i> 538 U.S. 408, 418(II), 123 S.Ct. 1513, 155 L.Ed.2d 585 (2003).</p>",
            &["538 U.S. 408", "123 S.Ct. 1513", "155 L.Ed.2d 585"][..],
        ),
        (
            "<p><i>Nantahala Power &amp; Light Co. v. Thornburg,</i> 476 U.S. 953, 74 PUR4th 464, 90 L.Ed.2d 943, 106 S.Ct. 2349 (1986).</p>",
            &["476 U.S. 953", "90 L.Ed.2d 943", "106 S.Ct. 2349"][..],
        ),
        (
            "<p><i>Turner Broadcasting System, Inc. v. FCC,</i> 512 U.S. 622, ----, 114 S.Ct. 2445, 2458, 129 L.Ed.2d 497 (1994).</p>",
            &["512 U.S. 622", "114 S.Ct. 2445", "129 L.Ed.2d 497"][..],
        ),
    ] {
        expect(markup(text), &[group]);
    }
}

// courtlistener-10pct/2418739 and 1545020: a case named without "v." after
// another decision's citations gets its own name and its own group.
#[test]
fn case_named_without_v_after_another_case_is_its_own_group() {
    let text = "the Supreme Court in Daubert, 509 U.S. 579, 113 S.Ct. 2786, 125 L.Ed.2d 469, \
                and Kumho Tire, 526 U.S. 137, 119 S.Ct. 1167, 143 L.Ed.2d 238. The Supreme Court";
    let daubert = &["509 U.S. 579", "113 S.Ct. 2786", "125 L.Ed.2d 469"][..];
    let kumho = &["526 U.S. 137", "119 S.Ct. 1167", "143 L.Ed.2d 238"][..];
    expect(plain(text), &[daubert, kumho]);
    expect(
        markup(&format!(
            "<p>{}</p>",
            text.replace("Kumho Tire,", "<i>Kumho Tire,</i>")
        )),
        &[daubert, kumho],
    );
    let cites = get_citations(text, false, "", &[]);
    assert_eq!(
        defendant(&cites, "119 S.Ct. 1167").as_deref(),
        Some("Kumho Tire")
    );

    let text = "Pacific Coast S. S. Co. (C. C. A.) 287 F. 679. In The Conqueror, 166 U. S. 125, \
                17 S. Ct. 510, 41 L. Ed. 937, the court said";
    expect(
        plain(text),
        &[&["166 U. S. 125", "17 S. Ct. 510", "41 L. Ed. 937"]],
    );
}

// courtlistener-10pct/3544348: a bracketed four-digit pin isn't a year.
#[test]
fn bracketed_pin_is_not_a_closing_year() {
    expect(
        plain("( In re Walker's Estate , 110 Cal. 387 , 42 P. 815 [1082], 30 L.R.A. 460, 52 Am. St. Rep. 104.)"),
        &[&["110 Cal. 387", "42 P. 815", "30 L.R.A. 460", "52 Am. St. Rep. 104"]],
    );
}

// Older reprint cites citare doesn't find, between parallel cites, from
// courtlistener-10pct/3886325, 3997530, 1448839, 2438989 and 3510348.
#[test]
fn reprint_and_syllabus_forms_stay_grouped() {
    for (text, group) in [
        (
            "<p><i>Smith v. Jones,</i> 226 Mass. 545, Ann. Cas. 1918C, 529, 110 N.E. 972.</p>",
            &["226 Mass. 545", "110 N.E. 972"][..],
        ),
        (
            "<p><i>Smith v. Jones,</i> 36 Cal.2d 301, 308 et seq. [223 P.2d 471].</p>",
            &["36 Cal.2d 301", "223 P.2d 471"][..],
        ),
        (
            "<p><i>State v. Jones,</i> 49 Ohio St.3d 228, syllabus ¶ 3, 551 N.E.2d 981 (1990).</p>",
            &["49 Ohio St.3d 228", "551 N.E.2d 981"][..],
        ),
        (
            "<p><i>Smith v. Jones,</i> 73 N.W. 724 (s.c. 75 Minn. 100).</p>",
            &["73 N.W. 724", "75 Minn. 100"][..],
        ),
    ] {
        expect(markup(text), &[group]);
    }
}

// In plain text, eyecite's scan stops at "(N.S.)" on its own; the markup scan
// doesn't.
#[test]
fn lra_reprint_stays_grouped_in_markup() {
    expect(
        markup("<p><i>Smith v. Jones,</i> 131 Ga. 3, 65 S.E. 1047, 27 L.R.A. (N.S.) 435, 137 Am. St. Rep. 839.</p>"),
        &[&["131 Ga. 3", "65 S.E. 1047", "137 Am. St. Rep. 839"]],
    );
}

// courtlistener-10pct/2193933 and 6857250: looking back past "v." for the
// plaintiff went over "and" and the citation before it into the earlier
// case.
#[test]
fn plaintiff_search_stops_at_an_earlier_citation() {
    let text = "See Wade v. State, 236 Ind. 350, 140 N.E.2d 98, and Hughes v. State, \
                212 Ind. 577, 10 N.E.2d 629.";
    expect(
        plain(text),
        &[
            &["236 Ind. 350", "140 N.E.2d 98"],
            &["212 Ind. 577", "10 N.E.2d 629"],
        ],
    );
    let cites = get_citations(text, false, "", &[]);
    let plaintiff = cites.iter().find_map(|c| match c {
        Citation::FullCase(d) if c.matched_text() == "212 Ind. 577" => d.metadata.plaintiff.clone(),
        _ => None,
    });
    assert_eq!(plaintiff.as_deref(), Some("Hughes"));

    expect(
        plain("of In re Harding, 120 U. S. 782, 7 S. Ct. 780, 30 L. Ed. 824, and Braun v. U. S. A., 16 F.(2d) 118, and Wagner"),
        &[&["120 U. S. 782", "7 S. Ct. 780", "30 L. Ed. 824"]],
    );
}

// courtlistener-10pct/342566: the year closing the earlier case, "(1970)",
// isn't part of the next case's name.
#[test]
fn case_after_closing_year_and_and_gets_its_name() {
    let text = "cases of McMann v. Richardson, 397 U.S. 759, 90 S.Ct. 1441, 25 L.Ed.2d 763 (1970) \
                and Brady v. United States, 397 U.S. 742, 90 S.Ct. 1463, 25 L.Ed.2d 747 (1970). 16";
    expect(
        plain(text),
        &[
            &["397 U.S. 759", "90 S.Ct. 1441", "25 L.Ed.2d 763"],
            &["397 U.S. 742", "90 S.Ct. 1463", "25 L.Ed.2d 747"],
        ],
    );
    let cites = get_citations(text, false, "", &[]);
    assert_eq!(
        defendant(&cites, "90 S.Ct. 1463").as_deref(),
        Some("United States")
    );
}

// courtlistener-10pct/524658: the last parallel cite is too far from the
// case name for its own search to reach it, and still joins the group.
#[test]
fn parallel_cite_beyond_the_search_window_stays_grouped() {
    let text = "Heizer was decided before Texas Industries, 451 U.S. at 629, 101 S.Ct. at 2061, \
                and Northwest Airlines, Inc. v. Transport Workers Union of America, 451 U.S. 77, \
                101 S.Ct. 1571, 67 L.Ed.2d 750 (1981). In both";
    expect(
        plain(text),
        &[&["451 U.S. 77", "101 S.Ct. 1571", "67 L.Ed.2d 750"]],
    );
}

// One decision is printed once per reporter, so two citations in one
// reporter are two decisions (eyecite's test_reference_resolution, and
// courtlistener-10pct/4490249).
#[test]
fn citations_in_one_reporter_are_not_parallel() {
    expect(
        plain("Foo v. Bar 1 U.S. 12, 347-348; 12 U.S. 1. someting; In Foo at 2, we see that"),
        &[],
    );
    expect(
        markup("<p><i>Smith v. Commissioner,</i> 13 B. T. A. 557; 13 B. T. A. 1274.</p>"),
        &[],
    );
}

// courtlistener-10pct/6636114: one emphasis tag around a whole string cite
// isn't a case name, and gave every case in it the tag's start.
#[test]
fn emphasis_around_a_string_cite_is_not_a_case_name() {
    let text = "<p>discretionary power.— <em>Bate v. Hill, 1 Car. &amp; P., 100; Turnpike Co. v. \
                Loomis, 32 N. Y., 127; La Beau v. The People, 34 N. Y., 223; Com. v. Jennings, \
                107 Mass., 488.</em></p>";
    expect(markup(text), &[]);
    let cites = get_citations(
        "",
        false,
        text,
        &[CleanStep::Html, CleanStep::InlineWhitespace],
    );
    assert_eq!(
        defendant(&cites, "34 N. Y., 223").as_deref(),
        Some("The People")
    );
    // "1 Car. & P., 100" isn't a reporter citare knows, so only the
    // semicolon marks where Bate v. Hill ends.
    assert_eq!(
        defendant(&cites, "32 N. Y., 127").as_deref(),
        Some("Loomis")
    );
}

// courtlistener-10pct/3564744: only a semicolon or citation inside the
// emphasis tag counts; one after it, before the citation, doesn't.
#[test]
fn semicolon_after_the_name_tag_keeps_the_name() {
    let text = "<p>(Supreme Court rule 58); <em>Bentley</em> v. <em>Colgate, 10 N.J. Mis. R. \
                1222</em>, 1227;<em>163 Atl. Rep. 98</em>; <em>Gibson</em> v. \
                <em>Gillespie</em> (Del.),<em>143 Atl. Rep. 544</em>; and more.</p>";
    let cites = get_citations(
        "",
        false,
        text,
        &[CleanStep::Html, CleanStep::InlineWhitespace],
    );
    let plaintiff = cites.iter().find_map(|c| match c {
        Citation::FullCase(d) if c.matched_text() == "163 Atl. Rep. 98" => {
            d.metadata.plaintiff.clone()
        }
        _ => None,
    });
    assert_eq!(plaintiff.as_deref(), Some("Bentley"));
}
