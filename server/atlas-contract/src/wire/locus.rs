use atlas_core::refs::BookId;
use atlas_graph::kjv_adapter::KJV_TRANSLATION;
use atlas_graph_types::text::{self, BibleLocusRange, TokenSpan, TranslationId, VerseRef};
use atlas_graph_types::EdgeKind;
use serde::{Serialize, Serializer};
use utoipa::ToSchema;

use super::NodeRef;

/// One unit of a corpus's text: a verse of the Bible, or a paragraph of the Book
/// of Concord.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum TextRef {
    Bible {
        /// The book's canon code, such as `GEN`.
        #[serde(serialize_with = "canon_code")]
        book: BookId,
        chapter: u16,
        verse: u16,
    },
    Concord {
        part: u8,
        article: u16,
        paragraph: u16,
    },
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

fn canon_code<S: Serializer>(book: &BookId, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(book.code())
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
