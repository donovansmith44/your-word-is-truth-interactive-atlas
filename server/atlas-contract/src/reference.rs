//! What a route's own path segment names, read as the reference it is.

use std::str::FromStr;

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;

use atlas_core::refs::{BookId, ScriptureRef, VerseId};
use atlas_graph_types::id::AnyNodeId;

use crate::error::ApiError;
use crate::graph_wire::decode_node_id;

/// A path segment read as the reference it names. A segment that names none is
/// `bad_ref`, quoted back exactly as the caller wrote it -- the one refusal every
/// route that takes a reference answers, whichever shape of reference it wanted.
pub struct Reference<T>(pub T);

impl<S: Send + Sync, T: FromStr> FromRequestParts<S> for Reference<T> {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, ApiError> {
        // A route whose path carries no single segment to read is this atlas's own
        // wiring mistake, not something the caller could have asked differently.
        let Path(raw) = Path::<String>::from_request_parts(parts, state).await.map_err(|_| ApiError::internal("a route that reads a reference declared no path segment to read it from"))?;
        T::from_str(&raw).map(Reference).map_err(|_| ApiError::bad_ref(&raw))
    }
}

/// That a segment names no reference of the shape wanted is the whole of what a
/// route needs to know: every one of them answers the same refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamesNoReference;

/// A reference naming one whole chapter, and nothing narrower or wider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChapterReference {
    pub book: BookId,
    pub chapter: u16,
}

impl FromStr for ChapterReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Chapter { book, chapter }) => Ok(ChapterReference { book, chapter }),
            _ => Err(NamesNoReference),
        }
    }
}

/// A reference naming one or more verses of a single chapter: one verse, or a
/// same-chapter span.
#[derive(Clone, Debug, PartialEq)]
pub struct VerseSpan(pub ScriptureRef);

impl FromStr for VerseSpan {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(span @ (ScriptureRef::Verse(_) | ScriptureRef::Passage { .. })) => Ok(VerseSpan(span)),
            _ => Err(NamesNoReference),
        }
    }
}

/// A reference naming exactly one verse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerseReference(pub VerseId);

impl FromStr for VerseReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        VerseId::parse_canonical(raw).map(VerseReference).map_err(|_| NamesNoReference)
    }
}

/// A reference naming one node of the graph, in the wire form every response hands
/// one back as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeReference(pub AnyNodeId);

impl FromStr for NodeReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        decode_node_id(raw).map(NodeReference).ok_or(NamesNoReference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chapter_reference_names_a_whole_chapter_and_nothing_else() {
        // Arrange
        let asked = ["EXO.14", "EXO", "EXO.14.21", "EXO.14.21-31", "nope"];
        // Act
        let read: Vec<Result<ChapterReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(
            read,
            vec![
                Ok(ChapterReference { book: BookId(1), chapter: 14 }),
                Err(NamesNoReference),
                Err(NamesNoReference),
                Err(NamesNoReference),
                Err(NamesNoReference),
            ]
        );
    }

    #[test]
    fn a_verse_span_names_one_verse_or_a_same_chapter_run_of_them() {
        // Arrange
        let asked = ["JOS.6.20", "JOS.6.20-21", "JOS.6", "JOS", "nope"];
        // Act
        let read: Vec<bool> = asked.iter().map(|raw| raw.parse::<VerseSpan>().is_ok()).collect();
        // Assert
        assert_eq!(read, vec![true, true, false, false, false]);
    }

    #[test]
    fn a_verse_reference_names_exactly_one_verse() {
        // Arrange
        let asked = ["JHN.3.16", "JHN.3", "JHN.3.16-18", "nope"];
        // Act
        let read: Vec<bool> = asked.iter().map(|raw| raw.parse::<VerseReference>().is_ok()).collect();
        // Assert
        assert_eq!(read, vec![true, false, false, false]);
    }

    #[test]
    fn a_node_reference_names_a_node_in_the_wire_form_a_response_hands_back() {
        // Arrange
        let asked = ["Event:ab_ur", "text-unit:JHN.3.16", "Event:", "not-even-a-colon-pair"];
        // Act
        let read: Vec<bool> = asked.iter().map(|raw| raw.parse::<NodeReference>().is_ok()).collect();
        // Assert
        assert_eq!(read, vec![true, true, false, false]);
    }
}
