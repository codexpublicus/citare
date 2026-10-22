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
//!
//! # Resolvers
//!
//! `resolveCitations(text, removeAmbiguous, options?, resolvers?)` takes
//! eyecite's resolver callbacks as an optional object, so citations can be
//! grouped by the caller's own resources (database ids, URLs):
//!
//! ```js
//! resolveCitations(text, false, undefined, {
//!   full: (citation) => lookup(citation),             // string | null | undefined
//!   shortcase: (citation, fullCitations) => undefined,
//!   supra: (citation, fullCitations) => undefined,
//!   reference: (citation, fullCitations) => undefined,
//!   id: (citation, lastResource) => undefined,
//! });
//! ```
//!
//! Every callback is optional and gets citations in the same form as
//! `getCitations`. It returns a resource id (a string), `null` to leave the
//! citation unresolved, or `undefined` for citare's built-in rule. The
//! built-in rules for short forms work on whatever `full` returned, so a
//! resolver that only maps full citations to ids gets their short, supra,
//! reference and id citations grouped under those ids too.
//! `fullCitations` is the full citations so far, in document order, as
//! `{ citation, resource }` with `resource` the id `full` gave it (`null`
//! when it gave none); it grows as resolution goes on, so copy it to keep
//! it. `lastResource` is the id the citation before resolved to, or `null`.
//! A callback that throws makes `resolveCitations` throw.
//!
//! With resolvers, each group also has `resource`, its id, unless it is a
//! built-in resource; `resource_text` is then the group's first full
//! citation.

use std::collections::BTreeMap;

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
    pub groups: BTreeMap<String, Option<String>>,
    pub year: Option<i32>,
    pub harvard_slug: Option<String>,
    /// The edition's stable slug: `harvard_slug` where CAP has the
    /// reporter, else one derived from the reporters-db edition name
    /// (`ny-slip-op`). A released slug for books CAP files under
    /// another slug keeps it, with CAP's as `harvard_slug` (`p-r-r`,
    /// `pr`). Every case reporter has one; laws and journals don't.
    pub slug: Option<String>,
    pub ambiguous: bool,
    pub metadata: BTreeMap<String, Option<String>>,
    /// A statute citation's jurisdiction, code, title or chapter, and
    /// section, each in its own field; absent for other citations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub law: Option<citare_core::models::LawFields>,
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
///
/// Non-exhaustive, so fields can be added without breaking Rust callers.
#[derive(Serialize)]
#[non_exhaustive]
pub struct JsResolution {
    /// The caller's resource id, when a resolver gave one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
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

#[wasm_bindgen(typescript_custom_section)]
const RESOLVERS_TS: &str = r#"
/** A full citation seen so far, with the resource `full` gave it. */
export interface ResolvedFullCitation {
    citation: any;
    resource: string | null;
}
/**
 * Resolver callbacks for `resolveCitations`. Each returns a resource id,
 * `null` for unresolved, or `undefined` for citare's built-in rule.
 */
export interface Resolvers {
    full?: (citation: any) => string | null | undefined;
    shortcase?: (citation: any, fullCitations: ResolvedFullCitation[]) => string | null | undefined;
    supra?: (citation: any, fullCitations: ResolvedFullCitation[]) => string | null | undefined;
    reference?: (citation: any, fullCitations: ResolvedFullCitation[]) => string | null | undefined;
    id?: (citation: any, lastResource: string | null) => string | null | undefined;
}
"#;

#[wasm_bindgen]
extern "C" {
    /// The JS `FindOptions` object, read with serde.
    #[wasm_bindgen(typescript_type = "FindOptions")]
    pub type FindOptionsJs;

    /// The JS `Resolvers` object.
    #[wasm_bindgen(typescript_type = "Resolvers")]
    pub type ResolversJs;
}

// ---------------------------------------------------------------------------
// Exported functions
// ---------------------------------------------------------------------------

/// Load the citare data bundle (`citare-data-<hash>.bin`, see
/// docs/data-bundle.md). A build without the `embedded-data` feature must
/// call this once before `getCitations` or `resolveCitations`; a build with
/// it may, to use a different bundle, if no citations have been found yet.
/// Throws if the bytes are not a bundle this build can read, or if data is
/// already loaded.
#[wasm_bindgen(js_name = "initData")]
pub fn init_data(bytes: &[u8]) -> Result<(), JsError> {
    citare_core::data::load(bytes).map_err(|e| JsError::new(&e.to_string()))
}

/// Whether citation data is available: embedded in this build, or loaded
/// with `initData`.
#[wasm_bindgen(js_name = "dataLoaded")]
pub fn data_loaded() -> bool {
    citare_core::data::is_available()
}

fn require_data() -> Result<(), JsError> {
    if citare_core::data::is_available() {
        Ok(())
    } else {
        Err(JsError::new(
            "citare data is not loaded: call initData(bytes) with a citare data bundle first",
        ))
    }
}

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
    require_data()?;
    let options = parse_options(options)?;
    let js_citations = find(text, remove_ambiguous, &options).map_err(|e| JsError::new(&e))?;
    Ok(serde_wasm_bindgen::to_value(&js_citations).unwrap_or(JsValue::NULL))
}

/// Find and resolve citations in `text` in one call.
///
/// Returns an array of `{ resource_text, citations }` objects, one per
/// distinct resource, in document order. `options` is as for
/// `getCitations`. `resolvers` are optional callbacks that resolve
/// citations to the caller's own resource ids; see the module docs. Throws
/// if a callback throws or returns something other than a string, `null`
/// or `undefined`.
#[wasm_bindgen(js_name = "resolveCitations")]
pub fn resolve_citations(
    text: &str,
    remove_ambiguous: bool,
    options: Option<FindOptionsJs>,
    resolvers: Option<ResolversJs>,
) -> Result<JsValue, JsError> {
    require_data()?;
    let options = parse_options(options)?;
    let resolvers = resolvers
        .map(JsValue::from)
        .filter(|r| !r.is_null() && !r.is_undefined());
    let js_resolutions = match resolvers {
        None => resolve(text, remove_ambiguous, &options),
        Some(resolvers) => {
            let (document, citations) =
                prepare(text, remove_ambiguous, &options).map_err(|e| JsError::new(&e))?;
            let converter = Converter::new(&document);
            let mut callbacks = JsCallbacks::new(&resolvers, &converter)?;
            resolve_with(&citations, &converter, &mut callbacks)
        }
    }
    .map_err(|e| JsError::new(&e))?;
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
    let converter = Converter::new(&document);
    // The built-in rules, through the one resolver the module compiles in.
    resolve_with(&citations, &converter, &mut Builtins)
}

// ---------------------------------------------------------------------------
// Resolver callbacks
// ---------------------------------------------------------------------------

/// A resource `resolveCitations` groups by: citare's own, or an id a
/// resolver callback returned.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Builtin(Box<citare_core::Resource>),
    Id(String),
}

/// Which resolver callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Full,
    Shortcase,
    Supra,
    Reference,
    Id,
}

/// What a resolver callback returned.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    /// A resource id.
    Id(String),
    /// `null`: leave the citation unresolved.
    Unresolved,
    /// `undefined`, or no callback: citare's built-in rule.
    Builtin,
}

/// The callbacks behind a [`CallbackResolver`]: JavaScript functions in the
/// browser, Rust closures in the native tests.
trait Callbacks {
    /// Call the `kind` callback on `citation`. `last` is the last
    /// resolution, passed to the `id` callback.
    fn call(
        &mut self,
        kind: Kind,
        citation: &citare_core::Citation,
        last: Option<&Key>,
    ) -> Result<Answer, String>;

    /// Record a full citation's resource for the short-form callbacks'
    /// `fullCitations`.
    fn full_resolved(&mut self, citation: &citare_core::Citation, resource: Option<&Key>);
}

/// No callbacks: citare's built-in rules for everything.
struct Builtins;

impl Callbacks for Builtins {
    fn call(
        &mut self,
        _: Kind,
        _: &citare_core::Citation,
        _: Option<&Key>,
    ) -> Result<Answer, String> {
        Ok(Answer::Builtin)
    }

    fn full_resolved(&mut self, _: &citare_core::Citation, _: Option<&Key>) {}
}

/// A core [`Resolver`](citare_core::Resolver) over [`Callbacks`]. The first
/// callback error stops further callbacks and is reported after resolution.
/// The callbacks are a trait object so that one copy of the resolution
/// code serves `resolve` and the JS callbacks.
struct CallbackResolver<'c> {
    callbacks: &'c mut dyn Callbacks,
    error: Option<String>,
}

impl CallbackResolver<'_> {
    fn ask(
        &mut self,
        kind: Kind,
        citation: &citare_core::Citation,
        last: Option<&Key>,
        builtin: impl FnOnce() -> Option<Key>,
    ) -> Option<Key> {
        if self.error.is_some() {
            return None;
        }
        match self.callbacks.call(kind, citation, last) {
            Ok(Answer::Id(id)) => Some(Key::Id(id)),
            Ok(Answer::Unresolved) => None,
            Ok(Answer::Builtin) => builtin(),
            Err(e) => {
                self.error = Some(e);
                None
            }
        }
    }
}

impl citare_core::Resolver for CallbackResolver<'_> {
    type Resource = Key;

    fn resolve_full(&mut self, citation: &citare_core::Citation) -> Option<Key> {
        let key = self.ask(Kind::Full, citation, None, || {
            Some(Key::Builtin(Box::new(citare_core::resolve_full_citation(
                citation,
            ))))
        });
        self.callbacks.full_resolved(citation, key.as_ref());
        key
    }

    fn resolve_shortcase(
        &mut self,
        citation: &citare_core::Citation,
        context: &citare_core::ResolveContext<'_, Key>,
    ) -> Option<Key> {
        self.ask(Kind::Shortcase, citation, None, || {
            context.resolve_shortcase(citation)
        })
    }

    fn resolve_supra(
        &mut self,
        citation: &citare_core::Citation,
        context: &citare_core::ResolveContext<'_, Key>,
    ) -> Option<Key> {
        self.ask(Kind::Supra, citation, None, || {
            context.resolve_supra(citation)
        })
    }

    fn resolve_reference(
        &mut self,
        citation: &citare_core::Citation,
        context: &citare_core::ResolveContext<'_, Key>,
    ) -> Option<Key> {
        self.ask(Kind::Reference, citation, None, || {
            context.resolve_reference(citation)
        })
    }

    fn resolve_id(
        &mut self,
        citation: &citare_core::Citation,
        context: &citare_core::ResolveContext<'_, Key>,
    ) -> Option<Key> {
        let last = context.last_resolution();
        self.ask(Kind::Id, citation, last, || context.resolve_id(citation))
    }
}

/// Resolve `citations` through `callbacks` and convert the groups.
fn resolve_with(
    citations: &[citare_core::Citation],
    converter: &Converter<'_>,
    callbacks: &mut dyn Callbacks,
) -> Result<Vec<JsResolution>, String> {
    let mut resolver = CallbackResolver {
        callbacks,
        error: None,
    };
    let resolutions = citare_core::resolve_citations_with(citations, &mut resolver);
    if let Some(error) = resolver.error {
        return Err(error);
    }

    // Document order, the order eyecite's insertion-ordered dict returns
    // resources in: a built-in resource by its own citation, an id by its
    // group's first citation.
    let mut resolutions: Vec<_> = resolutions.into_iter().collect();
    resolutions.sort_by_key(|(key, cites)| match key {
        Key::Builtin(resource) => resource.citation.span(),
        Key::Id(_) => cites[0].span(),
    });
    Ok(resolutions
        .into_iter()
        .map(|(key, cites)| {
            let (resource, resource_text) = match key {
                Key::Builtin(resource) => (None, resource.citation.matched_text().to_string()),
                Key::Id(id) => {
                    let first = cites.iter().find(|c| c.is_full()).unwrap_or(&cites[0]);
                    (Some(id), first.matched_text().to_string())
                }
            };
            JsResolution {
                resource,
                resource_text,
                citations: cites.iter().map(|c| converter.convert(c)).collect(),
            }
        })
        .collect())
}

/// The JavaScript `Resolvers` object's functions.
struct JsCallbacks<'a> {
    full: Option<js_sys::Function>,
    shortcase: Option<js_sys::Function>,
    supra: Option<js_sys::Function>,
    reference: Option<js_sys::Function>,
    id: Option<js_sys::Function>,
    /// `fullCitations`: `{ citation, resource }` for each full citation so far.
    full_citations: js_sys::Array,
    converter: &'a Converter<'a>,
}

impl<'a> JsCallbacks<'a> {
    fn new(resolvers: &JsValue, converter: &'a Converter<'a>) -> Result<Self, JsError> {
        let get = |name: &str| -> Result<Option<js_sys::Function>, JsError> {
            let value = js_sys::Reflect::get(resolvers, &JsValue::from_str(name))
                .map_err(|_| JsError::new("resolvers must be an object"))?;
            if value.is_undefined() || value.is_null() {
                return Ok(None);
            }
            value
                .dyn_into::<js_sys::Function>()
                .map(Some)
                .map_err(|_| JsError::new(&format!("resolvers.{name} must be a function")))
        };
        Ok(JsCallbacks {
            full: get("full")?,
            shortcase: get("shortcase")?,
            supra: get("supra")?,
            reference: get("reference")?,
            id: get("id")?,
            full_citations: js_sys::Array::new(),
            converter,
        })
    }

    fn to_js<T: Serialize>(value: &T) -> JsValue {
        serde_wasm_bindgen::to_value(value).unwrap_or(JsValue::NULL)
    }

    fn key_to_js(key: Option<&Key>) -> JsValue {
        match key {
            Some(Key::Id(id)) => JsValue::from_str(id),
            _ => JsValue::NULL,
        }
    }
}

impl Callbacks for JsCallbacks<'_> {
    fn call(
        &mut self,
        kind: Kind,
        citation: &citare_core::Citation,
        last: Option<&Key>,
    ) -> Result<Answer, String> {
        let (name, function) = match kind {
            Kind::Full => ("full", &self.full),
            Kind::Shortcase => ("shortcase", &self.shortcase),
            Kind::Supra => ("supra", &self.supra),
            Kind::Reference => ("reference", &self.reference),
            Kind::Id => ("id", &self.id),
        };
        let Some(function) = function else {
            return Ok(Answer::Builtin);
        };
        let js_citation = Self::to_js(&self.converter.convert(citation));
        let result = match kind {
            Kind::Full => function.call1(&JsValue::NULL, &js_citation),
            Kind::Id => function.call2(&JsValue::NULL, &js_citation, &Self::key_to_js(last)),
            _ => function.call2(&JsValue::NULL, &js_citation, &self.full_citations),
        };
        let value = result.map_err(|e| {
            let message = e
                .dyn_ref::<js_sys::Error>()
                .map(|e| String::from(e.message()))
                .or_else(|| e.as_string())
                .unwrap_or_else(|| format!("{e:?}"));
            format!("resolvers.{name} threw: {message}")
        })?;
        if value.is_undefined() {
            Ok(Answer::Builtin)
        } else if value.is_null() {
            Ok(Answer::Unresolved)
        } else if let Some(id) = value.as_string() {
            Ok(Answer::Id(id))
        } else {
            Err(format!(
                "resolvers.{name} must return a string, null or undefined"
            ))
        }
    }

    fn full_resolved(&mut self, citation: &citare_core::Citation, resource: Option<&Key>) {
        let entry = js_sys::Object::new();
        let citation = Self::to_js(&self.converter.convert(citation));
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str("citation"), &citation);
        let _ = js_sys::Reflect::set(
            &entry,
            &JsValue::from_str("resource"),
            &Self::key_to_js(resource),
        );
        self.full_citations.push(&entry);
    }
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
    let slug = citation
        .resource_common()
        .and_then(|r| r.edition_guess.as_ref())
        .and_then(|e| e.slug.clone())
        .or_else(|| {
            citation
                .resource_common()
                .and_then(|r| r.all_editions.first())
                .and_then(|e| e.slug.clone())
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
        groups: citation.common().groups.clone().into_iter().collect(),
        year,
        harvard_slug,
        slug,
        ambiguous: citation.is_ambiguous(),
        metadata,
        law: citation.law(),
    }
}

/// The citation's metadata as a uniform string map.
fn extract_metadata(citation: &citare_core::Citation) -> BTreeMap<String, Option<String>> {
    citation
        .metadata_fields()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.map(str::to_string)))
        .collect()
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

    /// Rust stand-ins for the JS resolver callbacks.
    struct FakeCallbacks<F> {
        answer: F,
        calls: Vec<(Kind, String, Option<Key>)>,
        full: Vec<(String, Option<Key>)>,
    }

    impl<F: FnMut(Kind, &citare_core::Citation) -> Result<Answer, String>> Callbacks
        for FakeCallbacks<F>
    {
        fn call(
            &mut self,
            kind: Kind,
            citation: &citare_core::Citation,
            last: Option<&Key>,
        ) -> Result<Answer, String> {
            self.calls
                .push((kind, citation.matched_text().to_string(), last.cloned()));
            (self.answer)(kind, citation)
        }

        fn full_resolved(&mut self, citation: &citare_core::Citation, resource: Option<&Key>) {
            self.full
                .push((citation.matched_text().to_string(), resource.cloned()));
        }
    }

    fn fake<F>(answer: F) -> FakeCallbacks<F>
    where
        F: FnMut(Kind, &citare_core::Citation) -> Result<Answer, String>,
    {
        FakeCallbacks {
            answer,
            calls: vec![],
            full: vec![],
        }
    }

    const RESOLVE_TEXT: &str = "Foo v. Bar, 1 U.S. 1, 3 (1999). Id. at 2. See Baz v. Qux, \
        2 U.S. 50 (2000). Foo, 1 U.S., at 4. Baz, supra, at 52.";

    fn run<F>(callbacks: &mut FakeCallbacks<F>) -> Result<Vec<JsResolution>, String>
    where
        F: FnMut(Kind, &citare_core::Citation) -> Result<Answer, String>,
    {
        let (document, citations) = prepare(RESOLVE_TEXT, false, &FindOptions::default())?;
        let converter = Converter::new(&document);
        resolve_with(&citations, &converter, callbacks)
    }

    fn texts(groups: &[JsResolution]) -> Vec<(Option<&str>, &str, Vec<&str>)> {
        groups
            .iter()
            .map(|g| {
                (
                    g.resource.as_deref(),
                    g.resource_text.as_str(),
                    g.citations
                        .iter()
                        .map(|c| c.matched_text.as_str())
                        .collect(),
                )
            })
            .collect()
    }

    #[test]
    fn resolvers_that_defer_match_the_built_in_resolution() {
        let mut callbacks = fake(|_, _| Ok(Answer::Builtin));
        let with = run(&mut callbacks).unwrap();
        let without = resolve(RESOLVE_TEXT, false, &FindOptions::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&with).unwrap(),
            serde_json::to_value(&without).unwrap()
        );
        assert!(serde_json::to_value(&with).unwrap()[0]
            .get("resource")
            .is_none());

        // ... and core's resolve_citations.
        let citations = citare_core::get_citations(RESOLVE_TEXT, false, "", &[]);
        let mut core: Vec<_> = citare_core::resolve_citations(&citations)
            .into_iter()
            .collect();
        core.sort_by_key(|(resource, _)| resource.citation.span());
        let core: Vec<(Option<&str>, &str, Vec<&str>)> = core
            .iter()
            .map(|(resource, cites)| {
                (
                    None,
                    resource.citation.matched_text(),
                    cites.iter().map(|c| c.matched_text()).collect(),
                )
            })
            .collect();
        assert_eq!(texts(&with), core);
        let kinds: Vec<Kind> = callbacks.calls.iter().map(|c| c.0).collect();
        assert_eq!(
            kinds,
            [
                Kind::Full,
                Kind::Id,
                Kind::Full,
                Kind::Shortcase,
                Kind::Supra
            ]
        );
    }

    #[test]
    fn full_resolver_ids_group_the_short_forms() {
        let mut callbacks = fake(|kind, c: &citare_core::Citation| {
            Ok(match (kind, c.matched_text()) {
                (Kind::Full, "1 U.S. 1") => Answer::Id("case-7".into()),
                (Kind::Full, "2 U.S. 50") => Answer::Id("case-9".into()),
                _ => Answer::Builtin,
            })
        });
        let groups = run(&mut callbacks).unwrap();
        assert_eq!(
            texts(&groups),
            [
                (
                    Some("case-7"),
                    "1 U.S. 1",
                    vec!["1 U.S. 1", "Id.", "1 U.S., at 4"]
                ),
                (Some("case-9"), "2 U.S. 50", vec!["2 U.S. 50", "supra,"]),
            ]
        );
        // The id callback sees the last resolution.
        let id_call = callbacks.calls.iter().find(|c| c.0 == Kind::Id).unwrap();
        assert_eq!(id_call.2, Some(Key::Id("case-7".into())));
        assert_eq!(
            callbacks.full,
            [
                ("1 U.S. 1".to_string(), Some(Key::Id("case-7".into()))),
                ("2 U.S. 50".to_string(), Some(Key::Id("case-9".into()))),
            ]
        );
    }

    #[test]
    fn null_leaves_a_citation_unresolved_and_ids_can_mix() {
        let mut callbacks = fake(|kind, c: &citare_core::Citation| {
            Ok(match (kind, c.matched_text()) {
                (Kind::Full, "2 U.S. 50") => Answer::Unresolved,
                (Kind::Id, _) => Answer::Unresolved,
                (Kind::Shortcase, _) => Answer::Id("short".into()),
                _ => Answer::Builtin,
            })
        });
        let groups = run(&mut callbacks).unwrap();
        assert_eq!(
            texts(&groups),
            [
                (None, "1 U.S. 1", vec!["1 U.S. 1"]),
                (Some("short"), "1 U.S., at 4", vec!["1 U.S., at 4"]),
            ]
        );
    }

    #[test]
    fn a_callback_error_fails_resolution() {
        let mut callbacks = fake(|kind, _: &citare_core::Citation| match kind {
            Kind::Supra => Err("resolvers.supra threw: no".to_string()),
            _ => Ok(Answer::Builtin),
        });
        assert_eq!(
            run(&mut callbacks).err().as_deref(),
            Some("resolvers.supra threw: no")
        );
    }
}
