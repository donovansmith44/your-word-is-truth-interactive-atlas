//! DB-4b: `bibex verify [--section <name>]` -- spec §3.5's verification
//! on demand: recompute the logical hash of each cached section from its
//! tables, the transport hash of each committed blob, and the root from
//! the manifest; print each against the manifest; non-zero exit
//! (`integrity_failed`, 6) on any mismatch. The offline tablet's "verified
//! update" primitive and CI's integrity check after `graph.bin` is gone.
//!
//! Reads `<data_dir>/manifest.toml` and `<data_dir>/sections/*.sqlite.zst`;
//! unpacks a cache miss into `<data_dir>/../cache/sections/` through the
//! same `CommittedZstdSource` the server uses (spec §2.4), so a corrupt
//! blob is refused here exactly as it would be at startup (spec §11).

use std::path::Path;

use atlas_graph::sections::Section;
use atlas_graph::sqlite::blob::sha256_hex_of_file;
use atlas_graph::sqlite::logical::{logical_dump_of_db, logical_hash};
use atlas_graph::sqlite::manifest::{read_manifest, root_of, Manifest};
use atlas_graph::sqlite::open_read_only;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout, SectionSource};

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
}

impl Report {
    pub fn failures(&self) -> Vec<String> {
        self.sections.iter().flat_map(|s| s.failures.iter().cloned()).collect()
    }
    pub fn checks(&self) -> usize {
        // per section: transport + logical (+ schema, counted with logical); plus the root
        1 + self.sections.len() * 2
    }
}

const DO: &str = "recompile (cargo run -p atlas-graph --bin atlas-graph-compile, from server/) or restore data/compiled from git; a tampered or truncated section must never be served";

fn section_named(name: &str) -> Option<Section> {
    Section::MANIFEST_ORDER.iter().copied().find(|s| s.name() == name)
}

/// Runs every check; the caller decides how to render and whether the
/// failures become an `integrity_failed` error.
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
    Ok(Report { manifest, recomputed_root, sections })
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
    out
}

fn fail(report: &Report) -> Option<CliError> {
    let failures = report.failures();
    if failures.is_empty() {
        return None;
    }
    Some(CliError::integrity_failed(
        format!("{} of {} checks failed", failures.len(), report.checks()),
        failures.join("; "),
        DO,
    ))
}

pub fn run(data_dir: &Path, only: Option<&str>) -> Result<String, CliError> {
    let report = check(data_dir, only)?;
    let text = render(&report);
    match fail(&report) {
        Some(e) => Err(e),
        None => Ok(text),
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
    Ok(serde_json::json!({
        "root": {
            "manifest": report.manifest.root,
            "recomputed": report.recomputed_root,
            "ok": report.manifest.root == report.recomputed_root,
        },
        "sections": sections,
    }))
}
