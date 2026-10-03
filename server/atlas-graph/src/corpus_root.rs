//! The one `Container` node at the top of each corpus's containment forest, and the `Contains`
//! rows that make the corpus's top-level containers -- its books, its documents -- that root's
//! members. A root's raw id IS the corpus key (`bible`, `concord`), so whoever knows the corpus
//! knows its root without a lookup.

use atlas_graph_types::container::CorpusContainer;
use atlas_graph_types::edge::{ContainerContent, Contains};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::ContainerNodeId;
use atlas_graph_types::ingest::ProvenanceId;
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::Corpus;

pub fn corpus_root_id<C: Corpus>() -> ContainerNodeId {
    ContainerNodeId::new(C::ID)
}

/// The root node and one row per member, in the order given. An empty corpus mints nothing: a
/// root over no members would be a fabricated node in a graph built from an empty fixture.
pub fn mint<C: Corpus>(graph: &mut Graph, root_container: CorpusContainer, provenance: &str, members: &[ContainerNodeId]) -> Vec<Contains<C>> {
    if members.is_empty() {
        return Vec::new();
    }
    let root = corpus_root_id::<C>();
    graph.nodes.insert(
        root.erase(),
        Node { id: root.erase(), payload: NodePayload::Container(root_container), provenance: provenance.to_string() },
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
    use atlas_graph_types::sections::{section_of_node, Section};
    use atlas_graph_types::text::{BibleTag, ConcordTag};

    const TITLE: &str = "The Holy Bible";
    const PROVENANCE: &str = "kjv";

    fn bible_root() -> CorpusContainer {
        CorpusContainer::Bible(atlas_graph_types::container::BibleContainer::Bible { title: TITLE.to_string() })
    }

    fn book(code: &str) -> ContainerNodeId {
        ContainerNodeId::new(format!("bible-book-{code}"))
    }

    fn root_contains(member: &ContainerNodeId) -> Contains<BibleTag> {
        Contains {
            container: corpus_root_id::<BibleTag>(),
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
        let rows = mint::<BibleTag>(&mut graph, bible_root(), PROVENANCE, &members);

        // Assert
        let root = graph.nodes.get(&corpus_root_id::<BibleTag>().erase()).expect("the root node");
        assert_eq!(
            root,
            &Node { id: corpus_root_id::<BibleTag>().erase(), payload: NodePayload::Container(bible_root()), provenance: PROVENANCE.to_string() }
        );
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(rows, [root_contains(&members[0]), root_contains(&members[1])]);
    }

    #[test]
    fn the_root_minted_for_a_corpus_files_under_that_corpus_section() {
        // Arrange
        let mut graph = Graph::default();
        let member = [ContainerNodeId::new("anything")];

        // Act
        mint::<BibleTag>(&mut graph, bible_root(), PROVENANCE, &member);
        mint::<ConcordTag>(&mut graph, CorpusContainer::Concord(atlas_graph_types::container::ConcordContainer::BookOfConcord { title: "The Book of Concord".to_string(), description: String::new() }), "concord", &member);

        // Assert
        let sections = [BibleTag::ID, ConcordTag::ID].map(|corpus| section_of_node(&graph.nodes[&ContainerNodeId::new(corpus).erase()]));
        assert_eq!(sections, [Section::Kjv, Section::Concord]);
    }

    #[test]
    fn an_empty_corpus_mints_no_root_and_no_rows() {
        // Arrange
        let mut graph = Graph::default();

        // Act
        let rows = mint::<BibleTag>(&mut graph, bible_root(), PROVENANCE, &[]);

        // Assert
        assert!(graph.nodes.is_empty());
        assert!(rows.is_empty());
    }
}
