//! The compiler pipeline is an ordered list of passes that is DATA rather than a hardcoded call
//! chain, so backing a pass out is removing its entry from the list.

use anyhow::{Context, Result};

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;

use crate::build::BuildStats;
use crate::event_world::{ChronologyDerivation, EventWorldStats};

/// Everything a pass needs: the typed inputs, the in-progress `Graph` every pass mutates in place, and
/// the running stats later passes and the caller read. Passes run in the order `pipeline()` lists, and
/// each may read or write any field.
pub struct BuildCtx<'a> {
    pub kjv_canon: &'a Canon,
    pub kjv_verses: &'a std::collections::HashMap<String, String>,
    /// The RAW KJV JSON text, where one exists. Kept separately from the parsed canon on purpose: the
    /// fidelity law's independent reader must re-parse these bytes sharing no code with the adapter
    /// path that built the parsed form, and collapsing the two would de-independent that law.
    pub kjv_json_source: Option<&'a str>,
    pub xrefs_tsv: &'a str,
    pub atlas: &'a AtlasData,
    /// Pre-parsed curated era rows, not read off `AtlasData` like the other adapter sources. An empty
    /// slice is a lawful "no eras this build", never a placeholder.
    pub eras: &'a [atlas_core::data::Era],
    pub graph: Graph,
    pub stats: BuildStats,
    pub event_world_stats: EventWorldStats,
    pub chrono: ChronologyDerivation,
    pub justified_by_count: usize,
    /// Absent means an honestly empty build, not a placeholder -- as for every other corpus field
    /// here. A reference rather than owned: these corpora are large, and every real caller already has
    /// one alive for the duration of the build.
    pub brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
    pub concord: Option<&'a crate::concord_adapter::ConcordBundle>,
    pub kretzmann: Option<&'a atlas_etl::kretzmann::KretzmannCorpus>,
    pub red_letter: Option<&'a atlas_etl::red_letter::RedLetterCorpus>,
    pub lexicon: Option<&'a atlas_etl::lexicon::LexiconCorpus>,
    /// Captured rather than returned and discarded, so a caller that drives `run_pipeline` itself can
    /// read the fill-rate afterwards without a second pass over the graph. Stays `Default` until
    /// `MergeAliasPass` runs.
    pub description_stats: crate::description_adapter::DescriptionStats,
}

impl<'a> BuildCtx<'a> {
    pub fn new(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
    ) -> Self {
        Self::with_eras(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, &[])
    }

    pub fn with_eras(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
    ) -> Self {
        Self::with_eras_and_brainfuel(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, eras, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_eras_and_brainfuel(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
        brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
    ) -> Self {
        Self::with_eras_and_brainfuel_and_concord(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_eras_and_brainfuel_and_concord(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
        brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&'a crate::concord_adapter::ConcordBundle>,
    ) -> Self {
        Self::with_eras_and_brainfuel_and_concord_and_kretzmann(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, concord, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_eras_and_brainfuel_and_concord_and_kretzmann(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
        brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&'a crate::concord_adapter::ConcordBundle>,
        kretzmann: Option<&'a atlas_etl::kretzmann::KretzmannCorpus>,
    ) -> Self {
        Self::with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
        brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&'a crate::concord_adapter::ConcordBundle>,
        kretzmann: Option<&'a atlas_etl::kretzmann::KretzmannCorpus>,
        red_letter: Option<&'a atlas_etl::red_letter::RedLetterCorpus>,
    ) -> Self {
        BuildCtx {
            kjv_canon,
            kjv_verses,
            kjv_json_source,
            xrefs_tsv,
            atlas,
            eras,
            brainfuel,
            concord,
            kretzmann,
            red_letter,
            lexicon: None,
            graph: Graph::default(),
            stats: BuildStats::default(),
            event_world_stats: EventWorldStats::default(),
            chrono: ChronologyDerivation::default(),
            justified_by_count: 0,
            description_stats: crate::description_adapter::DescriptionStats::default(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter_and_lexicon(
        kjv_canon: &'a Canon,
        kjv_verses: &'a std::collections::HashMap<String, String>,
        kjv_json_source: Option<&'a str>,
        xrefs_tsv: &'a str,
        atlas: &'a AtlasData,
        eras: &'a [atlas_core::data::Era],
        brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
        concord: Option<&'a crate::concord_adapter::ConcordBundle>,
        kretzmann: Option<&'a atlas_etl::kretzmann::KretzmannCorpus>,
        red_letter: Option<&'a atlas_etl::red_letter::RedLetterCorpus>,
        lexicon: Option<&'a atlas_etl::lexicon::LexiconCorpus>,
    ) -> Self {
        let mut ctx = Self::with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(kjv_canon, kjv_verses, kjv_json_source, xrefs_tsv, atlas, eras, brainfuel, concord, kretzmann, red_letter);
        ctx.lexicon = lexicon;
        ctx
    }
}

/// A compiler pass: a named, total step over the build context. `name()` exists for error attribution
/// and for the proof that the passes really are data rather than a hardcoded chain.
pub trait Pass {
    fn name(&self) -> &'static str;
    fn run(&self, ctx: &mut BuildCtx) -> Result<()>;
}

struct NormalizePass;
impl Pass for NormalizePass {
    fn name(&self) -> &'static str {
        "normalize"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        crate::kjv_adapter::normalize(ctx).context("normalizing the KJV canon/verses into TextUnit nodes")?;
        crate::bible_container_adapter::normalize(ctx).context("normalizing the canon into book/chapter Container nodes + Contains rows")?;
        crate::brainfuel_adapter::normalize(ctx);
        crate::xref_adapter::normalize(ctx).context("normalizing the raw cross-references TSV into cites rows")?;
        crate::event_world::normalize(ctx);
        crate::era_adapter::normalize(ctx);
        crate::polity_adapter::normalize(ctx);
        crate::catechism_adapter::normalize(ctx);
        crate::concord_adapter::normalize(ctx);
        ctx.stats.concord_citations = crate::citations::cite_scripture(&mut ctx.graph);
        crate::kretzmann_adapter::normalize(ctx);
        crate::person_adapter::normalize(ctx);
        crate::red_letter_adapter::normalize(ctx);
        // After the KJV adapter, so every verse node already exists and a token on a verse the graph
        // lacks is counted rather than left dangling.
        crate::lexicon_adapter::normalize(ctx);
        crate::peoples_adapter::normalize(ctx);
        crate::fulfillment_adapter::normalize(ctx);
        Ok(())
    }
}

struct MergeAliasPass;
impl Pass for MergeAliasPass {
    fn name(&self) -> &'static str {
        "merge_alias"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        crate::place_adapter::merge_alias(ctx);
        crate::catechism_adapter::merge_alias(ctx);
        crate::concord_adapter::merge_alias(ctx);
        crate::person_adapter::merge_alias(ctx);
        crate::peoples_adapter::merge_alias(ctx);
        ctx.stats.mention_spans = crate::mention_spans::locate_mentions(ctx);
        let description_stats = crate::description_adapter::fill_descriptions(ctx);
        ctx.description_stats = description_stats;
        Ok(())
    }
}

struct ResolvePass;
impl Pass for ResolvePass {
    fn name(&self) -> &'static str {
        "resolve"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        crate::event_world::resolve(ctx);
        Ok(())
    }
}

struct DerivePass;
impl Pass for DerivePass {
    fn name(&self) -> &'static str {
        "derive"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        // Reading order is already complete -- NORMALIZE built the spine -- and member-of is free, the
        // inverse projection of contains/attests. `parallel` stays unmaterialized, a standing gap.
        crate::event_world::derive(ctx);
        Ok(())
    }
}

struct IndexPass;
impl Pass for IndexPass {
    fn name(&self) -> &'static str {
        "index"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        ctx.graph.build_indexes();
        ctx.justified_by_count = crate::event_world::add_justified_by(&mut ctx.graph);
        Ok(())
    }
}

struct LabelPass;
impl Pass for LabelPass {
    fn name(&self) -> &'static str {
        "label"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        let geography = crate::geography::Geography::compile(&ctx.graph, ctx.atlas);
        crate::labels::compile(&mut ctx.graph, &crate::labels::ReaderNames::of(&geography, ctx.atlas));
        Ok(())
    }
}

struct MapPass;
impl Pass for MapPass {
    fn name(&self) -> &'static str {
        "map"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        crate::map_adapter::derive(ctx).context("deriving one Map per era from the scene at its window")?;
        Ok(())
    }
}

struct LawCheckPass;
impl Pass for LawCheckPass {
    fn name(&self) -> &'static str {
        "law_check"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        if let Some(source) = ctx.kjv_json_source {
            crate::fidelity::check_kjv_fidelity(source, &ctx.graph, ctx.brainfuel)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("KJV adapter fidelity law (bijection + reconstruction)")?;
        }
        crate::law_check::every_row_reference_resolves(&ctx.graph).context("referential integrity of authored edge rows")?;
        crate::law_check::container_containment_is_a_forest(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("NODE1-ROWS-1 container-containment forest law (acyclicity + single-parent)")?;
        crate::law_check::kinship_is_acyclic(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("D5 kinship law (acyclic, no duplicate parent-of pair)")?;
        crate::law_check::attestation_is_exclusive(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("ATTEST-1 attestation-exclusivity law (L2)")?;
        crate::law_check::analogue_rows_join_two_distinct_events(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("ATTEST-1 Analogue distinctness law (L4)")?;
        // The adapter-specific halves only: referential integrity of these rows' endpoints is already
        // covered by `every_row_reference_resolves` above, so re-checking it here would duplicate it.
        crate::person_adapter::check_person_fidelity(ctx.atlas, &ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Theographic person adapter fidelity law (bijection + mentions completeness)")?;
        crate::peoples_adapter::check_peoples_fidelity(ctx.atlas, &ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("PG-1a peoples adapter fidelity law (bijection + mentions completeness)")?;
        crate::peoples_adapter::every_named_after_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("PG-1a named-after grounding law (every row must carry >=1 Ground::Scripture)")?;
        crate::fulfillment_adapter::every_fulfillment_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("EDGE-1a fulfillment grounding law (every row must carry >=1 Ground::Scripture)")?;
        crate::fulfillment_adapter::every_typology_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("EDGE-1a typology grounding law (every row must carry >=1 Ground::Scripture)")?;
        Ok(())
    }
}

/// THE ordered contract: the named stages, one value each, in the order `run_pipeline` executes them.
/// This is the ONE place the order is declared; nothing else names the sequence.
pub fn pipeline() -> Vec<Box<dyn Pass>> {
    vec![
        Box::new(NormalizePass),
        Box::new(MergeAliasPass),
        Box::new(ResolvePass),
        Box::new(DerivePass),
        Box::new(IndexPass),
        Box::new(LabelPass),
        Box::new(MapPass),
        Box::new(IndexPass),
        Box::new(LabelPass),
        Box::new(LawCheckPass),
    ]
}

/// Runs every pass in `passes`, in order: a total function over WHATEVER list it is handed, so a
/// reduced list gets exactly that reduced behaviour.
pub fn run_pipeline(ctx: &mut BuildCtx, passes: &[Box<dyn Pass>]) -> Result<()> {
    for pass in passes {
        pass.run(ctx).with_context(|| format!("pipeline pass '{}' failed", pass.name()))?;
    }
    Ok(())
}

#[cfg(test)]
mod pipeline_tests {
    use super::*;

    fn empty_ctx() -> (Canon, std::collections::HashMap<String, String>, AtlasData) {
        (Canon { books: vec![] }, std::collections::HashMap::new(), crate::event_world::empty_atlas())
    }

    #[test]
    fn the_named_stages_run_in_the_documented_order() {
        let names: Vec<&str> = pipeline().iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["normalize", "merge_alias", "resolve", "derive", "index", "label", "map", "index", "label", "law_check"]);
    }

    #[test]
    fn a_pass_removed_from_the_list_never_runs() {
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let reduced: Vec<Box<dyn Pass>> = pipeline().into_iter().filter(|p| p.name() != "law_check").collect();
        assert_eq!(reduced.len(), 9, "every OTHER stage stays -- only law_check was backed out");
        assert!(run_pipeline(&mut ctx, &reduced).is_ok(), "a reduced pipeline still runs the passes it DOES list");
        assert_eq!(ctx.graph.nodes.len(), 0, "an empty fixture still builds an empty (not fabricated) graph");
    }

    #[test]
    fn a_failing_pass_names_itself_in_the_error() {
        struct AlwaysFails;
        impl Pass for AlwaysFails {
            fn name(&self) -> &'static str {
                "always_fails"
            }
            fn run(&self, _ctx: &mut BuildCtx) -> Result<()> {
                anyhow::bail!("deliberate failure")
            }
        }
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let passes: Vec<Box<dyn Pass>> = vec![Box::new(AlwaysFails)];
        let err = run_pipeline(&mut ctx, &passes).expect_err("must fail");
        assert!(format!("{err:#}").contains("always_fails"), "the error must name which pass failed: {err:#}");
    }

    const A_SOURCE_NAMING_ONE_VERSE: &str =
        r#"{"books":[{"name":"Genesis","chapters":[{"chapter":1,"verses":[{"verse":1,"text":"In the beginning God created the heaven and the earth."}]}]}]}"#;

    #[test]
    fn the_law_check_stage_refuses_a_graph_its_own_source_does_not_agree_with() {
        // Arrange
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, Some(A_SOURCE_NAMING_ONE_VERSE), "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);

        // Act
        let refusal = run_pipeline(&mut ctx, &pipeline()).expect_err("an empty canon cannot satisfy a source that names a verse");

        // Assert
        assert!(
            format!("{refusal:#}").contains("law_check") && format!("{refusal:#}").contains("KJV adapter fidelity law"),
            "{refusal:#}"
        );
    }

    #[test]
    fn full_pipeline_over_a_trivial_fixture_is_green() {
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        run_pipeline(&mut ctx, &pipeline()).expect("the full pipeline must run clean over an empty-but-honest fixture");
    }
}
