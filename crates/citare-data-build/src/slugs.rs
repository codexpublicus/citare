//! A stable slug for every case-reporter edition.
//!
//! An edition's slug is its Harvard Caselaw Access Project (CAP) slug when CAP
//! scanned the reporter, and otherwise one derived from the reporters-db
//! edition name: lowercased, with apostrophes dropped and every other run of
//! characters that aren't ASCII letters or digits turned into one hyphen
//! (`NY Slip Op` → `ny-slip-op`, `F.4th` → `f-4th`). CAP's and the derived
//! slugs share one namespace.
//!
//! Slugs are public: stare publishes its index under `<slug>/<volume>`, so
//! one must never change once released. Two reporters may share a slug when
//! their volumes never overlap: CAP's Ohio Reports (vols 1–20) and the Ohio
//! neutral citation (`2019-Ohio-1234`, volume 2019) are both `ohio`, and
//! their index paths are `ohio/20` and `ohio/2019`. [`check`] fails the build
//! when that can't be shown; see its rules.
//!
//! [`BY_REPORTER`] sets an edition's slug by hand where the rules need it,
//! keyed by reporters-db edition and reporter name, and [`SAME_BOOKS`] lists
//! slugs whose reporters-db reporters are one set of books.
//!
//! An edition's CAP slug (`harvard_slug`) follows from its slug
//! ([`cap_slug`]), or from [`CAP_SLUG_OF`] where a released slug names books
//! CAP files under another, and [`split_off`] says which editions sort after the one
//! that keeps a shared name's plain slug.

use std::collections::BTreeMap;

/// Slugs set by hand, as `(edition name, reporters-db reporter name, slug)`.
/// They split reporters that share an edition name but whose volumes overlap
/// (the second `Hill` is South Carolina's), give a name the CAP slug CAP files
/// it under (`Day.`), and put a neutral citation under the CAP reporter
/// with the same letters (`ME`, `me`). Never change or remove an entry once
/// released.
pub const BY_REPORTER: &[(&str, &str, &str)] = &[
    // CAP gives these short names to two reporters.
    (
        "Ct. Cl.",
        "Court of Claims Reports (United States)",
        "us-ct-cl",
    ),
    (
        "Ct. Cl.",
        "Report of the Court of Claims (West Virginia)",
        "wv-ct-cl",
    ),
    // CAP's numbered series (vols 1-15) and its annual one (vols 1980-2015),
    // which data/reporters-local.json adds as a reporter of its own.
    (
        "Mass. App. Div.",
        "Reports of Massachusetts Appellate Division",
        "mass-app-div",
    ),
    (
        "Mass. App. Div.",
        "Reports of Massachusetts Appellate Division (Annual)",
        "mass-app-div-annual",
    ),
    // reporters-db spellings of a CAP reporter.
    ("Day.", "Day's Reports (Connecticut)", "day"),
    (
        "Davis. L. Ct. Cas.",
        "Massachusetts Land Court decisions, 1898-1908",
        "davis-l-ct-cas",
    ),
    ("Ohio Law. Abs.", "Ohio Law Abstracts", "ohio-law-abs"),
    // Year-volume neutral citations under the CAP reporter with the same
    // letters, whose volumes all fall below 1700 (data/cap_volumes.json).
    // All but `ME` take a year only through data/reporters-local.json.
    ("LA", "Louisiana Neutral Citation", "la"),
    ("ME", "Maine Reports", "me"),
    ("ND", "North Dakota Neutral Citation", "nd"),
    ("NM", "New Mexico Neutral Citation", "nm"),
    ("PA", "Pennsylvania Neutral Citation", "pa"),
    ("SD", "South Dakota Neutral Citation", "sd"),
    ("VT", "Vermont Neutral Citation", "vt"),
    // Different reporters with one edition name. The CAP reporter, or the
    // one cited most, keeps the plain slug.
    (
        "Cranch",
        "District of Columbia Reports, Cranch",
        "cranch-dc",
    ),
    ("Hard.", "Hardesty Team Reports (Del.)", "hard-del"),
    ("Hayw.", "Tennessee Reports, Haywood", "hayw-tenn"),
    ("Hill", "South Carolina Reports, Hill", "hill-sc"),
    ("Ind. L. Rep.", "Indian Law Reporter", "ind-l-rep-indian"),
    ("Mart.", "North Carolina Reports, Martin", "mart-nc"),
    ("Met.", "Kentucky Reports, Metcalf", "met-ky"),
    ("Rob.", "Virginia Reports, Robinson", "rob-va"),
    (
        "Rob.",
        "Robertson's Reports (24-30 New York Superior)",
        "rob-ny",
    ),
    ("Sneed", "Kentucky Reports, Sneed", "sneed-ky"),
    ("Walker", "Mississippi Reports, Walker", "walker-miss"),
    ("Wall.", "Wallace's Circuit Court Reports", "wall-cc"),
    ("Wash.", "Virginia Reports, Washington", "wash-va"),
    ("Wilson", "Wilson's Reports (1-3 Oregon)", "wilson-or"),
    ("Wilson", "Wilson's Reports (48-59 Minn.)", "wilson-minn"),
    ("Woolw.", "Woolworth's Reports (1 Nebraska)", "woolw-neb"),
];

/// Slugs whose reporters-db reporters are one set of books listed twice,
/// so they share volumes by design, with the reason.
pub const SAME_BOOKS: &[(&str, &str)] = &[
    (
        "dall",
        "Dallas's Reports print Pennsylvania and U.S. Supreme Court cases in the same volumes",
    ),
    ("day", "`Day` and `Day.` are both Day's Connecticut Reports"),
    (
        "hun",
        "both entries are Hun's New York Supreme Court Reports",
    ),
    (
        "tread",
        "Treadway's Law Reports and Constitutional Reports are the same two volumes",
    ),
];

/// Released slugs whose books CAP files under another slug, as `(slug, CAP
/// slug, reason)`. An edition with one of these slugs keeps it, since slugs
/// never change once released, and takes the CAP slug as its `harvard_slug`
/// ([`cap_slug`]), so a citation of it still names CAP's volumes. Add an entry
/// only for a slug CAP has no volumes under and only when the two reporters
/// are one set of books.
pub const CAP_SLUG_OF: &[(&str, &str, &str)] = &[
    (
        "cai-r",
        "cai",
        "`Cai. R.` and `Cai.` are both Caines' Reports, vols 1-3",
    ),
    (
        "p-r-r",
        "pr",
        "`P.R.R.` and `P.R.` are both the Puerto Rico Reports, vols 1-100",
    ),
];

/// A numbered reporter shares a slug with a year-volume one only when its
/// highest volume is below this.
pub const FIRST_YEAR_VOLUME: u32 = 1700;

/// Whether `s` is a slug: `^[a-z0-9][a-z0-9-]*$`.
pub fn is_slug(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The derived slug for a reporters-db edition name, or `None` when the name
/// has no ASCII letter or digit to build one from.
pub fn derive(edition_name: &str) -> Option<String> {
    let mut slug = String::new();
    let mut gap = false;
    for c in edition_name.chars() {
        match c {
            '\'' | '\u{2019}' => {}
            c if c.is_ascii_alphanumeric() => {
                if gap && !slug.is_empty() {
                    slug.push('-');
                }
                gap = false;
                slug.push(c.to_ascii_lowercase());
            }
            _ => gap = true,
        }
    }
    (!slug.is_empty()).then_some(slug)
}

/// The slug for one edition of reporter `reporter_name`: its [`BY_REPORTER`]
/// entry, else its CAP slug, else the derived one.
pub fn assign(
    edition_name: &str,
    reporter_name: &str,
    harvard_slug: Option<&str>,
) -> Option<String> {
    BY_REPORTER
        .iter()
        .find(|(e, r, _)| *e == edition_name && *r == reporter_name)
        .map(|(_, _, s)| (*s).to_string())
        .or_else(|| harvard_slug.map(str::to_string))
        .or_else(|| derive(edition_name))
}

/// The CAP slug of an edition with slug `slug` whose citation regexes are
/// `regexes`: `slug` itself, or its [`CAP_SLUG_OF`] entry's CAP slug, when
/// that is a CAP slug (a key of `cap_volumes`, CAP slug → its first and last
/// volume) and the edition's volumes are of the kind CAP files under it,
/// years or numbers. Otherwise `None`.
///
/// So an edition's `harvard_slug` follows its `slug`: `P.R.R.` (`p-r-r`) is
/// CAP's `pr`, the federal `Ct. Cl.`
/// is `us-ct-cl`, not West Virginia's `wv-ct-cl`, and South Carolina's `Hill`
/// (`hill-sc`) has none. A year-volume neutral citation that shares a
/// numbered CAP reporter's slug (`2019-Ohio-1234`, slug `ohio`, CAP's Ohio
/// Reports, vols 1-20) has none either, since CAP has none of its volumes.
pub fn cap_slug(
    slug: &str,
    regexes: &[String],
    cap_volumes: &BTreeMap<String, (u32, u32)>,
) -> Option<String> {
    let cap = CAP_SLUG_OF
        .iter()
        .find(|(s, _, _)| *s == slug)
        .map_or(slug, |(_, c, _)| c);
    let &(first, _) = cap_volumes.get(cap)?;
    let years = volumes(regexes) == Volumes::Years;
    (years == (first >= FIRST_YEAR_VOLUME)).then(|| cap.to_string())
}

/// Whether an edition named `edition_name` with slug `slug` was split off a
/// name it shares with another reporter: its slug is neither a CAP slug (a
/// key of `cap_volumes`) nor the one derived from its name. These are the
/// [`BY_REPORTER`] entries for different reporters under one name
/// (`hill-sc`, `wash-va`); the reporter that keeps the plain slug is the CAP
/// one or the one cited most. The data bundle lists a split-off edition
/// after the others, so a citation citare can't narrow to one edition
/// (`119 Wash. at 299`) has the plain one first.
pub fn split_off(
    edition_name: &str,
    slug: &str,
    cap_volumes: &BTreeMap<String, (u32, u32)>,
) -> bool {
    !cap_volumes.contains_key(slug) && derive(edition_name).as_deref() != Some(slug)
}

/// One case-reporter edition with its slug, for [`check`].
#[derive(Debug, Clone)]
pub struct EditionSlug {
    pub name: String,
    pub reporter: String,
    pub slug: String,
    /// The edition's first and last years from reporters-db (`None` for an
    /// open end).
    pub start_year: Option<u32>,
    pub end_year: Option<u32>,
    /// The edition's citation regexes, variables substituted.
    pub regexes: Vec<String>,
}

/// The volumes an edition's regexes take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Volumes {
    /// Every regex takes only a year (no number below
    /// [`FIRST_YEAR_VOLUME`]).
    Years,
    /// Numbers up to this one, or any number (`None`).
    UpTo(Option<u32>),
}

/// The text of the `(?P<volume>...)` group in `regex`, if it has one.
fn volume_group(regex: &str) -> Option<&str> {
    let open = "(?P<volume>";
    let start = regex.find(open)? + open.len();
    let mut depth = 1usize;
    let mut escaped = false;
    for (i, c) in regex[start..].char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&regex[start..start + i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// What volumes `regexes` take. A regex without a volume group (`Wall.`'s
/// `$reporter,? p. $page`) adds none.
pub fn volumes(regexes: &[String]) -> Volumes {
    let mut years = true;
    let mut max: Option<u32> = Some(0);
    let mut any = false;
    for r in regexes {
        let Some(group) = volume_group(r) else {
            continue;
        };
        let Ok(re) = regex::Regex::new(&format!("^(?:{group})$")) else {
            return Volumes::UpTo(None);
        };
        any = true;
        if (1..FIRST_YEAR_VOLUME).any(|n| re.is_match(&n.to_string())) {
            years = false;
        }
        if re.is_match("10000") {
            max = None;
        } else if let Some(m) = max.as_mut() {
            let top = (1..10000u32).rev().find(|n| re.is_match(&n.to_string()));
            *m = (*m).max(top.unwrap_or(0));
        }
    }
    if !any {
        Volumes::UpTo(None)
    } else if years {
        Volumes::Years
    } else {
        Volumes::UpTo(max)
    }
}

/// The [`BY_REPORTER`] entries that name no edition in `editions`, which
/// means reporters-db renamed one and its slug fell back to another.
pub fn unmatched_by_reporter(editions: &[EditionSlug]) -> Vec<String> {
    BY_REPORTER
        .iter()
        .filter(|(n, r, _)| !editions.iter().any(|e| e.name == *n && e.reporter == *r))
        .map(|(n, r, s)| format!("BY_REPORTER entry {n:?} of {r:?} ({s:?}) names no edition"))
        .collect()
}

/// Check every edition's slug against CAP's reporters, as `(short name,
/// slug)`, and CAP's volume ranges, slug → `(first, last)`. Fails when:
///
/// 1. a derived slug is a CAP slug that CAP gives a reporter of another name;
/// 2. two edition names derive the same slug;
/// 3. two reporters share a slug and their volumes may overlap. Numbered
///    volumes start at 1, so two numbered reporters always may (unless
///    [`SAME_BOOKS`] lists the slug). A numbered reporter and a year-volume
///    one may unless the numbered one's highest volume is below
///    [`FIRST_YEAR_VOLUME`]: CAP's last volume when the slug is CAP's, else
///    the highest number its regexes take. Two year-volume reporters may
///    unless their reporters-db years are disjoint;
/// 4. CAP gives an edition's name to several reporters and [`BY_REPORTER`]
///    doesn't pick one;
///
/// or when a slug is malformed.
pub fn check(
    cap: &[(&str, &str)],
    cap_volumes: &BTreeMap<String, (u32, u32)>,
    editions: &[EditionSlug],
) -> Result<(), String> {
    let mut errors = Vec::new();
    let mut cap_names: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut cap_slugs: BTreeMap<&str, &str> = BTreeMap::new();
    for &(name, slug) in cap {
        cap_names.entry(name).or_default().push(slug);
        cap_slugs.entry(slug).or_insert(name);
    }

    let set_by_hand = |e: &EditionSlug| {
        BY_REPORTER
            .iter()
            .any(|(n, r, _)| *n == e.name && *r == e.reporter)
    };
    let mut derived: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut by_slug: BTreeMap<&str, Vec<&EditionSlug>> = BTreeMap::new();
    for e in editions {
        let (name, slug) = (e.name.as_str(), e.slug.as_str());
        if !is_slug(slug) {
            errors.push(format!("edition {name:?} has malformed slug {slug:?}"));
        }
        if cap_names.get(name).is_some_and(|s| s.len() > 1) && !set_by_hand(e) {
            errors.push(format!(
                "CAP gives {name:?} to several reporters ({:?}); add edition {name:?} of {:?} \
                 to citare_data_build::slugs::BY_REPORTER",
                cap_names[name], e.reporter
            ));
        }
        if !set_by_hand(e) && derive(name).as_deref() == Some(slug) {
            if let Some(cap_name) = cap_slugs.get(slug).filter(|n| **n != name) {
                errors.push(format!(
                    "edition {name:?} derives {slug:?}, CAP's slug for {cap_name:?}; \
                     set its slug in citare_data_build::slugs::BY_REPORTER"
                ));
            }
            let names = derived.entry(slug).or_default();
            if !names.contains(&name) {
                names.push(name);
            }
        }
        let group = by_slug.entry(slug).or_default();
        if !group
            .iter()
            .any(|g| g.name == e.name && g.reporter == e.reporter)
        {
            group.push(e);
        }
    }
    for (slug, names) in &derived {
        if names.len() > 1 {
            errors.push(format!(
                "editions {names:?} all derive slug {slug:?}; set all but one in \
                 citare_data_build::slugs::BY_REPORTER"
            ));
        }
    }

    for (slug, group) in &by_slug {
        if group.len() < 2 || SAME_BOOKS.iter().any(|(s, _)| s == slug) {
            continue;
        }
        let kinds: Vec<Volumes> = group
            .iter()
            .map(|e| match (volumes(&e.regexes), cap_volumes.get(*slug)) {
                (Volumes::UpTo(own), Some(&(_, last))) => {
                    Volumes::UpTo(Some(own.map_or(last, |o| o.max(last))))
                }
                (v, _) => v,
            })
            .collect();
        for i in 0..group.len() {
            for j in i + 1..group.len() {
                let (a, b) = (group[i], group[j]);
                let overlap = match (kinds[i], kinds[j]) {
                    (Volumes::Years, Volumes::Years) => {
                        let span = |e: &EditionSlug| {
                            (e.start_year.unwrap_or(0), e.end_year.unwrap_or(u32::MAX))
                        };
                        let ((s1, e1), (s2, e2)) = (span(a), span(b));
                        s1 <= e2 && s2 <= e1
                    }
                    (Volumes::Years, Volumes::UpTo(max)) | (Volumes::UpTo(max), Volumes::Years) => {
                        max.is_none_or(|m| m >= FIRST_YEAR_VOLUME)
                    }
                    (Volumes::UpTo(_), Volumes::UpTo(_)) => true,
                };
                if overlap {
                    errors.push(format!(
                        "{:?} of {:?} ({:?}) and {:?} of {:?} ({:?}) share slug {slug:?} but their \
                         volumes may overlap; split them in citare_data_build::slugs::BY_REPORTER",
                        a.name, a.reporter, kinds[i], b.name, b.reporter, kinds[j]
                    ));
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_from_edition_names() {
        for (name, slug) in [
            ("NY Slip Op", "ny-slip-op"),
            ("T.C. Memo.", "t-c-memo"),
            ("N.Y.S.3d", "n-y-s-3d"),
            ("A.D.3d", "a-d-3d"),
            ("F.4th", "f-4th"),
            ("IL App (1st)", "il-app-1st"),
            ("Ohio", "ohio"),
            ("F. App'x", "f-appx"),
            ("Commc\u{2019}ns Reg. (P & F)", "commcns-reg-p-f"),
        ] {
            assert_eq!(derive(name).as_deref(), Some(slug), "{name}");
            assert!(is_slug(slug));
        }
        assert_eq!(derive("§ ."), None);
    }

    #[test]
    fn hand_set_then_cap_then_derived() {
        assert_eq!(assign("A.2d", "R", Some("a2d")).as_deref(), Some("a2d"));
        assert_eq!(
            assign("Ohio", "Ohio Neutral Citation", Some("ohio")).as_deref(),
            Some("ohio")
        );
        assert_eq!(assign("F.4th", "R", None).as_deref(), Some("f-4th"));
        let us = "Court of Claims Reports (United States)";
        let wv = "Report of the Court of Claims (West Virginia)";
        assert_eq!(
            assign("Ct. Cl.", us, Some("wv-ct-cl")).as_deref(),
            Some("us-ct-cl")
        );
        assert_eq!(
            assign("Ct. Cl.", wv, Some("wv-ct-cl")).as_deref(),
            Some("wv-ct-cl")
        );
    }

    #[test]
    fn cap_slug_only_for_cap_volumes() {
        let caps = BTreeMap::from([
            ("ohio".to_string(), (1, 20)),
            ("us-ct-cl".to_string(), (1, 231)),
            ("mass-app-div-annual".to_string(), (1980, 2015)),
        ]);
        let r = |s: &str| vec![s.to_string()];
        // CAP's Ohio Reports keep `ohio`; the year-volume neutral doesn't.
        assert_eq!(cap_slug("ohio", &r(SMALL), &caps).as_deref(), Some("ohio"));
        assert_eq!(cap_slug("ohio", &r(YEAR), &caps), None);
        // Annual volumes under an annual CAP series, and not numbered ones.
        assert_eq!(
            cap_slug("mass-app-div-annual", &r(YEAR), &caps).as_deref(),
            Some("mass-app-div-annual")
        );
        assert_eq!(cap_slug("mass-app-div-annual", &r(SMALL), &caps), None);
        assert_eq!(
            cap_slug("us-ct-cl", &r(ANY), &caps).as_deref(),
            Some("us-ct-cl")
        );
        // A slug CAP doesn't have.
        assert_eq!(cap_slug("hill-sc", &r(ANY), &caps), None);
        // A released slug for books CAP files under another.
        let caps = BTreeMap::from([("pr".to_string(), (3, 100))]);
        assert_eq!(cap_slug("p-r-r", &r(ANY), &caps).as_deref(), Some("pr"));
        assert_eq!(cap_slug("p-r-r", &r(YEAR), &caps), None);
    }

    #[test]
    fn cap_slug_of_names_cap_slugs_only() {
        let caps: BTreeMap<String, (u32, u32)> =
            serde_json::from_str(include_str!("../../../data/cap_volumes.json")).unwrap();
        for (slug, cap, _) in CAP_SLUG_OF {
            assert!(is_slug(slug) && slug != cap, "{slug}");
            assert!(!caps.contains_key(*slug), "CAP has volumes under {slug}");
            assert!(caps.contains_key(*cap), "CAP has no {cap}");
        }
    }

    #[test]
    fn split_off_editions() {
        let caps = BTreeMap::from([
            ("wash".to_string(), (1, 200)),
            ("us-ct-cl".to_string(), (1, 231)),
        ]);
        assert!(split_off("Wash.", "wash-va", &caps));
        assert!(!split_off("Wash.", "wash", &caps));
        assert!(!split_off("Ct. Cl.", "us-ct-cl", &caps));
        assert!(!split_off("NY Slip Op", "ny-slip-op", &caps));
        // Every BY_REPORTER slug that is neither CAP's nor derived.
        let caps: BTreeMap<String, (u32, u32)> =
            serde_json::from_str(include_str!("../../../data/cap_volumes.json")).unwrap();
        let split: Vec<&str> = BY_REPORTER
            .iter()
            .filter(|(n, _, s)| split_off(n, s, &caps))
            .map(|(_, _, s)| *s)
            .collect();
        assert_eq!(split.len(), 16, "{split:?}");
    }

    #[test]
    fn slug_shape() {
        assert!(is_slug("a2d"));
        assert!(is_slug("ny-slip-op"));
        assert!(!is_slug(""));
        assert!(!is_slug("-a"));
        assert!(!is_slug("A2d"));
        assert!(!is_slug("a.2d"));
    }

    #[test]
    fn reads_volumes_from_regexes() {
        let r = |s: &str| vec![s.to_string()];
        assert_eq!(
            volumes(&r(r"(?P<volume>1[789]\d{2}|20\d{2}) Ohio (?P<page>\d+)")),
            Volumes::Years
        );
        assert_eq!(
            volumes(&r(r"(?P<volume>(19|20)\d{2}) ME (?P<page>\d+)")),
            Volumes::Years
        );
        assert_eq!(
            volumes(&r(r"(?P<volume>\d{1,3}) Ohio (?P<page>\d+)")),
            Volumes::UpTo(Some(999))
        );
        assert_eq!(
            volumes(&r(r"(?P<volume>[1-9]\d*) F\.4th (?P<page>\d+)")),
            Volumes::UpTo(None)
        );
        assert_eq!(
            volumes(&r(r"Wall\.,? p\. (?P<page>\d+)")),
            Volumes::UpTo(None)
        );
    }

    fn ed(
        name: &str,
        reporter: &str,
        slug: &str,
        regex: &str,
        years: (u32, Option<u32>),
    ) -> EditionSlug {
        EditionSlug {
            name: name.into(),
            reporter: reporter.into(),
            slug: slug.into(),
            start_year: Some(years.0),
            end_year: years.1,
            regexes: vec![regex.into()],
        }
    }

    const YEAR: &str = r"(?P<volume>1[789]\d{2}|20\d{2}) X (?P<page>\d+)";
    const SMALL: &str = r"(?P<volume>\d{1,3}) X (?P<page>\d+)";
    const ANY: &str = r"(?P<volume>[1-9]\d*) X (?P<page>\d+)";

    #[test]
    fn shared_slugs_need_disjoint_volumes() {
        let caps = BTreeMap::from([("ohio".to_string(), (1, 20))]);
        let cap = [("Ohio", "ohio")];
        // Ohio Reports (CAP, vols 1-20) and the Ohio neutral share `ohio`.
        let ohio = [
            ed("Ohio", "Ohio Reports", "ohio", ANY, (1821, Some(1851))),
            ed("Ohio", "Ohio Neutral Citation", "ohio", YEAR, (2002, None)),
        ];
        assert!(check(&cap, &caps, &ohio).is_ok());
        // Without CAP's range, a numbered reporter with any volume overlaps.
        assert!(check(&[], &BTreeMap::new(), &ohio).is_err());
        let bounded = [
            ed("Q", "Q Reports", "q", SMALL, (1821, Some(1851))),
            ed("Q", "Q Neutral", "q", YEAR, (2002, None)),
        ];
        assert!(check(&[], &BTreeMap::new(), &bounded).is_ok());
        // Two numbered reporters always may overlap.
        let numbered = [
            ed("Q", "Q Reports", "q", SMALL, (1821, Some(1851))),
            ed("Q", "Other Q Reports", "q", SMALL, (1900, Some(1910))),
        ];
        assert!(check(&[], &BTreeMap::new(), &numbered).is_err());
        // Two year-volume reporters overlap when their years do.
        let years = |end| {
            [
                ed("Q", "Q Neutral", "q", YEAR, (2000, Some(2010))),
                ed("Q", "Other Q Neutral", "q", YEAR, (end, None)),
            ]
        };
        assert!(check(&[], &BTreeMap::new(), &years(2011)).is_ok());
        assert!(check(&[], &BTreeMap::new(), &years(2010)).is_err());
    }

    #[test]
    fn rejects_cap_slugs_and_collisions() {
        let none = BTreeMap::new();
        // `ND` derives `nd`, CAP's slug for `N.D.`.
        let err = check(
            &[("N.D.", "nd")],
            &none,
            &[ed("ND", "R", "nd", YEAR, (1997, None))],
        )
        .unwrap_err();
        assert!(err.contains("CAP's slug"), "{err}");
        // `F. 4th` and `F.4th` both derive `f-4th`.
        let err = check(
            &[],
            &none,
            &[
                ed("F. 4th", "R", "f-4th", ANY, (2021, None)),
                ed("F.4th", "S", "f-4th", ANY, (2021, None)),
            ],
        )
        .unwrap_err();
        assert!(err.contains("all derive"), "{err}");
        assert!(check(&[], &none, &[ed("X", "R", "Bad Slug", ANY, (1900, None))]).is_err());
        // CAP gives `Ct. Cl.` to two reporters; BY_REPORTER must pick.
        let cap = [("Ct. Cl.", "us-ct-cl"), ("Ct. Cl.", "wv-ct-cl")];
        assert!(check(
            &cap,
            &none,
            &[ed("Ct. Cl.", "Other", "wv-ct-cl", ANY, (1900, None))]
        )
        .is_err());
    }
}
