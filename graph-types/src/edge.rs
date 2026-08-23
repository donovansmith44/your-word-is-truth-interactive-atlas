//! Edges: structurally-paired kinds, bijection-witnessed bidirectional
//! tables, justification as the one carrier of a claim's why.

use std::collections::{BTreeMap, BTreeSet};

use crate::id::{
    AnchorId, AnyNodeId, CatechismItemId, ContainerNodeId, ContentAddressed, EventId, Interned,
    NarrativeId, PersonId, PlaceId, Position, PositionKind, SourceId,
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
        JustifiedBy => "justified-by" / "justifies",
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

#[derive(Clone, Debug)]
pub struct CatechismLink {
    pub locus: TextLocus,
    pub item: CatechismItemId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

// ---------------------------------------------------------------------
// Imported rows.
// ---------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum PlaceOrPerson {
    Place(PlaceId),
    Person(PersonId),
}

#[derive(Clone, Debug)]
pub struct Mentions {
    pub locus: TextLocus,
    pub entity: PlaceOrPerson,
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
