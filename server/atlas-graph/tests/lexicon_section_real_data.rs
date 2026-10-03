mod common;

use common::committed_sections;

use atlas_graph::sqlite::manifest::read_manifest;
use atlas_graph::sqlite::source::SectionLayout;
use atlas_graph::sqlite::SCHEMA_VERSION;
use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
use atlas_graph_types::adjacency::{Cursor, EdgeQuery};
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::sections::Section;
use atlas_graph_types::store::GraphQuery;

const PINNED: [(&str, &str); 4] = [
    ("core", "72c0b6f5e507d52c7d6ecc9e2e09866f"),
    ("kjv", "5666e5a828c6df8dfd3e2f71807fdba2"),
    ("concord", "0858b72e2c3f1b12b495c7092b8467a2"),
    ("kretzmann", "02c86d1b9045d4b39c5501c24fded6a0"),
];

#[test]
fn the_other_four_sections_hashes_are_byte_identical_and_the_lexicon_line_is_fifth_and_optional() {
    let m = read_manifest(&SectionLayout::under(&common::compiled_dir()).manifest_path()).expect("manifest.toml");
    let names: Vec<&str> = m.sections.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, Section::MANIFEST_ORDER.iter().map(|s| s.name()).collect::<Vec<_>>());
    for (name, logical) in PINNED {
        let s = m.sections.iter().find(|s| s.name == name).unwrap();
        assert_eq!(s.logical, logical, "{name}: LEX-1 must not move any other section's hash (spec 8 row 7; core: only the registry rows, see PINNED)");
    }
    let lx = &m.sections[4];
    assert_eq!(lx.name, "lexicon");
    assert!(!lx.required, "the lexicon section is optional (spec 2.1)");
    assert_eq!(lx.schema_version, SCHEMA_VERSION);
    assert!(lx.bytes < 104_857_600, "under GitHub's per-file limit: {} bytes", lx.bytes);
    assert!(lx.bytes > 1_000_000, "a real section, not an empty one: {} bytes", lx.bytes);
    assert_ne!(m.root, "9c9697b846cd8625e475b135c7dea11e", "the root moves once (a fifth manifest line)");
}

fn node_of(p: &Position) -> AnyNodeId {
    match p {
        Position::Node(id) => id.clone(),
        other => panic!("a node position, not {other:?}"),
    }
}

#[test]
fn the_served_sections_carry_every_entry_and_a_verses_words_in_token_order() {
    let snap = committed_sections();
    assert_eq!(snap.present(), &Section::MANIFEST_ORDER[..]);
    let page = snap.nodes_of_kind(NodeKind::LexiconEntry, None, 1);
    assert_eq!(page.ids[0].raw, "G0001", "id (byte) order: the first Greek entry");
    let mut total = 1;
    let mut cursor = page.next;
    while let Some(c) = cursor {
        let p = snap.nodes_of_kind(NodeKind::LexiconEntry, Some(c), 5000);
        total += p.ids.len();
        cursor = p.next;
    }
    assert_eq!(total, 13_548);

    let jhn = atlas_core::canon::resolve_alias("John").unwrap().0;
    let verse = at(&atlas_graph::kjv_adapter::verse_node_id(jhn, 3, 16));
    let summary = snap.edge_summary(&verse);
    assert_eq!(summary.get(&EdgeKind::Directed(RelationId::Occurs, Direction::Inverse)).copied(), Some(21), "26 tagged tokens of 21 entries: one edge per entry, in ord (spec 7.3)");
    let words = snap.edges_with_nodes(&verse, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Inverse), cursor: Cursor::FIRST, limit: 100 });
    let ids: Vec<String> = words.entries.iter().map(|e| node_of(&e.entry.node).raw).collect();
    assert_eq!(&ids[..5], &["G3779", "G1063", "G0025", "G3588", "G2316"], "token order, not id order");
    let article = words.entries.iter().find(|e| node_of(&e.entry.node).raw == "G3588").unwrap();
    let rows = snap.rows_behind(&article.entry.edge);
    assert_eq!(rows.len(), 5, "the article five times in the verse: five rows behind one edge");
    assert!(rows.iter().all(|r| r.provenance == "stepbible-tagnt"));
    assert!(words.entries.iter().all(|e| e.node.is_some()), "every entry node rides with its edge");
}

#[test]
fn an_entrys_concordance_is_in_canonical_order_and_the_payload_is_as_published() {
    let snap = committed_sections();
    let logos = at(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() });
    let first = snap.edges_with_nodes(&logos, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Forward), cursor: Cursor::FIRST, limit: 3 });
    let mat = atlas_core::canon::resolve_alias("Matthew").unwrap().0;
    let verses: Vec<(u8, u16, u16)> = first.entries.iter().map(|e| atlas_graph::kjv_adapter::decode_text_unit(&node_of(&e.entry.node)).unwrap()).collect();
    assert_eq!(verses[0], (mat, 5, 32), "λόγος first occurs at Matthew 5:32");
    assert!(verses.windows(2).all(|w| w[0] < w[1]), "canonical reading order: {verses:?}");
    let node = snap.node(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() }).unwrap();
    match node.payload {
        atlas_graph_types::node::NodePayload::LexiconEntry { strong, lang, lemma, translit, glosses, root, .. } => {
            assert_eq!(strong, "G3056");
            assert_eq!(lang, "grc");
            assert_eq!(lemma, "λ\u{1f79}γος");
            assert_eq!(translit.as_deref(), Some("lógos"));
            assert_eq!(glosses[1], "word");
            assert_eq!(root.as_deref(), Some("G3004"));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(node.provenance, "stepbible-tbesg");
    let elohim = snap.node(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "H0430".into() }).unwrap();
    let gen = atlas_core::canon::resolve_alias("Genesis").unwrap().0;
    let first_h = snap.edges_with_nodes(&at(&elohim.id), &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Forward), cursor: Cursor::FIRST, limit: 1 });
    assert_eq!(atlas_graph::kjv_adapter::decode_text_unit(&node_of(&first_h.entries[0].entry.node)), Some((gen, 1, 1)));
    assert!(snap.rows_behind(&first_h.entries[0].entry.edge).iter().all(|r| r.provenance == "stepbible-tahot"));
}
