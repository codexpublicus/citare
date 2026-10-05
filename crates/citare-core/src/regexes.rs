//! Regex patterns for legal citation extraction.
//!
//! Ported from eyecite's `regexes.py`. Tokenizer regexes use the standard
//! `regex` crate. Metadata regexes (which require lookahead) use `fancy-regex`.
//!
//! Some Python regexes have duplicate named capture groups across alternation
//! branches. Since neither `regex` nor `fancy-regex` supports this, we rename
//! the groups with branch suffixes (`_a`, `_b`, etc.) and provide a
//! [`merge_branch_groups`] helper to coalesce them.

use std::collections::HashMap;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Helper functions for building regex patterns
// ---------------------------------------------------------------------------

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

/// Create a reference pin-cite regex pattern from a slice of reporter regexes.
pub fn reference_pin_cite_re(regexes: &[&str]) -> String {
    let joined = regexes.join("|");
    format!(r"\b(?:{joined})\s+{pin}", pin = *PIN_CITE_REGEX)
}

// ---------------------------------------------------------------------------
// Tokenizer regexes (standard `regex` crate — no lookaround)
// ---------------------------------------------------------------------------

/// Roman numerals 1–199, excluding 5 (`v`), 50 (`l`), and 100 (`c`).
/// Case-insensitive matching is expected to be applied at compile time.
pub const ROMAN_NUMERAL_REGEX: &str = concat!(
    // 10-199, but not 50-59 or 100-109 or 150-159:
    r"c?(?:xc|xl|l?x{1,3})(?:ix|iv|v?i{0,3})",
    "|",
    // 1-9, 51-59, 101-109, 151-159, but not 5, 55, 105, 155:
    r"(?:c?l?)(?:ix|iv|v?i{1,3})",
    "|",
    // 55, 105, 150, 155:
    r"(?:lv|cv|cl|clv)",
);

/// Page number: a plain digit, a roman numeral, or underscore placeholder.
pub static PAGE_NUMBER_REGEX: LazyLock<String> =
    LazyLock::new(|| format!(r"(?:\d+|{ROMAN_NUMERAL_REGEX}|_+)"));

/// Punctuation pattern (zero or more non-alphanumeric, non-space chars).
pub const PUNCTUATION_REGEX: &str = r"[^\sa-zA-Z0-9]*";

/// `id.` / `ibid.` token regex (with space/boundary wrapping).
pub static ID_REGEX: LazyLock<String> = LazyLock::new(|| space_boundaries_re(r"id\.,?|ibid\."));

/// `supra` token regex (with space/boundary and punctuation wrapping).
pub static SUPRA_REGEX: LazyLock<String> =
    LazyLock::new(|| space_boundaries_re(&strip_punctuation_re("supra")));

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

/// Stop-word token regex.
pub static STOP_WORD_REGEX: LazyLock<String> = LazyLock::new(|| {
    let joined = STOP_WORDS.join("|");
    let inner = strip_punctuation_re(&format!("(?P<stop_word>{joined})"));
    space_boundaries_re(&inner)
});

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

/// Placeholder citation regex (e.g. `___ U.S. ___`).
pub static PLACEHOLDER_CITATIONS: LazyLock<String> = LazyLock::new(|| {
    let reporters = COMMON_PLACEHOLDER_REPORTERS.join("|");
    format!(r"([_\u{{2014}}\u{{2013}}-]+\s({reporters})\s[_\u{{2014}}\u{{2013}}-]+)")
});

// ---------------------------------------------------------------------------
// Metadata regex building blocks (fancy-regex — may contain lookahead)
// ---------------------------------------------------------------------------
// Python's verbose-mode regexes are collapsed to single-line here.

/// Optional trailing parenthetical, e.g. ` (overruling Foo)`.
pub const PARENTHETICAL_REGEX: &str = r"(?: ?\((?P<parenthetical>.*)\))?";

/// The alternation body shared by all month regexes (no group wrapper).
const MONTH_BODY: &str = concat!(
    r"January|Jan\.|",
    r"February|Feb\.|",
    r"March|Mar\.|",
    r"April|Apr\.|",
    r"May|",
    r"June|Jun\.|",
    r"July|Jul\.|",
    r"August|Aug\.|",
    r"September|Sept?\.|",
    r"October|Oct\.|",
    r"November|Nov\.|",
    r"December|Dec\.",
);

/// Month names (long and abbreviated) — named group `month`.
pub static MONTH_REGEX: LazyLock<String> = LazyLock::new(|| format!("(?P<month>{MONTH_BODY})"));

/// Month regex with renamed group `month_la` for use inside lookahead
/// (avoids duplicate named group with the capture-position `MONTH_REGEX`).
pub static MONTH_REGEX_LOOKAHEAD: LazyLock<String> =
    LazyLock::new(|| format!("(?P<month_la>{MONTH_BODY})"));

/// The inner body shared by all year regexes (no group wrapper).
const YEAR_BODY: &str = r"\d{4}";

/// Four-digit year with optional two-digit range suffix — named group `year`.
pub static YEAR_REGEX: LazyLock<String> =
    LazyLock::new(|| format!("(?:(?P<year>{YEAR_BODY})(?:-\\d{{2}})?)"));

/// Year regex with renamed group `year_la` for use inside lookahead.
pub static YEAR_REGEX_LOOKAHEAD: LazyLock<String> =
    LazyLock::new(|| format!("(?:(?P<year_la>{YEAR_BODY})(?:-\\d{{2}})?)"));

/// A single pin-cite token (label + number/range).
pub const PIN_CITE_TOKEN_REGEX: &str = concat!(
    r"(?:",
    r"(?:",
    r"(?:& )?note|",
    r"(?:& )?nn?\.?|",
    r"(?:& )?fn?\.?|",
    r"\u{00B6}{1,2}|", // ¶
    r"\u{00A7}{1,2}|", // §
    r"\*{1,4}|",
    r"pg\.?|",
    r"pp?\.?",
    r") ?",
    r")?",
    r"(?:",
    r"\d+:\d+(?:-\d+(?::\d+)?)?|",
    r"[*]?\d+(?:-\d+)?",
    r")",
);

/// Pin cite regex (contains a lookahead — requires `fancy-regex`).
/// Named group: `pin_cite`.
pub static PIN_CITE_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?P<pin_cite>",
            r",? ?(?:at )?",
            r"{token}",
            r"(?:, ?{token})*",
            r"(?=",
            r"[,.;)\]\\]|",
            r" ?[(\[]|",
            r"$",
            r")",
            r")",
        ),
        token = PIN_CITE_TOKEN_REGEX,
    )
});

/// Pin cite regex variant A (group renamed to `pin_cite_a`).
/// Used in the first branch of `POST_FULL_CITATION_REGEX`.
pub static PIN_CITE_REGEX_A: LazyLock<String> =
    LazyLock::new(|| PIN_CITE_REGEX.replace("pin_cite", "pin_cite_a"));

/// Pin cite regex variant B (group renamed to `pin_cite_b`).
/// Used in the second branch of `POST_FULL_CITATION_REGEX`.
pub static PIN_CITE_REGEX_B: LazyLock<String> =
    LazyLock::new(|| PIN_CITE_REGEX.replace("pin_cite", "pin_cite_b"));

/// A single law subsection like `(a)`, `(1)`, `(viii)`.
pub const LAW_SUBSECTION: &str = r"(?:\([0-9a-zA-Z]{1,4}\))";

/// Law pin cite (subsections, "and", "et seq.").
/// Named group: `pin_cite`.
pub static LAW_PIN_CITE_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        r"(?P<pin_cite>{sub}*(?:\ and\ {sub}+)?(?:\ et\ seq\.)?)",
        sub = LAW_SUBSECTION,
    )
});

/// Short-cite antecedent regex.
/// Named group: `antecedent`.
pub const SHORT_CITE_ANTECEDENT_REGEX: &str = r"(?P<antecedent>[A-Za-z][\w\-.]+) ?,? ";

/// Supra-cite antecedent regex (rewritten with unique branch group names).
/// Named groups: `antecedent_a`, `volume_a`, `volume_b`, `antecedent_b`.
pub const SUPRA_ANTECEDENT_REGEX: &str = concat!(
    r"(?:",
    r"(?P<antecedent_a>[\w\-.]+) ?,? (?P<volume_a>\d+)|",
    r"(?P<volume_b>\d+)|",
    r"(?P<antecedent_b>[\w\-.]+) ?,?",
    r")",
    r" ", // final space
);

/// Mapping from logical group names to branch-specific names for
/// [`SUPRA_ANTECEDENT_REGEX`].
pub const SUPRA_ANTECEDENT_GROUPS: &[(&str, &[&str])] = &[
    ("antecedent", &["antecedent_a", "antecedent_b"]),
    ("volume", &["volume_a", "volume_b"]),
];

/// Post-full-citation regex (rewritten with unique branch group names).
///
/// This is the most complex regex. The Python original has:
/// - `PIN_CITE_REGEX` in two alternation branches (renamed `_a` / `_b`)
/// - `MONTH_REGEX` inside a lookahead AND as a capture (renamed `_la`)
/// - `YEAR_REGEX` inside a lookahead AND as a capture (renamed `_la`)
pub static POST_FULL_CITATION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?:",
            // Branch 1: full cite with court+date paren
            r"(?:",
            r"{pin_a}?",
            r",? ?",
            r"(?P<extra>[^(;]*)",
            r")",
            r"[(\[]",
            r"(?:",
            r"(?:",
            r"(?P<court>.*?)",
            r"(?=",
            r"\s+{month_la}|",
            r"\s+{year_la}",
            r")",
            r")?",
            r" ?",
            r"(?P<month>{month_body})? ?",
            r"(?P<day>\d{{1,2}})?,? ?",
            r"(?:(?P<year>{year_body})(?:-\d{{2}})?)",
            r")",
            r"[)\]]",
            r"{paren}",
            r"|",
            // Branch 2: pin cite only
            r"{pin_b}",
            r")",
        ),
        pin_a = *PIN_CITE_REGEX_A,
        pin_b = *PIN_CITE_REGEX_B,
        month_la = *MONTH_REGEX_LOOKAHEAD,
        month_body = MONTH_BODY,
        year_la = *YEAR_REGEX_LOOKAHEAD,
        year_body = YEAR_BODY,
        paren = PARENTHETICAL_REGEX,
    )
});

/// Mapping from logical group names to branch-specific names for
/// [`POST_FULL_CITATION_REGEX`].
pub const POST_FULL_CITATION_GROUPS: &[(&str, &[&str])] = &[
    ("pin_cite", &["pin_cite_a", "pin_cite_b"]),
    ("month", &["month", "month_la"]),
    ("year", &["year", "year_la"]),
];

/// Pre-full-citation regex (antecedent before a repeated full cite).
/// Named groups: `antecedent`, `pin_cite`.
pub static PRE_FULL_CITATION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(r"(?P<antecedent>[A-Z][a-z\-.]+) ?,?", r"{pin}?", r",? ?",),
        pin = *PIN_CITE_REGEX,
    )
});

/// Post-short-citation regex (pin cite + optional parenthetical).
pub static POST_SHORT_CITATION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(r"{pin}?", r" ?", r"{paren}",),
        pin = *PIN_CITE_REGEX,
        paren = PARENTHETICAL_REGEX,
    )
});

/// A subsection in a section pin cite: `(a)`, `(viii)`, a dotted or
/// hyphenated one (`(a.1)`, `(1.3)`, `(d-1)`), one with OCR spaces inside
/// (`( 1 )`), or a bracketed one (`[4]`, `[a]`).
const SECTION_PIN_SUB: &str = concat!(
    r"(?:\( ?[0-9a-zA-Z]{1,4}(?:[.\-\u{2010}\u{2011}\u{2013}\u{2014}][0-9a-zA-Z]{1,4})? ?\)",
    r"|\[[0-9a-zA-Z]{1,4}\])",
);

/// A subsection after a space (`§ 1677(7)(C) (iii)`, `§ 26-516 [a]`).
/// Narrower than [`SECTION_PIN_SUB`], so that a parenthetical such as
/// `(1990)` or `(West)` after the section stays a parenthetical.
const SECTION_PIN_SPACED_SUB: &str =
    r"(?:\ (?:\((?:\d{1,3}|[A-Za-z]|[ivxIVX]{2,5}|[a-z]{2})\)|\[[0-9a-zA-Z]{1,4}\]))";

/// A separator inside a section number: hyphen, dot, colon, the Unicode
/// hyphens U+2010 and U+2011, and the en dash. Two in a row (`75.-316`)
/// come from OCR.
const SECTION_PIN_SEP: &str = r"[-.:\u{2010}\u{2011}\u{2013}\u{2014}]{1,2}";

/// A section number: `1983`, `394-ccc`, `76‐9‐101`, `600.2912a`, `2B1.1`,
/// a roman numeral (`IV`), OCR's `l` for `1` (`§ l-618.3(a)`), or OCR's
/// spaced dash before a digit (`§ 14 — 501(b)(5)`).
static SECTION_PIN_NUMBER: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?:(?:\d|l(?=[-.\d(]))[0-9A-Za-z]*",
            r"(?:{sep}[0-9A-Za-z]+|\ [\u{{2013}}\u{{2014}}]\ \d[0-9A-Za-z]*)*",
            r"|[IVXL]{{1,6}}(?![A-Za-z]))",
        ),
        sep = SECTION_PIN_SEP,
    )
});

/// A designation that follows a section in a pin cite: `, subd. (a)`,
/// `, subd. 6`, `, cmt. (n.2)`, ` cmt. n. 1(B)(iii)`, `, comment a`,
/// ` application n. 5`, `, illus. 1`, `, cl. 1`, `, art. 2`. A value that is
/// a single letter must end there, so `comment by` isn't one.
static SECTION_PIN_DESIGNATION: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?:subds?\.|cmt\.|[Cc]omment|illus\.|cl\.|art\.|application\ n\.)",
            r"\ ?(?:n\.\ ?)?",
            r"(?:{sub}+",
            r"|\d+(?:\.\d+)*[A-Za-z]?{sub}*",
            r"|\((?:[^()\n]|\([^()\n]*\)){{1,40}}\)",
            r"|[a-zA-Z](?![A-Za-z]))",
        ),
        sub = SECTION_PIN_SUB,
    )
});

/// A section number with its subsections, optionally running to another
/// section or subsection, and its designations: `1983`, `394-ccc(2)-(3)`,
/// `1983-1985`, `12(b)`, `216.103 to 216.111`, `5304, subd. (a)`.
static SECTION_PIN_ITEM: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?:{num}{sub}*(?:[-\u{{2010}}\u{{2011}}]\d[0-9A-Za-z]*{sub}*)?|{sub}+)",
            r"(?:\ ?[-\u{{2010}}\u{{2011}}\u{{2013}}\u{{2014}}]\ ?{sub}+)?",
            r"(?:{spaced}(?:\ ?[-\u{{2013}}\u{{2014}}]\ ?{sub}+)?)*",
            r"(?:\ (?:to|through)\ (?:{num}|\.\d[0-9A-Za-z]*(?:{sep}[0-9A-Za-z]+)*){sub}*)?",
            r"(?:,?\ {designation})*",
        ),
        num = *SECTION_PIN_NUMBER,
        sub = SECTION_PIN_SUB,
        spaced = SECTION_PIN_SPACED_SUB,
        sep = SECTION_PIN_SEP,
        designation = *SECTION_PIN_DESIGNATION,
    )
});

/// An item after the first in a list of section pin cites: another section,
/// optionally with its own `§` (`§ 780.4, § 780.5`), a section that repeats
/// only the end of the one before (`, -3202`, `, .024(a)`), more
/// subsections (`, (3)-(7)`), or a designation (`and Comment a`).
static SECTION_PIN_LIST_ITEM: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?:(?:\u{{00A7}}{{1,2}}\s*)?{item}",
            r"|[-.\u{{2010}}\u{{2011}}]\d[0-9A-Za-z]*(?:{sep}[0-9A-Za-z]+)*{sub}*",
            r"|{sub}+(?:[-\u{{2013}}]{sub}+)?",
            r"|{designation})",
        ),
        item = *SECTION_PIN_ITEM,
        sep = SECTION_PIN_SEP,
        sub = SECTION_PIN_SUB,
        designation = *SECTION_PIN_DESIGNATION,
    )
});

/// Section pin cite after an id. citation (`Id. § 5`, `Id. at §§ 5-7`).
///
/// Not in eyecite, whose pin cite scan stops at the `§` token (citare #2).
/// Any whitespace may follow the `§`, including a line break. Whitespace
/// before it is captured as `gap`, and `at` as `at`, so the caller can
/// refuse a line break between a capitalized `Id.` and the `§` (#48).
/// A semicolon continues the list only when the next item ends at
/// punctuation, so `Id. § 5; 28 U.S.C. § 1331` stops at `§ 5`, and a list
/// item followed by a capitalized word is the volume of the next citation
/// (`Id., §501 (b), 17 P.S. §211.501 (b)`), so the list stops before it.
/// Named groups: `pin_cite`, `gap`, `at`, `parenthetical`.
pub static POST_ID_SECTION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"(?P<pin_cite>",
            r",?(?P<gap>\s*)(?P<at>at\s+)?\u{{00A7}}{{1,2}}\s*{item}",
            r"(?:(?:,\s?|,? and |,? & ){list_item}(?!\ [A-Z])",
            r"|; ?(?:\u{{00A7}}{{1,2}}\s*)?{item}(?=[,.;:)\]]|$))*",
            r"(?: et seq\.)?",
            r"(?=[,.;:)\]\\]| ?[(\[]|\s|$)",
            r")",
            r" ?{paren}",
        ),
        item = *SECTION_PIN_ITEM,
        list_item = *SECTION_PIN_LIST_ITEM,
        paren = PARENTHETICAL_REGEX,
    )
});

/// Post-law-citation regex (law pin cite + optional publisher/date/parenthetical).
pub static POST_LAW_CITATION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(
            r"{law_pin}?",
            r" ?",
            r"(?:\(",
            r"(?P<publisher>[A-Z][a-z]+\.?(?:\ Supp\.)?)?",
            r" ?",
            r"(?:{month} )?",
            r"(?P<day>\d{{1,2}})?,? ?",
            r"{year}?",
            r"\))?",
            r" ?",
            r"{paren}",
        ),
        law_pin = *LAW_PIN_CITE_REGEX,
        month = *MONTH_REGEX,
        year = *YEAR_REGEX,
        paren = PARENTHETICAL_REGEX,
    )
});

/// Post-journal-citation regex (pin cite + optional year + parenthetical).
pub static POST_JOURNAL_CITATION_REGEX: LazyLock<String> = LazyLock::new(|| {
    format!(
        concat!(r"{pin}?", r" ?", r"(?:\({year}\))?", r" ?", r"{paren}",),
        pin = *PIN_CITE_REGEX,
        year = *YEAR_REGEX,
        paren = PARENTHETICAL_REGEX,
    )
});

/// Defendant + year regex (e.g. `Smith v. Jones (2012)`).
/// Named groups: `defendant`, `year`.
pub const DEFENDANT_YEAR_REGEX: &str = r"(?P<defendant>.*)\s\((?P<year>\d{4})\)$";

// ---------------------------------------------------------------------------
// Compiled metadata regexes (fancy-regex)
// ---------------------------------------------------------------------------

/// Compiled [`POST_FULL_CITATION_REGEX`].
pub static POST_FULL_CITATION_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&POST_FULL_CITATION_REGEX)
        .expect("POST_FULL_CITATION_REGEX failed to compile")
});

/// Compiled [`PRE_FULL_CITATION_REGEX`].
pub static PRE_FULL_CITATION_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&PRE_FULL_CITATION_REGEX)
        .expect("PRE_FULL_CITATION_REGEX failed to compile")
});

/// Compiled [`POST_SHORT_CITATION_REGEX`].
pub static POST_SHORT_CITATION_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&POST_SHORT_CITATION_REGEX)
        .expect("POST_SHORT_CITATION_REGEX failed to compile")
});

/// Compiled [`POST_LAW_CITATION_REGEX`].
pub static POST_LAW_CITATION_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&POST_LAW_CITATION_REGEX)
        .expect("POST_LAW_CITATION_REGEX failed to compile")
});

/// Compiled [`POST_JOURNAL_CITATION_REGEX`].
pub static POST_JOURNAL_CITATION_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&POST_JOURNAL_CITATION_REGEX)
        .expect("POST_JOURNAL_CITATION_REGEX failed to compile")
});

/// Compiled [`SUPRA_ANTECEDENT_REGEX`].
pub static SUPRA_ANTECEDENT_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(SUPRA_ANTECEDENT_REGEX)
        .expect("SUPRA_ANTECEDENT_REGEX failed to compile")
});

/// Compiled [`SHORT_CITE_ANTECEDENT_REGEX`].
pub static SHORT_CITE_ANTECEDENT_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(SHORT_CITE_ANTECEDENT_REGEX)
        .expect("SHORT_CITE_ANTECEDENT_REGEX failed to compile")
});

/// Compiled [`DEFENDANT_YEAR_REGEX`].
pub static DEFENDANT_YEAR_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(DEFENDANT_YEAR_REGEX).expect("DEFENDANT_YEAR_REGEX failed to compile")
});

/// Compiled [`PIN_CITE_REGEX`].
pub static PIN_CITE_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&PIN_CITE_REGEX).expect("PIN_CITE_REGEX failed to compile")
});

/// Compiled [`LAW_PIN_CITE_REGEX`].
pub static LAW_PIN_CITE_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    fancy_regex::Regex::new(&LAW_PIN_CITE_REGEX).expect("LAW_PIN_CITE_REGEX failed to compile")
});

// ---------------------------------------------------------------------------
// Branch-group merge helper
// ---------------------------------------------------------------------------

/// Given captures from a regex with branch-unique group names (e.g.
/// `antecedent_a`, `antecedent_b`), return a map from the logical group name
/// to the first non-`None` match among its branches.
///
/// `group_mappings` is a slice of `(logical_name, &[branch_name, ...])`.
/// Any branch names that don't exist in the captures are silently skipped.
pub fn merge_branch_groups(
    captures: &fancy_regex::Captures,
    group_mappings: &[(&str, &[&str])],
) -> HashMap<String, Option<String>> {
    let mut result = HashMap::new();
    for &(logical_name, branch_names) in group_mappings {
        let value = branch_names
            .iter()
            .filter_map(|name| captures.name(name).map(|m| m.as_str().to_string()))
            .next();
        result.insert(logical_name.to_string(), value);
    }
    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Tokenizer regex tests --

    #[test]
    fn test_roman_numeral_regex() {
        let re = regex::Regex::new(&format!("^(?:{ROMAN_NUMERAL_REGEX})$")).unwrap();
        assert!(re.is_match("i"));
        assert!(re.is_match("ii"));
        assert!(re.is_match("iii"));
        assert!(re.is_match("iv"));
        assert!(re.is_match("vi"));
        assert!(re.is_match("ix"));
        assert!(re.is_match("xi"));
        assert!(re.is_match("xlii"));
        assert!(re.is_match("clv")); // 155
                                     // Excluded single letters:
        assert!(!re.is_match("v"));
        assert!(!re.is_match("l"));
        assert!(!re.is_match("c"));
        // Empty must not match:
        assert!(!re.is_match(""));
    }

    #[test]
    fn test_page_number_regex() {
        let re = regex::Regex::new(&format!("^(?:{})$", *PAGE_NUMBER_REGEX)).unwrap();
        assert!(re.is_match("123"));
        assert!(re.is_match("iv"));
        assert!(re.is_match("___"));
        assert!(!re.is_match(""));
    }

    #[test]
    fn test_id_regex() {
        let re = regex::Regex::new(&ID_REGEX).unwrap();
        assert!(re.is_match(" id. "));
        assert!(re.is_match(" ibid. "));
        assert!(re.is_match(" id., "));
    }

    #[test]
    fn test_supra_regex() {
        let re = regex::Regex::new(&SUPRA_REGEX).unwrap();
        assert!(re.is_match(" supra "));
        // with surrounding punctuation
        assert!(re.is_match(" ,supra, "));
    }

    #[test]
    fn test_stop_word_regex() {
        let re = regex::Regex::new(&STOP_WORD_REGEX).unwrap();
        assert!(re.is_match(" v "));
        assert!(re.is_match(" in re "));
        assert!(re.is_match(" see also "));
        assert!(re.is_match(" Cf "));
    }

    #[test]
    fn test_section_regex() {
        let re = regex::Regex::new(SECTION_REGEX).unwrap();
        assert!(re.is_match("\u{00A7}1234"));
        assert!(re.is_match("42\u{00A7}1983"));
    }

    #[test]
    fn test_paragraph_regex() {
        let re = regex::Regex::new(PARAGRAPH_REGEX).unwrap();
        assert!(re.is_match("\n"));
        assert!(!re.is_match("a"));
    }

    #[test]
    fn test_placeholder_citations() {
        let re = regex::Regex::new(&PLACEHOLDER_CITATIONS).unwrap();
        assert!(re.is_match("___ U.S. ___"));
        assert!(re.is_match("___ U. S. ___"));
        assert!(re.is_match("\u{2014}\u{2014}\u{2014} S.Ct. \u{2014}\u{2014}\u{2014}"));
    }

    // -- Metadata regex compilation tests --

    #[test]
    fn test_pin_cite_regex_compiles_and_matches() {
        let re = &*PIN_CITE_RE;
        // ", at 240," — the comma after 240 satisfies the lookahead
        assert!(re.is_match(", at 240,").unwrap());
        // ", 4-5 " — space + nothing satisfies $
        assert!(re.is_match(", 4-5,").unwrap());
        // page:paragraph
        assert!(re.is_match(", 123:24-25,").unwrap());
    }

    #[test]
    fn test_law_pin_cite_regex() {
        let re = &*LAW_PIN_CITE_RE;
        assert!(re.is_match("(a)(1)").unwrap());
        assert!(re.is_match("(a)(2) and (d)").unwrap());
        assert!(re.is_match("(a) et seq.").unwrap());
    }

    #[test]
    fn test_post_full_citation_regex_compiles() {
        // Just verify the complex regex compiles.
        let _re = &*POST_FULL_CITATION_RE;
    }

    #[test]
    fn test_post_full_citation_regex_branch1() {
        let re = &*POST_FULL_CITATION_RE;
        let text = ", 4-5, 2 S. Ct. 2, 6-7 (4th Cir. Jan. 1, 2012) (overruling foo)";
        let caps = re.captures(text).unwrap().unwrap();
        assert!(caps.name("pin_cite_a").is_some());
        assert_eq!(caps.name("court").unwrap().as_str(), "4th Cir.");
        assert_eq!(caps.name("month").unwrap().as_str(), "Jan.");
        assert_eq!(caps.name("day").unwrap().as_str(), "1");
        assert_eq!(caps.name("year").unwrap().as_str(), "2012");
        assert_eq!(
            caps.name("parenthetical").unwrap().as_str(),
            "overruling foo"
        );
    }

    #[test]
    fn test_post_full_citation_regex_branch2() {
        let re = &*POST_FULL_CITATION_RE;
        // Just a pin cite, no paren
        let text = ", at 332,";
        let caps = re.captures(text).unwrap().unwrap();
        assert!(caps.name("pin_cite_b").is_some() || caps.name("pin_cite_a").is_some());
    }

    #[test]
    fn test_pre_full_citation_regex() {
        let _re = &*PRE_FULL_CITATION_RE;
    }

    #[test]
    fn test_post_short_citation_regex() {
        let _re = &*POST_SHORT_CITATION_RE;
    }

    #[test]
    fn test_post_law_citation_regex() {
        let _re = &*POST_LAW_CITATION_RE;
    }

    #[test]
    fn test_post_journal_citation_regex() {
        let _re = &*POST_JOURNAL_CITATION_RE;
    }

    #[test]
    fn test_defendant_year_regex() {
        let re = &*DEFENDANT_YEAR_RE;
        let caps = re.captures("Smith v. Jones (2012)").unwrap().unwrap();
        assert_eq!(caps.name("defendant").unwrap().as_str(), "Smith v. Jones");
        assert_eq!(caps.name("year").unwrap().as_str(), "2012");
    }

    // -- Supra antecedent branch-group tests --

    #[test]
    fn test_supra_antecedent_branch1_antecedent_and_volume() {
        let re = &*SUPRA_ANTECEDENT_RE;
        let caps = re.captures("Adarand, 515 ").unwrap().unwrap();
        let groups = merge_branch_groups(&caps, SUPRA_ANTECEDENT_GROUPS);
        assert_eq!(
            groups.get("antecedent").unwrap().as_deref(),
            Some("Adarand")
        );
        assert_eq!(groups.get("volume").unwrap().as_deref(), Some("515"));
    }

    #[test]
    fn test_supra_antecedent_branch2_volume_only() {
        let re = &*SUPRA_ANTECEDENT_RE;
        let caps = re.captures("515 ").unwrap().unwrap();
        let groups = merge_branch_groups(&caps, SUPRA_ANTECEDENT_GROUPS);
        assert_eq!(groups.get("antecedent").unwrap().as_deref(), None);
        assert_eq!(groups.get("volume").unwrap().as_deref(), Some("515"));
    }

    #[test]
    fn test_supra_antecedent_branch3_antecedent_only() {
        let re = &*SUPRA_ANTECEDENT_RE;
        let caps = re.captures("Adarand, ").unwrap().unwrap();
        let groups = merge_branch_groups(&caps, SUPRA_ANTECEDENT_GROUPS);
        assert_eq!(
            groups.get("antecedent").unwrap().as_deref(),
            Some("Adarand")
        );
        assert_eq!(groups.get("volume").unwrap().as_deref(), None);
    }

    // -- Helper function tests --

    #[test]
    fn test_nonalphanum_boundaries_re() {
        let pattern = nonalphanum_boundaries_re("foo");
        let re = regex::Regex::new(&pattern).unwrap();
        assert!(re.is_match(" foo "));
        assert!(re.is_match(",foo."));
        assert!(!re.is_match("afoo"));
        assert!(!re.is_match("foob"));
    }

    #[test]
    fn test_space_boundaries_re() {
        let pattern = space_boundaries_re("foo");
        let re = regex::Regex::new(&pattern).unwrap();
        assert!(re.is_match(" foo "));
        assert!(re.is_match("\tfoo\n"));
        assert!(!re.is_match("afoo"));
    }

    #[test]
    fn test_strip_punctuation_re() {
        let pattern = strip_punctuation_re("supra");
        assert!(pattern.contains("supra"));
        assert!(pattern.starts_with(PUNCTUATION_REGEX));
    }

    #[test]
    fn test_short_cite_re() {
        let original = r"(?P<reporter>U\.S\.) (?P<page>\d+)";
        let short = short_cite_re(original);
        // The replacement should insert the "at" pattern before (?P<page>
        assert_eq!(
            short,
            r"(?P<reporter>U\.S\.) at\s?(p(\.|age)?)? (?P<page>\d+)"
        );
    }

    #[test]
    fn test_reference_pin_cite_re() {
        let pattern = reference_pin_cite_re(&[r"U\.S\.", r"F\.3d"]);
        assert!(pattern.contains(r"U\.S\.|F\.3d"));
        // Should compile with fancy-regex (contains lookahead via PIN_CITE_REGEX)
        let _re = fancy_regex::Regex::new(&pattern).unwrap();
    }

    // -- Post full citation merge groups test --

    #[test]
    fn test_post_full_citation_merge_groups() {
        let re = &*POST_FULL_CITATION_RE;
        let text = " (2012)";
        let caps = re.captures(text).unwrap().unwrap();
        let groups = merge_branch_groups(&caps, POST_FULL_CITATION_GROUPS);
        assert_eq!(groups.get("year").unwrap().as_deref(), Some("2012"));
    }
}
