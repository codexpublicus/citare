//! Utility functions and constants for citation processing.

use regex::Regex;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Reporters whose page numbers sometimes require correction.
pub static REPORTERS_THAT_NEED_PAGE_CORRECTION: &[&str] = &["NY Slip Op", "Misc. 3d"];

/// Names not allowed to be used as reference citations.
pub static DISALLOWED_NAMES: &[&str] = &[
    // Common options
    "state",
    "united states",
    "people",
    "commonwealth",
    "mass",
    "commissioner",
    // AGs
    "Akerman",
    "Ashcroft",
    "Barr",
    "Bates",
    "Bell",
    "Berrien",
    "Biddle",
    "Black",
    "Bonaparte",
    "Bork",
    "Bondi",
    "Bradford",
    "Breckinridge",
    "Brewster",
    "Brownell",
    "Butler",
    "Civiletti",
    "Clark",
    "Clement",
    "Clifford",
    "Crittenden",
    "Cummings",
    "Cushing",
    "Daugherty",
    "Devens",
    "Evarts",
    "Filip",
    "Garland",
    "Gerson",
    "Gilpin",
    "Gonzales",
    "Gregory",
    "Griggs",
    "Grundy",
    "Harmon",
    "Hoar",
    "Holder",
    "Jackson",
    "Johnson",
    "Katzenbach",
    "Keisler",
    "Kennedy",
    "Kleindienst",
    "Knox",
    "Lee",
    "Legaré",
    "Levi",
    "Lincoln",
    "Lynch",
    "MacVeagh",
    "Mason",
    "McGranery",
    "McGrath",
    "McKenna",
    "McReynolds",
    "Meese",
    "Miller",
    "Mitchell",
    "Moody",
    "Mukasey",
    "Murphy",
    "Nelson",
    "Olney",
    "Palmer",
    "Pierrepont",
    "Pinkney",
    "Randolph",
    "Reno",
    "Richardson",
    "Rodney",
    "Rogers",
    "Rush",
    "Sargent",
    "Saxbe",
    "Sessions",
    "Smith",
    "Speed",
    "Stanbery",
    "Stanton",
    "Stone",
    "Taft",
    "Taney",
    "Thornburgh",
    "Toucey",
    "Whitacker",
    "Wickersham",
    "Williams",
    "Wirt",
];

// ── Regex singletons ──────────────────────────────────────────────────────────

fn re_strip_punct_leading_quotes() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"^[\"']"#).unwrap())
}

fn re_strip_punct_backtick_quotes() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(``)").unwrap())
}

fn re_strip_punct_space_quote() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"([ (\[{<])""#).unwrap())
}

fn re_strip_punct_ellipsis() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\.\.\.").unwrap())
}

fn re_strip_punct_symbols() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[,;:@#$%&]").unwrap())
}

fn re_strip_punct_trailing_dot() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"([^.])(\.)([\])}>"']*)\s*$"#).unwrap())
}

fn re_strip_punct_question_excl() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[?!]").unwrap())
}

fn re_strip_punct_apostrophe_space() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"([^'])' ").unwrap())
}

fn re_strip_punct_brackets() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\]\[(){}<>]").unwrap())
}

fn re_strip_punct_dashes() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"--").unwrap())
}

fn re_strip_punct_double_quote() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r#"""#).unwrap())
}

fn re_strip_punct_trailing_apostrophe() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(\S)(''?)").unwrap())
}

fn re_wrap_html_tags() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(<[^>]+>)").unwrap())
}

fn re_placeholder_markup() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"<([/a-z])[^>]+>").unwrap())
}

// ── Public functions ──────────────────────────────────────────────────────────

/// Strip punctuation from a string (Penn Treebank style).
pub fn strip_punct(text: &str) -> String {
    let mut s = text.to_string();

    // Starting quotes
    s = re_strip_punct_leading_quotes().replace(&s, "").into_owned();
    s = re_strip_punct_backtick_quotes()
        .replace_all(&s, "")
        .into_owned();
    s = re_strip_punct_space_quote()
        .replace_all(&s, "")
        .into_owned();

    // Punctuation
    s = re_strip_punct_ellipsis().replace_all(&s, "").into_owned();
    s = re_strip_punct_symbols().replace_all(&s, "").into_owned();
    s = re_strip_punct_trailing_dot().replace(&s, "$1").into_owned();
    s = re_strip_punct_question_excl()
        .replace_all(&s, "")
        .into_owned();
    s = re_strip_punct_apostrophe_space()
        .replace_all(&s, "")
        .into_owned();

    // Parens, brackets, etc.
    s = re_strip_punct_brackets().replace_all(&s, "").into_owned();
    s = re_strip_punct_dashes().replace_all(&s, "").into_owned();

    // Ending quotes
    s = re_strip_punct_double_quote()
        .replace_all(&s, "")
        .into_owned();
    s = re_strip_punct_trailing_apostrophe()
        .replace_all(&s, "$1")
        .into_owned();

    s.trim().to_string()
}

/// Return `false` if `text` contains unbalanced HTML tags, otherwise `true`.
///
/// Uses a simple stack-based tag-balance check rather than a full XML parser.
pub fn is_balanced_html(text: &str) -> bool {
    if !text.contains('<') && !text.contains('>') {
        return true;
    }

    // Simple stack-based balance check: track opening tags and match closings.
    let mut stack: Vec<String> = Vec::new();
    let mut i = 0;
    let bytes = text.as_bytes();
    let len = bytes.len();

    while i < len {
        if bytes[i] == b'<' {
            // Find the closing '>'
            let Some(end) = text[i..].find('>') else {
                return false; // unclosed tag
            };
            let tag_content = &text[i + 1..i + end];
            let tag_content = tag_content.trim();

            if tag_content.is_empty() {
                i += end + 1;
                continue;
            }

            // Comments, CDATA, DOCTYPE — skip
            if tag_content.starts_with('!')
                || tag_content.starts_with('?')
                || tag_content.to_ascii_lowercase().starts_with("!doctype")
            {
                i += end + 1;
                continue;
            }

            if let Some(stripped) = tag_content.strip_prefix('/') {
                // Closing tag
                let tag_name = stripped
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                if stack.last().map(|s: &String| s.as_str()) == Some(&tag_name) {
                    stack.pop();
                } else {
                    return false;
                }
            } else if tag_content.ends_with('/') {
                // Self-closing tag — ignore
            } else {
                // Opening tag
                let tag_name = tag_content
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                // HTML void elements are self-closing by spec; don't push them
                if !is_void_element(&tag_name) {
                    stack.push(tag_name);
                }
            }
            i += end + 1;
        } else {
            i += 1;
        }
    }

    stack.is_empty()
}

/// HTML void (self-closing) elements that should not be pushed onto the stack.
fn is_void_element(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

/// Wrap every HTML tag in `text` with `before` and `after` strings.
pub fn wrap_html_tags(text: &str, before: &str, after: &str) -> String {
    re_wrap_html_tags()
        .replace_all(text, format!("{before}$1{after}").as_str())
        .into_owned()
}

/// Replace HTML tags with same-length placeholder tags for offset preservation.
///
/// `<em class="x">` becomes `<XXXXXXXXXXXXXXX>` and `</em>` becomes `</XX>`.
pub fn placeholder_markup(html: &str) -> String {
    re_placeholder_markup()
        .replace_all(html, |caps: &regex::Captures| {
            let tag = &caps[0];
            if tag.starts_with("</") {
                // closing tag: "</" + X*(len-3) + ">"
                let x_count = tag.len().saturating_sub(3);
                format!("</{}>", "X".repeat(x_count))
            } else {
                // opening tag: "<" + X*(len-2) + ">"
                let x_count = tag.len().saturating_sub(2);
                format!("<{}>", "X".repeat(x_count))
            }
        })
        .into_owned()
}

/// Return `true` if `name` is a valid citation party name.
pub fn is_valid_name(name: &str) -> bool {
    if name.len() <= 2 {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_uppercase() {
        return false;
    }
    if name.ends_with('.') {
        return false;
    }
    if name.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    let lower = name.to_lowercase();
    !DISALLOWED_NAMES.iter().any(|&d| d.to_lowercase() == lower)
}

/// Hash a pre-serialized string with SHA-256 and return the first 8 bytes as
/// a `u64` (big-endian), matching Python's `hash_sha256`.
pub fn hash_sha256(data: &str) -> u64 {
    let digest = Sha256::digest(data.as_bytes());
    u64::from_be_bytes(digest[..8].try_into().unwrap())
}

/// Try to include missing style tags within `tolerance` characters of a span.
///
/// Returns `(new_start, new_end, span_text)`.
pub fn maybe_balance_style_tags(
    mut start: usize,
    mut end: usize,
    plain_text: &str,
    tolerance: usize,
) -> (usize, usize, String) {
    let style_tags = ["i", "em", "b"];

    for tag in &style_tags {
        let opening_tag = format!("<{tag}>");
        let closing_tag = format!("</{tag}>");

        let span_text = &plain_text[start..end];
        let has_opening = span_text.contains(&opening_tag);
        let has_closing = span_text.contains(&closing_tag);

        if has_opening && !has_closing {
            // Look for closing tag after end, within tolerance
            let extended_end =
                advance_chars(plain_text, end, closing_tag.len() + tolerance).min(plain_text.len());
            let search_slice = &plain_text[start..extended_end];
            // Pick the first closing tag within tolerance
            if let Some(pos) = find_first(search_slice, &closing_tag) {
                end = start + pos + closing_tag.len();
            }
        }

        if !has_opening && has_closing {
            // Look for opening tag before start, within tolerance
            let extended_start = retreat_chars(plain_text, start, opening_tag.len() + tolerance);
            let search_slice = &plain_text[extended_start..end];
            // Pick the last opening tag within tolerance
            if let Some(pos) = find_last(search_slice, &opening_tag) {
                start = extended_start + pos;
            }
        }
    }

    let span_text = plain_text[start..end].to_string();
    (start, end, span_text)
}

/// The byte offset `n` characters after byte offset `from`, or the end of
/// `text`. eyecite counts offsets in characters; this moves a byte offset
/// by the same number of them.
pub(crate) fn advance_chars(text: &str, from: usize, n: usize) -> usize {
    text[from..]
        .char_indices()
        .nth(n)
        .map_or(text.len(), |(i, _)| from + i)
}

/// The byte offset `n` characters before byte offset `from`, or 0.
pub(crate) fn retreat_chars(text: &str, from: usize, n: usize) -> usize {
    if n == 0 {
        return from;
    }
    text[..from]
        .char_indices()
        .rev()
        .nth(n - 1)
        .map_or(0, |(i, _)| i)
}

/// Find the byte offset of the first occurrence of `needle` in `haystack`.
fn find_first(haystack: &str, needle: &str) -> Option<usize> {
    haystack.find(needle)
}

/// Find the byte offset of the last occurrence of `needle` in `haystack`.
fn find_last(haystack: &str, needle: &str) -> Option<usize> {
    haystack.rfind(needle)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_punct_basic() {
        assert_eq!(strip_punct("Hello, World!"), "Hello World");
    }

    #[test]
    fn test_strip_punct_trailing_dot() {
        // Trailing dot on a non-dot-ending word should be removed.
        assert_eq!(strip_punct("Smith v. Jones"), "Smith v. Jones");
        assert_eq!(strip_punct("Jones."), "Jones");
    }

    #[test]
    fn test_is_valid_name_valid() {
        assert!(is_valid_name("Adarand"));
        assert!(is_valid_name("Brown"));
    }

    #[test]
    fn test_is_valid_name_disallowed() {
        assert!(!is_valid_name("state"));
        assert!(!is_valid_name("State")); // case-insensitive
        assert!(!is_valid_name("Garland"));
    }

    #[test]
    fn test_is_valid_name_too_short() {
        assert!(!is_valid_name("ab"));
        assert!(!is_valid_name("A"));
    }

    #[test]
    fn test_is_valid_name_lowercase_start() {
        assert!(!is_valid_name("foo"));
    }

    #[test]
    fn test_is_valid_name_ends_with_dot() {
        assert!(!is_valid_name("Co."));
    }

    #[test]
    fn test_is_balanced_html_no_tags() {
        assert!(is_balanced_html("plain text"));
    }

    #[test]
    fn test_is_balanced_html_balanced() {
        assert!(is_balanced_html("<em>text</em>"));
        assert!(is_balanced_html("<b><i>text</i></b>"));
    }

    #[test]
    fn test_is_balanced_html_unbalanced() {
        assert!(!is_balanced_html("<em>text"));
        assert!(!is_balanced_html("text</em>"));
    }

    #[test]
    fn test_wrap_html_tags() {
        let result = wrap_html_tags("<em>text</em>", "[", "]");
        assert_eq!(result, "[<em>]text[</em>]");
    }

    #[test]
    fn test_placeholder_markup_length_preserved() {
        let original = "<em>text</em>";
        let placeholder = placeholder_markup(original);
        assert_eq!(original.len(), placeholder.len());
    }

    #[test]
    fn test_placeholder_markup_content() {
        // <span class='foo'> is 17 chars → <XXXXXXXXXXXXXXX> (17 chars)
        let result = placeholder_markup("<span class='foo'>text</span>");
        // opening: len("<span class='foo'>") = 18 → "<" + 16 X's + ">" = 18 ✓
        // closing: len("</span>") = 7 → "</" + 4 X's + ">" = 7 ✓
        assert!(!result.contains("span"));
        assert!(result.contains("text"));
        assert_eq!(result.len(), "<span class='foo'>text</span>".len());
    }

    #[test]
    fn test_hash_sha256() {
        // Deterministic: same input → same output
        let h1 = hash_sha256("hello");
        let h2 = hash_sha256("hello");
        assert_eq!(h1, h2);
        assert_ne!(hash_sha256("hello"), hash_sha256("world"));
    }

    #[test]
    fn test_maybe_balance_style_tags_no_change() {
        let text = "foo <em>bar</em> baz";
        let (s, e, span) = maybe_balance_style_tags(4, 16, text, 10);
        assert_eq!(&text[s..e], span);
    }

    #[test]
    fn test_maybe_balance_style_tags_extend_end() {
        // span has <em> but closing </em> is just outside end
        let text = "foo <em>bar</em> baz";
        // Start after <em>, so span has no closing tag
        let (s, e, span) = maybe_balance_style_tags(4, 11, text, 10);
        assert_eq!(&text[s..e], span);
        // Should have extended to include </em>
        assert!(span.contains("</em>") || e > 11);
    }

    #[test]
    fn test_reporters_that_need_page_correction() {
        assert!(REPORTERS_THAT_NEED_PAGE_CORRECTION.contains(&"NY Slip Op"));
        assert!(REPORTERS_THAT_NEED_PAGE_CORRECTION.contains(&"Misc. 3d"));
    }
}
