//! DB-2b: the section writer (spec §6.1). One `.sqlite` per partition,
//! named `<name>.<logical>.sqlite` once its logical hash is known, then
//! `manifest.toml`. Rows first, indexes after (spec §6.1 step 2), one
//! transaction per section, `VACUUM` before close.
//!
//! Judgment call 1 (plan): `edge_index` is written from Rust out of the
//! partition, not by `INSERT … SELECT` per family -- edge ids are content
//! hashes SQLite cannot compute, and `Graph::row_edges()` is the one
//! lowering both the in-memory indexes and this table are derived from.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use atlas_graph_types::canon::ids::{any_node_id_str, position_str};
use atlas_graph_types::canon::{Canon, CANON_VERSION};
use atlas_graph_types::explore::EdgeMeta;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{ContentAddressed, ContentHash};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::sha256::sha256;
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::columns::JustificationWriter;
use super::ddl::{create_indexes, create_tables};
use super::logical::{logical_dump_of_partition, logical_hash};
use super::manifest::{root_of, write_manifest, Manifest, ManifestSection, MANIFEST_SCHEMA};
use super::partition::{node_kind_ordinal, partition, EdgeEntryOut, SectionPartition};
use super::rows::insert_row;
use super::{hash_bytes, SqliteError, HASH_WIDTH, SCHEMA_VERSION};
use crate::sections::Section;

#[derive(Debug, Clone)]
pub struct WrittenSection {
    pub section: Section,
    pub path: PathBuf,
    pub logical: String,
    pub blob: String,
    pub bytes: u64,
    pub node_count: usize,
    pub row_count: usize,
    pub edge_count: usize,
    pub elapsed: Duration,
}

/// The display string hoisted onto `node.label` (informational only).
pub fn node_label(n: &Node) -> Option<&str> {
    match &n.payload {
        NodePayload::TextUnit { .. } => None,
        NodePayload::Container { title } => Some(title),
        NodePayload::Event { label, .. }
        | NodePayload::Narrative { label, .. }
        | NodePayload::Person { label, .. }
        | NodePayload::PeopleGroup { label, .. }
        | NodePayload::Era { label, .. }
        | NodePayload::Polity { label, .. }
        | NodePayload::CatechismItem { label }
        | NodePayload::Source { label }
        | NodePayload::Translation { label } => Some(label),
        NodePayload::Place { canonical, .. } => Some(canonical),
        NodePayload::Anchor { citation, .. } => Some(citation),
        NodePayload::CommentaryItem { work, .. } => Some(&work.0),
    }
}

/// `"Rel:hex"` -> the hash bytes for the `edge_id` BLOB.
pub fn edge_id_blob(id: &atlas_graph_types::edge::EdgeId) -> Result<Vec<u8>, SqliteError> {
    let (_, hex) = id.0.split_once(':').ok_or_else(|| SqliteError(format!("edge id {} has no ':'", id.0)))?;
    let h = ContentHash::from_hex(hex).ok_or_else(|| SqliteError(format!("edge id {} carries no ContentHash", id.0)))?;
    Ok(hash_bytes(&h))
}

fn insert_nodes(tx: &Transaction, nodes: &[&Node]) -> Result<(), SqliteError> {
    let mut stmt = tx.prepare_cached("INSERT INTO node (id, kind, pid, label, provenance, payload) VALUES (?, ?, ?, ?, ?, ?)")?;
    for n in nodes {
        stmt.execute(rusqlite::params![
            any_node_id_str(&n.id),
            node_kind_ordinal(n.id.kind),
            hash_bytes(&n.pid().hash),
            node_label(n),
            n.provenance,
            n.encode(),
        ])?;
    }
    Ok(())
}

fn insert_edges(tx: &Transaction, edges: &[EdgeEntryOut]) -> Result<(), SqliteError> {
    let mut stmt = tx.prepare_cached(
        "INSERT INTO edge_index (subject, rel, dir, ord, object, edge_id, meta_kind, meta_narrative, meta_votes, row_family, row_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )?;
    for e in edges {
        let (meta_kind, narrative, votes): (i64, Value, Value) = match &e.meta {
            EdgeMeta::None => (0, Value::Null, Value::Null),
            EdgeMeta::Narrative(n) => (1, Value::Text(n.0.clone()), Value::Null),
            EdgeMeta::Votes(v) => (2, Value::Null, Value::Integer(i64::from(*v))),
        };
        stmt.execute(rusqlite::params![
            position_str(&e.subject),
            e.rel,
            e.dir,
            e.ord,
            position_str(&e.object),
            edge_id_blob(&e.edge_id)?,
            meta_kind,
            narrative,
            votes,
            i64::from(e.row_family.ordinal()),
            e.row_id,
        ])?;
    }
    Ok(())
}

fn insert_meta(conn: &Connection, pairs: &[(&str, String)]) -> Result<(), SqliteError> {
    let mut stmt = conn.prepare_cached("INSERT INTO meta (key, value) VALUES (?, ?)")?;
    for (k, v) in pairs {
        stmt.execute(rusqlite::params![k, v])?;
    }
    Ok(())
}

/// RFC 3339 UTC, second precision, from the system clock (no chrono dep).
pub fn now_rfc3339() -> String {
    let secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, (rem % 3600) / 60, rem % 60)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_one(p: &SectionPartition, graph_version_hex: &str, compiler: &str, built: &str, out_dir: &Path) -> Result<WrittenSection, SqliteError> {
    let started = Instant::now();
    let name = p.section.name();
    let tmp = out_dir.join(format!("{name}.tmp.sqlite"));
    let _ = std::fs::remove_file(&tmp);

    // The logical hash is a pure function of the partition; compute it
    // first so it can be stamped into `meta` and named on the file.
    let logical = logical_hash(&logical_dump_of_partition(p));

    let mut conn = Connection::open(&tmp)?;
    create_tables(&conn, p.section)?;
    {
        let tx = conn.transaction()?;
        insert_nodes(&tx, &p.nodes)?;
        let mut jw = JustificationWriter::new();
        for (_family, ord, row) in &p.rows {
            insert_row(&tx, &mut jw, *ord, row)?;
        }
        insert_edges(&tx, &p.edges)?;
        if let Some((_corpus, order)) = p.spine {
            let mut stmt = tx.prepare_cached("INSERT INTO reading_spine (ord, node_id) VALUES (?, ?)")?;
            for (i, id) in order.iter().enumerate() {
                stmt.execute(rusqlite::params![i as i64, any_node_id_str(id)])?;
            }
        }
        tx.commit()?;
    }
    create_indexes(&conn, p.section)?;
    insert_meta(
        &conn,
        &[
            ("section_name", name.to_string()),
            ("schema_version", SCHEMA_VERSION.to_string()),
            ("logical_hash", logical.clone()),
            ("compiler", compiler.to_string()),
            ("canon_version", CANON_VERSION.to_string()),
            ("built", built.to_string()),
            ("hash_width", HASH_WIDTH.to_string()),
            ("graph_version", graph_version_hex.to_string()),
        ],
    )?;
    conn.execute_batch("VACUUM;")?;
    drop(conn);

    let bytes_of_file = std::fs::read(&tmp)?;
    let blob = hex(&sha256(&bytes_of_file));
    let bytes = bytes_of_file.len() as u64;
    drop(bytes_of_file);
    let path = out_dir.join(format!("{name}.{logical}.sqlite"));
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&tmp, &path)?;

    Ok(WrittenSection {
        section: p.section,
        path,
        logical,
        blob,
        bytes,
        node_count: p.nodes.len(),
        row_count: p.rows.len(),
        edge_count: p.edges.len(),
        elapsed: started.elapsed(),
    })
}

/// Writes every shipped section to `<out_dir>/<name>.<logical>.sqlite`
/// (stale `<name>.*.sqlite` files are deleted first) and
/// `<out_dir>/manifest.toml`.
pub fn write_sections(
    g: &Graph,
    graph_version_hex: &str,
    compiler: &str,
    out_dir: &Path,
) -> Result<(Manifest, Vec<WrittenSection>), SqliteError> {
    std::fs::create_dir_all(out_dir)?;
    let parts = partition(g)?;
    for p in &parts {
        let prefix = format!("{}.", p.section.name());
        for entry in std::fs::read_dir(out_dir)? {
            let entry = entry?;
            let file = entry.file_name();
            let file = file.to_string_lossy();
            if file.starts_with(&prefix) && file.ends_with(".sqlite") {
                std::fs::remove_file(entry.path())?;
            }
        }
    }
    let built = now_rfc3339();
    let mut written = Vec::with_capacity(parts.len());
    for p in &parts {
        written.push(write_one(p, graph_version_hex, compiler, &built, out_dir)?);
    }
    let sections: Vec<ManifestSection> = written
        .iter()
        .map(|w| ManifestSection {
            name: w.section.name().to_string(),
            required: w.section.required(),
            logical: w.logical.clone(),
            blob: w.blob.clone(),
            bytes: w.bytes,
            schema_version: SCHEMA_VERSION,
        })
        .collect();
    let manifest = Manifest { schema: MANIFEST_SCHEMA, compiler: compiler.to_string(), built, root: root_of(&sections), sections };
    write_manifest(&manifest, &out_dir.join("manifest.toml"))?;
    Ok((manifest, written))
}
