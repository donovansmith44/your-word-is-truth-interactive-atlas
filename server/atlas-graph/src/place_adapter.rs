//! Batch M-C, controller decision 2: the place adapter's own MERGE/ALIAS
//! half -- originally "places (incl. merge tables + KJV naming -> named
//! rows)" and "mentions (Theographic verse refs -> mentions rows, Place
//! objects now real not stubs)". Node construction (lat/lon + the KJV alias
//! payload) lives with the rest of event-world's own NORMALIZE work
//! (`event_world::place_node`, called from `populate_nodes_and_direct_rows`)
//! since it needs no other pass's output; THIS module builds the relation
//! row table(s) that cross a legacy-vocabulary boundary, matching the
//! ingestion contract's own "merge/alias (event and place merge tables
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
//! KJV NAMING, RETIRED AS A ROW (M-D3, owner ruling R2): `data/curated/
//! place-names-kjv.toml` (`AtlasData.place_name_aliases`) used to ALSO
//! become one `Named` row per place that has one, alongside the SAME alias
//! already riding the node's own payload (`NodePayload::Place::aliases`).
//! The owner retired the `named` relation whole -- manifest row, `Named`
//! struct, `graph.named` table -- because a `Named` row's own object is a
//! bare alias string with no `Position` representation to index through the
//! generic port (`graph.rs::build_indexes`'s own disclosed note), so those
//! rows never lowered into `pairs` at all: the payload was ALREADY the sole
//! serving path, and this adapter's own push was the "second, weaker path"
//! the discipline forbids, not a genuine second source. This module now
//! builds ONLY the `mentions` row table below.
//!
//! MENTIONS: `Place.verse_links` (canonical verse ids "attached by
//! geocoding," `atlas_core::data::Place`'s own doc comment) becomes one
//! `Mentions` row per link -- Theographic's own per-place verse geocoding
//! IS this data; nothing else in this workspace holds a separate
//! "Theographic verse ref" table distinct from what geocoding already
//! resolved onto `Place.verse_links`.

use atlas_graph_types::edge::{Mentions, MentionedEntity};
use atlas_graph_types::id::PlaceId;
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::text::{BibleLocus, TextLocus, VerseRef};

use crate::pipeline::BuildCtx;

#[derive(Debug, Clone, Default)]
pub struct PlaceAdapterStats {
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

        for vref in &p.verse_links {
            let Some(locus) = verse_locus(vref) else { continue };
            ctx.graph.mentions.push(Mentions {
                locus,
                entity: MentionedEntity::Place(place_id.clone()),
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
            // Batch GAZ-1-R1: `place_name_aliases` is now `Vec`-valued per id.
            d.place_name_aliases.insert(a.id.clone(), vec![a]);
        }
        d
    }

    // M-D3 (owner ruling R2): `named_row_built_only_when_a_kjv_alias_exists`
    // deleted alongside the `named`-row push it tested -- see this file's
    // own module doc comment for the retirement. `atlas_with_place`'s
    // `alias` parameter stays (still exercised by real callers of THIS
    // helper elsewhere in this file's own test module -- none currently do,
    // but it also costs nothing dead: it is a plain constructor argument,
    // not a law needing its own coverage).

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
            assert!(matches!(&row.entity, MentionedEntity::Place(p) if p.0 == "hebron"));
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
