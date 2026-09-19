//! DB-2b laws on synthetic data: the seams the section writer and the
//! SqliteSnapshot are built from, each proven in isolation before the
//! real-data gate (`sqlite_real_data.rs`) composes them.
use atlas_graph::sqlite::blob::sha256_hex_of_file;
use atlas_graph::sqlite::extras::Extras;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout, SectionSource};
use atlas_graph::sqlite::{
    hash_bytes, hash_from_bytes, open_read_only, stamp_pragmas, APPLICATION_ID, HASH_WIDTH,
    SCHEMA_VERSION,
};
use atlas_graph_types::id::ContentHash;

/// DB-4b: a test's private layout -- `compiled/` (manifest + `sections/`)
/// and `cache/sections/` under one temp dir.
fn layout_under(dir: &std::path::Path) -> SectionLayout {
    SectionLayout { compiled_dir: dir.join("compiled"), cache_dir: dir.join("cache").join("sections") }
}

/// Opens what `write_sections` wrote under `dir`, through the committed source.
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

// ---------------------------------------------------------------------
// Task 3: DDL per section; the LOCUS / RANGE / AUTHORED column codec
// ---------------------------------------------------------------------
use atlas_graph::sections::Section;
use atlas_graph::sqlite::columns::{
    bible_locus_values, locus_columns, read_bible_locus, read_justification, JustificationWriter,
};
use atlas_graph::sqlite::ddl::{create_indexes, create_tables, logical_table_order, row_tables_of};
use atlas_graph_types::canon::{Canon, RowFamily};
use atlas_graph_types::edge::{Ground, Justification, Occurs};
use atlas_graph_types::id::{AnchorId, SourceId, LexiconEntryId};
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
/// canonical bytes are ALREADY pinned there (container ids renamed so
/// spec 2.1's placement rule splits them: `passage-` -> core,
/// `bible-book-`/`bible-chapter-` -> kjv, `concord-` -> concord); extra
/// rows cover the other `ContainerContent` shape of each corpus and every
/// `DatePlacement` kind.
fn specimen_graph() -> atlas_graph_types::graph::Graph {
    use atlas_graph_types::graph::{Graph, ReadingSpine};
    use atlas_graph_types::id::{AnyNodeId, NodeKind};
    use atlas_graph_types::node::{Node, NodePayload};
    let mut g = Graph::default();
    // 1. contains_bible -- the flat-loci content, a non-empty set.
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("passage-creation"),
        content: ContainerContent::Loci(LocusSet([bl(1, 1, 1), bl(1, 1, 2)].into_iter().collect())),
        provenance: "kjv".into(),
        justification: Justification::default(),
    });
    // 1b. contains_bible -- the child-container content.
    g.contains_bible.push(Contains::<BibleTag> {
        container: ContainerNodeId::new("bible-book-GEN"),
        content: ContainerContent::Container(ContainerNodeId::new("bible-chapter-GEN-1")),
        provenance: "kjv".into(),
        justification: full_justification(),
    });
    // 2. contains_concord -- the recursive child-container content.
    g.contains_concord.push(Contains::<ConcordTag> {
        container: ContainerNodeId::new("concord-ac"),
        content: ContainerContent::Container(ContainerNodeId::new("concord-ac-1")),
        provenance: "concord".into(),
        justification: Justification::default(),
    });
    // 2b. contains_concord -- a loci set of two, one with a span.
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

    // 22. occurs (LEX-1): two tokens of one entry in one verse (ONE edge,
    // two rows behind it), and one Hebrew token on the next verse.
    let word = |book: u8, chapter: u16, verse: u16, layer: &str, tok: u16| TextLocus {
        at: TextRef::Bible(vr(book, chapter, verse)),
        span: Some(TokenSpan::new(TranslationId(layer.into()), tok, tok).unwrap()),
    };
    g.occurs.push(Occurs { entry: LexiconEntryId::new("G3056"), locus: word(1, 1, 1, "greek_textus_receptus", 1), provenance: "stepbible-tagnt".into() });
    g.occurs.push(Occurs { entry: LexiconEntryId::new("G3056"), locus: word(1, 1, 1, "greek_textus_receptus", 3), provenance: "stepbible-tagnt".into() });
    g.occurs.push(Occurs { entry: LexiconEntryId::new("H0430"), locus: word(1, 1, 2, "hebrew_masoretic", 2), provenance: "stepbible-tahot".into() });

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
        // DB-4b: one node of each projected kind (place / era / polity_era).
        node(
            NodeKind::Place,
            "ur-1",
            NodePayload::Place { canonical: "Ur".into(), lat: 30.96, lon: 46.1, aliases: vec![], description: None },
        ),
        node(NodeKind::Era, "patriarchs", NodePayload::Era { label: "Patriarchs".into(), from_year: -2100, to_year: -1800 }),
        // LEX-1: the two entries the occurs rows name (one with domains, one bare).
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
                domains: vec!["13.115".into(), "33.98".into()],
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
                domains: vec![],
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
            RowFamily::Occurs => out.extend(g.occurs.iter().map(RowRef::Occurs)),
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

// ---------------------------------------------------------------------
// Task 5: the partition, the manifest, and the writer
// ---------------------------------------------------------------------
use atlas_graph::sqlite::manifest::{read_manifest, root_of, Manifest, ManifestSection};
use atlas_graph::sqlite::partition::{edge_row_map, partition};
use atlas_graph::sqlite::writer::write_sections;

#[test]
fn every_index_entry_of_the_specimen_lands_in_exactly_one_section_and_names_its_row() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let parts = partition(&g).unwrap();
    let total: usize = parts.iter().map(|p| p.edges.len()).sum();
    let in_memory: usize = g
        .indexes
        .values()
        .map(|ix| ix.fwd.values().map(Vec::len).sum::<usize>() + ix.inv.values().map(Vec::len).sum::<usize>())
        .sum::<usize>()
        + g.symmetric_indexes.values().map(|ix| ix.fwd.values().map(Vec::len).sum::<usize>()).sum::<usize>();
    assert_eq!(total, in_memory, "no entry lost, none duplicated");
    assert!(total > 0, "the specimen graph indexes something");
    let map = edge_row_map(&g);
    let justified_code = atlas_graph_types::edge::RelationId::ALL
        .iter()
        .position(|r| *r == atlas_graph_types::edge::RelationId::JustifiedBy)
        .unwrap() as i64;
    let mut saw_justified = false;
    for p in &parts {
        for e in &p.edges {
            if e.rel != justified_code {
                let rows = &map[&e.edge_id];
                let (_, _, raw) = rows.iter().find(|(fam, ord, _)| (e.row_family, e.row_id) == (*fam, *ord)).unwrap_or_else(|| panic!("entry {:?} names a row that does not mint its id", (e.row_family, e.row_id)));
                assert_eq!(atlas_graph::sections::section_of_justified_by(e.row_family, raw.as_deref()), p.section);
            } else {
                saw_justified = true;
                // A justified-by entry's row is its SOURCE row, and its
                // section is that row's section.
                let end = if e.dir == 0 { &e.subject } else { &e.object };
                let source = match end {
                    atlas_graph_types::id::Position::Edge(id) => id,
                    other => panic!("justified-by source end must be an edge, got {other:?}"),
                };
                let (fam, ord, raw) = &map[source][0];
                assert_eq!((e.row_family, e.row_id), (*fam, *ord));
                assert_eq!(atlas_graph::sections::section_of_justified_by(*fam, raw.as_deref()), p.section);
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
        schema_version: 14,
    };
    let sections = vec![s("core", true, &"a".repeat(32)), s("kjv", true, &"b".repeat(32)), s("concord", false, &"c".repeat(32))];
    let root = root_of(&sections);
    assert_eq!(root.len(), 32);
    let m = Manifest {
        schema: 1,
        compiler: "test".into(),
        built: "2026-09-17T00:00:00Z".into(),
        root: root.clone(),
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
        assert_eq!(w.path.file_name().unwrap().to_str().unwrap(), format!("{}.sqlite", ms.logical), "the cache file is named by the logical hash");
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

// ---------------------------------------------------------------------
// Task 6: the logical dump from the SQLite file agrees with the partition
// ---------------------------------------------------------------------
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
        let from_mem = logical_dump_section(&g, p.section);
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

// ---------------------------------------------------------------------
// Task 7: SqliteSnapshot -- the read port over the attached sections
// ---------------------------------------------------------------------
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
    // LEX-1: two rows behind ONE occurs-in edge (the leper lesson), the
    // verse's `words` summary, and the inverse page back to the entry.
    {
        use atlas_graph_types::edge::{at, Direction, EdgeKind, RelationId};
        use atlas_graph_types::explore::EdgeQuery;
        use atlas_graph_types::id::{AnyNodeId, NodeKind};
        let entry = at(&AnyNodeId { kind: NodeKind::LexiconEntry, raw: "G3056".into() });
        let verse = at(&AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/1.1.1".into() });
        let page = snap.edges_with_nodes(&entry, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Forward), cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 2, "two tokens of one entry in one verse: two index entries (one per row)...");
        assert_eq!(page.entries[0].entry.edge, page.entries[1].entry.edge, "...under ONE edge id (the leper lesson)");
        assert_eq!(page.entries[0].entry.node, verse);
        let rows = snap.rows_behind(&page.entries[0].entry.edge);
        assert_eq!(rows.len(), 2, "...with BOTH rows behind it");
        assert_eq!(rows.iter().map(|r| (r.family, r.row_id)).collect::<Vec<_>>(), [(RowFamily::Occurs, 0), (RowFamily::Occurs, 1)]);
        assert!(rows.iter().all(|r| r.provenance == "stepbible-tagnt"));
        let summary = snap.edge_summary(&verse);
        assert_eq!(summary.get(&EdgeKind::Directed(RelationId::Occurs, Direction::Inverse)).copied(), Some(2), "`words` at the verse: its two tagged tokens");
        let back = snap.edges_with_nodes(&verse, &EdgeQuery { kind: EdgeKind::Directed(RelationId::Occurs, Direction::Inverse), cursor: None, limit: 10 });
        assert_eq!(back.entries[0].entry.node, entry);
        assert_eq!(snap.nodes_of_kind(NodeKind::LexiconEntry, None, 5).ids.len(), 2);
    }
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g), "SqliteSnapshot::version is the manifest root = the in-memory root");
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
    drop(snap); // Windows holds an open section file locked
    // LEX-1: the lexicon section is optional too -- absent, its entries are
    // uninhabited and a verse has no `words`.
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
fn paging_semantics_match_explore_rs_at_every_cursor_and_limit() {
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db2b-paging-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    // The container with a Loci set of two verses has 2 Contains entries:
    // walk every (cursor, limit) in 0..=3.
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
    for cursor in [None, Some(0), Some(1), Some(2), Some(3)] {
        for limit in 0..=3 {
            let q = atlas_graph_types::explore::EdgeQuery { kind, cursor, limit };
            assert_eq!(snap.edges(&container, &q), g.edges(&container, &q), "cursor {cursor:?} limit {limit}");
        }
    }
}

// ---------------------------------------------------------------------
// DB-3 Task 3: the SqliteSnapshot overrides for the widened port
// ---------------------------------------------------------------------
#[test]
fn the_sqlite_overrides_answer_the_widened_port_exactly_as_the_specimen_graph() {
    use atlas_graph_types::edge::{at, Direction, EdgeId, EdgeKind, RelationId, SymRelationId};
    use atlas_graph_types::explore::EdgeQuery;
    use atlas_graph_types::id::{NodeKind, Position};
    let mut g = specimen_graph();
    g.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut g);
    let dir = std::env::temp_dir().join(format!("db3-snap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    write_sections(&g, &Extras::default(), "test", &layout_under(&dir)).unwrap();
    let snap = open_written(&dir).unwrap();
    // nodes_of_kind pages across sections (Container lives in core, kjv AND
    // concord) in one byte order.
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
    // row_provenance: a directed row, a symmetric row, a justified-by edge
    // (None, judgment call 3), an unknown id (None).
    let located = g
        .edges(
            &at(&g.located_at[0].event.erase()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward), cursor: None, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    let r = snap.row_provenance(&located).unwrap();
    assert_eq!((r.family, r.row_id, r.provenance.as_str()), (RowFamily::LocatedAt, 0, "curated/events"));
    let analogue = g
        .edges(
            &at(&g.analogue[0].a.erase()),
            &EdgeQuery { kind: EdgeKind::Symmetric(SymRelationId::Analogue), cursor: None, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    assert_eq!(
        snap.row_provenance(&analogue).map(|r| (r.family, r.provenance)),
        Some((RowFamily::Analogue, "curated/analogues".into()))
    );
    // add_justified_by fans out grounds for DatedBy/Fulfills/Typology/NamedAfter
    // rows only; the specimen's second dated_by row carries full_justification().
    let dated = g
        .edges(
            &at(&g.dated_by[1].event.erase()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::DatedBy, Direction::Forward), cursor: None, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    let justified = g
        .edges(
            &Position::Edge(dated.clone()),
            &EdgeQuery { kind: EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: None, limit: 1 },
        )
        .entries[0]
        .edge
        .clone();
    assert_eq!(snap.row_provenance(&justified), None, "a synthesised edge has no row");
    assert_eq!(g.row_provenance(&justified), None, "and the model agrees");
    assert_eq!(snap.row_provenance(&EdgeId(format!("LocatedAt:{}", "0".repeat(HASH_WIDTH * 2)))), None);
    // position_of over both spines and off-spine.
    let v2 = &g.reading["bible"].order[1];
    assert_eq!(snap.position_of("bible", v2), Some(1));
    assert_eq!(snap.position_of("concord", v2), None);
    assert_eq!(snap.position_of("bible", &g.located_at[0].event.erase()), None);
    assert_eq!(snap.position_of("nope", v2), None);
    // And the whole harness, which now covers the five methods.
    assert_answers_match(&snap, &g);
}

// ---------------------------------------------------------------------
// DB-4a: one root
// ---------------------------------------------------------------------
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
    assert_eq!(snap.version().0, atlas_graph_types::sections::version_root(&g), "and equals the in-memory root (one root, spec 3.4)");
}

/// DB-4a: `MemStore::publish` stamps the manifest root (the atlas workspace
/// runs `canon-ids` ON).
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
    let v = store.publish(g);
    assert_eq!(v.0.hex(), m.root, "what MemStore stamps is what the manifest says");
}

// ---------------------------------------------------------------------
// DB-4b: extras -- the non-graph tables (projections, chronology, headings,
// red-letter spans, sidecars) through one row shape
// ---------------------------------------------------------------------
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
    // DB-4c: source_meta rides along (the Event wire's curated to_year/order_key); e2 has none.
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
    assert!(verse.rows.iter().any(|r| r == &vec![Col::Text("TextUnit:bible/1.1.1".into()), Col::Int(1), Col::Int(1), Col::Int(1)]), "{:?}", verse.rows);
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
    assert_eq!(extras.table("concord_unit").unwrap().rows, vec![vec![Col::Text("TextUnit:concord/1.1.1".into()), Col::Int(1), Col::Int(1), Col::Int(1)]]);

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
    // a graph nobody attached to writes empty extra tables and a different root
    let mut bare = specimen_graph();
    bare.build_indexes();
    atlas_graph::event_world::add_justified_by(&mut bare);
    let dir2 = std::env::temp_dir().join(format!("db4b-extras2-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir2);
    let (m2, _) = write_sections(&bare, &Extras::default(), "test", &layout_under(&dir2)).unwrap();
    assert_ne!(m2.root, _m.root, "the extras are in the root");
}

// ---------------------------------------------------------------------
// DB-4b: zstd blobs, the committed source and the cache (spec 2.3, 2.4, 11)
// ---------------------------------------------------------------------
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
        assert_eq!(w.path, layout.cache_path(&w.logical));
        assert_eq!(w.blob_path, layout.blob_path(w.section.name(), &w.logical));
        assert!(w.blob_path.is_file() && w.path.is_file());
        assert_eq!(sha256_hex_of_file(&w.blob_path).unwrap(), w.blob);
        assert_eq!(std::fs::metadata(&w.blob_path).unwrap().len(), w.bytes);
        assert!(w.bytes < w.uncompressed_bytes, "{}: zstd shrinks a sqlite file", w.section.name());
        assert!(!w.reused_blob);
        let ms = m.sections.iter().find(|s| s.name == w.section.name()).unwrap();
        assert_eq!((ms.blob.as_str(), ms.bytes), (w.blob.as_str(), w.bytes));
    }
    // a cold cache: delete it, resolve through the source, get a byte-identical file back
    let core_cache = layout.cache_path(&written[0].logical);
    let before = std::fs::read(&core_cache).unwrap();
    std::fs::remove_file(&core_cache).unwrap();
    let src = CommittedZstdSource { layout: layout.clone() };
    let resolved = src.resolve(&m.sections[0]).unwrap();
    assert_eq!(resolved, core_cache);
    assert_eq!(std::fs::read(&resolved).unwrap(), before, "unpacking the blob reproduces the written file byte for byte");
    // a tampered blob is refused, both hashes named, and nothing lands in the cache
    std::fs::remove_file(&core_cache).unwrap();
    let mut bytes = std::fs::read(&written[0].blob_path).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0x55;
    std::fs::write(&written[0].blob_path, &bytes).unwrap();
    let err = src.resolve(&m.sections[0]).unwrap_err().to_string();
    assert!(err.contains(&m.sections[0].blob) && err.contains("transport hash"), "{err}");
    assert!(!core_cache.exists() && !layout.cache_dir.join(format!("{}.sqlite.tmp", written[0].logical)).exists());
    // the snapshot refuses it too -- core is required
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
    // meta carries no timestamp: the section file itself is a pure function of the content
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
    assert_eq!(layout.cache_path("abc"), std::path::Path::new("data").join("cache").join("sections").join("abc.sqlite"));
}

// ---------------------------------------------------------------------
// DB-4c: one connection per worker, mmap, the user_version wall (spec 2.5, 11)
// ---------------------------------------------------------------------
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
    let expected: u64 = snap.manifest().sections.iter().map(|s| std::fs::metadata(layout.cache_path(&s.logical)).unwrap().len()).sum();
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
    // tamper the cached concord file's user_version (the cache is trusted by name, so the wall must catch it)
    let concord = layout.cache_path(&m.sections[2].logical);
    let conn = rusqlite::Connection::open(&concord).unwrap();
    conn.execute_batch("PRAGMA user_version = 99;").unwrap();
    drop(conn);
    let err = open_written(&dir).unwrap_err().to_string();
    assert!(err.contains("section concord user_version 99 unsupported") && err.contains("understands 14"), "{err}");
}
