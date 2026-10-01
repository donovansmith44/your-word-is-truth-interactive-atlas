//! Every node, row and index entry of the in-memory `Graph` is assigned to exactly one section.
//! Nothing here defaults: an index entry whose edge id names no row is an error, never a silent
//! Core.

use std::collections::BTreeMap;

use atlas_graph_types::canon::ids::{any_node_id_str, position_str};
use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::edge::{EdgeId, RelationId, SymRelationId};
use atlas_graph_types::explore::EdgeMeta;
use atlas_graph_types::graph::{EdgeRel, Graph};
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::Node;

use super::ddl::{has_spine, row_tables_of};
use super::rows::RowRef;
use super::SqliteError;
use crate::sections::{section_of_canon_succession, section_of_contains_bible, section_of_cross_ref, section_of_node, section_of_row, Section};

pub const DIR_FORWARD: i64 = 0;
pub const DIR_INVERSE: i64 = 1;
pub const DIR_SYMMETRIC: i64 = 2;
/// `edge_index.rel`: a symmetric relation's code is offset by 128.
pub const SYMMETRIC_REL_BASE: i64 = 128;

/// The ordinal is the relation's index in `RelationId::ALL`, so declaration order is the encoding.
pub fn directed_rel_code(r: RelationId) -> i64 {
    RelationId::ALL.iter().position(|x| *x == r).expect("ALL lists every RelationId") as i64
}
pub fn symmetric_rel_code(s: SymRelationId) -> i64 {
    SYMMETRIC_REL_BASE + SymRelationId::ALL.iter().position(|x| *x == s).expect("ALL lists every SymRelationId") as i64
}
pub fn rel_code_of(rel: EdgeRel) -> i64 {
    match rel {
        EdgeRel::Directed(r) => directed_rel_code(r),
        EdgeRel::Symmetric(s) => symmetric_rel_code(s),
    }
}
pub fn rel_of_code(code: i64) -> Option<EdgeRel> {
    if code >= SYMMETRIC_REL_BASE {
        SymRelationId::ALL.get(usize::try_from(code - SYMMETRIC_REL_BASE).ok()?).map(|s| EdgeRel::Symmetric(*s))
    } else {
        RelationId::ALL.get(usize::try_from(code).ok()?).map(|r| EdgeRel::Directed(*r))
    }
}

/// `NodeKind` ordinal: declaration order, written out so a new variant is
/// a compile error here rather than a silent renumbering.
pub fn node_kind_ordinal(k: NodeKind) -> i64 {
    match k {
        NodeKind::TextUnit => 0,
        NodeKind::Container => 1,
        NodeKind::Event => 2,
        NodeKind::Narrative => 3,
        NodeKind::Place => 4,
        NodeKind::Person => 5,
        NodeKind::Anchor => 6,
        NodeKind::Era => 7,
        NodeKind::Polity => 8,
        NodeKind::CatechismItem => 9,
        NodeKind::Source => 10,
        NodeKind::Translation => 11,
        NodeKind::PeopleGroup => 12,
        NodeKind::CommentaryItem => 13,
        NodeKind::LexiconEntry => 14,
        NodeKind::Map => 15,
    }
}

/// The inverse of `node_kind_ordinal`: `ALL` is declaration order, which is the ordinal.
pub fn node_kind_of_ordinal(code: i64) -> Option<NodeKind> {
    NodeKind::ALL.get(usize::try_from(code).ok()?).copied()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeEntryOut {
    pub subject: Position,
    pub rel: i64,
    /// 0 forward | 1 inverse | 2 symmetric.
    pub dir: i64,
    /// Index within the in-memory `(subject, rel, dir)` Vec -- THE cursor.
    pub ord: i64,
    pub object: Position,
    pub edge_id: EdgeId,
    pub meta: EdgeMeta,
    /// The row that produced the entry (a `justified-by` entry: its SOURCE row).
    pub row_family: RowFamily,
    /// The row's global ord (its index in its family's Vec).
    pub row_id: i64,
}

/// Everything one section file holds, borrowed from the Graph.
pub struct SectionPartition<'a> {
    pub section: Section,
    /// Sorted by `any_node_id_str` BYTE order -- the key SQLite's `ORDER BY id` (TEXT) uses, so the
    /// logical dump agrees on both sides. `AnyNodeId: Ord` is `(kind, raw)`, which is NOT that order.
    pub nodes: Vec<&'a Node>,
    /// Family by `row_tables_of(section)` order, then global ord ascending.
    pub rows: Vec<(RowFamily, i64, RowRef<'a>)>,
    /// In `(subject, rel, dir, ord)` order, subject by `position_str` bytes.
    pub edges: Vec<EdgeEntryOut>,
    /// `("bible", …)` for Kjv, `("concord", …)` for Concord.
    pub spine: Option<(&'static str, &'a [AnyNodeId])>,
}

/// The global ord is the row's index in its family Vec -- the same number for both halves of a
/// per-row family, so an ord is unique within its family across sections.
pub fn rows_of_section<'a>(g: &'a Graph, s: Section) -> Vec<(RowFamily, i64, RowRef<'a>)> {
    fn all<'a, T>(f: RowFamily, v: &'a [T], wrap: fn(&'a T) -> RowRef<'a>) -> Vec<(RowFamily, i64, RowRef<'a>)> {
        v.iter().enumerate().map(|(i, r)| (f, i as i64, wrap(r))).collect()
    }
    fn split<'a, T>(f: RowFamily, v: &'a [T], s: Section, section_of: fn(&T) -> Section, wrap: fn(&'a T) -> RowRef<'a>) -> Vec<(RowFamily, i64, RowRef<'a>)> {
        v.iter().enumerate().filter(|(_, r)| section_of(r) == s).map(|(i, r)| (f, i as i64, wrap(r))).collect()
    }
    let mut out = Vec::new();
    for f in row_tables_of(s) {
        let f = *f;
        out.extend(match f {
            RowFamily::ContainsBible => split(f, &g.contains_bible, s, section_of_contains_bible, RowRef::ContainsBible),
            RowFamily::ContainsConcord => all(f, &g.contains_concord, RowRef::ContainsConcord),
            RowFamily::Attests => all(f, &g.attests, RowRef::Attests),
            RowFamily::Succession => all(f, &g.succession, RowRef::Succession),
            RowFamily::CanonSuccession => split(f, &g.canon_succession, s, section_of_canon_succession, RowRef::CanonSuccession),
            RowFamily::DatedBy => all(f, &g.dated_by, RowRef::DatedBy),
            RowFamily::LocatedAt => all(f, &g.located_at, RowRef::LocatedAt),
            RowFamily::Fulfills => all(f, &g.fulfills, RowRef::Fulfills),
            RowFamily::Typology => all(f, &g.typology, RowRef::Typology),
            RowFamily::NamedAfter => all(f, &g.named_after, RowRef::NamedAfter),
            RowFamily::Catechism => all(f, &g.catechism, RowRef::Catechism),
            RowFamily::Mentions => all(f, &g.mentions, RowRef::Mentions),
            RowFamily::CorrespondsBible => all(f, &g.corresponds_bible, RowRef::CorrespondsBible),
            RowFamily::TemporalAdjacency => all(f, &g.temporal_adjacency, RowRef::TemporalAdjacency),
            RowFamily::Analogue => all(f, &g.analogue, RowRef::Analogue),
            RowFamily::Occurs => all(f, &g.occurs, RowRef::Occurs),
            RowFamily::ParentOf => all(f, &g.parent_of, RowRef::ParentOf),
            RowFamily::Spouses => all(f, &g.spouses, RowRef::Spouses),
            RowFamily::Participates => all(f, &g.participates, RowRef::Participates),
            RowFamily::Authored => all(f, &g.authored, RowRef::Authored),
            RowFamily::Shown => all(f, &g.shown, RowRef::Shown),
            RowFamily::MapSuccession => all(f, &g.map_succession, RowRef::MapSuccession),
            RowFamily::Brethren => all(f, &g.brethren, RowRef::Brethren),
            RowFamily::CrossRefs => split(f, &g.cross_refs, s, section_of_cross_ref, RowRef::CrossRefs),
            RowFamily::SpokenBy => all(f, &g.spoken_by, RowRef::SpokenBy),
            RowFamily::SpokenAt => all(f, &g.spoken_at, RowRef::SpokenAt),
            RowFamily::Quotes => all(f, &g.quotes, RowRef::Quotes),
            RowFamily::Confesses => all(f, &g.confesses, RowRef::Confesses),
            RowFamily::CommentsOn => all(f, &g.comments_on, RowRef::CommentsOn),
        });
    }
    out
}

/// Edge id -> EVERY row minting it, in `row_edges` order. Two rows CAN mint one id (two tokens of
/// one lexicon entry in one verse share `(rel, subject, object)`) and each has its own index entry,
/// so the k-th index entry under an id belongs to the k-th row.
pub fn edge_row_map(g: &Graph) -> BTreeMap<EdgeId, Vec<(RowFamily, i64)>> {
    let mut map: BTreeMap<EdgeId, Vec<(RowFamily, i64)>> = BTreeMap::new();
    for e in g.row_edges() {
        map.entry(Graph::edge_id_of(&e)).or_default().push((e.family, e.row_ord as i64));
    }
    map
}

/// One partition per `Section::SHIPPED` entry; a manifest lists shipped sections only.
pub fn partition(g: &Graph) -> Result<Vec<SectionPartition<'_>>, SqliteError> {
    let sections: Vec<Section> = Section::SHIPPED.to_vec();
    let slot = |s: Section| sections.iter().position(|x| *x == s).expect("every shipped section has a slot");

    let mut nodes: Vec<Vec<&Node>> = vec![Vec::new(); sections.len()];
    for n in g.nodes.values() {
        let section = section_of_node(n);
        // A node routed to a section that is not shipped is an error, never a silent drop or a
        // default into core. Today every section in `MANIFEST_ORDER` ships, so this cannot fire.
        let Some(i) = sections.iter().position(|x| *x == section) else {
            return Err(SqliteError(format!(
                "node {} routes to the {:?} section, which is not a shipped section",
                any_node_id_str(&n.id),
                section
            )));
        };
        nodes[i].push(n);
    }
    for v in &mut nodes {
        v.sort_by_cached_key(|n| any_node_id_str(&n.id));
    }

    let map = edge_row_map(g);
    let rows_of = |eid: &EdgeId| -> Result<&Vec<(RowFamily, i64)>, SqliteError> {
        map.get(eid).ok_or_else(|| SqliteError(format!("index entry {} names no row (edge_row_map)", eid.0)))
    };
    let mut ords: BTreeMap<(String, i64, i64), i64> = BTreeMap::new();
    let mut edges: Vec<Vec<EdgeEntryOut>> = vec![Vec::new(); sections.len()];
    let mut place = |subject: &Position, rel: i64, dir: i64, object: &Position, eid: &EdgeId, meta: &EdgeMeta, (row_family, row_id): (RowFamily, i64)| {
        let ord = ords.entry((position_str(subject), rel, dir)).or_insert(0);
        edges[slot(section_of_row(g, row_family, row_id as usize))].push(EdgeEntryOut {
            subject: subject.clone(),
            rel,
            dir,
            ord: *ord,
            object: object.clone(),
            edge_id: eid.clone(),
            meta: meta.clone(),
            row_family,
            row_id,
        });
        *ord += 1;
    };
    for e in g.row_edges() {
        let eid = Graph::edge_id_of(&e);
        let row = (e.family, e.row_ord as i64);
        match e.rel {
            EdgeRel::Directed(r) => {
                let code = directed_rel_code(r);
                place(&e.subject, code, DIR_FORWARD, &e.object, &eid, &e.meta, row);
                place(&e.object, code, DIR_INVERSE, &e.subject, &eid, &e.meta, row);
            }
            EdgeRel::Symmetric(s) => {
                let code = symmetric_rel_code(s);
                place(&e.subject, code, DIR_SYMMETRIC, &e.object, &eid, &e.meta, row);
                place(&e.object, code, DIR_SYMMETRIC, &e.subject, &eid, &e.meta, row);
            }
        }
    }
    if let Some(ix) = g.indexes.get(&RelationId::JustifiedBy) {
        let code = directed_rel_code(RelationId::JustifiedBy);
        for (dir, held) in [(DIR_FORWARD, &ix.fwd), (DIR_INVERSE, &ix.inv)] {
            for (subject, frontier) in held {
                for entry in frontier.edges() {
                    let source = if dir == DIR_FORWARD { subject } else { &entry.node };
                    let row = match source {
                        Position::Edge(source) => rows_of(source)?[0],
                        Position::Node(n) => {
                            return Err(SqliteError(format!(
                                "justified-by entry (dir {dir}) whose source end is a node {}",
                                any_node_id_str(n)
                            )))
                        }
                    };
                    place(subject, code, dir, &entry.node, &entry.edge, &entry.meta, row);
                }
            }
        }
    }
    for v in &mut edges {
        v.sort_by_cached_key(|e| (position_str(&e.subject), e.rel, e.dir, e.ord));
    }

    let mut out = Vec::with_capacity(sections.len());
    for (i, s) in sections.iter().enumerate() {
        let spine = if has_spine(*s) {
            let corpus: &'static str = match s {
                Section::Kjv => "bible",
                Section::Concord => "concord",
                _ => unreachable!("has_spine is Kjv | Concord"),
            };
            g.reading.get(corpus).map(|sp| (corpus, sp.order.as_slice()))
        } else {
            None
        };
        out.push(SectionPartition {
            section: *s,
            nodes: std::mem::take(&mut nodes[i]),
            rows: rows_of_section(g, *s),
            edges: std::mem::take(&mut edges[i]),
            spine,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_reads_back_from_the_ordinal_it_is_written_as() {
        // Arrange
        let every_kind = NodeKind::ALL;
        // Act
        let back: Vec<Option<NodeKind>> = every_kind.iter().map(|kind| node_kind_of_ordinal(node_kind_ordinal(*kind))).collect();
        // Assert
        assert_eq!(back, every_kind.map(Some).to_vec());
        assert_eq!(node_kind_of_ordinal(every_kind.len() as i64), None);
    }
}
