//! LEX-1 (spec §7) real-data tests over the adapter: the full graph built
//! from the real vendored sources WITH the lexicon corpus. Counts are the
//! reader's own pinned coverage (`atlas-etl/tests/lexicon_real_data.rs`):
//! 13,548 entries; 452,689 tokens of which 21,409 are `Align=unmatched`, so
//! 431,280 `Occurs` rows -- every one resolving to an entry node and a
//! verse node, in canonical reading order.

use std::path::Path;

use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::text::TextRef;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn built() -> (atlas_graph_types::graph::Graph, atlas_graph::build::BuildStats) {
    let raw = data_dir().join("raw");
    let curated = data_dir().join("curated");
    let atlas = atlas_etl::compile::compile(&raw, &curated).expect("the ETL compiles").data;
    let kjv_json = std::fs::read_to_string(raw.join("kjv.json")).unwrap();
    let xrefs_tsv = std::fs::read_to_string(raw.join("xrefs/cross_references.txt")).unwrap();
    let brainfuel = atlas_etl::brainfuel::read_all(&raw.join("brain-fuel-bible")).expect("brain-fuel");
    let lexicon = atlas_etl::lexicon::read_all(&raw.join("brain-fuel-bible")).expect("the vendored lexicon + morphology");
    let (mut g, stats, _, _) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(
        &kjv_json,
        &xrefs_tsv,
        &atlas,
        &[],
        Some(&brainfuel),
        None,
        None,
        None,
        Some(&lexicon),
    )
    .expect("the real sources build");
    g.build_indexes();
    (g, stats)
}

#[test]
fn the_adapter_authors_every_entry_and_one_occurs_row_per_aligned_token() {
    let (g, stats) = built();
    assert_eq!(stats.lexicon.entries, 13_548);
    assert_eq!(stats.lexicon.tokens_unmatched, 6615 + 14_794);
    assert_eq!(stats.lexicon.tokens_without_entry, 0, "every aligned token's Strong's id has an entry");
    assert_eq!(stats.lexicon.tokens_off_canon, 0, "both texts share the 31,102-verse canon");
    assert_eq!(stats.lexicon.occurs, 431_280);
    assert_eq!(g.occurs.len(), 431_280);
    assert_eq!(g.nodes.values().filter(|n| n.id.kind == NodeKind::LexiconEntry).count(), 13_548);
    atlas_graph::law_check::every_authored_edge_resolves(&g).expect("every Occurs row's entry and verse exist");
}

#[test]
fn occurs_rows_are_in_reading_order_and_carry_one_token_spans_with_the_right_provenance() {
    let (g, _) = built();
    let mut prev = None;
    for row in &g.occurs {
        let TextRef::Bible(v) = &row.locus.at else { panic!("a bible locus") };
        let span = row.locus.span.as_ref().expect("a word locus carries its span");
        assert_eq!(span.start, span.end, "one token");
        let key = (v.book, v.chapter, v.verse, span.start);
        if let Some(p) = prev {
            assert!(p < key, "{p:?} !< {key:?}");
        }
        prev = Some(key);
        let (layer, prov) = if v.book < 39 { ("hebrew_masoretic", "stepbible-tahot") } else { ("greek_textus_receptus", "stepbible-tagnt") };
        assert_eq!(span.layer.0, layer);
        assert_eq!(row.provenance, prov);
    }
}

#[test]
fn john_3_16_has_words_in_token_order_and_logos_has_a_canonical_concordance() {
    use atlas_graph_types::edge::at;
    use atlas_graph_types::explore::EdgeQuery;
    use atlas_graph_types::id::AnyNodeId;
    use atlas_graph_types::store::GraphQuery;
    let (g, _) = built();
    let jhn = atlas_core::canon::resolve_alias("John").unwrap().0;
    let verse = at(&atlas_graph::kjv_adapter::verse_node_id(jhn, 3, 16));
    let words = g.edges_with_nodes(&verse, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Inverse), cursor: None, limit: 100 });
    // 26 tokens -> 26 index entries in token order (spec 7.3: "its tagged
    // tokens in ord"); the article (G3588) five times and "him" (G0846)
    // twice share ONE edge id each, so 21 distinct edge ids.
    assert_eq!(words.entries.len(), 26);
    let distinct: std::collections::BTreeSet<_> = words.entries.iter().map(|e| e.entry.edge.clone()).collect();
    assert_eq!(distinct.len(), 21);
    let raw_of = |p: &atlas_graph_types::id::Position| match p {
        atlas_graph_types::id::Position::Node(id) => id.clone(),
        other => panic!("a node position, not {other:?}"),
    };
    let ids: Vec<String> = words.entries.iter().map(|e| raw_of(&e.entry.node).raw).collect();
    assert_eq!(&ids[..5], &["G3779", "G1063", "G0025", "G3588", "G2316"]);
    let article = words.entries.iter().find(|e| raw_of(&e.entry.node).raw == "G3588").unwrap();
    assert_eq!(g.rows_behind(&article.entry.edge).len(), 5, "five rows behind the article's one edge");
    let entry = |s: &str| at(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: s.into() });
    let logos = g.edges_with_nodes(&entry("G3056"), &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Forward), cursor: None, limit: 5 });
    let first = raw_of(&logos.entries[0].entry.node);
    assert_eq!(atlas_graph::kjv_adapter::decode_text_unit(&first), Some((jhn - 3, 5, 32)), "λόγος first occurs at MAT 5:32");
    assert_eq!(logos.next, Some(5));
    let all = g.edge_summary(&entry("G3056"));
    let n = all.get(&EdgeKind::Directed(RelationId::Occurs, Direction::Forward)).copied().unwrap_or(0);
    assert!(n > 250 && n <= 319, "319 tokens over at most that many verses: {n}");
    let node = g.node(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() }).unwrap();
    match node.payload {
        atlas_graph_types::node::NodePayload::LexiconEntry { lemma, glosses, .. } => {
            assert_eq!(lemma, "λ\u{1f79}γος");
            assert_eq!(glosses[1], "word");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(node.provenance, "stepbible-tbesg");
}
