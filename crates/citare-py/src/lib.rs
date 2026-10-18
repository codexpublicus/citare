//! The native half of the `citare` Python package.
//!
//! The Python package (`python/citare`) gives citare eyecite's API shape:
//! the same functions, keyword arguments and citation classes. This module
//! is what those wrap: finding, resolving, annotating, cleaning and dumping,
//! on citare-core's Rust API. Spans cross the boundary as Python string
//! indices (code points), as eyecite's do; citare-core works in UTF-8 bytes.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use citare_core::annotate::UnbalancedTags;
use citare_core::models::{Edition, ReporterSource};
use citare_core::resolve::{resolve_citations_with, ResolveContext, Resolver};
use citare_core::tokenizer::{build_extractors, TokenType, DEFAULT_TOKENIZER, EDITIONS_LOOKUP};
use citare_core::{
    annotate_citations, clean_text, dump_citations, get_citations_from_document_with,
    AhocorasickTokenizer, Citation, CitationSpans, CleanStep, Document, OffsetMap, OffsetUnit,
    TokenExtractor, Tokenize, Tokenizer,
};
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};

// ---------------------------------------------------------------------------
// Errors raised inside callbacks
// ---------------------------------------------------------------------------

/// The first error a Python callback raised while Rust code that can't
/// return one was running. The call that ran it raises it afterwards.
#[derive(Clone, Default)]
struct ErrorSlot(Arc<Mutex<Option<PyErr>>>);

impl ErrorSlot {
    fn set(&self, err: PyErr) {
        let mut slot = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = Some(err);
        }
    }

    fn is_set(&self) -> bool {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    fn check(&self) -> PyResult<()> {
        match self.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Clean steps
// ---------------------------------------------------------------------------

/// eyecite's built-in clean step names.
const CLEAN_STEP_NAMES: &[&str] = &[
    "html",
    "inline_whitespace",
    "all_whitespace",
    "underscores",
    "xml",
];

/// Convert eyecite's `clean_steps` (names or callables) to citare's.
fn clean_steps(steps: Option<&Bound<'_, PyAny>>, errors: &ErrorSlot) -> PyResult<Vec<CleanStep>> {
    let Some(steps) = steps else {
        return Ok(Vec::new());
    };
    if steps.is_none() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for step in steps.try_iter()? {
        let step = step?;
        if let Ok(name) = step.extract::<String>() {
            match CleanStep::from_name(&name) {
                Some(s) => out.push(s),
                None => {
                    return Err(PyValueError::new_err(format!(
                        "clean_text steps must be callable or one of {CLEAN_STEP_NAMES:?}"
                    )))
                }
            }
        } else if step.is_callable() {
            let func: Py<PyAny> = step.unbind();
            let errors = errors.clone();
            out.push(CleanStep::Custom(Box::new(move |text: &str| {
                if errors.is_set() {
                    return text.to_string();
                }
                Python::attach(|py| {
                    match func
                        .call1(py, (text,))
                        .and_then(|r| r.extract::<String>(py))
                    {
                        Ok(s) => s,
                        Err(e) => {
                            errors.set(e);
                            text.to_string()
                        }
                    }
                })
            })));
        } else {
            return Err(PyValueError::new_err(format!(
                "clean_text steps must be callable or one of {CLEAN_STEP_NAMES:?}"
            )));
        }
    }
    Ok(out)
}

/// eyecite's `clean_text(text, steps)`.
#[pyfunction]
#[pyo3(signature = (text, steps))]
fn clean(text: &str, steps: &Bound<'_, PyAny>) -> PyResult<String> {
    let errors = ErrorSlot::default();
    let steps = clean_steps(Some(steps), &errors)?;
    let out = clean_text(text, &steps).map_err(|e| PyValueError::new_err(e.to_string()))?;
    errors.check()?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Offsets
// ---------------------------------------------------------------------------

/// Byte offset -> code point offset, for spans citare-core reports.
fn to_chars(map: &OffsetMap<'_>, offset: usize) -> usize {
    map.convert(offset, OffsetUnit::Chars).unwrap_or(offset)
}

/// Code point offset -> byte offset in `text`, for spans Python passes in.
/// Offsets past the end clamp to the end, like Python slicing.
struct CharToByte {
    /// Byte offset of each character, plus the text length at the end.
    bytes: Option<Vec<usize>>,
    len: usize,
}

impl CharToByte {
    fn new(text: &str) -> Self {
        let bytes = (!text.is_ascii()).then(|| {
            text.char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(text.len()))
                .collect()
        });
        CharToByte {
            bytes,
            len: text.len(),
        }
    }

    fn get(&self, offset: usize) -> usize {
        match &self.bytes {
            None => offset.min(self.len),
            Some(b) => b[offset.min(b.len() - 1)],
        }
    }
}

// ---------------------------------------------------------------------------
// Editions
// ---------------------------------------------------------------------------

/// A reporter from reporters-db (or a law code or journal), eyecite's
/// `Reporter`.
#[pyclass(frozen, eq, hash, module = "citare._citare", name = "Reporter")]
#[derive(PartialEq, Eq, Hash)]
struct PyReporter(citare_core::Reporter);

#[pymethods]
impl PyReporter {
    #[getter]
    fn short_name(&self) -> &str {
        &self.0.short_name
    }
    #[getter]
    fn name(&self) -> &str {
        &self.0.name
    }
    #[getter]
    fn cite_type(&self) -> &str {
        &self.0.cite_type
    }
    /// `reporters`, `laws` or `journals`: which reporters-db file it is from.
    #[getter]
    fn source(&self) -> &'static str {
        match self.0.source {
            ReporterSource::Reporters => "reporters",
            ReporterSource::Laws => "laws",
            ReporterSource::Journals => "journals",
        }
    }
    #[getter]
    fn is_scotus(&self) -> bool {
        self.0.is_scotus
    }
    #[getter]
    fn jurisdiction(&self) -> Option<&str> {
        self.0.jurisdiction.as_deref()
    }
    fn __repr__(&self) -> String {
        format!(
            "Reporter(short_name={:?}, name={:?}, cite_type={:?}, source={:?})",
            self.0.short_name,
            self.0.name,
            self.0.cite_type,
            self.source()
        )
    }
}

/// One edition of a reporter, eyecite's `Edition`.
#[pyclass(frozen, eq, hash, module = "citare._citare", name = "Edition")]
#[derive(PartialEq, Eq, Hash)]
struct PyEdition(Edition);

#[pymethods]
impl PyEdition {
    #[getter]
    fn reporter(&self) -> PyReporter {
        PyReporter(self.0.reporter.clone())
    }
    #[getter]
    fn short_name(&self) -> &str {
        &self.0.short_name
    }
    /// The first date the edition covers, as an ISO-8601 string (eyecite
    /// has a `datetime`).
    #[getter]
    fn start(&self) -> Option<&str> {
        self.0.start.as_deref()
    }
    /// The last date the edition covers, as an ISO-8601 string, or `None`
    /// while it is current.
    #[getter]
    fn end(&self) -> Option<&str> {
        self.0.end.as_deref()
    }
    #[getter]
    fn harvard_slug(&self) -> Option<&str> {
        self.0.harvard_slug.as_deref()
    }
    #[getter]
    fn slug(&self) -> Option<&str> {
        self.0.slug.as_deref()
    }
    fn includes_year(&self, year: i32) -> bool {
        self.0.includes_year(year)
    }
    fn __repr__(&self) -> String {
        format!("Edition(short_name={:?})", self.0.short_name)
    }
}

fn editions(list: &[Edition]) -> Vec<PyEdition> {
    list.iter().cloned().map(PyEdition).collect()
}

// ---------------------------------------------------------------------------
// Citations
// ---------------------------------------------------------------------------

/// A citation's offsets in code points.
#[derive(Clone, Copy)]
struct CharSpans {
    token: (usize, usize),
    span_start: Option<usize>,
    span_end: Option<usize>,
    full_span_start: Option<usize>,
    full_span_end: Option<usize>,
    spans: CitationSpans,
}

impl CharSpans {
    fn new(citation: &Citation, map: &OffsetMap<'_>) -> Self {
        let c = citation.common();
        let conv = |o: Option<usize>| o.map(|o| to_chars(map, o));
        let bytes = CitationSpans::from_citation(citation);
        let spans = bytes.convert(map, OffsetUnit::Chars).unwrap_or(bytes);
        CharSpans {
            token: (to_chars(map, c.token.start()), to_chars(map, c.token.end())),
            span_start: conv(c.span_start),
            span_end: conv(c.span_end),
            full_span_start: conv(c.full_span_start),
            full_span_end: conv(c.full_span_end),
            spans,
        }
    }
}

/// One citation found by citare. The Python citation classes
/// (`citare.models`) wrap it as `_raw`.
#[pyclass(frozen, module = "citare._citare", name = "RawCitation")]
struct RawCitation {
    inner: Arc<Citation>,
    chars: CharSpans,
}

fn citation_hash(citation: &Citation) -> u64 {
    let mut h = DefaultHasher::new();
    citation.hash(&mut h);
    h.finish()
}

#[pymethods]
impl RawCitation {
    /// The eyecite class name: `FullCaseCitation`, `IdCitation`, ...
    #[getter]
    fn type_name(&self) -> &'static str {
        self.inner.type_name()
    }
    #[getter]
    fn matched_text(&self) -> &str {
        self.inner.matched_text()
    }
    /// The citation token's index in the document's tokens.
    #[getter]
    fn index(&self) -> usize {
        self.inner.common().index
    }
    #[getter]
    fn token_start(&self) -> usize {
        self.chars.token.0
    }
    #[getter]
    fn token_end(&self) -> usize {
        self.chars.token.1
    }
    #[getter]
    fn span_start(&self) -> Option<usize> {
        self.chars.span_start
    }
    #[getter]
    fn span_end(&self) -> Option<usize> {
        self.chars.span_end
    }
    #[getter]
    fn full_span_start(&self) -> Option<usize> {
        self.chars.full_span_start
    }
    #[getter]
    fn full_span_end(&self) -> Option<usize> {
        self.chars.full_span_end
    }
    fn span(&self) -> (usize, usize) {
        self.chars.spans.span
    }
    fn full_span(&self) -> (usize, usize) {
        self.chars.spans.full_span
    }
    #[getter]
    fn pin_cite_span_start(&self) -> Option<usize> {
        self.chars.spans.pin_cite_span_start
    }
    #[getter]
    fn pin_cite_span_end(&self) -> Option<usize> {
        self.chars.spans.pin_cite_span_end
    }
    /// The token's named regex groups, `None` for groups that didn't match.
    fn groups<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let d = PyDict::new(py);
        let mut groups: Vec<_> = self.inner.common().groups.iter().collect();
        groups.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in groups {
            d.set_item(k, v.as_deref())?;
        }
        Ok(d)
    }
    /// `(name, value)` pairs, citare-core's `Citation::metadata_fields`.
    fn metadata_fields(&self) -> Vec<(&'static str, Option<String>)> {
        self.inner
            .metadata_fields()
            .into_iter()
            .map(|(k, v)| (k, v.map(str::to_string)))
            .collect()
    }
    #[getter]
    fn is_resource(&self) -> bool {
        self.inner.resource_common().is_some()
    }
    #[getter]
    fn year(&self) -> Option<i32> {
        self.inner.resource_common().and_then(|r| r.year)
    }
    #[getter]
    fn exact_editions(&self) -> Vec<PyEdition> {
        self.inner
            .resource_common()
            .map_or_else(Vec::new, |r| editions(&r.exact_editions))
    }
    #[getter]
    fn variation_editions(&self) -> Vec<PyEdition> {
        self.inner
            .resource_common()
            .map_or_else(Vec::new, |r| editions(&r.variation_editions))
    }
    #[getter]
    fn all_editions(&self) -> Vec<PyEdition> {
        self.inner
            .resource_common()
            .map_or_else(Vec::new, |r| editions(&r.all_editions))
    }
    #[getter]
    fn edition_guess(&self) -> Option<PyEdition> {
        self.inner
            .resource_common()
            .and_then(|r| r.edition_guess.clone())
            .map(PyEdition)
    }
    fn corrected_reporter(&self) -> Option<String> {
        self.inner.corrected_reporter()
    }
    fn is_ambiguous(&self) -> bool {
        self.inner.is_ambiguous()
    }
    /// A statute citation's parts (citare's `Citation::law`), or `None`.
    fn law<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some(law) = self.inner.law() else {
            return Ok(None);
        };
        let d = PyDict::new(py);
        d.set_item("jurisdiction", law.jurisdiction)?;
        d.set_item("code", law.code)?;
        d.set_item("subject", law.subject)?;
        d.set_item("title", law.title)?;
        d.set_item("chapter", law.chapter)?;
        d.set_item("section", law.section)?;
        Ok(Some(d))
    }
    /// citare-core's citation hash: eyecite's equivalence rules.
    fn __hash__(&self) -> u64 {
        citation_hash(&self.inner)
    }
    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.cast::<RawCitation>() {
            Ok(other) => *self.inner == *other.get().inner,
            Err(_) => false,
        }
    }
}

fn raw_of<'py>(citation: &Bound<'py, PyAny>) -> PyResult<Bound<'py, RawCitation>> {
    let raw = citation
        .getattr("_raw")
        .map_err(|_| PyTypeError::new_err("expected citations returned by citare.get_citations"))?;
    raw.cast_into::<RawCitation>()
        .map_err(|_| PyTypeError::new_err("expected citations returned by citare.get_citations"))
}

// ---------------------------------------------------------------------------
// Tokenizers
// ---------------------------------------------------------------------------

/// A token extractor: a regex and what its matches are. eyecite's
/// `TokenExtractor`. Copies (`copy.copy`) drop the compiled regex, so a
/// copy whose `regex` is changed compiles the new one.
#[pyclass(module = "citare._citare", name = "TokenExtractor")]
struct PyTokenExtractor(TokenExtractor);

fn token_type(name: &str) -> PyResult<TokenType> {
    Ok(match name {
        "citation" => TokenType::Citation,
        "id" => TokenType::Id,
        "supra" => TokenType::Supra,
        "stop_word" => TokenType::StopWord,
        "section" => TokenType::Section,
        "paragraph" => TokenType::Paragraph,
        "placeholder" => TokenType::Placeholder,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unknown token type {name:?}: expected citation, id, supra, stop_word, \
                 section, paragraph or placeholder"
            )))
        }
    })
}

#[pymethods]
impl PyTokenExtractor {
    /// An extractor of `token_type` tokens (`citation`, `id`, `supra`,
    /// `stop_word`, `section`, `paragraph`, `placeholder`). A citation
    /// extractor built here has no editions; copy one from
    /// `citare.tokenizers.EXTRACTORS` to keep them.
    #[new]
    #[pyo3(signature = (regex, token_type = "citation", strings = Vec::new(), case_insensitive = false))]
    fn new(
        regex: String,
        token_type: &str,
        strings: Vec<String>,
        case_insensitive: bool,
    ) -> PyResult<Self> {
        Ok(PyTokenExtractor(TokenExtractor::new(
            regex,
            self::token_type(token_type)?,
            Default::default(),
            strings,
            case_insensitive,
        )))
    }
    #[getter]
    fn regex(&self) -> &str {
        &self.0.regex
    }
    #[setter]
    fn set_regex(&mut self, regex: String) {
        // A fresh copy forgets the regex compiled from the old pattern.
        let mut copy = self.0.clone();
        copy.regex = regex;
        self.0 = copy;
    }
    #[getter]
    fn strings(&self) -> Vec<String> {
        self.0.strings.clone()
    }
    #[setter]
    fn set_strings(&mut self, strings: Vec<String>) {
        self.0.strings = strings;
    }
    #[getter]
    fn case_insensitive(&self) -> bool {
        self.0.case_insensitive
    }
    #[getter]
    fn exact_editions(&self) -> Vec<PyEdition> {
        editions(&self.0.extra.exact_editions)
    }
    #[getter]
    fn variation_editions(&self) -> Vec<PyEdition> {
        editions(&self.0.extra.variation_editions)
    }
    #[getter]
    fn short(&self) -> bool {
        self.0.extra.short
    }
    fn __copy__(&self) -> Self {
        PyTokenExtractor(self.0.clone())
    }
    fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
        PyTokenExtractor(self.0.clone())
    }
    fn __repr__(&self) -> String {
        format!("TokenExtractor(regex={:?})", self.0.regex)
    }
}

fn extractors_of(list: Option<Vec<PyRef<'_, PyTokenExtractor>>>) -> Option<Vec<TokenExtractor>> {
    list.map(|l| l.iter().map(|e| e.0.clone()).collect())
}

/// citare's built-in extractors, eyecite's `EXTRACTORS`.
#[pyfunction]
fn extractors() -> Vec<PyTokenExtractor> {
    build_extractors()
        .into_iter()
        .map(PyTokenExtractor)
        .collect()
}

/// The editions a reporter string (canonical or a variation) names:
/// eyecite's `EDITIONS_LOOKUP[reporter]`.
#[pyfunction]
fn editions_lookup(reporter: &str) -> Vec<PyEdition> {
    EDITIONS_LOOKUP
        .get(reporter)
        .map_or_else(Vec::new, |e| editions(e))
}

/// eyecite's base `Tokenizer`: runs every extractor on every text. With no
/// extractors, citare's built-in ones. Slow; `AhocorasickTokenizer` finds
/// the same citations.
#[pyclass(frozen, module = "citare._citare", name = "Tokenizer")]
struct PyTokenizer(Arc<Tokenizer>);

#[pymethods]
impl PyTokenizer {
    #[new]
    #[pyo3(signature = (extractors = None))]
    fn new(extractors: Option<Vec<PyRef<'_, PyTokenExtractor>>>) -> Self {
        PyTokenizer(Arc::new(match extractors_of(extractors) {
            Some(e) => Tokenizer::from_extractors(e),
            None => Tokenizer::new(),
        }))
    }
    #[getter]
    fn extractors(&self) -> Vec<PyTokenExtractor> {
        self.0
            .extractors()
            .iter()
            .cloned()
            .map(PyTokenExtractor)
            .collect()
    }
}

/// eyecite's `AhocorasickTokenizer`: runs only the extractors whose strings
/// a text contains. With no extractors it is citare's default tokenizer,
/// shared by every call.
#[pyclass(frozen, module = "citare._citare", name = "AhocorasickTokenizer")]
struct PyAhocorasickTokenizer(Option<Arc<AhocorasickTokenizer>>);

#[pymethods]
impl PyAhocorasickTokenizer {
    #[new]
    #[pyo3(signature = (extractors = None))]
    fn new(extractors: Option<Vec<PyRef<'_, PyTokenExtractor>>>) -> Self {
        PyAhocorasickTokenizer(
            extractors_of(extractors).map(|e| Arc::new(AhocorasickTokenizer::from_extractors(e))),
        )
    }
    #[getter]
    fn extractors(&self) -> Vec<PyTokenExtractor> {
        let tokenizer: &AhocorasickTokenizer = match &self.0 {
            Some(t) => t,
            None => &DEFAULT_TOKENIZER,
        };
        tokenizer
            .extractors()
            .iter()
            .cloned()
            .map(PyTokenExtractor)
            .collect()
    }
}

/// A tokenizer that can cross threads, so finding can run detached from
/// the interpreter.
enum AnyTokenizer {
    Default,
    Simple(Arc<Tokenizer>),
    Aho(Arc<AhocorasickTokenizer>),
}

impl AnyTokenizer {
    fn from_py(tokenizer: Option<&Bound<'_, PyAny>>) -> PyResult<Self> {
        let Some(t) = tokenizer.filter(|t| !t.is_none()) else {
            return Ok(AnyTokenizer::Default);
        };
        if let Ok(t) = t.cast::<PyAhocorasickTokenizer>() {
            return Ok(match &t.get().0 {
                Some(a) => AnyTokenizer::Aho(a.clone()),
                None => AnyTokenizer::Default,
            });
        }
        if let Ok(t) = t.cast::<PyTokenizer>() {
            return Ok(AnyTokenizer::Simple(t.get().0.clone()));
        }
        Err(PyTypeError::new_err(
            "tokenizer must be a citare.tokenizers.Tokenizer or AhocorasickTokenizer",
        ))
    }

    fn get(&self) -> &dyn Tokenize {
        match self {
            AnyTokenizer::Default => &*DEFAULT_TOKENIZER,
            AnyTokenizer::Simple(t) => &**t,
            AnyTokenizer::Aho(t) => &**t,
        }
    }
}

// ---------------------------------------------------------------------------
// Finding
// ---------------------------------------------------------------------------

/// The text a find ran on. The Python `Document` wraps it.
#[pyclass(frozen, module = "citare._citare", name = "RawDocument")]
struct RawDocument {
    #[pyo3(get)]
    plain_text: String,
    #[pyo3(get)]
    markup_text: Option<String>,
    #[pyo3(get)]
    source_text: String,
}

fn raw_document(document: &Document) -> RawDocument {
    RawDocument {
        plain_text: document.plain_text.clone(),
        markup_text: document.markup_text.clone(),
        source_text: document.source_text.clone(),
    }
}

/// eyecite's `Document(plain_text, markup_text, clean_steps)`, without
/// finding citations: the cleaned text spans index.
#[pyfunction]
#[pyo3(signature = (plain_text = "", markup_text = "", clean_steps = None))]
fn document(
    py: Python<'_>,
    plain_text: &str,
    markup_text: &str,
    clean_steps: Option<&Bound<'_, PyAny>>,
) -> PyResult<RawDocument> {
    let errors = ErrorSlot::default();
    let steps = self::clean_steps(clean_steps, &errors)?;
    let document = py.detach(|| Document::new(plain_text, markup_text, &steps));
    errors.check()?;
    Ok(raw_document(&document))
}

/// eyecite's `get_citations`. Returns the document and the citations.
#[pyfunction]
#[pyo3(signature = (plain_text = "", remove_ambiguous = false, tokenizer = None, markup_text = "", clean_steps = None))]
fn find(
    py: Python<'_>,
    plain_text: &str,
    remove_ambiguous: bool,
    tokenizer: Option<&Bound<'_, PyAny>>,
    markup_text: &str,
    clean_steps: Option<&Bound<'_, PyAny>>,
) -> PyResult<(RawDocument, Vec<RawCitation>)> {
    let errors = ErrorSlot::default();
    let steps = self::clean_steps(clean_steps, &errors)?;
    let tokenizer = AnyTokenizer::from_py(tokenizer)?;
    let (document, citations) = py.detach(|| {
        let mut document = Document::new(plain_text, markup_text, &steps);
        let citations =
            get_citations_from_document_with(&mut document, remove_ambiguous, tokenizer.get());
        (document, citations)
    });
    errors.check()?;
    let map = OffsetMap::new(&document.plain_text);
    let citations = citations
        .into_iter()
        .map(|c| RawCitation {
            chars: CharSpans::new(&c, &map),
            inner: Arc::new(c),
        })
        .collect();
    Ok((raw_document(&document), citations))
}

// ---------------------------------------------------------------------------
// Resolving
// ---------------------------------------------------------------------------

/// A Python object used as a resource: grouped by its `__hash__` and
/// `__eq__`, as a dict key would be.
struct PyResource {
    obj: Py<PyAny>,
    hash: isize,
}

impl Clone for PyResource {
    fn clone(&self) -> Self {
        PyResource {
            obj: Python::attach(|py| self.obj.clone_ref(py)),
            hash: self.hash,
        }
    }
}

impl Hash for PyResource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for PyResource {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
            && Python::attach(|py| self.obj.bind(py).eq(other.obj.bind(py)).unwrap_or(false))
    }
}

impl Eq for PyResource {}

/// eyecite's resolver callbacks as a citare `Resolver`. Callbacks that
/// weren't passed fall back to citare's rules over the resources the
/// others returned.
struct PyResolver<'py, 'a> {
    py: Python<'py>,
    citations: &'a [Citation],
    objects: Vec<Bound<'py, PyAny>>,
    full: Bound<'py, PyAny>,
    shortcase: Option<Bound<'py, PyAny>>,
    supra: Option<Bound<'py, PyAny>>,
    reference: Option<Bound<'py, PyAny>>,
    id: Option<Bound<'py, PyAny>>,
    /// eyecite's `resolved_full_cites`: `(citation, resource)` tuples.
    resolved_full_cites: Bound<'py, PyList>,
    /// eyecite's `resolutions`: resource -> citations, in document order.
    resolutions: Bound<'py, PyDict>,
    error: Option<PyErr>,
}

impl<'py> PyResolver<'py, '_> {
    /// The Python object for `citation`, which is one of `self.citations`.
    fn object(&self, citation: &Citation) -> Bound<'py, PyAny> {
        let base = self.citations.as_ptr() as usize;
        let at = (citation as *const Citation as usize - base) / std::mem::size_of::<Citation>();
        self.objects[at].clone()
    }

    fn resource(&mut self, value: PyResult<Bound<'py, PyAny>>) -> Option<PyResource> {
        let value = match value {
            Ok(v) => v,
            Err(e) => {
                self.fail(e);
                return None;
            }
        };
        // eyecite records a resolution only when it is truthy.
        match value.is_truthy() {
            Ok(false) => return None,
            Err(e) => {
                self.fail(e);
                return None;
            }
            Ok(true) => {}
        }
        match value.hash() {
            Ok(hash) => Some(PyResource {
                obj: value.unbind(),
                hash,
            }),
            Err(e) => {
                self.fail(e);
                None
            }
        }
    }

    fn fail(&mut self, err: PyErr) {
        if self.error.is_none() {
            self.error = Some(err);
        }
    }

    /// Record `citation` under `resource` in the `resolutions` dict the
    /// callbacks see and the call returns.
    fn record(&mut self, citation: &Citation, resource: &Option<PyResource>) {
        let Some(resource) = resource else { return };
        let object = self.object(citation);
        let result = (|| -> PyResult<()> {
            let key = resource.obj.bind(self.py);
            match self.resolutions.get_item(key)? {
                Some(list) => list.cast_into::<PyList>()?.append(object),
                None => self
                    .resolutions
                    .set_item(key, PyList::new(self.py, [object])?),
            }
        })();
        if let Err(e) = result {
            self.fail(e);
        }
    }

    fn short_form(
        &mut self,
        callback: Option<Bound<'py, PyAny>>,
        citation: &Citation,
        default: impl FnOnce() -> Option<PyResource>,
    ) -> Option<PyResource> {
        if self.error.is_some() {
            return None;
        }
        let resource = match callback {
            None => default(),
            Some(f) => {
                let value = f.call1((self.object(citation), self.resolved_full_cites.clone()));
                self.resource(value)
            }
        };
        self.record(citation, &resource);
        resource
    }
}

impl Resolver for PyResolver<'_, '_> {
    type Resource = PyResource;

    fn resolve_full(&mut self, citation: &Citation) -> Option<PyResource> {
        if self.error.is_some() {
            return None;
        }
        let object = self.object(citation);
        let value = self.full.call1((object.clone(),));
        let resource = self.resource(value);
        let entry = PyTuple::new(
            self.py,
            [
                object,
                resource
                    .as_ref()
                    .map_or_else(|| self.py.None(), |r| r.obj.clone_ref(self.py))
                    .into_bound(self.py),
            ],
        )
        .and_then(|t| self.resolved_full_cites.append(t));
        if let Err(e) = entry {
            self.fail(e);
        }
        self.record(citation, &resource);
        resource
    }

    fn resolve_shortcase(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, PyResource>,
    ) -> Option<PyResource> {
        let callback = self.shortcase.clone();
        self.short_form(callback, citation, || context.resolve_shortcase(citation))
    }

    fn resolve_supra(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, PyResource>,
    ) -> Option<PyResource> {
        let callback = self.supra.clone();
        self.short_form(callback, citation, || context.resolve_supra(citation))
    }

    fn resolve_reference(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, PyResource>,
    ) -> Option<PyResource> {
        let callback = self.reference.clone();
        self.short_form(callback, citation, || context.resolve_reference(citation))
    }

    fn resolve_id(
        &mut self,
        citation: &Citation,
        context: &ResolveContext<'_, PyResource>,
    ) -> Option<PyResource> {
        if self.error.is_some() {
            return None;
        }
        let resource = match self.id.clone() {
            None => context.resolve_id(citation),
            Some(f) => {
                let last = context
                    .last_resolution()
                    .map_or_else(|| self.py.None(), |r| r.obj.clone_ref(self.py));
                let value = f.call1((self.object(citation), last, self.resolutions.clone()));
                self.resource(value)
            }
        };
        self.record(citation, &resource);
        resource
    }
}

/// eyecite's `resolve_citations`. `resolve_full_citation` is required (the
/// Python wrapper passes eyecite's default); the others fall back to
/// citare's rules when `None`. Returns the resolutions dict.
#[pyfunction]
#[pyo3(signature = (citations, resolve_full_citation, resolve_shortcase_citation = None, resolve_supra_citation = None, resolve_reference_citation = None, resolve_id_citation = None))]
fn resolve<'py>(
    py: Python<'py>,
    citations: &Bound<'py, PyAny>,
    resolve_full_citation: Bound<'py, PyAny>,
    resolve_shortcase_citation: Option<Bound<'py, PyAny>>,
    resolve_supra_citation: Option<Bound<'py, PyAny>>,
    resolve_reference_citation: Option<Bound<'py, PyAny>>,
    resolve_id_citation: Option<Bound<'py, PyAny>>,
) -> PyResult<Bound<'py, PyDict>> {
    let objects: Vec<Bound<'py, PyAny>> = citations.try_iter()?.collect::<PyResult<_>>()?;
    let rust: Vec<Citation> = objects
        .iter()
        .map(|o| raw_of(o).map(|r| (*r.get().inner).clone()))
        .collect::<PyResult<_>>()?;
    let not_none = |f: Option<Bound<'py, PyAny>>| f.filter(|f| !f.is_none());
    let mut resolver = PyResolver {
        py,
        citations: &rust,
        objects,
        full: resolve_full_citation,
        shortcase: not_none(resolve_shortcase_citation),
        supra: not_none(resolve_supra_citation),
        reference: not_none(resolve_reference_citation),
        id: not_none(resolve_id_citation),
        resolved_full_cites: PyList::empty(py),
        resolutions: PyDict::new(py),
        error: None,
    };
    resolve_citations_with(&rust, &mut resolver);
    if let Some(err) = resolver.error {
        return Err(err);
    }
    Ok(resolver.resolutions)
}

// ---------------------------------------------------------------------------
// Annotating
// ---------------------------------------------------------------------------

/// Marks an annotation's `before`/`after` that isn't a string, so the
/// annotator gets the original object back. U+E000 is a private-use
/// character.
const MARK: char = '\u{E000}';

/// eyecite's `annotate_citations`. Spans are code point offsets into
/// `plain_text`.
#[pyfunction]
#[pyo3(signature = (plain_text, annotations, source_text = "", unbalanced_tags = "unchecked", annotator = None))]
fn annotate<'py>(
    py: Python<'py>,
    plain_text: &str,
    annotations: &Bound<'py, PyAny>,
    source_text: &str,
    unbalanced_tags: &str,
    annotator: Option<&Bound<'py, PyAny>>,
) -> PyResult<String> {
    let unbalanced = match unbalanced_tags {
        "unchecked" => UnbalancedTags::Unchecked,
        "skip" => UnbalancedTags::Skip,
        "wrap" => UnbalancedTags::Wrap,
        _ => {
            return Err(PyValueError::new_err(format!(
                "unbalanced_tags must be 'unchecked', 'skip' or 'wrap', not {unbalanced_tags:?}"
            )))
        }
    };
    let annotator = annotator.filter(|a| !a.is_none());
    let bytes = CharToByte::new(plain_text);
    // Objects that aren't strings, which only an annotator can take.
    let mut objects: Vec<Bound<'py, PyAny>> = Vec::new();
    let mut text_of = |value: Bound<'py, PyAny>| -> PyResult<String> {
        if let Ok(s) = value.extract::<String>() {
            return Ok(s);
        }
        if annotator.is_none() || unbalanced == UnbalancedTags::Wrap {
            return Err(PyTypeError::new_err(
                "annotation before/after values must be strings unless an annotator is \
                 passed (and unbalanced_tags isn't 'wrap')",
            ));
        }
        objects.push(value);
        Ok(format!("{MARK}{}{MARK}", objects.len() - 1))
    };
    let mut list: Vec<((usize, usize), String, String)> = Vec::new();
    for item in annotations.try_iter()? {
        let item = item?;
        let ((start, end), before, after): ((usize, usize), Bound<'py, PyAny>, Bound<'py, PyAny>) =
            item.extract()?;
        list.push((
            (bytes.get(start), bytes.get(end)),
            text_of(before)?,
            text_of(after)?,
        ));
    }

    let Some(annotator) = annotator else {
        return Ok(annotate_citations(
            plain_text,
            &mut list,
            source_text,
            unbalanced,
            None,
            None,
        ));
    };
    let errors = ErrorSlot::default();
    let back = |s: &str| -> Bound<'py, PyAny> {
        let index = s
            .strip_prefix(MARK)
            .and_then(|s| s.strip_suffix(MARK))
            .and_then(|i| i.parse::<usize>().ok());
        match index.and_then(|i| objects.get(i)) {
            Some(o) => o.clone(),
            None => pyo3::types::PyString::new(py, s).into_any(),
        }
    };
    let call = |before: &str, span: &str, after: &str| -> String {
        if errors.is_set() {
            return String::new();
        }
        match annotator
            .call1((back(before), span, back(after)))
            .and_then(|r| r.extract::<String>())
        {
            Ok(s) => s,
            Err(e) => {
                errors.set(e);
                String::new()
            }
        }
    };
    let out = annotate_citations(
        plain_text,
        &mut list,
        source_text,
        unbalanced,
        Some(&call),
        None,
    );
    errors.check()?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Dumping
// ---------------------------------------------------------------------------

/// eyecite's `dump_citations`: citare-core's debug description.
#[pyfunction]
#[pyo3(signature = (citations, text, context_chars = 30, color = true))]
fn dump(
    citations: &Bound<'_, PyAny>,
    text: &str,
    context_chars: usize,
    color: bool,
) -> PyResult<String> {
    let rust: Vec<Citation> = citations
        .try_iter()?
        .map(|c| raw_of(&c?).map(|r| (*r.get().inner).clone()))
        .collect::<PyResult<_>>()?;
    Ok(dump_citations(&rust, text, context_chars, color))
}

#[pymodule]
fn _citare(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<RawCitation>()?;
    m.add_class::<RawDocument>()?;
    m.add_class::<PyReporter>()?;
    m.add_class::<PyEdition>()?;
    m.add_class::<PyTokenExtractor>()?;
    m.add_class::<PyTokenizer>()?;
    m.add_class::<PyAhocorasickTokenizer>()?;
    m.add_function(wrap_pyfunction!(find, m)?)?;
    m.add_function(wrap_pyfunction!(document, m)?)?;
    m.add_function(wrap_pyfunction!(resolve, m)?)?;
    m.add_function(wrap_pyfunction!(annotate, m)?)?;
    m.add_function(wrap_pyfunction!(clean, m)?)?;
    m.add_function(wrap_pyfunction!(dump, m)?)?;
    m.add_function(wrap_pyfunction!(extractors, m)?)?;
    m.add_function(wrap_pyfunction!(editions_lookup, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
