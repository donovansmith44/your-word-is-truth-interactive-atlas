//! Graph construction: pure `&str`-in / `Graph`-out builders. Only the caller touches the
//! filesystem.

use std::collections::HashMap;

use anyhow::Context;

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;

use crate::event_world::{ChronologyDerivation, EventWorldStats};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BuildStats {
    pub kjv_verses: usize,
    pub cites_rows: usize,
    pub cites_dropped_negative_votes: usize,
    /// The lexicon adapter's own counts, all zero without the corpus.
    pub lexicon: crate::lexicon_adapter::LexiconAdapterStats,
}

pub fn build_graph_from_sources(kjv_json: &str, xrefs_tsv: &str, atlas: &AtlasData) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras(kjv_json, xrefs_tsv, atlas, &[])
}

pub fn build_graph_from_sources_with_eras(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel(kjv_json, xrefs_tsv, atlas, eras, None)
}

pub fn build_graph_from_sources_with_eras_and_brainfuel(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel_and_concord(kjv_json, xrefs_tsv, atlas, eras, brainfuel, None)
}

#[allow(clippy::too_many_arguments)]
pub fn build_graph_from_sources_with_eras_and_brainfuel_and_concord(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, None)
}

#[allow(clippy::too_many_arguments)]
pub fn build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
    kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, None)
}

/// A caller's `red_letter` corpus must be aligned against the RESTORED verse text, never the raw
/// `kjv_json` parse: the span-alignment law runs against the graph's own restored casing.
#[allow(clippy::too_many_arguments)]
pub fn build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
    kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
    red_letter: Option<&atlas_etl::red_letter::RedLetterCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(kjv_json, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, red_letter, None)
}

pub fn build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(
    kjv_json: &str,
    xrefs_tsv: &str,
    atlas: &AtlasData,
    eras: &[atlas_core::data::Era],
    brainfuel: Option<&atlas_etl::brainfuel::BrainFuelCorpus>,
    concord: Option<&crate::concord_adapter::ConcordBundle>,
    kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
    red_letter: Option<&atlas_etl::red_letter::RedLetterCorpus>,
    lexicon: Option<&atlas_etl::lexicon::LexiconCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    let (canon, verses) = atlas_etl::kjv::parse(kjv_json).context("parsing the KJV source (kjv.json)")?;
    // The Tetragrammaton LORD/Lord case distinction our `kjv.json` lost is restored here, the one
    // point where the parsed verses and a real brain-fuel corpus are both in scope. Without a
    // corpus it is a byte-for-byte no-op, and the fidelity law re-derives the identical transform.
    let restored_verses;
    let verses: &HashMap<String, String> = match brainfuel {
        Some(corpus) => {
            restored_verses = atlas_etl::brainfuel::restore_kjv_case(corpus, &verses).0;
            &restored_verses
        }
        None => &verses,
    };
    run_pipeline_build_with_brainfuel(&canon, verses, Some(kjv_json), xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, red_letter, lexicon)
}

/// Starts from an already-parsed `(Canon, verses)` pair, so a caller that has both builds a graph
/// consistent with them by construction. No raw source bytes exist on this path, so the LAW-CHECK
/// stage skips the KJV fidelity law here.
pub fn build_graph_from_canon_and_verses(
    canon: &Canon,
    verses: &HashMap<String, String>,
    xrefs_tsv: &str,
    atlas: &AtlasData,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    build_graph_from_canon_and_verses_with_eras(canon, verses, xrefs_tsv, atlas, &[])
}

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
    run_pipeline_build_with_brainfuel(canon, verses, kjv_json_source, xrefs_tsv, atlas, eras, None, None, None, None, None)
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
    kretzmann: Option<&atlas_etl::kretzmann::KretzmannCorpus>,
    red_letter: Option<&atlas_etl::red_letter::RedLetterCorpus>,
    lexicon: Option<&atlas_etl::lexicon::LexiconCorpus>,
) -> anyhow::Result<(Graph, BuildStats, EventWorldStats, ChronologyDerivation)> {
    let mut ctx = crate::pipeline::BuildCtx::with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(canon, verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, red_letter, lexicon);
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
        assert_eq!(graph.nodes.len(), 11, "4 verses + 3 books + 3 chapters + the corpus root");
        let spine = graph.reading.get(kjv_adapter::BIBLE_CORPUS).expect("bible reading spine must exist");
        assert_eq!(spine.order.len(), 4);
        let decoded: Vec<_> = spine.order.iter().map(|id| kjv_adapter::decode_text_unit(id).unwrap()).collect();
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
