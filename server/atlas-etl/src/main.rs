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

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use atlas_etl::compile::compile;
use atlas_etl::report;

fn main() -> Result<()> {
    // Built from components (not a literal "../data/raw" string) so joined
    // paths use a consistent separator in error/report messages instead of
    // mixing '/' (as typed) and '\' (from PathBuf::join) on Windows.
    let data_dir: PathBuf = Path::new("..").join("data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");
    let compiled_dir = data_dir.join("compiled");

    let out = compile(&raw_dir, &curated_dir)?;
    let data = out.data;
    let rpt = out.report;
    let place_history_list = out.place_history_list;
    let place_name_alias_list = out.place_name_alias_list;

    // --- write compiled output ------------------------------------------
    fs::create_dir_all(&compiled_dir).with_context(|| format!("creating {}", compiled_dir.display()))?;
    write_json(&compiled_dir.join("canon.json"), &data.canon)?;
    // M-C2 DELETION EVENT: places.json/events.json/narratives.json/
    // verses-kjv.json/cross-refs.json retire -- `data.places`/`.events`/
    // `.narratives`/`.verses`/`.cross_refs` stay populated in THIS process
    // (this binary's own `validate::run`, and every graph adapter reading
    // this same `AtlasData` when this function is called from
    // `atlas-graph-compile`/`--build-from-raw`, still need them), but are
    // no longer written to disk -- no surface reads these five files
    // anymore (grep-proven in batch-mc2-report.md's own deletion
    // inventory), mirroring eras.json's own M-C retirement exactly.
    write_json(&compiled_dir.join("books-meta.json"), &data.books_meta)?;
    write_json(&compiled_dir.join("chronology-anchors.json"), &data.chronology_anchors)?;
    write_json(&compiled_dir.join("book-narration-windows.json"), &data.book_narration_windows)?;

    write_json(&compiled_dir.join("polities.json"), &data.polities)?;
    write_json(&compiled_dir.join("landmarks.json"), &data.landmarks)?;
    write_json(&compiled_dir.join("place-history.json"), &place_history_list)?;
    write_json(&compiled_dir.join("place-names-kjv.json"), &place_name_alias_list)?;
    write_json(&compiled_dir.join("land-mask.json"), &data.land_mask)?;
    write_json(&compiled_dir.join("catechism.json"), &data.catechism)?;

    let text = report::write(&rpt);
    fs::write(compiled_dir.join("report.txt"), &text).context("writing data/compiled/report.txt")?;
    print!("{text}");

    Ok(())
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).with_context(|| format!("serializing {}", path.display()))?;
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}
