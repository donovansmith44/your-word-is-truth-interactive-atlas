use atlas_core::data::EventKind;
use atlas_graph::heading::Heading;
use atlas_graph_types::id::NarrativeId;
use atlas_graph_types::{EdgeKind, NodeKind};
use serde::{Serialize, Serializer};
use utoipa::openapi::{RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

use super::TextSpan;

/// One node of the graph at a glance: what it is, what to call it, where it
/// came from, and what it connects to.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeCard {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    /// The id of the source that asserts this node; `/api/sources` names it.
    pub provenance: String,
    /// How many neighbours this node has of each kind, listing only the kinds it
    /// has any of.
    pub edge_summary: Vec<EdgeSummaryEntry>,
    /// A stamp identifying the data set this card was read from.
    pub version: String,
    /// Life facts, present only for a person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<PersonLife>,
    /// Public-domain dictionary or commentary prose about this node, absent when
    /// none is recorded. Never invented.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What is recorded about an event or a titled passage, present only for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventDetail>,
    /// Where a place is and what is recorded of its history, present only for a place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<PlaceDetail>,
    /// A catechism item's own words, present only for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catechism: Option<CatechismDetail>,
    /// Who wrote a book of the Bible, where and when, present only for a book this
    /// atlas records that of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<BookDetail>,
}

/// What is recorded about one person's life.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PersonLife {
    /// Absent when the source records none.
    pub gender: Option<String>,
    /// The year of birth where one is recorded; most people have none.
    pub birth: Option<super::Year>,
    /// The year of death where one is recorded.
    pub death: Option<super::Year>,
    /// The earliest year at which this person is mentioned -- the span of mentions,
    /// never a lifespan.
    pub first: Option<super::Year>,
    /// The latest year at which this person is mentioned.
    pub last: Option<super::Year>,
    /// True for a person Scripture presents as eternal, who therefore carries no
    /// years at all.
    pub eternal: bool,
    /// The verses on which that is claimed.
    pub eternal_grounds: Vec<String>,
    /// Other names this person is known by.
    pub also_called: Vec<String>,
}

/// What is recorded about one event or titled passage.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventDetail {
    pub kind: EventKind,
    /// The years the event spans; absent for a titled passage, which has no date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<super::TimeRange>,
    /// The section of Robertson's Harmony of the Gospels this event falls in, absent
    /// when it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    /// The section of the outline of Acts this event falls in, absent when it has
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    /// The titled section of this atlas's own outline of Scripture, absent when it
    /// has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    /// The superscription the King James Version prints over this passage, absent
    /// when it prints none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    /// A note on how this event's date and grouping were arrived at, absent when
    /// none was needed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
}

/// Where one place is, the name a reader of the King James Version knows it by, and
/// when it was founded and fell, where that is recorded.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlaceDetail {
    /// Latitude in degrees, north positive.
    pub lat: f64,
    /// Longitude in degrees, east positive.
    pub lon: f64,
    /// The name a reader of the King James Version knows this place by.
    pub display_name: String,
    /// The place's plain canonical name, present only when `display_name` is the
    /// translation's own wording and so differs from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    /// When the place was founded, absent when that is not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established: Option<super::DateClaim>,
    /// When the place was destroyed, absent when that is not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<super::DateClaim>,
}

/// One catechism item's own words and their explanation.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatechismDetail {
    /// The chief part this item belongs to, such as Baptism.
    pub part_title: String,
    /// The item's own words -- a commandment, a petition, an article -- absent for
    /// an item that has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The question the explanation answers, such as "What does this mean?".
    pub explanation_heading: String,
    pub explanation: String,
    /// Where in Scripture the item's own words are written, absent when it quotes
    /// none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
}

/// Who wrote one book of the Bible, where, and when.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BookDetail {
    /// The book's author, as this atlas records him.
    pub author: String,
    /// Where it was written, absent when that is not recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_place: Option<NodeRef>,
    /// The years it was written across, absent when it is not dated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<super::TimeRange>,
}

/// How many neighbours of one kind something has.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeSummaryEntry {
    pub kind: EdgeKind,
    pub count: usize,
}

/// One page of a node's neighbours of a single kind.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgePage {
    pub kind: EdgeKind,
    pub entries: Vec<EdgeEntry>,
    /// Pass this back as `cursor` for the following page; absent on the last page.
    pub next: Option<usize>,
    /// A stamp identifying the data set this page was read from.
    pub version: String,
}

/// One neighbour, with the edge that joins it and what that edge records.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeEntry {
    /// The edge's own id. The neighbour's page for the opposite kind carries this
    /// same id for this same connection, and the edge itself can be explored.
    pub edge: String,
    pub node: NodeRef,
    /// How many readers voted for this cross reference; only a citation carries it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<u32>,
    /// The narrative this leg belongs to; only a narrative's succession carries it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<NarrativeId>,
    /// Where an attestation stands in Scripture: for `attested-in`, the runs of
    /// verses its account reads on without a break; for `attests`, the verse itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loci: Option<Vec<TextSpan>>,
    /// How the account this attestation belongs to is cited, where it needed saying.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A reference to something the graph holds: enough to show it, and the id to
/// fetch it with. The `kind` is one of the graph's node kinds, or `Edge` when
/// the reference is to an edge, which can be explored in its own right.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NodeRef {
    pub id: String,
    pub kind: PositionKind,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PositionKind {
    Node(NodeKind),
    Edge,
}

/// The description this vocabulary publishes. It is a constant rather than a doc
/// comment because the schema below is hand-written, and a doc comment would be a
/// second copy of the same sentence.
const POSITION_KIND: &str = "What a reference in this atlas names: one kind of node, or an edge, which takes focus in its own right and so is a kind of its own here.";

impl PositionKind {
    pub fn name(self) -> &'static str {
        match self {
            PositionKind::Node(kind) => kind.name(),
            PositionKind::Edge => "Edge",
        }
    }
}

// Written out rather than declared through `vocabulary!`: one member of this set
// carries another whole vocabulary, which a flat member list cannot express.
impl Serialize for PositionKind {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl PartialSchema for PositionKind {
    fn schema() -> RefOr<Schema> {
        let names = NodeKind::ALL.iter().map(|kind| kind.name()).chain(std::iter::once(PositionKind::Edge.name()));
        atlas_graph_types::vocabulary::string_enum(names, POSITION_KIND.to_string())
    }
}

impl ToSchema for PositionKind {}

atlas_graph_types::vocabulary! {
    /// How much text one window of `/api/text` covers: the units around the
    /// reference asked for, or the whole chapter that reference names.
    TextScope {
        Verse => "verse",
        Chapter => "chapter",
    }
}

/// A window of one corpus's reading spine.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextWindow {
    pub units: Vec<TextUnit>,
    /// The reference one step further on in the direction travelled; absent at the
    /// end of the corpus.
    pub next: Option<String>,
    /// A stamp identifying the data set this window was read from.
    pub version: String,
}

/// One unit of a reading spine: a verse of Scripture, or a paragraph of the
/// Book of Concord.
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextUnit {
    pub r#ref: String,
    /// The same unit as `ref`, by its parts.
    pub locus: super::TextRef,
    pub text: String,
    /// The spans of `text` that are the words of Christ, in order. Always empty
    /// outside Scripture.
    pub words_of_christ: Vec<super::reading::WordsOfChristSpan>,
    /// The pericope heading that belongs above this verse, present only where one
    /// opens or carries on. Always absent outside Scripture.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<Heading>,
    /// How many neighbours this unit has of each kind, so a page can tell which
    /// units lead somewhere without asking after each one.
    pub edge_summary: Vec<EdgeSummaryEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_scope_round_trips_through_the_two_spans_a_window_can_cover() {
        // Arrange
        let every_variant = TextScope::ALL;
        // Act
        let json = serde_json::to_string(&every_variant).unwrap();
        let back: Vec<TextScope> = serde_json::from_str(&json).unwrap();
        // Assert
        assert_eq!(json, r#"["verse","chapter"]"#);
        assert_eq!(back, every_variant.to_vec());
    }

    #[test]
    fn an_unrecognised_scope_names_no_span_at_all() {
        // Arrange
        let requested = ["verse", "chapter", "paragraph"];
        // Act
        let resolved: Vec<Option<TextScope>> = requested.iter().map(|name| TextScope::named(name)).collect();
        // Assert
        assert_eq!(resolved, vec![Some(TextScope::Verse), Some(TextScope::Chapter), None]);
    }
}
