//! Citation resolution pipeline.
//!
//! Given a list of citations in document order, resolve each to a [`Resource`].
//! Full citations create new Resources; short, supra, id, and reference
//! citations are resolved to previously-seen full citations' Resources.
//!
//! Ported from eyecite's `resolve.py`.

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

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

/// Maps each resolved [`Resource`] to all [`Citation`]s that point to it.
///
/// Iteration order is a `HashMap`'s, so it varies per process. Sort by
/// `resource.citation.span()` for document order, which is the order
/// eyecite's dict returns resources in.
pub type Resolutions = HashMap<Resource, Vec<Citation>>;

/// A full citation seen so far, with what the later citations match on.
struct ResolvedFullCite {
    citation: Citation,
    resource: Resource,
    /// Equal for equal resources (see [`resource_key`]).
    key: u64,
}

/// The full citations before the citation being resolved, in document
/// order, indexed by what short and reference citations look up.
///
/// eyecite compares each short, supra and reference citation with every
/// earlier full citation. Looked up here by reporter and volume, or by
/// party name, short and reference citations cost the same however long the
/// document is (#44).
#[derive(Default)]
struct ResolvedFullCites {
    cites: Vec<ResolvedFullCite>,
    /// Full case citations by (corrected reporter, volume).
    by_reporter_volume: HashMap<(Option<String>, Option<String>), Vec<usize>>,
    /// Full citations by each of their metadata values (see
    /// [`get_metadata_values`]).
    by_metadata_value: HashMap<String, Vec<usize>>,
}

impl ResolvedFullCites {
    fn push(&mut self, citation: &Citation, resource: Resource) {
        let index = self.cites.len();
        if let Citation::FullCase(_) = citation {
            self.by_reporter_volume
                .entry(reporter_volume(citation))
                .or_default()
                .push(index);
        }
        for value in get_metadata_values(citation) {
            self.by_metadata_value.entry(value).or_default().push(index);
        }
        self.cites.push(ResolvedFullCite {
            citation: citation.clone(),
            key: resource_key(&resource),
            resource,
        });
    }
}

/// A hash equal for equal resources, as [`Resource`]'s `PartialEq` compares
/// them.
fn resource_key(resource: &Resource) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    resource.hash(&mut hasher);
    hasher.finish()
}

/// What a short case citation must share with its full citation.
fn reporter_volume(citation: &Citation) -> (Option<String>, Option<String>) {
    let volume = citation
        .common()
        .groups
        .get("volume")
        .and_then(|v| v.clone());
    (citation.corrected_reporter(), volume)
}

/// The resource of the matched full citations, if they are all one
/// resource. eyecite resolves only when exactly one resource matches.
fn single_resource(
    resolved: &ResolvedFullCites,
    mut matches: impl Iterator<Item = usize>,
) -> Option<&Resource> {
    let first = &resolved.cites[matches.next()?];
    matches
        .all(|i| resolved.cites[i].key == first.key)
        .then_some(&first.resource)
}

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
    let mut resolved_full_cites = ResolvedFullCites::default();
    let mut last_resolution: Option<Resource> = None;

    for citation in citations {
        let resolution = match citation {
            // Full citations create new resources.
            Citation::FullCase(_) | Citation::FullLaw(_) | Citation::FullJournal(_) => {
                let resource = resolve_full_citation(citation);
                resolved_full_cites.push(citation, resource.clone());
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
            resolutions
                .entry(resource)
                .or_default()
                .push(citation.clone());
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
    let candidates: &[usize] = resolved_full_cites
        .by_reporter_volume
        .get(&reporter_volume(citation))
        .map_or(&[], Vec::as_slice);

    // Unique resource — return immediately.
    if let Some(resource) = single_resource(resolved_full_cites, candidates.iter().copied()) {
        return Some(resource.clone());
    }

    // Attempt antecedent refinement.
    if let Some(ref antecedent) = data.metadata.antecedent_guess {
        return filter_by_matching_antecedent(
            resolved_full_cites,
            candidates.iter().copied(),
            antecedent,
        );
    }

    None
}

fn resolve_supra_citation(
    data: &SupraCitationData,
    resolved_full_cites: &ResolvedFullCites,
) -> Option<Resource> {
    let antecedent = data.metadata.antecedent_guess.as_ref()?;
    filter_by_matching_antecedent(
        resolved_full_cites,
        0..resolved_full_cites.cites.len(),
        antecedent,
    )
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
    // Full citations whose metadata holds any of this citation's values.
    let mut matches: Vec<usize> = [
        &data.metadata.defendant,
        &data.metadata.plaintiff,
        &data.metadata.resolved_case_name_short,
        &data.metadata.resolved_case_name,
    ]
    .into_iter()
    .flatten()
    .filter_map(|value| resolved_full_cites.by_metadata_value.get(value))
    .flatten()
    .copied()
    .collect();
    matches.sort_unstable();
    matches.dedup();

    single_resource(resolved_full_cites, matches.into_iter()).cloned()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Filter the full citations at `candidates` by whether the full case
/// citation's defendant or plaintiff contains `antecedent_guess`. Returns
/// the resource only when exactly one unique resource matches.
fn filter_by_matching_antecedent(
    resolved_full_cites: &ResolvedFullCites,
    candidates: impl Iterator<Item = usize>,
    antecedent_guess: &str,
) -> Option<Resource> {
    use crate::utils::strip_punct;

    let ag = strip_punct(antecedent_guess);
    let matches = candidates.filter(|&i| {
        let Citation::FullCase(data) = &resolved_full_cites.cites[i].citation else {
            return false;
        };
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
        defendant_match || plaintiff_match
    });

    single_resource(resolved_full_cites, matches).cloned()
}

/// Whether a pin cite names a section: `§ 5`, `§§ 5-7`, `at § 12(b)`.
fn is_section_pin(pin_cite: &str) -> bool {
    pin_cite
        .strip_prefix("at ")
        .unwrap_or(pin_cite)
        .starts_with('§')
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

    // A section pin cite only rules out a case. Statutes at Large, session laws
    // and other paged law citations are divided into sections, so `Id. § 4(b)`
    // after `83 Stat. 841` is a pin cite into it, not a page to range-check.
    // (eyecite never sees these pin cites: its `§` is a separate token.)
    if !matches!(full_cite, Citation::FullCase(_)) && is_section_pin(pin_cite) {
        return false;
    }

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

    // Extract the leading page number of the pin cite (e.g. "at 5" → 5).
    // Anchored like eyecite's `re.match`, so "at ¶ 4" and "§ 5" have none.
    static PIN_NUMBER_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^(?:at )?(\d+)").unwrap());
    let pin: i32 = match PIN_NUMBER_RE.captures(pin_cite) {
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

    // A reference citation that carries only the matched party name still
    // resolves to its full citation.
    #[test]
    fn test_resolve_reference_with_single_party() {
        let text = "Foo v. Bar 1 U.S. 12, 347-348. something something, In Foo at 62, we see that";
        let citations = crate::find::get_citations(text, false, "", &[]);
        assert!(matches!(citations[1], Citation::Reference(_)));
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 1);
        let (_, cites) = resolutions.iter().next().unwrap();
        assert_eq!(cites.len(), 2);
    }

    // Ported from eyecite tests/test_ResolveTest.py: two otherwise identical
    // full citations with missing pages must not resolve to the same
    // resource.
    #[test]
    fn test_blank_page_full_citations_are_distinct() {
        let text = "Foo v. Bar, 1 U.S. ____. Foo v. Bar, 1 U.S. ____.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        assert_eq!(citations.len(), 2);
        assert_ne!(citations[0], citations[1]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 2);
        for cites in resolutions.values() {
            assert_eq!(cites.len(), 1);
        }
    }

    // A blank-page citation is still equal to a clone of itself, so it can
    // serve as a stable resource key.
    #[test]
    fn test_blank_page_citation_equals_its_clone() {
        let citations = crate::find::get_citations("1 U.S. ___", false, "", &[]);
        let resource = resolve_full_citation(&citations[0]);
        assert_eq!(resource.citation, citations[0]);
        assert_eq!(resource, resolve_full_citation(&citations[0]));
    }

    // Ported from eyecite: an Id. pin cite after a full citation that only
    // has a placeholder page can't be resolved.
    #[test]
    fn test_id_after_blank_page_not_resolved() {
        let text = "Foo v. Bar, 1 U.S. ___. Id. at 100.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        assert_eq!(citations.len(), 2);
        let resolutions = resolve_citations(&citations);
        let total_resolved: usize = resolutions.values().map(|v| v.len()).sum();
        assert_eq!(total_resolved, 1);
    }

    #[test]
    fn test_short_cite_resolves_to_blank_page_full_cite() {
        let text = "Foo v. Bar, 1 U.S. ___ (2019). Foo, 1 U.S., at 5.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        assert_eq!(resolutions.len(), 1);
        let (_, cites) = resolutions.iter().next().unwrap();
        assert_eq!(cites.len(), 2);
    }
}
