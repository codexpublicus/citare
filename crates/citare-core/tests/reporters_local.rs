//! citare's supplement to reporters-db's reporters (`data/reporters-local.json`,
//! docs/reporters-local.md): every example of an edition it changes, a
//! reporter it adds or a variation it adds is recognized as that edition with
//! its volume, and every reject is not that edition.

use citare_core::{get_citations, Citation, Edition};

/// The editions (exact and guessed) and volume of every full case or journal
/// citation in `text`.
fn full_cites(text: &str) -> Vec<(Vec<Edition>, Option<String>)> {
    get_citations(text, false, "", &[])
        .iter()
        .filter(|c| matches!(c, Citation::FullCase(_) | Citation::FullJournal(_)))
        .map(|c| {
            let rc = c.resource_common().expect("a full case citation");
            let mut eds = rc.all_editions.clone();
            eds.extend(rc.edition_guess.clone());
            (eds, c.common().groups.get("volume").cloned().flatten())
        })
        .collect()
}

#[test]
fn every_supplement_example_and_reject_holds() {
    let local: serde_json::Value =
        serde_json::from_str(include_str!("../../../data/reporters-local.json")).unwrap();
    let mut n = 0;
    let entries = local["editions"]
        .as_array()
        .unwrap()
        .iter()
        .chain(local["reporters"].as_array().into_iter().flatten())
        .chain(local["variations"].as_array().into_iter().flatten())
        .chain(local["journals"].as_array().into_iter().flatten());
    for entry in entries {
        let (name, edition) = (
            entry["name"].as_str().unwrap(),
            entry["edition"].as_str().unwrap(),
        );
        let is_it = |e: &Edition| e.short_name == edition && e.reporter.name == name;
        let examples = entry["examples"].as_array().expect("examples");
        assert!(!examples.is_empty(), "{name}: no examples");
        for ex in examples {
            let (text, volume) = (ex["cite"].as_str().unwrap(), ex["volume"].as_str().unwrap());
            let found = full_cites(text);
            assert!(
                found
                    .iter()
                    .any(|(eds, v)| eds.iter().any(is_it) && v.as_deref() == Some(volume)),
                "{text:?} is not {edition} of {name} with volume {volume}: {found:?}"
            );
            n += 1;
        }
        for r in entry["rejects"].as_array().into_iter().flatten() {
            let text = r.as_str().unwrap();
            assert!(
                !full_cites(text)
                    .iter()
                    .any(|(eds, _)| eds.iter().any(is_it)),
                "{text:?} is still {edition} of {name}"
            );
            n += 1;
        }
    }
    assert!(n >= 27, "only {n} examples and rejects");
}

/// `127 Ohio 545` is the Ohio State Reports cited as `Ohio` (#80): its
/// edition is a variation match, so its reporter is still the text's `Ohio`,
/// its corrected reporter is `Ohio St.`, and its slug is `ohio-st`. A volume
/// the Ohio Reports have keeps their slug.
#[test]
fn ohio_volume_picks_the_ohio_reports_or_the_ohio_state_reports() {
    let one = |text: &str| {
        let cites = get_citations(text, false, "", &[]);
        assert_eq!(cites.len(), 1, "{text:?}: {cites:?}");
        let rc = cites[0]
            .resource_common()
            .expect("a full case citation")
            .clone();
        let guess = rc.edition_guess.clone().expect("one edition");
        (rc, guess)
    };

    let (rc, guess) = one("Troop A Riding Academy v. Miller (1934), 127 Ohio 545.");
    assert_eq!(guess.short_name, "Ohio St.");
    assert_eq!(guess.slug.as_deref(), Some("ohio-st"));
    assert!(rc.exact_editions.is_empty());
    assert_eq!(rc.variation_editions.len(), 1);

    let (rc, guess) = one("Kirk v. State, 14 Ohio 511");
    assert_eq!(guess.short_name, "Ohio");
    assert_eq!(guess.slug.as_deref(), Some("ohio"));
    assert_eq!(rc.exact_editions.len(), 1);
}

/// The New Mexico Appellate Reports take any volume, since the series is
/// still published (D80, docs/volume-caps.md). `N.M. App.` with a year before
/// their 2011 start is the New Mexico Reports with the court in the reporter
/// (`84 N.M.App. 83`, CAP nm/84), a variation match; with a later year or
/// none it is the Appellate Reports, and never ambiguous, so
/// `remove_ambiguous` drops neither.
#[test]
fn nm_app_is_the_new_mexico_reports_only_before_2011() {
    let guess = |text: &str| {
        for remove_ambiguous in [false, true] {
            let cites = get_citations(text, remove_ambiguous, "", &[]);
            assert!(
                cites.iter().any(|c| c.matched_text().contains("N.M")),
                "{text:?} (remove_ambiguous {remove_ambiguous}): {cites:?}"
            );
        }
        let cites = get_citations(text, false, "", &[]);
        let c = cites
            .iter()
            .find(|c| c.matched_text().contains("N.M"))
            .unwrap();
        assert!(!c.is_ambiguous(), "{text:?}");
        let rc = c.resource_common().expect("a full case citation");
        let e = rc.edition_guess.clone().expect("one edition");
        (e.short_name, e.slug)
    };
    let nm = ("N.M.".to_string(), Some("nm".to_string()));
    let nm_app = ("N.M. App.".to_string(), Some("nm-app".to_string()));
    assert_eq!(
        guess("State v. Doe, 84 N.M.App. 83, 500 P.2d 1 (1972)."),
        nm
    );
    assert_eq!(guess("State v. Doe, 84 N.M. App. 83 (Ct. App. 2010)."), nm);
    assert_eq!(guess("State v. Doe, 12 N.M. App. 40 (2011)."), nm_app);
    assert_eq!(guess("State v. Doe, 31 N.M. App. 40 (2024)."), nm_app);
    assert_eq!(guess("State v. Doe, 84 N.M.App. 83."), nm_app);
}
