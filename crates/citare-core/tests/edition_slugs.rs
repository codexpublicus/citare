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
/// slug only when its editions are different reporters (CAP's Ohio Reports,
/// `ohio`, and the Ohio neutral citation, `rdb-ohio`).
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
         index, so keep them (citare_data_build::slugs::OVERRIDES):\n{}",
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

// The reporters with the most CourtListener cases that CAP never had, plus
// two CAP reporters that keep their CAP slug.
#[rstest]
#[case("2021 NY Slip Op 01234", "rdb-ny-slip-op")]
#[case("2019-Ohio-1234", "rdb-ohio")]
#[case("2019 Ohio 1234", "rdb-ohio")]
#[case("T.C. Memo. 2019-123", "rdb-t-c-memo")]
#[case("100 N.Y.S.3d 200", "rdb-n-y-s-3d")]
#[case("150 A.D.3d 100", "ad3d")]
#[case("10 F.4th 100", "rdb-f-4th")]
#[case("400 P.3d 100", "p3d")]
#[case("2021 IL App (1st) 200123", "rdb-il-app-1st")]
#[case("1 Ohio 1", "ohio")]
fn slug_for_courtlistener_reporters(#[case] text: &str, #[case] slug: &str) {
    assert_eq!(slug_of(text).as_deref(), Some(slug), "{text}");
}

// The Ohio neutral citation shares its edition name with CAP's Ohio Reports
// (vols 1-20) but must never land in CAP's `ohio`.
#[test]
fn ohio_neutral_never_gets_caps_ohio() {
    for text in ["2019-Ohio-1234", "2002 Ohio 1", "2019-Ohio-1234, ¶ 12"] {
        let citations = get_citations(text, false, "", &[]);
        for c in &citations {
            let Some(rc) = c.resource_common() else {
                continue;
            };
            for e in rc.all_editions.iter().chain(rc.edition_guess.as_ref()) {
                assert_ne!(e.slug.as_deref(), Some("ohio"), "{text}: {e:?}");
            }
        }
        assert_eq!(slug_of(text).as_deref(), Some("rdb-ohio"), "{text}");
    }
}

#[test]
fn every_derived_slug_is_unique_to_its_name() {
    let mut owner: BTreeMap<String, String> = BTreeMap::new();
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
            if slug.starts_with("rdb-") {
                if let Some(other) = owner.insert(slug.clone(), name.clone()) {
                    panic!("{other:?} and {name:?} both derive {slug:?}");
                }
            }
        }
    }
}
