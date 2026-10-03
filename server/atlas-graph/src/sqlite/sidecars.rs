//! The nine former compiled JSON sidecars, folded into core's tables losslessly: every field of every loaded
//! struct lands in a column, because nothing reads a JSON sidecar. `unfold` is the inverse the served path uses.

use std::collections::HashMap;

use atlas_core::data::{
    EventId,
    AtlasData, BookMeta, BookNarrationWindow, Canon, CanonBook, CatechismItem, CatechismPart, CatechismQuestion,
    ChronologyAnchor, Landmark, LandmarkKind, LandmarkSize, PlaceDateClaim, PlaceHistory,
    PlaceNameAlias, PlaceNameEntry,
};
use atlas_core::sources::{Confidence, ProvenanceEntry, ProvenanceTitles, SourceCategory, SourceEntry, SourcesDocument};
use atlas_core::time::TimeRange;
use atlas_graph_types::canon::{serialize, Value};
use rusqlite::Connection;

use super::extras::{read_table, Col, ExtraTable, TableSpec};
use super::SqliteError;

pub static CANON_BOOK: TableSpec =
    TableSpec { name: "canon_book", columns: &["ord", "code", "name", "testament", "chapters"], pk: &["ord"] };
pub static CANON_CHAPTER_VERSES: TableSpec =
    TableSpec { name: "canon_chapter_verses", columns: &["book_ord", "chapter", "verses"], pk: &["book_ord", "chapter"] };
pub static BOOK_META: TableSpec =
    TableSpec { name: "book_meta", columns: &["book", "author", "write_place", "write_from", "write_to"], pk: &["book"] };
pub static CHRONOLOGY_ANCHOR: TableSpec = TableSpec {
    name: "chronology_anchor",
    columns: &["id", "ord", "label", "year", "event_id", "era_boundary", "source", "note"],
    pk: &["id"],
};
pub static BOOK_NARRATION_WINDOW: TableSpec =
    TableSpec { name: "book_narration_window", columns: &["book", "from_year", "to_year", "note"], pk: &["book"] };
pub static LANDMARK: TableSpec =
    TableSpec { name: "landmark", columns: &["ord", "name", "kind", "lat", "lon", "size"], pk: &["ord"] };
pub static LAND_MASK_REGION: TableSpec =
    TableSpec { name: "land_mask_region", columns: &["ord", "name", "ref_note", "rings_json"], pk: &["ord"] };
pub static CATECHISM_PART: TableSpec = TableSpec { name: "catechism_part", columns: &["id", "ord", "title"], pk: &["id"] };
pub static CATECHISM_ITEM: TableSpec = TableSpec {
    name: "catechism_item",
    columns: &["id", "part_id", "ord", "name", "text", "explanation_heading", "explanation", "where_written", "ref_note"],
    pk: &["id"],
};
pub static CATECHISM_ITEM_VERSE: TableSpec =
    TableSpec { name: "catechism_item_verse", columns: &["item_id", "ord", "sref"], pk: &["item_id", "ord"] };
pub static CATECHISM_QUESTION: TableSpec =
    TableSpec { name: "catechism_question", columns: &["item_id", "ord", "title", "source"], pk: &["item_id", "ord"] };
pub static CATECHISM_QUESTION_VERSE: TableSpec = TableSpec {
    name: "catechism_question_verse",
    columns: &["item_id", "question_ord", "ord", "sref"],
    pk: &["item_id", "question_ord", "ord"],
};
pub static PLACE_HISTORY: TableSpec = TableSpec {
    name: "place_history",
    columns: &["place_id", "est_from", "est_to", "est_note", "est_event", "dest_from", "dest_to", "dest_note", "dest_event"],
    pk: &["place_id"],
};
pub static PLACE_HISTORY_NAME: TableSpec = TableSpec {
    name: "place_history_name",
    columns: &["place_id", "ord", "name", "from_year", "to_year"],
    pk: &["place_id", "ord"],
};
pub static PLACE_HISTORY_VERSE: TableSpec = TableSpec {
    name: "place_history_verse",
    columns: &["place_id", "owner_kind", "owner_ord", "ord", "sref"],
    pk: &["place_id", "owner_kind", "owner_ord", "ord"],
};
pub static PLACE_NAME_ALIAS: TableSpec = TableSpec {
    name: "place_name_alias",
    columns: &["place_id", "alias_ord", "translation", "name"],
    pk: &["place_id", "alias_ord", "translation"],
};
pub static PLACE_NAME_ALIAS_VERSE: TableSpec = TableSpec {
    name: "place_name_alias_verse",
    columns: &["place_id", "alias_ord", "ord", "sref"],
    pk: &["place_id", "alias_ord", "ord"],
};
pub static SOURCE_CATEGORY: TableSpec = TableSpec { name: "source_category", columns: &["id", "ord", "label"], pk: &["id"] };
pub static SOURCE_ENTRY: TableSpec = TableSpec {
    name: "source_entry",
    columns: &["id", "ord", "category", "title", "what_it_is", "what_we_built", "license", "link", "licenses_row_key"],
    pk: &["id"],
};
pub static PROVENANCE_ENTRY: TableSpec =
    TableSpec { name: "provenance_entry", columns: &["id", "ord", "source", "confidence", "locator"], pk: &["id"] };
pub static PROVENANCE_TITLE: TableSpec = TableSpec { name: "provenance_title", columns: &["id", "title"], pk: &["id"] };

/// The 21 sidecar specs in `extra_tables_of(Core)` order, after the five graph-derived tables.
pub static SIDECAR_SPECS: [&TableSpec; 21] = [
    &CANON_BOOK,
    &CANON_CHAPTER_VERSES,
    &BOOK_META,
    &CHRONOLOGY_ANCHOR,
    &BOOK_NARRATION_WINDOW,
    &LANDMARK,
    &LAND_MASK_REGION,
    &CATECHISM_PART,
    &CATECHISM_ITEM,
    &CATECHISM_ITEM_VERSE,
    &CATECHISM_QUESTION,
    &CATECHISM_QUESTION_VERSE,
    &PLACE_HISTORY,
    &PLACE_HISTORY_NAME,
    &PLACE_HISTORY_VERSE,
    &PLACE_NAME_ALIAS,
    &PLACE_NAME_ALIAS_VERSE,
    &SOURCE_CATEGORY,
    &SOURCE_ENTRY,
    &PROVENANCE_ENTRY,
    &PROVENANCE_TITLE,
];

fn t(s: &str) -> Col {
    Col::Text(s.to_string())
}
fn ot(o: &Option<String>) -> Col {
    o.as_ref().map(|s| Col::Text(s.clone())).unwrap_or(Col::Null)
}
fn ow(word: Option<&str>) -> Col {
    word.map(|w| Col::Text(w.to_string())).unwrap_or(Col::Null)
}
fn oi(o: Option<i32>) -> Col {
    o.map(|x| Col::Int(x as i64)).unwrap_or(Col::Null)
}
fn i(x: impl Into<i64>) -> Col {
    Col::Int(x.into())
}

/// Books before index 39 are the Old Testament and the rest the New, by `atlas_core::canon::BOOKS`
/// order: the data carries no testament field.
pub fn testament_of(book_ord: usize) -> &'static str {
    if book_ord < 39 {
        "OT"
    } else {
        "NT"
    }
}

fn ring_json(ring: &[(f64, f64)]) -> Result<String, SqliteError> {
    let mut points = Vec::with_capacity(ring.len());
    for (lat, lon) in ring {
        points.push(Value::Arr(vec![Value::float(*lat)?, Value::float(*lon)?]));
    }
    Ok(String::from_utf8(serialize(&Value::Arr(points))).expect("canonical JSON is UTF-8"))
}

fn claim_cols(c: &Option<PlaceDateClaim>) -> [Col; 4] {
    match c {
        Some(c) => [i(c.when.from_year), i(c.when.to_year), ot(&c.note), ow(c.event.as_ref().map(|e| e.0.as_str()))],
        None => [Col::Null, Col::Null, Col::Null, Col::Null],
    }
}

/// The nine folds, as 21 tables, rows in source order -- `Extras::extend` sorts them into
/// primary-key order.
pub fn fold_sidecars(atlas: &AtlasData, sources: &SourcesDocument) -> Result<Vec<ExtraTable>, SqliteError> {
    let mut canon_book = Vec::new();
    let mut canon_chapter_verses = Vec::new();
    for (ord, b) in atlas.canon.books.iter().enumerate() {
        canon_book.push(vec![i(ord as i64), t(&b.code), t(&b.name), t(testament_of(ord)), i(b.chapters.len() as i64)]);
        for (ch, verses) in b.chapters.iter().enumerate() {
            canon_chapter_verses.push(vec![i(ord as i64), i(ch as i64 + 1), i(*verses)]);
        }
    }
    let book_meta = atlas
        .books_meta
        .iter()
        .map(|m| vec![t(&m.book), t(&m.author), ot(&m.write_place), oi(m.write_from), oi(m.write_to)])
        .collect();
    let chronology_anchor = atlas
        .chronology_anchors
        .iter()
        .enumerate()
        .map(|(ord, a)| {
            vec![t(&a.id), i(ord as i64), t(&a.label), i(a.year), ot(&a.event_id), i(a.era_boundary as i64), t(&a.source), ot(&a.note)]
        })
        .collect();
    let book_narration_window =
        atlas.book_narration_windows.iter().map(|w| vec![t(&w.book), i(w.from_year), i(w.to_year), ot(&w.note)]).collect();
    let landmark = atlas
        .landmarks
        .iter()
        .enumerate()
        .map(|(ord, l)| vec![i(ord as i64), t(&l.name), t(l.kind.name()), Col::Real(l.lat), Col::Real(l.lon), ow(l.size.map(LandmarkSize::name))])
        .collect();
    // The compiled land mask is flattened rings: a region's name and ref note were dropped before
    // this point, so both columns are honestly NULL.
    let mut land_mask_region = Vec::with_capacity(atlas.land_mask.len());
    for (ord, ring) in atlas.land_mask.iter().enumerate() {
        land_mask_region.push(vec![i(ord as i64), Col::Null, Col::Null, Col::Text(ring_json(ring)?)]);
    }
    let (mut part, mut item, mut item_verse, mut question, mut question_verse) = (vec![], vec![], vec![], vec![], vec![]);
    for (pord, p) in atlas.catechism.iter().enumerate() {
        part.push(vec![t(&p.id), i(pord as i64), t(&p.title)]);
        for (iord, it) in p.items.iter().enumerate() {
            item.push(vec![
                t(&it.id),
                t(&p.id),
                i(iord as i64),
                t(&it.name),
                ot(&it.text),
                t(&it.explanation_heading),
                t(&it.explanation),
                ot(&it.where_written),
                ot(&it.ref_note),
            ]);
            for (vord, v) in it.verses.iter().enumerate() {
                item_verse.push(vec![t(&it.id), i(vord as i64), t(v)]);
            }
            for (qord, q) in it.questions.iter().enumerate() {
                question.push(vec![t(&it.id), i(qord as i64), t(&q.title), t(&q.source)]);
                for (vord, v) in q.verses.iter().enumerate() {
                    question_verse.push(vec![t(&it.id), i(qord as i64), i(vord as i64), t(v)]);
                }
            }
        }
    }
    let (mut ph, mut ph_name, mut ph_verse) = (vec![], vec![], vec![]);
    let mut histories: Vec<_> = atlas.place_history.values().collect();
    histories.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    for h in histories {
        let [ef, et, en, ee] = claim_cols(&h.established);
        let [df, dt, dn, de] = claim_cols(&h.destroyed);
        ph.push(vec![t(&h.id), ef, et, en, ee, df, dt, dn, de]);
        for (ord, n) in h.names.iter().enumerate() {
            ph_name.push(vec![t(&h.id), i(ord as i64), t(&n.name), i(n.when.from_year), i(n.when.to_year)]);
            for (vord, v) in n.verses.iter().enumerate() {
                ph_verse.push(vec![t(&h.id), i(0i64), i(ord as i64), i(vord as i64), t(v)]);
            }
        }
        for (kind, claim) in [(1i64, &h.established), (2i64, &h.destroyed)] {
            if let Some(c) = claim {
                for (vord, v) in c.verses.iter().enumerate() {
                    ph_verse.push(vec![t(&h.id), i(kind), i(0i64), i(vord as i64), t(v)]);
                }
            }
        }
    }
    // One place may carry several alias rows, so the key widens with `alias_ord`.
    let (mut alias, mut alias_verse) = (vec![], vec![]);
    let mut alias_ids: Vec<_> = atlas.place_name_aliases.keys().collect();
    alias_ids.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    for id in alias_ids {
        for (aord, a) in atlas.place_name_aliases[id].iter().enumerate() {
            for (translation, name) in &a.translations {
                alias.push(vec![t(id), i(aord as i64), t(translation), t(name)]);
            }
            for (vord, v) in a.verses.iter().enumerate() {
                alias_verse.push(vec![t(id), i(aord as i64), i(vord as i64), t(v)]);
            }
        }
    }
    let source_category = sources.categories.iter().enumerate().map(|(ord, c)| vec![t(&c.id), i(ord as i64), t(&c.label)]).collect();
    let source_entry = sources
        .sources
        .iter()
        .enumerate()
        .map(|(ord, s)| {
            vec![
                t(&s.id),
                i(ord as i64),
                t(&s.category),
                t(&s.title),
                t(&s.what_it_is),
                t(&s.what_we_built),
                t(&s.license),
                ot(&s.link),
                t(&s.licenses_row_key),
            ]
        })
        .collect();
    let provenance_entry = sources
        .provenances
        .iter()
        .enumerate()
        .map(|(ord, p)| vec![t(&p.id), i(ord as i64), t(&p.source), t(p.confidence.name()), ot(&p.locator)])
        .collect();
    let provenance_title = atlas.provenance_titles.rows().map(|(id, title)| vec![t(id), t(title)]).collect();

    Ok(vec![
        ExtraTable { spec: &CANON_BOOK, rows: canon_book },
        ExtraTable { spec: &CANON_CHAPTER_VERSES, rows: canon_chapter_verses },
        ExtraTable { spec: &BOOK_META, rows: book_meta },
        ExtraTable { spec: &CHRONOLOGY_ANCHOR, rows: chronology_anchor },
        ExtraTable { spec: &BOOK_NARRATION_WINDOW, rows: book_narration_window },
        ExtraTable { spec: &LANDMARK, rows: landmark },
        ExtraTable { spec: &LAND_MASK_REGION, rows: land_mask_region },
        ExtraTable { spec: &CATECHISM_PART, rows: part },
        ExtraTable { spec: &CATECHISM_ITEM, rows: item },
        ExtraTable { spec: &CATECHISM_ITEM_VERSE, rows: item_verse },
        ExtraTable { spec: &CATECHISM_QUESTION, rows: question },
        ExtraTable { spec: &CATECHISM_QUESTION_VERSE, rows: question_verse },
        ExtraTable { spec: &PLACE_HISTORY, rows: ph },
        ExtraTable { spec: &PLACE_HISTORY_NAME, rows: ph_name },
        ExtraTable { spec: &PLACE_HISTORY_VERSE, rows: ph_verse },
        ExtraTable { spec: &PLACE_NAME_ALIAS, rows: alias },
        ExtraTable { spec: &PLACE_NAME_ALIAS_VERSE, rows: alias_verse },
        ExtraTable { spec: &SOURCE_CATEGORY, rows: source_category },
        ExtraTable { spec: &SOURCE_ENTRY, rows: source_entry },
        ExtraTable { spec: &PROVENANCE_ENTRY, rows: provenance_entry },
        ExtraTable { spec: &PROVENANCE_TITLE, rows: provenance_title },
    ])
}

fn text(c: &Col, table: &str) -> Result<String, SqliteError> {
    match c {
        Col::Text(s) => Ok(s.clone()),
        other => Err(SqliteError(format!("{table}: expected TEXT, got {other:?}"))),
    }
}
/// A closed vocabulary read back out of the TEXT column it was written to. A word no
/// member answers to means this artifact was not written by this atlas.
fn word<V>(c: &Col, table: &str, named: fn(&str) -> Option<V>) -> Result<V, SqliteError> {
    let raw = text(c, table)?;
    named(&raw).ok_or_else(|| SqliteError(format!("{table}: '{raw}' names no member of its own vocabulary")))
}

fn opt_word<V>(c: &Col, table: &str, named: fn(&str) -> Option<V>) -> Result<Option<V>, SqliteError> {
    match opt_text(c, table)? {
        None => Ok(None),
        Some(raw) => named(&raw).map(Some).ok_or_else(|| SqliteError(format!("{table}: '{raw}' names no member of its own vocabulary"))),
    }
}

fn opt_text(c: &Col, table: &str) -> Result<Option<String>, SqliteError> {
    match c {
        Col::Null => Ok(None),
        Col::Text(s) => Ok(Some(s.clone())),
        other => Err(SqliteError(format!("{table}: expected TEXT or NULL, got {other:?}"))),
    }
}
fn int(c: &Col, table: &str) -> Result<i64, SqliteError> {
    match c {
        Col::Int(i) => Ok(*i),
        other => Err(SqliteError(format!("{table}: expected INTEGER, got {other:?}"))),
    }
}
fn opt_int(c: &Col, table: &str) -> Result<Option<i32>, SqliteError> {
    match c {
        Col::Null => Ok(None),
        Col::Int(i) => Ok(Some(*i as i32)),
        other => Err(SqliteError(format!("{table}: expected INTEGER or NULL, got {other:?}"))),
    }
}
fn real(c: &Col, table: &str) -> Result<f64, SqliteError> {
    match c {
        Col::Real(f) => Ok(*f),
        Col::Int(i) => Ok(*i as f64),
        other => Err(SqliteError(format!("{table}: expected REAL, got {other:?}"))),
    }
}

/// `AtlasData`'s serving-path fields and `SourcesDocument`, read back from the 21 tables
/// `fold_sidecars` wrote; every other field stays `Default`, since nothing on the serving path
/// reads it. NOT `finish()`ed -- the caller does that.
pub fn unfold(conn: &Connection) -> Result<(AtlasData, SourcesDocument), SqliteError> {
    let rows = |spec: &TableSpec| read_table(conn, spec);
    let mut books: Vec<CanonBook> = Vec::new();
    for r in rows(&CANON_BOOK)? {
        books.push(CanonBook { code: text(&r[1], "canon_book")?, name: text(&r[2], "canon_book")?, chapters: Vec::new() });
    }
    for r in rows(&CANON_CHAPTER_VERSES)? {
        let (b, ch, v) = (int(&r[0], "canon_chapter_verses")?, int(&r[1], "canon_chapter_verses")?, int(&r[2], "canon_chapter_verses")?);
        let book = books.get_mut(b as usize).ok_or_else(|| SqliteError(format!("canon_chapter_verses: book_ord {b} out of range")))?;
        if book.chapters.len() + 1 != ch as usize {
            return Err(SqliteError(format!("canon_chapter_verses: {} chapter {ch} out of sequence", book.code)));
        }
        book.chapters.push(v as u16);
    }
    let canon = Canon { books };
    let mut books_meta = Vec::new();
    for r in rows(&BOOK_META)? {
        let t = "book_meta";
        books_meta.push(BookMeta { book: text(&r[0], t)?, author: text(&r[1], t)?, write_place: opt_text(&r[2], t)?, write_from: opt_int(&r[3], t)?, write_to: opt_int(&r[4], t)? });
    }
    let mut anchors: Vec<(i64, ChronologyAnchor)> = Vec::new();
    for r in rows(&CHRONOLOGY_ANCHOR)? {
        let t = "chronology_anchor";
        anchors.push((
            int(&r[1], t)?,
            ChronologyAnchor {
                id: text(&r[0], t)?,
                label: text(&r[2], t)?,
                year: int(&r[3], t)? as i32,
                event_id: opt_text(&r[4], t)?,
                era_boundary: int(&r[5], t)? != 0,
                source: text(&r[6], t)?,
                note: opt_text(&r[7], t)?,
            },
        ));
    }
    anchors.sort_by_key(|(ord, _)| *ord);
    let chronology_anchors = anchors.into_iter().map(|(_, a)| a).collect();
    // The table's primary key is the book and its consumers index by book, so pk order stands in
    // for the source order.
    let mut book_narration_windows = Vec::new();
    for r in rows(&BOOK_NARRATION_WINDOW)? {
        let t = "book_narration_window";
        book_narration_windows.push(BookNarrationWindow { book: text(&r[0], t)?, from_year: int(&r[1], t)? as i32, to_year: int(&r[2], t)? as i32, note: opt_text(&r[3], t)? });
    }
    let mut landmarks = Vec::new();
    for r in rows(&LANDMARK)? {
        let t = "landmark";
        landmarks.push(Landmark {
            name: text(&r[1], t)?,
            kind: word(&r[2], t, LandmarkKind::named)?,
            lat: real(&r[3], t)?,
            lon: real(&r[4], t)?,
            size: opt_word(&r[5], t, LandmarkSize::named)?,
        });
    }
    let mut land_mask: Vec<Vec<(f64, f64)>> = Vec::new();
    for r in rows(&LAND_MASK_REGION)? {
        let json = text(&r[3], "land_mask_region")?;
        let ring: Vec<(f64, f64)> = serde_json::from_str(&json).map_err(|e| SqliteError(format!("land_mask_region rings_json: {e}")))?;
        land_mask.push(ring);
    }
    let mut parts: Vec<(i64, CatechismPart)> = Vec::new();
    for r in rows(&CATECHISM_PART)? {
        let t = "catechism_part";
        parts.push((int(&r[1], t)?, CatechismPart { id: text(&r[0], t)?, title: text(&r[2], t)?, items: Vec::new() }));
    }
    parts.sort_by_key(|(ord, _)| *ord);
    let mut items: Vec<(String, i64, CatechismItem)> = Vec::new();
    for r in rows(&CATECHISM_ITEM)? {
        let t = "catechism_item";
        items.push((
            text(&r[1], t)?,
            int(&r[2], t)?,
            CatechismItem {
                id: text(&r[0], t)?,
                name: text(&r[3], t)?,
                text: opt_text(&r[4], t)?,
                explanation_heading: text(&r[5], t)?,
                explanation: text(&r[6], t)?,
                where_written: opt_text(&r[7], t)?,
                verses: Vec::new(),
                ref_note: opt_text(&r[8], t)?,
                questions: Vec::new(),
            },
        ));
    }
    let mut item_verses: HashMap<String, Vec<(i64, String)>> = HashMap::new();
    for r in rows(&CATECHISM_ITEM_VERSE)? {
        let t = "catechism_item_verse";
        item_verses.entry(text(&r[0], t)?).or_default().push((int(&r[1], t)?, text(&r[2], t)?));
    }
    let mut questions: HashMap<String, Vec<(i64, CatechismQuestion)>> = HashMap::new();
    for r in rows(&CATECHISM_QUESTION)? {
        let t = "catechism_question";
        questions.entry(text(&r[0], t)?).or_default().push((int(&r[1], t)?, CatechismQuestion { title: text(&r[2], t)?, verses: Vec::new(), source: text(&r[3], t)? }));
    }
    let mut question_verses: HashMap<(String, i64), Vec<(i64, String)>> = HashMap::new();
    for r in rows(&CATECHISM_QUESTION_VERSE)? {
        let t = "catechism_question_verse";
        question_verses.entry((text(&r[0], t)?, int(&r[1], t)?)).or_default().push((int(&r[2], t)?, text(&r[3], t)?));
    }
    items.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    for (part_id, _ord, mut item) in items {
        if let Some(mut vs) = item_verses.remove(&item.id) {
            vs.sort_by_key(|(o, _)| *o);
            item.verses = vs.into_iter().map(|(_, v)| v).collect();
        }
        if let Some(mut qs) = questions.remove(&item.id) {
            qs.sort_by_key(|(o, _)| *o);
            for (qord, mut q) in qs {
                if let Some(mut vs) = question_verses.remove(&(item.id.clone(), qord)) {
                    vs.sort_by_key(|(o, _)| *o);
                    q.verses = vs.into_iter().map(|(_, v)| v).collect();
                }
                item.questions.push(q);
            }
        }
        let part = parts.iter_mut().find(|(_, p)| p.id == part_id).ok_or_else(|| SqliteError(format!("catechism_item {}: unknown part {part_id}", item.id)))?;
        part.1.items.push(item);
    }
    let catechism: Vec<CatechismPart> = parts.into_iter().map(|(_, p)| p).collect();
    let mut place_history: HashMap<String, PlaceHistory> = HashMap::new();
    for r in rows(&PLACE_HISTORY)? {
        let t = "place_history";
        let id = text(&r[0], t)?;
        let claim = |f: &Col, to: &Col, note: &Col, event: &Col| -> Result<Option<PlaceDateClaim>, SqliteError> {
            Ok(match (opt_int(f, t)?, opt_int(to, t)?) {
                (Some(from_year), Some(to_year)) => Some(PlaceDateClaim {
                    when: TimeRange { from_year, to_year },
                    verses: Vec::new(),
                    note: opt_text(note, t)?,
                    event: opt_text(event, t)?.map(EventId::new),
                }),
                _ => None,
            })
        };
        let established = claim(&r[1], &r[2], &r[3], &r[4])?;
        let destroyed = claim(&r[5], &r[6], &r[7], &r[8])?;
        place_history.insert(id.clone(), PlaceHistory { id, names: Vec::new(), established, destroyed });
    }
    for r in rows(&PLACE_HISTORY_NAME)? {
        let t = "place_history_name";
        let id = text(&r[0], t)?;
        let h = place_history.get_mut(&id).ok_or_else(|| SqliteError(format!("{t}: unknown place {id}")))?;
        h.names.push(PlaceNameEntry { name: text(&r[2], t)?, when: TimeRange { from_year: int(&r[3], t)? as i32, to_year: int(&r[4], t)? as i32 }, verses: Vec::new() });
    }
    for r in rows(&PLACE_HISTORY_VERSE)? {
        let t = "place_history_verse";
        let id = text(&r[0], t)?;
        let (kind, owner) = (int(&r[1], t)?, int(&r[2], t)?);
        let sref = text(&r[4], t)?;
        let h = place_history.get_mut(&id).ok_or_else(|| SqliteError(format!("{t}: unknown place {id}")))?;
        match kind {
            0 => h.names.get_mut(owner as usize).ok_or_else(|| SqliteError(format!("{t}: {id} name {owner}")))?.verses.push(sref),
            1 => h.established.as_mut().ok_or_else(|| SqliteError(format!("{t}: {id} has no established claim")))?.verses.push(sref),
            2 => h.destroyed.as_mut().ok_or_else(|| SqliteError(format!("{t}: {id} has no destroyed claim")))?.verses.push(sref),
            other => return Err(SqliteError(format!("{t}: owner_kind {other}"))),
        }
    }
    let mut aliases: HashMap<String, Vec<PlaceNameAlias>> = HashMap::new();
    for r in rows(&PLACE_NAME_ALIAS)? {
        let t = "place_name_alias";
        let id = text(&r[0], t)?;
        let aord = int(&r[1], t)? as usize;
        let list = aliases.entry(id.clone()).or_default();
        while list.len() <= aord {
            list.push(PlaceNameAlias { id: id.clone(), translations: HashMap::new(), verses: Vec::new() });
        }
        list[aord].translations.insert(text(&r[2], t)?, text(&r[3], t)?);
    }
    for r in rows(&PLACE_NAME_ALIAS_VERSE)? {
        let t = "place_name_alias_verse";
        let id = text(&r[0], t)?;
        let aord = int(&r[1], t)? as usize;
        let list = aliases.entry(id.clone()).or_default();
        while list.len() <= aord {
            list.push(PlaceNameAlias { id: id.clone(), translations: HashMap::new(), verses: Vec::new() });
        }
        list[aord].verses.push(text(&r[3], t)?);
    }
    let mut categories: Vec<(i64, SourceCategory)> = Vec::new();
    for r in rows(&SOURCE_CATEGORY)? {
        let t = "source_category";
        categories.push((int(&r[1], t)?, SourceCategory { id: text(&r[0], t)?, label: text(&r[2], t)? }));
    }
    categories.sort_by_key(|(o, _)| *o);
    let mut entries: Vec<(i64, SourceEntry)> = Vec::new();
    for r in rows(&SOURCE_ENTRY)? {
        let t = "source_entry";
        entries.push((
            int(&r[1], t)?,
            SourceEntry {
                id: text(&r[0], t)?,
                category: text(&r[2], t)?,
                title: text(&r[3], t)?,
                what_it_is: text(&r[4], t)?,
                what_we_built: text(&r[5], t)?,
                license: text(&r[6], t)?,
                link: opt_text(&r[7], t)?,
                licenses_row_key: text(&r[8], t)?,
            },
        ));
    }
    entries.sort_by_key(|(o, _)| *o);
    let mut provenances: Vec<(i64, ProvenanceEntry)> = Vec::new();
    for r in rows(&PROVENANCE_ENTRY)? {
        let t = "provenance_entry";
        provenances.push((int(&r[1], t)?, ProvenanceEntry { id: text(&r[0], t)?, source: text(&r[2], t)?, confidence: word(&r[3], t, Confidence::named)?, locator: opt_text(&r[4], t)? }));
    }
    provenances.sort_by_key(|(o, _)| *o);
    let mut provenance_titles = Vec::new();
    for r in rows(&PROVENANCE_TITLE)? {
        let t = "provenance_title";
        provenance_titles.push((text(&r[0], t)?, text(&r[1], t)?));
    }
    let sources = SourcesDocument {
        categories: categories.into_iter().map(|(_, c)| c).collect(),
        sources: entries.into_iter().map(|(_, e)| e).collect(),
        provenances: provenances.into_iter().map(|(_, p)| p).collect(),
    };
    // `AtlasData`'s derived-index fields are private, so there is no struct update syntax here:
    // start from `Default` and set the serving-path fields.
    let mut atlas = AtlasData::default();
    atlas.canon = canon;
    atlas.books_meta = books_meta;
    atlas.landmarks = landmarks;
    atlas.land_mask = land_mask;
    atlas.place_history = place_history;
    atlas.place_name_aliases = aliases;
    atlas.catechism = catechism;
    atlas.chronology_anchors = chronology_anchors;
    atlas.book_narration_windows = book_narration_windows;
    atlas.provenance_titles = ProvenanceTitles::from_rows(provenance_titles).map_err(|e| SqliteError(format!("provenance_title: {e}")))?;
    Ok((atlas, sources))
}
