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
        Named       => "named" / "name-of",
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

#[derive(Clone, Debug)]
pub struct Named {
    pub place: PlaceId,
    pub alias: String,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

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

#[derive(Clone, Debug)]
pub struct CrossRef {
    pub from: TextLocus,
    pub to: TextLocus,
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
    pub fwd: BTreeMap<Position, Vec<(EdgeId, Position)>>,
    pub inv: BTreeMap<Position, Vec<(EdgeId, Position)>>,
}

impl BiIndex {
    pub fn build(rel: RelationId, pairs: &[(Position, Position)]) -> BiIndex {
        let mut ix = BiIndex::default();
        for (s, o) in pairs {
            let eid = entry_id(rel, s, o);
            ix.fwd.entry(s.clone()).or_default().push((eid.clone(), o.clone()));
            ix.inv.entry(o.clone()).or_default().push((eid, s.clone()));
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

/// Convenience: node position.
pub fn at(n: &AnyNodeId) -> Position {
    Position::Node(n.clone())
}
