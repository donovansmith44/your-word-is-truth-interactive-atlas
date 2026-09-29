mod common;

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
        GraphService::build(&common::raw_dir(), &atlas_graph::event_world::empty_atlas())
            .expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist (committed real data)")
    })
}

fn real_raw_graph() -> &'static Graph {
    static GRAPH: OnceLock<Graph> = OnceLock::new();
    GRAPH.get_or_init(|| {
        atlas_graph::build::build_graph_from_sources(&common::kjv_json(), &common::cross_references_tsv(), &atlas_graph::event_world::empty_atlas())
            .expect("the real KJV source must parse")
            .0
    })
}

fn bible_len() -> usize {
    static LEN: OnceLock<usize> = OnceLock::new();
    *LEN.get_or_init(|| real_graph().snapshot().reading_window(BIBLE, 0, usize::MAX).len())
}

fn partition_strategy() -> impl Strategy<Value = Vec<usize>> {
    proptest::collection::vec(1usize..=50, 1..=12)
}

proptest! {
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
