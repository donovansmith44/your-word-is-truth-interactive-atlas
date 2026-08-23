//! Batch M-C, controller decision 2: the place adapter's own MERGE/ALIAS
//! half -- "places (incl. merge tables + KJV naming -> named rows)" and
//! "mentions (Theographic verse refs -> mentions rows, Place objects now
//! real not stubs)". Node construction (lat/lon + the KJV alias payload)
//! lives with the rest of event-world's own NORMALIZE work
//! (`event_world::place_node`, called from `populate_nodes_and_direct_rows`)
//! since it needs no other pass's output; THIS module builds the two
//! relation row tables that cross a legacy-vocabulary boundary, matching
//! the ingestion contract's own "merge/alias (event and place merge tables
//! become assertion-level rules)" naming (design doc §7).
//!
//! MERGE TABLES, disclosed: `atlas_core::merge::apply_place_merges` already
//! ran, ETL-side, before `AtlasData.places` (this adapter's own source, the
//! SAME `atlas.places` `event_world::place_node` reads) is ever populated
//! -- a place's own absorbed/superseded ids are folded into its survivor's
//! `verse_links` and event references BEFORE this adapter ever sees the
//! data (`AtlasData`'s own doc comment: "ETL builds one of these... before
//! writing it to disk"). There is no separate id-alias table to surface as
//! graph rows beyond that: an absorbed id names no node of its own (it was
//! never a node to begin with, even pre-graph), so "incl. merge tables" is
//! satisfied by correctly reading the ALREADY-merged `atlas.places` --
//! exactly what `place_node` already did since M-B, unmodified by this
//! batch's own addition of lat/lon/aliases to its payload.
//!
//! KJV NAMING -> `named` rows: `data/curated/place-names-kjv.toml`
//! (`AtlasData.place_name_aliases`) becomes one `Named` row per place that
//! has one -- content-addressed, authored data, alongside (not instead of)
//! the SAME alias riding on the node's own payload (`NodePayload::Place::
//! aliases`) for actual queryability (`graph.rs::build_indexes`'s own
//! disclosed note: a `Named` row's object is a bare string, un-indexable
//! through the generic Position-typed port).
//!
//! MENTIONS: `Place.verse_links` (canonical verse ids "attached by
//! geocoding," `atlas_core::data::Place`'s own doc comment) becomes one
//! `Mentions` row per link -- Theographic's own per-place verse geocoding
//! IS this data; nothing else in this workspace holds a separate
//! "Theographic verse ref" table distinct from what geocoding already
//! resolved onto `Place.verse_links`.

use atlas_graph_types::edge::{Mentions, Named, PlaceOrPerson};
use atlas_graph_types::id::PlaceId;
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Default)]
pub struct PlaceAdapterStats {
    pub named_rows: usize,
    pub mentions_rows: usize,
}

fn verse_locus(vref: &str) -> Option<TextLocus> {
    let vid = atlas_core::refs::VerseId::parse_canonical(vref).ok()?;
    let vr = VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse };
    Some(TextLocus::from(BibleLocus::whole(vr)))
}

/// Pipeline-facing MERGE/ALIAS entry point (`pipeline::MergeAliasPass`).
pub fn merge_alias(ctx: &mut BuildCtx) -> PlaceAdapterStats {
    let mut stats = PlaceAdapterStats::default();

    for p in &ctx.atlas.places {
        let place_id = PlaceId::new(p.id.clone());

        if let Some(alias) = ctx
            .atlas
            .place_name_alias_for(&p.id)
            .and_then(|a| a.translations.get(crate::kjv_adapter::KJV_TRANSLATION))
        {
            ctx.graph.named.push(Named {
                place: place_id.clone(),
                alias: alias.clone(),
                provenance: ProvenanceId::from("place-names-kjv"),
                justification: Default::default(),
            });
            stats.named_rows += 1;
        }

        for vref in &p.verse_links {
            let Some(locus) = verse_locus(vref) else { continue };
            ctx.graph.mentions.push(Mentions {
                locus,
                entity: PlaceOrPerson::Place(place_id.clone()),
                provenance: ProvenanceId::from("theographic-geocoding"),
            });
            stats.mentions_rows += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, Place, PlaceNameAlias};
    use std::collections::HashMap;

    fn atlas_with_place(place: Place, alias: Option<PlaceNameAlias>) -> AtlasData {
        let mut d = AtlasData::new(Canon { books: vec![] }, vec![place], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        if let Some(a) = alias {
            d.place_name_aliases.insert(a.id.clone(), a);
        }
        d
    }

    #[test]
    fn named_row_built_only_when_a_kjv_alias_exists() {
        let with_alias = atlas_with_place(
            Place { id: "cush-2".into(), name: "Cush".into(), lat: 0.0, lon: 0.0, verse_links: vec![] },
            Some(PlaceNameAlias {
                id: "cush-2".into(),
                translations: HashMap::from([("kjv".to_string(), "Ethiopia".to_string())]),
                verses: vec!["GEN.2.13".into()],
            }),
        );
        let without_alias = atlas_with_place(Place { id: "jericho".into(), name: "Jericho".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }, None);

        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &with_alias);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.named_rows, 1);
        assert_eq!(ctx.graph.named[0].alias, "Ethiopia");

        let mut ctx2 = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &without_alias);
        let stats2 = merge_alias(&mut ctx2);
        assert_eq!(stats2.named_rows, 0);
    }

    #[test]
    fn mentions_rows_built_one_per_verse_link() {
        let atlas = atlas_with_place(
            Place { id: "hebron".into(), name: "Hebron".into(), lat: 0.0, lon: 0.0, verse_links: vec!["GEN.13.18".into(), "GEN.23.19".into()] },
            None,
        );
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.mentions_rows, 2);
        for row in &ctx.graph.mentions {
            assert!(matches!(&row.entity, PlaceOrPerson::Place(p) if p.0 == "hebron"));
        }
    }

    #[test]
    fn an_unparseable_verse_link_is_skipped_not_panicked_on() {
        let atlas = atlas_with_place(
            Place { id: "x".into(), name: "X".into(), lat: 0.0, lon: 0.0, verse_links: vec!["not-a-verse".into()] },
            None,
        );
        let canon = Canon { books: vec![] };
        let verses: HashMap<String, String> = HashMap::new();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        let stats = merge_alias(&mut ctx);
        assert_eq!(stats.mentions_rows, 0);
    }
}
