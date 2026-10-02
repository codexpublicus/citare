//! Regex helper functions and constants for build-time extractor generation.
//!
//! These mirror the functions in `citare-core/src/regexes.rs` but are available
//! at build time (in `build.rs`) without pulling in the full `citare-core` crate.

/// Roman numerals 1-199, excluding 5 (`v`), 50 (`l`), and 100 (`c`).
pub const ROMAN_NUMERAL_REGEX: &str = concat!(
    r"c?(?:xc|xl|l?x{1,3})(?:ix|iv|v?i{0,3})",
    "|",
    r"(?:c?l?)(?:ix|iv|v?i{1,3})",
    "|",
    r"(?:lv|cv|cl|clv)",
);

/// Page number: a plain digit, a roman numeral, or underscore placeholder.
pub fn page_number_regex() -> String {
    format!(r"(?:\d+|{ROMAN_NUMERAL_REGEX}|_+)")
}

/// Punctuation pattern (zero or more non-alphanumeric, non-space chars).
pub const PUNCTUATION_REGEX: &str = r"[^\sa-zA-Z0-9]*";

/// Wrap a regex so it requires non-alphanumeric (or start/end) boundaries and
/// places the inner pattern in a capture group.
pub fn nonalphanum_boundaries_re(regex: &str) -> String {
    format!(r"(?:^|[^a-zA-Z0-9])({regex})(?:[^a-zA-Z0-9]|$)")
}

/// Wrap a regex so it requires whitespace (or start/end) boundaries and places
/// the inner pattern in a capture group.
pub fn space_boundaries_re(regex: &str) -> String {
    format!(r"(?:^|\s)({regex})(?:\s|$)")
}

/// Wrap a regex with the punctuation pattern on both sides.
pub fn strip_punctuation_re(regex: &str) -> String {
    format!("{PUNCTUATION_REGEX}{regex}{PUNCTUATION_REGEX}")
}

/// Convert a full citation regex into a short citation regex by replacing
/// `(?P<page>` with `at\s?(p(\.|age)?)? (?P<page>`.
pub fn short_cite_re(regex: &str) -> String {
    regex.replace(r"(?P<page>", r"at\s?(p(\.|age)?)? (?P<page>")
}

/// Ordered list of stop words recognised during tokenisation.
pub const STOP_WORDS: &[&str] = &[
    "v",
    "in re",
    "re",
    "quoting",
    r"e\.g\.",
    "parte",
    "denied",
    "citing",
    "aff'd",
    "affirmed",
    "remanded",
    "see also",
    "see",
    "granted",
    "dismissed",
    "Cf",
];

/// Section token regex (e.g. `§1234`).
pub const SECTION_REGEX: &str = r"(\S*§\S*)";

/// Paragraph (newline) token regex.
pub const PARAGRAPH_REGEX: &str = r"(\n)";

/// Common placeholder reporter patterns.
pub const COMMON_PLACEHOLDER_REPORTERS: &[&str] = &[
    r"A\.?2d",
    r"A\.?3d",
    r"AD3d",
    r"Cal\.",
    r"F\.?3d",
    r"F\.Supp\.2d",
    r"Idaho",
    r"Iowa",
    r"L\.Ed\.",
    r"Mass\.",
    r"N\.?J\.?",
    r"N\.C\.\s?App\.",
    r"N\.E\.3d",
    r"N\.J\.\sSuper\.(\sat)?",
    r"N\.W\.2d",
    r"N\.W\.3d",
    r"N\.?Y\.?",
    r"Nev\.",
    r"NY3d",
    r"Ohio\sSt\.3d",
    r"P\.?3d",
    r"S\.?E\.?2d",
    r"S\.?E\.?3d",
    r"S\.?W\.?2d",
    r"S\.?W\.?3d",
    r"S\.C\.",
    r"S\.Ct\.",
    r"So\.?3d",
    r"U\.\s?S\.",
    r"W\.Va\.",
    r"Wis\.\s?2d",
];

/// Build the `id.` / `ibid.` token regex string.
pub fn id_regex() -> String {
    space_boundaries_re(r"id\.,?|ibid\.")
}

/// Build the `supra` token regex string.
pub fn supra_regex() -> String {
    space_boundaries_re(&strip_punctuation_re("supra"))
}

/// Build the stop-word token regex string.
pub fn stop_word_regex() -> String {
    let joined = STOP_WORDS.join("|");
    let inner = strip_punctuation_re(&format!("(?P<stop_word>{joined})"));
    space_boundaries_re(&inner)
}

/// Build the placeholder citation regex string.
pub fn placeholder_citations_regex() -> String {
    let reporters = COMMON_PLACEHOLDER_REPORTERS.join("|");
    format!(r"([_\u{{2014}}\u{{2013}}-]+\s({reporters})\s[_\u{{2014}}\u{{2013}}-]+)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_number_regex() {
        let re = regex::Regex::new(&format!("^(?:{})$", page_number_regex())).unwrap();
        assert!(re.is_match("123"));
        assert!(re.is_match("iv"));
        assert!(re.is_match("___"));
        assert!(!re.is_match(""));
    }

    #[test]
    fn test_id_regex() {
        let re = regex::Regex::new(&id_regex()).unwrap();
        assert!(re.is_match(" id. "));
        assert!(re.is_match(" ibid. "));
    }

    #[test]
    fn test_supra_regex() {
        let re = regex::Regex::new(&supra_regex()).unwrap();
        assert!(re.is_match(" supra "));
    }

    #[test]
    fn test_stop_word_regex() {
        let re = regex::Regex::new(&stop_word_regex()).unwrap();
        assert!(re.is_match(" v "));
        assert!(re.is_match(" in re "));
    }

    #[test]
    fn test_placeholder_citations_regex() {
        let re = regex::Regex::new(&placeholder_citations_regex()).unwrap();
        assert!(re.is_match("___ U.S. ___"));
    }

    #[test]
    fn test_nonalphanum_boundaries_re() {
        let pattern = nonalphanum_boundaries_re("foo");
        let re = regex::Regex::new(&pattern).unwrap();
        assert!(re.is_match(" foo "));
        assert!(!re.is_match("afoo"));
    }

    #[test]
    fn test_short_cite_re() {
        let original = r"(?P<reporter>U\.S\.) (?P<page>\d+)";
        let short = short_cite_re(original);
        assert_eq!(
            short,
            r"(?P<reporter>U\.S\.) at\s?(p(\.|age)?)? (?P<page>\d+)"
        );
    }
}
