//! The per-section logical dump recomputed from an open section file, and the hash string the
//! manifest carries.

use atlas_graph_types::canon::ids::any_node_id_str;
use atlas_graph_types::canon::Canon;
use atlas_graph_types::node::Node;
use rusqlite::Connection;

use atlas_graph_types::canon::Value;
use atlas_graph_types::id::ContentAddressed;
use atlas_graph_types::section_index::{derived_line_body, derived_table_named, DerivedTable};
use rusqlite::types::ValueRef;

use super::partition::node_kind_ordinal;
use super::rows::read_rows;
use super::{hash_bytes, hash_from_bytes, SqliteError};
pub use crate::sections::{row_line_body, spine_line_body};
use crate::sections::{has_spine, logical_table_order, Section};

fn line(out: &mut Vec<u8>, table: &str, body: &[u8]) {
    out.extend_from_slice(table.as_bytes());
    out.push(b'\t');
    out.extend_from_slice(body);
    out.push(b'\n');
}

fn read_derived(conn: &Connection, table: &DerivedTable) -> Result<Vec<Vec<u8>>, SqliteError> {
    let sql = format!("SELECT {} FROM {} ORDER BY {}", table.columns.join(", "), table.name, table.key.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut bodies = Vec::new();
    while let Some(row) = rows.next()? {
        let mut values = Vec::with_capacity(table.columns.len());
        for i in 0..table.columns.len() {
            values.push(match row.get_ref(i)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(n) => Value::Int(n),
                ValueRef::Text(t) => Value::Str(String::from_utf8(t.to_vec()).map_err(|e| SqliteError(format!("{}: {e}", table.name)))?),
                ValueRef::Blob(b) => Value::Str(hash_from_bytes(b)?.hex()),
                ValueRef::Real(_) => return Err(SqliteError(format!("{}: a REAL in a derived table", table.name))),
            });
        }
        bodies.push(derived_line_body(table.columns, values));
    }
    Ok(bodies)
}

/// The section logical hash as the manifest spells it: the hex of `sections::logical_hash`.
pub fn logical_hash(dump: &[u8]) -> String {
    atlas_graph_types::sections::logical_hash(dump).hex()
}

/// Streams the section's tables in `logical_table_order`, re-encoding every row it reads, so a
/// drifted column shows up as a hash drift instead of passing.
pub fn logical_dump_of_db(conn: &Connection, section: Section) -> Result<Vec<u8>, SqliteError> {
    let mut out: Vec<u8> = Vec::new();
    for table in logical_table_order(section) {
        match table {
            "node" => {
                let mut stmt = conn.prepare("SELECT id, kind, pid, provenance, payload FROM node ORDER BY id")?;
                let mut rows = stmt.query([])?;
                while let Some(row) = rows.next()? {
                    let id: String = row.get(0)?;
                    let stored = (row.get::<_, i64>(1)?, row.get::<_, Vec<u8>>(2)?, row.get::<_, String>(3)?);
                    let payload: Vec<u8> = row.get(4)?;
                    let decoded = Node::decode(&payload).map_err(|e| SqliteError(format!("node {id}: payload does not decode: {e}")))?;
                    let spelled = any_node_id_str(&decoded.id);
                    if spelled != id {
                        return Err(SqliteError(format!("node {id}: payload id is {spelled}")));
                    }
                    if stored != (node_kind_ordinal(decoded.id.kind), hash_bytes(&decoded.pid().hash), decoded.provenance.clone()) {
                        return Err(SqliteError(format!("node {id}: its kind, pid or provenance column disagrees with its payload")));
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
            derived if derived_table_named(derived).is_some() => {
                let table = derived_table_named(derived).expect("guarded");
                for body in read_derived(conn, table)? {
                    line(&mut out, derived, &body);
                }
            }
            extra if super::extras::spec_named(extra).is_some() => {
                let spec = super::extras::spec_named(extra).expect("guarded");
                for row in super::extras::read_table(conn, spec)? {
                    line(&mut out, extra, &super::extras::row_body(spec, &row)?);
                }
            }
            family_table => {
                let family = atlas_graph_types::canon::RowFamily::ALL
                    .iter()
                    .copied()
                    .find(|f| f.name() == family_table)
                    .ok_or_else(|| SqliteError(format!("{family_table} is not a row family table")))?;
                for (ord, row) in read_rows(conn, family)? {
                    line(&mut out, family_table, &row_line_body(family, ord, row.to_value()));
                }
            }
        }
    }
    debug_assert!(has_spine(section) == logical_table_order(section).contains(&"reading_spine"));
    Ok(out)
}
