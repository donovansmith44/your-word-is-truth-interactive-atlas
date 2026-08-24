//! EDGE-1a ("Prophecy & typology: the seed data" -- batch-edge1a-brief.md,
//! owner order 2026-08-23: "we also need a couple more edges: one for
//! Christological types, and one for prophecy/fulfillment"). The crate
//! relations (`Fulfills`/`Typology`) landed in commit 98a8dce -- this
//! module AUTHORS THE ROWS from two curated files, `data/curated/
//! fulfillments.toml` (`[[fulfillment]]`, decision 1a: the NT's own
//! explicit fulfillment-formula set) and `data/curated/typology.toml`
//! (`[[typology]]`, decision 1b: the Scripture-argued type/antitype seed
//! list) -- the SAME two-phase "curated seed -> typed graph row"
//! translation `peoples_adapter::normalize`'s own NamedAfter half already
//! establishes, one relation simpler (no eponym-existence conditional --
//! every row here is Scripture-only, text-to-text, nothing to look up
//! against an already-built node set).
//!
//! SELF-ATTESTING GROUNDS (both tables): a `FulfillmentSeed`/`TypologySeed`
//! row's own `text` field is the hand-verified KJV formula quote/figure
//! note (curated data, never invented here) -- its `Justification.grounds`
//! is ALWAYS exactly `{Ground::Scripture(<the fulfillment/antitype
//! passage>)}`, mirroring `Fulfills`'s own doc comment ("Scripture
//! frequently SELF-ATTESTS these rows: the NT fulfillment formulas... make
//! the fulfillment passage itself the natural Ground::Scripture") and
//! extending the identical convention to Typology (the antitype passage
//! is where Scripture itself argues the type -- Melchizedek's own case,
//! e.g., is HEB 7 arguing FROM the Genesis 14 record, so HEB 7 is the
//! ground). KJV INERRANCY DIRECTIVE: every row's `text` states fulfillment/
//! prefiguring as FACT, never hedged -- curated at authoring time, not
//! synthesized here.
//!
//! SKIP, DON'T PANIC (the SAME discipline `peoples_adapter::normalize`'s
//! own NamedAfter loop follows for an unparseable ground): a curated row
//! whose `prophecy`/`fulfillment` (or `type_passage`/`antitype_passage`)
//! locus fails to parse (bad verse ref, or an inverted range) is omitted,
//! not fatal -- `FulfillmentAdapterStats::{fulfillment,typology}_omitted`
//! names every skip with why. Never expected to fire over the real
//! committed data (every row is hand-verified against the KJV text before
//! being curated -- see the batch report), but a curated-TOML typo must
//! fail loud via the omission list, not silently corrupt a row.

use std::collections::BTreeSet;

use atlas_graph_types::edge::{Fulfills, Ground, Justification, Typology};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::text::BibleLocusRange;

use crate::peoples_adapter::ground_range;
use crate::pipeline::BuildCtx;

pub const PROVENANCE_FULFILLMENT: &str = "curated-fulfillment";
pub const PROVENANCE_TYPOLOGY: &str = "curated-typology";

#[derive(Debug, Clone, Default)]
pub struct FulfillmentAdapterStats {
    pub fulfillment_rows: usize,
    /// `(row index into data/curated/fulfillments.toml, reason)` -- every
    /// curated `[[fulfillment]]` row this adapter declined to build.
    pub fulfillment_omitted: Vec<(usize, String)>,
    pub typology_rows: usize,
    /// `(row index into data/curated/typology.toml, reason)` -- every
    /// curated `[[typology]]` row this adapter declined to build.
    pub typology_omitted: Vec<(usize, String)>,
}

fn scripture_ground(range: &BibleLocusRange) -> BTreeSet<Ground> {
    let mut grounds = BTreeSet::new();
    grounds.insert(Ground::Scripture(range.clone()));
    grounds
}

/// NORMALIZE: `graph.fulfills`/`graph.typology` rows from the two curated
/// seed lists. No dependency on any other pass's output (pure Scripture
/// text-to-text rows -- unlike `peoples_adapter`'s own NamedAfter half,
/// there is no node-existence check to perform), so this is
/// NORMALIZE-eligible the same way `person_adapter::normalize`'s own doc
/// comment already establishes for its analogous case. Called from
/// `pipeline::NormalizePass`.
pub fn normalize(ctx: &mut BuildCtx) -> FulfillmentAdapterStats {
    let mut stats = FulfillmentAdapterStats::default();

    for (i, row) in ctx.atlas.fulfillment_seeds.iter().enumerate() {
        let (Some(prophecy), Some(fulfillment)) = (ground_range(&row.prophecy), ground_range(&row.fulfillment)) else {
            stats.fulfillment_omitted.push((i, "prophecy or fulfillment locus failed to parse (bad verse ref, or an inverted range)".to_string()));
            continue;
        };
        let grounds = scripture_ground(&fulfillment);
        ctx.graph.fulfills.push(Fulfills {
            prophecy,
            fulfillment,
            provenance: ProvenanceId::from(PROVENANCE_FULFILLMENT),
            justification: Justification { text: Some(row.text.clone()), grounds },
        });
        stats.fulfillment_rows += 1;
    }

    for (i, row) in ctx.atlas.typology_seeds.iter().enumerate() {
        let (Some(type_passage), Some(antitype_passage)) = (ground_range(&row.type_passage), ground_range(&row.antitype_passage)) else {
            stats.typology_omitted.push((i, "type_passage or antitype_passage locus failed to parse (bad verse ref, or an inverted range)".to_string()));
            continue;
        };
        let grounds = scripture_ground(&antitype_passage);
        ctx.graph.typology.push(Typology {
            type_passage,
            antitype_passage,
            note: Some(row.note.clone()),
            provenance: ProvenanceId::from(PROVENANCE_TYPOLOGY),
            justification: Justification { text: Some(row.text.clone()), grounds },
        });
        stats.typology_rows += 1;
    }

    stats
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FulfillmentGroundingViolation(pub String);

impl std::fmt::Display for FulfillmentGroundingViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EDGE-1a fulfillment/typology grounding violation: {}", self.0)
    }
}
impl std::error::Error for FulfillmentGroundingViolation {}

/// Brief decision 5 ("every-row-has-Scripture-ground law test (same shape
/// as PG-1a's)"): a FRESH check over the built graph's own `fulfills`
/// table, independent of how `normalize` above constructed it -- the SAME
/// "check the built graph, don't just trust the adapter" discipline
/// `peoples_adapter::every_named_after_row_has_a_scripture_ground` already
/// follows.
pub fn every_fulfillment_row_has_a_scripture_ground(graph: &atlas_graph_types::graph::Graph) -> Result<(), FulfillmentGroundingViolation> {
    for row in &graph.fulfills {
        let has_scripture_ground = row.justification.grounds.iter().any(|g| matches!(g, Ground::Scripture(_)));
        if !has_scripture_ground {
            return Err(FulfillmentGroundingViolation(format!(
                "fulfills row (fulfillment first-verse book {:?}) carries no Ground::Scripture in its own justification -- {} ground(s) total",
                row.fulfillment.from.unit,
                row.justification.grounds.len()
            )));
        }
    }
    Ok(())
}

/// The Typology sibling of `every_fulfillment_row_has_a_scripture_ground`
/// above -- same discipline, `graph.typology`'s own table.
pub fn every_typology_row_has_a_scripture_ground(graph: &atlas_graph_types::graph::Graph) -> Result<(), FulfillmentGroundingViolation> {
    for row in &graph.typology {
        let has_scripture_ground = row.justification.grounds.iter().any(|g| matches!(g, Ground::Scripture(_)));
        if !has_scripture_ground {
            return Err(FulfillmentGroundingViolation(format!(
                "typology row ({}) carries no Ground::Scripture in its own justification -- {} ground(s) total",
                row.note.as_deref().unwrap_or("<no note>"),
                row.justification.grounds.len()
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, FulfillmentSeed, ScriptureGroundSeed, TypologySeed};
    use std::collections::HashMap;

    fn atlas_with(fulfillment_seeds: Vec<FulfillmentSeed>, typology_seeds: Vec<TypologySeed>) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.fulfillment_seeds = fulfillment_seeds;
        d.typology_seeds = typology_seeds;
        d
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData) -> BuildCtx<'a> {
        BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas)
    }

    fn g(from: &str, to: Option<&str>) -> ScriptureGroundSeed {
        ScriptureGroundSeed { from: from.into(), to: to.map(String::from) }
    }

    // --- normalize: fulfillments ---------------------------------------

    #[test]
    fn normalize_builds_one_fulfills_row_per_curated_seed() {
        let atlas = atlas_with(
            vec![FulfillmentSeed {
                prophecy: g("ISA.7.14", None),
                fulfillment: g("MAT.1.22", Some("MAT.1.23")),
                text: "that it might be fulfilled which was spoken of the Lord by the prophet, saying, Behold, a virgin shall be with child".into(),
            }],
            vec![],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.fulfillment_rows, 1);
        assert!(stats.fulfillment_omitted.is_empty());
        assert_eq!(ctx.graph.fulfills.len(), 1);

        let row = &ctx.graph.fulfills[0];
        assert_eq!(row.provenance, PROVENANCE_FULFILLMENT);
        assert_eq!(row.justification.text.as_deref(), Some("that it might be fulfilled which was spoken of the Lord by the prophet, saying, Behold, a virgin shall be with child"));
        assert_eq!(row.justification.grounds.len(), 1);
        match row.justification.grounds.iter().next().unwrap() {
            Ground::Scripture(range) => assert_eq!(*range, row.fulfillment, "the fulfillment passage self-attests as its own ground"),
            other => panic!("expected Ground::Scripture, got {other:?}"),
        }
    }

    #[test]
    fn normalize_omits_a_fulfillment_row_with_an_unparseable_locus() {
        let atlas = atlas_with(vec![FulfillmentSeed { prophecy: g("not-a-verse", None), fulfillment: g("MAT.1.22", None), text: "x".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.fulfillment_rows, 0);
        assert_eq!(stats.fulfillment_omitted.len(), 1);
        assert_eq!(stats.fulfillment_omitted[0].0, 0);
        assert!(ctx.graph.fulfills.is_empty());
    }

    // --- normalize: typology --------------------------------------------

    #[test]
    fn normalize_builds_one_typology_row_per_curated_seed() {
        let atlas = atlas_with(
            vec![],
            vec![TypologySeed {
                type_passage: g("NUM.21.8", Some("NUM.21.9")),
                antitype_passage: g("JHN.3.14", Some("JHN.3.15")),
                note: "the brasen serpent".into(),
                text: "And as Moses lifted up the serpent in the wilderness, even so must the Son of man be lifted up".into(),
            }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.typology_rows, 1);
        assert!(stats.typology_omitted.is_empty());

        let row = &ctx.graph.typology[0];
        assert_eq!(row.provenance, PROVENANCE_TYPOLOGY);
        assert_eq!(row.note.as_deref(), Some("the brasen serpent"));
        assert_eq!(row.justification.grounds.len(), 1);
        match row.justification.grounds.iter().next().unwrap() {
            Ground::Scripture(range) => assert_eq!(*range, row.antitype_passage, "the antitype passage self-attests as its own ground"),
            other => panic!("expected Ground::Scripture, got {other:?}"),
        }
    }

    #[test]
    fn normalize_omits_a_typology_row_with_an_inverted_range() {
        let atlas = atlas_with(vec![], vec![TypologySeed { type_passage: g("JHN.3.15", Some("JHN.3.14")), antitype_passage: g("NUM.21.8", None), note: "x".into(), text: "x".into() }]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.typology_rows, 0);
        assert_eq!(stats.typology_omitted.len(), 1);
    }

    #[test]
    fn a_fulfillment_and_a_typology_row_can_share_the_same_locus_pair() {
        // Brief decision 1b, verbatim: "Where a case is BOTH an explicit
        // fulfillment formula and a type, it may appear in both tables --
        // they are different claims." (the real EXO.12.46 -> JHN.19.36
        // case: the passover lamb, both fulfilled-in AND prefigures).
        let atlas = atlas_with(
            vec![FulfillmentSeed { prophecy: g("EXO.12.46", None), fulfillment: g("JHN.19.36", None), text: "x".into() }],
            vec![TypologySeed { type_passage: g("EXO.12.46", None), antitype_passage: g("JHN.19.36", None), note: "the passover lamb".into(), text: "y".into() }],
        );
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        let stats = normalize(&mut ctx);
        assert_eq!(stats.fulfillment_rows, 1);
        assert_eq!(stats.typology_rows, 1);
    }

    // --- grounding laws ---------------------------------------------------

    #[test]
    fn every_fulfillment_row_has_a_scripture_ground_is_green_when_true() {
        let atlas = atlas_with(vec![FulfillmentSeed { prophecy: g("ISA.7.14", None), fulfillment: g("MAT.1.22", None), text: "x".into() }], vec![]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        assert!(every_fulfillment_row_has_a_scripture_ground(&ctx.graph).is_ok());
    }

    #[test]
    fn every_fulfillment_row_has_a_scripture_ground_catches_an_empty_grounds_row() {
        let mut graph = atlas_graph_types::graph::Graph::default();
        graph.fulfills.push(Fulfills {
            prophecy: ground_range(&g("ISA.7.14", None)).unwrap(),
            fulfillment: ground_range(&g("MAT.1.22", None)).unwrap(),
            provenance: "test".into(),
            justification: Justification::default(),
        });
        let err = every_fulfillment_row_has_a_scripture_ground(&graph).expect_err("must catch a row with zero grounds");
        assert!(err.0.contains("fulfills"), "{}", err.0);
    }

    #[test]
    fn every_typology_row_has_a_scripture_ground_is_green_when_true() {
        let atlas = atlas_with(vec![], vec![TypologySeed { type_passage: g("JON.1.17", None), antitype_passage: g("MAT.12.40", None), note: "Jonah".into(), text: "x".into() }]);
        let canon = Canon { books: vec![] };
        let verses = HashMap::new();
        let mut ctx = ctx_with(&canon, &verses, &atlas);
        normalize(&mut ctx);
        assert!(every_typology_row_has_a_scripture_ground(&ctx.graph).is_ok());
    }

    #[test]
    fn every_typology_row_has_a_scripture_ground_catches_a_non_scripture_only_row() {
        let mut graph = atlas_graph_types::graph::Graph::default();
        graph.typology.push(Typology {
            type_passage: ground_range(&g("JON.1.17", None)).unwrap(),
            antitype_passage: ground_range(&g("MAT.12.40", None)).unwrap(),
            note: Some("Jonah".into()),
            provenance: "test".into(),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Source(atlas_graph_types::id::SourceId::new("some-source"))]) },
        });
        let err = every_typology_row_has_a_scripture_ground(&graph).expect_err("a Source-only ground must not satisfy this law");
        assert!(err.0.contains("Jonah"), "{}", err.0);
    }
}
