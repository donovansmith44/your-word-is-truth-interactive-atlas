//! DB-2b: the row families of the `kjv` section (spec §5.4) other than
//! `contains_bible`, which is `core::insert_contains`/`read_contains`
//! over the same DDL.

use atlas_graph_types::edge::{CanonSuccession, CrossRef, SpokenAt, SpokenBy};
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::super::columns::{
    bible_range_values, col, opt_text_locus_values, read_bible_range, read_opt_text_locus,
    read_text_locus, text_locus_values, JustificationWriter,
};
use super::super::SqliteError;
use super::{authored, id_col, insert, int, read_all, read_justification_at, text, D};

// ------------------------------------------------------ canon_succession

const COLS_CANON_SUCCESSION: &str = "prior_id, next_id, provenance, justification_id";

pub fn insert_canon_succession(
    tx: &Transaction,
    jw: &mut JustificationWriter,
    ord: i64,
    row: &CanonSuccession,
) -> Result<(), SqliteError> {
    let j = authored(tx, jw, &row.justification)?;
    insert(tx, "canon_succession", COLS_CANON_SUCCESSION, ord, vec![text(&row.prior.0), text(&row.next.0), text(&row.provenance), j])
}

pub fn read_canon_succession(conn: &Connection) -> Result<Vec<(i64, CanonSuccession)>, SqliteError> {
    read_all(conn, "canon_succession", COLS_CANON_SUCCESSION, |row| {
        Ok(CanonSuccession {
            prior: id_col(row, D, "prior_id")?,
            next: id_col(row, D + 1, "next_id")?,
            provenance: col(row, D + 2, "provenance")?,
            justification: read_justification_at(conn, row, D + 3)?,
        })
    })
}

// ------------------------------------------------------------ cross_refs

const COLS_CROSS_REFS: &str = "\
 from_corpus, from_a, from_b, from_c, from_layer, from_start, from_end, \
 to_corpus, to_a, to_b, to_c, to_layer, to_start, to_end, \
 to_last_corpus, to_last_a, to_last_b, to_last_c, to_last_layer, to_last_start, to_last_end, \
 target_display, votes, provenance";

pub fn insert_cross_refs(tx: &Transaction, ord: i64, row: &CrossRef) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(24);
    v.extend(text_locus_values(&row.from));
    v.extend(text_locus_values(&row.to));
    v.extend(opt_text_locus_values(&row.to_last));
    v.push(text(&row.target_display));
    v.push(int(i64::from(row.votes)));
    v.push(text(&row.provenance));
    insert(tx, "cross_refs", COLS_CROSS_REFS, ord, v)
}

pub fn read_cross_refs(conn: &Connection) -> Result<Vec<(i64, CrossRef)>, SqliteError> {
    read_all(conn, "cross_refs", COLS_CROSS_REFS, |row| {
        let votes: i64 = col(row, D + 22, "votes")?;
        Ok(CrossRef {
            from: read_text_locus(row, D)?,
            to: read_text_locus(row, D + 7)?,
            to_last: read_opt_text_locus(row, D + 14)?,
            target_display: col(row, D + 21, "target_display")?,
            votes: u32::try_from(votes).map_err(|_| SqliteError(format!("cross_refs votes {votes} out of range")))?,
            provenance: col(row, D + 23, "provenance")?,
        })
    })
}

// ------------------------------------------------------------- spoken_by

const COLS_SPOKEN_BY: &str = "\
 locus_from_corpus, locus_from_a, locus_from_b, locus_from_c, locus_from_layer, locus_from_start, locus_from_end, \
 locus_to_corpus, locus_to_a, locus_to_b, locus_to_c, locus_to_layer, locus_to_start, locus_to_end, \
 speaker_id, provenance, justification_id";

pub fn insert_spoken_by(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &SpokenBy) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(17);
    v.extend(bible_range_values(&row.locus));
    v.push(text(&row.speaker.0));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "spoken_by", COLS_SPOKEN_BY, ord, v)
}

pub fn read_spoken_by(conn: &Connection) -> Result<Vec<(i64, SpokenBy)>, SqliteError> {
    read_all(conn, "spoken_by", COLS_SPOKEN_BY, |row| {
        Ok(SpokenBy {
            locus: read_bible_range(row, D)?,
            speaker: id_col(row, D + 14, "speaker_id")?,
            provenance: col(row, D + 15, "provenance")?,
            justification: read_justification_at(conn, row, D + 16)?,
        })
    })
}

// ------------------------------------------------------------- spoken_at

const COLS_SPOKEN_AT: &str = "\
 locus_from_corpus, locus_from_a, locus_from_b, locus_from_c, locus_from_layer, locus_from_start, locus_from_end, \
 locus_to_corpus, locus_to_a, locus_to_b, locus_to_c, locus_to_layer, locus_to_start, locus_to_end, \
 place_id, provenance, justification_id";

pub fn insert_spoken_at(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &SpokenAt) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(17);
    v.extend(bible_range_values(&row.locus));
    v.push(text(&row.place.0));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "spoken_at", COLS_SPOKEN_AT, ord, v)
}

pub fn read_spoken_at(conn: &Connection) -> Result<Vec<(i64, SpokenAt)>, SqliteError> {
    read_all(conn, "spoken_at", COLS_SPOKEN_AT, |row| {
        Ok(SpokenAt {
            locus: read_bible_range(row, D)?,
            place: id_col(row, D + 14, "place_id")?,
            provenance: col(row, D + 15, "provenance")?,
            justification: read_justification_at(conn, row, D + 16)?,
        })
    })
}
