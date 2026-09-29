mod common;

use atlas_core::data::AtlasData;
use atlas_core::refs::VerseId;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::EventId;
use atlas_graph_types::text::VerseRef;

#[test]
fn every_place_date_claim_that_names_an_event_names_one_attested_in_its_own_verses() {
    // Arrange
    let (atlas, graph) = real_atlas_and_graph();
    let claims = atlas.place_history.values().flat_map(|h| [h.established.as_ref(), h.destroyed.as_ref()]).flatten();
    // Act
    let unattested: Vec<EventId> = claims
        .filter_map(|c| c.event.as_ref().map(|e| (e, &c.verses)))
        .filter(|(event, verses)| !verses.iter().any(|v| event_is_attested_in(&graph, event, v)))
        .map(|(event, _)| event.clone())
        .collect();
    // Assert
    assert_eq!(unattested, Vec::<EventId>::new());
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Claim {
    Established,
    Destroyed,
}

#[derive(Debug, PartialEq)]
struct ClaimedEvent {
    place: String,
    claim: Claim,
    event: Option<EventId>,
}

#[test]
fn the_curated_place_date_claims_name_these_events() {
    // Arrange
    let atlas = common::real_atlas();
    let claimed = |place: &str, claim: Claim, event: Option<&str>| ClaimedEvent { place: place.to_string(), claim, event: event.map(EventId::new) };
    // Act
    let mut named: Vec<ClaimedEvent> = atlas
        .place_history
        .values()
        .flat_map(|h| [(Claim::Established, h.established.as_ref()), (Claim::Destroyed, h.destroyed.as_ref())].into_iter().filter_map(move |(claim, c)| c.map(|c| claimed(&h.id, claim, c.event.as_ref().map(|e| e.0.as_str())))))
        .collect();
    named.sort_by(|a, b| a.place.cmp(&b.place).then(a.claim.cmp(&b.claim)));
    // Assert
    assert_eq!(
        named,
        vec![
            claimed("jerusalem", Claim::Established, Some("sam2_jerusalem_captured")),
            claimed("jerusalem", Claim::Destroyed, Some("exl_jerusalem")),
            claimed("nineveh", Claim::Established, Some("theo-87")),
            claimed("samaria_1022", Claim::Established, Some("theo-176")),
            claimed("samaria_1022", Claim::Destroyed, Some("2ki_fall_of_samaria")),
            claimed("shiloh", Claim::Established, Some("cq_shiloh")),
            claimed("shiloh", Claim::Destroyed, None),
        ]
    );
}

fn real_atlas_and_graph() -> (&'static AtlasData, Graph) {
    (common::real_atlas(), common::kjv_and_atlas_build(&[]).0)
}

fn event_is_attested_in(graph: &Graph, event: &EventId, verse: &str) -> bool {
    let id = VerseId::parse_canonical(verse).expect("a curated verse is canonical");
    let verse = VerseRef { book: id.book.0, chapter: id.chapter, verse: id.verse };
    graph.attests.iter().any(|row| row.event == *event && row.attestation.from.unit <= verse && verse <= row.attestation.to.unit)
}
