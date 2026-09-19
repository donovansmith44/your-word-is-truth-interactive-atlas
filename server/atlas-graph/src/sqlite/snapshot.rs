//! DB-2b: `SqliteSnapshot` -- the read port (`GraphQuery` +
//! `GraphSnapshot`) over the attached section files (spec §2.5, §5.2).
//! Every answer must equal the in-memory `Graph`'s; `assert_answers_match`
//! is the judge (the specimen in `sqlite_laws.rs`, the real graph in
//! `sqlite_real_data.rs`).
//!
//! DB-4c (spec §2.5 step 3): one connection PER WORKER -- `open_with_workers`
//! opens `n` read-only connections (each attaching the same files and
//! building its own TEMP views), `with_conn` hands a query the first free
//! one (round-robin `try_lock`, falling back to blocking on its own slot),
//! and `PRAGMA mmap_size` is the attached files' sum (capped). `open` is
//! the one-worker form the gates and `bibex verify` use.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use atlas_graph_types::canon::ids::{any_node_id_str, parse_any_node_id, position_str};
use atlas_graph_types::canon::Canon;
use atlas_graph_types::edge::{Direction, EdgeId, EdgeKind};
use atlas_graph_types::explore::{EdgeEntry, EdgeMeta, EdgePage, EdgeQuery, EdgeSummary, NodePage};
use atlas_graph_types::graph::EdgeRel;
use atlas_graph_types::id::{AnyNodeId, ContentAddressed, ContentHash, NarrativeId, NodeKind, Pid, Position};
use atlas_graph_types::node::Node;
use atlas_graph_types::store::{GraphQuery, GraphSnapshot, GraphVersion, RowRef};
use rusqlite::{Connection, OptionalExtension};

use super::manifest::{read_manifest, Manifest};
use super::SCHEMA_VERSION;
use super::source::{is_missing, SectionSource};
use super::partition::{directed_rel_code, node_kind_ordinal, rel_code_of, rel_of_code, DIR_FORWARD, DIR_INVERSE, DIR_SYMMETRIC};
use super::{hash_bytes, hash_from_bytes, open_read_only, SqliteError};
use crate::sections::Section;

pub struct SqliteSnapshot {
    /// One per worker (spec §2.5 step 3); a query takes the first free one.
    conns: Vec<Mutex<Connection>>,
    next: AtomicUsize,
    version: GraphVersion,
    /// Manifest order, attached sections only.
    present: Vec<Section>,
    /// Optional sections the manifest lists whose blob is absent (spec §11).
    absent: Vec<Section>,
    manifest: Manifest,
    /// `PRAGMA mmap_size` on every connection: the attached files' sum, capped.
    mmap_bytes: u64,
}

impl std::fmt::Debug for SqliteSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteSnapshot")
            .field("version", &self.version)
            .field("present", &self.present)
            .field("absent", &self.absent)
            .field("workers", &self.conns.len())
            .field("mmap_bytes", &self.mmap_bytes)
            .finish_non_exhaustive()
    }
}

/// Spec §2.5 step 3: "capped by platform" -- 1 GiB covers the four
/// shipped sections (356 MB) with room for the lexicon.
pub const MMAP_CAP: u64 = 1 << 30;

fn section_named(name: &str) -> Option<Section> {
    Section::MANIFEST_ORDER.iter().copied().find(|s| s.name() == name)
}

impl SqliteSnapshot {
    /// The one-worker form (the gates, `bibex verify`, the tests).
    /// ADMIT-PERF-1: one connection per core -- what an
    /// `assert_answers_match` admission needs to actually use the cores its
    /// sweep now spreads across (a single-connection pool would serialize
    /// every worker on one mutex, and a pool SMALLER than the sweep's own
    /// thread count leaves the surplus threads queueing: measured on this
    /// box, 16 sweep threads against a pool of 8 swept in 397.4 s, against
    /// a matched pool of 16 in 373.8 s).
    ///
    /// The pool is cheap where it matters: every connection opens the SAME
    /// section files, so their `mmap` pages are one shared set in the page
    /// cache, not a private copy each -- the per-connection cost is the
    /// handles, the statement cache and address space, never the ~540 MB of
    /// section bytes multiplied out. Capped at 16 anyway: past that the
    /// sweep is bound by those shared pages, not by cores.
    pub fn admission_workers() -> usize {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(16)
    }

    pub fn open(manifest_path: &Path, source: &dyn SectionSource) -> Result<SqliteSnapshot, SqliteError> {
        Self::open_with_workers(manifest_path, source, 1)
    }

    /// Spec §2.5 steps 1–4: read + verify the manifest; resolve every
    /// section through the `SectionSource` (DB-4b: `CommittedZstdSource`
    /// verifies the transport hash and unpacks on a cache miss); then, for
    /// each of `workers` connections: open `core` as `main`, ATTACH every
    /// other present section under its name in manifest order, refuse a
    /// `user_version` this build does not understand (spec §11, the
    /// artifact wall's successor), `PRAGMA mmap_size`, `PRAGMA query_only =
    /// ON`, and build the TEMP views `all_edge_index` and `all_node` over
    /// the attached sections. A `required` section that cannot be resolved
    /// is an error naming the section and its logical hash; an optional one
    /// whose blob is MISSING is recorded absent and skipped -- any other
    /// failure (a present but corrupt blob, an unpack error) is loud for
    /// optional sections too.
    pub fn open_with_workers(manifest_path: &Path, source: &dyn SectionSource, workers: usize) -> Result<SqliteSnapshot, SqliteError> {
        if workers == 0 {
            return Err(SqliteError("SqliteSnapshot needs at least one worker connection".into()));
        }
        let manifest = read_manifest(manifest_path)?;
        let mut present: Vec<(Section, PathBuf)> = Vec::new();
        let mut absent: Vec<Section> = Vec::new();
        for ms in &manifest.sections {
            let section = section_named(&ms.name)
                .ok_or_else(|| SqliteError(format!("manifest names an unknown section {}", ms.name)))?;
            if ms.schema_version != SCHEMA_VERSION {
                return Err(SqliteError(format!(
                    "section {} schema_version {} unsupported (this build understands {SCHEMA_VERSION})",
                    ms.name, ms.schema_version
                )));
            }
            match source.resolve(ms) {
                Ok(path) => present.push((section, path)),
                Err(e) if !ms.required && is_missing(&e) => absent.push(section),
                Err(e) => {
                    return Err(SqliteError(format!(
                        "{} section {} ({}) unavailable: {e}",
                        if ms.required { "required" } else { "optional" },
                        ms.name,
                        ms.logical
                    )))
                }
            }
        }
        let Some((Section::Core, _)) = present.first() else {
            return Err(SqliteError("manifest does not list core first".into()));
        };
        let mut total: u64 = 0;
        for (_, path) in &present {
            total = total.saturating_add(std::fs::metadata(path)?.len());
        }
        let mmap_bytes = total.min(MMAP_CAP);
        let mut edge_view = String::from("CREATE TEMP VIEW all_edge_index AS SELECT 0 AS sec, * FROM main.edge_index");
        let mut node_view = String::from("CREATE TEMP VIEW all_node AS SELECT 0 AS sec, * FROM main.node");
        for (rank, (section, _)) in present.iter().enumerate().skip(1) {
            edge_view.push_str(&format!(" UNION ALL SELECT {rank}, * FROM {}.edge_index", section.name()));
            node_view.push_str(&format!(" UNION ALL SELECT {rank}, * FROM {}.node", section.name()));
        }
        let mut conns = Vec::with_capacity(workers);
        for _ in 0..workers {
            let conn = open_read_only(&present[0].1)?;
            for (section, path) in present.iter().skip(1) {
                let sql = format!("ATTACH DATABASE ?1 AS {}", section.name());
                conn.execute(&sql, [path.to_string_lossy().as_ref()])?;
            }
            // The wall (spec §11): every attached file's own user_version.
            for (section, _) in &present {
                let schema = if *section == Section::Core { "main".to_string() } else { section.name().to_string() };
                let v: u32 = conn.query_row(&format!("PRAGMA {schema}.user_version"), [], |r| r.get(0))?;
                if v != SCHEMA_VERSION {
                    return Err(SqliteError(format!(
                        "section {} user_version {v} unsupported (this build understands {SCHEMA_VERSION})",
                        section.name()
                    )));
                }
            }
            // `open_read_only` already set `query_only = ON`, which also
            // refuses TEMP objects; lift it just long enough to build the two
            // views (the file itself stays read-only through the open flag).
            conn.execute_batch(&format!(
                "PRAGMA query_only = OFF; PRAGMA mmap_size = {mmap_bytes}; {edge_view}; {node_view}; PRAGMA query_only = ON;"
            ))?;
            conns.push(Mutex::new(conn));
        }
        // DB-4a: the version IS the manifest root (spec 3.4) -- the same
        // number `MemStore::publish` stamps from the in-memory graph.
        let version = ContentHash::from_hex(&manifest.root)
            .map(GraphVersion)
            .ok_or_else(|| SqliteError(format!("manifest root {} is not a ContentHash", manifest.root)))?;
        Ok(SqliteSnapshot {
            conns,
            next: AtomicUsize::new(0),
            version,
            present: present.into_iter().map(|(s, _)| s).collect(),
            absent,
            manifest,
            mmap_bytes,
        })
    }

    pub fn present(&self) -> &[Section] {
        &self.present
    }

    pub fn absent(&self) -> &[Section] {
        &self.absent
    }

    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    pub fn workers(&self) -> usize {
        self.conns.len()
    }

    pub fn mmap_bytes(&self) -> u64 {
        self.mmap_bytes
    }

    /// Runs `f` on the first free worker connection (round-robin start,
    /// `try_lock` across all, then a blocking lock on the start slot).
    /// `pub` so the serving companions (`sqlite::serve`, `sidecars::unfold`)
    /// load over the same connections.
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, SqliteError>) -> Result<T, SqliteError> {
        let n = self.conns.len();
        let start = self.next.fetch_add(1, Ordering::Relaxed) % n;
        for i in 0..n {
            if let Ok(guard) = self.conns[(start + i) % n].try_lock() {
                return f(&guard);
            }
        }
        let guard = self.conns[start].lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    fn kind_of(rel: i64, dir: i64) -> Option<EdgeKind> {
        match (rel_of_code(rel)?, dir) {
            (EdgeRel::Directed(r), DIR_FORWARD) => Some(EdgeKind::Directed(r, Direction::Forward)),
            (EdgeRel::Directed(r), DIR_INVERSE) => Some(EdgeKind::Directed(r, Direction::Inverse)),
            (EdgeRel::Symmetric(s), DIR_SYMMETRIC) => Some(EdgeKind::Symmetric(s)),
            _ => None,
        }
    }

    fn code_of(kind: EdgeKind) -> (i64, i64, String) {
        match kind {
            EdgeKind::Directed(r, d) => (
                rel_code_of(EdgeRel::Directed(r)),
                if d == Direction::Forward { DIR_FORWARD } else { DIR_INVERSE },
                format!("{r:?}"),
            ),
            EdgeKind::Symmetric(s) => (rel_code_of(EdgeRel::Symmetric(s)), DIR_SYMMETRIC, format!("{s:?}")),
        }
    }

    /// Paging is keyset, not OFFSET: `ord` IS the entry's position in the
    /// in-memory `(subject, rel, dir)` list (spec §5.2), and those
    /// positions are contiguous `0..n` across the attached sections, so
    /// `ord >= cursor LIMIT limit` reads exactly the rows `skip(cursor).
    /// take(limit)` reads -- O(limit) per page instead of O(cursor), and no
    /// `COUNT(*)`: one extra row (`LIMIT limit + 1`) decides `next` --
    /// `start + entries.len() < total` holds iff a row beyond the page
    /// exists (`explore.rs`'s own rule, `limit = 0` included).
    fn edges_inner(&self, p: &Position, q: &EdgeQuery) -> Result<EdgePage, SqliteError> {
        let (rel, dir, rel_name) = Self::code_of(q.kind);
        let subject = position_str(p);
        let start = q.cursor.unwrap_or(0);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT object, edge_id, meta_kind, meta_narrative, meta_votes FROM all_edge_index                  WHERE subject = ?1 AND rel = ?2 AND dir = ?3 AND ord >= ?4 ORDER BY ord LIMIT ?5",
            )?;
            let mut rows = stmt.query(rusqlite::params![subject, rel, dir, start as i64, q.limit as i64 + 1])?;
            let mut entries = Vec::new();
            let mut more = false;
            while let Some(row) = rows.next()? {
                if entries.len() == q.limit {
                    more = true;
                    break;
                }
                let object: String = row.get(0)?;
                let blob: Vec<u8> = row.get(1)?;
                let meta_kind: i64 = row.get(2)?;
                let narrative: Option<String> = row.get(3)?;
                let votes: Option<i64> = row.get(4)?;
                let meta = match (meta_kind, narrative, votes) {
                    (0, None, None) => EdgeMeta::None,
                    (1, Some(n), None) => EdgeMeta::Narrative(NarrativeId::new(n)),
                    (2, None, Some(v)) => EdgeMeta::Votes(
                        u32::try_from(v).map_err(|_| SqliteError(format!("meta_votes {v} out of range")))?,
                    ),
                    (k, n, v) => return Err(SqliteError(format!("edge meta ({k}, {n:?}, {v:?}) is malformed"))),
                };
                entries.push(EdgeEntry {
                    edge: EdgeId(format!("{rel_name}:{}", hash_from_bytes(&blob)?.hex())),
                    node: atlas_graph_types::canon::ids::parse_position(&object, "edge_index.object")?,
                    meta,
                });
            }
            let next = if more { Some(start + entries.len()) } else { None };
            Ok(EdgePage { kind: q.kind, entries, next })
        })
    }
}

impl GraphQuery for SqliteSnapshot {
    fn node(&self, id: &AnyNodeId) -> Option<Node> {
        let key = any_node_id_str(id);
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .prepare_cached("SELECT payload FROM all_node WHERE id = ?1")?
                .query_row([key.as_str()], |r| r.get(0))
                .optional()?;
            Ok(match payload {
                Some(bytes) => Some(Node::decode(&bytes)?),
                None => None,
            })
        })
        .unwrap_or(None)
    }

    fn derive(&self, pid: &Pid) -> Option<Vec<u8>> {
        let key = hash_bytes(&pid.hash);
        self.with_conn(|conn| {
            let payload: Option<Vec<u8>> = conn
                .prepare_cached("SELECT payload FROM all_node WHERE pid = ?1")?
                .query_row([key.as_slice()], |r| r.get(0))
                .optional()?;
            Ok(match payload {
                Some(bytes) => {
                    let node = Node::decode(&bytes)?;
                    // `PositionKind` is not a column: re-derive and compare,
                    // so a pid of another kind with the same hash is refused.
                    (node.pid() == *pid).then(|| node.canonical_bytes())
                }
                None => None,
            })
        })
        .unwrap_or(None)
    }

    fn edge_summary(&self, p: &Position) -> EdgeSummary {
        let subject = position_str(p);
        self.with_conn(|conn| {
            let mut stmt =
                conn.prepare_cached("SELECT rel, dir, COUNT(*) FROM all_edge_index WHERE subject = ?1 GROUP BY rel, dir")?;
            let mut rows = stmt.query([subject.as_str()])?;
            let mut out = EdgeSummary::new();
            while let Some(row) = rows.next()? {
                let rel: i64 = row.get(0)?;
                let dir: i64 = row.get(1)?;
                let n: i64 = row.get(2)?;
                let kind = Self::kind_of(rel, dir)
                    .ok_or_else(|| SqliteError(format!("edge_index (rel {rel}, dir {dir}) names no EdgeKind")))?;
                if n > 0 {
                    out.insert(kind, n as usize);
                }
            }
            Ok(out)
        })
        .unwrap_or_default()
    }

    fn edges(&self, p: &Position, q: &EdgeQuery) -> EdgePage {
        self.edges_inner(p, q).unwrap_or_else(|_| EdgePage { kind: q.kind, entries: Vec::new(), next: None })
    }

    fn reading_window(&self, corpus: &'static str, start: usize, n: usize) -> Vec<AnyNodeId> {
        let section = match corpus {
            "bible" => Section::Kjv,
            "concord" => Section::Concord,
            _ => return Vec::new(),
        };
        if !self.present.contains(&section) {
            return Vec::new();
        }
        let schema = if section == Section::Core { "main" } else { section.name() };
        self.with_conn(|conn| {
            let mut stmt = conn.prepare_cached(&format!(
                "SELECT node_id FROM {schema}.reading_spine WHERE ord >= ?1 ORDER BY ord LIMIT ?2"
            ))?;
            let mut rows = stmt.query(rusqlite::params![start as i64, n as i64])?;
            let mut out = Vec::new();
            while let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                out.push(parse_any_node_id(&id, "reading_spine.node_id")?);
            }
            Ok(out)
        })
        .unwrap_or_default()
    }

    // ---- DB-3 (spec 4): the three overrides the section indexes answer
    // directly; `nodes` and `edges_with_nodes` keep the trait's
    // compositions (plan judgment call 2).
    fn nodes_of_kind(&self, kind: NodeKind, cursor: Option<usize>, limit: usize) -> NodePage {
        let start = cursor.unwrap_or(0);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare_cached("SELECT id FROM all_node WHERE kind = ?1 ORDER BY id LIMIT ?2 OFFSET ?3")?;
            let mut rows = stmt.query(rusqlite::params![node_kind_ordinal(kind), limit.saturating_add(1) as i64, start as i64])?;
            let mut ids: Vec<AnyNodeId> = Vec::new();
            let mut more = false;
            while let Some(row) = rows.next()? {
                if ids.len() == limit {
                    more = true;
                    break;
                }
                let id: String = row.get(0)?;
                ids.push(parse_any_node_id(&id, "node.id")?);
            }
            let next = if more { Some(start + ids.len()) } else { None };
            Ok(NodePage { ids, next })
        })
        .unwrap_or(NodePage { ids: Vec::new(), next: None })
    }

    fn row_provenance(&self, e: &EdgeId) -> Option<RowRef> {
        self.rows_behind(e).into_iter().next()
    }

    /// Every DISTINCT `(row_family, row_id)` behind the id across the
    /// attached sections, in `(family, id)` order -- the same order the
    /// in-memory `edge_rows` keeps, so the first is the same first.
    fn rows_behind(&self, e: &EdgeId) -> Vec<RowRef> {
        let Ok(blob) = super::writer::edge_id_blob(e) else { return Vec::new() };
        let justified = directed_rel_code(atlas_graph_types::edge::RelationId::JustifiedBy);
        self.with_conn(|conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT DISTINCT sec, row_family, row_id FROM all_edge_index WHERE edge_id = ?1 AND rel != ?2 ORDER BY row_family, row_id",
            )?;
            let hits: Vec<(i64, i64, i64)> = stmt
                .query_map(rusqlite::params![blob, justified], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
            let mut out = Vec::with_capacity(hits.len());
            for (sec, family_ord, row_id) in hits {
                let family = u8::try_from(family_ord).ok().and_then(atlas_graph_types::canon::RowFamily::from_ordinal)
                    .ok_or_else(|| SqliteError(format!("edge_index.row_family {family_ord} is not a RowFamily")))?;
                let schema = usize::try_from(sec).ok().and_then(|r| self.schema_of(r))
                    .ok_or_else(|| SqliteError(format!("edge_index.sec {sec} names no attached section")))?;
                let provenance: String = conn
                    .prepare_cached(&format!("SELECT provenance FROM {schema}.{} WHERE id = ?1", family.name()))?
                    .query_row([row_id], |r| r.get(0))?;
                out.push(RowRef { family, row_id: row_id as u64, provenance });
            }
            Ok(out)
        })
        .unwrap_or_default()
    }

    fn position_of(&self, corpus: &'static str, id: &AnyNodeId) -> Option<usize> {
        let section = match corpus {
            "bible" => Section::Kjv,
            "concord" => Section::Concord,
            _ => return None,
        };
        if !self.present.contains(&section) {
            return None;
        }
        let key = any_node_id_str(id);
        self.with_conn(|conn| {
            let ord: Option<i64> = conn
                .prepare_cached(&format!("SELECT ord FROM {}.reading_spine WHERE node_id = ?1", section.name()))?
                .query_row([key.as_str()], |r| r.get(0))
                .optional()?;
            Ok(ord.and_then(|o| usize::try_from(o).ok()))
        })
        .unwrap_or(None)
    }
}

impl SqliteSnapshot {
    /// The schema name a section is attached under (`main` for core).
    fn schema_of(&self, rank: usize) -> Option<&'static str> {
        let s = *self.present.get(rank)?;
        Some(if s == Section::Core { "main" } else { s.name() })
    }
}

impl GraphSnapshot for SqliteSnapshot {
    fn version(&self) -> GraphVersion {
        self.version
    }
}
