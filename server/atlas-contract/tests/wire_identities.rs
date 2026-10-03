use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

const COMPONENTS: &str = "#/components/schemas/";
const RESPONSE: &str = "response";
const OK: &str = "200";
const JSON_BODY: &str = "application/json";
const ROOT: &str = "$";
const DISCRIMINATOR: &str = "discriminator";
const COMBINATORS: [&str; 3] = ["oneOf", "anyOf", "allOf"];

const ARTIFACT_ROOT: &str = "ArtifactRoot";
const EDGE_PAGE_CURSOR: &str = "EdgePageCursor";
const ELEMENT_PAGE_CURSOR: &str = "ElementPageCursor";
const NODE_ID: &str = "NodeId";
const EDGE_ID: &str = "EdgeId";
const ELEMENT_ID: &str = "ElementId";
const VERSE_REFERENCE: &str = "VerseReference";
const CHAPTER_REFERENCE: &str = "ChapterReference";
const BIBLE_REFERENCE: &str = "BibleReference";
const VERSE_SPAN_REFERENCE: &str = "VerseSpanReference";
const CROSS_REFERENCE_TARGET: &str = "CrossReferenceTarget";
const UNIT_REFERENCE: &str = "UnitReference";
const CONTENTS_REFERENCE: &str = "ContentsReference";
const TEXT_WINDOW_REFERENCE: &str = "TextWindowReference";
const CORPUS: &str = "Corpus";

type Site = (String, String, String);

fn site(operation: &str, location: &str, path: &str) -> Site {
    (operation.to_string(), location.to_string(), path.to_string())
}

fn sites(entries: &[(&str, &str, &str)]) -> BTreeSet<Site> {
    entries.iter().map(|(operation, location, path)| site(operation, location, path)).collect()
}

fn the_published_inventory() -> BTreeMap<&'static str, BTreeSet<Site>> {
    BTreeMap::from([
        (
            ARTIFACT_ROOT,
            sites(&[
                ("contents", RESPONSE, "$.version"),
                ("elements", RESPONSE, "$.elements[].node.version"),
                ("elements", RESPONSE, "$.version"),
                ("kretzmann_chapter", RESPONSE, "$.version"),
                ("node_edges", RESPONSE, "$.version"),
                ("node_record", RESPONSE, "$.version"),
                ("text_window", RESPONSE, "$.version"),
            ]),
        ),
        (EDGE_PAGE_CURSOR, sites(&[("node_edges", "query cursor", ROOT), ("node_edges", RESPONSE, "$.next"), ("node_edges", RESPONSE, "$.previous")])),
        (ELEMENT_PAGE_CURSOR, sites(&[("elements", "query cursor", ROOT), ("elements", RESPONSE, "$.next"), ("elements", RESPONSE, "$.previous")])),
        (
            NODE_ID,
            sites(&[
                ("chapter", RESPONSE, "$.verses[].places[].node.id"),
                ("contents", RESPONSE, "$.roots[].children[].id"),
                ("contents", RESPONSE, "$.roots[].id"),
                ("elements", RESPONSE, "$.elements[].edge.object.node.id"),
                ("elements", RESPONSE, "$.elements[].edge.subject.node.id"),
                ("elements", RESPONSE, "$.elements[].node.book.write_place.id"),
                ("elements", RESPONSE, "$.elements[].node.id"),
                ("elements", RESPONSE, "$.elements[].node.place.destroyed.event.id"),
                ("elements", RESPONSE, "$.elements[].node.place.established.event.id"),
                ("elements", RESPONSE, "$.elements[].node.text.anchors[].node.id"),
                ("eras", RESPONSE, "$[].node.id"),
                ("event", RESPONSE, "$.places[].node.id"),
                ("kretzmann_chapter", RESPONSE, "$.verses[].items[].id"),
                ("node_edges", RESPONSE, "$.entries[].neighbour.node.id"),
                ("node_record", "path id", ROOT),
                ("node_record", RESPONSE, "$.book.write_place.id"),
                ("node_record", RESPONSE, "$.id"),
                ("node_record", RESPONSE, "$.place.destroyed.event.id"),
                ("node_record", RESPONSE, "$.place.established.event.id"),
                ("node_record", RESPONSE, "$.text.anchors[].node.id"),
                ("polities", RESPONSE, "$.polities[].node.id"),
                ("scene_scripture", RESPONSE, "$.places[].node.id"),
                ("scene_scripture", RESPONSE, "$.quiet_places[].node.id"),
                ("scene_time", RESPONSE, "$.places[].node.id"),
                ("scene_time", RESPONSE, "$.quiet_places[].node.id"),
                ("text_window", RESPONSE, "$.units[].body.anchors[].node.id"),
                ("text_window", RESPONSE, "$.units[].heading.event.id"),
                ("text_window", RESPONSE, "$.units[].node.id"),
            ]),
        ),
        (
            EDGE_ID,
            sites(&[
                ("elements", RESPONSE, "$.elements[].edge.id"),
                ("elements", RESPONSE, "$.elements[].edge.object.edge.id"),
                ("elements", RESPONSE, "$.elements[].edge.subject.edge.id"),
                ("node_edges", RESPONSE, "$.entries[].edge.id"),
                ("node_edges", RESPONSE, "$.entries[].neighbour.edge.id"),
            ]),
        ),
        (ELEMENT_ID, sites(&[("elements", "query ids", "$[]"), ("elements", RESPONSE, "$.elements[].id"), ("node_edges", "path id", ROOT)])),
        (
            VERSE_REFERENCE,
            sites(&[
                ("catechism_item", RESPONSE, "$.verses[].vref"),
                ("event", RESPONSE, "$.mentioned_in[]"),
                ("event", RESPONSE, "$.witnesses[].verse_groups[].verses[]"),
                ("narrative_event_positions", RESPONSE, "$.narrative[].following.verse_groups[].verses[]"),
                ("narrative_event_positions", RESPONSE, "$.narrative[].prior.verse_groups[].verses[]"),
                ("narrative_event_positions", RESPONSE, "$.timeline.following.verse_groups[].verses[]"),
                ("narrative_event_positions", RESPONSE, "$.timeline.prior.verse_groups[].verses[]"),
                ("polities", RESPONSE, "$.polities[].fall.verses[]"),
                ("polities", RESPONSE, "$.polities[].transition.verses[]"),
                ("scene_scripture", RESPONSE, "$.places[].events[].verse_groups[].verses[]"),
                ("scene_time", RESPONSE, "$.places[].events[].verse_groups[].verses[]"),
            ]),
        ),
        (CHAPTER_REFERENCE, sites(&[("chapter", "path cref", ROOT), ("chapter", RESPONSE, "$.ref"), ("kretzmann_chapter", "path cref", ROOT)])),
        (BIBLE_REFERENCE, sites(&[("scene_scripture", "query ref", ROOT), ("scene_scripture", RESPONSE, "$.ref"), ("scene_time", RESPONSE, "$.ref")])),
        (VERSE_SPAN_REFERENCE, sites(&[("catechism_for_span", "path sref", ROOT), ("xrefs", "path sref", ROOT)])),
        (CROSS_REFERENCE_TARGET, sites(&[("xrefs", RESPONSE, "$[].target")])),
        (UNIT_REFERENCE, sites(&[("text_window", RESPONSE, "$.next"), ("text_window", RESPONSE, "$.units[].ref")])),
        (CONTENTS_REFERENCE, sites(&[("contents", RESPONSE, "$.roots[].children[].ref"), ("contents", RESPONSE, "$.roots[].ref")])),
        (TEXT_WINDOW_REFERENCE, sites(&[("text_window", "query ref", ROOT)])),
        (CORPUS, sites(&[("contents", "path corpus", ROOT), ("contents", RESPONSE, "$.corpus"), ("text_window", "query corpus", ROOT)])),
    ])
}

#[test]
fn every_identity_site_names_its_schema() {
    // Arrange
    let document = published();
    let expected = the_published_inventory();
    let named: BTreeSet<&str> = expected.keys().copied().collect();

    // Act
    let inventory = inventory_of(&document, &named);

    // Assert
    assert_eq!(inventory, expected);
}

#[test]
fn no_unnamed_identity_can_be_published() {
    // Arrange
    let document = published();

    // Act
    let unnamed = unnamed_primitive_sites(&document);
    let unargued: BTreeSet<&String> = unnamed.iter().filter(|at| !PROSE.contains(&at.as_str()) && !NAMED_LATER.contains(&at.as_str())).collect();
    let stale: BTreeSet<&str> = NAMED_LATER.iter().copied().filter(|at| !unnamed.contains(*at)).collect();

    // Assert
    assert_eq!((unargued, stale), (BTreeSet::new(), BTreeSet::new()));
}

#[test]
fn the_contents_corpus_segment_and_every_text_reference_tag_are_the_closed_corpus() {
    // Arrange
    let document = published();
    let corpus = schema_named(&document, CORPUS)["enum"].clone();

    // Act
    let segment = operation(&document, "contents")["parameters"].as_array().unwrap().iter().find(|parameter| parameter["name"] == "corpus").unwrap()["schema"].clone();
    let tags = schema_named(&document, "TextRef")["properties"]["corpus"]["enum"].clone();

    // Assert
    assert_eq!((segment, tags), (serde_json::json!({ "$ref": format!("{COMPONENTS}{CORPUS}") }), corpus));
}

#[test]
fn every_served_identity_in_the_committed_fixtures_and_pact_matches_its_published_shape() {
    // Arrange
    let document = published();
    let named: BTreeSet<&str> = the_published_inventory().keys().copied().collect();
    let bodies = committed_bodies();

    // Act
    let checked: Vec<(String, String, String)> = bodies.iter().flat_map(|(uri, body)| served_identities(&document, &named, uri, body)).collect();
    let misshapen: Vec<&(String, String, String)> = checked.iter().filter(|(identity, _, value)| !matches_shape(&document, identity, value)).collect();
    let seen: BTreeSet<&str> = checked.iter().map(|(identity, _, _)| identity.as_str()).collect();

    // Assert
    assert_eq!((seen, misshapen), (RECORDED_IDENTITIES.into_iter().collect(), Vec::<&(String, String, String)>::new()));
}

const RECORDED_IDENTITIES: [&str; 12] = [
    ARTIFACT_ROOT, BIBLE_REFERENCE, CHAPTER_REFERENCE, CONTENTS_REFERENCE, CORPUS, CROSS_REFERENCE_TARGET, EDGE_ID, EDGE_PAGE_CURSOR, ELEMENT_ID, NODE_ID, UNIT_REFERENCE, VERSE_REFERENCE,
];

#[test]
fn the_identities_widen_only_as_declared() {
    // Arrange
    let document = published();
    let expected: BTreeSet<(String, String)> = [
        ("BookId", BIBLE_REFERENCE),
        (CHAPTER_REFERENCE, BIBLE_REFERENCE),
        (CHAPTER_REFERENCE, CONTENTS_REFERENCE),
        (CHAPTER_REFERENCE, "ReadingReference"),
        (CHAPTER_REFERENCE, TEXT_WINDOW_REFERENCE),
        ("ConcordReference", CONTENTS_REFERENCE),
        ("ConcordReference", TEXT_WINDOW_REFERENCE),
        ("ConcordReference", UNIT_REFERENCE),
        (CONTENTS_REFERENCE, TEXT_WINDOW_REFERENCE),
        (EDGE_ID, ELEMENT_ID),
        (NODE_ID, ELEMENT_ID),
        ("PassageReference", BIBLE_REFERENCE),
        ("PassageReference", CROSS_REFERENCE_TARGET),
        ("PassageReference", VERSE_SPAN_REFERENCE),
        ("ReadingReference", BIBLE_REFERENCE),
        ("ReadingReference", TEXT_WINDOW_REFERENCE),
        (UNIT_REFERENCE, TEXT_WINDOW_REFERENCE),
        ("VerseRangeReference", CROSS_REFERENCE_TARGET),
        (VERSE_REFERENCE, BIBLE_REFERENCE),
        (VERSE_REFERENCE, CROSS_REFERENCE_TARGET),
        (VERSE_REFERENCE, "ReadingReference"),
        (VERSE_REFERENCE, TEXT_WINDOW_REFERENCE),
        (VERSE_REFERENCE, UNIT_REFERENCE),
        (VERSE_REFERENCE, VERSE_SPAN_REFERENCE),
        (VERSE_SPAN_REFERENCE, BIBLE_REFERENCE),
        (VERSE_SPAN_REFERENCE, CROSS_REFERENCE_TARGET),
    ]
    .into_iter()
    .map(|(narrower, wider)| (narrower.to_string(), wider.to_string()))
    .collect();

    // Act
    let declared = atlas_core::identity::widenings();
    let published_members: BTreeMap<String, BTreeSet<String>> = atlas_core::identity::members()
        .into_keys()
        .map(|union| {
            let members = schema_named(&document, &union)["oneOf"].as_array().unwrap().iter().map(|member| member["$ref"].as_str().unwrap().strip_prefix(COMPONENTS).unwrap().to_string()).collect();
            (union, members)
        })
        .collect();

    // Assert
    assert_eq!((declared, published_members), (expected, atlas_core::identity::members()));
}

#[test]
fn every_identity_site_has_its_own_type() {
    // Arrange
    use atlas_contract::wire::*;
    use atlas_core::refs::{ScriptureRef, VerseId};
    use utoipa::ToSchema;

    // Act
    let sites = BTreeMap::from([
        ("CatechismProofVerse.vref", type_of(|site: &CatechismProofVerse| &site.vref)),
        ("Chapter.ref", type_of(|site: &Chapter| &site.r#ref)),
        ("Contents.version", type_of(|site: &Contents| &site.version)),
        ("ContentsChild.id", type_of(|site: &ContentsChild| &site.id)),
        ("ContentsChild.ref", type_of(|site: &ContentsChild| &site.r#ref)),
        ("ContentsRoot.id", type_of(|site: &ContentsRoot| &site.id)),
        ("ContentsRoot.ref", type_of(|site: &ContentsRoot| &site.r#ref)),
        ("CrossRef.target", type_of(|site: &CrossRef| &site.target)),
        ("EdgePage.next", type_of(|site: &EdgePage| &site.next)),
        ("EdgePage.previous", type_of(|site: &EdgePage| &site.previous)),
        ("EdgePage.version", type_of(|site: &EdgePage| &site.version)),
        ("EdgeRecord.id", type_of(|site: &EdgeRecord| &site.id)),
        ("EdgeRef.id", type_of(|site: &EdgeRef| &site.id)),
        ("ElementPage.next", type_of(|site: &ElementPage| &site.next)),
        ("ElementPage.previous", type_of(|site: &ElementPage| &site.previous)),
        ("ElementPage.version", type_of(|site: &ElementPage| &site.version)),
        ("EventPage.mentioned_in", type_of(|site: &EventPage| &site.mentioned_in)),
        ("KretzmannChapter.version", type_of(|site: &KretzmannChapter| &site.version)),
        ("KretzmannChapterItem.id", type_of(|site: &KretzmannChapterItem| &site.id)),
        ("NodeRecord.id", type_of(|site: &NodeRecord| &site.id)),
        ("NodeRecord.version", type_of(|site: &NodeRecord| &site.version)),
        ("NodeRef.id", type_of(|site: &NodeRef| &site.id)),
        ("PolityDelta.verses", type_of(|site: &atlas_core::data::PolityDelta| &site.verses)),
        ("Scene.ref", type_of(|site: &atlas_core::wire::Scene| &site.r#ref)),
        ("TextUnit.ref", type_of(|site: &TextUnit| &site.r#ref)),
        ("TextWindow.next", type_of(|site: &TextWindow| &site.next)),
        ("TextWindow.version", type_of(|site: &TextWindow| &site.version)),
        ("VerseGroup.verses", type_of(|site: &atlas_core::wire::VerseGroup| &site.verses)),
    ]);
    let published_as = BTreeMap::from([
        (std::any::type_name::<ScriptureRef>(), ScriptureRef::name()),
        (std::any::type_name::<VerseId>(), VerseId::name()),
        (std::any::type_name::<NodeId>(), NodeId::name()),
        (std::any::type_name::<EdgeId>(), EdgeId::name()),
    ]);

    // Assert
    assert_eq!(
        (sites, published_as),
        (
            BTreeMap::from([
                ("CatechismProofVerse.vref", "atlas_core::refs::VerseId"),
                ("Chapter.ref", "atlas_core::identity::ChapterReference"),
                ("Contents.version", "atlas_core::identity::ArtifactRoot"),
                ("ContentsChild.id", "atlas_core::identity::NodeId"),
                ("ContentsChild.ref", "atlas_core::identity::ContentsReference"),
                ("ContentsRoot.id", "atlas_core::identity::NodeId"),
                ("ContentsRoot.ref", "atlas_core::identity::ContentsReference"),
                ("CrossRef.target", "atlas_core::identity::CrossReferenceTarget"),
                ("EdgePage.next", "core::option::Option<atlas_core::identity::EdgePageCursor>"),
                ("EdgePage.previous", "core::option::Option<atlas_core::identity::EdgePageCursor>"),
                ("EdgePage.version", "atlas_core::identity::ArtifactRoot"),
                ("EdgeRecord.id", "atlas_graph_types::edge::EdgeId"),
                ("EdgeRef.id", "atlas_graph_types::edge::EdgeId"),
                ("ElementPage.next", "core::option::Option<atlas_core::identity::ElementPageCursor>"),
                ("ElementPage.previous", "core::option::Option<atlas_core::identity::ElementPageCursor>"),
                ("ElementPage.version", "atlas_core::identity::ArtifactRoot"),
                ("EventPage.mentioned_in", "alloc::vec::Vec<atlas_core::refs::VerseId>"),
                ("KretzmannChapter.version", "atlas_core::identity::ArtifactRoot"),
                ("KretzmannChapterItem.id", "atlas_core::identity::NodeId"),
                ("NodeRecord.id", "atlas_core::identity::NodeId"),
                ("NodeRecord.version", "atlas_core::identity::ArtifactRoot"),
                ("NodeRef.id", "atlas_core::identity::NodeId"),
                ("PolityDelta.verses", "alloc::vec::Vec<atlas_core::refs::VerseId>"),
                ("Scene.ref", "core::option::Option<atlas_core::refs::ScriptureRef>"),
                ("TextUnit.ref", "atlas_core::identity::UnitReference"),
                ("TextWindow.next", "core::option::Option<atlas_core::identity::UnitReference>"),
                ("TextWindow.version", "atlas_core::identity::ArtifactRoot"),
                ("VerseGroup.verses", "alloc::vec::Vec<atlas_core::refs::VerseId>"),
            ]),
            BTreeMap::from([
                ("atlas_core::refs::ScriptureRef", BIBLE_REFERENCE.into()),
                ("atlas_core::refs::VerseId", VERSE_REFERENCE.into()),
                ("atlas_core::identity::NodeId", NODE_ID.into()),
                ("atlas_graph_types::edge::EdgeId", EDGE_ID.into()),
            ]),
        )
    );
}

fn type_of<Record, Field>(_: fn(&Record) -> &Field) -> &'static str {
    std::any::type_name::<Field>()
}

const LOCAL_NAMES: [&str; 3] = ["EraId", "NarrativeId", "PolityId"];

#[test]
fn a_kind_s_local_name_is_published_as_one_and_never_reads_as_a_node_id() {
    // Arrange
    let document = published();
    let named: BTreeSet<&str> = LOCAL_NAMES.into_iter().collect();
    let node_id = regex::Regex::new(schema_named(&document, NODE_ID)["pattern"].as_str().unwrap()).unwrap();

    // Act
    let descriptions: Vec<String> = LOCAL_NAMES.iter().map(|name| schema_named(&document, name)["description"].as_str().unwrap().to_string()).collect();
    let served: Vec<(String, String, String)> = committed_bodies().iter().flat_map(|(uri, body)| served_identities(&document, &named, uri, body)).collect();
    let read_as_node_ids: Vec<&(String, String, String)> = served.iter().filter(|(_, _, value)| node_id.is_match(value)).collect();
    let kinds: BTreeSet<&str> = served.iter().map(|(name, _, _)| name.as_str()).collect();

    // Assert
    assert_eq!(
        (descriptions, kinds, read_as_node_ids),
        (
            vec!["The local name of one Era node, without its kind.".to_string(), "The local name of one Narrative node, without its kind.".to_string(), "The local name of one Polity node, without its kind.".to_string()],
            BTreeSet::from(["EraId", "PolityId"]),
            Vec::<&(String, String, String)>::new()
        )
    );
}

const PROSE: [&str; 125] = [
    "Anchor.end", "Anchor.start", "BibleRef.chapter", "BibleRef.verse",
    "BookDetail.author", "CanonBook.chapters[]", "CanonBook.name", "CatechismDetail.explanation",
    "CatechismDetail.explanation_heading", "CatechismDetail.part_title", "CatechismDetail.text", "CatechismDetail.where_written",
    "CatechismItem.explanation", "CatechismItem.explanation_heading", "CatechismItem.name", "CatechismItem.part_title",
    "CatechismItem.text", "CatechismItem.where_written", "CatechismProofVerse.question", "CatechismProofVerse.text",
    "CatechismRef.name", "CatechismRef.question", "Chapter.book", "Chapter.chapter",
    "ConcordRef.article", "ConcordRef.paragraph", "ConcordRef.part", "Contents.description",
    "ContentsChild.count", "ContentsChild.section_title", "ContentsChild.title", "ContentsRoot.title",
    "Contract.manifest_schema", "Contract.section_schema_version", "CrossRef.preview", "CrossRef.votes",
    "DateClaim.label", "DateClaim.note", "EdgeEntry.note", "EdgeEntry.votes",
    "EdgeRecord.label", "EdgeRecord.votes", "EdgeRef.label", "EdgeSummaryEntry.count",
    "Era.from_year", "Era.name", "Era.to_year", "ErrorInner.message",
    "EventAnalogue.title", "EventDetail.acts_section", "EventDetail.atlas_section", "EventDetail.kjv_superscription",
    "EventDetail.ref_note", "EventDetail.robertson_section", "EventPage.acts_section", "EventPage.atlas_section",
    "EventPage.kjv_superscription", "EventPage.ref_note", "EventPage.robertson_section", "EventPage.title",
    "EventWitness.ref_note", "EventWitness.robertson_section", "Heading.title", "KretzmannChapterItem.heading",
    "KretzmannChapterVerse.verse", "Landmark.name", "Narrative.color", "Narrative.name",
    "NarrativeAdjacentEvent.label", "NarrativePosition.event_label", "NarrativePosition.narrative_name", "NodeRecord.description",
    "NodeRecord.label", "NodeRef.label", "PersonLife.also_called[]", "PersonLife.eternal_grounds[]",
    "PersonLife.gender", "PersonRef.name", "PlaceDetail.canonical_name", "PlaceDetail.display_name",
    "PlaceRef.name", "Polity.color_key", "Polity.from", "Polity.name",
    "Polity.to", "PolityDelta.ref_note", "Provenance.title", "ProvenanceEntry.locator",
    "QuietPlace.display_name", "QuietPlace.total_events", "SceneArrow.color", "SceneArrow.order",
    "SceneEvent.label", "SceneNarrative.color", "SceneNarrative.legs_in_scene", "SceneNarrative.name",
    "ScenePlace.brightness", "ScenePlace.display_name", "ScenePlace.name", "SourceCategory.label",
    "SourceEntry.license", "SourceEntry.link", "SourceEntry.title", "SourceEntry.what_it_is",
    "SourceEntry.what_we_built", "TextPoint.word", "TimeRange.label", "UnitText.text",
    "Verse.text", "Verse.verse", "Verse.xref_count", "VerseGroup.chapter",
    "VerseGroup.count", "WordsOfChristSpan.end", "WordsOfChristSpan.start", "Year.label",
    "Year.value", "YearSpan.from_year", "YearSpan.to_year", "node_edges query limit",
    "polities query from", "polities query to", "scene_time query from", "scene_time query to",
    "text_window query n",
];

const NAMED_LATER: [&str; 38] = [
    "CatechismItem.id", "CatechismRef.id", "EventAnalogue.id", "EventPage.id",
    "Heading.event_id", "Narrative.id", "Narrative.legs[]", "NarrativeAdjacentEvent.id",
    "NarrativeAdjacentEvent.places[]", "NarrativePosition.event_id", "NarrativePosition.narrative_id", "PersonRef.id",
    "PlaceRef.id", "PolityDelta.event", "QuietPlace.id", "QuietPlace.merged_ids[]",
    "SceneArrow.from_event", "SceneArrow.from_place", "SceneArrow.narrative", "SceneArrow.to_event",
    "SceneArrow.to_place", "SceneEvent.id", "SceneNarrative.id", "ScenePlace.id",
    "ScenePlace.merged_ids[]", "catechism_item path id", "event path id", "narrative_event_positions path id",
    "CanonBook.code", "EventWitness.book", "VerseGroup.book", "Provenance.id",
    "ProvenanceEntry.id", "ProvenanceEntry.source", "SourceCategory.id", "SourceEntry.category",
    "SourceEntry.id", "SourceEntry.licenses_row_key",
];

fn published() -> Value {
    serde_json::to_value(atlas_contract::document::openapi()).unwrap()
}

fn schema_named<'a>(document: &'a Value, name: &str) -> &'a Value {
    &document["components"]["schemas"][name]
}

fn operations(document: &Value) -> impl Iterator<Item = (&String, &Value)> {
    document["paths"].as_object().unwrap().iter().flat_map(|(path, methods)| methods.as_object().unwrap().values().map(move |operation| (path, operation)))
}

fn operation<'a>(document: &'a Value, id: &str) -> &'a Value {
    operations(document).map(|(_, operation)| operation).find(|operation| operation["operationId"] == id).unwrap()
}

fn inventory_of(document: &Value, named: &BTreeSet<&str>) -> BTreeMap<&'static str, BTreeSet<Site>> {
    let mut inventory: BTreeMap<&'static str, BTreeSet<Site>> = the_published_inventory().keys().map(|name| (*name, BTreeSet::new())).collect();
    for (_, operation) in operations(document) {
        let id = operation["operationId"].as_str().unwrap();
        let parameters = operation["parameters"].as_array().into_iter().flatten();
        let parameter_schemas = parameters.map(|parameter| (format!("{} {}", parameter["in"].as_str().unwrap(), parameter["name"].as_str().unwrap()), &parameter["schema"]));
        let response = std::iter::once((RESPONSE.to_string(), &operation["responses"][OK]["content"][JSON_BODY]["schema"]));
        for (location, schema) in parameter_schemas.chain(response) {
            let mut found = Vec::new();
            walk(document, schema, ROOT.to_string(), named, &mut Vec::new(), &mut found);
            for (identity, path) in found {
                let key = *inventory.keys().find(|key| **key == identity).unwrap();
                inventory.get_mut(key).unwrap().insert(site(id, &location, &path));
            }
        }
    }
    inventory
}

fn walk(document: &Value, schema: &Value, path: String, named: &BTreeSet<&str>, visiting: &mut Vec<String>, found: &mut Vec<(String, String)>) {
    if let Some(name) = schema["$ref"].as_str().and_then(|reference| reference.strip_prefix(COMPONENTS)) {
        if named.contains(name) {
            found.push((name.to_string(), path));
        } else if !visiting.iter().any(|seen| seen == name) {
            visiting.push(name.to_string());
            walk(document, schema_named(document, name), path, named, visiting, found);
            visiting.pop();
        }
        return;
    }
    for combinator in COMBINATORS {
        for member in schema[combinator].as_array().into_iter().flatten() {
            walk(document, member, path.clone(), named, visiting, found);
        }
    }
    for case in schema[DISCRIMINATOR]["mapping"].as_object().into_iter().flat_map(|mapping| mapping.values()) {
        walk(document, &serde_json::json!({ "$ref": case }), path.clone(), named, visiting, found);
    }
    for (property, member) in schema["properties"].as_object().into_iter().flatten() {
        walk(document, member, format!("{path}.{property}"), named, visiting, found);
    }
    if !schema["items"].is_null() {
        walk(document, &schema["items"], format!("{path}[]"), named, visiting, found);
    }
}

fn unnamed_primitive_sites(document: &Value) -> BTreeSet<String> {
    let mut unnamed = BTreeSet::new();
    for (name, schema) in document["components"]["schemas"].as_object().unwrap() {
        for (property, member) in schema["properties"].as_object().into_iter().flatten() {
            unnamed_within(member, format!("{name}.{property}"), &mut unnamed);
        }
        for part in COMBINATORS.iter().flat_map(|combinator| schema[*combinator].as_array().into_iter().flatten()) {
            for (property, member) in part["properties"].as_object().into_iter().flatten() {
                unnamed_within(member, format!("{name}.{property}"), &mut unnamed);
            }
        }
    }
    for (_, operation) in operations(document) {
        for parameter in operation["parameters"].as_array().into_iter().flatten() {
            unnamed_within(&parameter["schema"], format!("{} {} {}", operation["operationId"].as_str().unwrap(), parameter["in"].as_str().unwrap(), parameter["name"].as_str().unwrap()), &mut unnamed);
        }
    }
    unnamed
}

const PRIMITIVES: [&str; 2] = ["string", "integer"];

fn unnamed_within(schema: &Value, at: String, unnamed: &mut BTreeSet<String>) {
    let types: Vec<&str> = match &schema["type"] {
        Value::String(one) => vec![one.as_str()],
        Value::Array(many) => many.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    };
    if types.iter().any(|kind| PRIMITIVES.contains(kind)) && schema["enum"].is_null() && schema["pattern"].is_null() {
        unnamed.insert(at.clone());
    }
    if !schema["items"].is_null() {
        unnamed_within(&schema["items"], format!("{at}[]"), unnamed);
    }
    for member in COMBINATORS.iter().flat_map(|combinator| schema[*combinator].as_array().into_iter().flatten()) {
        unnamed_within(member, at.clone(), unnamed);
    }
}

fn committed_bodies() -> Vec<(String, Value)> {
    let contracts = atlas_contract::document::contracts_root();
    let fixtures = contracts.join("atlas-query-contract/fixtures");
    let recorded = |name: &str| -> Value { serde_json::from_str(&std::fs::read_to_string(fixtures.join(format!("{name}.json"))).unwrap()).unwrap() };
    let focus = atlas_contract::aqc_export::SEEDS.iter().map(|(kind, id)| (format!("/api/node/{}", atlas_contract::aqc_export::path_encode(id)), recorded(&format!("focus-{}", kind.to_lowercase()))));
    let fixtures_recorded = atlas_contract::aqc_export::FIXTURES.iter().map(|(name, uri)| (uri.to_string(), recorded(name)));
    let pact: Value = serde_json::from_str(&std::fs::read_to_string(contracts.join("pacts/http.json")).unwrap()).unwrap();
    let pacted = pact["entries"].as_object().unwrap().iter().filter_map(|(key, entry)| key.strip_prefix("GET ").map(|uri| (uri.to_string(), entry.clone())));
    focus.chain(fixtures_recorded).chain(pacted).filter(|(_, entry)| entry["status"] == 200).map(|(uri, entry)| (uri, entry["body"].clone())).collect()
}

fn served_identities(document: &Value, named: &BTreeSet<&str>, uri: &str, body: &Value) -> Vec<(String, String, String)> {
    let path = uri.split('?').next().unwrap();
    let (_, served_by) = operations(document).find(|(template, _)| answers(template, path)).unwrap_or_else(|| panic!("no operation serves {uri}"));
    let mut found = Vec::new();
    values_at(document, &served_by["responses"][OK]["content"][JSON_BODY]["schema"], body, named, &mut found);
    found.into_iter().map(|(identity, value)| (identity, uri.to_string(), value)).collect()
}

fn answers(template: &str, path: &str) -> bool {
    let wanted: Vec<&str> = template.split('/').collect();
    let asked: Vec<&str> = path.split('/').collect();
    wanted.len() == asked.len() && wanted.iter().zip(&asked).all(|(segment, given)| segment.starts_with('{') || segment == given)
}

fn values_at(document: &Value, schema: &Value, body: &Value, named: &BTreeSet<&str>, found: &mut Vec<(String, String)>) {
    if body.is_null() {
        return;
    }
    if let Some(name) = schema["$ref"].as_str().and_then(|reference| reference.strip_prefix(COMPONENTS)) {
        match named.contains(name) {
            true => found.push((name.to_string(), body.as_str().map_or_else(|| body.to_string(), str::to_string))),
            false => values_at(document, schema_named(document, name), body, named, found),
        }
        return;
    }
    if let Some(case) = schema[DISCRIMINATOR]["propertyName"].as_str().and_then(|tag| body[tag].as_str()).and_then(|tag| schema[DISCRIMINATOR]["mapping"][tag].as_str()) {
        let own = schema_named(document, case.strip_prefix(COMPONENTS).unwrap())["allOf"][1].clone();
        values_at(document, &own, body, named, found);
        return;
    }
    for member in schema["oneOf"].as_array().into_iter().flatten().chain(schema["anyOf"].as_array().into_iter().flatten()).chain(schema["allOf"].as_array().into_iter().flatten()) {
        values_at(document, member, body, named, found);
    }
    for (property, member) in schema["properties"].as_object().into_iter().flatten() {
        values_at(document, member, &body[property], named, found);
    }
    if let (false, Some(items)) = (schema["items"].is_null(), body.as_array()) {
        items.iter().for_each(|item| values_at(document, &schema["items"], item, named, found));
    }
}

fn matches_shape(document: &Value, identity: &str, value: &str) -> bool {
    leaves_of(document, identity).iter().filter(|leaf| leaf_admits(document, leaf, value)).count() == 1
}

fn leaves_of(document: &Value, identity: &str) -> Vec<String> {
    match schema_named(document, identity)["oneOf"].as_array() {
        Some(members) => members.iter().flat_map(|member| leaves_of(document, member["$ref"].as_str().unwrap().strip_prefix(COMPONENTS).unwrap())).collect(),
        None => vec![identity.to_string()],
    }
}

fn leaf_admits(document: &Value, leaf: &str, value: &str) -> bool {
    let shape = schema_named(document, leaf);
    match (&shape["pattern"], &shape["enum"], &shape["type"]) {
        (Value::String(pattern), _, _) => regex::Regex::new(pattern).unwrap().is_match(value),
        (_, Value::Array(values), _) => values.iter().any(|admitted| admitted == value),
        (_, _, Value::String(integer)) if integer == "integer" => value.parse::<u64>().is_ok(),
        _ => false,
    }
}
