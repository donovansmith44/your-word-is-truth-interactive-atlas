//! THE SERIALIZED ARTIFACT (design doc §9a/§9b; controller decision 4,
//! M-C): GraphStore IMPLEMENTATION #2. "The compiled artifact is defined
//! LOGICALLY -- a content-addressed set of canonical things + derived
//! indexes + a version root -- never as a file format." This module is
//! that logical definition, realized: a lossless, hand-written mirror of
//! every AUTHORED/IMPORTED row table plus the node table plus the reading
//! spine(s) -- deliberately NOT the derived indexes (`Graph::indexes`/
//! `symmetric_indexes`/`pid_index`), which are cheap to rebuild from the
//! rows via the SAME `Graph::build_indexes()` every other build path
//! already calls, and re-deriving them is itself part of the port's own
//! honesty (derived state is never trusted-and-copied, only computed).
//!
//! ON-DISK FORM, disclosed (this batch's own call, per the brief):
//! `bincode` over these DTOs -- compact and fast, chosen so the load-time
//! ceiling (<=3s, a committed law) is achievable over the dominant
//! ~344k-row `cites` table without JSON's per-byte text-parsing cost.
//!
//! WHY A HAND-WRITTEN DTO LAYER, NOT `#[derive(Serialize)]` ON
//! `graph-types` ITSELF: `graph-types/` is owner-approved and EXTEND-ONLY.
//! Deriving serde directly on its types would be additive in isolation,
//! but several of them are deeply generic/phantom-typed (`NodeId<K:
//! KindTag>`, `Locus<C: Corpus>`, `LocusSet<C>`) in ways that would force
//! either a crate-wide derive sweep (touching nearly every file in the
//! owner-approved crate for a serialization concern it was never designed
//! around) or per-type `#[serde(bound = ...)]` overrides threaded through
//! generic code. A self-contained mirror confined entirely to THIS crate
//! carries zero risk to graph-types and is easy to verify for lossless
//! round-tripping in isolation (see `tests` below) -- the tradeoff this
//! batch makes on purpose, documented here rather than silently.
//!
//! SCOPE, disclosed: `Graph`'s `contains_bible`/`contains_concord`/
//! `quotes`/`confesses`/`corresponds_bible` tables are ALWAYS EMPTY as of
//! this batch (no adapter populates Container/Quotes/Confesses/Corresponds
//! rows anywhere in this codebase -- confirmed by reading every adapter
//! fresh). `dump` asserts each is empty and returns a loud, named error if
//! not, rather than silently discarding real data a future batch might
//! add without updating this module -- "everything fail-loud," not a
//! silent drop.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};
use atlas_graph_types::edge::{
    Attests, CatechismLink, CrossRef, Ground, Justification, LocatedAt, Mentions, MentionedEntity, Succession,
};
use atlas_graph_types::graph::{Graph, ReadingSpine};
use atlas_graph_types::id::{AnchorId, AnyNodeId, CatechismItemId, EraId, EventId, NarrativeId, NodeKind, PeopleGroupId, PersonId, PlaceId, SourceId};
use atlas_graph_types::node::{Node, NodePayload, PolityEraPayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, ConcordRef, LocusRange, TextLocus, TextRef, TokenSpan, TranslationId, VerseRef};

use serde::{Deserialize, Serialize};

use crate::build::BuildStats;
use crate::event_world::{Chronology, ChronologyDerivation, EventWorldStats, PlacedChronology};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactError(pub String);
impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "artifact error: {}", self.0)
    }
}
impl std::error::Error for ArtifactError {}

// ---------------------------------------------------------------------
// Node identity + payload.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum DtoNodeKind {
    TextUnit,
    Container,
    Event,
    Narrative,
    Place,
    Person,
    Anchor,
    Era,
    Polity,
    CatechismItem,
    Source,
    Translation,
    PeopleGroup,
}

impl From<NodeKind> for DtoNodeKind {
    fn from(k: NodeKind) -> Self {
        match k {
            NodeKind::TextUnit => DtoNodeKind::TextUnit,
            NodeKind::Container => DtoNodeKind::Container,
            NodeKind::Event => DtoNodeKind::Event,
            NodeKind::Narrative => DtoNodeKind::Narrative,
            NodeKind::Place => DtoNodeKind::Place,
            NodeKind::Person => DtoNodeKind::Person,
            NodeKind::Anchor => DtoNodeKind::Anchor,
            NodeKind::Era => DtoNodeKind::Era,
            NodeKind::Polity => DtoNodeKind::Polity,
            NodeKind::CatechismItem => DtoNodeKind::CatechismItem,
            NodeKind::Source => DtoNodeKind::Source,
            NodeKind::Translation => DtoNodeKind::Translation,
            NodeKind::PeopleGroup => DtoNodeKind::PeopleGroup,
        }
    }
}
impl From<DtoNodeKind> for NodeKind {
    fn from(k: DtoNodeKind) -> Self {
        match k {
            DtoNodeKind::TextUnit => NodeKind::TextUnit,
            DtoNodeKind::Container => NodeKind::Container,
            DtoNodeKind::Event => NodeKind::Event,
            DtoNodeKind::Narrative => NodeKind::Narrative,
            DtoNodeKind::Place => NodeKind::Place,
            DtoNodeKind::Person => NodeKind::Person,
            DtoNodeKind::Anchor => NodeKind::Anchor,
            DtoNodeKind::Era => NodeKind::Era,
            DtoNodeKind::Polity => NodeKind::Polity,
            DtoNodeKind::CatechismItem => NodeKind::CatechismItem,
            DtoNodeKind::Source => NodeKind::Source,
            DtoNodeKind::Translation => NodeKind::Translation,
            DtoNodeKind::PeopleGroup => NodeKind::PeopleGroup,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct DtoNodeId {
    kind: DtoNodeKind,
    raw: String,
}
impl From<&AnyNodeId> for DtoNodeId {
    fn from(id: &AnyNodeId) -> Self {
        DtoNodeId { kind: id.kind.into(), raw: id.raw.clone() }
    }
}
impl From<DtoNodeId> for AnyNodeId {
    fn from(d: DtoNodeId) -> Self {
        AnyNodeId { kind: d.kind.into(), raw: d.raw }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoPolityDelta {
    event: String,
    verses: Vec<String>,
    ref_note: String,
}
impl From<&atlas_graph_types::node::PolityDeltaPayload> for DtoPolityDelta {
    fn from(d: &atlas_graph_types::node::PolityDeltaPayload) -> Self {
        DtoPolityDelta { event: d.event.clone(), verses: d.verses.clone(), ref_note: d.ref_note.clone() }
    }
}
impl From<DtoPolityDelta> for atlas_graph_types::node::PolityDeltaPayload {
    fn from(d: DtoPolityDelta) -> Self {
        atlas_graph_types::node::PolityDeltaPayload { event: d.event, verses: d.verses, ref_note: d.ref_note }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoPolityEra {
    name: String,
    from_year: i32,
    to_year: i32,
    rings: Vec<Vec<(f64, f64)>>,
    ref_note: String,
    transition: Option<DtoPolityDelta>,
    fall: Option<DtoPolityDelta>,
}
impl From<&PolityEraPayload> for DtoPolityEra {
    fn from(e: &PolityEraPayload) -> Self {
        DtoPolityEra {
            name: e.name.clone(),
            from_year: e.from_year,
            to_year: e.to_year,
            rings: e.rings.clone(),
            ref_note: e.ref_note.clone(),
            transition: e.transition.as_ref().map(DtoPolityDelta::from),
            fall: e.fall.as_ref().map(DtoPolityDelta::from),
        }
    }
}
impl From<DtoPolityEra> for PolityEraPayload {
    fn from(d: DtoPolityEra) -> Self {
        PolityEraPayload {
            name: d.name,
            from_year: d.from_year,
            to_year: d.to_year,
            rings: d.rings,
            ref_note: d.ref_note,
            transition: d.transition.map(Into::into),
            fall: d.fall.map(Into::into),
        }
    }
}

/// M-C2: DTO mirror of `graph_types::node::EventWitnessPayload`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoEventWitness {
    book: String,
    translations: BTreeMap<String, Vec<String>>,
    ref_note: Option<String>,
    robertson_section: Option<String>,
}
impl From<&atlas_graph_types::node::EventWitnessPayload> for DtoEventWitness {
    fn from(w: &atlas_graph_types::node::EventWitnessPayload) -> Self {
        DtoEventWitness { book: w.book.clone(), translations: w.translations.clone(), ref_note: w.ref_note.clone(), robertson_section: w.robertson_section.clone() }
    }
}
impl From<DtoEventWitness> for atlas_graph_types::node::EventWitnessPayload {
    fn from(d: DtoEventWitness) -> Self {
        atlas_graph_types::node::EventWitnessPayload { book: d.book, translations: d.translations, ref_note: d.ref_note, robertson_section: d.robertson_section }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoPayload {
    TextUnit { corpus: String, renderings: BTreeMap<String, String> },
    Container { title: String },
    /// M-C2: mirrors `NodePayload::Event`'s own widening -- see that
    /// variant's own doc comment for why each field rides the payload.
    /// M-D3 (owner ruling R1): from_year/to_year/order_key dropped -- see
    /// `NodePayload::Event`'s own doc comment; chronology now rides
    /// `ArtifactDump.chrono_years`, ordering-uninvolved.
    Event {
        label: String,
        kind: String,
        verses: Vec<String>,
        witnesses: Vec<DtoEventWitness>,
        robertson_section: Option<String>,
        acts_section: Option<String>,
        atlas_section: Option<String>,
        kjv_superscription: Option<String>,
        ref_note: Option<String>,
    },
    Narrative { label: String, color: String },
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String> },
    /// Batch P: mirrors `NodePayload::Person`'s own widening -- see that
    /// variant's own doc comment.
    Person { label: String, gender: Option<String>, birth_year: Option<i32>, death_year: Option<i32>, also_called: Vec<String> },
    Anchor { year: i32, month: Option<u8>, day: Option<u8>, citation: String },
    Era { label: String, from_year: i32, to_year: i32 },
    Polity { label: String, color_key: u8, eras: Vec<DtoPolityEra> },
    CatechismItem { label: String },
    Source { label: String },
    Translation { label: String },
    PeopleGroup { label: String },
}

fn payload_to_dto(p: &NodePayload) -> DtoPayload {
    match p {
        NodePayload::TextUnit { corpus, renderings } => DtoPayload::TextUnit {
            corpus: corpus.to_string(),
            renderings: renderings.iter().map(|(k, v)| (k.0.clone(), v.clone())).collect(),
        },
        NodePayload::Container { title } => DtoPayload::Container { title: title.clone() },
        NodePayload::Event { label, kind, verses, witnesses, robertson_section, acts_section, atlas_section, kjv_superscription, ref_note } => DtoPayload::Event {
            label: label.clone(),
            kind: kind.clone(),
            verses: verses.clone(),
            witnesses: witnesses.iter().map(DtoEventWitness::from).collect(),
            robertson_section: robertson_section.clone(),
            acts_section: acts_section.clone(),
            atlas_section: atlas_section.clone(),
            kjv_superscription: kjv_superscription.clone(),
            ref_note: ref_note.clone(),
        },
        NodePayload::Narrative { label, color } => DtoPayload::Narrative { label: label.clone(), color: color.clone() },
        NodePayload::Place { canonical, lat, lon, aliases } => DtoPayload::Place { canonical: canonical.clone(), lat: *lat, lon: *lon, aliases: aliases.clone() },
        NodePayload::Person { label, gender, birth_year, death_year, also_called } => DtoPayload::Person {
            label: label.clone(),
            gender: gender.clone(),
            birth_year: *birth_year,
            death_year: *death_year,
            also_called: also_called.clone(),
        },
        NodePayload::Anchor { at, citation } => DtoPayload::Anchor { year: at.year.get(), month: at.month, day: at.day, citation: citation.clone() },
        NodePayload::Era { label, from_year, to_year } => DtoPayload::Era { label: label.clone(), from_year: *from_year, to_year: *to_year },
        NodePayload::Polity { label, color_key, eras } => DtoPayload::Polity { label: label.clone(), color_key: *color_key, eras: eras.iter().map(DtoPolityEra::from).collect() },
        NodePayload::CatechismItem { label } => DtoPayload::CatechismItem { label: label.clone() },
        NodePayload::Source { label } => DtoPayload::Source { label: label.clone() },
        NodePayload::Translation { label } => DtoPayload::Translation { label: label.clone() },
        NodePayload::PeopleGroup { label } => DtoPayload::PeopleGroup { label: label.clone() },
    }
}

fn payload_from_dto(d: DtoPayload) -> Result<NodePayload, ArtifactError> {
    Ok(match d {
        DtoPayload::TextUnit { corpus, renderings } => {
            let corpus: &'static str = match corpus.as_str() {
                "bible" => "bible",
                "concord" => "concord",
                other => return Err(ArtifactError(format!("unknown corpus '{other}' in serialized artifact"))),
            };
            NodePayload::TextUnit { corpus, renderings: renderings.into_iter().map(|(k, v)| (TranslationId(k), v)).collect() }
        }
        DtoPayload::Container { title } => NodePayload::Container { title },
        DtoPayload::Event { label, kind, verses, witnesses, robertson_section, acts_section, atlas_section, kjv_superscription, ref_note } => NodePayload::Event {
            label,
            kind,
            verses,
            witnesses: witnesses.into_iter().map(Into::into).collect(),
            robertson_section,
            acts_section,
            atlas_section,
            kjv_superscription,
            ref_note,
        },
        DtoPayload::Narrative { label, color } => NodePayload::Narrative { label, color },
        DtoPayload::Place { canonical, lat, lon, aliases } => NodePayload::Place { canonical, lat, lon, aliases },
        DtoPayload::Person { label, gender, birth_year, death_year, also_called } => {
            NodePayload::Person { label, gender, birth_year, death_year, also_called }
        }
        DtoPayload::Anchor { year, month, day, citation } => {
            let at = atlas_graph_types::chrono::TimePoint::new(
                atlas_graph_types::chrono::Year::new(year).map_err(|_| ArtifactError("anchor year 0 in serialized artifact".into()))?,
                month,
                day,
            )
            .map_err(|_| ArtifactError("anchor day-without-month in serialized artifact".into()))?;
            NodePayload::Anchor { at, citation }
        }
        DtoPayload::Era { label, from_year, to_year } => NodePayload::Era { label, from_year, to_year },
        DtoPayload::Polity { label, color_key, eras } => NodePayload::Polity { label, color_key, eras: eras.into_iter().map(PolityEraPayload::from).collect() },
        DtoPayload::CatechismItem { label } => NodePayload::CatechismItem { label },
        DtoPayload::Source { label } => NodePayload::Source { label },
        DtoPayload::Translation { label } => NodePayload::Translation { label },
        DtoPayload::PeopleGroup { label } => NodePayload::PeopleGroup { label },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoNode {
    id: DtoNodeId,
    payload: DtoPayload,
    provenance: String,
}

// ---------------------------------------------------------------------
// Text addressing (loci) -- shared by several row tables.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoTokenSpan {
    layer: String,
    start: u16,
    end: u16,
}
impl From<&TokenSpan> for DtoTokenSpan {
    fn from(s: &TokenSpan) -> Self {
        DtoTokenSpan { layer: s.layer.0.clone(), start: s.start, end: s.end }
    }
}
impl TryFrom<DtoTokenSpan> for TokenSpan {
    type Error = ArtifactError;
    fn try_from(d: DtoTokenSpan) -> Result<Self, ArtifactError> {
        TokenSpan::new(TranslationId(d.layer), d.start, d.end).map_err(|_| ArtifactError("inverted token span in serialized artifact".into()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoTextRef {
    Bible { book: u8, chapter: u16, verse: u16 },
    Concord { part: u8, article: u16, paragraph: u16 },
}
impl From<&TextRef> for DtoTextRef {
    fn from(r: &TextRef) -> Self {
        match r {
            TextRef::Bible(v) => DtoTextRef::Bible { book: v.book, chapter: v.chapter, verse: v.verse },
            TextRef::Concord(c) => DtoTextRef::Concord { part: c.part, article: c.article, paragraph: c.paragraph },
        }
    }
}
impl From<DtoTextRef> for TextRef {
    fn from(d: DtoTextRef) -> Self {
        match d {
            DtoTextRef::Bible { book, chapter, verse } => TextRef::Bible(VerseRef { book, chapter, verse }),
            DtoTextRef::Concord { part, article, paragraph } => TextRef::Concord(ConcordRef { part, article, paragraph }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoTextLocus {
    at: DtoTextRef,
    span: Option<DtoTokenSpan>,
}
impl From<&TextLocus> for DtoTextLocus {
    fn from(l: &TextLocus) -> Self {
        DtoTextLocus { at: (&l.at).into(), span: l.span.as_ref().map(DtoTokenSpan::from) }
    }
}
impl TryFrom<DtoTextLocus> for TextLocus {
    type Error = ArtifactError;
    fn try_from(d: DtoTextLocus) -> Result<Self, ArtifactError> {
        Ok(TextLocus { at: d.at.into(), span: d.span.map(TokenSpan::try_from).transpose()? })
    }
}

fn bible_locus_to_dto(l: &BibleLocus) -> DtoTextLocus {
    DtoTextLocus::from(&TextLocus::from(BibleLocus { unit: l.unit.clone(), span: l.span.clone() }))
}
fn dto_to_bible_locus(d: DtoTextLocus) -> Result<BibleLocus, ArtifactError> {
    let tl = TextLocus::try_from(d)?;
    tl.as_bible().ok_or_else(|| ArtifactError("expected a Bible-corpus locus in serialized artifact".into()))
}
fn bible_range_to_dto(r: &BibleLocusRange) -> (DtoTextLocus, DtoTextLocus) {
    (bible_locus_to_dto(&r.from), bible_locus_to_dto(&r.to))
}
fn dto_to_bible_range(from: DtoTextLocus, to: DtoTextLocus) -> Result<BibleLocusRange, ArtifactError> {
    LocusRange::new(dto_to_bible_locus(from)?, dto_to_bible_locus(to)?).map_err(|_| ArtifactError("inverted Bible locus range in serialized artifact".into()))
}

// Deliberately no `concord_locus_to_dto`/`dto_to_concord_locus`: `dump`
// rejects a non-empty `confesses` table outright (the one row kind that
// would need one), so there is no real caller for a Concord-locus
// conversion yet -- adding it speculatively, untested against real
// Concord data, would be exactly the kind of code this project's own
// discipline argues against.

// ---------------------------------------------------------------------
// Justification / grounds.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoGround {
    Scripture(DtoTextLocus, DtoTextLocus),
    Anchor(String),
    Source(String),
}
fn ground_to_dto(g: &Ground) -> DtoGround {
    match g {
        Ground::Scripture(r) => {
            let (f, t) = bible_range_to_dto(r);
            DtoGround::Scripture(f, t)
        }
        Ground::Anchor(a) => DtoGround::Anchor(a.0.clone()),
        Ground::Source(s) => DtoGround::Source(s.0.clone()),
    }
}
fn dto_to_ground(d: DtoGround) -> Result<Ground, ArtifactError> {
    Ok(match d {
        DtoGround::Scripture(f, t) => Ground::Scripture(dto_to_bible_range(f, t)?),
        DtoGround::Anchor(a) => Ground::Anchor(AnchorId::new(a)),
        DtoGround::Source(s) => Ground::Source(SourceId::new(s)),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoJustification {
    text: Option<String>,
    grounds: Vec<DtoGround>,
}
fn justification_to_dto(j: &Justification) -> DtoJustification {
    DtoJustification { text: j.text.clone(), grounds: j.grounds.iter().map(ground_to_dto).collect() }
}
fn dto_to_justification(d: DtoJustification) -> Result<Justification, ArtifactError> {
    Ok(Justification { text: d.text, grounds: d.grounds.into_iter().map(dto_to_ground).collect::<Result<_, _>>()? })
}

// ---------------------------------------------------------------------
// Row tables.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoAttests {
    event: String,
    attestation_from: DtoTextLocus,
    attestation_to: DtoTextLocus,
    provenance: String,
    justification: DtoJustification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoSuccession {
    narrative: String,
    chain: Vec<String>,
    provenance: String,
    justification: DtoJustification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoDatePlacement {
    AnchorBinding { anchor: String, years: i32, months: u8, days: u8 },
    ReignYear { reign: String, year_of_reign: u8 },
    SequenceAfter { prior: String, years: i32, months: u8, days: u8 },
    EraOnly { era: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
enum DtoPlacementBasis {
    Textual,
    Traditional,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoDatedBy {
    event: String,
    placement: DtoDatePlacement,
    basis: DtoPlacementBasis,
    justification: DtoJustification,
    provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoLocatedAt {
    event: String,
    place: String,
    provenance: String,
    justification: DtoJustification,
}

// M-D3 (owner ruling R2): `DtoNamed` retired alongside the `named`
// relation whole -- see graph-types' own edge.rs/graph.rs doc comments and
// this crate's place_adapter.rs for the full retirement.

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCatechismLink {
    locus: DtoTextLocus,
    item: String,
    provenance: String,
    justification: DtoJustification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoMentionedEntity {
    Place(String),
    Person(String),
    PeopleGroup(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoMentions {
    locus: DtoTextLocus,
    entity: DtoMentionedEntity,
    provenance: String,
}

/// M-C2: mirrors `graph_types::edge::CrossRef`'s own `to_last`/
/// `target_display` widening -- see that type's own doc comment.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCrossRef {
    from: DtoTextLocus,
    to: DtoTextLocus,
    to_last: Option<DtoTextLocus>,
    target_display: String,
    votes: u32,
    provenance: String,
}

// ---------------------------------------------------------------------
// The whole dump.
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoPlacedChronology {
    event: String,
    placement: DtoDatePlacement,
    basis: DtoPlacementBasis,
    justification: DtoJustification,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDump {
    format_version: u32,
    nodes: Vec<DtoNode>,
    reading: Vec<(String, Vec<DtoNodeId>)>,
    attests: Vec<DtoAttests>,
    succession: Vec<DtoSuccession>,
    dated_by: Vec<DtoDatedBy>,
    located_at: Vec<DtoLocatedAt>,
    // M-D3 (owner ruling R2): `named: Vec<DtoNamed>` field retired here --
    // the `named` relation is gone from the graph entirely (see graph-types'
    // own edge.rs/graph.rs).
    catechism: Vec<DtoCatechismLink>,
    mentions: Vec<DtoMentions>,
    cross_refs: Vec<DtoCrossRef>,
    /// The chronology companion (`event_world::Chronology`) -- NOT
    /// re-derivable from the row tables alone at load time without
    /// `AtlasData` (see `event_world::Chronology::from_derivation`'s own
    /// doc comment), so it rides in the artifact as its own fields:
    /// `chrono_order` mirrors `ChronologyDerivation.order` exactly;
    /// `chrono_placements` mirrors `.placements`; `temporal_neighbors`
    /// mirrors `Chronology.temporal_neighbors` directly (already exactly
    /// the pairs the field holds, so no further reconstruction needed).
    ///
    /// M-D3 (owner ruling R1, CORRECTING the note this field used to carry):
    /// `.resolved` was "NOT serialized -- confirmed unread by any production
    /// code path" as of M-C2/fix-round-1 -- true THEN, because
    /// `NodePayload::Event` carried its own from_year/to_year/order_key
    /// mirror and every reader used that instead. That mirror is GONE now
    /// (owner R1: "only keep narrative"), so `.resolved` -- and the genuine
    /// `to_year`/`order_key` `ResolvedPlacement` never carried (see
    /// `event_world::SourceEventMeta`'s own doc comment) -- became the SOLE
    /// remaining source `legacy::event_from_node`/`heading::
    /// build_heading_index` need, on EVERY startup path including
    /// `from_artifact`, which never has an `AtlasData` to re-derive years
    /// from. `chrono_years` carries exactly the three genuine per-event
    /// scalars neither `chrono_order` (ids only) nor `chrono_placements`
    /// (`DatePlacement`+basis, not a resolved year) supply -- `(event_id,
    /// from_year, to_year, order_key)`, iterated in `chrono_order` order
    /// (the SAME determinism discipline `chrono_placements` already
    /// follows). `seq` is NOT serialized a second time -- it is recomputed,
    /// on load, as the event's own position in `chrono_order`
    /// (`ChronologyDerivation.resolved`'s own construction: `SeqKey(i as
    /// u32)`, exactly this order's index) -- `order_key` here is the
    /// DIFFERENT, literal curated value `heading::precedence` does NOT need
    /// but `legacy::event_from_node`'s reconstruction must reproduce
    /// exactly (`SourceEventMeta`'s own doc comment has the full "seq is a
    /// faithful ordering substitute, not the same VALUE" argument).
    chrono_order: Vec<String>,
    chrono_placements: Vec<DtoPlacedChronology>,
    chrono_years: Vec<(String, i32, i32, i32)>,
    temporal_neighbors: Vec<(String, Option<String>, Option<String>)>,
    /// Startup-log-friendly counts (`build::BuildStats`/
    /// `event_world::EventWorldStats`) -- carried verbatim so
    /// `GraphService::from_artifact`'s own startup log is the SAME real
    /// numbers the compile step measured, not a recomputed guess (some of
    /// which, like `cites_dropped_negative_votes`, are facts about the
    /// SOURCE the loaded graph alone cannot recover at all).
    stats: BuildStats,
    event_world_stats: EventWorldStats,
}

/// Fix round 1 (I-2): bumped 1 -> 2. CONVENTION (closing the gap this
/// review round found -- FORMAT_VERSION had never once been bumped since
/// its M-C introduction, through two separate DTO wire-shape widenings):
/// bump this whenever ANY `Dto*` struct's own shape changes (a field
/// added/removed/retyped/renamed on any of the `Dto*` types above, or a
/// new relation table added to `ArtifactDump` itself) -- bincode is
/// non-self-describing (no field tags, confirmed at this file's own
/// `encode`/`decode`), so a stale artifact paired with new server code has
/// NOTHING ELSE catching a shape mismatch; `to_service_parts`'s own check
/// below is the entire safety net. This bump's own trigger: M-C2's
/// `DtoPayload::Event`/`Narrative` widening and `DtoCrossRef`'s
/// `to_last`/`target_display` addition (commit `5e07e8f`) -- both already
/// shipped as of `f1a27ab`'s own graph.bin regeneration, so this bump
/// itself is a housekeeping catch-up, not a new shape change of its own.
///
/// M-D3 (owner rulings R1+R2): bumped 2 -> 3. Trigger: `DtoPayload::Event`
/// drops `from_year`/`to_year`/`order_key` (R1 -- narrative-only payload);
/// `DtoNamed`/`ArtifactDump.named` retired whole (R2 -- the `named`
/// relation is gone); `ArtifactDump.chrono_years` added (R1 propagation --
/// `.resolved`/genuine `to_year` must now survive the artifact round trip,
/// see that field's own doc comment). `data/compiled/graph.bin` rebuilt in
/// this same commit (the suites-green-every-commit law).
const FORMAT_VERSION: u32 = 3;

/// Dumps a built `Graph`'s own row/node tables (NOT the derived indexes --
/// see this module's own doc comment) plus the chronology companion and
/// startup stats. Errors loudly if any of the currently-always-empty
/// tables (`contains_bible`/`contains_concord`/`quotes`/`confesses`/
/// `corresponds_bible`) is non-empty -- this format does not yet carry
/// them; extending it is a real, deliberate future act, not something
/// this batch silently punts by dropping rows.
pub fn dump(g: &Graph, chronology: &Chronology, stats: &BuildStats, event_world_stats: &EventWorldStats) -> Result<ArtifactDump, ArtifactError> {
    if !g.contains_bible.is_empty() || !g.contains_concord.is_empty() || !g.quotes.is_empty() || !g.confesses.is_empty() || !g.corresponds_bible.is_empty() {
        return Err(ArtifactError(
            "the graph carries rows in a relation this artifact format does not yet serialize (contains/quotes/confesses/corresponds) -- extend artifact.rs before shipping this content".into(),
        ));
    }

    let nodes = g.nodes.values().map(|n: &Node| DtoNode { id: (&n.id).into(), payload: payload_to_dto(&n.payload), provenance: n.provenance.clone() }).collect();

    let reading = g.reading.iter().map(|(corpus, spine): (&&str, &ReadingSpine)| (corpus.to_string(), spine.order.iter().map(DtoNodeId::from).collect())).collect();

    let attests = g
        .attests
        .iter()
        .map(|r: &Attests| {
            let (f, t) = bible_range_to_dto(&r.attestation);
            DtoAttests { event: r.event.0.clone(), attestation_from: f, attestation_to: t, provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();

    let succession = g
        .succession
        .iter()
        .map(|r: &Succession| DtoSuccession {
            narrative: r.narrative.0.clone(),
            chain: r.chain.iter().map(|e| e.0.clone()).collect(),
            provenance: r.provenance.clone(),
            justification: justification_to_dto(&r.justification),
        })
        .collect();

    let dated_by = g
        .dated_by
        .iter()
        .map(|r: &DatedBy| {
            let placement = match &r.placement {
                DatePlacement::AnchorBinding { anchor, offset } => DtoDatePlacement::AnchorBinding { anchor: anchor.0.clone(), years: offset.years, months: offset.months, days: offset.days },
                DatePlacement::ReignYear { reign, year_of_reign } => DtoDatePlacement::ReignYear { reign: reign.0.clone(), year_of_reign: *year_of_reign },
                DatePlacement::SequenceAfter { prior, spacing } => DtoDatePlacement::SequenceAfter { prior: prior.0.clone(), years: spacing.years, months: spacing.months, days: spacing.days },
                DatePlacement::EraOnly { era } => DtoDatePlacement::EraOnly { era: era.0.clone() },
            };
            let basis = match r.basis {
                PlacementBasis::Textual => DtoPlacementBasis::Textual,
                PlacementBasis::Traditional => DtoPlacementBasis::Traditional,
            };
            DtoDatedBy { event: r.event.0.clone(), placement, basis, justification: justification_to_dto(&r.justification), provenance: r.provenance.clone() }
        })
        .collect();

    let located_at = g
        .located_at
        .iter()
        .map(|r: &LocatedAt| DtoLocatedAt { event: r.event.0.clone(), place: r.place.0.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) })
        .collect();

    let catechism = g
        .catechism
        .iter()
        .map(|r: &CatechismLink| DtoCatechismLink { locus: (&r.locus).into(), item: r.item.0.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) })
        .collect();

    let mentions = g
        .mentions
        .iter()
        .map(|r: &Mentions| {
            let entity = match &r.entity {
                MentionedEntity::Place(p) => DtoMentionedEntity::Place(p.0.clone()),
                MentionedEntity::Person(p) => DtoMentionedEntity::Person(p.0.clone()),
                MentionedEntity::PeopleGroup(g) => DtoMentionedEntity::PeopleGroup(g.0.clone()),
            };
            DtoMentions { locus: (&r.locus).into(), entity, provenance: r.provenance.clone() }
        })
        .collect();

    let cross_refs = g
        .cross_refs
        .iter()
        .map(|r: &CrossRef| DtoCrossRef {
            from: (&r.from).into(),
            to: (&r.to).into(),
            to_last: r.to_last.as_ref().map(DtoTextLocus::from),
            target_display: r.target_display.clone(),
            votes: r.votes,
            provenance: r.provenance.clone(),
        })
        .collect();

    // Iterates `chronology.chrono.order` (a deterministic `Vec<String>`),
    // NOT `.placements` (a `HashMap`) directly -- the SAME discipline
    // `event_world::populate_dated_by` already established (a HashMap's
    // iteration order is randomized per-instance; a serialized artifact's
    // own row order must be a pure function of content, not of that).
    let chrono_order = chronology.chrono.order.clone();
    let chrono_placements = chronology
        .chrono
        .order
        .iter()
        .filter_map(|id| {
            let placed = chronology.chrono.placements.get(id)?;
            let placement = match &placed.placement {
                DatePlacement::AnchorBinding { anchor, offset } => DtoDatePlacement::AnchorBinding { anchor: anchor.0.clone(), years: offset.years, months: offset.months, days: offset.days },
                DatePlacement::ReignYear { reign, year_of_reign } => DtoDatePlacement::ReignYear { reign: reign.0.clone(), year_of_reign: *year_of_reign },
                DatePlacement::SequenceAfter { prior, spacing } => DtoDatePlacement::SequenceAfter { prior: prior.0.clone(), years: spacing.years, months: spacing.months, days: spacing.days },
                DatePlacement::EraOnly { era } => DtoDatePlacement::EraOnly { era: era.0.clone() },
            };
            let basis = match placed.basis {
                PlacementBasis::Textual => DtoPlacementBasis::Textual,
                PlacementBasis::Traditional => DtoPlacementBasis::Traditional,
            };
            Some(DtoPlacedChronology { event: id.clone(), placement, basis, justification: justification_to_dto(&placed.justification) })
        })
        .collect();
    let temporal_neighbors = chronology.chrono.order.iter().filter_map(|id| chronology.temporal_neighbors.get(id).map(|(p, f)| (id.clone(), p.clone(), f.clone()))).collect();
    // M-D3 (R1 propagation, ArtifactDump.chrono_years's own doc comment):
    // the genuine (from_year, to_year, order_key) triple -- `resolved.
    // date.from.year` is the resolved anchor; `chrono.source_meta`'s own
    // genuine, ordering-uninvolved companion supplies the possibly-wider
    // `to_year` and the literal curated `order_key`, never the resolver's
    // own always-equal `.date.to` / ordering-only `.seq`.
    let chrono_years: Vec<(String, i32, i32, i32)> = chronology
        .chrono
        .order
        .iter()
        .filter_map(|id| {
            let from_year = chronology.chrono.resolved.get(id)?.date.from.year.get();
            let meta = chronology.chrono.source_meta.get(id).copied().unwrap_or(crate::event_world::SourceEventMeta { to_year: from_year, order_key: 0 });
            Some((id.clone(), from_year, meta.to_year, meta.order_key))
        })
        .collect();

    Ok(ArtifactDump {
        format_version: FORMAT_VERSION,
        nodes,
        reading,
        attests,
        succession,
        dated_by,
        located_at,
        catechism,
        mentions,
        cross_refs,
        chrono_order,
        chrono_placements,
        chrono_years,
        temporal_neighbors,
        stats: stats.clone(),
        event_world_stats: event_world_stats.clone(),
    })
}

/// Rebuilds a `Graph` from a dump -- row tables + node table + reading
/// spine(s) ONLY, discarding the chronology/stats fields. The caller MUST
/// run `graph.build_indexes()` (and, for the M-B `justified-by` relation,
/// `event_world::add_justified_by`) afterward, exactly like every other
/// build path in this crate -- the derived indexes are never part of the
/// dump (this module's own doc comment). Prefer `to_service_parts` for any
/// caller that also needs the chronology companion (every real
/// `GraphService` does); this wrapper exists for callers (round-trip
/// tests, mainly) that only care about the graph itself.
pub fn to_graph(d: ArtifactDump) -> Result<Graph, ArtifactError> {
    to_service_parts(d).map(|(g, ..)| g)
}

/// The full reconstruction: graph + startup stats + the chronology
/// companion (`event_world::Chronology`) -- everything `GraphService::
/// from_artifact` needs, with NO `AtlasData` involved anywhere on this
/// path.
pub fn to_service_parts(d: ArtifactDump) -> Result<(Graph, BuildStats, EventWorldStats, Chronology), ArtifactError> {
    if d.format_version != FORMAT_VERSION {
        return Err(ArtifactError(format!("artifact format_version {} unsupported (this build understands {FORMAT_VERSION})", d.format_version)));
    }

    let mut g = Graph::default();

    for n in d.nodes {
        let id: AnyNodeId = n.id.into();
        g.nodes.insert(id.clone(), Node { id, payload: payload_from_dto(n.payload)?, provenance: n.provenance });
    }

    for (corpus, order) in d.reading {
        let corpus: &'static str = match corpus.as_str() {
            "bible" => "bible",
            "concord" => "concord",
            other => return Err(ArtifactError(format!("unknown reading-spine corpus '{other}' in serialized artifact"))),
        };
        g.reading.insert(corpus, ReadingSpine { order: order.into_iter().map(AnyNodeId::from).collect() });
    }

    for r in d.attests {
        g.attests.push(Attests {
            event: EventId::new(r.event),
            attestation: dto_to_bible_range(r.attestation_from, r.attestation_to)?,
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    for r in d.succession {
        let chain: Vec<EventId> = r.chain.into_iter().map(EventId::new).collect();
        let row = Succession::new(NarrativeId::new(r.narrative), chain, r.provenance, dto_to_justification(r.justification)?)
            .map_err(|e| ArtifactError(format!("malformed succession chain in serialized artifact: {e:?}")))?;
        g.succession.push(row);
    }

    for r in d.dated_by {
        let placement = match r.placement {
            DtoDatePlacement::AnchorBinding { anchor, years, months, days } => DatePlacement::AnchorBinding { anchor: AnchorId::new(anchor), offset: Duration { years, months, days } },
            DtoDatePlacement::ReignYear { reign, year_of_reign } => DatePlacement::ReignYear { reign: AnchorId::new(reign), year_of_reign },
            DtoDatePlacement::SequenceAfter { prior, years, months, days } => DatePlacement::SequenceAfter { prior: EventId::new(prior), spacing: Duration { years, months, days } },
            DtoDatePlacement::EraOnly { era } => DatePlacement::EraOnly { era: EraId::new(era) },
        };
        let basis = match r.basis {
            DtoPlacementBasis::Textual => PlacementBasis::Textual,
            DtoPlacementBasis::Traditional => PlacementBasis::Traditional,
        };
        g.dated_by.push(DatedBy { event: EventId::new(r.event), placement, basis, justification: dto_to_justification(r.justification)?, provenance: r.provenance });
    }

    for r in d.located_at {
        g.located_at.push(LocatedAt { event: EventId::new(r.event), place: PlaceId::new(r.place), provenance: r.provenance, justification: dto_to_justification(r.justification)? });
    }

    for r in d.catechism {
        g.catechism.push(CatechismLink { locus: r.locus.try_into()?, item: CatechismItemId::new(r.item), provenance: r.provenance, justification: dto_to_justification(r.justification)? });
    }

    for r in d.mentions {
        let entity = match r.entity {
            DtoMentionedEntity::Place(p) => MentionedEntity::Place(PlaceId::new(p)),
            DtoMentionedEntity::Person(p) => MentionedEntity::Person(PersonId::new(p)),
            DtoMentionedEntity::PeopleGroup(g) => MentionedEntity::PeopleGroup(PeopleGroupId::new(g)),
        };
        g.mentions.push(Mentions { locus: r.locus.try_into()?, entity, provenance: r.provenance });
    }

    for r in d.cross_refs {
        g.cross_refs.push(CrossRef {
            from: r.from.try_into()?,
            to: r.to.try_into()?,
            to_last: r.to_last.map(TextLocus::try_from).transpose()?,
            target_display: r.target_display,
            votes: r.votes,
            provenance: r.provenance,
        });
    }

    // The chronology companion (this module's own `ArtifactDump.chrono_*`
    // doc comment) -- reconstructed directly from the serialized fields,
    // no AtlasData involved.
    let mut placements = std::collections::HashMap::new();
    for p in d.chrono_placements {
        let placement = match p.placement {
            DtoDatePlacement::AnchorBinding { anchor, years, months, days } => DatePlacement::AnchorBinding { anchor: AnchorId::new(anchor), offset: Duration { years, months, days } },
            DtoDatePlacement::ReignYear { reign, year_of_reign } => DatePlacement::ReignYear { reign: AnchorId::new(reign), year_of_reign },
            DtoDatePlacement::SequenceAfter { prior, years, months, days } => DatePlacement::SequenceAfter { prior: EventId::new(prior), spacing: Duration { years, months, days } },
            DtoDatePlacement::EraOnly { era } => DatePlacement::EraOnly { era: EraId::new(era) },
        };
        let basis = match p.basis {
            DtoPlacementBasis::Textual => PlacementBasis::Textual,
            DtoPlacementBasis::Traditional => PlacementBasis::Traditional,
        };
        placements.insert(p.event, PlacedChronology { placement, basis, justification: dto_to_justification(p.justification)? });
    }
    // M-D3 (R1 propagation): `.resolved`/`.source_meta` reconstructed from
    // the serialized `chrono_years` rows -- `seq` is recomputed as each
    // event's own position in `chrono_order` (NOT re-serialized; see
    // `ArtifactDump.chrono_years`'s own doc comment for why this is exact,
    // not approximate: it is the identical formula `derive_chronology`
    // used to assign `seq` in the first place). Looked up via a real
    // position index over `chrono_order` -- never assumed to align
    // positionally with `chrono_years`'s own iteration order, so this stays
    // correct even if the two ever carry a different length (a general-kind
    // id's genuine absence from `chrono_years`, e.g.).
    let order_position: std::collections::HashMap<&str, usize> = d.chrono_order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
    let mut resolved = std::collections::HashMap::new();
    let mut source_meta = std::collections::HashMap::new();
    for (event_id, from_year, to_year, order_key) in d.chrono_years {
        let tp = TimePoint::year_only(Year::new(from_year).map_err(|_| ArtifactError(format!("chrono_years entry '{event_id}' carries year 0 in serialized artifact")))?);
        let seq = order_position.get(event_id.as_str()).copied().ok_or_else(|| ArtifactError(format!("chrono_years entry '{event_id}' names no position in chrono_order")))?;
        let basis = placements.get(&event_id).map(|p: &PlacedChronology| p.basis).unwrap_or(PlacementBasis::Traditional);
        resolved.insert(event_id.clone(), ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq: SeqKey(seq as u32), basis });
        source_meta.insert(event_id, crate::event_world::SourceEventMeta { to_year, order_key });
    }
    let chrono = ChronologyDerivation { order: d.chrono_order, placements, resolved, source_meta };
    let temporal_neighbors = d.temporal_neighbors.into_iter().map(|(id, prior, following)| (id, (prior, following))).collect();
    let chronology = Chronology { chrono, temporal_neighbors };

    Ok((g, d.stats, d.event_world_stats, chronology))
}

/// Encodes a dump to bytes -- `bincode`, this batch's own disclosed choice
/// (module doc comment above).
pub fn encode(d: &ArtifactDump) -> Result<Vec<u8>, ArtifactError> {
    bincode::serialize(d).map_err(|e| ArtifactError(format!("bincode encode failed: {e}")))
}

pub fn decode(bytes: &[u8]) -> Result<ArtifactDump, ArtifactError> {
    bincode::deserialize(bytes).map_err(|e| ArtifactError(format!("bincode decode failed: {e}")))
}

pub fn write_file(d: &ArtifactDump, path: &std::path::Path) -> Result<(), ArtifactError> {
    let bytes = encode(d)?;
    std::fs::write(path, bytes).map_err(|e| ArtifactError(format!("writing {}: {e}", path.display())))
}

pub fn read_file(path: &std::path::Path) -> Result<ArtifactDump, ArtifactError> {
    let bytes = std::fs::read(path).map_err(|e| ArtifactError(format!("reading {}: {e}", path.display())))?;
    decode(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::store::GraphQuery;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [
          { "chapter": 1, "verses": [
            { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
            { "verse": 2, "text": "And the earth was without form, and void." }
          ] }
        ] }
      ]
    }"#;
    // Fix round 1 (I-1): a SECOND row, a genuine range target (Gen.1.2 ->
    // Gen.1.1-Gen.1.2, both endpoints already inside KJV_FIXTURE below --
    // filter_missing_first_verse only requires the target's own FIRST verse
    // to exist, so no KJV_FIXTURE widening is needed) -- the single-verse
    // row above never constructs `to_last: Some(...)` at all, so this round-
    // trip test previously never proved that branch survives bincode.
    const XREFS_FIXTURE: &str = "From Verse\tTo Verse\tVotes\t#comment\nGen.1.1\tGen.1.2\t9\nGen.1.2\tGen.1.1-Gen.1.2\t4\n";

    fn small_atlas() -> atlas_core::data::AtlasData {
        use atlas_core::data::{AtlasData, Canon, ChronologyAnchor, Event, Place};
        use std::collections::HashMap;
        let places = vec![Place { id: "eden".into(), name: "Eden".into(), lat: 33.0, lon: 44.0, verse_links: vec!["GEN.1.1".into()] }];
        // M-D3: a genuine RANGE (-4004..-4000, not a point) plus a
        // nonzero, non-seq-shaped order_key (137, an arbitrary curated-
        // looking value, deliberately NOT equal to this event's own
        // timeline position 0) -- so this round-trip test actually
        // exercises `chrono_years`'s own from_year != to_year case AND
        // proves `order_key` survives as the literal value, not `seq`.
        let events = vec![Event { id: "creation".into(), label: "Creation".into(), when: atlas_core::time::TimeRange::new(-4004, -4000).unwrap(), places: vec!["eden".into()], order_key: 137, ..Default::default() }];
        let mut d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.chronology_anchors = vec![ChronologyAnchor { id: "creation-anchor".into(), label: "Creation".into(), year: -4004, event_id: Some("creation".into()), era_boundary: false, source: "test".into(), note: None }];
        d
    }

    fn built_graph() -> (Graph, Chronology, BuildStats, EventWorldStats) {
        let atlas = small_atlas();
        let (graph, stats, event_world_stats, chrono) = crate::build::build_graph_from_sources(KJV_FIXTURE, XREFS_FIXTURE, &atlas).unwrap();
        (graph, Chronology::from_derivation(chrono), stats, event_world_stats)
    }

    #[test]
    fn dump_and_to_graph_round_trip_losslessly_over_bincode_bytes() {
        let (original, chronology, stats, ews) = built_graph();
        let dumped = dump(&original, &chronology, &stats, &ews).expect("dump must succeed over a real built graph");
        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (mut reconstructed, _stats2, _ews2, chronology2) = to_service_parts(decoded).expect("to_service_parts must succeed");
        reconstructed.build_indexes();
        crate::event_world::add_justified_by(&mut reconstructed);

        let mut original_indexed = original;
        original_indexed.build_indexes();
        crate::event_world::add_justified_by(&mut original_indexed);

        // THE ADMISSION LAW (design §9a: "implementation #2 passes the
        // same law as #1"): the reconstructed graph answers every question
        // identically to the original.
        atlas_graph_types::store::assert_answers_match(&reconstructed, &original_indexed);

        // The chronology companion round-trips too (order + placements +
        // temporal_neighbors) -- not covered by assert_answers_match
        // (which only ever sees the graph/port side).
        assert_eq!(chronology2.chrono.order, chronology.chrono.order);
        assert_eq!(chronology2.temporal_neighbors.len(), chronology.temporal_neighbors.len());

        // M-D3 (R1 propagation): `.resolved`/`.to_year` must ALSO survive
        // the round trip now -- `legacy::event_from_node`/`heading::
        // build_heading_index` depend on them on EVERY startup path,
        // including this one (`from_artifact`, which never has an
        // `AtlasData` to re-derive years from).
        let before = &chronology.chrono.resolved["creation"];
        let after = &chronology2.chrono.resolved["creation"];
        assert_eq!(after.date.from.year.get(), before.date.from.year.get(), "resolved from_year must round-trip");
        assert_eq!(after.date.from.year.get(), -4004, "sanity: creation's own real from_year");
        assert_eq!(after.seq, before.seq, "resolved seq must round-trip");
        assert_eq!(after.basis, before.basis, "resolved basis must round-trip");
        assert_eq!(chronology2.chrono.source_meta["creation"], chronology.chrono.source_meta["creation"], "the genuine source_meta companion (to_year + order_key) must round-trip");
        assert_eq!(chronology2.chrono.source_meta["creation"].to_year, -4000, "sanity: creation's own real, WIDER to_year -- the exact case ResolvedPlacement.date.to alone would have collapsed to -4004");
        assert_eq!(chronology2.chrono.source_meta["creation"].order_key, 137, "sanity: creation's own real, literal order_key -- the exact case seq (always 0, this fixture's only dated event) would NOT have reproduced");

        // Fix round 1 (I-1): to_last/target_display are NOT visible through
        // the GraphQuery port at all (EdgeEntry carries only the target's
        // first verse + vote count -- this struct's own `cross_refs_by_from`
        // doc comment in service.rs explains why), so assert_answers_match
        // above cannot catch a `to_last`/`target_display` round-trip bug by
        // construction. Read the raw `cross_refs` Vec directly instead --
        // the one field on this struct genuinely un-reachable through the
        // generic port.
        let find_from_gen_1_2 = |g: &Graph| {
            g.cross_refs
                .iter()
                .find(|r| r.from == TextLocus::from(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 2 })))
                .cloned()
                .expect("Gen.1.2 -> Gen.1.1-Gen.1.2 must survive the pipeline")
        };
        let before = find_from_gen_1_2(&original_indexed);
        let after = find_from_gen_1_2(&reconstructed);
        assert_eq!(before.to, TextLocus::from(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })), "fixture sanity: to is the range's own FIRST verse, Gen.1.1");
        assert_eq!(before.to_last, Some(TextLocus::from(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 2 }))), "fixture sanity: to_last is the range's own LAST verse, Gen.1.2");
        assert_eq!(after.to_last, before.to_last, "to_last must round-trip through bincode, not silently become None");
        assert_eq!(after.target_display, before.target_display, "target_display must round-trip losslessly");
        assert_eq!(after.target_display, "GEN.1.1-2", "sanity: the compressed same-chapter canonical form");
    }

    #[test]
    fn to_service_parts_rejects_a_mismatched_format_version_cleanly() {
        // Fix round 1 (I-2): FORMAT_VERSION had never been bumped despite
        // two consecutive DTO wire-shape widenings this crate's own history
        // carries (Event/Narrative payload widening; CrossRef to_last/
        // target_display) -- bincode is non-self-describing (no field tags),
        // so a stale artifact paired with new server code previously had
        // NOTHING catching the shape mismatch except this one already-
        // written check, which had ZERO test coverage of its own. A real
        // ArtifactDump (not a hand-built stub -- every private field stays
        // real and valid except the one under test) with its format_version
        // mutated to a value this build does not understand must fail loud,
        // with the named, purpose-built error -- never a raw bincode decode
        // panic/garbage struct.
        let (graph, chronology, stats, ews) = built_graph();
        let mut dumped = dump(&graph, &chronology, &stats, &ews).unwrap();
        assert_eq!(dumped.format_version, FORMAT_VERSION, "fixture sanity: dump() must stamp the CURRENT constant");
        dumped.format_version = FORMAT_VERSION + 1;

        // Not `.expect_err(...)`/`.unwrap_err()`: both require the Ok side
        // (`(Graph, BuildStats, EventWorldStats, Chronology)`) to implement
        // `Debug`, which `Chronology` does not (unrelated to this fix
        // round, not changed here) -- a manual match sidesteps that
        // entirely without adding a derive this test doesn't otherwise need.
        let err = match to_service_parts(dumped) {
            Ok(_) => panic!("a future/mismatched format_version must be rejected, not silently accepted"),
            Err(e) => e,
        };
        let msg = err.to_string();
        assert!(msg.contains("format_version"), "error must name the field: {msg}");
        assert!(msg.contains(&(FORMAT_VERSION + 1).to_string()), "error must name the artifact's OWN (rejected) version: {msg}");
        assert!(msg.contains(&FORMAT_VERSION.to_string()), "error must name what this build DOES understand: {msg}");
    }

    #[test]
    fn dump_rejects_a_graph_carrying_an_unsupported_relations_rows() {
        let mut g = Graph::default();
        g.quotes.push(atlas_graph_types::edge::Quotes {
            quoting: TextLocus::from(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })),
            quoted: LocusRange::new(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap(),
            provenance: "test".into(),
        });
        let empty_chrono = Chronology::from_derivation(ChronologyDerivation::default());
        assert!(dump(&g, &empty_chrono, &BuildStats::default(), &EventWorldStats::default()).is_err(), "an unsupported-but-populated relation must fail loud, not silently drop rows");
    }

    #[test]
    fn node_payloads_survive_the_round_trip_with_real_values() {
        let (original, chronology, stats, ews) = built_graph();
        let dumped = dump(&original, &chronology, &stats, &ews).unwrap();
        let reconstructed = to_graph(dumped).unwrap();

        let id = crate::kjv_adapter::verse_node_id(0, 1, 1);
        let before = original.node(&id).unwrap();
        let after = reconstructed.node(&id).unwrap();
        assert_eq!(format!("{:?}", before.payload), format!("{:?}", after.payload), "TextUnit payload must survive byte-for-byte");

        let place_node_id = atlas_graph_types::id::PlaceId::new("eden").erase();
        let before_place = original.node(&place_node_id).unwrap();
        let after_place = reconstructed.node(&place_node_id).unwrap();
        assert_eq!(format!("{:?}", before_place.payload), format!("{:?}", after_place.payload), "Place payload (lat/lon/aliases) must survive");
    }
}
