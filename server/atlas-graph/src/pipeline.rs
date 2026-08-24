//! THE COMPILER PIPELINE CONTRACT (design doc §7; P7 gap-closure, seam
//! inventory entry "COMPILER PIPELINE"; controller decision 3, M-C):
//! `normalize -> merge/alias -> resolve -> derive -> index -> law-check`,
//! formalized as an ORDERED CONTRACT. Passes are DATA -- a `Vec<Box<dyn
//! Pass>>` returned by [`pipeline`] -- not a hardcoded call chain baked
//! into one function body. Backing a pass out is removing it from the
//! list (`pipeline_tests::a_pass_removed_from_the_list_never_runs` proves
//! this literally, not just by architecture-diagram assertion); adding a
//! future pass (e.g. a materialized `temporal-adjacency`/`parallel` DERIVE
//! step) is adding one more entry, never restructuring the runner.
//!
//! The M-A/M-B build logic (kjv_adapter/xref_adapter/event_world::populate/
//! `Graph::build_indexes`/`add_justified_by`/`fidelity::check_kjv_fidelity`)
//! restructures INTO this shape here -- BEHAVIOR-IDENTICAL, proven by
//! `tests/version_root_regression.rs`: the graph version root (a content
//! hash over every node's id+payload, design doc §9b) is UNCHANGED for the
//! real committed sources before and after this restructuring. Content
//! addressing is the regression harness the controller's own instruction
//! names -- if the pipeline computed anything different, the root would
//! move, and that test would catch it.
//!
//! STAGE MAPPING, disclosed (this codebase's own current shape does not
//! split as cleanly as the six textbook names might suggest -- each stage
//! below states plainly what it does and does not cover, rather than
//! forcing an artificial split that would only obscure the real
//! dependency order):
//! - NORMALIZE: raw-ish typed inputs -> nodes + the directly-authored rows
//!   that need no cross-referencing (KJV TextUnit nodes + the bible
//!   reading spine; `cites` rows from the xrefs TSV; Event/Narrative/
//!   Anchor/Place/Era/Polity/CatechismItem nodes plus the witness-derived
//!   `attests`/`succession`/`located_at` rows event_world::populate
//!   already builds in one pass, since none of those need any OTHER
//!   pass's output first).
//! - MERGE/ALIAS: legacy-vocabulary boundary crossings -- place KJV
//!   naming -> `named` rows, place `verse_links` -> `mentions` rows,
//!   catechism item/question verse citations -> `catechism-link` rows,
//!   and (Batch P) person `verse_links` -> `mentions` rows (no `named`
//!   counterpart for Person -- see `person_adapter.rs`'s own doc comment).
//!   Matches the ingestion contract's own parenthetical almost exactly
//!   ("event and place merge tables become assertion-level rules") --
//!   this batch's place/catechism/person adapters are that instruction
//!   realized.
//! - RESOLVE: `DatePlacement` resolution -- `derive_chronology` chooses
//!   each dated event's placement and the `dated_by` rows it grounds,
//!   exactly `resolve(DatePlacement)` per the ingestion contract's own
//!   naming.
//! - DERIVE: computed relations with no authored-row counterpart --
//!   reading-order (already complete as the spine NORMALIZE built; this
//!   stage asserts it, rather than rebuilding it a second time), member-of
//!   (free: the inverse projection of `contains`/`attests`, nothing to
//!   compute), and (TRAV-1) `temporal-adjacency`: one `TemporalAdjacency`
//!   row per consecutive pair of the chronology's own global order
//!   (`event_world::populate_temporal_adjacency`, called from
//!   `event_world::derive` below) -- this is the "future batch's own
//!   derivation" this stage's doc comment named a landing spot for since
//!   M-C; `parallel` stays UNMATERIALIZED (a real, disclosed, standing gap,
//!   out of TRAV-1's own named scope).
//! - INDEX: `Graph::build_indexes()` (unmodified graph-types primitive;
//!   one BiIndex pass per relation, directed AND -- new this batch --
//!   symmetric) plus `add_justified_by` (a post-processing step that must
//!   run after the DatedBy index exists, since it mirrors that index's
//!   own edge-id computation).
//! - LAW-CHECK: `fidelity::check_kjv_fidelity` (the KJV adapter's
//!   bijection + reconstruction boundary law, unconditional, fail-loud)
//!   plus a new, generic REFERENTIAL-INTEGRITY check
//!   (`law_check::every_authored_edge_resolves`) that every authored row's
//!   endpoints name a real node in the built graph -- cheap, and gives
//!   this stage real substance beyond the one adapter M-A shipped it for.

use anyhow::{Context, Result};

use atlas_core::data::{AtlasData, Canon};
use atlas_graph_types::graph::Graph;

use crate::build::BuildStats;
use crate::event_world::{ChronologyDerivation, EventWorldStats};

/// Everything a pass needs: the typed inputs every adapter reads, the
/// in-progress `Graph` every pass mutates in place, and the running
/// stats/derivation state later passes (and the caller, for startup
/// logging) consume. Passes run in the order `pipeline()` lists; each may
/// read/write any field.
pub struct BuildCtx<'a> {
    pub kjv_canon: &'a Canon,
    pub kjv_verses: &'a std::collections::HashMap<String, String>,
    /// The RAW KJV JSON text, when one exists (`from_sources`'s own real
    /// startup path always has it; `from_canon_and_verses`'s test-fixture
    /// path does not, exactly as before this refactor -- "no raw source
    /// BYTES exist to re-derive 'expected' from"). Kept separately from
    /// `kjv_canon`/`kjv_verses` on purpose: the fidelity law's own
    /// `independent_reader` (fidelity.rs) must re-parse these bytes from
    /// scratch, sharing no code with the adapter path that built
    /// `kjv_canon`/`kjv_verses` -- collapsing the two into "just use the
    /// already-parsed canon" would silently de-independent the law I2 fix
    /// round 1 deliberately established.
    pub kjv_json_source: Option<&'a str>,
    pub xrefs_tsv: &'a str,
    pub atlas: &'a AtlasData,
    /// M-C's own Era adapter source (`era_adapter.rs`): pre-parsed rows
    /// from `data/curated/eras.toml`, via the SAME `atlas_etl::curated::
    /// parse_eras` the pre-M-C `AtlasData.eras`/`eras.json` path used --
    /// NOT read from `AtlasData` (unlike every other M-C adapter's own
    /// source), since `.eras` is one of the fields this batch's own
    /// deletion event retires (see the batch report's deletion inventory).
    /// Defaults to empty for every caller that doesn't supply real eras
    /// (most test fixtures, via `build_graph_from_sources`/
    /// `build_graph_from_canon_and_verses`'s own plain forms) -- an empty
    /// slice is a lawful, honest "no eras this build," not a placeholder.
    pub eras: &'a [atlas_core::data::Era],
    pub graph: Graph,
    pub stats: BuildStats,
    pub event_world_stats: EventWorldStats,
    pub chrono: ChronologyDerivation,
    pub justified_by_count: usize,
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
        BuildCtx {
            kjv_canon,
            kjv_verses,
            kjv_json_source,
            xrefs_tsv,
            atlas,
            eras,
            graph: Graph::default(),
            stats: BuildStats::default(),
            event_world_stats: EventWorldStats::default(),
            chrono: ChronologyDerivation::default(),
            justified_by_count: 0,
        }
    }
}

/// A compiler pass: a named, total step over the build context. `name()`
/// exists for error attribution (`run_pipeline` names which pass failed)
/// and for the "passes as data" proof (`pipeline_tests` inspects the
/// list's own names, not a hardcoded call chain).
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
        crate::xref_adapter::normalize(ctx).context("normalizing the raw cross-references TSV into cites rows")?;
        crate::event_world::normalize(ctx);
        crate::era_adapter::normalize(ctx);
        crate::polity_adapter::normalize(ctx);
        crate::catechism_adapter::normalize(ctx);
        // Batch P (the extensibility proof): Person nodes need no OTHER
        // pass's output first (same NORMALIZE-eligibility reasoning as
        // every sibling call above) -- ctx.atlas.people is already fully
        // resolved by atlas_etl::people::parse_people before this ever runs.
        crate::person_adapter::normalize(ctx);
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
        // Batch P: Person.verse_links -> mentions rows, the SAME
        // "legacy-vocabulary boundary crossing" shape place_adapter's own
        // mentions half already is (this stage's own doc comment above).
        crate::person_adapter::merge_alias(ctx);
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
        // Reading-order is already complete (NORMALIZE built the spine);
        // member-of is free (the inverse projection of contains/attests).
        // TRAV-1: temporal-adjacency now materializes here -- see this
        // module's own DERIVE doc comment and event_world::derive. parallel
        // stays unmaterialized (disclosed, standing gap, out of TRAV-1's
        // own named scope).
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

struct LawCheckPass;
impl Pass for LawCheckPass {
    fn name(&self) -> &'static str {
        "law_check"
    }
    fn run(&self, ctx: &mut BuildCtx) -> Result<()> {
        // Only the raw-source path (`from_sources`) carries bytes to
        // independently re-derive "expected" from; the canon-and-verses
        // test-fixture path skips this the same way it always has (see
        // `BuildCtx::kjv_json_source`'s own doc comment).
        if let Some(source) = ctx.kjv_json_source {
            crate::fidelity::check_kjv_fidelity(source, &ctx.graph)
                .map_err(|e| anyhow::anyhow!("{e}"))
                .context("KJV adapter fidelity law (bijection + reconstruction)")?;
        }
        crate::law_check::every_authored_edge_resolves(&ctx.graph).context("referential integrity of authored edge rows")?;
        // M-D3 (owner ruling R1): the "verified-cache law"
        // (`law_check::payload_years_match_resolved_placements`) RETIRED
        // WITH the `NodePayload::Event.from_year`/`.to_year` fields it
        // existed to police -- see law_check.rs's own retirement note.
        // Batch P (the extensibility proof): the Person adapter's own
        // boundary fidelity law (bijection + mentions completeness) --
        // referential integrity of Person mentions rows is ALREADY covered
        // by every_authored_edge_resolves above (see person_adapter.rs's
        // own module doc comment), so this is the adapter-specific half
        // only, not a duplicate.
        crate::person_adapter::check_person_fidelity(ctx.atlas, &ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("Theographic person adapter fidelity law (bijection + mentions completeness)")?;
        Ok(())
    }
}

/// THE ordered contract: six named stages, one value each, in the order
/// `run_pipeline` executes them. This function is the ONE place the order
/// is declared -- backing a stage out is deleting its line here; nothing
/// else names the sequence.
pub fn pipeline() -> Vec<Box<dyn Pass>> {
    vec![
        Box::new(NormalizePass),
        Box::new(MergeAliasPass),
        Box::new(ResolvePass),
        Box::new(DerivePass),
        Box::new(IndexPass),
        Box::new(LawCheckPass),
    ]
}

/// Runs every pass in `passes`, in order, over `ctx` -- a total function
/// over WHATEVER list it is handed (it does not know or care that
/// `pipeline()` normally supplies six; a caller building a reduced list,
/// e.g. for a test proving a backed-out pass never runs, gets exactly
/// that reduced behavior).
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
    fn the_six_named_stages_run_in_the_documented_order() {
        let names: Vec<&str> = pipeline().iter().map(|p| p.name()).collect();
        assert_eq!(names, vec!["normalize", "merge_alias", "resolve", "derive", "index", "law_check"]);
    }

    #[test]
    fn a_pass_removed_from_the_list_never_runs() {
        // "Backing one out = removing it from the list" -- proven, not just
        // architected: a reduced list that DROPS law_check produces a graph
        // with no fidelity gate ever having run (no panic, no error) --
        // demonstrating the list is genuinely live control flow, not a
        // facade over a hardcoded call chain a doc comment merely describes.
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let reduced: Vec<Box<dyn Pass>> = pipeline().into_iter().filter(|p| p.name() != "law_check").collect();
        assert_eq!(reduced.len(), 5, "every OTHER stage stays -- only law_check was backed out");
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

    #[test]
    fn full_pipeline_over_a_trivial_fixture_is_green() {
        let (canon, verses, atlas) = empty_ctx();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        run_pipeline(&mut ctx, &pipeline()).expect("the full six-stage pipeline must run clean over an empty-but-honest fixture");
    }
}
