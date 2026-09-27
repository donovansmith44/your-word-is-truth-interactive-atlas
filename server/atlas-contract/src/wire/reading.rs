use serde::Serialize;

use atlas_core::data::BookMeta;
use atlas_core::time::TimeRange;
use atlas_core::wire::VerseGroup;
use atlas_core::xrefs::AggregatedXref;
use atlas_graph::heading::Heading;

use super::catechism::CatechismRef;

/// One chapter of Scripture, verse by verse.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Chapter {
    #[serde(rename = "ref")]
    pub sref: String,
    /// The book's full name.
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<Verse>,
}

/// One verse: its text, and what the graph attests at it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Verse {
    pub verse: u16,
    pub text: String,
    /// The places this verse names, in a stable order. Empty for most verses.
    pub places: Vec<PlaceRef>,
    /// The people this verse names. Empty for most verses.
    pub persons: Vec<PersonRef>,
    /// The heading that belongs above this verse, present only where the verse opens
    /// one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
    /// How many cross references start at this verse; 0 for most of them.
    pub xref_count: usize,
    /// The spans of `text` that are the words of Christ, in order. Empty for most
    /// verses.
    pub words_of_christ: Vec<WordsOfChristSpan>,
}

/// A place named by something else on this response: its id, to explore or to
/// target on the map, and the name to show for it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceRef {
    pub id: String,
    pub name: String,
}

/// A person the graph attests at a verse -- never a name matched against the
/// text -- with the id to explore and the name to show for it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonRef {
    pub id: String,
    pub name: String,
}

/// One span of the words of Christ within a verse's text: character offsets into
/// it, half-open, so the span is the text from `start` up to but not including
/// `end`.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WordsOfChristSpan {
    pub start: usize,
    pub end: usize,
}

/// Every verse of one chapter that carries commentary, in canon order.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct KretzmannChapter {
    pub verses: Vec<KretzmannChapterVerse>,
    /// A stamp identifying the data set this commentary was read from.
    pub version: String,
}

/// One verse's commentary items, in document order. Only verses with at least
/// one item appear at all.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct KretzmannChapterVerse {
    pub verse: u16,
    pub items: Vec<KretzmannChapterItem>,
}

/// One commentary item: the id that fetches its prose, and its heading.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct KretzmannChapterItem {
    pub id: String,
    /// Absent for an item with no heading of its own.
    pub heading: Option<String>,
}

/// One verse in full, with everything the graph attaches to it.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct VerseDetail {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
    /// The spans of `text` that are the words of Christ, in order. Empty for most
    /// verses.
    pub words_of_christ: Vec<WordsOfChristSpan>,
    /// Who wrote this verse's book, where and when.
    pub book_meta: BookMeta,
    /// The events and titled passages this verse belongs to, oldest first.
    pub events: Vec<VerseEvent>,
    /// This verse's cross references, most strongly attested first.
    pub cross_refs: Vec<CrossRef>,
    /// The catechism items citing this verse. Empty for most verses.
    pub catechism: Vec<CatechismRef>,
    /// The id of the source this verse's text comes from; `/api/sources` names it.
    pub provenance: String,
    /// The sources behind the cross references above. Always present, and empty when
    /// there are none.
    pub cross_refs_provenance: Vec<String>,
    /// The sources behind the catechism citations above.
    pub catechism_provenance: Vec<String>,
}

/// One event or titled passage a verse belongs to.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct VerseEvent {
    pub id: String,
    pub label: String,
    /// The years the event spans; absent for a titled passage that has no date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<TimeRange>,
    /// The passages it is narrated in, grouped by book and chapter.
    pub verse_groups: Vec<VerseGroup>,
    /// The ids of the places it touches.
    pub places: Vec<String>,
    /// `event` for something that happened at a date, `general` for a titled
    /// passage that has none.
    pub kind: String,
    /// The id of the source that asserts the event this verse belongs to;
    /// `/api/sources` names it.
    pub provenance: String,
}

/// One cross reference: where it points, how strongly it is attested, and a
/// look at the text there.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CrossRef {
    /// The reference pointed at: one verse, or a span.
    pub target: String,
    /// How many readers voted for this reference. Higher sorts first.
    pub votes: i32,
    /// The text of the target's first verse.
    pub preview: String,
    /// The sources behind this response's cross references. Every element of one
    /// response carries the same set; `/api/sources` names each id.
    pub provenance: Vec<String>,
}

impl CrossRef {
    pub(crate) fn attributed(xref: AggregatedXref, provenance: &[String]) -> Self {
        CrossRef { target: xref.target, votes: xref.votes, preview: xref.preview, provenance: provenance.to_vec() }
    }
}
