//! Serde structs and parse function for courts-db JSON files.
//!
//! `courts.json` is a flat array of court objects.  Many courts have
//! `$variable` template placeholders inside their `regex` arrays — those
//! templates refer to variables defined in `variables.json` and
//! `places/` subdirectories.  Template expansion is only needed when
//! building full court-matching patterns; for citation resolution we only
//! need `id`, `citation_string`, `name`, and `type`, none of which contain
//! template references.
//!
//! Full template expansion is deferred to a later phase.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer};

/// Deserialize a nullable-or-absent string field as an empty string.
/// This handles three cases: field is absent, field is `null`, or field is a string.
fn nullable_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn default_empty() -> String {
    String::new()
}

/// Court record from `courts.json`.
#[derive(Debug, Clone, Deserialize)]
pub struct Court {
    pub id: String,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub name: String,
    /// May be null in the data.
    #[serde(default)]
    pub name_abbreviation: Option<String>,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub citation_string: String,
    #[serde(
        rename = "type",
        default = "default_empty",
        deserialize_with = "nullable_string"
    )]
    pub court_type: String,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub system: String,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub level: String,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub jurisdiction: String,
    #[serde(default = "default_empty", deserialize_with = "nullable_string")]
    pub location: String,
    pub parent: Option<String>,
    /// All other fields (regex, dates, examples, locations, etc.)
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Parse `courts.json` — a JSON array of court objects — into a `Vec<Court>`.
pub fn parse_courts(json_str: &str) -> Result<Vec<Court>, serde_json::Error> {
    serde_json::from_str(json_str)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_courts() {
        let data = include_str!("../../../data/courts-db/courts_db/data/courts.json");
        let courts = parse_courts(data).expect("parse courts.json");
        // Accept either 2805 or 2806 depending on submodule version
        assert!(
            courts.len() >= 2805,
            "expected at least 2805 courts, got {}",
            courts.len()
        );

        let with_citation_string = courts
            .iter()
            .filter(|c| !c.citation_string.is_empty())
            .count();
        assert_eq!(with_citation_string, 1956);
    }

    #[test]
    fn test_parse_courts_spot_check() {
        let raw = include_str!("../../../data/courts-db/courts_db/data/courts.json");
        let courts = parse_courts(raw).expect("parse courts.json");

        // Spot-check a known court
        let pennsylvaniad = courts.iter().find(|c| c.id == "pennsylvaniad");
        assert!(
            pennsylvaniad.is_some(),
            "expected court with id='pennsylvaniad'"
        );
        let court = pennsylvaniad.unwrap();
        assert_eq!(court.citation_string, "D. Pa.");
    }
}
