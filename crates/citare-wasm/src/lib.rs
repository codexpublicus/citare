//! WASM bindings for citare-core.
//!
//! Exposes `getCitations`, `resolveCitations`, and `annotateCitations` to
//! JavaScript via wasm-bindgen.

use std::collections::HashMap;

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
    pub span: (usize, usize),
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
    let js_citations: Vec<JsCitation> = citations.iter().map(citation_to_js).collect();
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

    let mut js_resolutions: Vec<JsResolution> = resolutions
        .into_iter()
        .map(|(resource, cites)| JsResolution {
            resource_text: resource.citation.matched_text().to_string(),
            citations: cites.iter().map(citation_to_js).collect(),
        })
        .collect();

    // Sort for deterministic output.
    js_resolutions.sort_by(|a, b| a.resource_text.cmp(&b.resource_text));

    serde_wasm_bindgen::to_value(&js_resolutions).unwrap_or(JsValue::NULL)
}

/// Insert annotation markers around citation spans in `plain_text`.
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
fn citation_to_js(citation: &citare_core::Citation) -> JsCitation {
    let (start, end) = citation.span();

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
        span: (start, end),
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
