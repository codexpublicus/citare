//! Convert citation spans from UTF-8 byte offsets to other units.
//!
//! citare's spans ([`Citation::span`], [`Citation::full_span`] and the
//! `pin_cite_span_*` metadata fields) are UTF-8 byte offsets into the text
//! passed to [`get_citations`](crate::get_citations). That is Rust's native
//! string index, so `&text[start..end]` is free, and it stays the canonical
//! representation. Other consumers count differently:
//!
//! - **Unicode code points** ([`OffsetUnit::Chars`]): Python's `str`
//!   indexing, so eyecite's spans, and the citation-bench protocol.
//!   Equivalent to `text.chars()` positions in Rust.
//! - **UTF-16 code units** ([`OffsetUnit::Utf16`]): JavaScript string
//!   indexes, as used by `String.prototype.slice`, `substring` and DOM
//!   `Range` offsets.
//!
//! [`OffsetMap`] converts any number of byte offsets over one text. Building
//! it is a single pass over the text that records only the non-ASCII
//! characters, and each lookup is a binary search over those, so converting
//! every span in a long opinion costs `O(n + k log m)` rather than `O(n)` per
//! span. For pure-ASCII text the map is empty and every unit is the identity.
//!
//! ```rust
//! use citare_core::offsets::{OffsetMap, OffsetUnit};
//!
//! let text = "“Roe” v. Wade, 410 U.S. 113 (1973) 🎉";
//! let citations = citare_core::get_citations(text, false, "", &[]);
//! let map = OffsetMap::new(text);
//! let (start, end) = citations[0].span();
//! assert_eq!(&text[start..end], "410 U.S. 113");
//! // Two curly quotes are 3 bytes each but 1 code point and 1 UTF-16 unit.
//! assert_eq!(map.to_chars(start).unwrap(), start - 4);
//! assert_eq!(map.to_utf16(start).unwrap(), start - 4);
//! let spans = map.citation_spans(&citations[0], OffsetUnit::Utf16).unwrap();
//! assert_eq!(spans.span, (start - 4, end - 4));
//! ```

use std::fmt;
use std::str::FromStr;

use serde::Serialize;
use thiserror::Error;

use crate::models::Citation;

/// The unit a text offset counts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OffsetUnit {
    /// UTF-8 bytes: Rust `str` indexes. citare's native unit.
    #[default]
    Bytes,
    /// Unicode code points (scalar values): Python `str` indexes, eyecite.
    Chars,
    /// UTF-16 code units: JavaScript string indexes.
    Utf16,
}

impl OffsetUnit {
    /// The lowercase name used by the CLI: `bytes`, `chars` or `utf16`.
    pub fn as_str(self) -> &'static str {
        match self {
            OffsetUnit::Bytes => "bytes",
            OffsetUnit::Chars => "chars",
            OffsetUnit::Utf16 => "utf16",
        }
    }
}

impl fmt::Display for OffsetUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Error returned by [`OffsetUnit::from_str`] for an unknown unit name.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("unknown offset unit {0:?} (expected bytes, chars or utf16)")]
pub struct ParseOffsetUnitError(pub String);

impl FromStr for OffsetUnit {
    type Err = ParseOffsetUnitError;

    /// Accepts `bytes`, `chars` (alias `codepoints`) and `utf16`
    /// (alias `utf-16`), case-insensitively.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "bytes" | "utf8" | "utf-8" => Ok(OffsetUnit::Bytes),
            "chars" | "codepoints" | "code-points" => Ok(OffsetUnit::Chars),
            "utf16" | "utf-16" => Ok(OffsetUnit::Utf16),
            _ => Err(ParseOffsetUnitError(s.to_string())),
        }
    }
}

/// A byte offset that cannot be converted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum OffsetError {
    /// The offset is past the end of the text.
    #[error("byte offset {offset} is past the end of the text ({len} bytes)")]
    OutOfBounds { offset: usize, len: usize },
    /// The offset falls inside a multi-byte UTF-8 sequence, so it has no
    /// code point or UTF-16 equivalent.
    #[error("byte offset {offset} is not on a UTF-8 character boundary")]
    NotCharBoundary { offset: usize },
}

/// Running totals just past one non-ASCII character.
#[derive(Debug, Clone, Copy)]
struct Mark {
    /// Byte offset immediately after the character.
    byte: usize,
    /// Code points up to and including the character.
    chars: usize,
    /// UTF-16 code units up to and including the character.
    utf16: usize,
}

/// Converts UTF-8 byte offsets in one text to code point or UTF-16 offsets.
///
/// Build it once per text and reuse it for every span; see the
/// [module documentation](self). Offsets must come from the same text that
/// was passed to [`get_citations`](crate::get_citations) (after any
/// cleaning steps, since citare's spans index the cleaned text).
#[derive(Debug, Clone)]
pub struct OffsetMap<'a> {
    text: &'a str,
    /// One entry per non-ASCII character, in text order. Between two marks
    /// every byte is an ASCII character, so the three units advance together.
    marks: Vec<Mark>,
}

impl<'a> OffsetMap<'a> {
    /// Index `text` in a single pass.
    pub fn new(text: &'a str) -> Self {
        let mut marks: Vec<Mark> = Vec::new();
        let mut prev = Mark {
            byte: 0,
            chars: 0,
            utf16: 0,
        };
        for (i, ch) in text.char_indices() {
            if ch.is_ascii() {
                continue;
            }
            // Every byte between the previous mark and `i` is one ASCII
            // character, worth one code point and one UTF-16 unit.
            let ascii = i - prev.byte;
            prev = Mark {
                byte: i + ch.len_utf8(),
                chars: prev.chars + ascii + 1,
                utf16: prev.utf16 + ascii + ch.len_utf16(),
            };
            marks.push(prev);
        }
        OffsetMap { text, marks }
    }

    /// The text this map indexes.
    pub fn text(&self) -> &'a str {
        self.text
    }

    /// `true` when the text is pure ASCII, so all units coincide.
    pub fn is_ascii(&self) -> bool {
        self.marks.is_empty()
    }

    fn lookup(&self, offset: usize) -> Result<Mark, OffsetError> {
        if offset > self.text.len() {
            return Err(OffsetError::OutOfBounds {
                offset,
                len: self.text.len(),
            });
        }
        if !self.text.is_char_boundary(offset) {
            return Err(OffsetError::NotCharBoundary { offset });
        }
        let idx = self.marks.partition_point(|m| m.byte <= offset);
        let base = if idx == 0 {
            Mark {
                byte: 0,
                chars: 0,
                utf16: 0,
            }
        } else {
            self.marks[idx - 1]
        };
        let ascii = offset - base.byte;
        Ok(Mark {
            byte: offset,
            chars: base.chars + ascii,
            utf16: base.utf16 + ascii,
        })
    }

    /// Convert a byte offset to a Unicode code point offset.
    pub fn to_chars(&self, offset: usize) -> Result<usize, OffsetError> {
        self.lookup(offset).map(|m| m.chars)
    }

    /// Convert a byte offset to a UTF-16 code unit offset.
    pub fn to_utf16(&self, offset: usize) -> Result<usize, OffsetError> {
        self.lookup(offset).map(|m| m.utf16)
    }

    /// Convert a byte offset to `unit`. [`OffsetUnit::Bytes`] only checks
    /// that the offset is a valid character boundary.
    pub fn convert(&self, offset: usize, unit: OffsetUnit) -> Result<usize, OffsetError> {
        let m = self.lookup(offset)?;
        Ok(match unit {
            OffsetUnit::Bytes => m.byte,
            OffsetUnit::Chars => m.chars,
            OffsetUnit::Utf16 => m.utf16,
        })
    }

    /// Convert a `(start, end)` byte span to `unit`.
    pub fn convert_span(
        &self,
        span: (usize, usize),
        unit: OffsetUnit,
    ) -> Result<(usize, usize), OffsetError> {
        Ok((self.convert(span.0, unit)?, self.convert(span.1, unit)?))
    }

    /// Every span of `citation`, converted to `unit`.
    pub fn citation_spans(
        &self,
        citation: &Citation,
        unit: OffsetUnit,
    ) -> Result<CitationSpans, OffsetError> {
        CitationSpans::from_citation(citation).convert(self, unit)
    }
}

/// All the spans citare records for one citation, in one unit.
///
/// `span` covers the citation itself (`410 U.S. 113`), `full_span` adds the
/// case name, pin cite and parenthetical where citare found them, and the
/// pin cite bounds are set only when citare located a pin cite (often just
/// one of the two, mirroring eyecite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CitationSpans {
    pub span: (usize, usize),
    pub full_span: (usize, usize),
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
}

impl CitationSpans {
    /// The citation's spans in citare's native UTF-8 byte offsets.
    pub fn from_citation(citation: &Citation) -> Self {
        let (pin_cite_span_start, pin_cite_span_end) = citation.pin_cite_span();
        CitationSpans {
            span: citation.span(),
            full_span: citation.full_span(),
            pin_cite_span_start,
            pin_cite_span_end,
        }
    }

    /// Convert these byte spans to `unit` using `map`.
    pub fn convert(&self, map: &OffsetMap<'_>, unit: OffsetUnit) -> Result<Self, OffsetError> {
        let opt = |o: Option<usize>| o.map(|o| map.convert(o, unit)).transpose();
        Ok(CitationSpans {
            span: map.convert_span(self.span, unit)?,
            full_span: map.convert_span(self.full_span, unit)?,
            pin_cite_span_start: opt(self.pin_cite_span_start)?,
            pin_cite_span_end: opt(self.pin_cite_span_end)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference implementation: count by brute force.
    fn naive(text: &str, offset: usize) -> (usize, usize) {
        let prefix = &text[..offset];
        (prefix.chars().count(), prefix.encode_utf16().count())
    }

    const SAMPLES: &[&str] = &[
        "",
        "plain ascii",
        "é",
        "§ 1983",
        "“smart” quotes",
        "emoji 🎉 then text",
        "🎉🎉",
        "mixed: Café — Bär § 🎉 ok",
        "trailing é",
    ];

    #[test]
    fn matches_naive_count_at_every_boundary() {
        for text in SAMPLES {
            let map = OffsetMap::new(text);
            for offset in 0..=text.len() {
                if text.is_char_boundary(offset) {
                    let (chars, utf16) = naive(text, offset);
                    assert_eq!(map.to_chars(offset), Ok(chars), "{text:?} @ {offset}");
                    assert_eq!(map.to_utf16(offset), Ok(utf16), "{text:?} @ {offset}");
                    assert_eq!(map.convert(offset, OffsetUnit::Bytes), Ok(offset));
                } else {
                    assert_eq!(
                        map.to_chars(offset),
                        Err(OffsetError::NotCharBoundary { offset })
                    );
                }
            }
        }
    }

    #[test]
    fn emoji_counts_two_utf16_units() {
        let text = "a🎉b";
        let map = OffsetMap::new(text);
        // 'b' starts at byte 5, code point 2, UTF-16 unit 3.
        assert_eq!(map.to_chars(5), Ok(2));
        assert_eq!(map.to_utf16(5), Ok(3));
    }

    #[test]
    fn ascii_text_is_identity_and_has_no_marks() {
        let text = "Roe v. Wade, 410 U.S. 113 (1973).";
        let map = OffsetMap::new(text);
        assert!(map.is_ascii());
        for offset in 0..=text.len() {
            assert_eq!(map.to_chars(offset), Ok(offset));
            assert_eq!(map.to_utf16(offset), Ok(offset));
        }
    }

    #[test]
    fn rejects_out_of_bounds_and_mid_character_offsets() {
        let text = "é";
        let map = OffsetMap::new(text);
        assert_eq!(
            map.to_chars(3),
            Err(OffsetError::OutOfBounds { offset: 3, len: 2 })
        );
        assert_eq!(
            map.to_utf16(1),
            Err(OffsetError::NotCharBoundary { offset: 1 })
        );
        assert_eq!(
            map.convert(1, OffsetUnit::Bytes),
            Err(OffsetError::NotCharBoundary { offset: 1 })
        );
    }

    #[test]
    fn parses_unit_names() {
        assert_eq!("bytes".parse(), Ok(OffsetUnit::Bytes));
        assert_eq!("Chars".parse(), Ok(OffsetUnit::Chars));
        assert_eq!("codepoints".parse(), Ok(OffsetUnit::Chars));
        assert_eq!("utf16".parse(), Ok(OffsetUnit::Utf16));
        assert_eq!("UTF-16".parse(), Ok(OffsetUnit::Utf16));
        assert!("words".parse::<OffsetUnit>().is_err());
        for unit in [OffsetUnit::Bytes, OffsetUnit::Chars, OffsetUnit::Utf16] {
            assert_eq!(unit.to_string().parse(), Ok(unit));
        }
    }
}
