use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::openapi::{ContentBuilder, RefOr, ResponseBuilder, ResponsesBuilder};
use utoipa::{IntoResponses, PartialSchema};

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}

impl ApiError {
    pub fn bad_window() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "bad_window",
            message: "from/to must both be present, non-zero integers with from <= to".into(),
        }
    }

    pub fn bad_ref(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_ref", message: format!("invalid scripture reference: '{raw}'") }
    }

    pub fn not_found(what: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, code: "not_found", message: format!("{what} not found") }
    }

    pub fn bad_kind(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_kind", message: format!("unknown or missing edge kind: '{raw}'") }
    }

    pub fn bad_dir(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_dir", message: message.into() }
    }

    pub fn bad_corpus(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: "bad_corpus", message: format!("unknown corpus: '{raw}' (expected 'bible' or 'concord')") }
    }

    /// A server-side invariant this API cannot serve around. Distinct from
    /// `not_found`: the resource exists, and this project's own data about it is
    /// incomplete, which must never reach a reader as a blank or a guess.
    pub fn internal(message: &str) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, code: "internal", message: message.to_string() }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub error: ErrorInner,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorInner {
    pub code: String,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: ErrorInner { code: self.code.to_string(), message: self.message } };
        (self.status, Json(body)).into_response()
    }
}

impl IntoResponses for ApiError {
    fn responses() -> BTreeMap<String, RefOr<utoipa::openapi::Response>> {
        let json = || ContentBuilder::new().schema(Some(ErrorBody::schema())).build();
        ResponsesBuilder::new()
            .response("400", ResponseBuilder::new().description("bad_window | bad_ref | bad_kind | bad_dir | bad_corpus").content("application/json", json()))
            .response("404", ResponseBuilder::new().description("not_found").content("application/json", json()))
            .response("500", ResponseBuilder::new().description("internal").content("application/json", json()))
            .build()
            .into()
    }
}
