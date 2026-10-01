//! Reading-order queries built entirely on `atlas_graph_types::store::GraphQuery`: every function here takes
//! `&dyn GraphQuery` and touches nothing else, so ref resolution -- which needs the reading-spine reverse index
//! the port does not model -- lives on `GraphService` instead.

use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::TranslationId;

use crate::kjv_adapter::KJV_TRANSLATION;

// `GraphQuery::reading_window` takes a plain `start` with no direction, so which spine index
// the same slice starts from is this crate's concern.
atlas_graph_types::vocabulary! {
    /// Which way a window runs from the reference it is anchored on: onward from that
    /// reference, or backward to it.
    WindowDir {
        Onward => "onward",
        Backward => "backward",
    }
}

/// Which spine index a `window` call starts its slice from -- exposed so a caller computing a next
/// or previous cursor derives it from this ONE formula instead of restating it.
pub fn resolved_start(start_pos: usize, n: usize, dir: WindowDir) -> usize {
    match dir {
        WindowDir::Onward => start_pos,
        WindowDir::Backward => start_pos.saturating_sub(n.saturating_sub(1)),
    }
}

/// `dir` only changes WHICH spine index the slice starts from; the slice itself is always
/// `GraphQuery::reading_window`, so both directions are built from the query the window law proves.
pub fn window(query: &dyn GraphQuery, corpus: &'static str, start_pos: usize, n: usize, dir: WindowDir) -> Vec<AnyNodeId> {
    if n == 0 {
        return Vec::new();
    }
    query.reading_window(corpus, resolved_start(start_pos, n, dir), n)
}

pub fn render(query: &dyn GraphQuery, id: &AnyNodeId) -> Option<String> {
    render_layer(query, id, KJV_TRANSLATION)
}

/// The corpus-generic sibling of `render`: the translation key is the caller's, so a canonical layer
/// that is not the King James Version reads through the identical port-only path.
pub fn render_layer(query: &dyn GraphQuery, id: &AnyNodeId, translation: &str) -> Option<String> {
    text_in(&query.node(id)?, translation).map(str::to_string)
}

pub fn text_in<'a>(node: &'a Node, translation: &str) -> Option<&'a str> {
    match &node.payload {
        NodePayload::TextUnit { renderings, .. } => renderings.get(&TranslationId(translation.to_string())).map(String::as_str),
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
        let pos = svc.position_of(0, 1, 3).unwrap();
        let backward = window(&snap, BIBLE_CORPUS, pos, 2, WindowDir::Backward);
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
        let (graph, ..) = crate::build::build_graph_from_sources(KJV_FIXTURE, NO_XREFS, &crate::event_world::empty_atlas()).unwrap();
        let svc = service();
        let snap = svc.snapshot();
        let pos = svc.position_of(0, 1, 1).unwrap();
        assert_eq!(window(&graph, BIBLE_CORPUS, pos, 3, WindowDir::Onward), window(&snap, BIBLE_CORPUS, pos, 3, WindowDir::Onward));
    }
}
