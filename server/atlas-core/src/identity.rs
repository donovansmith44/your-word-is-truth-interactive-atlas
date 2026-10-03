use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use atlas_graph_types::adjacency::Cursor;
use atlas_graph_types::edge::EdgeId;
use atlas_graph_types::id::{AnyNodeId, ContentHash, NodeKind, PlaceId};
use atlas_graph_types::store::{GraphQuery, GraphVersion};
use serde::{Serialize, Serializer};
use utoipa::openapi::schema::{ObjectBuilder, OneOfBuilder, SchemaType, Type};
use utoipa::openapi::{Ref, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

use crate::refs::{BookId, ScriptureRef, VerseId};

macro_rules! string_identity {
    ($name:ident, $description:expr, $pattern:expr) => {
        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.collect_str(self)
            }
        }

        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> {
                string_schema($description, $pattern)
            }
        }

        impl ToSchema for $name {}
    };
}

macro_rules! page_cursor {
    ($name:ident, $description:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Cursor);

        impl $name {
            pub const FIRST: $name = $name(Cursor::FIRST);

            pub fn at(cursor: Cursor) -> $name {
                $name(cursor)
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                self.0 .0.serialize(s)
            }
        }

        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> {
                ObjectBuilder::new()
                    .schema_type(SchemaType::Type(Type::Integer))
                    .description(Some($description))
                    .minimum(Some(0))
                    .default(Some(serde_json::json!($name::FIRST.0 .0)))
                    .into()
            }
        }

        impl ToSchema for $name {}
    };
}

macro_rules! wire_union {
    ($name:ident, $description:expr, { $($case:ident($member:ty)),+ $(,)? }) => {
        #[derive(Clone, Debug, PartialEq, Eq)]
        pub enum $name {
            $($case($member)),+
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                match self {
                    $($name::$case(member) => member.serialize(s)),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self {
                    $($name::$case(member) => member.fmt(f)),+
                }
            }
        }

        $(impl From<$member> for $name {
            fn from(member: $member) -> $name {
                $name::$case(member)
            }
        })+

        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> {
                one_of($description, &[$(<$member as ToSchema>::name()),+])
            }
        }

        impl ToSchema for $name {
            fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
                $(schemas.push((<$member as ToSchema>::name().to_string(), <$member as PartialSchema>::schema()));)+
            }
        }
    };
}

pub(crate) fn string_schema(description: &str, pattern: String) -> RefOr<Schema> {
    ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)).description(Some(description)).pattern(Some(pattern)).into()
}

pub(crate) fn one_of(description: &str, members: &[std::borrow::Cow<'static, str>]) -> RefOr<Schema> {
    members.iter().fold(OneOfBuilder::new(), |union, member| union.item(Ref::from_schema_name(member.as_ref()))).description(Some(description)).into()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NamesNoReference;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtifactRoot(GraphVersion);

impl ArtifactRoot {
    pub fn of(version: GraphVersion) -> ArtifactRoot {
        ArtifactRoot(version)
    }
}

impl fmt::Display for ArtifactRoot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0 .0.hex())
    }
}

const ARTIFACT_ROOT: &str = "The root of the compiled artifact a response was read from: two responses that carry the same root were read from the same data.";

string_identity!(ArtifactRoot, ARTIFACT_ROOT, format!("^[0-9a-f]{{{}}}$", ContentHash::HEX_WIDTH));


page_cursor!(EdgePageCursor, "Where a page of one position's neighbours resumes, as an edge page's `next` or `previous` hands it back; the first page is 0.");


page_cursor!(ElementPageCursor, "Where a read of many elements resumes in the list of ids it was asked, as an element page's `next` or `previous` hands it back; the first page is 0.");


#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Deserialize)]
#[serde(transparent)]
pub struct NodeId(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnreferencedUnit(pub AnyNodeId);

impl NodeId {
    pub const TEXT_UNIT: &'static str = "text-unit";

    pub const UNADDRESSABLE: [NodeKind; 3] = [NodeKind::TextUnit, NodeKind::Source, NodeKind::PeopleGroup];

    pub fn addressable() -> impl Iterator<Item = NodeKind> {
        NodeKind::ALL.into_iter().filter(|kind| !NodeId::UNADDRESSABLE.contains(kind))
    }

    pub fn encoded(ids: &[AnyNodeId], query: &(impl GraphQuery + ?Sized)) -> Result<Vec<NodeId>, UnreferencedUnit> {
        let units: Vec<AnyNodeId> = ids.iter().filter(|id| id.kind == NodeKind::TextUnit).cloned().collect();
        let mut references = query.references(&units).into_iter();
        ids.iter()
            .map(|id| match id.kind {
                NodeKind::TextUnit => references.next().flatten().map(|reference| NodeId(format!("{}:{reference}", NodeId::TEXT_UNIT))).ok_or_else(|| UnreferencedUnit(id.clone())),
                _ => Ok(NodeId::named(id)),
            })
            .collect()
    }

    pub fn of_place(place: &PlaceId) -> NodeId {
        NodeId::named(&place.erase())
    }

    fn named(id: &AnyNodeId) -> NodeId {
        NodeId(format!("{:?}:{}", id.kind, id.raw))
    }

    pub fn encoded_one(id: &AnyNodeId, query: &(impl GraphQuery + ?Sized)) -> Result<NodeId, UnreferencedUnit> {
        NodeId::encoded(std::slice::from_ref(id), query).map(|mut encoded| encoded.remove(0))
    }

    pub fn asked(accepted: &str) -> NodeId {
        NodeId(accepted.to_string())
    }

    fn pattern() -> String {
        let kinds: Vec<&str> = NodeId::addressable().map(|kind| kind.name()).collect();
        format!("^(?:{}:(?:{}|{})|(?:{}):.+)$", NodeId::TEXT_UNIT, verse_shape(), concord_shape(), kinds.join("|"))
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

const NODE_ID: &str = "The id of one node of the graph, as every response names it and every read takes it: its kind, a colon, and its name within that kind -- or, for a unit of text, `text-unit:` and the unit's reference.";

string_identity!(NodeId, NODE_ID, NodeId::pattern());



wire_union!(ElementId, "A node's id or an edge's id: the form the element read and a neighbour page take a position in. The two never share a prefix.", { Node(NodeId), Edge(EdgeId) });

const BOOK_SEPARATOR: char = '.';
const RUN_SEPARATOR: char = '-';
const CONCORD_PREFIX: &str = "BoC ";
const COUNTED_FROM_ONE: &str = "[1-9][0-9]*";
const COUNTED_FROM_ZERO: &str = "[0-9]+";

fn book_shape() -> String {
    let codes: Vec<&str> = crate::canon::BOOKS.iter().map(|book| book.code).collect();
    format!("(?:{})", codes.join("|"))
}

fn chapter_shape() -> String {
    format!(r"{}\.{COUNTED_FROM_ONE}", book_shape())
}

pub(crate) fn verse_shape() -> String {
    format!(r"{}\.{COUNTED_FROM_ONE}", chapter_shape())
}

fn passage_shape() -> String {
    format!("{}{RUN_SEPARATOR}{COUNTED_FROM_ONE}", verse_shape())
}

fn verse_range_shape() -> String {
    format!("{}{RUN_SEPARATOR}{}", verse_shape(), verse_shape())
}

fn concord_shape() -> String {
    format!(r"{CONCORD_PREFIX}{COUNTED_FROM_ZERO}\.{COUNTED_FROM_ZERO}\.{COUNTED_FROM_ZERO}")
}

pub(crate) fn whole(shape: String) -> String {
    format!("^{shape}$")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChapterReference {
    pub book: BookId,
    pub chapter: u16,
}

impl fmt::Display for ChapterReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{BOOK_SEPARATOR}{}", self.book.code(), self.chapter)
    }
}

impl FromStr for ChapterReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Chapter(chapter)) => Ok(chapter),
            _ => Err(NamesNoReference),
        }
    }
}

string_identity!(ChapterReference, "One whole chapter of the Bible, as `BOOK.CHAPTER`.", whole(chapter_shape()));


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassageReference {
    pub book: BookId,
    pub chapter: u16,
    pub from_verse: u16,
    pub to_verse: u16,
}

impl PassageReference {
    pub fn first_and_last(&self) -> (VerseId, VerseId) {
        (VerseId { book: self.book, chapter: self.chapter, verse: self.from_verse }, VerseId { book: self.book, chapter: self.chapter, verse: self.to_verse })
    }
}

impl fmt::Display for PassageReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{BOOK_SEPARATOR}{}{BOOK_SEPARATOR}{}{RUN_SEPARATOR}{}", self.book.code(), self.chapter, self.from_verse, self.to_verse)
    }
}

impl FromStr for PassageReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Passage(passage)) => Ok(passage),
            _ => Err(NamesNoReference),
        }
    }
}

string_identity!(PassageReference, "A run of verses within one chapter of the Bible, as `BOOK.CHAPTER.FIRST-LAST`, the first verse before the last.", whole(passage_shape()));


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerseRangeReference {
    pub from: VerseId,
    pub to: VerseId,
}

impl fmt::Display for VerseRangeReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{RUN_SEPARATOR}{}", self.from, self.to)
    }
}

impl FromStr for VerseRangeReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        let (from, to) = raw.split_once(RUN_SEPARATOR).ok_or(NamesNoReference)?;
        Ok(VerseRangeReference { from: VerseId::parse_canonical(from).map_err(|_| NamesNoReference)?, to: VerseId::parse_canonical(to).map_err(|_| NamesNoReference)? })
    }
}

string_identity!(VerseRangeReference, "A run of verses that crosses a chapter boundary, as two verse references joined by `-`.", whole(verse_range_shape()));


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConcordReference {
    pub part: u8,
    pub article: u16,
    pub paragraph: u16,
}

impl fmt::Display for ConcordReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{CONCORD_PREFIX}{}{BOOK_SEPARATOR}{}{BOOK_SEPARATOR}{}", self.part, self.article, self.paragraph)
    }
}

impl FromStr for ConcordReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        let rest = raw.strip_prefix(CONCORD_PREFIX).ok_or(NamesNoReference)?;
        let mut numbers = rest.split(BOOK_SEPARATOR);
        let (Some(part), Some(article), Some(paragraph), None) = (numbers.next(), numbers.next(), numbers.next(), numbers.next()) else {
            return Err(NamesNoReference);
        };
        Ok(ConcordReference {
            part: part.parse().map_err(|_| NamesNoReference)?,
            article: article.parse().map_err(|_| NamesNoReference)?,
            paragraph: paragraph.parse().map_err(|_| NamesNoReference)?,
        })
    }
}

string_identity!(ConcordReference, "One paragraph of the Book of Concord, as `BoC PART.ARTICLE.PARAGRAPH`.", whole(concord_shape()));




wire_union!(VerseSpanReference, "One verse, or a run of verses within one chapter.", { Verse(VerseId), Passage(PassageReference) });

impl VerseSpanReference {
    pub fn scripture(&self) -> ScriptureRef {
        match *self {
            VerseSpanReference::Verse(verse) => ScriptureRef::Verse(verse),
            VerseSpanReference::Passage(passage) => ScriptureRef::Passage(passage),
        }
    }
}

impl FromStr for VerseSpanReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Verse(verse)) => Ok(verse.into()),
            Ok(ScriptureRef::Passage(passage)) => Ok(passage.into()),
            _ => Err(NamesNoReference),
        }
    }
}

wire_union!(CrossReferenceTarget, "Where a cross reference points: one verse, a run of verses within a chapter, or a run that crosses chapters.", { Verse(VerseId), Passage(PassageReference), Range(VerseRangeReference) });

impl CrossReferenceTarget {
    pub fn first_and_last(&self) -> (VerseId, VerseId) {
        match self {
            CrossReferenceTarget::Verse(verse) => (*verse, *verse),
            CrossReferenceTarget::Passage(passage) => passage.first_and_last(),
            CrossReferenceTarget::Range(range) => (range.from, range.to),
        }
    }
}

impl FromStr for CrossReferenceTarget {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Verse(verse)) => Ok(verse.into()),
            Ok(ScriptureRef::Passage(passage)) => Ok(passage.into()),
            _ => raw.parse::<VerseRangeReference>().map(CrossReferenceTarget::from),
        }
    }
}

wire_union!(ReadingReference, "Where a reading of the Bible opens: a verse, or a whole chapter.", { Verse(VerseId), Chapter(ChapterReference) });

impl ReadingReference {
    pub fn chapter(&self) -> ChapterReference {
        match *self {
            ReadingReference::Verse(verse) => ChapterReference { book: verse.book, chapter: verse.chapter },
            ReadingReference::Chapter(chapter) => chapter,
        }
    }

    pub fn verse(&self) -> Option<u16> {
        match *self {
            ReadingReference::Verse(verse) => Some(verse.verse),
            ReadingReference::Chapter(_) => None,
        }
    }
}

impl FromStr for ReadingReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match ScriptureRef::parse(raw) {
            Ok(ScriptureRef::Verse(verse)) => Ok(verse.into()),
            Ok(ScriptureRef::Chapter(chapter)) => Ok(chapter.into()),
            _ => Err(NamesNoReference),
        }
    }
}

wire_union!(UnitReference, "One unit of a corpus's text by its reference: a verse of the Bible, or a paragraph of the Book of Concord.", { Verse(VerseId), Concord(ConcordReference) });

impl FromStr for UnitReference {
    type Err = NamesNoReference;

    fn from_str(raw: &str) -> Result<Self, NamesNoReference> {
        match raw.parse::<ConcordReference>() {
            Ok(paragraph) => Ok(paragraph.into()),
            Err(_) => VerseId::parse_canonical(raw).map(UnitReference::from).map_err(|_| NamesNoReference),
        }
    }
}

wire_union!(ContentsReference, "The reference a contents entry opens at: a chapter of the Bible, or a paragraph of the Book of Concord.", { Chapter(ChapterReference), Concord(ConcordReference) });

wire_union!(TextWindowReference, "Where a window of a corpus's reading spine opens: a verse or a chapter of the Bible, or a paragraph of the Book of Concord.", { Verse(VerseId), Chapter(ChapterReference), Concord(ConcordReference) });

macro_rules! catalogue {
    ($($identity:ty),+ $(,)?) => {
        pub fn schemas() -> Vec<(String, RefOr<Schema>)> {
            vec![$((<$identity as ToSchema>::name().to_string(), <$identity as PartialSchema>::schema())),+]
        }
    };
}

catalogue!(
    ArtifactRoot,
    EdgePageCursor,
    ElementPageCursor,
    NodeId,
    EdgeId,
    ElementId,
    VerseId,
    ChapterReference,
    PassageReference,
    VerseRangeReference,
    ConcordReference,
    ScriptureRef,
    VerseSpanReference,
    CrossReferenceTarget,
    ReadingReference,
    UnitReference,
    ContentsReference,
    TextWindowReference,
);

pub fn members() -> BTreeMap<String, BTreeSet<String>> {
    schemas()
        .into_iter()
        .filter_map(|(name, schema)| match schema {
            RefOr::T(Schema::OneOf(union)) => Some((name, union.items.iter().filter_map(|member| match member {
                RefOr::Ref(reference) => reference.ref_location.rsplit('/').next().map(str::to_string),
                RefOr::T(_) => None,
            }).collect())),
            _ => None,
        })
        .collect()
}

pub fn widenings() -> BTreeSet<(String, String)> {
    let unions = members();
    let into_unions = unions.iter().flat_map(|(union, members)| members.iter().map(move |member| (member.clone(), union.clone())));
    let between_unions = unions.iter().flat_map(|(narrower, within)| {
        unions.iter().filter(move |(wider, around)| wider != &narrower && within.is_subset(around)).map(move |(wider, _)| (narrower.clone(), wider.clone()))
    });
    into_unions.chain(between_unions).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chapter_reference_names_a_whole_chapter_and_nothing_else() {
        // Arrange
        let asked = ["EXO.14", "EXO", "EXO.14.21", "EXO.14.21-31", "nope"];
        // Act
        let read: Vec<Result<ChapterReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(read, vec![Ok(ChapterReference { book: BookId(1), chapter: 14 }), Err(NamesNoReference), Err(NamesNoReference), Err(NamesNoReference), Err(NamesNoReference)]);
    }

    #[test]
    fn a_reading_reference_is_a_verse_or_a_whole_chapter() {
        // Arrange
        let asked = ["JHN.3.16", "JHN.3", "JHN", "JHN.3.16-18", "nope"];
        // Act
        let read: Vec<Result<ReadingReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(
            read,
            vec![
                Ok(ReadingReference::Verse(VerseId { book: BookId(42), chapter: 3, verse: 16 })),
                Ok(ReadingReference::Chapter(ChapterReference { book: BookId(42), chapter: 3 })),
                Err(NamesNoReference),
                Err(NamesNoReference),
                Err(NamesNoReference),
            ]
        );
    }

    #[test]
    fn a_reading_reference_carries_its_chapter_always_and_its_verse_where_it_names_one() {
        // Arrange
        let asked = [ReadingReference::Verse(VerseId { book: BookId(42), chapter: 3, verse: 16 }), ReadingReference::Chapter(ChapterReference { book: BookId(42), chapter: 3 })];
        // Act
        let read: Vec<(ChapterReference, Option<u16>)> = asked.iter().map(|reference| (reference.chapter(), reference.verse())).collect();
        // Assert
        assert_eq!(read, vec![(ChapterReference { book: BookId(42), chapter: 3 }, Some(16)), (ChapterReference { book: BookId(42), chapter: 3 }, None)]);
    }

    #[test]
    fn a_concord_reference_names_a_part_an_article_and_a_paragraph() {
        // Arrange
        let asked = ["BoC 7.2.1", "BoC 7.2", "BoC 7.2.1.4", "BoC 7.2.x", "7.2.1", "nope"];
        // Act
        let read: Vec<Result<ConcordReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(
            read,
            vec![Ok(ConcordReference { part: 7, article: 2, paragraph: 1 }), Err(NamesNoReference), Err(NamesNoReference), Err(NamesNoReference), Err(NamesNoReference), Err(NamesNoReference)]
        );
    }

    #[test]
    fn a_verse_span_reference_names_one_verse_or_a_same_chapter_run_of_them() {
        // Arrange
        let asked = ["JOS.6.20", "JOS.6.20-21", "JOS.6", "JOS", "nope"];
        // Act
        let read: Vec<Result<VerseSpanReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(
            read,
            vec![
                Ok(VerseSpanReference::Verse(VerseId { book: BookId(5), chapter: 6, verse: 20 })),
                Ok(VerseSpanReference::Passage(PassageReference { book: BookId(5), chapter: 6, from_verse: 20, to_verse: 21 })),
                Err(NamesNoReference),
                Err(NamesNoReference),
                Err(NamesNoReference),
            ]
        );
    }

    #[test]
    fn a_cross_reference_target_is_a_verse_a_same_chapter_run_or_a_run_across_chapters() {
        // Arrange
        let asked = ["PSA.124.8", "COL.1.16-19", "MAT.5.3-MAT.6.2", "GEN.1", "garbage"];
        // Act
        let read: Vec<Option<(VerseId, VerseId)>> = asked.iter().map(|raw| raw.parse::<CrossReferenceTarget>().ok().map(|target| target.first_and_last())).collect();
        // Assert
        assert_eq!(
            read,
            vec![
                Some((VerseId { book: BookId(18), chapter: 124, verse: 8 }, VerseId { book: BookId(18), chapter: 124, verse: 8 })),
                Some((VerseId { book: BookId(50), chapter: 1, verse: 16 }, VerseId { book: BookId(50), chapter: 1, verse: 19 })),
                Some((VerseId { book: BookId(39), chapter: 5, verse: 3 }, VerseId { book: BookId(39), chapter: 6, verse: 2 })),
                None,
                None,
            ]
        );
    }

    #[test]
    fn a_unit_reference_is_a_verse_of_the_bible_or_a_paragraph_of_the_book_of_concord() {
        // Arrange
        let asked = ["JHN.3.16", "BoC 4.17.70", "JHN.3", "nope"];
        // Act
        let read: Vec<Result<UnitReference, NamesNoReference>> = asked.iter().map(|raw| raw.parse()).collect();
        // Assert
        assert_eq!(
            read,
            vec![
                Ok(UnitReference::Verse(VerseId { book: BookId(42), chapter: 3, verse: 16 })),
                Ok(UnitReference::Concord(ConcordReference { part: 4, article: 17, paragraph: 70 })),
                Err(NamesNoReference),
                Err(NamesNoReference),
            ]
        );
    }

    #[test]
    fn only_a_node_of_a_kind_a_caller_can_ask_for_is_addressable() {
        // Arrange
        let every = NodeKind::ALL;
        // Act
        let unaddressable: Vec<NodeKind> = every.into_iter().filter(|kind| !NodeId::addressable().any(|addressable| addressable == *kind)).collect();
        // Assert
        assert_eq!(unaddressable, vec![NodeKind::TextUnit, NodeKind::Source, NodeKind::PeopleGroup]);
    }
}
