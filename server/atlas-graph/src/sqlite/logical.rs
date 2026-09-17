//! DB-2b: the per-section logical dump and hash (spec §3.4).
//!
//! Line format, binding: for each table in `logical_table_order(section)`,
//! for each row in primary-key order, `<table>\t<canonical row JSON>\n`:
//! `node` -> `Node::encode()`, a row table -> `encode_row_in_family`, and
//! `reading_spine` -> `{"corpus":…,"node_id":…,"ord":N}` through
//! `canon::obj`/`serialize` (keys in byte order, no whitespace). Logical
//! hash = `sha256_prefixed_128(DOMAIN_PREFIX, dump)` as 32 lowercase hex.
//!
//! Node order on both sides is `any_node_id_str` BYTE order -- what
//! SQLite's `ORDER BY id` (TEXT) yields -- never `AnyNodeId: Ord`.

use atlas_graph_types::canon::ids::any_node_id_str;
use atlas_graph_types::canon::{encode_row_in_family, obj, serialize, str_value, Canon, Value, DOMAIN_PREFIX};
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::node::Node;
use atlas_graph_types::sha256::sha256_prefixed_128;
use rusqlite::Connection;

use super::ddl::{has_spine, logical_table_order};
use super::manifest::hex16;
use super::partition::SectionPartition;
use super::rows::read_rows;
use super::SqliteError;
use crate::sections::Section;

fn line(out: &mut Vec<u8>, table: &str, body: &[u8]) {
    out.extend_from_slice(table.as_bytes());
    out.push(b'\t');
    out.extend_from_slice(body);
    out.push(b'\n');
}

/// The `reading_spine` line body: `{"corpus":…,"node_id":…,"ord":N}`.
pub fn spine_line_body(corpus: &str, ord: i64, node_id: &str) -> Vec<u8> {
    serialize(&obj(vec![("corpus", str_value(corpus)), ("node_id", str_value(node_id)), ("ord", Value::Int(ord))]))
}

pub(crate) fn push_spine_lines(out: &mut Vec<u8>, corpus: &str, order: &[AnyNodeId]) {
    for (i, id) in order.iter().enumerate() {
        line(out, "reading_spine", &spine_line_body(corpus, i as i64, &any_node_id_str(id)));
    }
}

/// From the in-memory partition (the writer's side).
pub fn logical_dump_of_partition(p: &SectionPartition) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for n in &p.nodes {
        line(&mut out, "node", &n.encode());
    }
    // `p.rows` is already family-by-`row_tables_of`-order, ord ascending.
    for (family, _ord, row) in &p.rows {
        line(&mut out, family.name(), &encode_row_in_family(*family, row.to_value()));
    }
    if let Some((corpus, order)) = p.spine {
        push_spine_lines(&mut out, corpus, order);
    }
    out
}

/// 32 lowercase hex of `sha256_prefixed_128(DOMAIN_PREFIX, dump)`.
pub fn logical_hash(dump: &[u8]) -> String {
    hex16(&sha256_prefixed_128(DOMAIN_PREFIX, dump))
}

/// Recomputed from an open section file by streaming its tables in
/// `logical_table_order`: `node ORDER BY id` (the `payload` BLOB verbatim,
/// after `Node::decode` succeeds and its id agrees with the `id` column),
/// each family through `read_rows` (re-encoded, so a column drift shows as
/// a hash drift), `reading_spine ORDER BY ord`. Must equal
/// `logical_dump_of_partition` for the partition that wrote the file --
/// that equality is the gate.
pub fn logical_dump_of_db(conn: &Connection, section: Section) -> Result<Vec<u8>, SqliteError> {
    let mut out: Vec<u8> = Vec::new();
    for table in logical_table_order(section) {
        match table {
            "node" => {
                let mut stmt = conn.prepare("SELECT id, payload FROM node ORDER BY id")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let id: String = row.get(0)?;
                    let payload: Vec<u8> = row.get(1)?;
                    let decoded = Node::decode(&payload).map_err(|e| SqliteError(format!("node {id}: payload does not decode: {e}")))?;
                    let spelled = any_node_id_str(&decoded.id);
                    if spelled != id {
                        return Err(SqliteError(format!("node {id}: payload id is {spelled}")));
                    }
                    line(&mut out, "node", &payload);
                }
            }
            "reading_spine" => {
                let corpus: &'static str = match section {
                    Section::Kjv => "bible",
                    Section::Concord => "concord",
                    other => return Err(SqliteError(format!("{other:?} has no reading_spine"))),
                };
                let mut stmt = conn.prepare("SELECT ord, node_id FROM reading_spine ORDER BY ord")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let ord: i64 = row.get(0)?;
                    let node_id: String = row.get(1)?;
                    line(&mut out, "reading_spine", &spine_line_body(corpus, ord, &node_id));
                }
            }
            family_table => {
                let family = atlas_graph_types::canon::RowFamily::ALL
                    .iter()
                    .copied()
                    .find(|f| f.name() == family_table)
                    .ok_or_else(|| SqliteError(format!("{family_table} is not a row family table")))?;
                for (_ord, row) in read_rows(conn, family)? {
                    line(&mut out, family_table, &encode_row_in_family(family, row.to_value()));
                }
            }
        }
    }
    debug_assert!(has_spine(section) == logical_table_order(section).contains(&"reading_spine"));
    Ok(out)
}
