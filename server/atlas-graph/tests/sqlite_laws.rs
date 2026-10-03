use atlas_graph::sqlite::blob::sha256_hex_of_file;
use atlas_graph::sqlite::extras::Extras;
use atlas_graph::sqlite::source::{sibling_dir, CommittedZstdSource, SectionLayout, SectionSource};
use atlas_graph::sqlite::{
    hash_bytes, hash_from_bytes, open_read_only, stamp_pragmas, APPLICATION_ID, HASH_WIDTH,
    SCHEMA_VERSION,
};
use atlas_graph_types::id::ContentHash;

fn layout_under(dir: &std::path::Path) -> SectionLayout {
    SectionLayout { compiled_dir: dir.join("compiled"), cache_dir: dir.join("cache").join("sections") }
}

fn open_written(dir: &std::path::Path) -> Result<SqliteSnapshot, atlas_graph::sqlite::SqliteError> {
    let layout = layout_under(dir);
    SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout })
}

#[test]
fn hash_blob_round_trips_at_the_current_width() {
    let hex = "0123456789abcdef".repeat(HASH_WIDTH / 8);
    let h = ContentHash::from_hex(&hex).expect("the current width's hex");
    let b = hash_bytes(&h);
    assert_eq!(b.len(), HASH_WIDTH);
    let expected: Vec<u8> = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap()).collect();
    assert_eq!(b, expected, "the blob is the hex, decoded");
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

use atlas_graph::sections::Section;
use atlas_graph::sqlite::columns::{
    bible_locus_values, locus_columns, read_bible_locus, read_justification, JustificationWriter,
};
use atlas_graph::sqlite::ddl::{create_indexes, create_tables, logical_table_order, row_tables_of};
use atlas_graph_types::canon::{Canon, RowFamily};
use atlas_graph_types::edge::{Authored, Ground, Justification, MapSuccession, Occurs, Parentage, ParentOf, Participates, Spouses, Shown, Brethren};
use atlas_graph_types::id::{AnchorId, MapId, SourceId, LexiconEntryId};
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
        for common in ["meta", "node", "justification", "ground", "edge_index", "edge_count"] {
            assert!(tables.contains(common), "{s:?} lacks {common}");
        }
        for f in row_tables_of(s) {
            assert!(tables.contains(f.name()), "{s:?} lacks {}", f.name());
        }
        assert_eq!(tables.contains("reading_spine"), matches!(s, Section::Kjv | Section::Concord));
        let uv: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(uv, SCHEMA_VERSION);
        assert_eq!(logical_table_order(s)[0], "node");
    }
    let mut homes: std::collections::BTreeMap<RowFamily, usize> = Default::default();
    for s in Section::MANIFEST_ORDER {
        for f in row_tables_of(s) {
            *homes.entry(*f).or_default() += 1;
        }
    }
    for f in RowFamily::ALL {
        assert_eq!(
            homes.get(&f).copied().unwrap_or(0),
            if matches!(f, RowFamily::ContainsBible | RowFamily::CanonSuccession | RowFamily::CrossRefs) { 2 } else { 1 },
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
fn a_mentions_row_naming_no_kind_of_entity_is_refused_when_a_window_reads_it() {
    // Arrange
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    create_tables(&conn, Section::Core).unwrap();
    conn.execute(
        "INSERT INTO mentions (id, ord, locus_corpus, locus_a, locus_b, locus_c, locus_layer, locus_start, locus_end, entity_kind, entity_id, provenance) \
         VALUES (0, 0, ?1, 0, 1, 1, NULL, NULL, NULL, 9, 'hazor-1', 'test')",
        [<BibleTag as atlas_graph_types::text::Corpus>::ID],
    )
    .unwrap();
    let genesis_1_1 = VerseRef { book: 0, chapter: 1, verse: 1 };

    // Act
    let read = atlas_graph::sqlite::serve::mention_spans_in(&conn, &(genesis_1_1.clone()..=genesis_1_1)).map_err(|e| e.0);

    // Assert
    assert_eq!(read, Err("mentions entity_kind 9 is not 0..3".to_string()));
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

use atlas_graph::sqlite::rows::{insert_row, read_rows, RowRef};
use atlas_graph_types::canon::encode_row_in_family;
use atlas_graph_types::chrono::{DatePlacement, DatedBy, Duration, PlacementBasis};
use atlas_graph_types::edge::{
    Analogue, Attests, CanonSuccession, CatechismLink, CommentsOn, Confesses, ContainerContent,
    Contains, Corresponds, CrossRef, Fulfills, LocatedAt, MentionedEntity, Mentions, NamedAfter,
    Namesake, Quotes, SpokenAt, SpokenBy, Succession, TemporalAdjacency, Typology,
};
use atlas_graph_types::id::{
    CatechismItemId, CommentaryItemId, ContainerNodeId, EventId, NarrativeId, PeopleGroupId,
    PersonId, PlaceId,
};
use atlas_graph_types::text::{
    BibleLocus, BibleLocusRange, ConcordLocus, ConcordRef, ConcordTag, LocusSet, TextLocus, TextRef,
};

fn vr(book: u8, chapter: u16, verse: u16) -> VerseRef {
    VerseRef { book, chapter, verse }
}
fn bl(book: u8, chapter: u16, verse: u16) -> BibleLocus {
    Locus::whole(vr(book, chapter, verse))
}
fn blr(from: (u8, u16, u16), to: (u8, u16, u16)) -> BibleLocusRange {
    LocusRange::new(bl(from.0, from.1, from.2), bl(to.0, to.1, to.2)).expect("test range must be ordered")
}
fn tl(book: u8, chapter: u16, verse: u16) -> TextLocus {
    TextLocus { at: TextRef::Bible(vr(book, chapter, verse)), span: None }
}
fn cl(part: u8, article: u16, paragraph: u16) -> ConcordLocus {
    Locus::whole(ConcordRef { part, article, paragraph })
}
fn span(start: u16, end: u16) -> TokenSpan {
    TokenSpan::new(TranslationId("kjv".into()), start, end).expect("test span must be ordered")
}
fn full_justification() -> Justification {
    let grounds: BTreeSet<Ground> = [
        Ground::Scripture(blr((40, 3, 13), (40, 3, 17))),
        Ground::Anchor(AnchorId::new("ussher-4004bc")),
        Ground::Source(SourceId::new("openbible-geo")),
    ]
    .into_iter()
    .collect();
    Justification { text: Some("Jordan, at Bethabara".into()), grounds }
}

fn specimen_graph() -> atlas_graph_types::graph::Graph {
    use atlas_graph_types::graph::{Graph, ReadingSpine};
    use atlas_graph_types::id::{AnyNodeId, NodeKind};
    use atlas_graph_types::node::{Node, NodePayload};
    let mut g = Graph::default();
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("passage-creation"),
        content: ContainerContent::Loci(LocusSet([bl(1, 1, 1), bl(1, 1, 2)].into_iter().collect())),
        provenance: "kjv".into(),
        justification: Justification::default(),
    });
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("bible-book-GEN"),
        content: ContainerContent::Container(ContainerNodeId::new("bible-chapter-GEN-1")),
        provenance: "kjv".into(),
        justification: full_justification(),
    });
    g.contains_concord.push(Contains::<ConcordTag> {
        container: ContainerNodeId::new("concord-ac"),
        content: ContainerContent::Container(ContainerNodeId::new("concord-ac-1")),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    g.contains_concord.push(Contains::<ConcordTag> {
        container: ContainerNodeId::new("concord-ac-1"),
        content: ContainerContent::Loci(LocusSet(
            [cl(1, 1, 1), Locus { unit: ConcordRef { part: 1, article: 1, paragraph: 2 }, span: Some(span(0, 3)) }]
                .into_iter()
                .collect(),
        )),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    g.attests.push(Attests {
        event: EventId::new("jesus-baptized"),
        attestation: blr((40, 3, 13), (40, 3, 17)),
        provenance: "curated/events".into(),
        justification: full_justification(),
    });
    g.succession.push(
        Succession::new(
            NarrativeId::new("life-of-christ"),
            vec![EventId::new("nativity"), EventId::new("jesus-baptized")],
            "curated/narratives".into(),
            Justification::default(),
        )
        .expect("a distinct, non-empty chain"),
    );
    g.canon_succession.push(CanonSuccession {
        prior: ContainerNodeId::new("bible-chapter-GEN-50"),
        next: ContainerNodeId::new("bible-chapter-EXO-1"),
        provenance: "canon".into(),
        justification: Justification::default(),
    });
    g.canon_succession.push(CanonSuccession {
        prior: ContainerNodeId::new("concord-doc-preface"),
        next: ContainerNodeId::new("concord-doc-ecumenical-creeds"),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    g.dated_by.push(DatedBy {
        event: EventId::new("exodus"),
        placement: DatePlacement::AnchorBinding { anchor: AnchorId::new("abraham-called"), offset: Duration::years(430) },
        basis: PlacementBasis::Textual,
        justification: Justification::default(),
        provenance: "ussher".into(),
    });
    g.dated_by.push(DatedBy {
        event: EventId::new("nativity"),
        placement: DatePlacement::ReignYear { reign: AnchorId::new("herod"), year_of_reign: 3 },
        basis: PlacementBasis::Traditional,
        justification: full_justification(),
        provenance: "ussher".into(),
    });
    g.dated_by.push(DatedBy {
        event: EventId::new("jesus-baptized"),
        placement: DatePlacement::SequenceAfter {
            prior: EventId::new("nativity"),
            spacing: Duration { years: 30, months: 1, days: 2 },
        },
        basis: PlacementBasis::Textual,
        justification: Justification::default(),
        provenance: "ussher".into(),
    });
    g.dated_by.push(DatedBy {
        event: EventId::new("leper-healed-galilee"),
        placement: DatePlacement::EraOnly { era: atlas_graph_types::id::EraId::new("ministry") },
        basis: PlacementBasis::Traditional,
        justification: Justification::default(),
        provenance: "ussher".into(),
    });
    g.located_at.push(LocatedAt {
        event: EventId::new("jesus-baptized"),
        place: PlaceId::new("jordan-river"),
        provenance: "curated/events".into(),
        justification: full_justification(),
    });
    g.fulfills.push(Fulfills {
        prophecy: blr((23, 7, 14), (23, 7, 14)),
        fulfillment: blr((40, 1, 22), (40, 1, 23)),
        provenance: "curated/fulfillment".into(),
        justification: full_justification(),
    });
    g.typology.push(Typology {
        type_passage: blr((4, 21, 8), (4, 21, 9)),
        antitype_passage: blr((43, 3, 14), (43, 3, 14)),
        note: Some("the brasen serpent".into()),
        provenance: "curated/typology".into(),
        justification: Justification::default(),
    });
    g.named_after.push(NamedAfter {
        namesake: Namesake::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        eponym: PersonId::new("judah"),
        provenance: "curated/peoples".into(),
        justification: Justification::default(),
    });
    g.catechism.push(CatechismLink {
        locus: TextLocus { at: TextRef::Concord(ConcordRef { part: 1, article: 2, paragraph: 3 }), span: None },
        item: CatechismItemId::new("sc/1st-commandment"),
        provenance: "small-catechism".into(),
        justification: Justification::default(),
    });
    g.comments_on.push(CommentsOn {
        item: CommentaryItemId::new("kretzmann/JHN.3.16"),
        on: blr((43, 3, 16), (43, 3, 16)),
        provenance: "kretzmann".into(),
        justification: Justification::default(),
    });
    g.spoken_by.push(SpokenBy {
        locus: blr((43, 3, 16), (43, 3, 21)),
        speaker: PersonId::new("jesus"),
        provenance: "red-letter".into(),
        justification: Justification::default(),
    });
    g.spoken_at.push(SpokenAt {
        locus: blr((43, 3, 16), (43, 3, 21)),
        place: PlaceId::new("jerusalem"),
        provenance: "red-letter".into(),
        justification: Justification::default(),
    });
    g.mentions.push(Mentions {
        locus: TextLocus { at: TextRef::Bible(vr(7, 1, 2)), span: Some(span(3, 5)) },
        entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        provenance: "theographic".into(),
    });
    g.cross_refs.push(CrossRef {
        from: tl(51, 1, 15),
        to: tl(51, 1, 16),
        to_last: Some(tl(51, 1, 19)),
        target_display: "COL.1.16-19".into(),
        votes: 7,
        provenance: "openbible-xrefs".into(),
    });
    g.cross_refs.push(CrossRef {
        from: TextLocus { at: TextRef::Concord(ConcordRef { part: 1, article: 1, paragraph: 1 }), span: Some(TokenSpan::new(TranslationId("bente-dau".into()), 0, 0).unwrap()) },
        to: tl(1, 1, 1),
        to_last: Some(tl(1, 1, 2)),
        target_display: "GEN.1.1-2".into(),
        votes: 0,
        provenance: "concord-citations".into(),
    });
    g.quotes.push(Quotes { quoting: tl(40, 4, 4), quoted: blr((5, 8, 3), (5, 8, 3)), provenance: "curated/quotes".into() });
    g.confesses.push(Confesses {
        confessing: cl(1, 2, 3),
        confessed: blr((45, 3, 28), (45, 3, 28)),
        provenance: "concord".into(),
        justification: full_justification(),
    });
    g.corresponds_bible.push(Corresponds::<BibleTag> {
        a: Locus { unit: vr(43, 3, 16), span: Some(span(0, 4)) },
        b: Locus { unit: vr(43, 3, 16), span: Some(span(5, 9)) },
        provenance: "alignment".into(),
    });
    g.temporal_adjacency.push(TemporalAdjacency {
        earlier: EventId::new("nativity"),
        later: EventId::new("jesus-baptized"),
        provenance: "derived/chronology".into(),
    });
    g.analogue.push(Analogue {
        a: EventId::new("leper-healed-galilee"),
        b: EventId::new("leper-healed-capernaum"),
        provenance: "curated/analogues".into(),
    });

    let word = |book: u8, chapter: u16, verse: u16, layer: &str, tok: u16| TextLocus {
        at: TextRef::Bible(vr(book, chapter, verse)),
        span: Some(TokenSpan::new(TranslationId(layer.into()), tok, tok).unwrap()),
    };
    g.occurs.push(Occurs { entry: LexiconEntryId::new("G3056"), locus: word(1, 1, 1, "greek_textus_receptus", 1), provenance: "stepbible-tagnt".into() });
    g.occurs.push(Occurs { entry: LexiconEntryId::new("G3056"), locus: word(1, 1, 1, "greek_textus_receptus", 3), provenance: "stepbible-tagnt".into() });
    g.occurs.push(Occurs { entry: LexiconEntryId::new("H0430"), locus: word(1, 1, 2, "hebrew_masoretic", 2), provenance: "stepbible-tahot".into() });

    g.parent_of.push(ParentOf {
        parent: PersonId::new("abraham_1"),
        child: PersonId::new("isaac_1"),
        parentage: Parentage::Natural,
        provenance: "theographic-people".into(),
        justification: Justification::default(),
    });
    g.parent_of.push(ParentOf {
        parent: PersonId::new("god_1324"),
        child: PersonId::new("adam_78"),
        parentage: Parentage::Created,
        provenance: "curated-parentage".into(),
        justification: Justification { text: None, grounds: [Ground::Scripture(blr((41, 3, 38), (41, 3, 38)))].into_iter().collect() },
    });
    g.spouses.push(Spouses { a: PersonId::new("abraham_1"), b: PersonId::new("sarah_1"), provenance: "theographic-people".into() });
    g.brethren.push(Brethren {
        a: PersonId::new("james_719"),
        b: PersonId::new("jesus_905"),
        provenance: "curated-brethren".into(),
        justification: Justification { text: None, grounds: [Ground::Scripture(blr((47, 1, 19), (47, 1, 19)))].into_iter().collect() },
    });
    g.participates.push(Participates { person: PersonId::new("abraham_1"), event: EventId::new("jesus-baptized"), provenance: "theographic-people".into() });
    g.authored.push(Authored { book: ContainerNodeId::new("bible-book-GEN"), person: PersonId::new("moses_2108"), provenance: "books".into(), justification: Justification::default() });
    g.shown.push(Shown { map: MapId::new("era-patriarchs"), node: PlaceId::new("ur-1").erase(), provenance: "curated-eras".into() });
    g.map_succession.push(MapSuccession { prior: MapId::new("era-patriarchs"), next: MapId::new("era-exodus"), provenance: "curated-eras".into() });

    let node = |kind: NodeKind, raw: &str, payload: NodePayload| Node {
        id: AnyNodeId { kind, raw: raw.to_string() },
        payload,
        provenance: "prov".to_string(),
    };
    let unit = |corpus: &'static str, layer: &str, txt: &str| NodePayload::TextUnit {
        corpus,
        renderings: [(TranslationId(layer.into()), txt.to_string())].into_iter().collect(),
    };
    for n in [
        node(NodeKind::TextUnit, "bible/1.1.1", unit("bible", "kjv", "In the beginning")),
        node(NodeKind::TextUnit, "bible/1.1.2", unit("bible", "kjv", "And the earth")),
        node(NodeKind::TextUnit, "concord/1.1.1", unit("concord", "bente-dau", "We believe")),
        node(NodeKind::Container, "passage-creation", NodePayload::Container { title: "Creation".into() }),
        node(NodeKind::Container, "bible-book-GEN", NodePayload::Container { title: "Genesis".into() }),
        node(NodeKind::Container, "bible-chapter-GEN-1", NodePayload::Container { title: "Genesis 1".into() }),
        node(NodeKind::Container, "concord-ac", NodePayload::Container { title: "Augsburg Confession".into() }),
        node(NodeKind::Container, "concord-ac-1", NodePayload::Container { title: "Article I".into() }),
        node(
            NodeKind::CommentaryItem,
            "kretzmann/JHN.3.16",
            NodePayload::CommentaryItem {
                work: SourceId::new("kretzmann"),
                heading: Some("The love of God".into()),
                text: "For God so loved the world...".into(),
            },
        ),
        node(
            NodeKind::Place,
            "ur-1",
            NodePayload::Place { canonical: "Ur".into(), lat: 30.96, lon: 46.1, aliases: vec![], description: None },
        ),
        node(NodeKind::Era, "patriarchs", NodePayload::Era { label: "Patriarchs".into(), from_year: -2100, to_year: -1800 }),
        node(NodeKind::Map, "era-patriarchs", NodePayload::Map { label: "Patriarchs".into(), from_year: -2100, to_year: -1800 }),
        node(
            NodeKind::LexiconEntry,
            "G3056",
            NodePayload::LexiconEntry {
                strong: "G3056".into(),
                lang: "grc".into(),
                lemma: "λόγος".into(),
                translit: Some("lógos".into()),
                pos: Some("G:N-M".into()),
                glosses: vec!["word".into()],
                senses: vec!["something said".into()],
                root: Some("G3004".into()),
            },
        ),
        node(
            NodeKind::LexiconEntry,
            "H0430",
            NodePayload::LexiconEntry {
                strong: "H0430".into(),
                lang: "hbo".into(),
                lemma: "אֱלֹהִים".into(),
                translit: None,
                pos: None,
                glosses: vec![],
                senses: vec![],
                root: None,
            },
        ),
        node(
            NodeKind::Polity,
            "egypt",
            NodePayload::Polity {
                label: "Egypt".into(),
                color_key: 1,
                eras: vec![
                    atlas_graph_types::node::PolityEraPayload {
                        name: "Old Kingdom".into(),
                        from_year: -2686,
                        to_year: -2181,
                        rings: vec![vec![(30.0, 31.0), (30.5, 31.5), (30.0, 31.5)]],
                        ref_note: "test".into(),
                        transition: None,
                        fall: None,
                    },
                    atlas_graph_types::node::PolityEraPayload {
                        name: "Middle Kingdom".into(),
                        from_year: -2055,
                        to_year: -1650,
                        rings: vec![],
                        ref_note: "test".into(),
                        transition: None,
                        fall: None,
                    },
                ],
            },
        ),
    ] {
        g.nodes.insert(n.id.clone(), n);
    }
    g.reading.insert(
        "bible",
        ReadingSpine {
            order: vec![
                AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() },
                AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.2".into() },
            ],
        },
    );
    g.reading.insert(
        "concord",
        ReadingSpine { order: vec![AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/1.1.1".into() }] },
    );
    g
}

fn rows_of_section_explicit(g: &atlas_graph_types::graph::Graph, s: Section) -> Vec<RowRef<'_>> {
    let mut out = Vec::new();
    for f in row_tables_of(s) {
        match f {
            RowFamily::ContainsBible => out.extend(g.contains_bible.iter().map(RowRef::ContainsBible)),
            RowFamily::ContainsConcord => out.extend(g.contains_concord.iter().map(RowRef::ContainsConcord)),
            RowFamily::Attests => out.extend(g.attests.iter().map(RowRef::Attests)),
            RowFamily::Succession => out.extend(g.succession.iter().map(RowRef::Succession)),
            RowFamily::CanonSuccession => out.extend(g.canon_succession.iter().map(RowRef::CanonSuccession)),
            RowFamily::DatedBy => out.extend(g.dated_by.iter().map(RowRef::DatedBy)),
            RowFamily::LocatedAt => out.extend(g.located_at.iter().map(RowRef::LocatedAt)),
            RowFamily::Fulfills => out.extend(g.fulfills.iter().map(RowRef::Fulfills)),
            RowFamily::Typology => out.extend(g.typology.iter().map(RowRef::Typology)),
            RowFamily::NamedAfter => out.extend(g.named_after.iter().map(RowRef::NamedAfter)),
            RowFamily::Catechism => out.extend(g.catechism.iter().map(RowRef::Catechism)),
            RowFamily::Mentions => out.extend(g.mentions.iter().map(RowRef::Mentions)),
            RowFamily::CorrespondsBible => out.extend(g.corresponds_bible.iter().map(RowRef::CorrespondsBible)),
            RowFamily::TemporalAdjacency => out.extend(g.temporal_adjacency.iter().map(RowRef::TemporalAdjacency)),
            RowFamily::Analogue => out.extend(g.analogue.iter().map(RowRef::Analogue)),
            RowFamily::CrossRefs => out.extend(g.cross_refs.iter().map(RowRef::CrossRefs)),
            RowFamily::SpokenBy => out.extend(g.spoken_by.iter().map(RowRef::SpokenBy)),
            RowFamily::SpokenAt => out.extend(g.spoken_at.iter().map(RowRef::SpokenAt)),
            RowFamily::Quotes => out.extend(g.quotes.iter().map(RowRef::Quotes)),
            RowFamily::Confesses => out.extend(g.confesses.iter().map(RowRef::Confesses)),
            RowFamily::CommentsOn => out.extend(g.comments_on.iter().map(RowRef::CommentsOn)),
            RowFamily::Occurs => out.extend(g.occurs.iter().map(RowRef::Occurs)),
            RowFamily::ParentOf => out.extend(g.parent_of.iter().map(RowRef::ParentOf)),
            RowFamily::Spouses => out.extend(g.spouses.iter().map(RowRef::Spouses)),
            RowFamily::Participates => out.extend(g.participates.iter().map(RowRef::Participates)),
            RowFamily::Authored => out.extend(g.authored.iter().map(RowRef::Authored)),
            RowFamily::Shown => out.extend(g.shown.iter().map(RowRef::Shown)),
            RowFamily::MapSuccession => out.extend(g.map_succession.iter().map(RowRef::MapSuccession)),
            RowFamily::Brethren => out.extend(g.brethren.iter().map(RowRef::Brethren)),
        }
    }
    out
}

#[test]
fn every_family_round_trips_through_its_columns_with_identical_canon_bytes() {
    let g = specimen_graph();
    for s in [Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann] {
        let mut conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn, s).unwrap();
        let rows: Vec<RowRef> = rows_of_section_explicit(&g, s);
        let tx = conn.transaction().unwrap();
        let mut jw = JustificationWriter::new();
        for (i, r) in rows.iter().enumerate() {
            insert_row(&tx, &mut jw, i as i64, r).unwrap();
        }
        tx.commit().unwrap();
        create_indexes(&conn, s).unwrap();
        for f in row_tables_of(s) {
            let written: Vec<Vec<u8>> =
                rows.iter().filter(|r| r.family() == *f).map(|r| encode_row_in_family(*f, r.to_value())).collect();
            let read: Vec<Vec<u8>> = read_rows(&conn, *f)
                .unwrap()
                .into_iter()
                .map(|(_, r)| encode_row_in_family(*f, r.to_value()))
                .collect();
            assert!(!written.is_empty(), "{s:?}/{f:?}: the specimen graph must inhabit every family");
            assert_eq!(read, written, "{s:?}/{f:?}");
        }
    }
}

use atlas_graph::sqlite::manifest::{read_manifest, root_of, write_manifest, Manifest, ManifestSection};
use atlas_graph::sqlite::partition::partition;
use atlas_graph_types::section_index::edge_row_map;
use atlas_graph::sqlite::writer::write_sections;

#[test]
fn every_index_entry_of_the_specimen_lands_in_exactly_one_section_and_names_its_row() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let parts = partition(&g).unwrap();
    let total: usize = parts.iter().map(|p| p.index.entries.len()).sum();
    let justified = g
        .indexes
        .get(&atlas_graph_types::edge::RelationId::JustifiedBy)
        .map_or(0, |ix| ix.fwd.values().chain(ix.inv.values()).map(atlas_graph_types::adjacency::Adjacency::edge_count).sum::<usize>());
    let placed_rows = 2 * g.row_edges().len() + justified;
    assert_eq!(total, placed_rows, "every row placed under both of its ends, every synthesised ground once; none lost, none duplicated");
    assert!(total > 0, "the specimen graph indexes something");
    let map = edge_row_map(&g);
    let justified_code = atlas_graph_types::edge::RelationId::ALL
        .iter()
        .position(|r| *r == atlas_graph_types::edge::RelationId::JustifiedBy)
        .unwrap() as i64;
    let mut saw_justified = false;
    for p in &parts {
        for e in &p.index.entries {
            if e.rel != justified_code {
                assert!(map[&e.edge_id].contains(&(e.row_family, e.row_id)), "entry {:?} names a row that does not mint its id", (e.row_family, e.row_id));
                assert_eq!(atlas_graph::sections::section_of_row(&g, e.row_family, e.row_id as usize), p.section);
            } else {
                saw_justified = true;
                let end = if e.dir == 0 { &e.subject } else { &e.object };
                let source = match end {
                    atlas_graph_types::id::Position::Edge(id) => id,
                    other => panic!("justified-by source end must be an edge, got {other:?}"),
                };
                assert_eq!((e.row_family, e.row_id), map[source][0]);
                assert_eq!(atlas_graph::sections::section_of_row(&g, e.row_family, e.row_id as usize), p.section);
            }
        }
    }
    assert!(saw_justified, "the specimen's grounded rows synthesise justified-by entries");
    assert!(
        parts.iter().any(|p| p.section == Section::Core && p.rows.iter().any(|(f, _, _)| *f == RowFamily::ContainsBible)),
        "curated container row in core"
    );
    assert!(
        parts.iter().any(|p| p.section == Section::Kjv && p.rows.iter().any(|(f, _, _)| *f == RowFamily::ContainsBible)),
        "chapter container row in kjv"
    );
}

#[test]
fn the_manifest_round_trips_and_its_root_is_over_the_section_lines_only() {
    let s = |name: &str, req: bool, logical: &str| ManifestSection {
        name: name.into(),
        required: req,
        logical: logical.into(),
        blob: "00".repeat(32),
        bytes: 1,
        schema_version: SCHEMA_VERSION,
    };
    let sections = vec![s("core", true, &"a".repeat(32)), s("kjv", true, &"b".repeat(32)), s("concord", false, &"c".repeat(32))];
    let root = root_of(&sections);
    assert_eq!(root.len(), 32);
    let m = Manifest {
        schema: 1,
        compiler: "test".into(),
        built: "2026-09-17T00:00:00Z".into(),
        root: root.clone(),
        raw_root: None,
        sections: sections.clone(),
    };
    let dir = std::env::temp_dir().join(format!("db2b-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("manifest.toml");
    atlas_graph::sqlite::manifest::write_manifest(&m, &path).unwrap();
    assert_eq!(read_manifest(&path).unwrap(), m);
    let mut later = m.clone();
    later.built = "2030-01-01T00:00:00Z".into();
    later.sections[0].bytes = 999;
    assert_eq!(root_of(&later.sections), root, "timestamps and byte sizes are outside the root (spec 2.2)");
    let mut tampered = m.clone();
    tampered.sections[1].logical = "d".repeat(32);
    atlas_graph::sqlite::manifest::write_manifest(&tampered, &path).unwrap();
    assert!(read_manifest(&path).is_err(), "a manifest whose root does not recompute is refused (spec 11)");
}

#[test]
fn the_writer_produces_five_files_named_by_logical_hash_and_a_manifest_in_order() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-writer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    assert_eq!(m.sections.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(), ["core", "kjv", "concord", "kretzmann", "lexicon"]);
    for (w, ms) in written.iter().zip(&m.sections) {
        assert_eq!(w.path.file_name().unwrap().to_str().unwrap(), format!("{}.{}.sqlite", ms.logical, ms.schema_version), "the cache file is named by the logical hash and the schema version");
        assert_eq!(w.blob_path.file_name().unwrap().to_str().unwrap(), format!("{}.{}.sqlite.zst", ms.name, ms.logical));
        assert_eq!(ms.blob.len(), 64);
        assert_eq!(ms.bytes, std::fs::metadata(&w.blob_path).unwrap().len(), "bytes = the compressed size");
        assert_eq!(ms.required, matches!(w.section, Section::Core | Section::Kjv));
    }
    assert_eq!(read_manifest(&layout_under(&dir).manifest_path()).unwrap(), m);
    let (m2, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    assert_eq!(m2.root, m.root, "a rewrite of identical content has an identical root");
    assert_eq!(
        m2.sections.iter().map(|s| &s.logical).collect::<Vec<_>>(),
        m.sections.iter().map(|s| &s.logical).collect::<Vec<_>>()
    );
    let files: Vec<String> = std::fs::read_dir(layout_under(&dir).sections_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| f.ends_with(".sqlite.zst"))
        .collect();
    assert_eq!(files.len(), 5, "stale blobs are deleted after a rewrite: {files:?}");
}

use atlas_graph::sqlite::logical::{logical_dump_of_db, logical_hash};
use atlas_graph_types::sections::logical_dump_section;

#[test]
fn the_logical_dump_recomputed_from_each_written_file_equals_the_partitions_dump() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-logical-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let parts = partition(&g).unwrap();
    for (p, w) in parts.iter().zip(&written) {
        let from_mem = logical_dump_section(&g, &p.index).unwrap();
        let conn = open_read_only(&w.path).unwrap();
        let from_db = logical_dump_of_db(&conn, p.section).unwrap();
        assert_eq!(String::from_utf8_lossy(&from_db), String::from_utf8_lossy(&from_mem), "{:?}", p.section);
        assert_eq!(logical_hash(&from_db), m.sections.iter().find(|s| s.name == p.section.name()).unwrap().logical);
        let stamped: String =
            conn.query_row("SELECT value FROM meta WHERE key = 'logical_hash'", [], |r| r.get(0)).unwrap();
        assert_eq!(stamped, logical_hash(&from_mem));
        assert!(from_mem.starts_with(b"node\t{\"id\":\""), "first line is a node line");
        if matches!(p.section, Section::Kjv | Section::Concord) {
            assert!(
                String::from_utf8_lossy(&from_db).contains("\nreading_spine\t{\"corpus\":\""),
                "{:?} carries spine lines",
                p.section
            );
        }
    }
}

#[test]
fn a_changed_row_changes_the_logical_hash_and_a_changed_timestamp_does_not() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-logical2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m1, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (m2, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    assert_eq!(m1.root, m2.root);
    g.located_at[0].provenance = "another-source".into();
    let (m3, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    assert_ne!(m3.sections[0].logical, m1.sections[0].logical, "core moved");
    assert_eq!(m3.sections[1].logical, m1.sections[1].logical, "kjv did not");
    assert_ne!(m3.root, m1.root);
}

use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph_types::store::{assert_answers_match, GraphQuery, GraphSnapshot};

#[test]
fn the_sqlite_snapshot_answers_every_port_question_exactly_as_the_specimen_graph() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Concord, Section::Kretzmann, Section::Lexicon]);
    assert_answers_match(&snap, &g);
    {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        use atlas_graph_types::adjacency::{Cursor, EdgeQuery};
        use atlas_graph_types::id::{AnyNodeId, NodeKind};
        let entry = at(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() });
        let verse = at(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() });
        let page = snap.edges_with_nodes(&entry, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Forward), cursor: Cursor::FIRST, limit: 10 });
        assert_eq!(page.entries.len(), 1, "two tokens of one entry in one verse: ONE edge (the leper lesson)...");
        assert_eq!(page.entries[0].entry.node, verse);
        let rows = snap.rows_behind(&page.entries[0].entry.edge);
        assert_eq!(rows.len(), 2, "...with BOTH rows behind it");
        assert_eq!(rows.iter().map(|r| (r.family, r.row_id)).collect::<Vec<_>>(), [(RowFamily::Occurs, 0), (RowFamily::Occurs, 1)]);
        assert!(rows.iter().all(|r| r.provenance == "stepbible-tagnt"));
        let summary = snap.edge_summary(&verse);
        assert_eq!(summary.get(&EdgeKind::Directed(RelationId::Occurs, Direction::Inverse)).copied(), Some(1), "`words` at the verse: the one entry its two tagged tokens belong to");
        let back = snap.edges_with_nodes(&verse, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Inverse), cursor: Cursor::FIRST, limit: 10 });
        assert_eq!(back.entries[0].entry.node, entry);
        assert_eq!(snap.nodes_of_kind(NodeKind::LexiconEntry, None, 5).ids.len(), 2);
    }
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g).unwrap(), "SqliteSnapshot::version is the manifest root = the in-memory root");
}

#[test]
fn the_sqlite_snapshot_reads_many_nodes_at_once_exactly_as_it_reads_each_one() {
    // Arrange
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("c2-nodes-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let held: Vec<atlas_graph_types::id::AnyNodeId> = g.nodes.keys().rev().cloned().collect();
    let absent = atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::Place, raw: "nowhere".into() };
    let asked: Vec<atlas_graph_types::id::AnyNodeId> = held.iter().cloned().chain([absent, held[0].clone()]).collect();
    let bytes = |node: Option<atlas_graph_types::node::Node>| node.map(|n| atlas_graph_types::id::ContentAddressed::canonical_bytes(&n));

    // Act
    let at_once: Vec<Option<Vec<u8>>> = snap.nodes(&asked).into_iter().map(bytes).collect();

    // Assert
    assert_eq!(at_once, asked.iter().map(|id| bytes(snap.node(id))).collect::<Vec<_>>());
}

#[test]
fn an_absent_optional_section_is_recorded_and_its_kinds_are_simply_uninhabited() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-absent-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m, written) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let kz = written.iter().find(|w| w.section == Section::Kretzmann).unwrap();
    std::fs::remove_file(&kz.path).unwrap();
    std::fs::remove_file(&kz.blob_path).unwrap();
    let snap = open_written(&dir).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Concord, Section::Lexicon]);
    let item = g.comments_on[0].item.erase();
    assert!(snap.node(&item).is_none(), "the CommentaryItem node lives only in kretzmann");
    let verse_pos = atlas_graph_types::edge::at(&g.reading["bible"].order[0]);
    let kinds = snap.edge_summary(&verse_pos);
    assert!(!kinds.keys().any(|k| matches!(
        k,
        atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::CommentsOn, _)
    )));
    // Windows keeps an open section file locked, so the snapshot must be dropped before the file can be removed.
    drop(snap);
    let lx = written.iter().find(|w| w.section == Section::Lexicon).unwrap();
    std::fs::remove_file(&lx.path).unwrap();
    std::fs::remove_file(&lx.blob_path).unwrap();
    let snap = open_written(&dir).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Concord]);
    let entry = atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::LexiconEntry, raw: "G3056".into() };
    assert!(snap.node(&entry).is_none(), "the LexiconEntry node lives only in lexicon");
    assert!(snap.nodes_of_kind(atlas_graph_types::id::NodeKind::LexiconEntry, None, 10).ids.is_empty());
    assert!(!snap.edge_summary(&verse_pos).keys().any(|k| matches!(
        k,
        atlas_graph_types::edge::EdgeKind::Directed(atlas_graph_types::edge::RelationId::Occurs, _)
    )));
    drop(snap);
    let kjv = written.iter().find(|w| w.section == Section::Kjv).unwrap();
    std::fs::remove_file(&kjv.path).unwrap();
    std::fs::remove_file(&kjv.blob_path).unwrap();
    let err = open_written(&dir).unwrap_err();
    assert!(
        err.0.contains("kjv") && err.0.contains(&m.sections[1].logical),
        "a missing REQUIRED section is refused by name and hash (spec 11): {}",
        err.0
    );
}

#[test]
fn a_verse_naming_one_entity_twice_is_one_mention_edge_on_both_arms_with_two_rows_behind_it() {
    // Arrange
    use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
    use atlas_graph_types::adjacency::{Cursor, EdgeQuery};
    use atlas_graph_types::store::GraphQuery;
    let mut g = specimen_graph();
    g.mentions.push(Mentions {
        locus: TextLocus { at: TextRef::Bible(vr(7, 1, 2)), span: Some(span(8, 9)) },
        entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        provenance: "theographic".into(),
    });
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-mention-edge-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let verse = at(&atlas_graph::kjv_adapter::verse_node_id(7, 1, 2));
    let judah = at(&PeopleGroupId::new("tribe-of-judah").erase());
    let mentions = EdgeKind::Directed(RelationId::Mentions, Direction::Forward);
    let mentioned_in = EdgeKind::Directed(RelationId::Mentions, Direction::Inverse);
    let walk = |q: &dyn GraphQuery, p: &atlas_graph_types::id::Position, kind: EdgeKind| {
        let page = q.edges(p, &EdgeQuery { kind, cursor: Cursor::FIRST, limit: 10 });
        (q.edge_summary(p)[&kind], page.entries.len(), page.next, q.rows_behind(&page.entries[0].edge).len())
    };

    // Act
    let answers = [walk(&g, &verse, mentions), walk(&snap, &verse, mentions), walk(&g, &judah, mentioned_in), walk(&snap, &judah, mentioned_in)];

    // Assert
    assert_eq!(answers, [(1, 1, None, 2); 4]);
}

#[test]
fn paging_semantics_match_adjacency_rs_at_every_cursor_and_limit() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-paging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let container = atlas_graph_types::edge::at(
        &g.contains_bible
            .iter()
            .find(|c| matches!(c.content, atlas_graph_types::edge::ContainerContent::Loci(_)))
            .unwrap()
            .container
            .erase(),
    );
    let kind = atlas_graph_types::edge::EdgeKind::Directed(
        atlas_graph_types::edge::RelationId::Contains,
        atlas_graph_types::edge::Direction::Forward,
    );
    assert_eq!(g.edge_summary(&container)[&kind], 2);
    for cursor in [0, 1, 2, 3].map(atlas_graph_types::adjacency::Cursor) {
        for limit in 0..=3 {
            let q = atlas_graph_types::adjacency::EdgeQuery { kind, cursor, limit };
            assert_eq!(snap.edges(&container, &q), g.edges(&container, &q), "cursor {cursor:?} limit {limit}");
        }
    }
}

#[test]
fn the_sqlite_overrides_answer_the_widened_port_exactly_as_the_specimen_graph() {
    use atlas_graph_types::edge::{at, Direction, EdgeId, EdgeKind, RelationId, SymRelationId};
    use atlas_graph_types::adjacency::{Cursor, EdgeQuery};
    use atlas_graph_types::id::{NodeKind, Position};
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db3-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let containers = snap.nodes_of_kind(NodeKind::Container, None, 2);
    assert_eq!(
        containers.ids.iter().map(|i| i.raw.as_str()).collect::<Vec<_>>(),
        ["bible-book-GEN", "bible-chapter-GEN-1"]
    );
    assert_eq!(containers.next, Some(2));
    let rest = snap.nodes_of_kind(NodeKind::Container, Some(2), 10);
    assert_eq!(
        rest.ids.iter().map(|i| i.raw.as_str()).collect::<Vec<_>>(),
        ["concord-ac", "concord-ac-1", "passage-creation"]
    );
    assert_eq!(rest.next, None);
    assert_eq!(snap.nodes_of_kind(NodeKind::LexiconEntry, None, 5).ids.len(), 2, "LEX-1: the specimen carries two entries");
    assert_eq!(snap.nodes_of_kind(NodeKind::Container, Some(1), 0).next, Some(1), "limit 0 with more: next = cursor");
    let located = g
        .edges(
            &at(&g.located_at[0].event.erase()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward), cursor: Cursor::FIRST, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    let r = snap.row_provenance(&located).unwrap();
    assert_eq!((r.family, r.row_id, r.provenance.as_str()), (RowFamily::LocatedAt, 0, "curated/events"));
    let analogue = g
        .edges(
            &at(&g.analogue[0].a.erase()),
            &EdgeQuery { kind: EdgeKind::Symmetric(SymRelationId::Analogue), cursor: Cursor::FIRST, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    assert_eq!(
        snap.row_provenance(&analogue).map(|r| (r.family, r.provenance)),
        Some((RowFamily::Analogue, "curated/analogues".into()))
    );
    let dated = g
        .edges(
            &at(&g.dated_by[1].event.erase()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::DatedBy, Direction::Forward), cursor: Cursor::FIRST, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    let justified = g
        .edges(
            &Position::Edge(dated.clone()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: Cursor::FIRST, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    assert_eq!(snap.row_provenance(&justified), None, "a synthesised edge has no row");
    assert_eq!(g.row_provenance(&justified), None, "and the model agrees");
    assert_eq!(snap.row_provenance(&EdgeId(format!("LocatedAt:{}", "0".repeat(HASH_WIDTH * 2)))), None);
    let v2 = &g.reading["bible"].order[1];
    assert_eq!(snap.position_of("bible", v2), Some(1));
    assert_eq!(snap.position_of("concord", v2), None);
    assert_eq!(snap.position_of("bible", &g.located_at[0].event.erase()), None);
    assert_eq!(snap.position_of("nope", v2), None);
    assert_answers_match(&snap, &g);
}

#[test]
fn the_sqlite_snapshots_version_is_the_manifest_root_and_equals_the_in_memory_root() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4a-root-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    assert_eq!(snap.version().0.hex(), m.root, "SqliteSnapshot::version IS the manifest root");
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g).unwrap(), "and equals the in-memory root (one root, spec 3.4)");
}

#[test]
fn mem_store_stamps_the_same_root_the_sections_carry() {
    use atlas_graph_types::store::{GraphPublisher, MemStore};
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4a-root2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (m, _) = write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let mut store = MemStore::default();
    let v = store.publish(g).unwrap();
    assert_eq!(v.0.hex(), m.root, "what MemStore stamps is what the manifest says");
}

use atlas_graph::sqlite::extras::{read_table, row_body, spec_named, table_specs_of, Col};
use atlas_graph_types::chrono::{ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};

#[test]
fn every_extra_table_spec_matches_its_ddl_and_graph_types_lists_it() {
    for s in Section::SHIPPED {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        create_tables(&conn, s).unwrap();
        create_indexes(&conn, s).unwrap();
        let names: Vec<&str> = table_specs_of(s).iter().map(|t| t.name).collect();
        assert_eq!(names, atlas_graph_types::sections::extra_tables_of(s), "{s:?}: specs and graph-types agree on names and order");
        for spec in table_specs_of(s) {
            let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", spec.name)).unwrap();
            let info: Vec<(String, i64)> = stmt
                .query_map([], |r| Ok((r.get::<_, String>(1)?, r.get::<_, i64>(5)?)))
                .unwrap()
                .map(|r| r.unwrap())
                .collect();
            let cols: Vec<&str> = info.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(cols, spec.columns, "{}: DDL columns == spec.columns, in order", spec.name);
            let mut pk: Vec<(i64, &str)> = info.iter().filter(|(_, k)| *k > 0).map(|(n, k)| (*k, n.as_str())).collect();
            pk.sort();
            let pk: Vec<&str> = pk.into_iter().map(|(_, n)| n).collect();
            assert_eq!(pk, spec.pk, "{}: DDL primary key == spec.pk, in order", spec.name);
        }
    }
    assert!(spec_named("verse").is_some() && spec_named("nope").is_none());
}

#[test]
fn the_graph_derived_extras_of_the_specimen_round_trip_and_agree_with_the_attached_dump() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    atlas_graph::references::compile(&mut g);
    let mut resolved = std::collections::HashMap::new();
    resolved.insert(
        "e1".to_string(),
        ResolvedPlacement {
            date: ResolvedDate {
                from: TimePoint { year: Year::new(-1000).unwrap(), month: Some(3), day: None },
                to: TimePoint { year: Year::new(-999).unwrap(), month: None, day: None },
            },
            seq: SeqKey(0),
            basis: PlacementBasis::Traditional,
        },
    );
    resolved.insert(
        "e2".to_string(),
        ResolvedPlacement {
            date: ResolvedDate {
                from: TimePoint { year: Year::new(-900).unwrap(), month: None, day: None },
                to: TimePoint { year: Year::new(-900).unwrap(), month: None, day: None },
            },
            seq: SeqKey(1),
            basis: PlacementBasis::Textual,
        },
    );
    let mut source_meta = std::collections::HashMap::new();
    source_meta.insert("e1".to_string(), atlas_graph::event_world::SourceEventMeta { to_year: -990, order_key: 7 });
    let chrono = atlas_graph::event_world::ChronologyDerivation {
        order: vec!["e1".to_string(), "e2".to_string()],
        placements: std::collections::HashMap::new(),
        resolved,
        source_meta,
    };
    let mut red = std::collections::HashMap::new();
    red.insert("GEN.1.1".to_string(), vec![(0usize, 5usize), (10, 12)]);
    let extras = Extras::graph_derived(&g, &chrono, &red).unwrap();
    let verse = extras.table("verse").unwrap();
    assert!(verse.rows.iter().any(|r| r == &vec![Col::Text("TextUnit:bible/1.1.1".into()), Col::Int(1), Col::Int(1), Col::Int(1), Col::Text("EXO.1.1".into())]), "{:?}", verse.rows);
    let token = |verse: i64, ord: i64, char_start: i64, char_end: i64| {
        vec![Col::Int(1), Col::Int(1), Col::Int(verse), Col::Int(ord), Col::Int(char_start), Col::Int(char_end)]
    };
    assert_eq!(
        extras.table("kjv_token").unwrap().rows,
        vec![
            token(1, 0, 0, 2),
            token(1, 1, 3, 6),
            token(1, 2, 7, 16),
            token(2, 0, 0, 3),
            token(2, 1, 4, 7),
            token(2, 2, 8, 13),
        ]
    );
    let ed = extras.table("event_date").unwrap();
    assert_eq!(
        ed.rows[0],
        vec![Col::Text("e1".into()), Col::Int(-1000), Col::Int(-999), Col::Int(3), Col::Null, Col::Null, Col::Null, Col::Int(0), Col::Int(1), Col::Int(-990), Col::Int(7)]
    );
    assert_eq!(ed.rows[1][9], Col::Null, "no source_meta entry -> NULL meta_to_year");
    assert_eq!(ed.rows[1][10], Col::Null, "no source_meta entry -> NULL order_key");
    let rl = extras.table("red_letter_span").unwrap();
    assert_eq!(
        rl.rows,
        vec![
            vec![Col::Int(0), Col::Int(1), Col::Int(1), Col::Int(0), Col::Int(0), Col::Int(5)],
            vec![Col::Int(0), Col::Int(1), Col::Int(1), Col::Int(1), Col::Int(10), Col::Int(12)]
        ]
    );
    let place = extras.table("place").unwrap();
    assert_eq!(place.spec.columns, &["node_id", "canonical", "lat", "lon"]);
    assert_eq!(place.rows, vec![vec![Col::Text("Place:ur-1".into()), Col::Text("Ur".into()), Col::Real(30.96), Col::Real(46.1)]]);
    assert_eq!(extras.table("polity_era").unwrap().rows.len(), 2);
    assert_eq!(extras.table("era").unwrap().rows.len(), 1);
    assert_eq!(extras.table("concord_unit").unwrap().rows, vec![vec![Col::Text("TextUnit:concord/1.1.1".into()), Col::Int(1), Col::Int(1), Col::Int(1), Col::Text("BoC 1.1.1".into())]]);
    assert_eq!(
        extras.table("concord_token").unwrap().rows,
        vec![
            vec![Col::Int(1), Col::Int(1), Col::Int(1), Col::Int(0), Col::Int(0), Col::Int(2)],
            vec![Col::Int(1), Col::Int(1), Col::Int(1), Col::Int(1), Col::Int(3), Col::Int(10)],
        ]
    );

    extras.attach(&mut g);
    assert_eq!(g.extra_tables["place"], vec![b"{\"canonical\":\"Ur\",\"lat\":30.96,\"lon\":46.1,\"node_id\":\"Place:ur-1\"}".to_vec()]);
    let dir = std::env::temp_dir().join(format!("db4b-extras-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (_m, written) = write_sections(&g, &extras, "test", &layout_under(&dir)).unwrap();
    assert_eq!(written.iter().map(|w| w.extra_row_count).sum::<usize>(), extras.tables.iter().map(|t| t.rows.len()).sum::<usize>());
    for w in &written {
        let conn = open_read_only(&w.path).unwrap();
        for spec in table_specs_of(w.section) {
            let back = read_table(&conn, spec).unwrap();
            let expected: Vec<Vec<u8>> = g.extra_tables.get(spec.name).cloned().unwrap_or_default();
            let got: Vec<Vec<u8>> = back.iter().map(|r| row_body(spec, r).unwrap()).collect();
            assert_eq!(got, expected, "{}: SELECT … ORDER BY pk re-encodes to the attached bodies", spec.name);
        }
        assert_eq!(logical_hash(&logical_dump_of_db(&conn, w.section).unwrap()), w.logical, "{:?}", w.section);
    }
    let mut bare = specimen_graph();
    bare.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut bare);
    let dir2 = std::env::temp_dir().join(format!("db4b-extras2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir2);
    let (m2, _) = write_sections(&bare, &Extras::default(), "test", &layout_under(&dir2)).unwrap();
    assert_ne!(m2.root, _m.root, "the extras are in the root");
}

#[test]
fn the_writer_lands_cache_files_blobs_and_a_manifest_and_the_source_resolves_by_hash() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4b-blobs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    let (m, written) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    assert_eq!(written.len(), 5);
    for w in &written {
        assert_eq!(w.path, layout.cache_path(&w.logical, SCHEMA_VERSION));
        assert_eq!(w.blob_path, layout.blob_path(w.section.name(), &w.logical));
        assert!(w.blob_path.is_file() && w.path.is_file());
        assert_eq!(sha256_hex_of_file(&w.blob_path).unwrap(), w.blob);
        assert_eq!(std::fs::metadata(&w.blob_path).unwrap().len(), w.bytes);
        assert!(w.bytes < w.uncompressed_bytes, "{}: zstd shrinks a sqlite file", w.section.name());
        assert!(!w.reused_blob);
        let ms = m.sections.iter().find(|s| s.name == w.section.name()).unwrap();
        assert_eq!((ms.blob.as_str(), ms.bytes), (w.blob.as_str(), w.bytes));
    }
    let core_cache = layout.cache_path(&written[0].logical, SCHEMA_VERSION);
    let before = std::fs::read(&core_cache).unwrap();
    std::fs::remove_file(&core_cache).unwrap();
    let src = CommittedZstdSource { layout: layout.clone() };
    let resolved = src.resolve(&m.sections[0]).unwrap();
    assert_eq!(resolved, core_cache);
    assert_eq!(std::fs::read(&resolved).unwrap(), before, "unpacking the blob reproduces the written file byte for byte");
    std::fs::remove_file(&core_cache).unwrap();
    let mut bytes = std::fs::read(&written[0].blob_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    std::fs::write(&written[0].blob_path, &bytes).unwrap();
    let err = src.resolve(&m.sections[0]).unwrap_err().to_string();
    assert!(err.contains(&m.sections[0].blob) && err.contains("transport hash"), "{err}");
    assert!(!core_cache.exists() && !layout.cache_dir.join(format!("{}.sqlite.tmp", written[0].logical)).exists());
    let open_err = open_written(&dir).unwrap_err().to_string();
    assert!(open_err.contains("required section core") && open_err.contains("transport hash"), "{open_err}");
}

#[test]
fn a_corrupt_optional_blob_is_loud_where_a_missing_one_is_merely_absent() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4b-optblob-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    let (_m, written) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let concord = written.iter().find(|w| w.section == Section::Concord).unwrap();
    std::fs::remove_file(&concord.path).unwrap();
    let mut bytes = std::fs::read(&concord.blob_path).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&concord.blob_path, &bytes).unwrap();
    let err = open_written(&dir).unwrap_err().to_string();
    assert!(err.contains("optional section concord") && err.contains("transport hash"), "a present-but-corrupt optional blob is refused (spec 11): {err}");
    std::fs::remove_file(&concord.blob_path).unwrap();
    let snap = open_written(&dir).unwrap();
    assert_eq!(snap.present(), &[Section::Core, Section::Kjv, Section::Kretzmann, Section::Lexicon], "a missing optional blob is simply absent");
}

#[test]
fn a_schema_bump_recompresses_every_blob_even_where_no_logical_moved() {
    // Arrange
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4b-bump-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    let (current, _) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let older_sections: Vec<ManifestSection> =
        current.sections.iter().map(|s| ManifestSection { schema_version: SCHEMA_VERSION - 1, ..s.clone() }).collect();
    let older = Manifest {
        schema: current.schema,
        compiler: current.compiler.clone(),
        built: current.built.clone(),
        root: root_of(&older_sections),
        raw_root: None,
        sections: older_sections,
    };
    write_manifest(&older, &layout.manifest_path()).unwrap();

    // Act
    let (rebuilt, written) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();

    // Assert
    assert_eq!(written.iter().map(|w| w.reused_blob).collect::<Vec<bool>>(), vec![false; written.len()]);
    assert_eq!(rebuilt.sections.iter().map(|s| s.schema_version).collect::<Vec<u32>>(), vec![SCHEMA_VERSION; written.len()]);
}

#[test]
fn a_recompile_is_idempotent_and_reuses_unchanged_blobs() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4b-idem-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    let (m1, w1) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let text1 = std::fs::read_to_string(layout.manifest_path()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (m2, w2) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    assert_eq!(m1, m2, "root, blobs, bytes AND built are unchanged");
    assert_eq!(text1, std::fs::read_to_string(layout.manifest_path()).unwrap());
    assert!(w2.iter().all(|w| w.reused_blob) && w1.iter().all(|w| !w.reused_blob));
    let mut g2 = specimen_graph();
    g2.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g2);
    g2.analogue[0].provenance = "moved".into();
    let (m3, w3) = write_sections(&g2, &Extras::default(), "test", &layout).unwrap();
    assert_ne!(m3.root, m2.root);
    assert_ne!(m3.built, m2.built, "a changed root stamps a new built");
    assert!(!w3[0].reused_blob && w3[1].reused_blob, "core changed and was recompressed; kjv did not and was reused");
    let stale: Vec<String> = std::fs::read_dir(layout.sections_dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("core."))
        .collect();
    assert_eq!(stale.len(), 1, "the stale core blob was deleted: {stale:?}");
    let conn = open_read_only(&w3[1].path).unwrap();
    let built: Option<String> = conn.query_row("SELECT value FROM meta WHERE key = 'built'", [], |r| r.get(0)).ok();
    assert!(built.is_none(), "meta.built is gone (it would move the blob hash every compile)");
}

#[test]
fn the_blob_constants_are_the_specs() {
    assert_eq!(atlas_graph::sqlite::blob::BLOB_CEILING, 104_857_600);
    assert_eq!(atlas_graph::sqlite::blob::ZSTD_LEVEL, 19);
    let layout = SectionLayout::under(std::path::Path::new("data/compiled"));
    assert_eq!(layout.cache_dir, std::path::Path::new("data").join("cache").join("sections"));
    assert_eq!(layout.blob_path("core", "abc"), std::path::Path::new("data/compiled").join("sections").join("core.abc.sqlite.zst"));
    assert_eq!(layout.cache_path("abc", SCHEMA_VERSION), std::path::Path::new("data").join("cache").join("sections").join(format!("abc.{SCHEMA_VERSION}.sqlite")));
}

#[test]
fn the_raw_tree_and_the_other_data_directories_sit_beside_the_compiled_one() {
    // Arrange
    let compiled = std::path::Path::new("data/compiled");

    // Act
    let found = (SectionLayout::under(compiled).raw_dir(), sibling_dir(compiled, "curated"), sibling_dir(std::path::Path::new("compiled"), "raw"));

    // Assert
    assert_eq!(found, (std::path::Path::new("data").join("raw"), std::path::Path::new("data").join("curated"), std::path::PathBuf::from("raw")));
}

#[test]
fn a_directory_with_no_parent_keeps_its_siblings_under_itself() {
    // Arrange
    let root = std::path::Path::new("/");

    // Act
    let raw = sibling_dir(root, "raw");

    // Assert
    assert_eq!(raw, std::path::Path::new("/").join("raw"));
}

#[test]
fn the_snapshot_opens_one_connection_per_worker_and_every_one_answers() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4c-workers-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let snap = SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() }, 3).unwrap();
    assert_eq!(snap.workers(), 3);
    let expected: u64 = snap.manifest().sections.iter().map(|s| std::fs::metadata(layout.cache_path(&s.logical, s.schema_version)).unwrap().len()).sum();
    assert!(expected > 0 && snap.mmap_bytes() == expected, "mmap_size = the attached files' sum");
    for _ in 0..6 {
        snap.with_conn(|c| {
            let n: i64 = c.query_row("SELECT COUNT(*) FROM all_node", [], |r| r.get(0))?;
            assert!(n > 0);
            let m: i64 = c.query_row("PRAGMA mmap_size", [], |r| r.get(0))?;
            assert_eq!(m as u64, snap.mmap_bytes());
            Ok(())
        })
        .unwrap();
    }
    let snap = std::sync::Arc::new(snap);
    let ids: Vec<_> = g.nodes.keys().cloned().collect();
    let expected_ids: std::sync::Arc<Vec<Option<atlas_graph_types::id::AnyNodeId>>> =
        std::sync::Arc::new(ids.iter().map(|id| g.node(id).map(|n| n.id)).collect());
    let ids = std::sync::Arc::new(ids);
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let s = snap.clone();
            let ids = ids.clone();
            let expected_ids = expected_ids.clone();
            std::thread::spawn(move || {
                for _ in 0..50 {
                    for (id, want) in ids.iter().zip(expected_ids.iter()) {
                        assert_eq!(s.node(id).map(|n| n.id), *want);
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    assert_answers_match(&*snap, &g);
    assert!(snap.absent().is_empty());
    assert!(
        SqliteSnapshot::open_with_workers(&layout.manifest_path(), &CommittedZstdSource { layout: layout.clone() }, 0).is_err(),
        "zero workers is refused"
    );
}

#[test]
fn a_section_with_an_unknown_user_version_is_refused_like_an_old_artifact() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4c-wall-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let layout = layout_under(&dir);
    let (m, _) = write_sections(&g, &Extras::default(), "test", &layout).unwrap();
    let concord = layout.cache_path(&m.sections[2].logical, m.sections[2].schema_version);
    let conn = rusqlite::Connection::open(&concord).unwrap();
    conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    drop(conn);
    let err = open_written(&dir).unwrap_err().to_string();
    assert!(err.contains("section concord user_version 99 unsupported") && err.contains(&format!("understands {SCHEMA_VERSION}")), "{err}");
}

use atlas_core::data::demo_fixture;
use atlas_core::sources::SourcesDocument;
use atlas_graph::sqlite::sidecars::{fold_sidecars, unfold};

#[test]
fn a_place_date_claim_round_trips_through_the_sidecar_with_the_event_it_names() {
    // Arrange
    let mut atlas = demo_fixture();
    atlas.place_history.get_mut("hebron").unwrap().established.as_mut().unwrap().event = Some(EventId::new("theo-87"));
    let mut extras = Extras::default();
    extras.extend(fold_sidecars(&atlas, &SourcesDocument::default()).unwrap());
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db4b-claim-event-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &extras, "test", &layout_under(&dir)).unwrap();

    // Act
    let (unfolded, _) = open_written(&dir).unwrap().with_conn(unfold).unwrap();

    // Assert
    assert_eq!(unfolded.place_history, atlas.place_history);
}

#[test]
fn every_held_position_answers_one_compiled_label_and_every_edge_its_record_from_the_sections() {
    // Arrange
    let mut g = every_end_held(specimen_graph());
    let atlas = atlas_core::data::AtlasData::default();
    let geography = atlas_graph::geography::Geography::compile(&g, &atlas);
    atlas_graph::labels::compile(&mut g, &atlas_graph::labels::ReaderNames::of(&geography, &atlas));
    let dir = std::env::temp_dir().join(format!("f6-labels-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let positions: Vec<atlas_graph_types::id::Position> = g.positions().into_iter().collect();

    // Act
    let admitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_answers_match(&snap, &g))).is_ok();
    let unlabelled = snap.labels(&positions).iter().filter(|label| label.is_none()).count();
    let edges_read = g.edges_by_id.keys().filter(|id| snap.edge(id).is_some()).count();

    // Assert
    assert_eq!((admitted, unlabelled, edges_read), (true, 0, g.edges_by_id.len()));
}

#[test]
fn every_text_unit_answers_one_compiled_reference_and_no_other_node_answers_one() {
    // Arrange
    use atlas_graph_types::id::{AnyNodeId, NodeKind};
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    atlas_graph::references::compile(&mut g);
    let no_chronology = atlas_graph::event_world::ChronologyDerivation {
        order: Vec::new(),
        placements: std::collections::HashMap::new(),
        resolved: std::collections::HashMap::new(),
        source_meta: std::collections::HashMap::new(),
    };
    let extras = Extras::graph_derived(&g, &no_chronology, &std::collections::HashMap::new()).unwrap();
    extras.attach(&mut g);
    let dir = std::env::temp_dir().join(format!("f2-references-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &extras, "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let asked = [
        AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() },
        AnyNodeId { kind: NodeKind::TextUnit, raw: "concord/1.1.1".into() },
        AnyNodeId { kind: NodeKind::Era, raw: "patriarchs".into() },
    ];

    // Act
    let admitted = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_answers_match(&snap, &g))).is_ok();
    let read = snap.references(&asked);

    // Assert
    assert_eq!((admitted, read), (true, vec![Some("EXO.1.1".to_string()), Some("BoC 1.1.1".to_string()), None]));
}

fn every_end_held(mut g: atlas_graph_types::graph::Graph) -> atlas_graph_types::graph::Graph {
    use atlas_graph_types::id::Position;
    use atlas_graph_types::node::{Node, NodePayload};
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let atlas = atlas_core::data::AtlasData::default();
    let geography = atlas_graph::geography::Geography::default();
    let names = atlas_graph::labels::ReaderNames::of(&geography, &atlas);
    let unheld: Vec<_> = g
        .positions()
        .into_iter()
        .filter_map(|p| match p {
            Position::Node(id) if !g.nodes.contains_key(&id) && atlas_graph::labels::node_label(&id, None, &names).is_none() => Some(id),
            _ => None,
        })
        .collect();
    for id in unheld {
        let payload = NodePayload::Source { label: id.raw.clone() };
        g.nodes.insert(id.clone(), Node { id, payload, provenance: "test".into() });
    }
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    g
}

fn edge_summary_with_its_vm_steps(snap: &SqliteSnapshot, at: &atlas_graph_types::id::Position) -> (atlas_graph_types::adjacency::EdgeSummary, u64) {
    use std::sync::atomic::{AtomicU64, Ordering};
    let steps = std::sync::Arc::new(AtomicU64::new(0));
    let counter = std::sync::Arc::clone(&steps);
    snap.with_conn(|conn| {
        conn.progress_handler(1, Some(move || {
            counter.fetch_add(1, Ordering::Relaxed);
            false
        }));
        Ok(())
    })
    .unwrap();
    let summary = snap.edge_summary(at);
    (summary, steps.load(Ordering::Relaxed))
}

#[test]
fn an_edge_summary_costs_the_same_work_however_many_edges_its_position_holds() {
    // Arrange
    use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
    let hot = PlaceId::new("crowded-place");
    let summarised_at_degree = |degree: usize| {
        let mut g = specimen_graph();
        for i in 0..degree {
            g.located_at.push(LocatedAt { event: EventId::new(format!("crowd-{i}")), place: hot.clone(), provenance: "p".into(), justification: Justification::default() });
        }
        g.build_indexes();
        atlas_graph::event_world::add_justified_by(&mut g);
        let dir = std::env::temp_dir().join(format!("fix2s-growth-{degree}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
        let snap = open_written(&dir).unwrap();
        edge_summary_with_its_vm_steps(&snap, &at(&hot.erase()))
    };

    // Act
    let (few, few_steps) = summarised_at_degree(10);
    let (many, many_steps) = summarised_at_degree(10_000);

    // Assert
    let located_here = EdgeKind::Directed(RelationId::LocatedAt, Direction::Inverse);
    assert_eq!(
        (few.keys().collect::<Vec<_>>(), few[&located_here], many[&located_here], many_steps),
        (many.keys().collect::<Vec<_>>(), 10, 10_000, few_steps)
    );
}

#[test]
fn only_the_first_page_has_no_previous_and_every_other_previous_reads_the_page_before_it_in_both_stores() {
    // Arrange
    use atlas_graph_types::adjacency::{Cursor, EdgePage, EdgeQuery};
    use atlas_graph_types::id::Position;
    let mut g = specimen_graph();
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("passage-of-five"),
        content: ContainerContent::Loci(LocusSet((1..=5).map(|verse| bl(1, 1, verse)).collect())),
        provenance: "kjv".into(),
        justification: Justification::default(),
    });
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("fix3-previous-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    let stores: [&dyn GraphQuery; 2] = [&g, &snap];
    let walk = |store: &dyn GraphQuery, at: &Position, kind, limit| {
        let mut pages: Vec<EdgePage> = vec![store.edges(at, &EdgeQuery { kind, cursor: Cursor::FIRST, limit })];
        while let Some(cursor) = pages[pages.len() - 1].next {
            pages.push(store.edges(at, &EdgeQuery { kind, cursor, limit }));
        }
        pages
    };

    // Act
    let mut offenders = Vec::new();
    let mut walked = 0;
    let mut three_with_a_partial_final = 0;
    for store in stores {
        for at in g.positions() {
            for kind in store.edge_summary(&at).into_keys() {
                for limit in 1..=3 {
                    let pages = walk(store, &at, kind, limit);
                    walked += pages.len();
                    let before: Vec<Option<EdgePage>> = pages.iter().map(|page| page.previous.map(|cursor| store.edges(&at, &EdgeQuery { kind, cursor, limit }))).collect();
                    let expected: Vec<Option<EdgePage>> = std::iter::once(None).chain(pages.iter().take(pages.len() - 1).cloned().map(Some)).collect();
                    if before != expected {
                        offenders.push(format!("{at:?} {kind:?} limit {limit}"));
                    }
                    if pages.len() >= 3 && pages[pages.len() - 1].entries.len() < limit {
                        three_with_a_partial_final += 1;
                    }
                }
            }
        }
    }

    // Assert
    assert_eq!((walked > 100, three_with_a_partial_final > 0, offenders), (true, true, Vec::<String>::new()));
}

#[test]
fn every_cell_of_every_table_a_section_file_holds_but_meta_is_under_its_logical_hash() {
    // Arrange
    let mut g = every_end_held(specimen_graph());
    let atlas = atlas_core::data::AtlasData::default();
    let geography = atlas_graph::geography::Geography::compile(&g, &atlas);
    atlas_graph::labels::compile(&mut g, &atlas_graph::labels::ReaderNames::of(&geography, &atlas));
    atlas_graph::references::compile(&mut g);
    let no_chronology = atlas_graph::event_world::ChronologyDerivation {
        order: Vec::new(),
        placements: std::collections::HashMap::new(),
        resolved: std::collections::HashMap::new(),
        source_meta: std::collections::HashMap::new(),
    };
    let extras = Extras::graph_derived(&g, &no_chronology, &std::collections::HashMap::new()).unwrap();
    extras.attach(&mut g);
    let dir = std::env::temp_dir().join(format!("f39-every-cell-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let (_, written) = write_sections(&g, &extras, "test", &layout_under(&dir)).unwrap();

    // Act
    let mut unhashed: Vec<String> = Vec::new();
    let mut unproven: BTreeSet<String> = BTreeSet::new();
    let mut proven: BTreeSet<String> = BTreeSet::new();
    for w in &written {
        let probe = dir.join(format!("probe-{}.sqlite", w.section.name()));
        std::fs::copy(&w.path, &probe).unwrap();
        let conn = rusqlite::Connection::open(&probe).unwrap();
        let dumped = logical_table_order(w.section);
        for (table, column, key) in cells_of(&conn) {
            conn.execute_batch("BEGIN").unwrap();
            let changed = conn.execute(&perturb_one_cell(&table, &column, &key), []).unwrap();
            let moved = logical_dump_of_db(&conn, w.section).map_or(true, |dump| logical_hash(&dump) != w.logical);
            conn.execute_batch("ROLLBACK").unwrap();
            match (changed, moved) {
                (0, _) if !dumped.contains(&table.as_str()) => {
                    unproven.insert(table);
                }
                (0, _) => {}
                (_, false) => unhashed.push(format!("{}.{table}.{column}", w.section.name())),
                _ => {
                    proven.insert(table);
                }
            }
        }
    }
    let unreached: Vec<String> = unproven.difference(&proven).cloned().collect();

    // Assert
    assert_eq!((unhashed, unreached), (Vec::<String>::new(), Vec::<String>::new()));
}

fn cells_of(conn: &rusqlite::Connection) -> Vec<(String, String, Vec<String>)> {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name != 'meta' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let mut cells = Vec::new();
    for table in tables {
        let columns: Vec<(String, i64)> = conn
            .prepare(&format!("SELECT name, pk FROM pragma_table_info('{table}') ORDER BY cid"))
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let mut key: Vec<(i64, String)> = columns.iter().filter(|(_, pk)| *pk > 0).map(|(name, pk)| (*pk, name.clone())).collect();
        key.sort();
        let key: Vec<String> = key.into_iter().map(|(_, name)| name).collect();
        for (column, _) in &columns {
            cells.push((table.clone(), column.clone(), key.clone()));
        }
    }
    cells
}

fn perturb_one_cell(table: &str, column: &str, key: &[String]) -> String {
    let key = key.join(", ");
    format!(
        "UPDATE {table} SET {column} = CASE typeof({column}) \
           WHEN 'integer' THEN {column} + 1000003 WHEN 'real' THEN {column} + 0.5 WHEN 'text' THEN {column} || '~' \
           ELSE randomblob(length({column})) END \
         WHERE ({key}) = (SELECT {key} FROM {table} WHERE {column} IS NOT NULL ORDER BY {key} LIMIT 1)"
    )
}
