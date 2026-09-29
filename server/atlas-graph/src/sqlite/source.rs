//! The section source seam and its committed-blob implementation: what turns a manifest entry into
//! an openable `.sqlite` file.

use std::path::{Path, PathBuf};

use super::blob::decompress_verified;
use super::manifest::ManifestSection;
use super::SqliteError;

pub type SectionError = SqliteError;

/// Resolve a section by its LOGICAL hash to an openable SQLite file. An implementation verifies
/// the transport hash before it returns a path.
pub trait SectionSource {
    fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionLayout {
    /// `data/compiled`: `manifest.toml` and `sections/`.
    pub compiled_dir: PathBuf,
    /// `data/cache/sections`: `<logical>.<schema_version>.sqlite`, gitignored.
    pub cache_dir: PathBuf,
}

impl SectionLayout {
    /// The documented layout under a `data/compiled` directory: the cache is its sibling
    /// `data/cache/sections`.
    pub fn under(data_dir: &Path) -> SectionLayout {
        let parent = data_dir.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        SectionLayout { compiled_dir: data_dir.to_path_buf(), cache_dir: parent.join("cache").join("sections") }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.compiled_dir.join("manifest.toml")
    }

    pub fn sections_dir(&self) -> PathBuf {
        self.compiled_dir.join("sections")
    }

    /// `<compiled>/sections/<name>.<logical>.sqlite.zst` -- the name is for humans; the loader
    /// trusts only the hash.
    pub fn blob_path(&self, name: &str, logical: &str) -> PathBuf {
        self.sections_dir().join(format!("{name}.{logical}.sqlite.zst"))
    }

    /// The unpacked file's bytes are a function of the section's content AND the schema it was
    /// written under, so the name carries both: a schema bump never finds a stale unpack.
    pub fn cache_path(&self, logical: &str, schema_version: u32) -> PathBuf {
        self.cache_dir.join(format!("{logical}.{schema_version}.sqlite"))
    }
}

/// Whether a `resolve` error means the blob is simply absent -- an optional section a deployment
/// omitted -- as opposed to present but corrupt, unreadable or unpackable, which is always loud.
pub fn is_missing(e: &SectionError) -> bool {
    e.0.contains("has no blob at")
}

pub struct CommittedZstdSource {
    pub layout: SectionLayout,
}

impl SectionSource for CommittedZstdSource {
    fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError> {
        let cache = self.layout.cache_path(&entry.logical, entry.schema_version);
        if cache.is_file() {
            return Ok(cache);
        }
        let blob = self.layout.blob_path(&entry.name, &entry.logical);
        if !blob.is_file() {
            return Err(SqliteError(format!("section {} ({}) has no blob at {}", entry.name, entry.logical, blob.display())));
        }
        decompress_verified(&blob, &entry.blob, &cache)?;
        Ok(cache)
    }
}
