//! The API's one error type. Every handler that can fail returns
//! `Result<_, ApiError>`; `IntoResponse` renders it as
//! `{"error":{"code":"...","message":"..."}}` with the matching HTTP status.
//!
//! Controller ruling: axum extractor rejections must never reach the client
//! as axum's own default rejection body — handlers use lenient extractors
//! (`Query<HashMap<String, String>>`, plain `Path<String>`) that cannot
//! themselves reject on the inputs this API cares about, and turn "missing /
//! unparseable / out of shape" into one of the typed `ApiError`s below
//! instead.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    /// `from`/`to` missing, unparseable, zero, or inverted (ruling 1).
    pub fn bad_window() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_window",
            message: "from/to must both be present, non-zero integers with from <= to".into(),
        }
    }

    /// A scripture ref (`ref`, `{cref}`, or `{vref}`) that is missing or
    /// structurally malformed — not merely out-of-canon (see the
    /// `ruling-3-policy` doc comments on `handlers::scene_scripture` /
    /// `handlers::chapter` / `handlers::verse` for what counts as which).
    pub fn bad_ref(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_ref", message: format!("invalid scripture reference: '{raw}'") }
    }

    /// A syntactically fine identifier (place id, or a verse ref whose text
    /// this atlas doesn't have) that names no resource that exists.
    pub fn not_found(what: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, code: "not_found", message: format!("{what} not found") }
    }

    /// Batch M-A: `GET /api/node/{id}/edges?kind=` names a `kind` label that
    /// doesn't match any (relation, direction) or symmetric-relation label
    /// in graph-types' own relation manifest (`RelationId`/`SymRelationId`),
    /// or omits `kind` entirely -- same "typed error, not axum's default
    /// rejection body" discipline as `bad_ref`/`bad_window`.
    pub fn bad_kind(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_kind", message: format!("unknown or missing edge kind: '{raw}'") }
    }

    /// Fix round 1, I1: `GET /api/text?scope=chapter&dir=backward` -- a
    /// parameter combination with no honest meaning (a chapter-scoped
    /// window's bounds are already fully determined by the chapter itself;
    /// there is no direction left to walk) -- rejected explicitly rather
    /// than silently accepted-and-ignored or (the bug this replaces)
    /// silently misapplied to serve the wrong chapter's tail.
    pub fn bad_dir(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_dir", message: message.into() }
    }
}

#[derive(Serialize)]
struct ErrorBody<'a> {
    error: ErrorInner<'a>,
}

#[derive(Serialize)]
struct ErrorInner<'a> {
    code: &'a str,
    message: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: ErrorInner { code: self.code, message: &self.message } };
        (self.status, Json(body)).into_response()
    }
}
