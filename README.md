# Citare

A Rust/WASM legal citation extraction library, inspired by [eyecite](https://github.com/freelawproject/eyecite).

Part of the [stare.pub](https://stare.pub) project family.

## Features

- Extract legal citations from text (case law, statutes, journals)
- Resolve short-form citations (Id., supra, short case) to their full citations
- Annotate source text with citation markers
- Harvard CAP slug mapping — resolve citations to [Caselaw Access Project](https://case.law) data
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
- HTML-aware case name and reference extraction (`markup_text` input; CLI
  `--html`, WASM `{ format: "html" }`)
- Span offsets in UTF-8 bytes, code points or UTF-16 units (CLI `--offsets`,
  WASM `chars`/`utf16`; see [Span offsets](#span-offsets))
- Harvard CAP slug mapping (366/399 reporters matched)
- CLI, native Rust library, and WASM bindings
- Matches eyecite 2.7.8 on citation-bench: 202/202 find vectors, 268/278 on
  the v2 set; 278 tests

**Deferred:**
- npm package publishing

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
import init, { getCitations, resolveCitations, annotateCitations, cleanText } from '@lokkju/citare';

await init();

const citations = getCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(citations);
// [{citation_type: "FullCaseCitation", matched_text: "1 U.S. 1", ...}, ...]

const resolutions = resolveCitations("1 U.S. 1 (1999). Id. at 2.", false);
console.log(resolutions);
// [{resource_text: "1 U.S. 1", citations: [...]}]

// HTML input. `options` is optional: { format: "plain" | "html", cleanSteps: [...] }
const html = "<p>In <i>Roe v. Wade</i>, 410 U.S. 113 (1973). Later, in <i>Roe</i>, ...</p>";
const cites = getCitations(html, false, { format: "html" });
// utf16/chars/span index the cleaned text; `markup` holds the same spans in the HTML.
const plain = cleanText(html, ["html"]);
plain.slice(...cites[0].utf16.span);        // "410 U.S. 113"
html.slice(...cites[0].markup.utf16.span);  // "410 U.S. 113"
```

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

366 of 399 Harvard reporters are matched. The `harvard_slug` field appears in CLI, WASM, and Rust API output.

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
```

## License

[Polyform Shield 1.0.0](LICENSE.md)

## Acknowledgments

Citare is a Rust port of [eyecite](https://github.com/freelawproject/eyecite) by the [Free Law Project](https://free.law).
