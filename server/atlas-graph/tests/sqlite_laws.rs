//! DB-2b laws on synthetic data: the seams the section writer and the
//! SqliteSnapshot are built from, each proven in isolation before the
//! real-data gate (`sqlite_real_data.rs`) composes them.
use atlas_graph::sqlite::{
    hash_bytes, hash_from_bytes, open_read_only, stamp_pragmas, APPLICATION_ID, HASH_WIDTH,
    SCHEMA_VERSION,
};
use atlas_graph_types::id::ContentHash;

#[test]
fn hash_blob_round_trips_at_the_current_width() {
    let h = ContentHash(0x0123_4567_89ab_cdef);
    let b = hash_bytes(&h);
    assert_eq!(b.len(), HASH_WIDTH);
    assert_eq!(b, 0x0123_4567_89ab_cdefu64.to_be_bytes().to_vec());
    assert_eq!(hash_from_bytes(&b).unwrap(), h);
    assert!(
        hash_from_bytes(&b[..HASH_WIDTH - 1]).is_err(),
        "a short blob is refused, never padded"
    );
}

#[test]
fn pragmas_are_stamped_and_read_back_from_a_read_only_open() {
    let dir = std::env::temp_dir().join(format!("db2b-laws-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("pragmas.sqlite");
    let _ = std::fs::remove_file(&path);
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        stamp_pragmas(&conn).unwrap();
        conn.execute_batch(
            "CREATE TABLE t (k TEXT PRIMARY KEY) WITHOUT ROWID; INSERT INTO t VALUES ('x');",
        )
        .unwrap();
    }
    let ro = open_read_only(&path).unwrap();
    let uv: u32 = ro.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
    let ai: u32 = ro.query_row("PRAGMA application_id", [], |r| r.get(0)).unwrap();
    let ps: u32 = ro.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap();
    let qo: u32 = ro.query_row("PRAGMA query_only", [], |r| r.get(0)).unwrap();
    assert_eq!((uv, ai, ps, qo), (SCHEMA_VERSION, APPLICATION_ID, 4096, 1));
    assert!(
        ro.execute("INSERT INTO t VALUES ('y')", []).is_err(),
        "read-only means read-only"
    );
}

// ---------------------------------------------------------------------
// Task 3: DDL per section; the LOCUS / RANGE / AUTHORED column codec
// ---------------------------------------------------------------------
use atlas_graph::sections::Section;
use atlas_graph::sqlite::columns::{
    bible_locus_values, locus_columns, read_bible_locus, read_justification, JustificationWriter,
};
use atlas_graph::sqlite::ddl::{create_indexes, create_tables, logical_table_order, row_tables_of};
use atlas_graph_types::canon::{Canon, RowFamily};
use atlas_graph_types::edge::{Ground, Justification};
use atlas_graph_types::id::{AnchorId, SourceId};
use atlas_graph_types::text::{BibleTag, Locus, LocusRange, TokenSpan, TranslationId, VerseRef};
use std::collections::BTreeSet;

#[test]
fn every_section_schema_creates_in_memory_and_lists_its_tables() {
    for s in Section::MANIFEST_ORDER {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn, s).unwrap();
        create_indexes(&conn, s).unwrap();
        let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").unwrap();
        let tables: BTreeSet<String> = stmt.query_map([], |r| r.get(0)).unwrap().map(Result::unwrap).collect();
        for common in ["meta", "node", "justification", "ground", "edge_index"] {
            assert!(tables.contains(common), "{s:?} lacks {common}");
        }
        for f in row_tables_of(s) {
            assert!(tables.contains(f.name()), "{s:?} lacks {}", f.name());
        }
        assert_eq!(tables.contains("reading_spine"), matches!(s, Section::Kjv | Section::Concord));
        let uv: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(uv, 14);
        assert_eq!(logical_table_order(s)[0], "node");
    }
    // Every family has exactly one home except ContainsBible (core + kjv).
    let mut homes: std::collections::BTreeMap<RowFamily, usize> = Default::default();
    for s in Section::MANIFEST_ORDER {
        for f in row_tables_of(s) {
            *homes.entry(*f).or_default() += 1;
        }
    }
    for f in RowFamily::ALL {
        assert_eq!(
            homes.get(&f).copied().unwrap_or(0),
            if f == RowFamily::ContainsBible { 2 } else { 1 },
            "{f:?}"
        );
    }
}

#[test]
fn a_bible_locus_with_a_span_round_trips_through_seven_columns() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(&format!(
        "CREATE TABLE t (id INTEGER PRIMARY KEY, {});",
        locus_columns("p")
            .replace(",", " ,")
            .replace("p_corpus", "p_corpus TEXT")
            .replace("p_a ", "p_a INTEGER ")
            .replace("p_b ", "p_b INTEGER ")
            .replace("p_c ", "p_c INTEGER ")
            .replace("p_layer", "p_layer TEXT")
            .replace("p_start", "p_start INTEGER")
            .replace("p_end", "p_end INTEGER")
    ))
    .unwrap();
    let l = Locus::<BibleTag> {
        unit: VerseRef { book: 43, chapter: 3, verse: 16 },
        span: Some(TokenSpan::new(TranslationId("kjv".into()), 2, 5).unwrap()),
    };
    let v = bible_locus_values(&l);
    conn.execute(
        "INSERT INTO t (p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end) VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params_from_iter(v.iter()),
    )
    .unwrap();
    let back: Locus<BibleTag> = conn
        .query_row("SELECT p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end FROM t", [], |r| {
            Ok(read_bible_locus(r, 0).unwrap())
        })
        .unwrap();
    assert_eq!(back.encode(), l.encode(), "canon bytes equal => the struct is the same struct");
    let whole = Locus::<BibleTag> { unit: VerseRef { book: 1, chapter: 1, verse: 1 }, span: None };
    conn.execute("DELETE FROM t", []).unwrap();
    conn.execute(
        "INSERT INTO t (p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end) VALUES (?, ?, ?, ?, ?, ?, ?)",
        rusqlite::params_from_iter(bible_locus_values(&whole).iter()),
    )
    .unwrap();
    let back: Locus<BibleTag> = conn
        .query_row("SELECT p_corpus, p_a, p_b, p_c, p_layer, p_start, p_end FROM t", [], |r| {
            Ok(read_bible_locus(r, 0).unwrap())
        })
        .unwrap();
    assert_eq!(back.encode(), whole.encode());
}

#[test]
fn a_justification_with_three_ground_kinds_round_trips_and_an_empty_one_is_null() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    create_tables(&conn, Section::Core).unwrap();
    let mut grounds = BTreeSet::new();
    let from = Locus::<BibleTag> { unit: VerseRef { book: 2, chapter: 20, verse: 1 }, span: None };
    let to = Locus::<BibleTag> { unit: VerseRef { book: 2, chapter: 20, verse: 17 }, span: None };
    grounds.insert(Ground::Scripture(LocusRange::new(from, to).unwrap()));
    grounds.insert(Ground::Anchor(AnchorId::new("exodus")));
    grounds.insert(Ground::Source(SourceId::new("ussher")));
    let j = Justification { text: Some("because".into()), grounds };
    let mut conn = conn;
    let (id, none_id) = {
        let tx = conn.transaction().unwrap();
        let mut w = JustificationWriter::new();
        let id = w.write(&tx, &j).unwrap();
        let none_id = w.write(&tx, &Justification::default()).unwrap();
        tx.commit().unwrap();
        (id, none_id)
    };
    assert_eq!(id, Some(1));
    assert_eq!(none_id, None, "an empty justification writes no row and binds NULL");
    let back = read_justification(&conn, id).unwrap();
    assert_eq!(back.encode(), j.encode());
    assert_eq!(read_justification(&conn, None).unwrap().encode(), Justification::default().encode());
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM ground", [], |r| r.get(0)).unwrap();
    assert_eq!(n, 3);
}
