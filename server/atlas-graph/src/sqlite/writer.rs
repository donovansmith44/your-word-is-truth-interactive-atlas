//! DB-2b: the section writer (spec §6.1). One `.sqlite` per partition,
//! rows first, indexes after (spec §6.1 step 2), one transaction per
//! section, `VACUUM` before close. DB-4b: the file is written straight into
//! the unpack cache (`<cache>/<logical>.sqlite` -- byte-identical to what
//! unpacking its own blob yields, so the first start after a compile is a
//! cache hit), compressed to `<compiled>/sections/<name>.<logical>.sqlite.zst`
//! (zstd 19, transport hash = SHA-256 of the blob), then `manifest.toml`.
//! A recompile is idempotent: an unchanged section's blob is reused (its
//! recorded hash re-checked), `built` is preserved when root and blobs are
//! unchanged, and the manifest is rewritten only when it differs -- an idle
//! recompile leaves `git status` clean. `meta.built` is NOT in the section
//! file (it would move the blob hash on every compile).
//!
//! DB-4a: the manifest `root` IS the version root
//! (`atlas_graph_types::sections::version_root`); `meta.graph_version` is
//! gone and `SqliteSnapshot::version` reads the root.
//!
//! Judgment call 1 (plan): `edge_index` is written from Rust out of the
//! partition, not by `INSERT … SELECT` per family -- edge ids are content
//! hashes SQLite cannot compute, and `Graph::row_edges()` is the one
//! lowering both the in-memory indexes and this table are derived from.

use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use atlas_graph_types::canon::ids::{any_node_id_str, position_str};
use atlas_graph_types::canon::{Canon, CANON_VERSION};
use atlas_graph_types::explore::EdgeMeta;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{ContentAddressed, ContentHash};
use atlas_graph_types::node::{Node, NodePayload};
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::columns::JustificationWriter;
use super::ddl::{create_indexes, create_tables};
use super::blob::{compress_file, sha256_hex_of_file, BLOB_CEILING};
use super::extras::{insert_table, table_specs_of, Extras};
use super::manifest::read_manifest;
use super::source::SectionLayout;
use super::logical::logical_hash;
use atlas_graph_types::sections::logical_dump_section;
use super::manifest::{root_of, write_manifest, Manifest, ManifestSection, MANIFEST_SCHEMA};
use super::partition::{node_kind_ordinal, partition, EdgeEntryOut, SectionPartition};
use super::rows::insert_row;
use super::{hash_bytes, SqliteError, HASH_WIDTH, SCHEMA_VERSION};
use crate::sections::Section;

#[derive(Debug, Clone)]
pub struct WrittenSection {
    pub section: Section,
    /// The uncompressed file: `<cache>/<logical>.sqlite`.
    pub path: PathBuf,
    /// The committed blob: `<compiled>/sections/<name>.<logical>.sqlite.zst`.
    pub blob_path: PathBuf,
    pub logical: String,
    /// SHA-256 of the blob (64 hex): the manifest's transport hash.
    pub blob: String,
    /// Compressed size (the manifest's `bytes`).
    pub bytes: u64,
    pub uncompressed_bytes: u64,
    /// The blob already existed with the recorded hash: not recompressed.
    pub reused_blob: bool,
    pub node_count: usize,
    pub row_count: usize,
    /// DB-4b: rows of the section's extra tables (projections, sidecars).
    pub extra_row_count: usize,
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
        NodePayload::LexiconEntry { lemma, .. } => Some(lemma),
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

    // The logical hash is a pure function of the graph and the section
    // (`sections::logical_dump_section`, the same walk `version_root`
    // hashes); computed first so it can be stamped into `meta` and named
    // on the file.
    let logical = logical_hash(&logical_dump_section(g, p.section));

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
        insert_edges(&tx, &p.edges)?;
        if let Some((_corpus, order)) = p.spine {
            let mut stmt = tx.prepare_cached("INSERT INTO reading_spine (ord, node_id) VALUES (?, ?)")?;
            for (i, id) in order.iter().enumerate() {
                stmt.execute(rusqlite::params![i as i64, any_node_id_str(id)])?;
            }
        }
        // DB-4b: the section's extra tables, typed rows in pk order; their
        // canonical bodies are already in `g.extra_tables` (the dump).
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
    let path = layout.cache_path(&logical);
    let _ = std::fs::remove_file(&path);
    std::fs::rename(&tmp, &path)?;

    // The blob: reused when the previous manifest recorded this very
    // (name, logical) and the file on disk still hashes to what it said;
    // compressed otherwise. Either way the size is checked against the
    // git ceiling (spec §2.3).
    let blob_path = layout.blob_path(name, &logical);
    let prev = previous.and_then(|m| m.sections.iter().find(|s| s.name == name && s.logical == logical));
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
        edge_count: p.edges.len(),
        elapsed: started.elapsed(),
    })
}

/// Writes every shipped section: `<cache>/<logical>.sqlite`,
/// `<compiled>/sections/<name>.<logical>.sqlite.zst` (a stale
/// `<name>.*.sqlite.zst` is deleted; the cache is content-addressed and
/// kept), then `<compiled>/manifest.toml` -- rewritten only when it
/// differs, its `built` preserved when root and every blob are unchanged.
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
    let manifest = Manifest { schema: MANIFEST_SCHEMA, compiler: compiler.to_string(), built, root, sections };
    if previous.as_ref() != Some(&manifest) {
        write_manifest(&manifest, &layout.manifest_path())?;
    }
    Ok((manifest, written))
}
