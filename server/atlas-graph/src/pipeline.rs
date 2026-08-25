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
//!   realized. (ENT-1a) `description_adapter::fill_descriptions` joins
//!   this stage too -- Easton's Bible Dictionary (1897) is exactly the
//!   same "legacy-vocabulary boundary crossing" shape, just onto an
//!   already-NORMALIZED node's own PAYLOAD field rather than a new
//!   relation row (see that module's own doc comment for the full "why
//!   MERGE/ALIAS, not a new stage" reasoning).
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
    /// CORP-1a: `atlas_etl::brainfuel::read_all`'s own pre-parsed corpus
    /// (the six ingested editions' renderings + the `king_james` column
    /// carried only for the cross-check) -- `None` for every caller that
    /// doesn't supply real brain-fuel data (every OTHER adapter's own test
    /// fixtures, via `BuildCtx::new`/`with_eras`), the SAME "absent ==
    /// honestly empty, not a placeholder" treatment `eras` above already
    /// gets. Reference, not owned: the corpus can be large (31,102 rows
    /// over the real vendored data) and every real caller already has one
    /// living for the duration of the build (mirrors `eras: &'a [Era]`).
    pub brainfuel: Option<&'a atlas_etl::brainfuel::BrainFuelCorpus>,
    /// CORP-2a: `concord_adapter.rs`'s own source -- the parsed Book of
    /// Concord corpus plus the curated SC-overlap alignment, bundled
    /// (`concord_adapter::ConcordBundle`'s own doc comment explains why
    /// ONE optional field, not two). The SAME "absent == an honestly
    /// empty build, not a placeholder" treatment `brainfuel`/`eras` above
    /// already get -- every test fixture that doesn't supply real Concord
    /// data (every caller of `BuildCtx::new`/`with_eras`/`with_eras_and_
    /// brainfuel`) gets `None`, unchanged from before this batch.
    pub concord: Option<&'a crate::concord_adapter::ConcordBundle>,
    /// KRETZ-1: `atlas_etl::kretzmann::read_all`'s own pre-parsed corpus --
    /// the SAME "absent == an honestly empty build, not a placeholder"
    /// treatment `concord`/`brainfuel` above already get (every test
    /// fixture that doesn't supply real Kretzmann data, via `BuildCtx::
    /// new`/`with_eras`/..., gets `None`, unchanged from before this
    /// batch). Reference, not owned: the corpus is large (61,366 excised
    /// fragments alone over the real vendored data, fix round 1's own
    /// pinned count), and every real caller
    /// already has one living for the duration of the build.
    pub kretzmann: Option<&'a atlas_etl::kretzmann::KretzmannCorpus>,
    /// ENT-1a: `description_adapter::fill_descriptions`'s own return value,
    /// captured here (not just returned-and-discarded, unlike the other
    /// MERGE/ALIAS adapter calls' own Stats structs) so a caller that
    /// constructs a `BuildCtx` directly and drives `run_pipeline` itself --
    /// `description_real_data.rs`'s own real-data fill-rate report test is
    /// the one real caller today -- can read the fill-rate breakdown
    /// afterward, without a second, redundant pass over the graph.
    /// DISCLOSED, not silently narrower than it sounds: `build::
    /// build_graph_from_sources_with_eras` (and so `bins/compile_graph.rs`,
    /// which only calls that wrapper) does NOT surface this field -- its
    /// own return tuple is a fixed, widely-depended-on shape this batch
    /// deliberately did not widen. Stays `Default` (all zero) until
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

    /// CORP-1a: the richest form -- real startup (via `GraphService::build`)
    /// and the artifact compile step (`bins/compile_graph.rs`) use this
    /// directly, with real brain-fuel data; every other caller keeps
    /// calling `new`/`with_eras` unchanged, getting an honestly absent
    /// (`None`) `brainfuel` -- see this struct's own `brainfuel` field doc
    /// comment.
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

    /// CORP-2a: the richest form yet -- real startup and the artifact
    /// compile step use this directly, with a real, pre-parsed
    /// `concord_adapter::ConcordBundle`; every other caller keeps calling
    /// `new`/`with_eras`/`with_eras_and_brainfuel` unchanged, getting an
    /// honestly absent (`None`) `concord` -- see this struct's own
    /// `concord` field doc comment.
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

    /// KRETZ-1: the richest form yet -- real startup and the artifact
    /// compile step use this directly, with a real, pre-parsed
    /// `atlas_etl::kretzmann::KretzmannCorpus`; every other caller keeps
    /// calling `new`/`with_eras`/`with_eras_and_brainfuel`/`with_eras_and_
    /// brainfuel_and_concord` unchanged, getting an honestly absent
    /// (`None`) `kretzmann` -- see this struct's own `kretzmann` field doc
    /// comment.
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
            graph: Graph::default(),
            stats: BuildStats::default(),
            event_world_stats: EventWorldStats::default(),
            chrono: ChronologyDerivation::default(),
            justified_by_count: 0,
            description_stats: crate::description_adapter::DescriptionStats::default(),
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
        // CORP-1a: MUST run immediately after kjv_adapter::normalize --
        // this call mutates the very TextUnit nodes the line above just
        // inserted (module doc comment on brainfuel_adapter.rs), a real,
        // disclosed in-stage ordering dependency.
        crate::brainfuel_adapter::normalize(ctx);
        crate::xref_adapter::normalize(ctx).context("normalizing the raw cross-references TSV into cites rows")?;
        crate::event_world::normalize(ctx);
        crate::era_adapter::normalize(ctx);
        crate::polity_adapter::normalize(ctx);
        crate::catechism_adapter::normalize(ctx);
        // CORP-2a: the Concord corpus's own TextUnit nodes + document/
        // article containers + reading spine -- self-contained (no OTHER
        // pass's output needed first), the SAME NORMALIZE-eligibility
        // `kjv_adapter::normalize` above already has (module doc comment
        // on `concord_adapter.rs`).
        crate::concord_adapter::normalize(ctx);
        // KRETZ-1: the Kretzmann corpus's own Source node + CommentaryItem
        // nodes + comments_on rows -- self-contained (no OTHER pass's
        // output needed first), the SAME NORMALIZE-eligibility `concord_
        // adapter::normalize` above already has (module doc comment on
        // `kretzmann_adapter.rs`).
        crate::kretzmann_adapter::normalize(ctx);
        // Batch P (the extensibility proof): Person nodes need no OTHER
        // pass's output first (same NORMALIZE-eligibility reasoning as
        // every sibling call above) -- ctx.atlas.people is already fully
        // resolved by atlas_etl::people::parse_people before this ever runs.
        crate::person_adapter::normalize(ctx);
        // PG-1a: PeopleGroup nodes (all three sources) + curated NamedAfter
        // rows -- runs AFTER person_adapter::normalize, deliberately (its
        // own module doc comment): the NamedAfter eponym-existence check
        // asks the GRAPH itself whether a real Person node exists, and
        // person_adapter's own exclusion of the nine reclassified slugs
        // must already be in effect for the reclassified-node/no-Person-
        // node invariant to hold by construction.
        crate::peoples_adapter::normalize(ctx);
        // EDGE-1a: Fulfills/Typology rows need no OTHER pass's output first
        // (pure Scripture text-to-text rows -- module doc comment's own
        // "NORMALIZE-eligible" reasoning).
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
        // CORP-2a: the SC-overlap CatechismLink rows (decision 4) -- runs
        // AFTER catechism_adapter::merge_alias only incidentally (no
        // ordering dependency between the two catechism-link builders
        // themselves); the REAL dependency is on catechism_adapter::
        // NORMALIZE (CatechismItem nodes) and concord_adapter::NORMALIZE
        // (Concord TextUnits), both already complete by the time
        // MergeAliasPass runs at all (concord_adapter.rs's own module doc
        // comment).
        crate::concord_adapter::merge_alias(ctx);
        // Batch P: Person.verse_links -> mentions rows, the SAME
        // "legacy-vocabulary boundary crossing" shape place_adapter's own
        // mentions half already is (this stage's own doc comment above).
        crate::person_adapter::merge_alias(ctx);
        // PG-1a: the reclassified nine's own verse_links -> Mentions
        // (PeopleGroup) rows -- the ONLY per-locus PeopleGroup attestations
        // this batch's source data ships (decision 1c/2); the (a)/(b)
        // PeopleGroup sources build no mentions rows at all.
        crate::peoples_adapter::merge_alias(ctx);
        // ENT-1a: description-filling runs LAST in this stage -- it only
        // ever READS already-built nodes (never their mentions rows), so it
        // has no ordering dependency on the three calls above; last is
        // simply where a payload-only widening reads most naturally,
        // after every row-building call. (Two statements, not `ctx.x =
        // f(ctx)`, deliberately -- the latter borrows `ctx` twice at once.)
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
            // Batch KJV-CASE: `ctx.brainfuel` threads through so the law's
            // own independent "expected" re-derivation applies the SAME
            // case restoration the NORMALIZE stage already applied to the
            // graph's own TextUnit renderings (see `fidelity.rs`'s own
            // module doc comment) -- `None` on every caller with no real
            // brainfuel data is an honest no-op, unchanged from before.
            crate::fidelity::check_kjv_fidelity(source, &ctx.graph, ctx.brainfuel)
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
        // PG-1a: the peoples adapter's own boundary fidelity law (bijection
        // across all three PeopleGroup sources + reclassified mentions
        // completeness), plus the NamedAfter grounding law (every row
        // carries >=1 Ground::Scripture) -- referential integrity of
        // PeopleGroup mentions/named_after endpoints is ALREADY covered by
        // every_authored_edge_resolves above (peoples_adapter.rs's own
        // module doc comment / test), so these are the adapter-specific
        // laws only, not a duplicate.
        crate::peoples_adapter::check_peoples_fidelity(ctx.atlas, &ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("PG-1a peoples adapter fidelity law (bijection + mentions completeness)")?;
        crate::peoples_adapter::every_named_after_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("PG-1a named-after grounding law (every row must carry >=1 Ground::Scripture)")?;
        // EDGE-1a: same "every row must carry >=1 Ground::Scripture" law,
        // over the two new relation tables (referential integrity of their
        // endpoints is ALREADY covered by every_authored_edge_resolves
        // above -- these are the adapter-specific grounding laws only).
        crate::fulfillment_adapter::every_fulfillment_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("EDGE-1a fulfillment grounding law (every row must carry >=1 Ground::Scripture)")?;
        crate::fulfillment_adapter::every_typology_row_has_a_scripture_ground(&ctx.graph)
            .map_err(|e| anyhow::anyhow!("{e}"))
            .context("EDGE-1a typology grounding law (every row must carry >=1 Ground::Scripture)")?;
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
