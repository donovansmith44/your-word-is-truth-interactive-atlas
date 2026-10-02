//! The SQLite section artifact: writer, logical dump, manifest and the read port.
pub mod blob;
pub mod columns;
pub mod ddl;
pub mod extras;
pub mod logical;
pub mod manifest;
pub mod partition;
pub mod reload;
pub mod rows;
pub mod serve;
pub mod sidecars;
pub mod snapshot;
pub mod source;
pub mod words;
pub mod writer;

use std::path::Path;

use atlas_graph_types::id::ContentHash;
use rusqlite::{Connection, OpenFlags};

/// Every section file's `PRAGMA user_version`: the manifest's own schema version, so a file and
/// the line that lists it can never disagree.
pub const SCHEMA_VERSION: u32 = atlas_graph_types::sections::SECTION_SCHEMA_VERSION;
/// Every section file's `PRAGMA application_id`: the ASCII bytes `BLGA`.
pub const APPLICATION_ID: u32 = 0x424C_4741;
/// Bytes per hash column (`node.pid`, `edge_index.edge_id`): the current `ContentHash` width -- 8 while
/// `canon-ids` is off, 16 once it is on.
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

// The hash width is a graph-types feature this crate cannot `cfg` on, and `ContentHash::hex`
// is width-honest in both states, so the BLOB is that hex decoded back to bytes: one path,
// no cfg, and the column width follows the hash.
pub fn hash_bytes(h: &ContentHash) -> Vec<u8> {
    let hex = h.hex();
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

/// Run BEFORE any table exists: `page_size` and `encoding` only take effect on an empty file.
pub fn stamp_pragmas(conn: &Connection) -> Result<(), SqliteError> {
    conn.execute_batch(&format!(
        "PRAGMA page_size = 4096; PRAGMA encoding = 'UTF-8'; PRAGMA journal_mode = OFF; \
         PRAGMA synchronous = OFF; PRAGMA user_version = {SCHEMA_VERSION}; \
         PRAGMA application_id = {APPLICATION_ID};"
    ))?;
    Ok(())
}
