//! `gen_sources` binary: reads `data/curated/sources.toml`, validates it
//! (structurally, then 1:1 against LICENSES.md's own "## Per-source
//! table" -- `atlas_etl::sources`'s own doc comment has the full
//! reasoning), and writes `data/compiled/sources.json`. Run as
//! `cargo run -p atlas-etl --bin gen_sources` from `server/` (paths below
//! are relative to that working directory -- the same convention
//! `atlas-etl`'s own primary binary, `main.rs`, already uses).
//!
//! Deliberately its OWN binary, never folded into `main.rs`'s own
//! `compile()` pipeline: batch-s-brief.md's own finalization block
//! requires any Rust helper this batch adds stay OUTSIDE the graph
//! pipeline, so `graph.bin`/`data/exports/` stay byte-untouched by
//! anything here -- this binary never reads `data/raw/`, never touches
//! `AtlasData`, and writes exactly one new file.
//!
//! Lives at `src/bins/gen_sources.rs` (note the "s"), NOT Cargo's
//! auto-discovered `src/bin/gen_sources.rs` -- this repo's own root
//! `.gitignore` carries a broad `**/bin/` rule (meant for the Blazor
//! client's own .NET build-output `bin/` directories), which would
//! silently exclude a real source file at that conventional path. Same
//! disclosed workaround `atlas-graph/Cargo.toml`'s own `[[bin]]` entry
//! for `compile_graph.rs` already established -- declared explicitly in
//! `atlas-etl/Cargo.toml`'s own `[[bin]]` table rather than relying on
//! auto-discovery.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

fn main() -> Result<()> {
    // Built from components, same as main.rs's own data_dir -- consistent
    // separators in error messages instead of mixing '/' (as typed) and
    // '\' (from PathBuf::join) on Windows.
    let repo_root = Path::new("..");
    let sources_toml_path = repo_root.join("data").join("curated").join("sources.toml");
    let licenses_path = repo_root.join("LICENSES.md");
    let compiled_path = repo_root.join("data").join("compiled").join("sources.json");

    let toml_input = fs::read_to_string(&sources_toml_path)
        .with_context(|| format!("reading {}", sources_toml_path.display()))?;
    let doc = atlas_etl::sources::parse_sources(&toml_input)?;
    atlas_etl::sources::validate_structure(&doc)?;

    let licenses_md =
        fs::read_to_string(&licenses_path).with_context(|| format!("reading {}", licenses_path.display()))?;
    atlas_etl::sources::validate_against_licenses(&doc, &licenses_md)?;

    let json = serde_json::to_string_pretty(&doc).context("serializing sources.json")?;
    if let Some(parent) = compiled_path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(&compiled_path, json).with_context(|| format!("writing {}", compiled_path.display()))?;

    println!(
        "gen_sources: wrote {} categories, {} sources to {} (validated 1:1 against LICENSES.md's per-source table)",
        doc.categories.len(),
        doc.sources.len(),
        compiled_path.display()
    );
    Ok(())
}
