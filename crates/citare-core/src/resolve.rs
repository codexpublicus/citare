//! Citation resolution pipeline.
//!
//! Given a list of citations in document order, resolve each to a [`Resource`].
//! Full citations create new Resources; short, supra, id, and reference
//! citations are resolved to previously-seen full citations' Resources.
//!
//! Ported from eyecite's `resolve.py`.

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
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

/// Maps each resolved resource to all [`Citation`]s that point to it.
///
/// The resource type is citare's [`Resource`] unless a [`Resolver`] with
/// its own resource type was used ([`resolve_citations_with`]).
///
/// Iteration order is a `HashMap`'s, so it varies per process. Sort by
/// `resource.citation.span()` (or by each group's first citation) for
/// document order, which is the order eyecite's dict returns resources in.
pub type Resolutions<R = Resource> = HashMap<R, Vec<Citation>>;

/// A full citation seen so far, with what the later citations match on.
struct ResolvedFullCite<R> {
    citation: Citation,
    /// `None` when the resolver's `resolve_full` left it unresolved.
    resource: Option<R>,
    /// Equal for equal resources (see [`resource_key`]).
    key: u64,
    /// The corrected reporter, for choosing among parallel citations.
    reporter: Option<String>,
}

/// The full citations before the citation being resolved, in document
/// order, indexed by what short and reference citations look up.
///
/// eyecite compares each short, supra and reference citation with every
/// earlier full citation. Looked up here by reporter and volume, or by
/// party name, short and reference citations cost the same however long the
/// document is (#44).
struct ResolvedFullCites<R> {
    cites: Vec<ResolvedFullCite<R>>,
    /// Full case citations by (corrected reporter, volume).
    by_reporter_volume: HashMap<(Option<String>, Option<String>), Vec<usize>>,
    /// Full citations by each of their metadata values (see
    /// [`get_metadata_values`]).
    by_metadata_value: HashMap<String, Vec<usize>>,
    /// The resources of the last full citation and those cited in
    /// parallel just before it (see [`is_parallel`]), by [`resource_key`],
    /// sorted.
    current_decision: Vec<u64>,
    /// Each distinct set of resources cited in parallel, as
    /// `current_decision` was after each parallel citation.
    decisions: Vec<Vec<u64>>,
    decision_ids: HashMap<Vec<u64>, usize>,
    /// The `decisions` each resource is in.
    decisions_by_key: HashMap<u64, Vec<usize>>,
    /// The most resources in one of `decisions`.
    max_decision_len: usize,
}

impl<R> Default for ResolvedFullCites<R> {
    fn default() -> Self {
        ResolvedFullCites {
            cites: Vec::new(),
            by_reporter_volume: HashMap::new(),
            by_metadata_value: HashMap::new(),
            current_decision: Vec::new(),
            decisions: Vec::new(),
            decision_ids: HashMap::new(),
            decisions_by_key: HashMap::new(),
            max_decision_len: 0,
        }
    }
}

impl<R: Hash> ResolvedFullCites<R> {
    /// Add a full citation. `parallel` says it is cited in parallel with
    /// the last one added.
    fn push(&mut self, citation: &Citation, resource: Option<R>, parallel: bool) {
        let index = self.cites.len();
        let key = resource_key(&resource);
        let (reporter, _) = reporter_volume(citation);
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
            key,
            resource,
            reporter,
        });

        if !parallel {
            self.current_decision.clear();
        }
        if let Err(at) = self.current_decision.binary_search(&key) {
            self.current_decision.insert(at, key);
        }
        if self.current_decision.len() > 1
            && !self.decision_ids.contains_key(&self.current_decision)
        {
            let id = self.decisions.len();
            for &key in &self.current_decision {
                self.decisions_by_key.entry(key).or_default().push(id);
            }
            self.max_decision_len = self.max_decision_len.max(self.current_decision.len());
            self.decision_ids.insert(self.current_decision.clone(), id);
            self.decisions.push(self.current_decision.clone());
        }
    }

    /// Whether some parallel citation of the resource `key` also cited every
    /// resource in `keys`.
    fn one_decision(&self, key: u64, keys: &[u64]) -> bool {
        self.decisions_by_key.get(&key).is_some_and(|ids| {
            ids.iter().any(|&id| {
                let decision = &self.decisions[id];
                keys.iter().all(|k| decision.binary_search(k).is_ok())
            })
        })
    }
}

/// A hash equal for equal resources, as [`Resource`]'s `PartialEq` compares
/// them. Unresolved full citations (`None`) all share one key, so a short
/// form matching one of them and a resolved citation is ambiguous, as in
/// eyecite.
fn resource_key<R: Hash>(resource: &Option<R>) -> u64 {
    use std::hash::Hasher;
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

/// Whether `citation` is cited in parallel with `previous`, the citation
/// just before it: both are full case citations under one case name, as
/// in "Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005)". These are the
/// pairs `find` copies the case name and year across.
fn is_parallel(previous: &Citation, citation: &Citation) -> bool {
    let (Citation::FullCase(previous), Citation::FullCase(current)) = (previous, citation) else {
        return false;
    };
    let start = current.resource.common.full_span_start;
    start.is_some()
        && start == previous.resource.common.full_span_start
        && current.metadata.plaintiff == previous.metadata.plaintiff
        && current.metadata.defendant == previous.metadata.defendant
}

/// The resource a short or supra citation names, given the full citations
/// it matched.
///
/// When they are all one resource, that resource, as eyecite resolves.
/// When they are different resources but all one decision cited in
/// parallel, the matched citation in `reporter` (the short form's), or
/// else the first one. Otherwise none: the citation is ambiguous.
fn decision_resource<'a, R: Hash>(
    resolved: &'a ResolvedFullCites<R>,
    mut matches: impl Iterator<Item = usize>,
    reporter: Option<&str>,
) -> Option<&'a R> {
    let first = &resolved.cites[matches.next()?];
    let mut keys = vec![first.key];
    let mut in_reporter =
        (reporter.is_some() && first.reporter.as_deref() == reporter).then_some(first);
    for i in matches {
        let cite = &resolved.cites[i];
        if !keys.contains(&cite.key) {
            // More resources than any one decision was cited in.
            if keys.len() == resolved.max_decision_len.max(1) {
                return None;
            }
            keys.push(cite.key);
        }
        if in_reporter.is_none() && reporter.is_some() && cite.reporter.as_deref() == reporter {
            in_reporter = Some(cite);
        }
    }
    if keys.len() == 1 {
        first.resource.as_ref()
    } else if resolved.one_decision(first.key, &keys) {
        in_reporter.unwrap_or(first).resource.as_ref()
    } else {
        None
    }
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
/// that were resolved to it. This is [`resolve_citations_with`] and the
/// [`DefaultResolver`].
pub fn resolve_citations(citations: &[Citation]) -> Resolutions {
    resolve_citations_with(citations, &mut DefaultResolver::default())
}

/// How [`resolve_citations_with`] resolves each kind of citation to a
/// resource: eyecite's `resolve_full_citation`, `resolve_shortcase_citation`,
/// `resolve_supra_citation`, `resolve_reference_citation` and
/// `resolve_id_citation` callbacks, as one trait.
///
/// A resource is whatever the caller groups citations by: citare's own
/// [`Resource`], a database id, a URL. Only [`resolve_full`] is required.
/// Every other method defaults to citare's built-in rules, applied to the
/// resources `resolve_full` returned, so a resolver that maps full
/// citations to its own ids gets short, supra, reference and id citations
/// grouped under those ids without reimplementing them. An override can
/// still call the built-in rule through the [`ResolveContext`] (for
/// instance, to post-filter it).
///
/// Returning `None` leaves a citation unresolved: it appears in no group.
/// An unresolved full citation still counts when a later short form is
/// matched against the full citations, so a short form that matches it and
/// a resolved one is ambiguous rather than resolved to the other.
///
/// [`resolve_full`]: Resolver::resolve_full
///
/// # Example
///
/// Group citations by a caller's own case ids:
///
/// ```rust
/// use citare_core::resolve::{resolve_citations_with, Resolver};
/// use citare_core::{get_citations, Citation};
///
/// struct ById;
///
/// impl Resolver for ById {
///     type Resource = String;
///
///     fn resolve_full(&mut self, citation: &Citation) -> Option<String> {
///         // Look the citation up in your own index here.
///         Some(format!("case:{}", citation.matched_text()))
///     }
/// }
///
/// let text = "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2. Foo, 1 U.S., at 3.";
/// let citations = get_citations(text, false, "", &[]);
/// let groups = resolve_citations_with(&citations, &mut ById);
/// assert_eq!(groups["case:1 U.S. 1"].len(), 3);
/// ```
pub trait Resolver {
    /// What citations are grouped by. Equal resources are one group.
    type Resource: Clone + Eq + Hash;

    /// Resolve a full case, law or journal citation.
    fn resolve_full(&mut self, citation: &Citation) -> Option<Self::Resource>;

    /// Resolve a short case citation (`Foo, 1 U.S., at 2`). The default is
    /// [`ResolveContext::resolve_shortcase`].
    fn resolve_shortcase(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, Self::Resource>,
    ) -> Option<Self::Resource> {
        context.resolve_shortcase(citation)
    }

    /// Resolve a supra citation (`Foo, supra, at 2`). The default is
    /// [`ResolveContext::resolve_supra`].
    fn resolve_supra(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, Self::Resource>,
    ) -> Option<Self::Resource> {
        context.resolve_supra(citation)
    }

    /// Resolve a reference citation (`In Foo at 2`). The default is
    /// [`ResolveContext::resolve_reference`].
    fn resolve_reference(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, Self::Resource>,
    ) -> Option<Self::Resource> {
        context.resolve_reference(citation)
    }

    /// Resolve an id. citation (`Id. at 2`). The default is
    /// [`ResolveContext::resolve_id`].
    fn resolve_id(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, Self::Resource>,
    ) -> Option<Self::Resource> {
        context.resolve_id(citation)
    }
}

/// The resolver [`resolve_citations`] uses: each full citation is its own
/// [`Resource`] ([`resolve_full_citation`]) and every other kind follows
/// citare's built-in rules.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct DefaultResolver;

impl Resolver for DefaultResolver {
    type Resource = Resource;

    fn resolve_full(&mut self, citation: &Citation) -> Option<Resource> {
        Some(resolve_full_citation(citation))
    }
}

/// What a [`Resolver`] can see when resolving a short form: the full
/// citations before it, with their resources (eyecite's
/// `resolved_full_cites`), the last citation's resolution and the groups so
/// far. Its `resolve_*` methods are citare's built-in rules over that state.
pub struct ResolveContext<'a, R> {
    resolved_full_cites: &'a ResolvedFullCites<R>,
    last_resolution: Option<&'a R>,
    resolutions: &'a Resolutions<R>,
}

impl<'a, R: Clone + Eq + Hash> ResolveContext<'a, R> {
    /// The full citations so far, in document order, each with the resource
    /// [`Resolver::resolve_full`] gave it (`None` if it left it unresolved).
    pub fn full_citations(&self) -> impl Iterator<Item = (&'a Citation, Option<&'a R>)> + 'a {
        self.resolved_full_cites
            .cites
            .iter()
            .map(|c| (&c.citation, c.resource.as_ref()))
    }

    /// The resource the citation just before this one resolved to, if any.
    pub fn last_resolution(&self) -> Option<&'a R> {
        self.last_resolution
    }

    /// The groups so far.
    pub fn resolutions(&self) -> &'a Resolutions<R> {
        self.resolutions
    }

    /// citare's rule for a short case citation: the full case citations
    /// with its reporter and volume, if they are one resource, or one
    /// decision cited in parallel (then the one in the short form's
    /// reporter); otherwise those whose party names contain its antecedent,
    /// under the same condition. `None` for any other kind of citation.
    pub fn resolve_shortcase(&self, citation: &Citation) -> Option<R> {
        let Citation::ShortCase(data) = citation else {
            return None;
        };
        resolve_shortcase_citation(citation, data, self.resolved_full_cites)
    }

    /// citare's rule for a supra citation: the full case citations whose
    /// party names contain its antecedent, if they are one resource or one
    /// decision cited in parallel (then the first). `None` for any other
    /// kind of citation.
    pub fn resolve_supra(&self, citation: &Citation) -> Option<R> {
        let Citation::Supra(data) = citation else {
            return None;
        };
        resolve_supra_citation(data, self.resolved_full_cites)
    }

    /// citare's rule for a reference citation: the full citations whose
    /// party or case names equal one of its names, if they are one
    /// resource. `None` for any other kind of citation.
    pub fn resolve_reference(&self, citation: &Citation) -> Option<R> {
        let Citation::Reference(data) = citation else {
            return None;
        };
        resolve_reference_citation(data, self.resolved_full_cites)
    }

    /// citare's rule for an id. citation: the last citation's resource,
    /// unless its pin cite can't be in the first citation of that resource
    /// (see `docs/divergences.md` for section pin cites). `None` for any
    /// other kind of citation.
    pub fn resolve_id(&self, citation: &Citation) -> Option<R> {
        let Citation::Id(data) = citation else {
            return None;
        };
        resolve_id_citation(data, self.last_resolution, self.resolutions)
    }
}

/// Resolve a slice of citations (in document order) with `resolver`.
///
/// eyecite's `resolve_citations` with its resolver callbacks. Returns a map
/// from each distinct resource to the citations resolved to it, in document
/// order. Unknown citations are never resolved.
pub fn resolve_citations_with<V: Resolver + ?Sized>(
    citations: &[Citation],
    resolver: &mut V,
) -> Resolutions<V::Resource> {
    let mut state = ResolveState {
        resolutions: HashMap::new(),
        resolved_full_cites: ResolvedFullCites::default(),
        last_resolution: None,
    };

    let mut i = 0;
    while i < citations.len() {
        let previous = i.checked_sub(1).map(|p| &citations[p]);
        let citation = &citations[i];
        match citations.get(i + 1) {
            // An id. citation that stands for the case name of the full
            // citation after it ("Id., (1990), 52 Ohio St.3d 155") is an id.
            // of that citation, not of the one before (#78). The full
            // citation is resolved first, and the id. citation after it.
            Some(full) if names_next_citation(citation, full) => {
                let resource = state.resolve(resolver, full, Some(citation));
                state.last_resolution = resource.clone();
                if let Some(resource) = resource {
                    state.record(resource, full.clone());
                }
                let id = state.resolve(resolver, citation, None);
                if let Some(id_resource) = id {
                    state.record(id_resource.clone(), citation.clone());
                    // In text order: the id. citation before the full one.
                    let group = state.resolutions.get_mut(&id_resource).unwrap();
                    if group.len() > 1 && state.last_resolution.as_ref() == Some(&id_resource) {
                        let n = group.len();
                        group.swap(n - 2, n - 1);
                    }
                }
                i += 2;
            }
            _ => {
                let resource = state.resolve(resolver, citation, previous);
                state.last_resolution = resource.clone();
                if let Some(resource) = resource {
                    state.record(resource, citation.clone());
                }
                i += 1;
            }
        }
    }

    state.resolutions
}

/// What [`resolve_citations_with`] has built so far.
struct ResolveState<R> {
    resolutions: Resolutions<R>,
    resolved_full_cites: ResolvedFullCites<R>,
    last_resolution: Option<R>,
}

impl<R: Clone + Eq + Hash> ResolveState<R> {
    /// Resolve one citation. `previous` is the citation before it in the
    /// text, for full citations cited in parallel.
    fn resolve<V: Resolver<Resource = R> + ?Sized>(
        &mut self,
        resolver: &mut V,
        citation: &Citation,
        previous: Option<&Citation>,
    ) -> Option<R> {
        match citation {
            // Full citations create new resources.
            Citation::FullCase(_) | Citation::FullLaw(_) | Citation::FullJournal(_) => {
                let resource = resolver.resolve_full(citation);
                let parallel = previous.is_some_and(|p| is_parallel(p, citation));
                self.resolved_full_cites
                    .push(citation, resource.clone(), parallel);
                resource
            }

            // Unknown: skip.
            Citation::Unknown(_) => None,

            _ => {
                let context = ResolveContext {
                    resolved_full_cites: &self.resolved_full_cites,
                    last_resolution: self.last_resolution.as_ref(),
                    resolutions: &self.resolutions,
                };
                match citation {
                    // Short case: match by reporter + volume.
                    Citation::ShortCase(_) => resolver.resolve_shortcase(citation, &context),
                    // Supra: match by antecedent.
                    Citation::Supra(_) => resolver.resolve_supra(citation, &context),
                    // Reference: match by plaintiff/defendant/resolved names.
                    Citation::Reference(_) => resolver.resolve_reference(citation, &context),
                    // Id: resolve to last resolution if the pin cite is plausible.
                    Citation::Id(_) => resolver.resolve_id(citation, &context),
                    _ => None,
                }
            }
        }
    }

    fn record(&mut self, resource: R, citation: Citation) {
        self.resolutions.entry(resource).or_default().push(citation);
    }
}

/// Whether `citation` is an id. citation standing for the case name of the
/// full case citation `next` that directly follows it, as in "In re Murray
/// (1990), 52 Ohio St.3d 155, 157; Stanley v. Illinois (1972), 405 U.S.
/// 645, 651. Id., (1990), 52 Ohio St.3d 155" or "Grain Cleaner Co. v.
/// Smith, 110 N. Y. 83; Id., 34 Hun, 91" (#78). The search for `next`'s
/// case name took the id. citation in, so `next`'s full span starts at or
/// before it, and that name is the id. citation and at most a court or
/// year in parentheses or an old circuit ("Id., 9 Cir., 152 F.2d 298").
/// Anything else after it, such as "Id. See also Benoit v. Turner" or
/// "Id. [15]", a footnote marker, makes the id. citation its own.
fn names_next_citation(citation: &Citation, next: &Citation) -> bool {
    static ID_AS_NAME_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"^(?i:id|ibid)\.?(?:[\s,]|\([^()\[\]]*\)?|\d+(?:st|nd|rd|th)?\s+Cir\.)*$",
        )
        .unwrap()
    });
    let (Citation::Id(id), Citation::FullCase(full)) = (citation, next) else {
        return false;
    };
    if id.metadata.pin_cite.is_some() || full.metadata.plaintiff.is_some() {
        return false;
    }
    let (id_start, id_end) = citation.span();
    full.resource
        .common
        .full_span_start
        .is_some_and(|start| start <= id_start && id_end <= next.span().0)
        && full
            .metadata
            .defendant
            .as_deref()
            .is_some_and(|name| ID_AS_NAME_RE.is_match(name.trim()))
}

// ---------------------------------------------------------------------------
// Per-variant resolution helpers
// ---------------------------------------------------------------------------

fn resolve_shortcase_citation<R: Clone + Hash>(
    citation: &Citation,
    data: &ShortCaseCitationData,
    resolved_full_cites: &ResolvedFullCites<R>,
) -> Option<R> {
    let candidates: &[usize] = resolved_full_cites
        .by_reporter_volume
        .get(&reporter_volume(citation))
        .map_or(&[], Vec::as_slice);

    let reporter = citation.corrected_reporter();
    let reporter = reporter.as_deref();

    // Unique decision — return immediately.
    if let Some(resource) =
        decision_resource(resolved_full_cites, candidates.iter().copied(), reporter)
    {
        return Some(resource.clone());
    }

    // Attempt antecedent refinement.
    if let Some(ref antecedent) = data.metadata.antecedent_guess {
        return filter_by_matching_antecedent(
            resolved_full_cites,
            candidates.iter().copied(),
            antecedent,
            reporter,
        );
    }

    None
}

fn resolve_supra_citation<R: Clone + Hash>(
    data: &SupraCitationData,
    resolved_full_cites: &ResolvedFullCites<R>,
) -> Option<R> {
    let antecedent = data.metadata.antecedent_guess.as_ref()?;
    filter_by_matching_antecedent(
        resolved_full_cites,
        0..resolved_full_cites.cites.len(),
        antecedent,
        None,
    )
}

fn resolve_id_citation<R: Clone + Eq + Hash>(
    data: &IdCitationData,
    last_resolution: Option<&R>,
    resolutions: &Resolutions<R>,
) -> Option<R> {
    let last = last_resolution?;

    // Retrieve the full citation that created this resource.
    let full_cite_list = resolutions.get(last)?;
    let full_cite = full_cite_list.first()?;

    if has_invalid_pin_cite(full_cite, data) {
        return None;
    }

    Some(last.clone())
}

fn resolve_reference_citation<R: Clone + Hash>(
    data: &ReferenceCitationData,
    resolved_full_cites: &ResolvedFullCites<R>,
) -> Option<R> {
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

    // One decision cited in parallel is several resources; resolve to its
    // first citation, as for a supra (#67).
    decision_resource(resolved_full_cites, matches.into_iter(), None).cloned()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Filter the full citations at `candidates` by whether the full case
/// citation's defendant or plaintiff contains `antecedent_guess`. Returns
/// a resource only when the matches are one decision (see
/// [`decision_resource`]), preferring a citation in `reporter`.
fn filter_by_matching_antecedent<R: Clone + Hash>(
    resolved_full_cites: &ResolvedFullCites<R>,
    candidates: impl Iterator<Item = usize>,
    antecedent_guess: &str,
    reporter: Option<&str>,
) -> Option<R> {
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

    decision_resource(resolved_full_cites, matches, reporter).cloned()
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

    /// Each citation's type and the matched text of the full citation it
    /// resolves to, in document order.
    fn resolved(text: &str) -> Vec<(&'static str, Option<String>)> {
        let citations = crate::find::get_citations(text, false, "", &[]);
        let resolutions = resolve_citations(&citations);
        citations
            .iter()
            .map(|citation| {
                let resource = resolutions
                    .iter()
                    .find(|(_, cites)| cites.iter().any(|c| c.span() == citation.span()))
                    .map(|(resource, _)| resource.citation.matched_text().to_string());
                (citation.type_name(), resource)
            })
            .collect()
    }

    fn to(text: &str) -> Option<String> {
        Some(text.to_string())
    }

    // #60: a supra whose case name matches a decision cited in parallel
    // resolves to its first citation, and an id. after it follows.
    #[test]
    fn test_supra_after_parallel_state_cites() {
        let text = "Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). The court agreed. \
                    Smith, supra, at 46. Id. at 47.";
        assert_eq!(
            resolved(text),
            [
                ("FullCaseCitation", to("275 Conn. 38")),
                ("FullCaseCitation", to("881 A.2d 194")),
                ("SupraCitation", to("275 Conn. 38")),
                ("IdCitation", to("275 Conn. 38")),
            ]
        );
    }

    #[test]
    fn test_supra_after_parallel_federal_cites() {
        let text = "Roe v. Wade, 410 U.S. 113, 93 S. Ct. 705, 35 L. Ed. 2d 147 (1973). \
                    The court agreed. Roe, supra, at 120.";
        assert_eq!(
            resolved(text),
            [
                ("FullCaseCitation", to("410 U.S. 113")),
                ("FullCaseCitation", to("93 S. Ct. 705")),
                ("FullCaseCitation", to("35 L. Ed. 2d 147")),
                ("SupraCitation", to("410 U.S. 113")),
            ]
        );
    }

    // A short cite in the parallel reporter still resolves to its own
    // citation, as in eyecite.
    #[test]
    fn test_short_cite_after_parallel_cites_keeps_its_reporter() {
        let text = "Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). The court agreed. \
                    Smith, 881 A.2d at 199.";
        assert_eq!(resolved(text)[2], ("ShortCaseCitation", to("881 A.2d 194")));
    }

    // Connecticut opinions repeat the regional cite after each supra
    // ("Haynes v. Middletown, supra, 314 Conn. at 312, 101 A.3d 249"). The
    // repeated citation is the same decision as the first pair, so a later
    // supra still has one antecedent.
    #[test]
    fn test_supra_after_repeated_parallel_cite() {
        let text = "Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). The court \
                    agreed. Haynes v. Middletown, supra, 314 Conn. at 312, 101 A.3d 249. \
                    The court agreed again. Haynes, supra, at 313.";
        let resolved = resolved(text);
        assert_eq!(
            resolved.last(),
            Some(&("SupraCitation", to("314 Conn. 303"))),
            "{resolved:?}"
        );
        assert!(
            resolved.iter().all(|(_, resource)| resource.is_some()),
            "{resolved:?}"
        );
    }

    // A decision first cited in one reporter and later in parallel is still
    // one decision.
    #[test]
    fn test_supra_after_single_then_parallel_cite() {
        let text = "Smith v. Jones, 881 A.2d 194 (Conn. 2005). The court agreed. \
                    Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). Smith, supra, at 46.";
        assert_eq!(
            resolved(text).last(),
            Some(&("SupraCitation", to("881 A.2d 194")))
        );
    }

    // Two decisions with the same name leave a supra ambiguous.
    #[test]
    fn test_supra_after_two_cases_with_one_name_is_unresolved() {
        let text = "Smith v. Jones, 1 U.S. 1 (1990). The court agreed. See also \
                    Smith v. Jones, 410 U.S. 113 (1999). Smith, supra, at 3.";
        assert_eq!(
            resolved(text),
            [
                ("FullCaseCitation", to("1 U.S. 1")),
                ("FullCaseCitation", to("410 U.S. 113")),
                ("SupraCitation", None),
            ]
        );
    }

    // The same, when each decision is cited in parallel.
    #[test]
    fn test_supra_after_two_parallel_cases_with_one_name_is_unresolved() {
        let text = "Smith v. Jones, 275 Conn. 38, 881 A.2d 194 (2005). The court agreed. \
                    See also Smith v. Jones, 280 Conn. 1, 900 A.2d 1 (2006). \
                    Smith, supra, at 46.";
        assert_eq!(resolved(text).last(), Some(&("SupraCitation", None)));
    }

    // #67: a reference to a decision cited in parallel resolves to its
    // first citation, as a supra does (#60).
    #[test]
    fn test_reference_after_parallel_state_cites() {
        let text = "Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). The court \
                    agreed. In Haynes at 312, the court held that the statute applied.";
        assert_eq!(
            resolved(text),
            [
                ("FullCaseCitation", to("314 Conn. 303")),
                ("FullCaseCitation", to("101 A.3d 249")),
                ("ReferenceCitation", to("314 Conn. 303")),
            ]
        );
    }

    #[test]
    fn test_reference_after_parallel_federal_cites() {
        let text = "Ankenbrandt v. Richards, 504 U.S. 689, 112 S.Ct. 2206, 119 L.Ed.2d 468 \
                    (1992). The court agreed. Under Ankenbrandt at 703, the exception is narrow.";
        assert_eq!(
            resolved(text).last(),
            Some(&("ReferenceCitation", to("504 U.S. 689")))
        );
    }

    // Two decisions with the same name leave a reference ambiguous, cited
    // singly or in parallel.
    #[test]
    fn test_reference_after_two_parallel_cases_with_one_name_is_unresolved() {
        let text = "Haynes v. Middletown, 314 Conn. 303, 101 A.3d 249 (2014). The court \
                    agreed. See also Haynes v. Middletown, 320 Conn. 1, 120 A.3d 1 (2015). \
                    In Haynes at 312, the court held.";
        assert_eq!(resolved(text).last(), Some(&("ReferenceCitation", None)));
    }

    #[test]
    fn test_reference_after_two_cases_with_one_name_is_unresolved() {
        let text = "Haynes v. Middletown, 314 Conn. 303 (2014). The court agreed. See also \
                    Haynes v. Middletown, 320 Conn. 1 (2015). In Haynes at 312, the court held.";
        assert_eq!(resolved(text).last(), Some(&("ReferenceCitation", None)));
    }

    // A reference in markup, an emphasized name after the decision's
    // parallel citations.
    #[test]
    fn test_markup_reference_after_parallel_cites() {
        let markup = "<p><i>Haynes v. Middletown,</i> 314 Conn. 303, 101 A.3d 249 (2014). The \
                      court agreed. In <i>Haynes,</i> the court held that the statute applied.</p>";
        let citations = crate::find::get_citations(
            "",
            false,
            markup,
            &[crate::CleanStep::Html, crate::CleanStep::InlineWhitespace],
        );
        let resolutions = resolve_citations(&citations);
        let reference = citations
            .iter()
            .find(|c| matches!(c, Citation::Reference(_)))
            .expect("a reference citation");
        let resource = resolutions
            .iter()
            .find(|(_, cites)| cites.iter().any(|c| c.span() == reference.span()))
            .map(|(resource, _)| resource.citation.matched_text().to_string());
        assert_eq!(resource.as_deref(), Some("314 Conn. 303"));
    }

    // #78: an id. citation that stands for the case name of the full
    // citation after it is an id. of that citation (courtlistener-10pct/
    // 3759943, footnotes 3 and 4), not of Stanley v. Illinois before it.
    #[test]
    fn test_id_as_case_name_of_the_next_citation() {
        let text = "In re Murray (1990), 52 Ohio St.3d 155, 157; Stanley v. Illinois (1972), \
                    405 U.S. 645, 651. Id., (1990), 52 Ohio St.3d 155, 157, citing, Santosky v. \
                    Kramer (1982), 455 U.S. 745, 753.";
        assert_eq!(
            resolved(text),
            [
                ("FullCaseCitation", to("52 Ohio St.3d 155")),
                ("FullCaseCitation", to("405 U.S. 645")),
                ("IdCitation", to("52 Ohio St.3d 155")),
                ("FullCaseCitation", to("52 Ohio St.3d 155")),
                ("FullCaseCitation", to("455 U.S. 745")),
            ]
        );
    }

    // The same in markup, with the id. citation before the full one in its
    // group, as in the text.
    #[test]
    fn test_markup_id_as_case_name_keeps_text_order() {
        let markup = "<p><em>In re Murray</em> (1990), 52 Ohio St.3d 155, 157; <em>Stanley v. \
                      Illinois</em> (1972), 405 U.S. 645, 651.</p><p><em>Id.,</em> (1990), 52 Ohio \
                      St.3d 155, 157, citing, <em>Santosky v. Kramer</em> (1982), 455 U.S. 745, \
                      753.</p>";
        let citations = crate::find::get_citations(
            "",
            false,
            markup,
            &[crate::CleanStep::Html, crate::CleanStep::InlineWhitespace],
        );
        let resolutions = resolve_citations(&citations);
        let murray = resolutions
            .iter()
            .find(|(resource, _)| resource.citation.matched_text() == "52 Ohio St.3d 155")
            .map(|(_, cites)| cites.iter().map(|c| c.type_name()).collect::<Vec<_>>());
        assert_eq!(
            murray,
            Some(vec!["FullCaseCitation", "IdCitation", "FullCaseCitation"])
        );
    }

    /// What the one id. citation in `text` resolves to.
    fn id_resolution(text: &str) -> Option<String> {
        resolved(text)
            .into_iter()
            .find(|(kind, _)| *kind == "IdCitation")
            .and_then(|(_, resource)| resource)
    }

    // "Id." for the same case name before another decision in the same case
    // (courtlistener-10pct/5509821, 225404).
    #[test]
    fn test_id_as_case_name_of_a_later_decision() {
        let text = "Grain Cleaner Co. v. Smith, 110 N. Y. 83, 17 N. E. 671; Id., 34 Hun, 91.";
        assert_eq!(id_resolution(text), to("34 Hun, 91"));
        let text = "First National Benefit Society v. Stuart, 9 Cir., 134 F.2d 438; Id., 9 Cir., \
                    152 F.2d 298; Commissioner v. National Reserve Ins. Co., 7 Cir., 129 F.2d 1.";
        assert_eq!(id_resolution(text), to("152 F.2d 298"));
        let text = "Duckworth v. Thompson (Tex.Com.App.) 37 S.W.2d 731, 733; Id. (Tex.Civ.App.) \
                    22 S.W.2d 528, 530.";
        assert_eq!(id_resolution(text), to("22 S.W.2d 528"));
    }

    // An id. citation followed by another case's name is an id. of the
    // citation before it (courtlistener-10pct/2966729, 1937908).
    #[test]
    fn test_id_before_another_case_is_its_own() {
        let text = "Smith v. Jones, 1 U.S. 1 (1990). It may award whatever damages are incident \
                    to the complaint. Id. Soviero v. United States, 967 F.2d 791, 793 (2d Cir. \
                    1992).";
        assert_eq!(id_resolution(text), to("1 U.S. 1"));
        let text = "Smith v. Jones, 1 U.S. 1 (1990). We will strike the reply brief. Id., Jacober \
                    v. High Hill Realty, Inc., 22 Md. App. 115, 321 A.2d 838 (1974).";
        assert_eq!(id_resolution(text), to("1 U.S. 1"));
    }

    // A resolver of its own resolves each full citation once, and the id.
    // citation follows the citation it names.
    #[test]
    fn test_id_as_case_name_with_a_custom_resolver() {
        struct ById(Vec<String>);
        impl Resolver for ById {
            type Resource = String;
            fn resolve_full(&mut self, citation: &Citation) -> Option<String> {
                self.0.push(citation.matched_text().to_string());
                Some(format!("case:{}", citation.matched_text()))
            }
        }
        let text = "Grain Cleaner Co. v. Smith, 110 N. Y. 83, 17 N. E. 671; Id., 34 Hun, 91.";
        let citations = crate::find::get_citations(text, false, "", &[]);
        let mut resolver = ById(Vec::new());
        let groups = resolve_citations_with(&citations, &mut resolver);
        assert_eq!(resolver.0, ["110 N. Y. 83", "17 N. E. 671", "34 Hun, 91"]);
        let hun: Vec<_> = groups["case:34 Hun, 91"]
            .iter()
            .map(|c| c.type_name())
            .collect();
        assert_eq!(hun, ["IdCitation", "FullCaseCitation"]);
    }
}
