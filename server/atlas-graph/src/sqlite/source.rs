//! DB-4b: the section source (spec §2.4) -- the seam that makes growth
//! cheap. A `SectionSource` resolves a manifest entry to an openable
//! `.sqlite` file by its LOGICAL hash, verifying the transport hash before
//! it hands one over. Implementation #1, `CommittedZstdSource`: the blob
//! committed at `data/compiled/sections/<name>.<logical>.sqlite.zst`,
//! unpacked once into `data/cache/sections/<logical>.sqlite`. Cache hits
//! skip the verify (the file is named by its logical hash and was verified
//! when written; `bibex verify` re-checks on demand, spec §3.5).
//! Implementation #2 (a fetching source) lands when a section first
//! outgrows the git ceiling compressed; the loader, the server and the
//! port do not change.

use std::path::{Path, PathBuf};

use super::blob::decompress_verified;
use super::manifest::ManifestSection;
use super::SqliteError;

pub type SectionError = SqliteError;

/// Resolve a section by its LOGICAL hash to an openable SQLite file.
/// Implementations verify the transport hash before returning.
pub trait SectionSource {
    fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError>;
}

/// Where the committed blobs, the manifest and the unpack cache live
/// (spec §2.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionLayout {
    /// `data/compiled`: `manifest.toml` and `sections/`.
    pub compiled_dir: PathBuf,
    /// `data/cache/sections`: `<logical>.sqlite`, gitignored.
    pub cache_dir: PathBuf,
}

impl SectionLayout {
    /// The documented layout under a `data/compiled` directory: the cache
    /// is its sibling `data/cache/sections`.
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

    /// `<compiled>/sections/<name>.<logical>.sqlite.zst` -- the name is for
    /// humans; the loader trusts only the hash.
    pub fn blob_path(&self, name: &str, logical: &str) -> PathBuf {
        self.sections_dir().join(format!("{name}.{logical}.sqlite.zst"))
    }

    /// `<cache>/<logical>.sqlite`.
    pub fn cache_path(&self, logical: &str) -> PathBuf {
        self.cache_dir.join(format!("{logical}.sqlite"))
    }
}

/// Whether a `resolve` error means the blob is simply not there (an
/// optional section a deployment omitted), as opposed to present but
/// corrupt, unreadable or unpackable -- which is always loud.
pub fn is_missing(e: &SectionError) -> bool {
    e.0.contains("has no blob at")
}

pub struct CommittedZstdSource {
    pub layout: SectionLayout,
}

impl SectionSource for CommittedZstdSource {
    fn resolve(&self, entry: &ManifestSection) -> Result<PathBuf, SectionError> {
        let cache = self.layout.cache_path(&entry.logical);
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
