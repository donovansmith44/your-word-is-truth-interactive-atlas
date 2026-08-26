pub mod canon;
pub mod catechism;
pub mod chronology;
pub mod data;
pub mod event_merge;
pub mod history;
pub mod merge;
pub mod narrative;
pub mod nt_calibration;
pub mod refs;
pub mod scene;
pub mod sources;
pub mod time;
pub mod translation;
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
    /// Batch T (events as narrative nodes): a `PASSAGE`'s `content` is a
    /// MAPPING translation -> verse set (the owner's own internal
    /// representation, verbatim in batch-t-brief.md: "this set of passages
    /// with their titles maps to a mapping of translation to a set of
    /// verses, so that we can expand into different translations and keep
    /// mappings the same"). KJV is the only translation this app compiles
    /// today, but the indirection is a REAL lookup, not a comment -- asking
    /// for any other translation code fails loud (`translation::resolve`),
    /// never silently falls back to KJV or panics. See that module's own
    /// doc comment.
    #[error("unknown translation '{0}' (this atlas only compiles KJV today)")]
    UnknownTranslation(String),
}
