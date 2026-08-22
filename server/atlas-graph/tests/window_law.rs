//! WINDOW LAW (design doc §6): "for any partition of the walk into windows,
//! concatenation is invariant" -- property test over sampled partitions of
//! the REAL KJV reading order (built once, reused across cases, via
//! `OnceLock`), proving lazy loading can never change what the Bible says.
//! Runs `GraphState::window` (the SAME primitive both `/api/text` and the
//! swapped `/api/chapter` view call), not a reimplementation of it.

use std::sync::OnceLock;

use atlas_graph::{GraphState, WindowDir};
use proptest::prelude::*;

fn real_graph() -> &'static GraphState {
    static GRAPH: OnceLock<GraphState> = OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        GraphState::build(&dir)
            .expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist (committed real data)")
    })
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
        let g = real_graph();
        let total_len = g.bible_len();
        let start = ((total_len as f64) * start_fraction) as usize;
        let whole_n: usize = sizes.iter().sum();
        prop_assume!(start + whole_n <= total_len);

        let whole = g.window(start, whole_n, WindowDir::Onward);

        let mut partitioned = Vec::with_capacity(whole.len());
        let mut cursor = start;
        for size in &sizes {
            let piece = g.window(cursor, *size, WindowDir::Onward);
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
        let g = real_graph();
        let total_len = g.bible_len();
        let end = ((total_len as f64) * start_fraction) as usize;
        prop_assume!(end >= n.saturating_sub(1));

        let backward = g.window(end, n, WindowDir::Backward);
        let onward_start = end.saturating_sub(n.saturating_sub(1));
        let onward = g.window(onward_start, n, WindowDir::Onward);

        prop_assert_eq!(backward, onward, "a backward window is exactly the onward window starting where it starts");
    }
}
