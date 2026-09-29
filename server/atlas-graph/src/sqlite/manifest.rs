//! `manifest.toml` and its version root. The root's preimage is only
//! `name|logical|schema_version|required` per section, so byte sizes, blob hashes, timestamps and
//! the raw root stay outside it and a byte-identical rebuild on another day still has the same root.

use std::fmt;
use std::path::{Path, PathBuf};

use atlas_graph_types::raw_manifest::RawHash;
use serde::{Deserialize, Serialize};

use super::source::SectionLayout;
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
    /// The raw tree the sections were compiled from, as `data/raw/MANIFEST.toml` recorded it:
    /// provenance, never preimage (D1), so a refetch that changes nothing compiled moves nothing here.
    #[serde(default, with = "raw_hash_hex", skip_serializing_if = "Option::is_none")]
    pub raw_root: Option<RawHash>,
    #[serde(rename = "section")]
    pub sections: Vec<ManifestSection>,
}

/// A `RawHash` at the TOML boundary: its 32 lowercase hex digits, refused as the manifest is
/// read when they are anything else.
mod raw_hash_hex {
    use atlas_graph_types::raw_manifest::RawHash;
    use serde::de::Error as _;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(hash: &Option<RawHash>, serializer: S) -> Result<S::Ok, S::Error> {
        match hash {
            Some(hash) => serializer.serialize_str(&hash.hex()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<RawHash>, D::Error> {
        let text = String::deserialize(deserializer)?;
        RawHash::parse(&text).map(Some).map_err(|e| D::Error::custom(format!("{e:?}")))
    }
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

pub fn raw_manifest_path(layout: &SectionLayout) -> PathBuf {
    layout.raw_dir().join("MANIFEST.toml")
}

/// The root `bibex raw bless` recorded, or `None` where no manifest was ever written (a fresh
/// clone, or a tree never blessed). Only the root is read: the tree beneath is `bibex verify`'s
/// to recompute, and here the root is copied as provenance.
pub fn recorded_raw_root(raw_manifest: &Path) -> Result<Option<RawHash>, SqliteError> {
    let text = match std::fs::read_to_string(raw_manifest) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let rows: RawManifestRoot =
        toml::from_str(&text).map_err(|e| SqliteError(format!("raw manifest parse {}: {e}", raw_manifest.display())))?;
    let root = RawHash::parse(&rows.root)
        .map_err(|e| SqliteError(format!("raw manifest {}: root {:?} is not a hash: {e:?}", raw_manifest.display(), rows.root)))?;
    Ok(Some(root))
}

#[derive(Deserialize)]
struct RawManifestRoot {
    root: String,
}

/// What the two manifests say about the inputs: whether the raw tree `bibex raw bless` last
/// recorded is the one the compiled artifact was built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawProvenance {
    Unrecorded { compiled_from: Option<RawHash>, recorded: Option<RawHash> },
    Unchanged(RawHash),
    Changed { compiled_from: RawHash, recorded: RawHash },
}

pub fn raw_provenance(compiled_from: Option<RawHash>, recorded: Option<RawHash>) -> RawProvenance {
    match (compiled_from, recorded) {
        (Some(compiled_from), Some(recorded)) if compiled_from == recorded => RawProvenance::Unchanged(recorded),
        (Some(compiled_from), Some(recorded)) => RawProvenance::Changed { compiled_from, recorded },
        (compiled_from, recorded) => RawProvenance::Unrecorded { compiled_from, recorded },
    }
}

impl fmt::Display for RawProvenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RawProvenance::Unrecorded { compiled_from, recorded } => write!(
                f,
                "raw provenance unrecorded: the compiled artifact {}, data/raw/MANIFEST.toml {}",
                compiled_from.map_or("names no raw root".to_string(), |h| format!("was built from raw {}", h.hex())),
                recorded.map_or("records none".to_string(), |h| format!("records {}", h.hex()))
            ),
            RawProvenance::Unchanged(root) => write!(
                f,
                "the raw inputs did not move: data/raw/MANIFEST.toml records {}, the root the compiled artifact was built from, so the difference is in the code (or in a file 'bibex verify' would name)",
                root.hex()
            ),
            RawProvenance::Changed { compiled_from, recorded } => write!(
                f,
                "the raw inputs ALSO moved: the compiled artifact was built from raw {} but data/raw/MANIFEST.toml records {} -- a refetch or 'bibex raw bless' changed the inputs since the artifact was compiled",
                compiled_from.hex(),
                recorded.hex()
            ),
        }
    }
}
