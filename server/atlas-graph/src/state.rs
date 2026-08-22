//! `GraphState`: the one built-graph handle atlas-server holds (behind an
//! `Arc`, built once at startup). Wraps the raw `atlas_graph_types::graph::Graph`
//! with the small amount of server-side infrastructure the design doc
//! describes but the compiling `graph-types` crate deliberately omits (a
//! reading-spine reverse index for "N nodes from cursor C" queries; the
//! `GraphVersion` stamp) -- none of this is a graph-types shape change,
//! it's the adapter/storage layer the M-A brief assigns to THIS crate.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Context;

use atlas_core::data::Canon;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::TranslationId;

use crate::build::{self, BuildStats};
use crate::fidelity::{self, FidelityViolation};
use crate::kjv_adapter::{self, KJV_TRANSLATION};
use crate::version::GraphVersion;

/// Direction of a windowed reading-order query (design doc §6 `ReadDir`,
/// prose-only in the types spec -- graph-types' own `Graph::reading_window`
/// takes a plain `start: usize` with no direction, so which spine index to
/// start the SAME slice from for "backward" is this crate's own concern,
/// not a graph-types extension).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDir {
    Onward,
    Backward,
}

pub struct GraphState {
    pub graph: Graph,
    pub version: GraphVersion,
    pub stats: BuildStats,
    /// Reverse index: TextUnit node id -> its position in the bible reading
    /// spine. Built once, alongside the graph -- every "start from this
    /// ref" query (chapter view, mini-reader, split view, `/api/text`)
    /// resolves through this before calling `Graph::reading_window`.
    bible_position: BTreeMap<AnyNodeId, usize>,
    /// The exact bytes the graph was built from -- kept so
    /// `check_fidelity` re-derives its "expected" side from the SAME
    /// source without a second filesystem read, and so a fixture-built
    /// `GraphState` (tests) can run the same check with no filesystem
    /// access at all.
    kjv_source: String,
}

impl GraphState {
    pub fn from_sources(kjv_json: &str, xrefs_tsv: &str) -> anyhow::Result<Self> {
        let (graph, stats) = build::build_graph_from_sources(kjv_json, xrefs_tsv)?;
        Self::assemble(graph, stats, kjv_json.to_string())
    }

    /// Builds from an already-parsed `(Canon, verses)` pair instead of raw
    /// KJV JSON text -- see `build::build_graph_from_canon_and_verses`'s own
    /// doc comment: this is what lets a test fixture's `AtlasData` and its
    /// `GraphState` derive from ONE canon/verses value instead of two
    /// independently-authored ones that can silently disagree about which
    /// chapters exist. `check_fidelity` is not meaningful on a `GraphState`
    /// built this way (there is no raw source text to re-derive "expected"
    /// from) and is never called on one in this crate or its callers.
    pub fn from_canon_and_verses(canon: &Canon, verses: &std::collections::HashMap<String, String>, xrefs_tsv: &str) -> anyhow::Result<Self> {
        let (graph, stats) = build::build_graph_from_canon_and_verses(canon, verses, xrefs_tsv)?;
        Self::assemble(graph, stats, String::new())
    }

    fn assemble(graph: Graph, stats: BuildStats, kjv_source: String) -> anyhow::Result<Self> {
        let bible_position = graph
            .reading
            .get(kjv_adapter::BIBLE_CORPUS)
            .map(|spine| spine.order.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect())
            .unwrap_or_default();
        let version = GraphVersion::compute(&graph);
        Ok(GraphState { graph, version, stats, bible_position, kjv_source })
    }

    /// Reads `raw_dir/kjv.json` and `raw_dir/xrefs/cross_references.txt`
    /// (the same two raw sources `atlas-etl`'s own binary reads out of
    /// `data/raw/`) and builds from them. The only filesystem-touching
    /// function in this crate.
    pub fn build(raw_dir: &Path) -> anyhow::Result<Self> {
        let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json"))
            .with_context(|| format!("reading {}", raw_dir.join("kjv.json").display()))?;
        let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt"))
            .with_context(|| format!("reading {}", raw_dir.join("xrefs/cross_references.txt").display()))?;
        Self::from_sources(&kjv_json, &xrefs_tsv)
    }

    /// The FIDELITY LAW, fail-loud (design doc P3): call once at startup and
    /// refuse to serve if it fails.
    pub fn check_fidelity(&self) -> Result<(), FidelityViolation> {
        fidelity::check_kjv_fidelity(&self.kjv_source, &self.graph)
    }

    pub fn position_of(&self, book: u8, chapter: u16, verse: u16) -> Option<usize> {
        self.bible_position.get(&kjv_adapter::verse_node_id(book, chapter, verse)).copied()
    }

    /// The (start, n) window covering exactly one chapter -- `scope=chapter`'s
    /// own "N derived server-side" (M-A brief requirement 4): still just an
    /// input to the SAME `window`/`reading_window` primitive below, not a
    /// separate query mechanism. Derived from the graph's own reading spine
    /// (a contiguous run sharing (book, chapter) starting at verse 1), not
    /// from any external canon table -- this crate never needs to know
    /// chapter lengths, only walk what it already built.
    pub fn chapter_span(&self, book: u8, chapter: u16) -> Option<(usize, usize)> {
        let start = self.position_of(book, chapter, 1)?;
        let order = &self.graph.reading.get(kjv_adapter::BIBLE_CORPUS)?.order;
        let mut n = 0usize;
        for id in &order[start..] {
            match kjv_adapter::decode_text_unit(id) {
                Some((b, c, _)) if b == book && c == chapter => n += 1,
                _ => break,
            }
        }
        if n == 0 {
            None
        } else {
            Some((start, n))
        }
    }

    /// The one windowed-reading primitive every text-serving endpoint --
    /// old (`/api/chapter`, reimplemented as a view) and new (`/api/text`)
    /// alike -- calls. `dir` only changes WHICH spine index the slice
    /// starts from; the slice itself is always `Graph::reading_window`
    /// (unmodified, graph-types' own primitive), so both directions stay
    /// provably built from the query the WINDOW LAW is proven over.
    pub fn window(&self, start_pos: usize, n: usize, dir: WindowDir) -> Vec<AnyNodeId> {
        if n == 0 {
            return Vec::new();
        }
        self.graph.reading_window(kjv_adapter::BIBLE_CORPUS, Self::resolved_start(start_pos, n, dir), n)
    }

    /// Which spine index a `window(start_pos, n, dir)` call actually starts
    /// its slice from -- exposed so callers that need to compute a "next"/
    /// "previous" cursor around a window (e.g. `/api/text`'s own `next`)
    /// derive it from this ONE formula instead of restating it.
    pub fn resolved_start(start_pos: usize, n: usize, dir: WindowDir) -> usize {
        match dir {
            WindowDir::Onward => start_pos,
            WindowDir::Backward => start_pos.saturating_sub(n.saturating_sub(1)),
        }
    }

    /// The KJV rendering text of a TextUnit node, if any.
    pub fn render(&self, id: &AnyNodeId) -> Option<&str> {
        match &self.graph.nodes.get(id)?.payload {
            NodePayload::TextUnit { renderings, .. } => {
                renderings.get(&TranslationId(KJV_TRANSLATION.to_string())).map(String::as_str)
            }
            _ => None,
        }
    }

    /// The full length of the bible reading spine -- used by `next`-cursor
    /// bounds-checking in the `/api/text` handler.
    pub fn bible_len(&self) -> usize {
        self.graph.reading.get(kjv_adapter::BIBLE_CORPUS).map(|s| s.order.len()).unwrap_or(0)
    }

    pub fn bible_unit_at(&self, pos: usize) -> Option<&AnyNodeId> {
        self.graph.reading.get(kjv_adapter::BIBLE_CORPUS)?.order.get(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn fixture() -> GraphState {
        GraphState::from_sources(KJV_FIXTURE, NO_XREFS).unwrap()
    }

    #[test]
    fn chapter_span_covers_exactly_that_chapters_verses() {
        let g = fixture();
        let (start, n) = g.chapter_span(0, 1).unwrap();
        assert_eq!(n, 3, "Genesis 1 has 3 verses in this fixture");
        let ids = g.window(start, n, WindowDir::Onward);
        let texts: Vec<&str> = ids.iter().map(|id| g.render(id).unwrap()).collect();
        assert_eq!(
            texts,
            vec![
                "In the beginning God created the heaven and the earth.",
                "And the earth was without form, and void.",
                "And God said, Let there be light: and there was light.",
            ]
        );
        // The window must not spill into chapter 2.
        assert!(!texts.iter().any(|t| t.contains("Thus the heavens")));
    }

    #[test]
    fn onward_and_backward_windows_agree_on_overlap() {
        let g = fixture();
        let pos = g.position_of(0, 1, 3).unwrap(); // Genesis 1:3
        let backward = g.window(pos, 2, WindowDir::Backward); // expect [1:2, 1:3]
        let onward = g.window(g.position_of(0, 1, 2).unwrap(), 2, WindowDir::Onward); // expect [1:2, 1:3]
        assert_eq!(backward, onward, "the same two verses, reached from either direction, must be the SAME window");
    }

    #[test]
    fn unknown_verse_position_is_none_not_a_panic() {
        let g = fixture();
        assert_eq!(g.position_of(0, 99, 1), None);
    }
}
