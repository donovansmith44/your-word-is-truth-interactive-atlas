//! Red-letter `SpokenBy` rows, one per maximal contiguous verse range, plus the `SpokenAt` rows
//! derived from them: a range fully inside an event's attested span takes that event's places, deduped
//! by distinct place. Reading `attests`/`located_at` means this must run after `event_world`.

use std::collections::{BTreeMap, BTreeSet};

use atlas_graph_types::edge::{Ground, Justification, SpokenAt, SpokenBy};
use atlas_graph_types::id::PersonId;
use atlas_graph_types::text::{BibleLocusRange, Locus, VerseRef};

use crate::pipeline::BuildCtx;

/// The Theographic person id for Jesus Christ. The source carries TWO records named "Jesus": this one
/// has the surname "Christ", 1,831 verses and an Easton's "Christ" link, while the other is a
/// one-verse record for "Jesus, who is called Justus" that the source itself flags as ambiguous.
pub const JESUS_PERSON_ID: &str = "jesus_905";

const SPOKEN_BY_PROVENANCE: &str = "red-letter";

#[derive(Debug, Clone, Copy, Default)]
pub struct RedLetterAdapterStats {
    pub spoken_by_rows: usize,
    pub spoken_at_rows: usize,
    /// The denominator of the coverage disclosure: every SpokenBy range built, whether or not it
    /// resolved to a place.
    pub spoken_at_ranges_total: usize,
    /// Numerator: SpokenBy ranges that resolved to >=1 place.
    pub spoken_at_ranges_covered: usize,
}

/// The canon-order key both the containment check and the event-span reconstruction below share.
fn verse_key(v: &VerseRef) -> (u8, u16, u16) {
    (v.book, v.chapter, v.verse)
}

fn locus_range(from: (u8, u16, u16), to: (u8, u16, u16)) -> Option<BibleLocusRange> {
    let f = Locus::whole(VerseRef { book: from.0, chapter: from.1, verse: from.2 });
    let t = Locus::whole(VerseRef { book: to.0, chapter: to.1, verse: to.2 });
    BibleLocusRange::new(f, t).ok()
}

pub fn normalize(ctx: &mut BuildCtx) -> RedLetterAdapterStats {
    let mut stats = RedLetterAdapterStats::default();
    let Some(corpus) = ctx.red_letter else {
        return stats;
    };

    let speaker = PersonId::new(JESUS_PERSON_ID.to_string());
    // This adapter's rows reference a node `person_adapter::normalize` builds, a coupling no sibling
    // adapter has. Without that node, no rows at all: a degraded fixture can vendor red-letter data
    // without people data, and not authoring is a better failure than a dangling reference.
    if !ctx.graph.nodes.contains_key(&speaker.clone().erase()) {
        return stats;
    }

    let canon = ctx.kjv_canon;
    let counts = |book: u8, chapter: u16| canon.verses_in(atlas_core::refs::BookId(book), chapter);
    let ranges = atlas_etl::red_letter::contiguous_ranges(&corpus.verses, &counts);

    // Built ONCE rather than per range; `located_at` is already fully populated earlier in this pass.
    let mut located_at_by_event: BTreeMap<String, Vec<atlas_graph_types::id::PlaceId>> = BTreeMap::new();
    for loc in &ctx.graph.located_at {
        located_at_by_event.entry(loc.event.0.clone()).or_default().push(loc.place.clone());
    }

    // An event's OVERALL attested span, reconstructed as the min..max verse across its `attests` rows:
    // one row is emitted per witnessed VERSE, so a multi-verse event's attestation is the union of
    // many single-verse rows and never one row already spanning it.
    struct EventSpan {
        min: (u8, u16, u16),
        max: (u8, u16, u16),
        provenance: String,
    }
    let mut event_spans: BTreeMap<String, EventSpan> = BTreeMap::new();
    for att in &ctx.graph.attests {
        let f = verse_key(&att.attestation.from.unit);
        let t = verse_key(&att.attestation.to.unit);
        event_spans
            .entry(att.event.0.clone())
            .and_modify(|s| {
                if f < s.min {
                    s.min = f;
                }
                if t > s.max {
                    s.max = t;
                }
            })
            .or_insert_with(|| EventSpan { min: f, max: t, provenance: att.provenance.clone() });
    }

    for (from, to) in ranges {
        let Some(range) = locus_range(from, to) else { continue };
        let range_from = verse_key(&range.from.unit);
        let range_to = verse_key(&range.to.unit);

        let mut spoken_by_grounds = BTreeSet::new();
        spoken_by_grounds.insert(Ground::Scripture(range.clone()));
        ctx.graph.spoken_by.push(SpokenBy {
            locus: range.clone(),
            speaker: speaker.clone(),
            provenance: SPOKEN_BY_PROVENANCE.to_string(),
            justification: Justification { text: None, grounds: spoken_by_grounds },
        });
        stats.spoken_by_rows += 1;

        stats.spoken_at_ranges_total += 1;
        let mut places_seen: BTreeSet<String> = BTreeSet::new();
        for (event_id, span) in &event_spans {
            let Some(places) = located_at_by_event.get(event_id) else { continue };
            if !(range_from >= span.min && range_to <= span.max) {
                continue;
            }
            let Some(attested_range) = locus_range(span.min, span.max) else { continue };
            for place in places {
                if !places_seen.insert(place.0.clone()) {
                    continue;
                }
                let mut grounds = BTreeSet::new();
                grounds.insert(Ground::Scripture(attested_range.clone()));
                grounds.insert(Ground::Scripture(range.clone()));
                ctx.graph.spoken_at.push(SpokenAt {
                    locus: range.clone(),
                    place: place.clone(),
                    provenance: span.provenance.clone(),
                    justification: Justification { text: Some(format!("derived: falls within {event_id}'s own attested range")), grounds },
                });
                stats.spoken_at_rows += 1;
            }
        }
        if !places_seen.is_empty() {
            stats.spoken_at_ranges_covered += 1;
        }
    }

    stats
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{AtlasData, Canon, CanonBook, Event, Place};
    use atlas_etl::red_letter::{RedLetterCorpus, RedLetterVerse};
    use std::collections::HashMap;

    fn canon_matthew() -> Canon {
        let mut chapters = vec![25u16; 28];
        chapters[3] = 25;
        chapters[4] = 48;
        Canon { books: vec![CanonBook { code: "MAT".into(), name: "Matthew".into(), chapters }] }
    }

    fn corpus_with(verses: Vec<RedLetterVerse>) -> RedLetterCorpus {
        RedLetterCorpus { verses, stats: Default::default() }
    }

    fn rv(chapter: u16, verse: u16, span_len: usize) -> RedLetterVerse {
        RedLetterVerse { book_index: 39, chapter, verse, spans: vec![(0, span_len)] }
    }

    fn insert_jesus_node(ctx: &mut BuildCtx) {
        use atlas_graph_types::node::{Node, NodePayload};
        let id = PersonId::new(JESUS_PERSON_ID.to_string()).erase();
        ctx.graph.nodes.insert(
            id.clone(),
            Node { id, payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() },
        );
    }

    fn ctx_with<'a>(canon: &'a Canon, verses: &'a HashMap<String, String>, atlas: &'a AtlasData, corpus: &'a RedLetterCorpus) -> BuildCtx<'a> {
        let mut ctx = BuildCtx::new(canon, verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", atlas);
        ctx.red_letter = Some(corpus);
        insert_jesus_node(&mut ctx);
        ctx
    }

    #[test]
    fn absent_red_letter_bundle_is_a_true_no_op() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        assert!(ctx.red_letter.is_none());
        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 0);
        assert!(ctx.graph.spoken_by.is_empty());
        assert!(ctx.graph.spoken_at.is_empty());
    }

    #[test]
    fn no_jesus_person_node_means_no_rows_at_all_never_a_dangling_reference() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = BuildCtx::new(&canon, &verses, None, "From Verse\tTo Verse\tVotes\t#comment\n", &atlas);
        ctx.red_letter = Some(&corpus);
        assert!(ctx.graph.nodes.is_empty(), "sanity: no Jesus node (or any node) exists in this fixture");

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 0);
        assert_eq!(stats.spoken_at_rows, 0);
        assert!(ctx.graph.spoken_by.is_empty());
        assert!(ctx.graph.spoken_at.is_empty());
    }

    #[test]
    fn normalize_builds_one_spoken_by_row_per_maximal_contiguous_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = crate::event_world::empty_atlas();
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10), rv(5, 10, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 2, "ranges keep the table honest to discourse shape -- not one row per verse");
        assert_eq!(ctx.graph.spoken_by.len(), 2);

        let first = &ctx.graph.spoken_by[0];
        assert_eq!(first.locus.from.unit, VerseRef { book: 39, chapter: 5, verse: 3 });
        assert_eq!(first.locus.to.unit, VerseRef { book: 39, chapter: 5, verse: 4 });
        assert_eq!(first.speaker.0, JESUS_PERSON_ID);
        assert_eq!(first.provenance, SPOKEN_BY_PROVENANCE);
        assert!(first.justification.grounds.contains(&Ground::Scripture(first.locus.clone())), "SpokenBy's own justification grounds in its own locus (decision 3)");

        assert!(ctx.graph.spoken_at.is_empty(), "no Event/LocatedAt data in this fixture -- zero SpokenAt rows, honestly, never fabricated");
    }

    fn atlas_with_located_event(event_id: &str, place_id: &str, verses: &[&str]) -> AtlasData {
        let places = vec![Place { id: place_id.into(), name: "Test Place".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: event_id.into(),
            label: "Test Event".into(),
            when: atlas_core::time::TimeRange::new(-5, 33).unwrap(),
            places: vec![place_id.into()],
            verses: verses.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }];
        AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish()
    }

    #[test]
    fn spoken_at_derives_a_place_when_the_range_falls_inside_a_located_events_attested_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = atlas_with_located_event("sermon-event", "mountain", &["MAT.5.1", "MAT.5.2", "MAT.5.3", "MAT.5.4", "MAT.5.5"]);
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 1);
        assert_eq!(stats.spoken_at_ranges_covered, 1);
        assert_eq!(stats.spoken_at_ranges_total, 1);

        let row = &ctx.graph.spoken_at[0];
        assert_eq!(row.place.0, "mountain");
        assert!(row.justification.text.as_deref().unwrap_or("").contains("sermon-event"));
    }

    #[test]
    fn spoken_at_is_honestly_empty_when_no_located_event_contains_the_range() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = atlas_with_located_event("unrelated-event", "somewhere", &["MAT.4.1", "MAT.4.2"]);
        let corpus = corpus_with(vec![rv(4, 19, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 0);
        assert_eq!(stats.spoken_at_ranges_covered, 0);
        assert_eq!(stats.spoken_at_ranges_total, 1, "the range still counts toward the coverage denominator, honestly, even with zero hits");
    }

    #[test]
    fn spoken_at_dedupes_two_events_that_resolve_to_the_same_place_into_one_row() {
        let canon = canon_matthew();
        let verses = HashMap::new();
        let places = vec![Place { id: "mountain".into(), name: "Test Place".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![
            Event { id: "wide-event".into(), label: "Wide".into(), when: atlas_core::time::TimeRange::new(-5, 33).unwrap(), places: vec!["mountain".into()], verses: vec!["MAT.5.1".into(), "MAT.5.2".into(), "MAT.5.3".into(), "MAT.5.4".into(), "MAT.5.5".into()], ..Default::default() },
            Event { id: "narrow-event".into(), label: "Narrow".into(), when: atlas_core::time::TimeRange::new(-5, 33).unwrap(), places: vec!["mountain".into()], verses: vec!["MAT.5.3".into(), "MAT.5.4".into()], ..Default::default() },
        ];
        let atlas = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish();
        let corpus = corpus_with(vec![rv(5, 3, 10), rv(5, 4, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);

        let stats = normalize(&mut ctx);
        assert_eq!(stats.spoken_by_rows, 1);
        assert_eq!(stats.spoken_at_rows, 1, "both events resolve to the SAME place -- one row, not two identical ones (decision 3)");
    }

    #[test]
    fn spoken_by_and_spoken_at_rows_lower_into_the_directed_index_both_ways() {
        use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
        use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
        use atlas_graph_types::id::{NodeKind as NK, Position};
        use atlas_graph_types::node::{Node, NodePayload};

        let canon = canon_matthew();
        let verses = HashMap::new();
        let atlas = atlas_with_located_event("sermon-event", "mountain", &["MAT.5.1", "MAT.5.2", "MAT.5.3", "MAT.5.4"]);
        let corpus = corpus_with(vec![rv(5, 3, 10)]);
        let mut ctx = ctx_with(&canon, &verses, &atlas, &corpus);
        crate::event_world::normalize(&mut ctx);
        normalize(&mut ctx);

        let verse_id = atlas_graph_types::id::AnyNodeId { kind: NK::TextUnit, raw: "bible/39.5.3".into() };
        ctx.graph.nodes.insert(
            verse_id.clone(),
            Node { id: verse_id.clone(), payload: NodePayload::TextUnit { corpus: "bible", renderings: Default::default() }, provenance: "test".into() },
        );
        let jesus_id = PersonId::new(JESUS_PERSON_ID.to_string()).erase();
        ctx.graph.nodes.insert(
            jesus_id.clone(),
            Node { id: jesus_id.clone(), payload: NodePayload::Person { label: "Jesus".into(), gender: None, birth_year: None, death_year: None, also_called: vec![], description: None, first_year: None, last_year: None, eternal: false, eternal_grounds: vec![] }, provenance: "test".into() },
        );
        let place_id = atlas_graph_types::id::PlaceId::new("mountain").erase();
        ctx.graph.nodes.insert(place_id.clone(), Node { id: place_id.clone(), payload: NodePayload::Place { canonical: "Mountain".into(), lat: 0.0, lon: 0.0, aliases: vec![], description: None }, provenance: "test".into() });

        ctx.graph.build_indexes();

        let forward_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Forward);
        let page = PositionRef(Position::Node(verse_id.clone())).edges(&ctx.graph, &EdgeQuery { kind: forward_by, cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1, "the verse's own forward 'spoken-by' frontier reaches Jesus");
        assert_eq!(page.entries[0].node, Position::Node(jesus_id.clone()));

        let inverse_by = EdgeKind::Directed(RelationId::SpokenBy, Direction::Inverse);
        let back = PositionRef(Position::Node(jesus_id)).edges(&ctx.graph, &EdgeQuery { kind: inverse_by, cursor: None, limit: 10 });
        assert_eq!(back.entries.len(), 1, "Jesus's own inverse 'speech-of' frontier lists the verse back");
        assert_eq!(back.entries[0].edge, page.entries[0].edge, "the SAME edge id, from either end -- the bijection witness");

        let forward_at = EdgeKind::Directed(RelationId::SpokenAt, Direction::Forward);
        let at_page = PositionRef(Position::Node(verse_id)).edges(&ctx.graph, &EdgeQuery { kind: forward_at, cursor: None, limit: 10 });
        assert_eq!(at_page.entries.len(), 1, "the verse's own forward 'spoken-at' frontier reaches the mountain");
        assert_eq!(at_page.entries[0].node, Position::Node(place_id));
    }
}
