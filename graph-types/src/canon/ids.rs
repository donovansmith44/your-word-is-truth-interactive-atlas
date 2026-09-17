//! Canonical STRING forms for the identity types.
//!
//! `AnyNodeId` and `Position` have no string spelling in the crate today
//! (only `{:?}`), so these functions MINT the artifact's one: kind name,
//! `:`, raw. The raw part may itself contain colons (`bible/JHN.3.16` is
//! colon-free, but nothing forbids them and edge ids carry one), so the
//! parse side splits at the FIRST colon and hands the whole remainder
//! back as the raw -- that makes the round trip total for every raw.
//!
//! Each parser takes the `path` it is decoding under (R10b): these are
//! called from inside a field decoder, and an error that cannot say WHERE
//! the bad id sat is most of the way to useless.

use crate::edge::EdgeId;
use crate::id::{AnyNodeId, NodeKind, Position};

use super::CanonError;

/// The canonical name of a node kind: its `Debug` name, so the encoding
/// and the source read the same.
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
/// reports at that node id's own location rather than nowhere (R10b).
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
/// edges (the edges-as-positions law), so both need a spelling.
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
