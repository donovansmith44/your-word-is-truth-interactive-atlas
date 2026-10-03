//! `bibex chapter <ref>` -- every verse of a KJV chapter, one line each. A Concord ref is
//! refused: an article's paragraph count varies too widely for a chapter span to mean
//! anything consistent.

use std::collections::HashMap;

use atlas_core::refs::ScriptureRef;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::store::GraphQuery;

use crate::error::CliError;

/// Never a silent skip or a blanked line. Generic over the query so a node carrying no
/// rendering at all can be injected in a test.
fn resolve_verse_line(snap: &impl GraphQuery, id: &AnyNodeId, chapter_ref: &str) -> Result<(String, String), CliError> {
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
    Ok((sref, text))
}

fn render_verse_line(snap: &impl GraphQuery, id: &AnyNodeId, chapter_ref: &str, red_letter_spans: &HashMap<String, Vec<(usize, usize)>>) -> Result<String, CliError> {
    let (sref, text) = resolve_verse_line(snap, id, chapter_ref)?;
    let spans = red_letter_spans.get(&sref).cloned().unwrap_or_default();
    Ok(format!("{sref}  {}\n", super::verse::mark_red_letter(&text, &spans)))
}

/// The two fail-loud steps both output modes must agree on: `bad_ref` on a ref that is not
/// chapter-shaped, `not_found` on a well-shaped book/chapter that is absent.
fn resolve_span(graph: &GraphService, ref_raw: &str) -> Result<(usize, usize), CliError> {
    let (book, chapter) = match ScriptureRef::parse(ref_raw) {
        Ok(ScriptureRef::Chapter(atlas_core::identity::ChapterReference { book, chapter })) => (book, chapter),
        _ => {
            return Err(CliError::bad_ref(
                format!("'{ref_raw}' is not a valid chapter reference"),
                "expected BOOK.CHAPTER (e.g. GEN.1) -- not a bare book, a verse, or a Concord citation",
                "drop any verse number, or check the book code",
            ))
        }
    };

    graph.chapter_span(book.0, chapter).ok_or_else(|| {
        CliError::not_found(
            format!("no chapter '{ref_raw}'"),
            "the reference parsed fine but this graph has no such book/chapter combination",
            "check the chapter number is within that book's real length",
        )
    })
}

pub fn run(graph: &GraphService, ref_raw: &str) -> Result<String, CliError> {
    let (start, n) = resolve_span(graph, ref_raw)?;

    let snap = graph.snapshot();
    let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);

    let mut out = String::new();
    for id in &ids {
        out.push_str(&render_verse_line(&snap, id, ref_raw, &graph.red_letter_spans)?);
    }
    Ok(out)
}

/// No `next` field: a chapter's span is always the whole chapter, never a page of it.
pub fn run_json(graph: &GraphService, ref_raw: &str) -> Result<serde_json::Value, CliError> {
    let (start, n) = resolve_span(graph, ref_raw)?;

    let snap = graph.snapshot();
    let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward);

    let mut units = Vec::with_capacity(ids.len());
    for id in &ids {
        let (sref, text) = resolve_verse_line(&snap, id, ref_raw)?;
        let spans = graph.red_letter_spans.get(&sref).cloned().unwrap_or_default();
        let woc: Vec<_> = spans.iter().map(|&(s, e)| serde_json::json!({"start": s, "end": e})).collect();
        units.push(serde_json::json!({"ref": sref, "text": text, "words_of_christ": woc}));
    }
    Ok(serde_json::Value::Array(units))
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

    fn snapshot_with_a_textless_verse() -> impl GraphQuery {
        let id = atlas_graph::kjv_adapter::verse_node_id(0, 1, 1);
        let node = Node { id: id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings: LayerMap::new() }, provenance: ProvenanceId::from("test-fixture") };
        let mut g = Graph::default();
        g.nodes.insert(id, node);
        g.build_indexes();
        let mut store = MemStore::default();
        let v = store.publish(g).unwrap();
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
        let snap = snapshot_with_a_textless_verse();
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
        let v = store.publish(g).unwrap();
        let snap = store.open(v).unwrap();

        let line = render_verse_line(&snap, &id, "GEN.1", &HashMap::new()).expect("a real rendering must succeed");
        assert_eq!(line, "GEN.1.1  In the beginning...\n");
    }
}
