#[cfg(not(feature = "canon-ids"))]
use std::collections::hash_map::DefaultHasher;
#[cfg(not(feature = "canon-ids"))]
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub type Interned = String;

crate::vocabulary! {
    /// What kind of thing one node of this atlas stands for.
    #[derive(PartialOrd, Ord, Hash)]
    NodeKind {
        TextUnit, Container, Event, Narrative, Place, Person, Anchor, Era, Polity,
        CatechismItem, Source, Translation, PeopleGroup, CommentaryItem, LexiconEntry, Map,
    }
}

pub trait KindTag {
    const KIND: NodeKind;
}

macro_rules! kind_tags {
    ($($tag:ident => $kind:ident),+ $(,)?) => {
        $(
            #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $tag;
            impl KindTag for $tag { const KIND: NodeKind = NodeKind::$kind; }
        )+

        /// The kind each tag stands for, read back through the tag. The match is over
        /// `NodeKind`, so the compiler refuses a kind the list above leaves untagged.
        #[cfg(test)]
        fn tagged(kind: NodeKind) -> NodeKind {
            match kind { $(NodeKind::$kind => <$tag as KindTag>::KIND),+ }
        }
    };
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
    LexiconEntryTag => LexiconEntry,
    MapTag => Map,
}

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
impl<K: KindTag> std::hash::Hash for NodeId<K> {
    fn hash<H: std::hash::Hasher>(&self, st: &mut H) {
        std::hash::Hash::hash(&self.0, st);
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
pub type LexiconEntryId = NodeId<LexiconEntryTag>;
pub type MapId = NodeId<MapTag>;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnyNodeId {
    pub kind: NodeKind,
    pub raw: Interned,
}

impl AnyNodeId {
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    Node(AnyNodeId),
    Edge(crate::edge::EdgeId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PositionKind {
    Node(NodeKind),
    Edge(crate::edge::EdgeKind),
    Version,
}

#[cfg(not(feature = "canon-ids"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub u64);

#[cfg(not(feature = "canon-ids"))]
impl ContentHash {
    pub const HEX_WIDTH: usize = 16;

    pub fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }

    pub fn from_hex(s: &str) -> Option<ContentHash> {
        if s.len() != ContentHash::HEX_WIDTH || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return None;
        }
        u64::from_str_radix(s, 16).ok().map(ContentHash)
    }
}

#[cfg(feature = "canon-ids")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub [u8; 16]);

#[cfg(feature = "canon-ids")]
impl ContentHash {
    pub const HEX_WIDTH: usize = 32;

    pub fn hex(&self) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(32);
        for b in self.0 {
            let _ = write!(s, "{b:02x}");
        }
        s
    }

    pub fn from_hex(s: &str) -> Option<ContentHash> {
        if s.len() != ContentHash::HEX_WIDTH || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
            return None;
        }
        let bytes = s.as_bytes();
        let mut out = [0u8; 16];
        for (i, slot) in out.iter_mut().enumerate() {
            let pair = std::str::from_utf8(&bytes[i * 2..i * 2 + 2]).ok()?;
            *slot = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(ContentHash(out))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid {
    pub kind: PositionKind,
    pub hash: ContentHash,
}

pub trait ContentAddressed {
    fn canonical_bytes(&self) -> Vec<u8>;
    fn position_kind(&self) -> PositionKind;

    #[cfg(not(feature = "canon-ids"))]
    fn pid(&self) -> Pid {
        let mut h = DefaultHasher::new();
        self.canonical_bytes().hash(&mut h);
        Pid { kind: self.position_kind(), hash: ContentHash(h.finish()) }
    }

    #[cfg(feature = "canon-ids")]
    fn pid(&self) -> Pid {
        let hash = crate::sha256::sha256_prefixed_128(
            crate::canon::DOMAIN_PREFIX,
            &self.canonical_bytes(),
        );
        Pid { kind: self.position_kind(), hash: ContentHash(hash) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tombstone {
    pub retired: Pid,
    pub superseded_by: Option<Pid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_node_kind_has_a_tag_that_stands_for_it() {
        // Arrange
        let every_kind = NodeKind::ALL;
        // Act
        let stood_for = every_kind.map(tagged);
        // Assert
        assert_eq!(stood_for, every_kind);
    }
}
