//! Batch FQ-0 (frontier.rs contract revision), design review B1's
//! recommended falsifiability test: "every relation named by any
//! `Capability::edges()` cell has ≥1 row in the compiled artifact." The
//! bridge in `graph-types/src/frontier.rs` was compiled to be honest, not
//! merely to compile -- this is the test that makes a decorative cell
//! (the original `Parallel` cell: zero producers, zero consumers anywhere
//! in the repo) impossible to ship silently again.
//!
//! Lives here, not in `graph-types`, because `graph-types` is pure
//! types+laws and cannot load a compiled artifact (owner ruling,
//! `progress.md` 2026-09-07 ruling (f)); this crate can, via the same
//! `real_graph()` pattern `kretzmann_adapter_real_data.rs` already
//! established.
//!
//! Batch NODE-1 UN-GATED the one `#[ignore]` this file shipped with:
//! `Members` (`Contains`-forward, Chapter/Book) now has real rows --
//! `bible_container_adapter` mints the 66 book + 1,189 chapter `Container`
//! nodes and the chapter->verses Bible-corpus `Contains` rows, and
//! `artifact.rs` serializes `contains_bible` (FORMAT_VERSION 11) -- so
//! `Members` is folded back into the live sweep below, exactly per the
//! un-gate instructions the ignored test's own doc comment carried.
//! Every other capability's relation was checked at FQ-0 and has real
//! rows in the committed data (see the sweep below); `Succession` in
//! particular was verified non-decorative here (owner ruling (f): "check
//! Succession's row count and gate honestly if zero") -- it is NOT zero
//! (curated-narrative chains, `event_world::populate_nodes_and_direct_
//! rows`), and NODE-1's own chapter/book prev/next chains are DERIVED
//! index entries over the same relation (see `bible_container_adapter::
//! add_derived_membership_and_succession`'s own doc comment).

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

/// Same construction `kretzmann_adapter_real_data.rs` already uses
/// (deliberately duplicated rather than shared -- each `_real_data.rs`
/// file in this crate builds its own real graph; see that file's own
/// header for the precedent).
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

/// Maps an `EdgeKind` a `Capability::edges()` cell might name to the real
/// row table backing it. A `panic!` on an unmapped relation is
/// deliberate: it means a new `Capability::edges()` cell named a relation
/// this sweep does not yet know how to count, which is exactly the
/// "silently decorative" failure mode B1 exists to catch -- fail loud,
/// don't skip.
fn relation_row_count(graph: &atlas_graph_types::graph::Graph, kind: atlas_graph_types::edge::EdgeKind) -> usize {
    use atlas_graph_types::edge::{EdgeKind, RelationId as R, SymRelationId as S};
    match kind {
        EdgeKind::Directed(R::Cites, _) => graph.cross_refs.len(),
        EdgeKind::Directed(R::Attests, _) => graph.attests.len(),
        EdgeKind::Directed(R::Mentions, _) => graph.mentions.len(),
        EdgeKind::Directed(R::Succession, _) => graph.succession.len(),
        EdgeKind::Directed(R::DatedBy, _) => graph.dated_by.len(),
        EdgeKind::Directed(R::LocatedAt, _) => graph.located_at.len(),
        EdgeKind::Directed(R::CommentsOn, _) => graph.comments_on.len(),
        EdgeKind::Directed(R::Contains, _) => graph.contains_bible.len(),
        EdgeKind::Symmetric(S::CatechismLink) => graph.catechism.len(),
        EdgeKind::Symmetric(S::TemporalAdjacency) => graph.temporal_adjacency.len(),
        other => panic!(
            "frontier_falsifiability.rs's relation_row_count has no mapping for {other:?} -- \
             a new Capability::edges() cell names a relation this sweep doesn't know how to \
             count; add an arm here before trusting the falsifiability sweep for it"
        ),
    }
}

/// THE FALSIFIABILITY TEST (design review B1, owner ruling (f)): every
/// relation named by any `Capability::edges()` cell -- `Members`
/// included, since NODE-1 -- must have at least one row in the real
/// compiled graph.
#[test]
fn every_capability_has_real_rows_in_the_compiled_artifact() {
    use atlas_graph_types::frontier::Capability;

    let graph = real_graph();
    for cap in Capability::ALL {
        for edge in cap.edges() {
            let n = relation_row_count(graph, edge.kind);
            assert!(
                n > 0,
                "capability {cap:?}'s edge {:?} has ZERO rows in the real compiled graph -- \
                 the unity bridge is decorative here (design review B1's exact defect: the \
                 original `Parallel` cell named a relation with zero producers and zero \
                 consumers anywhere in the repo)",
                edge.kind
            );
        }
    }
}

/// NODE-1 (un-gated per this test's own former TODO(NODE-1) doc comment,
/// steps 1-3 executed): `Capability::Members` (`Contains`-forward,
/// Chapter/Book) now has real rows -- `bible_container_adapter::normalize`
/// emits one chapter->verses `Contains<BibleTag>` row per chapter (1,189
/// over the real canon), and `artifact.rs` serializes `contains_bible`
/// instead of guarding it empty (FORMAT_VERSION 11).
#[test]
fn members_capability_has_real_bible_contains_rows_since_node_1() {
    let graph = real_graph();
    assert!(
        graph.contains_bible.len() > 0,
        "NODE-1's own Bible-corpus Contains rows are missing from the compiled graph -- \
         Capability::Members would be as decorative as the original Parallel cell; re-gate it \
         honestly if this ever regresses to zero"
    );
}

/// `Succession` specifically, pinned per owner ruling (f) ("check
/// Succession's row count and gate honestly if zero"): it is NOT zero --
/// curated-narrative chains with real legs produce real rows
/// (`event_world::populate_nodes_and_direct_rows`), independent of
/// NODE-1. The CHAPTER/BOOK-specific use of this same relation (prev/next
/// chapter navigation) landed with NODE-1 as DERIVED index entries, not
/// rows (`Succession`'s own row type chains `Vec<EventId>` -- see
/// `bible_container_adapter::add_derived_membership_and_succession`'s doc
/// comment), so THIS row-level pin still counts only the narrative
/// chains; the chapter/book chains are covered by
/// `tests/bible_containers_real_data.rs`'s own edge-level assertions.
#[test]
fn succession_has_real_rows_today_independent_of_node_1() {
    let graph = real_graph();
    assert!(
        graph.succession.len() > 0,
        "Succession has zero rows in the real compiled graph -- Capability::Chronology would \
         be as decorative as the original Parallel cell; gate Chronology honestly if this ever \
         regresses to zero"
    );
}
