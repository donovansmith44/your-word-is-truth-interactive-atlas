//! DB-2b: the one row family of the `kretzmann` section (spec §5.6).

use atlas_graph_types::edge::CommentsOn;
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::super::columns::{bible_range_values, col, read_bible_range, JustificationWriter};
use super::super::SqliteError;
use super::{authored, id_col, insert, read_all, read_justification_at, text, D};

const COLS_COMMENTS_ON: &str = "item_id, \
 on_from_corpus, on_from_a, on_from_b, on_from_c, on_from_layer, on_from_start, on_from_end, \
 on_to_corpus, on_to_a, on_to_b, on_to_c, on_to_layer, on_to_start, on_to_end, \
 provenance, justification_id";

pub fn insert_comments_on(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &CommentsOn) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(17);
    v.push(text(&row.item.0));
    v.extend(bible_range_values(&row.on));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "comments_on", COLS_COMMENTS_ON, ord, v)
}

pub fn read_comments_on(conn: &Connection) -> Result<Vec<(i64, CommentsOn)>, SqliteError> {
    read_all(conn, "comments_on", COLS_COMMENTS_ON, |row| {
        Ok(CommentsOn {
            item: id_col(row, D, "item_id")?,
            on: read_bible_range(row, D + 1)?,
            provenance: col(row, D + 15, "provenance")?,
            justification: read_justification_at(conn, row, D + 16)?,
        })
    })
}
