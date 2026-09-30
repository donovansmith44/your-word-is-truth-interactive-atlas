use atlas_core::refs::BookId;
use atlas_graph::kjv_adapter::KJV_TRANSLATION;
use atlas_graph_types::text::{self, BibleLocusRange, TokenSpan, TranslationId, VerseRef};
use atlas_graph_types::EdgeKind;
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use utoipa::openapi::extensions::Extensions;
use utoipa::openapi::schema::{AdditionalProperties, AllOfBuilder, ObjectBuilder, Schema, SchemaType, Type};
use utoipa::openapi::{Ref, RefOr};
use utoipa::{PartialSchema, ToSchema};

use super::NodeRef;

/// One unit of a corpus's text: a verse of the Bible, or a paragraph of the Book
/// of Concord. Each is tagged with its corpus on the wire, beside its own parts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextRef {
    Bible { book: BookId, chapter: u16, verse: u16 },
    Concord { part: u8, article: u16, paragraph: u16 },
}

impl TextRef {
    pub fn of_verse(verse: &VerseRef) -> TextRef {
        TextRef::Bible { book: BookId(verse.book), chapter: verse.chapter, verse: verse.verse }
    }
}

impl From<&text::TextRef> for TextRef {
    fn from(unit: &text::TextRef) -> TextRef {
        match unit {
            text::TextRef::Bible(verse) => TextRef::of_verse(verse),
            text::TextRef::Concord(paragraph) => TextRef::Concord { part: paragraph.part, article: paragraph.article, paragraph: paragraph.paragraph },
        }
    }
}

// Written out rather than derived: serde's attributes take only literals, and the
// corpus tags and part names are declared once, below, for this and the schema alike.
impl Serialize for TextRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut unit = s.serialize_map(None)?;
        match self {
            TextRef::Bible { book, chapter, verse } => {
                unit.serialize_entry(CORPUS, BIBLE.tag)?;
                unit.serialize_entry(BOOK, book.code())?;
                unit.serialize_entry(CHAPTER, chapter)?;
                unit.serialize_entry(VERSE, verse)?;
            }
            TextRef::Concord { part, article, paragraph } => {
                unit.serialize_entry(CORPUS, CONCORD.tag)?;
                unit.serialize_entry(PART, part)?;
                unit.serialize_entry(ARTICLE, article)?;
                unit.serialize_entry(PARAGRAPH, paragraph)?;
            }
        }
        unit.end()
    }
}

const CORPUS: &str = "corpus";
const BOOK: &str = "book";
const CHAPTER: &str = "chapter";
const VERSE: &str = "verse";
const PART: &str = "part";
const ARTICLE: &str = "article";
const PARAGRAPH: &str = "paragraph";
const TEXT_REF: &str = "One unit of a corpus's text, named by the corpus it belongs to.";

/// One corpus a `TextRef` can name: the tag its variant is written with, and the
/// component that publishes that variant's parts.
struct Subtype {
    tag: &'static str,
    name: &'static str,
    description: &'static str,
}

const BIBLE: Subtype = Subtype { tag: "bible", name: "BibleRef", description: "A verse of the Bible." };
const CONCORD: Subtype = Subtype { tag: "concord", name: "ConcordRef", description: "A paragraph of the Book of Concord." };
const SUBTYPES: [Subtype; 2] = [BIBLE, CONCORD];

// NSwag generates a sum type only from OpenAPI inheritance -- a base naming the
// discriminator, and one `allOf` subtype per variant -- and utoipa derives that shape
// for no enum, so it is written out here. utoipa's schema model has no field for an
// object's `discriminator` or an `allOf`'s `unevaluatedProperties`; its extensions
// serialise their keys as given, so both keywords travel there. The base is open
// because each subtype adds its own parts; each subtype is closed over the base and
// its parts together.
impl PartialSchema for TextRef {
    fn schema() -> RefOr<Schema> {
        let tags = ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)).enum_values(Some(SUBTYPES.map(|s| s.tag)));
        let mapping: serde_json::Map<String, serde_json::Value> =
            SUBTYPES.iter().map(|s| (s.tag.to_string(), Ref::from_schema_name(s.name).ref_location.into())).collect();
        ObjectBuilder::new()
            .schema_type(SchemaType::Type(Type::Object))
            .description(Some(TEXT_REF))
            .property(CORPUS, tags)
            .required(CORPUS)
            .additional_properties(Some(AdditionalProperties::FreeForm(true)))
            .extensions(Some(Extensions::from_iter([("discriminator", serde_json::json!({ "propertyName": CORPUS, "mapping": mapping }))])))
            .into()
    }
}

impl ToSchema for TextRef {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((BIBLE.name.to_string(), subtype(&BIBLE, [(BOOK, Ref::from_schema_name(<BookId as ToSchema>::name()).into()), (CHAPTER, u16::schema()), (VERSE, u16::schema())])));
        schemas.push((CONCORD.name.to_string(), subtype(&CONCORD, [(PART, u8::schema()), (ARTICLE, u16::schema()), (PARAGRAPH, u16::schema())])));
        schemas.push((<BookId as ToSchema>::name().to_string(), BookId::schema()));
    }
}

fn subtype<const N: usize>(corpus: &Subtype, parts: [(&str, RefOr<Schema>); N]) -> RefOr<Schema> {
    let own = parts.into_iter().fold(ObjectBuilder::new().schema_type(SchemaType::Type(Type::Object)), |object, (name, part)| object.property(name, part).required(name));
    AllOfBuilder::new()
        .item(Ref::from_schema_name(TextRef::name()))
        .item(own)
        .description(Some(corpus.description))
        .extensions(Some(Extensions::from_iter([("unevaluatedProperties", false)])))
        .into()
}

/// A place in a corpus's text: a unit, and where one word of it is named, that
/// word.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextPoint {
    pub unit: TextRef,
    /// The word's position in the unit's base text, counting from zero; absent
    /// where the point is the unit's own edge, so that the span it bounds takes
    /// the unit whole.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<u16>,
}

/// A stretch of text, from one point through another, both included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub from: TextPoint,
    pub to: TextPoint,
}

impl TextSpan {
    pub fn whole(unit: TextRef) -> TextSpan {
        TextSpan { from: TextPoint { unit: unit.clone(), word: None }, to: TextPoint { unit, word: None } }
    }

    /// A word on the wire always counts through its unit's base text, so a span
    /// counted in any other layer's words cannot be sent as one.
    pub fn of_bible_range(range: &BibleLocusRange) -> Result<TextSpan, ForeignLayer> {
        Ok(TextSpan {
            from: TextPoint { unit: TextRef::of_verse(&range.from.unit), word: base_word(range.from.span.as_ref(), |words| words.start)? },
            to: TextPoint { unit: TextRef::of_verse(&range.to.unit), word: base_word(range.to.span.as_ref(), |words| words.end)? },
        })
    }
}

fn base_word(words: Option<&TokenSpan>, edge: fn(&TokenSpan) -> u16) -> Result<Option<u16>, ForeignLayer> {
    words.map(|words| if words.layer.0 == KJV_TRANSLATION { Ok(edge(words)) } else { Err(ForeignLayer { layer: words.layer.clone() }) }).transpose()
}

/// A span whose words were counted in a layer other than its unit's base text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignLayer {
    pub layer: TranslationId,
}

/// A stretch of a unit's text that stands for something the graph holds,
/// half-open: the text from `start` up to but not including `end`.
#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    /// A character offset (a Unicode scalar value, not a byte) into the owning unit's text.
    pub start: usize,
    /// A character offset (a Unicode scalar value, not a byte) into the owning unit's text.
    pub end: usize,
    pub kind: EdgeKind,
    pub node: NodeRef,
}
