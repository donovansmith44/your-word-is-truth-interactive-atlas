//! DB-2b: `manifest.toml` (spec §2.2) and the version root (spec §3.4).
//! The root hashes ONLY `name|logical|schema_version|required` per
//! section in manifest order -- byte sizes, blob hashes and timestamps are
//! outside it, so a byte-identical rebuild on another day has the same
//! root and a changed row does not.

use std::path::Path;

use atlas_graph_types::canon::DOMAIN_PREFIX;
use atlas_graph_types::sha256::sha256_prefixed_128;
use serde::{Deserialize, Serialize};

use super::SqliteError;

/// The manifest's own schema (the `schema = N` line), not the section schema.
pub const MANIFEST_SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ManifestSection {
    pub name: String,
    pub required: bool,
    /// 32 lowercase hex: the section's logical hash (spec §3.4).
    pub logical: String,
    /// 64 lowercase hex: SHA-256 of the `.sqlite` file as written (DB-4
    /// moves this to the compressed blob).
    pub blob: String,
    pub bytes: u64,
    pub schema_version: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema: u32,
    pub compiler: String,
    pub built: String,
    pub root: String,
    #[serde(rename = "section")]
    pub sections: Vec<ManifestSection>,
}

/// `name|logical|schema_version|required\n` per section, manifest order.
pub fn manifest_lines(sections: &[ManifestSection]) -> Vec<u8> {
    let mut out = Vec::new();
    for s in sections {
        out.extend_from_slice(
            format!("{}|{}|{}|{}\n", s.name, s.logical, s.schema_version, if s.required { "true" } else { "false" }).as_bytes(),
        );
    }
    out
}

/// Hex of `sha256_prefixed_128(DOMAIN_PREFIX, manifest_lines)`.
pub fn root_of(sections: &[ManifestSection]) -> String {
    hex16(&sha256_prefixed_128(DOMAIN_PREFIX, &manifest_lines(sections)))
}

pub(crate) fn hex16(b: &[u8; 16]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn write_manifest(m: &Manifest, path: &Path) -> Result<(), SqliteError> {
    let text = toml::to_string_pretty(m).map_err(|e| SqliteError(format!("manifest serialize: {e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Reads and VERIFIES: a manifest whose root does not recompute from its
/// own section lines is refused (spec §11).
pub fn read_manifest(path: &Path) -> Result<Manifest, SqliteError> {
    let text = std::fs::read_to_string(path)?;
    let m: Manifest = toml::from_str(&text).map_err(|e| SqliteError(format!("manifest parse {}: {e}", path.display())))?;
    let recomputed = root_of(&m.sections);
    if recomputed != m.root {
        return Err(SqliteError(format!(
            "manifest {}: root {} does not recompute from its sections ({recomputed})",
            path.display(),
            m.root
        )));
    }
    Ok(m)
}
