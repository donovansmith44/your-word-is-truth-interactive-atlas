//! These functions MINT the artifact's one string spelling for an id: kind name, colon, raw.
//! The raw part may itself contain colons, so the parse side splits at the FIRST colon and
//! hands back the whole remainder, which makes the round trip total for every raw.

use crate::edge::EdgeId;
use crate::id::{AnyNodeId, NodeKind, Position};

use super::CanonError;

/// The canonical name of a node kind: its `Debug` name, so the encoding
pub fn node_kind_str(k: NodeKind) -> &'static str {
    match k {
        NodeKind::TextUnit => "TextUnit",
        NodeKind::Container => "Container",
        NodeKind::Event => "Event",
        NodeKind::Narrative => "Narrative",
        NodeKind::Place => "Place",
        NodeKind::Person => "Person",
        NodeKind::Anchor => "Anchor",
        NodeKind::Era => "Era",
        NodeKind::Polity => "Polity",
        NodeKind::CatechismItem => "CatechismItem",
        NodeKind::Source => "Source",
        NodeKind::Translation => "Translation",
        NodeKind::PeopleGroup => "PeopleGroup",
        NodeKind::CommentaryItem => "CommentaryItem",
        NodeKind::LexiconEntry => "LexiconEntry",
    }
}

/// `path` is where the caller sits, so a bad kind inside a node id
pub fn parse_node_kind(s: &str, path: &str) -> Result<NodeKind, CanonError> {
    match s {
        "TextUnit" => Ok(NodeKind::TextUnit),
        "Container" => Ok(NodeKind::Container),
        "Event" => Ok(NodeKind::Event),
        "Narrative" => Ok(NodeKind::Narrative),
        "Place" => Ok(NodeKind::Place),
        "Person" => Ok(NodeKind::Person),
        "Anchor" => Ok(NodeKind::Anchor),
        "Era" => Ok(NodeKind::Era),
        "Polity" => Ok(NodeKind::Polity),
        "CatechismItem" => Ok(NodeKind::CatechismItem),
        "Source" => Ok(NodeKind::Source),
        "Translation" => Ok(NodeKind::Translation),
        "PeopleGroup" => Ok(NodeKind::PeopleGroup),
        "CommentaryItem" => Ok(NodeKind::CommentaryItem),
        "LexiconEntry" => Ok(NodeKind::LexiconEntry),
        other => Err(CanonError::new(path, format!("unknown node kind `{other}`"))),
    }
}

/// `"Place:jerusalem"`, `"TextUnit:bible/JHN.3.16"`.
pub fn any_node_id_str(id: &AnyNodeId) -> String {
    format!("{}:{}", node_kind_str(id.kind), id.raw)
}

pub fn parse_any_node_id(s: &str, path: &str) -> Result<AnyNodeId, CanonError> {
    let (kind, raw) = s.split_once(':').ok_or_else(|| {
        CanonError::new(path, format!("node id `{s}` has no `kind:raw` separator"))
    })?;
    Ok(AnyNodeId { kind: parse_node_kind(kind, path)?, raw: raw.to_string() })
}

/// `"n:"` + the node id, or `"e:"` + the edge id. Positions include
pub fn position_str(p: &Position) -> String {
    match p {
        Position::Node(id) => format!("n:{}", any_node_id_str(id)),
        Position::Edge(e) => format!("e:{}", e.0),
    }
}

pub fn parse_position(s: &str, path: &str) -> Result<Position, CanonError> {
    match s.split_once(':') {
        Some(("n", rest)) => Ok(Position::Node(parse_any_node_id(rest, path)?)),
        Some(("e", rest)) => Ok(Position::Edge(EdgeId(rest.to_string()))),
        _ => Err(CanonError::new(path, format!("position `{s}` is neither `n:` nor `e:`"))),
    }
}

/// The bytes an edge id is minted from: `{"object":…,"rel":…,"subject":…}`, keys in byte order,
/// no whitespace, and decodable. A symmetric relation's two ends are sorted by the caller.
pub fn edge_canonical_bytes(rel_name: &str, subject: &Position, object: &Position) -> Vec<u8> {
    super::serialize(&super::obj(vec![
        ("object", super::str_value(&position_str(object))),
        ("rel", super::str_value(rel_name)),
        ("subject", super::str_value(&position_str(subject))),
    ]))
}
