//! `CliError`: the one error type every command function returns. Fixes
//! the CONTRACT's five-class taxonomy (CONTRACT.md "Error taxonomy") --
//! one fixed nonzero exit code per class, one fixed message shape
//! (`atlas: error (<code>): <WHAT> -- <WHY> -- <WHAT TO DO>`), printed to
//! stderr by `main.rs`'s own top-level dispatch, never inline by a command
//! function itself (one rendering site, so the shape can never drift
//! between commands).

use std::fmt;

#[derive(Debug)]
pub enum CliError {
    /// The command line itself is unparseable -- unknown subcommand,
    /// unknown flag, a missing required value, extra positional args.
    BadUsage { what: String, why: String, do_: String },
    /// A ref/id argument does not parse against its own grammar.
    BadRef { what: String, why: String, do_: String },
    /// The ref/id parses cleanly but names nothing this graph has.
    NotFound { what: String, why: String, do_: String },
    /// `graph.bin` or a required compiled JSON file is missing, unreadable,
    /// or fails to parse at startup.
    DataLoadFailed { what: String, why: String, do_: String },
    /// The command ran correctly end-to-end but its own entire answer is
    /// zero rows (CONTRACT.md: distinct from `NotFound` -- the id/ref is
    /// real, the *question* about it just has no answer right now).
    EmptyResult { what: String, why: String, do_: String },
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

    /// The taxonomy's own fixed class name, exactly as CONTRACT.md's table
    /// names it -- printed in the `(<code>)` slot and used by tests to
    /// assert which class fired.
    pub fn code(&self) -> &'static str {
        match self {
            CliError::BadUsage { .. } => "bad_usage",
            CliError::BadRef { .. } => "bad_ref",
            CliError::NotFound { .. } => "not_found",
            CliError::DataLoadFailed { .. } => "data_load_failed",
            CliError::EmptyResult { .. } => "empty_result",
        }
    }

    /// The fixed exit code for this class (CONTRACT.md's table).
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::EmptyResult { .. } => 1,
            CliError::BadRef { .. } => 2,
            CliError::NotFound { .. } => 3,
            CliError::BadUsage { .. } => 4,
            CliError::DataLoadFailed { .. } => 5,
        }
    }

    fn parts(&self) -> (&str, &str, &str) {
        match self {
            CliError::BadUsage { what, why, do_ }
            | CliError::BadRef { what, why, do_ }
            | CliError::NotFound { what, why, do_ }
            | CliError::DataLoadFailed { what, why, do_ }
            | CliError::EmptyResult { what, why, do_ } => (what, why, do_),
        }
    }

    /// BIBEX-1 (--json mode, "ERRORS STAY FAIL-LOUD, MACHINE-READABLY"):
    /// the SAME taxonomy this type already carries, rendered as
    /// `{"error":{"code","message","hint"}}` instead of the plain-mode
    /// `atlas: error (<code>): <what> -- <why> -- <what to do>` line --
    /// ONE source of truth (this type's own fields), TWO renderings
    /// (`Display` above for plain mode, this for `--json`), never a
    /// second, independently-maintained error text. `message` folds
    /// `what`/`why` together (the WHAT and the WHY read as one sentence in
    /// plain mode too, joined by " -- "); `hint` is `do_` (the WHAT TO DO)
    /// verbatim -- the CONTRACT's own declared two-field shape.
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
