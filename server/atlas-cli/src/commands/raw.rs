//! `bibex raw bless` -- the one writer of `data/raw/MANIFEST.toml`. A separate verb, never a
//! `--fix` on `verify` (D4: a pin that is re-blessed as a matter of routine stops being read), and
//! it refuses an empty tree, which is what a killed fetch or a followed junction leaves behind.
//! The drift between a recorded tree and a walked one lives here too: `verify` reports it,
//! `bless` summarizes it.

use std::path::{Path, PathBuf};

use atlas_cli::raw::{read_manifest, walk, write_manifest, RawManifest, MANIFEST_FILE};
use atlas_graph_types::raw_manifest::{RawEntry, RawHash, Sha256};

use crate::commands::verify::commas;
use crate::error::CliError;

/// `data/raw` sits beside `data/compiled`, as `data/cache` does.
const RAW_DIR: &str = "raw";

pub fn bless(data_dir: &Path) -> Result<String, CliError> {
    let blessing = perform(data_dir)?;
    let mut out = format!("raw {} recorded ({} files) at {}\n", blessing.root.hex(), commas(blessing.files as u64), blessing.manifest.display());
    match &blessing.previous {
        None => out.push_str("previous none\n"),
        Some(previous) => {
            out.push_str(&format!("previous {}\n", previous.root.hex()));
            if previous.is_unchanged() {
                out.push_str("unchanged\n");
            }
            for path in &previous.added {
                out.push_str(&format!("added {path}\n"));
            }
            for path in &previous.removed {
                out.push_str(&format!("removed {path}\n"));
            }
            for path in &previous.changed {
                out.push_str(&format!("changed {path}\n"));
            }
        }
    }
    Ok(out)
}

pub fn bless_json(data_dir: &Path) -> Result<serde_json::Value, CliError> {
    let blessing = perform(data_dir)?;
    let (previous, added, removed, changed) = match blessing.previous {
        None => (None, Vec::new(), Vec::new(), Vec::new()),
        Some(previous) => (Some(previous.root.hex()), previous.added, previous.removed, previous.changed),
    };
    Ok(serde_json::json!({
        "root": blessing.root.hex(),
        "files": blessing.files,
        "manifest": blessing.manifest.display().to_string(),
        "previous": previous,
        "added": added,
        "removed": removed,
        "changed": changed,
    }))
}

pub fn raw_dir_beside(data_dir: &Path) -> PathBuf {
    data_dir.parent().unwrap_or(data_dir).join(RAW_DIR)
}

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

/// The datasets by name, then the links: a link is not content and moves no hash, so it is
/// compared as a list of its own, and one added or removed is a change the tree must see.
pub fn drift(recorded: &RawManifest, walked: &RawManifest) -> Vec<Drift> {
    let mut out = Vec::new();
    drift_into("", &recorded.datasets, &walked.datasets, &mut out);
    out.extend(recorded.links.iter().filter(|path| !walked.links.contains(path)).map(|path| Drift::LinkRemoved { path: path.clone() }));
    out.extend(walked.links.iter().filter(|path| !recorded.links.contains(path)).map(|path| Drift::LinkAdded { path: path.clone() }));
    out
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

struct Blessing {
    root: RawHash,
    files: usize,
    manifest: PathBuf,
    previous: Option<Previous>,
}

struct Previous {
    root: RawHash,
    added: Vec<String>,
    removed: Vec<String>,
    changed: Vec<String>,
}

impl Previous {
    fn is_unchanged(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

fn perform(data_dir: &Path) -> Result<Blessing, CliError> {
    let raw_dir = raw_dir_beside(data_dir);
    let walked = walk(&raw_dir).map_err(|e| {
        CliError::data_load_failed(
            format!("{} could not be walked", raw_dir.display()),
            e.to_string(),
            "pass --data-dir so that data/raw sits beside it, and fetch the raw tree first with data/fetch-raw.ps1",
        )
    })?;
    let files = leaves(&walked.datasets);
    if files == 0 {
        return Err(CliError::data_load_failed(
            format!("{} has no files to record", raw_dir.display()),
            "an empty tree is what a killed fetch or a followed junction leaves behind, and a blessing would make it the truth",
            "run data/fetch-raw.ps1 (or restore data/raw from the archive), check it with 'bibex verify', then bless",
        ));
    }
    let manifest = raw_dir.join(MANIFEST_FILE);
    let previous = previous_at(&manifest, &walked)?;
    write_manifest(&manifest, &walked).map_err(|e| CliError::data_load_failed(format!("{} could not be written", manifest.display()), e.to_string(), "check that data/raw is writable"))?;
    Ok(Blessing { root: walked.root, files, manifest, previous })
}

fn previous_at(manifest: &Path, walked: &RawManifest) -> Result<Option<Previous>, CliError> {
    if !manifest.is_file() {
        return Ok(None);
    }
    let recorded = read_manifest(manifest).map_err(|e| {
        CliError::integrity_failed(
            format!("{} does not read, so there is nothing to diff against", manifest.display()),
            e.to_string(),
            "restore data/raw/MANIFEST.toml from git, or delete it deliberately, then bless again",
        )
    })?;
    let mut previous = Previous { root: recorded.root, added: Vec::new(), removed: Vec::new(), changed: Vec::new() };
    for d in drift(&recorded, walked) {
        match d {
            Drift::Extra { path } | Drift::LinkAdded { path } => previous.added.push(path),
            Drift::Missing { path } | Drift::LinkRemoved { path } => previous.removed.push(path),
            Drift::Truncated { path, .. } | Drift::Changed { path, .. } => previous.changed.push(path),
        }
    }
    Ok(Some(previous))
}

fn drift_into(parent: &str, recorded: &[RawEntry], walked: &[RawEntry], out: &mut Vec<Drift>) {
    let mut recorded = recorded.iter().peekable();
    let mut walked = walked.iter().peekable();
    loop {
        match (recorded.peek(), walked.peek()) {
            (None, None) => return,
            (Some(r), None) => {
                out.push(Drift::Missing { path: path_under(parent, name_of(r)) });
                recorded.next();
            }
            (None, Some(w)) => {
                out.push(Drift::Extra { path: path_under(parent, name_of(w)) });
                walked.next();
            }
            (Some(r), Some(w)) => match name_of(r).cmp(name_of(w)) {
                std::cmp::Ordering::Less => {
                    out.push(Drift::Missing { path: path_under(parent, name_of(r)) });
                    recorded.next();
                }
                std::cmp::Ordering::Greater => {
                    out.push(Drift::Extra { path: path_under(parent, name_of(w)) });
                    walked.next();
                }
                std::cmp::Ordering::Equal => {
                    drift_of_pair(&path_under(parent, name_of(r)), r, w, out);
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

fn name_of(entry: &RawEntry) -> &str {
    match entry {
        RawEntry::Leaf(leaf) => &leaf.name,
        RawEntry::Node(node) => &node.name,
    }
}

fn path_under(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}
