//! `atlas chapter <ref>` -- every verse in a KJV chapter, one line each.
//! Ref parsed via `atlas_core::refs::ScriptureRef::parse` (the shared
//! locus parser, not a hand-rolled one -- R2). Concord is deliberately
//! NOT accepted here (CONTRACT.md's own "atlas chapter" section: a
//! Concord article's own paragraph count varies too widely for a
//! server-derived chapter span to mean anything consistent --
//! `graph_handlers::text_window` rejects the identical combination for
//! the same reason, and this command inherits that disclosed scope
//! limit rather than inventing its own answer).
//!
//! FIX ROUND 1 (review C-1): the per-verse loop used to map both a
//! `decode_text_unit` mismatch AND a `window::render` miss to a silent
//! skip / `.unwrap_or_default()` blank -- a real, if dormant, "silent
//! failing" violation (the committed graph never triggers either
//! condition, since `chapter_span`'s own window only ever walks real KJV
//! positions each carrying a real rendering, but the CODE PATH itself
//! disagreed with `verse.rs`'s handling of the IDENTICAL condition on the
//! IDENTICAL primitive). `render_verse_line` below is the shared,
//! fail-loud replacement -- extracted specifically so the injected-miss
//! case can be unit tested directly (this crate's own real-binary
//! integration tests, R6, can only exercise the real committed graph,
//! which cannot produce either miss; see this module's own `#[cfg(test)]`
//! block for the hand-built `GraphQuery` fixture that does).

use std::collections::HashMap;

use atlas_core::refs::ScriptureRef;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::store::GraphQuery;

use crate::error::CliError;

/// One verse's rendered output line (`"REF  text\n"`, red-letter marked),
/// or a fail-loud `CliError` -- never a silent skip or a blanked line.
/// Generic over `&impl GraphQuery` (not `GraphService`) so the missing-
/// rendering case can be reproduced against a small hand-built fixture in
/// a unit test, without needing a full `GraphService` (which has no
/// public constructor that accepts a deliberately broken node table).
fn render_verse_line(snap: &impl GraphQuery, id: &AnyNodeId, chapter_ref: &str, red_letter_spans: &HashMap<String, Vec<(usize, usize)>>) -> Result<String, CliError> {
    let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id).ok_or_else(|| {
        CliError::not_found(
            format!("no chapter '{chapter_ref}'"),
            "a position inside this chapter's own window did not decode as a KJV verse -- a graph-internal inconsistency, not a bad reference",
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ to rebuild the artifact, or report this as a bug",
        )
    })?;
    let sref = atlas_graph::kjv_adapter::dot_ref(b, c, v);
    let text = window::render(snap, id).ok_or_else(|| {
        CliError::not_found(
            format!("no text for '{sref}' inside chapter '{chapter_ref}'"),
            "this position has no KJV rendering -- a graph-internal inconsistency, not a bad reference",
            "run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ to rebuild the artifact, or report this as a bug",
        )
    })?;
    let spans = red_letter_spans.get(&sref).cloned().unwrap_or_default();
    Ok(format!("{sref}  {}\n", super::verse::mark_red_letter(&text, &spans)))
}

pub fn run(graph: &GraphService, ref_raw: &str) -> Result<String, CliError> {
    let (book, chapter) = match ScriptureRef::parse(ref_raw) {
        Ok(ScriptureRef::Chapter { book, chapter }) => (book, chapter),
        _ => {
            return Err(CliError::bad_ref(
                format!("'{ref_raw}' is not a valid chapter reference"),
                "expected BOOK.CHAPTER (e.g. GEN.1) -- not a bare book, a verse, or a Concord citation",
                "drop any verse number, or check the book code",
            ))
        }
    };

    let (start, n) = graph.chapter_span(book.0, chapter).ok_or_else(|| {
        CliError::not_found(
            format!("no chapter '{ref_raw}'"),
            "the reference parsed fine but this graph has no such book/chapter combination",
            "check the chapter number is within that book's real length",
        )
    })?;

    let snap = graph.snapshot();
    let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);

    let mut out = String::new();
    for id in &ids {
        out.push_str(&render_verse_line(&snap, id, ref_raw, &graph.red_letter_spans)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::graph::Graph;
    use atlas_graph_types::id::NodeKind;
    use atlas_graph_types::ingest::ProvenanceId;
    use atlas_graph_types::node::{Node, NodePayload};
    use atlas_graph_types::store::{GraphPublisher, GraphStore, MemStore};
    use atlas_graph_types::text::{LayerMap, TranslationId};

    /// C-1 fix round: a hand-built fixture reproducing the exact condition
    /// `.unwrap_or_default()` used to swallow -- a real TextUnit node
    /// present in the graph, at a real KJV-shaped id, but with an EMPTY
    /// `LayerMap` (no "kjv" rendering at all). The real committed
    /// `data/compiled/graph.bin` can never produce this (every verse
    /// `kjv_adapter` parses gets a rendering unconditionally), so this is
    /// injected here rather than reproduced end-to-end -- disclosed, per
    /// the review's own "or a unit test on the shared rendering path with
    /// the miss injected" instruction.
    fn snapshot_with_a_textless_verse() -> impl GraphQuery {
        let id = atlas_graph::kjv_adapter::verse_node_id(0, 1, 1); // GEN.1.1's own id shape
        let node = Node { id: id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings: LayerMap::new() }, provenance: ProvenanceId::from("test-fixture") };
        let mut g = Graph::default();
        g.nodes.insert(id, node);
        g.build_indexes();
        let mut store = MemStore::default();
        let v = store.publish(g);
        store.open(v).expect("just-published version must open")
    }

    #[test]
    fn render_verse_line_fails_loud_when_the_kjv_rendering_is_missing() {
        let snap = snapshot_with_a_textless_verse();
        let id = atlas_graph::kjv_adapter::verse_node_id(0, 1, 1);
        let err = render_verse_line(&snap, &id, "GEN.1", &HashMap::new()).expect_err("a missing rendering must be a loud error, never a blanked line");
        assert_eq!(err.code(), "not_found", "must be the not_found taxonomy class, matching verse.rs's own handling of the identical condition");
        assert_eq!(err.exit_code(), 3);
        assert!(err.to_string().contains("GEN.1.1"), "the error must name the exact verse whose rendering is missing: {err}");
    }

    #[test]
    fn render_verse_line_fails_loud_when_the_id_does_not_decode_as_a_kjv_verse() {
        let id = AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/1.1.1".to_string() };
        let snap = snapshot_with_a_textless_verse(); // any real GraphQuery; this id is never looked up
        let err = render_verse_line(&snap, &id, "GEN.1", &HashMap::new()).expect_err("an id that isn't KJV-shaped must be a loud error, never a silent skip");
        assert_eq!(err.code(), "not_found");
        assert_eq!(err.exit_code(), 3);
    }

    #[test]
    fn render_verse_line_succeeds_on_a_real_rendering() {
        let id = atlas_graph::kjv_adapter::verse_node_id(0, 1, 1);
        let mut renderings = LayerMap::new();
        renderings.insert(TranslationId(atlas_graph::kjv_adapter::KJV_TRANSLATION.to_string()), "In the beginning...".to_string());
        let node = Node { id: id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings }, provenance: ProvenanceId::from("test-fixture") };
        let mut g = Graph::default();
        g.nodes.insert(id.clone(), node);
        g.build_indexes();
        let mut store = MemStore::default();
        let v = store.publish(g);
        let snap = store.open(v).unwrap();

        let line = render_verse_line(&snap, &id, "GEN.1", &HashMap::new()).expect("a real rendering must succeed");
        assert_eq!(line, "GEN.1.1  In the beginning...\n");
    }
}
