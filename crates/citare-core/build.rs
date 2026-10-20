use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use citare_data_build::bundle::{
    BundleEdition, BundleOneoff, BundleReporterMapEntry, DataBundle, Str,
};
use citare_data_build::courts::parse_courts;
use citare_data_build::harvard::{parse_harvard, slugs_by_edition};
use citare_data_build::regexes::{
    id_regex, nonalphanum_boundaries_re, placeholder_citations_regex, relax_ws, short_cite_re,
    stop_word_regex, supra_regex, PARAGRAPH_REGEX, SECTION_REGEX, STOP_WORDS,
};
use citare_data_build::reporters::{
    apply_local_reporters, merge_local_journals, merge_local_laws, parse_journals, parse_laws,
    parse_local_laws, parse_local_reporters, parse_reporters,
};
use citare_data_build::slugs;
use citare_data_build::template::process_variables;

// ---------------------------------------------------------------------------
// Edition representation for build-time deduplication
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BuildEdition {
    reporter_short_name: String,
    /// Sorts an edition split off a shared name (`hill-sc`, `wash-va`) after
    /// the one that keeps the name's slug, so an ambiguous citation's first
    /// edition is that one (citare_data_build::slugs::split_off). Not in the
    /// bundle; it only orders the editions.
    split_off: bool,
    reporter_name: String,
    reporter_cite_type: String,
    reporter_source: u8, // 0=Reporters, 1=Laws, 2=Journals
    short_name: String,
    start: Option<String>,
    end: Option<String>,
    harvard_slug: Option<String>,
    slug: Option<String>,
    jurisdiction: Option<String>,
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
// TOKEN_TYPE_CITATION (0) is implicit — citation extractors use templates,
// not one-off extractors, so the constant is not needed here.
const TOKEN_TYPE_ID: u8 = 1;
const TOKEN_TYPE_SUPRA: u8 = 2;
const TOKEN_TYPE_STOPWORD: u8 = 3;
const TOKEN_TYPE_SECTION: u8 = 4;
const TOKEN_TYPE_PARAGRAPH: u8 = 5;
const TOKEN_TYPE_PLACEHOLDER: u8 = 6;

// (GenExtractor struct removed — replaced by template + reporter map approach)

/// The `law_section` regex variable, overriding reporters-db's
/// `\d+(?:[\-.:]\d+){,3}` (see the override in `main`). A part is digits
/// with an optional suffix of up to two lowercase letters or one capital
/// (`2912a`, `2000ee`, `314A`), or, after a hyphen, up to three letters
/// (`394-ccc`). The second branch, a section with parenthesized parts
/// (`81(a)(2)`), is reporters-db's.
const LAW_SECTION_REGEX: &str = concat!(
    r"(?P<section>",
    r"(?:\d+(?:[a-z]{1,2}|[A-Z])?",
    "(?:[\\-.:\u{2010}\u{2011}]\\d+(?:[a-z]{1,2}|[A-Z])?|[\\-\u{2010}\u{2011}][a-zA-Z]{1,3}){,3})",
    r"|(?:\d+(?:\((?:[a-zA-Z]{1}|\d{1,2})\))+))",
);

fn main() {
    println!("cargo::rerun-if-changed=../../data/");
    println!("cargo::rerun-if-changed=../../data/harvard_reporters.json");
    println!("cargo::rerun-if-changed=../../data/laws-local.json");
    println!("cargo::rerun-if-changed=../../data/cap_volumes.json");
    println!("cargo::rerun-if-changed=../../data/reporters-local.json");
    println!("cargo::rerun-if-env-changed=CITARE_DUMP_BUNDLE_JSON");

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");

    // -----------------------------------------------------------------------
    // 1. Load and process regex variables with overrides
    // -----------------------------------------------------------------------
    let regexes_json = include_str!("../../data/reporters-db/reporters_db/data/regexes.json");
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
        // citare's own override, not eyecite's: a statute section number may
        // end in a letter (`Mich. Comp. Laws § 600.2912a`, `42 U.S.C.
        // § 2000e`), have a letter part after a hyphen (`N.Y. Gen. Bus. Law
        // § 394-ccc`), and use the Unicode hyphens U+2010 and U+2011
        // (`Utah Code Ann. § 76‐9‐101`). reporters-db's `law_section` allows
        // none of these, and the citation stops at the first part (#48).
        if let Some(law) = obj.get_mut("law").and_then(|l| l.as_object_mut()) {
            law.insert(
                "section".to_string(),
                serde_json::Value::String(LAW_SECTION_REGEX.to_string()),
            );
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
    let reporters_json = include_str!("../../data/reporters-db/reporters_db/data/reporters.json");
    let mut reporters = parse_reporters(reporters_json).expect("Failed to parse reporters.json");
    // citare's own supplement: neutral citations whose volume is a year
    // (docs/reporters-local.md).
    let local_reporters = parse_local_reporters(include_str!("../../data/reporters-local.json"))
        .expect("Failed to parse data/reporters-local.json");
    apply_local_reporters(&mut reporters, &local_reporters).unwrap_or_else(|e| panic!("{e}"));
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
    let mut laws = parse_laws(laws_json).expect("Failed to parse laws.json");
    println!("cargo::warning=Loaded {} laws", laws.len());

    // citare's own supplement: statute forms reporters-db doesn't have, like
    // `General Statutes § 52-557n` and `42 Pa.C.S. § 5322` (docs/laws-local.md).
    let local_laws = parse_local_laws(include_str!("../../data/laws-local.json"))
        .expect("Failed to parse data/laws-local.json");
    merge_local_laws(&mut laws, &local_laws).unwrap_or_else(|e| panic!("{e}"));
    println!(
        "cargo::warning=Added {} local law entries",
        local_laws.laws.values().map(Vec::len).sum::<usize>()
    );

    let journals_json = include_str!("../../data/reporters-db/reporters_db/data/journals.json");
    let mut journals = parse_journals(journals_json).expect("Failed to parse journals.json");
    // citare's own supplement: secondary sources reporters-db lists as a
    // reporter or not at all, like American Jurisprudence (docs/reporters-local.md).
    merge_local_journals(&mut journals, &local_reporters).unwrap_or_else(|e| panic!("{e}"));
    println!("cargo::warning=Loaded {} journals", journals.len());

    let courts_json = include_str!("../../data/courts-db/courts_db/data/courts.json");
    let courts = parse_courts(courts_json).expect("Failed to parse courts.json");
    println!("cargo::warning=Loaded {} courts", courts.len());

    // Harvard CAP reporter slugs, keyed by reporters-db edition name (CAP
    // short names that are reporters-db variations are moved to their edition).
    let harvard_json = include_str!("../../data/harvard_reporters.json");
    let harvard_slug_map = slugs_by_edition(
        &parse_harvard(harvard_json).expect("Failed to parse harvard_reporters.json"),
        &reporters,
    );
    println!(
        "cargo::warning=Loaded {} Harvard CAP slug mappings",
        harvard_slug_map.len()
    );

    // -----------------------------------------------------------------------
    // 3. Build editions_by_regex (same logic as tokenizer.rs build_extractors)
    // -----------------------------------------------------------------------
    let mut editions_by_regex: HashMap<String, EditionsByRegexEntry> = HashMap::new();

    // Helper: substitute $edition in a template with escaped edition names,
    // relaxing inter-token whitespace so `N.Y.S.2d` also matches
    // `N. Y. S. 2d` (eyecite #305).
    fn substitute_edition(template: &str, edition_names: &[&str]) -> String {
        let edition = edition_names
            .iter()
            .map(|e| relax_ws(&regex::escape(e)))
            .collect::<Vec<_>>()
            .join("|");
        citare_data_build::template::safe_substitute(
            template,
            &HashMap::from([("edition".to_string(), edition)]),
        )
    }

    // Helper: add a regex for an edition
    let add_regex = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                     kind: &str,
                     reporter_strings: &[&str],
                     explicit_strings: &[String],
                     edition: &BuildEdition,
                     regex: &str| {
        let entry = editions_by_regex.entry(regex.to_string()).or_default();
        match kind {
            "editions" => entry.editions.push(edition.clone()),
            "variations" => entry.variations.push(edition.clone()),
            _ => {}
        }

        // The reporter is embedded with relaxed whitespace (see relax_ws), so
        // look for the relaxed form.
        let have_strings = regex.contains(&relax_ws(&regex::escape(reporter_strings[0])));
        if have_strings {
            for &s in reporter_strings {
                entry.strings.insert(s.to_string());
            }
        } else {
            // A custom regex without the name (laws-local.json's prefilter).
            entry.strings.extend(explicit_strings.iter().cloned());
        }

        // Add short cite variant
        let short_regex = short_cite_re(regex);
        if short_regex != *regex {
            let short_entry = editions_by_regex.entry(short_regex.clone()).or_default();
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
            } else {
                short_entry.strings.extend(explicit_strings.iter().cloned());
            }
        }
    };

    // Helper: expand regex templates for an edition
    let add_regexes = |editions_by_regex: &mut HashMap<String, EditionsByRegexEntry>,
                       regex_templates: &[String],
                       edition_name: &str,
                       edition: &BuildEdition,
                       variation_names: &[String],
                       explicit_strings: &[String],
                       regex_variables: &HashMap<String, String>| {
        for template in regex_templates {
            let resolved =
                citare_data_build::template::recursive_substitute(template, regex_variables, 100)
                    .unwrap_or_else(|e| {
                        eprintln!(
                            "cargo::warning=Failed to resolve template '{}': {}",
                            template, e
                        );
                        template.clone()
                    });

            // Fix Python-ism: {,N} -> {0,N}
            let resolved = resolved.replace("{,", "{0,");

            // If the template doesn't contain $edition, there's only one regex
            if !resolved.contains("$edition") && !resolved.contains("${edition}") {
                add_regex(
                    editions_by_regex,
                    "editions",
                    &[edition_name],
                    explicit_strings,
                    edition,
                    &resolved,
                );
                continue;
            }

            // Generate separate regex for the canonical edition name
            let regex = substitute_edition(&resolved, &[edition_name]);
            add_regex(
                editions_by_regex,
                "editions",
                &[edition_name],
                explicit_strings,
                edition,
                &regex,
            );

            // Generate a SEPARATE regex for EACH variation
            for var_name in variation_names {
                let var_regex = substitute_edition(&resolved, &[var_name.as_str()]);
                add_regex(
                    editions_by_regex,
                    "variations",
                    &[var_name.as_str()],
                    explicit_strings,
                    edition,
                    &var_regex,
                );
            }
        }
    };

    // CAP slug → its first and last volume, from CAP's VolumesMetadata.json.
    let cap_volumes: std::collections::BTreeMap<String, (u32, u32)> =
        serde_json::from_str(include_str!("../../data/cap_volumes.json"))
            .expect("Failed to parse data/cap_volumes.json");

    // Every case-reporter edition's slug, checked below.
    let mut edition_slugs: Vec<slugs::EditionSlug> = Vec::new();

    // --- Add reporters.json ---
    for (source_key, source_cluster) in &reporters {
        for source in source_cluster {
            for (edition_name, edition_data) in &source.editions {
                let regex_templates: Vec<String> = edition_data
                    .regexes
                    .as_ref()
                    .filter(|v| !v.is_empty())
                    .cloned()
                    .unwrap_or_else(|| vec!["$full_cite".to_string()]);

                let slug = slugs::assign(
                    edition_name,
                    &source.name,
                    harvard_slug_map.get(edition_name).map(String::as_str),
                )
                .unwrap_or_else(|| {
                    panic!("edition {edition_name:?} has no letters or digits for a slug")
                });
                let regexes: Vec<String> = regex_templates
                    .iter()
                    .filter_map(|t| {
                        citare_data_build::template::recursive_substitute(t, &regex_variables, 100)
                            .ok()
                    })
                    .collect();
                // CAP's slug for the reporter this edition is, which follows
                // its slug rather than its name (#72): none for a year-volume
                // neutral citation that shares a numbered CAP reporter's slug.
                let harvard_slug = slugs::cap_slug(&slug, &regexes, &cap_volumes);
                let year = |d: &Option<String>| d.as_deref()?.get(..4)?.parse().ok();
                edition_slugs.push(slugs::EditionSlug {
                    name: edition_name.clone(),
                    reporter: source.name.clone(),
                    slug: slug.clone(),
                    start_year: year(&edition_data.start),
                    end_year: year(&edition_data.end),
                    regexes,
                });

                let edition = BuildEdition {
                    reporter_short_name: source_key.clone(),
                    split_off: slugs::split_off(edition_name, &slug, &cap_volumes),
                    reporter_name: source.name.clone(),
                    reporter_cite_type: source.cite_type.clone(),
                    reporter_source: 0, // Reporters
                    short_name: edition_name.clone(),
                    start: edition_data.start.clone(),
                    end: edition_data.end.clone(),
                    harvard_slug,
                    slug: Some(slug),
                    jurisdiction: None,
                };

                let edition_variations: Vec<String> = source
                    .variations
                    .iter()
                    .filter(|(_, v)| v.as_str() == edition_name)
                    .map(|(k, _)| k.clone())
                    .collect();

                add_regexes(
                    &mut editions_by_regex,
                    &regex_templates,
                    edition_name,
                    &edition,
                    &edition_variations,
                    &[],
                    &regex_variables,
                );

                // reporters-local.json's variations of this edition, each
                // with its own regexes (`127 Ohio 545` for the Ohio State
                // Reports, but only above the Ohio Reports' last volume).
                for v in local_reporters.variations.iter().filter(|v| {
                    v.reporter == *source_key && v.name == source.name && v.edition == *edition_name
                }) {
                    for template in &v.regexes {
                        let resolved = citare_data_build::template::recursive_substitute(
                            template,
                            &regex_variables,
                            100,
                        )
                        .unwrap_or_else(|e| panic!("variation {:?}: {e}", v.variation))
                        .replace("{,", "{0,");
                        add_regex(
                            &mut editions_by_regex,
                            "variations",
                            &[v.variation.as_str()],
                            &[],
                            &edition,
                            &substitute_edition(&resolved, &[v.variation.as_str()]),
                        );
                    }
                }
            }
        }
    }

    // --- Add laws.json ---
    for (source_key, source_cluster) in &laws {
        for source in source_cluster {
            let edition = BuildEdition {
                reporter_short_name: source_key.clone(),
                split_off: false,
                reporter_name: source.name.clone(),
                reporter_cite_type: source.cite_type.clone(),
                reporter_source: 1, // Laws
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
                // CAP slugs are for case reporters. Looking laws up by key
                // gave the `N.Y.` statute entry New York Reports' slug.
                harvard_slug: None,
                slug: None,
                jurisdiction: Some(source.jurisdiction.clone()).filter(|j| !j.is_empty()),
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
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &source.strings,
                &regex_variables,
            );
        }
    }

    // --- Add journals.json ---
    for (source_key, source_cluster) in &journals {
        for source in source_cluster {
            let edition = BuildEdition {
                reporter_short_name: source_key.clone(),
                split_off: false,
                reporter_name: source.name.clone(),
                reporter_cite_type: source.cite_type.clone(),
                reporter_source: 2, // Journals
                short_name: source_key.clone(),
                start: source.start.clone(),
                end: source.end.clone(),
                harvard_slug: None,
                slug: None,
                jurisdiction: None,
            };

            let regex_templates: Vec<String> = if source.regexes.is_empty() {
                vec!["$full_cite".to_string()]
            } else {
                source.regexes.clone()
            };

            let variation_names: Vec<String> = source.variations.clone();

            add_regexes(
                &mut editions_by_regex,
                &regex_templates,
                source_key,
                &edition,
                &variation_names,
                &[],
                &regex_variables,
            );
        }
    }

    // Slugs are public paths in stare's index (`<slug>/<volume>`), so any
    // slug that could put two reporters' volumes in one place fails the
    // build (citare_data_build::slugs::check).
    let cap_reporters: Vec<(String, String)> =
        serde_json::from_str::<Vec<serde_json::Value>>(harvard_json)
            .expect("Failed to parse harvard_reporters.json")
            .iter()
            .filter_map(|e| {
                let name = e.get("short_name")?.as_str()?;
                let slug = e.get("slug")?.as_str()?;
                Some((name.to_string(), slug.to_string()))
            })
            .collect();
    let cap_pairs: Vec<(&str, &str)> = cap_reporters
        .iter()
        .map(|(n, s)| (n.as_str(), s.as_str()))
        .collect();
    let unmatched = slugs::unmatched_by_reporter(&edition_slugs);
    if !unmatched.is_empty() {
        panic!("Edition slugs are invalid:\n{}", unmatched.join("\n"));
    }
    if let Err(e) = slugs::check(&cap_pairs, &cap_volumes, &edition_slugs) {
        panic!("Edition slugs are invalid:\n{e}");
    }
    let cap_slug_set: HashSet<&str> = cap_reporters.iter().map(|(_, s)| s.as_str()).collect();
    println!(
        "cargo::warning={} edition slugs, {} not CAP's",
        edition_slugs.len(),
        edition_slugs
            .iter()
            .filter(|e| !cap_slug_set.contains(e.slug.as_str()))
            .count()
    );

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
    // 5. Deduplicate citation extractors into templates + reporter map
    // -----------------------------------------------------------------------
    //
    // Instead of emitting 10K+ full regex strings (one per extractor), we
    // extract the reporter-specific content from each regex, replace it with
    // a placeholder to get a *template*, and deduplicate templates.  The
    // generated output contains ~300 template strings plus a reporter map
    // that records, for each (template_id, reporter_string) pair, the
    // edition and variation indices.  At runtime `build_extractors()`
    // reconstructs the same 10K extractors by substituting reporter strings
    // back into templates.
    //
    // The `(?P<reporter>ESCAPED_NAME)` group is present in every citation
    // regex produced by the data pipeline.  We use a regex to locate it,
    // replace the inner content with a fixed placeholder, and use the
    // resulting string as the template key.

    let reporter_group_re =
        regex::Regex::new(r"\(\?P<reporter>([^)]+)\)").expect("reporter group regex");

    const REPORTER_PLACEHOLDER: &str = "__REPORTER__";

    /// A reporter map entry: one (template, reporter) combination.
    struct ReporterMapEntry {
        template_idx: usize,
        reporter_string: String,
        strings: Vec<String>,
        short: bool,
        edition_indices: Vec<usize>,
        variation_indices: Vec<usize>,
    }

    let mut templates: Vec<String> = Vec::new();
    let mut template_index_map: HashMap<String, usize> = HashMap::new();
    let mut reporter_map_entries: Vec<ReporterMapEntry> = Vec::new();
    let mut skipped = 0usize;

    for (regex, cluster) in &editions_by_regex {
        let wrapped = nonalphanum_boundaries_re(regex);

        // Validate the regex compiles
        if let Err(e) = regex::Regex::new(&wrapped) {
            skipped += 1;
            eprintln!(
                "cargo::warning=Skipping invalid citation regex: {} (error: {})",
                &wrapped[..wrapped.len().min(80)],
                e
            );
            continue;
        }

        // De-duplicate editions, dropping variations that are also exact
        // matches (eyecite #317): a custom regex without `$edition` yields
        // byte-identical canonical and variation regexes, which collapse
        // into one entry listing the same edition twice.
        let mut edition_indices: Vec<usize> = Vec::new();
        for e in &cluster.editions {
            let idx = intern_edition(e, &mut all_editions, &mut edition_index_map);
            if !edition_indices.contains(&idx) {
                edition_indices.push(idx);
            }
        }
        let mut variation_indices: Vec<usize> = Vec::new();
        for e in &cluster.variations {
            let idx = intern_edition(e, &mut all_editions, &mut edition_index_map);
            if !edition_indices.contains(&idx) && !variation_indices.contains(&idx) {
                variation_indices.push(idx);
            }
        }

        // Try to extract the reporter group and create a template
        if let Some(caps) = reporter_group_re.captures(&wrapped) {
            let reporter_content = caps.get(1).unwrap().as_str().to_string();
            let template_str = reporter_group_re
                .replace(&wrapped, &format!("(?P<reporter>{REPORTER_PLACEHOLDER})"))
                .to_string();

            let template_idx = if let Some(&idx) = template_index_map.get(&template_str) {
                idx
            } else {
                let idx = templates.len();
                templates.push(template_str.clone());
                template_index_map.insert(template_str, idx);
                idx
            };

            reporter_map_entries.push(ReporterMapEntry {
                template_idx,
                reporter_string: reporter_content,
                strings: cluster.strings.iter().cloned().collect(),
                short: cluster.short,
                edition_indices,
                variation_indices,
            });
        } else {
            // No reporter group — treat as a template with no reporter placeholder.
            // This shouldn't happen for citation extractors, but handle gracefully.
            let template_idx = if let Some(&idx) = template_index_map.get(&wrapped) {
                idx
            } else {
                let idx = templates.len();
                templates.push(wrapped.clone());
                template_index_map.insert(wrapped, idx);
                idx
            };

            reporter_map_entries.push(ReporterMapEntry {
                template_idx,
                reporter_string: String::new(),
                strings: cluster.strings.iter().cloned().collect(),
                short: cluster.short,
                edition_indices,
                variation_indices,
            });
        }
    }

    // -----------------------------------------------------------------------
    // 6. Add one-off extractors (Id, Supra, etc.)
    // -----------------------------------------------------------------------
    struct GenOneoffExtractor {
        regex: String,
        token_type: u8,
        case_insensitive: bool,
        strings: Vec<String>,
    }

    let mut oneoff_extractors: Vec<GenOneoffExtractor> = Vec::new();

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
                oneoff_extractors.push(GenOneoffExtractor {
                    regex: regex_with_flags,
                    token_type,
                    case_insensitive,
                    strings,
                });
            }
            Err(e) => {
                skipped += 1;
                eprintln!("cargo::warning=Skipping invalid one-off regex: {}", e);
            }
        }
    }

    println!(
        "cargo::warning=Generated {} templates + {} reporter map entries + {} one-off extractors ({} skipped), {} unique editions",
        templates.len(),
        reporter_map_entries.len(),
        oneoff_extractors.len(),
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

    // Sort reporter map entries for deterministic output
    reporter_map_entries.sort_by(|a, b| {
        a.template_idx
            .cmp(&b.template_idx)
            .then_with(|| a.reporter_string.cmp(&b.reporter_string))
    });

    // -----------------------------------------------------------------------
    // 8. Canonicalize and write the data bundle
    // -----------------------------------------------------------------------
    // The tables above are built from HashMaps, so template and edition
    // numbering vary from build to build. Sort them so the same reporters-db
    // and courts-db always give a byte-identical bundle (and so the same
    // content hash, which is what keeps a published bundle cached).
    let mut ed_order: Vec<usize> = (0..all_editions.len()).collect();
    ed_order.sort_by(|&a, &b| all_editions[a].cmp(&all_editions[b]));
    let mut ed_new = vec![0u32; all_editions.len()];
    for (new, &old) in ed_order.iter().enumerate() {
        ed_new[old] = new as u32;
    }
    let remap_eds = |v: &[usize]| -> Vec<u32> {
        let mut out: Vec<u32> = v.iter().map(|&i| ed_new[i]).collect();
        out.sort_unstable();
        out
    };

    let mut tmpl_order: Vec<usize> = (0..templates.len()).collect();
    tmpl_order.sort_by(|&a, &b| templates[a].cmp(&templates[b]));
    let mut tmpl_new = vec![0u32; templates.len()];
    for (new, &old) in tmpl_order.iter().enumerate() {
        tmpl_new[old] = new as u32;
    }

    let mut reporter_map: Vec<BundleReporterMapEntry> = reporter_map_entries
        .iter()
        .map(|e| {
            let mut strings = e.strings.clone();
            strings.sort();
            BundleReporterMapEntry {
                template_idx: tmpl_new[e.template_idx],
                reporter_string: e.reporter_string.clone().into(),
                strings: strings.into_iter().map(Into::into).collect(),
                short: e.short,
                edition_indices: remap_eds(&e.edition_indices),
                variation_indices: remap_eds(&e.variation_indices),
            }
        })
        .collect();
    reporter_map.sort_by(|a, b| {
        (
            a.template_idx,
            &a.reporter_string,
            a.short,
            &a.edition_indices,
            &a.variation_indices,
        )
            .cmp(&(
                b.template_idx,
                &b.reporter_string,
                b.short,
                &b.edition_indices,
                &b.variation_indices,
            ))
    });

    let court_keys: Vec<(Str, Str)> = courts
        .iter()
        .map(|court| {
            // get_court_by_paren matches a parenthetical against each court's
            // citation_string reduced to lowercase alphanumerics; precompute
            // that form so the runtime never parses courts.json.
            let key: String = court
                .citation_string
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            (key.into(), court.id.clone().into())
        })
        .collect();

    let bundle = DataBundle {
        // No citare version here: a bundle's bytes, and so its content hash
        // and cache lifetime, should change only when the data does.
        meta: vec![
            (
                "reporters_db_commit".into(),
                submodule_commit("reporters-db").into(),
            ),
            (
                "courts_db_commit".into(),
                submodule_commit("courts-db").into(),
            ),
        ],
        editions: ed_order
            .iter()
            .map(|&i| {
                let e = &all_editions[i];
                BundleEdition {
                    reporter_short_name: e.reporter_short_name.clone().into(),
                    reporter_name: e.reporter_name.clone().into(),
                    reporter_cite_type: e.reporter_cite_type.clone().into(),
                    reporter_source: e.reporter_source,
                    short_name: e.short_name.clone().into(),
                    start: e.start.clone().map(Into::into),
                    end: e.end.clone().map(Into::into),
                    harvard_slug: e.harvard_slug.clone().map(Into::into),
                    slug: e.slug.clone().map(Into::into),
                    reporter_jurisdiction: e.jurisdiction.clone().map(Into::into),
                }
            })
            .collect(),
        templates: tmpl_order
            .iter()
            .map(|&i| templates[i].clone().into())
            .collect(),
        reporter_map,
        oneoffs: oneoff_extractors
            .iter()
            .map(|o| BundleOneoff {
                regex: o.regex.clone().into(),
                token_type: o.token_type,
                case_insensitive: o.case_insensitive,
                strings: o.strings.iter().cloned().map(Into::into).collect(),
            })
            .collect(),
        court_keys,
    };
    assert_eq!(
        REPORTER_PLACEHOLDER,
        citare_data_build::bundle::REPORTER_PLACEHOLDER
    );

    let bytes = bundle.encode();
    assert_eq!(
        DataBundle::decode(&bytes).as_ref(),
        Ok(&bundle),
        "data bundle does not round-trip"
    );
    let bundle_path = Path::new(&out_dir).join("citare-data.bin");
    fs::write(&bundle_path, &bytes).expect("Failed to write citare-data.bin");
    println!(
        "cargo::warning=Wrote {} byte data bundle to {}",
        bytes.len(),
        bundle_path.display()
    );
    if std::env::var_os("CITARE_DUMP_BUNDLE_JSON").is_some() {
        let json_path = Path::new(&out_dir).join("citare-data.json");
        fs::write(&json_path, serde_json::to_vec(&bundle).unwrap()).unwrap();
        println!("cargo::warning=Wrote {}", json_path.display());
    }
}

/// The commit a data submodule is checked out at, or "unknown" outside a git
/// checkout. Recorded in the bundle's metadata for provenance only.
fn submodule_commit(name: &str) -> String {
    std::process::Command::new("git")
        .args(["-C", &format!("../../data/{name}"), "rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}
