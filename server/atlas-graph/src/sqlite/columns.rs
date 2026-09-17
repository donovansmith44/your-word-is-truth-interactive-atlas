//! DB-2b: the LOCUS / RANGE / AUTHORED column codec (spec §5.0). One
//! spelling of "seven columns per locus" for every row writer and reader,
//! and the `justification` + `ground` side tables behind every AUTHORED
//! family's `justification_id`.
//!
//! Readers are strict: a corpus that is not the one the column typed, an
//! integer outside the ref's width, or a half-present TokenSpan is an
//! error naming the column -- never a silently coerced value.

use std::collections::BTreeSet;

use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{AnchorId, SourceId};
use atlas_graph_types::text::{
    BibleTag, ConcordRef, ConcordTag, Corpus, Locus, LocusRange, TextLocus, TextRef, TokenSpan,
    TranslationId, VerseRef,
};
use rusqlite::types::Value;
use rusqlite::{Connection, Row, Transaction};

use super::SqliteError;

/// The seven LOCUS columns, in spec order, under a prefix:
/// `{p}_corpus, {p}_a, {p}_b, {p}_c, {p}_layer, {p}_start, {p}_end`.
pub fn locus_columns(prefix: &str) -> String {
    format!("{prefix}_corpus, {prefix}_a, {prefix}_b, {prefix}_c, {prefix}_layer, {prefix}_start, {prefix}_end")
}

/// Seven `?` placeholders, for an INSERT's VALUES list.
pub fn locus_placeholders() -> &'static str {
    "?, ?, ?, ?, ?, ?, ?"
}

fn span_values(span: &Option<TokenSpan>) -> [Value; 3] {
    match span {
        Some(s) => [
            Value::Text(s.layer.0.clone()),
            Value::Integer(i64::from(s.start)),
            Value::Integer(i64::from(s.end)),
        ],
        None => [Value::Null, Value::Null, Value::Null],
    }
}

fn locus_values(corpus: &str, a: i64, b: i64, c: i64, span: &Option<TokenSpan>) -> [Value; 7] {
    let [l, s, e] = span_values(span);
    [Value::Text(corpus.to_string()), Value::Integer(a), Value::Integer(b), Value::Integer(c), l, s, e]
}

/// `("bible", book, chapter, verse, layer?, start?, end?)`.
pub fn bible_locus_values(l: &Locus<BibleTag>) -> [Value; 7] {
    locus_values(BibleTag::ID, i64::from(l.unit.book), i64::from(l.unit.chapter), i64::from(l.unit.verse), &l.span)
}

/// `("concord", part, article, paragraph, layer?, start?, end?)`.
pub fn concord_locus_values(l: &Locus<ConcordTag>) -> [Value; 7] {
    locus_values(
        ConcordTag::ID,
        i64::from(l.unit.part),
        i64::from(l.unit.article),
        i64::from(l.unit.paragraph),
        &l.span,
    )
}

/// Corpus from the `TextRef` arm.
pub fn text_locus_values(l: &TextLocus) -> [Value; 7] {
    match &l.at {
        TextRef::Bible(v) => {
            locus_values(BibleTag::ID, i64::from(v.book), i64::from(v.chapter), i64::from(v.verse), &l.span)
        }
        TextRef::Concord(c) => locus_values(
            ConcordTag::ID,
            i64::from(c.part),
            i64::from(c.article),
            i64::from(c.paragraph),
            &l.span,
        ),
    }
}

/// All seven NULL when `None`.
pub fn opt_text_locus_values(l: &Option<TextLocus>) -> [Value; 7] {
    match l {
        Some(l) => text_locus_values(l),
        None => [Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null, Value::Null],
    }
}

/// The 14 RANGE values: `from` at 0..7, `to` at 7..14.
pub fn bible_range_values(r: &LocusRange<BibleTag>) -> [Value; 14] {
    let f = bible_locus_values(&r.from);
    let t = bible_locus_values(&r.to);
    let mut out: [Value; 14] = std::array::from_fn(|_| Value::Null);
    for (i, v) in f.into_iter().chain(t).enumerate() {
        out[i] = v;
    }
    out
}

// ---------------------------------------------------------------------
// Readers
// ---------------------------------------------------------------------

fn col<T: rusqlite::types::FromSql>(row: &Row, i: usize, what: &str) -> Result<T, SqliteError> {
    row.get::<_, T>(i).map_err(|e| SqliteError(format!("column {i} ({what}): {e}")))
}

fn int_as<T: TryFrom<i64>>(row: &Row, i: usize, what: &str) -> Result<T, SqliteError> {
    let v: i64 = col(row, i, what)?;
    T::try_from(v).map_err(|_| SqliteError(format!("column {i} ({what}): {v} is out of range")))
}

/// The optional TokenSpan at columns `i..i+3`: all NULL or all present.
fn read_span(row: &Row, i: usize) -> Result<Option<TokenSpan>, SqliteError> {
    let layer: Option<String> = col(row, i, "layer")?;
    let start: Option<i64> = col(row, i + 1, "start")?;
    let end: Option<i64> = col(row, i + 2, "end")?;
    match (layer, start, end) {
        (None, None, None) => Ok(None),
        (Some(layer), Some(start), Some(end)) => {
            let start = u16::try_from(start)
                .map_err(|_| SqliteError(format!("column {} (start): {start} is out of range", i + 1)))?;
            let end = u16::try_from(end)
                .map_err(|_| SqliteError(format!("column {} (end): {end} is out of range", i + 2)))?;
            TokenSpan::new(TranslationId(layer), start, end)
                .map(Some)
                .map_err(|e| SqliteError(format!("column {i} (span): {e:?}")))
        }
        _ => Err(SqliteError(format!("column {i} (span): half-present TokenSpan (layer/start/end must all be NULL or all set)"))),
    }
}

fn expect_corpus(row: &Row, i: usize, want: &str) -> Result<(), SqliteError> {
    let got: String = col(row, i, "corpus")?;
    if got == want {
        Ok(())
    } else {
        Err(SqliteError(format!("column {i} (corpus): expected {want}, found {got}")))
    }
}

fn read_verse_ref(row: &Row, i: usize) -> Result<VerseRef, SqliteError> {
    Ok(VerseRef { book: int_as(row, i, "book")?, chapter: int_as(row, i + 1, "chapter")?, verse: int_as(row, i + 2, "verse")? })
}

fn read_concord_ref(row: &Row, i: usize) -> Result<ConcordRef, SqliteError> {
    Ok(ConcordRef {
        part: int_as(row, i, "part")?,
        article: int_as(row, i + 1, "article")?,
        paragraph: int_as(row, i + 2, "paragraph")?,
    })
}

/// Seven columns from `i`; refuses `corpus != "bible"`.
pub fn read_bible_locus(row: &Row, i: usize) -> Result<Locus<BibleTag>, SqliteError> {
    expect_corpus(row, i, BibleTag::ID)?;
    Ok(Locus { unit: read_verse_ref(row, i + 1)?, span: read_span(row, i + 4)? })
}

/// Seven columns from `i`; refuses `corpus != "concord"`.
pub fn read_concord_locus(row: &Row, i: usize) -> Result<Locus<ConcordTag>, SqliteError> {
    expect_corpus(row, i, ConcordTag::ID)?;
    Ok(Locus { unit: read_concord_ref(row, i + 1)?, span: read_span(row, i + 4)? })
}

/// Seven columns from `i`; the corpus column picks the `TextRef` arm.
pub fn read_text_locus(row: &Row, i: usize) -> Result<TextLocus, SqliteError> {
    let corpus: String = col(row, i, "corpus")?;
    let at = match corpus.as_str() {
        c if c == BibleTag::ID => TextRef::Bible(read_verse_ref(row, i + 1)?),
        c if c == ConcordTag::ID => TextRef::Concord(read_concord_ref(row, i + 1)?),
        other => return Err(SqliteError(format!("column {i} (corpus): unknown corpus {other}"))),
    };
    Ok(TextLocus { at, span: read_span(row, i + 4)? })
}

/// `None` iff the corpus column is NULL.
pub fn read_opt_text_locus(row: &Row, i: usize) -> Result<Option<TextLocus>, SqliteError> {
    let corpus: Option<String> = col(row, i, "corpus")?;
    match corpus {
        None => Ok(None),
        Some(_) => read_text_locus(row, i).map(Some),
    }
}

/// 14 columns: `from` at `i`, `to` at `i + 7`; `LocusRange::new` is fallible.
pub fn read_bible_range(row: &Row, i: usize) -> Result<LocusRange<BibleTag>, SqliteError> {
    let from = read_bible_locus(row, i)?;
    let to = read_bible_locus(row, i + 7)?;
    LocusRange::new(from, to).map_err(|e| SqliteError(format!("column {i} (range): {e:?}")))
}

// ---------------------------------------------------------------------
// AUTHORED: justification + ground
// ---------------------------------------------------------------------

pub const GROUND_SCRIPTURE: i64 = 0;
pub const GROUND_ANCHOR: i64 = 1;
pub const GROUND_SOURCE: i64 = 2;

/// Hands out `justification.id`s (from 1, per section) and writes the
/// `justification` + `ground` rows behind an AUTHORED column.
pub struct JustificationWriter {
    next_id: i64,
}

impl Default for JustificationWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl JustificationWriter {
    pub fn new() -> Self {
        JustificationWriter { next_id: 1 }
    }

    /// Returns the id to bind, or `None` (bind NULL) when the justification
    /// is empty: spec says NULL when the family has no justification, and
    /// -- ruled in the plan -- an authored row's EMPTY justification binds
    /// NULL too, since `Justification::default()` round-trips through it.
    pub fn write(&mut self, tx: &Transaction, j: &Justification) -> Result<Option<i64>, SqliteError> {
        if j.text.is_none() && j.grounds.is_empty() {
            return Ok(None);
        }
        let id = self.next_id;
        self.next_id += 1;
        tx.execute("INSERT INTO justification (id, text) VALUES (?, ?)", rusqlite::params![id, j.text])?;
        let mut stmt = tx.prepare_cached(
            "INSERT INTO ground (justification_id, ord, kind, \
             scr_from_corpus, scr_from_a, scr_from_b, scr_from_c, scr_from_layer, scr_from_start, scr_from_end, \
             scr_to_corpus, scr_to_a, scr_to_b, scr_to_c, scr_to_layer, scr_to_start, scr_to_end, \
             anchor_id, source_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )?;
        for (ord, g) in j.grounds.iter().enumerate() {
            let mut vals: Vec<Value> = vec![Value::Integer(id), Value::Integer(ord as i64)];
            match g {
                Ground::Scripture(range) => {
                    vals.push(Value::Integer(GROUND_SCRIPTURE));
                    vals.extend(bible_range_values(range));
                    vals.push(Value::Null);
                    vals.push(Value::Null);
                }
                Ground::Anchor(a) => {
                    vals.push(Value::Integer(GROUND_ANCHOR));
                    vals.extend(std::iter::repeat_n(Value::Null, 14));
                    vals.push(Value::Text(a.0.clone()));
                    vals.push(Value::Null);
                }
                Ground::Source(s) => {
                    vals.push(Value::Integer(GROUND_SOURCE));
                    vals.extend(std::iter::repeat_n(Value::Null, 14));
                    vals.push(Value::Null);
                    vals.push(Value::Text(s.0.clone()));
                }
            }
            stmt.execute(rusqlite::params_from_iter(vals.iter()))?;
        }
        Ok(Some(id))
    }
}

/// `None` -> `Justification::default()`; `Some(id)` -> text + grounds in
/// stored `ord` order, which must equal the `BTreeSet` order they are
/// read back into (the writer wrote set order; a drift in `Ground: Ord`
/// is refused rather than silently re-sorted).
pub fn read_justification(conn: &Connection, id: Option<i64>) -> Result<Justification, SqliteError> {
    let Some(id) = id else { return Ok(Justification::default()) };
    let text: Option<String> = conn
        .query_row("SELECT text FROM justification WHERE id = ?", [id], |r| r.get(0))
        .map_err(|e| SqliteError(format!("justification {id}: {e}")))?;
    let mut stmt = conn.prepare_cached(
        "SELECT kind, \
         scr_from_corpus, scr_from_a, scr_from_b, scr_from_c, scr_from_layer, scr_from_start, scr_from_end, \
         scr_to_corpus, scr_to_a, scr_to_b, scr_to_c, scr_to_layer, scr_to_start, scr_to_end, \
         anchor_id, source_id \
         FROM ground WHERE justification_id = ? ORDER BY ord",
    )?;
    let mut in_order: Vec<Ground> = Vec::new();
    let mut rows = stmt.query([id])?;
    while let Some(row) = rows.next()? {
        let kind: i64 = col(row, 0, "ground.kind")?;
        let g = match kind {
            GROUND_SCRIPTURE => Ground::Scripture(read_bible_range(row, 1)?),
            GROUND_ANCHOR => Ground::Anchor(AnchorId::new(col::<String>(row, 15, "ground.anchor_id")?)),
            GROUND_SOURCE => Ground::Source(SourceId::new(col::<String>(row, 16, "ground.source_id")?)),
            other => return Err(SqliteError(format!("ground.kind {other} is not 0|1|2"))),
        };
        in_order.push(g);
    }
    let grounds: BTreeSet<Ground> = in_order.iter().cloned().collect();
    let as_set: Vec<&Ground> = grounds.iter().collect();
    if as_set.len() != in_order.len() || as_set.iter().zip(&in_order).any(|(a, b)| *a != b) {
        return Err(SqliteError(format!("justification {id}: stored ground order is not BTreeSet order")));
    }
    Ok(Justification { text, grounds })
}
