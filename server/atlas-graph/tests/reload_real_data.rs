mod common;

use common::{CORPUS_ROOTS, MAPS};

use atlas_graph::sqlite::reload::committed_graph;
use atlas_graph_types::sections::{logical_dump_section, version_root, Section};

#[test]
fn the_sections_read_back_into_the_graph_that_wrote_them() {
    let t = std::time::Instant::now();
    let (g, snap) = committed_graph(&common::compiled_dir()).unwrap();
    println!("DB-5 RELOAD: {} nodes read back in {:?}", g.nodes.len(), t.elapsed());
    assert_eq!(version_root(&g).hex(), snap.manifest().root, "the read-back graph publishes the manifest root");
    for s in Section::SHIPPED {
        let hash = atlas_graph::sqlite::logical::logical_hash(&logical_dump_section(&g, s));
        let ms = snap.manifest().sections.iter().find(|m| m.name == s.name()).unwrap();
        assert_eq!(hash, ms.logical, "{s:?}: the read-back dump is the section's own");
    }
    assert_eq!(g.nodes.len(), 6263 + 32357 + 3972 + 50602 + 13548 + MAPS + CORPUS_ROOTS);
    assert_eq!(g.cross_refs.len(), 343558);
    assert_eq!(g.reading["bible"].order.len(), 31102);
    assert_eq!(g.occurs.len(), 431_280, "LEX-1: one Occurs row per aligned token");
    assert!(g.extra_tables.len() == 32, "{} extra tables re-attached", g.extra_tables.len());
    assert_eq!(snap.present().len(), 5);
}
