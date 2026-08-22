//! Graph construction: pure `&str`-in/`Graph`-out builders (mirrors
//! atlas-etl's own "every module is `&str`-in / data-out; only the caller
//! touches the filesystem" discipline -- `state::GraphState::build` is the
//! one place in this crate that reads files).
//!
//! Controller ruling 2 (M-A pragmatic materialization): builds the whole
//! graph IN MEMORY from the raw sources, once, at call time (server
//! startup calls this once via `GraphState::build`) -- 31,102 KJV verses
//! plus ~344k raw cross-reference rows is well within "trivial startup
//! work." The serialized one-artifact form (design doc P1, fully realized)
//! lands at M-C when the parallel `data/compiled/*.json` artifacts retire.

use std::collections::HashMap;

use anyhow::Context;

use atlas_core::data::Canon;
use atlas_graph_types::edge::CrossRef as GraphCrossRef;
use atlas_graph_types::graph::{Graph, ReadingSpine};

use crate::{kjv_adapter, xref_adapter};

/// Startup-log-friendly counts -- also asserted against in tests (the
/// FIDELITY LAW's own bijection count and the xref adapter's disclosed
/// negative-vote drop are both externally observable here, not just log
/// noise).
#[derive(Debug, Clone, Default)]
pub struct BuildStats {
    pub kjv_verses: usize,
    pub cites_rows: usize,
    pub cites_dropped_negative_votes: usize,
}

/// Builds the graph from the two raw sources' own text content -- the KJV
/// adapter (first relation to prove node materialization) and the
/// cross-references adapter (first relation to prove the generic edge path
/// end-to-end, controller ruling 3). Node insertion, the reading spine, and
/// the `cites` row table are all populated here; `Graph::build_indexes` (an
/// unmodified graph-types primitive) does the one BiIndex pass at the end.
pub fn build_graph_from_sources(kjv_json: &str, xrefs_tsv: &str) -> anyhow::Result<(Graph, BuildStats)> {
    let (canon, verses) = atlas_etl::kjv::parse(kjv_json).context("parsing the KJV source (kjv.json)")?;
    build_graph_from_canon_and_verses(&canon, &verses, xrefs_tsv)
}

/// The same build, starting from an already-parsed `(Canon, verses)` pair
/// instead of raw KJV JSON text -- lets a caller that already has both (an
/// `AtlasData` fixture, most notably) build a graph that is BY CONSTRUCTION
/// consistent with that same canon/verses, rather than maintaining a second,
/// independently-authored raw-JSON fixture that can silently drift from it.
pub fn build_graph_from_canon_and_verses(canon: &Canon, verses: &HashMap<String, String>, xrefs_tsv: &str) -> anyhow::Result<(Graph, BuildStats)> {
    let ordered = kjv_adapter::ordered_verses_from_canon(canon, verses).context("walking the KJV canon/verses into reading order")?;

    let mut graph = Graph::default();
    let mut order = Vec::with_capacity(ordered.len());
    let mut verses_by_ref: HashMap<String, String> = HashMap::with_capacity(ordered.len());
    for v in &ordered {
        let node = kjv_adapter::verse_node(v);
        let id = node.id.clone();
        graph.nodes.insert(id.clone(), node);
        order.push(id);
        verses_by_ref.insert(kjv_adapter::dot_ref(v.book_index, v.chapter, v.verse), v.text.clone());
    }
    graph.reading.insert(kjv_adapter::BIBLE_CORPUS, ReadingSpine { order });

    let (xref_rows, xref_stats) =
        xref_adapter::read_xrefs_ordered(xrefs_tsv, &verses_by_ref).context("parsing the cross-references source (xrefs/cross_references.txt)")?;
    for row in &xref_rows {
        graph.cross_refs.push(GraphCrossRef {
            from: row.from.clone(),
            to: row.to.clone(),
            votes: row.votes,
            provenance: "openbible.info-cross-references".to_string(),
        });
    }

    graph.build_indexes();

    let stats = BuildStats {
        kjv_verses: ordered.len(),
        cites_rows: xref_rows.len(),
        cites_dropped_negative_votes: xref_stats.dropped_negative_votes,
    };
    Ok((graph, stats))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::{Explorable, PositionRef};
    use atlas_graph_types::id::Position;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
          { "verse": 2, "text": "And the earth was without form, and void." }
        ] } ] },
        { "name": "I Samuel", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "Now there was a certain man of Ramathaimzophim." }
        ] } ] },
        { "name": "Revelation of John", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "The Revelation of Jesus Christ." }
        ] } ] }
      ]
    }"#;

    const XREFS_FIXTURE: &str = "From Verse\tTo Verse\tVotes\t#comment\nGen.1.1\t1Sam.1.1\t9\n";

    #[test]
    fn builds_one_text_unit_per_verse_in_canon_order() {
        let (graph, stats) = build_graph_from_sources(KJV_FIXTURE, XREFS_FIXTURE).unwrap();
        assert_eq!(stats.kjv_verses, 4);
        assert_eq!(graph.nodes.len(), 4);
        let spine = graph.reading.get(kjv_adapter::BIBLE_CORPUS).expect("bible reading spine must exist");
        assert_eq!(spine.order.len(), 4);
        let decoded: Vec<_> = spine.order.iter().map(|id| kjv_adapter::decode_text_unit(id).unwrap()).collect();
        // Genesis (book 0) before 1 Samuel (book 8) before Revelation (book 65).
        assert_eq!(decoded, vec![(0, 1, 1), (0, 1, 2), (8, 1, 1), (65, 1, 1)]);
    }

    #[test]
    fn cites_row_is_queryable_through_the_generic_explorable_machinery() {
        let (graph, stats) = build_graph_from_sources(KJV_FIXTURE, XREFS_FIXTURE).unwrap();
        assert_eq!(stats.cites_rows, 1);

        let gen11 = kjv_adapter::verse_node_id(0, 1, 1);
        let cites = EdgeKind::Directed(RelationId::Cites, Direction::Forward);
        let page = PositionRef(Position::Node(gen11))
            .edges(&graph, &atlas_graph_types::explore::EdgeQuery { kind: cites, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1);
        let target = kjv_adapter::verse_node_id(8, 1, 1);
        assert_eq!(page.entries[0].node, Position::Node(target));
    }
}
