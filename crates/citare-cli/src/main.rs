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
//! ```

use std::collections::HashMap;
use std::io::{self, Read};

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
    span: [usize; 2],
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
        } => {
            let input = resolve_input(text);
            let citations = citare_core::get_citations(&input, remove_ambiguous, "", &[]);
            let output: Vec<CitationJson> = citations.iter().map(citation_to_json).collect();
            print_json(&output);
        }

        Command::Resolve {
            text,
            remove_ambiguous,
        } => {
            let input = resolve_input(text);
            let citations = citare_core::get_citations(&input, remove_ambiguous, "", &[]);
            let resolutions = citare_core::resolve_citations(&citations);

            let mut output: Vec<ResolutionJson> = resolutions
                .into_iter()
                .map(|(resource, cites)| ResolutionJson {
                    resource: resource.citation.matched_text().to_string(),
                    citations: cites.iter().map(citation_to_json).collect(),
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

/// Serialise a value as indented JSON and print it.
fn print_json<T: Serialize>(value: &T) {
    let json = serde_json::to_string_pretty(value).expect("serialisation failed");
    println!("{}", json);
}

/// Convert a `citare_core::Citation` to its JSON-friendly representation.
fn citation_to_json(citation: &citare_core::Citation) -> CitationJson {
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

    CitationJson {
        citation_type: citation.type_name().to_string(),
        matched_text: citation.matched_text().to_string(),
        span: [start, end],
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
            insert_opt(&mut map, "resolved_case_name_short", &m.resolved_case_name_short);
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
            insert_opt(&mut map, "resolved_case_name_short", &m.resolved_case_name_short);
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
