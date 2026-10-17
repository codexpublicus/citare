//! The reporter, edition and court tables citare matches against.
//!
//! They are generated from reporters-db and courts-db at build time into one
//! binary data bundle (format: `docs/data-bundle.md`, codec:
//! [`citare_data_build::bundle`]). With the default `embedded-data` feature
//! the bundle is compiled into the library and loads itself on first use.
//! Without it, the library is smaller and the caller must hand it a bundle
//! with [`load`] before finding citations; the WASM module's `initData` does
//! that with a bundle fetched alongside it.

use std::sync::OnceLock;

pub use citare_data_build::bundle::{
    read_meta, BundleError, DataBundle, FORMAT_VERSION, REPORTER_PLACEHOLDER,
};

use crate::models::edition::{Edition, Reporter, ReporterSource};

/// The bundle compiled into this build.
#[cfg(feature = "embedded-data")]
pub static EMBEDDED_BUNDLE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/citare-data.bin"));

/// A decoded bundle plus the runtime form of its editions.
pub struct Data {
    pub bundle: DataBundle<'static>,
    /// `bundle.editions` as [`Edition`]s, in the same order.
    pub editions: Vec<Edition>,
}

impl Data {
    /// Decode a data bundle. Its strings borrow from `bytes`.
    pub fn from_bytes(bytes: &'static [u8]) -> Result<Self, BundleError> {
        let bundle = DataBundle::decode(bytes)?;
        let editions = bundle
            .editions
            .iter()
            .map(|e| Edition {
                reporter: Reporter::new(
                    e.reporter_short_name.to_string(),
                    e.reporter_name.to_string(),
                    e.reporter_cite_type.to_string(),
                    match e.reporter_source {
                        1 => ReporterSource::Laws,
                        2 => ReporterSource::Journals,
                        _ => ReporterSource::Reporters,
                    },
                )
                .with_jurisdiction(e.reporter_jurisdiction.as_deref().map(str::to_string)),
                short_name: e.short_name.to_string(),
                start: e.start.as_deref().map(str::to_string),
                end: e.end.as_deref().map(str::to_string),
                harvard_slug: e.harvard_slug.as_deref().map(str::to_string),
                slug: e.slug.as_deref().map(str::to_string),
            })
            .collect();
        Ok(Self { bundle, editions })
    }

    /// A provenance value from the bundle's metadata, e.g.
    /// `reporters_db_commit`.
    pub fn meta(&self, key: &str) -> Option<&str> {
        self.bundle
            .meta
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_ref())
    }
}

/// The lowercase hex SHA-256 of a bundle's bytes. Published bundles are
/// named `citare-data-<first 16 hex digits>.bin`, so a name never refers to
/// two different files and can be cached forever.
pub fn content_hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

static DATA: OnceLock<Data> = OnceLock::new();

/// Why [`load`] failed.
#[derive(Debug)]
pub enum LoadError {
    /// The bytes are not a readable bundle.
    Bundle(BundleError),
    /// Data is already loaded (or the embedded data is already in use).
    AlreadyLoaded,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Bundle(e) => e.fmt(f),
            LoadError::AlreadyLoaded => write!(f, "citare data is already loaded"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Load the data bundle every later call uses. Call it once, before the
/// first `get_citations`; a second call returns [`LoadError::AlreadyLoaded`].
/// Builds with `embedded-data` don't need it, but may use it to substitute
/// another bundle if nothing has used the data yet.
pub fn load(bytes: &[u8]) -> Result<(), LoadError> {
    if DATA.get().is_some() {
        return Err(LoadError::AlreadyLoaded);
    }
    // The data lives for the rest of the process, so keep one copy of the
    // bytes for its strings to borrow from.
    let bytes: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
    let data = Data::from_bytes(bytes).map_err(LoadError::Bundle)?;
    DATA.set(data).map_err(|_| LoadError::AlreadyLoaded)
}

/// Whether data is loaded, or embedded and so always available.
pub fn is_available() -> bool {
    cfg!(feature = "embedded-data") || DATA.get().is_some()
}

/// The loaded data.
///
/// # Panics
///
/// Without the `embedded-data` feature, if [`load`] has not been called.
pub fn get() -> &'static Data {
    DATA.get_or_init(default_data)
}

#[cfg(feature = "embedded-data")]
fn default_data() -> Data {
    Data::from_bytes(EMBEDDED_BUNDLE).expect("embedded citare data bundle is invalid")
}

#[cfg(not(feature = "embedded-data"))]
fn default_data() -> Data {
    panic!(
        "citare data is not loaded: this build has no embedded data, so call \
         citare_core::data::load with a data bundle first"
    )
}
