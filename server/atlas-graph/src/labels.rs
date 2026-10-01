use std::collections::BTreeMap;

use atlas_core::data::AtlasData;
use atlas_graph_types::edge::{EdgeId, EdgeKind, EdgeRecord};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnchorId, AnyNodeId, Position};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{ConcordRef, ConcordTag, Corpus};

use crate::geography::Geography;

pub struct ReaderNames<'a> {
    geography: &'a Geography,
    anchors: BTreeMap<AnyNodeId, &'a str>,
}

impl<'a> ReaderNames<'a> {
    pub fn of(geography: &'a Geography, atlas: &'a AtlasData) -> ReaderNames<'a> {
        let anchors = atlas.chronology_anchors.iter().map(|anchor| (AnchorId::new(anchor.id.as_str()).erase(), anchor.label.as_str())).collect();
        ReaderNames { geography, anchors }
    }

    fn of_node(&self, node: &Node) -> Option<String> {
        match &node.payload {
            NodePayload::TextUnit { corpus, .. } => Some(format!("text unit ({corpus})")),
            NodePayload::Container { title } => Some(title.clone()),
            NodePayload::Event { label, .. }
            | NodePayload::Narrative { label, .. }
            | NodePayload::Person { label, .. }
            | NodePayload::Era { label, .. }
            | NodePayload::Map { label, .. }
            | NodePayload::Polity { label, .. }
            | NodePayload::CatechismItem { label }
            | NodePayload::Source { label }
            | NodePayload::Translation { label }
            | NodePayload::PeopleGroup { label, .. } => Some(label.clone()),
            NodePayload::CommentaryItem { heading, .. } => Some(heading.clone().unwrap_or_else(|| "Commentary".to_string())),
            NodePayload::LexiconEntry { lemma, .. } => Some(lemma.clone()),
            NodePayload::Place { .. } => self.geography.place_default(&node.id).map(|default| default.display_name.clone()),
            NodePayload::Anchor { .. } => self.anchors.get(&node.id).map(|label| label.to_string()),
        }
    }
}

pub fn node_label(id: &AnyNodeId, node: Option<&Node>, names: &ReaderNames) -> Option<String> {
    if let Some((book, chapter, verse)) = crate::kjv_adapter::decode_text_unit(id) {
        return Some(crate::kjv_adapter::dot_ref(book, chapter, verse));
    }
    if let Some((part, article, paragraph)) = crate::concord_adapter::decode_text_unit(id) {
        return Some(ConcordTag::cite(&ConcordRef { part, article, paragraph }));
    }
    node.and_then(|n| names.of_node(n))
}

pub fn edge_label(kind: EdgeKind, subject: &str, object: &str) -> String {
    format!("{subject} · {} · {object}", kind.display_label())
}

pub fn compile(graph: &mut Graph, names: &ReaderNames) {
    let edges = graph.edge_records();
    let mut labels = std::mem::take(&mut graph.labels);
    for position in graph.positions().into_iter().chain(edges.keys().map(|id| Position::Edge(id.clone()))) {
        label_of(&position, graph, &edges, names, &mut labels);
    }
    graph.labels = labels;
    graph.edges_by_id = edges;
}

fn label_of(position: &Position, graph: &Graph, edges: &BTreeMap<EdgeId, EdgeRecord>, names: &ReaderNames, labels: &mut BTreeMap<Position, String>) -> Option<String> {
    if let Some(known) = labels.get(position) {
        return Some(known.clone());
    }
    let label = match position {
        Position::Node(id) => node_label(id, graph.nodes.get(id), names)?,
        Position::Edge(id) => {
            let record = edges.get(id)?;
            let subject = label_of(&record.subject, graph, edges, names, labels)?;
            let object = label_of(&record.object, graph, edges, names, labels)?;
            edge_label(record.kind, &subject, &object)
        }
    };
    labels.insert(position.clone(), label.clone());
    Some(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::adjacency::EdgeMeta;
    use atlas_graph_types::edge::{entry_id, Direction, EdgeKind, EdgeRecord, Justification, LocatedAt, RelationId};
    use atlas_graph_types::graph::Graph;
    use atlas_graph_types::id::{EventId, PlaceId, Position};
    use atlas_graph_types::ingest::ProvenanceId;
    use atlas_graph_types::node::{Node, NodePayload};

    static NO_GEOGRAPHY: std::sync::LazyLock<Geography> = std::sync::LazyLock::new(Geography::default);
    static NO_ATLAS: std::sync::LazyLock<AtlasData> = std::sync::LazyLock::new(AtlasData::default);

    fn no_names() -> ReaderNames<'static> {
        ReaderNames::of(&NO_GEOGRAPHY, &NO_ATLAS)
    }

    fn place(raw: &str, canonical: &str) -> Node {
        Node {
            id: PlaceId::new(raw).erase(),
            payload: NodePayload::Place { canonical: canonical.to_string(), lat: 0.0, lon: 0.0, aliases: Vec::new(), description: None },
            provenance: ProvenanceId::from("test"),
        }
    }

    const GENESIS: u8 = 0;
    const JOHN: u8 = 42;

    fn event(raw: &str, label: &str) -> Node {
        Node {
            id: EventId::new(raw).erase(),
            payload: NodePayload::Event {
                label: label.to_string(),
                kind: "event".to_string(),
                verses: Vec::new(),
                witnesses: Vec::new(),
                robertson_section: None,
                acts_section: None,
                atlas_section: None,
                kjv_superscription: None,
                ref_note: None,
            },
            provenance: ProvenanceId::from("test"),
        }
    }

    fn held(nodes: Vec<Node>, rows: Vec<LocatedAt>) -> Graph {
        let mut g = Graph::default();
        for node in nodes {
            g.nodes.insert(node.id.clone(), node);
        }
        g.located_at = rows;
        g.build_indexes();
        g
    }

    fn located(event: &str, place: &str) -> LocatedAt {
        LocatedAt { event: EventId::new(event), place: PlaceId::new(place), provenance: "test".into(), justification: Justification::default() }
    }

    #[test]
    fn a_verse_is_labelled_by_its_reference() {
        // Arrange
        let verse = crate::kjv_adapter::verse_node_id(JOHN, 3, 16);

        // Act
        let label = node_label(&verse, None, &no_names());

        // Assert
        assert_eq!(label, Some("JHN.3.16".to_string()));
    }

    #[test]
    fn a_concord_paragraph_is_labelled_by_its_citation() {
        // Arrange
        let paragraph = crate::concord_adapter::text_unit_id(1, 2, 3);
        let cited = atlas_graph_types::text::ConcordRef { part: 1, article: 2, paragraph: 3 };

        // Act
        let label = node_label(&paragraph, None, &no_names());

        // Assert
        assert_eq!(label, Some(<atlas_graph_types::text::ConcordTag as atlas_graph_types::text::Corpus>::cite(&cited)));
    }

    #[test]
    fn any_other_node_is_labelled_by_its_own_payload_and_an_unheld_one_by_nothing() {
        // Arrange
        let flood = event("flood", "The Flood");

        // Act
        let labels = (node_label(&flood.id, Some(&flood), &no_names()), node_label(&EventId::new("unheld").erase(), None, &no_names()));

        // Assert
        assert_eq!(labels, (Some("The Flood".to_string()), None));
    }

    #[test]
    fn an_edge_is_labelled_by_its_subject_its_kind_and_its_object() {
        // Arrange
        let kind = EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward);

        // Act
        let label = edge_label(kind, "The Flood", "Ararat");

        // Assert
        assert_eq!(label, "The Flood · Located at · Ararat");
    }

    #[test]
    fn compiling_labels_every_position_the_graph_holds() {
        // Arrange
        let mut g = held(vec![event("flood", "The Flood")], vec![located("flood", "ararat")]);
        g.nodes.insert(PlaceId::new("ararat").erase(), place("ararat", "Ararat"));
        let geography = Geography::compile(&g, &NO_ATLAS);
        let flood = Position::Node(EventId::new("flood").erase());
        let ararat = Position::Node(PlaceId::new("ararat").erase());
        let edge = entry_id(RelationId::LocatedAt, &flood, &ararat);

        // Act
        compile(&mut g, &ReaderNames::of(&geography, &NO_ATLAS));

        // Assert
        assert_eq!(
            (g.labels.clone(), g.edges_by_id.clone()),
            (
                [
                    (flood.clone(), "The Flood".to_string()),
                    (ararat.clone(), "Ararat".to_string()),
                    (Position::Edge(edge.clone()), "The Flood · Located at · Ararat".to_string()),
                ]
                .into_iter()
                .collect(),
                [(edge.clone(), EdgeRecord { id: edge, kind: EdgeKind::Directed(RelationId::LocatedAt, Direction::Forward), subject: flood, object: ararat, meta: EdgeMeta::None })]
                    .into_iter()
                    .collect(),
            )
        );
    }

    #[test]
    fn an_end_no_node_holds_is_left_unlabelled_and_so_is_every_edge_that_reaches_it() {
        // Arrange
        let mut g = held(vec![event("flood", "The Flood")], vec![located("flood", "nowhere")]);
        let flood = Position::Node(EventId::new("flood").erase());

        // Act
        compile(&mut g, &no_names());

        // Assert
        assert_eq!((g.labels.clone(), g.edges_by_id.len()), ([(flood, "The Flood".to_string())].into_iter().collect(), 1));
    }

    #[test]
    fn a_verse_no_node_holds_is_still_labelled_by_its_reference() {
        // Arrange
        let verse = crate::kjv_adapter::verse_node_id(GENESIS, 1, 99);

        // Act
        let label = node_label(&verse, None, &no_names());

        // Assert
        assert_eq!(label, Some("GEN.1.99".to_string()));
    }

    #[test]
    fn a_place_is_labelled_by_its_compiled_default_display_name_and_never_its_disambiguated_canonical_name() {
        // Arrange
        let bethel = place("bethel-1", "Bethel 1");
        let mut g = held(vec![event("dream", "Jacob's dream")], vec![located("dream", "bethel-1")]);
        g.nodes.insert(bethel.id.clone(), bethel.clone());
        let geography = Geography::compile(&g, &NO_ATLAS);
        let dream = Position::Node(EventId::new("dream").erase());
        let edge = entry_id(RelationId::LocatedAt, &dream, &Position::Node(bethel.id.clone()));

        // Act
        compile(&mut g, &ReaderNames::of(&geography, &NO_ATLAS));

        // Assert
        assert_eq!(
            (g.labels.get(&Position::Node(bethel.id)).cloned(), g.labels.get(&Position::Edge(edge)).cloned()),
            (Some("Bethel".to_string()), Some("Jacob's dream · Located at · Bethel".to_string()))
        );
    }

    #[test]
    fn a_place_with_no_compiled_default_is_left_unlabelled() {
        // Arrange
        let bethel = place("bethel-1", "Bethel 1");

        // Act
        let label = node_label(&bethel.id, Some(&bethel), &no_names());

        // Assert
        assert_eq!(label, None);
    }

    #[test]
    fn an_anchor_is_labelled_by_its_curated_label_and_never_its_composed_citation() {
        // Arrange
        let mut atlas = AtlasData::default();
        atlas.chronology_anchors.push(atlas_core::data::ChronologyAnchor {
            id: "exodus".to_string(),
            label: "The Exodus from Egypt".to_string(),
            year: -1491,
            event_id: None,
            era_boundary: false,
            source: "Ussher".to_string(),
            note: Some("see atlas_core::chronology::ANCHOR_DEFERRALS".to_string()),
        });
        let exodus = Node {
            id: AnchorId::new("exodus").erase(),
            payload: NodePayload::Anchor { at: atlas_graph_types::chrono::TimePoint::year_only(atlas_graph_types::chrono::Year::new(-1491).unwrap()), citation: "The Exodus from Egypt — 1491 BC. Source: Ussher. see atlas_core::chronology::ANCHOR_DEFERRALS".to_string() },
            provenance: ProvenanceId::from("test"),
        };
        let names = ReaderNames::of(&NO_GEOGRAPHY, &atlas);

        // Act
        let labels = (node_label(&exodus.id, Some(&exodus), &names), node_label(&exodus.id, Some(&exodus), &no_names()));

        // Assert
        assert_eq!(labels, (Some("The Exodus from Egypt".to_string()), None));
    }
}
