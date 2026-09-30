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

struct Subtype {
    tag: &'static str,
    name: &'static str,
    description: &'static str,
}

const BIBLE: Subtype = Subtype { tag: "bible", name: "BibleRef", description: "A verse of the Bible." };
const CONCORD: Subtype = Subtype { tag: "concord", name: "ConcordRef", description: "A paragraph of the Book of Concord." };
const SUBTYPES: [Subtype; 2] = [BIBLE, CONCORD];

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A place in a corpus's text: a unit, and where one word of it is named, that word's position in the unit's base text counting from zero; without a word the point is the unit's own edge, so a span it bounds takes the unit whole.")]
pub struct TextPoint {
    pub unit: TextRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A stretch of text, from one point through another, both included.")]
pub struct TextSpan {
    pub from: TextPoint,
    pub to: TextPoint,
}

impl TextSpan {
    pub fn whole(unit: TextRef) -> TextSpan {
        TextSpan { from: TextPoint { unit: unit.clone(), word: None }, to: TextPoint { unit, word: None } }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignLayer {
    pub layer: TranslationId,
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A stretch of a unit's text that stands for something the graph holds, half-open: the text from `start` up to but not including `end`, both counted in characters (Unicode scalar values, not bytes) into the unit's text.")]
pub struct Anchor {
    pub start: usize,
    pub end: usize,
    pub kind: EdgeKind,
    pub node: NodeRef,
}
