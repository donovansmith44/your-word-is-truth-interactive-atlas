pub mod canon;
pub mod refs;
pub mod time;
pub mod wire;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("year cannot be zero")]
    ZeroYear,
    #[error("time range is inverted (from > to)")]
    InvertedRange,
    #[error("invalid scripture reference: {0}")]
    BadRef(String),
}
