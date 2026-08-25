//! The window/text path — reading-order queries built ENTIRELY on top of
//! `atlas_graph_types::store::GraphQuery` (design doc §9a; fix round 1,
//! C1: "the server binds to the PORT... do NOT define any parallel store/
//! query trait in atlas-graph"). Every function here takes `&dyn GraphQuery`
//! and touches nothing else — no `atlas_graph_types::graph::Graph` field,
//! no backend-specific state. `Graph` itself implements `GraphQuery`
//! (graph-types' own canonical instance), and so does `MemSnapshot`
//! (implementation #1, by pure delegation) — this module works identically
//! against either, and against whatever M-C's serialized artifact (or any
//! future database) implements the SAME trait as.
//!
//! Ref-resolution (`chapter_span`/`position_of`, needing the reading-spine
//! reverse index `GraphQuery` doesn't model) lives on `GraphService`
//! instead of here — see that module's own doc comment for why that split
//! is the right one: everything backend-agnostic lives here, generic over
//! the port; the one adapter-specific companion index stays with the
//! concrete service that built it.

use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::TranslationId;

use crate::kjv_adapter::KJV_TRANSLATION;

/// Direction of a windowed reading-order query (design doc §6 `ReadDir`,
/// prose-only in the types spec — `GraphQuery::reading_window` takes a
/// plain `start: usize` with no direction; which spine index to start the
/// SAME slice from for "backward" is this crate's own concern).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDir {
    Onward,
    Backward,
}

/// Which spine index a `window(query, corpus, start_pos, n, dir)` call
/// actually starts its slice from — exposed so a caller computing a
/// "next"/"previous" cursor around a window derives it from this ONE
/// formula instead of restating it.
pub fn resolved_start(start_pos: usize, n: usize, dir: WindowDir) -> usize {
    match dir {
        WindowDir::Onward => start_pos,
        WindowDir::Backward => start_pos.saturating_sub(n.saturating_sub(1)),
    }
}

/// The one windowed-reading primitive every text-serving endpoint — old
/// (`/api/chapter`, a view) and new (`/api/text`) alike — calls. `dir`
/// only changes WHICH spine index the slice starts from; the slice itself
/// is always `GraphQuery::reading_window` (the port's own primitive,
/// unmodified), so both directions stay provably built from the query the
/// WINDOW LAW is proven over. Pure port consumption: works against `&Graph`
/// directly, a `MemSnapshot`, or any other `GraphQuery` implementor.
pub fn window(query: &dyn GraphQuery, corpus: &'static str, start_pos: usize, n: usize, dir: WindowDir) -> Vec<AnyNodeId> {
    if n == 0 {
        return Vec::new();
    }
    query.reading_window(corpus, resolved_start(start_pos, n, dir), n)
}

/// The KJV rendering text of a TextUnit node, if any — via
/// `GraphQuery::node`, never a direct field reach.
pub fn render(query: &dyn GraphQuery, id: &AnyNodeId) -> Option<String> {
    render_layer(query, id, KJV_TRANSLATION)
}

/// CORP-2a: the corpus-generic sibling of `render` — same shape, the
/// translation KEY is a caller-supplied parameter instead of the
/// hardcoded KJV constant, so a Concord TextUnit's own canonical layer
/// (`concord_adapter::CONCORD_TRANSLATION`, "bente-dau" — genuinely NOT
/// the King James Version) reads through the identical port-only path.
/// `render` above is now a thin wrapper (byte-identical behavior for
/// every existing caller).
pub fn render_layer(query: &dyn GraphQuery, id: &AnyNodeId, translation: &str) -> Option<String> {
    match query.node(id)?.payload {
        NodePayload::TextUnit { renderings, .. } => renderings.get(&TranslationId(translation.to_string())).cloned(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kjv_adapter::BIBLE_CORPUS;
    use crate::service::GraphService;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [
          { "chapter": 1, "verses": [
            { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
            { "verse": 2, "text": "And the earth was without form, and void." },
            { "verse": 3, "text": "And God said, Let there be light: and there was light." }
          ] }
        ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    fn service() -> GraphService {
        GraphService::from_sources(KJV_FIXTURE, NO_XREFS, &crate::event_world::empty_atlas()).unwrap()
    }

    #[test]
    fn onward_and_backward_windows_agree_on_overlap() {
        let svc = service();
        let snap = svc.snapshot();
        let pos = svc.position_of(0, 1, 3).unwrap(); // Genesis 1:3
        let backward = window(&snap, BIBLE_CORPUS, pos, 2, WindowDir::Backward); // expect [1:2, 1:3]
        let onward = window(&snap, BIBLE_CORPUS, svc.position_of(0, 1, 2).unwrap(), 2, WindowDir::Onward);
        assert_eq!(backward, onward, "the same two verses, reached from either direction, must be the SAME window");
    }

    #[test]
    fn render_reads_through_the_port_only() {
        let svc = service();
        let snap = svc.snapshot();
        let id = crate::kjv_adapter::verse_node_id(0, 1, 1);
        assert_eq!(render(&snap, &id).as_deref(), Some("In the beginning God created the heaven and the earth."));
    }

    /// CORP-2a: `render` is a thin wrapper over `render_layer` -- proves
    /// the delegation directly (same query, same id, same result) and
    /// that a DIFFERENT translation key on the SAME node reads back
    /// `None` (never a wrong-layer's text) — the exact guard a Concord
    /// TextUnit's own non-KJV layer needs.
    #[test]
    fn render_layer_generalizes_render_and_is_layer_selective() {
        let svc = service();
        let snap = svc.snapshot();
        let id = crate::kjv_adapter::verse_node_id(0, 1, 1);
        assert_eq!(render_layer(&snap, &id, crate::kjv_adapter::KJV_TRANSLATION), render(&snap, &id));
        assert_eq!(render_layer(&snap, &id, "bente-dau"), None, "a translation key this node carries no layer for is None, not a guess");
    }

    #[test]
    fn window_works_identically_against_the_raw_graph_and_a_snapshot() {
        // Proves the "generic over any GraphQuery implementor" claim
        // literally: build once, then query both the raw Graph (which
        // implements GraphQuery directly, graph-types' own canonical
        // instance) and a MemSnapshot presenting the SAME content, and
        // confirm this module's own functions agree.
        let (graph, ..) = crate::build::build_graph_from_sources(KJV_FIXTURE, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let svc = service();
        let snap = svc.snapshot();
        let pos = svc.position_of(0, 1, 1).unwrap();
        assert_eq!(window(&graph, BIBLE_CORPUS, pos, 3, WindowDir::Onward), window(&snap, BIBLE_CORPUS, pos, 3, WindowDir::Onward));
    }
}
