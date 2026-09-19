//! DB-2b: one INSERT and one SELECT per row family (spec §5.3–5.6),
//! proven by canon-byte round-trip (`tests/sqlite_laws.rs`). The two
//! sides of every family share ONE column-list constant so they cannot
//! drift; `id` is the global `ord` in this batch (one INTEGER PRIMARY KEY,
//! one UNIQUE(ord), coinciding by construction -- `read_all` asserts it).
//!
//! Ids are the erased raw strings (`x.0`), never `any_node_id_str`: each
//! column is typed by its family (spec §5.0, "node ids are TEXT in their
//! existing string form").

pub mod concord;
pub mod core;
pub mod kjv;
pub mod kretzmann;
pub mod lexicon;

use atlas_graph_types::canon::{Canon, RowFamily, Value as CanonValue};
use atlas_graph_types::chrono::DatedBy;
use atlas_graph_types::edge::{
    Analogue, Attests, CanonSuccession, CatechismLink, CommentsOn, Confesses, Contains,
    Corresponds, CrossRef, Fulfills, Justification, LocatedAt, Mentions, NamedAfter, Occurs, Quotes,
    SpokenAt, SpokenBy, Succession, TemporalAdjacency, Typology,
};
use atlas_graph_types::text::{BibleTag, ConcordTag};
use rusqlite::types::Value;
use rusqlite::{Connection, Row, Transaction};

use super::columns::JustificationWriter;
use super::SqliteError;

/// One row of one family, borrowed from the Graph (the writer's input).
#[derive(Clone, Copy, Debug)]
pub enum RowRef<'a> {
    ContainsBible(&'a Contains<BibleTag>),
    ContainsConcord(&'a Contains<ConcordTag>),
    Attests(&'a Attests),
    Succession(&'a Succession),
    CanonSuccession(&'a CanonSuccession),
    DatedBy(&'a DatedBy),
    LocatedAt(&'a LocatedAt),
    Fulfills(&'a Fulfills),
    Typology(&'a Typology),
    NamedAfter(&'a NamedAfter),
    Catechism(&'a CatechismLink),
    Mentions(&'a Mentions),
    CorrespondsBible(&'a Corresponds<BibleTag>),
    TemporalAdjacency(&'a TemporalAdjacency),
    Analogue(&'a Analogue),
    CrossRefs(&'a CrossRef),
    SpokenBy(&'a SpokenBy),
    SpokenAt(&'a SpokenAt),
    Quotes(&'a Quotes),
    Confesses(&'a Confesses),
    CommentsOn(&'a CommentsOn),
    Occurs(&'a Occurs),
}

/// Owned rows read back (the reader's output) -- the same 21 arms.
#[derive(Clone, Debug)]
pub enum RowOwned {
    ContainsBible(Contains<BibleTag>),
    ContainsConcord(Contains<ConcordTag>),
    Attests(Attests),
    Succession(Succession),
    CanonSuccession(CanonSuccession),
    DatedBy(DatedBy),
    LocatedAt(LocatedAt),
    Fulfills(Fulfills),
    Typology(Typology),
    NamedAfter(NamedAfter),
    Catechism(CatechismLink),
    Mentions(Mentions),
    CorrespondsBible(Corresponds<BibleTag>),
    TemporalAdjacency(TemporalAdjacency),
    Analogue(Analogue),
    CrossRefs(CrossRef),
    SpokenBy(SpokenBy),
    SpokenAt(SpokenAt),
    Quotes(Quotes),
    Confesses(Confesses),
    CommentsOn(CommentsOn),
    Occurs(Occurs),
}

macro_rules! per_arm {
    ($self:expr, $r:ident => $body:expr) => {
        match $self {
            Self::ContainsBible($r) => $body,
            Self::ContainsConcord($r) => $body,
            Self::Attests($r) => $body,
            Self::Succession($r) => $body,
            Self::CanonSuccession($r) => $body,
            Self::DatedBy($r) => $body,
            Self::LocatedAt($r) => $body,
            Self::Fulfills($r) => $body,
            Self::Typology($r) => $body,
            Self::NamedAfter($r) => $body,
            Self::Catechism($r) => $body,
            Self::Mentions($r) => $body,
            Self::CorrespondsBible($r) => $body,
            Self::TemporalAdjacency($r) => $body,
            Self::Analogue($r) => $body,
            Self::CrossRefs($r) => $body,
            Self::SpokenBy($r) => $body,
            Self::SpokenAt($r) => $body,
            Self::Quotes($r) => $body,
            Self::Confesses($r) => $body,
            Self::CommentsOn($r) => $body,
            Self::Occurs($r) => $body,
        }
    };
}

macro_rules! family_of {
    ($self:expr) => {
        match $self {
            Self::ContainsBible(_) => RowFamily::ContainsBible,
            Self::ContainsConcord(_) => RowFamily::ContainsConcord,
            Self::Attests(_) => RowFamily::Attests,
            Self::Succession(_) => RowFamily::Succession,
            Self::CanonSuccession(_) => RowFamily::CanonSuccession,
            Self::DatedBy(_) => RowFamily::DatedBy,
            Self::LocatedAt(_) => RowFamily::LocatedAt,
            Self::Fulfills(_) => RowFamily::Fulfills,
            Self::Typology(_) => RowFamily::Typology,
            Self::NamedAfter(_) => RowFamily::NamedAfter,
            Self::Catechism(_) => RowFamily::Catechism,
            Self::Mentions(_) => RowFamily::Mentions,
            Self::CorrespondsBible(_) => RowFamily::CorrespondsBible,
            Self::TemporalAdjacency(_) => RowFamily::TemporalAdjacency,
            Self::Analogue(_) => RowFamily::Analogue,
            Self::CrossRefs(_) => RowFamily::CrossRefs,
            Self::SpokenBy(_) => RowFamily::SpokenBy,
            Self::SpokenAt(_) => RowFamily::SpokenAt,
            Self::Quotes(_) => RowFamily::Quotes,
            Self::Confesses(_) => RowFamily::Confesses,
            Self::CommentsOn(_) => RowFamily::CommentsOn,
            Self::Occurs(_) => RowFamily::Occurs,
        }
    };
}

impl<'a> RowRef<'a> {
    pub fn family(&self) -> RowFamily {
        family_of!(self)
    }
    /// Delegates to `Canon::to_value` -- the row's canonical Value.
    pub fn to_value(&self) -> CanonValue {
        per_arm!(self, r => r.to_value())
    }
    /// The row's justification, when its family is AUTHORED.
    pub fn justification(&self) -> Option<&'a Justification> {
        match self {
            Self::ContainsBible(r) => Some(&r.justification),
            Self::ContainsConcord(r) => Some(&r.justification),
            Self::Attests(r) => Some(&r.justification),
            Self::Succession(r) => Some(&r.justification),
            Self::CanonSuccession(r) => Some(&r.justification),
            Self::DatedBy(r) => Some(&r.justification),
            Self::LocatedAt(r) => Some(&r.justification),
            Self::Fulfills(r) => Some(&r.justification),
            Self::Typology(r) => Some(&r.justification),
            Self::NamedAfter(r) => Some(&r.justification),
            Self::Catechism(r) => Some(&r.justification),
            Self::SpokenBy(r) => Some(&r.justification),
            Self::SpokenAt(r) => Some(&r.justification),
            Self::Confesses(r) => Some(&r.justification),
            Self::CommentsOn(r) => Some(&r.justification),
            Self::Mentions(_)
            | Self::CorrespondsBible(_)
            | Self::TemporalAdjacency(_)
            | Self::Analogue(_)
            | Self::CrossRefs(_)
            | Self::Quotes(_)
            | Self::Occurs(_) => None,
        }
    }
}

impl RowOwned {
    pub fn family(&self) -> RowFamily {
        family_of!(self)
    }
    pub fn to_value(&self) -> CanonValue {
        per_arm!(self, r => r.to_value())
    }
    pub fn as_ref(&self) -> RowRef<'_> {
        match self {
            Self::ContainsBible(r) => RowRef::ContainsBible(r),
            Self::ContainsConcord(r) => RowRef::ContainsConcord(r),
            Self::Attests(r) => RowRef::Attests(r),
            Self::Succession(r) => RowRef::Succession(r),
            Self::CanonSuccession(r) => RowRef::CanonSuccession(r),
            Self::DatedBy(r) => RowRef::DatedBy(r),
            Self::LocatedAt(r) => RowRef::LocatedAt(r),
            Self::Fulfills(r) => RowRef::Fulfills(r),
            Self::Typology(r) => RowRef::Typology(r),
            Self::NamedAfter(r) => RowRef::NamedAfter(r),
            Self::Catechism(r) => RowRef::Catechism(r),
            Self::Mentions(r) => RowRef::Mentions(r),
            Self::CorrespondsBible(r) => RowRef::CorrespondsBible(r),
            Self::TemporalAdjacency(r) => RowRef::TemporalAdjacency(r),
            Self::Analogue(r) => RowRef::Analogue(r),
            Self::CrossRefs(r) => RowRef::CrossRefs(r),
            Self::SpokenBy(r) => RowRef::SpokenBy(r),
            Self::SpokenAt(r) => RowRef::SpokenAt(r),
            Self::Quotes(r) => RowRef::Quotes(r),
            Self::Confesses(r) => RowRef::Confesses(r),
            Self::CommentsOn(r) => RowRef::CommentsOn(r),
            Self::Occurs(r) => RowRef::Occurs(r),
        }
    }
}

/// Write one row at global ord `ord`, primary key `id = ord`.
pub fn insert_row(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &RowRef) -> Result<(), SqliteError> {
    match row {
        RowRef::ContainsBible(r) => core::insert_contains(tx, jw, ord, r, "contains_bible"),
        RowRef::ContainsConcord(r) => core::insert_contains(tx, jw, ord, r, "contains_concord"),
        RowRef::Attests(r) => core::insert_attests(tx, jw, ord, r),
        RowRef::Succession(r) => core::insert_succession(tx, jw, ord, r),
        RowRef::CanonSuccession(r) => kjv::insert_canon_succession(tx, jw, ord, r),
        RowRef::DatedBy(r) => core::insert_dated_by(tx, jw, ord, r),
        RowRef::LocatedAt(r) => core::insert_located_at(tx, jw, ord, r),
        RowRef::Fulfills(r) => core::insert_fulfills(tx, jw, ord, r),
        RowRef::Typology(r) => core::insert_typology(tx, jw, ord, r),
        RowRef::NamedAfter(r) => core::insert_named_after(tx, jw, ord, r),
        RowRef::Catechism(r) => core::insert_catechism(tx, jw, ord, r),
        RowRef::Mentions(r) => core::insert_mentions(tx, ord, r),
        RowRef::CorrespondsBible(r) => core::insert_corresponds_bible(tx, ord, r),
        RowRef::TemporalAdjacency(r) => core::insert_temporal_adjacency(tx, ord, r),
        RowRef::Analogue(r) => core::insert_analogue(tx, ord, r),
        RowRef::CrossRefs(r) => kjv::insert_cross_refs(tx, ord, r),
        RowRef::SpokenBy(r) => kjv::insert_spoken_by(tx, jw, ord, r),
        RowRef::SpokenAt(r) => kjv::insert_spoken_at(tx, jw, ord, r),
        RowRef::Quotes(r) => concord::insert_quotes(tx, ord, r),
        RowRef::Confesses(r) => concord::insert_confesses(tx, jw, ord, r),
        RowRef::CommentsOn(r) => kretzmann::insert_comments_on(tx, jw, ord, r),
        RowRef::Occurs(r) => lexicon::insert_occurs(tx, ord, r),
    }
}

/// Every row of `family` in this section, `ORDER BY id`, with its ord.
pub fn read_rows(conn: &Connection, family: RowFamily) -> Result<Vec<(i64, RowOwned)>, SqliteError> {
    fn wrap<T>(v: Vec<(i64, T)>, f: impl Fn(T) -> RowOwned) -> Vec<(i64, RowOwned)> {
        v.into_iter().map(|(o, r)| (o, f(r))).collect()
    }
    Ok(match family {
        RowFamily::ContainsBible => wrap(core::read_contains::<BibleTag>(conn, "contains_bible")?, RowOwned::ContainsBible),
        RowFamily::ContainsConcord => {
            wrap(core::read_contains::<ConcordTag>(conn, "contains_concord")?, RowOwned::ContainsConcord)
        }
        RowFamily::Attests => wrap(core::read_attests(conn)?, RowOwned::Attests),
        RowFamily::Succession => wrap(core::read_succession(conn)?, RowOwned::Succession),
        RowFamily::CanonSuccession => wrap(kjv::read_canon_succession(conn)?, RowOwned::CanonSuccession),
        RowFamily::DatedBy => wrap(core::read_dated_by(conn)?, RowOwned::DatedBy),
        RowFamily::LocatedAt => wrap(core::read_located_at(conn)?, RowOwned::LocatedAt),
        RowFamily::Fulfills => wrap(core::read_fulfills(conn)?, RowOwned::Fulfills),
        RowFamily::Typology => wrap(core::read_typology(conn)?, RowOwned::Typology),
        RowFamily::NamedAfter => wrap(core::read_named_after(conn)?, RowOwned::NamedAfter),
        RowFamily::Catechism => wrap(core::read_catechism(conn)?, RowOwned::Catechism),
        RowFamily::Mentions => wrap(core::read_mentions(conn)?, RowOwned::Mentions),
        RowFamily::CorrespondsBible => wrap(core::read_corresponds_bible(conn)?, RowOwned::CorrespondsBible),
        RowFamily::TemporalAdjacency => wrap(core::read_temporal_adjacency(conn)?, RowOwned::TemporalAdjacency),
        RowFamily::Analogue => wrap(core::read_analogue(conn)?, RowOwned::Analogue),
        RowFamily::CrossRefs => wrap(kjv::read_cross_refs(conn)?, RowOwned::CrossRefs),
        RowFamily::SpokenBy => wrap(kjv::read_spoken_by(conn)?, RowOwned::SpokenBy),
        RowFamily::SpokenAt => wrap(kjv::read_spoken_at(conn)?, RowOwned::SpokenAt),
        RowFamily::Quotes => wrap(concord::read_quotes(conn)?, RowOwned::Quotes),
        RowFamily::Confesses => wrap(concord::read_confesses(conn)?, RowOwned::Confesses),
        RowFamily::CommentsOn => wrap(kretzmann::read_comments_on(conn)?, RowOwned::CommentsOn),
        RowFamily::Occurs => wrap(lexicon::read_occurs(conn)?, RowOwned::Occurs),
    })
}

// ---------------------------------------------------------------------
// Shared plumbing for the per-section files
// ---------------------------------------------------------------------

pub(super) fn text(s: &str) -> Value {
    Value::Text(s.to_string())
}
pub(super) fn opt_text(s: &Option<String>) -> Value {
    s.as_ref().map(|s| Value::Text(s.clone())).unwrap_or(Value::Null)
}
pub(super) fn int(i: i64) -> Value {
    Value::Integer(i)
}

/// The AUTHORED `justification_id` value: an id, or NULL for an empty one.
pub(super) fn authored(tx: &Transaction, jw: &mut JustificationWriter, j: &Justification) -> Result<Value, SqliteError> {
    Ok(jw.write(tx, j)?.map(Value::Integer).unwrap_or(Value::Null))
}

/// `INSERT INTO <table> (id, ord, <cols>) VALUES (?, ?, …)` with `vals`
/// being exactly the `<cols>` values; `id = ord`.
pub(super) fn insert(tx: &Transaction, table: &str, cols: &str, ord: i64, vals: Vec<Value>) -> Result<(), SqliteError> {
    let n = cols.split(',').count();
    if n != vals.len() {
        return Err(SqliteError(format!("{table}: {n} columns but {} values", vals.len())));
    }
    let placeholders = std::iter::repeat_n("?", n + 2).collect::<Vec<_>>().join(", ");
    let sql = format!("INSERT INTO {table} (id, ord, {cols}) VALUES ({placeholders})");
    let mut all = Vec::with_capacity(n + 2);
    all.push(Value::Integer(ord));
    all.push(Value::Integer(ord));
    all.extend(vals);
    tx.prepare_cached(&sql)?.execute(rusqlite::params_from_iter(all.iter()))?;
    Ok(())
}

/// `SELECT id, ord, <cols> FROM <table> ORDER BY id`, asserting `id == ord`
/// on every row; `f` reads the row's data columns starting at index 2.
pub(super) fn read_all<T>(
    conn: &Connection,
    table: &str,
    cols: &str,
    mut f: impl FnMut(&Row) -> Result<T, SqliteError>,
) -> Result<Vec<(i64, T)>, SqliteError> {
    let sql = format!("SELECT id, ord, {cols} FROM {table} ORDER BY id");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let ord: i64 = row.get(1)?;
        if id != ord {
            return Err(SqliteError(format!("{table}: id {id} != ord {ord} (this batch writes id = ord)")));
        }
        out.push((ord, f(row)?));
    }
    Ok(out)
}

/// The data-column offset inside a `read_all` row (after `id, ord`).
pub(super) const D: usize = 2;

pub(super) fn read_justification_at(conn: &Connection, row: &Row, i: usize) -> Result<Justification, SqliteError> {
    let jid: Option<i64> = super::columns::col(row, i, "justification_id")?;
    super::columns::read_justification(conn, jid)
}

/// A typed node id column: the erased raw string.
pub(super) fn id_col<K: atlas_graph_types::id::KindTag>(
    row: &Row,
    i: usize,
    what: &str,
) -> Result<atlas_graph_types::id::NodeId<K>, SqliteError> {
    let raw: String = super::columns::col(row, i, what)?;
    Ok(atlas_graph_types::id::NodeId::new(raw))
}
