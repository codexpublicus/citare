//! `citare` — command-line interface for citare-core.
//!
//! # Usage
//!
//! ```text
//! # Read from stdin
//! echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare find
//! echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare resolve
//! echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare annotate
//!
//! # Pass text directly
//! citare find --text "1 U.S. 1"
//! citare resolve --text "1 U.S. 1"
//! citare annotate --text "Foo v. Bar, 1 U.S. 1 (1999)."
//!
//! # Report spans as JavaScript (UTF-16) or Python (code point) offsets
//! citare find --offsets utf16 --text "“Roe” 410 U.S. 113"
//! citare find --offsets chars --text "“Roe” 410 U.S. 113"
//!
//! # HTML input, and cleaning
//! citare find --html < opinion.html
//! citare find --clean-steps all_whitespace,underscores < opinion.txt
//! citare clean --html --clean-steps all_whitespace < opinion.html
//!
//! # Debug output (eyecite's dump_citations)
//! citare dump --text "Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3."
//!
//! # Run every extractor, without the Aho-Corasick prefilter (slow)
//! citare find --tokenizer simple --text "1 U.S. 1"
//! ```
//!
//! `--offsets` sets the unit of every span in `find` and `resolve` output:
//! `bytes` (UTF-8, the default and citare's native unit), `chars` (Unicode
//! code points, as eyecite and Python count) or `utf16` (UTF-16 code units,
//! as JavaScript strings count).
//!
//! `--clean-steps` cleans the input before citations are found, and the
//! spans then index the cleaned text, which `citare clean` with the same
//! flags prints. `--html` marks the input as HTML or XML: citare strips it
//! to plain text (adding the `html` step first if `--clean-steps` lacks it)
//! and uses its `<i>`/`<em>` tags to find case names and reference
//! citations. Each citation from `find` and `resolve` then also has a
//! `markup` object whose spans index the HTML input itself, and `annotate`
//! inserts its markers into the HTML.
//!
//! `--tokenizer` picks how `find`, `resolve`, `annotate` and `dump`
//! tokenize: `aho-corasick` (the default) runs only the extractors whose
//! reporter strings appear in the text; `simple` runs all of them, as
//! eyecite's base `Tokenizer` does, and compiles every reporter regex on
//! first use. The two find the same citations, so `simple` is a check on
//! the prefilter.
//!
//! Resolution uses citare's built-in rules. Resolving to your own resources
//! (eyecite's resolver callbacks) is a library feature: `Resolver` in
//! citare-core, `resolvers` in citare-wasm.

use std::collections::BTreeMap;
use std::io::{self, Read, Write};

use std::io::IsTerminal;
use std::sync::LazyLock;

use citare_core::tokenizer::DEFAULT_TOKENIZER;
use citare_core::{
    CitationSpans, CleanStep, Document, OffsetMap, OffsetUnit, Tokenize, Tokenizer, UnbalancedTags,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

// ---------------------------------------------------------------------------
// CLI argument types
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "citare", about = "Legal citation extraction CLI")]
struct Cli {
    /// Load reporter and court data from this data bundle instead of the
    /// one built in (see `citare data export` and docs/data-bundle.md).
    #[arg(long, global = true, value_name = "FILE")]
    data: Option<std::path::PathBuf>,
    #[command(subcommand)]
    command: Command,
}

/// Where the input comes from and how to prepare it.
#[derive(Args)]
struct InputArgs {
    /// Input text (reads from stdin when omitted).
    #[arg(long)]
    text: Option<String>,
    /// The input is HTML or XML markup. Citations are found in its text,
    /// using its <i>/<em> tags for case names and references.
    #[arg(long, default_value_t = false)]
    html: bool,
    /// Comma-separated clean steps applied before citations are found:
    /// html, inline_whitespace, all_whitespace, underscores, xml.
    #[arg(long, value_delimiter = ',', value_parser = parse_clean_step)]
    clean_steps: Vec<String>,
}

/// Which tokenizer finds the citation tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum TokenizerChoice {
    /// Run only the extractors whose reporter strings appear in the text.
    #[default]
    AhoCorasick,
    /// Run every extractor (eyecite's base Tokenizer). Slow: it compiles
    /// every reporter regex.
    Simple,
}

impl TokenizerChoice {
    fn tokenizer(self) -> &'static dyn Tokenize {
        static SIMPLE: LazyLock<Tokenizer> = LazyLock::new(Tokenizer::new);
        match self {
            TokenizerChoice::AhoCorasick => &*DEFAULT_TOKENIZER,
            TokenizerChoice::Simple => &*SIMPLE,
        }
    }
}

/// When `dump` colors its output.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum ColorChoice {
    /// When stdout is a terminal.
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Subcommand)]
enum Command {
    /// Extract citations and output as a JSON array.
    Find {
        #[command(flatten)]
        input: InputArgs,
        /// Drop citations that could not be unambiguously matched to a
        /// reporter edition.
        #[arg(long, default_value_t = false)]
        remove_ambiguous: bool,
        /// Unit for span offsets: bytes (UTF-8, default), chars (Unicode
        /// code points, as in eyecite/Python) or utf16 (JavaScript string
        /// indexes).
        #[arg(long, default_value = "bytes", value_parser = parse_offset_unit)]
        offsets: OffsetUnit,
        /// Tokenizer: aho-corasick (default) or simple.
        #[arg(long, value_enum, default_value_t)]
        tokenizer: TokenizerChoice,
    },
    /// Extract and resolve citations; output as a JSON array of resource groups.
    Resolve {
        #[command(flatten)]
        input: InputArgs,
        /// Drop citations that could not be unambiguously matched to a
        /// reporter edition.
        #[arg(long, default_value_t = false)]
        remove_ambiguous: bool,
        /// Unit for span offsets: bytes (UTF-8, default), chars (Unicode
        /// code points, as in eyecite/Python) or utf16 (JavaScript string
        /// indexes).
        #[arg(long, default_value = "bytes", value_parser = parse_offset_unit)]
        offsets: OffsetUnit,
        /// Tokenizer: aho-corasick (default) or simple.
        #[arg(long, value_enum, default_value_t)]
        tokenizer: TokenizerChoice,
    },
    /// Extract citations and wrap each in `<cite>…</cite>` markers; output
    /// the annotated text (the HTML itself with `--html`) as a JSON string.
    Annotate {
        #[command(flatten)]
        input: InputArgs,
        /// What to do when a citation span in HTML input cuts across a tag:
        /// unchecked (insert markers anyway, the default, as eyecite does),
        /// skip (widen the span over nearby style tags, else drop it) or
        /// wrap (put markers around each piece so the HTML stays balanced).
        #[arg(long, default_value = "unchecked", value_parser = parse_unbalanced_tags)]
        unbalanced_tags: UnbalancedTags,
        /// Tokenizer: aho-corasick (default) or simple.
        #[arg(long, value_enum, default_value_t)]
        tokenizer: TokenizerChoice,
    },
    /// Describe each citation for debugging: its line of text, spans,
    /// groups and metadata (eyecite's dump_citations). Spans index the
    /// cleaned text.
    Dump {
        #[command(flatten)]
        input: InputArgs,
        /// Drop citations that could not be unambiguously matched to a
        /// reporter edition.
        #[arg(long, default_value_t = false)]
        remove_ambiguous: bool,
        /// Characters of context to show either side of a citation.
        #[arg(long, default_value_t = 30)]
        context_chars: usize,
        /// Color the output: auto (when stdout is a terminal), always or never.
        #[arg(long, value_enum, default_value_t)]
        color: ColorChoice,
        /// Tokenizer: aho-corasick (default) or simple.
        #[arg(long, value_enum, default_value_t)]
        tokenizer: TokenizerChoice,
    },
    /// Print the cleaned text that `find` with the same flags indexes.
    Clean {
        #[command(flatten)]
        input: InputArgs,
    },
    /// Inspect or export the reporter and court data bundle.
    #[command(subcommand)]
    Data(DataCommand),
}

#[derive(Subcommand)]
enum DataCommand {
    /// Print the bundle's format version, provenance and table sizes as JSON
    /// (the built-in bundle, or the one given with --data).
    Info,
    /// Write the built-in data bundle to a file, named
    /// `citare-data-<first 16 hex digits of its SHA-256>.bin` inside DIR.
    Export {
        /// Directory to write into.
        #[arg(long, default_value = ".")]
        dir: std::path::PathBuf,
    },
}

// ---------------------------------------------------------------------------
// Serialisable output types  (mirrors citare-wasm JsCitation / JsResolution)
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct CitationJson {
    #[serde(rename = "type")]
    citation_type: String,
    matched_text: String,
    /// Span offsets into the (cleaned) text, in the unit chosen with
    /// `--offsets`.
    span: [usize; 2],
    full_span: [usize; 2],
    pin_cite_span_start: Option<usize>,
    pin_cite_span_end: Option<usize>,
    /// The same spans in the HTML input, with `--html` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    markup: Option<SpansJson>,
    groups: BTreeMap<String, Option<String>>,
    year: Option<i32>,
    harvard_slug: Option<String>,
    /// The edition's stable slug: `harvard_slug` where CAP has the
    /// reporter, else one derived from the reporters-db edition name
    /// (`ny-slip-op`). Every case reporter has one; laws and journals
    /// don't.
    slug: Option<String>,
    ambiguous: bool,
    metadata: BTreeMap<String, Option<String>>,
    /// A statute citation's jurisdiction, code, title or chapter, and
    /// section, each in its own field; absent for other citations.
    #[serde(skip_serializing_if = "Option::is_none")]
    law: Option<citare_core::models::LawFields>,
}

#[derive(Serialize)]
struct SpansJson {
    span: [usize; 2],
    full_span: [usize; 2],
    pin_cite_span_start: Option<usize>,
    pin_cite_span_end: Option<usize>,
}

impl From<CitationSpans> for SpansJson {
    fn from(s: CitationSpans) -> Self {
        SpansJson {
            span: [s.span.0, s.span.1],
            full_span: [s.full_span.0, s.full_span.1],
            pin_cite_span_start: s.pin_cite_span_start,
            pin_cite_span_end: s.pin_cite_span_end,
        }
    }
}

#[derive(Serialize)]
struct ResolutionJson {
    resource: String,
    citations: Vec<CitationJson>,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    if let Some(path) = &cli.data {
        let bytes = std::fs::read(path).unwrap_or_else(|e| {
            eprintln!("error: reading {}: {e}", path.display());
            std::process::exit(2);
        });
        if let Err(e) = citare_core::data::load(&bytes) {
            eprintln!("error: {}: {e}", path.display());
            std::process::exit(2);
        }
    }

    match cli.command {
        Command::Data(DataCommand::Info) => {
            let data = citare_core::data::get();
            let b = &data.bundle;
            let meta: serde_json::Map<String, serde_json::Value> = b
                .meta
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string().into()))
                .collect();
            print_json(&serde_json::json!({
                "format_version": citare_core::data::FORMAT_VERSION,
                "meta": meta,
                "editions": b.editions.len(),
                "templates": b.templates.len(),
                "extractors": b.reporter_map.len() + b.oneoffs.len(),
                "court_keys": b.court_keys.len(),
            }));
        }

        Command::Data(DataCommand::Export { dir }) => {
            let bytes = citare_core::data::EMBEDDED_BUNDLE;
            let name = format!(
                "citare-data-{}.bin",
                &citare_core::data::content_hash(bytes)[..16]
            );
            let path = dir.join(name);
            std::fs::write(&path, bytes).unwrap_or_else(|e| {
                eprintln!("error: writing {}: {e}", path.display());
                std::process::exit(2);
            });
            println!("{}", path.display());
        }

        Command::Find {
            input,
            remove_ambiguous,
            offsets,
            tokenizer,
        } => {
            let (document, citations) = prepare(&input, remove_ambiguous, tokenizer);
            let converter = Converter::new(&document, offsets);
            let output: Vec<CitationJson> =
                citations.iter().map(|c| converter.convert(c)).collect();
            print_json(&output);
        }

        Command::Resolve {
            input,
            remove_ambiguous,
            offsets,
            tokenizer,
        } => {
            let (document, citations) = prepare(&input, remove_ambiguous, tokenizer);
            let resolutions = citare_core::resolve_citations(&citations);
            let converter = Converter::new(&document, offsets);

            // Document order of each resource's full citation, the order
            // eyecite's insertion-ordered dict returns them in.
            let mut resolutions: Vec<_> = resolutions.into_iter().collect();
            resolutions.sort_by_key(|(resource, _)| resource.citation.span());
            let output: Vec<ResolutionJson> = resolutions
                .into_iter()
                .map(|(resource, cites)| ResolutionJson {
                    resource: resource.citation.matched_text().to_string(),
                    citations: cites.iter().map(|c| converter.convert(c)).collect(),
                })
                .collect();
            print_json(&output);
        }

        Command::Annotate {
            input,
            unbalanced_tags,
            tokenizer,
        } => {
            let (document, citations) = prepare(&input, false, tokenizer);
            let annotated = annotate(&document, &citations, unbalanced_tags);
            // Output as a JSON string so the caller always gets valid JSON.
            let json = serde_json::to_string(&annotated).expect("serialisation failed");
            println!("{}", json);
        }

        Command::Dump {
            input,
            remove_ambiguous,
            context_chars,
            color,
            tokenizer,
        } => {
            let (document, citations) = prepare(&input, remove_ambiguous, tokenizer);
            let color = match color {
                ColorChoice::Auto => io::stdout().is_terminal(),
                ColorChoice::Always => true,
                ColorChoice::Never => false,
            };
            let dump =
                citare_core::dump_citations(&citations, &document.plain_text, context_chars, color);
            if !dump.is_empty() {
                println!("{dump}");
            }
        }

        Command::Clean { input } => {
            let document = build_document(&input);
            io::stdout()
                .write_all(document.plain_text.as_bytes())
                .expect("failed to write stdout");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read text from `--text` argument or from stdin.
fn resolve_input(text: Option<String>) -> String {
    match text {
        Some(t) => t,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .expect("failed to read stdin");
            buf
        }
    }
}

/// Check a `--clean-steps` entry names a built-in step.
fn parse_clean_step(s: &str) -> Result<String, String> {
    match CleanStep::from_name(s) {
        Some(_) => Ok(s.to_string()),
        None => Err(format!(
            "unknown clean step {s:?} (expected html, inline_whitespace, all_whitespace, \
             underscores or xml)"
        )),
    }
}

/// Parse the `--unbalanced-tags` value.
fn parse_unbalanced_tags(s: &str) -> Result<UnbalancedTags, String> {
    match s {
        "unchecked" => Ok(UnbalancedTags::Unchecked),
        "skip" => Ok(UnbalancedTags::Skip),
        "wrap" => Ok(UnbalancedTags::Wrap),
        _ => Err(format!(
            "unknown mode {s:?} (expected unchecked, skip or wrap)"
        )),
    }
}

/// Build the document described by `input`: its text, cleaned, and for
/// `--html` the markup and the maps between the two.
fn build_document_from(text: &str, input: &InputArgs) -> Document {
    let mut steps: Vec<CleanStep> = input
        .clean_steps
        .iter()
        .map(|name| CleanStep::from_name(name).expect("validated by clap"))
        .collect();
    if input.html {
        if !steps.iter().any(|s| matches!(s, CleanStep::Html)) {
            steps.insert(0, CleanStep::Html);
        }
        Document::new("", text, &steps)
    } else {
        Document::new(text, "", &steps)
    }
}

fn build_document(input: &InputArgs) -> Document {
    let text = resolve_input(input.text.clone());
    build_document_from(&text, input)
}

/// Build the document and find its citations.
fn prepare(
    input: &InputArgs,
    remove_ambiguous: bool,
    tokenizer: TokenizerChoice,
) -> (Document, Vec<citare_core::Citation>) {
    let mut document = build_document(input);
    let citations = citare_core::get_citations_from_document_with(
        &mut document,
        remove_ambiguous,
        tokenizer.tokenizer(),
    );
    (document, citations)
}

/// Wrap every citation in `<cite>` markers: in the markup for HTML input,
/// in the cleaned text otherwise.
fn annotate(
    document: &Document,
    citations: &[citare_core::Citation],
    unbalanced_tags: UnbalancedTags,
) -> String {
    let mut annotations: Vec<((usize, usize), String, String)> = citations
        .iter()
        .map(|c| (c.span(), "<cite>".to_string(), "</cite>".to_string()))
        .collect();
    citare_core::annotate_citations(
        &document.plain_text,
        &mut annotations,
        document.markup_text.as_deref().unwrap_or(""),
        unbalanced_tags,
        None,
        None,
    )
}

/// Converts citations from one document to their JSON form.
struct Converter<'a> {
    unit: OffsetUnit,
    plain: OffsetMap<'a>,
    /// The markup's offset map and the plain -> markup span map.
    markup: Option<(OffsetMap<'a>, &'a citare_core::SpanUpdater)>,
}

impl<'a> Converter<'a> {
    fn new(document: &'a Document, unit: OffsetUnit) -> Self {
        let markup = match (&document.markup_text, &document.plain_to_markup) {
            (Some(text), Some(updater)) => Some((OffsetMap::new(text), updater)),
            _ => None,
        };
        Converter {
            unit,
            plain: OffsetMap::new(&document.plain_text),
            markup,
        }
    }

    fn convert(&self, citation: &citare_core::Citation) -> CitationJson {
        let mut json = citation_to_json(citation, &self.plain, self.unit);
        if let Some((map, updater)) = &self.markup {
            let bytes = CitationSpans::from_citation(citation).update(updater);
            json.markup = Some(convert_spans(bytes, map, self.unit).into());
        }
        json
    }
}

/// Convert byte spans to `unit`. A span that doesn't fall on a character
/// boundary would be a citare bug; it aborts with an error rather than
/// printing a misleading offset.
fn convert_spans(spans: CitationSpans, map: &OffsetMap<'_>, unit: OffsetUnit) -> CitationSpans {
    spans.convert(map, unit).unwrap_or_else(|e| {
        eprintln!("citare: cannot convert span to {unit}: {e}");
        std::process::exit(1);
    })
}

/// Parse the `--offsets` value.
fn parse_offset_unit(s: &str) -> Result<OffsetUnit, String> {
    s.parse()
        .map_err(|e: citare_core::offsets::ParseOffsetUnitError| e.to_string())
}

/// Serialise a value as indented JSON and print it.
fn print_json<T: Serialize>(value: &T) {
    let json = serde_json::to_string_pretty(value).expect("serialisation failed");
    println!("{}", json);
}

/// Convert a `citare_core::Citation` to its JSON-friendly representation,
/// with spans in `unit` converted through `map`.
fn citation_to_json(
    citation: &citare_core::Citation,
    map: &OffsetMap<'_>,
    unit: OffsetUnit,
) -> CitationJson {
    let spans = convert_spans(CitationSpans::from_citation(citation), map, unit);
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

    CitationJson {
        citation_type: citation.type_name().to_string(),
        matched_text: citation.matched_text().to_string(),
        span: [spans.span.0, spans.span.1],
        full_span: [spans.full_span.0, spans.full_span.1],
        pin_cite_span_start: spans.pin_cite_span_start,
        pin_cite_span_end: spans.pin_cite_span_end,
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

    const TEXT: &str = "“Quoted” 🎉 Café v. Bär, 410 U.S. 113, 115 (1973). See § 1983.";

    fn case_json(unit: OffsetUnit) -> CitationJson {
        let citations = citare_core::get_citations(TEXT, false, "", &[]);
        let map = OffsetMap::new(TEXT);
        let case = citations
            .iter()
            .find(|c| c.type_name() == "FullCaseCitation")
            .expect("full case citation");
        citation_to_json(case, &map, unit)
    }

    #[test]
    fn offsets_flag_defaults_to_bytes() {
        let cli = Cli::try_parse_from(["citare", "find", "--text", "x"]).unwrap();
        let Command::Find { offsets, .. } = cli.command else {
            panic!("expected find");
        };
        assert_eq!(offsets, OffsetUnit::Bytes);

        let cli = Cli::try_parse_from(["citare", "resolve", "--offsets", "utf16"]).unwrap();
        let Command::Resolve { offsets, .. } = cli.command else {
            panic!("expected resolve");
        };
        assert_eq!(offsets, OffsetUnit::Utf16);

        assert!(Cli::try_parse_from(["citare", "find", "--offsets", "words"]).is_err());
    }

    #[test]
    fn spans_follow_the_requested_unit() {
        let bytes = case_json(OffsetUnit::Bytes);
        assert_eq!(&TEXT[bytes.span[0]..bytes.span[1]], "410 U.S. 113");

        let chars = case_json(OffsetUnit::Chars);
        let cps: Vec<char> = TEXT.chars().collect();
        let sliced: String = cps[chars.span[0]..chars.span[1]].iter().collect();
        assert_eq!(sliced, "410 U.S. 113");
        let sliced: String = cps[chars.full_span[0]..chars.full_span[1]].iter().collect();
        assert_eq!(sliced, &TEXT[bytes.full_span[0]..bytes.full_span[1]]);

        let utf16 = case_json(OffsetUnit::Utf16);
        let units: Vec<u16> = TEXT.encode_utf16().collect();
        let sliced = String::from_utf16(&units[utf16.span[0]..utf16.span[1]]).unwrap();
        assert_eq!(sliced, "410 U.S. 113");
        // The emoji is one code point but two UTF-16 units.
        assert_eq!(utf16.span[0], chars.span[0] + 1);
        assert!(chars.span[0] < bytes.span[0]);

        let pin = (
            bytes.span[1],
            bytes.pin_cite_span_end.expect("pin cite end"),
        );
        assert_eq!(&TEXT[pin.0..pin.1], ", 115");
        let pin = (utf16.span[1], utf16.pin_cite_span_end.unwrap());
        assert_eq!(String::from_utf16(&units[pin.0..pin.1]).unwrap(), ", 115");
    }

    const HTML: &str = "<p>“Quoted” 🎉 In <i>Café v. Bär</i>, 410 U.S. 113, 115 \
        (1973), the court held. Later, in <i>Bär</i>, the point was settled.</p>";

    fn input(html: bool, steps: &[&str]) -> InputArgs {
        InputArgs {
            text: None,
            html,
            clean_steps: steps.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn find_in(text: &str, args: &InputArgs, unit: OffsetUnit) -> (Document, Vec<CitationJson>) {
        let mut document = build_document_from(text, args);
        let citations = citare_core::get_citations_from_document(&mut document, false);
        let json = {
            let converter = Converter::new(&document, unit);
            citations.iter().map(|c| converter.convert(c)).collect()
        };
        (document, json)
    }

    /// Slice `text` by a span in `unit`.
    fn slice(text: &str, unit: OffsetUnit, [s, e]: [usize; 2]) -> String {
        match unit {
            OffsetUnit::Bytes => text[s..e].to_string(),
            OffsetUnit::Chars => text.chars().skip(s).take(e - s).collect(),
            OffsetUnit::Utf16 => {
                let units: Vec<u16> = text.encode_utf16().collect();
                String::from_utf16(&units[s..e]).unwrap()
            }
        }
    }

    /// `text` without tags or whitespace.
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

    #[test]
    fn html_and_clean_steps_flags_parse() {
        let cli = Cli::try_parse_from([
            "citare",
            "find",
            "--html",
            "--clean-steps",
            "html,all_whitespace",
        ])
        .unwrap();
        let Command::Find { input, .. } = cli.command else {
            panic!("expected find");
        };
        assert!(input.html);
        assert_eq!(input.clean_steps, ["html", "all_whitespace"]);

        for cmd in ["resolve", "annotate", "clean"] {
            let cli =
                Cli::try_parse_from(["citare", cmd, "--html", "--clean-steps", "underscores"]);
            assert!(cli.is_ok(), "{cmd}");
        }
        assert!(Cli::try_parse_from(["citare", "find", "--clean-steps", "html,bogus"]).is_err());
        assert!(Cli::try_parse_from(["citare", "annotate", "--unbalanced-tags", "fix"]).is_err());
    }

    #[test]
    fn html_spans_round_trip_into_cleaned_text_and_markup() {
        let args = input(true, &[]);
        for unit in [OffsetUnit::Bytes, OffsetUnit::Chars, OffsetUnit::Utf16] {
            let (document, citations) = find_in(HTML, &args, unit);
            let types: Vec<&str> = citations.iter().map(|c| c.citation_type.as_str()).collect();
            assert_eq!(types, ["FullCaseCitation", "ReferenceCitation"], "{unit}");
            assert_eq!(citations[0].metadata["plaintiff"].as_deref(), Some("Café"));

            for c in &citations {
                let m = c.markup.as_ref().expect("markup spans");
                for (plain_span, markup_span) in [(c.span, m.span), (c.full_span, m.full_span)] {
                    let plain = slice(&document.plain_text, unit, plain_span);
                    let in_markup = slice(HTML, unit, markup_span);
                    assert_eq!(bare(&plain), bare(&in_markup), "{unit} {in_markup:?}");
                }
            }
            let case = &citations[0];
            assert_eq!(slice(&document.plain_text, unit, case.span), "410 U.S. 113");
            let m = case.markup.as_ref().unwrap();
            assert_eq!(slice(HTML, unit, m.span), "410 U.S. 113");
            let pin = [m.span[1], m.pin_cite_span_end.unwrap()];
            assert_eq!(slice(HTML, unit, pin), ", 115");
        }
    }

    #[test]
    fn clean_steps_change_the_indexed_text() {
        let text = "Café  v.\tBär,\n\n410 U.S. 113 (1973)";
        let args = input(false, &["all_whitespace"]);
        let (document, citations) = find_in(text, &args, OffsetUnit::Utf16);
        assert_eq!(document.plain_text, "Café v. Bär, 410 U.S. 113 (1973)");
        assert!(citations[0].markup.is_none());
        assert_eq!(
            slice(
                &document.plain_text,
                OffsetUnit::Utf16,
                citations[0].full_span
            ),
            "Café v. Bär, 410 U.S. 113 (1973)"
        );

        // `clean --html` adds the html step and prints what `find` indexes.
        let cleaned = build_document_from(HTML, &input(true, &["all_whitespace"])).plain_text;
        assert!(cleaned.starts_with("“Quoted” 🎉 In Café v. Bär , 410 U.S. 113"));
        assert_eq!(
            cleaned,
            citare_core::clean_text(HTML, &[CleanStep::Html, CleanStep::AllWhitespace]).unwrap()
        );
    }

    #[test]
    fn annotate_html_marks_up_the_markup() {
        let args = input(true, &[]);
        let mut document = build_document_from(HTML, &args);
        let citations = citare_core::get_citations_from_document(&mut document, false);

        let out = annotate(&document, &citations, UnbalancedTags::Unchecked);
        assert!(out.starts_with("<p>“Quoted” 🎉 In <i>Café v. Bär</i>, <cite>410 U.S. 113</cite>"));
        assert!(out.ends_with("settled.</p>"));

        let out = annotate(&document, &citations, UnbalancedTags::Skip);
        assert!(
            out.contains("in <cite><i>Bär</i></cite>, the point"),
            "{out}"
        );

        // Plain input annotates the cleaned text.
        let mut plain = build_document_from(
            "Café  v.  Bär, 410 U.S. 113",
            &input(false, &["all_whitespace"]),
        );
        let citations = citare_core::get_citations_from_document(&mut plain, false);
        assert_eq!(
            annotate(&plain, &citations, UnbalancedTags::Unchecked),
            "Café v. Bär, <cite>410 U.S. 113</cite>"
        );
    }

    #[test]
    fn tokenizer_and_dump_flags_parse() {
        let cli = Cli::try_parse_from(["citare", "find", "--tokenizer", "simple"]).unwrap();
        let Command::Find { tokenizer, .. } = cli.command else {
            panic!("expected find");
        };
        assert_eq!(tokenizer, TokenizerChoice::Simple);
        for cmd in ["find", "resolve", "annotate", "dump"] {
            let cli = Cli::try_parse_from(["citare", cmd, "--tokenizer", "aho-corasick"]);
            assert!(cli.is_ok(), "{cmd}");
        }
        assert!(Cli::try_parse_from(["citare", "find", "--tokenizer", "hyperscan"]).is_err());

        let cli = Cli::try_parse_from([
            "citare",
            "dump",
            "--html",
            "--context-chars",
            "10",
            "--color",
            "never",
        ])
        .unwrap();
        let Command::Dump {
            input,
            context_chars,
            color,
            tokenizer,
            remove_ambiguous,
        } = cli.command
        else {
            panic!("expected dump");
        };
        assert!(input.html && !remove_ambiguous);
        assert_eq!(
            (context_chars, color, tokenizer),
            (10, ColorChoice::Never, TokenizerChoice::AhoCorasick)
        );
        assert!(Cli::try_parse_from(["citare", "dump", "--color", "sometimes"]).is_err());
    }

    #[test]
    fn simple_tokenizer_finds_what_the_default_finds() {
        let mut args = input(false, &[]);
        args.text = Some(TEXT.to_string());
        let spans = |tokenizer| -> Vec<(&'static str, (usize, usize))> {
            prepare(&args, false, tokenizer)
                .1
                .iter()
                .map(|c| (c.type_name(), c.span()))
                .collect()
        };
        let default = spans(TokenizerChoice::AhoCorasick);
        assert!(!default.is_empty());
        assert_eq!(spans(TokenizerChoice::Simple), default);
    }

    #[test]
    fn dump_describes_html_input_in_its_cleaned_text() {
        let args = input(true, &[]);
        let mut document = build_document_from(HTML, &args);
        let citations = citare_core::get_citations_from_document(&mut document, false);
        let dump = citare_core::dump_citations(&citations, &document.plain_text, 20, false);
        assert!(dump.starts_with("FullCaseCitation: "), "{dump}");
        assert!(dump.contains("ReferenceCitation: "), "{dump}");
        assert!(dump.contains("    * plaintiff=\"Café\""), "{dump}");
        assert!(!dump.contains('<'), "{dump}");
    }
}
