//! Graph construction: pure `&str`-in/`Graph`-out builders (mirrors
//! atlas-etl's own "every module is `&str`-in / data-out; only the caller
//! touches the filesystem" discipline -- `state::GraphState::build` is the
//! one place in this crate that reads files).
//!
//! Controller ruling 2 (M-A pragmatic materialization): builds the whole
//! graph IN MEMORY from the raw sources, once, at call time (server
//! startup calls this once via `GraphState::build`) -- 31,102 KJV verses
//! plus ~344k raw cross-reference rows is well within "trivial startup
//! work." The serialized one-artifact form (design doc P1, fully realized)
//! lands at M-C when the parallel `data/compiled/*.json` artifacts retire.

use std::collections::HashMap;

use anyhow::Context;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;

use crate::event_world::{ChronologyDerivation, EventWorldStats};

/// Startup-log-friendly counts -- also asserted against in tests (the
/// FIDELITY LAW's own bijection count and the xref adapter's disclosed
/// negative-vote drop are both externally observable here, not just log
/// noise).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct BuildStats {
    pub kjv_verses: usize,
    pub cites_rows: usize,
    pub cites_dropped_negative_votes: usize,
}

/// Builds the graph from the two raw KJV/xrefs sources PLUS the event world
/// (Batch M-B: events/attestations/narratives/anchors/chronology, from the
/// already-built `atlas` -- see `event_world`'s own module doc comment for
/// why `AtlasData` is this adapter's source, not a second independent TOML
/// parse).
///
/// BATCH M-C (controller decision 3): drives the six-stage compiler
/// pipeline contract (`pipeline::pipeline`/`pipeline::run_pipeline`) rather
/// than calling each adapter step directly in a hardcoded sequence -- see
/// `pipeline.rs`'s own module doc comment for the full stage mapping and
/// the version-root regression proof that this restructuring is
/// behavior-identical to the pre-M-C call chain it replaces.
pub fn build_graph_from_sources(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras(kjv_json, xrefs_tsv, atlas, &[])
}

/// M-C: the richer form real startup (and the artifact compile step) use --
/// `eras` is `era_adapter.rs`'s own pre-parsed source (see
/// `pipeline::BuildCtx::eras`'s own doc comment for why it isn't read off
/// `AtlasData`). Every OTHER caller (most test fixtures) keeps calling the
/// plain `build_graph_from_sources` above, unaffected, getting an honestly
/// empty era set.
pub fn build_graph_from_sources_with_eras(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, None)
}

/// CORP-1a: the richest raw-source form -- `bins/compile_graph.rs` (the
/// real compile step) and `GraphService::build`'s own `--build-from-raw`
/// dev fallback both use this directly, with a real, pre-parsed
/// `atlas_etl::brainfuel::BrainFuelCorpus` (`atlas_etl::brainfuel::
/// read_all`'s own return value -- the ONE filesystem-touching call, made
/// by each of those two real callers, never by this pure function).
/// `build_graph_from_sources_with_eras` above delegates here with `None`,
/// so every existing caller's own behavior is byte-identical to before
/// this batch.
pub fn build_graph_from_sources_with_eras_and_brainfuel(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel_and_concord(kjv_json, xrefs_tsv, atlas, eras, brainfuel, None)
}

/// CORP-2a: the richest raw-source form yet -- `bins/compile_graph.rs`
/// and `GraphService::build`'s own dev fallback both use this directly,
/// with a real, pre-parsed `concord_adapter::ConcordBundle` (`concord_
/// adapter::ConcordBundle`'s own doc comment). `build_graph_from_sources_
/// with_eras_and_brainfuel` above delegates here with `None`, so every
/// existing caller's own behavior is byte-identical to before this batch.
#[allow(clippy::too_many_arguments)]
pub fn build_graph_from_sources_with_eras_and_brainfuel_and_concord(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    let (canon, verses) = atlas_etl::kjv::parse(kjv_json).context("parsing the KJV source (kjv.json)")?;
    // Batch KJV-CASE (owner ruling; batch-kjv-case-brief.md): restore the
    // Tetragrammaton LORD/Lord case distinction our canonical kjv.json
    // lost -- this is the ONE spot both `verses` (just parsed above) and a
    // real `brainfuel` corpus are jointly in scope on every real-source
    // caller (this function's own richest form; `compile_graph.rs`/
    // `GraphService::build` both funnel through here). See
    // `atlas_etl::brainfuel::restore_kjv_case`'s own doc comment for the
    // case-only law it enforces. `fidelity::check_kjv_fidelity`'s own
    // independent re-derivation applies the IDENTICAL transform to its own
    // independently-parsed "expected" text (see that module's own doc
    // comment) -- so the KJV boundary law still proves what it always
    // proved: this build matches ITS OWN declared source, source now
    // honestly meaning "kjv.json, case-restored" rather than "kjv.json,
    // verbatim." `None` (no real brainfuel, e.g. most test fixtures) is a
    // true no-op: `verses` passes through unchanged, byte-for-byte.
    let restored_verses;
    let verses: &HashMap<String, String> = match brainfuel {
        Some(corpus) => {
            restored_verses = atlas_etl::brainfuel::restore_kjv_case(corpus, &verses).0;
            &restored_verses
        }
        None => &verses,
    };
    run_pipeline_build_with_brainfuel(&canon, verses, Some(kjv_json), xrefs_tsv, atlas, eras, brainfuel, concord)
}

/// The same build, starting from an already-parsed `(Canon, verses)` pair
/// instead of raw KJV JSON text -- lets a caller that already has both (an
/// `AtlasData` fixture, most notably) build a graph that is BY CONSTRUCTION
/// consistent with that same canon/verses, rather than maintaining a second,
/// independently-authored raw-JSON fixture that can silently drift from it.
/// No raw source bytes exist on this path, so the pipeline's own LAW-CHECK
/// stage skips the KJV fidelity law here, exactly as before this batch's
/// pipeline restructuring (see `pipeline::BuildCtx::kjv_json_source`'s own
/// doc comment).
pub fn build_graph_from_canon_and_verses(
    canon: &Canon,
    verses: &HashMap<String, String>,
    xrefs_tsv: &str,
    atlas: &AtlasData,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, &[])
}

/// M-C: the eras-carrying form of `build_graph_from_canon_and_verses` --
/// same "test fixture that already has a Canon" rationale, extended to a
/// fixture that also wants real Era nodes (e.g. `AtlasData::demo_fixture`'s
/// own `.eras`, so `/api/eras` has something real to serve in tests that
/// exercise it).
pub fn build_graph_from_canon_and_verses_with_eras(
    canon: &Canon,
    verses: &HashMap<String, String>,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    run_pipeline_build(canon, verses, None, xrefs_tsv, atlas, eras)
}

fn run_pipeline_build(
    canon: &Canon,
    verses: &HashMap<String, String>,
    kjv_json_source: Option<&str>,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    run_pipeline_build_with_brainfuel(canon, verses, kjv_json_source, xrefs_tsv, atlas, eras, None, None)
}

#[allow(clippy::too_many_arguments)]
fn run_pipeline_build_with_brainfuel(
    canon: &Canon,
    verses: &HashMap<String, String>,
    kjv_json_source: Option<&str>,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    let mut ctx = crate::pipeline::BuildCtx::with_eras_and_brainfuel_and_concord(canon, verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, concord);
    crate::pipeline::run_pipeline(&mut ctx, &crate::pipeline::pipeline())?;
    Ok((ctx.graph, ctx.stats, ctx.event_world_stats, ctx.chrono))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kjv_adapter;
    use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
    use atlas_graph_types::explore::{Explorable, PositionRef};
    use atlas_graph_types::id::Position;

    const KJV_FIXTURE: &str = r#"{
      "translation": "KJV",
      "books": [
        { "name": "Genesis", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "In the beginning God created the heaven and the earth." },
          { "verse": 2, "text": "And the earth was without form, and void." }
        ] } ] },
        { "name": "I Samuel", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "Now there was a certain man of Ramathaimzophim." }
        ] } ] },
        { "name": "Revelation of John", "chapters": [ { "chapter": 1, "verses": [
          { "verse": 1, "text": "The Revelation of Jesus Christ." }
        ] } ] }
      ]
    }"#;

    const XREFS_FIXTURE: &str = "From Verse\tTo Verse\tVotes\t#comment\nGen.1.1\t1Sam.1.1\t9\n";

    #[test]
    fn builds_one_text_unit_per_verse_in_canon_order() {
        let (graph, stats, ..) = build_graph_from_sources(KJV_FIXTURE, XREFS_FIXTURE, &crate::event_world::empty_atlas()).unwrap();
        assert_eq!(stats.kjv_verses, 4);
        assert_eq!(graph.nodes.len(), 4);
        let spine = graph.reading.get(kjv_adapter::BIBLE_CORPUS).expect("bible reading spine must exist");
        assert_eq!(spine.order.len(), 4);
        let decoded: Vec<_> = spine.order.iter().map(|id| kjv_adapter::decode_text_unit(id).unwrap()).collect();
        // Genesis (book 0) before 1 Samuel (book 8) before Revelation (book 65).
        assert_eq!(decoded, vec![(0, 1, 1), (0, 1, 2), (8, 1, 1), (65, 1, 1)]);
    }

    #[test]
    fn cites_row_is_queryable_through_the_generic_explorable_machinery() {
        let (graph, stats, ..) = build_graph_from_sources(KJV_FIXTURE, XREFS_FIXTURE, &crate::event_world::empty_atlas()).unwrap();
        assert_eq!(stats.cites_rows, 1);

        let gen11 = kjv_adapter::verse_node_id(0, 1, 1);
        let cites = EdgeKind::Directed(RelationId::Cites, Direction::Forward);
        let page = PositionRef(Position::Node(gen11))
            .edges(&graph, &atlas_graph_types::explore::EdgeQuery { kind: cites, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1);
        let target = kjv_adapter::verse_node_id(8, 1, 1);
        assert_eq!(page.entries[0].node, Position::Node(target));
    }
}
