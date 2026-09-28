//! The place adapter's merge/alias half: the `mentions` rows from `Place.verse_links`, the verse
//! links geocoding resolved. The place merges already ran ETL-side, before `AtlasData.places` was
//! populated, so an absorbed id names no node here and no id-alias table is left to surface.

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
            d.place_name_aliases.insert(a.id.clone(), vec![a]);
        }
        d
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
