use serde::Serialize;

use atlas_core::wire::NodeRef;
use atlas_core::xrefs::AggregatedXref;
use atlas_graph::heading::Heading;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One chapter of Scripture, verse by verse.")]
pub struct Chapter {
    pub r#ref: super::ChapterReference,
    pub book: String,
    pub chapter: u16,
    pub verses: Vec<Verse>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One verse: its text, and what the graph attests at it.")]
pub struct Verse {
    pub verse: u16,
    pub text: String,
    pub places: Vec<PlaceRef>,
    pub persons: Vec<PersonRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
    pub xref_count: usize,
    pub words_of_christ: Vec<WordsOfChristSpan>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A place named by something else on this response: its id, to target on the map, the name to show for it, and its node in the graph.")]
pub struct PlaceRef {
    pub id: String,
    pub name: String,
    pub node: NodeRef,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A person the graph attests at a verse -- never a name matched against the text -- with the id to read and the name to show for it.")]
pub struct PersonRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One span of the words of Christ within a verse's text: character offsets into it, half-open, so the span is the text from `start` up to but not including `end`.")]
pub struct WordsOfChristSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "Every verse of one chapter that carries commentary, in canon order.")]
pub struct KretzmannChapter {
    pub verses: Vec<KretzmannChapterVerse>,
    pub version: super::ArtifactRoot,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One verse's commentary items, in document order. Only verses with at least one item appear at all.")]
pub struct KretzmannChapterVerse {
    pub verse: u16,
    pub items: Vec<KretzmannChapterItem>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One commentary item: the id that fetches its prose, and its heading.")]
pub struct KretzmannChapterItem {
    pub id: super::NodeId,
    pub heading: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One cross reference: where it points, how strongly it is attested, and a look at the text there.")]
pub struct CrossRef {
    pub target: super::CrossReferenceTarget,
    pub votes: i32,
    pub preview: String,
    pub provenance: Vec<super::Provenance>,
}

impl CrossRef {
    pub(crate) fn attributed(xref: AggregatedXref, provenance: &[super::Provenance]) -> Self {
        CrossRef { target: xref.target, votes: xref.votes, preview: xref.preview, provenance: provenance.to_vec() }
    }
}
