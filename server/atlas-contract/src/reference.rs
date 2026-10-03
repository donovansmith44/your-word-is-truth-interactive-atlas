//! What a route reads a reference as: every shape a path segment or a `ref`
//! parameter may name, and the one refusal a segment that names none answers.

use std::str::FromStr;

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;

use atlas_core::identity::{ElementId, NamesNoReference, NodeId};
use atlas_graph_types::edge::EdgeId;
use atlas_graph_types::graph::edge_hash;
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElementPosition {
    Node(AnyNodeId),
    Edge(EdgeId),
}

pub fn decode_element_id(raw: &str) -> Option<ElementPosition> {
    decode_node_id(raw).map(ElementPosition::Node).or_else(|| decode_edge_id(raw).map(ElementPosition::Edge))
}

fn decode_edge_id(raw: &str) -> Option<EdgeId> {
    let id = EdgeId(raw.to_string());
    id.recorded_kind()?;
    edge_hash(&id)?;
    Some(id)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PositionReference(pub ElementPosition);

impl FromStr for PositionReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        decode_element_id(raw).map(PositionReference).ok_or(NamesNoReference)
    }
}

pub const ELEMENT_ID_SEPARATOR: char = ',';

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AskedElement {
    pub asked: ElementId,
    pub id: ElementPosition,
}

impl AskedElement {
    fn read(raw: &str) -> Option<AskedElement> {
        decode_element_id(raw).map(|id| AskedElement {
            asked: match &id {
                ElementPosition::Node(_) => NodeId::asked(raw).into(),
                ElementPosition::Edge(edge) => edge.clone().into(),
            },
            id,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementIds(pub Vec<AskedElement>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ElementIdsRefused {
    NoId,
    Malformed(String),
}

impl FromStr for ElementIds {
    type Err = ElementIdsRefused;

    fn from_str(raw: &str) -> Result<Self, ElementIdsRefused> {
        if raw.is_empty() {
            return Err(ElementIdsRefused::NoId);
        }
        raw.split(ELEMENT_ID_SEPARATOR)
            .map(|asked| AskedElement::read(asked).ok_or_else(|| ElementIdsRefused::Malformed(asked.to_string())))
            .collect::<Result<Vec<_>, _>>()
            .map(ElementIds)
    }
}

impl<'de> serde::Deserialize<'de> for ElementIds {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(|refused: ElementIdsRefused| serde::de::Error::custom(format!("{refused:?}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_reference_names_a_node_in_the_wire_form_a_response_hands_back() {
        // Arrange
        let asked = ["Event:ab_ur", "text-unit:JHN.3.16", "Event:", "not-even-a-colon-pair"];
        // Act
        let read: Vec<bool> = asked.iter().map(|raw| raw.parse::<NodeReference>().is_ok()).collect();
        // Assert
        assert_eq!(read, vec![true, true, false, false]);
    }

    const AN_EDGE: &str = "LocatedAt:000102030405060708090a0b0c0d0e0f";
    const A_SYMMETRIC_EDGE: &str = "Analogue:000102030405060708090a0b0c0d0e0f";

    #[test]
    fn an_element_id_is_a_node_id_or_a_relation_named_edge_id() {
        // Arrange
        let asked = ["Event:ab_ur", AN_EDGE, A_SYMMETRIC_EDGE, "LocatedAt:00ff", "Nowhere:000102030405060708090a0b0c0d0e0f", "nope"];

        // Act
        let read: Vec<Option<ElementPosition>> = asked.iter().map(|raw| decode_element_id(raw)).collect();

        // Assert
        assert_eq!(
            read,
            vec![
                Some(ElementPosition::Node(decode_node_id("Event:ab_ur").unwrap())),
                Some(ElementPosition::Edge(EdgeId(AN_EDGE.to_string()))),
                Some(ElementPosition::Edge(EdgeId(A_SYMMETRIC_EDGE.to_string()))),
                None,
                None,
                None,
            ]
        );
    }

    #[test]
    fn a_position_reference_names_a_node_or_an_edge() {
        // Arrange
        let asked = ["text-unit:JHN.3.16", AN_EDGE, "nope"];

        // Act
        let read: Vec<Result<PositionReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();

        // Assert
        assert_eq!(
            read,
            vec![
                Ok(PositionReference(ElementPosition::Node(decode_node_id("text-unit:JHN.3.16").unwrap()))),
                Ok(PositionReference(ElementPosition::Edge(EdgeId(AN_EDGE.to_string())))),
                Err(NamesNoReference),
            ]
        );
    }

    #[test]
    fn a_list_of_element_ids_is_read_between_separators_and_refused_whole() {
        // Arrange
        let asked = [format!("Event:ab_ur,{AN_EDGE}"), String::new(), "Event:ab_ur,nope".to_string(), "Event:ab_ur,".to_string()];

        // Act
        let read: Vec<Result<ElementIds, ElementIdsRefused>> = asked.iter().map(|raw| raw.parse()).collect();

        // Assert
        assert_eq!(
            read,
            vec![
                Ok(ElementIds(vec![
                    AskedElement { asked: NodeId::asked("Event:ab_ur").into(), id: ElementPosition::Node(decode_node_id("Event:ab_ur").unwrap()) },
                    AskedElement { asked: EdgeId(AN_EDGE.to_string()).into(), id: ElementPosition::Edge(EdgeId(AN_EDGE.to_string())) },
                ])),
                Err(ElementIdsRefused::NoId),
                Err(ElementIdsRefused::Malformed("nope".to_string())),
                Err(ElementIdsRefused::Malformed(String::new())),
            ]
        );
    }
}
