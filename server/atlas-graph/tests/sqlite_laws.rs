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

// ---------------------------------------------------------------------
// Task 4: row writers and readers for all 21 families
// ---------------------------------------------------------------------
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
/// A justification exercising all three `Ground` variants and the prose.
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

/// One hand-built row per family plus the nodes and spines the rows
/// refer to. The 21 row constructions are COPIED VERBATIM from the golden
/// test `graph-types/tests/canon_row_vectors.rs` (DB-2a), so their
/// canonical bytes are ALREADY pinned there; extra rows cover the other
/// `ContainerContent` shape of each corpus and every `DatePlacement` kind.
fn specimen_graph() -> atlas_graph_types::graph::Graph {
    use atlas_graph_types::graph::{Graph, ReadingSpine};
    use atlas_graph_types::id::{AnyNodeId, NodeKind};
    use atlas_graph_types::node::{Node, NodePayload};
    let mut g = Graph::default();
    // 1. contains_bible -- the flat-loci content, a non-empty set.
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("bible/GEN.1"),
        content: ContainerContent::Loci(LocusSet([bl(1, 1, 1), bl(1, 1, 2)].into_iter().collect())),
        provenance: "kjv".into(),
        justification: Justification::default(),
    });
    // 1b. contains_bible -- the child-container content.
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("bible/GEN"),
        content: ContainerContent::Container(ContainerNodeId::new("bible/GEN.1")),
        provenance: "kjv".into(),
        justification: full_justification(),
    });
    // 2. contains_concord -- the recursive child-container content.
    g.contains_concord.push(Contains::<ConcordTag> {
        container: ContainerNodeId::new("concord/ac"),
        content: ContainerContent::Container(ContainerNodeId::new("concord/ac.1")),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    // 2b. contains_concord -- a loci set of two, one with a span.
    g.contains_concord.push(Contains::<ConcordTag> {
        container: ContainerNodeId::new("concord/ac.1"),
        content: ContainerContent::Loci(LocusSet(
            [cl(1, 1, 1), Locus { unit: ConcordRef { part: 1, article: 1, paragraph: 2 }, span: Some(span(0, 3)) }]
                .into_iter()
                .collect(),
        )),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    // 3. attests
    g.attests.push(Attests {
        event: EventId::new("jesus-baptized"),
        attestation: blr((40, 3, 13), (40, 3, 17)),
        provenance: "curated/events".into(),
        justification: full_justification(),
    });
    // 4. succession
    g.succession.push(
        Succession::new(
            NarrativeId::new("life-of-christ"),
            vec![EventId::new("nativity"), EventId::new("jesus-baptized")],
            "curated/narratives".into(),
            Justification::default(),
        )
        .expect("a distinct, non-empty chain"),
    );
    // 5. canon_succession
    g.canon_succession.push(CanonSuccession {
        prior: ContainerNodeId::new("bible/GEN.50"),
        next: ContainerNodeId::new("bible/EXO.1"),
        provenance: "canon".into(),
        justification: Justification::default(),
    });
    // 6. dated_by (the four placement kinds, so every reader arm fires)
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
    // 7. located_at
    g.located_at.push(LocatedAt {
        event: EventId::new("jesus-baptized"),
        place: PlaceId::new("jordan-river"),
        provenance: "curated/events".into(),
        justification: full_justification(),
    });
    // 8. fulfills
    g.fulfills.push(Fulfills {
        prophecy: blr((23, 7, 14), (23, 7, 14)),
        fulfillment: blr((40, 1, 22), (40, 1, 23)),
        provenance: "curated/fulfillment".into(),
        justification: full_justification(),
    });
    // 9. typology -- `note` present.
    g.typology.push(Typology {
        type_passage: blr((4, 21, 8), (4, 21, 9)),
        antitype_passage: blr((43, 3, 14), (43, 3, 14)),
        note: Some("the brasen serpent".into()),
        provenance: "curated/typology".into(),
        justification: Justification::default(),
    });
    // 10. named_after
    g.named_after.push(NamedAfter {
        namesake: Namesake::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        eponym: PersonId::new("judah"),
        provenance: "curated/peoples".into(),
        justification: Justification::default(),
    });
    // 11. catechism -- a Concord-side TextLocus.
    g.catechism.push(CatechismLink {
        locus: TextLocus { at: TextRef::Concord(ConcordRef { part: 1, article: 2, paragraph: 3 }), span: None },
        item: CatechismItemId::new("sc/1st-commandment"),
        provenance: "small-catechism".into(),
        justification: Justification::default(),
    });
    // 12. comments_on
    g.comments_on.push(CommentsOn {
        item: CommentaryItemId::new("kretzmann/JHN.3.16"),
        on: blr((43, 3, 16), (43, 3, 16)),
        provenance: "kretzmann".into(),
        justification: Justification::default(),
    });
    // 13. spoken_by
    g.spoken_by.push(SpokenBy {
        locus: blr((43, 3, 16), (43, 3, 21)),
        speaker: PersonId::new("jesus"),
        provenance: "red-letter".into(),
        justification: Justification::default(),
    });
    // 14. spoken_at
    g.spoken_at.push(SpokenAt {
        locus: blr((43, 3, 16), (43, 3, 21)),
        place: PlaceId::new("jerusalem"),
        provenance: "red-letter".into(),
        justification: Justification::default(),
    });
    // 15. mentions -- a locus WITH a token span (the layer-tagged case).
    g.mentions.push(Mentions {
        locus: TextLocus { at: TextRef::Bible(vr(7, 1, 2)), span: Some(span(3, 5)) },
        entity: MentionedEntity::PeopleGroup(PeopleGroupId::new("tribe-of-judah")),
        provenance: "theographic".into(),
    });
    // 16. cross_refs -- `to_last` present, votes carried.
    g.cross_refs.push(CrossRef {
        from: tl(51, 1, 15),
        to: tl(51, 1, 16),
        to_last: Some(tl(51, 1, 19)),
        target_display: "COL.1.16-19".into(),
        votes: 7,
        provenance: "openbible-xrefs".into(),
    });
    // 17. quotes
    g.quotes.push(Quotes { quoting: tl(40, 4, 4), quoted: blr((5, 8, 3), (5, 8, 3)), provenance: "curated/quotes".into() });
    // 18. confesses -- a Concord locus confessing Scripture.
    g.confesses.push(Confesses {
        confessing: cl(1, 2, 3),
        confessed: blr((45, 3, 28), (45, 3, 28)),
        provenance: "concord".into(),
        justification: full_justification(),
    });
    // 19. corresponds_bible -- span-level alignment.
    g.corresponds_bible.push(Corresponds::<BibleTag> {
        a: Locus { unit: vr(43, 3, 16), span: Some(span(0, 4)) },
        b: Locus { unit: vr(43, 3, 16), span: Some(span(5, 9)) },
        provenance: "alignment".into(),
    });
    // 20. temporal_adjacency
    g.temporal_adjacency.push(TemporalAdjacency {
        earlier: EventId::new("nativity"),
        later: EventId::new("jesus-baptized"),
        provenance: "derived/chronology".into(),
    });
    // 21. analogue
    g.analogue.push(Analogue {
        a: EventId::new("leper-healed-galilee"),
        b: EventId::new("leper-healed-capernaum"),
        provenance: "curated/analogues".into(),
    });

    // Nodes the rows can reach from an edge endpoint, and the two spines.
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
        node(NodeKind::TextUnit, "concord/1.1.1", unit("concord", "en", "We believe")),
        node(NodeKind::Container, "bible/GEN", NodePayload::Container { title: "Genesis".into() }),
        node(NodeKind::Container, "bible/GEN.1", NodePayload::Container { title: "Genesis 1".into() }),
        node(NodeKind::Container, "concord/ac", NodePayload::Container { title: "Augsburg Confession".into() }),
        node(NodeKind::Container, "concord/ac.1", NodePayload::Container { title: "Article I".into() }),
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

/// Every row of the section's families, borrowed, in family order --
/// the explicit per-family chain (Task 5 swaps in `partition::rows_of_section`).
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
