//! Batch KRETZ-1 fix round 1 (review finding 6, LOW): "pin the comments-on
//! total with a real-data graph-level count test." Builds the REAL graph
//! (real `kjv.json` + real vendored brain-fuel/Concord/Kretzmann data) and
//! pins `graph.comments_on.len()` -- the GRAPH-LEVEL row count
//! `kretzmann_adapter::normalize` actually produces over real data, as
//! opposed to `kretzmann_real_data.rs`'s own PARSER-LEVEL `corpus.stats.
//! units` pin (a different crate, a different stage of the pipeline; this
//! test is the "did the adapter actually lower every parsed unit into a
//! real graph row, one for one" proof `kretzmann_adapter.rs`'s own unit
//! tests -- synthetic fixtures only -- cannot give).

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
        // Fix round 1: `kretzmann::read_all` now also takes the dot-ref KJV
        // verse map (the OVER-EXCISION GUARD's own real canonical source) --
        // UN-restored text parsed straight from the SAME `kjv_json` string
        // already in scope above.
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

/// PINNED (fix round 1): moves only with a new commit that deliberately
/// changes the parser or the vendored source, with a one-line reason --
/// the SAME "regression harness, moves only with a disclosed reason"
/// discipline `version_root_regression.rs`'s own header comment
/// establishes. Post-fix-round-1 value (was 50,439 pre-fix, per the
/// original batch report -- moved to 50,602 for the SAME reason
/// `kretzmann_real_data.rs`'s own module doc comment gives for its own
/// `corpus.stats.units` pin: the OVER-EXCISION GUARD recovering real
/// Kretzmann prose that used to be silently excised as if it were KJV
/// lemma text changes which raw units survive the "drop empty units"
/// filter -- one `CommentsOn` row per surviving `CommentaryItem`, one for
/// one, `kretzmann_adapter::normalize`'s own construction; no unit's own
/// range was ever observed inverted in the real corpus, so this equals
/// `corpus.stats.units` exactly, not merely approximately). Re-verified,
/// unchanged, in fix round 2 (the inline-verse-marker fix): that fix adds
/// FRAGMENTS (one per swallowed verse, now correctly its own), never new
/// UNITS -- the newly-split-out verse joins the SAME Type-B pericope
/// unit's own existing range, so `comments_on`/`CommentaryItem` counts
/// stay exactly 50,602, empirically confirmed, not assumed from "units
/// didn't move" alone.
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

/// KRETZ-SCALE-1 (batch-corp1-review.md Q-1, batch-corp1-report.md §5,
/// batch-finalp1-brief.md ticket 2): the chapter-scoped listing that
/// replaces `Kretzmann.razor`'s own retired 176-request-per-chapter
/// client-side fan-out, over a REAL chapter with a REAL count -- PSA 119,
/// the exact pileup the brief itself names. Cross-checked directly against
/// `graph.comments_on` (never trusting `chapter_commentary`'s own output
/// alone) so this proves the new server-side path serves the SAME rows the
/// pre-existing per-verse edge query already proved correct, not merely
/// "returns something."
#[test]
fn chapter_commentary_serves_psalm_119_with_real_counts_matching_a_direct_comments_on_cross_check() {
    let graph = real_graph();

    let atlas_core::refs::ScriptureRef::Chapter { book, chapter } = atlas_core::refs::ScriptureRef::parse("PSA.119").expect("PSA.119 must parse as a chapter ref") else {
        panic!("PSA.119 must parse as ScriptureRef::Chapter");
    };
    assert_eq!(chapter, 119);
    let verse_count = 176u16; // Psalm 119's own real, well-known verse count -- the KJV's longest chapter, and this ticket's own named example.

    let rows = atlas_graph::kretzmann_adapter::chapter_commentary(graph, book.0, chapter, verse_count);
    assert!(!rows.is_empty(), "Psalm 119 must have at least one real Kretzmann commentary row in the committed data");

    // Every returned row: verse in range, item_id a REAL, resolvable
    // CommentaryItem node (never a dangling/synthetic id).
    for row in &rows {
        assert!((1..=verse_count).contains(&row.verse), "row verse {} must be within Psalm 119's own 1..={verse_count} range", row.verse);
        let node = atlas_graph_types::store::GraphQuery::node(graph, &row.item_id).unwrap_or_else(|| panic!("{:?} must resolve to a real node", row.item_id));
        assert_eq!(node.id.kind, atlas_graph_types::id::NodeKind::CommentaryItem);
    }

    // Verse ascending, and within a verse, ordinal ascending -- the exact
    // ordering the retired client-side grouping already relied on.
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

    // Independent cross-check: every `graph.comments_on` row whose own
    // locus range touches ANY verse of PSA 119 must appear in `rows` at
    // every verse it covers -- and nothing else. Built directly from the
    // graph's own comments_on Vec (the SAME data source `chapter_commentary`
    // itself queries through the edge index, walked here a completely
    // different way -- structural field access, not edges() -- so this is
    // a real independent proof, not a tautology).
    let mut expected: std::collections::BTreeSet<(u16, String)> = std::collections::BTreeSet::new();
    for row in &graph.comments_on {
        if row.on.from.unit.book != book.0 || row.on.to.unit.book != book.0 {
            continue;
        }
        if row.on.from.unit.chapter != chapter {
            continue; // Psalm 119's own units never straddle a chapter boundary in the real corpus
        }
        for v in row.on.from.unit.verse..=row.on.to.unit.verse {
            if (1..=verse_count).contains(&v) {
                expected.insert((v, row.item.0.clone()));
            }
        }
    }
    let actual: std::collections::BTreeSet<(u16, String)> = rows.iter().map(|r| (r.verse, r.item_id.raw.clone())).collect();
    assert_eq!(actual, expected, "chapter_commentary's own rows must exactly match a direct graph.comments_on cross-check for PSA 119");

    // KRETZ-m2 (batch-finalp2-brief.md ticket 5): this cross-check's own
    // full-range expansion (`for v in row.on.from.unit.verse..=row.on.to.
    // unit.verse` above) happens to equal a single-verse set for EVERY real
    // `comments_on` row touching PSA 119 -- verified empirically here as a
    // precondition, not assumed -- so this test does NOT exercise (and
    // never has exercised) the multi-verse-spanning-unit case at all. See
    // `chapter_commentary_shows_a_multi_verse_spanning_unit_only_at_its_own_
    // first_verse_kretz_m2` below (a DIFFERENT, real chapter, found
    // generically) for the test that actually does, and for the documented
    // reason `chapter_commentary` cannot do full-range attribution today.
    assert!(
        graph
            .comments_on
            .iter()
            .filter(|row| row.on.from.unit.book == book.0 && row.on.to.unit.book == book.0 && row.on.from.unit.chapter == chapter)
            .all(|row| row.on.from.unit.verse == row.on.to.unit.verse),
        "precondition for the cross-check above: PSA 119 must carry no multi-verse-spanning comments_on row in the real corpus (if this ever fails, the cross-check above was silently passing vacuously on the multi-verse case, and the KRETZ-m2 test below is not, in fact, the only real coverage of it)"
    );
}

/// KRETZ-m2 (parked since KRETZ-1's own batch, flagged for "before popover
/// work"; batch-finalp2-brief.md ticket 5): `comments_on`'s own lowering
/// into the generic directed edge index (`graph-types/src/graph.rs`'s own
/// `build_indexes`, a standing-veto crate this batch may not touch)
/// anchors EVERY row -- including `ChapterIntro`/`PericopeIntro` units,
/// whose real range routinely spans dozens of verses (`atlas-etl`'s own
/// `kretzmann.rs`, the pericope/chapter-intro range-backfill pass:
/// `ChapterIntro` = `(1, chapter_verse_count)`) -- at its range's OWN FIRST
/// VERSE ONLY (`graph.rs`'s own doc comment: "the range's own FIRST verse
/// is the edge endpoint... full multi-verse popover surfacing is
/// deferred"). `chapter_commentary` (KRETZ-SCALE-1) queries
/// `commented-on-by` edges at EACH verse's own position in turn, so it
/// inherits this exactly: a chapter-intro/pericope-intro unit's own
/// commentary appears ONLY under its first verse's row, never under any
/// LATER verse its own range covers.
///
/// NOT fixable within this batch: `GraphQuery` (the owner-approved port --
/// `node`/`derive`/`edge_summary`/`edges`/`reading_window`, `service.rs`'s
/// own "never a direct `Graph` field reach" law) offers no way to recover a
/// `CommentsOn` row's own full range from an edge query alone (only its
/// indexed endpoint is queryable) -- fixing the ROOT would mean either
/// re-indexing in `graph-types` (the standing veto) or reaching around the
/// port to a concrete `Graph` (forbidden by `service.rs`'s own documented
/// law, which exists precisely so a second `GraphStore` implementation
/// stays possible). DOCUMENTED here exactly like RED-m2's own SpokenAt
/// disposition (the same brief's ticket 2, "DOCUMENT-ONLY... a test pinning
/// current coverage + a comment -- no artifact moves"): this test pins
/// CURRENT, real, live behavior over the real corpus, discovered
/// generically (not a hardcoded book/chapter), so any future fix (or
/// regression) to this gap shows up here first.
///
/// Disclosed judgment call: for the CHAPTER-LISTING use case `chapter_
/// commentary` actually serves today, first-verse-only attribution is
/// arguably the RIGHT product behavior (nobody wants a 176-verse chapter's
/// intro repeated under all 176 verses) -- the real risk this test flags is
/// a FUTURE "verse -> applicable commentary" popover lookup silently
/// missing coverage for any verse past a multi-verse unit's own first.
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

    // `verse_count = last_verse` is sufficient (not the chapter's true real
    // length): `chapter_commentary` only needs to walk far enough to cover
    // every verse this ONE row's own range touches -- this test does not
    // need a real `AtlasData` chapter-length lookup to prove its point.
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
