//! Markup-aware case names and references, after the cases in eyecite
//! 2.7.8's `tests/test_FindTest.py`.

use citare_core::{get_citations, Citation, CleanStep};

type Expected = (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
    Option<&'static str>,
);

type Summary = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// (type, plaintiff, defendant, antecedent_guess, pin_cite)
fn summarize(c: &Citation) -> Summary {
    let name = c.type_name().to_string();
    match c {
        Citation::FullCase(d) => (
            name,
            d.metadata.plaintiff.clone(),
            d.metadata.defendant.clone(),
            d.metadata.antecedent_guess.clone(),
            d.metadata.case.pin_cite.clone(),
        ),
        Citation::ShortCase(d) => (
            name,
            None,
            None,
            d.metadata.antecedent_guess.clone(),
            d.metadata.case.pin_cite.clone(),
        ),
        Citation::Reference(d) => (
            name,
            d.metadata.plaintiff.clone(),
            d.metadata.defendant.clone(),
            None,
            d.metadata.pin_cite.clone(),
        ),
        Citation::Supra(d) => (
            name,
            None,
            None,
            d.metadata.antecedent_guess.clone(),
            d.metadata.pin_cite.clone(),
        ),
        Citation::Id(d) => (name, None, None, None, d.metadata.pin_cite.clone()),
        _ => (name, None, None, None, None),
    }
}

fn markup_citations(markup: &str, steps: &[&str]) -> Vec<Citation> {
    let steps: Vec<CleanStep> = steps
        .iter()
        .map(|s| CleanStep::from_name(s).unwrap())
        .collect();
    get_citations("", false, markup, &steps)
}

fn check(case: usize, markup: &str, steps: &[&str], expected: &[Expected]) {
    let got: Vec<Summary> = markup_citations(markup, steps)
        .iter()
        .map(summarize)
        .collect();
    let want: Vec<Summary> = expected
        .iter()
        .map(|(t, p, d, a, pc)| {
            (
                t.to_string(),
                p.map(String::from),
                d.map(String::from),
                a.map(String::from),
                pc.map(String::from),
            )
        })
        .collect();
    assert_eq!(got, want, "case {case}: {markup:?}");
}

#[test]
fn test_reference_extraction_from_markup() {
    // https://www.courtlistener.com/api/rest/v4/opinions/1985850/
    let markup = r#"
        <i>citing, </i><i>U.S. v. Halper,</i> 490 <i>U.S.</i> 435, 446, 109 <i>
        S.Ct.</i> 1892, 1901, 104 <i>L.Ed.</i>2d 487 (1989).
        ; and see, <i>Bae v. Shalala,</i> 44 <i>F.</i>3d 489 (7th Cir.1995).
        <p>In <i>Bae,</i> the 7th Circuit Court further interpreted
        the holding of <i>Halper.</i> In <i>Bae,</i> the court... by the
        <i>ex post facto</i> clause of the U.S. Constitution...</p>
        <p>In <i>Bae,</i> the circuit court rejected the defendant's
        argument that since debarment served both remedial <i>and</i>
        punitive goals it must be characterized as punishment. Bae's argument
        evidently relied on the <i>Halper</i> court's use of the word "solely"
        in the discussion leading to its holding. The circuit court's
        interpretation was much more pragmatic: "A civil sanction that can
        fairly be said solely to serve remedial goals will not fail under
        <i>ex post facto</i> scrutiny simply because it is consistent with
        punitive goals as well." 44 <i>F.</i>3d at 493.</p>"#;
    let refs: Vec<String> = markup_citations(markup, &["html", "all_whitespace"])
        .iter()
        .filter(|c| matches!(c, Citation::Reference(_)))
        .map(|c| c.matched_text().trim_matches(&[',', '.'][..]).to_string())
        .collect();
    // Order and count matter; "Bae's argument" must not be picked up.
    assert_eq!(refs, ["Bae", "Halper", "Bae", "Bae", "Halper"]);
}

#[test]
fn test_reference_filtering() {
    // References that overlap other citations are dropped.
    let texts = [
        // https://www.courtlistener.com/api/rest/v4/opinions/9435339/
        "<em>Bell Atlantic Corp. </em>v. <em>Twombly, </em>550 U. S. 544 (2007),
            which discussed... apellate court’s core competency.
             <em>Twombly, </em>550 U. S., at 557. Evaluating...
            In <em>Twombly</em>, supra, at 553-554, the Court found...
            Another, in <em>Twombly, supra</em>, at 553-554, the Court found
            ",
        "
            <em>Johnson </em>v. <em>Jones, </em>515 U. S. 304, 309 (1995)
             something... with,” <em>Swint </em>v. <em>Chambers County Comm’n,
             </em>514 U. S. 35, 51 (1995), and “directly implicated by,”
             <em>Hartman, supra, </em>at 257, n. 5, the qualified-immunity
             defense.</p>\n<p id=\"b773-6\">Respondent counters that our
             holding in <em>Johnson, </em>515 U. S. 304, confirms
            ",
        // https://www.courtlistener.com/opinion/8524158/in-re-cahill/
        " was not con-firmable. <em>Nobelman v. Am. Sav. Bank, </em>
            508 U.S. 324, 113 S.Ct. 2106, 124 L.Ed.2d 228 (1993). That plan
             residence.” <em>Nobelman </em>at 332, 113 S.Ct. 2106.
             Section 1123(b)(5) codifies the
            ",
    ];
    for markup in texts {
        let cites = markup_citations(markup, &["html", "all_whitespace"]);
        assert!(
            !cites.iter().any(|c| matches!(c, Citation::Reference(_))),
            "unexpected reference in {markup:?}"
        );
    }
}

#[test]
fn test_find_citations_html() {
    // eyecite test_find_citations, html cases.
    let steps = ["html", "inline_whitespace"];
    check(
        122,
        "<p>In <i>Foo</i> v. <i>Bar,</i> 1 U.S. 1, ... yadda yadda yadda the case of <i>Foo</i> v. <i>Bar, supra</i><i>,</i> was affirmed.</p>",
        &steps,
        &[
            ("FullCaseCitation", Some("Foo"), Some("Bar"), None, None),
            ("SupraCitation", None, None, Some("Bar"), None),
        ],
    );
    check(
        123,
        "<em>Grant </em>v. <em>Esquire, Inc. </em>(S.D.N.Y. 1973) 367 F.Supp. 876, 879.) ...\n             <em>Grant </em>v. <em>Esquire, Inc., supra, </em>316 F.Supp. at 884.)</p>\n             <p id=\"b883-5\">III. </p> ",
        &steps,
        &[
            ("FullCaseCitation", Some("Grant"), Some("Esquire, Inc."), None, Some("879")),
            ("ShortCaseCitation", None, None, Some("Esquire, Inc., supra"), Some("884")),
        ],
    );
    check(
        127,
        "See <em>Bivens </em>v. <em>Six Unknown Fed. Narcotics Agents, </em>403 U. S. 388 (1971). The legal issue there was whether a <em>Bivens </em> at 122, action can be employed...",
        &steps,
        &[
            ("FullCaseCitation", Some("Bivens"), Some("Six Unknown Fed. Narcotics Agents"), None, None),
            ("ReferenceCitation", Some("Bivens"), None, None, Some("at 122")),
        ],
    );
}

#[test]
fn test_markup_plaintiff_and_antecedent_guesses() {
    check(
        0,
        "and more and more <em>Jin Fuey Moy</em><em>v. United States,</em>\n            254 U.S. 189. Petitioner contends",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Jin Fuey Moy"), Some("United States"), None, None),
        ],
    );
    check(
        1,
        "<em>Overruled</em> and so on <em>Jin Fuey Moy v. United States,</em> 254 U.S. 189. Petitioner contends",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Jin Fuey Moy"), Some("United States"), None, None),
        ],
    );
    check(
        2,
        "<em>Overruled</em> and so on <em>Jin Fuey Moy</em> <em>v. United States,</em> 254 U.S. 189. Petitioner contends",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Jin Fuey Moy"), Some("United States"), None, None),
        ],
    );
    check(
        3,
        "<em>Bell Atlantic Corp. </em>v. <em>Twombly, </em>550 U. S. 544 (2007),",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("Bell Atlantic Corp."),
            Some("Twombly"),
            None,
            None,
        )],
    );
    check(
        4,
        "con-firmable. <em>United States v. Am. Sav. Bank, </em> 508 U.S. 324 (1993). That plan proposed to bifurcate the claim and",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("United States"), Some("Am. Sav. Bank"), None, None),
        ],
    );
    check(
        5,
        ". <em>Jin Fuey Moy</em> <em>v. United States,</em> 254 U.S. 189. Petitioner contends.  Regardless in <em>Jin Fuey Moy</em> the court ruled",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Jin Fuey Moy"), Some("United States"), None, None),
            ("ReferenceCitation", Some("Jin Fuey Moy"), None, None, None),
        ],
    );
    check(
        6,
        "<p id=\"b453-6\">\n  The supreme court of Connecticut, in\n  <em>\n   Beardsley\n  </em>\n  v.\n  <em>\n   Hartford,\n  </em>\n  50 Conn. 529, 541-542, after quoting the maxim of the common law;\n  <em>\n   cessante ratione legis-, cessat ipsa lex,\n  </em>",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Beardsley"), Some("Hartford"), None, Some("541-542")),
        ],
    );
    check(
        7,
        " partially secured by a debtor’s principal residence was not con-firmable. <em>Smart Nobelman v. Am. Sav. Bank, </em>508 U.S. 324 (1993). That plan proposed to bifurcate the claim and... pay the unsecured... only by a lien on the debtor’s principal residence.” codifies the <em>Smart Nobelman </em>decision in individual debtor chapter 11 cases.",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Smart Nobelman"), Some("Am. Sav. Bank"), None, None),
            ("ReferenceCitation", Some("Smart Nobelman"), None, None, None),
        ],
    );
    check(
        8,
        "the court in <em>Smith Johnson</em>, 1 U. S., at 2",
        &["html", "all_whitespace"],
        &[(
            "ShortCaseCitation",
            None,
            None,
            Some("Smith Johnson"),
            Some("2"),
        )],
    );
    check(
        9,
        "Judge Regan (dissenting) in <i>Thrift Funds Canal,</i> Inc. v. Foy, 242 So.2d 253, 257 (La.App. 4 Cir. 1970), calls",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Thrift Funds Canal, Inc."), Some("Foy"), None, Some("257")),
        ],
    );
    check(
        10,
        "<em>\n   Smart v. Tatum,\n  </em>\n \n  541 U.S. 1085 (2004);\n  <em>\n",
        &["html", "inline_whitespace"],
        &[("FullCaseCitation", Some("Smart"), Some("Tatum"), None, None)],
    );
    check(
        11,
        "Craig v. Harrah, ___ Nev. ___ [201 P.2d 1081]. (",
        &[],
        &[(
            "FullCaseCitation",
            Some("Craig"),
            Some("Harrah"),
            None,
            None,
        )],
    );
    check(
        12,
        " <i>United States</i> v. <i>Hodgson,</i> ___ Iowa ___, 44 N.J. 151, 207 A. 2d 542;",
        &["html", "all_whitespace"],
        &[
            (
                "FullCaseCitation",
                Some("United States"),
                Some("Hodgson"),
                None,
                None,
            ),
            (
                "FullCaseCitation",
                Some("United States"),
                Some("Hodgson"),
                None,
                None,
            ),
        ],
    );
    check(
        13,
        " <i>United States ex rel. Russo v. New Jersey</i>, 351 F.2d 429 something something",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("United States ex rel. Russo"),
            Some("New Jersey"),
            None,
            None,
        )],
    );
    check(
        14,
        " partially secured by a debtor’s principal residence was not con-firmable. <em>Nobelman v. Am. Sav. Bank, </em>508 U.S. 324 (1993). That plan proposed to bifurcate the claim and... pay the unsecured... only by a lien on the debtor’s principal residence.” codifies the  a lien on the debtor’s principal residence.” <em>Nobelman </em>at 332, decision in individual debtor chapter 11 cases.",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Nobelman"), Some("Am. Sav. Bank"), None, None),
            ("ReferenceCitation", Some("Nobelman"), None, None, Some("at 332")),
        ],
    );
    check(
        15,
        "<i>See <span class=\"SpellE\">DeSantis</span> v. Wackenhut Corp.</i>, 793 S.W.2d 670;",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("DeSantis"),
            Some("Wackenhut Corp."),
            None,
            None,
        )],
    );
    check(
        16,
        "</span>§ 3.1 (2d ed. 1977), <i>Strawberry Hill</i>, 725 S.W.2d at 176 (Gonzalez, J., dissenting);",
        &["html", "all_whitespace"],
        &[
            ("UnknownCitation", None, None, None, None),
            ("ShortCaseCitation", None, None, Some("Strawberry Hill"), Some("176")),
        ],
    );
    check(
        17,
        "</span>§ 3.1 (2d ed. 1977), <i>(See Hill</i>, 725 S.W.2d at 176 (Gonzalez, J., dissenting));",
        &["html", "all_whitespace"],
        &[
            ("UnknownCitation", None, None, None, None),
            ("ShortCaseCitation", None, None, Some("Hill"), Some("176")),
        ],
    );
    check(
        18,
        "<i>United States</i> v. <i>Carignan,</i> <span class=\"star-pagination\">*528</span> 342 U. S. 36, 41;",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("United States"), Some("Carignan"), None, Some("41")),
        ],
    );
    check(
        19,
        "objection. <i>Our Lady of the Lake Hosp. v. Vanner,</i> 95-0754, p. 3 (La.App. 1 Cir. 12/15/95), 669 So.2d 463, 464;",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Our Lady of the Lake Hosp."), Some("Vanner"), None, Some("464")),
        ],
    );
    check(
        20,
        "immunity outlined by the Supreme Court in <em>Saucier, </em><page-number citation-index=\"1\" label=\"1029\">*1029</page-number>533 U.S. at 201, 121 S.Ct. 2151",
        &["html", "all_whitespace"],
        &[
            ("ShortCaseCitation", None, None, Some("Saucier"), Some("201")),
            ("FullCaseCitation", None, Some("Saucier"), None, None),
        ],
    );
    check(
        21,
        "this Court.\n  <em>\n   Holm; In Re Carroll’s Estate\n  </em>\n  (1921), 59 Mont. 403, 413.",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", None, Some("Carroll’s Estate"), None, Some("413")),
        ],
    );
    check(
        22,
        "party.”\n              <em>\n               Anderson, 477\n              </em>\n              U.S. at 248",
        &["html", "all_whitespace"],
        &[
            ("ShortCaseCitation", None, None, Some("Anderson"), Some("248")),
        ],
    );
    check(
        23,
        "and <em>In re Spellman, 4 </em>A.D.2d 215 (N.Y.App.Div.1957). <em>",
        &["html", "all_whitespace"],
        &[("FullCaseCitation", None, Some("Spellman"), None, None)],
    );
    check(
        24,
        "<em>Cf. Akins v. State, </em>104 So.3d 1173 (Fla. 1st DCA 2012)",
        &["html", "all_whitespace"],
        &[("FullCaseCitation", Some("Akins"), Some("State"), None, None)],
    );
    check(
        25,
        "<em>\n   In re Dixon,\n  </em>\n  41 Cal.2d 756 (1953).",
        &["html", "all_whitespace"],
        &[("FullCaseCitation", None, Some("Dixon"), None, None)],
    );
    check(
        26,
        "dentification. Stovall v. Denno, 388 U.S. 293, ",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("Stovall"),
            Some("Denno"),
            None,
            None,
        )],
    );
    check(
        27,
        "<em>see also Cass v. Stephens</em>,\r\n156 S.W.3d 38",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("Cass"),
            Some("Stephens"),
            None,
            None,
        )],
    );
    check(
        28,
        "the trial court.â€™â€ (quoting Ohree v. Commonwealth, </pre><span class=\"citation no-link\"><span class=\"volume\">26</span> <span class=\"reporter\">Va. App.</span> <span class=\"page\">299</span>",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Ohree"), Some("Commonwealth"), None, None),
        ],
    );
    check(
        29,
        "See id.; see also Harper v. State, </pre><span class=\"citation no-link\">286 Ga. 216</span><pre class=\"inline\">, 218\n(1) (686 SE2d 786) (2009)",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Harper"), Some("State"), None, None),
            ("FullCaseCitation", Some("Harper"), Some("State"), None, None),
        ],
    );
    check(
        30,
        "\n\nLynch v. State ex rel. Killebrew, 179 Tenn. 339",
        &["html", "all_whitespace"],
        &[(
            "FullCaseCitation",
            Some("Lynch"),
            Some("State ex rel. Killebrew"),
            None,
            None,
        )],
    );
    check(
        31,
        " <i>See </i><i>Loup-Miller Const. Co. v. City and County of Denver,</i> 676 P.2d 1170 (Colo.1984) .... <i>See </i><i>Loup-Miller,</i> 676 P.2d 1170 and so on <i>Loup-Miller</i>",
        &["html", "all_whitespace"],
        &[
            ("FullCaseCitation", Some("Loup-Miller Const. Co."), Some("City and County of Denver"), None, None),
            ("FullCaseCitation", None, Some("Loup-Miller"), None, None),
            ("ReferenceCitation", None, Some("Loup-Miller"), None, None),
        ],
    );
}
