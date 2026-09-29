//! The drift between the raw tree `data/raw/MANIFEST.toml` records and the one on disk, by path,
//! so `verify` can name the file that moved rather than only the root that did.

use std::path::{Path, PathBuf};

use atlas_graph_types::raw_manifest::{RawEntry, Sha256};

/// `data/raw` sits beside `data/compiled`, as `data/cache` does.
const RAW_DIR: &str = "raw";

pub fn raw_dir_beside(data_dir: &Path) -> PathBuf {
    data_dir.parent().unwrap_or(data_dir).join(RAW_DIR)
}

/// How a walked tree differs from a recorded one, by path from the root of `data/raw`. Kept as
/// values so `verify` can name each in its own words.
#[derive(Debug, PartialEq, Eq)]
pub enum Drift {
    Missing { path: String },
    Extra { path: String },
    Truncated { path: String, recorded: u64, found: u64 },
    Changed { path: String, recorded: Sha256, found: Sha256 },
}

pub fn drift(recorded: &[RawEntry], walked: &[RawEntry]) -> Vec<Drift> {
    let mut out = Vec::new();
    drift_into("", recorded, walked, &mut out);
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
