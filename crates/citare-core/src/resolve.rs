//! Citation resolution pipeline.
//!
//! Given a list of citations in document order, resolve each to a [`Resource`].
//! Full citations create new Resources; short, supra, id, and reference
//! citations are resolved to previously-seen full citations' Resources.
//!
//! Ported from eyecite's `resolve.py`.

use std::collections::{HashMap, HashSet};

use crate::models::{
    Citation, IdCitationData, ReferenceCitationData, Resource, ShortCaseCitationData,
    SupraCitationData,
};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_OPINION_PAGE_COUNT: i32 = 150;

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

type ResolvedFullCite = (Citation, Resource);
type ResolvedFullCites = Vec<ResolvedFullCite>;

/// Maps each resolved [`Resource`] to all [`Citation`]s that point to it.
pub type Resolutions = HashMap<Resource, Vec<Citation>>;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Wrap a full citation in a new [`Resource`].
pub fn resolve_full_citation(citation: &Citation) -> Resource {
    Resource {
        citation: citation.clone(),
    }
}

/// Resolve a slice of citations (in document order) to their resources.
///
/// Returns a map from each distinct [`Resource`] to the list of [`Citation`]s
/// that were resolved to it.
pub fn resolve_citations(citations: &[Citation]) -> Resolutions {
    let mut resolutions: Resolutions = HashMap::new();
    let mut resolved_full_cites: ResolvedFullCites = Vec::new();
    let mut last_resolution: Option<Resource> = None;

    for citation in citations {
        let resolution = match citation {
            // Full citations create new resources.
            Citation::FullCase(_) | Citation::FullLaw(_) | Citation::FullJournal(_) => {
                let resource = resolve_full_citation(citation);
                resolved_full_cites.push((citation.clone(), resource.clone()));
                Some(resource)
            }

            // Short case: match by reporter + volume.
            Citation::ShortCase(data) => {
                resolve_shortcase_citation(citation, data, &resolved_full_cites)
            }

            // Supra: match by antecedent.
            Citation::Supra(data) => resolve_supra_citation(data, &resolved_full_cites),

            // Reference: match by plaintiff/defendant/resolved names.
            Citation::Reference(data) => resolve_reference_citation(data, &resolved_full_cites),

            // Id: resolve to last resolution if the pin cite is plausible.
            Citation::Id(data) => resolve_id_citation(data, &last_resolution, &resolutions),

            // Unknown: skip.
            Citation::Unknown(_) => None,
        };

        last_resolution = resolution.clone();
        if let Some(resource) = resolution {
            resolutions.entry(resource).or_default().push(citation.clone());
        }
    }

    resolutions
}

// ---------------------------------------------------------------------------
// Per-variant resolution helpers
// ---------------------------------------------------------------------------

fn resolve_shortcase_citation(
    citation: &Citation,
    data: &ShortCaseCitationData,
    resolved_full_cites: &ResolvedFullCites,
) -> Option<Resource> {
    let short_reporter = citation.corrected_reporter();
    let short_volume = citation
        .common()
        .groups
        .get("volume")
        .and_then(|v| v.as_ref());

    let mut candidates: ResolvedFullCites = Vec::new();

    for (full_cite, resource) in resolved_full_cites {
        if let Citation::FullCase(_) = full_cite {
            let full_reporter = full_cite.corrected_reporter();
            let full_volume = full_cite
                .common()
                .groups
                .get("volume")
                .and_then(|v| v.as_ref());

            if short_reporter == full_reporter && short_volume == full_volume {
                candidates.push((full_cite.clone(), resource.clone()));
            }
        }
    }

    // Unique resource — return immediately.
    let unique_resources: HashSet<&Resource> = candidates.iter().map(|(_, r)| r).collect();
    if unique_resources.len() == 1 {
        return Some(candidates[0].1.clone());
    }

    // Attempt antecedent refinement.
    if let Some(ref antecedent) = data.metadata.antecedent_guess {
        return filter_by_matching_antecedent(&candidates, antecedent);
    }

    None
}

fn resolve_supra_citation(
    data: &SupraCitationData,
    resolved_full_cites: &ResolvedFullCites,
) -> Option<Resource> {
    let antecedent = data.metadata.antecedent_guess.as_ref()?;
    filter_by_matching_antecedent(resolved_full_cites, antecedent)
}

fn resolve_id_citation(
    data: &IdCitationData,
    last_resolution: &Option<Resource>,
    resolutions: &Resolutions,
) -> Option<Resource> {
    let last = last_resolution.as_ref()?;

    // Retrieve the full citation that created this resource.
    let full_cite_list = resolutions.get(last)?;
    let full_cite = full_cite_list.first()?;

    if has_invalid_pin_cite(full_cite, data) {
        return None;
    }

    Some(last.clone())
}

fn resolve_reference_citation(
    data: &ReferenceCitationData,
    resolved_full_cites: &ResolvedFullCites,
) -> Option<Resource> {
    // Collect all non-None reference values for this citation.
    let ref_values: HashSet<String> = [
        &data.metadata.defendant,
        &data.metadata.plaintiff,
        &data.metadata.resolved_case_name_short,
        &data.metadata.resolved_case_name,
    ]
    .iter()
    .filter_map(|v| v.as_ref().cloned())
    .collect();

    if ref_values.is_empty() {
        return None;
    }

    let mut matches: Vec<Resource> = Vec::new();

    for (full_cite, resource) in resolved_full_cites {
        let full_values = get_metadata_values(full_cite);
        if ref_values.iter().any(|rv| full_values.contains(rv)) {
            matches.push(resource.clone());
        }
    }

    matches.sort_by_key(|r| format!("{:?}", r));
    matches.dedup();

    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Filter `resolved_full_cites` by whether the full citation's defendant or
/// plaintiff contains `antecedent_guess`.  Returns the resource only when
/// exactly one unique resource matches.
fn filter_by_matching_antecedent(
    resolved_full_cites: &ResolvedFullCites,
    antecedent_guess: &str,
) -> Option<Resource> {
    use crate::utils::strip_punct;

    let ag = strip_punct(antecedent_guess);
    let mut matches: Vec<Resource> = Vec::new();

    for (full_cite, resource) in resolved_full_cites {
        if let Citation::FullCase(data) = full_cite {
            let defendant_match = data
                .metadata
                .defendant
                .as_ref()
                .is_some_and(|d| d.contains(&ag));
            let plaintiff_match = data
                .metadata
                .plaintiff
                .as_ref()
                .is_some_and(|p| p.contains(&ag));

            if defendant_match || plaintiff_match {
                matches.push(resource.clone());
            }
        }
    }

    matches.sort_by_key(|r| format!("{:?}", r));
    matches.dedup();

    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        None
    }
}

/// Return `true` when `id_data`'s pin cite is implausible for `full_cite`.
fn has_invalid_pin_cite(full_cite: &Citation, id_data: &IdCitationData) -> bool {
    // A full case citation without a page can never produce a valid id. cite.
    if let Citation::FullCase(_) = full_cite {
        if full_cite
            .common()
            .groups
            .get("page")
            .and_then(|v| v.as_ref())
            .is_none()
        {
            return true;
        }
    }

    // No pin cite on the id. citation — nothing to validate.
    let pin_cite = match &id_data.metadata.pin_cite {
        Some(p) => p,
        None => return false,
    };

    // If the full citation's page is not purely numeric, we can't range-check.
    let page_str = match full_cite
        .common()
        .groups
        .get("page")
        .and_then(|v| v.as_ref())
    {
        Some(p) if p.chars().all(|c| c.is_ascii_digit()) => p.clone(),
        _ => return false,
    };
    let page: i32 = page_str.parse().unwrap_or(0);

    // Extract the numeric portion of the pin cite (e.g. "at 5" → 5).
    let re = regex::Regex::new(r"(?:at )?(\d+)").unwrap();
    let pin: i32 = match re.captures(pin_cite) {
        Some(caps) => caps[1].parse().unwrap_or(0),
        // Non-numeric pin cite is assumed invalid.
        None => return true,
    };

    // The pin cite must fall within a reasonable distance from the page.
    pin < page || pin > page + MAX_OPINION_PAGE_COUNT
}

/// Collect all non-empty string values from a citation's metadata fields,
/// used for reference citation matching.
fn get_metadata_values(citation: &Citation) -> HashSet<String> {
    let mut values: HashSet<String> = HashSet::new();

    match citation {
        Citation::FullCase(data) => {
            for v in [
                &data.metadata.plaintiff,
                &data.metadata.defendant,
                &data.metadata.resolved_case_name_short,
                &data.metadata.resolved_case_name,
                &data.metadata.antecedent_guess,
            ]
            .into_iter()
            .flatten()
            {
                if !v.is_empty() {
                    values.insert(v.clone());
                }
            }
        }
        Citation::FullLaw(data) => {
            for v in [&data.metadata.publisher].into_iter().flatten() {
                if !v.is_empty() {
                    values.insert(v.clone());
                }
            }
        }
        _ => {}
    }

    values
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_single_full_citation() {
        let citations = crate::find::get_citations("1 U.S. 1", false, "", &[]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 1);
        let (_, cites) = resolutions.iter().next().unwrap();
        assert_eq!(cites.len(), 1);
    }

    #[test]
    fn test_resolve_full_and_short() {
        let text = "Foo v. Bar, 1 U.S. 1 (1999). Bar, 1 U.S., at 2.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        // Both should resolve to the same resource.
        assert_eq!(resolutions.len(), 1);
        let (_, cites) = resolutions.iter().next().unwrap();
        assert!(cites.len() >= 2);
    }

    #[test]
    fn test_resolve_id_citation() {
        let text = "1 U.S. 1 (1999). Id. at 2.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 1);
    }

    #[test]
    fn test_resolve_supra() {
        let text = "Foo v. Bar, 1 U.S. 1 (1999). Foo, supra, at 2.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 1);
    }

    #[test]
    fn test_invalid_pin_cite_filtered() {
        // Pin cite at 999 from page 1 exceeds MAX_OPINION_PAGE_COUNT.
        let text = "1 U.S. 1 (1999). Id. at 999.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        // The id. citation must NOT be resolved; only the full cite counts.
        let total_resolved: usize = resolutions.values().map(|v| v.len()).sum();
        assert_eq!(total_resolved, 1);
    }

    #[test]
    fn test_resolve_full_citation_creates_resource() {
        let citations = crate::find::get_citations("410 U.S. 113", false, "", &[]);
        assert!(!citations.is_empty());
        let resource = resolve_full_citation(&citations[0]);
        // Resource wraps the same citation.
        assert_eq!(resource.citation, citations[0]);
    }

    #[test]
    fn test_unknown_citation_not_resolved() {
        // Only full citations should produce resolutions.
        let text = "1 U.S. 1";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        // All resolved items are full citations.
        for resource in resolutions.keys() {
            assert!(resource.citation.is_full());
        }
    }

    #[test]
    fn test_multiple_full_citations_separate_resources() {
        let text = "1 U.S. 1 (1999). 2 U.S. 2 (2000).";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 2);
    }
}
