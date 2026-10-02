use std::collections::HashMap;

use sha2::{Digest, Sha256};

use super::edition::Edition;
use super::token::Token;

// ---------------------------------------------------------------------------
// Common data
// ---------------------------------------------------------------------------

/// Fields shared by every citation variant.
#[derive(Debug, Clone)]
pub struct CitationCommon {
    pub token: Token,
    pub index: usize,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
    pub full_span_start: Option<usize>,
    pub full_span_end: Option<usize>,
    pub groups: HashMap<String, Option<String>>,
}

/// Fields shared by all resource citations (case, law, journal).
#[derive(Debug, Clone)]
pub struct ResourceCitationCommon {
    pub common: CitationCommon,
    pub exact_editions: Vec<Edition>,
    pub variation_editions: Vec<Edition>,
    pub all_editions: Vec<Edition>,
    pub edition_guess: Option<Edition>,
    pub year: Option<i32>,
}

// ---------------------------------------------------------------------------
// Per-variant metadata
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct CaseMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub court: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct FullCaseMetadata {
    pub case: CaseMetadata,
    pub plaintiff: Option<String>,
    pub defendant: Option<String>,
    pub extra: Option<String>,
    pub antecedent_guess: Option<String>,
    pub resolved_case_name_short: Option<String>,
    pub resolved_case_name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ShortCaseMetadata {
    pub case: CaseMetadata,
    pub antecedent_guess: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LawMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct JournalMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SupraMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub antecedent_guess: Option<String>,
    pub volume: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct IdMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct ReferenceMetadata {
    pub parenthetical: Option<String>,
    pub pin_cite: Option<String>,
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub plaintiff: Option<String>,
    pub defendant: Option<String>,
    pub resolved_case_name_short: Option<String>,
    pub resolved_case_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Per-variant data structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FullCaseCitationData {
    pub resource: ResourceCitationCommon,
    pub metadata: FullCaseMetadata,
}

#[derive(Debug, Clone)]
pub struct ShortCaseCitationData {
    pub resource: ResourceCitationCommon,
    pub metadata: ShortCaseMetadata,
}

#[derive(Debug, Clone)]
pub struct FullLawCitationData {
    pub resource: ResourceCitationCommon,
    pub metadata: LawMetadata,
}

#[derive(Debug, Clone)]
pub struct FullJournalCitationData {
    pub resource: ResourceCitationCommon,
    pub metadata: JournalMetadata,
}

#[derive(Debug, Clone)]
pub struct SupraCitationData {
    pub common: CitationCommon,
    pub metadata: SupraMetadata,
}

#[derive(Debug, Clone)]
pub struct IdCitationData {
    pub common: CitationCommon,
    pub metadata: IdMetadata,
}

#[derive(Debug, Clone)]
pub struct ReferenceCitationData {
    pub common: CitationCommon,
    pub metadata: ReferenceMetadata,
}

#[derive(Debug, Clone)]
pub struct UnknownCitationData {
    pub common: CitationCommon,
}

// ---------------------------------------------------------------------------
// The main Citation enum
// ---------------------------------------------------------------------------

/// A legal citation extracted from text.
///
/// Each variant corresponds to a distinct citation form:
///
/// | Variant | Example |
/// |---------|---------|
/// | `FullCase` | `Foo v. Bar, 1 U.S. 1 (1999)` |
/// | `ShortCase` | `Foo, 1 U.S., at 2` |
/// | `FullLaw` | `Mass. Gen. Laws ch. 1, § 2` |
/// | `FullJournal` | `1 Minn. L. Rev. 1` |
/// | `Supra` | `Foo, supra, at 2` |
/// | `Id` | `Id. at 2` |
/// | `Reference` | A bare party-name reference resolved back to a full case |
/// | `Unknown` | A section marker (`§`) whose citation context could not be determined |
#[derive(Debug, Clone)]
pub enum Citation {
    /// A full case citation with reporter, volume, and page (e.g. `1 U.S. 1 (1999)`).
    FullCase(FullCaseCitationData),
    /// A short-form case citation referencing a previously-cited case (e.g. `Foo, 1 U.S., at 2`).
    ShortCase(ShortCaseCitationData),
    /// A full statutory citation (e.g. `Mass. Gen. Laws ch. 1, § 2`).
    FullLaw(FullLawCitationData),
    /// A full law-review journal citation (e.g. `1 Minn. L. Rev. 1`).
    FullJournal(FullJournalCitationData),
    /// A *supra* short-form citation (e.g. `Foo, supra, at 2`).
    Supra(SupraCitationData),
    /// An *id.* or *ibid.* citation referring to the immediately preceding citation.
    Id(IdCitationData),
    /// A bare party-name reference resolved back to a full case citation.
    Reference(ReferenceCitationData),
    /// A section marker (`§`) whose surrounding context could not be resolved.
    Unknown(UnknownCitationData),
}

// ---------------------------------------------------------------------------
// Methods
// ---------------------------------------------------------------------------

impl Citation {
    /// Return the common fields shared by every citation.
    pub fn common(&self) -> &CitationCommon {
        match self {
            Citation::FullCase(d) => &d.resource.common,
            Citation::ShortCase(d) => &d.resource.common,
            Citation::FullLaw(d) => &d.resource.common,
            Citation::FullJournal(d) => &d.resource.common,
            Citation::Supra(d) => &d.common,
            Citation::Id(d) => &d.common,
            Citation::Reference(d) => &d.common,
            Citation::Unknown(d) => &d.common,
        }
    }

    /// Return a mutable reference to the common fields.
    pub fn common_mut(&mut self) -> &mut CitationCommon {
        match self {
            Citation::FullCase(d) => &mut d.resource.common,
            Citation::ShortCase(d) => &mut d.resource.common,
            Citation::FullLaw(d) => &mut d.resource.common,
            Citation::FullJournal(d) => &mut d.resource.common,
            Citation::Supra(d) => &mut d.common,
            Citation::Id(d) => &mut d.common,
            Citation::Reference(d) => &mut d.common,
            Citation::Unknown(d) => &mut d.common,
        }
    }

    /// The raw matched text of the citation token.
    pub fn matched_text(&self) -> &str {
        self.common().token.data()
    }

    /// The (start, end) byte span of this citation in the source text.
    ///
    /// Falls back to the token's own span when no explicit override is set.
    pub fn span(&self) -> (usize, usize) {
        let c = self.common();
        let start = c.span_start.unwrap_or_else(|| c.token.start());
        let end = c.span_end.unwrap_or_else(|| c.token.end());
        (start, end)
    }

    /// The full span including case names, post-citation clauses, etc.
    ///
    /// Falls back to `span()` when no explicit override is set.
    pub fn full_span(&self) -> (usize, usize) {
        let c = self.common();
        let (span_start, span_end) = self.span();
        let start = c.full_span_start.unwrap_or(span_start);
        let end = c.full_span_end.unwrap_or(span_end);
        (start, end)
    }

    /// Returns `true` for citations that identify a resource (full citations).
    pub fn is_full(&self) -> bool {
        matches!(
            self,
            Citation::FullCase(_) | Citation::FullLaw(_) | Citation::FullJournal(_)
        )
    }

    /// Returns `true` for full or short case citations.
    pub fn is_case(&self) -> bool {
        matches!(self, Citation::FullCase(_) | Citation::ShortCase(_))
    }

    /// Returns `true` if this is a resource citation with multiple possible
    /// editions that could not be narrowed down to a single guess.
    ///
    /// This happens when the reporter string matches multiple editions
    /// (e.g., different series) and the year (if any) doesn't disambiguate.
    /// eyecite silently drops these when `remove_ambiguous=true`.
    /// citare preserves them with this flag so consumers can decide.
    pub fn is_ambiguous(&self) -> bool {
        if let Some(rc) = self.resource_common() {
            rc.edition_guess.is_none() && rc.all_editions.len() > 1
        } else {
            false
        }
    }

    /// Returns the `ResourceCitationCommon` if this is a resource citation.
    pub fn resource_common(&self) -> Option<&ResourceCitationCommon> {
        match self {
            Citation::FullCase(d) => Some(&d.resource),
            Citation::ShortCase(d) => Some(&d.resource),
            Citation::FullLaw(d) => Some(&d.resource),
            Citation::FullJournal(d) => Some(&d.resource),
            _ => None,
        }
    }

    /// Return the corrected reporter short name.
    ///
    /// Prefers `edition_guess.short_name` when available; falls back to the
    /// `"reporter"` capture group from the token.
    pub fn corrected_reporter(&self) -> Option<String> {
        self.resource_common().map(|r| {
            r.edition_guess
                .as_ref()
                .map(|e| e.short_name.clone())
                .unwrap_or_else(|| {
                    r.common
                        .groups
                        .get("reporter")
                        .and_then(|v| v.clone())
                        .unwrap_or_default()
                })
        })
    }

    /// A stable string identifier for the concrete citation type.
    pub fn type_name(&self) -> &'static str {
        match self {
            Citation::FullCase(_) => "FullCaseCitation",
            Citation::ShortCase(_) => "ShortCaseCitation",
            Citation::FullLaw(_) => "FullLawCitation",
            Citation::FullJournal(_) => "FullJournalCitation",
            Citation::Supra(_) => "SupraCitation",
            Citation::Id(_) => "IdCitation",
            Citation::Reference(_) => "ReferenceCitation",
            Citation::Unknown(_) => "UnknownCitation",
        }
    }
}

// ---------------------------------------------------------------------------
// Hash / Eq – mirrors eyecite's __hash__ semantics
// ---------------------------------------------------------------------------

/// Hash `groups` + `all_editions` for a resource citation into `hasher`.
fn hash_groups_and_editions(hasher: &mut Sha256, r: &ResourceCitationCommon) {
    let groups = &r.common.groups;
    let mut sorted_groups: Vec<_> = groups.iter().collect();
    sorted_groups.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in sorted_groups {
        hasher.update(format!("{}:{}", k, v.as_deref().unwrap_or("")));
    }
    let mut editions: Vec<&str> = r
        .all_editions
        .iter()
        .map(|e| e.short_name.as_str())
        .collect();
    editions.sort_unstable();
    for e in editions {
        hasher.update(format!("edition:{}", e));
    }
}

impl std::hash::Hash for Citation {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            // Case citations: hash by volume + corrected_reporter + page.
            // If page is absent the citation is treated as unique.
            Citation::FullCase(_) | Citation::ShortCase(_) => {
                let groups = &self.common().groups;
                let page = groups.get("page").and_then(|v| v.as_ref());
                if page.is_none() {
                    // Unique — use object address as discriminant.
                    (self as *const Self).hash(state);
                } else {
                    let mut hasher = Sha256::new();
                    if let Some(Some(vol)) = groups.get("volume") {
                        hasher.update(format!("volume:{}", vol));
                    }
                    hasher.update(format!("page:{}", page.unwrap()));
                    if let Some(reporter) = self.corrected_reporter() {
                        hasher.update(format!("reporter:{}", reporter));
                    }
                    hasher.update(format!("class:{}", self.type_name()));
                    hasher.finalize().as_slice().hash(state);
                }
            }

            // Id and Unknown are always unique.
            Citation::Id(_) | Citation::Unknown(_) => {
                (self as *const Self).hash(state);
            }

            // Law and journal: hash by groups + all_editions.
            Citation::FullLaw(d) => {
                let mut hasher = Sha256::new();
                hash_groups_and_editions(&mut hasher, &d.resource);
                hasher.update("class:FullLawCitation");
                hasher.finalize().as_slice().hash(state);
            }
            Citation::FullJournal(d) => {
                let mut hasher = Sha256::new();
                hash_groups_and_editions(&mut hasher, &d.resource);
                hasher.update("class:FullJournalCitation");
                hasher.finalize().as_slice().hash(state);
            }

            // Supra and Reference: hash by groups.
            Citation::Supra(_) | Citation::Reference(_) => {
                let mut hasher = Sha256::new();
                let groups = &self.common().groups;
                let mut sorted_groups: Vec<_> = groups.iter().collect();
                sorted_groups.sort_by_key(|(k, _)| k.as_str());
                for (k, v) in sorted_groups {
                    hasher.update(format!("{}:{}", k, v.as_deref().unwrap_or("")));
                }
                hasher.update(format!("class:{}", self.type_name()));
                hasher.finalize().as_slice().hash(state);
            }
        }
    }
}

impl PartialEq for Citation {
    fn eq(&self, other: &Self) -> bool {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        let mut h2 = DefaultHasher::new();
        self.hash(&mut h1);
        other.hash(&mut h2);
        h1.finish() == h2.finish()
    }
}

impl Eq for Citation {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::edition::{Reporter, ReporterSource};
    use crate::models::token::{CitationTokenData, TokenData};

    fn make_reporter() -> Reporter {
        Reporter::new(
            "U.S.".to_string(),
            "United States Reports".to_string(),
            "federal".to_string(),
            ReporterSource::Reporters,
        )
    }

    fn make_token(data: &str, start: usize, end: usize) -> Token {
        Token::Citation(CitationTokenData {
            token_data: TokenData {
                data: data.to_string(),
                start,
                end,
                groups: HashMap::new(),
            },
            exact_editions: vec![],
            variation_editions: vec![],
            short: false,
        })
    }

    fn make_common(token: Token) -> CitationCommon {
        CitationCommon {
            token,
            index: 0,
            span_start: None,
            span_end: None,
            full_span_start: None,
            full_span_end: None,
            groups: HashMap::new(),
        }
    }

    fn make_resource(token: Token) -> ResourceCitationCommon {
        ResourceCitationCommon {
            common: make_common(token),
            exact_editions: vec![],
            variation_editions: vec![],
            all_editions: vec![],
            edition_guess: None,
            year: None,
        }
    }

    fn make_full_case(data: &str, start: usize, end: usize) -> Citation {
        Citation::FullCase(FullCaseCitationData {
            resource: make_resource(make_token(data, start, end)),
            metadata: FullCaseMetadata::default(),
        })
    }

    fn make_id(data: &str) -> Citation {
        Citation::Id(IdCitationData {
            common: make_common(make_token(data, 0, data.len())),
            metadata: IdMetadata::default(),
        })
    }

    // --- type_name ---

    #[test]
    fn test_type_name_full_case() {
        let c = make_full_case("1 U.S. 1", 0, 8);
        assert_eq!(c.type_name(), "FullCaseCitation");
    }

    #[test]
    fn test_type_name_all_variants() {
        let token = make_token("x", 0, 1);
        let common = make_common(token.clone());
        let resource = make_resource(token.clone());

        let variants: Vec<(&'static str, Citation)> = vec![
            (
                "FullCaseCitation",
                Citation::FullCase(FullCaseCitationData {
                    resource: make_resource(token.clone()),
                    metadata: Default::default(),
                }),
            ),
            (
                "ShortCaseCitation",
                Citation::ShortCase(ShortCaseCitationData {
                    resource: make_resource(token.clone()),
                    metadata: Default::default(),
                }),
            ),
            (
                "FullLawCitation",
                Citation::FullLaw(FullLawCitationData {
                    resource: make_resource(token.clone()),
                    metadata: Default::default(),
                }),
            ),
            (
                "FullJournalCitation",
                Citation::FullJournal(FullJournalCitationData {
                    resource,
                    metadata: Default::default(),
                }),
            ),
            (
                "SupraCitation",
                Citation::Supra(SupraCitationData {
                    common: common.clone(),
                    metadata: Default::default(),
                }),
            ),
            (
                "IdCitation",
                Citation::Id(IdCitationData {
                    common: common.clone(),
                    metadata: Default::default(),
                }),
            ),
            (
                "ReferenceCitation",
                Citation::Reference(ReferenceCitationData {
                    common: common.clone(),
                    metadata: Default::default(),
                }),
            ),
            (
                "UnknownCitation",
                Citation::Unknown(UnknownCitationData { common }),
            ),
        ];

        for (expected, c) in variants {
            assert_eq!(c.type_name(), expected);
        }
    }

    // --- span / full_span ---

    #[test]
    fn test_span_falls_back_to_token() {
        let c = make_full_case("1 U.S. 1", 10, 18);
        assert_eq!(c.span(), (10, 18));
    }

    #[test]
    fn test_span_override() {
        let mut data = FullCaseCitationData {
            resource: make_resource(make_token("1 U.S. 1", 10, 18)),
            metadata: FullCaseMetadata::default(),
        };
        data.resource.common.span_start = Some(5);
        data.resource.common.span_end = Some(20);
        let c = Citation::FullCase(data);
        assert_eq!(c.span(), (5, 20));
    }

    #[test]
    fn test_full_span_falls_back_to_span() {
        let c = make_full_case("1 U.S. 1", 10, 18);
        assert_eq!(c.full_span(), (10, 18));
    }

    #[test]
    fn test_full_span_override() {
        let mut data = FullCaseCitationData {
            resource: make_resource(make_token("1 U.S. 1", 10, 18)),
            metadata: FullCaseMetadata::default(),
        };
        data.resource.common.full_span_start = Some(0);
        data.resource.common.full_span_end = Some(30);
        let c = Citation::FullCase(data);
        assert_eq!(c.full_span(), (0, 30));
    }

    // --- is_full / is_case ---

    #[test]
    fn test_is_full() {
        let token = make_token("x", 0, 1);
        assert!(Citation::FullCase(FullCaseCitationData {
            resource: make_resource(token.clone()),
            metadata: Default::default()
        })
        .is_full());
        assert!(Citation::FullLaw(FullLawCitationData {
            resource: make_resource(token.clone()),
            metadata: Default::default()
        })
        .is_full());
        assert!(!make_id("id.").is_full());
    }

    #[test]
    fn test_is_case() {
        let token = make_token("x", 0, 1);
        assert!(Citation::FullCase(FullCaseCitationData {
            resource: make_resource(token.clone()),
            metadata: Default::default()
        })
        .is_case());
        assert!(Citation::ShortCase(ShortCaseCitationData {
            resource: make_resource(token.clone()),
            metadata: Default::default()
        })
        .is_case());
        assert!(!Citation::FullLaw(FullLawCitationData {
            resource: make_resource(token.clone()),
            metadata: Default::default()
        })
        .is_case());
    }

    // --- corrected_reporter ---

    #[test]
    fn test_corrected_reporter_from_edition_guess() {
        let edition = Edition {
            reporter: make_reporter(),
            short_name: "U.S.".to_string(),
            start: None,
            end: None,
            harvard_slug: None,
        };
        let mut resource = make_resource(make_token("1 U.S. 1", 0, 8));
        resource.edition_guess = Some(edition);
        let c = Citation::FullCase(FullCaseCitationData {
            resource,
            metadata: FullCaseMetadata::default(),
        });
        assert_eq!(c.corrected_reporter(), Some("U.S.".to_string()));
    }

    #[test]
    fn test_corrected_reporter_from_groups() {
        let mut resource = make_resource(make_token("1 U.S. 1", 0, 8));
        resource
            .common
            .groups
            .insert("reporter".to_string(), Some("U.S.".to_string()));
        let c = Citation::FullCase(FullCaseCitationData {
            resource,
            metadata: FullCaseMetadata::default(),
        });
        assert_eq!(c.corrected_reporter(), Some("U.S.".to_string()));
    }

    // --- Id citations are always unique ---

    #[test]
    fn test_id_citations_not_equal() {
        let c1 = make_id("id.");
        let c2 = make_id("id.");
        // Id citations are pointer-unique, so two separately created ones differ.
        assert_ne!(c1, c2);
    }

    // --- matched_text ---

    #[test]
    fn test_matched_text() {
        let c = make_full_case("410 U.S. 113", 0, 12);
        assert_eq!(c.matched_text(), "410 U.S. 113");
    }
}
