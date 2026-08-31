//! Identity: kind-tagged node ids, positions, content-addressed pids.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

/// Skeleton stand-in for an interner handle.
pub type Interned = String;

/// Closed node-kind vocabulary. Extending it is a deliberate act every
/// exhaustive match must acknowledge. TextUnit's LEVEL (book / chapter /
/// verse — or a Concord part / article / paragraph) is known to its
/// corpus scheme, not to this enum (sweep F11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NodeKind {
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
    /// KRETZ-1: a verse-anchored unit of a commentary work's prose
    /// (annotation shape -- owner order 2026-08-24).
    CommentaryItem,
    /// SVEB-1: a topical grouping of proof texts under a catechism item.
    CatechismTopic,
    /// PARTS-1: a CHIEF PART of the catechism -- the level above an item.
    /// Materialized so part-level content has somewhere to attach: the
    /// brain-fuel repo carries whole files of part-OVERVIEW questions
    /// ("The Ten Commandments", "The Apostles' Creed") that belong to a
    /// part rather than to any one of its items, and before this they had
    /// nowhere to live and were deferred.
    CatechismPart,
    /// SVEB-1: one addressable unit of Svebilius' Catechism.
    SvebiliusUnit,
}

/// Kind tag for phantom-typed ids: a cross-kind reference is a type
/// error, not a runtime surprise.
pub trait KindTag {
    const KIND: NodeKind;
}

macro_rules! kind_tags {
    ($($tag:ident => $kind:ident),+ $(,)?) => {$(
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $tag;
        impl KindTag for $tag { const KIND: NodeKind = NodeKind::$kind; }
    )+};
}

kind_tags! {
    TextUnitTag => TextUnit,
    ContainerTag => Container,
    EventTag => Event,
    NarrativeTag => Narrative,
    PlaceTag => Place,
    PersonTag => Person,
    AnchorTag => Anchor,
    EraTag => Era,
    PolityTag => Polity,
    CatechismItemTag => CatechismItem,
    SourceTag => Source,
    TranslationTag => Translation,
    PeopleGroupTag => PeopleGroup,
    CommentaryItemTag => CommentaryItem,
    CatechismTopicTag => CatechismTopic,
    CatechismPartTag => CatechismPart,
    SvebiliusUnitTag => SvebiliusUnit,
}

/// Typed in-memory handle; renders to its Pid at the boundary.
#[derive(Debug)]
pub struct NodeId<K: KindTag>(pub Interned, pub PhantomData<K>);

impl<K: KindTag> NodeId<K> {
    pub fn new(raw: impl Into<Interned>) -> Self {
        NodeId(raw.into(), PhantomData)
    }
    pub fn erase(&self) -> AnyNodeId {
        AnyNodeId { kind: K::KIND, raw: self.0.clone() }
    }
}

// Manual impls: derives would wrongly bound K itself.
impl<K: KindTag> Clone for NodeId<K> {
    fn clone(&self) -> Self {
        NodeId(self.0.clone(), PhantomData)
    }
}
impl<K: KindTag> PartialEq for NodeId<K> {
    fn eq(&self, o: &Self) -> bool {
        self.0 == o.0
    }
}
impl<K: KindTag> Eq for NodeId<K> {}
impl<K: KindTag> PartialOrd for NodeId<K> {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl<K: KindTag> Ord for NodeId<K> {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.cmp(&o.0)
    }
}
impl<K: KindTag> Hash for NodeId<K> {
    fn hash<H: Hasher>(&self, st: &mut H) {
        self.0.hash(st);
    }
}

pub type TextUnitId = NodeId<TextUnitTag>;
pub type ContainerNodeId = NodeId<ContainerTag>;
pub type EventId = NodeId<EventTag>;
pub type NarrativeId = NodeId<NarrativeTag>;
pub type PlaceId = NodeId<PlaceTag>;
pub type PersonId = NodeId<PersonTag>;
pub type AnchorId = NodeId<AnchorTag>;
pub type EraId = NodeId<EraTag>;
pub type PolityId = NodeId<PolityTag>;
pub type CatechismItemId = NodeId<CatechismItemTag>;
pub type SourceId = NodeId<SourceTag>;
pub type TranslationNodeId = NodeId<TranslationTag>;
pub type PeopleGroupId = NodeId<PeopleGroupTag>;
pub type CommentaryItemId = NodeId<CommentaryItemTag>;
pub type CatechismTopicId = NodeId<CatechismTopicTag>;
pub type CatechismPartId = NodeId<CatechismPartTag>;
pub type SvebiliusUnitId = NodeId<SvebiliusUnitTag>;

/// Erased form for the wire/UI boundary and heterogeneous holdings.
/// Narrowing back to a typed id is a checked parse.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnyNodeId {
    pub kind: NodeKind,
    pub raw: Interned,
}

impl AnyNodeId {
    /// Checked narrowing: Ok only if the kind agrees.
    pub fn narrow<K: KindTag>(&self) -> Result<NodeId<K>, KindMismatch> {
        if self.kind == K::KIND {
            Ok(NodeId(self.raw.clone(), PhantomData))
        } else {
            Err(KindMismatch { expected: K::KIND, found: self.kind })
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindMismatch {
    pub expected: NodeKind,
    pub found: NodeKind,
}

/// Positions include edges: a claim's justification is walkable
/// (edges-as-positions law).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    Node(AnyNodeId),
    Edge(crate::edge::EdgeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PositionKind {
    Node(NodeKind),
    Edge(crate::edge::EdgeKind),
    Exploration,
}

/// Content-addressed identity: pid = kind + hash(canonical bytes); the
/// id is a key from which the thing is derivable (self-verifying store).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub u64);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid {
    pub kind: PositionKind,
    pub hash: ContentHash,
}

/// Everything addressable defines one canonical byte form — the same
/// form `derive` returns; hash(canonical_bytes(derive(pid))) == pid.
pub trait ContentAddressed {
    fn canonical_bytes(&self) -> Vec<u8>;
    fn position_kind(&self) -> PositionKind;
    fn pid(&self) -> Pid {
        let mut h = DefaultHasher::new();
        self.canonical_bytes().hash(&mut h);
        Pid { kind: self.position_kind(), hash: ContentHash(h.finish()) }
    }
}

/// Retirement never recycles: a retired pid resolves to a tombstone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tombstone {
    pub retired: Pid,
    pub superseded_by: Option<Pid>,
}
