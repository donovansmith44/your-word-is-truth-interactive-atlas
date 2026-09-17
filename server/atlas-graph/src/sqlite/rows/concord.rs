//! DB-2b: the row families of the `concord` section (spec §5.5) other
//! than `contains_concord`, which is `core::insert_contains`/
//! `read_contains` over the same DDL shape.

use atlas_graph_types::edge::{Confesses, Quotes};
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::super::columns::{
    bible_range_values, col, concord_locus_values, read_bible_range, read_concord_locus,
    read_text_locus, text_locus_values, JustificationWriter,
};
use super::super::SqliteError;
use super::{authored, insert, read_all, read_justification_at, text, D};

// ---------------------------------------------------------------- quotes

const COLS_QUOTES: &str = "\
 quoting_corpus, quoting_a, quoting_b, quoting_c, quoting_layer, quoting_start, quoting_end, \
 quoted_from_corpus, quoted_from_a, quoted_from_b, quoted_from_c, quoted_from_layer, quoted_from_start, quoted_from_end, \
 quoted_to_corpus, quoted_to_a, quoted_to_b, quoted_to_c, quoted_to_layer, quoted_to_start, quoted_to_end, \
 provenance";

pub fn insert_quotes(tx: &Transaction, ord: i64, row: &Quotes) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(22);
    v.extend(text_locus_values(&row.quoting));
    v.extend(bible_range_values(&row.quoted));
    v.push(text(&row.provenance));
    insert(tx, "quotes", COLS_QUOTES, ord, v)
}

pub fn read_quotes(conn: &Connection) -> Result<Vec<(i64, Quotes)>, SqliteError> {
    read_all(conn, "quotes", COLS_QUOTES, |row| {
        Ok(Quotes {
            quoting: read_text_locus(row, D)?,
            quoted: read_bible_range(row, D + 7)?,
            provenance: col(row, D + 21, "provenance")?,
        })
    })
}

// ------------------------------------------------------------- confesses

const COLS_CONFESSES: &str = "\
 confessing_corpus, confessing_a, confessing_b, confessing_c, confessing_layer, confessing_start, confessing_end, \
 confessed_from_corpus, confessed_from_a, confessed_from_b, confessed_from_c, confessed_from_layer, confessed_from_start, confessed_from_end, \
 confessed_to_corpus, confessed_to_a, confessed_to_b, confessed_to_c, confessed_to_layer, confessed_to_start, confessed_to_end, \
 provenance, justification_id";

pub fn insert_confesses(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &Confesses) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(23);
    v.extend(concord_locus_values(&row.confessing));
    v.extend(bible_range_values(&row.confessed));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "confesses", COLS_CONFESSES, ord, v)
}

pub fn read_confesses(conn: &Connection) -> Result<Vec<(i64, Confesses)>, SqliteError> {
    read_all(conn, "confesses", COLS_CONFESSES, |row| {
        Ok(Confesses {
            confessing: read_concord_locus(row, D)?,
            confessed: read_bible_range(row, D + 7)?,
            provenance: col(row, D + 21, "provenance")?,
            justification: read_justification_at(conn, row, D + 22)?,
        })
    })
}
