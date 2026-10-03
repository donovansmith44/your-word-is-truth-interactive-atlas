//! Reads the curated sources file, validates it, and writes the compiled sources document:
//! `cargo run -p atlas-etl --bin gen_sources` from `server/`. It lives under `src/bins/`, not Cargo's
//! auto-discovered `src/bin/`, because this repo's `.gitignore` excludes every `bin/` directory.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

const USAGE: &str = "usage: gen_sources [OUTPUT_PATH]";
const MISUSE: i32 = 2;

fn main() -> Result<()> {
    let repo_root = Path::new("..");
    let sources_toml_path = repo_root.join("data").join("curated").join("sources.toml");
    let licenses_path = repo_root.join("LICENSES.md");
    let compiled_path = match Vec::from_iter(std::env::args().skip(1)).as_slice() {
        [] => repo_root.join("data").join("compiled").join("sources.json"),
        [path] => PathBuf::from(path),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(MISUSE);
        }
    };

    let toml_input = fs::read_to_string(&sources_toml_path)
        .with_context(|| format!("reading {}", sources_toml_path.display()))?;
    let doc = atlas_etl::sources::admit_sources(&toml_input)?.into_document();

    let licenses_md =
        fs::read_to_string(&licenses_path).with_context(|| format!("reading {}", licenses_path.display()))?;
    atlas_etl::sources::validate_against_licenses(&doc, &licenses_md)?;

    let json = serde_json::to_string_pretty(&doc).context("serializing sources.json")?;
    if let Some(parent) = compiled_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&compiled_path, json).with_context(|| format!("writing {}", compiled_path.display()))?;

    println!(
        "gen_sources: wrote {} categories, {} sources, {} provenance rows to {} (validated 1:1 against LICENSES.md's per-source table)",
        doc.categories.len(),
        doc.sources.len(),
        doc.provenances.len(),
        compiled_path.display()
    );
    Ok(())
}
