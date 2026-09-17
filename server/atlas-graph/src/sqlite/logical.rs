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
use atlas_graph_types::sha256::sha256_prefixed_128;

use super::manifest::hex16;
use super::partition::SectionPartition;

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
