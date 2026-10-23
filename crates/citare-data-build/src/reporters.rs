//! Serde structs and parse functions for reporters-db JSON files.
//!
//! Data files:
//! - `reporters.json` — reporter clusters, each with `editions` (HashMap) and
//!   `variations` (HashMap<String, String>)
//! - `laws.json` — law clusters, flat structure with `variations: Vec<String>`
//! - `journals.json` — same schema as laws

use std::collections::HashMap;

use serde::Deserialize;

// ---------------------------------------------------------------------------
// reporters.json
// ---------------------------------------------------------------------------

/// One cluster entry in `reporters.json`.  The outer JSON is
/// `HashMap<String, Vec<ReporterCluster>>` (reporter short-name → list of clusters).
#[derive(Debug, Clone, Deserialize)]
pub struct ReporterCluster {
    pub name: String,
    pub cite_type: String,
    pub editions: HashMap<String, EditionData>,
    #[serde(default)]
    pub variations: HashMap<String, String>,
    #[serde(default)]
    pub mlz_jurisdiction: Vec<String>,
    pub cite_format: Option<String>,
    /// Absorb unknown fields (publisher, examples, notes, href, …)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Per-edition metadata inside `reporters.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct EditionData {
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub regexes: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// laws.json
// ---------------------------------------------------------------------------

/// One cluster entry in `laws.json`.  The outer JSON is
/// `HashMap<String, Vec<LawCluster>>`.
///
/// Note: `variations` is a `Vec<String>` here (not a HashMap like reporters).
#[derive(Debug, Clone, Deserialize)]
pub struct LawCluster {
    pub name: String,
    pub cite_type: String,
    #[serde(default)]
    pub regexes: Vec<String>,
    #[serde(default)]
    pub jurisdiction: String,
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub variations: Vec<String>,
    /// Prefilter strings for regexes that don't contain the reporter's name
    /// (citare's supplement only; reporters-db has no such field). Without
    /// them such a regex runs on every document.
    #[serde(default)]
    pub strings: Vec<String>,
    /// Absorb unknown fields (examples, href, notes, …)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// journals.json — same schema as laws.json
// ---------------------------------------------------------------------------

/// One cluster entry in `journals.json`.
pub type JournalCluster = LawCluster;

// ---------------------------------------------------------------------------
// Convenience type aliases for the top-level maps
// ---------------------------------------------------------------------------

pub type ReportersMap = HashMap<String, Vec<ReporterCluster>>;
pub type LawsMap = HashMap<String, Vec<LawCluster>>;
pub type JournalsMap = HashMap<String, Vec<JournalCluster>>;

// ---------------------------------------------------------------------------
// Parse functions
// ---------------------------------------------------------------------------

/// Parse reporters.json into a map of reporter key → list of clusters.
pub fn parse_reporters(json_str: &str) -> Result<ReportersMap, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Parse laws.json into a map of law key → list of clusters.
pub fn parse_laws(json_str: &str) -> Result<LawsMap, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// One law in citare's own supplement to laws.json, `data/laws-local.json`:
/// the same fields, all optional for a key reporters-db already has.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalLaw {
    pub name: Option<String>,
    pub cite_type: Option<String>,
    pub jurisdiction: Option<String>,
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub regexes: Vec<String>,
    #[serde(default)]
    pub variations: Vec<String>,
    #[serde(default)]
    pub strings: Vec<String>,
    /// Citations this entry must recognize; citare's tests check each.
    #[serde(default)]
    pub examples: Vec<String>,
    /// Why the entry exists: the forms it covers and where they were seen.
    pub notes: String,
}

/// The supplement file: a comment and the laws, keyed like laws.json.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalLaws {
    #[serde(rename = "#")]
    pub comment: Vec<String>,
    pub laws: std::collections::BTreeMap<String, Vec<LocalLaw>>,
}

/// Parse `data/laws-local.json`.
pub fn parse_local_laws(json_str: &str) -> Result<LocalLaws, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Add the supplement's laws to `laws`, each as one more cluster under its
/// key. A law under a key reporters-db has takes that key's first cluster's
/// name, cite type, jurisdiction and dates, so it is the same edition, and
/// its regexes if it gives none (a variations-only entry). A new key must
/// give a name, cite type, jurisdiction and regexes.
pub fn merge_local_laws(laws: &mut LawsMap, local: &LocalLaws) -> Result<(), String> {
    for (key, entries) in &local.laws {
        let base = laws.get(key).and_then(|c| c.first()).cloned();
        for l in entries {
            let need = |field: &Option<String>, inherited: Option<String>, what: &str| {
                field
                    .clone()
                    .or(inherited)
                    .ok_or_else(|| format!("laws-local.json: new law {key:?} needs a {what}"))
            };
            let regexes = if !l.regexes.is_empty() {
                l.regexes.clone()
            } else if let Some(b) = &base {
                b.regexes.clone()
            } else {
                return Err(format!("laws-local.json: new law {key:?} needs regexes"));
            };
            let cluster = LawCluster {
                name: need(&l.name, base.as_ref().map(|b| b.name.clone()), "name")?,
                cite_type: need(
                    &l.cite_type,
                    base.as_ref().map(|b| b.cite_type.clone()),
                    "cite_type",
                )?,
                regexes,
                jurisdiction: need(
                    &l.jurisdiction,
                    base.as_ref().map(|b| b.jurisdiction.clone()),
                    "jurisdiction",
                )?,
                start: l
                    .start
                    .clone()
                    .or(base.as_ref().and_then(|b| b.start.clone())),
                end: l.end.clone().or(base.as_ref().and_then(|b| b.end.clone())),
                variations: l.variations.clone(),
                strings: l.strings.clone(),
                extra: HashMap::new(),
            };
            laws.entry(key.clone()).or_default().push(cluster);
        }
    }
    Ok(())
}

/// One edition in citare's own supplement to reporters.json,
/// `data/reporters-local.json`, whose regexes replace reporters-db's. The
/// edition is named by its reporters.json key, its cluster's `name`, and its
/// edition name, since one key can hold several reporters.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalEdition {
    pub reporter: String,
    pub name: String,
    pub edition: String,
    pub regexes: Vec<String>,
    /// A start date that replaces reporters-db's, given only to correct it.
    #[serde(default)]
    pub start: Option<String>,
    /// An end date that replaces reporters-db's, given only to correct it.
    #[serde(default)]
    pub end: Option<String>,
    /// Citations the entry must recognize, with the volume each must have;
    /// citare's tests check each.
    #[serde(default)]
    pub examples: Vec<LocalExample>,
    /// Citations the entry must no longer recognize as this edition.
    #[serde(default)]
    pub rejects: Vec<String>,
    /// Why the entry exists.
    pub notes: String,
}

/// A citation a [`LocalEdition`] must recognize, and its volume.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalExample {
    pub cite: String,
    pub volume: String,
}

/// A reporter in `data/reporters-local.json` that reporters-db doesn't have,
/// added under reporters.json key `reporter` as a cluster of its own with one
/// edition. The cluster's `name` must not be one reporters-db already has
/// under that key.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalReporter {
    pub reporter: String,
    pub name: String,
    pub cite_type: String,
    pub edition: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub regexes: Vec<String>,
    /// As [`LocalEdition::examples`].
    #[serde(default)]
    pub examples: Vec<LocalExample>,
    /// As [`LocalEdition::rejects`].
    #[serde(default)]
    pub rejects: Vec<String>,
    /// Why the entry exists.
    pub notes: String,
}

/// A variation in `data/reporters-local.json`: another abbreviation for an
/// edition, named as in [`LocalEdition`], that matches only through its own
/// regexes. reporters-db's variations take their edition's regexes, so they
/// can't be limited to some volumes; `127 Ohio 545` is the Ohio State
/// Reports only because the Ohio Reports have no volume 127.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalVariation {
    pub reporter: String,
    pub name: String,
    pub edition: String,
    pub variation: String,
    pub regexes: Vec<String>,
    /// As [`LocalEdition::examples`].
    #[serde(default)]
    pub examples: Vec<LocalExample>,
    /// As [`LocalEdition::rejects`].
    #[serde(default)]
    pub rejects: Vec<String>,
    /// Why the entry exists.
    pub notes: String,
}

/// A secondary source in `data/reporters-local.json` that reporters-db's
/// journals.json doesn't have, such as an encyclopedia, added to the
/// journals under key `edition`. A citation that matches it is a journal
/// citation, even when the same abbreviation is also a reporter's.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalJournal {
    pub edition: String,
    pub name: String,
    pub cite_type: String,
    pub start: Option<String>,
    pub end: Option<String>,
    pub regexes: Vec<String>,
    /// As [`LocalEdition::examples`].
    #[serde(default)]
    pub examples: Vec<LocalExample>,
    /// As [`LocalEdition::rejects`].
    #[serde(default)]
    pub rejects: Vec<String>,
    /// Why the entry exists.
    pub notes: String,
}

/// The supplement file: a comment, the editions whose regexes it replaces,
/// the reporters it adds, the variations it adds, and the secondary sources
/// it adds to the journals.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalReporters {
    #[serde(rename = "#")]
    pub comment: Vec<String>,
    pub editions: Vec<LocalEdition>,
    #[serde(default)]
    pub reporters: Vec<LocalReporter>,
    #[serde(default)]
    pub variations: Vec<LocalVariation>,
    #[serde(default)]
    pub journals: Vec<LocalJournal>,
}

/// Parse `data/reporters-local.json`.
pub fn parse_local_reporters(json_str: &str) -> Result<LocalReporters, serde_json::Error> {
    serde_json::from_str(json_str)
}

/// Replace the regexes of each supplement edition in `reporters`, then add
/// each supplement reporter. Fails when an edition or variation entry names
/// no edition, so a reporters-db rename can't drop one silently, and when
/// reporters-db already has a reporter or variation the supplement adds.
/// A variation may name a supplement reporter's edition (`4 Bradf. 268` for
/// Bradford's Surrogate Reports). The variations themselves are added by the
/// build, with their own regexes.
pub fn apply_local_reporters(
    reporters: &mut ReportersMap,
    local: &LocalReporters,
) -> Result<(), String> {
    apply_local_editions(reporters, local)?;
    for l in &local.reporters {
        let clusters = reporters.entry(l.reporter.clone()).or_default();
        if clusters.iter().any(|c| c.name == l.name) {
            return Err(format!(
                "reporters-local.json: reporters-db now has {:?} under {:?}; drop the local one",
                l.name, l.reporter
            ));
        }
        if l.regexes.is_empty() {
            return Err(format!("reporters-local.json: {:?} needs regexes", l.name));
        }
        clusters.push(ReporterCluster {
            name: l.name.clone(),
            cite_type: l.cite_type.clone(),
            editions: HashMap::from([(
                l.edition.clone(),
                EditionData {
                    start: l.start.clone(),
                    end: l.end.clone(),
                    regexes: Some(l.regexes.clone()),
                },
            )]),
            variations: HashMap::new(),
            mlz_jurisdiction: Vec::new(),
            cite_format: None,
            extra: HashMap::new(),
        });
    }
    check_local_variations(reporters, local)
}

/// Add the supplement's secondary sources to `journals`. Fails when
/// journals.json already has the key, so the local entry can be dropped.
pub fn merge_local_journals(
    journals: &mut JournalsMap,
    local: &LocalReporters,
) -> Result<(), String> {
    for l in &local.journals {
        if journals.contains_key(&l.edition) {
            return Err(format!(
                "reporters-local.json: journals.json now has {:?}; drop the local one",
                l.edition
            ));
        }
        if l.regexes.is_empty() {
            return Err(format!("reporters-local.json: {:?} needs regexes", l.name));
        }
        journals.insert(
            l.edition.clone(),
            vec![JournalCluster {
                name: l.name.clone(),
                cite_type: l.cite_type.clone(),
                regexes: l.regexes.clone(),
                jurisdiction: String::new(),
                start: l.start.clone(),
                end: l.end.clone(),
                variations: Vec::new(),
                strings: Vec::new(),
                extra: HashMap::new(),
            }],
        );
    }
    Ok(())
}

fn apply_local_editions(
    reporters: &mut ReportersMap,
    local: &LocalReporters,
) -> Result<(), String> {
    for l in &local.editions {
        let edition = reporters
            .get_mut(&l.reporter)
            .and_then(|clusters| clusters.iter_mut().find(|c| c.name == l.name))
            .and_then(|c| c.editions.get_mut(&l.edition))
            .ok_or_else(|| {
                format!(
                    "reporters-local.json: no edition {:?} of {:?} under {:?}",
                    l.edition, l.name, l.reporter
                )
            })?;
        if l.regexes.is_empty() {
            return Err(format!(
                "reporters-local.json: {:?} needs regexes",
                l.edition
            ));
        }
        edition.regexes = Some(l.regexes.clone());
        for (date, local_date, what) in [
            (&mut edition.start, &l.start, "start"),
            (&mut edition.end, &l.end, "end"),
        ] {
            if let Some(d) = local_date {
                if date.as_ref() == Some(d) {
                    return Err(format!(
                        "reporters-local.json: reporters-db now has {what} {d:?} for {:?}; drop the local one",
                        l.edition
                    ));
                }
                *date = Some(d.clone());
            }
        }
    }
    Ok(())
}

fn check_local_variations(reporters: &ReportersMap, local: &LocalReporters) -> Result<(), String> {
    for v in &local.variations {
        let cluster = reporters
            .get(&v.reporter)
            .and_then(|clusters| clusters.iter().find(|c| c.name == v.name))
            .filter(|c| c.editions.contains_key(&v.edition))
            .ok_or_else(|| {
                format!(
                    "reporters-local.json: no edition {:?} of {:?} under {:?}",
                    v.edition, v.name, v.reporter
                )
            })?;
        if cluster.variations.contains_key(&v.variation) {
            return Err(format!(
                "reporters-local.json: reporters-db now has variation {:?} of {:?}; drop the local one",
                v.variation, v.name
            ));
        }
        if v.regexes.is_empty() {
            return Err(format!(
                "reporters-local.json: variation {:?} needs regexes",
                v.variation
            ));
        }
    }
    Ok(())
}

/// Parse journals.json into a map of journal key → list of clusters.
pub fn parse_journals(json_str: &str) -> Result<JournalsMap, serde_json::Error> {
    serde_json::from_str(json_str)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reporters() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
        let reporters = parse_reporters(data).unwrap();
        assert_eq!(reporters.len(), 1236);
        // Count total editions
        let total_editions: usize = reporters
            .values()
            .flat_map(|clusters| clusters.iter())
            .map(|c| c.editions.len())
            .sum();
        assert_eq!(total_editions, 1368);
    }

    fn local_variation(edition: &str, variation: &str) -> LocalReporters {
        parse_local_reporters(&format!(
            r##"{{"#": [], "editions": [], "variations": [{{"reporter": "Ohio St.",
               "name": "Ohio State Reports", "edition": "{edition}", "variation": "{variation}",
               "regexes": ["$full_cite"], "notes": ""}}]}}"##
        ))
        .unwrap()
    }

    #[test]
    fn local_variation_must_name_an_edition_and_be_new() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
        let mut reporters = parse_reporters(data).unwrap();
        apply_local_reporters(&mut reporters, &local_variation("Ohio St.", "Ohio")).unwrap();
        let err = apply_local_reporters(&mut reporters, &local_variation("Ohio St. 4th", "Ohio"))
            .unwrap_err();
        assert!(err.contains("no edition"), "{err}");
        let err = apply_local_reporters(&mut reporters, &local_variation("Ohio St.", "O.S."))
            .unwrap_err();
        assert!(err.contains("now has variation"), "{err}");
    }

    fn local_dates(start: &str, end: &str) -> LocalReporters {
        parse_local_reporters(&format!(
            r##"{{"#": [], "editions": [{{"reporter": "Pa. Fid.",
               "name": "Pennsylvania Fiduciary Reporter", "edition": "Pa. Fid.",
               "regexes": ["$full_cite"], "start": "{start}", "end": "{end}", "notes": ""}}]}}"##
        ))
        .unwrap()
    }

    #[test]
    fn local_edition_dates_replace_reporters_dbs_until_they_agree() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
        let mut reporters = parse_reporters(data).unwrap();
        let local = local_dates("1977-01-01T00:00:00", "2010-12-31T00:00:00");
        let err = apply_local_reporters(&mut reporters, &local).unwrap_err();
        assert!(err.contains("now has start"), "{err}");
        let local = local_dates("1978-01-01T00:00:00", "2010-12-31T00:00:00");
        apply_local_reporters(&mut reporters, &local).unwrap();
        let e = &reporters["Pa. Fid."][0].editions["Pa. Fid."];
        assert_eq!(e.start.as_deref(), Some("1978-01-01T00:00:00"));
        assert_eq!(e.end.as_deref(), Some("2010-12-31T00:00:00"));
    }

    #[test]
    fn local_variation_may_name_a_local_reporter() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
        let mut reporters = parse_reporters(data).unwrap();
        let local = parse_local_reporters(
            r##"{"#": [], "editions": [],
               "reporters": [{"reporter": "Q.", "name": "Q Reports", "cite_type": "state",
                 "edition": "Q.", "start": null, "end": null, "regexes": ["$full_cite"],
                 "notes": ""}],
               "variations": [{"reporter": "Q.", "name": "Q Reports", "edition": "Q.",
                 "variation": "Qu.", "regexes": ["$full_cite"], "notes": ""}]}"##,
        )
        .unwrap();
        apply_local_reporters(&mut reporters, &local).unwrap();
        assert!(reporters["Q."].iter().any(|c| c.name == "Q Reports"));
    }

    #[test]
    fn test_parse_laws() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/laws.json");
        let laws = parse_laws(data).unwrap();
        assert_eq!(laws.len(), 371);
    }

    #[test]
    fn test_parse_journals() {
        let data = include_str!("../../../data/reporters-db/reporters_db/data/journals.json");
        let journals = parse_journals(data).unwrap();
        assert_eq!(journals.len(), 797);
    }

    #[test]
    fn test_parse_reporters_spot_check() {
        let raw = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
        let map = parse_reporters(raw).expect("parse reporters.json");

        // Spot-check U.S. reporter
        let us = map.get("U.S.").expect("missing 'U.S.' reporter");
        assert!(!us.is_empty());
        let cluster = &us[0];
        assert!(cluster.editions.contains_key("U.S."));
        assert_eq!(cluster.cite_type, "federal");
    }

    #[test]
    fn test_parse_laws_spot_check() {
        let raw = include_str!("../../../data/reporters-db/reporters_db/data/laws.json");
        let map = parse_laws(raw).expect("parse laws.json");

        let first = map.get("ASBCA").expect("missing 'ASBCA' law");
        assert!(!first.is_empty());
        let cluster = &first[0];
        assert_eq!(cluster.cite_type, "admin_docket");
        assert!(!cluster.regexes.is_empty());
    }
}
