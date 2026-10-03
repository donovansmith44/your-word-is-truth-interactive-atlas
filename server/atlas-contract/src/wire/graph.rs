use atlas_core::data::EventKind;
use atlas_graph_types::id::NarrativeId;
use atlas_graph_types::{EdgeKind, NodeKind};
use serde::ser::SerializeMap;
use serde::{Serialize, Serializer};
use utoipa::openapi::{Ref, RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

pub use atlas_core::wire::NodeRef;

use super::union::{case_of, tagged_by, Case};
use super::TextSpan;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One node of the graph at a glance: what it is, what to call it, where it came from, and what it connects to. `version` stamps the data set it was read from.")]
pub struct NodeRecord {
    pub id: String,
    pub kind: NodeKind,
    pub label: String,
    pub provenance: String,
    pub edge_summary: Vec<EdgeSummaryEntry>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub person: Option<PersonLife>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<EventDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<PlaceDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catechism: Option<CatechismDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub book: Option<BookDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map: Option<MapDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era: Option<EraDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub polity: Option<PolityDetail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<UnitText>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The years one map of the world shows, labelled.")]
pub struct MapDetail {
    pub window: super::TimeRange,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The years one era of this atlas's timeline spans, labelled.")]
pub struct EraDetail {
    pub window: super::TimeRange,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The years one polity stood, from the first year of its first era to the last year of its last, labelled.")]
pub struct PolityDetail {
    pub reign: super::TimeRange,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "What is recorded about one person's life.")]
pub struct PersonLife {
    pub gender: Option<String>,
    pub birth: Option<super::Year>,
    pub death: Option<super::Year>,
    pub first: Option<super::Year>,
    pub last: Option<super::Year>,
    pub eternal: bool,
    pub eternal_grounds: Vec<String>,
    pub also_called: Vec<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "What is recorded about one event or titled passage. `when` is absent for a titled passage, which has no date; each section, the superscription and the note are absent where none is recorded.")]
pub struct EventDetail {
    pub kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<super::TimeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acts_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_section: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kjv_superscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "Where one place is (latitude north positive, longitude east positive), the name a reader of the King James Version knows it by, its plain canonical name only where that differs, a sentence on its history across every period recorded for it, and when it was founded and fell, each only where recorded.")]
pub struct PlaceDetail {
    pub lat: f64,
    pub lon: f64,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blurb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub established: Option<super::DateClaim>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destroyed: Option<super::DateClaim>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One catechism item's own words and their explanation: the chief part it belongs to, the question the explanation answers, and where in Scripture its words are written, where it quotes any.")]
pub struct CatechismDetail {
    pub part_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub explanation_heading: String,
    pub explanation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub where_written: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "Who wrote one book of the Bible, where, and when; the place and the years are absent where they are not recorded.")]
pub struct BookDetail {
    pub author: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_place: Option<NodeRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub written: Option<super::TimeRange>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "How many neighbours of one kind something has.")]
pub struct EdgeSummaryEntry {
    pub kind: EdgeKind,
    pub count: usize,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One page of a node's neighbours of a single kind.")]
pub struct EdgePage {
    pub kind: EdgeKind,
    pub entries: Vec<EdgeEntry>,
    pub previous: Option<usize>,
    pub next: Option<usize>,
    pub version: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One neighbour, with the edge that joins it and what that edge records: `neighbour` is what the edge leads to, a node or, on a `justifies` page, the edge the node grounds; `votes` only on a cross reference, `narrative` only on a narrative's succession, `loci` on an attestation (for `attested-in`, the runs of verses its account reads on without a break; for `attests`, the verse itself) and on a mention (each occurrence of the name in the verse as a span of its words, absent where the name is not found among the verse's words), `note` only on an attestation, `parentage` only on a parent-of edge. A page lists an edge once however many rows record it.")]
pub struct EdgeEntry {
    pub edge: EdgeRef,
    pub neighbour: PositionRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<NarrativeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loci: Option<Vec<TextSpan>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parentage: Option<atlas_graph_types::edge::Parentage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A reference to an edge: its id, which the page of either end carries for this same connection and the element read answers, the kind it is recorded in, and its compiled label.")]
pub struct EdgeRef {
    pub id: String,
    pub kind: EdgeKind,
    pub label: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One edge of the graph: its id, the kind it is recorded in, its compiled label, its two ends in that direction, the source that asserts it (absent for an edge derived from the grounds of the edge it justifies), what it records (`votes` on a cross reference, `narrative` on a narrative's succession, `parentage` on a parent-of edge), and how many neighbours it has of each kind.")]
pub struct EdgeRecord {
    pub id: String,
    pub kind: EdgeKind,
    pub label: String,
    pub subject: PositionRef,
    pub object: PositionRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub votes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrative: Option<NarrativeId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parentage: Option<atlas_graph_types::edge::Parentage>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
}

#[derive(Debug)]
pub enum Element {
    Node { node: NodeRecord },
    Edge { edge: EdgeRecord },
    Missing { id: String },
}

const ELEMENT: &str = "element";
const ELEMENT_DESCRIPTION: &str = "One answer of the element read: the node or the edge an id names, or the id itself where it names nothing. `element` says which shape follows.";
const NODE_ELEMENT: Case = Case { tag: "node", name: "NodeElement", description: "The node an id names, as its own record." };
const EDGE_ELEMENT: Case = Case { tag: "edge", name: "EdgeElement", description: "The edge an id names, as its own record." };
const MISSING_ELEMENT: Case = Case { tag: "missing", name: "MissingElement", description: "An id that reads as a node's or an edge's but names nothing this atlas holds." };
const ELEMENT_CASES: [Case; 3] = [NODE_ELEMENT, EDGE_ELEMENT, MISSING_ELEMENT];
const MISSING_ID: &str = "id";

impl Serialize for Element {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut element = s.serialize_map(None)?;
        match self {
            Element::Node { node } => {
                element.serialize_entry(ELEMENT, NODE_ELEMENT.tag)?;
                element.serialize_entry(NODE_ELEMENT.tag, node)?;
            }
            Element::Edge { edge } => {
                element.serialize_entry(ELEMENT, EDGE_ELEMENT.tag)?;
                element.serialize_entry(EDGE_ELEMENT.tag, edge)?;
            }
            Element::Missing { id } => {
                element.serialize_entry(ELEMENT, MISSING_ELEMENT.tag)?;
                element.serialize_entry(MISSING_ID, id)?;
            }
        }
        element.end()
    }
}

impl PartialSchema for Element {
    fn schema() -> RefOr<Schema> {
        tagged_by(ELEMENT, &ELEMENT_CASES, ELEMENT_DESCRIPTION)
    }
}

impl ToSchema for Element {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((NODE_ELEMENT.name.to_string(), case_of(&Element::name(), &NODE_ELEMENT, [(NODE_ELEMENT.tag, Ref::from_schema_name(NodeRecord::name()).into())])));
        schemas.push((EDGE_ELEMENT.name.to_string(), case_of(&Element::name(), &EDGE_ELEMENT, [(EDGE_ELEMENT.tag, Ref::from_schema_name(EdgeRecord::name()).into())])));
        schemas.push((MISSING_ELEMENT.name.to_string(), case_of(&Element::name(), &MISSING_ELEMENT, [(MISSING_ID, String::schema())])));
        schemas.push((NodeRecord::name().to_string(), NodeRecord::schema()));
        NodeRecord::schemas(schemas);
        schemas.push((EdgeRecord::name().to_string(), EdgeRecord::schema()));
        EdgeRecord::schemas(schemas);
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The elements asked for, in the order their ids were given: `elements[i]` answers the i-th id. At most the server's largest page is answered; `next` continues. `version` stamps the data set they were read from.")]
pub struct ElementPage {
    pub elements: Vec<Element>,
    pub previous: Option<usize>,
    pub next: Option<usize>,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PositionRef {
    Node { node: NodeRef },
    Edge { edge: EdgeRef },
}

const POSITION: &str = "position";
const POSITION_REF: &str = "What an edge leads to: a node, or, where one relation grounds another, the edge it grounds. `position` says which shape follows.";
const NODE: Case = Case { tag: "node", name: "NodePosition", description: "An edge's far end when it is a node." };
const EDGE: Case = Case { tag: "edge", name: "EdgePosition", description: "An edge's far end when it is another edge: a `justifies` page names the edge the node grounds." };
const CASES: [Case; 2] = [NODE, EDGE];

impl Serialize for PositionRef {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut neighbour = s.serialize_map(None)?;
        match self {
            PositionRef::Node { node } => {
                neighbour.serialize_entry(POSITION, NODE.tag)?;
                neighbour.serialize_entry(NODE.tag, node)?;
            }
            PositionRef::Edge { edge } => {
                neighbour.serialize_entry(POSITION, EDGE.tag)?;
                neighbour.serialize_entry(EDGE.tag, edge)?;
            }
        }
        neighbour.end()
    }
}

impl PartialSchema for PositionRef {
    fn schema() -> RefOr<Schema> {
        tagged_by(POSITION, &CASES, POSITION_REF)
    }
}

impl ToSchema for PositionRef {
    fn schemas(schemas: &mut Vec<(String, RefOr<Schema>)>) {
        schemas.push((NODE.name.to_string(), case_of(&PositionRef::name(), &NODE, [(NODE.tag, Ref::from_schema_name(NodeRef::name()).into())])));
        schemas.push((EDGE.name.to_string(), case_of(&PositionRef::name(), &EDGE, [(EDGE.tag, Ref::from_schema_name(EdgeRef::name()).into())])));
        schemas.push((NodeRef::name().to_string(), NodeRef::schema()));
        NodeRef::schemas(schemas);
        schemas.push((EdgeRef::name().to_string(), EdgeRef::schema()));
    }
}

atlas_graph_types::vocabulary! {
    TextScope {
        Verse => "verse",
        Chapter => "chapter",
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "A window of one corpus's reading spine.")]
pub struct TextWindow {
    pub units: Vec<TextUnit>,
    pub next: Option<String>,
    pub version: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "One unit of a reading spine: a verse of Scripture, or a paragraph of the Book of Concord.")]
pub struct TextUnit {
    pub r#ref: String,
    pub node: NodeRef,
    pub body: UnitText,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<UnitHeading>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "What a text unit is: where it stands, its words, the words of Christ in it, and what its words stand for, each anchor in reading order.")]
pub struct UnitText {
    pub locus: super::TextRef,
    pub text: String,
    pub words_of_christ: Vec<super::reading::WordsOfChristSpan>,
    pub anchors: Vec<super::Anchor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(description = "The pericope heading above a verse: the event or titled passage that covers it, whose label is the heading's words; `is_continuation` is true where this verse carries on coverage that began in an earlier chapter rather than opening it, so a reader can render it as a continued heading.")]
pub struct UnitHeading {
    pub event: NodeRef,
    pub kind: EventKind,
    pub is_continuation: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    const HAZOR: &str = "Place:hazor-1";
    const HAZOR_LABEL: &str = "Hazor 1";
    const A_DATING: &str = "DatedBy:00ff";
    const A_DATING_LABEL: &str = "Solomon crowned · Dated by · 970 BC";

    #[test]
    fn a_neighbour_is_written_as_its_position_then_the_node_or_the_edge_it_leads_to() {
        // Arrange
        let neighbours = [
            PositionRef::Node { node: NodeRef { id: HAZOR.to_string(), kind: NodeKind::Place, label: HAZOR_LABEL.to_string() } },
            PositionRef::Edge { edge: EdgeRef { id: A_DATING.to_string(), kind: EdgeKind::Directed(atlas_graph_types::edge::RelationId::DatedBy, atlas_graph_types::edge::Direction::Forward), label: A_DATING_LABEL.to_string() } },
        ];
        // Act
        let written = serde_json::to_value(neighbours).unwrap();
        // Assert
        assert_eq!(
            written,
            serde_json::json!([
                { "position": "node", "node": { "id": HAZOR, "kind": "Place", "label": HAZOR_LABEL } },
                { "position": "edge", "edge": { "id": A_DATING, "kind": "dated-by", "label": A_DATING_LABEL } },
            ])
        );
    }

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
