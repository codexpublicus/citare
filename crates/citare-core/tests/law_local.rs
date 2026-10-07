//! Statute forms from citare's supplement to reporters-db's laws data
//! (`data/laws-local.json`, docs/laws-local.md), and the split of codes
//! cited by subject (#49). Each recognized form is a full law citation with
//! its jurisdiction, code and section in fields of their own
//! (`Citation::law`).

use citare_core::models::LawFields;
use citare_core::{get_citations, Citation};
use rstest::rstest;

/// The one full law citation in `text`.
fn law(text: &str) -> Citation {
    let citations = get_citations(text, false, "", &[]);
    let laws: Vec<_> = citations
        .into_iter()
        .filter(|c| matches!(c, Citation::FullLaw(_)))
        .collect();
    assert_eq!(laws.len(), 1, "expected one law citation in {text:?}");
    laws.into_iter().next().unwrap()
}

fn fields(text: &str) -> LawFields {
    let c = law(text);
    let f = c.law().expect("law fields");
    assert_eq!(
        c.corrected_reporter().as_deref(),
        Some(f.code.as_str()),
        "the normalized reporter is the code"
    );
    f
}

/// Every form the supplement claims gives a law citation with a
/// jurisdiction, a code and a section.
#[test]
fn every_supplement_example_is_recognized() {
    let local: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/laws-local.json")).unwrap();
    let mut n = 0;
    for (key, entries) in local["laws"].as_object().unwrap() {
        for entry in entries.as_array().unwrap() {
            let examples = entry["examples"].as_array().expect("examples");
            assert!(!examples.is_empty(), "{key}: no examples");
            for ex in examples {
                let text = ex.as_str().unwrap();
                let c = law(text);
                let guess = c
                    .resource_common()
                    .and_then(|r| r.edition_guess.clone())
                    .unwrap_or_else(|| panic!("{text:?}: no edition guess"));
                assert_eq!(&guess.short_name, key, "{text:?}");
                let f = c.law().unwrap();
                assert!(f.jurisdiction.is_some(), "{text:?}: no jurisdiction");
                assert!(f.section.is_some(), "{text:?}: no section");
                n += 1;
            }
        }
    }
    assert!(n > 50, "only {n} examples");
}

#[rstest]
// Connecticut (the Brooks v. Powers forms)
#[case(
    "General Statutes § 52-557n",
    "Connecticut",
    "Conn. Gen. Stat.",
    None,
    "52-557n"
)]
#[case(
    "in violation of General Statutes (Rev. to 2013) § 53a-54a (a)",
    "Connecticut",
    "Conn. Gen. Stat.",
    None,
    "53a-54a"
)]
#[case(
    "Conn. Gen. Stat. § 52-557n",
    "Connecticut",
    "Conn. Gen. Stat.",
    None,
    "52-557n"
)]
#[case(
    "C.G.S.A. § 31-294c",
    "Connecticut",
    "Conn. Gen. Stat.",
    None,
    "31-294c"
)]
#[case(
    "Practice Book § 17-49",
    "Connecticut",
    "Conn. Practice Book",
    None,
    "17-49"
)]
// Georgia, Arizona, Hawaii, North Carolina, North Dakota, Tennessee
#[case("OCGA § 16-9-93 (b) (2)", "Georgia", "Ga. Code Ann.", None, "16-9-93")]
#[case(
    "O.C.G.A. § 51-12-5.1(b)",
    "Georgia",
    "Ga. Code Ann.",
    None,
    "51-12-5.1"
)]
#[case("A.R.S. § 13-604.01", "Arizona", "Ariz. Rev. Stat.", None, "13-604.01")]
#[case("HRS § 707-720(3)", "Hawaii", "Haw. Rev. Stat.", None, "707-720")]
#[case(
    "N.C.G.S. § 143-291",
    "North Carolina",
    "N.C. Gen. Stat.",
    None,
    "143-291"
)]
#[case("G.S. § 14-17", "North Carolina", "N.C. Gen. Stat.", None, "14-17")]
#[case(
    "N.D.C.C. § 14-05-24",
    "North Dakota",
    "N.D. Cent. Code",
    None,
    "14-05-24"
)]
#[case(
    "T.C.A. § 49-2-303(a)",
    "Tennessee",
    "Tenn. Code Ann.",
    None,
    "49-2-303"
)]
// Codes cited by title
#[case(
    "42 Pa.C.S. § 5322(a)(4)",
    "Pennsylvania",
    "Pa. Cons. Stat.",
    Some("42"),
    "5322"
)]
#[case(
    "42 Pa.C.S.A. § 7102(c)",
    "Pennsylvania",
    "Pa. Cons. Stat.",
    Some("42"),
    "7102"
)]
#[case(
    "40 P.S. § 532.7",
    "Pennsylvania",
    "Pa. Stat. Ann.",
    Some("40"),
    "532.7"
)]
#[case(
    "19-A M.R.S. § 1653(6)",
    "Maine",
    "Me. Rev. Stat. Ann.",
    Some("19-A"),
    "1653"
)]
#[case(
    "15 M.R.S.A., § 393",
    "Maine",
    "Me. Rev. Stat. Ann.",
    Some("15"),
    "393"
)]
#[case("19 V.S.A. § 230", "Vermont", "Vt. Stat. Ann.", Some("19"), "230")]
#[case(
    "12 O.S.2011 § 1141.5",
    "Oklahoma",
    "Okla. Stat.",
    Some("12"),
    "1141.5"
)]
#[case("22 O.S.2001, § 1089", "Oklahoma", "Okla. Stat.", Some("22"), "1089")]
// Codes cited by subject
#[case(
    "N.Y. Gen. Bus. Law § 394",
    "New York",
    "N.Y. Gen. Bus. Law",
    None,
    "394"
)]
#[case("Penal Law § 160.05", "New York", "N.Y. Penal Law", None, "160.05")]
#[case(
    "Vehicle and Traffic Law § 1192 (3)",
    "New York",
    "N.Y. Vehicle and Traffic Law",
    None,
    "1192"
)]
#[case(
    "Family Ct Act § 1046 [a] [ii]",
    "New York",
    "N.Y. Family Ct Act",
    None,
    "1046"
)]
#[case("N.Y. C.P.L.R. § 5501", "New York", "N.Y. C.P.L.R.", None, "5501")]
#[case("(Pen. Code, § 211)", "California", "Cal. Pen. Code", None, "211")]
#[case(
    "(Code Civ. Proc., § 472c.)",
    "California",
    "Cal. Civ. Proc. Code",
    None,
    "472c"
)]
#[case("Cal. Civ. Code § 1714", "California", "Cal. Civ. Code", None, "1714")]
#[case(
    "TEX. PENAL CODE ANN. § 1.03(a)",
    "Texas",
    "Tex. Penal Code Ann.",
    None,
    "1.03"
)]
#[case(
    "TEX. CIV. PRAC. & REM. CODE ANN. § 16.026(a)",
    "Texas",
    "Tex. Civ. Prac. & Rem. Code Ann.",
    None,
    "16.026"
)]
#[case(
    "Tex. Prop.Code Ann. § 204.010(a)(16)",
    "Texas",
    "Tex. Prop. Code Ann.",
    None,
    "204.010"
)]
#[case(
    "Md. Code Ann., Cts. & Jud. Proc. § 5-101",
    "Maryland",
    "Md. Code Ann., Cts. & Jud. Proc.",
    None,
    "5-101"
)]
// Others
#[case("G. L. c. 184, § 15", "Massachusetts", "Mass. Gen. Laws", None, "15")]
#[case(
    "NMSA 1978, § 32A-4-28",
    "New Mexico",
    "N.M. Stat. Ann.",
    None,
    "32A-4-28"
)]
#[case("MCL § 500.3101", "Michigan", "Mich. Comp. Laws", None, "500.3101")]
#[case(
    "Ala. Code 1975, § 12-15-65(m)",
    "Alabama",
    "Ala. Code",
    None,
    "12-15-65"
)]
#[case("A.C.A. § 4-9-309", "Arkansas", "Ark. Code Ann.", None, "4-9-309")]
#[case(
    "La.Rev.Stat. § 13:841.3",
    "Louisiana",
    "La. Stat. Ann.",
    None,
    "13:841.3"
)]
#[case(
    "G.L. 1956 § 11-23-1",
    "Rhode Island",
    "R.I. Gen. Laws",
    None,
    "11-23-1"
)]
#[case(
    "West Virginia Code § 23-4-2(c)(2)",
    "West Virginia",
    "W. Va. Code",
    None,
    "23-4-2"
)]
#[case("WIS. STAT. § 847.07", "Wisconsin", "Wis. Stat.", None, "847.07")]
// reporters-db forms keep working
#[case("42 U.S.C. § 1983", "United States", "U.S.C.", Some("42"), "1983")]
#[case(
    "Mass. Gen. Laws ch. 93A, § 2",
    "Massachusetts",
    "Mass. Gen. Laws",
    None,
    "2"
)]
fn statute_fields(
    #[case] text: &str,
    #[case] jurisdiction: &str,
    #[case] code: &str,
    #[case] title: Option<&str>,
    #[case] section: &str,
) {
    let f = fields(text);
    assert_eq!(f.jurisdiction.as_deref(), Some(jurisdiction), "{text:?}");
    assert_eq!(f.code, code, "{text:?}");
    assert_eq!(f.title.as_deref(), title, "{text:?}");
    assert_eq!(f.section.as_deref(), Some(section), "{text:?}");
}

#[test]
fn chapter_is_its_own_field() {
    let f = fields("G.L.c. 94C, § 32E (a)(2)");
    assert_eq!(f.chapter.as_deref(), Some("94C"));
    assert_eq!(f.section.as_deref(), Some("32E"));
}

/// #49: two reporters-db entries match `N.Y. Gen. Bus. Law § 394`, `N.Y.`
/// (subject `Gen. Bus. Law`) and `N.Y. Law` (subject `Gen. Bus.`). The more
/// specific `N.Y. Law` wins, the same split as for `Cal. Civ. Code` and
/// `Tex. Penal Code Ann.`, and the code keeps its name.
#[test]
fn new_york_code_split() {
    let c = law("See N.Y. Gen. Bus. Law § 394 (2020).");
    let g = &c.common().groups;
    assert_eq!(g["reporter"].as_deref(), Some("N.Y. Gen. Bus. Law"));
    assert_eq!(g["subject"].as_deref(), Some("Gen. Bus."));
    assert_eq!(
        c.corrected_reporter().as_deref(),
        Some("N.Y. Gen. Bus. Law")
    );
    let f = c.law().unwrap();
    assert_eq!(f.subject.as_deref(), Some("Gen. Bus."));
    // Not New York Reports' CAP slug, which reporters-db's `N.Y.` key used
    // to pick up.
    let r = c.resource_common().unwrap();
    assert!(r.all_editions.iter().all(|e| e.harvard_slug.is_none()));
}

/// Look-alikes that must not become one of the supplement's codes.
#[rstest]
#[case("Tex. Penal Code § 22.01 applies.")] // Texas, not Cal. `Pen. Code, §`
#[case("Family Law § 9-101 governs.")] // Maryland article, not a N.Y. law
#[case("Criminal Law § 3-202 applies.")]
#[case("3 Sedgwick, Damages, 9th Ed., §§ 984, 991.")]
#[case("under § 341 of the Code")]
fn not_a_supplement_code(#[case] text: &str) {
    for c in get_citations(text, false, "", &[]) {
        if let Some(f) = c.law() {
            assert!(
                !["California", "New York"].contains(&f.jurisdiction.as_deref().unwrap_or("")),
                "{text:?} became {f:?}"
            );
        }
    }
}
