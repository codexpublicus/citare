//! citare's supplement to reporters-db's reporters (`data/reporters-local.json`,
//! docs/reporters-local.md): every example of an edition it changes or a
//! reporter it adds is recognized as that edition with its volume, and every
//! reject is not that edition.

use citare_core::{get_citations, Citation, Edition};

/// The editions (exact and guessed) and volume of every full case citation
/// in `text`.
fn full_cites(text: &str) -> Vec<(Vec<Edition>, Option<String>)> {
    get_citations(text, false, "", &[])
        .iter()
        .filter(|c| matches!(c, Citation::FullCase(_)))
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
        .chain(local["reporters"].as_array().into_iter().flatten());
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
    assert!(n >= 18, "only {n} examples and rejects");
}
