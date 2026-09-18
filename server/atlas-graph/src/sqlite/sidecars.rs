//! DB-4b: the nine `data/compiled/*.json` sidecars folded into core's
//! tables (spec §5.3 "folded sidecars", amended by the DB-4b plan's
//! judgment calls 3 and 6): lossless -- every field of every loaded struct
//! lands in a column, because DB-5 deletes the JSONs and nothing may be
//! lost on the way. Where the spec's column list was narrower than the
//! struct, the column was added (`chronology_anchor.event_id/era_boundary/
//! source/note`, `provenance_entry.locator`); where the data is narrower
//! than the spec, the column is derived (`canon_book.testament` from
//! `canon::BOOKS` order) or honestly NULL (`land_mask_region.name/
//! ref_note`: the compiled `land-mask.json` is flattened rings, the ETL
//! dropped the region metadata); where the data is wider than the spec's
//! key (one place, two alias rows), the key widened (`alias_ord`).
//!
//! `polities.json` is loaded by `AtlasData` and served by nothing
//! (`/api/polities` reads Polity payloads): retired at DB-5, not folded
//! (spec §5.3's closing note).
//!
//! Sources: `AtlasData::load` (atlas-core `data.rs`) for eight files and
//! `sources.json` -> `SourcesDocument` (atlas-core `sources.rs`, the file
//! `atlas-server::load::load_sources` reads; NOT part of `AtlasData`).

use std::path::Path;

use atlas_core::data::{AtlasData, PlaceDateClaim};
use atlas_core::sources::SourcesDocument;
use atlas_graph_types::canon::{serialize, Value};

use super::extras::{Col, ExtraTable, TableSpec};
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
    columns: &["place_id", "est_from", "est_to", "est_note", "dest_from", "dest_to", "dest_note"],
    pk: &["place_id"],
};
pub static PLACE_HISTORY_NAME: TableSpec = TableSpec {
    name: "place_history_name",
    columns: &["place_id", "ord", "name", "from_year", "to_year"],
    pk: &["place_id", "ord"],
};
pub static PLACE_HISTORY_BLURB: TableSpec = TableSpec {
    name: "place_history_blurb",
    columns: &["place_id", "ord", "text", "from_year", "to_year", "breadth"],
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

/// The 21 sidecar specs in `extra_tables_of(Core)` order (after the five
/// graph-derived tables).
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
    &PLACE_HISTORY_BLURB,
    &PLACE_HISTORY_VERSE,
    &PLACE_NAME_ALIAS,
    &PLACE_NAME_ALIAS_VERSE,
    &SOURCE_CATEGORY,
    &SOURCE_ENTRY,
    &PROVENANCE_ENTRY,
];

/// The two sidecar documents the fold reads.
pub struct Sidecars {
    pub atlas: AtlasData,
    pub sources: SourcesDocument,
}

impl Sidecars {
    /// `None` when `<data_dir>/canon.json` is absent (a fixture directory,
    /// an older snapshot); every other missing or unparsable file is an
    /// error -- a real compiled directory carries all of them.
    pub fn load(data_dir: &Path) -> anyhow::Result<Option<Sidecars>> {
        if !data_dir.join("canon.json").is_file() {
            return Ok(None);
        }
        let atlas = AtlasData::load(data_dir)?.finish();
        let sources_path = data_dir.join("sources.json");
        let text = std::fs::read_to_string(&sources_path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", sources_path.display()))?;
        let sources: SourcesDocument =
            serde_json::from_str(&text).map_err(|e| anyhow::anyhow!("parsing {}: {e}", sources_path.display()))?;
        Ok(Some(Sidecars { atlas, sources }))
    }
}

fn t(s: &str) -> Col {
    Col::Text(s.to_string())
}
fn ot(o: &Option<String>) -> Col {
    o.as_ref().map(|s| Col::Text(s.clone())).unwrap_or(Col::Null)
}
fn oi(o: Option<i32>) -> Col {
    o.map(|x| Col::Int(x as i64)).unwrap_or(Col::Null)
}
fn i(x: impl Into<i64>) -> Col {
    Col::Int(x.into())
}

/// Books before index 39 (GEN..MAL) are the Old Testament, the rest the
/// New (`atlas_core::canon::BOOKS` order; there is no source field).
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

fn claim_cols(c: &Option<PlaceDateClaim>) -> [Col; 3] {
    match c {
        Some(c) => [i(c.when.from_year), i(c.when.to_year), ot(&c.note)],
        None => [Col::Null, Col::Null, Col::Null],
    }
}

/// The nine folds, as 21 tables (rows in source order; `Extras::extend`
/// sorts them into primary-key order).
pub fn fold_sidecars(atlas: &AtlasData, sources: &SourcesDocument) -> Result<Vec<ExtraTable>, SqliteError> {
    // canon.json
    let mut canon_book = Vec::new();
    let mut canon_chapter_verses = Vec::new();
    for (ord, b) in atlas.canon.books.iter().enumerate() {
        canon_book.push(vec![i(ord as i64), t(&b.code), t(&b.name), t(testament_of(ord)), i(b.chapters.len() as i64)]);
        for (ch, verses) in b.chapters.iter().enumerate() {
            canon_chapter_verses.push(vec![i(ord as i64), i(ch as i64 + 1), i(*verses)]);
        }
    }
    // books-meta.json
    let book_meta = atlas
        .books_meta
        .iter()
        .map(|m| vec![t(&m.book), t(&m.author), ot(&m.write_place), oi(m.write_from), oi(m.write_to)])
        .collect();
    // chronology-anchors.json
    let chronology_anchor = atlas
        .chronology_anchors
        .iter()
        .enumerate()
        .map(|(ord, a)| {
            vec![t(&a.id), i(ord as i64), t(&a.label), i(a.year), ot(&a.event_id), i(a.era_boundary as i64), t(&a.source), ot(&a.note)]
        })
        .collect();
    // book-narration-windows.json
    let book_narration_window =
        atlas.book_narration_windows.iter().map(|w| vec![t(&w.book), i(w.from_year), i(w.to_year), ot(&w.note)]).collect();
    // landmarks.json
    let landmark = atlas
        .landmarks
        .iter()
        .enumerate()
        .map(|(ord, l)| vec![i(ord as i64), t(&l.name), t(&l.kind), Col::Real(l.lat), Col::Real(l.lon), ot(&l.size)])
        .collect();
    // land-mask.json (flattened rings; names lost at ETL -- NULL, honestly)
    let mut land_mask_region = Vec::with_capacity(atlas.land_mask.len());
    for (ord, ring) in atlas.land_mask.iter().enumerate() {
        land_mask_region.push(vec![i(ord as i64), Col::Null, Col::Null, Col::Text(ring_json(ring)?)]);
    }
    // catechism.json
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
    // place-history.json
    let (mut ph, mut ph_name, mut ph_blurb, mut ph_verse) = (vec![], vec![], vec![], vec![]);
    let mut histories: Vec<_> = atlas.place_history.values().collect();
    histories.sort_by(|a, b| a.id.as_bytes().cmp(b.id.as_bytes()));
    for h in histories {
        let [ef, et, en] = claim_cols(&h.established);
        let [df, dt, dn] = claim_cols(&h.destroyed);
        ph.push(vec![t(&h.id), ef, et, en, df, dt, dn]);
        for (ord, n) in h.names.iter().enumerate() {
            ph_name.push(vec![t(&h.id), i(ord as i64), t(&n.name), i(n.when.from_year), i(n.when.to_year)]);
            for (vord, v) in n.verses.iter().enumerate() {
                ph_verse.push(vec![t(&h.id), i(0i64), i(ord as i64), i(vord as i64), t(v)]);
            }
        }
        for (ord, b) in h.blurbs.iter().enumerate() {
            ph_blurb.push(vec![t(&h.id), i(ord as i64), t(&b.text), i(b.when.from_year), i(b.when.to_year), t(&b.breadth)]);
        }
        for (kind, claim) in [(1i64, &h.established), (2i64, &h.destroyed)] {
            if let Some(c) = claim {
                for (vord, v) in c.verses.iter().enumerate() {
                    ph_verse.push(vec![t(&h.id), i(kind), i(0i64), i(vord as i64), t(v)]);
                }
            }
        }
    }
    // place-names-kjv.json (one place may carry several alias rows)
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
    // sources.json
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
        .map(|(ord, p)| vec![t(&p.id), i(ord as i64), t(&p.source), t(&p.confidence), ot(&p.locator)])
        .collect();

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
        ExtraTable { spec: &PLACE_HISTORY_BLURB, rows: ph_blurb },
        ExtraTable { spec: &PLACE_HISTORY_VERSE, rows: ph_verse },
        ExtraTable { spec: &PLACE_NAME_ALIAS, rows: alias },
        ExtraTable { spec: &PLACE_NAME_ALIAS_VERSE, rows: alias_verse },
        ExtraTable { spec: &SOURCE_CATEGORY, rows: source_category },
        ExtraTable { spec: &SOURCE_ENTRY, rows: source_entry },
        ExtraTable { spec: &PROVENANCE_ENTRY, rows: provenance_entry },
    ])
}
