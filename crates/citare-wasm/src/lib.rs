//! WASM bindings for citare-core.
//!
//! Exposes `getCitations`, `resolveCitations`, and `annotateCitations` to
//! JavaScript via wasm-bindgen.
//!
//! # Offsets
//!
//! Every citation carries its spans three ways:
//!
//! - `span`, `full_span`, `pin_cite_span_start`, `pin_cite_span_end`: UTF-8
//!   byte offsets, citare's native unit. Pass these back to
//!   `annotateCitations`. They are wrong for JavaScript string indexing
//!   whenever the text contains non-ASCII characters.
//! - `utf16`: the same spans in UTF-16 code units, i.e. JavaScript string
//!   indexes. `text.slice(...c.utf16.span)` returns the citation.
//! - `chars`: the same spans in Unicode code points, the unit eyecite,
//!   Python and the citation-bench protocol use. Store these when offsets
//!   must outlive the JS string, e.g. as a `charspan` in a text layer.
//!
//! `utf16` and `chars` are objects `{ span, full_span, pin_cite_span_start,
//! pin_cite_span_end }` and are `undefined` only if a span does not fall on
//! a character boundary, which would be a citare bug.

use std::collections::HashMap;

use citare_core::{CitationSpans, OffsetMap, OffsetUnit};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JS-friendly serialisable types
// ---------------------------------------------------------------------------

/// A citation object as seen from JavaScript.
#[derive(Serialize)]
pub struct JsCitation {
    pub citation_type: String,
    pub matched_text: String,
    /// UTF-8 byte span of the citation. See the module docs on offsets.
    pub span: (usize, usize),
    /// UTF-8 byte span including case name, pin cite and parenthetical.
    pub full_span: (usize, usize),
    /// UTF-8 byte offset where the pin cite starts, if citare found it.
    pub pin_cite_span_start: Option<usize>,
    /// UTF-8 byte offset where the pin cite ends, if citare found it.
    pub pin_cite_span_end: Option<usize>,
    /// All spans in UTF-16 code units (JavaScript string indexes).
    pub utf16: Option<CitationSpans>,
    /// All spans in Unicode code points (eyecite / Python indexes).
    pub chars: Option<CitationSpans>,
    pub groups: HashMap<String, Option<String>>,
    pub year: Option<i32>,
    pub harvard_slug: Option<String>,
    pub ambiguous: bool,
    pub metadata: HashMap<String, Option<String>>,
}

/// A resolution group: one resource and all citations that point to it.
#[derive(Serialize)]
pub struct JsResolution {
    pub resource_text: String,
    pub citations: Vec<JsCitation>,
}

/// An annotation descriptor supplied by JavaScript.
#[derive(Deserialize, Default)]
pub struct JsAnnotation {
    pub span: Vec<usize>,
    pub before: String,
    pub after: String,
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

/// Find citations in `text` and return them as a JSON-serialisable array.
///
/// `remove_ambiguous` — when `true`, drop citations that could not be
/// unambiguously matched to a reporter edition.
#[wasm_bindgen(js_name = "getCitations")]
pub fn get_citations(text: &str, remove_ambiguous: bool) -> JsValue {
    let citations = citare_core::get_citations(text, remove_ambiguous, "", &[]);
    let map = OffsetMap::new(text);
    let js_citations: Vec<JsCitation> = citations.iter().map(|c| citation_to_js(c, &map)).collect();
    serde_wasm_bindgen::to_value(&js_citations).unwrap_or(JsValue::NULL)
}

/// Find and resolve citations in `text` in one call.
///
/// Returns an array of `{ resource_text, citations }` objects, one per
/// distinct resource.
#[wasm_bindgen(js_name = "resolveCitations")]
pub fn resolve_citations(text: &str, remove_ambiguous: bool) -> JsValue {
    let citations = citare_core::get_citations(text, remove_ambiguous, "", &[]);
    let resolutions = citare_core::resolve_citations(&citations);
    let map = OffsetMap::new(text);

    let mut js_resolutions: Vec<JsResolution> = resolutions
        .into_iter()
        .map(|(resource, cites)| JsResolution {
            resource_text: resource.citation.matched_text().to_string(),
            citations: cites.iter().map(|c| citation_to_js(c, &map)).collect(),
        })
        .collect();

    // Sort for deterministic output.
    js_resolutions.sort_by(|a, b| a.resource_text.cmp(&b.resource_text));

    serde_wasm_bindgen::to_value(&js_resolutions).unwrap_or(JsValue::NULL)
}

/// Insert annotation markers around citation spans in `plain_text`.
///
/// Spans are UTF-8 byte offsets: pass a citation's `span` (or `full_span`)
/// field, not its `utf16` or `chars` spans.
///
/// `annotations_js` must be a JavaScript array of objects with the shape:
/// ```json
/// { "span": [start, end], "before": "<a>", "after": "</a>" }
/// ```
///
/// Returns the annotated string.
#[wasm_bindgen(js_name = "annotateCitations")]
pub fn annotate_citations(plain_text: &str, annotations_js: JsValue) -> JsValue {
    let annotations: Vec<JsAnnotation> =
        serde_wasm_bindgen::from_value(annotations_js).unwrap_or_default();

    let mut core_annotations: Vec<((usize, usize), String, String)> = annotations
        .into_iter()
        .filter(|a| a.span.len() >= 2)
        .map(|a| ((a.span[0], a.span[1]), a.before, a.after))
        .collect();

    let result = citare_core::annotate_citations(
        plain_text,
        &mut core_annotations,
        "",
        citare_core::UnbalancedTags::Unchecked,
        None,
        None,
    );

    JsValue::from_str(&result)
}

// ---------------------------------------------------------------------------
// Internal conversion helpers
// ---------------------------------------------------------------------------

/// Convert a `citare_core::Citation` into its JS-friendly representation.
fn citation_to_js(citation: &citare_core::Citation, map: &OffsetMap<'_>) -> JsCitation {
    let bytes = CitationSpans::from_citation(citation);
    let utf16 = bytes.convert(map, OffsetUnit::Utf16).ok();
    let chars = bytes.convert(map, OffsetUnit::Chars).ok();

    let year = citation.resource_common().and_then(|r| r.year);

    let harvard_slug = citation
        .resource_common()
        .and_then(|r| r.edition_guess.as_ref())
        .and_then(|e| e.harvard_slug.clone())
        .or_else(|| {
            citation
                .resource_common()
                .and_then(|r| r.all_editions.first())
                .and_then(|e| e.harvard_slug.clone())
        });

    let metadata = extract_metadata(citation);

    JsCitation {
        citation_type: citation.type_name().to_string(),
        matched_text: citation.matched_text().to_string(),
        span: bytes.span,
        full_span: bytes.full_span,
        pin_cite_span_start: bytes.pin_cite_span_start,
        pin_cite_span_end: bytes.pin_cite_span_end,
        utf16,
        chars,
        groups: citation.common().groups.clone(),
        year,
        harvard_slug,
        ambiguous: citation.is_ambiguous(),
        metadata,
    }
}

/// Flatten per-variant metadata structs into a uniform string map.
fn extract_metadata(citation: &citare_core::Citation) -> HashMap<String, Option<String>> {
    let mut map: HashMap<String, Option<String>> = HashMap::new();

    match citation {
        citare_core::Citation::FullCase(d) => {
            let m = &d.metadata;
            let c = &m.case;
            insert_opt(&mut map, "parenthetical", &c.parenthetical);
            insert_opt(&mut map, "pin_cite", &c.pin_cite);
            insert_opt(&mut map, "year", &c.year);
            insert_opt(&mut map, "month", &c.month);
            insert_opt(&mut map, "day", &c.day);
            insert_opt(&mut map, "court", &c.court);
            insert_opt(&mut map, "plaintiff", &m.plaintiff);
            insert_opt(&mut map, "defendant", &m.defendant);
            insert_opt(&mut map, "extra", &m.extra);
            insert_opt(&mut map, "antecedent_guess", &m.antecedent_guess);
            insert_opt(
                &mut map,
                "resolved_case_name_short",
                &m.resolved_case_name_short,
            );
            insert_opt(&mut map, "resolved_case_name", &m.resolved_case_name);
        }
        citare_core::Citation::ShortCase(d) => {
            let m = &d.metadata;
            let c = &m.case;
            insert_opt(&mut map, "parenthetical", &c.parenthetical);
            insert_opt(&mut map, "pin_cite", &c.pin_cite);
            insert_opt(&mut map, "year", &c.year);
            insert_opt(&mut map, "month", &c.month);
            insert_opt(&mut map, "day", &c.day);
            insert_opt(&mut map, "court", &c.court);
            insert_opt(&mut map, "antecedent_guess", &m.antecedent_guess);
        }
        citare_core::Citation::FullLaw(d) => {
            let m = &d.metadata;
            insert_opt(&mut map, "parenthetical", &m.parenthetical);
            insert_opt(&mut map, "pin_cite", &m.pin_cite);
            insert_opt(&mut map, "year", &m.year);
            insert_opt(&mut map, "month", &m.month);
            insert_opt(&mut map, "day", &m.day);
            insert_opt(&mut map, "publisher", &m.publisher);
        }
        citare_core::Citation::FullJournal(d) => {
            let m = &d.metadata;
            insert_opt(&mut map, "parenthetical", &m.parenthetical);
            insert_opt(&mut map, "pin_cite", &m.pin_cite);
            insert_opt(&mut map, "year", &m.year);
            insert_opt(&mut map, "month", &m.month);
            insert_opt(&mut map, "day", &m.day);
        }
        citare_core::Citation::Supra(d) => {
            let m = &d.metadata;
            insert_opt(&mut map, "parenthetical", &m.parenthetical);
            insert_opt(&mut map, "pin_cite", &m.pin_cite);
            insert_opt(&mut map, "antecedent_guess", &m.antecedent_guess);
            insert_opt(&mut map, "volume", &m.volume);
        }
        citare_core::Citation::Id(d) => {
            let m = &d.metadata;
            insert_opt(&mut map, "parenthetical", &m.parenthetical);
            insert_opt(&mut map, "pin_cite", &m.pin_cite);
        }
        citare_core::Citation::Reference(d) => {
            let m = &d.metadata;
            insert_opt(&mut map, "parenthetical", &m.parenthetical);
            insert_opt(&mut map, "pin_cite", &m.pin_cite);
            insert_opt(&mut map, "plaintiff", &m.plaintiff);
            insert_opt(&mut map, "defendant", &m.defendant);
            insert_opt(
                &mut map,
                "resolved_case_name_short",
                &m.resolved_case_name_short,
            );
            insert_opt(&mut map, "resolved_case_name", &m.resolved_case_name);
        }
        citare_core::Citation::Unknown(_) => {}
    }

    map
}

/// Insert `key -> value.clone()` into `map`.  The value is `None` when the
/// `Option` is `None`, so the key is still present for schema consistency.
#[inline]
fn insert_opt(map: &mut HashMap<String, Option<String>>, key: &str, value: &Option<String>) {
    map.insert(key.to_string(), value.clone());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_citation_offsets_index_non_ascii_text() {
        let text = "“Quoted” 🎉 Café v. Bär, 410 U.S. 113, 115 (1973). Id. at 116.";
        let citations = citare_core::get_citations(text, false, "", &[]);
        let map = OffsetMap::new(text);
        let units: Vec<u16> = text.encode_utf16().collect();
        let chars: Vec<char> = text.chars().collect();
        assert!(!citations.is_empty());
        for c in &citations {
            let js = citation_to_js(c, &map);
            // `span` can extend past `matched_text` (e.g. "Id. at 116"),
            // so compare against the byte slice, not the token text.
            let expected = &text[js.span.0..js.span.1];
            assert!(expected.starts_with(&js.matched_text));

            let (s, e) = js.utf16.expect("utf16 spans").span;
            assert_eq!(String::from_utf16(&units[s..e]).unwrap(), expected);

            let (s, e) = js.chars.expect("char spans").span;
            assert_eq!(chars[s..e].iter().collect::<String>(), expected);

            assert_eq!(js.full_span, c.full_span());
            // Non-ASCII before every citation, so the units must differ.
            assert!(js.utf16.unwrap().span.0 < js.span.0);
            assert!(js.chars.unwrap().span.0 < js.utf16.unwrap().span.0);
        }
    }
}
