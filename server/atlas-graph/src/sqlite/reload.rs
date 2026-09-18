//! DB-5: the sections read back into the `Graph` that wrote them -- the
//! inverse of `writer` and the one "scan port" DB-1 OQ-8 deferred "until a
//! second consumer wants one". The consumers are the real-data tests and
//! the benches, which used to load `graph.bin`; nothing on the served
//! path calls this (`GraphService::from_sections` reads companions, not
//! whole tables). It is also the writer's round-trip law: a graph read
//! back publishes the manifest's root and every section's own logical
//! hash (`tests/reload_real_data.rs`).
//!
//! Each section's cached `.sqlite` is opened on its own connection, so the
//! per-family readers (`rows::read_rows`, `extras::read_table`) run
//! unqualified, exactly as the logical dump does.

use std::path::Path;

use atlas_graph_types::canon::ids::parse_any_node_id;
use atlas_graph_types::canon::Canon;
use atlas_graph_types::graph::{Graph, ReadingSpine};
use atlas_graph_types::node::Node;

use super::extras::{read_table, row_body, table_specs_of};
use super::manifest::Manifest;
use super::rows::{read_rows, RowOwned};
use super::snapshot::SqliteSnapshot;
use super::source::{CommittedZstdSource, SectionLayout};
use super::{open_read_only, SqliteError};
use crate::sections::{row_tables_of, spine_corpus, Section};
use atlas_graph_types::canon::RowFamily;

fn section_named(name: &str) -> Option<Section> {
    Section::MANIFEST_ORDER.iter().copied().find(|s| s.name() == name)
}

fn push_row(g: &mut Graph, ord: i64, row: RowOwned) -> Result<(), SqliteError> {
    macro_rules! push {
        ($vec:expr, $r:expr, $name:expr) => {{
            if $vec.len() != ord as usize {
                return Err(SqliteError(format!("{}: ord {ord} arrives at index {} (a gap in the table)", $name, $vec.len())));
            }
            $vec.push($r);
        }};
    }
    match row {
        RowOwned::ContainsBible(r) => push!(g.contains_bible, r, "contains_bible"),
        RowOwned::ContainsConcord(r) => push!(g.contains_concord, r, "contains_concord"),
        RowOwned::Attests(r) => push!(g.attests, r, "attests"),
        RowOwned::Succession(r) => push!(g.succession, r, "succession"),
        RowOwned::CanonSuccession(r) => push!(g.canon_succession, r, "canon_succession"),
        RowOwned::DatedBy(r) => push!(g.dated_by, r, "dated_by"),
        RowOwned::LocatedAt(r) => push!(g.located_at, r, "located_at"),
        RowOwned::Fulfills(r) => push!(g.fulfills, r, "fulfills"),
        RowOwned::Typology(r) => push!(g.typology, r, "typology"),
        RowOwned::NamedAfter(r) => push!(g.named_after, r, "named_after"),
        RowOwned::Catechism(r) => push!(g.catechism, r, "catechism"),
        RowOwned::Mentions(r) => push!(g.mentions, r, "mentions"),
        RowOwned::CorrespondsBible(r) => push!(g.corresponds_bible, r, "corresponds_bible"),
        RowOwned::TemporalAdjacency(r) => push!(g.temporal_adjacency, r, "temporal_adjacency"),
        RowOwned::Analogue(r) => push!(g.analogue, r, "analogue"),
        RowOwned::CrossRefs(r) => push!(g.cross_refs, r, "cross_refs"),
        RowOwned::SpokenBy(r) => push!(g.spoken_by, r, "spoken_by"),
        RowOwned::SpokenAt(r) => push!(g.spoken_at, r, "spoken_at"),
        RowOwned::Quotes(r) => push!(g.quotes, r, "quotes"),
        RowOwned::Confesses(r) => push!(g.confesses, r, "confesses"),
        RowOwned::CommentsOn(r) => push!(g.comments_on, r, "comments_on"),
    }
    Ok(())
}

/// Reads every present section's cached file back into one `Graph`:
/// nodes, every row family in `ord` order (`contains_bible`'s two homes
/// merged by their shared global ord), both spines, the extra tables'
/// canonical bodies; then `build_indexes` + `add_justified_by`, the shape
/// `from_artifact` used to return.
pub fn graph_from_sections(layout: &SectionLayout, manifest: &Manifest, present: &[Section]) -> Result<Graph, SqliteError> {
    let mut g = Graph::default();
    let mut contains_bible: Vec<(i64, RowOwned)> = Vec::new();
    for ms in &manifest.sections {
        let Some(section) = section_named(&ms.name) else {
            return Err(SqliteError(format!("manifest names an unknown section {}", ms.name)));
        };
        if !present.contains(&section) {
            continue;
        }
        let conn = open_read_only(&layout.cache_path(&ms.logical))?;
        {
            let mut stmt = conn.prepare("SELECT payload FROM node")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let payload: Vec<u8> = r.get(0)?;
                let node = Node::decode(&payload).map_err(|e| SqliteError(format!("{}: node payload: {e}", ms.name)))?;
                g.nodes.insert(node.id.clone(), node);
            }
        }
        for family in row_tables_of(section) {
            let rows = read_rows(&conn, *family)?;
            if *family == RowFamily::ContainsBible {
                contains_bible.extend(rows);
            } else {
                for (ord, row) in rows {
                    push_row(&mut g, ord, row)?;
                }
            }
        }
        if let Some(corpus) = spine_corpus(section) {
            let mut stmt = conn.prepare("SELECT node_id FROM reading_spine ORDER BY ord")?;
            let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let mut order = Vec::new();
            for id in ids {
                let id = id?;
                order.push(parse_any_node_id(&id, "reading_spine").map_err(|e| SqliteError(format!("reading_spine {id}: {e}")))?);
            }
            g.reading.insert(corpus, ReadingSpine { order });
        }
        for spec in table_specs_of(section) {
            let mut bodies = Vec::new();
            for row in read_table(&conn, spec)? {
                bodies.push(row_body(spec, &row)?);
            }
            g.extra_tables.insert(spec.name, bodies);
        }
    }
    contains_bible.sort_by_key(|(ord, _)| *ord);
    for (ord, row) in contains_bible {
        push_row(&mut g, ord, row)?;
    }
    g.build_indexes();
    crate::event_world::add_justified_by(&mut g);
    Ok(g)
}

/// The committed sections under `data_dir`, opened (one worker; the cache
/// populated) and read back. What the real-data tests call.
pub fn committed_graph(data_dir: &Path) -> anyhow::Result<(Graph, SqliteSnapshot)> {
    let layout = SectionLayout::under(data_dir);
    let snap = SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() })
        .map_err(|e| anyhow::anyhow!("opening the sections under {}: {e}", data_dir.display()))?;
    let g = graph_from_sections(&layout, snap.manifest(), snap.present()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((g, snap))
}
