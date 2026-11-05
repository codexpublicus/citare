# Citare: Rust/WASM Legal Citation Extractor — Master Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port eyecite's legal citation extraction pipeline to Rust, compilable to WASM, for purely client-side use in stare.pub projects.

**Architecture:** A Cargo workspace with two crates: `citare-core` (the library) and `citare-wasm` (wasm-bindgen bindings). Data from `reporters-db` and `courts-db` is embedded at compile time via a build script that reads JSON, performs template variable substitution, and emits pre-compiled regex patterns as Rust source. The core pipeline mirrors eyecite's: clean → tokenize → find → resolve → annotate.

**Tech Stack:** Rust, wasm-bindgen, serde, fancy-regex (metadata regexes with lookaround + duplicate named groups), regex (tokenizer regexes), aho-corasick, scraper (HTML DOM parsing), dmp (diff-match-patch for span mapping), thiserror (error types)

**License:** Polyform Shield 1.0.0

---

## Reference: eyecite Architecture

The Python eyecite codebase consists of these modules:

| Module | Purpose | Rust equivalent |
|--------|---------|-----------------|
| `clean.py` | Text preprocessing (HTML strip, whitespace collapse) | `src/clean.rs` |
| `tokenizers.py` | Build regex extractors from reporters-db, tokenize text | `src/tokenizer.rs` + `build.rs` |
| `models.py` | Token types, citation types, Document, Resource | `src/models/` |
| `find.py` | Extract typed citations from tokens | `src/find.rs` |
| `helpers.py` | Post-citation parsing, case name extraction, filtering | `src/helpers.rs` |
| `resolve.py` | Link short/id/supra citations to full citations | `src/resolve.rs` |
| `annotate.py` | SpanUpdater + annotation insertion | `src/annotate.rs` |
| `regexes.py` | Regex constants for metadata extraction | `src/regexes.rs` |
| `utils.py` | Misc utilities (HTML balance, hashing, punctuation) | `src/utils.rs` |

### External data dependencies

| Package | Data | Size | How eyecite uses it |
|---------|------|------|---------------------|
| `reporters-db` | `reporters.json` (1,235 reporters, 1,367 editions), `laws.json` (371), `journals.json` (797), `regexes.json` (7 top-level keys → 66 variables after processing) | ~1.4MB JSON | Builds ~2,700 regex extractors at init via template substitution |
| `courts-db` | `courts.json` (2,805 courts) | ~1.9MB JSON | Simple `citation_string` lookup in `get_court_by_paren()` |

### Key Python dependencies requiring Rust equivalents

| Python | Rust equivalent | Notes |
|--------|-----------------|-------|
| `pyahocorasick` | `aho-corasick` crate | BurntSushi's — best-in-class |
| `regex` (PyPI) | `fancy-regex` crate | **Critical:** eyecite uses PyPI `regex` (not stdlib `re`) in `helpers.py` for duplicate named capture groups and lookaround. Rust `regex` crate supports neither. `fancy-regex` supports lookaround but NOT duplicate named groups — those regexes must be rewritten with unique group names per branch. See Phase 4 for details. |
| `lxml` | `scraper` crate | HTML DOM parsing + CSS selectors. `lol_html` is streaming and cannot replicate `lxml.html.fromstring()` + XPath behavior. `scraper` builds a full DOM like lxml. |
| `fast-diff-match-patch` | `dmp` crate | SpanUpdater diff algorithm. The `similar` crate returns text content in diffs, but eyecite's SpanUpdater expects `(operation, char_count)` tuples — `dmp` matches this API. |
| `reporters_db.utils.process_variables` | Custom in `build.rs` | Recursive `$variable` template substitution |
| `reporters_db.utils.recursive_substitute` | Custom in `build.rs` | `string.Template.safe_substitute` loop |

---

## File Structure

```
citare/
├── Cargo.toml                          # Workspace root
├── LICENSE.md                          # Polyform Shield 1.0.0
├── README.md
├── docs/
│   └── plans/
│       └── master-plan.md              # This file
├── upstream/
│   └── eyecite/                        # Git submodule: reference + test data
├── data/
│   ├── reporters-db/                   # Git submodule
│   └── courts-db/                      # Git submodule
├── crates/
│   ├── citare-core/
│   │   ├── Cargo.toml
│   │   ├── build.rs                    # Data compilation: JSON → Rust source
│   │   └── src/
│   │       ├── lib.rs                  # Public API: get_citations, resolve, annotate
│   │       ├── clean.rs                # Text preprocessing
│   │       ├── regexes.rs              # Regex constants (ported from regexes.py)
│   │       ├── tokenizer.rs            # Aho-Corasick tokenizer
│   │       ├── find.rs                 # Citation extraction from tokens
│   │       ├── helpers.rs              # Post-citation parsing, case name extraction
│   │       ├── resolve.rs              # Citation resolution
│   │       ├── annotate.rs             # SpanUpdater + annotation
│   │       ├── utils.rs                # Shared utilities
│   │       └── models/
│   │           ├── mod.rs
│   │           ├── token.rs            # Token, CitationToken, StopWordToken, etc.
│   │           ├── citation.rs         # CitationBase, FullCaseCitation, etc.
│   │           ├── edition.rs          # Reporter, Edition
│   │           ├── document.rs         # Document struct
│   │           └── resource.rs         # Resource for resolution
│   ├── citare-wasm/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs                  # wasm-bindgen exports
│   └── citare-data-build/
│       ├── Cargo.toml                  # Shared build-time data compilation logic
│       └── src/
│           ├── lib.rs
│           ├── template.rs             # $variable substitution (process_variables equiv)
│           ├── reporters.rs            # Parse reporters/laws/journals JSON
│           └── courts.rs               # Parse courts JSON
├── tests/
│   ├── fixtures/                       # Test data extracted from eyecite tests
│   │   ├── find_test_pairs.json        # Ported from test_FindTest.py
│   │   ├── resolve_test_pairs.json     # Ported from test_ResolveTest.py
│   │   └── opinion.txt                 # Real opinion text for integration tests
│   ├── test_find.rs                    # Citation finding tests
│   ├── test_resolve.rs                 # Resolution tests
│   ├── test_annotate.rs                # Annotation tests
│   ├── test_clean.rs                   # Cleaning tests
│   └── test_tokenize.rs               # Tokenizer tests
└── scripts/
    └── extract_test_data.py            # Extract test cases from eyecite into JSON
```

---

## Burndown Checklist

### Phase 1: Project Foundation
- [ ] 1.1 Initialize Cargo workspace
- [ ] 1.2 Create `citare-core` crate skeleton
- [ ] 1.3 Create `citare-wasm` crate skeleton
- [ ] 1.4 Create `citare-data-build` crate skeleton
- [ ] 1.5 Add git submodules (eyecite, reporters-db, courts-db)
- [ ] 1.6 Add LICENSE.md (Polyform Shield 1.0.0)
- [ ] 1.7 Add .gitignore
- [ ] 1.8 Add GitHub Actions CI (cargo test, cargo clippy, wasm-pack build)
- [ ] 1.9 Initial commit

### Phase 2: Data Layer (citare-data-build)
- [ ] 2.1 Implement `$variable` template substitution (`template.rs`)
- [ ] 2.2 Write tests for template substitution against known eyecite outputs
- [ ] 2.3 Define serde models for reporters-db JSON schema (`reporters.rs`)
- [ ] 2.4 Parse and validate `reporters.json` — test: 1,235 reporters, 1,367 editions
- [ ] 2.5 Parse and validate `laws.json` — test: 371 laws
- [ ] 2.6 Parse and validate `journals.json` — test: 797 journals
- [ ] 2.7 Parse and validate `regexes.json` — test: 7 top-level keys → 66 variables after processing (33 base + 33 optional); 68 after tokenizer overrides
- [ ] 2.8 Define serde models for courts-db JSON schema (`courts.rs`)
- [ ] 2.9 Parse and validate `courts.json` — test: 2,805 courts
- [ ] 2.10 Build script (`build.rs`): compile data into generated Rust source
- [ ] 2.11 Verify build script output: correct extractor count, regex validity

### Phase 3: Core Models
- [ ] 3.1 Define `Reporter` and `Edition` structs (`models/edition.rs`)
- [ ] 3.2 Define `Token` enum and variants (`models/token.rs`)
- [ ] 3.3 Define `CitationBase` enum with per-variant metadata (`models/citation.rs`)
- [ ] 3.4 Implement `Span`, `matched_text()`, `full_span()` on citations
- [ ] 3.5 Implement `Hash` and `Eq` for citations (matching eyecite semantics)
- [ ] 3.6 Define `Document` data struct (`models/document.rs`) — fields only, no construction logic yet. Construction depends on `clean.rs` (Phase 5), `SpanUpdater` (Phase 5b), and `placeholder_markup` (Phase 5).
- [ ] 3.7 Define `Resource` struct (`models/resource.rs`)
- [ ] 3.8 Create test helper functions (`tests/helpers.rs`) — Rust equivalent of eyecite's `test_factories.py` (builders for `case_citation()`, `law_citation()`, `id_citation()`, etc.)
- [ ] 3.9 Write unit tests for model construction, equality, hashing

### Phase 4: Regex Patterns

**CRITICAL REGEX CONCERNS:**
1. **Lookahead (`(?=`):** Used in `PIN_CITE_REGEX` (and transitively in `POST_FULL_CITATION_REGEX`, `POST_SHORT_CITATION_REGEX`, `POST_LAW_CITATION_REGEX`, `POST_JOURNAL_CITATION_REGEX`, `PRE_FULL_CITATION_REGEX`), and in the court/date section of `POST_FULL_CITATION_REGEX`. Solution: use `fancy-regex` for all metadata regexes (Phases 4.3–4.6).
2. **Duplicate named capture groups:** `helpers.py` uses PyPI `regex` (not stdlib `re`) which allows the same `(?P<name>...)` in multiple alternation branches. Affected: `SUPRA_ANTECEDENT_REGEX` (has `(?P<antecedent>...)` and `(?P<volume>...)` each in multiple branches), and `POST_FULL_CITATION_REGEX` (has `PIN_CITE_REGEX` containing `(?P<pin_cite>...)` in two branches; `MONTH_REGEX`/`YEAR_REGEX` groups appear both in lookahead and capture). **Neither Rust `regex` nor `fancy-regex` supports duplicate named groups.** These regexes must be rewritten with unique group names per branch (e.g., `(?P<antecedent_1>...)` / `(?P<antecedent_2>...)`) and merged in code.

- [ ] 4.1 Port simple regexes: `ROMAN_NUMERAL_REGEX`, `PAGE_NUMBER_REGEX`, `PUNCTUATION_REGEX` — these use standard `regex` crate (no lookaround)
- [ ] 4.2 Port token regexes: `ID_REGEX`, `SUPRA_REGEX`, `STOP_WORD_REGEX`, `SECTION_REGEX`, `PARAGRAPH_REGEX`, `PLACEHOLDER_CITATIONS` — standard `regex` crate
- [ ] 4.3 Port metadata regexes with `fancy-regex`: `PIN_CITE_REGEX` (has lookahead), `YEAR_REGEX`, `MONTH_REGEX`, `PARENTHETICAL_REGEX`
- [ ] 4.4 Port post-citation regexes with `fancy-regex`: `POST_FULL_CITATION_REGEX`, `POST_SHORT_CITATION_REGEX`, `POST_LAW_CITATION_REGEX`, `POST_JOURNAL_CITATION_REGEX`
- [ ] 4.5 Port pre-citation regexes with `fancy-regex`: `PRE_FULL_CITATION_REGEX`
- [ ] 4.6 Port antecedent regexes: `SHORT_CITE_ANTECEDENT_REGEX`, `SUPRA_ANTECEDENT_REGEX`
- [ ] 4.7 **Rewrite duplicate named group regexes:** Catalog every regex with duplicate named groups. Rewrite each branch to use unique group names. Write helper functions that extract the first non-None value across branch-specific groups. Affected regexes: `SUPRA_ANTECEDENT_REGEX`, `POST_FULL_CITATION_REGEX` (via embedded `PIN_CITE_REGEX` and `MONTH_REGEX`/`YEAR_REGEX`).
- [ ] 4.8 Audit all regexes: verify every pattern compiles with its target engine (`regex` or `fancy-regex`)
- [ ] 4.9 Write tests: each regex matches known inputs from eyecite test suite

### Phase 5: Text Cleaning, Utilities & SpanUpdater
- [ ] 5.1 Implement `all_whitespace()` — regex `[\u200b\s]+ → " "`
- [ ] 5.2 Implement `inline_whitespace()` — regex `[ \t]+ → " "`
- [ ] 5.3 Implement `underscores()` — regex `__+ → ""`
- [ ] 5.4 Implement `html()` — extract visible text from HTML using `scraper` crate (DOM parse, walk text nodes, skip `style`/`script`/`head`/`link`/`page-number` parents and elements with class `star-pagination`, join with spaces)
- [ ] 5.5 Implement `xml()` — strip XML declaration
- [ ] 5.6 Implement `clean_text()` pipeline (named steps + custom callables via `Box<dyn Fn>`)
- [ ] 5.7 Implement utilities from `utils.py`: `strip_punct()`, `is_balanced_html()` (use `scraper` or simple tag-balance check), `wrap_html_tags()`, `placeholder_markup()`, `is_valid_name()`, `DISALLOWED_NAMES` list (108 entries)
- [ ] 5.8 Implement `SpanUpdater` using `dmp` crate — needed by Phase 7 (markup_text mode) and Phase 8. Port the `get_diff_steps()` → offset/updater array algorithm from `annotate.py`.
- [ ] 5.9 Implement `Document` construction logic (builder pattern): handle 4 modes (plain only, markup only, both, neither), call `clean_text()`, create `SpanUpdater` instances, call `identify_emphasis_tags()`
- [ ] 5.10 Write tests matching eyecite's test_UtilsTest clean_text cases
- [ ] 5.11 Write SpanUpdater tests: offset mapping for known before/after text pairs

### Phase 6: Tokenizer
- [ ] 6.1 Implement `TokenExtractor` struct (regex + constructor + strings)
- [ ] 6.2 Generate extractor list from build script data
- [ ] 6.3 Implement base `Tokenizer::tokenize()` — sort matches, resolve overlaps, split text
- [ ] 6.4 Implement `AhocorasickTokenizer` — filter extractors by string presence
- [ ] 6.5 Implement `Tokenizer::append_text()` — split on spaces, preserve whitespace tokens
- [ ] 6.6 Implement token merging (CitationToken edition dedup)
- [ ] 6.7 Implement nominative reporter overlap resolution — port `NOMINATIVE_REPORTER_NAMES` hardcoded set (Thompson, Cooke, Holmes, Olcott, Chase, Gilmer, Bee, Deady, Taney) and `token_is_from_nominative_reporter()` logic
- [ ] 6.8 Write tests: tokenize known citation strings, verify token types and spans
- [ ] 6.9 Benchmark: compare tokenizer performance to Python eyecite

### Phase 7: Citation Finding

**Note:** Phase 7 implements the full pipeline including HTML-aware paths, since SpanUpdater is now available from Phase 5.8. `get_citations()` calls `find_case_name_in_html()` when `markup_text` is provided, which requires `Document.plain_to_markup` (a SpanUpdater).

- [ ] 7.1 Implement `_extract_full_citation()` — dispatch to FullCase/Law/Journal
- [ ] 7.2 Implement `_extract_shortform_citation()`
- [ ] 7.3 Implement `_extract_supra_citation()`
- [ ] 7.4 Implement `_extract_id_citation()`
- [ ] 7.5 Implement `match_on_tokens()` helper
- [ ] 7.6 Implement `add_post_citation()` — parse pin_cite, court, year, parenthetical
- [ ] 7.7 Implement `add_pre_citation()` — backward scan for antecedent
- [ ] 7.8 Implement `find_case_name()` — backward scan for plaintiff/defendant (plain text)
- [ ] 7.9 Implement `add_law_metadata()`, `add_journal_metadata()`
- [ ] 7.10 Implement `extract_pin_cite()`
- [ ] 7.11 Implement `get_court_by_paren()` — court string lookup
- [ ] 7.12 Implement `filter_citations()` — overlap resolution, sorting
- [ ] 7.13 Implement `disambiguate_reporters()`
- [ ] 7.14 Implement `get_citations()` — main entry point
- [ ] 7.15 Extract test data: run `scripts/extract_test_data.py` on eyecite's test_FindTest
- [ ] 7.16 Write parameterized tests from extracted test data
- [ ] 7.17 Achieve parity with eyecite test_FindTest (500+ cases)

### Phase 8: HTML-Aware Case Name Extraction

**Note:** `SpanUpdater` and `Document` construction (including `identify_emphasis_tags()`) are implemented in Phase 5.8–5.9. This phase focuses on the HTML-specific case name extraction and reference citation logic that builds on that foundation.

- [ ] 8.1 Implement `find_html_tags_at_position()` — locate emphasis tags containing a given character position using `plain_to_markup` SpanUpdater
- [ ] 8.2 Implement `convert_html_to_plain_text_and_loc()` — map emphasis tag boundaries to plain text positions
- [ ] 8.3 Implement `find_case_name_in_html()` — extract plaintiff/defendant from styled text
- [ ] 8.4 Implement `find_reference_citations_from_markup()`
- [ ] 8.5 Implement `extract_pincited_reference_citations()`
- [ ] 8.6 Write tests for HTML-aware extraction

### Phase 9: Citation Resolution
- [ ] 9.1 Implement `resolve_full_citation()` — default Resource creation
- [ ] 9.2 Implement `_resolve_shortcase_citation()` — match by reporter+volume
- [ ] 9.3 Implement `_resolve_supra_citation()` — match by antecedent
- [ ] 9.4 Implement `_resolve_id_citation()` — resolve to last resolution
- [ ] 9.5 Implement `_resolve_reference_citation()` — match by plaintiff/defendant
- [ ] 9.6 Implement `_has_invalid_pin_cite()` — page range validation
- [ ] 9.7 Implement `_filter_by_matching_antecedent()`
- [ ] 9.8 Implement `resolve_citations()` — main resolution loop
- [ ] 9.9 Port test data from test_ResolveTest.py
- [ ] 9.10 Write parameterized resolution tests

### Phase 10: Annotation

**Note:** `SpanUpdater` is implemented in Phase 5.8. This phase implements the annotation pipeline that uses it.

- [ ] 10.1 Implement `annotate_citations()` — insert before/after markers
- [ ] 10.3 Implement unbalanced HTML handling (skip, wrap modes)
- [ ] 10.4 Implement `maybe_balance_style_tags()`
- [ ] 10.5 Port test data from test_AnnotateTest.py
- [ ] 10.6 Write annotation tests

### Phase 11: Public API & Integration
- [ ] 11.1 Wire up `lib.rs` public API: `get_citations()`, `resolve_citations()`, `annotate_citations()`
- [ ] 11.2 Integration test: full pipeline on `tests/fixtures/opinion.txt`
- [ ] 11.3 Cross-validation script: run same inputs through Python eyecite and Rust citare, diff outputs
- [ ] 11.4 Performance benchmarks (criterion)

### Phase 12: WASM Bindings
- [ ] 12.1 Define JS-friendly types with `#[wasm_bindgen]`
- [ ] 12.2 Expose `getCitations(text)` → `JsValue` (array of citation objects)
- [ ] 12.3 Expose `resolveCitations(citations)` → `JsValue`
- [ ] 12.4 Expose `annotateCitations(text, annotations)` → `String`
- [ ] 12.5 Set up wasm-pack build
- [ ] 12.6 Write JS integration tests (wasm-bindgen-test or Node.js)
- [ ] 12.7 npm package setup (`@starepub/citare`)
- [ ] 12.8 Bundle size optimization (wasm-opt, tree shaking)

### Phase 13: CI/CD & Release
- [ ] 13.1 GitHub Actions: test matrix (native + wasm)
- [ ] 13.2 GitHub Actions: publish to crates.io on tag
- [ ] 13.3 GitHub Actions: publish to npm on tag
- [ ] 13.4 README with usage examples (Rust, JS/WASM, Node.js)
- [ ] 13.5 Upstream sync documentation (how to pull new reporters-db/courts-db)

---

## Phase Details

### Phase 1: Project Foundation

#### Task 1.1: Initialize Cargo workspace

**Files:**
- Create: `Cargo.toml`

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
[workspace]
resolver = "2"
members = [
    "crates/citare-core",
    "crates/citare-wasm",
    "crates/citare-data-build",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "Polyform-Shield-1.0.0"
repository = "https://github.com/starepub/citare"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
fancy-regex = "0.14"
aho-corasick = "1"
```

- [ ] **Step 2: Commit**

```bash
git add Cargo.toml
git commit -m "feat: initialize Cargo workspace"
```

#### Task 1.2: Create citare-core crate skeleton

**Files:**
- Create: `crates/citare-core/Cargo.toml`
- Create: `crates/citare-core/src/lib.rs`
- Create: `crates/citare-core/build.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "citare-core"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Legal citation extraction library — Rust port of eyecite"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
regex = { workspace = true }
fancy-regex = "0.14"      # metadata regexes (lookaround + rewritten duplicate groups)
aho-corasick = { workspace = true }
log = "0.4"
dmp = "0.2"               # diff-match-patch for SpanUpdater (matches eyecite's API)
scraper = "0.22"          # HTML DOM parsing (replaces lxml)
sha2 = "0.10"             # deterministic hashing
thiserror = "2"           # error types

[build-dependencies]
citare-data-build = { path = "../citare-data-build" }

[dev-dependencies]
rstest = "0.25"
pretty_assertions = "1"
criterion = { version = "0.5", features = ["html_reports"] }
```

- [ ] **Step 2: Create lib.rs with module stubs**

```rust
//! Citare: Legal citation extraction library
//!
//! Inspired by eyecite (https://github.com/freelawproject/eyecite)

pub mod annotate;
pub mod clean;
pub mod find;
pub mod helpers;
pub mod models;
pub mod regexes;
pub mod resolve;
pub mod tokenizer;
pub mod utils;
```

- [ ] **Step 3: Create stub modules**

Create empty files for each module declared in `lib.rs`.

- [ ] **Step 4: Create build.rs stub**

```rust
fn main() {
    // Phase 2 will populate this with data compilation
    println!("cargo::rerun-if-changed=../../data/");
}
```

- [ ] **Step 5: Verify workspace compiles**

Run: `cargo check` from workspace root
Expected: compiles with no errors (just empty modules)

- [ ] **Step 6: Commit**

```bash
git add crates/citare-core/
git commit -m "feat: add citare-core crate skeleton"
```

#### Task 1.3: Create citare-wasm crate skeleton

**Files:**
- Create: `crates/citare-wasm/Cargo.toml`
- Create: `crates/citare-wasm/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "citare-wasm"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "WASM bindings for citare legal citation extraction"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
citare-core = { path = "../citare-core" }
wasm-bindgen = "0.2"
serde-wasm-bindgen = "0.6"
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

- [ ] **Step 2: Create lib.rs stub**

```rust
use wasm_bindgen::prelude::*;

/// Get citations from text.
/// Full implementation in Phase 12.
#[wasm_bindgen(js_name = "getCitations")]
pub fn get_citations(_text: &str) -> JsValue {
    JsValue::NULL
}
```

- [ ] **Step 3: Commit**

```bash
git add crates/citare-wasm/
git commit -m "feat: add citare-wasm crate skeleton"
```

#### Task 1.4: Create citare-data-build crate skeleton

**Files:**
- Create: `crates/citare-data-build/Cargo.toml`
- Create: `crates/citare-data-build/src/lib.rs`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "citare-data-build"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "Build-time data compilation for citare"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
regex = { workspace = true }
```

- [ ] **Step 2: Create lib.rs with module stubs**

```rust
//! Build-time data compilation.
//!
//! Reads reporters-db and courts-db JSON, performs template variable
//! substitution, and generates Rust source for compiled extractors.

pub mod courts;
pub mod reporters;
pub mod template;
```

- [ ] **Step 3: Create stub modules**

- [ ] **Step 4: Commit**

```bash
git add crates/citare-data-build/
git commit -m "feat: add citare-data-build crate skeleton"
```

#### Task 1.5: Add git submodules

- [ ] **Step 1: Add eyecite as reference submodule**

```bash
git submodule add https://github.com/freelawproject/eyecite.git upstream/eyecite
```

- [ ] **Step 2: Add reporters-db**

```bash
git submodule add https://github.com/freelawproject/reporters-db.git data/reporters-db
```

- [ ] **Step 3: Add courts-db**

```bash
git submodule add https://github.com/freelawproject/courts-db.git data/courts-db
```

- [ ] **Step 4: Commit**

```bash
git add .gitmodules upstream/ data/
git commit -m "feat: add upstream submodules (eyecite, reporters-db, courts-db)"
```

#### Task 1.6–1.9: Boilerplate

- [ ] **Step 1: Add LICENSE.md** (Polyform Shield 1.0.0)
- [ ] **Step 2: Add .gitignore** (Rust standard + `target/`, `pkg/`, `node_modules/`)
- [ ] **Step 3: Add basic README.md**
- [ ] **Step 4: Add GitHub Actions CI** (`.github/workflows/ci.yml`)
- [ ] **Step 5: Commit**

---

### Phase 2: Data Layer (citare-data-build)

This is the most critical and novel phase. eyecite's `_populate_reporter_extractors()` runs at import time, processing ~3MB of JSON into ~2,700 compiled regex extractors. In Rust, we do this at **build time**.

#### Task 2.1: Template Variable Substitution

**Files:**
- Create: `crates/citare-data-build/src/template.rs`
- Test: `crates/citare-data-build/src/template.rs` (inline tests)

The template system must replicate Python's `string.Template.safe_substitute` behavior:
- `$variable` or `${variable}` → replaced with value
- Unknown variables left as-is (safe substitute)
- Recursive: substitute until no more `$` references remain

- [ ] **Step 1: Write failing test for basic substitution**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_substitute_basic() {
        let vars = HashMap::from([
            ("foo".to_string(), "bar".to_string()),
        ]);
        assert_eq!(safe_substitute("$foo baz", &vars), "bar baz");
    }

    #[test]
    fn test_safe_substitute_unknown_left_alone() {
        let vars = HashMap::new();
        assert_eq!(safe_substitute("$unknown", &vars), "$unknown");
    }
}
```

- [ ] **Step 2: Implement `safe_substitute()`**

```rust
use std::collections::HashMap;

/// Perform Python-style `string.Template.safe_substitute`:
/// Replace `$name` or `${name}` with values from `vars`.
/// Unknown variables are left as-is.
pub fn safe_substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let re = regex::Regex::new(r"\$\{?([a-zA-Z_][a-zA-Z0-9_]*)\}?").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let name = &caps[1];
        match vars.get(name) {
            Some(val) => val.clone(),
            None => caps[0].to_string(),
        }
    }).to_string()
}
```

- [ ] **Step 3: Write failing test for recursive substitution**

```rust
#[test]
fn test_recursive_substitute() {
    let vars = HashMap::from([
        ("a".to_string(), "$b".to_string()),
        ("b".to_string(), "resolved".to_string()),
    ]);
    assert_eq!(recursive_substitute("$a", &vars, 100), "resolved");
}
```

- [ ] **Step 4: Implement `recursive_substitute()`**

```rust
/// Recursively substitute until stable or max_depth reached.
pub fn recursive_substitute(
    template: &str,
    vars: &HashMap<String, String>,
    max_depth: usize,
) -> String {
    let mut result = template.to_string();
    for _ in 0..max_depth {
        let next = safe_substitute(&result, vars);
        if next == result {
            break;
        }
        result = next;
    }
    result
}
```

- [ ] **Step 5: Write failing test for `process_variables()`**

This must replicate the Python function that:
1. Flattens nested dicts (joining keys with `_`)
2. Strips `#` comment keys
3. Creates `_optional` variants (wrapping in `(?:... ?)?`)
4. Recursively resolves all `$variable` references

```rust
#[test]
fn test_process_variables_flattening() {
    // Input: {"volume": {"": "\\d+", "year": "\\d{4}"}}
    // Output: {"volume": "\\d+", "volume_year": "\\d{4}",
    //          "volume_optional": "(?:\\d+ ?)?", ...}
    let raw = serde_json::json!({
        "volume": {"": "(?P<volume>[1-9]\\d*)", "year": "(?P<volume>\\d{4})"}
    });
    let result = process_variables(&raw);
    assert_eq!(result.get("volume").unwrap(), "(?P<volume>[1-9]\\d*)");
    assert_eq!(result.get("volume_year").unwrap(), "(?P<volume>\\d{4})");
    assert!(result.get("volume_optional").unwrap().starts_with("(?:"));
}
```

- [ ] **Step 6: Implement `process_variables()`**
- [ ] **Step 7: Validate against eyecite**: load actual `regexes.json`, process, compare output variable count (should be 66: 33 base + 33 optional variants from 7 top-level keys). After tokenizer overrides (adding custom `full_cite` and `page`), expect 68.
- [ ] **Step 8: Commit**

#### Task 2.3–2.7: Parse reporters-db data

**Files:**
- Create/modify: `crates/citare-data-build/src/reporters.rs`

- [ ] **Step 1: Define serde structs for reporters.json schema**

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ReporterCluster {
    pub name: String,
    pub cite_type: String,
    pub editions: HashMap<String, EditionData>,
    pub variations: HashMap<String, String>,
    #[serde(default)]
    pub mlz_jurisdiction: Vec<String>,
    pub cite_format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EditionData {
    pub start: Option<String>,  // ISO datetime or null
    pub end: Option<String>,
    pub regexes: Option<Vec<String>>,
}
```

- [ ] **Step 2: Write test that parses `reporters.json` and counts entries**

```rust
#[test]
fn test_parse_reporters_json() {
    let data = include_str!("../../../data/reporters-db/reporters_db/data/reporters.json");
    let reporters: HashMap<String, Vec<ReporterCluster>> = serde_json::from_str(data).unwrap();
    assert_eq!(reporters.len(), 1235);
    let total_editions: usize = reporters.values()
        .flat_map(|clusters| clusters.iter())
        .map(|c| c.editions.len())
        .sum();
    assert!(total_editions >= 1300); // ~1367
}
```

- [ ] **Step 3: Run test — expect FAIL (struct doesn't parse yet)**
- [ ] **Step 4: Fix serde structs until test passes**
- [ ] **Step 5: Repeat for laws.json** (371 entries, different schema — has `regexes`, `jurisdiction`, `variations` as Vec)
- [ ] **Step 6: Repeat for journals.json** (797 entries)
- [ ] **Step 7: Commit**

#### Task 2.8–2.9: Parse courts-db data

- [ ] **Step 1: Define serde structs for courts.json**

```rust
#[derive(Debug, Deserialize)]
pub struct Court {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub name_abbreviation: Option<String>,
    pub citation_string: String,
    #[serde(rename = "type", default)]
    pub court_type: String,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub level: String,
    pub jurisdiction: String,
    pub location: String,
    #[serde(default)]
    pub locations: i32,
    #[serde(default)]
    pub dates: Vec<CourtDateRange>,
    #[serde(default)]
    pub regex: Vec<String>,
    #[serde(default)]
    pub sub_names: Vec<String>,
    #[serde(default)]
    pub case_types: Vec<String>,
    #[serde(default)]
    pub examples: Vec<String>,
    pub parent: Option<String>,
    // Note: eyecite only uses `id` and `citation_string` fields
    // (in get_court_by_paren). Other fields included for completeness.
}

#[derive(Debug, Deserialize)]
pub struct CourtDateRange {
    pub start: Option<String>,
    pub end: Option<String>,
}
```

**Note on courts-db processing:** The raw `courts.json` contains `$variable` placeholders and ordinal range patterns (`${1-56}`). The Python `courts_db` package resolves these at import time. The Rust build script must replicate this:
1. Load `variables.json` (87 regex variables for state names, court types)
2. Load all 28 `places/*.txt` files, join lines with `|`, wrap in `(...)` as regex alternation
3. Expand ordinal ranges: `${1-56}` → `(First|Second|...|Fifty-sixth)` regex alternation
4. Apply `string.Template.substitute()` to resolve all `$variable` references
5. Parse the resolved JSON

Alternatively, we could vendor a pre-processed snapshot. Decision: replicate the processing in `build.rs` for maintainability with upstream updates.

Note: courts-db uses template variables in its JSON that get resolved at load time. The Rust build script must:
1. Load `variables.json` (87 regex variables)
2. Load all `places/*.txt` files and join as regex alternations
3. Expand ordinal ranges like `${1-56}` into alternations
4. Substitute all `$variable` references in `courts.json`
5. Parse the resolved JSON

- [ ] **Step 2: Write test — parse courts.json, expect 2,805 entries**
- [ ] **Step 3: Implement courts template resolution**
- [ ] **Step 4: Commit**

#### Task 2.10–2.11: Build script

**Files:**
- Modify: `crates/citare-core/build.rs`

The build script calls into `citare-data-build` to:
1. Load and process all data
2. For each reporter edition + its variations, expand regex templates and substitute `$edition`
3. Generate a Rust source file with a `static EXTRACTORS: &[ExtractorData]` containing all compiled regex patterns and their associated edition/reporter metadata

- [ ] **Step 1: Implement build.rs that generates extractor data**
- [ ] **Step 2: Write integration test that loads generated data and counts extractors**
- [ ] **Step 3: Validate regex compilability — every generated regex must compile with Rust `regex` crate**
- [ ] **Step 4: Commit**

---

### Phase 3: Core Models

#### Task 3.1–3.2: Reporter, Edition, Token types

**Files:**
- Create: `crates/citare-core/src/models/edition.rs`
- Create: `crates/citare-core/src/models/token.rs`

- [ ] **Step 1: Define Reporter and Edition**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Reporter {
    pub short_name: String,
    pub name: String,
    pub cite_type: String,
    pub source: ReporterSource,  // Reporters, Laws, Journals
    pub is_scotus: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edition {
    pub reporter: Reporter,
    pub short_name: String,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}
```

- [ ] **Step 2: Define Token enum**

```rust
#[derive(Debug, Clone)]
pub enum Token {
    Plain(String),              // Regular text word
    Citation(CitationTokenData),
    Id(TokenSpan),
    Supra(TokenSpan),
    StopWord(StopWordData),
    Section(TokenSpan),
    Paragraph(TokenSpan),
    Placeholder(TokenSpan),
    CaseReference(TokenSpan),
}

#[derive(Debug, Clone)]
pub struct TokenSpan {
    pub data: String,
    pub start: usize,
    pub end: usize,
    pub groups: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone)]
pub struct CitationTokenData {
    pub span: TokenSpan,
    pub exact_editions: Vec<Edition>,
    pub variation_editions: Vec<Edition>,
    pub short: bool,
}
```

- [ ] **Step 3: Write tests for token construction and Display**
- [ ] **Step 4: Commit**

#### Task 3.3–3.7: Citation types

**Files:**
- Create: `crates/citare-core/src/models/citation.rs`
- Create: `crates/citare-core/src/models/document.rs`
- Create: `crates/citare-core/src/models/resource.rs`

- [ ] **Step 1: Define Citation enum**

```rust
#[derive(Debug, Clone)]
pub enum Citation {
    FullCase(FullCaseCitationData),
    FullLaw(FullLawCitationData),
    FullJournal(FullJournalCitationData),
    ShortCase(ShortCaseCitationData),
    Supra(SupraCitationData),
    Id(IdCitationData),
    Reference(ReferenceCitationData),
    Unknown(UnknownCitationData),
}

/// Common fields across all citation types
#[derive(Debug, Clone)]
pub struct CitationCommon {
    pub token_index: usize,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
    pub full_span_start: Option<usize>,
    pub full_span_end: Option<usize>,
    pub groups: HashMap<String, Option<String>>,
}
```

Each variant wraps `CitationCommon` plus type-specific metadata:

```rust
#[derive(Debug, Clone, Default)]
pub struct FullCaseMetadata {
    pub plaintiff: Option<String>,
    pub defendant: Option<String>,
    pub court: Option<String>,
    pub year: Option<String>,
    pub month: Option<String>,
    pub day: Option<String>,
    pub pin_cite: Option<String>,
    pub parenthetical: Option<String>,
    pub extra: Option<String>,
    pub antecedent_guess: Option<String>,
    // ... etc
}
```

- [ ] **Step 2: Implement `span()`, `full_span()`, `matched_text()` via trait or impl on Citation**
- [ ] **Step 3: Implement `Hash` and `Eq` matching eyecite semantics**
  - `CaseCitation`: hash by volume + corrected_reporter + page (unless page is None → unique)
  - `IdCitation`, `UnknownCitation`: always unique
  - `ResourceCitation`: hash by groups + all_editions
- [ ] **Step 4: Write tests for equality and hashing**
- [ ] **Step 5: Define Document struct**
- [ ] **Step 6: Define Resource struct**
- [ ] **Step 7: Commit**

---

### Phase 4: Regex Patterns

**Files:**
- Create: `crates/citare-core/src/regexes.rs`

Port all regex constants from eyecite's `regexes.py`. There are **two** compatibility concerns:

**Concern 1: Lookahead (`(?=`)** — Used in `PIN_CITE_REGEX` and the court/date section of `POST_FULL_CITATION_REGEX`. Since `PIN_CITE_REGEX` is embedded in 5 other regexes, this affects most metadata regexes. Solution: use `fancy-regex` for all metadata regexes (tasks 4.3–4.6).

**Concern 2: Duplicate named capture groups** — eyecite's `helpers.py` imports `regex` (PyPI), not `re` (stdlib). The PyPI `regex` module allows the same `(?P<name>...)` to appear in multiple alternation branches. **Neither Rust `regex` nor `fancy-regex` supports this.** Affected regexes:
- `SUPRA_ANTECEDENT_REGEX`: `(?P<antecedent>...)` appears in 2 branches, `(?P<volume>...)` in 2 branches
- `POST_FULL_CITATION_REGEX`: `(?P<pin_cite>...)` appears in 2 branches (via embedded `PIN_CITE_REGEX`); `(?P<month>...)` and `(?P<year>...)` appear in both lookahead and capture positions

**Rewrite strategy:** Rename duplicates to `(?P<antecedent_a>...)` / `(?P<antecedent_b>...)` etc., then write a helper function `fn merge_branch_groups(captures: &Captures) -> HashMap<String, String>` that finds the first non-None match for each logical group name.

**Two regex engines:** Use standard `regex` crate for tokenizer regexes (performance-critical, no lookaround needed). Use `fancy-regex` for metadata regexes (lookaround required, not in hot path).

- [ ] **Step 1–2: Port simple regexes** (standard `regex` crate) — see burndown 4.1–4.2
- [ ] **Step 3–6: Port metadata regexes** (`fancy-regex`) — see burndown 4.3–4.6
- [ ] **Step 7: Rewrite duplicate named group regexes** — see burndown 4.7. Write `merge_branch_groups()` helper.
- [ ] **Step 8: Audit — verify every regex compiles with its target engine**
- [ ] **Step 9: Write validation tests**
- [ ] **Step 10: Commit**

---

### Phase 5–10: Implementation

Phases 5–10 follow the same TDD pattern established above. Each implements one stage of the pipeline, with tests ported from eyecite's test suite.

**Key implementation notes per phase:**

**Phase 5 (Clean + Utils + SpanUpdater):** Direct regex replacements for whitespace/underscores. HTML extraction via `scraper` crate — parse DOM with `Html::parse_document()`, walk all text nodes, skip nodes whose ancestors are `style`, `script`, `head`, `link`, `page-number`, or have class `star-pagination`. Also implements `SpanUpdater` (using `dmp` crate), `Document` construction, and utility functions from `utils.py` (`strip_punct`, `is_balanced_html`, `placeholder_markup`, `is_valid_name`, `DISALLOWED_NAMES`).

**Phase 6 (Tokenizer):** The Aho-Corasick automaton is built once at startup from the generated extractor data. The `aho-corasick` crate's `AhoCorasick::builder()` with `MatchKind::Standard` maps directly. The critical `append_text()` function can use Rust's zero-copy `&str` slicing instead of Python's string copies — major perf win.

**Phase 7 (Find):** Largest phase. The `match_on_tokens()` helper is the workhorse — it scans forward/backward from a token, building a string to match against metadata regexes (which use `fancy-regex`). In Rust, use `String::with_capacity()` to pre-allocate the 300-char buffer. The `match_on_tokens()` results use `fancy_regex::Captures` and must go through `merge_branch_groups()` for regexes with duplicate named groups. HTML-aware code paths are available since SpanUpdater is in Phase 5.

**Phase 8 (HTML case names):** Implements the HTML-specific helpers that use SpanUpdater (from Phase 5.8) to map between markup and plain text positions. These are called by `FullCaseCitation.add_metadata()` and `_extract_shortform_citation()` when `markup_text` is present.

**Phase 9 (Resolve):** Straightforward port. The resolver accepts closures for each resolution step, matching eyecite's callback pattern. In Rust, use trait objects or generics. Also port `strip_punct()` (used for antecedent matching — already in Phase 5.7).

**Phase 10 (Annotate):** Implements `annotate_citations()` which uses SpanUpdater (from Phase 5.8) to map annotation spans from clean text to source text. Also implements unbalanced HTML handling modes and `maybe_balance_style_tags()`.

**Error handling:** Use `thiserror` for a `CitareError` enum with variants for: `InvalidInput`, `RegexCompilation`, `HtmlParsing`, `InvalidState`. Public API functions return `Result<T, CitareError>`. Internal functions use `Result` or `Option` as appropriate.

**Deliberate omissions:** `HyperscanTokenizer` is not ported (hyperscan is a C library, not WASM-compatible). The `joke_cite` easter egg is not ported.

---

### Phase 11–13: Integration, WASM, Release

**Phase 11:** Wire up the public API. Run the full pipeline on `opinion.txt` (86KB real court opinion from eyecite's test assets). Cross-validation: a Python script that runs both eyecite and citare on the same inputs and compares citation spans, types, and metadata.

**Phase 12 (WASM):** The JS API should be simple:

```typescript
interface Citation {
  type: "full_case" | "short_case" | "supra" | "id" | "reference" | "law" | "journal" | "unknown";
  matched_text: string;
  span: [number, number];
  groups: Record<string, string | null>;
  metadata: Record<string, string | null>;
  year?: number;
}

function getCitations(text: string, options?: { removeAmbiguous?: boolean }): Citation[];
function resolveCitations(citations: Citation[]): Map<string, Citation[]>;
function annotateCitations(text: string, annotations: [span, before, after][]): string;
```

Bundle size target: < 3MB gzipped (regex data for ~2,700 patterns is the main cost). Measure early in Phase 12.1 and adjust optimization strategy if needed. The `fancy-regex` dependency adds size; consider whether metadata regexes can be rewritten to avoid lookaround post-MVP.

**Phase 13:** Standard CI/CD. The sync strategy for upstream data changes:

```bash
# Update reporters-db
git -C data/reporters-db pull origin main
# Update courts-db
git -C data/courts-db pull origin main
# Rebuild — build.rs re-processes data automatically
cargo build
cargo test
```

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Regex: duplicate named capture groups | HIGH | HIGH | Rewrite with unique branch names + `merge_branch_groups()` helper. Affects `SUPRA_ANTECEDENT_REGEX`, `POST_FULL_CITATION_REGEX`. Phase 4.7 is dedicated to this. |
| Regex: lookahead in PIN_CITE_REGEX | HIGH | MEDIUM | Use `fancy-regex` for all metadata regexes (5 affected). Standard `regex` for tokenizer (hot path). |
| WASM bundle size > 3MB | MEDIUM | MEDIUM | Pre-compile regex into DFA tables; use wasm-opt -Os. Measure early in Phase 12. |
| reporters-db template edge cases | MEDIUM | HIGH | Validate generated regex count matches Python's (~2,700) |
| HTML extraction parity (scraper vs lxml) | MEDIUM | MEDIUM | `scraper` is DOM-based like lxml but uses CSS selectors not XPath. The eyecite `html()` XPath is simple enough to rewrite as tree-walking. Cross-validation (Phase 11.3) catches divergences. |
| Court template resolution complexity | MEDIUM | MEDIUM | Ordinal expansion (`${1-56}`) is custom but bounded. Consider vendoring pre-processed data as fallback. |
| `fancy-regex` performance in WASM | LOW | MEDIUM | Metadata regexes are NOT in the hot path (called once per citation, not per token). Profile in Phase 11.4 to confirm. |

## Definition of Done

Citare v0.1.0 is complete when:
1. All ported eyecite tests pass (500+ find tests, 9 resolve tests, 4 annotate tests)
2. Cross-validation against Python eyecite shows < 1% divergence on opinion.txt
3. WASM build works and JS API tests pass
4. npm and crates.io packages are publishable
5. README documents usage for Rust, JS, and Node.js consumers
