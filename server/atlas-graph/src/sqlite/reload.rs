//! The sections read back into the `Graph` that wrote them, for the tests and benches that want one whole
//! graph. Nothing on the served path calls this: `GraphService::from_sections` reads companions, not whole
//! tables.

use std::path::Path;

use atlas_graph_types::canon::ids::{parse_any_node_id, parse_position};
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
use std::collections::BTreeMap;

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
        RowOwned::Occurs(r) => push!(g.occurs, r, "occurs"),
        RowOwned::ParentOf(r) => push!(g.parent_of, r, "parent_of"),
        RowOwned::Spouses(r) => push!(g.spouses, r, "spouses"),
        RowOwned::Participates(r) => push!(g.participates, r, "participates"),
        RowOwned::Authored(r) => push!(g.authored, r, "authored"),
        RowOwned::Shown(r) => push!(g.shown, r, "shown"),
        RowOwned::MapSuccession(r) => push!(g.map_succession, r, "map_succession"),
        RowOwned::Brethren(r) => push!(g.brethren, r, "brethren"),
        RowOwned::CrossRefs(r) => push!(g.cross_refs, r, "cross_refs"),
        RowOwned::SpokenBy(r) => push!(g.spoken_by, r, "spoken_by"),
        RowOwned::SpokenAt(r) => push!(g.spoken_at, r, "spoken_at"),
        RowOwned::Quotes(r) => push!(g.quotes, r, "quotes"),
        RowOwned::Confesses(r) => push!(g.confesses, r, "confesses"),
        RowOwned::CommentsOn(r) => push!(g.comments_on, r, "comments_on"),
    }
    Ok(())
}

/// Reads every present section back into one `Graph`: a family's rows are gathered from every
/// section that homes it and pushed in their shared global `ord`, then the derived indexes are
/// rebuilt.
pub fn graph_from_sections(layout: &SectionLayout, manifest: &Manifest, present: &[Section]) -> Result<Graph, SqliteError> {
    let mut g = Graph::default();
    let mut rows_by_family: BTreeMap<RowFamily, Vec<(i64, RowOwned)>> = BTreeMap::new();
    for ms in &manifest.sections {
        let Some(section) = section_named(&ms.name) else {
            return Err(SqliteError(format!("manifest names an unknown section {}", ms.name)));
        };
        if !present.contains(&section) {
            continue;
        }
        let conn = open_read_only(&layout.cache_path(&ms.logical, ms.schema_version))?;
        {
            let mut stmt = conn.prepare("SELECT payload FROM node")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let payload: Vec<u8> = r.get(0)?;
                let node = Node::decode(&payload).map_err(|e| SqliteError(format!("{}: node payload: {e}", ms.name)))?;
                g.nodes.insert(node.id.clone(), node);
            }
        }
        {
            let mut stmt = conn.prepare("SELECT position, label FROM label")?;
            let mut rows = stmt.query([])?;
            while let Some(r) = rows.next()? {
                let position: String = r.get(0)?;
                let position = parse_position(&position, "label").map_err(|e| SqliteError(format!("{}: label {position}: {e}", ms.name)))?;
                g.labels.insert(position, r.get(1)?);
            }
        }
        for family in row_tables_of(section) {
            rows_by_family.entry(*family).or_default().extend(read_rows(&conn, *family)?);
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
    for (_, mut rows) in rows_by_family {
        rows.sort_by_key(|(ord, _)| *ord);
        for (ord, row) in rows {
            push_row(&mut g, ord, row)?;
        }
    }
    g.build_indexes();
    crate::event_world::add_justified_by(&mut g);
    g.edges_by_id = g.edge_records();
    Ok(g)
}

pub fn committed_graph(data_dir: &Path) -> anyhow::Result<(Graph, SqliteSnapshot)> {
    let layout = SectionLayout::under(data_dir);
    let snap = SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() })
        .map_err(|e| anyhow::anyhow!("opening the sections under {}: {e}", data_dir.display()))?;
    let g = graph_from_sections(&layout, snap.manifest(), snap.present()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((g, snap))
}
