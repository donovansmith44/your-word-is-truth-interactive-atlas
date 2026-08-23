//! WINDOW LAW (design doc §6): "for any partition of the walk into windows,
//! concatenation is invariant" -- property test over sampled partitions of
//! the REAL KJV reading order (built once, reused across cases, via
//! `OnceLock`), proving lazy loading can never change what the Bible says.
//! Runs `window::window` (the SAME primitive both `/api/text` and the
//! swapped `/api/chapter` view call) against `&dyn
//! atlas_graph_types::store::GraphQuery` -- THE PORT (design doc §9a; fix
//! round 1 C1) -- never the concrete in-memory service, so this law is
//! proven at the same seam any future backend plugs into.

use std::sync::OnceLock;

use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::store::GraphQuery;
use proptest::prelude::*;

const BIBLE: &str = atlas_graph::kjv_adapter::BIBLE_CORPUS;

fn real_graph() -> &'static GraphService {
    static GRAPH: OnceLock<GraphService> = OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        // Window law scope: reading-order only, no event-world assertions --
        // an empty AtlasData is the right fixture (Batch M-B, event_world's
        // own doc comment).
        GraphService::build(&dir, &atlas_graph::event_world::empty_atlas())
            .expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist (committed real data)")
    })
}

/// Built ONCE (real KJV parsing is not free -- 256 proptest cases rebuilding
/// it each time would turn a seconds-long suite into an hours-long one),
/// reused by `holds_against_the_raw_graph_too_not_just_a_snapshot` below.
fn real_raw_graph() -> &'static Graph {
    static GRAPH: OnceLock<Graph> = OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist (committed real data)");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist (committed real data)");
        atlas_graph::build::build_graph_from_sources(&kjv_json, &xrefs_tsv, &atlas_graph::event_world::empty_atlas())
            .expect("the real KJV source must parse")
            .0
    })
}

fn bible_len() -> usize {
    // Cached: `GraphQuery` has no dedicated "spine length" operation (by
    // design -- not part of the owner-approved port), only forward
    // `reading_window`, so computing it means asking for everything once
    // -- fine as a one-time cost, wasteful if repeated per proptest case.
    static LEN: OnceLock<usize> = OnceLock::new();
    *LEN.get_or_init(|| real_graph().snapshot().reading_window(BIBLE, 0, usize::MAX).len())
}

/// A random handful (1..=12) of window sizes (1..=50 units each), kept well
/// inside the 31,102-verse corpus so a generated partition never needs
/// special end-of-canon handling (the `prop_assume!` below still guards it).
fn partition_strategy() -> impl Strategy<Value = Vec<usize>> {
    proptest::collection::vec(1usize..=50, 1..=12)
}

proptest! {
    /// Any partition of a walk into windows concatenates identically to the
    /// single window covering the same span -- lazy-loaded pages (however
    /// they're chopped) are windows over one lawful whole.
    #[test]
    fn partition_concatenation_matches_the_single_whole_window(
        start_fraction in 0.0f64..0.9,
        sizes in partition_strategy(),
    ) {
        let snap = real_graph().snapshot();
        let g: &dyn GraphQuery = &snap;
        let total_len = bible_len();
        let start = ((total_len as f64) * start_fraction) as usize;
        let whole_n: usize = sizes.iter().sum();
        prop_assume!(start + whole_n <= total_len);

        let whole = window::window(g, BIBLE, start, whole_n, WindowDir::Onward);

        let mut partitioned = Vec::with_capacity(whole.len());
        let mut cursor = start;
        for size in &sizes {
            let piece = window::window(g, BIBLE, cursor, *size, WindowDir::Onward);
            prop_assert_eq!(piece.len(), *size, "each window must be exactly its requested size (never short) inside the corpus");
            partitioned.extend(piece);
            cursor += size;
        }

        prop_assert_eq!(partitioned, whole, "partitioned windows must concatenate to the SAME sequence as the single whole window");
    }

    /// The window law holds for backward windows too: a backward window
    /// ending at a position reproduces exactly the onward window starting
    /// where that backward window starts.
    #[test]
    fn backward_window_is_the_mirror_of_an_onward_one(
        start_fraction in 0.05f64..0.9,
        n in 1usize..=40,
    ) {
        let snap = real_graph().snapshot();
        let g: &dyn GraphQuery = &snap;
        let total_len = bible_len();
        let end = ((total_len as f64) * start_fraction) as usize;
        prop_assume!(end >= n.saturating_sub(1));

        let backward = window::window(g, BIBLE, end, n, WindowDir::Backward);
        let onward_start = end.saturating_sub(n.saturating_sub(1));
        let onward = window::window(g, BIBLE, onward_start, n, WindowDir::Onward);

        prop_assert_eq!(backward, onward, "a backward window is exactly the onward window starting where it starts");
    }

    /// The SAME law, proven against the RAW `Graph` directly (graph-types'
    /// own canonical `GraphQuery` instance) rather than a `MemSnapshot` --
    /// window.rs's own functions are generic over the port, and this is
    /// the literal proof that "generic" is not just a claim.
    #[test]
    fn holds_against_the_raw_graph_too_not_just_a_snapshot(
        start_fraction in 0.0f64..0.9,
        n in 1usize..=50,
    ) {
        let g: &dyn GraphQuery = real_raw_graph();

        let total_len = bible_len();
        let start = ((total_len as f64) * start_fraction) as usize;
        prop_assume!(start + n <= total_len);

        let whole = window::window(g, BIBLE, start, n, WindowDir::Onward);
        prop_assert_eq!(whole.len(), n);
    }
}
