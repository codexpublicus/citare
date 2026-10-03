//! The citare data bundle: every table the tokenizer needs, generated from
//! reporters-db and courts-db, in one versioned binary file.
//!
//! `citare-core` embeds a bundle by default. A build without the
//! `embedded-data` feature (the small WASM module) loads one at runtime, and
//! other tools can read the same file. The format is documented in
//! `docs/data-bundle.md`; this module is its reference encoder and decoder.
//!
//! # Layout (format version 1)
//!
//! All integers after the 12-byte header are unsigned LEB128 varints, and a
//! string is a varint byte length followed by that many UTF-8 bytes.
//!
//! ```text
//! magic          8 bytes   "CITDATA\0"
//! version        u16 LE    1
//! flags          u16 LE    0 (reserved)
//! section count  varint
//! sections       { id varint, byte length varint, payload }
//! ```
//!
//! Sections appear in id order, at most once each. A reader skips ids it
//! does not know, so later versions can add sections without breaking older
//! readers; a change to an existing section bumps the format version.

use std::borrow::Cow;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The 8 bytes every bundle starts with.
pub const MAGIC: &[u8; 8] = b"CITDATA\0";
/// The format version this module reads and writes.
pub const FORMAT_VERSION: u16 = 1;

const SEC_META: u64 = 1;
const SEC_STRINGS: u64 = 2;
const SEC_REPORTERS: u64 = 3;
const SEC_EDITIONS: u64 = 4;
const SEC_TEMPLATES: u64 = 5;
const SEC_REPORTER_MAP: u64 = 6;
const SEC_ONEOFFS: u64 = 7;
const SEC_EDITIONS_LOOKUP: u64 = 8;
const SEC_COURT_KEYS: u64 = 9;

/// A string borrowed from the bundle's bytes when decoded, or owned when
/// built.
pub type Str<'a> = Cow<'a, str>;

/// One edition of a reporter, law or journal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BundleEdition<'a> {
    pub reporter_short_name: Str<'a>,
    pub reporter_name: Str<'a>,
    pub reporter_cite_type: Str<'a>,
    /// 0 = reporters.json, 1 = laws.json, 2 = journals.json.
    pub reporter_source: u8,
    pub short_name: Str<'a>,
    pub start: Option<Str<'a>>,
    pub end: Option<Str<'a>>,
    pub harvard_slug: Option<Str<'a>>,
}

/// One citation extractor: a regex template with a reporter substituted in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BundleReporterMapEntry<'a> {
    /// Index into [`DataBundle::templates`].
    pub template_idx: u32,
    /// Regex text that replaces [`REPORTER_PLACEHOLDER`] in the template.
    pub reporter_string: Str<'a>,
    /// Strings the text must contain for this extractor to match.
    pub strings: Vec<Str<'a>>,
    /// Whether this is a short-form citation extractor.
    pub short: bool,
    /// Indices into [`DataBundle::editions`] matched exactly.
    pub edition_indices: Vec<u32>,
    /// Indices into [`DataBundle::editions`] matched through a variation.
    pub variation_indices: Vec<u32>,
}

/// A non-citation extractor (id., supra, stop words, sections, ...).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct BundleOneoff<'a> {
    pub regex: Str<'a>,
    /// 1 = id, 2 = supra, 3 = stop word, 4 = section, 5 = paragraph,
    /// 6 = placeholder.
    pub token_type: u8,
    pub case_insensitive: bool,
    pub strings: Vec<Str<'a>>,
}

/// Everything the tokenizer needs, as plain owned data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(deserialize = "'de: 'a"))]
pub struct DataBundle<'a> {
    /// Free-form provenance: `reporters_db_commit`, `courts_db_commit`.
    pub meta: Vec<(Str<'a>, Str<'a>)>,
    pub editions: Vec<BundleEdition<'a>>,
    /// Citation regex templates containing [`REPORTER_PLACEHOLDER`].
    pub templates: Vec<Str<'a>>,
    pub reporter_map: Vec<BundleReporterMapEntry<'a>>,
    pub oneoffs: Vec<BundleOneoff<'a>>,
    /// Reporter string to the editions it may name, in reporters-db order
    /// (an edition can repeat, once per regex it appears in).
    pub editions_lookup: Vec<(Str<'a>, Vec<u32>)>,
    /// `(citation_string reduced to lowercase alphanumerics, court id)`, in
    /// courts-db order.
    pub court_keys: Vec<(Str<'a>, Str<'a>)>,
}

/// The placeholder for the reporter in [`DataBundle::templates`].
pub const REPORTER_PLACEHOLDER: &str = "__REPORTER__";

/// Why a bundle could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    BadMagic,
    UnsupportedVersion(u16),
    Truncated,
    InvalidUtf8,
    BadIndex(&'static str),
    MissingSection(&'static str),
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BundleError::BadMagic => write!(f, "not a citare data bundle (bad magic)"),
            BundleError::UnsupportedVersion(v) => write!(
                f,
                "citare data bundle format version {v} is not supported (this build reads version {FORMAT_VERSION})"
            ),
            BundleError::Truncated => write!(f, "citare data bundle is truncated"),
            BundleError::InvalidUtf8 => write!(f, "citare data bundle has a string that is not UTF-8"),
            BundleError::BadIndex(what) => write!(f, "citare data bundle has an out-of-range {what} index"),
            BundleError::MissingSection(what) => write!(f, "citare data bundle has no {what} section"),
        }
    }
}

impl std::error::Error for BundleError {}

// ---------------------------------------------------------------------------
// Encoding
// ---------------------------------------------------------------------------

fn put_varint(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

fn put_str(out: &mut Vec<u8>, s: &str) {
    put_varint(out, s.len() as u64);
    out.extend_from_slice(s.as_bytes());
}

/// Interns every string once; ids are assigned in sorted order so similar
/// strings sit next to each other in the table, which compresses better.
struct Interner {
    ids: HashMap<String, u64>,
}

impl Interner {
    fn new<'a>(all: impl Iterator<Item = &'a str>) -> (Self, Vec<&'a str>) {
        let mut sorted: Vec<&str> = all.collect();
        sorted.sort_unstable();
        sorted.dedup();
        let ids = sorted
            .iter()
            .enumerate()
            .map(|(i, s)| (s.to_string(), i as u64))
            .collect();
        (Self { ids }, sorted)
    }
    fn id(&self, s: &str) -> u64 {
        self.ids[s]
    }
    /// `None` is 0, `Some(s)` is the string's id plus one.
    fn opt(&self, s: &Option<Str>) -> u64 {
        s.as_deref().map_or(0, |s| self.id(s) + 1)
    }
}

impl DataBundle<'_> {
    fn all_strings(&self) -> impl Iterator<Item = &str> {
        let editions = self.editions.iter().flat_map(|e| {
            [
                Some(e.reporter_short_name.as_ref()),
                Some(e.reporter_name.as_ref()),
                Some(e.reporter_cite_type.as_ref()),
                Some(e.short_name.as_ref()),
                e.start.as_deref(),
                e.end.as_deref(),
                e.harvard_slug.as_deref(),
            ]
            .into_iter()
            .flatten()
        });
        let map = self.reporter_map.iter().flat_map(|m| {
            std::iter::once(m.reporter_string.as_ref()).chain(m.strings.iter().map(AsRef::as_ref))
        });
        let oneoffs = self.oneoffs.iter().flat_map(|o| {
            std::iter::once(o.regex.as_ref()).chain(o.strings.iter().map(AsRef::as_ref))
        });
        editions
            .chain(self.templates.iter().map(AsRef::as_ref))
            .chain(map)
            .chain(oneoffs)
            .chain(self.editions_lookup.iter().map(|(k, _)| k.as_ref()))
            .chain(
                self.court_keys
                    .iter()
                    .flat_map(|(k, v)| [k.as_ref(), v.as_ref()]),
            )
    }

    /// Encode as a format-version-1 bundle. The output is deterministic: the
    /// same tables always give the same bytes, so a content hash of the file
    /// changes only when the data does.
    pub fn encode(&self) -> Vec<u8> {
        let (strs, table) = Interner::new(self.all_strings());
        let mut sections: Vec<(u64, Vec<u8>)> = Vec::new();

        let mut s = Vec::new();
        put_varint(&mut s, self.meta.len() as u64);
        for (k, v) in &self.meta {
            put_str(&mut s, k);
            put_str(&mut s, v);
        }
        sections.push((SEC_META, s));

        // String table: all lengths, then all bytes.
        let mut s = Vec::new();
        put_varint(&mut s, table.len() as u64);
        for t in &table {
            put_varint(&mut s, t.len() as u64);
        }
        for t in &table {
            s.extend_from_slice(t.as_bytes());
        }
        sections.push((SEC_STRINGS, s));

        // Reporters, interned out of the editions.
        type ReporterKey<'a> = (&'a str, &'a str, &'a str, u8);
        let mut reporters: Vec<ReporterKey> = Vec::new();
        let mut reporter_idx: HashMap<ReporterKey, u64> = HashMap::new();
        let edition_reporter: Vec<u64> = self
            .editions
            .iter()
            .map(|e| {
                let key = (
                    e.reporter_short_name.as_ref(),
                    e.reporter_name.as_ref(),
                    e.reporter_cite_type.as_ref(),
                    e.reporter_source,
                );
                *reporter_idx.entry(key).or_insert_with(|| {
                    reporters.push(key);
                    (reporters.len() - 1) as u64
                })
            })
            .collect();
        let mut s = Vec::new();
        put_varint(&mut s, reporters.len() as u64);
        for r in &reporters {
            put_varint(&mut s, strs.id(r.0));
        }
        for r in &reporters {
            put_varint(&mut s, strs.id(r.1));
        }
        for r in &reporters {
            put_varint(&mut s, strs.id(r.2));
        }
        for r in &reporters {
            put_varint(&mut s, r.3 as u64);
        }
        sections.push((SEC_REPORTERS, s));

        // Editions, one column at a time.
        let mut s = Vec::new();
        put_varint(&mut s, self.editions.len() as u64);
        for r in &edition_reporter {
            put_varint(&mut s, *r);
        }
        for e in &self.editions {
            put_varint(&mut s, strs.id(&e.short_name));
        }
        for e in &self.editions {
            put_varint(&mut s, strs.opt(&e.start));
        }
        for e in &self.editions {
            put_varint(&mut s, strs.opt(&e.end));
        }
        for e in &self.editions {
            put_varint(&mut s, strs.opt(&e.harvard_slug));
        }
        sections.push((SEC_EDITIONS, s));

        let mut s = Vec::new();
        put_varint(&mut s, self.templates.len() as u64);
        for t in &self.templates {
            put_varint(&mut s, strs.id(t));
        }
        sections.push((SEC_TEMPLATES, s));

        // Reporter map. template_idx is delta-coded (the map is sorted by
        // template); the index lists are count-prefixed.
        let mut s = Vec::new();
        let m = &self.reporter_map;
        put_varint(&mut s, m.len() as u64);
        let mut prev = 0u64;
        for e in m {
            let t = e.template_idx as u64;
            // Not sorted by template: fall back to absolute values, flagged
            // by the low bit.
            if t >= prev {
                put_varint(&mut s, (t - prev) << 1);
            } else {
                put_varint(&mut s, (t << 1) | 1);
            }
            prev = t;
        }
        for e in m {
            put_varint(&mut s, strs.id(&e.reporter_string));
        }
        for e in m {
            put_varint(&mut s, e.short as u64);
        }
        for e in m {
            put_varint(&mut s, e.strings.len() as u64);
            for x in &e.strings {
                put_varint(&mut s, strs.id(x));
            }
        }
        for e in m {
            put_varint(&mut s, e.edition_indices.len() as u64);
            for &i in &e.edition_indices {
                put_varint(&mut s, i as u64);
            }
        }
        for e in m {
            put_varint(&mut s, e.variation_indices.len() as u64);
            for &i in &e.variation_indices {
                put_varint(&mut s, i as u64);
            }
        }
        sections.push((SEC_REPORTER_MAP, s));

        let mut s = Vec::new();
        put_varint(&mut s, self.oneoffs.len() as u64);
        for o in &self.oneoffs {
            put_varint(&mut s, strs.id(&o.regex));
            put_varint(&mut s, o.token_type as u64);
            put_varint(&mut s, o.case_insensitive as u64);
            put_varint(&mut s, o.strings.len() as u64);
            for x in &o.strings {
                put_varint(&mut s, strs.id(x));
            }
        }
        sections.push((SEC_ONEOFFS, s));

        let mut s = Vec::new();
        put_varint(&mut s, self.editions_lookup.len() as u64);
        for (k, idx) in &self.editions_lookup {
            put_varint(&mut s, strs.id(k));
            put_varint(&mut s, idx.len() as u64);
            for &i in idx {
                put_varint(&mut s, i as u64);
            }
        }
        sections.push((SEC_EDITIONS_LOOKUP, s));

        let mut s = Vec::new();
        put_varint(&mut s, self.court_keys.len() as u64);
        for (k, v) in &self.court_keys {
            put_varint(&mut s, strs.id(k));
            put_varint(&mut s, strs.id(v));
        }
        sections.push((SEC_COURT_KEYS, s));

        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        put_varint(&mut out, sections.len() as u64);
        for (id, payload) in sections {
            put_varint(&mut out, id);
            put_varint(&mut out, payload.len() as u64);
            out.extend_from_slice(&payload);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Decoding
// ---------------------------------------------------------------------------

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn varint(&mut self) -> Result<u64, BundleError> {
        let mut v = 0u64;
        let mut shift = 0;
        loop {
            let b = *self.buf.get(self.pos).ok_or(BundleError::Truncated)?;
            self.pos += 1;
            if shift >= 64 {
                return Err(BundleError::Truncated);
            }
            v |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(v);
            }
            shift += 7;
        }
    }
    /// A count of items that each take at least one byte, so a corrupt count
    /// can't trigger a huge allocation.
    fn count(&mut self) -> Result<usize, BundleError> {
        let n = self.varint()? as usize;
        if n > self.buf.len() - self.pos {
            return Err(BundleError::Truncated);
        }
        Ok(n)
    }
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], BundleError> {
        let end = self.pos.checked_add(n).ok_or(BundleError::Truncated)?;
        let b = self.buf.get(self.pos..end).ok_or(BundleError::Truncated)?;
        self.pos = end;
        Ok(b)
    }
    fn str(&mut self) -> Result<&'a str, BundleError> {
        let n = self.varint()? as usize;
        std::str::from_utf8(self.bytes(n)?).map_err(|_| BundleError::InvalidUtf8)
    }
}

struct Strings<'a>(Vec<&'a str>);

impl<'a> Strings<'a> {
    fn get(&self, id: u64) -> Result<&'a str, BundleError> {
        self.0
            .get(id as usize)
            .copied()
            .ok_or(BundleError::BadIndex("string"))
    }
    fn string(&self, r: &mut Reader) -> Result<Str<'a>, BundleError> {
        Ok(Cow::Borrowed(self.get(r.varint()?)?))
    }
    fn opt(&self, r: &mut Reader) -> Result<Option<Str<'a>>, BundleError> {
        match r.varint()? {
            0 => Ok(None),
            id => Ok(Some(Cow::Borrowed(self.get(id - 1)?))),
        }
    }
}

fn index_list(
    r: &mut Reader<'_>,
    bound: usize,
    what: &'static str,
) -> Result<Vec<u32>, BundleError> {
    let n = r.count()?;
    (0..n)
        .map(|_| {
            let i = r.varint()?;
            if (i as usize) < bound {
                Ok(i as u32)
            } else {
                Err(BundleError::BadIndex(what))
            }
        })
        .collect()
}

/// Read the header and return `(format version, sections)` without decoding
/// the sections. Useful to inspect a bundle's metadata cheaply.
fn sections(buf: &[u8]) -> Result<Vec<(u64, &[u8])>, BundleError> {
    if buf.len() < 12 {
        return Err(BundleError::Truncated);
    }
    if &buf[..8] != MAGIC {
        return Err(BundleError::BadMagic);
    }
    let version = u16::from_le_bytes([buf[8], buf[9]]);
    if version != FORMAT_VERSION {
        return Err(BundleError::UnsupportedVersion(version));
    }
    let mut r = Reader::new(&buf[12..]);
    let n = r.count()?;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let id = r.varint()?;
        let len = r.varint()? as usize;
        out.push((id, r.bytes(len)?));
    }
    Ok(out)
}

/// The provenance metadata of a bundle, without decoding its tables.
pub fn read_meta(buf: &[u8]) -> Result<Vec<(Str<'_>, Str<'_>)>, BundleError> {
    let secs = sections(buf)?;
    let (_, payload) = secs
        .iter()
        .find(|(id, _)| *id == SEC_META)
        .ok_or(BundleError::MissingSection("meta"))?;
    decode_meta(payload)
}

fn decode_meta(payload: &[u8]) -> Result<Vec<(Str<'_>, Str<'_>)>, BundleError> {
    let mut r = Reader::new(payload);
    let n = r.count()?;
    (0..n)
        .map(|_| Ok((Cow::Borrowed(r.str()?), Cow::Borrowed(r.str()?))))
        .collect()
}

impl<'a> DataBundle<'a> {
    /// Decode a bundle written by [`DataBundle::encode`]. Strings borrow
    /// from `buf`; nothing is copied but the index columns.
    pub fn decode(buf: &'a [u8]) -> Result<Self, BundleError> {
        let secs = sections(buf)?;
        let find = |id: u64, what: &'static str| {
            secs.iter()
                .find(|(i, _)| *i == id)
                .map(|(_, p)| *p)
                .ok_or(BundleError::MissingSection(what))
        };

        let meta = decode_meta(find(SEC_META, "meta")?)?;

        let mut r = Reader::new(find(SEC_STRINGS, "strings")?);
        let n = r.count()?;
        let lens: Vec<usize> = (0..n)
            .map(|_| r.varint().map(|v| v as usize))
            .collect::<Result<_, _>>()?;
        let mut table = Vec::with_capacity(n);
        for len in lens {
            table.push(std::str::from_utf8(r.bytes(len)?).map_err(|_| BundleError::InvalidUtf8)?);
        }
        let strs = Strings(table);

        let mut r = Reader::new(find(SEC_REPORTERS, "reporters")?);
        let n = r.count()?;
        let short: Vec<&'a str> = (0..n)
            .map(|_| strs.get(r.varint()?))
            .collect::<Result<_, _>>()?;
        let name: Vec<&'a str> = (0..n)
            .map(|_| strs.get(r.varint()?))
            .collect::<Result<_, _>>()?;
        let ctype: Vec<&'a str> = (0..n)
            .map(|_| strs.get(r.varint()?))
            .collect::<Result<_, _>>()?;
        let source: Vec<u8> = (0..n)
            .map(|_| r.varint().map(|v| v as u8))
            .collect::<Result<_, _>>()?;

        let mut r = Reader::new(find(SEC_EDITIONS, "editions")?);
        let n = r.count()?;
        let rep: Vec<usize> = (0..n)
            .map(|_| {
                let i = r.varint()? as usize;
                if i < short.len() {
                    Ok(i)
                } else {
                    Err(BundleError::BadIndex("reporter"))
                }
            })
            .collect::<Result<_, _>>()?;
        let ed_short: Vec<Str<'a>> = (0..n)
            .map(|_| strs.string(&mut r))
            .collect::<Result<_, _>>()?;
        let start: Vec<Option<Str<'a>>> =
            (0..n).map(|_| strs.opt(&mut r)).collect::<Result<_, _>>()?;
        let end: Vec<Option<Str<'a>>> =
            (0..n).map(|_| strs.opt(&mut r)).collect::<Result<_, _>>()?;
        let slug: Vec<Option<Str<'a>>> =
            (0..n).map(|_| strs.opt(&mut r)).collect::<Result<_, _>>()?;
        let editions: Vec<BundleEdition<'a>> = rep
            .into_iter()
            .zip(ed_short)
            .zip(start)
            .zip(end)
            .zip(slug)
            .map(
                |((((ri, short_name), start), end), harvard_slug)| BundleEdition {
                    reporter_short_name: Cow::Borrowed(short[ri]),
                    reporter_name: Cow::Borrowed(name[ri]),
                    reporter_cite_type: Cow::Borrowed(ctype[ri]),
                    reporter_source: source[ri],
                    short_name,
                    start,
                    end,
                    harvard_slug,
                },
            )
            .collect();
        let n_ed = editions.len();

        let mut r = Reader::new(find(SEC_TEMPLATES, "templates")?);
        let n = r.count()?;
        let templates: Vec<Str<'a>> = (0..n)
            .map(|_| strs.string(&mut r))
            .collect::<Result<_, _>>()?;

        let mut r = Reader::new(find(SEC_REPORTER_MAP, "reporter map")?);
        let n = r.count()?;
        let mut prev = 0u64;
        let tidx: Vec<u32> = (0..n)
            .map(|_| {
                let v = r.varint()?;
                let t = if v & 1 == 0 { prev + (v >> 1) } else { v >> 1 };
                prev = t;
                if (t as usize) < templates.len() {
                    Ok(t as u32)
                } else {
                    Err(BundleError::BadIndex("template"))
                }
            })
            .collect::<Result<_, _>>()?;
        let rstr: Vec<Str<'a>> = (0..n)
            .map(|_| strs.string(&mut r))
            .collect::<Result<_, _>>()?;
        let shortf: Vec<bool> = (0..n)
            .map(|_| r.varint().map(|v| v != 0))
            .collect::<Result<_, _>>()?;
        let strings: Vec<Vec<Str<'a>>> = (0..n)
            .map(|_| {
                let k = r.count()?;
                (0..k).map(|_| strs.string(&mut r)).collect()
            })
            .collect::<Result<_, _>>()?;
        let eds: Vec<Vec<u32>> = (0..n)
            .map(|_| index_list(&mut r, n_ed, "edition"))
            .collect::<Result<_, _>>()?;
        let vars: Vec<Vec<u32>> = (0..n)
            .map(|_| index_list(&mut r, n_ed, "edition"))
            .collect::<Result<_, _>>()?;
        let reporter_map = tidx
            .into_iter()
            .zip(rstr)
            .zip(shortf)
            .zip(strings)
            .zip(eds)
            .zip(vars)
            .map(
                |(
                    ((((template_idx, reporter_string), short), strings), edition_indices),
                    variation_indices,
                )| {
                    BundleReporterMapEntry {
                        template_idx,
                        reporter_string,
                        strings,
                        short,
                        edition_indices,
                        variation_indices,
                    }
                },
            )
            .collect();

        let mut r = Reader::new(find(SEC_ONEOFFS, "one-off extractors")?);
        let n = r.count()?;
        let oneoffs = (0..n)
            .map(|_| {
                Ok(BundleOneoff {
                    regex: strs.string(&mut r)?,
                    token_type: r.varint()? as u8,
                    case_insensitive: r.varint()? != 0,
                    strings: {
                        let k = r.count()?;
                        (0..k)
                            .map(|_| strs.string(&mut r))
                            .collect::<Result<_, _>>()?
                    },
                })
            })
            .collect::<Result<_, BundleError>>()?;

        let mut r = Reader::new(find(SEC_EDITIONS_LOOKUP, "editions lookup")?);
        let n = r.count()?;
        let editions_lookup = (0..n)
            .map(|_| Ok((strs.string(&mut r)?, index_list(&mut r, n_ed, "edition")?)))
            .collect::<Result<_, BundleError>>()?;

        let mut r = Reader::new(find(SEC_COURT_KEYS, "court keys")?);
        let n = r.count()?;
        let court_keys = (0..n)
            .map(|_| Ok((strs.string(&mut r)?, strs.string(&mut r)?)))
            .collect::<Result<_, BundleError>>()?;

        Ok(DataBundle {
            meta,
            editions,
            templates,
            reporter_map,
            oneoffs,
            editions_lookup,
            court_keys,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> DataBundle<'static> {
        let ed = BundleEdition {
            reporter_short_name: "A.".into(),
            reporter_name: "Atlantic Reporter".into(),
            reporter_cite_type: "state_regional".into(),
            reporter_source: 0,
            short_name: "A.2d".into(),
            start: Some("1938-01-01T00:00:00".into()),
            end: None,
            harvard_slug: Some("a2d".into()),
        };
        DataBundle {
            meta: vec![("reporters_db_commit".into(), "abc".into())],
            editions: vec![
                ed.clone(),
                BundleEdition {
                    short_name: "A.3d".into(),
                    harvard_slug: None,
                    ..ed
                },
            ],
            templates: vec![format!("(?P<reporter>{REPORTER_PLACEHOLDER}) (?P<page>\\d+)").into()],
            reporter_map: vec![BundleReporterMapEntry {
                template_idx: 0,
                reporter_string: "A\\.\\s?2d".into(),
                strings: vec!["A.2d".into()],
                short: false,
                edition_indices: vec![0],
                variation_indices: vec![1],
            }],
            oneoffs: vec![BundleOneoff {
                regex: "(?i)id\\.".into(),
                token_type: 1,
                case_insensitive: true,
                strings: vec!["id.".into()],
            }],
            editions_lookup: vec![("A.2d".into(), vec![0, 0])],
            court_keys: vec![("pa".into(), "pa".into())],
        }
    }

    #[test]
    fn round_trips() {
        let b = sample();
        let bytes = b.encode();
        assert_eq!(&bytes[..8], MAGIC);
        assert_eq!(DataBundle::decode(&bytes).unwrap(), b);
        assert_eq!(read_meta(&bytes).unwrap(), b.meta);
    }

    #[test]
    fn rejects_bad_input() {
        let bytes = sample().encode();
        assert_eq!(DataBundle::decode(b"nope"), Err(BundleError::Truncated));
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert_eq!(DataBundle::decode(&bad), Err(BundleError::BadMagic));
        let mut v2 = bytes.clone();
        v2[8] = 2;
        assert_eq!(
            DataBundle::decode(&v2),
            Err(BundleError::UnsupportedVersion(2))
        );
        for cut in [13, bytes.len() / 2, bytes.len() - 1] {
            assert!(DataBundle::decode(&bytes[..cut]).is_err(), "cut at {cut}");
        }
    }
}
