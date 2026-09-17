//! DB-2b: the section partition (spec §2.1). Every node, every row and
//! every index entry of the in-memory `Graph` is assigned to exactly one
//! section, through DB-2a's `sections.rs` rules: nodes by
//! `section_of_node`, rows by `row_tables_of` (+ `section_of_contains_bible`
//! for the one family split by row), index entries by the ROW that
//! produced them (`Graph::row_edges` names it; `edge_row_map` indexes it
//! by edge id) -- a `justified-by` entry by its SOURCE row.
//!
//! Nothing here is a default: an index entry whose edge id names no row
//! is an error, never a silent Core.

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
use crate::sections::{section_of_contains_bible, section_of_justified_by, section_of_node, Section};

/// Spec §5.1 `edge_index.dir`.
pub const DIR_FORWARD: i64 = 0;
pub const DIR_INVERSE: i64 = 1;
pub const DIR_SYMMETRIC: i64 = 2;
/// Spec §5.1 `edge_index.rel`: symmetric relations are offset by 128.
pub const SYMMETRIC_REL_BASE: i64 = 128;

/// `RelationId` ordinal = its index in `RelationId::ALL` (declaration order).
pub fn directed_rel_code(r: RelationId) -> i64 {
    RelationId::ALL.iter().position(|x| *x == r).expect("ALL lists every RelationId") as i64
}
/// `128 + SymRelationId` ordinal.
pub fn symmetric_rel_code(s: SymRelationId) -> i64 {
    SYMMETRIC_REL_BASE + SymRelationId::ALL.iter().position(|x| *x == s).expect("ALL lists every SymRelationId") as i64
}
pub fn rel_code_of(rel: EdgeRel) -> i64 {
    match rel {
        EdgeRel::Directed(r) => directed_rel_code(r),
        EdgeRel::Symmetric(s) => symmetric_rel_code(s),
    }
}
/// The inverse of `directed_rel_code` / `symmetric_rel_code`.
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
    }
}
pub fn node_kind_of_ordinal(o: i64) -> Option<NodeKind> {
    const ALL: [NodeKind; 14] = [
        NodeKind::TextUnit,
        NodeKind::Container,
        NodeKind::Event,
        NodeKind::Narrative,
        NodeKind::Place,
        NodeKind::Person,
        NodeKind::Anchor,
        NodeKind::Era,
        NodeKind::Polity,
        NodeKind::CatechismItem,
        NodeKind::Source,
        NodeKind::Translation,
        NodeKind::PeopleGroup,
        NodeKind::CommentaryItem,
    ];
    ALL.get(usize::try_from(o).ok()?).copied()
}

/// One `edge_index` row before it is written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EdgeEntryOut {
    pub subject: Position,
    /// `RelationId` ordinal, or `128 + SymRelationId` ordinal.
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
    /// Sorted by `any_node_id_str` BYTE order -- the same key SQLite's
    /// `ORDER BY id` (TEXT) uses, so the logical dump agrees on both
    /// sides. (`AnyNodeId: Ord` is `(kind, raw)`, which is NOT that order.)
    pub nodes: Vec<&'a Node>,
    /// Family by `row_tables_of(section)` order, then global ord ascending.
    pub rows: Vec<(RowFamily, i64, RowRef<'a>)>,
    /// In `(subject, rel, dir, ord)` order, subject by `position_str` bytes.
    pub edges: Vec<EdgeEntryOut>,
    /// `("bible", …)` for Kjv, `("concord", …)` for Concord.
    pub spine: Option<(&'static str, &'a [AnyNodeId])>,
}

/// The section's rows: every family `row_tables_of(section)` names, in
/// that order; `ContainsBible` filtered by `section_of_contains_bible`.
/// The global ord is the row's index in its family Vec (`row_ord` of
/// `Graph::row_edges`) -- the same number for the core and kjv halves of
/// `contains_bible`, so an ord is unique within its family across sections.
pub fn rows_of_section<'a>(g: &'a Graph, s: Section) -> Vec<(RowFamily, i64, RowRef<'a>)> {
    fn all<'a, T>(f: RowFamily, v: &'a [T], wrap: fn(&'a T) -> RowRef<'a>) -> Vec<(RowFamily, i64, RowRef<'a>)> {
        v.iter().enumerate().map(|(i, r)| (f, i as i64, wrap(r))).collect()
    }
    let mut out = Vec::new();
    for f in row_tables_of(s) {
        let f = *f;
        out.extend(match f {
            RowFamily::ContainsBible => g
                .contains_bible
                .iter()
                .enumerate()
                .filter(|(_, r)| section_of_contains_bible(r) == s)
                .map(|(i, r)| (f, i as i64, RowRef::ContainsBible(r)))
                .collect(),
            RowFamily::ContainsConcord => all(f, &g.contains_concord, RowRef::ContainsConcord),
            RowFamily::Attests => all(f, &g.attests, RowRef::Attests),
            RowFamily::Succession => all(f, &g.succession, RowRef::Succession),
            RowFamily::CanonSuccession => all(f, &g.canon_succession, RowRef::CanonSuccession),
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
            RowFamily::CrossRefs => all(f, &g.cross_refs, RowRef::CrossRefs),
            RowFamily::SpokenBy => all(f, &g.spoken_by, RowRef::SpokenBy),
            RowFamily::SpokenAt => all(f, &g.spoken_at, RowRef::SpokenAt),
            RowFamily::Quotes => all(f, &g.quotes, RowRef::Quotes),
            RowFamily::Confesses => all(f, &g.confesses, RowRef::Confesses),
            RowFamily::CommentsOn => all(f, &g.comments_on, RowRef::CommentsOn),
        });
    }
    out
}

/// Edge id -> (family, global ord, container raw for `ContainsBible`
/// rows): the map every index entry resolves its row through. Two rows
/// minting the same id (identical `(rel, subject, object)`) keep the
/// first -- they are indistinguishable to the index anyway.
pub fn edge_row_map(g: &Graph) -> BTreeMap<EdgeId, (RowFamily, i64, Option<String>)> {
    let mut map = BTreeMap::new();
    for e in g.row_edges() {
        let id = Graph::edge_id_of(&e);
        let container = match e.family {
            RowFamily::ContainsBible => Some(g.contains_bible[e.row_ord].container.0.clone()),
            _ => None,
        };
        map.entry(id).or_insert((e.family, e.row_ord as i64, container));
    }
    map
}

/// One partition per `Section::MANIFEST_ORDER` entry except `Lexicon`
/// (no tables until LEX-1; a manifest lists shipped sections only).
pub fn partition(g: &Graph) -> Result<Vec<SectionPartition<'_>>, SqliteError> {
    let sections: Vec<Section> = Section::MANIFEST_ORDER.iter().copied().filter(|s| *s != Section::Lexicon).collect();
    let slot = |s: Section| sections.iter().position(|x| *x == s).expect("every non-Lexicon section has a slot");

    let mut nodes: Vec<Vec<&Node>> = vec![Vec::new(); sections.len()];
    for n in g.nodes.values() {
        nodes[slot(section_of_node(n))].push(n);
    }
    for v in &mut nodes {
        v.sort_by_cached_key(|n| any_node_id_str(&n.id));
    }

    let map = edge_row_map(g);
    let row_of = |eid: &EdgeId| -> Result<&(RowFamily, i64, Option<String>), SqliteError> {
        map.get(eid).ok_or_else(|| SqliteError(format!("index entry {} names no row (edge_row_map)", eid.0)))
    };
    let mut edges: Vec<Vec<EdgeEntryOut>> = vec![Vec::new(); sections.len()];
    let mut place = |subject: &Position, rel: i64, dir: i64, ord: usize, object: &Position, eid: &EdgeId, meta: &EdgeMeta, justified: bool| -> Result<(), SqliteError> {
        let (fam, row_id, container) = if justified {
            // A justified-by entry runs edge -> ground node; the forward
            // reading has the SOURCE edge as subject, the inverse reading
            // has it as object. Either way the row is the source row.
            let source = if dir == DIR_FORWARD { subject } else { object };
            match source {
                Position::Edge(source) => row_of(source)?,
                Position::Node(n) => {
                    return Err(SqliteError(format!(
                        "justified-by entry (dir {dir}) whose source end is a node {}",
                        any_node_id_str(n)
                    )))
                }
            }
        } else {
            row_of(eid)?
        };
        let section = section_of_justified_by(*fam, container.as_deref());
        edges[slot(section)].push(EdgeEntryOut {
            subject: subject.clone(),
            rel,
            dir,
            ord: ord as i64,
            object: object.clone(),
            edge_id: eid.clone(),
            meta: meta.clone(),
            row_family: *fam,
            row_id: *row_id,
        });
        Ok(())
    };
    for (rel, ix) in &g.indexes {
        let code = directed_rel_code(*rel);
        let justified = *rel == RelationId::JustifiedBy;
        for (subject, entries) in &ix.fwd {
            for (i, (eid, object, meta)) in entries.iter().enumerate() {
                place(subject, code, DIR_FORWARD, i, object, eid, meta, justified)?;
            }
        }
        for (subject, entries) in &ix.inv {
            for (i, (eid, object, meta)) in entries.iter().enumerate() {
                place(subject, code, DIR_INVERSE, i, object, eid, meta, justified)?;
            }
        }
    }
    for (rel, ix) in &g.symmetric_indexes {
        let code = symmetric_rel_code(*rel);
        for (subject, entries) in &ix.fwd {
            for (i, (eid, object, meta)) in entries.iter().enumerate() {
                place(subject, code, DIR_SYMMETRIC, i, object, eid, meta, false)?;
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
