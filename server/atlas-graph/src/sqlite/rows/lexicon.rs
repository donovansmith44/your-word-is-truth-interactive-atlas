//! LEX-1: the `lexicon` section's one row family, `occurs` (spec §5.7):
//! `entry_id`, the seven LOCUS columns (layer/start/end NOT NULL -- a word
//! locus always carries its one-token span), `provenance`. Imported, no
//! justification column.

use atlas_graph_types::edge::Occurs;
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::super::columns::{col, read_text_locus, text_locus_values};
use super::super::SqliteError;
use super::{id_col, insert, read_all, text, D};

const COLS_OCCURS: &str = "entry_id, \
 locus_corpus, locus_a, locus_b, locus_c, locus_layer, locus_start, locus_end, \
 provenance";

pub fn insert_occurs(tx: &Transaction, ord: i64, row: &Occurs) -> Result<(), SqliteError> {
    match &row.locus.span {
        Some(s) if s.start == s.end => {}
        Some(s) => return Err(SqliteError(format!("occurs ord {ord}: span {}..{} is not one token", s.start, s.end))),
        None => return Err(SqliteError(format!("occurs ord {ord}: a word locus must carry its token span"))),
    }
    let mut v: Vec<Value> = Vec::with_capacity(9);
    v.push(text(&row.entry.0));
    v.extend(text_locus_values(&row.locus));
    v.push(text(&row.provenance));
    insert(tx, "occurs", COLS_OCCURS, ord, v)
}

pub fn read_occurs(conn: &Connection) -> Result<Vec<(i64, Occurs)>, SqliteError> {
    read_all(conn, "occurs", COLS_OCCURS, |row| {
        Ok(Occurs { entry: id_col(row, D, "entry_id")?, locus: read_text_locus(row, D + 1)?, provenance: col(row, D + 8, "provenance")? })
    })
}
