//! Pure parsers, validation and report formatting, plus the reusable compile orchestration. Every
//! module except `compile` is `&str`-in / data-out with no filesystem or network I/O, and the binary
//! is the only place that writes.

pub mod brainfuel;
pub mod catechism_map;
pub mod compile;
pub mod concord;
pub mod curated;
pub mod easton;
pub mod geo;
pub mod kjv;
pub mod kretzmann;
pub mod lexicon;
pub mod osis;
pub mod people;
pub mod people_groups;
pub mod polities;
pub mod red_letter;
pub mod report;
pub mod sources;
pub mod theographic;
pub mod validate;
pub mod xrefs;
