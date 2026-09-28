//! The serving companions read out of the section tables once at startup. Only the cross-refs
//! (344k rows) are large enough to stay a per-request seek instead of being read whole.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use atlas_core::data::CrossRef;
use atlas_core::refs::ScriptureRef;
use atlas_graph_types::canon::RowFamily;
use atlas_graph_types::chrono::{PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};
use atlas_graph_types::id::NodeKind;
use rusqlite::Connection;

use super::partition::node_kind_ordinal;
use super::SqliteError;
use crate::build::BuildStats;
use crate::event_world::{ChronologyDerivation, EventWorldStats, SourceEventMeta};
use crate::heading::Heading;
use crate::kjv_adapter::dot_ref;
use crate::sections::{row_tables_of, Section};

fn year(y: i64) -> Result<Year, SqliteError> {
    Year::new(y as i32).map_err(|e| SqliteError(format!("event_date year {y}: {e:?}")))
}

fn opt_u8(v: Option<i64>) -> Option<u8> {
    v.map(|x| x as u8)
}

/// `core.event_date` -> the chronology companion: `source_meta` only where the curated pair is
/// present (a NULL column means no entry), and `placements` stays empty because nothing on the
/// serving path reads it.
pub fn load_chronology(conn: &Connection) -> Result<ChronologyDerivation, SqliteError> {
    let mut stmt = conn.prepare(
        "SELECT event_id, from_year, to_year, from_month, from_day, to_month, to_day, seq, basis, meta_to_year, order_key \
         FROM event_date ORDER BY seq",
    )?;
    let mut rows = stmt.query([])?;
    let mut out = ChronologyDerivation::default();
    while let Some(r) = rows.next()? {
        let id: String = r.get(0)?;
        let basis: i64 = r.get(8)?;
        let placement = ResolvedPlacement {
            date: ResolvedDate {
                from: TimePoint { year: year(r.get(1)?)?, month: opt_u8(r.get(3)?), day: opt_u8(r.get(4)?) },
                to: TimePoint { year: year(r.get(2)?)?, month: opt_u8(r.get(5)?), day: opt_u8(r.get(6)?) },
            },
            seq: SeqKey(r.get::<_, i64>(7)? as u32),
            basis: match basis {
                0 => PlacementBasis::Textual,
                1 => PlacementBasis::Traditional,
                other => return Err(SqliteError(format!("event_date {id}: basis {other}"))),
            },
        };
        let meta_to_year: Option<i64> = r.get(9)?;
        let order_key: Option<i64> = r.get(10)?;
        if let (Some(t), Some(k)) = (meta_to_year, order_key) {
            out.source_meta.insert(id.clone(), SourceEventMeta { to_year: t as i32, order_key: k as i32 });
        }
        out.resolved.insert(id.clone(), placement);
        out.order.push(id);
    }
    Ok(out)
}

/// `core.heading_index` -> dot-ref -> the one heading that wins at that ref.
pub fn load_heading_index(conn: &Connection) -> Result<BTreeMap<String, Heading>, SqliteError> {
    let mut stmt = conn.prepare("SELECT book, chapter, verse, event_id, title, kind, continuation FROM heading_index")?;
    let mut rows = stmt.query([])?;
    let mut out = BTreeMap::new();
    while let Some(r) = rows.next()? {
        let (b, c, v): (i64, i64, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
        out.insert(
            dot_ref(b as u8, c as u16, v as u16),
            Heading { event_id: r.get(3)?, title: r.get(4)?, kind: r.get(5)?, is_continuation: r.get::<_, i64>(6)? != 0 },
        );
    }
    Ok(out)
}

/// `kjv.red_letter_span` -> dot-ref -> char-offset spans, in `ord` order.
pub fn load_red_letter_spans(conn: &Connection) -> Result<HashMap<String, Vec<(usize, usize)>>, SqliteError> {
    let mut stmt = conn.prepare("SELECT book, chapter, verse, start, end_ FROM kjv.red_letter_span ORDER BY book, chapter, verse, ord")?;
    let mut rows = stmt.query([])?;
    let mut out: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
    while let Some(r) = rows.next()? {
        let (b, c, v): (i64, i64, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
        let (s, e): (i64, i64) = (r.get(3)?, r.get(4)?);
        out.entry(dot_ref(b as u8, c as u16, v as u16)).or_default().push((s as usize, e as usize));
    }
    Ok(out)
}

/// `core.succession` + `succession_step` -> narrative id -> its chain of node ids.
pub fn load_narrative_legs(conn: &Connection) -> Result<BTreeMap<String, Vec<String>>, SqliteError> {
    let mut stmt = conn.prepare(
        "SELECT s.narrative_id, st.event_id FROM succession s JOIN succession_step st ON st.succession_id = s.id ORDER BY s.id, st.ord",
    )?;
    let mut rows = stmt.query([])?;
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    while let Some(r) = rows.next()? {
        let narrative: String = r.get(0)?;
        out.entry(narrative).or_default().push(r.get(1)?);
    }
    Ok(out)
}

fn schema_of(section: Section) -> &'static str {
    match section {
        Section::Core => "main",
        other => other.name(),
    }
}

/// The provenance families of every present section. Every family is a key even when its section
/// is absent, so the index lists every family it swept, inhabited or not.
pub fn load_provenance_families(
    conn: &Connection,
    present: &[Section],
) -> Result<BTreeMap<&'static str, BTreeSet<String>>, SqliteError> {
    let mut out: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT DISTINCT provenance FROM all_node")?;
        let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
        let set = out.entry("nodes").or_default();
        for id in ids {
            set.insert(id?);
        }
    }
    for section in present {
        for family in row_tables_of(*section) {
            let name: &'static str = family.name();
            let mut stmt = conn.prepare(&format!("SELECT DISTINCT provenance FROM {}.{}", schema_of(*section), name))?;
            let ids = stmt.query_map([], |r| r.get::<_, String>(0))?;
            let set = out.entry(name).or_default();
            for id in ids {
                set.insert(id?);
            }
        }
    }
    for f in RowFamily::ALL {
        out.entry(f.name()).or_default();
    }
    Ok(out)
}

fn count(conn: &Connection, sql: &str) -> Result<usize, SqliteError> {
    let n: i64 = conn.query_row(sql, [], |r| r.get(0))?;
    Ok(n as usize)
}

fn count_kind(conn: &Connection, kind: NodeKind) -> Result<usize, SqliteError> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM all_node WHERE kind = ?1", [node_kind_ordinal(kind)], |r| r.get(0))?;
    Ok(n as usize)
}

/// The boot log's counters. `cites_dropped_negative_votes` counts rows that were never written and
/// is not derivable from the tables, so it is reported as 0 here.
pub fn load_counters(conn: &Connection, present: &[Section]) -> Result<(BuildStats, EventWorldStats), SqliteError> {
    let has = |s: Section| present.contains(&s);
    let stats = BuildStats {
        kjv_verses: if has(Section::Kjv) { count(conn, "SELECT COUNT(*) FROM kjv.verse")? } else { 0 },
        cites_rows: if has(Section::Kjv) { count(conn, "SELECT COUNT(*) FROM kjv.cross_refs")? } else { 0 },
        cites_dropped_negative_votes: 0,
        lexicon: Default::default(),
    };
    let ews = EventWorldStats {
        places: count_kind(conn, NodeKind::Place)?,
        events: count_kind(conn, NodeKind::Event)?,
        dated_events: count(conn, "SELECT COUNT(*) FROM event_date")?,
        narratives: count_kind(conn, NodeKind::Narrative)?,
        succession_rows: count(conn, "SELECT COUNT(*) FROM succession")?,
        anchors: count_kind(conn, NodeKind::Anchor)?,
        attests_rows: count(conn, "SELECT COUNT(*) FROM attests")?,
        located_at_rows: count(conn, "SELECT COUNT(*) FROM located_at")?,
        dated_by_rows: count(conn, "SELECT COUNT(*) FROM dated_by")?,
        event_mentions_rows: count(conn, "SELECT COUNT(*) FROM mentions WHERE entity_kind = 3")?,
        analogue_rows: count(conn, "SELECT COUNT(*) FROM analogue")?,
    };
    Ok((stats, ews))
}

/// A seek on `xref_by_from`, never a scan.
pub fn cross_refs_for_span(conn: &Connection, span: &ScriptureRef) -> Result<HashMap<String, Vec<CrossRef>>, SqliteError> {
    let base = "SELECT from_a, from_b, from_c, target_display, votes FROM kjv.cross_refs";
    let (sql, params): (String, Vec<i64>) = match span {
        ScriptureRef::Book(b) => (format!("{base} WHERE from_a = ?1 ORDER BY ord"), vec![b.0 as i64]),
        ScriptureRef::Chapter { book, chapter } => {
            (format!("{base} WHERE from_a = ?1 AND from_b = ?2 ORDER BY ord"), vec![book.0 as i64, *chapter as i64])
        }
        ScriptureRef::Passage { book, chapter, from_verse, to_verse } => (
            format!("{base} WHERE from_a = ?1 AND from_b = ?2 AND from_c BETWEEN ?3 AND ?4 ORDER BY ord"),
            vec![book.0 as i64, *chapter as i64, *from_verse as i64, *to_verse as i64],
        ),
        ScriptureRef::Verse(v) => (
            format!("{base} WHERE from_a = ?1 AND from_b = ?2 AND from_c = ?3 ORDER BY ord"),
            vec![v.book.0 as i64, v.chapter as i64, v.verse as i64],
        ),
    };
    let mut stmt = conn.prepare_cached(&sql)?;
    let mut rows = stmt.query(rusqlite::params_from_iter(params))?;
    let mut out: HashMap<String, Vec<CrossRef>> = HashMap::new();
    while let Some(r) = rows.next()? {
        let (a, b, c): (i64, i64, i64) = (r.get(0)?, r.get(1)?, r.get(2)?);
        let target: String = r.get(3)?;
        let votes: i64 = r.get(4)?;
        out.entry(dot_ref(a as u8, b as u16, c as u16)).or_default().push(CrossRef { target, votes: votes as i32 });
    }
    Ok(out)
}
