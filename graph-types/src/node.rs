//! Nodes: identity + payload (NodeData), with Card demoted to a view
//! function — capability and presentation deliberately separated.

use std::collections::BTreeMap;

use crate::chrono::TimePoint;
use crate::id::{AnyNodeId, ContentAddressed, PositionKind};
use crate::ingest::ProvenanceId;
use crate::text::LayerMap;

/// M-C2: one witness account of an Event -- a plain data mirror of
/// `atlas_core::data::EventWitness`'s own load-bearing fields (book +
/// translations + ref_note + robertson_section), kept FULLY STRUCTURED
/// (not collapsed to a display string) so `handlers::event`'s own
/// `EventDetailOut.witnesses` -- and any other consumer needing a real
/// `atlas_core::data::Event` -- reconstructs losslessly from the payload
/// alone, the SAME "real payload, not a stub" precedent M-C's Place/Polity
/// widening already set (controller decision 2). `translations` is a
/// `BTreeMap` (the source `EventWitness.translations` is a `HashMap`) --
/// deliberately, for serialization determinism (M-C2 requirement 3: the
/// serialized graph artifact must be byte-deterministic; a `HashMap`'s
/// randomized iteration order has no place riding into a payload that gets
/// dumped to bytes).
#[derive(Clone, Debug)]
pub struct EventWitnessPayload {
    pub book: String,
    pub translations: BTreeMap<String, Vec<String>>,
    pub ref_note: Option<String>,
    pub robertson_section: Option<String>,
}

/// One Scripture-mapped historical delta at an era boundary (rise/fall/
/// internal transition) -- a plain data mirror of
/// `atlas_core::data::PolityDelta`'s own load-bearing fields, kept FULLY
/// STRUCTURED (not collapsed to display prose) so the map's own wire
/// response (`PolityDeltaOut { event, verses, ref_note }`) reconstructs
/// losslessly from the payload alone.
#[derive(Clone, Debug)]
pub struct PolityDeltaPayload {
    pub event: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

/// One time-ranged, colored, bordered ERA of one Polity node's own
/// lifetime (M-C: "border data as node payloads — the map consumes
/// payloads, not new relation kinds," controller decision 2). A plain
/// data mirror of `atlas_core::data::PolityEra`'s own load-bearing fields
/// (name/from/to/rings/ref_note/transition/fall).
#[derive(Clone, Debug)]
pub struct PolityEraPayload {
    pub name: String,
    pub from_year: i32,
    pub to_year: i32,
    pub rings: Vec<Vec<(f64, f64)>>,
    pub ref_note: String,
    pub transition: Option<PolityDeltaPayload>,
    pub fall: Option<PolityDeltaPayload>,
}

#[derive(Clone, Debug)]
pub enum NodePayload {
    /// ONE node per skeleton position; ALL layer renderings as payload
    /// (canonical layer required, others optional) — chains stay
    /// homogeneous (sweep F1).
    TextUnit { corpus: &'static str, renderings: LayerMap },
    Container { title: String },
    /// M-C2: real payload, not a stub (same controller-decision-2 precedent
    /// as Place/Polity's own M-C widening) -- every field
    /// `atlas_core::data::Event` carries beyond its own `id`/`places`/
    /// `verses`-as-edges (`places` rides `located-at` edges, order-
    /// preserved; witness/attestation verses ride `attested-in` edges,
    /// one row per verse -- both explorable relations, not payload facts).
    /// `verses` here is the CONTAINER's own top-level verse set
    /// (`Event.verses`, distinct from witness verses -- scene composition's
    /// scripture-mode filtering needs this exact set, not a derived one).
    /// `from_year`/`to_year` mirror `Event.when` (a general-kind passage's
    /// own `TimeRange::undated()` sentinel included, verbatim -- never
    /// re-derived, so a reconstructed `Event` can never disagree with the
    /// source about whether a passage is dated).
    Event {
        label: String,
        kind: String,
        from_year: i32,
        to_year: i32,
        order_key: i32,
        verses: Vec<String>,
        witnesses: Vec<EventWitnessPayload>,
        robertson_section: Option<String>,
        acts_section: Option<String>,
        atlas_section: Option<String>,
        kjv_superscription: Option<String>,
        ref_note: Option<String>,
    },
    /// M-C2: `color` joins `label` -- a narrative's own map-arrow color is a
    /// fact ABOUT the narrative, not an explorable relation (mirrors
    /// Polity's own `color_key`). `legs` deliberately does NOT ride here --
    /// the `succession` relation (`follows-in`/`precedes-in` edges, tagged
    /// by this narrative) is already the single, authoritative ordered
    /// chain; duplicating it onto the payload would be exactly the
    /// "second, weaker path" this migration's own discipline forbids.
    Narrative { label: String, color: String },
    /// M-C: real payload, not a stub (controller decision 2) — geographic
    /// coordinates join the canonical name so the map can plot a Place
    /// node directly from its own payload, with no companion lookup.
    /// `aliases` (E3 KJV naming) rides here rather than as `named` edges:
    /// a `Named` row's object is a bare alias string, which has no
    /// `Position` representation to index through the generic port (see
    /// `graph.rs::build_indexes`'s own disclosed note) -- the payload is
    /// where a fact ABOUT a place, not a further explorable thing, belongs.
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String> },
    /// Batch P (the extensibility proof): widened the SAME way M-C widened
    /// Place/Polity (controller decision 2) -- real payload, not a stub.
    /// `label` is the display name; `gender`/`birth_year`/`death_year` ride
    /// verbatim as tagged by the Theographic source (life years absent for
    /// the overwhelming majority of real persons -- `Option`, never a
    /// fabricated sentinel); `also_called` is the source's own comma-split
    /// alternate-name list -- the SAME "payload, not a new relation kind"
    /// shape Place's own KJV aliases and Polity's own border data already
    /// use (a fact ABOUT the person, not a further explorable thing).
    Person { label: String, gender: Option<String>, birth_year: Option<i32>, death_year: Option<i32>, also_called: Vec<String> },
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
        NodePayload::Event { label, .. }
        | NodePayload::Narrative { label, .. }
        | NodePayload::Person { label, .. }
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
