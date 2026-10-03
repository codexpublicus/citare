# citare data bundle

citare matches citations against tables generated from reporters-db and courts-db: the
editions of every reporter, law and journal, the regex templates for each, the strings
that select which regexes to run, and the court abbreviations. `build.rs` writes all of
them into one binary file, the data bundle. `citare-core` compiles it in by default
(the `embedded-data` feature). A build without that feature, such as the small WASM
module, loads a bundle at runtime instead, and any other tool can read the same file.

The reference encoder and decoder are in `crates/citare-data-build/src/bundle.rs`.

## Getting a bundle

- `citare data export --dir DIR` writes the bundle built into the CLI as
  `DIR/citare-data-<hash>.bin`.
- `citare data info` prints its format version, provenance and table sizes;
  `citare --data FILE data info` does the same for another bundle.
- `citare --data FILE find ...` finds citations with that bundle instead of the
  built-in one.

`<hash>` is the first 16 hex digits of the file's SHA-256. The encoder is deterministic,
so the same reporters-db and courts-db (and the same citare derivation logic) always
give the same bytes, and the name changes only when the data does. The bundle carries
no citare version for that reason.

## Loading it

Rust, with `citare-core` built with `default-features = false`:

```rust
let bytes = std::fs::read("citare-data-3415ed94af721530.bin")?;
citare_core::data::load(&bytes)?;          // once, before get_citations
let citations = citare_core::get_citations(text, false, "", &[]);
```

`get_citations` panics if no data is loaded. `citare_core::data::is_available()` says
whether it is.

JavaScript, with `citare-wasm` built with `--no-default-features`:

```js
const [_, bytes] = await Promise.all([
  init(),                                   // the WASM module
  fetch(bundleUrl).then(r => r.arrayBuffer()),
]);
initData(new Uint8Array(bytes));           // throws on a bad or unsupported bundle
getCitations(text, false);
```

`getCitations` and `resolveCitations` throw until `initData` has run. `dataLoaded()`
reports whether it has.

## Format, version 1

A bundle is a 12-byte header followed by sections. Every integer after the header is an
unsigned LEB128 varint (7 bits per byte, low bits first, high bit set on every byte but
the last). A *string* is a varint byte length followed by that many bytes of UTF-8.

```text
offset 0   magic          8 bytes  "CITDATA\0"
offset 8   version        u16 LE   1
offset 10  flags          u16 LE   0, reserved
offset 12  section count  varint
           sections       { id: varint, length: varint, payload: length bytes }
```

Sections appear in increasing id order, each at most once. A reader skips any id it
does not know. Adding a section is a compatible change and keeps the version; changing
the layout of an existing section bumps the version, and readers reject versions they
were not written for.

Most sections store tables by column: all values of the first field, then all values of
the second, and so on. Columns of similar values compress better than interleaved rows.
A *string id* is an index into the string table (section 2); an *optional string id* is
0 for none or the string id plus one.

| id | section | payload |
|---:|---|---|
| 1 | meta | count, then count pairs of (key string, value string). Keys: `reporters_db_commit`, `courts_db_commit`. Provenance only; may be `unknown`. |
| 2 | strings | count, then count byte lengths, then the strings' bytes back to back. Sorted and deduplicated; every other section refers to strings by their position here. |
| 3 | reporters | count *R*, then four columns of *R* values: short name (string id), full name (string id), cite type (string id), source (0 reporters.json, 1 laws.json, 2 journals.json). |
| 4 | editions | count *E*, then five columns of *E* values: reporter (index into section 3), edition short name (string id), start date (optional string id), end date (optional string id), Caselaw Access Project slug (optional string id). |
| 5 | templates | count *T*, then *T* string ids. Each is a citation regex with `__REPORTER__` where the reporter goes. |
| 6 | reporter map | count *M*, then six columns of *M* values: template index, delta-coded (`d << 1` for a template index `d` above the previous one, or `t << 1 \| 1` for an absolute index `t`); reporter regex (string id); short-form flag (0 or 1); prefilter strings (count, then string ids); exact editions (count, then edition indices); variation editions (count, then edition indices). |
| 7 | one-off extractors | count, then per extractor: regex (string id), token type (1 id., 2 supra, 3 stop word, 4 section, 5 paragraph, 6 placeholder), case-insensitive flag, strings (count, then string ids). |
| 8 | editions lookup | count, then per entry: reporter string (string id), editions (count, then edition indices). |
| 9 | court keys | count, then per court: citation string reduced to lowercase alphanumerics (string id), courts-db id (string id), in courts-db order. |

One reporter-map row is one citation extractor: substitute the reporter regex for
`__REPORTER__` in its template to get the regex citare runs, and run it only on text
that contains one of its prefilter strings (compared with whitespace removed). A match
is a citation to the exact editions, or, through a variant spelling, to the variation
editions.

## Publishing

The bundle changes only when reporters-db, courts-db or citare's derivation of the
tables changes, so a published copy is immutable and can be cached forever:

- attached to each citare release as `citare-data-<hash>.bin`, with its SHA-256 in the
  release notes;
- on stare.pub at `/data/citare/v1/citare-data-<hash>.bin`, served with
  `Cache-Control: public, max-age=31536000, immutable`. `v1` is the format version; a
  version-2 bundle goes under `v2/` and both stay up while clients that read either
  exist.

A client pins the exact file name that matches its WASM build, so a citare release that
leaves the data alone leaves the URL, and the browser's cached copy, alone too.
