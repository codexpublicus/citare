//! Harvard Caselaw Access Project (CAP) reporter slugs.
//!
//! CAP names its reporters with its own short names, which match a
//! reporters-db edition name most of the time but are sometimes one of that
//! edition's variations instead: CAP's "So.2d" and "N.Y. 2d" are
//! reporters-db's "So. 2d" and "N.Y.2d". Slugs are keyed here by
//! reporters-db edition name, so every spelling the tokenizer finds for an
//! edition reaches the same slug through the edition.

use std::collections::{HashMap, HashSet};

use crate::reporters::ReportersMap;

/// Parse `harvard_reporters.json` into CAP short_name -> slug.
pub fn parse_harvard(json_str: &str) -> Result<HashMap<String, String>, serde_json::Error> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(json_str)?;
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let short_name = entry.get("short_name")?.as_str()?.to_string();
            let slug = entry.get("slug")?.as_str()?.to_string();
            Some((short_name, slug))
        })
        .collect())
}

/// Re-key CAP slugs by reporters-db edition name.
///
/// A CAP short name that is an edition name keeps its key. One that is a
/// variation is moved to the edition the variation points to, provided the
/// variation points to exactly one edition and that edition has no CAP entry
/// of its own; an ambiguous variation (CAP's "P.R." is a variation of
/// Pacific Reporter as well as two other editions) is dropped rather than
/// guessed. Short names that are neither are kept as-is, for laws and
/// journals, which are looked up by their own keys.
pub fn slugs_by_edition(
    cap: &HashMap<String, String>,
    reporters: &ReportersMap,
) -> HashMap<String, String> {
    let mut editions: HashSet<&str> = HashSet::new();
    let mut variations: HashMap<&str, HashSet<&str>> = HashMap::new();
    for cluster in reporters.values().flatten() {
        editions.extend(cluster.editions.keys().map(String::as_str));
        for (variation, edition) in &cluster.variations {
            variations
                .entry(variation.as_str())
                .or_default()
                .insert(edition.as_str());
        }
    }

    let mut out: HashMap<String, String> = HashMap::new();
    for (name, slug) in cap {
        if editions.contains(name.as_str()) {
            out.insert(name.clone(), slug.clone());
        }
    }
    // Sorted so that two CAP names resolving to one edition pick a winner
    // deterministically.
    let mut names: Vec<&String> = cap.keys().collect();
    names.sort();
    for name in names {
        if editions.contains(name.as_str()) {
            continue;
        }
        match variations.get(name.as_str()) {
            Some(targets) if targets.len() == 1 => {
                let edition = *targets.iter().next().expect("one target");
                out.entry(edition.to_string())
                    .or_insert_with(|| cap[name].clone());
            }
            Some(_) => {}
            None => {
                out.insert(name.clone(), cap[name].clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reporters::parse_reporters;

    fn real() -> HashMap<String, String> {
        let cap = parse_harvard(include_str!("../../../data/harvard_reporters.json")).unwrap();
        let reporters = parse_reporters(include_str!(
            "../../../data/reporters-db/reporters_db/data/reporters.json"
        ))
        .unwrap();
        slugs_by_edition(&cap, &reporters)
    }

    #[test]
    fn edition_names_keep_their_slug() {
        let m = real();
        assert_eq!(m.get("U.S.").map(String::as_str), Some("us"));
        assert_eq!(m.get("F.2d").map(String::as_str), Some("f2d"));
        assert_eq!(m.get("S. Ct.").map(String::as_str), Some("s-ct"));
    }

    #[test]
    fn variation_names_move_to_their_edition() {
        let m = real();
        assert_eq!(m.get("So. 2d").map(String::as_str), Some("so2d"));
        assert_eq!(m.get("So. 3d").map(String::as_str), Some("so3d"));
        assert_eq!(m.get("N.Y.2d").map(String::as_str), Some("ny-2d"));
        assert_eq!(m.get("Misc. 2d").map(String::as_str), Some("misc2d"));
        assert_eq!(m.get("Misc. 3d").map(String::as_str), Some("misc3d"));
    }

    #[test]
    fn ambiguous_variation_is_dropped() {
        let m = real();
        assert!(!m.values().any(|s| s == "pr"), "P.R. is ambiguous");
        assert_eq!(m.get("P."), None);
    }

    #[test]
    fn edition_entry_wins_over_variation_entry() {
        let reporters = parse_reporters(
            r#"{"X.": [{"name": "X", "cite_type": "state",
                "editions": {"X.": {"start": null, "end": null}},
                "variations": {"X": "X."}}]}"#,
        )
        .unwrap();
        let cap = HashMap::from([
            ("X.".to_string(), "x".to_string()),
            ("X".to_string(), "x-var".to_string()),
        ]);
        let m = slugs_by_edition(&cap, &reporters);
        assert_eq!(m.get("X.").map(String::as_str), Some("x"));
        assert_eq!(m.get("X"), None);
    }
}
