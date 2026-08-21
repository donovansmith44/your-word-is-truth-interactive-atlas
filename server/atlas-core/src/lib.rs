pub mod canon;
pub mod catechism;
pub mod data;
pub mod history;
pub mod merge;
pub mod narrative;
pub mod refs;
pub mod scene;
pub mod time;
pub mod wire;
pub mod xrefs;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("year cannot be zero")]
    ZeroYear,
    #[error("time range is inverted (from > to)")]
    InvertedRange,
    #[error("invalid scripture reference: {0}")]
    BadRef(String),
    #[error("reading {path}: {source}")]
    Io { path: String, #[source] source: std::io::Error },
    #[error("parsing {path}: {source}")]
    Json { path: String, #[source] source: serde_json::Error },
}
