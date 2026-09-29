use atlas_contract::wire::{Anchor, ForeignLayer, NodeRef, PositionKind, TextPoint, TextRef, TextSpan};
use atlas_core::refs::BookId;
use atlas_graph::kjv_adapter::KJV_TRANSLATION;
use atlas_graph_types::text::{BibleLocusRange, ConcordRef, Locus, TokenSpan, TranslationId, VerseRef};
use atlas_graph_types::{Direction, EdgeKind, NodeKind, RelationId};

const GENESIS: u8 = 0;
const FIRST_SAMUEL: u8 = 8;
const ANOTHER_LAYER: &str = "greek_textus_receptus";
const WAS: u16 = 3;
const FORM: u16 = 5;
const PRAYED: u16 = 4;
const LAST_WORD_OF_1SA_1_27: u16 = 17;
const FIRST_WORD: u16 = 0;
const AS_LONG_AS_ENDS: u16 = 11;
const HAZOR_STARTS: usize = 22;
const HAZOR_ENDS: usize = 27;

#[test]
fn a_bible_ref_serialises_tagged_with_its_corpus_beside_the_book_code() {
    // Arrange
    let r = TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 };
    // Act
    let json = serde_json::to_value(&r).unwrap();
    // Assert
    assert_eq!(json, serde_json::json!({ "corpus": "bible", "book": "GEN", "chapter": 1, "verse": 1 }));
}

#[test]
fn a_concord_ref_serialises_tagged_with_its_corpus() {
    // Arrange
    let r = TextRef::Concord { part: 7, article: 2, paragraph: 1 };
    // Act
    let json = serde_json::to_value(&r).unwrap();
    // Assert
    assert_eq!(json, serde_json::json!({ "corpus": "concord", "part": 7, "article": 2, "paragraph": 1 }));
}

#[test]
fn a_verse_ref_becomes_a_bible_ref_of_the_same_book_chapter_and_verse() {
    // Arrange
    let verse = VerseRef { book: GENESIS, chapter: 1, verse: 1 };
    // Act
    let r = TextRef::of_verse(&verse);
    // Assert
    assert_eq!(r, TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 });
}

#[test]
fn the_graphs_corpus_erased_ref_widens_to_the_wire_ref() {
    // Arrange
    let bible = atlas_graph_types::text::TextRef::Bible(VerseRef { book: GENESIS, chapter: 1, verse: 1 });
    let concord = atlas_graph_types::text::TextRef::Concord(ConcordRef { part: 7, article: 2, paragraph: 1 });
    // Act
    let widened = (TextRef::from(&bible), TextRef::from(&concord));
    // Assert
    assert_eq!(
        widened,
        (TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 }, TextRef::Concord { part: 7, article: 2, paragraph: 1 })
    );
}

#[test]
fn a_point_at_a_unit_edge_omits_its_word() {
    // Arrange
    let point = TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 }, word: None };
    // Act
    let json = serde_json::to_value(&point).unwrap();
    // Assert
    assert_eq!(json, serde_json::json!({ "unit": { "corpus": "bible", "book": "GEN", "chapter": 1, "verse": 1 } }));
}

#[test]
fn a_whole_unit_is_a_span_from_its_edge_to_its_edge() {
    // Arrange
    let unit = TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 };
    // Act
    let span = TextSpan::whole(unit);
    // Assert
    assert_eq!(
        span,
        TextSpan {
            from: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 }, word: None },
            to: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 }, word: None },
        }
    );
}

#[test]
fn a_whole_verse_range_is_a_span_between_unit_edges() {
    // Arrange
    let range = BibleLocusRange { from: Locus::whole(genesis(1, 1)), to: Locus::whole(genesis(1, 3)) };
    // Act
    let span = TextSpan::of_bible_range(&range);
    // Assert
    assert_eq!(
        span,
        Ok(TextSpan {
            from: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 1 }, word: None },
            to: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 3 }, word: None },
        })
    );
}

#[test]
fn a_word_span_inside_one_verse_names_its_first_and_last_word() {
    // Arrange
    let was_without_form = Locus { unit: genesis(1, 2), span: Some(words(KJV_TRANSLATION, WAS, FORM)) };
    let range = BibleLocusRange { from: was_without_form.clone(), to: was_without_form };
    // Act
    let span = TextSpan::of_bible_range(&range);
    // Assert
    assert_eq!(
        span,
        Ok(TextSpan {
            from: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 2 }, word: Some(WAS) },
            to: TextPoint { unit: TextRef::Bible { book: BookId(GENESIS), chapter: 1, verse: 2 }, word: Some(FORM) },
        })
    );
}

#[test]
fn a_word_span_across_verses_names_where_it_starts_and_ends() {
    // Arrange
    let range = BibleLocusRange {
        from: Locus { unit: first_samuel(1, 27), span: Some(words(KJV_TRANSLATION, PRAYED, LAST_WORD_OF_1SA_1_27)) },
        to: Locus { unit: first_samuel(1, 28), span: Some(words(KJV_TRANSLATION, FIRST_WORD, AS_LONG_AS_ENDS)) },
    };
    // Act
    let span = TextSpan::of_bible_range(&range);
    // Assert
    assert_eq!(
        span,
        Ok(TextSpan {
            from: TextPoint { unit: TextRef::Bible { book: BookId(FIRST_SAMUEL), chapter: 1, verse: 27 }, word: Some(PRAYED) },
            to: TextPoint { unit: TextRef::Bible { book: BookId(FIRST_SAMUEL), chapter: 1, verse: 28 }, word: Some(AS_LONG_AS_ENDS) },
        })
    );
}

#[test]
fn a_span_in_another_layer_is_refused() {
    // Arrange
    let starting_in_another_layer =
        BibleLocusRange { from: Locus { unit: genesis(1, 2), span: Some(words(ANOTHER_LAYER, WAS, FORM)) }, to: Locus::whole(genesis(1, 3)) };
    let ending_in_another_layer =
        BibleLocusRange { from: Locus::whole(genesis(1, 1)), to: Locus { unit: genesis(1, 2), span: Some(words(ANOTHER_LAYER, WAS, FORM)) } };
    // Act
    let spans = [TextSpan::of_bible_range(&starting_in_another_layer), TextSpan::of_bible_range(&ending_in_another_layer)];
    // Assert
    assert_eq!(
        spans,
        [Err(ForeignLayer { layer: TranslationId(ANOTHER_LAYER.to_string()) }), Err(ForeignLayer { layer: TranslationId(ANOTHER_LAYER.to_string()) })]
    );
}

#[test]
fn an_anchor_serialises_its_offsets_kind_and_node() {
    // Arrange
    let anchor = Anchor {
        start: HAZOR_STARTS,
        end: HAZOR_ENDS,
        kind: EdgeKind::Directed(RelationId::Mentions, Direction::Forward),
        node: NodeRef { id: "Place:hazor-1".to_string(), kind: PositionKind::Node(NodeKind::Place), label: "Hazor".to_string() },
    };
    // Act
    let json = serde_json::to_value(&anchor).unwrap();
    // Assert
    assert_eq!(
        json,
        serde_json::json!({ "start": HAZOR_STARTS, "end": HAZOR_ENDS, "kind": "mentions", "node": { "id": "Place:hazor-1", "kind": "Place", "label": "Hazor" } })
    );
}

fn genesis(chapter: u16, verse: u16) -> VerseRef {
    VerseRef { book: GENESIS, chapter, verse }
}

fn first_samuel(chapter: u16, verse: u16) -> VerseRef {
    VerseRef { book: FIRST_SAMUEL, chapter, verse }
}

fn words(layer: &str, first: u16, last: u16) -> TokenSpan {
    TokenSpan { layer: TranslationId(layer.to_string()), start: first, end: last }
}

#[test]
fn a_bible_refs_book_publishes_as_one_of_the_canons_codes() {
    // Arrange
    let document = serde_json::to_value(atlas_contract::document::openapi()).unwrap();
    let canon_codes: Vec<&str> = atlas_core::canon::BOOKS.iter().map(|book| book.code).collect();
    // Act
    let book = &document["components"]["schemas"]["BookId"];
    // Assert
    assert_eq!(*book, serde_json::json!({ "type": "string", "description": "A book of the Bible, by its canon code.", "enum": canon_codes }));
}

#[test]
fn a_text_ref_publishes_as_a_base_naming_its_corpus_with_one_subtype_per_corpus() {
    // Arrange
    let document = serde_json::to_value(atlas_contract::document::openapi()).unwrap();
    let unit_number = serde_json::json!({ "type": "integer", "format": "int32", "minimum": 0 });
    // Act
    let schemas = ["TextRef", "BibleRef", "ConcordRef"].map(|name| document["components"]["schemas"][name].clone());
    // Assert
    assert_eq!(
        schemas,
        [
            serde_json::json!({
                "type": "object",
                "description": "One unit of a corpus's text, named by the corpus it belongs to.",
                "required": ["corpus"],
                "properties": { "corpus": { "type": "string", "enum": ["bible", "concord"] } },
                "additionalProperties": true,
                "discriminator": { "propertyName": "corpus", "mapping": { "bible": "#/components/schemas/BibleRef", "concord": "#/components/schemas/ConcordRef" } },
            }),
            serde_json::json!({
                "description": "A verse of the Bible.",
                "allOf": [
                    { "$ref": "#/components/schemas/TextRef" },
                    {
                        "type": "object",
                        "required": ["book", "chapter", "verse"],
                        "properties": { "book": { "$ref": "#/components/schemas/BookId" }, "chapter": unit_number, "verse": unit_number },
                    },
                ],
                "unevaluatedProperties": false,
            }),
            serde_json::json!({
                "description": "A paragraph of the Book of Concord.",
                "allOf": [
                    { "$ref": "#/components/schemas/TextRef" },
                    {
                        "type": "object",
                        "required": ["part", "article", "paragraph"],
                        "properties": { "part": unit_number, "article": unit_number, "paragraph": unit_number },
                    },
                ],
                "unevaluatedProperties": false,
            }),
        ]
    );
}
