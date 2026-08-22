//! Nodes: identity + payload (NodeData), with Card demoted to a view
//! function — capability and presentation deliberately separated.

use crate::chrono::TimePoint;
use crate::id::{AnyNodeId, ContentAddressed, PositionKind};
use crate::ingest::ProvenanceId;
use crate::text::LayerMap;

#[derive(Clone, Debug)]
pub enum NodePayload {
    /// ONE node per skeleton position; ALL layer renderings as payload
    /// (canonical layer required, others optional) — chains stay
    /// homogeneous (sweep F1).
    TextUnit { corpus: &'static str, renderings: LayerMap },
    Container { title: String },
    Event { label: String },
    Narrative { label: String },
    Place { canonical: String },
    Person { label: String },
    /// Explorable "why this date?" — day-capable (sweep F4).
    Anchor { at: TimePoint, citation: String },
    Era { label: String },
    Polity { label: String },
    CatechismItem { label: String },
    Source { label: String },
    Translation { label: String },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: AnyNodeId,
    pub payload: NodePayload,
    pub provenance: ProvenanceId,
}

/// What a node IS — every node has this trivially. (What exploration
/// means lives in `Explorable`; the split is deliberate.)
pub trait NodeData {
    fn id(&self) -> AnyNodeId;
    fn payload(&self) -> &NodePayload;
    fn provenance(&self) -> &ProvenanceId;
}

impl NodeData for Node {
    fn id(&self) -> AnyNodeId {
        self.id.clone()
    }
    fn payload(&self) -> &NodePayload {
        &self.payload
    }
    fn provenance(&self) -> &ProvenanceId {
        &self.provenance
    }
}

impl ContentAddressed for Node {
    fn canonical_bytes(&self) -> Vec<u8> {
        format!("{:?}|{:?}", self.id, self.payload_discriminant()).into_bytes()
    }
    fn position_kind(&self) -> PositionKind {
        PositionKind::Node(self.id.kind)
    }
}

impl Node {
    fn payload_discriminant(&self) -> String {
        format!("{:?}", self.payload)
    }
}

/// Card is a VIEW, not a capability: assembled from NodeData plus
/// law-computed selections, rendered via Presentable.
#[derive(Clone, Debug)]
pub struct Card {
    pub id: AnyNodeId,
    pub label: String,
    pub provenance: ProvenanceId,
}

pub fn card(n: &dyn NodeData) -> Card {
    let label = match n.payload() {
        NodePayload::TextUnit { corpus, .. } => format!("text unit ({corpus})"),
        NodePayload::Container { title } => title.clone(),
        NodePayload::Event { label }
        | NodePayload::Narrative { label }
        | NodePayload::Person { label }
        | NodePayload::Era { label }
        | NodePayload::Polity { label }
        | NodePayload::CatechismItem { label }
        | NodePayload::Source { label }
        | NodePayload::Translation { label } => label.clone(),
        NodePayload::Place { canonical } => canonical.clone(),
        NodePayload::Anchor { citation, .. } => citation.clone(),
    };
    Card { id: n.id(), label, provenance: n.provenance().clone() }
}
