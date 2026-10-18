# citare for Python

citare's citation finder with eyecite's Python API. The functions, their keyword arguments and
the citation classes have eyecite's names and shapes, so code written against eyecite runs
against citare with the import changed:

```python
from citare import get_citations, resolve_citations, annotate_citations, clean_text

text = "Foo v. Bar, 1 U.S. 1, 2 (1999). Id. at 3. Foo, 1 U.S., at 4."
citations = get_citations(text)
citations[0]            # FullCaseCitation('1 U.S. 1', groups={...}, metadata=...)
citations[0].span()     # (12, 20): string indices, as in eyecite
resolve_citations(citations)              # {Resource(...): [all three citations]}
annotate_citations(text, [(c.span(), "<cite>", "</cite>") for c in citations])
```

What citare finds and how it resolves follow citare's rules, not eyecite's. eyecite is the API
this package copies, not the answer key: where citare deliberately finds or resolves something
else, [docs/divergences.md](../../docs/divergences.md) has the case and its evidence.

The package is the Rust crate `citare-py` (PyO3, one abi3 wheel for CPython 3.10 and later)
plus a thin Python layer, `python/citare`, that builds eyecite's classes. It isn't published to
PyPI; the classifier `Private :: Do Not Upload` makes PyPI refuse it.

## Building

From a checkout with submodules (citare-core's build reads `data/reporters-db` and
`data/courts-db`), with [uv](https://docs.astral.sh/uv/) and a Rust toolchain:

```bash
uv sync --project crates/citare-py                 # builds the extension into crates/citare-py/.venv
uv run --project crates/citare-py pytest crates/citare-py/tests
uvx maturin build --release -m crates/citare-py/Cargo.toml   # a wheel in target/wheels
```

To use it from another project, `uv add path/to/citare/crates/citare-py` or install the wheel.

## The API, against eyecite's

| eyecite | citare | Notes |
|---|---|---|
| `get_citations(plain_text, remove_ambiguous, tokenizer, markup_text, clean_steps)` | same | All five arguments. Finding runs without holding the GIL, so threads find in parallel. |
| `resolve_citations(citations, resolve_full_citation, resolve_shortcase_citation, resolve_supra_citation, resolve_reference_citation, resolve_id_citation)` | same | Callbacks take eyecite's arguments; see [Resolving](#resolving). Returns the citation objects passed in, grouped in the order their resources were first cited. |
| `annotate_citations(plain_text, annotations, source_text, unbalanced_tags, use_dmp, annotator, offset_updater)` | same | `use_dmp` is ignored (citare always uses its own diff). `offset_updater` raises `NotImplementedError`. |
| `clean_text(text, steps)` | same | Step names or callables; `citare.clean.html` and the other step functions are accepted too. |
| `eyecite.utils.dump_citations(citations, text, context_chars)` | `citare.dump_citations(..., color=True)` | citare-core's output, which differs from eyecite's (below). |
| `eyecite.models`: `FullCaseCitation`, `ShortCaseCitation`, `FullLawCitation`, `FullJournalCitation`, `SupraCitation`, `IdCitation`, `ReferenceCitation`, `UnknownCitation` and the bases `CitationBase`, `ResourceCitation`, `FullCitation`, `CaseCitation` | `citare.models`, same hierarchy | `isinstance` checks carry over. |
| `Resource`, `Document`, `Edition`, `Reporter` | `citare.models` | `Document(plain_text=, markup_text=, clean_steps=)` gives the cleaned `plain_text` that spans index. |
| `eyecite.tokenizers`: `Tokenizer`, `AhocorasickTokenizer`, `HyperscanTokenizer`, `EXTRACTORS`, `EDITIONS_LOOKUP`, `TokenExtractor`, `default_tokenizer` | `citare.tokenizers` | `HyperscanTokenizer` is an `AhocorasickTokenizer`. `EXTRACTORS` is built on first use. |

Citation attributes and methods: `groups`, `metadata` (a dataclass per class with eyecite's
fields, so `vars(citation.metadata)` works), `index`, `token`, `document`, `span_start` and the
other span overrides, `span()`, `full_span()`, `span_with_pincite()`, `matched_text()`,
`corrected_citation()`, `corrected_citation_full()`, `dump()`, and on resource citations `year`,
`exact_editions`, `variation_editions`, `all_editions`, `edition_guess`, `corrected_reporter()`,
`corrected_page()`. Supra and id citations have `formatted()`. Citations compare and hash by
eyecite's rules (citare-core implements them). citare adds `is_ambiguous()` on every citation
and `law()` (jurisdiction, code, subject, title, chapter, section) on `FullLawCitation`.

### Resolving

`resolve_full_citation` maps each full citation to a resource: any hashable value, such as a
database id. A falsy return leaves the citation unresolved. The other four callbacks default to
citare's own rules, applied to the resources `resolve_full_citation` returned, so a resolver that
maps full citations to your ids gets the short, supra, reference and id citations grouped under
those ids without reimplementing anything:

```python
groups = resolve_citations(citations, resolve_full_citation=lambda c: lookup_id(c))
```

A callback you pass gets eyecite's arguments: `(citation, resolved_full_cites)` for short case,
supra and reference citations, where `resolved_full_cites` is the list of `(full citation,
resource)` pairs so far, and `(citation, last_resolution, resolutions)` for id citations. An
exception raised in a callback (or in a clean step or annotator) stops the call and is raised
from it.

## Where it differs from eyecite

Behaviour:

- What is found and how it resolves: [docs/divergences.md](../../docs/divergences.md).
- A law citation's `corrected_reporter()` is the code with its subject (`N.Y. Gen. Bus. Law`),
  where eyecite gives reporters-db's key (`N.Y. Law`)
  ([codes cited by subject](../../docs/divergences.md#law-subject-split)).
- `dump_citations` prints citare-core's description: the span, sorted groups, metadata, year,
  corrected reporter and edition guess. Its spans are UTF-8 byte offsets, not string indices.
  `color` is citare's; eyecite always prints ANSI colors.
- Both `markup_text` and `plain_text` with `clean_steps`: eyecite raises `ValueError`; citare
  logs a warning and ignores the steps. Neither text: eyecite raises; citare finds nothing.
- Hash values are citare's own. They are stable across runs but are not eyecite's numbers, so a
  test that pins eyecite's hashes (`test_persistent_hash`) won't match.
- `Edition.start` and `end` are ISO-8601 strings; eyecite has `datetime`s.
- Citations found in separate `get_citations` calls and resolved together all start at offset 0.
  Two full citations under one case name then look like one decision cited in parallel to
  citare's resolver, so a short form after
  them resolves where eyecite would leave it ambiguous
  ([supra after parallel citations](../../docs/divergences.md#supra-after-parallel)). In one
  document this can't happen.

Gaps, which eyecite has and this package doesn't:

- Citation objects can't be built by hand (`eyecite.test_factories`, or calling
  `FullCaseCitation(token, index, ...)`). They come from `get_citations`.
- Changing a citation's attributes changes the Python object only. Resolving, comparing and
  dumping use what citare found; eyecite's `metadata.resolved_case_name_short` followed by
  `extract_reference_citations` has no equivalent.
- eyecite's internals: `eyecite.helpers` (`filter_citations`, `extract_reference_citations`,
  `get_year`, ...), `eyecite.find.extract_*`, `eyecite.utils.maybe_balance_style_tags`,
  `SpanUpdater` and a tokenizer's `tokenize()` output.
- The default short-form resolvers aren't exposed as Python functions (eyecite's
  `_resolve_shortcase_citation` and friends), so a callback can't call the default and filter
  it. Leave the callback out to get citare's rule.
- `EDITIONS_LOOKUP` supports lookups (`[]`, `in`, `get`), not iteration.
- A `TokenExtractor` built from Python has no editions, so a custom citation extractor finds
  citations without an edition guess. Copy one from `EXTRACTORS` (`copy.copy`, then set
  `regex`) to keep them, as eyecite's `test_custom_tokenizer` does.
- `HyperscanTokenizer` is citare's Aho-Corasick tokenizer under eyecite's name; there is no
  Hyperscan, and `cache_dir` is ignored.

## Tests

`tests/eyecite_port` is eyecite v2.7.8's own tests, from `upstream/eyecite/tests`, run against
citare with the import swapped: `test_find_citations` and the law, journal and tax court tables,
`test_custom_tokenizer`, `test_citation_fullspan`, the resolution tests, the model comparison and
correction tests, `test_annotate` (plain text and markup), `test_long_diff`,
`test_span_with_pincite` and the clean-text tests. Their provenance is engine tests: they show
eyecite's API works unchanged, and they aren't ground truth for what citare should find. A case
where citare differs on purpose is `xfail` with the reason. Tests that need eyecite's internals
or hand-built citations are left out, and the factory-built comparisons in `test_models.py` use
found citations instead.

`tests/test_api.py` is citare's own: spans over non-ASCII text, markup input, resolver
callbacks and their arguments, errors raised from callbacks, tokenizers and extractors, and
finding from several threads.

## Benchmarking with citation-bench

citation-bench's eyecite adapter (`adapters/eyecite/adapter.py`) runs on this package when
`eyecite`, `eyecite.models` and `eyecite.tokenizers` resolve to `citare`, `citare.models` and
`citare.tokenizers`. At the citation-bench commit citare pins, its responses are identical to the
Rust adapter's on all 278 vectors (apart from the Rust adapter's extra `cap_slug` field), so the
Python API adds no differences of its own. Speed measured that way includes Python object
overhead and isn't comparable with the Rust adapter's.
