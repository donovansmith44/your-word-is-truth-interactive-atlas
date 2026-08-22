//! The KJV adapter's fidelity boundary law (design doc P3; types doc §5's
//! `BoundaryLaw` trait): BIJECTION (every source verse becomes exactly one
//! TextUnit rendering) and RECONSTRUCTION (the full reading-order walk
//! reproduces the source text byte-for-byte). Fail-loud: `check_kjv_fidelity`
//! is called at server startup (`GraphState::check_fidelity`) and a failure
//! there must refuse to serve traffic, not degrade quietly.

use std::collections::BTreeSet;

use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::TranslationId;

use crate::kjv_adapter::{self, KJV_TRANSLATION};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FidelityViolation(pub String);

impl std::fmt::Display for FidelityViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KJV adapter fidelity violation: {}", self.0)
    }
}
impl std::error::Error for FidelityViolation {}

/// Re-parses `source_kjv_json` (the SAME raw bytes the graph was built
/// from) via the SAME adapter reader (`kjv_adapter::read_kjv_ordered`) to
/// get the independently-derived "expected" ordered verse list, then checks
/// `built` against it. Re-deriving from source on every check (rather than
/// trusting whatever the builder happened to record) is the point: this is
/// the proof that source and built graph agree, not a self-consistency
/// check of the builder alone.
pub fn check_kjv_fidelity(source_kjv_json: &str, built: &Graph) -> Result<(), FidelityViolation> {
    let (_canon, expected) = kjv_adapter::read_kjv_ordered(source_kjv_json)
        .map_err(|e| FidelityViolation(format!("source failed to parse: {e}")))?;

    // BIJECTION: every source verse <-> exactly one TextUnit node, and its
    // rendering matches the source text verbatim.
    let mut expected_ids = BTreeSet::new();
    for v in &expected {
        let id = kjv_adapter::verse_node_id(v.book_index, v.chapter, v.verse);
        expected_ids.insert(id.clone());
        let node = built
            .nodes
            .get(&id)
            .ok_or_else(|| FidelityViolation(format!("source verse {} has no TextUnit node in the built graph", kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse))))?;
        let rendering = match &node.payload {
            NodePayload::TextUnit { renderings, .. } => renderings.get(&TranslationId(KJV_TRANSLATION.to_string())),
            _ => None,
        }
        .ok_or_else(|| FidelityViolation(format!("TextUnit {} carries no KJV rendering", kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse))))?;
        if rendering != &v.text {
            return Err(FidelityViolation(format!(
                "TextUnit {} rendering does not match its source text byte-for-byte",
                kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse)
            )));
        }
    }
    // ...and no EXTRA TextUnit node claims to be a KJV verse the source
    // doesn't have (bijection is two-sided).
    for id in built.nodes.keys() {
        if id.kind == NodeKind::TextUnit && !expected_ids.contains(id) {
            return Err(FidelityViolation(format!("built graph has a TextUnit node {id:?} with no corresponding source verse")));
        }
    }

    // RECONSTRUCTION: the full reading-order walk, in order, reproduces the
    // source text byte-for-byte.
    let spine = built
        .reading
        .get(kjv_adapter::BIBLE_CORPUS)
        .ok_or_else(|| FidelityViolation("built graph has no bible reading spine".to_string()))?;
    if spine.order.len() != expected.len() {
        return Err(FidelityViolation(format!(
            "reading-order spine has {} units but the source has {} verses",
            spine.order.len(),
            expected.len()
        )));
    }
    for (id, v) in spine.order.iter().zip(expected.iter()) {
        let expected_id = kjv_adapter::verse_node_id(v.book_index, v.chapter, v.verse);
        if *id != expected_id {
            return Err(FidelityViolation(format!(
                "reading order diverges from source order at {} (spine has {:?})",
                kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse),
                id
            )));
        }
    }
    let reconstructed: Vec<&str> = spine
        .order
        .iter()
        .map(|id| match &built.nodes[id].payload {
            NodePayload::TextUnit { renderings, .. } => {
                renderings.get(&TranslationId(KJV_TRANSLATION.to_string())).map(String::as_str).unwrap_or_default()
            }
            _ => "",
        })
        .collect();
    let source_texts: Vec<&str> = expected.iter().map(|v| v.text.as_str()).collect();
    if reconstructed != source_texts {
        return Err(FidelityViolation("the reading-order walk does not reproduce the source text byte-for-byte".to_string()));
    }

    Ok(())
}

/// Symbolic conformance to the types doc's own `BoundaryLaw` trait shape
/// (§5 of the ingestion contract) -- not load-bearing machinery (nothing in
/// graph-types consumes `dyn BoundaryLaw` today), but documents that this
/// adapter DOES ship the boundary law the contract calls for, in the exact
/// shape the contract names.
pub struct KjvBoundaryLaw;

impl atlas_graph_types::ingest::BoundaryLaw for KjvBoundaryLaw {
    fn check(&self, source: &atlas_graph_types::ingest::SourceBytes, built: &Graph) -> Result<(), atlas_graph_types::ingest::LawViolation> {
        let text = std::str::from_utf8(&source.0)
            .map_err(|e| atlas_graph_types::ingest::LawViolation(format!("source is not valid utf-8: {e}")))?;
        check_kjv_fidelity(text, built).map_err(|e| atlas_graph_types::ingest::LawViolation(e.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::build_graph_from_sources;

    const GOOD_KJV: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
          { "verse": 2, "text": "And the earth was without form, and void." }
        ] } ] }
      ]
    }"#;
    const NO_XREFS: &str = "From Verse\tTo Verse\tVotes\t#comment\n";

    #[test]
    fn green_on_a_clean_fixture() {
        let (graph, _) = build_graph_from_sources(GOOD_KJV, NO_XREFS).unwrap();
        assert_eq!(check_kjv_fidelity(GOOD_KJV, &graph), Ok(()));
    }

    #[test]
    fn red_when_a_verse_is_dropped_from_the_built_graph() {
        let (mut graph, _) = build_graph_from_sources(GOOD_KJV, NO_XREFS).unwrap();
        let dropped_id = kjv_adapter::verse_node_id(0, 1, 2);
        graph.nodes.remove(&dropped_id);
        if let Some(spine) = graph.reading.get_mut(kjv_adapter::BIBLE_CORPUS) {
            spine.order.retain(|id| *id != dropped_id);
        }
        let result = check_kjv_fidelity(GOOD_KJV, &graph);
        assert!(result.is_err(), "a graph missing a source verse must fail the bijection check");
    }

    #[test]
    fn red_when_a_rendering_byte_is_mutated() {
        let (mut graph, _) = build_graph_from_sources(GOOD_KJV, NO_XREFS).unwrap();
        let id = kjv_adapter::verse_node_id(0, 1, 1);
        if let Some(node) = graph.nodes.get_mut(&id) {
            if let NodePayload::TextUnit { renderings, .. } = &mut node.payload {
                let text = renderings.get_mut(&TranslationId(KJV_TRANSLATION.to_string())).unwrap();
                text.replace_range(0..1, "X"); // "In the..." -> "Xn the..."
            }
        }
        let result = check_kjv_fidelity(GOOD_KJV, &graph);
        assert!(result.is_err(), "a mutated byte must fail the bijection/reconstruction check");
    }

    #[test]
    fn green_on_the_real_committed_kjv_source() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt"))
            .expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");
        let (graph, stats) = build_graph_from_sources(&kjv_json, &xrefs_tsv).expect("the real KJV source must parse");
        assert_eq!(stats.kjv_verses, 31_102, "the real KJV text is 31,102 verses");
        assert_eq!(check_kjv_fidelity(&kjv_json, &graph), Ok(()), "the real KJV source must satisfy its own bijection + reconstruction law");
    }
}
