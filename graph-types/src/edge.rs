use std::collections::{BTreeMap, BTreeSet};

use crate::id::{
    AnchorId, AnyNodeId, CatechismItemId, CommentaryItemId, ContainerNodeId, ContentAddressed,
    EventId, Interned, LexiconEntryId, MapId,
    NarrativeId, PeopleGroupId, PersonId, PlaceId, PolityId, Position, PositionKind, SourceId,
};
use crate::adjacency::{EdgeEntry, EdgeMeta, Adjacency};
use crate::ingest::ProvenanceId;
use crate::text::{BibleLocusRange, ConcordLocus, Corpus, LocusSet, TextLocus};

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
            /// The relation's own name, spelled from the declaration rather than read
            /// off `Debug`, which is free to change its rendering without notice.
            pub fn name(self) -> &'static str {
                match self { $(RelationId::$dr => ::core::stringify!($dr)),+ }
            }
            pub fn forward_label(self) -> &'static str {
                match self { $(RelationId::$dr => $fwd),+ }
            }
            pub fn inverse_label(self) -> &'static str {
                match self { $(RelationId::$dr => $inv),+ }
            }
        }

        impl SymRelationId {
            pub const ALL: &'static [SymRelationId] = &[$(SymRelationId::$sr),+];
            /// The relation's own name, spelled from the declaration for the same
            /// reason [`RelationId::name`] is.
            pub fn name(self) -> &'static str {
                match self { $(SymRelationId::$sr => ::core::stringify!($sr)),+ }
            }
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
        DerivedFrom => "derived-from" / "derives",
        Occurs      => "occurs-in" / "words",
        ParentOf     => "parent-of" / "child-of",
        Participates => "participates-in" / "participants",
        AuthoredBy   => "authored-by" / "authored",
        Shows        => "shows" / "shown-on"
    }
    symmetric {
        Analogue          => "analogous-to",
        CatechismLink     => "catechism-link",
        Corresponds       => "corresponds-to",
        Parallel          => "parallel",
        TemporalAdjacency => "temporal-adjacency",
        Spouses           => "spouse-of",
        Brethren          => "brethren-of"
    }
}

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

    pub fn all() -> impl Iterator<Item = EdgeKind> {
        RelationId::ALL
            .iter()
            .flat_map(|r| [EdgeKind::Directed(*r, Direction::Forward), EdgeKind::Directed(*r, Direction::Inverse)])
            .chain(SymRelationId::ALL.iter().map(|s| EdgeKind::Symmetric(*s)))
    }

    pub fn labels() -> impl Iterator<Item = &'static str> {
        Self::all().map(EdgeKind::label)
    }

    pub fn from_label(label: &str) -> Option<EdgeKind> {
        Self::all().find(|k| k.label() == label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub Interned);

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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GroundTarget {
    Scripture(BibleLocusRange),
    Anchor(AnchorId),
    Source(SourceId),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ContainerContent<C: Corpus> {
    Loci(LocusSet<C>),
    Container(ContainerNodeId),
}

impl<C: Corpus> Clone for ContainerContent<C> {
    fn clone(&self) -> Self {
        match self {
            ContainerContent::Loci(l) => ContainerContent::Loci(l.clone()),
            ContainerContent::Container(c) => ContainerContent::Container(c.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contains<C: Corpus> {
    pub container: ContainerNodeId,
    pub content: ContainerContent<C>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonSuccession {
    pub prior: ContainerNodeId,
    pub next: ContainerNodeId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attests {
    pub event: EventId,
    pub attestation: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Succession {
    pub narrative: NarrativeId,
    pub chain: Vec<EventId>,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocatedAt {
    pub event: EventId,
    pub place: PlaceId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fulfills {
    pub prophecy: BibleLocusRange,
    pub fulfillment: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typology {
    pub type_passage: BibleLocusRange,
    pub antitype_passage: BibleLocusRange,
    pub note: Option<String>,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedAfter {
    pub namesake: Namesake,
    pub eponym: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Namesake {
    PeopleGroup(PeopleGroupId),
    Place(PlaceId),
    Polity(PolityId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatechismLink {
    pub locus: TextLocus,
    pub item: CatechismItemId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentsOn {
    pub item: CommentaryItemId,
    pub on: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenBy {
    pub locus: BibleLocusRange,
    pub speaker: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpokenAt {
    pub locus: BibleLocusRange,
    pub place: PlaceId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MentionedEntity {
    Place(PlaceId),
    Person(PersonId),
    PeopleGroup(PeopleGroupId),
    Event(EventId),
}

impl MentionedEntity {
    pub fn node_id(&self) -> AnyNodeId {
        match self {
            MentionedEntity::Place(p) => p.erase(),
            MentionedEntity::Person(p) => p.erase(),
            MentionedEntity::PeopleGroup(g) => g.erase(),
            MentionedEntity::Event(e) => e.erase(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mentions {
    pub locus: TextLocus,
    pub entity: MentionedEntity,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analogue {
    pub a: EventId,
    pub b: EventId,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Occurs {
    pub entry: LexiconEntryId,
    pub locus: TextLocus,
    pub provenance: ProvenanceId,
}

crate::vocabulary! {
    #[doc = "How a parent stands to a child. Every parent-of row is natural unless Scripture declares otherwise: God the Father begets the Son from eternity, the Virgin Mary bore Him, Joseph was His father as was supposed, and God created Adam and Eve."]
    #[derive(PartialOrd, Ord, Hash)]
    Parentage {
        Natural => "natural",
        Eternal => "eternal",
        Virgin => "virgin",
        Legal => "legal",
        Created => "created",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParentOf {
    pub parent: PersonId,
    pub child: PersonId,
    pub parentage: Parentage,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spouses {
    pub a: PersonId,
    pub b: PersonId,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brethren {
    pub a: PersonId,
    pub b: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participates {
    pub person: PersonId,
    pub event: EventId,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authored {
    pub book: ContainerNodeId,
    pub person: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub map: MapId,
    pub node: AnyNodeId,
    pub provenance: ProvenanceId,
}

/// The third row family lowering into `Succession`, beside `Succession` and `CanonSuccession`:
/// a Map is not a container, so `CanonSuccession`'s typed ids cannot carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSuccession {
    pub prior: MapId,
    pub next: MapId,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemporalAdjacency {
    pub earlier: EventId,
    pub later: EventId,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossRef {
    pub from: TextLocus,
    pub to: TextLocus,
    pub to_last: Option<TextLocus>,
    pub target_display: String,
    pub votes: u32,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Quotes {
    pub quoting: TextLocus,
    pub quoted: BibleLocusRange,
    pub provenance: ProvenanceId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confesses {
    pub confessing: ConcordLocus,
    pub confessed: BibleLocusRange,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Corresponds<C: Corpus> {
    pub a: crate::text::Locus<C>,
    pub b: crate::text::Locus<C>,
    pub provenance: ProvenanceId,
}

pub trait Relation {
    type Row;
    const ID: RelationId;
    fn endpoints(row: &Self::Row) -> Vec<(Position, Position)>;
}

#[derive(Clone, Debug)]
pub struct EdgeRecord {
    pub id: EdgeId,
    pub kind: EdgeKind,
    pub subject: Position,
    pub object: Position,
}

#[derive(Debug, Default)]
pub struct BiIndex {
    pub fwd: BTreeMap<Position, Adjacency>,
    pub inv: BTreeMap<Position, Adjacency>,
}

impl BiIndex {
    pub fn build(
        rel: RelationId,
        pairs: &[(Position, Position, EdgeMeta)],
    ) -> BiIndex {
        let mut fwd: BTreeMap<Position, Vec<EdgeEntry>> = BTreeMap::new();
        let mut inv: BTreeMap<Position, Vec<EdgeEntry>> = BTreeMap::new();
        for (s, o, m) in pairs {
            let eid = entry_id(rel, s, o);
            fwd.entry(s.clone()).or_default().push(EdgeEntry { edge: eid.clone(), node: o.clone(), meta: m.clone() });
            inv.entry(o.clone()).or_default().push(EdgeEntry { edge: eid, node: s.clone(), meta: m.clone() });
        }
        BiIndex { fwd: adjacencies(fwd), inv: adjacencies(inv) }
    }

    pub fn build_symmetric(
        rel: SymRelationId,
        pairs: &[(Position, Position, EdgeMeta)],
    ) -> BiIndex {
        let mut fwd: BTreeMap<Position, Vec<EdgeEntry>> = BTreeMap::new();
        for (a, b, m) in pairs {
            let eid = entry_id_symmetric(rel, a, b);
            fwd.entry(a.clone()).or_default().push(EdgeEntry { edge: eid.clone(), node: b.clone(), meta: m.clone() });
            fwd.entry(b.clone()).or_default().push(EdgeEntry { edge: eid, node: a.clone(), meta: m.clone() });
        }
        BiIndex { fwd: adjacencies(fwd), inv: BTreeMap::new() }
    }
}

fn adjacencies(rows: BTreeMap<Position, Vec<EdgeEntry>>) -> BTreeMap<Position, Adjacency> {
    rows.into_iter().map(|(position, rows)| (position, Adjacency::of_rows(rows))).collect()
}

pub fn entry_id(rel: RelationId, s: &Position, o: &Position) -> EdgeId {
    struct E<'a>(RelationId, &'a Position, &'a Position);
    impl<'a> ContentAddressed for E<'a> {
        #[cfg(not(feature = "canon-ids"))]
        fn canonical_bytes(&self) -> Vec<u8> {
            format!("{:?}|{:?}|{:?}", self.0, self.1, self.2).into_bytes()
        }
        #[cfg(feature = "canon-ids")]
        fn canonical_bytes(&self) -> Vec<u8> {
            crate::canon::ids::edge_canonical_bytes(&format!("{:?}", self.0), self.1, self.2)
        }
        fn position_kind(&self) -> PositionKind {
            PositionKind::Edge(EdgeKind::Directed(self.0, Direction::Forward))
        }
    }
    let pid = E(rel, s, o).pid();
    EdgeId(format!("{:?}:{}", rel, pid.hash.hex()))
}

pub fn entry_id_symmetric(rel: SymRelationId, a: &Position, b: &Position) -> EdgeId {
    let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
    struct E<'a>(SymRelationId, &'a Position, &'a Position);
    impl<'a> ContentAddressed for E<'a> {
        #[cfg(not(feature = "canon-ids"))]
        fn canonical_bytes(&self) -> Vec<u8> {
            format!("{:?}|{:?}|{:?}", self.0, self.1, self.2).into_bytes()
        }
        #[cfg(feature = "canon-ids")]
        fn canonical_bytes(&self) -> Vec<u8> {
            crate::canon::ids::edge_canonical_bytes(&format!("{:?}", self.0), self.1, self.2)
        }
        fn position_kind(&self) -> PositionKind {
            PositionKind::Edge(EdgeKind::Symmetric(self.0))
        }
    }
    let pid = E(rel, lo, hi).pid();
    EdgeId(format!("{:?}:{}", rel, pid.hash.hex()))
}

pub fn at(n: &AnyNodeId) -> Position {
    Position::Node(n.clone())
}

/// A list handed over in reading order IS its succession: nothing is re-sorted or derived from
/// the ids.
pub fn steps_between<Id: Clone, Step>(ids: &[Id], step: impl Fn(Id, Id) -> Step) -> Vec<Step> {
    ids.windows(2).map(|pair| step(pair[0].clone(), pair[1].clone())).collect()
}

impl CanonSuccession {
    pub fn steps_between(containers: &[ContainerNodeId], provenance: &str) -> Vec<CanonSuccession> {
        steps_between(containers, |prior, next| CanonSuccession {
            prior,
            next,
            provenance: ProvenanceId::from(provenance),
            justification: Justification::default(),
        })
    }
}

impl MapSuccession {
    pub fn steps_between(maps: &[MapId], provenance: &str) -> Vec<MapSuccession> {
        steps_between(maps, |prior, next| MapSuccession { prior, next, provenance: ProvenanceId::from(provenance) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROVENANCE: &str = "test";

    fn canon_step(prior: &str, next: &str) -> CanonSuccession {
        CanonSuccession {
            prior: ContainerNodeId::new(prior),
            next: ContainerNodeId::new(next),
            provenance: ProvenanceId::from(PROVENANCE),
            justification: Justification::default(),
        }
    }

    #[test]
    fn containers_in_reading_order_step_from_each_to_the_next_and_a_lone_container_steps_nowhere() {
        // Arrange
        let three = [ContainerNodeId::new("a"), ContainerNodeId::new("b"), ContainerNodeId::new("c")];
        let one = [ContainerNodeId::new("a")];

        // Act
        let steps = (CanonSuccession::steps_between(&three, PROVENANCE), CanonSuccession::steps_between(&one, PROVENANCE));

        // Assert
        assert_eq!(steps, (vec![canon_step("a", "b"), canon_step("b", "c")], vec![]));
    }

    #[test]
    fn maps_in_era_order_step_from_each_to_the_next() {
        // Arrange
        let maps = [MapId::new("era-patriarchs"), MapId::new("era-conquest")];

        // Act
        let steps = MapSuccession::steps_between(&maps, PROVENANCE);

        // Assert
        assert_eq!(
            steps,
            vec![MapSuccession { prior: MapId::new("era-patriarchs"), next: MapId::new("era-conquest"), provenance: ProvenanceId::from(PROVENANCE) }]
        );
    }
}
