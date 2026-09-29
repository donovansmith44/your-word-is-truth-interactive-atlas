//! The one `Container` node at the top of each corpus's containment forest, and the `Contains`
//! rows that make the corpus's top-level containers -- its books, its documents -- that root's
//! members. A root's raw id IS the corpus key (`bible`, `concord`), so whoever knows the corpus
//! knows its root without a lookup.

use atlas_graph_types::edge::{ContainerContent, Contains};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::ContainerNodeId;
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::Corpus;

pub fn corpus_root_id(corpus: &str) -> ContainerNodeId {
    ContainerNodeId::new(corpus)
}

/// The root node and one row per member, in the order given. An empty corpus mints nothing: a
/// root over no members would be a fabricated node in a graph built from an empty fixture.
pub fn mint<C: Corpus>(graph: &mut Graph, corpus: &str, title: &str, provenance: &str, members: &[ContainerNodeId]) -> Vec<Contains<C>> {
    if members.is_empty() {
        return Vec::new();
    }
    let root = corpus_root_id(corpus);
    graph.nodes.insert(
        root.erase(),
        Node { id: root.erase(), payload: NodePayload::Container { title: title.to_string() }, provenance: provenance.to_string() },
    );
    members
        .iter()
        .map(|member| Contains {
            container: root.clone(),
            content: ContainerContent::Container(member.clone()),
            provenance: ProvenanceId::from(provenance),
            justification: Default::default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::text::BibleTag;

    const CORPUS: &str = "bible";
    const TITLE: &str = "The Holy Bible";
    const PROVENANCE: &str = "kjv";

    fn book(code: &str) -> ContainerNodeId {
        ContainerNodeId::new(format!("bible-book-{code}"))
    }

    fn root_contains(member: &ContainerNodeId) -> Contains<BibleTag> {
        Contains {
            container: corpus_root_id(CORPUS),
            content: ContainerContent::Container(member.clone()),
            provenance: ProvenanceId::from(PROVENANCE),
            justification: Default::default(),
        }
    }

    #[test]
    fn a_corpus_with_members_gets_one_root_node_that_contains_each_member_in_order() {
        // Arrange
        let mut graph = Graph::default();
        let members = [book("GEN"), book("EXO")];

        // Act
        let rows: Vec<Contains<BibleTag>> = mint(&mut graph, CORPUS, TITLE, PROVENANCE, &members);

        // Assert
        let root = graph.nodes.get(&corpus_root_id(CORPUS).erase()).expect("the root node");
        assert_eq!(root.id, corpus_root_id(CORPUS).erase());
        assert_eq!(format!("{:?}", root.payload), format!("{:?}", NodePayload::Container { title: TITLE.to_string() }));
        assert_eq!(root.provenance, PROVENANCE);
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(format!("{rows:?}"), format!("{:?}", [root_contains(&members[0]), root_contains(&members[1])]));
    }

    #[test]
    fn an_empty_corpus_mints_no_root_and_no_rows() {
        // Arrange
        let mut graph = Graph::default();

        // Act
        let rows: Vec<Contains<BibleTag>> = mint(&mut graph, CORPUS, TITLE, PROVENANCE, &[]);

        // Assert
        assert!(graph.nodes.is_empty());
        assert!(rows.is_empty());
    }
}
