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
