# Citare

A Rust/WASM legal citation extraction library, inspired by [eyecite](https://github.com/freelawproject/eyecite).

Part of the [stare.pub](https://stare.pub) project family.

## Features

- Extract legal citations from text (case law, statutes, journals)
- Statutes as state courts cite them (`General Statutes § 52-557n`, `42 Pa.C.S. § 5322`,
  `Penal Law § 160.05`), each with its jurisdiction, code and section in fields of their
  own ([docs/laws-local.md](docs/laws-local.md))
- Resolve short-form citations (Id., supra, short case) to their full citations, or to
  your own resources (database ids, URLs) through pluggable resolvers
- Annotate source text with citation markers
- Harvard CAP slug mapping — resolve citations to [Caselaw Access Project](https://case.law) data
- Runs natively, as WASM in browsers/Node.js, or from Python with eyecite's API

## Citation Types

| Type | Example |
|------|---------|
| Full Case | `Foo v. Bar, 1 U.S. 1 (1999)` |
| Short Case | `Foo, 1 U.S., at 2` |
| Supra | `Foo, supra, at 2` |
| Id. | `Id. at 2` |
| Law | `Mass. Gen. Laws ch. 1, § 2` |
| Journal | `1 Minn. L. Rev. 1` |

## Status

**Implemented:**
- Full citation finding pipeline (case, law, journal, short, supra, id, reference)
- Citation resolution (linking short forms to full citations), with pluggable
  resolvers (Rust `Resolver`, WASM `resolvers`)
- eyecite's API shape: `get_citations_with` takes eyecite's `get_citations`
  arguments including a custom tokenizer, `resolve_citations_with` its resolver
  callbacks, and `dump_citations` its debug output (see
  [eyecite's API in citare](#eyecites-api-in-citare))
- Text annotation with citation markers
- Text cleaning (HTML extraction, whitespace normalization)
- HTML-aware case name and reference extraction (`markup_text` input; CLI
  `--html`, WASM `{ format: "html" }`)
- Span offsets in UTF-8 bytes, code points or UTF-16 units (CLI `--offsets`,
  WASM `chars`/`utf16`; see [Span offsets](#span-offsets))
- Harvard CAP slug mapping (366/399 reporters matched)
- CLI, native Rust library, WASM bindings, and a Python package with eyecite's
  API ([crates/citare-py](crates/citare-py/README.md))
- Matches eyecite 2.7.8 on citation-bench: 202/202 find vectors, 269/278 on
  the v2 set; 278 tests
- Deliberate differences from eyecite, each with its evidence from real
  opinions, tests and an upstream issue: [docs/divergences.md](docs/divergences.md)

**Deferred:**
- npm package publishing
- PyPI publishing (the Python package builds from a checkout)

Release notes are in [CHANGELOG.md](CHANGELOG.md).

## CLI

```bash
# Extract citations as JSON
echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare find

# Resolve citations to resources
echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare resolve

# Annotate text with <cite> markers
echo "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2." | citare annotate

# Inline text
citare find --text "1 U.S. 1"

# Span offsets as UTF-16 units (JavaScript) or code points (eyecite/Python)
citare find --offsets utf16 --text "“Roe” 410 U.S. 113"
citare find --offsets chars --text "“Roe” 410 U.S. 113"

# HTML input: case names and references come from <i>/<em> tags, and each
# citation also gets a `markup` object with spans into the HTML
citare find --html --offsets utf16 < opinion.html
citare annotate --html --unbalanced-tags skip < opinion.html

# Clean steps (html, inline_whitespace, all_whitespace, underscores, xml);
# spans then index the cleaned text, which `clean` prints
citare find --clean-steps all_whitespace,underscores < opinion.txt
citare clean --html --clean-steps all_whitespace < opinion.html

# Debug output: each citation's line, spans, groups and metadata
citare dump --text "Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3."

# Run every extractor, without the Aho-Corasick prefilter (slow; a check on it)
citare find --tokenizer simple < opinion.txt
```

The CLI resolves with citare's built-in rules; resolving to your own resources is a
library feature (Rust and WASM, below).

## Quick Start (Rust)

```rust
use citare_core::{get_citations, resolve_citations, annotate_citations, UnbalancedTags};

let text = "Foo v. Bar, 1 U.S. 1 (1999). Id. at 2.";

// Find citations
let citations = get_citations(text, false, "", &[]);

// Resolve short forms to their full citations
let resolutions = resolve_citations(&citations);

// Annotate text
let mut annotations: Vec<_> = citations.iter()
    .enumerate()
    .map(|(i, c)| {
        let (start, end) = c.span();
        ((start, end), format!("<cite id=\"{}\">", i), "</cite>".to_string())
    })
    .collect();
let annotated = annotate_citations(text, &mut annotations, "", UnbalancedTags::Unchecked, None, None);
```

### eyecite's API in citare

| eyecite | citare-core |
|---|---|
| `get_citations(plain_text, remove_ambiguous, tokenizer, markup_text, clean_steps)` | `get_citations_with(plain_text, &FindOptions)`; `get_citations` without a tokenizer |
| `Tokenizer(extractors)`, `AhocorasickTokenizer(extractors)` | `Tokenizer::from_extractors`, `AhocorasickTokenizer::from_extractors`, both `Tokenize` |
| `EXTRACTORS`, `TokenExtractor(...)`, `copy(extractor)` | `build_extractors()`, `TokenExtractor::new`, `clone()` |
| `resolve_citations(citations, resolve_full_citation=..., ...)` | `resolve_citations_with(&citations, &mut resolver)` with a `Resolver` |
| `clean_text(text, steps)` | `clean_text(text, &steps)`, `CleanStep::Custom` for a callable |
| `annotate_citations(...)` | `annotate_citations(...)` |
| `dump_citations(citations, text, context_chars)` | `dump_citations(&citations, text, context_chars, color)` |

The shape follows eyecite so code written against it carries over; what citare finds and
how it resolves follow citare's own rules, which differ from eyecite's where
[docs/divergences.md](docs/divergences.md) says so.

```rust
use citare_core::{get_citations_with, resolve_citations_with, FindOptions, Citation, Resolver};
use citare_core::tokenizer::build_extractors;
use citare_core::Tokenizer;

// eyecite's test_custom_tokenizer: reporters with commas for periods.
let extractors = build_extractors()
    .iter()
    .map(|e| {
        let mut e = e.clone();
        e.regex = e.regex.replace(r"\.", "[.,]");
        e
    })
    .collect();
let tokenizer = Tokenizer::from_extractors(extractors);
let citations = get_citations_with("1 U,S, 1", &FindOptions::new().tokenizer(&tokenizer));

// Resolve to your own ids. Only full citations need a rule: short, supra,
// reference and id citations follow citare's rules over the ids you return,
// unless you override resolve_shortcase, resolve_supra and the rest.
struct ById;
impl Resolver for ById {
    type Resource = u64;
    fn resolve_full(&mut self, citation: &Citation) -> Option<u64> {
        lookup(citation) // None leaves it unresolved
    }
}
let groups = resolve_citations_with(&citations, &mut ById);
```

## Quick Start (WASM/JavaScript)

```javascript
// Note: npm package not yet published. Build from source with wasm-pack.
import init, { getCitations, resolveCitations, annotateCitations, cleanText } from '@lokkju/citare';

await init();

const citations = getCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(citations);
// [{citation_type: "FullCaseCitation", matched_text: "1 U.S. 1", ...}, ...]

const resolutions = resolveCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(resolutions);
// [{resource_text: "1 U.S. 1", citations: [...]}]

// Your own resources: each callback returns an id, null (unresolved) or
// undefined (citare's rule). Short forms follow the ids `full` returns.
const byId = resolveCitations(text, false, undefined, {
  full: (citation) => index.lookup(citation.slug, citation.groups),
});
// [{resource: "case-7", resource_text: "1 U.S. 1", citations: [...]}]

// HTML input. `options` is optional: { format: "plain" | "html", cleanSteps: [...] }
const html = "<p>In <i>Roe v. Wade</i>, 410 U.S. 113 (1973). Later, in <i>Roe</i>, ...</p>";
const cites = getCitations(html, false, { format: "html" });
// utf16/chars/span index the cleaned text; `markup` holds the same spans in the HTML.
const plain = cleanText(html, ["html"]);
plain.slice(...cites[0].utf16.span);        // "410 U.S. 113"
html.slice(...cites[0].markup.utf16.span);  // "410 U.S. 113"
```

## Quick Start (Python)

```python
# Not on PyPI. Build from a checkout: uv sync --project crates/citare-py
from citare import get_citations, resolve_citations

citations = get_citations("Foo v. Bar, 1 U.S. 1 (1999). Id. at 2.")
resolve_citations(citations, resolve_full_citation=lambda c: lookup_id(c))
```

The package has eyecite's functions, arguments and citation classes, so code written
against eyecite runs with `import citare` in its place; behaviour is citare's. Its
[README](crates/citare-py/README.md) maps the API onto eyecite's and lists the gaps.

## Span offsets

citare's spans are UTF-8 byte offsets, so `&text[start..end]` is free in
Rust. Other languages index strings differently, and the numbers diverge as
soon as the text contains a curly quote, `§` or an accented name:

| Unit | Who counts this way | Where citare emits it |
|------|---------------------|-----------------------|
| UTF-8 bytes | Rust | `span()`, `full_span()`, `pin_cite_span()`; WASM `span`/`full_span`; CLI default |
| Code points | Python, eyecite, citation-bench | `OffsetMap::to_chars`; WASM `chars`; CLI `--offsets chars` |
| UTF-16 units | JavaScript `String.prototype.slice` | `OffsetMap::to_utf16`; WASM `utf16`; CLI `--offsets utf16` |

```rust
use citare_core::{get_citations, OffsetMap, OffsetUnit};

let text = "“Roe” v. Wade, 410 U.S. 113 (1973)";
let map = OffsetMap::new(text); // one pass; reuse for every span
for c in get_citations(text, false, "", &[]) {
    let spans = map.citation_spans(&c, OffsetUnit::Chars).unwrap();
    println!("{:?} {:?}", spans.span, spans.full_span);
}
```

In JavaScript, `text.slice(...c.utf16.span)` returns the citation; store
`c.chars.span` when offsets must match a code point protocol. Pass the byte
`span` back to `annotateCitations`.

## Harvard CAP Integration

Citations are automatically mapped to [Harvard Caselaw Access Project](https://case.law) reporter slugs, enabling direct resolution to case text:

```
"31 A.2d 647" → reporter=A.2d, slug=a2d, volume=31, page=647
                 → https://static.case.law/a2d/31.zip
```

396 of CAP's 401 reporters are matched to an edition. The `harvard_slug` field appears in CLI, WASM, and Rust API output.

Every case-reporter edition also has a `slug`, next to `harvard_slug` in the same outputs. It is the CAP slug where CAP has the reporter, and otherwise one derived from the reporters-db edition name, in the same namespace: `2021 NY Slip Op 01234` → `ny-slip-op`, `10 F.4th 100` → `f-4th`. Reporters share a slug only when their volumes can't overlap, so `2019-Ohio-1234` and `20 Ohio 1` are both `ohio` (volume 2019 and volume 20), while the two `Ct. Cl.` reporters are `us-ct-cl` and `wv-ct-cl`. Where CAP splits one series by volume, citare does too: `10 Mass. App. Div. 12` is `mass-app-div` and `1995 Mass. App. Div. 12` is `mass-app-div-annual`. An edition's `harvard_slug` is its `slug` when that is the CAP reporter it is, and null otherwise: `2019-Ohio-1234` shares `ohio` with CAP's Ohio Reports but has no `harvard_slug`, since CAP has none of its year volumes. A citation citare can't narrow to one edition (`172 Wash. 688`, with no year) takes the slugs of the reporter that keeps the name's plain slug (`wash`), not of one split off it (`wash-va`). Neutral citations whose volume is a year share the slug of the CAP reporter with the same letters (`2021 ND 123` and `50 N.D. 1` are both `nd`); [`data/reporters-local.json`](data/reporters-local.json) limits the ones reporters-db leaves open to a year (docs/reporters-local.md). The build fails on any slug that could put two reporters' volumes in one place (`citare_data_build::slugs`, with CAP's volume ranges in [`data/cap_volumes.json`](data/cap_volumes.json)). Slugs are public paths in stare's index and never change once released: [`data/edition-slugs.json`](data/edition-slugs.json) lists every edition name's slugs, and a test fails if one changes or disappears. Laws and journals have no slug.

## Data Sources

Citation patterns are sourced from:
- [reporters-db](https://github.com/freelawproject/reporters-db) — 1,236 reporters, 1,368 editions
- [courts-db](https://github.com/freelawproject/courts-db) — 2,800+ courts
- [Harvard CAP](https://case.law) — 399 reporter slug mappings

## Building

```bash
# Native
cargo build --release

# WASM
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p citare-wasm

# Python (a wheel in target/wheels; uv sync builds it into a venv instead)
uvx maturin build --release -m crates/citare-py/Cargo.toml
```

## Conformance (citation-bench)

CI's `Parity (citation-bench)` job runs the
[citation-bench](https://github.com/codexpublicus/citation-bench) conformance vectors
against the PR's checkout. It fails on any vector that isn't listed in citation-bench's
`baselines/citare.known-failures`, and the job summary lists listed vectors that now pass,
so they can come off the list. The same run locally, from a citation-bench checkout:

```bash
CITARE_PATH=../citare mise run bench:citare
```

citation-bench is pinned by commit (`.github/citation-bench-ref`), so new vectors, a
rebaselined known-failures file or a changed report reach citare only through a PR here.
To bump it, run the command above in citation-bench at the new commit, then put that
commit's full SHA in the file in its own PR. The job checks citation-bench out with the
`CITATION_BENCH_DEPLOY_KEY` secret that codexpublicus/infra provisions, and skips with a
notice while that secret is missing.

Where citare differs from eyecite on purpose ([docs/divergences.md](docs/divergences.md)),
the expected output is citare's, in citation-bench's separate `vectors/divergences/` set:

```bash
CITARE_PATH=../citare mise run bench:divergences citare
```

### Benchmark reports

After the ratchet, the same job runs citare over the one-percent CourtListener sample
(779 opinions) and writes a report: vector scores per request kind, speed (ms/vector,
ms/KB, median and slowest opinion, cold start), and the citations gained, lost and
changed against eyecite 2.7.8 and against the last release. eyecite's side is
citation-bench's stored run, keyed by eyecite's version and the harness files, so eyecite
only runs here when that key misses. The report goes to the job summary, the
`bench-report` artifact (with the CSVs and the saved runs) and one PR comment that later
pushes edit in place. It never fails the build; the ratchet does.

Each release gets the same report on the ten-percent sample (7,632 opinions):
`release-plz.yml` calls `bench-release.yml` once it cuts the tag, which attaches
`bench-report.md`, `bench-summary.json` and the saved runs to the GitHub release and
adds the headline numbers to its notes. PR reports compare against the latest release's
attached one-percent run. To backfill a release, or rerun one:

```bash
gh workflow run bench-release.yml -f tag=v0.4.1                # attach to the release
gh workflow run bench-release.yml -f tag=v0.4.1 -f publish=false  # report only
```

## CI runners

Every workflow job runs on the org's self-hosted `homelab` runners (`runs-on: homelab`,
from lokkju/infra's GARM setup): one ephemeral pod per job, on an actions-runner image
with gh, git, curl, jq, build-essential and pkg-config and no passwordless sudo.
`.github/actions/homelab-setup` checks for the C toolchain and pkg-config the Rust jobs
need and fails, naming what's missing, if the image lacks them and sudo doesn't work;
rustup, mise and uv come from their own actions.
Benchmark speeds depend on the machine, so each report names the runner it ran on.
eyecite's stored runs in citation-bench were timed on GitHub-hosted runners, so ms/KB
comparisons with eyecite cross machine types until those runs are re-baselined on homelab.

## License

[Polyform Shield 1.0.0](LICENSE.md)

## Acknowledgments

Citare is a Rust port of [eyecite](https://github.com/freelawproject/eyecite) by the [Free Law Project](https://free.law).
