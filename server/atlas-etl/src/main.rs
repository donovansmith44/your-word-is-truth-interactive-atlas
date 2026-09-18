//! atlas-etl binary: the only place in this crate that WRITES to the
//! filesystem. Calls `atlas_etl::compile::compile` (reads `data/raw/` +
//! `data/curated/`, parses, merges, hard-validates -- M-C2 extracted this
//! into a library function so `atlas-graph`'s compile-step binary and
//! `atlas-server`'s `--build-from-raw` dev fallback can call the SAME
//! orchestration; see that module's own doc comment), then writes
//! `data/compiled/*.json` + `report.txt`. Run as `cargo run -p atlas-etl`
//! from `server/` (paths below are relative to that working directory).
//!
//! M-C2 DELETION EVENT (requirement 2, "delete the 5... with grep proof"):
//! `places.json`/`events.json`/`narratives.json`/`verses-kjv.json`/
//! `cross-refs.json` are no longer written here -- the graph's own node/
//! edge tables (place/event/era/polity/catechism adapters, `cites`) are
//! the only place this data lives once compiled (P1: one artifact). Every
//! OTHER compiled file this binary has always written keeps being written,
//! unchanged, straight off the SAME `AtlasData`/`Report` `compile()`
//! returns.

use std::path::{Path, PathBuf};

use anyhow::Result;

use atlas_etl::compile::compile;
use atlas_etl::report;

fn main() -> Result<()> {
    // Built from components (not a literal "../data/raw" string) so joined
    // paths use a consistent separator in error/report messages instead of
    // mixing '/' (as typed) and '\' (from PathBuf::join) on Windows.
    let data_dir: PathBuf = Path::new("..").join("data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let out = compile(&raw_dir, &curated_dir)?;
    let rpt = out.report;

    // DB-5: nothing is written under data/compiled any more -- the compile
    // (`atlas-graph-compile`) folds this same in-memory `AtlasData` into the
    // SQLite sections; this binary validates and reports.
    let text = report::write(&rpt);
    print!("{text}");

    Ok(())
}
