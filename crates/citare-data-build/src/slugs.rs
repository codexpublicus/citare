//! A stable slug for every case-reporter edition.
//!
//! An edition's slug is its Harvard Caselaw Access Project (CAP) slug when CAP
//! scanned the reporter, and otherwise one derived from the reporters-db
//! edition name: [`DERIVED_PREFIX`] followed by the name lowercased, with
//! apostrophes dropped and every other run of characters that aren't ASCII
//! letters or digits turned into one hyphen (`NY Slip Op` → `rdb-ny-slip-op`,
//! `F.4th` → `rdb-f-4th`).
//!
//! Slugs are public: stare publishes its index under `<slug>/<volume>`, so
//! one must never change once released. The prefix keeps every derived slug
//! out of CAP's namespace by construction, and [`check`] fails the build if
//! CAP ever used the prefix or if two edition names derive the same slug.
//!
//! An edition whose volume is always a year (`2019-Ohio-1234`, `2021 IL App
//! (1st) 200123`) never takes a CAP slug. CAP numbers its reporters' volumes
//! (its one year-numbered series, `mass-app-div-annual`, is no reporters-db
//! edition's), so a year-volume edition that shares a name with a CAP
//! reporter, like the Ohio neutral citation and CAP's Ohio Reports (`ohio`),
//! is a different reporter and gets a derived slug.
//!
//! CAP gives two reporters the short names `Ct. Cl.` and `Mass. App. Div.`,
//! and its slug lookup by edition name keeps only one of each, so
//! [`CAP_BY_REPORTER`] picks the CAP slug for those editions by their
//! reporters-db reporter, and [`check`] fails the build if CAP shares any
//! other short name an edition uses.

use std::collections::BTreeMap;

/// The prefix of every derived slug. No CAP slug starts with it.
pub const DERIVED_PREFIX: &str = "rdb-";

/// Derived slugs set by hand, as `(edition name, slug)`, for an edition name
/// whose derived slug would collide with another's. Each slug must start with
/// [`DERIVED_PREFIX`]. Never change or remove an entry once released.
pub const OVERRIDES: &[(&str, &str)] = &[];

/// The CAP slug for an edition named by a CAP short name that CAP gives
/// more than one reporter, as `(edition name, reporters-db reporter name,
/// CAP slug)`. `Mass. App. Div.` is the numbered series; CAP's
/// `mass-app-div-annual` numbers its volumes by year and is in no
/// reporters-db edition of its own.
pub const CAP_BY_REPORTER: &[(&str, &str, &str)] = &[
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
    (
        "Mass. App. Div.",
        "Reports of Massachusetts Appellate Division",
        "mass-app-div",
    ),
];

/// Whether `s` is a slug: `^[a-z0-9][a-z0-9-]*$`.
pub fn is_slug(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// The derived slug for a reporters-db edition name, or `None` when the name
/// has no ASCII letter or digit to build one from.
pub fn derive(edition_name: &str) -> Option<String> {
    if let Some((_, slug)) = OVERRIDES.iter().find(|(n, _)| *n == edition_name) {
        return Some((*slug).to_string());
    }
    let mut body = String::new();
    let mut gap = false;
    for c in edition_name.chars() {
        match c {
            '\'' | '\u{2019}' => {}
            c if c.is_ascii_alphanumeric() => {
                if gap && !body.is_empty() {
                    body.push('-');
                }
                gap = false;
                body.push(c.to_ascii_lowercase());
            }
            _ => gap = true,
        }
    }
    (!body.is_empty()).then(|| format!("{DERIVED_PREFIX}{body}"))
}

/// The slug for one edition of reporter `reporter_name`: its CAP slug
/// ([`CAP_BY_REPORTER`], else `harvard_slug`), unless it has none or its
/// volume is always a year, else the derived one.
pub fn assign(
    edition_name: &str,
    reporter_name: &str,
    harvard_slug: Option<&str>,
    year_volume: bool,
) -> Option<String> {
    let cap = CAP_BY_REPORTER
        .iter()
        .find(|(e, r, _)| *e == edition_name && *r == reporter_name)
        .map(|(_, _, s)| *s)
        .or(harvard_slug);
    match cap {
        Some(s) if !year_volume => Some(s.to_string()),
        _ => derive(edition_name),
    }
}

/// Check the slugs given to every edition, as `(edition name, reporter
/// name, slug)`, against all of CAP's reporters, as `(short name, slug)`.
/// Fails when a CAP slug starts with [`DERIVED_PREFIX`], when CAP gives two
/// reporters an edition's name and [`CAP_BY_REPORTER`] doesn't say which
/// the edition is, when a slug is malformed, when an override lacks the
/// prefix, or when two edition names derive the same slug.
pub fn check<'a>(
    cap: impl IntoIterator<Item = (&'a str, &'a str)>,
    assigned: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> Result<(), String> {
    let mut errors = Vec::new();
    let mut cap_names: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, s) in cap {
        if s.starts_with(DERIVED_PREFIX) {
            errors.push(format!(
                "CAP slug {s:?} starts with the derived-slug prefix {DERIVED_PREFIX:?}"
            ));
        }
        cap_names.entry(name).or_default().push(s);
    }
    for (name, slug) in OVERRIDES {
        if !slug.starts_with(DERIVED_PREFIX) || !is_slug(slug) {
            errors.push(format!(
                "override for {name:?} must be a slug starting with {DERIVED_PREFIX:?}, not {slug:?}"
            ));
        }
    }
    let mut derived: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (name, reporter, slug) in assigned {
        if cap_names.get(name).is_some_and(|s| s.len() > 1)
            && !CAP_BY_REPORTER
                .iter()
                .any(|(e, r, _)| *e == name && *r == reporter)
        {
            errors.push(format!(
                "CAP gives {:?} to several reporters ({:?}); add edition {name:?} of {reporter:?} \
                 to citare_data_build::slugs::CAP_BY_REPORTER",
                name, cap_names[name]
            ));
        }
        if !is_slug(slug) {
            errors.push(format!("edition {name:?} has malformed slug {slug:?}"));
        }
        if slug.starts_with(DERIVED_PREFIX) {
            let names = derived.entry(slug).or_default();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    for (slug, mut names) in derived {
        if names.len() > 1 {
            names.sort_unstable();
            errors.push(format!(
                "editions {names:?} all derive slug {slug:?}; give all but the one that \
                 released with it an entry in citare_data_build::slugs::OVERRIDES"
            ));
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
            ("NY Slip Op", "rdb-ny-slip-op"),
            ("T.C. Memo.", "rdb-t-c-memo"),
            ("N.Y.S.3d", "rdb-n-y-s-3d"),
            ("A.D.3d", "rdb-a-d-3d"),
            ("F.4th", "rdb-f-4th"),
            ("IL App (1st)", "rdb-il-app-1st"),
            ("Ohio", "rdb-ohio"),
            ("F. App'x", "rdb-f-appx"),
            ("Commc\u{2019}ns Reg. (P & F)", "rdb-commcns-reg-p-f"),
        ] {
            assert_eq!(derive(name).as_deref(), Some(slug), "{name}");
            assert!(is_slug(slug));
        }
        assert_eq!(derive("§ ."), None);
    }

    #[test]
    fn cap_slug_wins_unless_volume_is_a_year() {
        let a = |n, h, y| assign(n, "R", h, y);
        assert_eq!(a("A.2d", Some("a2d"), false).as_deref(), Some("a2d"));
        assert_eq!(a("Ohio", Some("ohio"), false).as_deref(), Some("ohio"));
        assert_eq!(a("Ohio", Some("ohio"), true).as_deref(), Some("rdb-ohio"));
        assert_eq!(a("F.4th", None, false).as_deref(), Some("rdb-f-4th"));
    }

    #[test]
    fn shared_cap_names_pick_by_reporter() {
        let us = "Court of Claims Reports (United States)";
        let wv = "Report of the Court of Claims (West Virginia)";
        assert_eq!(
            assign("Ct. Cl.", us, Some("wv-ct-cl"), false).as_deref(),
            Some("us-ct-cl")
        );
        assert_eq!(
            assign("Ct. Cl.", wv, Some("wv-ct-cl"), false).as_deref(),
            Some("wv-ct-cl")
        );
        let cap = [("Ct. Cl.", "us-ct-cl"), ("Ct. Cl.", "wv-ct-cl")];
        assert!(check(cap, [("Ct. Cl.", us, "us-ct-cl")]).is_ok());
        assert!(check(cap, [("Ct. Cl.", "Other", "wv-ct-cl")]).is_err());
    }

    #[test]
    fn slug_shape() {
        assert!(is_slug("a2d"));
        assert!(is_slug("rdb-ny-slip-op"));
        assert!(!is_slug(""));
        assert!(!is_slug("-a"));
        assert!(!is_slug("A2d"));
        assert!(!is_slug("a.2d"));
    }

    #[test]
    fn check_rejects_collisions_and_prefixed_cap_slugs() {
        let ok = [("A.2d", "R", "a2d"), ("F.4th", "R", "rdb-f-4th")];
        assert!(check([("A.2d", "a2d")], ok).is_ok());
        // Two editions sharing a name share its slug.
        assert!(check(
            [],
            [
                ("Cranch", "R1", "rdb-cranch"),
                ("Cranch", "R2", "rdb-cranch")
            ]
        )
        .is_ok());
        let err = check(
            [],
            [("MSPB", "R", "rdb-mspb"), ("M S P B", "R", "rdb-mspb")],
        )
        .unwrap_err();
        assert!(err.contains("rdb-mspb"), "{err}");
        assert!(check([("X.", "rdb-x")], []).is_err());
        assert!(check([], [("X", "R", "Bad Slug")]).is_err());
    }
}
