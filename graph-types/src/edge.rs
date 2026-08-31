//! Edges: structurally-paired kinds, bijection-witnessed bidirectional
//! tables, justification as the one carrier of a claim's why.

use std::collections::{BTreeMap, BTreeSet};

use crate::id::{
    AnchorId, AnyNodeId, CatechismItemId, CommentaryItemId, ContainerNodeId, ContentAddressed,
    EventId, Interned,
    NarrativeId, PeopleGroupId, PersonId, PlaceId, PolityId, Position, PositionKind, SourceId,
};
use crate::ingest::ProvenanceId;
use crate::text::{BibleLocusRange, ConcordLocus, Corpus, LocusSet, TextLocus};

/// The two readings of a directed relation. Flipping a two-variant enum
/// is involutive by construction — the dual is a shape, not a tested
/// property.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    Forward,
    Inverse,
}

impl Direction {
    pub fn flip(self) -> Direction {
        match self {
            Direction::Forward => Direction::Inverse,
            Direction::Inverse => Direction::Forward,
        }
    }
}

/// The relation manifest: ONE definition site. The macro generates the
/// id enums and the label tables; an unpaired kind or label drift is
/// unrepresentable because there is no second list.
/// C1 (map-system contract, 2026-08-24): exported so a sibling system
/// can declare ITS OWN relation manifest with the same one-definition-
/// site algebra (paired labels, unrepresentable drift). The macro body
/// is self-contained -- it generates fresh enums in the caller's crate
/// and references nothing of ours, so exporting it shares the SHAPE,
/// never our vocabulary.
#[macro_export]
macro_rules! relations {
    (
        directed { $($dr:ident => $fwd:literal / $inv:literal),+ $(,)? }
        symmetric { $($sr:ident => $sym:literal),+ $(,)? }
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum RelationId { $($dr),+ }

        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum SymRelationId { $($sr),+ }

        impl RelationId {
            pub const ALL: &'static [RelationId] = &[$(RelationId::$dr),+];
            pub fn forward_label(self) -> &'static str {
                match self { $(RelationId::$dr => $fwd),+ }
            }
            pub fn inverse_label(self) -> &'static str {
                match self { $(RelationId::$dr => $inv),+ }
            }
        }

        impl SymRelationId {
            pub const ALL: &'static [SymRelationId] = &[$(SymRelationId::$sr),+];
            pub fn label(self) -> &'static str {
                match self { $(SymRelationId::$sr => $sym),+ }
            }
        }
    };
}

relations! {
    directed {
        Contains    => "contains" / "member-of",
        Attests     => "attested-in" / "attests",
        Succession  => "follows-in" / "precedes-in",
        DatedBy     => "dated-by" / "dates",
        LocatedAt   => "located-at" / "site-of",
        Mentions    => "mentions" / "mentioned-in",
        Cites       => "cites" / "cited-by",
        Quotes      => "quotes" / "quoted-by",
        Confesses   => "confesses" / "confessed-in",
        Fulfillment => "fulfilled-in" / "fulfills",
        Typology    => "prefigures" / "prefigured-by",
        NamedAfter  => "named-after" / "namesake-of",
        JustifiedBy => "justified-by" / "justifies",
        CommentsOn  => "comments-on" / "commented-on-by",
        SpokenBy    => "spoken-by" / "speech-of",
        SpokenAt    => "spoken-at" / "site-of-speech",
        DerivedFrom => "derived-from" / "derives"
    }
    symmetric {
        CatechismLink     => "catechism-link",
        Corresponds       => "corresponds-to",
        Parallel          => "parallel",
        TemporalAdjacency => "temporal-adjacency"
    }
}

/// A kind is a relation plus a direction, or a symmetric relation. The
/// dual flips the direction; symmetric kinds are fixed points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    Directed(RelationId, Direction),
    Symmetric(SymRelationId),
}

pub fn dual(k: EdgeKind) -> EdgeKind {
    match k {
        EdgeKind::Directed(r, d) => EdgeKind::Directed(r, d.flip()),
        sym => sym,
    }
}

impl EdgeKind {
    pub fn label(self) -> &'static str {
        match self {
            EdgeKind::Directed(r, Direction::Forward) => r.forward_label(),
            EdgeKind::Directed(r, Direction::Inverse) => r.inverse_label(),
            EdgeKind::Symmetric(s) => s.label(),
        }
    }
}

/// Entry identity: the SAME id reached from either end — the bijection's
/// literal witness. Content-derived (relation + endpoint pids), so
/// set-valued rows expand deterministically (sweep F10).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub Interned);

/// What justifies a claim: optional prose plus the set of grounds it
/// rests on (owner: "a justification has a set of grounds"). Empty
/// justification is lawful (populated progressively). Derivation is NOT
/// justification — it is mechanical, compiler-emitted (DerivedFrom).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Justification {
    pub text: Option<String>,
    pub grounds: BTreeSet<Ground>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ground {
    Scripture(BibleLocusRange),
    Anchor(AnchorId),
    Source(SourceId),
}

/// The target vocabulary for justified-by edges.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroundTarget {
    Scripture(BibleLocusRange),
    Anchor(AnchorId),
    Source(SourceId),
}

// ---------------------------------------------------------------------
// Authored rows (each carries provenance + justification).
// ---------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Contains<C: Corpus> {
    pub container: ContainerNodeId,
    pub content: LocusSet<C>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug)]
pub struct Attests {
    pub event: EventId,
    pub attestation: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// A narrative IS the list — malformed chains cannot be stored, only
/// fail to construct.
#[derive(Clone, Debug)]
pub struct Succession {
    pub narrative: NarrativeId,
    pub chain: Vec<EventId>, // >= 1, validated distinct
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

impl Succession {
    pub fn new(
        narrative: NarrativeId,
        chain: Vec<EventId>,
        provenance: ProvenanceId,
        justification: Justification,
    ) -> Result<Self, ChainError> {
        if chain.is_empty() {
            return Err(ChainError::Empty);
        }
        let distinct: BTreeSet<_> = chain.iter().collect();
        if distinct.len() != chain.len() {
            return Err(ChainError::Duplicate);
        }
        Ok(Succession { narrative, chain, provenance, justification })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainError {
    Empty,
    Duplicate,
}

#[derive(Clone, Debug)]
pub struct LocatedAt {
    pub event: EventId,
    pub place: PlaceId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

// M-D3 (owner ruling R2): the `Named` row struct was retired with the
// `named` manifest row -- a place's aliases are `NodePayload::Place`'s
// own `aliases` payload field, the sole path since M-C.

/// EDGE-1 (owner order 2026-08-23: "we also need a couple more edges:
/// one for Christological types, and one for prophecy/fulfillment").
/// A prophecy's fulfillment -- Scripture-only, text-to-text, DIRECTED
/// (prophecy -> fulfillment; forward reads "fulfilled-in", inverse
/// "fulfills"). Scripture frequently SELF-ATTESTS these rows: the NT
/// fulfillment formulas ("that it might be fulfilled which was spoken
/// by the prophet") make the fulfillment passage itself the natural
/// `Ground::Scripture` of the justification. Positive register per
/// the KJV inerrancy directive: fulfillment is stated as fact, never
/// hedged.
#[derive(Clone, Debug)]
pub struct Fulfills {
    pub prophecy: BibleLocusRange,
    pub fulfillment: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// EDGE-1: a Christological type -- an OT passage prefiguring its NT
/// antitype (ROM 5:14's own "figure of him that was to come").
/// Scripture-only, text-to-text, DIRECTED (type -> antitype; forward
/// "prefigures", inverse "prefigured-by"). v1 is deliberately
/// text-to-text because the classic cases are ARGUED FROM passages
/// (GEN 14 -> HEB 7 Melchizedek; NUM 21:8-9 -> JHN 3:14 the serpent;
/// JON 1:17 -> MAT 12:40); a node-typed subject (the Person/Event
/// that IS the type) is a later EXTEND, not a v1 field. `note` names
/// the figure for display ("the brasen serpent").
#[derive(Clone, Debug)]
pub struct Typology {
    pub type_passage: BibleLocusRange,
    pub antitype_passage: BibleLocusRange,
    pub note: Option<String>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// PG-1: eponymy, curated. The SUBJECT is the named thing; the OBJECT
/// is the person it is named for: tribe-of-Judah --named-after-->
/// Judah the man, and from the man's end, Judah --namesake-of-->
/// tribe-of-Judah (the edge direction follows the LABELS, so both
/// read as sentences). Grounds are the naming passages themselves
/// (the GEN 29-30 etymologies; GEN 32:28 for Israel; GEN 19:37-38
/// for Moab and Ammon) -- the distinction the owner ordered becomes
/// EXPLORABLE, not just labeled.
#[derive(Clone, Debug)]
pub struct NamedAfter {
    pub namesake: Namesake,
    pub eponym: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// The named thing in an eponymy row -- the kinds Scripture actually
/// names after persons (tribe/nation, place, kingdom).
#[derive(Clone, Debug)]
pub enum Namesake {
    PeopleGroup(PeopleGroupId),
    Place(PlaceId),
    Polity(PolityId),
}

#[derive(Clone, Debug)]
pub struct CatechismLink {
    pub locus: TextLocus,
    pub item: CatechismItemId,
    /// SVEB-1: WHICH topical grouping this citation came from, when it came
    /// from one at all (`None` for Luther's own item-level embedded
    /// citations, which belong to no topic by construction).
    ///
    /// This field is the repair of a documented gap. `catechism_adapter`'s
    /// own header used to read: "`catechism-link` is a flat, symmetric,
    /// locus <-> item relation ... with NO ROOM FOR A QUESTION TAG", and so
    /// 7,251 curated topic-attributed citations collapsed to 6,531
    /// item-to-verse rows -- 740 dropped outright as duplicates, and the
    /// topic lost on every survivor. Making room here (rather than minting
    /// a new relation) keeps the vocabulary unchanged and widens the dedup
    /// key from (locus, item) to (locus, item, topic), which is what
    /// actually recovers the dropped rows.
    pub topic: Option<crate::id::CatechismTopicId>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// PARTS-1: a citation attached to a chief PART rather than to one of its
/// items.
///
/// A separate row table rather than making `CatechismLink.item` optional:
/// every existing consumer of `CatechismLink` reads `row.item` on the
/// assumption it is there, and loosening that to satisfy new content would
/// push an `Option` through code that has no business handling one. The
/// two tables lower into the SAME symmetric `catechism-link` relation, so
/// traversal does not care which table a row came from.
#[derive(Clone, Debug)]
pub struct CatechismPartLink {
    pub locus: TextLocus,
    pub part: crate::id::CatechismPartId,
    /// Which part-overview topic gathered this citation, when one did.
    pub topic: Option<crate::id::CatechismTopicId>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// PARTS-1: the structural part -> item edge, so a part can be walked down
/// into its own items (and an item back up to its part).
#[derive(Clone, Debug)]
pub struct CatechismMembership {
    pub part: crate::id::CatechismPartId,
    pub item: CatechismItemId,
    pub provenance: ProvenanceId,
}

/// KRETZ-1: one verse-anchored commentary unit's target -- the unit's
/// own Bible locus RANGE (lemma spans lower to ranges; pericope intros
/// to the pericope range; chapter intros to the chapter's full range),
/// per the owner-ruled verse-mapped-index law. Justification is
/// grounded in the lemma's own locus.
#[derive(Clone, Debug)]
pub struct CommentsOn {
    pub item: CommentaryItemId,
    pub on: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// RED-1 (owner orders 2026-08-25: "Red letters on Jesus' words in
/// every translation"; "SpokenAt is another edge"): direct speech as
/// graph fact. SpokenBy is the GENERAL relation (red letters are its
/// Jesus-speaker rendering; any speaker's words become queryable);
/// verse-granular — edition-specific sub-verse display offsets are a
/// RENDERING concern and live in compiled data, never in this
/// vocabulary.
#[derive(Clone, Debug)]
pub struct SpokenBy {
    pub locus: BibleLocusRange,
    pub speaker: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// RED-1: where the words were spoken — the place edge, parallel to
/// LocatedAt's naming. v1 rows derive from located events whose
/// attested range contains the speech locus (Confidence::Derived).
#[derive(Clone, Debug)]
pub struct SpokenAt {
    pub locus: BibleLocusRange,
    pub place: PlaceId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

// ---------------------------------------------------------------------
// Imported rows.
// ---------------------------------------------------------------------

/// PG-1: the attested sense of an in-text mention -- Place, Person,
/// or PeopleGroup. JDG 1:2 "Judah shall go up" mentions the TRIBE,
/// not the man; the link points where the data says, never where a
/// string guesses. (Widened from the retired two-way `PlaceOrPerson`
/// name -- with three variants the old name stopped being true.)
#[derive(Clone, Debug)]
pub enum MentionedEntity {
    Place(PlaceId),
    Person(PersonId),
    PeopleGroup(PeopleGroupId),
}

#[derive(Clone, Debug)]
pub struct Mentions {
    pub locus: TextLocus,
    pub entity: MentionedEntity,
    pub provenance: ProvenanceId,
}

/// TRAV-1: one adjacent pair in the chronology's total order, DERIVED
/// at compile time from `temporal_order` (the ETL emits each
/// consecutive pair once). The RELATION is symmetric --
/// "adjacent-in-time" -- and lowers through the symmetric index,
/// closing the gap `build_indexes` documented since M-A; the ROW
/// still names its ends honestly (`earlier`/`later`, from the order
/// itself) so a consumer serving a Chronology block never re-derives
/// direction.
#[derive(Clone, Debug)]
pub struct TemporalAdjacency {
    pub earlier: EventId,
    pub later: EventId,
    pub provenance: ProvenanceId,
}

/// M-C2 (requirement 2: "extend the cites relation to range-level
/// (span-capable per the types)"): `to` remains the graph's own edge
/// endpoint (the target's FIRST verse -- unchanged index shape, unchanged
/// `build_indexes` behavior) alongside the honest resolution of the
/// verse-level simplification M-A's own decision 1 disclosed (design doc
/// §4: "cross-ref: verse-level today, loci by design"). `to_last`/
/// `target_display` are SPAN DATA riding the row, not a second edge: a
/// cross-ref target may cite a same-chapter or cross-book/chapter SPAN
/// (`"COL.1.16-19"`, `"MAT.5.3-MAT.6.2"`), and `aggregate_span_xrefs`'s own
/// self-target-subset check needs the target's TRUE last verse (not just
/// its first) to tell "starts inside the span" apart from "wholly inside
/// the span" -- collapsing to `to` alone (the pre-M-C2 shape) can only ever
/// approximate that check. `target_display` is the ORIGINAL citation
/// string exactly as imported (openbible.info's own three canonical
/// shapes) -- `to`/`to_last` are a lossless structured decomposition of
/// it into typed loci, but the wire's own `CrossRefOut.target` field is
/// this exact string, never a re-synthesized one (a source `COL.1.16-19`
/// must never round-trip as `COL.1.16-COL.1.19`).
#[derive(Clone, Debug)]
pub struct CrossRef {
    pub from: TextLocus,
    pub to: TextLocus,
    pub to_last: Option<TextLocus>,
    pub target_display: String,
    pub votes: u32,
    pub provenance: ProvenanceId,
}

/// Quotation points TOWARD the norming norm; the reverse direction has
/// no constructor.
#[derive(Clone, Debug)]
pub struct Quotes {
    pub quoting: TextLocus,
    pub quoted: BibleLocusRange,
    pub provenance: ProvenanceId,
}

/// Doctrinal accord: a normed-norm locus confesses what Scripture
/// teaches. Never the reverse — asymmetry by construction.
#[derive(Clone, Debug)]
pub struct Confesses {
    pub confessing: ConcordLocus,
    pub confessed: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

/// Span-level alignment between layers of ONE corpus family —
/// same-type-ness enforced by the parameter.
#[derive(Clone, Debug)]
pub struct Corresponds<C: Corpus> {
    pub a: crate::text::Locus<C>,
    pub b: crate::text::Locus<C>,
    pub provenance: ProvenanceId,
}

// ---------------------------------------------------------------------
// The relation trait: typed endpoints; direction machinery comes from
// the EdgeKind shape.
// ---------------------------------------------------------------------

pub trait Relation {
    type Row;
    const ID: RelationId;
    fn endpoints(row: &Self::Row) -> Vec<(Position, Position)>;
}

/// Erased edge record the graph serves; the typed rows are the source.
#[derive(Clone, Debug)]
pub struct EdgeRecord {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub subject: Position,
    pub object: Position,
}

/// Built, never authored: both adjacency maps from one row table. A
/// set-valued row expands to one entry per element; entry ids are
/// content-derived, so expansion is stable across recompiles.
#[derive(Debug, Default)]
pub struct BiIndex {
    pub fwd: BTreeMap<Position, Vec<(EdgeId, Position, crate::explore::EdgeMeta)>>,
    pub inv: BTreeMap<Position, Vec<(EdgeId, Position, crate::explore::EdgeMeta)>>,
}

impl BiIndex {
    pub fn build(
        rel: RelationId,
        pairs: &[(Position, Position, crate::explore::EdgeMeta)],
    ) -> BiIndex {
        let mut ix = BiIndex::default();
        for (s, o, m) in pairs {
            let eid = entry_id(rel, s, o);
            ix.fwd
                .entry(s.clone())
                .or_default()
                .push((eid.clone(), o.clone(), m.clone()));
            ix.inv
                .entry(o.clone())
                .or_default()
                .push((eid, s.clone(), m.clone()));
        }
        ix
    }

    /// The symmetric sibling of `build`: BOTH ends are interchangeable, so
    /// each pair populates `fwd` from EITHER end -- querying from `a` or
    /// from `b` returns the other one, under the SAME `EdgeId`
    /// (`entry_id_symmetric`'s own sort-then-hash is what makes that true).
    /// `inv` stays empty: a symmetric relation has no second reading to
    /// hold (design doc §3's own "no directions" shape) --
    /// `raw_neighbors`'s `EdgeKind::Symmetric` arm only ever reads `fwd`.
    pub fn build_symmetric(
        rel: SymRelationId,
        pairs: &[(Position, Position, crate::explore::EdgeMeta)],
    ) -> BiIndex {
        let mut ix = BiIndex::default();
        for (a, b, m) in pairs {
            let eid = entry_id_symmetric(rel, a, b);
            ix.fwd
                .entry(a.clone())
                .or_default()
                .push((eid.clone(), b.clone(), m.clone()));
            ix.fwd
                .entry(b.clone())
                .or_default()
                .push((eid, a.clone(), m.clone()));
        }
        ix
    }
}

/// Content-derived entry id: hash of (relation, subject, object).
pub fn entry_id(rel: RelationId, s: &Position, o: &Position) -> EdgeId {
    struct E<'a>(RelationId, &'a Position, &'a Position);
    impl<'a> ContentAddressed for E<'a> {
        fn canonical_bytes(&self) -> Vec<u8> {
            format!("{:?}|{:?}|{:?}", self.0, self.1, self.2).into_bytes()
        }
        fn position_kind(&self) -> PositionKind {
            PositionKind::Edge(EdgeKind::Directed(self.0, Direction::Forward))
        }
    }
    let pid = E(rel, s, o).pid();
    EdgeId(format!("{:?}:{:016x}", rel, pid.hash.0))
}

/// The symmetric sibling of `entry_id`: a SYMMETRIC relation's two ends are
/// interchangeable (design doc §3: "a symmetric relation lives in a
/// separate id space that HAS no directions"), so the id must be the SAME
/// regardless of which end a caller happens to hash first -- sorting the
/// pair by `Position`'s own `Ord` before hashing is what makes that true
/// (the bijection witness, symmetric case).
pub fn entry_id_symmetric(rel: SymRelationId, a: &Position, b: &Position) -> EdgeId {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    struct E<'a>(SymRelationId, &'a Position, &'a Position);
    impl<'a> ContentAddressed for E<'a> {
        fn canonical_bytes(&self) -> Vec<u8> {
            format!("{:?}|{:?}|{:?}", self.0, self.1, self.2).into_bytes()
        }
        fn position_kind(&self) -> PositionKind {
            PositionKind::Edge(EdgeKind::Symmetric(self.0))
        }
    }
    let pid = E(rel, lo, hi).pid();
    EdgeId(format!("{:?}:{:016x}", rel, pid.hash.0))
}

/// Convenience: node position.
pub fn at(n: &AnyNodeId) -> Position {
    Position::Node(n.clone())
}
