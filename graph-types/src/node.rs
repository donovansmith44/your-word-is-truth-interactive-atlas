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
    /// M-C2 widened this to a real payload (controller decision 2); M-D3
    /// NARROWED it to narrative-only by owner order (2026-08-23, ledgered
    /// R1: "we don't need to have both narrative and chronological stuff
    /// in the payload right now. only keep narrative"). Chronology lives
    /// in ONE place: `dated-by` edges resolved through `DatePlacement` ->
    /// `ResolvedPlacement` (chrono.rs) -- the payload carries NO
    /// `from_year`/`to_year`/`order_key` mirror, so an Event's date can
    /// never disagree with its placement (the M-C2 verified-cache law
    /// retired WITH the duplicate fields it existed to police; deleting
    /// the copy is the stronger fix). `places` rides `located-at` edges,
    /// order-preserved; witness/attestation verses ride `attested-in`
    /// edges, one row per verse -- both explorable relations, not payload
    /// facts. `verses` here is the CONTAINER's own top-level verse set
    /// (`Event.verses`, distinct from witness verses -- scene
    /// composition's scripture-mode filtering needs this exact set, not a
    /// derived one).
    Event {
        label: String,
        kind: String,
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
    /// `aliases` (E3 KJV naming) rides here as payload: a bare alias
    /// string has no `Position` representation to index through the
    /// generic port -- the payload is where a fact ABOUT a place, not a
    /// further explorable thing, belongs. (M-D3, owner ruling R2: the
    /// vacant `named` relation -- manifest row, `Named` row struct, and
    /// `graph.named` table -- was RETIRED outright; this payload field
    /// was already the sole serving path, so the parallel authored rows
    /// were exactly the "second, weaker path" the discipline forbids.)
    /// ENT-1 (owner order 2026-08-23: "we actually want meaningful
    /// information about who or what someone is, having that be backed
    /// by scripture"): `description` is Easton's PD prose ABOUT the
    /// entity -- a payload FACT (the aliases/border-data precedent);
    /// key-passage SELECTION stays a law-computed query (P4:
    /// presentation = selection), never stored. `None` until the
    /// Easton's adapter fills a source-attested match -- NO fabricated
    /// prose, ever.
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String>, description: Option<String> },
    /// Batch P (the extensibility proof): widened the SAME way M-C widened
    /// Place/Polity (controller decision 2) -- real payload, not a stub.
    /// `label` is the display name; `gender`/`birth_year`/`death_year` ride
    /// verbatim as tagged by the Theographic source (life years absent for
    /// the overwhelming majority of real persons -- `Option`, never a
    /// fabricated sentinel); `also_called` is the source's own comma-split
    /// alternate-name list -- the SAME "payload, not a new relation kind"
    /// shape Place's own KJV aliases and Polity's own border data already
    /// use (a fact ABOUT the person, not a further explorable thing).
    Person { label: String, gender: Option<String>, birth_year: Option<i32>, death_year: Option<i32>, also_called: Vec<String>, description: Option<String> },
    /// PG-1 (owner order 2026-08-23: "we need a way to distinguish
    /// between the names of the twelve tribes and the people theyre
    /// named after"; "pull in Peoples or Nations info so I can find
    /// out who the ammonites are"). A people group (tribe/nation/
    /// clan) is its OWN kind of thing -- not a Person, not a Place --
    /// so a mention can attest WHICH sense a name carries, and "who
    /// are the Ammonites?" has a node to answer from. ENT-1 widened
    /// `description` here the same hour it widened Place/Person -- see
    /// their shared doc note above.
    PeopleGroup { label: String, description: Option<String> },
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
    /// SVEB-1: one TOPICAL grouping of proof texts under a catechism item
    /// -- the brain-fuel/catechism repo's own per-question titles ("Worship
    /// God Alone", "Fear, Love, and Trust in God"), 574 of them.
    ///
    /// WHY THIS IS A NODE AND NOT AN EDGE TAG: before this batch the topic
    /// did not survive onto the graph at all. `catechism_adapter`'s own doc
    /// comment said so plainly -- `catechism-link` is "a flat, symmetric,
    /// locus <-> item relation ... with no room for a question tag" -- so
    /// 7,251 curated topic-attributed citations collapsed into 6,531
    /// item-to-verse rows, losing 740 outright and losing the topic on all
    /// of them. Making the topic a NODE (item `contains` topic, topic
    /// `cites` verse) restores every citation with its attribution and, as
    /// the point of the exercise, makes the topic itself traversable from
    /// both ends: "what does this topic gather" and "which topics does this
    /// verse serve".
    ///
    /// `source` is the provenance tag the curated data already carries
    /// ("brain-fuel/catechism" or "deut5-parallel"), kept on the node so a
    /// topic never loses which mapping authored it.
    CatechismTopic { label: String, source: String },
    /// PARTS-1: one chief part. `curated` is false for a part this app
    /// materializes for content that has no `catechism.toml` counterpart
    /// (the appendices, and the brain-fuel repo's own topical sections) --
    /// disclosed on the node itself rather than inferred, so a reader can
    /// always tell Luther's own six chief parts from the extras.
    CatechismPart { label: String, curated: bool },
    /// SVEB-1: one addressable unit of Svebilius' Catechism -- a numbered
    /// question with its answer in the seven Q&A sections, or one prose
    /// block in the two that are not Q&A (see `text::SvebiliusTag`). The
    /// question and its answer are ONE node, the same "one catechetical
    /// unit is one thing" call `catechism.toml` already makes for
    /// text+explanation and the Concord parser makes for its own
    /// question/answer sub-lettering.
    SvebiliusUnit { question: Option<String>, answer: String },
    /// KRETZ-1 (owner order 2026-08-24: "pull kretzmann commentary
    /// (public domain version) into our corpora"; ruled the ANNOTATION
    /// shape: "a comprehensive commentary without the verses interleaved
    /// into it, and it's indexed so that each verse mapped bit of
    /// commentary is mapped to the appropriate verse in our graph").
    /// One node per verse-anchored unit of the work's prose; the bold
    /// KJV lemma is the parser's join key and is EXCISED -- verse text
    /// has ONE source (the canonical layer); verse + commentary compose
    /// at render. `work` is the commentary work's Source node; `heading`
    /// carries the pericope context where the source printed one.
    CommentaryItem { work: crate::id::SourceId, heading: Option<String>, text: String },
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
        | NodePayload::Translation { label }
        | NodePayload::PeopleGroup { label, .. } => label.clone(),
        NodePayload::CommentaryItem { heading, .. } => {
            heading.clone().unwrap_or_else(|| "Commentary".to_string())
        }
        NodePayload::CatechismTopic { label, .. } | NodePayload::CatechismPart { label, .. } => label.clone(),
        // SVEB-1: the question IS the label where there is one. The two
        // non-Q&A sections (psalms, the confession formula) have no
        // question, and fall back to their answer prose -- truncation is
        // the caller's business, the same way CommentaryItem leaves its
        // own long `text` to the renderer.
        NodePayload::SvebiliusUnit { question, answer } => {
            question.clone().unwrap_or_else(|| answer.clone())
        }
        NodePayload::Place { canonical, .. } => canonical.clone(),
        NodePayload::Anchor { citation, .. } => citation.clone(),
    };
    Card { id: n.id(), label, provenance: n.provenance().clone() }
}
