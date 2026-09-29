//! Walking `data/raw` into the raw input tree, `MANIFEST.toml` as its text form, and the drift
//! between the two. The tree carries bytes ([`Sha256`], [`RawHash`]); the TOML boundary is the
//! only place they are hex.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use atlas_graph_types::raw_manifest::{node_hash, HexError, RawEntry, RawHash, RawLeaf, RawNode, Sha256};
use atlas_graph_types::sha256::sha256;
use serde::{Deserialize, Serialize};

/// The manifest's own schema (the `schema = N` line).
pub const MANIFEST_SCHEMA: u32 = 1;
pub const MANIFEST_FILE: &str = "MANIFEST.toml";
/// The TOML key whose value is the root hash; it names the place when that value is not hex.
const ROOT_KEY: &str = "root";

/// Git already pins these two at the root of `data/raw`: the manifest records what git does not,
/// and a manifest cannot contain its own hash.
const KEPT_BY_GIT: [&str; 2] = ["README.md", MANIFEST_FILE];

/// The whole of a walk. `datasets` are the root's entries, name-ordered; `root` is their
/// [`node_hash`]. `links` are the reparse points (junctions, symlinks) the walk refused to
/// follow, as paths relative to the root in walk order: a link is never inlined as its target
/// (following one is how `data/raw` was emptied on 2026-09-28), and it is never silent either,
/// so a verify can see that one is there.
#[derive(Debug, PartialEq, Eq)]
pub struct RawManifest {
    pub schema: u32,
    pub root: RawHash,
    pub datasets: Vec<RawEntry>,
    pub links: Vec<String>,
}

/// Why a `MANIFEST.toml` is not a manifest.
#[derive(Debug)]
pub enum ManifestError {
    Io(io::Error),
    Toml(toml::de::Error),
    Schema { found: u32 },
    Hex { path: String, error: HexError },
    Node { path: String, recorded: RawHash, recomputed: RawHash },
    Root { recorded: RawHash, recomputed: RawHash },
}

pub fn walk(root: &Path) -> io::Result<RawManifest> {
    let mut links = Vec::new();
    let datasets = children_of(root, "", &KEPT_BY_GIT, &mut links)?;
    Ok(RawManifest { schema: MANIFEST_SCHEMA, root: node_hash(&datasets), datasets, links })
}

fn children_of(dir: &Path, relative: &str, skip: &[&str], links: &mut Vec<String>) -> io::Result<Vec<RawEntry>> {
    let mut named = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().into_string().map_err(|unspellable| {
            io::Error::new(io::ErrorKind::InvalidData, format!("{}: a name the manifest cannot spell: {unspellable:?}", dir.display()))
        })?;
        if !skip.contains(&name.as_str()) {
            named.push((name, entry));
        }
    }
    named.sort_by(|a, b| a.0.cmp(&b.0));
    let mut children = Vec::with_capacity(named.len());
    for (name, entry) in named {
        let path = relative_path(relative, &name);
        if let Some(child) = entry_of(name, &entry.path(), &path, entry.file_type()?, links)? {
            children.push(child);
        }
    }
    Ok(children)
}

/// A link is listed, never followed or hashed, so it yields no entry.
fn entry_of(name: String, path: &Path, relative: &str, kind: fs::FileType, links: &mut Vec<String>) -> io::Result<Option<RawEntry>> {
    if kind.is_symlink() {
        links.push(relative.to_string());
        return Ok(None);
    }
    if kind.is_dir() {
        let children = children_of(path, relative, &[], links)?;
        return Ok(Some(RawEntry::Node(RawNode { name, hash: node_hash(&children), children })));
    }
    let bytes = fs::read(path).map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))?;
    Ok(Some(RawEntry::Leaf(RawLeaf { name, sha256: Sha256(sha256(&bytes)), bytes: bytes.len() as u64 })))
}

/// What stands at one path under the root, hashed as [`walk`] would hash it: `entry` is `None`
/// when nothing is there or a link is, and `links` lists every reparse point met, the path
/// itself included when it is one, so a link is never mistaken for content that is missing.
pub struct Found {
    pub entry: Option<RawEntry>,
    pub links: Vec<String>,
}

/// `relative` is in the manifest's spelling (`/`-separated, no root). Absence is an answer, not
/// an error: a caller asks precisely to learn whether the path is there.
pub fn walk_at(root: &Path, relative: &str) -> io::Result<Found> {
    let path = relative.split('/').fold(root.to_path_buf(), |path, component| path.join(component));
    let name = match relative.rfind('/') {
        Some(slash) => &relative[slash + 1..],
        None => relative,
    };
    let mut links = Vec::new();
    let entry = match fs::symlink_metadata(&path) {
        Ok(meta) => entry_of(name.to_string(), &path, relative, meta.file_type(), &mut links)?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    Ok(Found { entry, links })
}

fn relative_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// Reads and VERIFIES: every directory's hash must recompute from its children and the root
/// from its datasets, so a hand-edited or truncated manifest is refused before it is trusted.
pub fn read_manifest(path: &Path) -> Result<RawManifest, ManifestError> {
    let text = fs::read_to_string(path)?;
    let rows: ManifestRows = toml::from_str(&text)?;
    if rows.schema != MANIFEST_SCHEMA {
        return Err(ManifestError::Schema { found: rows.schema });
    }
    let root = RawHash::parse(&rows.root).map_err(|error| ManifestError::Hex { path: ROOT_KEY.into(), error })?;
    let datasets = entries_of_rows(rows.datasets, "")?;
    let recomputed = node_hash(&datasets);
    if recomputed != root {
        return Err(ManifestError::Root { recorded: root, recomputed });
    }
    Ok(RawManifest { schema: rows.schema, root, datasets, links: rows.links })
}

fn entries_of_rows(mut rows: Vec<EntryRow>, parent: &str) -> Result<Vec<RawEntry>, ManifestError> {
    rows.sort_by(|a, b| a.name().cmp(b.name()));
    rows.into_iter().map(|row| entry_of_row(row, parent)).collect()
}

fn entry_of_row(row: EntryRow, parent: &str) -> Result<RawEntry, ManifestError> {
    let path = relative_path(parent, row.name());
    match row {
        EntryRow::File(file) => {
            let sha256 = Sha256::parse(&file.sha256).map_err(|error| ManifestError::Hex { path, error })?;
            Ok(RawEntry::Leaf(RawLeaf { name: file.name, sha256, bytes: file.bytes }))
        }
        EntryRow::Dir(dir) => {
            let recorded = RawHash::parse(&dir.hash).map_err(|error| ManifestError::Hex { path: path.clone(), error })?;
            let rows = dir.files.into_iter().map(EntryRow::File).chain(dir.dirs.into_iter().map(EntryRow::Dir)).collect();
            let children = entries_of_rows(rows, &path)?;
            let recomputed = node_hash(&children);
            if recomputed != recorded {
                return Err(ManifestError::Node { path, recorded, recomputed });
            }
            Ok(RawEntry::Node(RawNode { name: dir.name, hash: recorded, children }))
        }
    }
}

pub fn write_manifest(path: &Path, manifest: &RawManifest) -> io::Result<()> {
    let rows = ManifestRows {
        schema: manifest.schema,
        root: manifest.root.hex(),
        links: manifest.links.clone(),
        datasets: manifest.datasets.iter().map(row_of_entry).collect(),
    };
    let text = toml::to_string(&rows).expect("strings, integers and tables of the same: always TOML");
    fs::write(path, text)
}

fn row_of_entry(entry: &RawEntry) -> EntryRow {
    match entry {
        RawEntry::Leaf(leaf) => EntryRow::File(FileRow { name: leaf.name.clone(), sha256: leaf.sha256.hex(), bytes: leaf.bytes }),
        RawEntry::Node(node) => {
            let mut files = Vec::new();
            let mut dirs = Vec::new();
            for child in node.children.iter().map(row_of_entry) {
                match child {
                    EntryRow::File(file) => files.push(file),
                    EntryRow::Dir(dir) => dirs.push(dir),
                }
            }
            EntryRow::Dir(DirRow { name: node.name.clone(), hash: node.hash.hex(), files, dirs })
        }
    }
}

/// The TOML shape: `schema`, `root`, optional `links`, then one `[[dataset]]` per root entry,
/// each a file row or a directory row. A directory's files come before its subdirectories
/// because TOML arrays of tables are typed per key; the reader restores name order.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestRows {
    schema: u32,
    root: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    links: Vec<String>,
    #[serde(default, rename = "dataset", skip_serializing_if = "Vec::is_empty")]
    datasets: Vec<EntryRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum EntryRow {
    File(FileRow),
    Dir(DirRow),
}

impl EntryRow {
    fn name(&self) -> &str {
        match self {
            EntryRow::File(file) => &file.name,
            EntryRow::Dir(dir) => &dir.name,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRow {
    name: String,
    sha256: String,
    bytes: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DirRow {
    name: String,
    hash: String,
    #[serde(default, rename = "file", skip_serializing_if = "Vec::is_empty")]
    files: Vec<FileRow>,
    #[serde(default, rename = "dir", skip_serializing_if = "Vec::is_empty")]
    dirs: Vec<DirRow>,
}

impl From<io::Error> for ManifestError {
    fn from(error: io::Error) -> Self {
        ManifestError::Io(error)
    }
}

impl From<toml::de::Error> for ManifestError {
    fn from(error: toml::de::Error) -> Self {
        ManifestError::Toml(error)
    }
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ManifestError::Io(error) => write!(f, "{error}"),
            ManifestError::Toml(error) => write!(f, "manifest parse: {error}"),
            ManifestError::Schema { found } => write!(f, "manifest schema {found} is not {MANIFEST_SCHEMA}"),
            ManifestError::Hex { path, error: HexError::Length { expected, found } } => {
                write!(f, "manifest {path}: expected {expected} hex digits, found {found}")
            }
            ManifestError::Hex { path, error: HexError::Digit { at, found } } => {
                write!(f, "manifest {path}: '{found}' at {at} is not a lowercase hex digit")
            }
            ManifestError::Node { path, recorded, recomputed } => {
                write!(f, "manifest {path}: hash {} does not recompute from its children ({})", recorded.hex(), recomputed.hex())
            }
            ManifestError::Root { recorded, recomputed } => {
                write!(f, "manifest root {} does not recompute from its datasets ({})", recorded.hex(), recomputed.hex())
            }
        }
    }
}

pub const DO_RAW: &str = "restore data/raw from the archive under Documents/bible-atlas-backups or refetch it with data/fetch-raw.ps1; if the change was deliberate, record it with 'bibex raw bless' and commit data/raw/MANIFEST.toml";

/// How a walked tree differs from a recorded one, by path from the root of `data/raw`. Kept as
/// values so `verify` can name each in its own words and `bless` in its.
#[derive(Debug, PartialEq, Eq)]
pub enum Drift {
    Missing { path: String },
    Extra { path: String },
    Truncated { path: String, recorded: u64, found: u64 },
    Changed { path: String, recorded: Sha256, found: Sha256 },
    LinkAdded { path: String },
    LinkRemoved { path: String },
}

pub enum Unrecorded {
    NoManifest { manifest: PathBuf },
    NoFiles { raw_dir: PathBuf },
}

/// The datasets by name, then the links: a link is not content and moves no hash, so it is
/// compared as a list of its own, and one added or removed is a change the tree must see.
pub fn drift(recorded: &RawManifest, walked: &RawManifest) -> Vec<Drift> {
    let mut out = Vec::new();
    drift_into("", &recorded.datasets, &walked.datasets, &mut out);
    out.extend(link_drift(&recorded.links, &walked.links));
    out
}

pub fn drift_at(path: &str, recorded: &RawEntry, found: Option<&RawEntry>) -> Vec<Drift> {
    let mut out = Vec::new();
    match found {
        None => out.push(Drift::Missing { path: path.into() }),
        Some(found) => drift_of_pair(path, recorded, found, &mut out),
    }
    out
}

pub fn link_drift(recorded: &[String], walked: &[String]) -> Vec<Drift> {
    let removed = recorded.iter().filter(|path| !walked.contains(path)).map(|path| Drift::LinkRemoved { path: path.clone() });
    let added = walked.iter().filter(|path| !recorded.contains(path)).map(|path| Drift::LinkAdded { path: path.clone() });
    removed.chain(added).collect()
}

pub fn leaves(entries: &[RawEntry]) -> usize {
    entries
        .iter()
        .map(|entry| match entry {
            RawEntry::Leaf(_) => 1,
            RawEntry::Node(node) => leaves(&node.children),
        })
        .sum()
}

fn drift_into(parent: &str, recorded: &[RawEntry], walked: &[RawEntry], out: &mut Vec<Drift>) {
    let mut recorded = recorded.iter().peekable();
    let mut walked = walked.iter().peekable();
    loop {
        match (recorded.peek(), walked.peek()) {
            (None, None) => return,
            (Some(r), None) => {
                out.push(Drift::Missing { path: relative_path(parent, r.name()) });
                recorded.next();
            }
            (None, Some(w)) => {
                out.push(Drift::Extra { path: relative_path(parent, w.name()) });
                walked.next();
            }
            (Some(r), Some(w)) => match r.name().cmp(w.name()) {
                std::cmp::Ordering::Less => {
                    out.push(Drift::Missing { path: relative_path(parent, r.name()) });
                    recorded.next();
                }
                std::cmp::Ordering::Greater => {
                    out.push(Drift::Extra { path: relative_path(parent, w.name()) });
                    walked.next();
                }
                std::cmp::Ordering::Equal => {
                    drift_of_pair(&relative_path(parent, r.name()), r, w, out);
                    recorded.next();
                    walked.next();
                }
            },
        }
    }
}

fn drift_of_pair(path: &str, recorded: &RawEntry, walked: &RawEntry, out: &mut Vec<Drift>) {
    match (recorded, walked) {
        (RawEntry::Leaf(r), RawEntry::Leaf(w)) => {
            if r.sha256 == w.sha256 {
                return;
            }
            if w.bytes < r.bytes {
                out.push(Drift::Truncated { path: path.into(), recorded: r.bytes, found: w.bytes });
            } else {
                out.push(Drift::Changed { path: path.into(), recorded: r.sha256, found: w.sha256 });
            }
        }
        (RawEntry::Node(r), RawEntry::Node(w)) => {
            if r.hash != w.hash {
                drift_into(path, &r.children, &w.children, out);
            }
        }
        (RawEntry::Leaf(_), RawEntry::Node(_)) | (RawEntry::Node(_), RawEntry::Leaf(_)) => {
            out.push(Drift::Missing { path: path.into() });
            out.push(Drift::Extra { path: path.into() });
        }
    }
}

impl fmt::Display for Drift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Drift::Missing { path } => write!(f, "raw {path}: MISSING"),
            Drift::Extra { path } => write!(f, "raw {path}: EXTRA (not in {MANIFEST_FILE})"),
            Drift::Truncated { path, recorded, found } => write!(f, "raw {path}: TRUNCATED manifest {recorded} bytes file {found}"),
            Drift::Changed { path, recorded, found } => write!(f, "raw {path}: MISMATCH manifest {} file {}", recorded.hex(), found.hex()),
            Drift::LinkAdded { path } => write!(f, "raw {path}: LINK (a junction or symlink {MANIFEST_FILE} does not list)"),
            Drift::LinkRemoved { path } => write!(f, "raw {path}: LINK MISSING ({MANIFEST_FILE} lists a junction or symlink there)"),
        }
    }
}

impl fmt::Display for Unrecorded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unrecorded::NoManifest { manifest } => write!(f, "no {MANIFEST_FILE} at {}", manifest.display()),
            Unrecorded::NoFiles { raw_dir } => write!(f, "{} has no files", raw_dir.display()),
        }
    }
}
