//! Nodes: identity + payload (NodeData), with Card demoted to a view
//! function — capability and presentation deliberately separated.

use crate::chrono::TimePoint;
use crate::id::{AnyNodeId, ContentAddressed, PositionKind};
use crate::ingest::ProvenanceId;
use crate::text::LayerMap;

/// One time-ranged, colored, bordered ERA of one Polity node's own
/// lifetime (M-C: "border data as node payloads — the map consumes
/// payloads, not new relation kinds," controller decision 2). A plain
/// data mirror of `atlas_core::data::PolityEra`'s own load-bearing fields
/// (name/from/to/rings/ref_note); transition/fall summaries are carried
/// as short display prose rather than the richer nested delta shape —
/// the map's own popover need, not a new edge kind.
#[derive(Clone, Debug)]
pub struct PolityEraPayload {
    pub name: String,
    pub from_year: i32,
    pub to_year: i32,
    pub rings: Vec<Vec<(f64, f64)>>,
    pub ref_note: String,
    pub transition: Option<String>,
    pub fall: Option<String>,
}

#[derive(Clone, Debug)]
pub enum NodePayload {
    /// ONE node per skeleton position; ALL layer renderings as payload
    /// (canonical layer required, others optional) — chains stay
    /// homogeneous (sweep F1).
    TextUnit { corpus: &'static str, renderings: LayerMap },
    Container { title: String },
    Event { label: String },
    Narrative { label: String },
    /// M-C: real payload, not a stub (controller decision 2) — geographic
    /// coordinates join the canonical name so the map can plot a Place
    /// node directly from its own payload, with no companion lookup.
    /// `aliases` (E3 KJV naming) rides here rather than as `named` edges:
    /// a `Named` row's object is a bare alias string, which has no
    /// `Position` representation to index through the generic port (see
    /// `graph.rs::build_indexes`'s own disclosed note) -- the payload is
    /// where a fact ABOUT a place, not a further explorable thing, belongs.
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String> },
    Person { label: String },
    /// Explorable "why this date?" — day-capable (sweep F4).
    Anchor { at: TimePoint, citation: String },
    /// M-C: a time-range boundary node for the map/era selector — payload
    /// carries the range directly (no separate edge kind for "when,"
    /// matching the Polity payload's own border-data-as-payload
    /// precedent).
    Era { label: String, from_year: i32, to_year: i32 },
    /// M-C: every era of this polity's own lifetime, as payload (border
    /// data as node payloads — controller decision 2). `color_key` is
    /// constant across the polity's own eras (one hue for its whole
    /// lifetime, even as `eras[].name` changes, e.g. "Egypt" ->
    /// "Ptolemaic Egypt").
    Polity { label: String, color_key: u8, eras: Vec<PolityEraPayload> },
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
        | NodePayload::Era { label, .. }
        | NodePayload::Polity { label, .. }
        | NodePayload::CatechismItem { label }
        | NodePayload::Source { label }
        | NodePayload::Translation { label } => label.clone(),
        NodePayload::Place { canonical, .. } => canonical.clone(),
        NodePayload::Anchor { citation, .. } => citation.clone(),
    };
    Card { id: n.id(), label, provenance: n.provenance().clone() }
}
