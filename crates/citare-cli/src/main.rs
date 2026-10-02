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
//! ```
//!
//! `--offsets` sets the unit of every span in `find` and `resolve` output:
//! `bytes` (UTF-8, the default and citare's native unit), `chars` (Unicode
//! code points, as eyecite and Python count) or `utf16` (UTF-16 code units,
//! as JavaScript strings count).

use std::collections::HashMap;
use std::io::{self, Read};

use citare_core::{CitationSpans, OffsetMap, OffsetUnit};
use clap::{Parser, Subcommand};
use serde::Serialize;

// ---------------------------------------------------------------------------
// CLI argument types
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "citare", about = "Legal citation extraction CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract citations and output as a JSON array.
    Find {
        /// Input text (reads from stdin when omitted).
        #[arg(long)]
        text: Option<String>,
        /// Drop citations that could not be unambiguously matched to a
        /// reporter edition.
        #[arg(long, default_value_t = false)]
        remove_ambiguous: bool,
        /// Unit for span offsets: bytes (UTF-8, default), chars (Unicode
        /// code points, as in eyecite/Python) or utf16 (JavaScript string
        /// indexes).
        #[arg(long, default_value = "bytes", value_parser = parse_offset_unit)]
        offsets: OffsetUnit,
    },
    /// Extract and resolve citations; output as a JSON array of resource groups.
    Resolve {
        /// Input text (reads from stdin when omitted).
        #[arg(long)]
        text: Option<String>,
        /// Drop citations that could not be unambiguously matched to a
        /// reporter edition.
        #[arg(long, default_value_t = false)]
        remove_ambiguous: bool,
        /// Unit for span offsets: bytes (UTF-8, default), chars (Unicode
        /// code points, as in eyecite/Python) or utf16 (JavaScript string
        /// indexes).
        #[arg(long, default_value = "bytes", value_parser = parse_offset_unit)]
        offsets: OffsetUnit,
    },
    /// Extract citations and wrap each in `<cite>…</cite>` markers; output
    /// the annotated text string.
    Annotate {
        /// Input text (reads from stdin when omitted).
        #[arg(long)]
        text: Option<String>,
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
    /// Span offsets, in the unit chosen with `--offsets`.
    span: [usize; 2],
    full_span: [usize; 2],
    pin_cite_span_start: Option<usize>,
    pin_cite_span_end: Option<usize>,
    groups: HashMap<String, Option<String>>,
    year: Option<i32>,
    harvard_slug: Option<String>,
    ambiguous: bool,
    metadata: HashMap<String, Option<String>>,
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

    match cli.command {
        Command::Find {
            text,
            remove_ambiguous,
            offsets,
        } => {
            let input = resolve_input(text);
            let citations = citare_core::get_citations(&input, remove_ambiguous, "", &[]);
            let map = OffsetMap::new(&input);
            let output: Vec<CitationJson> = citations
                .iter()
                .map(|c| citation_to_json(c, &map, offsets))
                .collect();
            print_json(&output);
        }

        Command::Resolve {
            text,
            remove_ambiguous,
            offsets,
        } => {
            let input = resolve_input(text);
            let citations = citare_core::get_citations(&input, remove_ambiguous, "", &[]);
            let resolutions = citare_core::resolve_citations(&citations);
            let map = OffsetMap::new(&input);

            let mut output: Vec<ResolutionJson> = resolutions
                .into_iter()
                .map(|(resource, cites)| ResolutionJson {
                    resource: resource.citation.matched_text().to_string(),
                    citations: cites
                        .iter()
                        .map(|c| citation_to_json(c, &map, offsets))
                        .collect(),
                })
                .collect();

            // Deterministic ordering.
            output.sort_by(|a, b| a.resource.cmp(&b.resource));
            print_json(&output);
        }

        Command::Annotate { text } => {
            let input = resolve_input(text);
            let citations = citare_core::get_citations(&input, false, "", &[]);

            let mut annotations: Vec<((usize, usize), String, String)> = citations
                .iter()
                .map(|c| (c.span(), "<cite>".to_string(), "</cite>".to_string()))
                .collect();

            let annotated = citare_core::annotate_citations(
                &input,
                &mut annotations,
                "",
                citare_core::UnbalancedTags::Unchecked,
                None,
                None,
            );

            // Output as a JSON string so the caller always gets valid JSON.
            let json = serde_json::to_string(&annotated).expect("serialisation failed");
            println!("{}", json);
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

/// Convert a `citare_core::Citation` to its JSON-friendly representation.
///
/// Spans are converted to `unit` through `map`. A span that doesn't fall
/// on a character boundary would be a citare bug; it aborts with an error
/// rather than printing a misleading offset.
fn citation_to_json(
    citation: &citare_core::Citation,
    map: &OffsetMap<'_>,
    unit: OffsetUnit,
) -> CitationJson {
    let spans = CitationSpans::from_citation(citation)
        .convert(map, unit)
        .unwrap_or_else(|e| {
            eprintln!("citare: cannot convert span to {unit}: {e}");
            std::process::exit(1);
        });
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

    CitationJson {
        citation_type: citation.type_name().to_string(),
        matched_text: citation.matched_text().to_string(),
        span: [spans.span.0, spans.span.1],
        full_span: [spans.full_span.0, spans.full_span.1],
        pin_cite_span_start: spans.pin_cite_span_start,
        pin_cite_span_end: spans.pin_cite_span_end,
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

#[inline]
fn insert_opt(map: &mut HashMap<String, Option<String>>, key: &str, value: &Option<String>) {
    map.insert(key.to_string(), value.clone());
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
}
