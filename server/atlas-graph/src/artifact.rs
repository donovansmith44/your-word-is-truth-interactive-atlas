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
//! SCOPE, disclosed: `Graph`'s `contains_bible`/`quotes`/`confesses`/
//! `corresponds_bible` tables are ALWAYS EMPTY as of this batch (no
//! adapter populates Bible-Container/Quotes/Confesses/Corresponds rows
//! anywhere in this codebase -- confirmed by reading every adapter
//! fresh). `dump` asserts each is empty and returns a loud, named error if
//! not, rather than silently discarding real data a future batch might
//! add without updating this module -- "everything fail-loud," not a
//! silent drop. Batch CORP-2a CLOSES the `contains_concord` member of that
//! set: `concord_adapter.rs` is the first real caller of `Contains<
//! ConcordTag>` in this codebase, so `DtoContains`/`concord_locus_to_dto`
//! below are real, tested, serialized content now -- not the placeholder
//! this doc comment (and the crate's own "no real caller yet" note,
//! previously living where `concord_locus_to_dto` is defined below) used
//! to describe.

use std::collections::BTreeMap;

use atlas_graph_types::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};
use atlas_graph_types::edge::{
    Attests, CatechismLink, CommentsOn, CrossRef, Fulfills, Ground, Justification, LocatedAt, Mentions, MentionedEntity, NamedAfter, Namesake, SpokenAt, SpokenBy, Succession, Typology,
};
use atlas_graph_types::graph::{Graph, ReadingSpine};
use atlas_graph_types::id::{AnchorId, AnyNodeId, CatechismItemId, CommentaryItemId, EraId, EventId, NarrativeId, NodeKind, PeopleGroupId, PersonId, PlaceId, PolityId, SourceId};
use atlas_graph_types::node::{Node, NodePayload, PolityEraPayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, ConcordRef, LocusRange, SvebiliusRef, TextLocus, TextRef, TokenSpan, TranslationId, VerseRef};

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
    /// KRETZ-1: appended (bincode decodes enum variants by index, so an
    /// APPENDED variant is benign for reading an older artifact -- this
    /// module's own `FORMAT_VERSION` doc comment has the full "appended
    /// variant vs. added field" distinction, PG-1's own precedent).
    CommentaryItem,
    /// SVEB-1: appended, same discipline as CommentaryItem above.
    CatechismTopic,
    SvebiliusUnit,
    /// PARTS-1: appended, same discipline as CommentaryItem above.
    CatechismPart,
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
            NodeKind::CommentaryItem => DtoNodeKind::CommentaryItem,
            NodeKind::CatechismTopic => DtoNodeKind::CatechismTopic,
            NodeKind::SvebiliusUnit => DtoNodeKind::SvebiliusUnit,
            NodeKind::CatechismPart => DtoNodeKind::CatechismPart,
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
            DtoNodeKind::CommentaryItem => NodeKind::CommentaryItem,
            DtoNodeKind::CatechismTopic => NodeKind::CatechismTopic,
            DtoNodeKind::SvebiliusUnit => NodeKind::SvebiliusUnit,
            DtoNodeKind::CatechismPart => NodeKind::CatechismPart,
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
    Place { canonical: String, lat: f64, lon: f64, aliases: Vec<String>, description: Option<String> },
    /// Batch P: mirrors `NodePayload::Person`'s own widening -- see that
    /// variant's own doc comment.
    Person { label: String, gender: Option<String>, birth_year: Option<i32>, death_year: Option<i32>, also_called: Vec<String>, description: Option<String> },
    Anchor { year: i32, month: Option<u8>, day: Option<u8>, citation: String },
    Era { label: String, from_year: i32, to_year: i32 },
    Polity { label: String, color_key: u8, eras: Vec<DtoPolityEra> },
    CatechismItem { label: String },
    Source { label: String },
    Translation { label: String },
    PeopleGroup { label: String, description: Option<String> },
    /// KRETZ-1: mirrors `NodePayload::CommentaryItem` -- `work` is the
    /// commentary work's `Source` node id (a plain `String`, the SAME "id
    /// carried as a bare string, re-typed on read" convention every other
    /// node-id-carrying Dto field already uses).
    CommentaryItem { work: String, heading: Option<String>, text: String },
    /// SVEB-1: appended.
    CatechismTopic { label: String, source: String },
    SvebiliusUnit { question: Option<String>, answer: String },
    /// PARTS-1: appended. `curated` rides on the node so a reader never
    /// has to infer from an id whether a part is one Luther numbered.
    CatechismPart { label: String, curated: bool },
}

fn payload_to_dto(p: &NodePayload) -> DtoPayload {
    match p {
        NodePayload::CommentaryItem { work, heading, text } => DtoPayload::CommentaryItem { work: work.0.clone(), heading: heading.clone(), text: text.clone() },
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
        NodePayload::Place { canonical, lat, lon, aliases, description } => DtoPayload::Place { canonical: canonical.clone(), lat: *lat, lon: *lon, aliases: aliases.clone(), description: description.clone() },
        NodePayload::Person { label, gender, birth_year, death_year, also_called, description } => DtoPayload::Person {
            label: label.clone(),
            gender: gender.clone(),
            birth_year: *birth_year,
            death_year: *death_year,
            also_called: also_called.clone(),
            description: description.clone(),
        },
        NodePayload::Anchor { at, citation } => DtoPayload::Anchor { year: at.year.get(), month: at.month, day: at.day, citation: citation.clone() },
        NodePayload::Era { label, from_year, to_year } => DtoPayload::Era { label: label.clone(), from_year: *from_year, to_year: *to_year },
        NodePayload::Polity { label, color_key, eras } => DtoPayload::Polity { label: label.clone(), color_key: *color_key, eras: eras.iter().map(DtoPolityEra::from).collect() },
        NodePayload::CatechismItem { label } => DtoPayload::CatechismItem { label: label.clone() },
        NodePayload::Source { label } => DtoPayload::Source { label: label.clone() },
        NodePayload::Translation { label } => DtoPayload::Translation { label: label.clone() },
        NodePayload::PeopleGroup { label, description } => DtoPayload::PeopleGroup { label: label.clone(), description: description.clone() },
        NodePayload::CatechismTopic { label, source } => DtoPayload::CatechismTopic { label: label.clone(), source: source.clone() },
        NodePayload::CatechismPart { label, curated } => DtoPayload::CatechismPart { label: label.clone(), curated: *curated },
        NodePayload::SvebiliusUnit { question, answer } => DtoPayload::SvebiliusUnit { question: question.clone(), answer: answer.clone() },
    }
}

fn payload_from_dto(d: DtoPayload) -> Result<NodePayload, ArtifactError> {
    Ok(match d {
        DtoPayload::TextUnit { corpus, renderings } => {
            let corpus: &'static str = match corpus.as_str() {
                "bible" => "bible",
                "concord" => "concord",
                "svebilius" => "svebilius",
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
        DtoPayload::Place { canonical, lat, lon, aliases, description } => NodePayload::Place { canonical, lat, lon, aliases, description },
        DtoPayload::Person { label, gender, birth_year, death_year, also_called, description } => {
            NodePayload::Person { label, gender, birth_year, death_year, also_called, description }
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
        DtoPayload::PeopleGroup { label, description } => NodePayload::PeopleGroup { label, description },
        DtoPayload::CatechismTopic { label, source } => NodePayload::CatechismTopic { label, source },
        DtoPayload::CatechismPart { label, curated } => NodePayload::CatechismPart { label, curated },
        DtoPayload::SvebiliusUnit { question, answer } => NodePayload::SvebiliusUnit { question, answer },
        DtoPayload::CommentaryItem { work, heading, text } => NodePayload::CommentaryItem { work: SourceId::new(work), heading, text },
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
    /// SVEB-1: appended.
    Svebilius { section: u8, unit: u16 },
}
impl From<&TextRef> for DtoTextRef {
    fn from(r: &TextRef) -> Self {
        match r {
            TextRef::Bible(v) => DtoTextRef::Bible { book: v.book, chapter: v.chapter, verse: v.verse },
            TextRef::Concord(c) => DtoTextRef::Concord { part: c.part, article: c.article, paragraph: c.paragraph },
            TextRef::Svebilius(v) => DtoTextRef::Svebilius { section: v.section, unit: v.unit },
        }
    }
}
impl From<DtoTextRef> for TextRef {
    fn from(d: DtoTextRef) -> Self {
        match d {
            DtoTextRef::Bible { book, chapter, verse } => TextRef::Bible(VerseRef { book, chapter, verse }),
            DtoTextRef::Concord { part, article, paragraph } => TextRef::Concord(ConcordRef { part, article, paragraph }),
            DtoTextRef::Svebilius { section, unit } => TextRef::Svebilius(SvebiliusRef { section, unit }),
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

/// CORP-2a: the Concord-corpus sibling of `bible_locus_to_dto`/
/// `dto_to_bible_locus` above -- SAME shape, narrowed to `ConcordTag`
/// instead of `BibleTag`. This is the real caller `graph-types`' own
/// `TextLocus::as_bible` doc comment anticipated a Concord counterpart
/// for; rather than widen that OWNER-APPROVED, EXTEND-ONLY method with a
/// sibling `as_concord` (a graph-types shape change for a purely
/// artifact-boundary concern), the narrowing is done here instead, by a
/// direct match on `TextRef` (already public) -- zero graph-types changes.
fn concord_locus_to_dto(l: &atlas_graph_types::text::ConcordLocus) -> DtoTextLocus {
    DtoTextLocus::from(&TextLocus::from(atlas_graph_types::text::ConcordLocus { unit: l.unit.clone(), span: l.span.clone() }))
}
/// SVEB-1: mirrors `concord_locus_to_dto`/`dto_to_concord_locus` exactly.
fn svebilius_locus_to_dto(l: &atlas_graph_types::text::SvebiliusLocus) -> DtoTextLocus {
    DtoTextLocus::from(&TextLocus::from(atlas_graph_types::text::SvebiliusLocus { unit: l.unit.clone(), span: l.span.clone() }))
}
fn dto_to_svebilius_locus(d: DtoTextLocus) -> Result<atlas_graph_types::text::SvebiliusLocus, ArtifactError> {
    let tl = TextLocus::try_from(d)?;
    match tl.at {
        TextRef::Svebilius(v) => Ok(atlas_graph_types::text::Locus { unit: v, span: tl.span }),
        TextRef::Bible(_) => Err(ArtifactError("expected a Svebilius-corpus locus in serialized artifact, found a Bible-corpus one".into())),
        TextRef::Concord(_) => Err(ArtifactError("expected a Svebilius-corpus locus in serialized artifact, found a Concord-corpus one".into())),
    }
}

fn dto_to_concord_locus(d: DtoTextLocus) -> Result<atlas_graph_types::text::ConcordLocus, ArtifactError> {
    let tl = TextLocus::try_from(d)?;
    match tl.at {
        TextRef::Concord(c) => Ok(atlas_graph_types::text::Locus { unit: c, span: tl.span }),
        TextRef::Bible(_) => Err(ArtifactError("expected a Concord-corpus locus in serialized artifact, found a Bible-corpus one".into())),
        TextRef::Svebilius(_) => Err(ArtifactError("expected a Concord-corpus locus in serialized artifact, found a Svebilius-corpus one".into())),
    }
}

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

/// PG-1a: mirrors `graph_types::edge::Namesake` -- the three kinds
/// Scripture actually names after persons. Every row this batch's own
/// adapter (`peoples_adapter.rs`) builds is `PeopleGroup`; `Place`/`Polity`
/// ride here for the SAME reason the typed enum carries all three
/// (schema completeness, not speculative -- a future batch reaching for
/// either needs no artifact-format change, only a row).
#[derive(Debug, Clone, Serialize, Deserialize)]
enum DtoNamesake {
    PeopleGroup(String),
    Place(String),
    Polity(String),
}

/// PG-1a: mirrors `graph_types::edge::NamedAfter` -- eponymy, curated.
/// `dump`'s own guard fired on this table the moment `peoples_adapter`
/// started emitting real rows (this module's own `dump` doc comment); it
/// is real, serialized content now, no longer guarded.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoNamedAfter {
    namesake: DtoNamesake,
    eponym: String,
    provenance: String,
    justification: DtoJustification,
}

/// EDGE-1a: mirrors `graph_types::edge::Fulfills` -- the SAME `{from, to}`
/// pair-of-`DtoTextLocus` shape `DtoAttests.attestation_{from,to}` already
/// establishes, one relation wider (two ranges, not one). `dump`'s own
/// guard fired on this table (and `typology` below) the moment
/// `fulfillment_adapter` started emitting real rows -- exactly the EDGE-1/
/// PG-1/TRAV-1/PG-1a precedent this module's own `dump` doc comment names
/// as the expected trigger class.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoFulfills {
    prophecy_from: DtoTextLocus,
    prophecy_to: DtoTextLocus,
    fulfillment_from: DtoTextLocus,
    fulfillment_to: DtoTextLocus,
    provenance: String,
    justification: DtoJustification,
}

/// EDGE-1a: mirrors `graph_types::edge::Typology` -- same shape as
/// `DtoFulfills` above, plus the optional `note` (the figure's own display
/// name, e.g. "the brasen serpent").
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoTypology {
    type_from: DtoTextLocus,
    type_to: DtoTextLocus,
    antitype_from: DtoTextLocus,
    antitype_to: DtoTextLocus,
    note: Option<String>,
    provenance: String,
    justification: DtoJustification,
}

/// CORP-2a: mirrors `graph_types::edge::Contains<ConcordTag>` -- `content`
/// (a `LocusSet<ConcordTag>`, i.e. a `BTreeSet<ConcordLocus>`) serializes
/// as a plain `Vec` (bincode has no native set type; `BTreeSet`'s own
/// iteration is already deterministic ascending order, so round-tripping
/// through a `Vec` and back through a set constructor is lossless and
/// order-independent either way).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoContains {
    container: String,
    content: Vec<DtoTextLocus>,
    provenance: String,
    justification: DtoJustification,
}

/// SVEB-1: mirrors `graph_types::edge::Quotes` -- `quoting` is a GENERIC
/// `TextLocus` (any corpus may quote Scripture; Svebilius is the first that
/// does), `quoted` a Bible range carried as the same `{from,to}` pair
/// `DtoAttests` already uses. No `justification` field: `Quotes` itself
/// carries none.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoQuotes {
    quoting: DtoTextLocus,
    quoted_from: DtoTextLocus,
    quoted_to: DtoTextLocus,
    provenance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCatechismLink {
    locus: DtoTextLocus,
    item: String,
    /// SVEB-1: the owning topical grouping's node id, `None` for an
    /// item-level citation. An ADDED FIELD (not an appended variant), so
    /// `FORMAT_VERSION` had to move -- see its own doc comment.
    topic: Option<String>,
    provenance: String,
    justification: DtoJustification,
}

/// PARTS-1: mirrors `graph_types::edge::CatechismPartLink` -- the same
/// shape `DtoCatechismLink` uses, with `part` where it has `item`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCatechismPartLink {
    locus: DtoTextLocus,
    part: String,
    topic: Option<String>,
    provenance: String,
    justification: DtoJustification,
}

/// PARTS-1: mirrors `graph_types::edge::CatechismMembership` -- which
/// chief part contains which item. Structure, so it carries no locus and
/// no justification: nothing here is a claim about a text.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCatechismMembership {
    part: String,
    item: String,
    provenance: String,
}

/// KRETZ-1: mirrors `graph_types::edge::CommentsOn` -- the SAME `{node id,
/// range-as-a-{from,to}-pair-of-DtoTextLocus, provenance, justification}`
/// shape `DtoAttests` already establishes (`event`/`attestation` there,
/// `item`/`on` here).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoCommentsOn {
    item: String,
    on_from: DtoTextLocus,
    on_to: DtoTextLocus,
    provenance: String,
    justification: DtoJustification,
}

/// RED-1: mirrors `graph_types::edge::SpokenBy` -- the SAME `{node id,
/// range-as-a-{from,to}-pair-of-DtoTextLocus, provenance, justification}`
/// shape `DtoCommentsOn` already establishes (`item`/`on` there,
/// `speaker`/`locus` here).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoSpokenBy {
    locus_from: DtoTextLocus,
    locus_to: DtoTextLocus,
    speaker: String,
    provenance: String,
    justification: DtoJustification,
}

/// RED-1: mirrors `graph_types::edge::SpokenAt` -- the SAME shape as
/// `DtoSpokenBy` immediately above, one relation wider (place instead of
/// person).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoSpokenAt {
    locus_from: DtoTextLocus,
    locus_to: DtoTextLocus,
    place: String,
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

/// TRAV-1: mirrors `graph_types::edge::TemporalAdjacency` -- `earlier`/
/// `later` are event ids, carried honestly (never re-derived by a
/// consumer) exactly like the typed row's own doc comment names. No
/// `DtoJustification` field: the row itself carries none (it is
/// compile-derived, not authored -- see that type's own doc comment).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DtoTemporalAdjacency {
    earlier: String,
    later: String,
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
    /// PG-1a: `graph.named_after`'s own row table -- see `DtoNamedAfter`'s
    /// own doc comment.
    named_after: Vec<DtoNamedAfter>,
    /// EDGE-1a: `graph.fulfills`'s own row table -- see `DtoFulfills`'s own
    /// doc comment.
    fulfills: Vec<DtoFulfills>,
    /// EDGE-1a: `graph.typology`'s own row table -- see `DtoTypology`'s own
    /// doc comment.
    typology: Vec<DtoTypology>,
    /// CORP-2a: `graph.contains_concord`'s own row table -- see
    /// `DtoContains`'s own doc comment. The FIRST populated `Contains<C>`
    /// table in this codebase (`contains_bible` stays empty/guarded,
    /// this batch's own scope note above).
    contains_concord: Vec<DtoContains>,
    /// SVEB-1: `graph.contains_svebilius`'s own row table -- the THIRD
    /// populated `Contains<C>`. Serialized here for the same reason
    /// CORP-2a serialized the second: an emitted row that no serializer
    /// carries is a SILENT DROP, and this batch emits real ones.
    contains_svebilius: Vec<DtoContains>,
    /// SVEB-1: `graph.quotes`'s own row table -- the TWELFTH member of the
    /// original guarded set to close, on the same schedule as every prior
    /// data batch (the moment `svebilius_adapter` started emitting real
    /// rows).
    quotes: Vec<DtoQuotes>,
    catechism: Vec<DtoCatechismLink>,
    /// PARTS-1: the part-level citation and membership row tables. Brand
    /// new, so -- exactly like `contains_svebilius` before them -- they
    /// are serialized in the SAME batch that introduces them; the dump
    /// guard cannot catch a table it has never heard of.
    catechism_part: Vec<DtoCatechismPartLink>,
    catechism_membership: Vec<DtoCatechismMembership>,
    /// KRETZ-1: `graph.comments_on`'s own row table -- see `DtoCommentsOn`'s
    /// own doc comment. The NINTH member of the original guarded set
    /// (`dump`'s own doc comment) to close, on the same schedule as every
    /// prior data batch.
    comments_on: Vec<DtoCommentsOn>,
    /// RED-1: `graph.spoken_by`/`graph.spoken_at`'s own row tables -- see
    /// `DtoSpokenBy`/`DtoSpokenAt`'s own doc comments. The TENTH and
    /// ELEVENTH members of the original guarded set (`dump`'s own doc
    /// comment) to close, on the same schedule as every prior data batch
    /// (`comments_on` immediately above was the ninth).
    spoken_by: Vec<DtoSpokenBy>,
    spoken_at: Vec<DtoSpokenAt>,
    mentions: Vec<DtoMentions>,
    cross_refs: Vec<DtoCrossRef>,
    /// TRAV-1: the real `temporal_adjacency` row table (controller decision
    /// 1) -- a plain row table like `succession`/`located_at` above, not
    /// part of the chronology-companion group below (it lives on `Graph`
    /// itself, not on `Chronology`). `GraphService::assemble` (service.rs)
    /// folds these rows into its own `temporal_neighbors` lookup at load
    /// time on EVERY path (from-sources and from-artifact alike), so
    /// nothing else needs to ride the artifact for that -- see this
    /// struct's own former `temporal_neighbors` field, retired below.
    temporal_adjacency: Vec<DtoTemporalAdjacency>,
    /// The chronology companion (`event_world::Chronology`) -- NOT
    /// re-derivable from the row tables alone at load time without
    /// `AtlasData` (see `event_world::Chronology::from_derivation`'s own
    /// doc comment), so it rides in the artifact as its own fields:
    /// `chrono_order` mirrors `ChronologyDerivation.order` exactly;
    /// `chrono_placements` mirrors `.placements`.
    ///
    /// TRAV-1: this group's own former `temporal_neighbors` field (mirroring
    /// `Chronology.temporal_neighbors`, itself retired -- see that struct's
    /// own doc comment) is GONE: that fact is now cheaply refoldable from
    /// `temporal_adjacency` above at `GraphService::assemble` time, so
    /// serializing a second, redundant copy of it here would be exactly the
    /// "two representations of one fact" shape dead-code law forbids.
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
///
/// Writer window 2026-08-24 (PG-1 + ENT-1): bumped 3 -> 4. Triggers:
/// PG-1 APPENDED enum variants (`DtoMentionedEntity::PeopleGroup`,
/// `DtoNodeKind::PeopleGroup`, `DtoPayload::PeopleGroup`) -- benign for
/// READING an old artifact (bincode decodes by index; appended variants
/// never collide) but a version-3 reader handed a version-4 artifact
/// carrying the new variants would not be; ENT-1 ADDED FIELDS to
/// existing variants (`description` on Place/Person/PeopleGroup in
/// `DtoPayload`) -- a genuine wire-shape break both directions.
/// `data/compiled/graph.bin` rebuilt in this same commit.
///
/// TRAV-1 (2026-08-24): bumped 4 -> 5. Two triggers, landed together:
/// `ArtifactDump.temporal_adjacency: Vec<DtoTemporalAdjacency>` ADDED (a
/// new relation table -- `dump`'s own guard forced this the moment the ETL
/// started emitting real `temporal_adjacency` rows, per this module's own
/// doc comment); `ArtifactDump.temporal_neighbors` REMOVED (the
/// `Chronology.temporal_neighbors` field it mirrored is retired whole --
/// `GraphService::assemble` now folds `temporal_adjacency` into an
/// equivalent lookup at load time instead, so serializing both was two
/// representations of one fact). Both are genuine wire-shape breaks both
/// directions -- bincode field COUNT changed, not just a value inside an
/// existing field. `data/compiled/graph.bin` rebuilt in this same commit.
///
/// PG-1a (2026-08-24): bumped 5 -> 6. Trigger: `ArtifactDump.named_after:
/// Vec<DtoNamedAfter>` ADDED (a new relation table -- `dump`'s own guard
/// forced this the moment `peoples_adapter` started emitting real
/// `named_after` rows, exactly the TRAV-1 precedent immediately above
/// named as the expected trigger class: "the EDGE-1/PG-1 data batches hit
/// this guard ON PURPOSE"). A genuine wire-shape break both directions --
/// bincode field COUNT changed. `data/compiled/graph.bin` rebuilt in this
/// same commit.
///
/// EDGE-1a (2026-08-24): bumped 6 -> 7. Trigger: `ArtifactDump.fulfills:
/// Vec<DtoFulfills>` and `ArtifactDump.typology: Vec<DtoTypology>` ADDED
/// (two new relation tables -- `dump`'s own guard forced this the moment
/// `fulfillment_adapter` started emitting real `fulfills`/`typology` rows,
/// the SIXTH and SEVENTH members of the original guarded set to close,
/// same schedule as every prior data batch: "the EDGE-1/PG-1 data batches
/// hit this guard ON PURPOSE"). A genuine wire-shape break both
/// directions -- bincode field COUNT changed (two fields, one commit).
/// `data/compiled/graph.bin` rebuilt in this same commit.
/// CORP-2a (2026-08-24): bumped 7 -> 8. Trigger: `ArtifactDump.
/// contains_concord: Vec<DtoContains>` ADDED (a new relation table --
/// `dump`'s own guard forced this the moment `concord_adapter` started
/// emitting real `contains_concord` rows, the EIGHTH member of the
/// original guarded set to close, same schedule as every prior data
/// batch). A genuine wire-shape break both directions -- bincode field
/// COUNT changed. `data/compiled/graph.bin` rebuilt in this same commit.
///
/// KRETZ-1 (2026-08-25): bumped 8 -> 9. Triggers, landed together:
/// `DtoNodeKind::CommentaryItem`/`DtoPayload::CommentaryItem { work,
/// heading, text }` ADDED (appended enum variants -- benign for reading an
/// OLDER artifact, the SAME "appended, never inserted" discipline the PG-1
/// writer-window note above already established, but a version-8 reader
/// handed a version-9 artifact carrying real CommentaryItem nodes would not
/// decode them correctly); `ArtifactDump.comments_on: Vec<DtoCommentsOn>`
/// ADDED (a new relation table -- `dump`'s own guard forced this the
/// moment `kretzmann_adapter` started emitting real `comments_on` rows and
/// CommentaryItem nodes, the NINTH member of the original guarded set to
/// close, same schedule as every prior data batch). A genuine wire-shape
/// break both directions -- bincode field COUNT changed. `data/compiled/
/// graph.bin` rebuilt in this same commit.
///
/// RED-1 (2026-08-25): bumped 9 -> 10. Trigger: `ArtifactDump.spoken_by:
/// Vec<DtoSpokenBy>` and `ArtifactDump.spoken_at: Vec<DtoSpokenAt>` ADDED
/// (two new relation tables -- `dump`'s own guard forced this the moment
/// `red_letter_adapter` started emitting real `spoken_by`/`spoken_at` rows,
/// the TENTH and ELEVENTH members of the original guarded set to close,
/// same schedule as every prior data batch). A genuine wire-shape break
/// both directions -- bincode field COUNT changed (two fields, one
/// commit). VERSION ROOT DOES NOT MOVE (verified, not assumed): the
/// content-addressed root hashes every node's own id+payload only (design
/// doc §9b) -- this batch adds no node kind and changes no node payload,
/// only two new EDGE tables and a compiled-data-side span table, neither
/// of which the root's own hash touches; `tests/version_root_regression.rs`
/// proves this for the real committed data in the same commit. `data/
/// compiled/graph.bin` rebuilt in this same commit.
// SVEB-1: 10 -> 11. Three Dto enums gained appended variants
// (DtoNodeKind, DtoPayload, DtoTextRef) and the corpus whitelist learned
// "svebilius" -- appended variants are benign to READ from an older
// artifact, but the standing convention here is to bump on any Dto shape
// change at all, not only on breaking ones.
// SVEB-1: 11 -> 12. `ArtifactDump` gained two row tables
// (`contains_svebilius`, `quotes`) on top of this batch's own earlier
// enum widenings -- a new table is the clearest possible Dto shape change.
const FORMAT_VERSION: u32 = 13;

/// Dumps a built `Graph`'s own row/node tables (NOT the derived indexes --
/// see this module's own doc comment) plus the chronology companion and
/// startup stats. Errors loudly if any of the currently-always-empty
/// tables (`contains_bible`/`quotes`/`confesses`/`corresponds_bible`) is
/// non-empty -- this format does not yet carry them; extending it is a
/// real, deliberate future act, not something this batch silently punts
/// by dropping rows. The EDGE-1/PG-1 data batches hit this guard ON
/// PURPOSE: serializing their rows is part of each batch's own scope.
/// TRAV-1 (2026-08-24) closed the FOURTH member of the original set
/// (`temporal_adjacency`); PG-1a (2026-08-24) closed the FIFTH
/// (`named_after`); EDGE-1a (2026-08-24) closed the SIXTH and SEVENTH
/// (`fulfills`/`typology`); CORP-2a (2026-08-24) closed the EIGHTH
/// (`contains_concord`); KRETZ-1 (2026-08-25) closed the NINTH
/// (`comments_on`, alongside CommentaryItem nodes) the same way, on
/// schedule, the moment `kretzmann_adapter` started emitting real rows;
/// RED-1 (2026-08-25) closes the TENTH and ELEVENTH (`spoken_by`/
/// `spoken_at`) the same way, the moment `red_letter_adapter` started
/// emitting real rows; SVEB-1 (2026-08-30) closes the TWELFTH (`quotes`,
/// alongside the new `contains_svebilius` table) the moment
/// `svebilius_adapter` started emitting real rows -- all are REAL
/// SERIALIZED CONTENT below now, no longer guarded.
///
/// `contains_svebilius` was never IN this guard (it did not exist), which
/// is precisely why it is worth naming here: a brand-new row table is the
/// one case the guard cannot catch on its own, so serializing it is part
/// of the same batch that introduces it, never a later cleanup.
pub fn dump(g: &Graph, chronology: &Chronology, stats: &BuildStats, event_world_stats: &EventWorldStats) -> Result<ArtifactDump, ArtifactError> {
    if !g.contains_bible.is_empty() || !g.confesses.is_empty() || !g.corresponds_bible.is_empty() {
        return Err(ArtifactError(
            "the graph carries rows in a relation this artifact format does not yet serialize (contains_bible/confesses/corresponds) -- extend artifact.rs before shipping this content".into(),
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

    // PG-1a: `graph.named_after`'s own row table -- a plain authored-row
    // mapping, the SAME shape `located_at` above already is (endpoints +
    // provenance + justification), one arm wider for the three-variant
    // `Namesake` sum.
    let named_after = g
        .named_after
        .iter()
        .map(|r: &NamedAfter| {
            let namesake = match &r.namesake {
                Namesake::PeopleGroup(g) => DtoNamesake::PeopleGroup(g.0.clone()),
                Namesake::Place(p) => DtoNamesake::Place(p.0.clone()),
                Namesake::Polity(p) => DtoNamesake::Polity(p.0.clone()),
            };
            DtoNamedAfter { namesake, eponym: r.eponym.0.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();

    // EDGE-1a: `graph.fulfills`/`graph.typology`'s own row tables -- the
    // SAME `{from, to}`-pair-of-loci shape `attests` above already uses,
    // one relation wider each.
    let fulfills = g
        .fulfills
        .iter()
        .map(|r: &Fulfills| {
            let (prophecy_from, prophecy_to) = bible_range_to_dto(&r.prophecy);
            let (fulfillment_from, fulfillment_to) = bible_range_to_dto(&r.fulfillment);
            DtoFulfills { prophecy_from, prophecy_to, fulfillment_from, fulfillment_to, provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();

    let typology = g
        .typology
        .iter()
        .map(|r: &Typology| {
            let (type_from, type_to) = bible_range_to_dto(&r.type_passage);
            let (antitype_from, antitype_to) = bible_range_to_dto(&r.antitype_passage);
            DtoTypology { type_from, type_to, antitype_from, antitype_to, note: r.note.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();

    // CORP-2a: `graph.contains_concord`'s own row table -- see
    // `DtoContains`'s own doc comment.
    let contains_concord = g
        .contains_concord
        .iter()
        .map(|r: &atlas_graph_types::edge::Contains<atlas_graph_types::text::ConcordTag>| DtoContains {
            container: r.container.0.clone(),
            content: r.content.0.iter().map(concord_locus_to_dto).collect(),
            provenance: r.provenance.clone(),
            justification: justification_to_dto(&r.justification),
        })
        .collect();

    // SVEB-1: the third `Contains<C>` table, dumped exactly as the second.
    let contains_svebilius = g
        .contains_svebilius
        .iter()
        .map(|r: &atlas_graph_types::edge::Contains<atlas_graph_types::text::SvebiliusTag>| DtoContains {
            container: r.container.0.clone(),
            content: r.content.0.iter().map(svebilius_locus_to_dto).collect(),
            provenance: r.provenance.clone(),
            justification: justification_to_dto(&r.justification),
        })
        .collect();

    let quotes = g
        .quotes
        .iter()
        .map(|r: &atlas_graph_types::edge::Quotes| DtoQuotes {
            quoting: (&r.quoting).into(),
            quoted_from: (&TextLocus::from(r.quoted.from.clone())).into(),
            quoted_to: (&TextLocus::from(r.quoted.to.clone())).into(),
            provenance: r.provenance.clone(),
        })
        .collect();

    let catechism = g
        .catechism
        .iter()
        .map(|r: &CatechismLink| DtoCatechismLink { locus: (&r.locus).into(), item: r.item.0.clone(), topic: r.topic.as_ref().map(|t| t.0.clone()), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) })
        .collect();

    let catechism_part = g
        .catechism_part
        .iter()
        .map(|r: &atlas_graph_types::edge::CatechismPartLink| DtoCatechismPartLink {
            locus: (&r.locus).into(),
            part: r.part.0.clone(),
            topic: r.topic.as_ref().map(|t| t.0.clone()),
            provenance: r.provenance.clone(),
            justification: justification_to_dto(&r.justification),
        })
        .collect();

    let catechism_membership = g
        .catechism_membership
        .iter()
        .map(|r: &atlas_graph_types::edge::CatechismMembership| DtoCatechismMembership {
            part: r.part.0.clone(),
            item: r.item.0.clone(),
            provenance: r.provenance.clone(),
        })
        .collect();

    // KRETZ-1: `graph.comments_on`'s own row table -- the SAME `{from, to}`
    // range shape `attests` above already uses, one relation wider.
    let comments_on = g
        .comments_on
        .iter()
        .map(|r: &CommentsOn| {
            let (on_from, on_to) = bible_range_to_dto(&r.on);
            DtoCommentsOn { item: r.item.0.clone(), on_from, on_to, provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();

    // RED-1: `graph.spoken_by`/`graph.spoken_at`'s own row tables -- the
    // SAME `{from, to}` range shape `attests`/`comments_on` above already
    // use, one relation wider each (a node-typed field alongside the
    // range, like `comments_on`'s own `item`).
    let spoken_by = g
        .spoken_by
        .iter()
        .map(|r: &SpokenBy| {
            let (locus_from, locus_to) = bible_range_to_dto(&r.locus);
            DtoSpokenBy { locus_from, locus_to, speaker: r.speaker.0.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
        .collect();
    let spoken_at = g
        .spoken_at
        .iter()
        .map(|r: &SpokenAt| {
            let (locus_from, locus_to) = bible_range_to_dto(&r.locus);
            DtoSpokenAt { locus_from, locus_to, place: r.place.0.clone(), provenance: r.provenance.clone(), justification: justification_to_dto(&r.justification) }
        })
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

    // TRAV-1: a plain row table straight off `g` (like `cross_refs` above),
    // NOT the chronology companion below -- `graph.temporal_adjacency`'s
    // own insertion order is already a pure function of `chrono.order`
    // (`event_world::populate_temporal_adjacency`'s own `windows(2)` walk),
    // so no extra determinism care is needed here beyond preserving it.
    let temporal_adjacency = g
        .temporal_adjacency
        .iter()
        .map(|r| DtoTemporalAdjacency { earlier: r.earlier.0.clone(), later: r.later.0.clone(), provenance: r.provenance.clone() })
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
        named_after,
        fulfills,
        typology,
        contains_concord,
        contains_svebilius,
        catechism_part,
        catechism_membership,
        quotes,
        catechism,
        comments_on,
        spoken_by,
        spoken_at,
        mentions,
        cross_refs,
        temporal_adjacency,
        chrono_order,
        chrono_placements,
        chrono_years,
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

    // PERF-m1 (the disclosed follow-up named in `BENCHMARKS.md`'s own
    // `artifact_load` "disclosed floor, not chased this batch" note --
    // verbatim: "`to_service_parts`'s own per-relation DTO->domain
    // conversion loops are STRUCTURALLY the same shape `build_indexes`'s
    // row-lowering loops were before this batch's fix -- a disclosed,
    // low-risk follow-up (chunk-then-concatenate a `Vec`, simpler than
    // `build_indexes`'s own chunk-then-merge-a-map, since a flat `Vec`
    // split needs no per-key ordering argument at all)"). Applied here,
    // verbatim to that prescription, to the four DOMINANT row tables by
    // count on the real committed artifact (`M-C ARTIFACT SOURCE GRAPH:
    // 31102 text units, 343558 cites edges, ...` -- `nodes` and
    // `cross_refs`/`cites` are the two largest by a wide margin;
    // `comments_on` (50,602 rows, KRETZ-1) and `mentions` are the next
    // tier): `std::thread::scope`, chunk size off
    // `std::thread::available_parallelism()`, each chunk runs the EXACT
    // SAME per-row conversion the sequential version ran, handles are
    // joined and their chunks concatenated IN ORIGINAL VECTOR ORDER
    // (`Vec::extend`, never a re-sort) -- the same "handles processed in
    // vector order regardless of which thread the OS finishes first"
    // guarantee `Graph::build_indexes`'s own doc comment states, so the
    // resulting row tables are bit-for-bit identical in content to the
    // pre-parallel sequential version; only wall-clock changes. Every
    // SMALLER row table below (curated-data-sized -- hundreds to low
    // thousands of rows: attests/succession/dated_by/located_at/
    // named_after/fulfills/typology/contains_concord/catechism/spoken_by/
    // spoken_at/temporal_adjacency) stays sequential, unchanged from
    // before this batch -- thread-spawn overhead is not worth it at that
    // size, and leaving them alone keeps this diff minimal and easy to
    // verify against the pre-existing code.
    fn owned_chunks<T>(v: Vec<T>, chunk_size: usize) -> Vec<Vec<T>> {
        let mut it = v.into_iter();
        let mut out = Vec::new();
        loop {
            let chunk: Vec<T> = (&mut it).take(chunk_size).collect();
            if chunk.is_empty() {
                break;
            }
            out.push(chunk);
        }
        out
    }

    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    // Lengths captured BEFORE each field is moved into `owned_chunks` below
    // (argument evaluation would otherwise try to read the just-moved
    // field a second time for its own chunk-size computation).
    let node_chunk_size = d.nodes.len().div_ceil(n_threads).max(1);
    let cross_ref_chunk_size = d.cross_refs.len().div_ceil(n_threads).max(1);
    let comments_on_chunk_size = d.comments_on.len().div_ceil(n_threads).max(1);
    let mentions_chunk_size = d.mentions.len().div_ceil(n_threads).max(1);
    let node_chunks = owned_chunks(d.nodes, node_chunk_size);
    let cross_ref_chunks = owned_chunks(d.cross_refs, cross_ref_chunk_size);
    let comments_on_chunks = owned_chunks(d.comments_on, comments_on_chunk_size);
    let mentions_chunks = owned_chunks(d.mentions, mentions_chunk_size);

    let (node_rows, cross_ref_rows, comments_on_rows, mentions_rows): (Vec<(AnyNodeId, Node)>, Vec<CrossRef>, Vec<CommentsOn>, Vec<Mentions>) = std::thread::scope(|scope| -> Result<_, ArtifactError> {
        let node_handles: Vec<_> = node_chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || -> Result<Vec<(AnyNodeId, Node)>, ArtifactError> {
                    chunk
                        .into_iter()
                        .map(|n| {
                            let id: AnyNodeId = n.id.into();
                            Ok((id.clone(), Node { id, payload: payload_from_dto(n.payload)?, provenance: n.provenance }))
                        })
                        .collect()
                })
            })
            .collect();

        let cross_ref_handles: Vec<_> = cross_ref_chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || -> Result<Vec<CrossRef>, ArtifactError> {
                    chunk
                        .into_iter()
                        .map(|r| {
                            Ok(CrossRef {
                                from: r.from.try_into()?,
                                to: r.to.try_into()?,
                                to_last: r.to_last.map(TextLocus::try_from).transpose()?,
                                target_display: r.target_display,
                                votes: r.votes,
                                provenance: r.provenance,
                            })
                        })
                        .collect()
                })
            })
            .collect();

        let comments_on_handles: Vec<_> = comments_on_chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || -> Result<Vec<CommentsOn>, ArtifactError> {
                    chunk
                        .into_iter()
                        .map(|r| {
                            Ok(CommentsOn { item: CommentaryItemId::new(r.item), on: dto_to_bible_range(r.on_from, r.on_to)?, provenance: r.provenance, justification: dto_to_justification(r.justification)? })
                        })
                        .collect()
                })
            })
            .collect();

        let mentions_handles: Vec<_> = mentions_chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || -> Result<Vec<Mentions>, ArtifactError> {
                    chunk
                        .into_iter()
                        .map(|r| {
                            let entity = match r.entity {
                                DtoMentionedEntity::Place(p) => MentionedEntity::Place(PlaceId::new(p)),
                                DtoMentionedEntity::Person(p) => MentionedEntity::Person(PersonId::new(p)),
                                DtoMentionedEntity::PeopleGroup(pg) => MentionedEntity::PeopleGroup(PeopleGroupId::new(pg)),
                            };
                            Ok(Mentions { locus: r.locus.try_into()?, entity, provenance: r.provenance })
                        })
                        .collect()
                })
            })
            .collect();

        let mut node_rows = Vec::new();
        for h in node_handles {
            node_rows.extend(h.join().expect("node-conversion worker panicked")?);
        }
        let mut cross_ref_rows = Vec::new();
        for h in cross_ref_handles {
            cross_ref_rows.extend(h.join().expect("cross-ref-conversion worker panicked")?);
        }
        let mut comments_on_rows = Vec::new();
        for h in comments_on_handles {
            comments_on_rows.extend(h.join().expect("comments-on-conversion worker panicked")?);
        }
        let mut mentions_rows = Vec::new();
        for h in mentions_handles {
            mentions_rows.extend(h.join().expect("mentions-conversion worker panicked")?);
        }

        Ok((node_rows, cross_ref_rows, comments_on_rows, mentions_rows))
    })?;

    let mut g = Graph::default();

    for (id, node) in node_rows {
        g.nodes.insert(id, node);
    }

    for (corpus, order) in d.reading {
        let corpus: &'static str = match corpus.as_str() {
            "bible" => "bible",
            "concord" => "concord",
            // SVEB-1: the third corpus with a reading spine of its own.
            "svebilius" => "svebilius",
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

    // PG-1a: a plain row table, like `located_at` above.
    for r in d.named_after {
        let namesake = match r.namesake {
            DtoNamesake::PeopleGroup(g) => Namesake::PeopleGroup(PeopleGroupId::new(g)),
            DtoNamesake::Place(p) => Namesake::Place(PlaceId::new(p)),
            DtoNamesake::Polity(p) => Namesake::Polity(PolityId::new(p)),
        };
        g.named_after.push(NamedAfter { namesake, eponym: PersonId::new(r.eponym), provenance: r.provenance, justification: dto_to_justification(r.justification)? });
    }

    // EDGE-1a: plain row tables, like `attests`/`located_at` above.
    for r in d.fulfills {
        g.fulfills.push(Fulfills {
            prophecy: dto_to_bible_range(r.prophecy_from, r.prophecy_to)?,
            fulfillment: dto_to_bible_range(r.fulfillment_from, r.fulfillment_to)?,
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }
    for r in d.typology {
        g.typology.push(Typology {
            type_passage: dto_to_bible_range(r.type_from, r.type_to)?,
            antitype_passage: dto_to_bible_range(r.antitype_from, r.antitype_to)?,
            note: r.note,
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    // CORP-2a: a plain row table, like `attests`/`located_at` above --
    // `content` reconstructs as a `BTreeSet` (`LocusSet`'s own inner
    // shape); `Vec` -> `BTreeSet` is lossless regardless of the serialized
    // order (`DtoContains`'s own doc comment).
    for r in d.contains_concord {
        let content: Result<std::collections::BTreeSet<_>, ArtifactError> = r.content.into_iter().map(dto_to_concord_locus).collect();
        g.contains_concord.push(atlas_graph_types::edge::Contains {
            container: atlas_graph_types::id::ContainerNodeId::new(r.container),
            content: atlas_graph_types::text::LocusSet(content?),
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    for r in d.contains_svebilius {
        let content: Result<std::collections::BTreeSet<_>, ArtifactError> = r.content.into_iter().map(dto_to_svebilius_locus).collect();
        g.contains_svebilius.push(atlas_graph_types::edge::Contains {
            container: atlas_graph_types::id::ContainerNodeId::new(r.container),
            content: atlas_graph_types::text::LocusSet(content?),
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    for r in d.quotes {
        let from = dto_to_bible_locus(r.quoted_from)?;
        let to = dto_to_bible_locus(r.quoted_to)?;
        let quoted = atlas_graph_types::text::LocusRange::new(from, to)
            .map_err(|_| ArtifactError("inverted quoted range in serialized artifact".into()))?;
        g.quotes.push(atlas_graph_types::edge::Quotes {
            quoting: r.quoting.try_into()?,
            quoted,
            provenance: r.provenance,
        });
    }

    for r in d.catechism {
        g.catechism.push(CatechismLink { locus: r.locus.try_into()?, item: CatechismItemId::new(r.item), topic: r.topic.map(atlas_graph_types::id::CatechismTopicId::new), provenance: r.provenance, justification: dto_to_justification(r.justification)? });
    }

    for r in d.catechism_part {
        g.catechism_part.push(atlas_graph_types::edge::CatechismPartLink {
            locus: r.locus.try_into()?,
            part: atlas_graph_types::id::CatechismPartId::new(r.part),
            topic: r.topic.map(atlas_graph_types::id::CatechismTopicId::new),
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    for r in d.catechism_membership {
        g.catechism_membership.push(atlas_graph_types::edge::CatechismMembership {
            part: atlas_graph_types::id::CatechismPartId::new(r.part),
            item: CatechismItemId::new(r.item),
            provenance: r.provenance,
        });
    }

    // KRETZ-1: a plain row table, like `attests`/`fulfills` above --
    // PERF-m1: already converted chunk-parallel above (`comments_on_rows`,
    // one of the four dominant tables), so this is a plain move, not a
    // per-row loop, unlike its sibling tables here.
    g.comments_on = comments_on_rows;

    // RED-1: plain row tables, like `attests`/`fulfills`/`comments_on`
    // above.
    for r in d.spoken_by {
        g.spoken_by.push(SpokenBy {
            locus: dto_to_bible_range(r.locus_from, r.locus_to)?,
            speaker: PersonId::new(r.speaker),
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }
    for r in d.spoken_at {
        g.spoken_at.push(SpokenAt {
            locus: dto_to_bible_range(r.locus_from, r.locus_to)?,
            place: PlaceId::new(r.place),
            provenance: r.provenance,
            justification: dto_to_justification(r.justification)?,
        });
    }

    // PERF-m1: already converted chunk-parallel above (`mentions_rows`, one
    // of the four dominant tables) -- plain move, not a per-row loop.
    g.mentions = mentions_rows;

    // PERF-m1: already converted chunk-parallel above (`cross_ref_rows` --
    // `cross_refs`/`cites` is "by far the largest single relation," the
    // BENCHMARKS.md-named primary target of this follow-up) -- plain move,
    // not a per-row loop.
    g.cross_refs = cross_ref_rows;

    // TRAV-1: a plain row table, like `cross_refs` above -- NOT part of
    // the chronology companion reconstruction below.
    for r in d.temporal_adjacency {
        g.temporal_adjacency.push(atlas_graph_types::edge::TemporalAdjacency { earlier: EventId::new(r.earlier), later: EventId::new(r.later), provenance: r.provenance });
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
    // TRAV-1: no `temporal_neighbors` field left to reconstruct here --
    // `GraphService::assemble` folds `g.temporal_adjacency` (populated
    // above) into its own equivalent lookup at load time instead (service.rs's
    // own doc comment), on EVERY path, this one included.
    let chronology = Chronology { chrono };

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

        // The chronology companion round-trips too (order + placements) --
        // not covered by assert_answers_match (which only ever sees the
        // graph/port side). TRAV-1: `temporal_adjacency` itself IS a graph
        // row table now (not part of `Chronology`), so it rides inside
        // `assert_answers_match` above via the symmetric index/port --
        // `small_atlas()`'s own single dated event ("creation") produces
        // zero rows either side of the round trip (`windows(2)` on a
        // length-1 order), which this equality still honestly proves
        // (0 == 0, not skipped); see
        // `temporal_adjacency_round_trips_losslessly_with_real_rows` below
        // for the >=2-event case that actually exercises non-empty rows.
        assert_eq!(chronology2.chrono.order, chronology.chrono.order);
        assert_eq!(reconstructed.temporal_adjacency.len(), original_indexed.temporal_adjacency.len());

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

    // TRAV-1: `small_atlas()`'s own single dated event ("creation") can
    // never exercise a NON-EMPTY `temporal_adjacency` round trip (`windows(2)`
    // on a length-1 order is empty) -- this fixture is a fresh, dedicated
    // two-event atlas specifically so this test proves real rows (not just
    // zero rows both sides) survive dump -> encode -> decode ->
    // to_service_parts losslessly, values included, not merely a count.
    #[test]
    fn temporal_adjacency_round_trips_losslessly_with_real_rows() {
        use atlas_core::data::{AtlasData, Canon, Event};
        use std::collections::HashMap;

        let events = vec![
            Event { id: "first".into(), label: "First".into(), when: atlas_core::time::TimeRange::new(-2000, -2000).unwrap(), ..Default::default() },
            Event { id: "second".into(), label: "Second".into(), when: atlas_core::time::TimeRange::new(-1000, -1000).unwrap(), ..Default::default() },
        ];
        let atlas = AtlasData::new(Canon { books: vec![] }, vec![], events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let (graph, stats, event_world_stats, chrono) = crate::build::build_graph_from_sources(KJV_FIXTURE, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas).unwrap();
        assert_eq!(graph.temporal_adjacency.len(), 1, "fixture sanity: two dated events -> windows(2) -> exactly one consecutive pair");
        assert_eq!(graph.temporal_adjacency[0].earlier.0, "first");
        assert_eq!(graph.temporal_adjacency[0].later.0, "second");

        let chronology = Chronology::from_derivation(chrono);
        let dumped = dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed with real temporal_adjacency rows -- the guard's whole point was to force this extension, not to forbid the content forever");
        assert_eq!(dumped.temporal_adjacency.len(), 1);
        assert_eq!(dumped.temporal_adjacency[0].earlier, "first");
        assert_eq!(dumped.temporal_adjacency[0].later, "second");
        assert_eq!(dumped.temporal_adjacency[0].provenance, "chronology-derivation");

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, _stats2, _ews2, chronology2) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.temporal_adjacency.len(), 1, "the row must survive the full round trip, not silently drop");
        assert_eq!(reconstructed.temporal_adjacency[0].earlier.0, "first");
        assert_eq!(reconstructed.temporal_adjacency[0].later.0, "second");
        assert_eq!(reconstructed.temporal_adjacency[0].provenance, "chronology-derivation");
        assert_eq!(chronology2.chrono.order, vec!["first".to_string(), "second".to_string()]);
    }

    // PG-1a: `small_atlas()`'s own fixture carries no `named_after_seeds`
    // (and so, through the real `peoples_adapter::normalize` path, zero
    // `named_after` rows) -- this is a fresh, dedicated fixture proving a
    // REAL row, built through the actual production adapter (not a
    // hand-built stub), survives dump -> encode -> decode ->
    // to_service_parts losslessly, values included, the SAME
    // "real rows, not just zero rows both sides" discipline
    // `temporal_adjacency_round_trips_losslessly_with_real_rows` already
    // establishes for its own relation.
    #[test]
    fn named_after_round_trips_losslessly_with_a_real_row() {
        use atlas_core::data::{AtlasData, Canon, NamedAfterSeed, Person, PeopleGroupSeed, ScriptureGroundSeed};
        use std::collections::HashMap;

        let people = vec![Person { id: "ben-ammi_451".into(), name: "Ben-ammi".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], verse_links: vec![], dict_text: None }];
        let mut atlas = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        atlas.people = people;
        atlas.people_group_seeds = vec![PeopleGroupSeed { id: "ammonites".into(), label: "Ammonites".into() }];
        atlas.named_after_seeds = vec![NamedAfterSeed {
            namesake_kind: "people_group".into(),
            namesake_id: "ammonites".into(),
            eponym: "ben-ammi_451".into(),
            text: Some("She called his name Ben-ammi.".into()),
            grounds: vec![ScriptureGroundSeed { from: "GEN.19.38".into(), to: None }],
        }];

        let (graph, stats, event_world_stats, chrono) = crate::build::build_graph_from_sources(KJV_FIXTURE, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas).unwrap();
        assert_eq!(graph.named_after.len(), 1, "fixture sanity: the real peoples_adapter::normalize path must have built exactly one row");
        assert_eq!(graph.named_after[0].eponym.0, "ben-ammi_451");

        let chronology = Chronology::from_derivation(chrono);
        let dumped = dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed with a real named_after row -- the guard's whole point was to force this extension, not to forbid the content forever");
        assert_eq!(dumped.named_after.len(), 1);
        assert_eq!(dumped.named_after[0].eponym, "ben-ammi_451");
        assert!(matches!(&dumped.named_after[0].namesake, DtoNamesake::PeopleGroup(g) if g == "ammonites"));
        assert_eq!(dumped.named_after[0].justification.text.as_deref(), Some("She called his name Ben-ammi."));
        assert_eq!(dumped.named_after[0].justification.grounds.len(), 1);

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, _stats2, _ews2, _chronology2) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.named_after.len(), 1, "the row must survive the full round trip, not silently drop");
        let row = &reconstructed.named_after[0];
        assert_eq!(row.eponym.0, "ben-ammi_451");
        match &row.namesake {
            Namesake::PeopleGroup(g) => assert_eq!(g.0, "ammonites"),
            other => panic!("expected Namesake::PeopleGroup, got {other:?}"),
        }
        assert_eq!(row.justification.text.as_deref(), Some("She called his name Ben-ammi."));
        assert_eq!(row.justification.grounds.len(), 1, "the Ground::Scripture(GEN.19.38) ground must round-trip");
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(_)));
    }

    /// EDGE-1a: the SAME "real row, through the real production adapter,
    /// not a hand-built stub" discipline `named_after_round_trips_losslessly_
    /// with_a_real_row` immediately above already establishes, for
    /// `graph.fulfills`. `KJV_FIXTURE`'s own Gen 1:1-2 is reused as BOTH
    /// endpoints (a real, if theologically arbitrary, text-to-text pair --
    /// this test proves the WIRE ROUND TRIP, not the content's own
    /// truth, which the curated `data/curated/fulfillments.toml` real-data
    /// tests own separately).
    #[test]
    fn fulfills_round_trips_losslessly_with_a_real_row() {
        use atlas_core::data::{AtlasData, Canon, FulfillmentSeed, ScriptureGroundSeed};
        use std::collections::HashMap;

        let mut atlas = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        atlas.fulfillment_seeds = vec![FulfillmentSeed {
            prophecy: ScriptureGroundSeed { from: "GEN.1.1".into(), to: None },
            fulfillment: ScriptureGroundSeed { from: "GEN.1.2".into(), to: None },
            text: "a real fulfillment-formula quote".into(),
        }];

        let (graph, stats, event_world_stats, chrono) = crate::build::build_graph_from_sources(KJV_FIXTURE, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas).unwrap();
        assert_eq!(graph.fulfills.len(), 1, "fixture sanity: the real fulfillment_adapter::normalize path must have built exactly one row");

        let chronology = Chronology::from_derivation(chrono);
        let dumped = dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed with a real fulfills row -- the guard's whole point was to force this extension, not to forbid the content forever");
        assert_eq!(dumped.fulfills.len(), 1);
        assert_eq!(dumped.fulfills[0].justification.text.as_deref(), Some("a real fulfillment-formula quote"));
        assert_eq!(dumped.fulfills[0].justification.grounds.len(), 1, "the fulfillment passage self-attests as its own ground");

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, _stats2, _ews2, _chronology2) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.fulfills.len(), 1, "the row must survive the full round trip, not silently drop");
        let row = &reconstructed.fulfills[0];
        let gen_1_1 = BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 });
        let gen_1_2 = BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 2 });
        assert_eq!(row.prophecy, BibleLocusRange::new(gen_1_1.clone(), gen_1_1.clone()).unwrap());
        assert_eq!(row.fulfillment, BibleLocusRange::new(gen_1_2.clone(), gen_1_2.clone()).unwrap());
        assert_eq!(row.justification.text.as_deref(), Some("a real fulfillment-formula quote"));
        assert_eq!(row.justification.grounds.len(), 1);
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(_)));
    }

    /// KRETZ-1: the CommentaryItem/comments_on sibling of `fulfills_round_
    /// trips_losslessly_with_a_real_row` above -- proves the guard shrink
    /// (`dump`'s own doc comment, "closes the NINTH member") with a REAL
    /// `kretzmann_adapter::normalize` build, not a hand-built row (the
    /// node payload AND the row must both survive, and `dump` must no
    /// longer refuse them).
    #[test]
    fn commentary_item_and_comments_on_round_trip_losslessly_with_a_real_kretzmann_build() {
        use atlas_etl::kretzmann::{ChapterStats, KretzUnit, KretzmannCorpus, ParsedChapter, UnitKind};

        let corpus = KretzmannCorpus {
            chapters: vec![ParsedChapter {
                book_index: 0,
                chapter: 1,
                units: vec![KretzUnit {
                    id: "kretzmann/0.1.0".to_string(),
                    book_index: 0,
                    chapter: 1,
                    verse_from: 1,
                    verse_to: 1,
                    kind: UnitKind::Verse,
                    heading: Some("The Creation of the World.".to_string()),
                    text: "In the beginning, cp. John 1, 1.".to_string(),
                }],
                fragments: vec![],
                stats: ChapterStats::default(),
            }],
            stats: Default::default(),
        };

        let atlas = crate::event_world::empty_atlas();
        let (graph, stats, event_world_stats, chrono) =
            crate::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(KJV_FIXTURE, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas, &[], None, None, Some(&corpus))
                .expect("a real build with Kretzmann data must succeed");
        assert_eq!(graph.comments_on.len(), 1, "fixture sanity: kretzmann_adapter::normalize must have built exactly one row");

        let chronology = Chronology::from_derivation(chrono);
        let dumped = dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed with real CommentaryItem nodes + comments_on rows -- the guard's whole point was to force this extension");
        assert_eq!(dumped.comments_on.len(), 1);

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, ..) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.comments_on.len(), 1, "the row must survive the full round trip, not silently drop");
        let row = &reconstructed.comments_on[0];
        assert_eq!(row.item.0, "kretzmann/0.1.0");
        let gen_1_1 = BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 });
        assert_eq!(row.on, BibleLocusRange::new(gen_1_1.clone(), gen_1_1).unwrap());
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(_)), "grounded in the lemma's own locus (decision 4)");

        let node_id = atlas_graph_types::id::CommentaryItemId::new("kretzmann/0.1.0".to_string()).erase();
        let node = reconstructed.nodes.get(&node_id).expect("the CommentaryItem node itself must survive the round trip");
        match &node.payload {
            NodePayload::CommentaryItem { work, heading, text } => {
                assert_eq!(work.0, crate::kretzmann_adapter::KRETZMANN_SOURCE_ID);
                assert_eq!(heading.as_deref(), Some("The Creation of the World."));
                assert_eq!(text, "In the beginning, cp. John 1, 1.");
            }
            other => panic!("expected CommentaryItem payload, got {other:?}"),
        }
    }

    /// EDGE-1a: the Typology sibling of `fulfills_round_trips_losslessly_
    /// with_a_real_row` immediately above -- same discipline, `graph.
    /// typology`'s own table, plus the `note` field neither `DtoFulfills`
    /// nor `DtoNamedAfter` carries.
    #[test]
    fn typology_round_trips_losslessly_with_a_real_row() {
        use atlas_core::data::{AtlasData, Canon, ScriptureGroundSeed, TypologySeed};
        use std::collections::HashMap;

        let mut atlas = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        atlas.typology_seeds = vec![TypologySeed {
            type_passage: ScriptureGroundSeed { from: "GEN.1.1".into(), to: None },
            antitype_passage: ScriptureGroundSeed { from: "GEN.1.2".into(), to: None },
            note: "a real figure".into(),
            text: "a real typology quote".into(),
        }];

        let (graph, stats, event_world_stats, chrono) = crate::build::build_graph_from_sources(KJV_FIXTURE, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas).unwrap();
        assert_eq!(graph.typology.len(), 1, "fixture sanity: the real fulfillment_adapter::normalize path must have built exactly one row");

        let chronology = Chronology::from_derivation(chrono);
        let dumped = dump(&graph, &chronology, &stats, &event_world_stats).expect("dump must succeed with a real typology row");
        assert_eq!(dumped.typology.len(), 1);
        assert_eq!(dumped.typology[0].note.as_deref(), Some("a real figure"));

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, _stats2, _ews2, _chronology2) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.typology.len(), 1, "the row must survive the full round trip, not silently drop");
        let row = &reconstructed.typology[0];
        let gen_1_1 = BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 });
        let gen_1_2 = BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 2 });
        assert_eq!(row.type_passage, BibleLocusRange::new(gen_1_1.clone(), gen_1_1.clone()).unwrap());
        assert_eq!(row.antitype_passage, BibleLocusRange::new(gen_1_2.clone(), gen_1_2.clone()).unwrap());
        assert_eq!(row.note.as_deref(), Some("a real figure"));
        assert_eq!(row.justification.text.as_deref(), Some("a real typology quote"));
        assert_eq!(row.justification.grounds.len(), 1);
        assert!(matches!(row.justification.grounds.iter().next().unwrap(), Ground::Scripture(_)));
    }

    /// CORP-2a: the `Contains<ConcordTag>` sibling of `fulfills_round_
    /// trips_losslessly_with_a_real_row` above -- constructed directly
    /// (the concord_adapter's own pipeline wiring is proven by ITS OWN
    /// tests; this proves artifact.rs's own round trip, in isolation,
    /// the same "prove the DTO layer, not the adapter, here" discipline
    /// `node_payloads_survive_the_round_trip_with_real_values` already
    /// follows for Place/TextUnit payloads).
    #[test]
    fn contains_concord_round_trips_losslessly_with_a_real_row() {
        use atlas_graph_types::edge::Contains;
        use atlas_graph_types::id::ContainerNodeId;
        use atlas_graph_types::text::{ConcordRef, ConcordTag, Locus, LocusSet};

        let mut g = Graph::default();
        let p1 = Locus::<ConcordTag>::whole(ConcordRef { part: 3, article: 4, paragraph: 1 });
        let p2 = Locus::<ConcordTag>::whole(ConcordRef { part: 3, article: 4, paragraph: 2 });
        let mut content = std::collections::BTreeSet::new();
        content.insert(p1);
        content.insert(p2);
        g.contains_concord.push(Contains {
            container: ContainerNodeId::new("concord-ac-iv"),
            content: LocusSet(content),
            provenance: "concord-adapter".into(),
            justification: Justification { text: Some("Augsburg Confession Article IV, paragraphs 1-2".into()), grounds: Default::default() },
        });

        let empty_chrono = Chronology::from_derivation(ChronologyDerivation::default());
        let dumped = dump(&g, &empty_chrono, &BuildStats::default(), &EventWorldStats::default()).expect("dump must succeed with a real contains_concord row -- the guard's whole point was to force this extension");
        assert_eq!(dumped.contains_concord.len(), 1);
        assert_eq!(dumped.contains_concord[0].content.len(), 2);

        let bytes = encode(&dumped).expect("encode must succeed");
        let decoded = decode(&bytes).expect("decode must succeed");
        let (reconstructed, ..) = to_service_parts(decoded).expect("to_service_parts must succeed");

        assert_eq!(reconstructed.contains_concord.len(), 1, "the row must survive the full round trip, not silently drop");
        let row = &reconstructed.contains_concord[0];
        assert_eq!(row.container.0, "concord-ac-iv");
        assert_eq!(row.content.0.len(), 2);
        assert!(row.content.0.contains(&Locus::<ConcordTag>::whole(ConcordRef { part: 3, article: 4, paragraph: 1 })));
        assert!(row.content.0.contains(&Locus::<ConcordTag>::whole(ConcordRef { part: 3, article: 4, paragraph: 2 })));
        assert_eq!(row.justification.text.as_deref(), Some("Augsburg Confession Article IV, paragraphs 1-2"));
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
        // SVEB-1: this test used to populate `quotes`. That relation is
        // SERIALIZED now (this batch closed it), so it no longer trips the
        // guard -- and a guard test that cannot fail is worse than none.
        // Re-pointed at `confesses`, which remains unserialized, so the
        // test keeps proving the thing it was written to prove: an
        // emitted-but-uncarried row fails loud instead of vanishing.
        let mut g = Graph::default();
        g.confesses.push(atlas_graph_types::edge::Confesses {
            confessing: atlas_graph_types::text::ConcordLocus::whole(atlas_graph_types::text::ConcordRef {
                part: 7,
                article: 2,
                paragraph: 1,
            }),
            confessed: LocusRange::new(BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 }), BibleLocus::whole(VerseRef { book: 0, chapter: 1, verse: 1 })).unwrap(),
            provenance: "test".into(),
            justification: Default::default(),
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

    /// RED-1: `spoken_by`/`spoken_at` round-trip losslessly, VALUES
    /// included (not just counts) -- the SAME "real values, not just
    /// shape" discipline `temporal_adjacency_round_trips_losslessly_with_
    /// real_rows` (below) already establishes for its own relation.
    /// Hand-built rows over a small fixture graph (mirrors `red_letter_
    /// adapter.rs`'s own bijection test) rather than the full pipeline --
    /// `built_graph()`'s own fixture never supplies a real `red_letter`
    /// corpus, so this is the dedicated coverage this table's own wire
    /// shape needs.
    #[test]
    fn spoken_by_and_spoken_at_round_trip_losslessly_with_real_values() {
        use atlas_graph_types::edge::{Ground, Justification, SpokenAt, SpokenBy};
        use atlas_graph_types::id::{PersonId, PlaceId};
        use std::collections::BTreeSet;

        let (mut original, chronology, stats, ews) = built_graph();
        let range = BibleLocusRange::new(BibleLocus::whole(VerseRef { book: 39, chapter: 4, verse: 19 }), BibleLocus::whole(VerseRef { book: 39, chapter: 4, verse: 19 })).unwrap();
        let mut grounds = BTreeSet::new();
        grounds.insert(Ground::Scripture(range.clone()));
        original.spoken_by.push(SpokenBy { locus: range.clone(), speaker: PersonId::new("jesus_905"), provenance: "red-letter".into(), justification: Justification { text: None, grounds: grounds.clone() } });
        original.spoken_at.push(SpokenAt {
            locus: range.clone(),
            place: PlaceId::new("capernaum"),
            provenance: "event-witnesses".into(),
            justification: Justification { text: Some("derived: falls within calling-of-the-first-disciples's own attested range".into()), grounds },
        });

        let dumped = dump(&original, &chronology, &stats, &ews).expect("dump must succeed with real spoken_by/spoken_at rows");
        let bytes = encode(&dumped).unwrap();
        let decoded = decode(&bytes).unwrap();
        let reconstructed = to_graph(decoded).unwrap();

        assert_eq!(reconstructed.spoken_by.len(), 1);
        let by = &reconstructed.spoken_by[0];
        assert_eq!(by.speaker.0, "jesus_905");
        assert_eq!(by.locus.from.unit, VerseRef { book: 39, chapter: 4, verse: 19 });
        assert_eq!(by.provenance, "red-letter");
        assert!(by.justification.grounds.contains(&Ground::Scripture(range.clone())));

        assert_eq!(reconstructed.spoken_at.len(), 1);
        let at = &reconstructed.spoken_at[0];
        assert_eq!(at.place.0, "capernaum");
        assert_eq!(at.provenance, "event-witnesses");
        assert_eq!(at.justification.text.as_deref(), Some("derived: falls within calling-of-the-first-disciples's own attested range"));
    }
}
