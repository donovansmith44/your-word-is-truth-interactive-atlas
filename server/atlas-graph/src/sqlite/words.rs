use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use atlas_graph_types::text::{ConcordRef, VerseRef};
use rusqlite::Connection;

use super::extras::{Col, Extras, TableSpec, CONCORD_TOKEN, KJV_TOKEN};
use super::SqliteError;
use crate::sections::Section;
use crate::tokens::Token;

pub type Words<U> = BTreeMap<U, Vec<Token>>;

pub trait WordedUnit: Ord + Clone + Sized {
    const SECTION: Section;
    const TABLE: &'static TableSpec;
    fn of(key: [i64; 3]) -> Option<Self>;
    fn key(&self) -> [i64; 3];
}

impl WordedUnit for VerseRef {
    const SECTION: Section = Section::Kjv;
    const TABLE: &'static TableSpec = &KJV_TOKEN;
    fn of([book, chapter, verse]: [i64; 3]) -> Option<Self> {
        Some(VerseRef { book: u8::try_from(book).ok()?, chapter: u16::try_from(chapter).ok()?, verse: u16::try_from(verse).ok()? })
    }
    fn key(&self) -> [i64; 3] {
        [i64::from(self.book), i64::from(self.chapter), i64::from(self.verse)]
    }
}

impl WordedUnit for ConcordRef {
    const SECTION: Section = Section::Concord;
    const TABLE: &'static TableSpec = &CONCORD_TOKEN;
    fn of([part, article, paragraph]: [i64; 3]) -> Option<Self> {
        Some(ConcordRef { part: u8::try_from(part).ok()?, article: u16::try_from(article).ok()?, paragraph: u16::try_from(paragraph).ok()? })
    }
    fn key(&self) -> [i64; 3] {
        [i64::from(self.part), i64::from(self.article), i64::from(self.paragraph)]
    }
}

pub fn words_in<U: WordedUnit>(conn: &Connection, units: &RangeInclusive<U>) -> Result<Words<U>, SqliteError> {
    let [a, b, c, ord] = <[&str; 4]>::try_from(U::TABLE.pk).map_err(|_| SqliteError(format!("{} is not keyed by unit and ordinal", U::TABLE.name)))?;
    let sql = format!(
        "SELECT {a}, {b}, {c}, {ord}, char_start, char_end FROM {}.{} WHERE ({a}, {b}, {c}) BETWEEN (?1, ?2, ?3) AND (?4, ?5, ?6) ORDER BY {a}, {b}, {c}, {ord}",
        U::SECTION.name(),
        U::TABLE.name
    );
    let ([f1, f2, f3], [t1, t2, t3]) = (units.start().key(), units.end().key());
    let mut stmt = conn.prepare_cached(&sql)?;
    let rows = stmt.query_map(rusqlite::params![f1, f2, f3, t1, t2, t3], |r| Ok([r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?]))?;
    let mut read = Vec::new();
    for row in rows {
        read.push(row?);
    }
    words_of(read)
}

pub fn compiled_words<U: WordedUnit>(extras: &Extras) -> Result<Words<U>, SqliteError> {
    let Some(table) = extras.table(U::TABLE.name) else { return Ok(Words::new()) };
    let rows = table
        .rows
        .iter()
        .map(|row| match row.as_slice() {
            [Col::Int(a), Col::Int(b), Col::Int(c), Col::Int(ord), Col::Int(start), Col::Int(end)] => Ok([*a, *b, *c, *ord, *start, *end]),
            other => Err(SqliteError(format!("{}: {other:?} is not a unit's word", U::TABLE.name))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    words_of(rows)
}

fn words_of<U: WordedUnit>(rows: Vec<[i64; 6]>) -> Result<Words<U>, SqliteError> {
    let mut words = Words::new();
    for [a, b, c, ord, start, end] in rows {
        let unit = U::of([a, b, c]).ok_or_else(|| SqliteError(format!("{}: ({a}, {b}, {c}) names no unit", U::TABLE.name)))?;
        let word = Token {
            ord: u16::try_from(ord).map_err(|_| SqliteError(format!("{}: word {ord} of ({a}, {b}, {c}) is past a unit's words", U::TABLE.name)))?,
            char_start: usize::try_from(start).map_err(|_| SqliteError(format!("{}: word {ord} of ({a}, {b}, {c}) starts before its text", U::TABLE.name)))?,
            char_end: usize::try_from(end).map_err(|_| SqliteError(format!("{}: word {ord} of ({a}, {b}, {c}) ends before its text", U::TABLE.name)))?,
        };
        words.entry(unit).or_insert_with(Vec::new).push(word);
    }
    Ok(words)
}
