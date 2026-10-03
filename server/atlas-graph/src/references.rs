use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::AnyNodeId;
use atlas_graph_types::text::{ConcordRef, ConcordTag, Corpus};

pub fn unit_reference(id: &AnyNodeId) -> Option<String> {
    if let Some((book, chapter, verse)) = crate::kjv_adapter::decode_text_unit(id) {
        return Some(crate::kjv_adapter::dot_ref(book, chapter, verse));
    }
    crate::concord_adapter::decode_text_unit(id).map(|(part, article, paragraph)| ConcordTag::cite(&ConcordRef { part, article, paragraph }))
}

pub fn compile(graph: &mut Graph) {
    graph.references = graph.nodes.keys().filter_map(|id| unit_reference(id).map(|reference| (id.clone(), reference))).collect();
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::id::EventId;
    use atlas_graph_types::ingest::ProvenanceId;
    use atlas_graph_types::node::{Node, NodePayload};

    const JOHN: u8 = 42;
    const FORMULA_OF_CONCORD: u8 = 7;

    fn held(ids: &[AnyNodeId]) -> Graph {
        let mut g = Graph::default();
        for id in ids {
            let payload = NodePayload::Source { label: id.raw.clone() };
            g.nodes.insert(id.clone(), Node { id: id.clone(), payload, provenance: ProvenanceId::from("test") });
        }
        g
    }

    #[test]
    fn a_verse_is_referenced_by_its_book_code_chapter_and_verse() {
        // Arrange
        let verse = crate::kjv_adapter::verse_node_id(JOHN, 3, 16);

        // Act
        let reference = unit_reference(&verse);

        // Assert
        assert_eq!(reference, Some("JHN.3.16".to_string()));
    }

    #[test]
    fn a_concord_paragraph_is_referenced_by_its_part_article_and_paragraph() {
        // Arrange
        let paragraph = crate::concord_adapter::text_unit_id(FORMULA_OF_CONCORD, 2, 1);

        // Act
        let reference = unit_reference(&paragraph);

        // Assert
        assert_eq!(reference, Some("BoC 7.2.1".to_string()));
    }

    #[test]
    fn compiling_references_every_text_unit_the_graph_holds() {
        // Arrange
        let verse = crate::kjv_adapter::verse_node_id(JOHN, 3, 16);
        let paragraph = crate::concord_adapter::text_unit_id(FORMULA_OF_CONCORD, 2, 1);
        let event = EventId::new("flood").erase();
        let mut g = held(&[verse.clone(), paragraph.clone(), event]);

        // Act
        compile(&mut g);

        // Assert
        assert_eq!(g.references, [(verse, "JHN.3.16".to_string()), (paragraph, "BoC 7.2.1".to_string())].into_iter().collect());
    }
}
