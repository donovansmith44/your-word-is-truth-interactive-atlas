pub mod canon;
pub mod catechism;
pub mod chronology;
pub mod data;
pub mod event_merge;
pub mod history;
pub mod label;
pub mod merge;
pub mod narrative;
pub mod nt_calibration;
pub mod refs;
pub mod scene;
pub mod scene_source;
pub mod sources;
pub mod time;
pub mod translation;
pub mod wire;
pub mod xrefs;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("year cannot be zero")]
    ZeroYear,
    #[error("time range is inverted (from > to)")]
    InvertedRange,
    #[error("a span records one of its ends and not the other")]
    OneEndedSpan,
    #[error("invalid scripture reference: {0}")]
    BadRef(String),
    #[error("unknown translation '{0}' (this atlas only compiles KJV today)")]
    UnknownTranslation(String),
}
