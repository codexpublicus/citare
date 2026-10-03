//! WASM bindings for citare-core.
//!
//! Exposes `getCitations`, `resolveCitations`, `annotateCitations` and
//! `cleanText` to JavaScript via wasm-bindgen.
//!
//! # Input options
//!
//! `getCitations(text, removeAmbiguous, options?)` and
//! `resolveCitations(text, removeAmbiguous, options?)` take an optional
//! options object:
//!
//! ```js
//! { format: "plain" | "html", cleanSteps: ["html", "all_whitespace", ...] }
//! ```
//!
//! - `format` (default `"plain"`): `"html"` means `text` is HTML or XML
//!   markup. citare strips it to plain text, finds citations there, and uses
//!   the markup's `<i>`/`<em>` tags to find case names and reference
//!   citations, as eyecite does with `markup_text`.
//! - `cleanSteps` (default `[]`): cleaning steps applied before citations
//!   are found, by name: `html`, `inline_whitespace`, `all_whitespace`,
//!   `underscores`, `xml`. With `format: "html"`, `html` is put first if it
//!   is missing. An unknown name or format throws.
//!
//! Omitting `options` behaves as before: plain text, no cleaning.
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
//!
//! All of these index the *cleaned* text, which is the input itself unless
//! `cleanSteps` or `format: "html"` changed it. `cleanText(text, steps)`
//! returns that text; for HTML input it is `cleanText(markup, ["html",
//! ...cleanSteps])` (without the extra `"html"` if `cleanSteps` already
//! starts with it).
//!
//! For `format: "html"` every citation also has a `markup` object of the
//! same shape (`span`, `full_span`, `pin_cite_span_*` in bytes, plus `utf16`
//! and `chars`) whose spans index the markup string itself, so
//! `markup.slice(...c.markup.utf16.span)` is the citation inside the HTML.
//! These come from the diff between the markup and its cleaned text that
//! citare also uses to find case names in `<i>`/`<em>` tags. A markup span
//! can start or end next to a tag it only half covers: a case name set in
//! `<i>` gives a `full_span` that runs from inside the `<i>` past its `</i>`,
//! and a reference citation can begin at the opening `<i>`.

use std::collections::HashMap;

use citare_core::{CitationSpans, CleanStep, Document, OffsetMap, OffsetUnit};
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
    /// The spans mapped onto the markup, for `format: "html"` input only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markup: Option<JsSpans>,
    pub groups: HashMap<String, Option<String>>,
    pub year: Option<i32>,
    pub harvard_slug: Option<String>,
    pub ambiguous: bool,
    pub metadata: HashMap<String, Option<String>>,
}

/// One set of citation spans in UTF-8 bytes, UTF-16 units and code points.
#[derive(Serialize)]
pub struct JsSpans {
    pub span: (usize, usize),
    pub full_span: (usize, usize),
    pub pin_cite_span_start: Option<usize>,
    pub pin_cite_span_end: Option<usize>,
    pub utf16: Option<CitationSpans>,
    pub chars: Option<CitationSpans>,
}

impl JsSpans {
    fn new(bytes: CitationSpans, map: &OffsetMap<'_>) -> Self {
        JsSpans {
            span: bytes.span,
            full_span: bytes.full_span,
            pin_cite_span_start: bytes.pin_cite_span_start,
            pin_cite_span_end: bytes.pin_cite_span_end,
            utf16: bytes.convert(map, OffsetUnit::Utf16).ok(),
            chars: bytes.convert(map, OffsetUnit::Chars).ok(),
        }
    }
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

/// What the input text is.
#[derive(Deserialize, Default, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InputFormat {
    /// Plain text.
    #[default]
    Plain,
    /// HTML or XML markup.
    Html,
}

/// The `options` argument of `getCitations` and `resolveCitations`.
#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FindOptions {
    #[serde(default)]
    pub format: InputFormat,
    #[serde(default)]
    pub clean_steps: Vec<String>,
}

#[wasm_bindgen(typescript_custom_section)]
const FIND_OPTIONS_TS: &str = r#"
/** Options for `getCitations` and `resolveCitations`. */
export interface FindOptions {
    /** `"html"` when the text is HTML or XML markup. Default `"plain"`. */
    format?: "plain" | "html";
    /** Clean steps applied before citations are found, in order. */
    cleanSteps?: Array<"html" | "inline_whitespace" | "all_whitespace" | "underscores" | "xml">;
}
"#;

#[wasm_bindgen]
extern "C" {
    /// The JS `FindOptions` object, read with serde.
    #[wasm_bindgen(typescript_type = "FindOptions")]
    pub type FindOptionsJs;
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

/// Find citations in `text` and return them as a JSON-serialisable array.
///
/// `remove_ambiguous` — when `true`, drop citations that could not be
/// unambiguously matched to a reporter edition.
///
/// `options` — optional `{ format: "plain" | "html", cleanSteps: [...] }`;
/// see the module docs. Throws on an unknown format or clean step.
#[wasm_bindgen(js_name = "getCitations")]
pub fn get_citations(
    text: &str,
    remove_ambiguous: bool,
    options: Option<FindOptionsJs>,
) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    let js_citations = find(text, remove_ambiguous, &options).map_err(|e| JsError::new(&e))?;
    Ok(serde_wasm_bindgen::to_value(&js_citations).unwrap_or(JsValue::NULL))
}

/// Find and resolve citations in `text` in one call.
///
/// Returns an array of `{ resource_text, citations }` objects, one per
/// distinct resource. `options` is as for `getCitations`.
#[wasm_bindgen(js_name = "resolveCitations")]
pub fn resolve_citations(
    text: &str,
    remove_ambiguous: bool,
    options: Option<FindOptionsJs>,
) -> Result<JsValue, JsError> {
    let options = parse_options(options)?;
    let js_resolutions = resolve(text, remove_ambiguous, &options).map_err(|e| JsError::new(&e))?;
    Ok(serde_wasm_bindgen::to_value(&js_resolutions).unwrap_or(JsValue::NULL))
}

/// Apply cleaning steps to `text` and return the result.
///
/// `steps` is an array of step names: `html`, `inline_whitespace`,
/// `all_whitespace`, `underscores`, `xml`, applied in order. Throws on an
/// unknown name.
#[wasm_bindgen(js_name = "cleanText")]
pub fn clean_text(text: &str, steps: Vec<String>) -> Result<String, JsError> {
    clean(text, &steps).map_err(|e| JsError::new(&e))
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
// Pipeline (plain Rust, so native tests can drive it)
// ---------------------------------------------------------------------------

/// Read the optional `options` argument.
fn parse_options(options: Option<FindOptionsJs>) -> Result<FindOptions, JsError> {
    let Some(options) = options.map(JsValue::from).filter(|o| !o.is_null()) else {
        return Ok(FindOptions::default());
    };
    serde_wasm_bindgen::from_value(options)
        .map_err(|e| JsError::new(&format!("invalid options: {e}")))
}

/// Look up clean steps by name.
fn parse_steps(names: &[String]) -> Result<Vec<CleanStep>, String> {
    names
        .iter()
        .map(|name| {
            CleanStep::from_name(name).ok_or_else(|| {
                format!(
                    "unknown clean step {name:?} (expected html, inline_whitespace, \
                     all_whitespace, underscores or xml)"
                )
            })
        })
        .collect()
}

/// Apply named clean steps to `text`.
fn clean(text: &str, steps: &[String]) -> Result<String, String> {
    let steps = parse_steps(steps)?;
    citare_core::clean_text(text, &steps).map_err(|e| e.to_string())
}

/// Build the document `options` describe and find its citations.
fn prepare(
    text: &str,
    remove_ambiguous: bool,
    options: &FindOptions,
) -> Result<(Document, Vec<citare_core::Citation>), String> {
    let mut steps = parse_steps(&options.clean_steps)?;
    let mut document = match options.format {
        InputFormat::Plain => Document::new(text, "", &steps),
        InputFormat::Html => {
            if !steps.iter().any(|s| matches!(s, CleanStep::Html)) {
                steps.insert(0, CleanStep::Html);
            }
            Document::new("", text, &steps)
        }
    };
    let citations = citare_core::get_citations_from_document(&mut document, remove_ambiguous);
    Ok((document, citations))
}

/// Converts citations from one document to their JS form.
struct Converter<'a> {
    plain: OffsetMap<'a>,
    /// The markup's offset map and the plain -> markup span map.
    markup: Option<(OffsetMap<'a>, &'a citare_core::SpanUpdater)>,
}

impl<'a> Converter<'a> {
    fn new(document: &'a Document) -> Self {
        let markup = match (&document.markup_text, &document.plain_to_markup) {
            (Some(text), Some(updater)) => Some((OffsetMap::new(text), updater)),
            _ => None,
        };
        Converter {
            plain: OffsetMap::new(&document.plain_text),
            markup,
        }
    }

    fn convert(&self, citation: &citare_core::Citation) -> JsCitation {
        let mut js = citation_to_js(citation, &self.plain);
        if let Some((map, updater)) = &self.markup {
            let bytes = CitationSpans::from_citation(citation).update(updater);
            js.markup = Some(JsSpans::new(bytes, map));
        }
        js
    }
}

fn find(
    text: &str,
    remove_ambiguous: bool,
    options: &FindOptions,
) -> Result<Vec<JsCitation>, String> {
    let (document, citations) = prepare(text, remove_ambiguous, options)?;
    let converter = Converter::new(&document);
    Ok(citations.iter().map(|c| converter.convert(c)).collect())
}

fn resolve(
    text: &str,
    remove_ambiguous: bool,
    options: &FindOptions,
) -> Result<Vec<JsResolution>, String> {
    let (document, citations) = prepare(text, remove_ambiguous, options)?;
    let resolutions = citare_core::resolve_citations(&citations);
    let converter = Converter::new(&document);

    let mut js_resolutions: Vec<JsResolution> = resolutions
        .into_iter()
        .map(|(resource, cites)| JsResolution {
            resource_text: resource.citation.matched_text().to_string(),
            citations: cites.iter().map(|c| converter.convert(c)).collect(),
        })
        .collect();

    // Sort for deterministic output.
    js_resolutions.sort_by(|a, b| a.resource_text.cmp(&b.resource_text));
    Ok(js_resolutions)
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
        markup: None,
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

    const HTML: &str = "<p>“Quoted” 🎉 In <i>Café v. Bär</i>, 410 U.S. 113, 115 \
        (1973), the court held. Later, in <i>Bär</i>, the point was settled. \
        <i>Id.</i> at 116.</p>";

    fn html_options() -> FindOptions {
        FindOptions {
            format: InputFormat::Html,
            clean_steps: vec![],
        }
    }

    /// `text` with tags and all whitespace removed, for comparing a markup
    /// slice against the plain text it became.
    fn bare(text: &str) -> String {
        let mut out = String::new();
        let mut in_tag = false;
        for ch in text.chars() {
            match ch {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag && !c.is_whitespace() => out.push(c),
                _ => {}
            }
        }
        out
    }

    fn utf16_slice(units: &[u16], (s, e): (usize, usize)) -> String {
        String::from_utf16(&units[s..e]).unwrap()
    }

    fn char_slice(chars: &[char], (s, e): (usize, usize)) -> String {
        chars[s..e].iter().collect()
    }

    #[test]
    fn options_parse_and_reject_unknowns() {
        let o: FindOptions =
            serde_json::from_str(r#"{"format":"html","cleanSteps":["all_whitespace"]}"#).unwrap();
        assert_eq!(o.format, InputFormat::Html);
        assert_eq!(o.clean_steps, ["all_whitespace"]);
        let o: FindOptions = serde_json::from_str("{}").unwrap();
        assert_eq!(o.format, InputFormat::Plain);
        assert!(serde_json::from_str::<FindOptions>(r#"{"format":"pdf"}"#).is_err());
        assert!(serde_json::from_str::<FindOptions>(r#"{"clean_steps":[]}"#).is_err());

        let bad = FindOptions {
            format: InputFormat::Plain,
            clean_steps: vec!["html".into(), "bogus".into()],
        };
        let err = find("1 U.S. 1", false, &bad).err().unwrap();
        assert!(err.contains("\"bogus\""), "{err}");
        assert!(clean("x", &["bogus".into()]).is_err());
    }

    #[test]
    fn html_input_finds_emphasis_case_names_and_references() {
        let citations = find(HTML, false, &html_options()).unwrap();
        let types: Vec<&str> = citations.iter().map(|c| c.citation_type.as_str()).collect();
        assert_eq!(
            types,
            ["FullCaseCitation", "ReferenceCitation", "IdCitation"]
        );
        let case = &citations[0];
        assert_eq!(case.metadata["plaintiff"].as_deref(), Some("Café"));
        assert_eq!(case.metadata["defendant"].as_deref(), Some("Bär"));
        assert_eq!(citations[1].matched_text.trim(), "Bär");

        // Plain input loses the reference: it needs the <i> tags.
        let plain = clean(HTML, &["html".into()]).unwrap();
        let plain_types: Vec<String> = find(&plain, false, &FindOptions::default())
            .unwrap()
            .into_iter()
            .map(|c| c.citation_type)
            .collect();
        assert!(!plain_types.iter().any(|t| t == "ReferenceCitation"));
    }

    #[test]
    fn html_spans_round_trip_into_cleaned_text_and_markup() {
        let options = html_options();
        let citations = find(HTML, false, &options).unwrap();
        let (document, _) = prepare(HTML, false, &options).unwrap();
        let plain = clean(HTML, &["html".into()]).unwrap();
        // cleanText reproduces the text the spans index.
        assert_eq!(plain, document.plain_text);

        let p16: Vec<u16> = plain.encode_utf16().collect();
        let pch: Vec<char> = plain.chars().collect();
        let m16: Vec<u16> = HTML.encode_utf16().collect();
        let mch: Vec<char> = HTML.chars().collect();

        for c in &citations {
            for (bytes, utf16, chars) in [
                (c.span, c.utf16.unwrap().span, c.chars.unwrap().span),
                (
                    c.full_span,
                    c.utf16.unwrap().full_span,
                    c.chars.unwrap().full_span,
                ),
            ] {
                let expected = &plain[bytes.0..bytes.1];
                assert_eq!(utf16_slice(&p16, utf16), expected);
                assert_eq!(char_slice(&pch, chars), expected);
            }
            assert!(plain[c.span.0..c.span.1].starts_with(&c.matched_text));

            let m = c.markup.as_ref().expect("markup spans for html input");
            let (mu, mc) = (m.utf16.unwrap(), m.chars.unwrap());
            for (bytes, utf16, chars, plain_bytes) in [
                (m.span, mu.span, mc.span, c.span),
                (m.full_span, mu.full_span, mc.full_span, c.full_span),
            ] {
                let in_markup = &HTML[bytes.0..bytes.1];
                assert_eq!(utf16_slice(&m16, utf16), in_markup);
                assert_eq!(char_slice(&mch, chars), in_markup);
                assert_eq!(
                    bare(in_markup),
                    bare(&plain[plain_bytes.0..plain_bytes.1]),
                    "{in_markup:?}"
                );
            }
        }

        let case = &citations[0];
        let m = case.markup.as_ref().unwrap();
        assert_eq!(&HTML[m.span.0..m.span.1], "410 U.S. 113");
        let pin = (m.span.1, m.pin_cite_span_end.unwrap());
        assert_eq!(&HTML[pin.0..pin.1], ", 115");
        // Non-ASCII precedes the citation, so every unit differs.
        assert!(m.chars.unwrap().span.0 < m.utf16.unwrap().span.0);
        assert!(m.utf16.unwrap().span.0 < m.span.0);
    }

    #[test]
    fn clean_steps_apply_to_plain_text() {
        let text = "Café  v.\tBär,\n\n410 U.S.\u{200b}113 (1973)";
        let options = FindOptions {
            format: InputFormat::Plain,
            clean_steps: vec!["all_whitespace".into()],
        };
        let cleaned = clean(text, &options.clean_steps).unwrap();
        assert_eq!(cleaned, "Café v. Bär, 410 U.S. 113 (1973)");
        let citations = find(text, false, &options).unwrap();
        assert_eq!(citations.len(), 1);
        let c = &citations[0];
        assert!(c.markup.is_none());
        assert_eq!(&cleaned[c.span.0..c.span.1], "410 U.S. 113");
        let units: Vec<u16> = cleaned.encode_utf16().collect();
        assert_eq!(utf16_slice(&units, c.utf16.unwrap().span), "410 U.S. 113");
        assert_eq!(
            &cleaned[c.full_span.0..c.full_span.1],
            "Café v. Bär, 410 U.S. 113 (1973)"
        );
    }

    #[test]
    fn resolve_accepts_html() {
        let groups = resolve(HTML, false, &html_options()).unwrap();
        assert_eq!(groups.len(), 1);
        let types: Vec<&str> = groups[0]
            .citations
            .iter()
            .map(|c| c.citation_type.as_str())
            .collect();
        assert_eq!(
            types,
            ["FullCaseCitation", "ReferenceCitation", "IdCitation"]
        );
        assert!(groups[0].citations.iter().all(|c| c.markup.is_some()));
    }

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
