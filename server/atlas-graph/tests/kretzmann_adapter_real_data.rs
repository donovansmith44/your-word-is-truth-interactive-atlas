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
