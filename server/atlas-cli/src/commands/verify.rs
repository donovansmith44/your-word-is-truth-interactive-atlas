//! `bibex verify [--section <name>]` -- recompute each cached section's logical hash, each
//! committed blob's transport hash and the manifest root, print them against the manifest,
//! and exit non-zero on any mismatch. A full verify also walks `data/raw` against
//! `data/raw/MANIFEST.toml` and names the file that moved.

use std::fmt;
use std::path::{Path, PathBuf};

use atlas_cli::raw::{read_manifest as read_raw_manifest, walk, MANIFEST_FILE};
use atlas_graph::sections::Section;
use atlas_graph::sqlite::blob::sha256_hex_of_file;
use atlas_graph::sqlite::logical::{logical_dump_of_db, logical_hash};
use atlas_graph::sqlite::manifest::{read_manifest, root_of, Manifest};
use atlas_graph::sqlite::open_read_only;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout, SectionSource};
use atlas_graph_types::raw_manifest::RawHash;

use crate::commands::raw::{drift, leaves, raw_dir_beside, Drift};
use crate::error::CliError;

pub struct SectionReport {
    pub name: String,
    pub required: bool,
    pub logical: String,
    pub blob: String,
    pub bytes: u64,
    /// `ok` | `missing` (required, no blob) | `absent` (optional, no blob) | `mismatch`
    pub transport: &'static str,
    /// `ok` | `mismatch` | `skipped` (no blob, or the transport failed first)
    pub logical_check: &'static str,
    pub schema_version: u32,
    pub uncompressed_bytes: Option<u64>,
    /// Every failing check, one line each (empty when the section verifies).
    pub failures: Vec<String>,
}

pub struct Report {
    pub manifest: Manifest,
    pub recomputed_root: String,
    pub sections: Vec<SectionReport>,
    /// `None` under `--section`: that asks about one compiled section, and the raw tree is not one.
    pub raw: Option<RawSection>,
}

/// The raw tree beside the compiled one (D1: a sibling of the compiled chain, never a link in it).
pub enum RawSection {
    /// Nothing to check against, and not a failure: a fresh clone has the manifest and no tree.
    Unrecorded(Unrecorded),
    Checked { root: RawHash, files: usize, drift: Vec<Drift> },
}

pub enum Unrecorded {
    NoManifest { manifest: PathBuf },
    NoFiles { raw_dir: PathBuf },
}

impl Report {
    fn compiled_failures(&self) -> Vec<String> {
        self.sections.iter().flat_map(|s| s.failures.iter().cloned()).collect()
    }
    fn raw_failures(&self) -> Vec<String> {
        match &self.raw {
            Some(RawSection::Checked { drift, .. }) => drift.iter().map(raw_failure).collect(),
            Some(RawSection::Unrecorded(_)) | None => Vec::new(),
        }
    }
    pub fn checks(&self) -> usize {
        // Two checks per section -- transport and logical, the schema counted with the
        // logical one -- plus the root, plus the raw tree when there is one to check.
        let raw = match &self.raw {
            Some(RawSection::Checked { .. }) => 1,
            Some(RawSection::Unrecorded(_)) | None => 0,
        };
        1 + self.sections.len() * 2 + raw
    }
}

const DO: &str = "recompile (cargo run -p atlas-graph --bin atlas-graph-compile, from server/) or restore data/compiled from git; a tampered or truncated section must never be served";
const DO_RAW: &str = "restore data/raw from the archive under Documents/bible-atlas-backups or refetch it with data/fetch-raw.ps1; if the change was deliberate, record it with 'bibex raw bless' and commit data/raw/MANIFEST.toml";

fn section_named(name: &str) -> Option<Section> {
    Section::MANIFEST_ORDER.iter().copied().find(|s| s.name() == name)
}

pub fn check(data_dir: &Path, only: Option<&str>) -> Result<Report, CliError> {
    let layout = SectionLayout::under(data_dir);
    let manifest_path = layout.manifest_path();
    if !manifest_path.is_file() {
        return Err(CliError::data_load_failed(
            format!("could not find {}", manifest_path.display()),
            "there is no manifest.toml to verify against -- this data directory predates the committed sections (DB-4b)",
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ first, or pass --data-dir to point at a directory that has manifest.toml",
        ));
    }
    let manifest = read_manifest(&manifest_path).map_err(|e| {
        CliError::integrity_failed(format!("{} does not verify", manifest_path.display()), e.to_string(), DO)
    })?;
    if let Some(name) = only {
        if !manifest.sections.iter().any(|s| s.name == name) {
            let known: Vec<&str> = manifest.sections.iter().map(|s| s.name.as_str()).collect();
            return Err(CliError::bad_usage(
                format!("unknown section '{name}'"),
                format!("the manifest lists {}", known.join(", ")),
                "run 'bibex verify' with no --section, or name one of the listed sections",
            ));
        }
    }
    let recomputed_root = root_of(&manifest.sections);
    let source = CommittedZstdSource { layout: layout.clone() };
    let mut sections = Vec::with_capacity(manifest.sections.len());
    for ms in &manifest.sections {
        if only.is_some_and(|o| o != ms.name) {
            continue;
        }
        let mut r = SectionReport {
            name: ms.name.clone(),
            required: ms.required,
            logical: ms.logical.clone(),
            blob: ms.blob.clone(),
            bytes: ms.bytes,
            transport: "ok",
            logical_check: "skipped",
            schema_version: ms.schema_version,
            uncompressed_bytes: None,
            failures: Vec::new(),
        };
        let blob_path = layout.blob_path(&ms.name, &ms.logical);
        if !blob_path.is_file() {
            if ms.required {
                r.transport = "missing";
                r.failures.push(format!("{}: required blob MISSING at {}", ms.name, blob_path.display()));
            } else {
                r.transport = "absent";
            }
            sections.push(r);
            continue;
        }
        match sha256_hex_of_file(&blob_path) {
            Ok(actual) if actual == ms.blob => {}
            Ok(actual) => {
                r.transport = "mismatch";
                r.failures.push(format!("{}: transport MISMATCH manifest {} file {}", ms.name, ms.blob, actual));
            }
            Err(e) => {
                r.transport = "mismatch";
                r.failures.push(format!("{}: blob unreadable: {e}", ms.name));
            }
        }
        if r.transport != "ok" {
            sections.push(r);
            continue;
        }
        let section = match section_named(&ms.name) {
            Some(s) => s,
            None => {
                r.failures.push(format!("{}: not a section this binary knows", ms.name));
                sections.push(r);
                continue;
            }
        };
        let logical_result = (|| -> Result<(u64, u32, String), String> {
            let path = source.resolve(ms).map_err(|e| e.to_string())?;
            let bytes = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
            let conn = open_read_only(&path).map_err(|e| e.to_string())?;
            let user_version: u32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|e| e.to_string())?;
            let dump = logical_dump_of_db(&conn, section).map_err(|e| e.to_string())?;
            Ok((bytes, user_version, logical_hash(&dump)))
        })();
        match logical_result {
            Ok((bytes, user_version, hash)) => {
                r.uncompressed_bytes = Some(bytes);
                if user_version != ms.schema_version {
                    r.logical_check = "mismatch";
                    r.failures.push(format!("{}: schema MISMATCH manifest {} file {user_version}", ms.name, ms.schema_version));
                } else if hash != ms.logical {
                    r.logical_check = "mismatch";
                    r.failures.push(format!("{}: logical MISMATCH manifest {} file {hash}", ms.name, ms.logical));
                } else {
                    r.logical_check = "ok";
                }
            }
            Err(e) => {
                r.logical_check = "mismatch";
                r.failures.push(format!("{}: {e}", ms.name));
            }
        }
        sections.push(r);
    }
    let raw = match only {
        Some(_) => None,
        None => Some(check_raw(data_dir)?),
    };
    Ok(Report { manifest, recomputed_root, sections, raw })
}

/// The manifest is looked for before the tree is walked, so a clone with neither pays nothing;
/// a walked tree with no files is unrecorded too, so an emptied `data/raw` can never pass.
fn check_raw(data_dir: &Path) -> Result<RawSection, CliError> {
    let raw_dir = raw_dir_beside(data_dir);
    let manifest = raw_dir.join(MANIFEST_FILE);
    if !manifest.is_file() {
        return Ok(RawSection::Unrecorded(Unrecorded::NoManifest { manifest }));
    }
    let walked = walk(&raw_dir).map_err(|e| CliError::integrity_failed(format!("{} could not be walked", raw_dir.display()), e.to_string(), DO_RAW))?;
    let files = leaves(&walked.datasets);
    if files == 0 {
        return Ok(RawSection::Unrecorded(Unrecorded::NoFiles { raw_dir }));
    }
    let recorded = read_raw_manifest(&manifest).map_err(|e| CliError::integrity_failed(format!("{} does not verify", manifest.display()), e.to_string(), DO_RAW))?;
    Ok(RawSection::Checked { root: walked.root, files, drift: drift(&recorded.datasets, &walked.datasets) })
}

fn raw_failure(drift: &Drift) -> String {
    match drift {
        Drift::Missing { path } => format!("raw {path}: MISSING"),
        Drift::Extra { path } => format!("raw {path}: EXTRA (not in {MANIFEST_FILE})"),
        Drift::Truncated { path, recorded, found } => format!("raw {path}: TRUNCATED manifest {recorded} bytes file {found}"),
        Drift::Changed { path, recorded, found } => format!("raw {path}: MISMATCH manifest {} file {}", recorded.hex(), found.hex()),
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

fn commas(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn render(report: &Report) -> String {
    let mut out = String::new();
    for s in &report.sections {
        let logical = match s.logical_check {
            "ok" => "OK".to_string(),
            "skipped" => "skipped".to_string(),
            _ => s.failures.iter().find(|f| f.contains("logical") || f.contains("schema") || !f.contains("transport")).map(|f| format!("MISMATCH ({})", f.trim_start_matches(&format!("{}: ", s.name)))).unwrap_or_else(|| "MISMATCH".into()),
        };
        let transport = match s.transport {
            "ok" => "OK".to_string(),
            "absent" => "absent (optional)".to_string(),
            "missing" => "MISSING".to_string(),
            _ => s.failures.iter().find(|f| f.contains("transport") || f.contains("unreadable")).map(|f| format!("MISMATCH ({})", f.trim_start_matches(&format!("{}: ", s.name)))).unwrap_or_else(|| "MISMATCH".into()),
        };
        let sizes = match s.uncompressed_bytes {
            Some(u) => format!("{} -> {} bytes", commas(u), commas(s.bytes)),
            None => format!("{} bytes", commas(s.bytes)),
        };
        out.push_str(&format!("{:<10} logical {}  {:<8} transport {:<8} {}\n", s.name, s.logical, logical, transport, sizes));
    }
    let root_ok = report.recomputed_root == report.manifest.root;
    out.push_str(&format!(
        "root {} {} (recomputed from {} section lines)\n",
        report.manifest.root,
        if root_ok { "OK" } else { "MISMATCH" },
        report.manifest.sections.len()
    ));
    match &report.raw {
        Some(RawSection::Unrecorded(why)) => out.push_str(&format!("raw: unrecorded ({why})\n")),
        Some(RawSection::Checked { root, files, .. }) => out.push_str(&format!("raw {} OK ({} files)\n", root.hex(), commas(*files as u64))),
        None => {}
    }
    out
}

/// The raw tree is one check however many files drifted: every drifted file is named, but the
/// count says how many checks failed, not how many lines say so.
fn fail(report: &Report) -> Option<CliError> {
    let compiled = report.compiled_failures();
    let raw = report.raw_failures();
    let do_ = match (compiled.is_empty(), raw.is_empty()) {
        (true, true) => return None,
        (false, true) => DO.to_string(),
        (true, false) => DO_RAW.to_string(),
        (false, false) => format!("{DO}; {DO_RAW}"),
    };
    let failed = compiled.len() + usize::from(!raw.is_empty());
    let failures: Vec<String> = compiled.into_iter().chain(raw).collect();
    Some(CliError::integrity_failed(format!("{failed} of {} checks failed", report.checks()), failures.join("; "), do_))
}

pub fn run(data_dir: &Path, only: Option<&str>) -> Result<String, CliError> {
    let report = check(data_dir, only)?;
    match fail(&report) {
        Some(e) => Err(e),
        None => Ok(render(&report)),
    }
}

pub fn run_json(data_dir: &Path, only: Option<&str>) -> Result<serde_json::Value, CliError> {
    let report = check(data_dir, only)?;
    if let Some(e) = fail(&report) {
        return Err(e);
    }
    let sections: Vec<serde_json::Value> = report
        .sections
        .iter()
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "required": s.required,
                "logical": s.logical,
                "blob": s.blob,
                "bytes": s.bytes,
                "transport": s.transport,
                "logical_check": s.logical_check,
                "schema_version": s.schema_version,
                "uncompressed_bytes": s.uncompressed_bytes,
            })
        })
        .collect();
    let raw = match &report.raw {
        Some(RawSection::Unrecorded(why)) => serde_json::json!({ "status": "unrecorded", "why": why.to_string() }),
        Some(RawSection::Checked { root, files, .. }) => serde_json::json!({ "status": "ok", "root": root.hex(), "files": files }),
        None => serde_json::Value::Null,
    };
    Ok(serde_json::json!({
        "root": {
            "manifest": report.manifest.root,
            "recomputed": report.recomputed_root,
            "ok": report.manifest.root == report.recomputed_root,
        },
        "sections": sections,
        "raw": raw,
    }))
}
