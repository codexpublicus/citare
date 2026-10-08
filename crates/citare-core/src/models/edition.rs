use std::fmt;

/// A top-level reporter from reporters-db, like "S.W." or "U.S."
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reporter {
    pub short_name: String,
    pub name: String,
    pub cite_type: String,
    pub source: ReporterSource,
    pub is_scotus: bool,
    /// The jurisdiction of a law from laws.json (`New York`, `United
    /// States`); `None` for reporters and journals.
    pub jurisdiction: Option<String>,
}

/// The source database a reporter comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReporterSource {
    Reporters,
    Laws,
    Journals,
}

impl Reporter {
    pub fn new(
        short_name: String,
        name: String,
        cite_type: String,
        source: ReporterSource,
    ) -> Self {
        let is_scotus = (cite_type == "federal" && name.to_lowercase().contains("supreme"))
            || cite_type.to_lowercase().contains("scotus");
        Self {
            short_name,
            name,
            cite_type,
            source,
            is_scotus,
            jurisdiction: None,
        }
    }

    /// Set the jurisdiction (laws only).
    pub fn with_jurisdiction(mut self, jurisdiction: Option<String>) -> Self {
        self.jurisdiction = jurisdiction;
        self
    }
}

/// An individual edition of a reporter, e.g. "S.W." and "S.W.2d".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edition {
    pub reporter: Reporter,
    /// The edition's abbreviated name as it appears in a citation.
    pub short_name: String,
    /// ISO-8601 date string (YYYY-...) or None if unknown.
    pub start: Option<String>,
    /// ISO-8601 date string (YYYY-...) or None if open-ended.
    pub end: Option<String>,
    /// Harvard Caselaw Access Project reporter slug (e.g. "us", "a2d"), if known.
    pub harvard_slug: Option<String>,
}

impl Edition {
    /// Return true if this edition contains cases for the given year.
    pub fn includes_year(&self, year: i32) -> bool {
        let current_year = 2026_i32;
        if year > current_year {
            return false;
        }
        if let Some(ref start) = self.start {
            if let Some(start_year) = Self::extract_year(start) {
                if start_year > year {
                    return false;
                }
            }
        }
        if let Some(ref end) = self.end {
            if let Some(end_year) = Self::extract_year(end) {
                if end_year < year {
                    return false;
                }
            }
        }
        true
    }

    fn extract_year(date_str: &str) -> Option<i32> {
        date_str.get(..4)?.parse().ok()
    }
}

impl fmt::Display for Edition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_name)
    }
}

/// De-duplicate editions while preserving order and genuine ambiguity.
///
/// Mirrors eyecite's `_dedupe_editions` (eyecite #317). One edition can be
/// reached both as an exact match and as a variation, e.g. when the relaxed
/// reporter whitespace makes both `F.3d` and its variation `F. 3d` match the
/// same text, or when a custom regex omits `$edition` so its canonical and
/// variation forms are identical. Each list is de-duplicated, and variation
/// editions that are already exact matches are dropped, so distinct editions
/// (like the two `B.R.` reporters) are kept but none is listed twice.
pub fn dedupe_editions(exact: &mut Vec<Edition>, variation: &mut Vec<Edition>) {
    let mut seen: Vec<Edition> = Vec::with_capacity(exact.len() + variation.len());
    exact.retain(|e| {
        if seen.contains(e) {
            false
        } else {
            seen.push(e.clone());
            true
        }
    });
    variation.retain(|e| {
        if seen.contains(e) {
            false
        } else {
            seen.push(e.clone());
            true
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dedupe_editions() {
        let a = make_edition(Some("1900"), None);
        let b = make_edition(Some("1950"), None);
        let mut exact = vec![a.clone(), a.clone(), b.clone()];
        let mut variation = vec![b.clone(), a.clone(), b.clone()];
        dedupe_editions(&mut exact, &mut variation);
        assert_eq!(exact, vec![a.clone(), b.clone()]);
        assert!(variation.is_empty());

        let mut exact = vec![];
        let mut variation = vec![b.clone(), a.clone(), b.clone()];
        dedupe_editions(&mut exact, &mut variation);
        assert_eq!(variation, vec![b, a]);
    }

    fn make_reporter(cite_type: &str, name: &str) -> Reporter {
        Reporter::new(
            "U.S.".to_string(),
            name.to_string(),
            cite_type.to_string(),
            ReporterSource::Reporters,
        )
    }

    fn make_edition(start: Option<&str>, end: Option<&str>) -> Edition {
        Edition {
            reporter: make_reporter("state", "Southwest Reporter"),
            short_name: "S.W.".to_string(),
            start: start.map(String::from),
            end: end.map(String::from),
            harvard_slug: None,
        }
    }

    // --- Reporter::new / is_scotus ---

    #[test]
    fn test_scotus_via_federal_supreme() {
        let r = make_reporter("federal", "United States Supreme Court Reports");
        assert!(r.is_scotus);
    }

    #[test]
    fn test_scotus_via_cite_type_scotus() {
        let r = make_reporter("scotus", "Some Reporter");
        assert!(r.is_scotus);
    }

    #[test]
    fn test_not_scotus_state() {
        let r = make_reporter("state", "Southwest Reporter");
        assert!(!r.is_scotus);
    }

    #[test]
    fn test_not_scotus_federal_no_supreme() {
        let r = make_reporter("federal", "Federal Reporter");
        assert!(!r.is_scotus);
    }

    // --- Edition::includes_year ---

    #[test]
    fn test_includes_year_no_bounds() {
        let e = make_edition(None, None);
        assert!(e.includes_year(1900));
        assert!(e.includes_year(2020));
    }

    #[test]
    fn test_includes_year_future_rejected() {
        let e = make_edition(None, None);
        assert!(!e.includes_year(3000));
    }

    #[test]
    fn test_includes_year_start_bound() {
        let e = make_edition(Some("1950-01-01"), None);
        assert!(!e.includes_year(1940));
        assert!(e.includes_year(1950));
        assert!(e.includes_year(2000));
    }

    #[test]
    fn test_includes_year_end_bound() {
        let e = make_edition(None, Some("1980-12-31"));
        assert!(e.includes_year(1980));
        assert!(!e.includes_year(1981));
    }

    #[test]
    fn test_includes_year_start_and_end() {
        let e = make_edition(Some("1950-01-01"), Some("1970-12-31"));
        assert!(!e.includes_year(1940));
        assert!(e.includes_year(1960));
        assert!(!e.includes_year(1975));
    }

    #[test]
    fn test_edition_display() {
        let e = make_edition(None, None);
        assert_eq!(e.to_string(), "S.W.");
    }

    #[test]
    fn test_edition_harvard_slug_none_by_default() {
        let e = make_edition(None, None);
        assert_eq!(e.harvard_slug, None);
    }

    #[test]
    fn test_edition_harvard_slug_some() {
        let mut e = make_edition(None, None);
        e.harvard_slug = Some("sw".to_string());
        assert_eq!(e.harvard_slug, Some("sw".to_string()));
    }
}
