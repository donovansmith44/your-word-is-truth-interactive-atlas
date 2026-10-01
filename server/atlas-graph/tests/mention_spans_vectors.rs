mod common;

use atlas_graph::mention_spans::{scan, Name, NameSegment};
use atlas_graph_types::edge::MentionedEntity;
use atlas_graph_types::id::{PersonId, PlaceId};
use serde_json::Value;

const MENTION_SPANS_VECTORS: &str = "mention-spans.json";

#[test]
fn the_ported_name_search_agrees_with_every_recorded_case() {
    // Arrange
    let vectors: Value = serde_json::from_str(
        &std::fs::read_to_string(common::contract_vectors_dir().join(MENTION_SPANS_VECTORS)).expect("the mention-spans vectors are committed"),
    )
    .expect("the vectors are JSON");
    let cases = vectors["cases"].as_array().expect("the vectors list their cases");
    let expected: Vec<(&str, Vec<NameSegment>)> = cases.iter().map(|case| (name_of(case), segments(&case["mentions"]))).collect();
    // Act
    let scanned: Vec<(&str, Vec<NameSegment>)> = cases.iter().map(|case| (name_of(case), scan(text_of(case), &names(case)))).collect();
    // Assert
    assert_eq!(scanned, expected);
}

fn name_of(case: &Value) -> &str {
    case["name"].as_str().expect("a case is named")
}

fn text_of(case: &Value) -> &str {
    case["text"].as_str().expect("a case carries its text")
}

fn names(case: &Value) -> Vec<Name> {
    let named = |list: &str, kind: &str| -> Vec<Name> {
        case[list]
            .as_array()
            .expect("a case lists its places and its persons")
            .iter()
            .map(|n| Name { entity: entity(kind, &n["id"]), text: n["name"].as_str().expect("a name has its text").to_string() })
            .collect()
    };
    named("places", "place").into_iter().chain(named("persons", "person")).collect()
}

fn segments(mentions: &Value) -> Vec<NameSegment> {
    let offset = |m: &Value, key: &str| usize::try_from(m[key].as_u64().expect("a mention names its character range")).expect("an offset fits a usize");
    mentions
        .as_array()
        .expect("a case lists the mentions it finds")
        .iter()
        .map(|m| NameSegment { chars: offset(m, "start")..offset(m, "end"), entity: entity(m["kind"].as_str().expect("a mention names its kind"), &m["id"]) })
        .collect()
}

fn entity(kind: &str, id: &Value) -> MentionedEntity {
    let id = id.as_str().expect("an entity has its id").to_string();
    match kind {
        "place" => MentionedEntity::Place(PlaceId::new(id)),
        "person" => MentionedEntity::Person(PersonId::new(id)),
        other => panic!("the vectors name only places and persons, not {other}"),
    }
}
