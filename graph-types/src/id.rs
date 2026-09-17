//! Identity: kind-tagged node ids, positions, content-addressed pids.

// FINAL REVIEW item 6: `DefaultHasher` and the two traits it is driven
// through are OFF-state machinery and carry the SAME gate, so the import
// block cannot hold a name the ON build does not use. The manual
// `impl Hash for NodeId` below therefore spells both trait paths in full
// (`std::hash::Hash` / `std::hash::Hasher`) rather than relying on these
// imports: that impl exists in BOTH states, the imports do not.
#[cfg(not(feature = "canon-ids"))]
use std::collections::hash_map::DefaultHasher;
#[cfg(not(feature = "canon-ids"))]
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
///
/// DB-2a widens this. OFF (default) it is what it has always been: a
/// 64-bit `DefaultHasher` output -- an algorithm std itself says "may
/// change between releases", which is exactly why the relational artifact
/// cannot keep it. ON (`--features canon-ids`) it is the first 128 bits
/// of a SHA-256 over domain-prefixed canonical JSON: stable across
/// toolchains, wide enough that a ~10^6-thing corpus has no birthday
/// problem, and the width spec §3.1 fixes for the artifact's id column.
///
/// `hex()` is how a hash reaches the wire in BOTH states, so no caller
/// has to know the width: 16 lowercase hex chars OFF, 32 ON.
#[cfg(not(feature = "canon-ids"))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash(pub u64);

#[cfg(not(feature = "canon-ids"))]
impl ContentHash {
    /// Fixed-width lowercase hex -- zero-padded, so equal hashes are
    /// byte-identical strings (ETag comparison depends on it).
    pub fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }

    /// The inverse of `hex`, strict: exactly 16 lowercase hex digits.
    pub fn from_hex(s: &str) -> Option<ContentHash> {
        if s.len() != 16 || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
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
    /// 32 lowercase hex chars -- the artifact's id spelling. Written into
    /// one pre-sized String rather than allocating a `format!` per byte:
    /// this runs once per node on every graph load.
    pub fn hex(&self) -> String {
        use std::fmt::Write;
        let mut s = String::with_capacity(32);
        for b in self.0 {
            let _ = write!(s, "{b:02x}"); // writing into a String cannot fail
        }
        s
    }

    /// The inverse of `hex`, strict: exactly 32 lowercase hex digits.
    pub fn from_hex(s: &str) -> Option<ContentHash> {
        if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) {
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

/// Everything addressable defines one canonical byte form — the same
/// form `derive` returns; hash(canonical_bytes(derive(pid))) == pid.
pub trait ContentAddressed {
    fn canonical_bytes(&self) -> Vec<u8>;
    fn position_kind(&self) -> PositionKind;

    /// OFF: the historical 64-bit `DefaultHasher` digest, byte for byte.
    #[cfg(not(feature = "canon-ids"))]
    fn pid(&self) -> Pid {
        let mut h = DefaultHasher::new();
        self.canonical_bytes().hash(&mut h);
        Pid { kind: self.position_kind(), hash: ContentHash(h.finish()) }
    }

    /// ON: SHA-256-128 over `DOMAIN_PREFIX ‖ canonical_bytes`. The prefix
    /// is hashed, never stored — the bytes on disk stay exactly what
    /// `Canon::encode` produced, while a node's digest can never collide
    /// with a digest of the same bytes meaning something else.
    #[cfg(feature = "canon-ids")]
    fn pid(&self) -> Pid {
        let hash = crate::sha256::sha256_prefixed_128(
            crate::canon::DOMAIN_PREFIX,
            &self.canonical_bytes(),
        );
        Pid { kind: self.position_kind(), hash: ContentHash(hash) }
    }
}

/// Retirement never recycles: a retired pid resolves to a tombstone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tombstone {
    pub retired: Pid,
    pub superseded_by: Option<Pid>,
}
