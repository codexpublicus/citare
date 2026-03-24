# Citare

A Rust/WASM legal citation extraction library, inspired by [eyecite](https://github.com/freelawproject/eyecite).

Part of the [stare.pub](https://stare.pub) project family.

## Features

- Extract legal citations from text (case law, statutes, journals)
- Resolve short-form citations (Id., supra, short case) to their full citations
- Annotate source text with citation markers
- Runs natively or as WASM in browsers/Node.js

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
- Citation resolution (linking short forms to full citations)
- Text annotation with citation markers
- Text cleaning (HTML extraction, whitespace normalization)
- CLI, native Rust library, and WASM bindings
- 193 tests, 44/44 cross-validation with eyecite

**Deferred:**
- HTML-aware case name extraction (markup_text mode)
- npm package publishing

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
```

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

## Quick Start (WASM/JavaScript)

```javascript
// Note: npm package not yet published. Build from source with wasm-pack.
import init, { getCitations, resolveCitations, annotateCitations } from '@lokkju/citare';

await init();

const citations = getCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(citations);
// [{citation_type: "FullCaseCitation", matched_text: "1 U.S. 1", ...}, ...]

const resolutions = resolveCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(resolutions);
// [{resource_text: "1 U.S. 1", citations: [...]}]
```

## Data Sources

Citation patterns are sourced from:
- [reporters-db](https://github.com/freelawproject/reporters-db) — 1,235 reporters, 1,367 editions
- [courts-db](https://github.com/freelawproject/courts-db) — 2,800+ courts

## Building

```bash
# Native
cargo build --release

# WASM
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown -p citare-wasm
```

## License

[Polyform Shield 1.0.0](LICENSE.md)

## Acknowledgments

Citare is a Rust port of [eyecite](https://github.com/freelawproject/eyecite) by the [Free Law Project](https://free.law).
