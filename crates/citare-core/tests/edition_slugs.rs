//! Every case-reporter edition's slug (#70), and the guard that keeps a
//! released slug from ever changing.
//!
//! stare publishes its CourtListener index at `v/<ver>/<slug>/<vol>.cl`, so
//! a slug is a public path. `data/edition-slugs.json` records every edition
//! name's slugs; this test fails when any of them changes or disappears,
//! which is what a reporters-db bump that renames an edition or a
//! derivation change would do. New edition names fail it too, until they
//! are added with
//!
//! ```text
//! CITARE_UPDATE_EDITION_SLUGS=1 cargo test -p citare-core --test edition_slugs
//! ```
//!
//! which adds new names and slugs but refuses to change or drop one.

use std::collections::{BTreeMap, BTreeSet};

use citare_core::models::edition::ReporterSource;
use citare_core::{data, get_citations, Citation};
use rstest::rstest;

const SNAPSHOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/edition-slugs.json");

type Slugs = BTreeMap<String, BTreeSet<String>>;

/// Edition name → its slugs, from the loaded data. A name has more than one
/// slug only when its editions are different reporters whose volumes overlap
/// (`Ct. Cl.`: `us-ct-cl` and `wv-ct-cl`).
fn current() -> Slugs {
    let mut out = Slugs::new();
    for e in &data::get().editions {
        if e.reporter.source != ReporterSource::Reporters {
            assert_eq!(e.slug, None, "{} is not a case reporter", e.short_name);
            continue;
        }
        let slug = e
            .slug
            .clone()
            .unwrap_or_else(|| panic!("edition {:?} has no slug", e.short_name));
        out.entry(e.short_name.clone()).or_default().insert(slug);
    }
    out
}

/// One line per edition name, so a diff shows exactly which slugs moved.
fn render(slugs: &Slugs) -> String {
    let mut out = String::from("{\n");
    for (i, (name, set)) in slugs.iter().enumerate() {
        let list: Vec<String> = set
            .iter()
            .map(|s| serde_json::to_string(s).unwrap())
            .collect();
        out.push_str(&format!(
            "  {}: [{}]{}\n",
            serde_json::to_string(name).unwrap(),
            list.join(", "),
            if i + 1 < slugs.len() { "," } else { "" }
        ));
    }
    out.push_str("}\n");
    out
}

#[test]
fn released_slugs_never_change() {
    let text = std::fs::read_to_string(SNAPSHOT).unwrap_or_else(|_| "{}".to_string());
    let snapshot: Slugs = serde_json::from_str(&text).expect("edition-slugs.json is not valid");
    let now = current();

    let mut changed = Vec::new();
    let mut added = Vec::new();
    for (name, slugs) in &snapshot {
        let have = now.get(name).cloned().unwrap_or_default();
        if !slugs.is_subset(&have) {
            changed.push(format!("{name:?}: released {slugs:?}, now {have:?}"));
        }
    }
    for (name, slugs) in &now {
        let had = snapshot.get(name).cloned().unwrap_or_default();
        if !had.is_superset(slugs) {
            added.push(format!(
                "{name:?}: {:?}",
                slugs.difference(&had).collect::<Vec<_>>()
            ));
        }
    }
    assert!(
        changed.is_empty(),
        "released edition slugs changed or disappeared; they are public paths in stare's \
         index, so keep them (citare_data_build::slugs::BY_REPORTER):\n{}",
        changed.join("\n")
    );
    if added.is_empty() {
        assert_eq!(
            text,
            render(&now),
            "edition-slugs.json is not in canonical form"
        );
    } else if std::env::var_os("CITARE_UPDATE_EDITION_SLUGS").is_some() {
        let mut merged = snapshot;
        for (name, slugs) in now {
            merged.entry(name).or_default().extend(slugs);
        }
        std::fs::write(SNAPSHOT, render(&merged)).unwrap();
    } else {
        panic!(
            "{} edition names or slugs are not in data/edition-slugs.json; add them with \
             CITARE_UPDATE_EDITION_SLUGS=1:\n{}",
            added.len(),
            added.join("\n")
        );
    }
}

/// The slug of the first full case citation in `text`, picked the way the
/// CLI and WASM bindings pick it.
fn slug_of(text: &str) -> Option<String> {
    let citations = get_citations(text, false, "", &[]);
    let rc = citations
        .iter()
        .find(|c| matches!(c, Citation::FullCase(_)))
        .unwrap_or_else(|| panic!("no full case citation in {text:?}"))
        .resource_common()?;
    rc.edition_guess
        .as_ref()
        .and_then(|e| e.slug.clone())
        .or_else(|| rc.all_editions.first().and_then(|e| e.slug.clone()))
}

/// The `(slug, volume)` index path of the first full case citation in `text`.
fn path_of(text: &str) -> (Option<String>, Option<String>) {
    let citations = get_citations(text, false, "", &[]);
    let c = citations
        .iter()
        .find(|c| matches!(c, Citation::FullCase(_)))
        .unwrap_or_else(|| panic!("no full case citation in {text:?}"));
    (
        slug_of(text),
        c.common().groups.get("volume").cloned().flatten(),
    )
}

// The reporters with the most CourtListener cases that CAP never had, the
// neutral citations that share a CAP reporter's slug or take their own, and
// CAP reporters that keep their CAP slug.
#[rstest]
#[case("2021 NY Slip Op 01234", "ny-slip-op")]
#[case("2019-Ohio-1234", "ohio")]
#[case("2019 Ohio 1234", "ohio")]
#[case("1 Ohio 1", "ohio")]
#[case("T.C. Memo. 2019-123", "t-c-memo")]
#[case("100 N.Y.S.3d 200", "n-y-s-3d")]
#[case("150 A.D.3d 100", "ad3d")]
#[case("10 F.4th 100", "f-4th")]
#[case("400 P.3d 100", "p3d")]
#[case("2021 IL App (1st) 200123", "il-app-1st")]
#[case("2019 Pa. Super. 12", "pa-super")]
#[case("2019 ME 12", "me")]
#[case("2021 ND 123", "nd")]
#[case("50 N.D. 1", "nd")]
#[case("2019 VT 12", "vt")]
#[case("1999 Conn. Super. Ct. 13301-M", "conn-super-ct")]
#[case("5 Conn. Super. Ct. 123", "conn-super-ct")]
fn slug_for_courtlistener_reporters(#[case] text: &str, #[case] slug: &str) {
    assert_eq!(slug_of(text).as_deref(), Some(slug), "{text}");
}

// The Ohio neutral citation and CAP's Ohio Reports (vols 1-20) share `ohio`;
// the neutral's volumes are years, so their index paths never meet.
#[test]
fn ohio_neutral_and_ohio_reports_share_a_slug_by_volume() {
    for text in ["2019-Ohio-1234", "2002 Ohio 1", "2019-Ohio-1234, ¶ 12"] {
        let (slug, volume) = path_of(text);
        assert_eq!(slug.as_deref(), Some("ohio"), "{text}");
        let volume: u32 = volume.expect("a volume").parse().unwrap();
        assert!(volume >= 1700, "{text}: volume {volume}");
    }
    let (slug, volume) = path_of("20 Ohio 1");
    assert_eq!(
        (slug.as_deref(), volume.as_deref()),
        (Some("ohio"), Some("20"))
    );
}

// North Dakota's neutral citation shares `nd` with CAP's North Dakota
// Reports (vols 1-79) at year volumes.
#[test]
fn north_dakota_neutral_shares_nd_by_volume() {
    let (slug, volume) = path_of("2021 ND 123");
    assert_eq!(
        (slug.as_deref(), volume.as_deref()),
        (Some("nd"), Some("2021"))
    );
    let (slug, volume) = path_of("50 N.D. 1");
    assert_eq!(
        (slug.as_deref(), volume.as_deref()),
        (Some("nd"), Some("50"))
    );
}

// CAP files the Massachusetts Appellate Division Reports in two series, and
// so does citare (#75): the numbered volumes are `mass-app-div` and the
// annual ones, numbered by year, `mass-app-div-annual`.
#[test]
fn mass_app_div_series_split_by_volume() {
    for (text, slug, volume) in [
        ("10 Mass. App. Div. 12", "mass-app-div", "10"),
        ("1995 Mass. App. Div. 12", "mass-app-div-annual", "1995"),
        ("2015 Mass. App. Div. 1", "mass-app-div-annual", "2015"),
    ] {
        let (s, v) = path_of(text);
        assert_eq!((s.as_deref(), v.as_deref()), (Some(slug), Some(volume)));
        assert_eq!(harvard_slug_of(text).as_deref(), Some(slug), "{text}");
    }
}

// Five CAP reporters that had no edition of their own (#99) take CAP's slug,
// written as CAP cites their cases. The released slugs of the same names stay:
// Iowa's `Bradf.` is `bradf` and R.M. Charlton's `Charlton` is `charlton`.
// reporters-db's `Cai. R.` and `P.R.R.` are the same books as CAP's `cai` and
// `pr` and share those slugs (D78), whatever year the citation gives.
#[rstest]
#[case("2 Brad. 432", "brad", "2", Some("brad"))]
#[case("4 Bradf. 268", "brad", "4", Some("brad"))]
#[case("1 Bradf. 356", "bradf", "1", Some("bradf"))]
#[case("1 Cai. 1", "cai", "1", Some("cai"))]
#[case("1 Cai. 22a", "cai", "1", Some("cai"))]
#[case("2 Cai. R. 169", "cai", "2", Some("cai"))]
#[case("2 Cai.R. 169", "cai", "2", Some("cai"))]
#[case("1 Cai. Cas. 1", "cai-cas", "1", Some("cai-cas"))]
#[case("1 Charlton Rep. 13", "charlton-rep", "1", Some("charlton-rep"))]
#[case("1 Charlton 1", "charlton", "1", Some("charlton"))]
#[case("5 N.M. App. 1", "nm-app", "5", Some("nm-app"))]
#[case("45 P.R. 100", "pr", "45", Some("pr"))]
#[case("16 P. R. 153", "pr", "16", Some("pr"))]
#[case("45 P.R.R. 100", "pr", "45", Some("pr"))]
#[case("45 Puerto Rico 100", "pr", "45", Some("pr"))]
#[case("100 P.R.R. 1", "pr", "100", Some("pr"))]
#[case("45 P.R.R. 100 (1850)", "pr", "45", Some("pr"))]
#[case("2 Cai. R. 169 (1900)", "cai", "2", Some("cai"))]
fn cap_reporters_without_an_edition_take_caps_slug(
    #[case] text: &str,
    #[case] slug: &str,
    #[case] volume: &str,
    #[case] harvard: Option<&str>,
) {
    let (s, v) = path_of(text);
    assert_eq!(
        (s.as_deref(), v.as_deref()),
        (Some(slug), Some(volume)),
        "{text}"
    );
    assert_eq!(harvard_slug_of(text).as_deref(), harvard, "{text}");
}

// `P.R.` is also a reporters-db variation of Penrose and Watts (1829-1832):
// a year in its run picks it over the Puerto Rico Reports.
#[test]
fn a_year_outside_the_puerto_rico_reports_picks_a_variation() {
    assert_eq!(slug_of("1 P.R. 1 (1831)").as_deref(), Some("pen-w"));
    assert_eq!(slug_of("1 P.R. 1 (1900)").as_deref(), Some("pr"));
}

// The same rule on reporters-db's own data (docs/divergences.md#year-rules-out-exact):
// eyecite keeps an exact edition whatever the year says, and guesses
// Howard's Supreme Court Reports, the Illinois Reports' Gilman and, for
// `Wash.`, nothing.
#[rstest]
#[case("Smith v. Jones, 5 How. 100 (1838)", "howard")]
#[case("Smith v. Jones, 5 How. 100 (1850)", "how")]
#[case("Smith v. Jones, 1 Gilm. 1 (1820)", "gilmer")]
#[case("Smith v. Jones, 1 Gilm. 1 (1845)", "gilm")]
#[case("Smith v. Jones, 2 Wash. 473 (1856)", "wash-terr")]
fn a_year_outside_every_exact_edition_picks_a_variation(#[case] text: &str, #[case] slug: &str) {
    assert_eq!(slug_of(text).as_deref(), Some(slug), "{text}");
}

/// The `harvard_slug` of the first full case citation in `text`, picked the
/// way the CLI and WASM bindings pick it.
fn harvard_slug_of(text: &str) -> Option<String> {
    let citations = get_citations(text, false, "", &[]);
    let rc = citations
        .iter()
        .find(|c| matches!(c, Citation::FullCase(_)))
        .unwrap_or_else(|| panic!("no full case citation in {text:?}"))
        .resource_common()?;
    rc.edition_guess
        .as_ref()
        .or(rc.all_editions.first())
        .and_then(|e| e.harvard_slug.clone())
}

// `harvard_slug` names the CAP reporter a citation is in (#72): none for a
// year-volume neutral citation that shares a numbered CAP reporter's slug,
// the federal Court of Claims' own for `Ct. Cl.`, and none for a reporter
// split off a name CAP uses for another.
#[rstest]
#[case("2019-Ohio-1234", None)]
#[case("20 Ohio 1", Some("ohio"))]
#[case("2019 Pa. Super. 12", None)]
#[case("50 Pa. Super. 1", Some("pa-super"))]
#[case("2017 Ark. 173", None)]
#[case("2014 Ark. App. 442", None)]
#[case("2024 N.H. 12", None)]
#[case("82 Ct. Cl. 1 (1935)", Some("us-ct-cl"))]
#[case("20 Ct. Cl. 59 (1994)", Some("wv-ct-cl"))]
#[case("172 Wash. 688", Some("wash"))]
#[case("1 Day. 1", Some("day"))]
fn harvard_slug_is_the_cap_reporter_cited(#[case] text: &str, #[case] cap: Option<&str>) {
    assert_eq!(harvard_slug_of(text).as_deref(), cap, "{text}");
}

// A citation citare can't narrow to one edition takes the slug of the
// reporter that keeps the name's plain slug, not the one split off it.
#[rstest]
#[case("172 Wash. 688", "wash")]
#[case("14 Wall. 442", "wall")]
#[case("2 Met. 140", "met")]
#[case("4 Sneed, 415", "sneed")]
#[case("100 Ct. Cl. 123", "us-ct-cl")]
fn ambiguous_citation_takes_the_plain_slug(#[case] text: &str, #[case] slug: &str) {
    assert_eq!(slug_of(text).as_deref(), Some(slug), "{text}");
}

#[test]
fn harvard_slug_is_the_slug_or_none() {
    for e in &data::get().editions {
        if let Some(h) = &e.harvard_slug {
            assert_eq!(
                e.slug.as_ref(),
                Some(h),
                "{} ({})",
                e.short_name,
                e.reporter.name
            );
        }
    }
}

#[test]
fn every_slug_is_well_formed() {
    for (name, slugs) in current() {
        for slug in slugs {
            assert!(
                slug.chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                    && slug
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{name:?}: {slug:?}"
            );
        }
    }
}
