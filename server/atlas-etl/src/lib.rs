//! atlas-etl library: pure parsers + validation + report formatting.
//!
//! Every module here is `&str`-in / data-out with no filesystem or network
//! I/O (per the project's ETL design: "ETL does no networking" and parsers
//! are pure). `main.rs` is the only place that touches the filesystem: it
//! reads `data/raw/` and `data/curated/`, calls into these modules, merges
//! the results, validates, and writes `data/compiled/*.json` + report.txt.

pub mod catechism_map;
pub mod curated;
pub mod geo;
pub mod kjv;
pub mod osis;
pub mod polities;
pub mod report;
pub mod theographic;
pub mod validate;
pub mod xrefs;
