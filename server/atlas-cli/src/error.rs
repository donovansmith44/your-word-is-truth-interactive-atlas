//! One fixed exit code and one fixed message shape per class, rendered in a single place so
//! the shape cannot drift between commands.

use std::fmt;

#[derive(Debug)]
pub enum CliError {
    BadUsage { what: String, why: String, do_: String },
    /// A ref or id argument does not parse against its grammar.
    BadRef { what: String, why: String, do_: String },
    /// The ref or id parses cleanly but names nothing this graph has.
    NotFound { what: String, why: String, do_: String },
    /// The sections are missing, unreadable or refused at open, before any command runs.
    DataLoadFailed { what: String, why: String, do_: String },
    /// The command ran correctly but its whole answer is zero rows: the id is real, the
    /// question about it simply has no answer.
    EmptyResult { what: String, why: String, do_: String },
    /// The data on disk disagrees with its manifest: a hash mismatch, a required blob
    /// missing, or a root that does not recompute.
    IntegrityFailed { what: String, why: String, do_: String },
}

impl CliError {
    pub fn bad_usage(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::BadUsage { what: what.into(), why: why.into(), do_: do_.into() }
    }
    pub fn bad_ref(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::BadRef { what: what.into(), why: why.into(), do_: do_.into() }
    }
    pub fn not_found(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::NotFound { what: what.into(), why: why.into(), do_: do_.into() }
    }
    pub fn data_load_failed(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::DataLoadFailed { what: what.into(), why: why.into(), do_: do_.into() }
    }
    pub fn empty_result(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::EmptyResult { what: what.into(), why: why.into(), do_: do_.into() }
    }
    pub fn integrity_failed(what: impl Into<String>, why: impl Into<String>, do_: impl Into<String>) -> Self {
        CliError::IntegrityFailed { what: what.into(), why: why.into(), do_: do_.into() }
    }

    pub fn code(&self) -> &'static str {
        match self {
            CliError::BadUsage { .. } => "bad_usage",
            CliError::BadRef { .. } => "bad_ref",
            CliError::NotFound { .. } => "not_found",
            CliError::DataLoadFailed { .. } => "data_load_failed",
            CliError::EmptyResult { .. } => "empty_result",
            CliError::IntegrityFailed { .. } => "integrity_failed",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::EmptyResult { .. } => 1,
            CliError::BadRef { .. } => 2,
            CliError::NotFound { .. } => 3,
            CliError::BadUsage { .. } => 4,
            CliError::DataLoadFailed { .. } => 5,
            CliError::IntegrityFailed { .. } => 6,
        }
    }

    fn parts(&self) -> (&str, &str, &str) {
        match self {
            CliError::BadUsage { what, why, do_ }
            | CliError::BadRef { what, why, do_ }
            | CliError::NotFound { what, why, do_ }
            | CliError::DataLoadFailed { what, why, do_ }
            | CliError::EmptyResult { what, why, do_ }
            | CliError::IntegrityFailed { what, why, do_ } => (what, why, do_),
        }
    }

    /// The same fields as the plain rendering, so there is never a second error text:
    /// `message` folds the what and the why, `hint` is the what-to-do verbatim.
    pub fn to_json(&self) -> serde_json::Value {
        let (what, why, do_) = self.parts();
        serde_json::json!({
            "error": {
                "code": self.code(),
                "message": format!("{what} -- {why}"),
                "hint": do_,
            }
        })
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (what, why, do_) = self.parts();
        write!(f, "atlas: error ({}): {} -- {} -- {}", self.code(), what, why, do_)
    }
}

impl std::error::Error for CliError {}
