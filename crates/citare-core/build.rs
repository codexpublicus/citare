use citare_data_build::courts::parse_courts;
use citare_data_build::reporters::{parse_journals, parse_laws, parse_reporters};
use citare_data_build::template::process_variables;

fn main() {
    println!("cargo::rerun-if-changed=../../data/");

    // Validate reporters.json
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

    // Validate laws.json
    let laws_json = include_str!("../../data/reporters-db/reporters_db/data/laws.json");
    let laws = parse_laws(laws_json).expect("Failed to parse laws.json");
    println!("cargo::warning=Loaded {} laws", laws.len());

    // Validate journals.json
    let journals_json =
        include_str!("../../data/reporters-db/reporters_db/data/journals.json");
    let journals = parse_journals(journals_json).expect("Failed to parse journals.json");
    println!("cargo::warning=Loaded {} journals", journals.len());

    // Validate courts.json
    let courts_json = include_str!("../../data/courts-db/courts_db/data/courts.json");
    let courts = parse_courts(courts_json).expect("Failed to parse courts.json");
    println!("cargo::warning=Loaded {} courts", courts.len());

    // Validate regex variables
    let regexes_json =
        include_str!("../../data/reporters-db/reporters_db/data/regexes.json");
    let raw: serde_json::Value =
        serde_json::from_str(regexes_json).expect("Failed to parse regexes.json");
    let variables = process_variables(&raw);
    println!("cargo::warning=Processed {} regex variables", variables.len());
}
