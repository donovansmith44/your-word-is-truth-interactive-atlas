//! DB-2b: the SQLite section artifact (spec §2, §5, §6) — writer,
//! logical dump, manifest and the `SqliteSnapshot` read port. Everything
//! SQLite in the workspace lives under this module; `graph-types` never
//! sees it (zero-dep covenant, spec §1.2).
pub mod columns;
pub mod ddl;
// pub mod logical;   // Task 6
// pub mod manifest;  // Task 5
// pub mod partition; // Task 5
pub mod rows;
// pub mod snapshot;  // Task 7
// pub mod writer;    // Task 5

use std::path::Path;

use atlas_graph_types::id::ContentHash;
use rusqlite::{Connection, OpenFlags};

/// Spec §5.0: every section's `PRAGMA user_version`.
pub const SCHEMA_VERSION: u32 = 14;
/// Spec §5.0: 'BLGA'.
pub const APPLICATION_ID: u32 = 0x424C_4741;
/// Bytes per hash column (`node.pid`, `edge_index.edge_id`): the current
/// `ContentHash` width — 8 while `canon-ids` is off, 16 once it is on.
pub const HASH_WIDTH: usize = std::mem::size_of::<ContentHash>();

#[derive(Debug)]
pub struct SqliteError(pub String);

impl std::fmt::Display for SqliteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SqliteError {}
impl From<rusqlite::Error> for SqliteError {
    fn from(e: rusqlite::Error) -> Self {
        SqliteError(format!("sqlite: {e}"))
    }
}
impl From<atlas_graph_types::canon::CanonError> for SqliteError {
    fn from(e: atlas_graph_types::canon::CanonError) -> Self {
        SqliteError(format!("canon: {e}"))
    }
}
impl From<std::io::Error> for SqliteError {
    fn from(e: std::io::Error) -> Self {
        SqliteError(format!("io: {e}"))
    }
}

// The hash width is graph-types' feature, which this crate cannot `cfg`
// on. `ContentHash::hex()` exists in both states and is width-honest
// (16 hex chars OFF, 32 ON), so the BLOB is the hex decoded to bytes —
// one path, no cfg. OFF: `hex()` is `{:016x}` of the u64, so the 8 bytes
// ARE the big-endian u64; ON: the 16 bytes themselves.
pub fn hash_bytes(h: &ContentHash) -> Vec<u8> {
    let hex = h.hex(); // 16 or 32 lowercase hex chars
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("ContentHash::hex is hex"))
        .collect()
}

pub fn hash_from_bytes(b: &[u8]) -> Result<ContentHash, SqliteError> {
    if b.len() != HASH_WIDTH {
        return Err(SqliteError(format!(
            "hash blob is {} bytes, expected {HASH_WIDTH}",
            b.len()
        )));
    }
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    ContentHash::from_hex(&hex)
        .ok_or_else(|| SqliteError(format!("hash blob {hex} is not a ContentHash")))
}

pub fn open_read_only(path: &Path) -> Result<Connection, SqliteError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conn)
}

/// Spec §5.0 pragmas. Run BEFORE any table exists — `page_size` and
/// `encoding` only take effect on an empty file.
pub fn stamp_pragmas(conn: &Connection) -> Result<(), SqliteError> {
    conn.execute_batch(&format!(
        "PRAGMA page_size = 4096; PRAGMA encoding = 'UTF-8'; PRAGMA journal_mode = OFF; \
         PRAGMA synchronous = OFF; PRAGMA user_version = {SCHEMA_VERSION}; \
         PRAGMA application_id = {APPLICATION_ID};"
    ))?;
    Ok(())
}
