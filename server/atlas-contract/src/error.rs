use std::collections::BTreeMap;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use utoipa::openapi::{ContentBuilder, Ref, RefOr, ResponseBuilder, ResponsesBuilder};
use utoipa::{IntoResponses, ToSchema};

use atlas_graph::window::WindowDir;

use crate::query;
use crate::wire::{Corpus, TextScope};

atlas_graph_types::vocabulary! {
    /// Why this API refused a request: the one word a consumer branches on, beside
    /// which the message is prose for a reader.
    ErrorCode {
        BadRef => "bad_ref",
        BadKind => "bad_kind",
        BadScope => "bad_scope",
        BadWindow => "bad_window",
        BadCorpus => "bad_corpus",
        BadDir => "bad_dir",
        NotFound => "not_found",
        Internal => "internal",
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: ErrorCode,
    pub message: String,
}

impl ApiError {
    pub fn bad_window() -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: ErrorCode::BadWindow,
            message: "from/to must both be present, non-zero integers with from <= to".into(),
        }
    }

    pub fn bad_ref(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: ErrorCode::BadRef, message: format!("invalid scripture reference: '{raw}'") }
    }

    pub fn not_found(what: &str) -> Self {
        Self { status: StatusCode::NOT_FOUND, code: ErrorCode::NotFound, message: format!("{what} not found") }
    }

    pub fn bad_kind(raw: &str) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: ErrorCode::BadKind, message: format!("unknown or missing edge kind: '{raw}'") }
    }

    /// The refusals that are not about one word but about a combination of them: a
    /// window whose scope and direction cannot both be honoured. They carry
    /// `unknown_dir`'s code because the direction is what the caller must change.
    pub fn bad_dir(message: impl Into<String>) -> Self {
        Self { status: StatusCode::BAD_REQUEST, code: ErrorCode::BadDir, message: message.into() }
    }

    pub fn unknown_dir(raw: &str) -> Self {
        let [leading @ .., last] = WindowDir::ALL.map(WindowDir::name);
        unknown_word(ErrorCode::BadDir, query::DIR, raw, &leading, last)
    }

    pub fn bad_scope(raw: &str) -> Self {
        let [leading @ .., last] = TextScope::ALL.map(TextScope::name);
        unknown_word(ErrorCode::BadScope, query::SCOPE, raw, &leading, last)
    }

    pub fn bad_corpus(raw: &str) -> Self {
        let [leading @ .., last] = Corpus::ALL.map(Corpus::name);
        unknown_word(ErrorCode::BadCorpus, query::CORPUS, raw, &leading, last)
    }

    /// A server-side invariant this API cannot serve around. Distinct from
    /// `not_found`: the resource exists, and this project's own data about it is
    /// incomplete, which must never reach a reader as a blank or a guess.
    pub fn internal(message: &str) -> Self {
        Self { status: StatusCode::INTERNAL_SERVER_ERROR, code: ErrorCode::Internal, message: message.to_string() }
    }
}

/// A query parameter whose word names no member of its own closed vocabulary. The
/// words it could have been are read off that vocabulary, so a member added to one
/// can never be missing from the refusal that lists it.
fn unknown_word(code: ErrorCode, parameter: &str, raw: &str, leading: &[&str], last: &str) -> ApiError {
    ApiError {
        status: StatusCode::BAD_REQUEST,
        code,
        message: format!("unknown {parameter}: '{raw}' (expected {})", one_of(leading, last)),
    }
}

/// A list of accepted words as this API's own refusals read it: `'a'`, `'a' or 'b'`,
/// `'a', 'b' or 'c'`. It is given the last word apart from the words before it,
/// which is a list no vocabulary can hand it empty.
fn one_of(leading: &[&str], last: &str) -> String {
    let quoted = |word: &str| format!("'{word}'");
    match leading {
        [] => quoted(last),
        _ => format!("{} or {}", leading.iter().map(|word| quoted(word)).collect::<Vec<_>>().join(", "), quoted(last)),
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
    pub code: ErrorCode,
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorBody { error: ErrorInner { code: self.code, message: self.message } };
        (self.status, Json(body)).into_response()
    }
}

/// Declares one route's refusals: the 400 codes its own parameters can produce, and
/// the 404 and 500 every route shares. Written as a macro so the five sets read as
/// one table, and placed above them because that is where a macro must be declared.
macro_rules! refusals {
    ($( $(#[doc = $doc:expr])* $name:ident { $($code:ident),* } )+) => {$(
        $(#[doc = $doc])*
        pub struct $name;

        impl IntoResponses for $name {
            fn responses() -> BTreeMap<String, RefOr<utoipa::openapi::Response>> {
                refusal_responses(&[$(ErrorCode::$code),*])
            }
        }
    )+};
}

refusals! {
    /// A route that refuses nothing it is asked with: every parameter it takes is a
    /// path segment, and one that names nothing is `not_found` rather than unreadable.
    NoRefusals {}
    /// A route that reads a reference.
    ReferenceRefusals { BadRef }
    /// A route that reads a span of years.
    WindowRefusals { BadWindow }
    /// A route that reads a reference and which of a node's frontiers to answer.
    FrontierRefusals { BadRef, BadKind }
    /// A route that reads a reference and every word a reading window is asked with.
    ReadingWindowRefusals { BadRef, BadDir, BadScope, BadCorpus }
}

/// A route that can refuse no word publishes no 400 at all, so the document never
/// advertises a refusal a route cannot make.
fn refusal_responses(codes: &[ErrorCode]) -> BTreeMap<String, RefOr<utoipa::openapi::Response>> {
    let json = || ContentBuilder::new().schema(Some(Ref::from_schema_name(ErrorBody::name()))).build();
    let mut responses = ResponsesBuilder::new();
    if !codes.is_empty() {
        responses = responses.response("400", ResponseBuilder::new().description(unreadable(codes)).content("application/json", json()));
    }
    responses
        .response("404", ResponseBuilder::new().description(NOTHING_THERE).content("application/json", json()))
        .response("500", ResponseBuilder::new().description(INCOMPLETE).content("application/json", json()))
        .build()
        .into()
}

const UNREADABLE: &str = "The reference, window or query could not be read; the body's `code` ";
const NOTHING_THERE: &str = "Nothing in this atlas answers to that reference.";
const INCOMPLETE: &str = "The request was well formed but this atlas's own data for it is incomplete.";

/// One route's 400 sentence. A route with a single code names it outright; a route
/// with several says which of them the body carries.
fn unreadable(codes: &[ErrorCode]) -> String {
    let quoted: Vec<String> = codes.iter().map(|code| format!("`{}`", code.name())).collect();
    match quoted.as_slice() {
        [only] => format!("{UNREADABLE}is {only}."),
        _ => format!("{UNREADABLE}says which ({}).", quoted.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_of_accepted_words_reads_as_this_api_writes_one() {
        // Arrange
        let vocabularies: [(&[&str], &str); 3] = [(&[], "verse"), (&["bible"], "concord"), (&["water", "mountain"], "region")];
        // Act
        let listed: Vec<String> = vocabularies.iter().map(|(leading, last)| one_of(leading, last)).collect();
        // Assert
        assert_eq!(listed, vec!["'verse'", "'bible' or 'concord'", "'water', 'mountain' or 'region'"]);
    }

    #[test]
    fn a_route_with_one_refusal_names_it_and_a_route_with_several_says_which() {
        // Arrange
        let sets: [&[ErrorCode]; 2] = [&[ErrorCode::BadWindow], &[ErrorCode::BadRef, ErrorCode::BadKind]];
        // Act
        let sentences: Vec<String> = sets.iter().map(|codes| unreadable(codes)).collect();
        // Assert
        assert_eq!(
            sentences,
            vec![
                "The reference, window or query could not be read; the body's `code` is `bad_window`.".to_string(),
                "The reference, window or query could not be read; the body's `code` says which (`bad_ref`, `bad_kind`).".to_string(),
            ]
        );
    }

    #[test]
    fn a_route_that_refuses_no_word_publishes_no_bad_request_at_all() {
        // Act
        let refusable = ReferenceRefusals::responses();
        let unrefusable = NoRefusals::responses();
        // Assert
        assert_eq!(refusable.keys().collect::<Vec<_>>(), vec!["400", "404", "500"]);
        assert_eq!(unrefusable.keys().collect::<Vec<_>>(), vec!["404", "500"]);
    }
}
