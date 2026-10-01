mod common;

use common::{CONCORD_CITATIONS, CORPUS_ROOTS, MAPS};

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
    assert_eq!(g.cross_refs.len(), 343558 + CONCORD_CITATIONS);
    assert_eq!(g.reading["bible"].order.len(), 31102);
    assert_eq!(g.occurs.len(), 431_280, "LEX-1: one Occurs row per aligned token");
    assert!(g.extra_tables.len() == 35, "{} extra tables re-attached", g.extra_tables.len());
    assert_eq!(snap.present().len(), 5);
}

#[test]
fn every_row_familys_adjacency_is_read_by_the_edge_with_its_rows_behind_it() {
    // Arrange
    use atlas_graph_types::canon::RowFamily;
    use atlas_graph_types::edge::{Direction, EdgeKind};
    use atlas_graph_types::adjacency::{EdgeQuery, Adjacency};
    use atlas_graph_types::graph::EdgeRel;
    use atlas_graph_types::id::Position;
    use atlas_graph_types::store::GraphQuery;
    let (g, snap) = committed_graph(&common::compiled_dir()).unwrap();
    let drain = |q: &dyn GraphQuery, p: &Position, kind: EdgeKind, limit: usize| {
        let mut cursor = None;
        let mut edges = Vec::new();
        loop {
            let page = q.edges(p, &EdgeQuery { kind, cursor, limit });
            edges.extend(page.entries.into_iter().map(|e| e.edge));
            match page.next {
                Some(next) => cursor = Some(next),
                None => return edges,
            }
        }
    };
    let widest = |held: &std::collections::BTreeMap<Position, Adjacency>| held.iter().max_by_key(|(_, f)| f.edge_count()).map(|(p, f)| (f.edge_count(), p.clone()));

    // Act
    let walked: Vec<(RowFamily, Option<(bool, bool, bool, bool, bool)>)> = RowFamily::ALL
        .iter()
        .map(|family| {
            let widest_of_family = match family.relation() {
                EdgeRel::Directed(rel) => g.indexes.get(&rel).and_then(|ix| {
                    [(Direction::Forward, widest(&ix.fwd)), (Direction::Inverse, widest(&ix.inv))]
                        .into_iter()
                        .filter_map(|(dir, found)| found.map(|(count, p)| (count, EdgeKind::Directed(rel, dir), p)))
                        .max_by_key(|(count, _, _)| *count)
                }),
                EdgeRel::Symmetric(rel) => g.symmetric_indexes.get(&rel).and_then(|ix| widest(&ix.fwd)).map(|(count, p)| (count, EdgeKind::Symmetric(rel), p)),
            };
            let Some((_, kind, position)) = widest_of_family else { return (*family, None) };
            let one_at_a_time = drain(&g, &position, kind, 1);
            let paged = drain(&g, &position, kind, usize::MAX);
            let served = drain(&snap, &position, kind, usize::MAX);
            let distinct: std::collections::BTreeSet<_> = paged.iter().collect();
            let rows_behind: usize = paged.iter().map(|edge| g.rows_behind(edge).len()).sum();
            (
                *family,
                Some((
                    g.edge_summary(&position)[&kind] == paged.len(),
                    one_at_a_time == paged,
                    served == paged,
                    distinct.len() == paged.len(),
                    rows_behind > paged.len(),
                )),
            )
        })
        .collect();

    // Assert
    let read_by_the_edge: Vec<(RowFamily, Option<(bool, bool, bool, bool)>)> = walked.iter().map(|(f, w)| (*f, w.map(|(summary, one, served, distinct, _)| (summary, one, served, distinct)))).collect();
    let families_with_rows: Vec<(RowFamily, Option<(bool, bool, bool, bool)>)> = walked.iter().map(|(f, w)| (*f, w.map(|_| (true, true, true, true)))).collect();
    let some_family_keeps_several_rows_behind_one_edge = walked.iter().any(|(_, w)| w.is_some_and(|w| w.4));
    assert_eq!((read_by_the_edge, some_family_keeps_several_rows_behind_one_edge), (families_with_rows, true));
}
