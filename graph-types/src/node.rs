//! Node identity and payload; a card is a view assembled from them, not a capability.

use std::collections::BTreeMap;

use crate::chrono::TimePoint;
use crate::id::{AnyNodeId, ContentAddressed, PositionKind};
use crate::ingest::ProvenanceId;
use crate::text::LayerMap;

/// Kept fully structured rather than collapsed to a display string, so a consumer
/// reconstructs the account losslessly from the payload alone. `translations` is a
/// `BTreeMap` for determinism: a hashed order must never ride into bytes that get hashed.
#[derive(Clone, Debug)]
pub struct EventWitnessPayload {
    pub book: String,
    pub translations: BTreeMap<String, Vec<String>>,
    pub ref_note: Option<String>,
    pub robertson_section: Option<String>,
}

/// Kept fully structured rather than collapsed to prose, so the wire reconstructs it
/// losslessly from the payload alone.
#[derive(Clone, Debug)]
pub struct PolityDeltaPayload {
    pub event: String,
    pub verses: Vec<String>,
    pub ref_note: String,
}

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
    /// One node per position in the reading spine, carrying every layer's rendering as
    /// payload (the canonical layer required, the rest optional), so chains stay homogeneous.
    TextUnit { corpus: &'static str, renderings: LayerMap },
    Container { title: String },
    /// No date rides here: chronology lives only on `dated-by` edges, so an event's date can
    /// never disagree with its placement. Places and witness verses ride edges too. `verses`
    /// is the container's own top-level set, which is distinct from its witnesses'.
    Event {
        label: String,
        /// The closed set of event kinds as the bare string it is written from. Typing it
        /// would change this payload's `Debug` spelling, which IS the canonical bytes
        /// without `canon-ids`, so every content address and the version root would move.
        kind: String,
        verses: Vec<String>,
        witnesses: Vec<EventWitnessPayload>,
        robertson_section: Option<String>,
        acts_section: Option<String>,
        atlas_section: Option<String>,
        kjv_superscription: Option<String>,
        ref_note: Option<String>,
    },
    /// `legs` deliberately does not ride here: the succession edges are the one authoritative
    /// ordered chain, and a payload copy would be a second, weaker path.
    Narrative { label: String, color: String },
    /// Coordinates ride the payload so a map plots a place with no companion lookup. An alias
    /// has no position to index through the port, so it is a payload fact rather than a
    /// further explorable thing. `description` stays `None` until a source attests one.
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String>, description: Option<String> },
    /// Life years are absent for most persons: `Option`, never a fabricated sentinel.
    /// `first_year`/`last_year` are the span of the corpus's MENTIONS of the person, not a
    /// lifespan. An eternal person has no lifespan and shows no years.
    Person {
        label: String,
        gender: Option<String>,
        birth_year: Option<i32>,
        death_year: Option<i32>,
        also_called: Vec<String>,
        description: Option<String>,
        first_year: Option<i32>,
        last_year: Option<i32>,
        eternal: bool,
        eternal_grounds: Vec<String>,
    },
    /// A people group is its own kind of thing -- not a person, not a place -- so a mention
    /// can attest which sense a name carries.
    PeopleGroup { label: String, description: Option<String> },
    Anchor { at: TimePoint, citation: String },
    /// The range rides the payload: there is no edge kind for "when".
    Era { label: String, from_year: i32, to_year: i32 },
    /// The world at an era's window, drawn. What it shows rides `shows` edges, never the
    /// payload, so the map and its era can never disagree about what was in view.
    Map { label: String, from_year: i32, to_year: i32 },
    /// `color_key` is constant across a polity's eras, even as an era's name changes.
    Polity { label: String, color_key: u8, eras: Vec<PolityEraPayload> },
    CatechismItem { label: String },
    /// One node per verse-anchored unit of a work's prose. The bold lemma the parser joins on
    /// is EXCISED: verse text has one source, and verse and commentary compose at render.
    CommentaryItem { work: crate::id::SourceId, heading: Option<String>, text: String },
    Source { label: String },
    Translation { label: String },
    /// Keyed by its Strong's number, which is the node id's raw part. `glosses`/`senses` in
    /// source order, `domains` sorted.
    LexiconEntry {
        strong: String,
        lang: String,
        lemma: String,
        translit: Option<String>,
        pos: Option<String>,
        glosses: Vec<String>,
        senses: Vec<String>,
        domains: Vec<String>,
        root: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct Node {
    pub id: AnyNodeId,
    pub payload: NodePayload,
    pub provenance: ProvenanceId,
}

/// What a node IS; what exploring it means lives in `Explorable`, deliberately apart.
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
    /// A debug print is not a promise in principle, but the fixtures pin this one, so it
    /// must not move while the feature is off.
    #[cfg(not(feature = "canon-ids"))]
    fn canonical_bytes(&self) -> Vec<u8> {
        format!("{:?}|{:?}", self.id, self.payload_discriminant()).into_bytes()
    }

    /// One value, one byte spelling, and decodable back into the node that produced it --
    /// which is what makes the store self-verifying rather than merely agreeing.
    #[cfg(feature = "canon-ids")]
    fn canonical_bytes(&self) -> Vec<u8> {
        crate::canon::Canon::encode(self)
    }

    fn position_kind(&self) -> PositionKind {
        PositionKind::Node(self.id.kind)
    }
}

/// That encoding's payload spelling, with no other caller.
#[cfg(not(feature = "canon-ids"))]
impl Node {
    fn payload_discriminant(&self) -> String {
        format!("{:?}", self.payload)
    }
}

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
        | NodePayload::Map { label, .. }
        | NodePayload::Polity { label, .. }
        | NodePayload::CatechismItem { label }
        | NodePayload::Source { label }
        | NodePayload::Translation { label }
        | NodePayload::PeopleGroup { label, .. } => label.clone(),
        NodePayload::CommentaryItem { heading, .. } => {
            heading.clone().unwrap_or_else(|| "Commentary".to_string())
        }
        NodePayload::Place { canonical, .. } => canonical.clone(),
        NodePayload::Anchor { citation, .. } => citation.clone(),
        NodePayload::LexiconEntry { lemma, .. } => lemma.clone(),
    };
    Card { id: n.id(), label, provenance: n.provenance().clone() }
}
