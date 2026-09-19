//! DB-4b: the section tables that are not graph-native (spec §5.3–5.6):
//! one generic row shape (`Col`, `TableSpec`, `ExtraTable`), one INSERT,
//! one `SELECT … ORDER BY pk`, one canonical encoder
//! (`sections::extra_line_body`), and the graph-derived builders (node
//! projections, the resolved chronology, the heading index, red-letter
//! spans). The folded sidecars live in `sidecars.rs` and share the shape.
//!
//! `Graph::extra_tables` carries the ENCODED bodies so the version root
//! covers them (graph-types learns only a map of bytes); the writer
//! inserts the TYPED rows from the same `Extras`. The law that the dump
//! recomputed from the file equals the attached dump (`sqlite_laws.rs`,
//! gate 9) is what proves `attach` and `insert_table` agree.
//!
//! Primary-key order on both sides is SQLite's BINARY order per column
//! (`pk_cmp`): integers numerically, text by bytes. No extra table has a
//! REAL or a NULL in its primary key.

use std::cmp::Ordering;
use std::collections::HashMap;

use atlas_graph_types::canon::ids::any_node_id_str;
use atlas_graph_types::canon::Value;
use atlas_graph_types::chrono::PlacementBasis;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::sections::{extra_line_body, Section};
use rusqlite::{Connection, Transaction};

use super::SqliteError;

/// One column value. The four SQLite storage classes an extra table uses
/// (no BLOB: hashes live only in `node`/`edge_index`).
#[derive(Clone, Debug, PartialEq)]
pub enum Col {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
}

/// A table's name, its columns in DDL order, and its primary key in key
/// order. `sqlite_laws.rs` pins every spec against `PRAGMA table_info` of
/// the DDL and against `sections::extra_tables_of`.
pub struct TableSpec {
    pub name: &'static str,
    pub columns: &'static [&'static str],
    pub pk: &'static [&'static str],
}

/// One table's rows, one `Col` per `spec.columns` entry.
pub struct ExtraTable {
    pub spec: &'static TableSpec,
    pub rows: Vec<Vec<Col>>,
}

/// Every extra table of every section, rows in primary-key order once
/// `extend`ed.
#[derive(Default)]
pub struct Extras {
    pub tables: Vec<ExtraTable>,
}

pub static PLACE: TableSpec = TableSpec { name: "place", columns: &["node_id", "canonical", "lat", "lon"], pk: &["node_id"] };
pub static ERA: TableSpec = TableSpec { name: "era", columns: &["node_id", "label", "from_year", "to_year"], pk: &["node_id"] };
pub static POLITY_ERA: TableSpec =
    TableSpec { name: "polity_era", columns: &["node_id", "ord", "name", "from_year", "to_year"], pk: &["node_id", "ord"] };
pub static EVENT_DATE: TableSpec = TableSpec {
    name: "event_date",
    columns: &["event_id", "from_year", "to_year", "from_month", "from_day", "to_month", "to_day", "seq", "basis", "meta_to_year", "order_key"],
    pk: &["event_id"],
};
pub static HEADING_INDEX: TableSpec = TableSpec {
    name: "heading_index",
    columns: &["book", "chapter", "verse", "event_id", "title", "kind", "continuation"],
    pk: &["book", "chapter", "verse"],
};
pub static VERSE: TableSpec = TableSpec { name: "verse", columns: &["node_id", "book", "chapter", "verse"], pk: &["node_id"] };
pub static RED_LETTER_SPAN: TableSpec = TableSpec {
    name: "red_letter_span",
    columns: &["book", "chapter", "verse", "ord", "start", "end_"],
    pk: &["book", "chapter", "verse", "ord"],
};
pub static CONCORD_UNIT: TableSpec =
    TableSpec { name: "concord_unit", columns: &["node_id", "part", "article", "paragraph"], pk: &["node_id"] };
// LEX-1 (spec 5.7).
pub static LEXICON_ENTRY: TableSpec = TableSpec {
    name: "lexicon_entry",
    columns: &["node_id", "strong", "lang", "lemma", "translit", "pos", "root_strong"],
    pk: &["node_id"],
};
pub static LEXICON_DOMAIN: TableSpec = TableSpec { name: "lexicon_domain", columns: &["node_id", "ord", "code"], pk: &["node_id", "ord"] };
pub static TOKEN: TableSpec = TableSpec {
    name: "token",
    columns: &["book", "chapter", "verse", "layer", "ord", "form", "lemma", "xpos", "translit", "strong", "aligned"],
    pk: &["book", "chapter", "verse", "layer", "ord"],
};

/// Core's specs: the five graph-derived tables, then the 21 folded
/// sidecars (`sidecars::SIDECAR_SPECS`), in `extra_tables_of` order.
static CORE_SPECS: [&TableSpec; 26] = [
    &PLACE,
    &ERA,
    &POLITY_ERA,
    &EVENT_DATE,
    &HEADING_INDEX,
    &super::sidecars::CANON_BOOK,
    &super::sidecars::CANON_CHAPTER_VERSES,
    &super::sidecars::BOOK_META,
    &super::sidecars::CHRONOLOGY_ANCHOR,
    &super::sidecars::BOOK_NARRATION_WINDOW,
    &super::sidecars::LANDMARK,
    &super::sidecars::LAND_MASK_REGION,
    &super::sidecars::CATECHISM_PART,
    &super::sidecars::CATECHISM_ITEM,
    &super::sidecars::CATECHISM_ITEM_VERSE,
    &super::sidecars::CATECHISM_QUESTION,
    &super::sidecars::CATECHISM_QUESTION_VERSE,
    &super::sidecars::PLACE_HISTORY,
    &super::sidecars::PLACE_HISTORY_NAME,
    &super::sidecars::PLACE_HISTORY_BLURB,
    &super::sidecars::PLACE_HISTORY_VERSE,
    &super::sidecars::PLACE_NAME_ALIAS,
    &super::sidecars::PLACE_NAME_ALIAS_VERSE,
    &super::sidecars::SOURCE_CATEGORY,
    &super::sidecars::SOURCE_ENTRY,
    &super::sidecars::PROVENANCE_ENTRY,
];
static KJV_SPECS: [&TableSpec; 2] = [&VERSE, &RED_LETTER_SPAN];
static CONCORD_SPECS: [&TableSpec; 1] = [&CONCORD_UNIT];
static LEXICON_SPECS: [&TableSpec; 3] = [&LEXICON_ENTRY, &LEXICON_DOMAIN, &TOKEN];

/// The extra tables a section carries -- the same names, in the same
/// order, as `sections::extra_tables_of` (a law pins it).
pub fn table_specs_of(section: Section) -> &'static [&'static TableSpec] {
    match section {
        Section::Core => &CORE_SPECS,
        Section::Kjv => &KJV_SPECS,
        Section::Concord => &CONCORD_SPECS,
        Section::Lexicon => &LEXICON_SPECS,
        Section::Kretzmann => &[],
    }
}

pub fn spec_named(name: &str) -> Option<&'static TableSpec> {
    Section::SHIPPED.iter().flat_map(|s| table_specs_of(*s).iter().copied()).find(|t| t.name == name)
}

fn col_value(c: &Col) -> Result<Value, SqliteError> {
    Ok(match c {
        Col::Null => Value::Null,
        Col::Int(i) => Value::Int(*i),
        Col::Real(f) => Value::float(*f)?,
        Col::Text(s) => Value::Str(s.clone()),
    })
}

/// The row's canonical body: `extra_line_body` over `(column, value)`.
pub fn row_body(spec: &TableSpec, row: &[Col]) -> Result<Vec<u8>, SqliteError> {
    if row.len() != spec.columns.len() {
        return Err(SqliteError(format!("{}: row has {} cols, spec has {}", spec.name, row.len(), spec.columns.len())));
    }
    let mut cols = Vec::with_capacity(row.len());
    for (name, c) in spec.columns.iter().zip(row) {
        cols.push((*name, col_value(c)?));
    }
    Ok(extra_line_body(cols))
}

/// SQLite's BINARY collation order within one column: NULL < numeric < TEXT;
/// numerics by value, text by bytes.
fn col_cmp(a: &Col, b: &Col) -> Ordering {
    fn class(c: &Col) -> u8 {
        match c {
            Col::Null => 0,
            Col::Int(_) | Col::Real(_) => 1,
            Col::Text(_) => 2,
        }
    }
    match (a, b) {
        (Col::Int(x), Col::Int(y)) => x.cmp(y),
        (Col::Real(x), Col::Real(y)) => x.partial_cmp(y).unwrap_or(Ordering::Equal),
        (Col::Int(x), Col::Real(y)) => (*x as f64).partial_cmp(y).unwrap_or(Ordering::Equal),
        (Col::Real(x), Col::Int(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(Ordering::Equal),
        (Col::Text(x), Col::Text(y)) => x.as_bytes().cmp(y.as_bytes()),
        _ => class(a).cmp(&class(b)),
    }
}

/// Primary-key order of two rows of `spec`.
pub fn pk_cmp(spec: &TableSpec, a: &[Col], b: &[Col]) -> Ordering {
    for k in spec.pk {
        let i = spec.columns.iter().position(|c| c == k).expect("a pk column is a column");
        match col_cmp(&a[i], &b[i]) {
            Ordering::Equal => continue,
            o => return o,
        }
    }
    Ordering::Equal
}

impl ExtraTable {
    pub fn sorted(mut self) -> Self {
        let spec = self.spec;
        self.rows.sort_by(|a, b| pk_cmp(spec, a, b));
        self
    }
}

impl Extras {
    pub fn table(&self, name: &str) -> Option<&ExtraTable> {
        self.tables.iter().find(|t| t.spec.name == name)
    }

    /// Adds tables, each sorted into primary-key order.
    pub fn extend(&mut self, more: Vec<ExtraTable>) {
        self.tables.extend(more.into_iter().map(ExtraTable::sorted));
    }

    /// `g.extra_tables[name] = the canonical bodies, in pk order` for every
    /// table here (replacing any earlier attachment of the same table).
    pub fn attach(&self, g: &mut Graph) {
        for t in &self.tables {
            let bodies: Vec<Vec<u8>> =
                t.rows.iter().map(|r| row_body(t.spec, r).expect("attach: every extra row encodes")).collect();
            g.extra_tables.insert(t.spec.name, bodies);
        }
    }

    /// The graph-derived tables: projections of `Place`/`Era`/`Polity`/
    /// `TextUnit` payloads, `event_date` from the chronology (`resolved`:
    /// `seq` is the total order, R-DB4a-1; DB-4c: plus `source_meta`'s
    /// curated `to_year`/`order_key`, the values the Event wire serves --
    /// NULL for an event without an entry, which `legacy::event_from_node`
    /// substitutes from `from_year`/`0`), `heading_index` through
    /// `heading::build_heading_index`, and `red_letter_span` from the
    /// char-offset span map.
    pub fn graph_derived(
        g: &Graph,
        chrono: &crate::event_world::ChronologyDerivation,
        red_letter: &HashMap<String, Vec<(usize, usize)>>,
    ) -> Result<Extras, SqliteError> {
        let resolved = &chrono.resolved;
        let (mut place, mut era, mut polity_era, mut verse, mut concord) = (vec![], vec![], vec![], vec![], vec![]);
        let (mut lexicon_entry, mut lexicon_domain) = (vec![], vec![]);
        for n in g.nodes.values() {
            let id = any_node_id_str(&n.id);
            match &n.payload {
                NodePayload::Place { canonical, lat, lon, .. } => {
                    place.push(vec![Col::Text(id), Col::Text(canonical.clone()), Col::Real(*lat), Col::Real(*lon)])
                }
                NodePayload::Era { label, from_year, to_year } => {
                    era.push(vec![Col::Text(id), Col::Text(label.clone()), Col::Int(*from_year as i64), Col::Int(*to_year as i64)])
                }
                NodePayload::Polity { eras, .. } => {
                    for (i, e) in eras.iter().enumerate() {
                        polity_era.push(vec![
                            Col::Text(id.clone()),
                            Col::Int(i as i64),
                            Col::Text(e.name.clone()),
                            Col::Int(e.from_year as i64),
                            Col::Int(e.to_year as i64),
                        ]);
                    }
                }
                NodePayload::LexiconEntry { strong, lang, lemma, translit, pos, domains, root, .. } => {
                    lexicon_entry.push(vec![
                        Col::Text(id.clone()),
                        Col::Text(strong.clone()),
                        Col::Text(lang.clone()),
                        Col::Text(lemma.clone()),
                        opt_text(translit),
                        opt_text(pos),
                        opt_text(root),
                    ]);
                    for (i, code) in domains.iter().enumerate() {
                        lexicon_domain.push(vec![Col::Text(id.clone()), Col::Int(i as i64), Col::Text(code.clone())]);
                    }
                }
                NodePayload::TextUnit { .. } => {
                    if let Some((b, c, v)) = crate::kjv_adapter::decode_text_unit(&n.id) {
                        verse.push(vec![Col::Text(id), Col::Int(b as i64), Col::Int(c as i64), Col::Int(v as i64)]);
                    } else if let Some((p, a, par)) = crate::concord_adapter::decode_text_unit(&n.id) {
                        concord.push(vec![Col::Text(id), Col::Int(p as i64), Col::Int(a as i64), Col::Int(par as i64)]);
                    } else {
                        return Err(SqliteError(format!("TextUnit {id} is neither a bible nor a concord unit")));
                    }
                }
                _ => {}
            }
        }
        let event_date = resolved
            .iter()
            .map(|(id, r)| {
                vec![
                    Col::Text(id.clone()),
                    Col::Int(r.date.from.year.get() as i64),
                    Col::Int(r.date.to.year.get() as i64),
                    opt_u8(r.date.from.month),
                    opt_u8(r.date.from.day),
                    opt_u8(r.date.to.month),
                    opt_u8(r.date.to.day),
                    Col::Int(r.seq.0 as i64),
                    Col::Int(match r.basis {
                        PlacementBasis::Textual => 0,
                        PlacementBasis::Traditional => 1,
                    }),
                    chrono.source_meta.get(id).map(|m| Col::Int(m.to_year as i64)).unwrap_or(Col::Null),
                    chrono.source_meta.get(id).map(|m| Col::Int(m.order_key as i64)).unwrap_or(Col::Null),
                ]
            })
            .collect();
        let heading = crate::heading::build_heading_index(g, resolved);
        let mut heading_rows = Vec::with_capacity(heading.len());
        for (sref, h) in &heading {
            let (b, c, v) = verse_triple(sref)?;
            heading_rows.push(vec![
                Col::Int(b),
                Col::Int(c),
                Col::Int(v),
                Col::Text(h.event_id.clone()),
                Col::Text(h.title.clone()),
                Col::Text(h.kind.clone()),
                Col::Int(h.continuation as i64),
            ]);
        }
        let mut red = Vec::new();
        for (sref, spans) in red_letter {
            let (b, c, v) = verse_triple(sref)?;
            for (i, (s, e)) in spans.iter().enumerate() {
                red.push(vec![Col::Int(b), Col::Int(c), Col::Int(v), Col::Int(i as i64), Col::Int(*s as i64), Col::Int(*e as i64)]);
            }
        }
        let mut out = Extras::default();
        out.extend(vec![
            ExtraTable { spec: &PLACE, rows: place },
            ExtraTable { spec: &ERA, rows: era },
            ExtraTable { spec: &POLITY_ERA, rows: polity_era },
            ExtraTable { spec: &EVENT_DATE, rows: event_date },
            ExtraTable { spec: &HEADING_INDEX, rows: heading_rows },
            ExtraTable { spec: &VERSE, rows: verse },
            ExtraTable { spec: &RED_LETTER_SPAN, rows: red },
            ExtraTable { spec: &CONCORD_UNIT, rows: concord },
            ExtraTable { spec: &LEXICON_ENTRY, rows: lexicon_entry },
            ExtraTable { spec: &LEXICON_DOMAIN, rows: lexicon_domain },
        ]);
        Ok(out)
    }

    /// LEX-1: the `token` inventory from the corpus (spec 5.7) -- every
    /// token, matched or not; NOT graph-derived (the graph carries only the
    /// aligned ones, as `Occurs` rows), so it rides in from the reader.
    pub fn tokens(tokens: &[atlas_etl::lexicon::TokenRow]) -> ExtraTable {
        let rows = tokens
            .iter()
            .map(|t| {
                vec![
                    Col::Int(t.book.0 as i64),
                    Col::Int(t.chapter as i64),
                    Col::Int(t.verse as i64),
                    Col::Text(t.layer.to_string()),
                    Col::Int(t.ord as i64),
                    Col::Text(t.form.clone()),
                    opt_text(&t.lemma),
                    opt_text(&t.xpos),
                    opt_text(&t.translit),
                    opt_text(&t.strong),
                    Col::Int(t.aligned as i64),
                ]
            })
            .collect();
        ExtraTable { spec: &TOKEN, rows }
    }
}

fn opt_text(v: &Option<String>) -> Col {
    v.as_ref().map(|s| Col::Text(s.clone())).unwrap_or(Col::Null)
}

fn opt_u8(v: Option<u8>) -> Col {
    v.map(|x| Col::Int(x as i64)).unwrap_or(Col::Null)
}

/// `"GEN.1.1"` -> `(book index, chapter, verse)` through
/// `atlas_core::refs::ScriptureRef::parse` (its `Verse` arm).
pub fn verse_triple(sref: &str) -> Result<(i64, i64, i64), SqliteError> {
    match atlas_core::refs::ScriptureRef::parse(sref) {
        Ok(atlas_core::refs::ScriptureRef::Verse(v)) => Ok((v.book.0 as i64, v.chapter as i64, v.verse as i64)),
        Ok(other) => Err(SqliteError(format!("{sref} is not a single-verse dot-ref: {other:?}"))),
        Err(e) => Err(SqliteError(format!("{sref} is not a dot-ref: {e}"))),
    }
}

/// The whole fold, from memory (DB-5): the graph-derived tables plus the
/// nine sidecars folded from the ETL's own in-memory `AtlasData` and the
/// `sources.json` document -- what the compile attaches to both graphs and
/// writes into the sections. No JSON sidecar is read back from disk.
pub fn compute(
    g: &Graph,
    chrono: &crate::event_world::ChronologyDerivation,
    red_letter: &HashMap<String, Vec<(usize, usize)>>,
    atlas: &atlas_core::data::AtlasData,
    sources: &atlas_core::sources::SourcesDocument,
    tokens: &[atlas_etl::lexicon::TokenRow],
) -> Result<Extras, SqliteError> {
    let mut ex = Extras::graph_derived(g, chrono, red_letter)?;
    ex.extend(super::sidecars::fold_sidecars(atlas, sources)?);
    ex.extend(vec![Extras::tokens(tokens)]);
    Ok(ex)
}

/// `INSERT INTO <table> (<columns>) VALUES (?, …)` per row.
pub fn insert_table(tx: &Transaction, t: &ExtraTable) -> Result<(), SqliteError> {
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        t.spec.name,
        t.spec.columns.join(", "),
        vec!["?"; t.spec.columns.len()].join(", ")
    );
    let mut stmt = tx.prepare_cached(&sql)?;
    for row in &t.rows {
        if row.len() != t.spec.columns.len() {
            return Err(SqliteError(format!("{}: row has {} cols, spec has {}", t.spec.name, row.len(), t.spec.columns.len())));
        }
        let params: Vec<rusqlite::types::Value> = row
            .iter()
            .map(|c| match c {
                Col::Null => rusqlite::types::Value::Null,
                Col::Int(i) => rusqlite::types::Value::Integer(*i),
                Col::Real(f) => rusqlite::types::Value::Real(*f),
                Col::Text(s) => rusqlite::types::Value::Text(s.clone()),
            })
            .collect();
        stmt.execute(rusqlite::params_from_iter(params))?;
    }
    Ok(())
}

/// `SELECT <columns> FROM <table> ORDER BY <pk>` -- the read-back the
/// logical dump re-encodes.
pub fn read_table(conn: &Connection, spec: &TableSpec) -> Result<Vec<Vec<Col>>, SqliteError> {
    let sql = format!("SELECT {} FROM {} ORDER BY {}", spec.columns.join(", "), spec.name, spec.pk.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    let mut out = Vec::new();
    while let Some(r) = rows.next()? {
        let mut cols = Vec::with_capacity(spec.columns.len());
        for i in 0..spec.columns.len() {
            cols.push(match r.get_ref(i)? {
                rusqlite::types::ValueRef::Null => Col::Null,
                rusqlite::types::ValueRef::Integer(i) => Col::Int(i),
                rusqlite::types::ValueRef::Real(f) => Col::Real(f),
                rusqlite::types::ValueRef::Text(t) => {
                    Col::Text(String::from_utf8(t.to_vec()).map_err(|e| SqliteError(format!("{}: {e}", spec.name)))?)
                }
                rusqlite::types::ValueRef::Blob(_) => return Err(SqliteError(format!("{}: BLOB in an extra table", spec.name))),
            });
        }
        out.push(cols);
    }
    Ok(out)
}
