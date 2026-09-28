//! The only place in this crate that touches the filesystem. Run as `cargo run -p atlas-etl` from
//! `server/`: the paths below are relative to that working directory.

use std::path::{Path, PathBuf};

use anyhow::Result;

use atlas_etl::compile::compile;
use atlas_etl::report;

fn main() -> Result<()> {
    // Built from components rather than one literal path so joined paths carry a consistent separator
    // in error and report messages, instead of mixing the typed '/' with a joined '\' on Windows.
    let data_dir: PathBuf = Path::new("..").join("data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let out = compile(&raw_dir, &curated_dir)?;
    let rpt = out.report;

    // Nothing is written under the compiled directory: the graph compile folds this same in-memory `AtlasData` into
    // the SQLite sections, and this binary validates and reports.
    let text = report::write(&rpt);
    print!("{text}");

    Ok(())
}
