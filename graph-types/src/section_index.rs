use std::collections::{BTreeMap, HashSet};

use crate::adjacency::EdgeMeta;
use crate::canon::ids::{any_node_id_str, position_str};
use crate::canon::{str_value, RowFamily, Value};
use crate::edge::{EdgeId, RelationId, SymRelationId};
use crate::graph::{EdgeRel, Graph};
use crate::id::Position;
use crate::sections::{extra_line_body, section_of_node, section_of_row, Section};

pub const DIR_FORWARD: i64 = 0;
pub const DIR_INVERSE: i64 = 1;
pub const DIR_SYMMETRIC: i64 = 2;
pub const SYMMETRIC_REL_BASE: i64 = 128;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexEntry {
    pub subject: Position,
    pub rel: i64,
    pub dir: i64,
    pub ord: i64,
    pub object: Position,
    pub edge_id: EdgeId,
    pub meta: EdgeMeta,
    pub row_family: RowFamily,
    pub row_id: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexCount {
    pub subject: Position,
    pub rel: i64,
    pub dir: i64,
    pub count: i64,
}

pub struct SectionIndex<'a> {
    pub section: Section,
    pub entries: Vec<IndexEntry>,
    pub counts: Vec<IndexCount>,
    pub labels: Vec<(Position, &'a str)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IndexError(pub String);

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for IndexError {}

pub struct DerivedTable {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    pub key: &'static [&'static str],
}

pub const LABEL_COLUMNS: [&str; 2] = ["position", "label"];
pub const EDGE_INDEX_COLUMNS: [&str; 12] =
    ["subject", "rel", "dir", "ord", "object", "edge_id", "meta_kind", "meta_narrative", "meta_votes", "meta_parentage", "row_family", "row_id"];
pub const EDGE_COUNT_COLUMNS: [&str; 4] = ["subject", "rel", "dir", "count"];

pub const DERIVED_TABLES: [DerivedTable; 3] = [
    DerivedTable { name: "label", columns: &LABEL_COLUMNS, key: &["position"] },
    DerivedTable { name: "edge_index", columns: &EDGE_INDEX_COLUMNS, key: &["subject", "rel", "dir", "ord"] },
    DerivedTable { name: "edge_count", columns: &EDGE_COUNT_COLUMNS, key: &["subject", "rel", "dir"] },
];

pub fn derived_table_named(name: &str) -> Option<&'static DerivedTable> {
    DERIVED_TABLES.iter().find(|t| t.name == name)
}

pub fn edge_meta_values(meta: &EdgeMeta) -> [Value; 4] {
    match meta {
        EdgeMeta::None => [Value::Int(0), Value::Null, Value::Null, Value::Null],
        EdgeMeta::Narrative(n) => [Value::Int(1), str_value(&n.0), Value::Null, Value::Null],
        EdgeMeta::Votes(v) => [Value::Int(2), Value::Null, Value::Int(i64::from(*v)), Value::Null],
        EdgeMeta::Parentage(p) => [Value::Int(3), Value::Null, Value::Null, str_value(p.name())],
    }
}

pub fn edge_id_hex(id: &EdgeId) -> Result<&str, IndexError> {
    id.0.split_once(':').map(|(_, hex)| hex).ok_or_else(|| IndexError(format!("edge id {} has no ':'", id.0)))
}

pub fn label_values(position: &Position, label: &str) -> [Value; 2] {
    [str_value(&position_str(position)), str_value(label)]
}

pub fn edge_index_values(e: &IndexEntry) -> Result<[Value; 12], IndexError> {
    let [meta_kind, narrative, votes, parentage] = edge_meta_values(&e.meta);
    Ok([
        str_value(&position_str(&e.subject)),
        Value::Int(e.rel),
        Value::Int(e.dir),
        Value::Int(e.ord),
        str_value(&position_str(&e.object)),
        str_value(edge_id_hex(&e.edge_id)?),
        meta_kind,
        narrative,
        votes,
        parentage,
        Value::Int(i64::from(e.row_family.ordinal())),
        Value::Int(e.row_id),
    ])
}

pub fn edge_count_values(c: &IndexCount) -> [Value; 4] {
    [str_value(&position_str(&c.subject)), Value::Int(c.rel), Value::Int(c.dir), Value::Int(c.count)]
}

pub fn derived_line_body(columns: &[&'static str], values: impl IntoIterator<Item = Value>) -> Vec<u8> {
    extra_line_body(columns.iter().copied().zip(values).collect())
}

pub fn derived_lines(index: &SectionIndex<'_>, mut line: impl FnMut(&str, &[u8])) -> Result<(), IndexError> {
    for (position, label) in &index.labels {
        line("label", &derived_line_body(&LABEL_COLUMNS, label_values(position, label)));
    }
    for e in &index.entries {
        line("edge_index", &derived_line_body(&EDGE_INDEX_COLUMNS, edge_index_values(e)?));
    }
    for c in &index.counts {
        line("edge_count", &derived_line_body(&EDGE_COUNT_COLUMNS, edge_count_values(c)));
    }
    Ok(())
}

pub fn edge_row_map(g: &Graph) -> BTreeMap<EdgeId, Vec<(RowFamily, i64)>> {
    let mut map: BTreeMap<EdgeId, Vec<(RowFamily, i64)>> = BTreeMap::new();
    for e in g.row_edges() {
        map.entry(Graph::edge_id_of(&e)).or_default().push((e.family, e.row_ord as i64));
    }
    map
}

pub fn index_sections(g: &Graph) -> Result<Vec<SectionIndex<'_>>, IndexError> {
    let sections: Vec<Section> = Section::SHIPPED.to_vec();
    let slot = |s: Section| sections.iter().position(|x| *x == s).expect("every shipped section has a slot");
    let map = edge_row_map(g);
    let rows_of = |eid: &EdgeId| -> Result<&Vec<(RowFamily, i64)>, IndexError> {
        map.get(eid).ok_or_else(|| IndexError(format!("index entry {} names no row (edge_row_map)", eid.0)))
    };
    let mut ords: BTreeMap<(String, i64, i64), i64> = BTreeMap::new();
    let mut entries: Vec<Vec<IndexEntry>> = vec![Vec::new(); sections.len()];
    let mut place = |subject: &Position, rel: i64, dir: i64, object: &Position, eid: &EdgeId, meta: &EdgeMeta, (row_family, row_id): (RowFamily, i64)| {
        let ord = ords.entry((position_str(subject), rel, dir)).or_insert(0);
        entries[slot(section_of_row(g, row_family, row_id as usize))].push(IndexEntry {
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
            for (subject, adjacency) in held {
                for entry in adjacency.edges() {
                    let source = if dir == DIR_FORWARD { subject } else { &entry.node };
                    let row = match source {
                        Position::Edge(source) => rows_of(source)?[0],
                        Position::Node(n) => {
                            return Err(IndexError(format!("justified-by entry (dir {dir}) whose source end is a node {}", any_node_id_str(n))))
                        }
                    };
                    place(subject, code, dir, &entry.node, &entry.edge, &entry.meta, row);
                }
            }
        }
    }
    for v in &mut entries {
        v.sort_by_cached_key(|e| (position_str(&e.subject), e.rel, e.dir, e.ord));
    }
    let mut labels = labels_of_sections(g, &sections, &entries);
    let mut counts = counts_of_sections(&entries);
    Ok(sections
        .iter()
        .enumerate()
        .map(|(i, s)| SectionIndex {
            section: *s,
            entries: std::mem::take(&mut entries[i]),
            counts: std::mem::take(&mut counts[i]),
            labels: std::mem::take(&mut labels[i]),
        })
        .collect())
}

pub fn counts_of_sections(entries: &[Vec<IndexEntry>]) -> Vec<Vec<IndexCount>> {
    let mut counted: HashSet<(&Position, i64, i64, &EdgeId)> = HashSet::new();
    let mut out = Vec::with_capacity(entries.len());
    for held in entries {
        let mut counts: Vec<IndexCount> = Vec::new();
        for e in held {
            if !counted.insert((&e.subject, e.rel, e.dir, &e.edge_id)) {
                continue;
            }
            match counts.last_mut() {
                Some(c) if c.subject == e.subject && c.rel == e.rel && c.dir == e.dir => c.count += 1,
                _ => counts.push(IndexCount { subject: e.subject.clone(), rel: e.rel, dir: e.dir, count: 1 }),
            }
        }
        out.push(counts);
    }
    out
}

fn labels_of_sections<'a>(g: &'a Graph, sections: &[Section], entries: &[Vec<IndexEntry>]) -> Vec<Vec<(Position, &'a str)>> {
    let slot = |s: Section| sections.iter().position(|x| *x == s);
    let mut placed: Vec<BTreeMap<String, (Position, &'a str)>> = vec![BTreeMap::new(); sections.len()];
    for (position, label) in &g.labels {
        if let Position::Node(id) = position {
            let home = g.nodes.get(id).map_or(Section::Core, section_of_node);
            if let Some(i) = slot(home) {
                placed[i].insert(position_str(position), (position.clone(), label.as_str()));
            }
        }
    }
    for (i, held) in entries.iter().enumerate() {
        for entry in held.iter().filter(|e| e.dir == DIR_FORWARD || (e.dir == DIR_SYMMETRIC && e.subject <= e.object)) {
            let at = Position::Edge(entry.edge_id.clone());
            if let Some(label) = g.labels.get(&at) {
                placed[i].insert(position_str(&at), (at, label.as_str()));
            }
        }
    }
    placed.into_iter().map(|labels| labels.into_values().collect()).collect()
}

#[cfg(test)]
mod laws {
    use super::*;
    use crate::id::{AnyNodeId, NodeKind};

    #[test]
    fn a_distinct_edge_counts_once_at_its_position_however_many_rows_or_sections_hold_it() {
        // Arrange
        let place = Position::Node(AnyNodeId { kind: NodeKind::Place, raw: "p".into() });
        let entry = |dir: i64, ord: i64, edge: &str| IndexEntry {
            subject: place.clone(),
            rel: 0,
            dir,
            ord,
            object: Position::Node(AnyNodeId { kind: NodeKind::Event, raw: edge.into() }),
            edge_id: EdgeId(format!("LocatedAt:{edge}").into()),
            meta: EdgeMeta::None,
            row_family: RowFamily::LocatedAt,
            row_id: ord,
        };
        let core = vec![entry(DIR_FORWARD, 0, "a"), entry(DIR_INVERSE, 0, "a"), entry(DIR_INVERSE, 1, "a"), entry(DIR_INVERSE, 2, "b")];
        let kjv = vec![entry(DIR_INVERSE, 3, "a"), entry(DIR_INVERSE, 4, "c")];

        // Act
        let counts = counts_of_sections(&[core, kjv]);

        // Assert
        let count = |dir: i64, count: i64| IndexCount { subject: place.clone(), rel: 0, dir, count };
        assert_eq!(counts, vec![vec![count(DIR_FORWARD, 1), count(DIR_INVERSE, 2)], vec![count(DIR_INVERSE, 1)]]);
    }
}
