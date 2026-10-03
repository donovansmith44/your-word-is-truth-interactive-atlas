//! The section writer. The file is written straight into the unpack cache, byte-identical to what
//! unpacking its own blob yields, so the first start after a compile is a cache hit. `meta.built`
//! is deliberately NOT in the section file: it would move the blob hash on every compile.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use atlas_graph_types::canon::ids::any_node_id_str;
use atlas_graph_types::canon::{Canon, CANON_VERSION};
use atlas_graph_types::canon::Value as CanonValue;
use atlas_graph_types::section_index::{
    edge_count_values, edge_index_values, label_values, SectionIndex, EDGE_COUNT_COLUMNS, EDGE_INDEX_COLUMNS, LABEL_COLUMNS,
};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{ContentAddressed, ContentHash};
use atlas_graph_types::node::Node;
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::columns::JustificationWriter;
use super::ddl::{create_indexes, create_tables};
use super::blob::{compress_file, sha256_hex_of_file, BLOB_CEILING};
use super::extras::{insert_table, table_specs_of, Extras};
use super::manifest::{raw_manifest_path, read_manifest, recorded_raw_root};
use super::source::SectionLayout;
use super::logical::logical_hash;
use atlas_graph_types::sections::logical_dump_section;
use super::manifest::{root_of, write_manifest, Manifest, ManifestSection, MANIFEST_SCHEMA};
use super::partition::{node_kind_ordinal, partition, SectionPartition};
use super::rows::insert_row;
use super::{hash_bytes, SqliteError, HASH_WIDTH, SCHEMA_VERSION};
use crate::sections::Section;

#[derive(Debug, Clone)]
pub struct WrittenSection {
    pub section: Section,
    pub path: PathBuf,
    /// The committed blob: `<compiled>/sections/<name>.<logical>.sqlite.zst`.
    pub blob_path: PathBuf,
    pub logical: String,
    /// SHA-256 of the blob, 64 hex: the manifest's transport hash.
    pub blob: String,
    pub bytes: u64,
    pub uncompressed_bytes: u64,
    /// The blob already existed with the recorded hash: not recompressed.
    pub reused_blob: bool,
    pub node_count: usize,
    pub row_count: usize,
    /// Rows of the section's extra tables (projections, sidecars).
    pub extra_row_count: usize,
    pub edge_count: usize,
    pub elapsed: Duration,
}

/// `"Rel:hex"` -> the hash bytes for the `edge_id` BLOB.
pub fn edge_id_blob(id: &atlas_graph_types::edge::EdgeId) -> Result<Vec<u8>, SqliteError> {
    let (_, hex) = id.0.split_once(':').ok_or_else(|| SqliteError(format!("edge id {} has no ':'", id.0)))?;
    let h = ContentHash::from_hex(hex).ok_or_else(|| SqliteError(format!("edge id {} carries no ContentHash", id.0)))?;
    Ok(hash_bytes(&h))
}

fn insert_nodes(tx: &Transaction, nodes: &[&Node]) -> Result<(), SqliteError> {
    let mut stmt = tx.prepare_cached("INSERT INTO node (id, kind, pid, provenance, payload) VALUES (?, ?, ?, ?, ?)")?;
    for n in nodes {
        stmt.execute(rusqlite::params![
            any_node_id_str(&n.id),
            node_kind_ordinal(n.id.kind),
            hash_bytes(&n.pid().hash),
            n.provenance,
            n.encode(),
        ])?;
    }
    Ok(())
}

fn sql_value(v: CanonValue) -> Result<Value, SqliteError> {
    match v {
        CanonValue::Null => Ok(Value::Null),
        CanonValue::Int(i) => Ok(Value::Integer(i)),
        CanonValue::Str(s) => Ok(Value::Text(s)),
        other => Err(SqliteError(format!("a derived column holds {other:?}, which no derived table stores"))),
    }
}

fn insert_derived<const N: usize>(tx: &Transaction, table: &str, columns: [&str; N], rows: impl Iterator<Item = Result<[Value; N], SqliteError>>) -> Result<(), SqliteError> {
    let sql = format!("INSERT INTO {table} ({}) VALUES ({})", columns.join(", "), vec!["?"; N].join(", "));
    let mut stmt = tx.prepare_cached(&sql)?;
    for row in rows {
        stmt.execute(rusqlite::params_from_iter(row?))?;
    }
    Ok(())
}

fn sql_row<const N: usize>(values: [CanonValue; N]) -> Result<[Value; N], SqliteError> {
    let mut out: [Value; N] = std::array::from_fn(|_| Value::Null);
    for (slot, v) in out.iter_mut().zip(values) {
        *slot = sql_value(v)?;
    }
    Ok(out)
}

fn insert_index(tx: &Transaction, index: &SectionIndex<'_>) -> Result<(), SqliteError> {
    insert_derived(tx, "label", LABEL_COLUMNS, index.labels.iter().map(|(position, label)| sql_row(label_values(position, label))))?;
    let edge_id = EDGE_INDEX_COLUMNS.iter().position(|c| *c == "edge_id").expect("edge_index has an edge_id column");
    insert_derived(
        tx,
        "edge_index",
        EDGE_INDEX_COLUMNS,
        index.entries.iter().map(|e| {
            let mut row = sql_row(edge_index_values(e)?)?;
            row[edge_id] = Value::Blob(edge_id_blob(&e.edge_id)?);
            Ok(row)
        }),
    )?;
    insert_derived(tx, "edge_count", EDGE_COUNT_COLUMNS, index.counts.iter().map(|c| sql_row(edge_count_values(c))))
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

fn write_one(
    g: &Graph,
    p: &SectionPartition,
    extras: &Extras,
    compiler: &str,
    layout: &SectionLayout,
    previous: Option<&Manifest>,
) -> Result<WrittenSection, SqliteError> {
    let started = Instant::now();
    let name = p.section.name();
    std::fs::create_dir_all(&layout.cache_dir)?;
    let tmp = layout.cache_dir.join(format!("{name}.build.tmp"));
    let _ = std::fs::remove_file(&tmp);

    // The logical hash is a pure function of the graph and the section, computed first so it can
    // be stamped into `meta` and used to name the file.
    let logical = logical_hash(&logical_dump_section(g, &p.index)?);

    let mut conn = Connection::open(&tmp)?;
    create_tables(&conn, p.section)?;
    let mut extra_rows = 0usize;
    {
        let tx = conn.transaction()?;
        insert_nodes(&tx, &p.nodes)?;
        let mut jw = JustificationWriter::new();
        for (_family, ord, row) in &p.rows {
            insert_row(&tx, &mut jw, *ord, row)?;
        }
        insert_index(&tx, &p.index)?;
        if let Some((_corpus, order)) = p.spine {
            let mut stmt = tx.prepare_cached("INSERT INTO reading_spine (ord, node_id) VALUES (?, ?)")?;
            for (i, id) in order.iter().enumerate() {
                stmt.execute(rusqlite::params![i as i64, any_node_id_str(id)])?;
            }
        }
        for spec in table_specs_of(p.section) {
            if let Some(t) = extras.table(spec.name) {
                insert_table(&tx, t)?;
                extra_rows += t.rows.len();
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
            ("hash_width", HASH_WIDTH.to_string()),
        ],
    )?;
    conn.execute_batch("VACUUM;")?;
    drop(conn);

    let uncompressed_bytes = std::fs::metadata(&tmp)?.len();
    let path = layout.cache_path(&logical, SCHEMA_VERSION);
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&tmp, &path)?;

    let blob_path = layout.blob_path(name, &logical);
    // A blob's bytes carry the schema version its file was stamped with, so a previous manifest
    // vouches for one only under the schema this build writes.
    let prev = previous.and_then(|m| m.sections.iter().find(|s| s.name == name && s.logical == logical && s.schema_version == SCHEMA_VERSION));
    let (blob, bytes, reused_blob) = match prev {
        Some(prev) if blob_path.is_file() && sha256_hex_of_file(&blob_path)? == prev.blob => (prev.blob.clone(), prev.bytes, true),
        _ => {
            let (h, b) = compress_file(&path, &blob_path)?;
            (h, b, false)
        }
    };
    if bytes > BLOB_CEILING {
        return Err(SqliteError(format!(
            "section {name} compressed to {bytes} bytes, over the {BLOB_CEILING}-byte ceiling (spec 2.3): it needs the fetching source (spec 2.4, implementation #2) before it can ship"
        )));
    }

    Ok(WrittenSection {
        section: p.section,
        path,
        blob_path,
        logical,
        blob,
        bytes,
        uncompressed_bytes,
        reused_blob,
        node_count: p.nodes.len(),
        row_count: p.rows.len(),
        extra_row_count: extra_rows,
        edge_count: p.index.entries.len(),
        elapsed: started.elapsed(),
    })
}

/// Writes every shipped section, deleting a stale `<name>.*.sqlite.zst` while keeping the
/// content-addressed cache, then the manifest -- rewritten only when it differs, its `built`
/// preserved when the root and every blob are unchanged.
pub fn write_sections(
    g: &Graph,
    extras: &Extras,
    compiler: &str,
    layout: &SectionLayout,
) -> Result<(Manifest, Vec<WrittenSection>), SqliteError> {
    std::fs::create_dir_all(layout.sections_dir())?;
    std::fs::create_dir_all(&layout.cache_dir)?;
    // A previous manifest that fails its own root check is treated as absent.
    let previous = read_manifest(&layout.manifest_path()).ok();
    let raw_root = recorded_raw_root(&raw_manifest_path(layout))?;
    let parts = partition(g)?;
    let mut written = Vec::with_capacity(parts.len());
    for p in &parts {
        written.push(write_one(g, p, extras, compiler, layout, previous.as_ref())?);
    }
    for entry in std::fs::read_dir(layout.sections_dir())? {
        let entry = entry?;
        let file = entry.file_name();
        let file = file.to_string_lossy();
        for w in &written {
            let prefix = format!("{}.", w.section.name());
            let current = format!("{}.{}.sqlite.zst", w.section.name(), w.logical);
            if file.starts_with(&prefix) && file.ends_with(".sqlite.zst") && file != current {
                std::fs::remove_file(entry.path())?;
            }
        }
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
    let root = root_of(&sections);
    let built = match &previous {
        Some(prev)
            if prev.root == root
                && prev.sections.iter().map(|s| (&s.name, &s.blob)).eq(sections.iter().map(|s| (&s.name, &s.blob))) =>
        {
            prev.built.clone()
        }
        _ => now_rfc3339(),
    };
    let manifest = Manifest { schema: MANIFEST_SCHEMA, compiler: compiler.to_string(), built, root, raw_root, sections };
    if previous.as_ref() != Some(&manifest) {
        write_manifest(&manifest, &layout.manifest_path())?;
    }
    Ok((manifest, written))
}
