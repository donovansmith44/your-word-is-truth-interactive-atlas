use atlas_graph_types::canon::{Canon, Value};
use atlas_graph_types::id::{AnyNodeId, NodeKind};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{LayerMap, Piece, Rendering, TextPart, TextPartRole, TranslationId};
use proptest::prelude::*;

const CANONICAL_LAYER: &str = "kjv";
const MOST_PIECES: usize = 8;
const WORDING: &str = "[a-c\u{e9}\\[\\] ]{0,5}";

proptest! {
    #[test]
    fn every_rendering_is_partitioned_into_parts_in_normal_form(pieces in any_pieces()) {
        // Arrange
        let expected = partition_of(&pieces);

        // Act
        let rendering = Rendering::compose(&pieces);

        // Assert
        prop_assert_eq!((rendering.text().to_string(), rendering.parts().to_vec()), expected);
    }

    #[test]
    fn composing_and_splitting_a_rendering_round_trip(pieces in any_pieces()) {
        // Arrange
        let rendering = Rendering::compose(&pieces);

        // Act
        let split = rendering.pieces();
        let recomposed = Rendering::compose(&split);

        // Assert
        prop_assert_eq!((split, recomposed), (normal_form(&pieces), rendering));
    }

    #[test]
    fn a_whole_text_rendering_is_one_text_part_and_encodes_as_its_text_alone(text in WORDING) {
        // Arrange
        let node = unit_with(Rendering::whole(text.clone()));

        // Act
        let encoded = node.to_value();

        // Assert
        prop_assert_eq!(
            (Rendering::whole(text.clone()), encoded),
            (Rendering::compose(&[Piece { role: TextPartRole::Text, text: text.clone() }]), unit_value(Value::Str(text)))
        );
    }

    #[test]
    fn a_rendering_round_trips_through_its_canonical_encoding(pieces in any_pieces()) {
        // Arrange
        let node = unit_with(Rendering::compose(&pieces));

        // Act
        let decoded = Node::decode(&node.encode());

        // Assert
        prop_assert_eq!(decoded, Ok(node));
    }
}

#[test]
fn a_rendering_with_labelled_parts_encodes_as_its_pieces_in_order() {
    // Arrange
    let pieces = [heading("The First Article."), text(" I believe in God the Father Almighty, "), bracket("[Maker]")];
    let node = unit_with(Rendering::compose(&pieces));

    // Act
    let encoded = node.to_value();

    // Assert
    assert_eq!(
        encoded,
        unit_value(Value::Arr(vec![
            piece_value("heading", "The First Article."),
            piece_value("text", " I believe in God the Father Almighty, "),
            piece_value("bracket", "[Maker]"),
        ]))
    );
}

#[test]
fn an_encoding_that_is_not_in_normal_form_is_refused() {
    // Arrange
    let spellings = [
        Value::Arr(vec![piece_value("text", "one spelling")]),
        Value::Arr(vec![piece_value("heading", "Twice"), piece_value("heading", " named")]),
        Value::Arr(vec![piece_value("heading", "Empty"), piece_value("text", "")]),
        Value::Arr(vec![]),
    ];

    // Act
    let refused: Vec<bool> = spellings.into_iter().map(|spelling| Node::from_value(&unit_value(spelling)).is_err()).collect();

    // Assert
    assert_eq!(refused, vec![true, true, true, true]);
}

#[test]
fn an_encoding_with_an_unknown_role_is_refused() {
    // Arrange
    let spelling = Value::Arr(vec![piece_value("question", "What does this mean?"), piece_value("text", " Answer.")]);

    // Act
    let refused = Node::from_value(&unit_value(spelling)).is_err();

    // Assert
    assert!(refused);
}

fn any_pieces() -> impl Strategy<Value = Vec<Piece>> {
    prop::collection::vec((prop::sample::select(TextPartRole::ALL.to_vec()), WORDING), 0..MOST_PIECES)
        .prop_map(|drawn| drawn.into_iter().map(|(role, text)| Piece { role, text }).collect())
}

fn normal_form(pieces: &[Piece]) -> Vec<Piece> {
    let mut merged: Vec<Piece> = Vec::new();
    for piece in pieces.iter().filter(|piece| !piece.text.is_empty()) {
        match merged.last_mut() {
            Some(last) if last.role == piece.role => last.text.push_str(&piece.text),
            _ => merged.push(piece.clone()),
        }
    }
    merged
}

fn partition_of(pieces: &[Piece]) -> (String, Vec<TextPart>) {
    let mut text = String::new();
    let mut parts = Vec::new();
    for piece in normal_form(pieces) {
        let start = text.len();
        text.push_str(&piece.text);
        parts.push(TextPart { role: piece.role, start, end: text.len() });
    }
    (text, parts)
}

fn unit_with(rendering: Rendering) -> Node {
    let renderings: LayerMap = [(TranslationId(CANONICAL_LAYER.into()), rendering)].into_iter().collect();
    Node { id: AnyNodeId { kind: NodeKind::TextUnit, raw: "bible/GEN.1.1".into() }, payload: NodePayload::TextUnit { corpus: "bible", renderings }, provenance: "kjv".into() }
}

fn unit_value(rendering: Value) -> Value {
    object(&[
        ("id", Value::Str("TextUnit:bible/GEN.1.1".into())),
        (
            "payload",
            object(&[("TextUnit", object(&[("corpus", Value::Str("bible".into())), ("renderings", object(&[(CANONICAL_LAYER, rendering)]))]))]),
        ),
        ("provenance", Value::Str("kjv".into())),
    ])
}

fn piece_value(role: &str, text: &str) -> Value {
    object(&[("role", Value::Str(role.into())), ("text", Value::Str(text.into()))])
}

fn object(pairs: &[(&str, Value)]) -> Value {
    Value::Obj(pairs.iter().map(|(key, value)| (key.to_string(), value.clone())).collect())
}

fn heading(words: &str) -> Piece {
    Piece { role: TextPartRole::Heading, text: words.into() }
}

fn text(words: &str) -> Piece {
    Piece { role: TextPartRole::Text, text: words.into() }
}

fn bracket(words: &str) -> Piece {
    Piece { role: TextPartRole::Bracket, text: words.into() }
}
