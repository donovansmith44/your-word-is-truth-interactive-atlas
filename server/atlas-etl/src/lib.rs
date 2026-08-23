//! atlas-etl library: pure parsers + validation + report formatting, plus
//! (M-C2) the reusable compile orchestration.
//!
//! Every module except `compile` is `&str`-in / data-out with no
//! filesystem or network I/O (per the project's ETL design: "ETL does no
//! networking" and parsers are pure). `compile::compile` is the one
//! disclosed exception (module doc comment there has the full reasoning):
//! `main.rs`'s own pre-M-C2 compute phase -- reading `data/raw/`+
//! `data/curated/`, calling into these pure parsers, merging, hard-
//! validating -- relocated here so `atlas-graph`'s compile-step binary and
//! `atlas-server`'s `--build-from-raw` dev fallback can call the SAME
//! orchestration `main.rs` calls, rather than a second copy. `main.rs`
//! remains the only place that WRITES (`data/compiled/*.json` +
//! report.txt) -- it calls `compile::compile`, then writes only the
//! surviving files, reading every value straight off the returned
//! `AtlasData`/`Report`.

pub mod catechism_map;
pub mod compile;
pub mod curated;
pub mod geo;
pub mod kjv;
pub mod osis;
pub mod people;
pub mod polities;
pub mod report;
pub mod theographic;
pub mod validate;
pub mod xrefs;
