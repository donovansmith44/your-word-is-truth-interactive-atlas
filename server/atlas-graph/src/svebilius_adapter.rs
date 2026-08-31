//! SVEB-1: the Svebilius adapter -- Olaus Svebilius' *Simple Exposition of
//! the Catechism* as a third text corpus, alongside the Bible and the Book
//! of Concord.
//!
//! SHAPE, and why it mirrors `concord_adapter` rather than inventing one:
//! NORMALIZE emits one `TextUnit` node per addressable unit plus the
//! "svebilius" reading spine and one `Container` per section -- exactly the
//! "nodes + spine, self-contained, no cross-adapter dependency" split that
//! adapter established. MERGE/ALIAS lowers the Scripture the Exposition
//! quotes into `Quotes` rows.
//!
//! WHY `Quotes` AND NOT `Confesses`: `Confesses` is typed to a
//! `ConcordLocus` and carries a deliberate asymmetry -- "a normed-norm
//! locus confesses what Scripture teaches" (its own doc comment). Svebilius
//! is not a confession; it is a teaching work ABOUT one, which is why its
//! corpus role is `Reference` and not `NormaNormata`. What its answers
//! actually do is QUOTE Scripture -- they print the verse text inline --
//! and `Quotes { quoting: TextLocus, quoted: BibleLocusRange }` takes a
//! generic source locus, so it fits without widening anything.
//!
//! DELIBERATELY DEFERRED, disclosed not forgotten: a curated question ->
//! `CatechismItem` alignment. `CatechismLink`'s own `locus` is a generic
//! `TextLocus`, so Svebilius units COULD link to Luther's items the same
//! way `concord_adapter` lowers `concord-sc-overlap.toml` -- but that file
//! is hand-verified row by row, and doing the same honestly for 314
//! questions is a curation pass with its own review, not a line of code.
//! The section-to-chief-part correspondence is obvious; the
//! question-to-item one is not (several questions bear on a whole part, and
//! two sections -- the Penitential Psalms and the Confession of Sins --
//! have no Luther counterpart at all). Left unaligned rather than
//! force-fit.

use std::collections::BTreeSet;

use atlas_graph_types::edge::{Contains, Quotes};
use atlas_graph_types::graph::ReadingSpine;
use atlas_graph_types::id::{AnyNodeId, ContainerNodeId, TextUnitId};
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{
    BibleLocus, LocusSet, SvebiliusLocus, SvebiliusRef, TextLocus, TranslationId, VerseRef,
};

use crate::pipeline::BuildCtx;

/// The corpus id carried on every `TextUnit` payload this adapter emits,
/// and the key of its reading spine.
pub const SVEBILIUS_CORPUS: &str = "svebilius";

/// The single rendering layer. One translation chain -- Svebilius' Swedish,
/// through Juslenius' Finnish (1745), through Kotti's 2007 edition, into
/// English -- so the layer names the edition, not a language.
pub const SVEBILIUS_TRANSLATION: &str = "svebilius-juslenius-en";

#[derive(Debug, Clone, Copy, Default)]
pub struct SvebiliusAdapterStats {
    pub sections: usize,
    pub units: usize,
    pub quote_rows: usize,
}

pub fn text_unit_id(section: u8, unit: u16) -> AnyNodeId {
    TextUnitId::new(format!("svebilius/{section}.{unit}")).erase()
}

/// The inverse of `text_unit_id` -- mirrors `concord_adapter::
/// decode_text_unit` exactly.
pub fn decode_text_unit(id: &atlas_graph_types::id::AnyNodeId) -> Option<(u8, u16)> {
    if id.kind != atlas_graph_types::id::NodeKind::TextUnit {
        return None;
    }
    let rest = id.raw.strip_prefix("svebilius/")?;
    let mut parts = rest.split('.');
    let section: u8 = parts.next()?.parse().ok()?;
    let unit: u16 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((section, unit))
}

fn section_container_id(section: u8) -> ContainerNodeId {
    ContainerNodeId::new(format!("svebilius-section/{section}"))
}

fn verse_locus(vref: &str) -> Option<BibleLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    Some(BibleLocus::whole(VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse }))
}

/// NORMALIZE: unit nodes, section containers, and the reading spine.
///
/// Absent `ctx.svebilius` is a true no-op -- the same "honestly empty, not
/// a placeholder" treatment every other optional bundle gets, so fixtures
/// that supply no Svebilius data build a graph without it rather than a
/// graph with empty scaffolding in it.
pub fn normalize(ctx: &mut BuildCtx) -> SvebiliusAdapterStats {
    let mut stats = SvebiliusAdapterStats::default();
    let Some(units) = ctx.svebilius else {
        return stats;
    };

    let mut order: Vec<AnyNodeId> = Vec::new();
    let mut by_section: Vec<(u8, BTreeSet<SvebiliusLocus>)> = Vec::new();

    for u in units {
        let id = text_unit_id(u.section, u.unit);
        let mut renderings = atlas_graph_types::text::LayerMap::new();
        // Question and answer are ONE unit's prose, joined -- the same "one
        // catechetical unit is one thing" call `catechism.toml` makes for
        // text+explanation and the Concord parser makes for its own
        // question/answer sub-lettering.
        let text = match &u.question {
            Some(q) => format!("{q} {}", u.answer),
            None => u.answer.clone(),
        };
        renderings.insert(TranslationId(SVEBILIUS_TRANSLATION.to_string()), text);
        ctx.graph.nodes.insert(
            id.clone(),
            Node {
                id: id.clone(),
                payload: NodePayload::TextUnit { corpus: SVEBILIUS_CORPUS, renderings },
                provenance: "svebilius".to_string(),
            },
        );
        order.push(id);
        stats.units += 1;

        let locus = SvebiliusLocus::whole(SvebiliusRef { section: u.section, unit: u.unit });
        match by_section.last_mut() {
            Some((sec, set)) if *sec == u.section => {
                set.insert(locus);
            }
            _ => {
                let mut set = BTreeSet::new();
                set.insert(locus);
                by_section.push((u.section, set));
            }
        }
    }

    for (section, content) in by_section {
        let container = section_container_id(section);
        let title = atlas_etl::svebilius::SECTIONS
            .get(section as usize - 1)
            .copied()
            .unwrap_or("Svebilius")
            .to_string();
        ctx.graph.nodes.insert(
            container.erase(),
            Node { id: container.erase(), payload: NodePayload::Container { title }, provenance: "svebilius".to_string() },
        );
        ctx.graph.contains_svebilius.push(Contains {
            container,
            content: LocusSet(content),
            provenance: ProvenanceId::from("svebilius"),
            justification: Default::default(),
        });
        stats.sections += 1;
    }

    ctx.graph.reading.insert(SVEBILIUS_CORPUS, ReadingSpine { order });
    stats
}

/// MERGE/ALIAS: one `Quotes` row per Scripture citation an answer carries.
///
/// Deduped by (unit, verse): an answer that prints the same verse twice
/// quotes it once. Across DIFFERENT units the same verse is a genuinely
/// separate quotation and keeps its own row.
pub fn merge_alias(ctx: &mut BuildCtx) -> SvebiliusAdapterStats {
    let mut stats = SvebiliusAdapterStats::default();
    let Some(units) = ctx.svebilius else {
        return stats;
    };

    for u in units {
        let quoting: TextLocus =
            SvebiliusLocus::whole(SvebiliusRef { section: u.section, unit: u.unit }).into();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for vref in &u.verses {
            if !seen.insert(vref.clone()) {
                continue;
            }
            let Some(locus) = verse_locus(vref) else { continue };
            let Ok(quoted) = atlas_graph_types::text::LocusRange::new(locus.clone(), locus) else { continue };
            ctx.graph.quotes.push(Quotes {
                quoting: quoting.clone(),
                quoted,
                provenance: ProvenanceId::from("svebilius"),
            });
            stats.quote_rows += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon};
    use atlas_etl::svebilius::SvebiliusUnit;
    use std::collections::HashMap;

    /// Mirrors `concord_adapter`'s own `ctx_with_concord` -- the real
    /// constructor, with just this adapter's units threaded in.
    fn ctx_with<'a>(
        canon: &'a Canon,
        verses: &'a HashMap<String, String>,
        atlas: &'a AtlasData,
        units: &'a [SvebiliusUnit],
    ) -> BuildCtx<'a> {
        let mut ctx = BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas);
        ctx.svebilius = Some(units);
        ctx
    }

    fn empty_ctx<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData) -> BuildCtx<'a> {
        BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas)
    }

    fn units() -> Vec<SvebiliusUnit> {
        vec![
            SvebiliusUnit {
                section: 1,
                unit: 1,
                question: Some("Are you a Christian?".into()),
                answer: "I am.".into(),
                verses: vec![],
            },
            SvebiliusUnit {
                section: 1,
                unit: 2,
                question: Some("Why are you called a Christian?".into()),
                answer: "Because I am baptized.".into(),
                // The same verse twice -- one row, not two.
                verses: vec!["GAL.3.27".into(), "GAL.3.27".into(), "ACT.4.12".into()],
            },
            SvebiliusUnit { section: 9, unit: 1, question: None, answer: "I, a poor sinner.".into(), verses: vec![] },
        ]
    }

    #[test]
    fn normalize_emits_units_sections_and_a_spine() {
        let (canon, verses, atlas) = (Canon::default(), HashMap::new(), AtlasData::default());
        let units = units();
        let mut ctx = ctx_with(&canon, &verses, &atlas, &units);
        let stats = normalize(&mut ctx);

        assert_eq!(stats.units, 3);
        // Two distinct sections in the fixture (1 and 9).
        assert_eq!(stats.sections, 2);
        let spine = ctx.graph.reading.get(SVEBILIUS_CORPUS).expect("spine");
        assert_eq!(spine.order.len(), 3);
        // Question and answer land as one unit's prose.
        match &ctx.graph.nodes[&text_unit_id(1, 1)].payload {
            NodePayload::TextUnit { corpus, renderings } => {
                assert_eq!(*corpus, SVEBILIUS_CORPUS);
                let t = renderings.get(&TranslationId(SVEBILIUS_TRANSLATION.to_string())).unwrap();
                assert_eq!(t, "Are you a Christian? I am.");
            }
            other => panic!("expected a TextUnit, got {other:?}"),
        }
    }

    #[test]
    fn a_prose_unit_carries_only_its_answer() {
        let (canon, verses, atlas) = (Canon::default(), HashMap::new(), AtlasData::default());
        let units = units();
        let mut ctx = ctx_with(&canon, &verses, &atlas, &units);
        normalize(&mut ctx);
        match &ctx.graph.nodes[&text_unit_id(9, 1)].payload {
            NodePayload::TextUnit { renderings, .. } => {
                let t = renderings.get(&TranslationId(SVEBILIUS_TRANSLATION.to_string())).unwrap();
                assert_eq!(t, "I, a poor sinner.");
            }
            other => panic!("expected a TextUnit, got {other:?}"),
        }
    }

    #[test]
    fn quotes_are_deduped_within_a_unit() {
        let (canon, verses, atlas) = (Canon::default(), HashMap::new(), AtlasData::default());
        let units = units();
        let mut ctx = ctx_with(&canon, &verses, &atlas, &units);
        let stats = merge_alias(&mut ctx);
        // GAL.3.27 appears twice in the fixture, ACT.4.12 once -> 2 rows.
        assert_eq!(stats.quote_rows, 2);
        assert_eq!(ctx.graph.quotes.len(), 2);
    }

    #[test]
    fn absent_svebilius_data_is_a_true_no_op() {
        let (canon, verses, atlas) = (Canon::default(), HashMap::new(), AtlasData::default());
        let mut ctx = empty_ctx(&canon, &verses, &atlas);
        assert_eq!(normalize(&mut ctx).units, 0);
        assert_eq!(merge_alias(&mut ctx).quote_rows, 0);
        assert!(ctx.graph.reading.get(SVEBILIUS_CORPUS).is_none());
        assert!(ctx.graph.contains_svebilius.is_empty());
    }
}
