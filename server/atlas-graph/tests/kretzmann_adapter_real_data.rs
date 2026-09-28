use std::path::Path;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
        let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
        let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
        let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
        let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };
        let (_, kjv_verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
        let kretzmann_corpus = atlas_etl::kretzmann::read_all(&dir.join("kretzmann"), &kjv_verses).expect("data/raw/kretzmann must exist -- run data/fetch-raw.ps1 first");

        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
            &kjv_json,
            &xrefs_tsv,
            &atlas,
            &atlas.eras,
            Some(&brainfuel),
            Some(&concord_bundle),
            Some(&kretzmann_corpus),
        )
        .expect("the real committed sources must build");
        graph.build_indexes();
        graph
    })
}

#[test]
fn kretzmann_comments_on_rows_have_the_pinned_real_count() {
    let graph = real_graph();
    let comments_on_count = graph.comments_on.len();
    assert_eq!(comments_on_count, 50602);

    let commentary_item_count = graph.nodes.keys().filter(|id| id.kind == atlas_graph_types::id::NodeKind::CommentaryItem).count();
    assert_eq!(commentary_item_count, comments_on_count, "one CommentsOn row per CommentaryItem node, one for one -- no unit's own range was ever inverted in the real corpus");

    let source_count = graph.nodes.values().filter(|n| matches!(n.payload, atlas_graph_types::node::NodePayload::Source { .. }) && n.provenance == "kretzmann").count();
    assert_eq!(source_count, 1, "one Source node for the whole work, decision 4's own law");
}

#[test]
fn chapter_commentary_serves_psalm_119_with_real_counts_matching_a_direct_comments_on_cross_check() {
    let graph = real_graph();

    let atlas_core::refs::ScriptureRef::Chapter { book, chapter } = atlas_core::refs::ScriptureRef::parse("PSA.119").expect("PSA.119 must parse as a chapter ref") else {
        panic!("PSA.119 must parse as ScriptureRef::Chapter");
    };
    assert_eq!(chapter, 119);
    let verse_count = 176u16;

    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(graph, book.0, chapter, verse_count);
    assert!(!rows.is_empty(), "Psalm 119 must have at least one real Kretzmann commentary row in the committed data");

    for row in &rows {
        assert!((1..=verse_count).contains(&row.verse), "row verse {} must be within Psalm 119's own 1..={verse_count} range", row.verse);
        let node = atlas_graph_types::store::GraphQuery::node(graph, &row.item_id).unwrap_or_else(|| panic!("{:?} must resolve to a real node", row.item_id));
        assert_eq!(node.id.kind, atlas_graph_types::id::NodeKind::CommentaryItem);
    }

    let mut last_verse = 0u16;
    let mut last_ordinal_in_verse: Option<u64> = None;
    for row in &rows {
        assert!(row.verse >= last_verse, "rows must be verse-ascending");
        if row.verse != last_verse {
            last_ordinal_in_verse = None;
        }
        let ordinal: u64 = row.item_id.raw.rsplit('.').next().and_then(|s| s.parse().ok()).unwrap_or(0);
        if let Some(prev) = last_ordinal_in_verse {
            assert!(ordinal > prev, "within one verse, items must be document-order ascending");
        }
        last_ordinal_in_verse = Some(ordinal);
        last_verse = row.verse;
    }

    let mut expected: std::collections::BTreeSet<(u16, String)> = std::collections::BTreeSet::new();
    for row in &graph.comments_on {
        if row.on.from.unit.book != book.0 || row.on.to.unit.book != book.0 {
            continue;
        }
        if row.on.from.unit.chapter != chapter {
            continue;
        }
        for v in row.on.from.unit.verse..=row.on.to.unit.verse {
            if (1..=verse_count).contains(&v) {
                expected.insert((v, row.item.0.clone()));
            }
        }
    }
    let actual: std::collections::BTreeSet<(u16, String)> = rows.iter().map(|r| (r.verse, r.item_id.raw.clone())).collect();
    assert_eq!(actual, expected, "chapter_commentary's own rows must exactly match a direct graph.comments_on cross-check for PSA 119");

    assert!(
        graph
            .comments_on
            .iter()
            .filter(|row| row.on.from.unit.book == book.0 && row.on.to.unit.book == book.0 && row.on.from.unit.chapter == chapter)
            .all(|row| row.on.from.unit.verse == row.on.to.unit.verse),
        "precondition for the cross-check above: PSA 119 must carry no multi-verse-spanning comments_on row in the real corpus (if this ever fails, the cross-check above was silently passing vacuously on the multi-verse case, and the KRETZ-m2 test below is not, in fact, the only real coverage of it)"
    );
}

#[test]
fn chapter_commentary_shows_a_multi_verse_spanning_unit_only_at_its_own_first_verse_kretz_m2() {
    let graph = real_graph();

    let spanning = graph
        .comments_on
        .iter()
        .find(|r| r.on.from.unit.book == r.on.to.unit.book && r.on.from.unit.chapter == r.on.to.unit.chapter && r.on.to.unit.verse > r.on.from.unit.verse)
        .expect("the real Kretzmann corpus must contain at least one multi-verse-spanning unit (a ChapterIntro or PericopeIntro -- kretzmann.rs's own range-backfill pass)");

    let book_index = spanning.on.from.unit.book;
    let chapter = spanning.on.from.unit.chapter;
    let first_verse = spanning.on.from.unit.verse;
    let last_verse = spanning.on.to.unit.verse;
    let item_id_str = spanning.item.0.clone();
    assert!(last_verse > first_verse, "sanity: the discovered row must genuinely span more than one verse");

    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(graph, book_index, chapter, last_verse);

    assert!(
        rows.iter().any(|r| r.verse == first_verse && r.item_id.raw == item_id_str),
        "the spanning unit must appear at its own first verse ({first_verse}) -- that's the position `comments_on` is indexed at"
    );

    for v in (first_verse + 1)..=last_verse {
        assert!(
            !rows.iter().any(|r| r.verse == v && r.item_id.raw == item_id_str),
            "KRETZ-m2 (documented, not fixable this batch -- see this test's own doc comment): the spanning unit must be ABSENT from verse {v}, inside its own range ({first_verse}..={last_verse}) but past its indexed first verse. If this assertion ever fails, the port gained a way to recover full-range attribution -- update this test AND `chapter_commentary`'s own doc comment (kretzmann_adapter.rs) to reflect the fix, don't just relax the assertion."
        );
    }
}
