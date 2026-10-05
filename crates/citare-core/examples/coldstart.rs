//! Times citare's one-time setup (decoding the data bundle, building the
//! extractors and the Aho-Corasick prefilter) and the first and later
//! `get_citations` calls over a set of texts.
//!
//! usage: cargo run --release --example coldstart -- <texts.json>
//!
//! `texts.json` is a JSON array of objects with a `text` field.
use std::time::Instant;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: coldstart <texts.json>");
    let ops: Vec<serde_json::Value> =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let texts: Vec<&str> = ops.iter().map(|o| o["text"].as_str().unwrap()).collect();

    let t = Instant::now();
    let data = citare_core::data::Data::from_bytes(citare_core::data::EMBEDDED_BUNDLE).unwrap();
    let t_decode = t.elapsed();
    drop(data);
    let t = Instant::now();
    citare_core::data::get();
    let t_get = t.elapsed();
    let t = Instant::now();
    let ex = citare_core::tokenizer::build_extractors();
    let t_ex = t.elapsed();
    let t = Instant::now();
    let tok = citare_core::AhocorasickTokenizer::new();
    let t_tok = t.elapsed();
    drop((ex, tok));

    let t = Instant::now();
    citare_core::get_citations(texts[0], false, "", &[]);
    let t_first = t.elapsed();
    let t = Instant::now();
    for x in &texts[1..] {
        citare_core::get_citations(x, false, "", &[]);
    }
    let t_rest = t.elapsed();
    let t = Instant::now();
    for x in &texts {
        citare_core::get_citations(x, false, "", &[]);
    }
    let t_steady = t.elapsed();
    println!(
        "bundle decode {t_decode:?} (data::get {t_get:?}), build_extractors {t_ex:?}, \
         tokenizer incl. extractors and AC {t_tok:?}, first call {t_first:?}, \
         rest of first pass {t_rest:?}, steady pass {t_steady:?}"
    );
}
