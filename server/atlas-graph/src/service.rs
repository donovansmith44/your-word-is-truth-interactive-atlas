//! `GraphService`: the one built-graph handle atlas-server holds (behind
//! an `Arc`, built once at startup).
//!
//! FIX ROUND 1 (C1): the GraphStore port -- `GraphQuery`/`GraphSnapshot`/
//! `GraphStore`/`GraphPublisher` -- is now the OWNER-APPROVED shape in
//! `atlas_graph_types::store` (commit `ab75c8b`, "GraphQuery — the shared
//! interface"). This crate does NOT define, and must never define, a
//! competing/parallel store or query trait (a Critical finding against an
//! earlier draft of this file that did exactly that). `GraphService` is
//! purely an ADAPTER-SIDE convenience wrapper around the owner's own
//! `MemStore`/`MemSnapshot` (implementation #1 of the port) plus the one
//! small piece of bookkeeping the port itself doesn't model: a
//! reading-spine reverse index (`GraphQuery` has no "ref -> spine
//! position" lookup, only forward `reading_window`). Every actual GRAPH
//! QUERY this crate or atlas-server performs goes through
//! `atlas_graph_types::store::GraphQuery`'s own trait methods
//! (`node`/`edges`/`edge_summary`/`reading_window`) -- see `window.rs`,
//! which is generic over `&dyn GraphQuery` and touches nothing else.
//!
//! FIDELITY (design doc P3) is enforced HERE, unconditionally, as part of
//! `from_sources`/`build` -- a `GraphService` built from real raw KJV JSON
//! cannot exist without having already passed `fidelity::check_kjv_fidelity`
//! (checked on the raw `Graph`, BEFORE it is ever published to the store --
//! "proof at the boundary" is the boundary between raw source bytes and
//! the compiled graph, which is exactly the pre-publish moment; publishing
//! moves the `Graph` by value, and `GraphQuery` has no "enumerate every
//! node" operation, so a full bijection check isn't expressible through
//! the port alone after that point in any case).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::store::{GraphPublisher, GraphQuery, GraphStore, GraphVersion, MemSnapshot, MemStore};

use crate::build::{self, BuildStats};
use crate::event_world::{Chronology, EventWorldStats};
use crate::fidelity;

pub struct GraphService {
    snapshot: MemSnapshot,
    /// Reverse index: TextUnit node id -> its position in the bible
    /// reading spine. `GraphQuery` (the port) has no such lookup (only
    /// forward `reading_window`); building this once, alongside the
    /// graph, is what lets `chapter_span`/`position_of` below resolve
    /// "start from this ref" without scanning the whole spine per call.
    bible_position: HashMap<AnyNodeId, usize>,
    pub stats: BuildStats,
    /// Batch M-B (narrowed at M-C, renamed `EventWorld` -> `Chronology`):
    /// the chronology companion index -- same status as `bible_position`
    /// above (the generic `GraphQuery` port does not model
    /// `temporal-adjacency`; see `event_world::Chronology`'s own doc
    /// comment for why it's disclosed, not silent, and for the M-C
    /// retirement of this struct's own former narrative-positions half,
    /// now served by the generic port's `EdgeMeta::Narrative`).
    pub chronology: Chronology,
    pub event_world_stats: EventWorldStats,
}

/// The longest KJV chapter (Psalm 119) has 176 verses; this probe width is
/// a comfortable, documented margin -- see `chapter_span` below.
const MAX_CHAPTER_SPAN_PROBE: usize = 200;

impl GraphService {
    /// The real KJV/xrefs raw-source path -- the FIDELITY LAW is enforced
    /// unconditionally here (module doc comment above). `atlas`: Batch M-B's
    /// own event-world source (see `event_world`'s own module doc comment).
    pub fn from_sources(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, _chrono) = build::build_graph_from_sources(kjv_json, xrefs_tsv, atlas)?;
        fidelity::check_kjv_fidelity(kjv_json, &graph).map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self::assemble(graph, stats, event_world_stats, atlas))
    }

    /// Test-fixture path: builds from an already-parsed `(Canon, verses)`
    /// pair (see `build::build_graph_from_canon_and_verses`'s own doc
    /// comment) -- no raw source BYTES exist to re-derive "expected" from,
    /// so the fidelity law is not applicable here (there is nothing
    /// independent to check the already-typed input against).
    pub fn from_canon_and_verses(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<Self> {
        let (graph, stats, event_world_stats, _chrono) = build::build_graph_from_canon_and_verses(canon, verses, xrefs_tsv, atlas)?;
        Ok(Self::assemble(graph, stats, event_world_stats, atlas))
    }

    /// Reads `raw_dir/kjv.json` and `raw_dir/xrefs/cross_references.txt`
    /// and builds from them, plus the event world from `atlas` — the only
    /// filesystem-touching function in this crate.
    pub fn build(raw_dir: &Path, atlas: &AtlasData) -> anyhow::Result<Self> {
        let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json"))
            .with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
        let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
            .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
        Self::from_sources(&kjv_json, &xrefs_tsv, atlas)
    }

    fn assemble(graph: Graph, stats: BuildStats, event_world_stats: EventWorldStats, atlas: &AtlasData) -> Self {
        let bible_position = graph
            .reading
            .get(crate::kjv_adapter::BIBLE_CORPUS)
            .map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect())
            .unwrap_or_default();
        // Batch M-B (renamed at M-C): the chronology companion index, built
        // from the SAME `atlas` the graph's own Event/Narrative/Anchor rows
        // were just populated from -- see `Chronology`'s own doc comment.
        let chronology = Chronology::build(atlas);
        // GraphPublisher::publish (design doc §9a): the compiler
        // publishes; serving never writes. One publish, at startup; M-A
        // never calls it again (no hot-reload exists yet) -- MemStore's
        // own version-history retention (it keeps every published
        // version, not just the latest) is what would make a future
        // republish safe for any reader still holding an older snapshot,
        // without this crate needing to do anything extra to earn that.
        let mut store = MemStore::default();
        let version = store.publish(graph);
        let snapshot = store.open(version).expect("the version just published must always be open-able");
        GraphService { snapshot, bible_position, stats, chronology, event_world_stats }
    }

    /// The version this service published at construction (M-A: the only
    /// version there will ever be, for this process's lifetime).
    pub fn version(&self) -> GraphVersion {
        atlas_graph_types::store::GraphSnapshot::version(&self.snapshot)
    }

    /// Opens THE PORT's own snapshot handle -- `MemSnapshot` implements
    /// `atlas_graph_types::store::GraphQuery`, so every actual graph
    /// query goes through its trait methods from here on, never through a
    /// direct `Graph` field reach. Cheap to clone (an `Arc<Graph>` inside).
    pub fn snapshot(&self) -> MemSnapshot {
        self.snapshot.clone()
    }

    /// `id`'s own position in the bible reading spine, if it has one --
    /// resolves a ref into a window's own starting index. NOT part of the
    /// generic port (see this struct's own doc comment) -- an adapter-side
    /// companion, same status as `chapter_span` below.
    pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize> {
        self.bible_position.get(&crate::kjv_adapter::verse_node_id(book, chapter, verse)).copied()
    }

    /// The (start, n) window covering exactly one chapter -- `scope=chapter`'s
    /// own "N derived server-side" (M-A brief requirement 4): still just an
    /// input to `window::window` (the SAME generic, port-only primitive
    /// every other window call uses), not a separate query mechanism.
    /// Derived by probing the port's own `reading_window` for a generous
    /// upper bound starting at the chapter's first verse, then trimming to
    /// the contiguous run that actually shares (book, chapter).
    pub fn chapter_span(&self, book: u8, chapter: u16) -> Option<(usize, usize)> {
        let start = self.position_of(book, chapter, 1)?;
        let probe = self.snapshot.reading_window(crate::kjv_adapter::BIBLE_CORPUS, start, MAX_CHAPTER_SPAN_PROBE);
        let n = probe.iter().take_while(|id| matches!(crate::kjv_adapter::decode_text_unit(id), Some((b, c, _)) if b == book && c == chapter)).count();
        if n == 0 {
            None
        } else {
            Some((start, n))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::store::GraphSnapshot as _;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [
          { "chapter": 1, "verses": [
            { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
            { "verse": 2, "text": "And the earth was without form, and void." },
            { "verse": 3, "text": "And God said, Let there be light: and there was light." }
          ] },
          { "chapter": 2, "verses": [
            { "verse": 1, "text": "Thus the heavens and the earth were finished." }
          ] }
        ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    fn service() -> GraphService {
        GraphService::from_sources(KJV_FIXTURE, NO_XREFS, &crate::event_world::empty_atlas()).unwrap()
    }

    #[test]
    fn from_sources_publishes_and_opens_a_snapshot_at_that_version() {
        let svc = service();
        assert_eq!(svc.snapshot().version(), svc.version(), "the opened snapshot must be exactly the version this service published");
    }

    #[test]
    fn bible_position_resolves_through_the_reverse_index() {
        let svc = service();
        assert_eq!(svc.position_of(0, 1, 2), Some(1));
        assert_eq!(svc.position_of(0, 99, 1), None, "unknown verse position is None, not a panic");
    }

    #[test]
    fn chapter_span_covers_exactly_that_chapters_verses_and_no_more() {
        let svc = service();
        let (start, n) = svc.chapter_span(0, 1).unwrap();
        assert_eq!(n, 3, "Genesis 1 has 3 verses in this fixture");
        let ids = svc.snapshot().reading_window(crate::kjv_adapter::BIBLE_CORPUS, start, n);
        let decoded: Vec<_> = ids.iter().map(|id| crate::kjv_adapter::decode_text_unit(id).unwrap()).collect();
        assert_eq!(decoded, vec![(0, 1, 1), (0, 1, 2), (0, 1, 3)], "must not spill into chapter 2");
    }
}
