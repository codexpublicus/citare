use std::collections::{HashMap, HashSet};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;

use citare_data_build::courts::parse_courts;
use citare_data_build::regexes::{
    id_regex, nonalphanum_boundaries_re, placeholder_citations_regex, short_cite_re,
    stop_word_regex, supra_regex, PARAGRAPH_REGEX, SECTION_REGEX, STOP_WORDS,
};
use citare_data_build::reporters::{parse_journals, parse_laws, parse_reporters};
use citare_data_build::template::process_variables;

// ---------------------------------------------------------------------------
// Edition representation for build-time deduplication
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct BuildEdition {
    reporter_short_name: String,
    reporter_name: String,
    reporter_cite_type: String,
    reporter_source: u8, // 0=Reporters, 1=Laws, 2=Journals
    short_name: String,
    start: Option<String>,
    end: Option<String>,
    harvard_slug: Option<String>,
}

// ---------------------------------------------------------------------------
// Extractor accumulator (same role as EditionsByRegexEntry in tokenizer.rs)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct EditionsByRegexEntry {
    editions: Vec<BuildEdition>,
    variations: Vec<BuildEdition>,
    strings: HashSet<String>,
    short: bool,
}

// ---------------------------------------------------------------------------
// Token type constants (must match TokenType enum in tokenizer.rs)
// ---------------------------------------------------------------------------
const TOKEN_TYPE_CITATION: u8 = 0;
const TOKEN_TYPE_ID: u8 = 1;
const TOKEN_TYPE_SUPRA: u8 = 2;
const TOKEN_TYPE_STOPWORD: u8 = 3;
const TOKEN_TYPE_SECTION: u8 = 4;
const TOKEN_TYPE_PARAGRAPH: u8 = 5;
const TOKEN_TYPE_PLACEHOLDER: u8 = 6;

// ---------------------------------------------------------------------------
// Generated extractor representation
// ---------------------------------------------------------------------------

struct GenExtractor {
    regex: String,
    token_type: u8,
    case_insensitive: bool,
    strings: Vec<String>,
    short: bool,
    edition_indices: Vec<usize>,
    variation_indices: Vec<usize>,
}

fn main() {
    println!("cargo::rerun-if-changed=../../data/");
    println!("cargo::rerun-if-changed=../../data/harvard_reporters.json");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir).join("generated_extractors.rs");

    // -----------------------------------------------------------------------
    // 1. Load and process regex variables with overrides
    // -----------------------------------------------------------------------
    let regexes_json =
        include_str!("../../data/reporters-db/reporters_db/data/regexes.json");
    let mut raw: serde_json::Value =
        serde_json::from_str(regexes_json).expect("Failed to parse regexes.json");

    let page_number_regex = citare_data_build::regexes::page_number_regex();

    // Apply the Python overrides before processing:
    if let Some(obj) = raw.as_object_mut() {
        if let Some(full_cite) = obj.get_mut("full_cite") {
            if let Some(fc_obj) = full_cite.as_object_mut() {
                fc_obj.insert(
                    String::new(),
                    serde_json::Value::String("$volume $reporter,? $page".to_string()),
                );
            }
        }
        if let Some(page) = obj.get_mut("page") {
            if let Some(page_obj) = page.as_object_mut() {
                page_obj.insert(
                    String::new(),
                    serde_json::Value::String(format!("(?P<page>{})", page_number_regex)),
                );
            }
        }
    }

    let regex_variables = process_variables(&raw);
    println!(
        "cargo::warning=Processed {} regex variables",
        regex_variables.len()
    );

    // -----------------------------------------------------------------------
    // 2. Parse data files
    // -----------------------------------------------------------------------
    let reporters_json =
        include_str!("../../data/reporters-db/reporters_db/data/reporters.json");
    let reporters = parse_reporters(reporters_json).expect("Failed to parse reporters.json");
    let total_editions: usize = reporters
        .values()
        .flat_map(|clusters| clusters.iter())
        .map(|c| c.editions.len())
        .sum();
    println!(
        "cargo::warning=Loaded {} reporters with {} editions",
        reporters.len(),
        total_editions
    );

    let laws_json = include_str!("../../data/reporters-db/reporters_db/data/laws.json");
    let laws = parse_laws(laws_json).expect("Failed to parse laws.json");
    println!("cargo::warning=Loaded {} laws", laws.len());

    let journals_json =
        include_str!("../../data/reporters-db/reporters_db/data/journals.json");
    let journals = parse_journals(journals_json).expect("Failed to parse journals.json");
    println!("cargo::warning=Loaded {} journals", journals.len());

    let courts_json = include_str!("../../data/courts-db/courts_db/data/courts.json");
    let courts = parse_courts(courts_json).expect("Failed to parse courts.json");
    println!("cargo::warning=Loaded {} courts", courts.len());

    // Load Harvard CAP reporter slug map: short_name -> slug
    let harvard_json = include_str!("../../data/harvard_reporters.json");
    let harvard_entries: Vec<serde_json::Value> =
        serde_json::from_str(harvard_json).expect("Failed to parse harvard_reporters.json");
    let harvard_slug_map: HashMap<String, String> = harvard_entries
        .into_iter()
        .filter_map(|entry| {
            let short_name = entry.get("short_name")?.as_str()?.to_string();
            let slug = entry.get("slug")?.as_str()?.to_string();
            Some((short_name, slug))
        })
        .collect();
    println!(
        "cargo::warning=Loaded {} Harvard CAP slug mappings",
        harvard_slug_map.len()
    );

    // -----------------------------------------------------------------------
    // 3. Build editions_by_regex (same logic as tokenizer.rs build_extractors)
    // -----------------------------------------------------------------------
    let mut editions_by_regex: HashMap<String, EditionsByRegexEntry> = HashMap::new();
    let mut editions_lookup: HashMap<String, Vec<BuildEdition>> = HashMap::new();

    // Helper: substitute $edition in a template with escaped edition names
    fn substitute_edition(template: &str, edition_names: &[&str]) -> String {
        let edition = edition_names
            .iter()
            .map(|e| regex::escape(e))
            .collect::<Vec<_>>()
            .join("|");
        citare_data_build::template::safe_substitute(
            template,
            &HashMap::from([("edition".to_string(), edition)]),
        )
    }

    // Helper: add a regex for an edition
    let add_regex = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                     editions_lookup: &mut HashMap<String, Vec<BuildEdition>>,
                     kind: &str,
                     reporter_strings: &[&str],
                     edition: &BuildEdition,
                     regex: &str| {
        for &reporter in reporter_strings {
            editions_lookup
                .entry(reporter.to_string())
                .or_default()
                .push(edition.clone());
        }

        let entry = editions_by_regex.entry(regex.to_string()).or_default();
        match kind {
            "editions" => entry.editions.push(edition.clone()),
            "variations" => entry.variations.push(edition.clone()),
            _ => {}
        }

        let have_strings = regex.contains(&regex::escape(reporter_strings[0]));
        if have_strings {
            for &s in reporter_strings {
                entry.strings.insert(s.to_string());
            }
        }

        // Add short cite variant
        let short_regex = short_cite_re(regex);
        if short_regex != *regex {
            let short_entry = editions_by_regex
                .entry(short_regex.clone())
                .or_default();
            match kind {
                "editions" => short_entry.editions.push(edition.clone()),
                "variations" => short_entry.variations.push(edition.clone()),
                _ => {}
            }
            short_entry.short = true;
            if have_strings {
                for &s in reporter_strings {
                    short_entry.strings.insert(s.to_string());
                }
            }
        }
    };

    // Helper: expand regex templates for an edition
    let add_regexes = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                       editions_lookup: &mut HashMap<String, Vec<BuildEdition>>,
                       regex_templates: &[String],
                       edition_name: &str,
                       edition: &BuildEdition,
                       variation_names: &[String],
                       regex_variables: &HashMap<String, String>| {
        for template in regex_templates {
            let resolved = citare_data_build::template::recursive_substitute(
                template,
                regex_variables,
                100,
            )
            .unwrap_or_else(|e| {
                eprintln!("cargo::warning=Failed to resolve template '{}': {}", template, e);
                template.clone()
            });

            // Fix Python-ism: {,N} -> {0,N}
            let resolved = resolved.replace("{,", "{0,");

            // If the template doesn't contain $edition, there's only one regex
            if !resolved.contains("$edition") && !resolved.contains("${edition}") {
                add_regex(
                    editions_by_regex,
                    editions_lookup,
                    "editions",
                    &[edition_name],
                    edition,
                    &resolved,
                );
                continue;
            }

            // Generate separate regex for the canonical edition name
            let regex = substitute_edition(&resolved, &[edition_name]);
            add_regex(
                editions_by_regex,
                editions_lookup,
                "editions",
                &[edition_name],
                edition,
                &regex,
            );

            // Generate a SEPARATE regex for EACH variation
            for var_name in variation_names {
                let var_regex = substitute_edition(&resolved, &[var_name.as_str()]);
                add_regex(
                    editions_by_regex,
                    editions_lookup,
                    "variations",
                    &[var_name.as_str()],
                    edition,
                    &var_regex,
                );
            }
        }
    };

    // --- Add reporters.json ---
    for (source_key, source_cluster) in &reporters {
        for source in source_cluster {
            for (edition_name, edition_data) in &source.editions {
                let edition = BuildEdition {
                    reporter_short_name: source_key.clone(),
                    reporter_name: source.name.clone(),
                    reporter_cite_type: source.cite_type.clone(),
                    reporter_source: 0, // Reporters
                    short_name: edition_name.clone(),
                    start: edition_data.start.clone(),
                    end: edition_data.end.clone(),
                    harvard_slug: harvard_slug_map.get(edition_name).cloned(),
                };

                let regex_templates: Vec<String> = edition_data
                    .regexes
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .cloned()
                    .unwrap_or_else(|| vec!["$full_cite".to_string()]);

                let edition_variations: Vec<String> = source
                    .variations
                    .iter()
                    .filter(|(_, v)| v.as_str() == edition_name)
                    .map(|(k, _)| k.clone())
                    .collect();

                add_regexes(
                    &mut editions_by_regex,
                    &mut editions_lookup,
                    &regex_templates,
                    edition_name,
                    &edition,
                    &edition_variations,
                    &regex_variables,
                );
            }
        }
    }

    // --- Add laws.json ---
    for (source_key, source_cluster) in &laws {
        for source in source_cluster {
            let edition = BuildEdition {
                reporter_short_name: source_key.clone(),
                reporter_name: source.name.clone(),
                reporter_cite_type: source.cite_type.clone(),
                reporter_source: 1, // Laws
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
                harvard_slug: harvard_slug_map.get(source_key).cloned(),
            };

            let regex_templates: Vec<String> = if source.regexes.is_empty() {
                vec!["$full_cite".to_string()]
            } else {
                source
                    .regexes
                    .iter()
                    .map(|r| r.replace("§ ", "§§? ?"))
                    .collect()
            };

            let variation_names: Vec<String> = source.variations.clone();

            add_regexes(
                &mut editions_by_regex,
                &mut editions_lookup,
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &regex_variables,
            );
        }
    }

    // --- Add journals.json ---
    for (source_key, source_cluster) in &journals {
        for source in source_cluster {
            let edition = BuildEdition {
                reporter_short_name: source_key.clone(),
                reporter_name: source.name.clone(),
                reporter_cite_type: source.cite_type.clone(),
                reporter_source: 2, // Journals
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
                harvard_slug: harvard_slug_map.get(source_key).cloned(),
            };

            let regex_templates: Vec<String> = if source.regexes.is_empty() {
                vec!["$full_cite".to_string()]
            } else {
                source.regexes.clone()
            };

            let variation_names: Vec<String> = source.variations.clone();

            add_regexes(
                &mut editions_by_regex,
                &mut editions_lookup,
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &regex_variables,
            );
        }
    }

    // -----------------------------------------------------------------------
    // 4. Deduplicate editions and build the flat array
    // -----------------------------------------------------------------------
    let mut all_editions: Vec<BuildEdition> = Vec::new();
    let mut edition_index_map: HashMap<BuildEdition, usize> = HashMap::new();

    fn intern_edition(
        edition: &BuildEdition,
        all_editions: &mut Vec<BuildEdition>,
        edition_index_map: &mut HashMap<BuildEdition, usize>,
    ) -> usize {
        if let Some(&idx) = edition_index_map.get(edition) {
            idx
        } else {
            let idx = all_editions.len();
            all_editions.push(edition.clone());
            edition_index_map.insert(edition.clone(), idx);
            idx
        }
    }

    // -----------------------------------------------------------------------
    // 5. Build GenExtractor list from citation extractors
    // -----------------------------------------------------------------------
    let mut gen_extractors: Vec<GenExtractor> = Vec::new();
    let mut skipped = 0usize;

    for (regex, cluster) in &editions_by_regex {
        let wrapped = nonalphanum_boundaries_re(regex);

        // Validate the regex compiles
        match regex::Regex::new(&wrapped) {
            Ok(_) => {
                let edition_indices: Vec<usize> = cluster
                    .editions
                    .iter()
                    .map(|e| intern_edition(e, &mut all_editions, &mut edition_index_map))
                    .collect();
                let variation_indices: Vec<usize> = cluster
                    .variations
                    .iter()
                    .map(|e| intern_edition(e, &mut all_editions, &mut edition_index_map))
                    .collect();

                gen_extractors.push(GenExtractor {
                    regex: wrapped,
                    token_type: TOKEN_TYPE_CITATION,
                    case_insensitive: false,
                    strings: cluster.strings.iter().cloned().collect(),
                    short: cluster.short,
                    edition_indices,
                    variation_indices,
                });
            }
            Err(e) => {
                skipped += 1;
                eprintln!(
                    "cargo::warning=Skipping invalid citation regex: {} (error: {})",
                    &wrapped[..wrapped.len().min(80)],
                    e
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // 6. Add one-off extractors
    // -----------------------------------------------------------------------
    let oneoff_specs: Vec<(String, u8, bool, Vec<String>)> = vec![
        (
            id_regex(),
            TOKEN_TYPE_ID,
            true,
            vec!["id.".to_string(), "ibid.".to_string()],
        ),
        (
            supra_regex(),
            TOKEN_TYPE_SUPRA,
            true,
            vec!["supra".to_string()],
        ),
        (
            PARAGRAPH_REGEX.to_string(),
            TOKEN_TYPE_PARAGRAPH,
            false,
            vec![],
        ),
        (
            stop_word_regex(),
            TOKEN_TYPE_STOPWORD,
            true,
            STOP_WORDS.iter().map(|s| s.to_string()).collect(),
        ),
        (
            placeholder_citations_regex(),
            TOKEN_TYPE_PLACEHOLDER,
            true,
            vec![],
        ),
        (
            SECTION_REGEX.to_string(),
            TOKEN_TYPE_SECTION,
            false,
            vec!["\u{00A7}".to_string()],
        ),
    ];

    for (regex_str, token_type, case_insensitive, strings) in oneoff_specs {
        let regex_with_flags = if case_insensitive {
            format!("(?i){}", regex_str)
        } else {
            regex_str
        };
        match regex::Regex::new(&regex_with_flags) {
            Ok(_) => {
                gen_extractors.push(GenExtractor {
                    regex: regex_with_flags,
                    token_type,
                    case_insensitive,
                    strings,
                    short: false,
                    edition_indices: vec![],
                    variation_indices: vec![],
                });
            }
            Err(e) => {
                skipped += 1;
                eprintln!(
                    "cargo::warning=Skipping invalid one-off regex: {}",
                    e
                );
            }
        }
    }

    println!(
        "cargo::warning=Generated {} extractors ({} skipped), {} unique editions",
        gen_extractors.len(),
        skipped,
        all_editions.len()
    );

    if skipped > 0 {
        panic!(
            "Build failed: {} regex patterns failed to compile. \
             This likely means reporters-db contains Python-specific regex syntax \
             that needs normalization (e.g., {{,N}} → {{0,N}}). \
             See build warnings above for details.",
            skipped
        );
    }

    // -----------------------------------------------------------------------
    // 7. Also generate the editions_lookup data
    // -----------------------------------------------------------------------
    // editions_lookup maps reporter string -> list of edition indices
    let mut gen_editions_lookup: Vec<(String, Vec<usize>)> = Vec::new();
    for (key, editions) in &editions_lookup {
        let indices: Vec<usize> = editions
            .iter()
            .map(|e| intern_edition(e, &mut all_editions, &mut edition_index_map))
            .collect();
        gen_editions_lookup.push((key.clone(), indices));
    }
    // Sort for deterministic output
    gen_editions_lookup.sort_by(|a, b| a.0.cmp(&b.0));

    // -----------------------------------------------------------------------
    // 8. Write generated Rust source file
    // -----------------------------------------------------------------------
    let mut output = String::with_capacity(1024 * 1024); // ~1MB initial

    writeln!(output, "/// Auto-generated extractor data. Do not edit.").unwrap();
    writeln!(output, "/// Generated from reporters-db and courts-db data at build time.").unwrap();
    writeln!(output).unwrap();

    // -- Edition struct and array --
    writeln!(output, "#[derive(Debug, Clone)]").unwrap();
    writeln!(output, "pub struct GeneratedEdition {{").unwrap();
    writeln!(output, "    pub reporter_short_name: &'static str,").unwrap();
    writeln!(output, "    pub reporter_name: &'static str,").unwrap();
    writeln!(output, "    pub reporter_cite_type: &'static str,").unwrap();
    writeln!(output, "    pub reporter_source: u8,").unwrap();
    writeln!(output, "    pub short_name: &'static str,").unwrap();
    writeln!(output, "    pub start: Option<&'static str>,").unwrap();
    writeln!(output, "    pub end: Option<&'static str>,").unwrap();
    writeln!(output, "    pub harvard_slug: Option<&'static str>,").unwrap();
    writeln!(output, "}}").unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "pub static GENERATED_EDITIONS: &[GeneratedEdition] = &["
    )
    .unwrap();
    for ed in &all_editions {
        writeln!(output, "    GeneratedEdition {{").unwrap();
        writeln!(
            output,
            "        reporter_short_name: {},",
            rust_str_literal(&ed.reporter_short_name)
        )
        .unwrap();
        writeln!(
            output,
            "        reporter_name: {},",
            rust_str_literal(&ed.reporter_name)
        )
        .unwrap();
        writeln!(
            output,
            "        reporter_cite_type: {},",
            rust_str_literal(&ed.reporter_cite_type)
        )
        .unwrap();
        writeln!(output, "        reporter_source: {},", ed.reporter_source).unwrap();
        writeln!(
            output,
            "        short_name: {},",
            rust_str_literal(&ed.short_name)
        )
        .unwrap();
        writeln!(
            output,
            "        start: {},",
            rust_option_str(&ed.start)
        )
        .unwrap();
        writeln!(
            output,
            "        end: {},",
            rust_option_str(&ed.end)
        )
        .unwrap();
        writeln!(
            output,
            "        harvard_slug: {},",
            rust_option_str(&ed.harvard_slug)
        )
        .unwrap();
        writeln!(output, "    }},").unwrap();
    }
    writeln!(output, "];").unwrap();
    writeln!(output).unwrap();

    // -- Extractor struct and array --
    writeln!(output, "#[derive(Debug, Clone)]").unwrap();
    writeln!(output, "pub struct GeneratedExtractor {{").unwrap();
    writeln!(output, "    pub regex: &'static str,").unwrap();
    writeln!(output, "    pub token_type: u8,").unwrap();
    writeln!(output, "    pub case_insensitive: bool,").unwrap();
    writeln!(output, "    pub strings: &'static [&'static str],").unwrap();
    writeln!(output, "    pub short: bool,").unwrap();
    writeln!(output, "    pub edition_indices: &'static [usize],").unwrap();
    writeln!(output, "    pub variation_indices: &'static [usize],").unwrap();
    writeln!(output, "}}").unwrap();
    writeln!(output).unwrap();

    writeln!(
        output,
        "pub static GENERATED_EXTRACTORS: &[GeneratedExtractor] = &["
    )
    .unwrap();
    for ext in &gen_extractors {
        writeln!(output, "    GeneratedExtractor {{").unwrap();
        writeln!(
            output,
            "        regex: {},",
            rust_str_literal(&ext.regex)
        )
        .unwrap();
        writeln!(output, "        token_type: {},", ext.token_type).unwrap();
        writeln!(
            output,
            "        case_insensitive: {},",
            ext.case_insensitive
        )
        .unwrap();
        writeln!(
            output,
            "        strings: &[{}],",
            ext.strings
                .iter()
                .map(|s| rust_str_literal(s))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        writeln!(output, "        short: {},", ext.short).unwrap();
        writeln!(
            output,
            "        edition_indices: &[{}],",
            ext.edition_indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        writeln!(
            output,
            "        variation_indices: &[{}],",
            ext.variation_indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
        writeln!(output, "    }},").unwrap();
    }
    writeln!(output, "];").unwrap();
    writeln!(output).unwrap();

    // -- Editions lookup --
    writeln!(
        output,
        "pub static GENERATED_EDITIONS_LOOKUP: &[(&str, &[usize])] = &["
    )
    .unwrap();
    for (key, indices) in &gen_editions_lookup {
        writeln!(
            output,
            "    ({}, &[{}]),",
            rust_str_literal(key),
            indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }
    writeln!(output, "];").unwrap();

    // Write to file
    let mut file = fs::File::create(&out_path).expect("Failed to create generated_extractors.rs");
    file.write_all(output.as_bytes())
        .expect("Failed to write generated_extractors.rs");

    println!(
        "cargo::warning=Wrote {} bytes to {}",
        output.len(),
        out_path.display()
    );
}

// ---------------------------------------------------------------------------
// String literal helpers
// ---------------------------------------------------------------------------

/// Produce a Rust string literal from a string, properly escaping special characters.
fn rust_str_literal(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\0' => result.push_str("\\0"),
            c if c.is_ascii_graphic() || c == ' ' => result.push(c),
            c => {
                // Use unicode escape for non-ASCII
                write!(result, "\\u{{{:04x}}}", c as u32).unwrap();
            }
        }
    }
    result.push('"');
    result
}

/// Produce a Rust `Option<&'static str>` literal.
fn rust_option_str(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("Some({})", rust_str_literal(s)),
        None => "None".to_string(),
    }
}
