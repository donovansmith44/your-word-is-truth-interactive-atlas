//! DB-2b: the row families authored in the `core` section (spec §5.3),
//! plus the generic `contains_*` pair the kjv (`contains_bible`) and
//! concord (`contains_concord`) sections reuse.

use std::collections::BTreeSet;

use atlas_graph_types::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis};
use atlas_graph_types::edge::{
    Analogue, Attests, CatechismLink, ContainerContent, Contains, Corresponds, Fulfills, LocatedAt,
    MentionedEntity, Mentions, NamedAfter, Namesake, Succession, TemporalAdjacency, Typology, ParentOf, Participates, Partners};
use atlas_graph_types::id::{AnchorId, ContainerNodeId, EraId, EventId, NodeId};
use atlas_graph_types::text::{
    BibleTag, ConcordRef, Corpus, Locus, LocusSet, TokenSpan, VerseRef,
};
use rusqlite::types::Value;
use rusqlite::{Connection, Transaction};

use super::super::columns::{
    bible_locus_values, bible_range_values, col, read_bible_locus, read_bible_range, read_span,
    read_text_locus, span_values, text_locus_values, JustificationWriter,
};
use super::super::SqliteError;
use super::{authored, id_col, insert, int, opt_text, read_all, read_justification_at, text, D};

// ------------------------------------------------------------ contains_*

/// The three integer columns of a `contains_*_locus` row, per corpus ref.
pub trait Abc: Sized {
    fn abc(&self) -> (i64, i64, i64);
    fn from_abc(a: i64, b: i64, c: i64) -> Result<Self, SqliteError>;
}
impl Abc for VerseRef {
    fn abc(&self) -> (i64, i64, i64) {
        (i64::from(self.book), i64::from(self.chapter), i64::from(self.verse))
    }
    fn from_abc(a: i64, b: i64, c: i64) -> Result<Self, SqliteError> {
        let narrow = |v: i64, w: &str| -> Result<u16, SqliteError> {
            u16::try_from(v).map_err(|_| SqliteError(format!("contains locus {w}: {v} out of range")))
        };
        Ok(VerseRef {
            book: u8::try_from(a).map_err(|_| SqliteError(format!("contains locus book: {a} out of range")))?,
            chapter: narrow(b, "chapter")?,
            verse: narrow(c, "verse")?,
        })
    }
}
impl Abc for ConcordRef {
    fn abc(&self) -> (i64, i64, i64) {
        (i64::from(self.part), i64::from(self.article), i64::from(self.paragraph))
    }
    fn from_abc(a: i64, b: i64, c: i64) -> Result<Self, SqliteError> {
        let narrow = |v: i64, w: &str| -> Result<u16, SqliteError> {
            u16::try_from(v).map_err(|_| SqliteError(format!("contains locus {w}: {v} out of range")))
        };
        Ok(ConcordRef {
            part: u8::try_from(a).map_err(|_| SqliteError(format!("contains locus part: {a} out of range")))?,
            article: narrow(b, "article")?,
            paragraph: narrow(c, "paragraph")?,
        })
    }
}

const COLS_CONTAINS: &str = "container_id, child_container_id, provenance, justification_id";
const COLS_CONTAINS_LOCUS: &str = "a, b, c, layer, start, end_";

/// `table` is `contains_bible` or `contains_concord`; the sub-table is
/// `<table>_locus` (`contains_id = id`, `ord` = set order).
pub fn insert_contains<C: Corpus>(
    tx: &Transaction,
    jw: &mut JustificationWriter,
    ord: i64,
    row: &Contains<C>,
    table: &str,
) -> Result<(), SqliteError>
where
    C::Ref: Abc,
{
    let child = match &row.content {
        ContainerContent::Container(c) => text(&c.0),
        ContainerContent::Loci(_) => Value::Null,
    };
    let j = authored(tx, jw, &row.justification)?;
    insert(tx, table, COLS_CONTAINS, ord, vec![text(&row.container.0), child, text(&row.provenance), j])?;
    if let ContainerContent::Loci(set) = &row.content {
        let sql = format!("INSERT INTO {table}_locus (contains_id, ord, {COLS_CONTAINS_LOCUS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?)");
        let mut stmt = tx.prepare_cached(&sql)?;
        for (i, l) in set.0.iter().enumerate() {
            let (a, b, c) = l.unit.abc();
            let [layer, start, end] = span_values(&l.span);
            stmt.execute(rusqlite::params![ord, i as i64, a, b, c, layer, start, end])?;
        }
    }
    Ok(())
}

pub fn read_contains<C: Corpus>(conn: &Connection, table: &str) -> Result<Vec<(i64, Contains<C>)>, SqliteError>
where
    C::Ref: Abc,
    Locus<C>: Ord,
{
    let locus_sql = format!("SELECT {COLS_CONTAINS_LOCUS} FROM {table}_locus WHERE contains_id = ? ORDER BY ord");
    read_all(conn, table, COLS_CONTAINS, |row| {
        let id: i64 = row.get(0)?;
        let container: ContainerNodeId = id_col(row, D, "container_id")?;
        let child: Option<String> = col(row, D + 1, "child_container_id")?;
        let provenance: String = col(row, D + 2, "provenance")?;
        let justification = read_justification_at(conn, row, D + 3)?;
        let content = match child {
            Some(c) => ContainerContent::Container(NodeId::new(c)),
            None => {
                let mut stmt = conn.prepare_cached(&locus_sql)?;
                let mut rows = stmt.query([id])?;
                let mut set: BTreeSet<Locus<C>> = BTreeSet::new();
                while let Some(r) = rows.next()? {
                    let a: i64 = col(r, 0, "a")?;
                    let b: i64 = col(r, 1, "b")?;
                    let c: i64 = col(r, 2, "c")?;
                    let unit = C::Ref::from_abc(a, b, c)?;
                    let span: Option<TokenSpan> = read_span(r, 3)?;
                    set.insert(Locus { unit, span });
                }
                ContainerContent::Loci(LocusSet(set))
            }
        };
        Ok(Contains { container, content, provenance, justification })
    })
}

// --------------------------------------------------------------- attests

const COLS_ATTESTS: &str = "event_id, \
 att_from_corpus, att_from_a, att_from_b, att_from_c, att_from_layer, att_from_start, att_from_end, \
 att_to_corpus, att_to_a, att_to_b, att_to_c, att_to_layer, att_to_start, att_to_end, \
 provenance, justification_id";

pub fn insert_attests(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &Attests) -> Result<(), SqliteError> {
    let mut v = vec![text(&row.event.0)];
    v.extend(bible_range_values(&row.attestation));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "attests", COLS_ATTESTS, ord, v)
}

pub fn read_attests(conn: &Connection) -> Result<Vec<(i64, Attests)>, SqliteError> {
    read_all(conn, "attests", COLS_ATTESTS, |row| {
        Ok(Attests {
            event: id_col(row, D, "event_id")?,
            attestation: read_bible_range(row, D + 1)?,
            provenance: col(row, D + 15, "provenance")?,
            justification: read_justification_at(conn, row, D + 16)?,
        })
    })
}

// ------------------------------------------------------------ succession

const COLS_SUCCESSION: &str = "narrative_id, provenance, justification_id";

pub fn insert_succession(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &Succession) -> Result<(), SqliteError> {
    let j = authored(tx, jw, &row.justification)?;
    insert(tx, "succession", COLS_SUCCESSION, ord, vec![text(&row.narrative.0), text(&row.provenance), j])?;
    let mut stmt = tx.prepare_cached("INSERT INTO succession_step (succession_id, ord, event_id) VALUES (?, ?, ?)")?;
    for (i, e) in row.chain.iter().enumerate() {
        stmt.execute(rusqlite::params![ord, i as i64, e.0])?;
    }
    Ok(())
}

pub fn read_succession(conn: &Connection) -> Result<Vec<(i64, Succession)>, SqliteError> {
    read_all(conn, "succession", COLS_SUCCESSION, |row| {
        let id: i64 = row.get(0)?;
        let narrative = id_col(row, D, "narrative_id")?;
        let provenance: String = col(row, D + 1, "provenance")?;
        let justification = read_justification_at(conn, row, D + 2)?;
        let mut stmt = conn.prepare_cached("SELECT event_id FROM succession_step WHERE succession_id = ? ORDER BY ord")?;
        let chain: Vec<EventId> = stmt
            .query_map([id], |r| r.get::<_, String>(0))?
            .map(|r| r.map(EventId::new))
            .collect::<Result<_, _>>()?;
        Succession::new(narrative, chain, provenance, justification)
            .map_err(|e| SqliteError(format!("succession {id}: {e:?}")))
    })
}

// -------------------------------------------------------------- dated_by

const COLS_DATED_BY: &str = "event_id, placement_kind, anchor_id, prior_event_id, era_id, \
 years, months, days, year_of_reign, basis, provenance, justification_id";

fn duration_values(d: &Duration) -> [Value; 3] {
    [int(i64::from(d.years)), int(i64::from(d.months)), int(i64::from(d.days))]
}

pub fn insert_dated_by(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &DatedBy) -> Result<(), SqliteError> {
    let n = Value::Null;
    let (kind, anchor, prior, era, dur, yor): (i64, Value, Value, Value, [Value; 3], Value) = match &row.placement {
        DatePlacement::AnchorBinding { anchor, offset } => {
            (0, text(&anchor.0), n.clone(), n.clone(), duration_values(offset), n.clone())
        }
        DatePlacement::ReignYear { reign, year_of_reign } => (
            1,
            text(&reign.0),
            n.clone(),
            n.clone(),
            [n.clone(), n.clone(), n.clone()],
            int(i64::from(*year_of_reign)),
        ),
        DatePlacement::SequenceAfter { prior, spacing } => {
            (2, n.clone(), text(&prior.0), n.clone(), duration_values(spacing), n.clone())
        }
        DatePlacement::EraOnly { era } => (3, n.clone(), n.clone(), text(&era.0), [n.clone(), n.clone(), n.clone()], n.clone()),
    };
    let basis = match row.basis {
        PlacementBasis::Textual => 0,
        PlacementBasis::Traditional => 1,
    };
    let [y, m, d] = dur;
    let j = authored(tx, jw, &row.justification)?;
    insert(
        tx,
        "dated_by",
        COLS_DATED_BY,
        ord,
        vec![text(&row.event.0), int(kind), anchor, prior, era, y, m, d, yor, int(basis), text(&row.provenance), j],
    )
}

pub fn read_dated_by(conn: &Connection) -> Result<Vec<(i64, DatedBy)>, SqliteError> {
    read_all(conn, "dated_by", COLS_DATED_BY, |row| {
        let event = id_col(row, D, "event_id")?;
        let kind: i64 = col(row, D + 1, "placement_kind")?;
        let anchor: Option<String> = col(row, D + 2, "anchor_id")?;
        let prior: Option<String> = col(row, D + 3, "prior_event_id")?;
        let era: Option<String> = col(row, D + 4, "era_id")?;
        let years: Option<i64> = col(row, D + 5, "years")?;
        let months: Option<i64> = col(row, D + 6, "months")?;
        let days: Option<i64> = col(row, D + 7, "days")?;
        let yor: Option<i64> = col(row, D + 8, "year_of_reign")?;
        let basis: i64 = col(row, D + 9, "basis")?;
        let provenance: String = col(row, D + 10, "provenance")?;
        let justification = read_justification_at(conn, row, D + 11)?;
        let need = |v: Option<String>, what: &str| v.ok_or_else(|| SqliteError(format!("dated_by kind {kind}: {what} is NULL")));
        let duration = || -> Result<Duration, SqliteError> {
            let y = years.ok_or_else(|| SqliteError(format!("dated_by kind {kind}: years is NULL")))?;
            let m = months.ok_or_else(|| SqliteError(format!("dated_by kind {kind}: months is NULL")))?;
            let d = days.ok_or_else(|| SqliteError(format!("dated_by kind {kind}: days is NULL")))?;
            Ok(Duration {
                years: i32::try_from(y).map_err(|_| SqliteError(format!("dated_by years {y} out of range")))?,
                months: u8::try_from(m).map_err(|_| SqliteError(format!("dated_by months {m} out of range")))?,
                days: u8::try_from(d).map_err(|_| SqliteError(format!("dated_by days {d} out of range")))?,
            })
        };
        let placement = match kind {
            0 => DatePlacement::AnchorBinding { anchor: AnchorId::new(need(anchor, "anchor_id")?), offset: duration()? },
            1 => DatePlacement::ReignYear {
                reign: AnchorId::new(need(anchor, "anchor_id")?),
                year_of_reign: u8::try_from(yor.ok_or_else(|| SqliteError("dated_by kind 1: year_of_reign is NULL".into()))?)
                    .map_err(|_| SqliteError("dated_by year_of_reign out of range".into()))?,
            },
            2 => DatePlacement::SequenceAfter { prior: EventId::new(need(prior, "prior_event_id")?), spacing: duration()? },
            3 => DatePlacement::EraOnly { era: EraId::new(need(era, "era_id")?) },
            other => return Err(SqliteError(format!("dated_by placement_kind {other} is not 0..3"))),
        };
        let basis = match basis {
            0 => PlacementBasis::Textual,
            1 => PlacementBasis::Traditional,
            other => return Err(SqliteError(format!("dated_by basis {other} is not 0|1"))),
        };
        Ok(DatedBy { event, placement, basis, justification, provenance })
    })
}

// ------------------------------------------------------------ located_at

const COLS_LOCATED_AT: &str = "event_id, place_id, provenance, justification_id";

pub fn insert_located_at(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &LocatedAt) -> Result<(), SqliteError> {
    let j = authored(tx, jw, &row.justification)?;
    insert(tx, "located_at", COLS_LOCATED_AT, ord, vec![text(&row.event.0), text(&row.place.0), text(&row.provenance), j])
}

pub fn read_located_at(conn: &Connection) -> Result<Vec<(i64, LocatedAt)>, SqliteError> {
    read_all(conn, "located_at", COLS_LOCATED_AT, |row| {
        Ok(LocatedAt {
            event: id_col(row, D, "event_id")?,
            place: id_col(row, D + 1, "place_id")?,
            provenance: col(row, D + 2, "provenance")?,
            justification: read_justification_at(conn, row, D + 3)?,
        })
    })
}

// -------------------------------------------------------------- fulfills

const COLS_FULFILLS: &str = "\
 prophecy_from_corpus, prophecy_from_a, prophecy_from_b, prophecy_from_c, prophecy_from_layer, prophecy_from_start, prophecy_from_end, \
 prophecy_to_corpus, prophecy_to_a, prophecy_to_b, prophecy_to_c, prophecy_to_layer, prophecy_to_start, prophecy_to_end, \
 fulfillment_from_corpus, fulfillment_from_a, fulfillment_from_b, fulfillment_from_c, fulfillment_from_layer, fulfillment_from_start, fulfillment_from_end, \
 fulfillment_to_corpus, fulfillment_to_a, fulfillment_to_b, fulfillment_to_c, fulfillment_to_layer, fulfillment_to_start, fulfillment_to_end, \
 provenance, justification_id";

pub fn insert_fulfills(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &Fulfills) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(30);
    v.extend(bible_range_values(&row.prophecy));
    v.extend(bible_range_values(&row.fulfillment));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "fulfills", COLS_FULFILLS, ord, v)
}

pub fn read_fulfills(conn: &Connection) -> Result<Vec<(i64, Fulfills)>, SqliteError> {
    read_all(conn, "fulfills", COLS_FULFILLS, |row| {
        Ok(Fulfills {
            prophecy: read_bible_range(row, D)?,
            fulfillment: read_bible_range(row, D + 14)?,
            provenance: col(row, D + 28, "provenance")?,
            justification: read_justification_at(conn, row, D + 29)?,
        })
    })
}

// -------------------------------------------------------------- typology

const COLS_TYPOLOGY: &str = "\
 type_from_corpus, type_from_a, type_from_b, type_from_c, type_from_layer, type_from_start, type_from_end, \
 type_to_corpus, type_to_a, type_to_b, type_to_c, type_to_layer, type_to_start, type_to_end, \
 antitype_from_corpus, antitype_from_a, antitype_from_b, antitype_from_c, antitype_from_layer, antitype_from_start, antitype_from_end, \
 antitype_to_corpus, antitype_to_a, antitype_to_b, antitype_to_c, antitype_to_layer, antitype_to_start, antitype_to_end, \
 note, provenance, justification_id";

pub fn insert_typology(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &Typology) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(31);
    v.extend(bible_range_values(&row.type_passage));
    v.extend(bible_range_values(&row.antitype_passage));
    v.push(opt_text(&row.note));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "typology", COLS_TYPOLOGY, ord, v)
}

pub fn read_typology(conn: &Connection) -> Result<Vec<(i64, Typology)>, SqliteError> {
    read_all(conn, "typology", COLS_TYPOLOGY, |row| {
        Ok(Typology {
            type_passage: read_bible_range(row, D)?,
            antitype_passage: read_bible_range(row, D + 14)?,
            note: col(row, D + 28, "note")?,
            provenance: col(row, D + 29, "provenance")?,
            justification: read_justification_at(conn, row, D + 30)?,
        })
    })
}

// ----------------------------------------------------------- named_after

const COLS_NAMED_AFTER: &str = "namesake_kind, namesake_id, eponym_id, provenance, justification_id";

pub fn insert_named_after(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &NamedAfter) -> Result<(), SqliteError> {
    let (kind, raw) = match &row.namesake {
        Namesake::PeopleGroup(g) => (0, &g.0),
        Namesake::Place(p) => (1, &p.0),
        Namesake::Polity(p) => (2, &p.0),
    };
    let j = authored(tx, jw, &row.justification)?;
    insert(tx, "named_after", COLS_NAMED_AFTER, ord, vec![int(kind), text(raw), text(&row.eponym.0), text(&row.provenance), j])
}

pub fn read_named_after(conn: &Connection) -> Result<Vec<(i64, NamedAfter)>, SqliteError> {
    read_all(conn, "named_after", COLS_NAMED_AFTER, |row| {
        let kind: i64 = col(row, D, "namesake_kind")?;
        let raw: String = col(row, D + 1, "namesake_id")?;
        let namesake = match kind {
            0 => Namesake::PeopleGroup(NodeId::new(raw)),
            1 => Namesake::Place(NodeId::new(raw)),
            2 => Namesake::Polity(NodeId::new(raw)),
            other => return Err(SqliteError(format!("named_after namesake_kind {other} is not 0|1|2"))),
        };
        Ok(NamedAfter {
            namesake,
            eponym: id_col(row, D + 2, "eponym_id")?,
            provenance: col(row, D + 3, "provenance")?,
            justification: read_justification_at(conn, row, D + 4)?,
        })
    })
}

// ------------------------------------------------------------- catechism

const COLS_CATECHISM: &str = "locus_corpus, locus_a, locus_b, locus_c, locus_layer, locus_start, locus_end, \
 item_id, provenance, justification_id";

pub fn insert_catechism(tx: &Transaction, jw: &mut JustificationWriter, ord: i64, row: &CatechismLink) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(10);
    v.extend(text_locus_values(&row.locus));
    v.push(text(&row.item.0));
    v.push(text(&row.provenance));
    v.push(authored(tx, jw, &row.justification)?);
    insert(tx, "catechism", COLS_CATECHISM, ord, v)
}

pub fn read_catechism(conn: &Connection) -> Result<Vec<(i64, CatechismLink)>, SqliteError> {
    read_all(conn, "catechism", COLS_CATECHISM, |row| {
        Ok(CatechismLink {
            locus: read_text_locus(row, D)?,
            item: id_col(row, D + 7, "item_id")?,
            provenance: col(row, D + 8, "provenance")?,
            justification: read_justification_at(conn, row, D + 9)?,
        })
    })
}

// -------------------------------------------------------------- mentions

const COLS_MENTIONS: &str = "locus_corpus, locus_a, locus_b, locus_c, locus_layer, locus_start, locus_end, \
 entity_kind, entity_id, provenance";

pub fn insert_mentions(tx: &Transaction, ord: i64, row: &Mentions) -> Result<(), SqliteError> {
    let (kind, raw) = match &row.entity {
        MentionedEntity::Place(p) => (0, &p.0),
        MentionedEntity::Person(p) => (1, &p.0),
        MentionedEntity::PeopleGroup(g) => (2, &g.0),
        MentionedEntity::Event(e) => (3, &e.0),
    };
    let mut v: Vec<Value> = Vec::with_capacity(10);
    v.extend(text_locus_values(&row.locus));
    v.push(int(kind));
    v.push(text(raw));
    v.push(text(&row.provenance));
    insert(tx, "mentions", COLS_MENTIONS, ord, v)
}

pub fn read_mentions(conn: &Connection) -> Result<Vec<(i64, Mentions)>, SqliteError> {
    read_all(conn, "mentions", COLS_MENTIONS, |row| {
        let kind: i64 = col(row, D + 7, "entity_kind")?;
        let raw: String = col(row, D + 8, "entity_id")?;
        let entity = match kind {
            0 => MentionedEntity::Place(NodeId::new(raw)),
            1 => MentionedEntity::Person(NodeId::new(raw)),
            2 => MentionedEntity::PeopleGroup(NodeId::new(raw)),
            3 => MentionedEntity::Event(NodeId::new(raw)),
            other => return Err(SqliteError(format!("mentions entity_kind {other} is not 0..3"))),
        };
        Ok(Mentions { locus: read_text_locus(row, D)?, entity, provenance: col(row, D + 9, "provenance")? })
    })
}

// ----------------------------------------------------- corresponds_bible

const COLS_CORRESPONDS_BIBLE: &str = "a_corpus, a_a, a_b, a_c, a_layer, a_start, a_end, \
 b_corpus, b_a, b_b, b_c, b_layer, b_start, b_end, provenance";

pub fn insert_corresponds_bible(tx: &Transaction, ord: i64, row: &Corresponds<BibleTag>) -> Result<(), SqliteError> {
    let mut v: Vec<Value> = Vec::with_capacity(15);
    v.extend(bible_locus_values(&row.a));
    v.extend(bible_locus_values(&row.b));
    v.push(text(&row.provenance));
    insert(tx, "corresponds_bible", COLS_CORRESPONDS_BIBLE, ord, v)
}

pub fn read_corresponds_bible(conn: &Connection) -> Result<Vec<(i64, Corresponds<BibleTag>)>, SqliteError> {
    read_all(conn, "corresponds_bible", COLS_CORRESPONDS_BIBLE, |row| {
        Ok(Corresponds { a: read_bible_locus(row, D)?, b: read_bible_locus(row, D + 7)?, provenance: col(row, D + 14, "provenance")? })
    })
}

// ---------------------------------------------------- temporal_adjacency

const COLS_TEMPORAL_ADJACENCY: &str = "earlier_id, later_id, provenance";

pub fn insert_temporal_adjacency(tx: &Transaction, ord: i64, row: &TemporalAdjacency) -> Result<(), SqliteError> {
    insert(tx, "temporal_adjacency", COLS_TEMPORAL_ADJACENCY, ord, vec![text(&row.earlier.0), text(&row.later.0), text(&row.provenance)])
}

pub fn read_temporal_adjacency(conn: &Connection) -> Result<Vec<(i64, TemporalAdjacency)>, SqliteError> {
    read_all(conn, "temporal_adjacency", COLS_TEMPORAL_ADJACENCY, |row| {
        Ok(TemporalAdjacency {
            earlier: id_col(row, D, "earlier_id")?,
            later: id_col(row, D + 1, "later_id")?,
            provenance: col(row, D + 2, "provenance")?,
        })
    })
}

// -------------------------------------------------------------- analogue

const COLS_ANALOGUE: &str = "a_id, b_id, provenance";

pub fn insert_analogue(tx: &Transaction, ord: i64, row: &Analogue) -> Result<(), SqliteError> {
    insert(tx, "analogue", COLS_ANALOGUE, ord, vec![text(&row.a.0), text(&row.b.0), text(&row.provenance)])
}

pub fn read_analogue(conn: &Connection) -> Result<Vec<(i64, Analogue)>, SqliteError> {
    read_all(conn, "analogue", COLS_ANALOGUE, |row| {
        Ok(Analogue { a: id_col(row, D, "a_id")?, b: id_col(row, D + 1, "b_id")?, provenance: col(row, D + 2, "provenance")? })
    })
}

// ------------------------------------------------------ D5: kinship rows

const COLS_PARENT_OF: &str = "parent_id, child_id, provenance";

pub fn insert_parent_of(tx: &Transaction, ord: i64, row: &ParentOf) -> Result<(), SqliteError> {
    insert(tx, "parent_of", COLS_PARENT_OF, ord, vec![text(&row.parent.0), text(&row.child.0), text(&row.provenance)])
}

pub fn read_parent_of(conn: &Connection) -> Result<Vec<(i64, ParentOf)>, SqliteError> {
    read_all(conn, "parent_of", COLS_PARENT_OF, |row| {
        Ok(ParentOf { parent: id_col(row, D, "parent_id")?, child: id_col(row, D + 1, "child_id")?, provenance: col(row, D + 2, "provenance")? })
    })
}

const COLS_PARTNERS: &str = "a_id, b_id, provenance";

pub fn insert_partners(tx: &Transaction, ord: i64, row: &Partners) -> Result<(), SqliteError> {
    insert(tx, "partners", COLS_PARTNERS, ord, vec![text(&row.a.0), text(&row.b.0), text(&row.provenance)])
}

pub fn read_partners(conn: &Connection) -> Result<Vec<(i64, Partners)>, SqliteError> {
    read_all(conn, "partners", COLS_PARTNERS, |row| {
        Ok(Partners { a: id_col(row, D, "a_id")?, b: id_col(row, D + 1, "b_id")?, provenance: col(row, D + 2, "provenance")? })
    })
}

const COLS_PARTICIPATES: &str = "person_id, event_id, provenance";

pub fn insert_participates(tx: &Transaction, ord: i64, row: &Participates) -> Result<(), SqliteError> {
    insert(tx, "participates", COLS_PARTICIPATES, ord, vec![text(&row.person.0), text(&row.event.0), text(&row.provenance)])
}

pub fn read_participates(conn: &Connection) -> Result<Vec<(i64, Participates)>, SqliteError> {
    read_all(conn, "participates", COLS_PARTICIPATES, |row| {
        Ok(Participates { person: id_col(row, D, "person_id")?, event: id_col(row, D + 1, "event_id")?, provenance: col(row, D + 2, "provenance")? })
    })
}
