//! `manifest.toml` and its version root. The root's preimage is only
//! `name|logical|schema_version|required` per section, so byte sizes, blob hashes and timestamps
//! stay outside it and a byte-identical rebuild on another day still has the same root.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::SqliteError;

/// The manifest's own schema (the `schema = N` line), not the section schema.
pub const MANIFEST_SCHEMA: u32 = 1;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ManifestSection {
    pub name: String,
    pub required: bool,
    /// 32 lowercase hex: the section's logical hash.
    pub logical: String,
    /// 64 lowercase hex: SHA-256 of the committed `.zst` blob's bytes.
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

pub fn manifest_lines(sections: &[ManifestSection]) -> Vec<u8> {
    let entries: Vec<(&str, &str, u32, bool)> =
        sections.iter().map(|s| (s.name.as_str(), s.logical.as_str(), s.schema_version, s.required)).collect();
    atlas_graph_types::sections::manifest_lines(&entries)
}

/// The root as the manifest spells it: the hex of `sections::root_of_lines`.
pub fn root_of(sections: &[ManifestSection]) -> String {
    atlas_graph_types::sections::root_of_lines(&manifest_lines(sections)).hex()
}

pub fn write_manifest(m: &Manifest, path: &Path) -> Result<(), SqliteError> {
    let text = toml::to_string_pretty(m).map_err(|e| SqliteError(format!("manifest serialize: {e}")))?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Reads and VERIFIES: a manifest whose root does not recompute from its own section lines is
/// refused.
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
