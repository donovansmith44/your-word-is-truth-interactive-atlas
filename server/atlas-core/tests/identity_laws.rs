use std::collections::BTreeSet;

use atlas_core::identity::{
    ArtifactRoot, ChapterReference, ConcordReference, CrossReferenceTarget, EdgePageCursor, ElementPageCursor, NodeId, PassageReference, ReadingReference, UnitReference, VerseRangeReference, VerseSpanReference,
};
use atlas_core::refs::{BookId, ScriptureRef, VerseId};
use atlas_graph_types::adjacency::Cursor;
use atlas_graph_types::edge::{EdgeId, RelationId, SymRelationId};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnyNodeId, ContentHash, NodeKind};
use atlas_graph_types::store::GraphVersion;
use proptest::prelude::*;
use serde_json::Value;
use utoipa::{PartialSchema, ToSchema};

const BOOKS: u8 = 66;
const HASH_BYTES: usize = 16;

fn book() -> impl Strategy<Value = BookId> {
    (0..BOOKS).prop_map(BookId)
}

fn counted() -> impl Strategy<Value = u16> {
    1u16..200
}

fn verse() -> impl Strategy<Value = VerseId> {
    (book(), counted(), counted()).prop_map(|(book, chapter, verse)| VerseId { book, chapter, verse })
}

fn chapter() -> impl Strategy<Value = ChapterReference> {
    (book(), counted()).prop_map(|(book, chapter)| ChapterReference { book, chapter })
}

fn passage() -> impl Strategy<Value = PassageReference> {
    (book(), counted(), counted(), counted()).prop_map(|(book, chapter, from_verse, longer_by)| PassageReference { book, chapter, from_verse, to_verse: from_verse + longer_by })
}

fn verse_range() -> impl Strategy<Value = VerseRangeReference> {
    (verse(), verse()).prop_map(|(from, to)| VerseRangeReference { from, to })
}

fn concord() -> impl Strategy<Value = ConcordReference> {
    (any::<u8>(), any::<u16>(), any::<u16>()).prop_map(|(part, article, paragraph)| ConcordReference { part, article, paragraph })
}

fn hash() -> impl Strategy<Value = ContentHash> {
    any::<[u8; HASH_BYTES]>().prop_map(ContentHash)
}

fn edge_id() -> impl Strategy<Value = EdgeId> {
    let relations: Vec<&'static str> = RelationId::ALL.iter().map(|relation| relation.name()).chain(SymRelationId::ALL.iter().map(|relation| relation.name())).collect();
    (prop::sample::select(relations), hash()).prop_map(|(relation, hash)| EdgeId(format!("{relation}:{}", hash.hex())))
}

fn named_node() -> impl Strategy<Value = AnyNodeId> {
    let kinds: Vec<NodeKind> = NodeId::addressable().collect();
    (prop::sample::select(kinds), "[a-z0-9_/.-]{1,16}").prop_map(|(kind, raw)| AnyNodeId { kind, raw })
}

fn unit_reference() -> impl Strategy<Value = String> {
    prop_oneof![verse().prop_map(|verse| verse.to_string()), concord().prop_map(|paragraph| paragraph.to_string())]
}

fn node_id() -> impl Strategy<Value = NodeId> {
    prop_oneof![
        named_node().prop_map(|id| NodeId::encoded_one(&id, &Graph::default()).unwrap()),
        unit_reference().prop_map(|reference| {
            let unit = AnyNodeId { kind: NodeKind::TextUnit, raw: reference.clone() };
            let mut graph = Graph::default();
            graph.references.insert(unit.clone(), reference);
            NodeId::encoded_one(&unit, &graph).unwrap()
        }),
    ]
}

fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value).unwrap().as_str().unwrap().to_string()
}

fn sample() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        book().prop_map(|book| (<BookId as ToSchema>::name().to_string(), wire(&ScriptureRef::Book(book)))),
        verse().prop_map(|verse| (VerseId::name().to_string(), wire(&verse))),
        chapter().prop_map(|chapter| (ChapterReference::name().to_string(), wire(&chapter))),
        passage().prop_map(|passage| (PassageReference::name().to_string(), wire(&passage))),
        verse_range().prop_map(|range| (VerseRangeReference::name().to_string(), wire(&range))),
        concord().prop_map(|paragraph| (ConcordReference::name().to_string(), wire(&paragraph))),
        node_id().prop_map(|id| (NodeId::name().to_string(), wire(&id))),
        edge_id().prop_map(|id| (EdgeId::name().to_string(), wire(&id))),
        hash().prop_map(|hash| (ArtifactRoot::name().to_string(), wire(&ArtifactRoot::of(GraphVersion(hash))))),
    ]
}

fn string_leaves() -> Vec<(String, Value)> {
    let book = (<BookId as ToSchema>::name().to_string(), serde_json::to_value(BookId::schema()).unwrap());
    atlas_core::identity::schemas()
        .into_iter()
        .map(|(name, schema)| (name, serde_json::to_value(schema).unwrap()))
        .filter(|(_, schema)| schema["type"] == "string")
        .chain(std::iter::once(book))
        .collect()
}

fn admits(shape: &Value, value: &str) -> bool {
    match (&shape["pattern"], &shape["enum"]) {
        (Value::String(pattern), _) => regex::Regex::new(pattern).unwrap().is_match(value),
        (_, Value::Array(values)) => values.iter().any(|admitted| admitted == value),
        _ => false,
    }
}

proptest! {
    #[test]
    fn every_identity_value_reads_as_exactly_its_own_shape((leaf, value) in sample()) {
        // Arrange
        let leaves = string_leaves();

        // Act
        let admitting: BTreeSet<&String> = leaves.iter().filter(|(_, shape)| admits(shape, &value)).map(|(name, _)| name).collect();

        // Assert
        prop_assert_eq!(admitting, BTreeSet::from([&leaf]), "{}", value);
    }

    #[test]
    fn an_artifact_root_is_written_as_the_hex_of_its_version(hash in hash()) {
        // Arrange
        let version = GraphVersion(hash);

        // Act
        let written = serde_json::to_string(&ArtifactRoot::of(version)).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&version.0.hex()).unwrap());
    }

    #[test]
    fn a_page_cursor_is_written_as_the_position_it_resumes_at(at in any::<usize>()) {
        // Arrange
        let cursor = Cursor(at);

        // Act
        let written = (serde_json::to_string(&EdgePageCursor::at(cursor)).unwrap(), serde_json::to_string(&ElementPageCursor::at(cursor)).unwrap());

        // Assert
        prop_assert_eq!(written, (serde_json::to_string(&at).unwrap(), serde_json::to_string(&at).unwrap()));
    }

    #[test]
    fn a_node_id_is_written_as_its_kind_and_its_name(id in named_node()) {
        // Arrange
        let graph = Graph::default();

        // Act
        let written = serde_json::to_string(&NodeId::encoded_one(&id, &graph).unwrap()).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&format!("{:?}:{}", id.kind, id.raw)).unwrap());
    }

    #[test]
    fn a_text_unit_s_id_is_written_as_its_compiled_reference(reference in unit_reference()) {
        // Arrange
        let unit = AnyNodeId { kind: NodeKind::TextUnit, raw: reference.clone() };
        let mut graph = Graph::default();
        graph.references.insert(unit.clone(), reference.clone());

        // Act
        let written = serde_json::to_string(&NodeId::encoded_one(&unit, &graph).unwrap()).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&format!("text-unit:{reference}")).unwrap());
    }

    #[test]
    fn an_edge_id_is_written_as_the_id_it_carries(id in edge_id()) {
        // Arrange
        let before = id.0.clone();

        // Act
        let written = serde_json::to_string(&id).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&before).unwrap());
    }

    #[test]
    fn a_chapter_reference_is_written_as_its_book_code_and_chapter(chapter in chapter()) {
        // Arrange
        let before = format!("{}.{}", chapter.book.code(), chapter.chapter);

        // Act
        let written = serde_json::to_string(&chapter).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&before).unwrap());
    }

    #[test]
    fn a_bible_reference_is_written_as_it_displays(reference in prop_oneof![
        book().prop_map(ScriptureRef::Book),
        chapter().prop_map(ScriptureRef::Chapter),
        verse().prop_map(ScriptureRef::Verse),
        passage().prop_map(ScriptureRef::Passage),
    ]) {
        // Arrange
        let before = reference.to_string();

        // Act
        let written = serde_json::to_string(&reference).unwrap();

        // Assert
        prop_assert_eq!(written, serde_json::to_string(&before).unwrap());
    }

    #[test]
    fn every_reference_reads_back_as_the_reference_it_was_written_from(
        verse in verse(),
        chapter in chapter(),
        passage in passage(),
        range in verse_range(),
        paragraph in concord(),
    ) {
        // Arrange
        let written = (verse.to_string(), chapter.to_string(), passage.to_string(), range.to_string(), paragraph.to_string());

        // Act
        let read = (
            VerseId::parse_canonical(&written.0).ok(),
            written.1.parse::<ChapterReference>().ok(),
            written.2.parse::<PassageReference>().ok(),
            written.3.parse::<VerseRangeReference>().ok(),
            written.4.parse::<ConcordReference>().ok(),
        );

        // Assert
        prop_assert_eq!(read, (Some(verse), Some(chapter), Some(passage), Some(range), Some(paragraph)));
    }

    #[test]
    fn every_union_reads_back_the_member_it_was_written_from(verse in verse(), chapter in chapter(), passage in passage(), range in verse_range(), paragraph in concord()) {
        // Arrange
        let written = (
            VerseSpanReference::from(passage).to_string(),
            CrossReferenceTarget::from(range).to_string(),
            ReadingReference::from(chapter).to_string(),
            UnitReference::from(paragraph).to_string(),
            UnitReference::from(verse).to_string(),
        );

        // Act
        let read = (
            written.0.parse::<VerseSpanReference>().ok(),
            written.1.parse::<CrossReferenceTarget>().ok(),
            written.2.parse::<ReadingReference>().ok(),
            written.3.parse::<UnitReference>().ok(),
            written.4.parse::<UnitReference>().ok(),
        );

        // Assert
        prop_assert_eq!(
            read,
            (Some(passage.into()), Some(range.into()), Some(chapter.into()), Some(paragraph.into()), Some(verse.into()))
        );
    }
}
