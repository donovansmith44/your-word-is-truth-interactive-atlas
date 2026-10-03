use std::sync::Arc;

use atlas_graph::corpus_root::corpus_root_id;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{BibleTag, ConcordTag, VerseRef};
use axum::extract::{Path, State};
use axum::Json;

use crate::error::{ApiError, NoRefusals};
use crate::graph_wire::{encode_node_id, UnreferencedUnit};
use crate::wire;

const CONTAINS: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Forward);

/// The contents of one corpus as a two-level tree: its books or documents, and each one's chapters or articles.
///
/// `{corpus}` is `bible` or `concord`; anything else is `not_found`. Every entry
/// carries the reference to open for it, how many members it holds, and, for a
/// book, which half of the canon it belongs to.
#[utoipa::path(get, path = "/api/contents/{corpus}", params(("corpus" = String, Path)), responses((status = 200, body = wire::Contents), NoRefusals), tag = "contents")]
pub async fn contents(State(graph): State<Arc<GraphService>>, Path(corpus): Path<String>) -> Result<Json<wire::Contents>, ApiError> {
    let snap = graph.snapshot();
    let corpus = wire::Corpus::named(&corpus).ok_or_else(|| ApiError::not_found("corpus"))?;
    let roots = match corpus {
        wire::Corpus::Bible => members(&snap, &corpus_root_id::<BibleTag>().erase()).iter().map(|book| book_root(&snap, book)).collect::<Result<_, _>>()?,
        wire::Corpus::Concord => members(&snap, &corpus_root_id::<ConcordTag>().erase()).iter().map(|document| document_root(&snap, document)).collect::<Result<_, _>>()?,
    };
    Ok(Json(wire::Contents { corpus, version: atlas_graph::version_hex(graph.version()), roots }))
}

fn members<S: GraphQuery>(snap: &S, container: &AnyNodeId) -> Vec<AnyNodeId> {
    crate::reading::drain_edges(snap, &Position::Node(container.clone()), CONTAINS)
        .iter()
        .filter_map(|e| match &e.node {
            Position::Node(id) => Some(id.clone()),
            Position::Edge(_) => None,
        })
        .collect()
}

/// A member of the Bible's root that is not a book is a defect in the graph, never a book to leave out.
fn book_root<S: GraphQuery>(snap: &S, book: &AnyNodeId) -> Result<wire::ContentsRoot, ApiError> {
    let index = atlas_graph::bible_container_adapter::decode_book_container(book)
        .unwrap_or_else(|| panic!("the Bible's root contains {}, which is not a book container", book.raw)) as usize;
    let code = atlas_core::canon::BOOKS[index].code;
    let children = members(snap, book)
        .iter()
        .filter_map(|child| {
            let (_, chapter) = atlas_graph::bible_container_adapter::decode_chapter_container(child)?;
            let (b, c, v) = members(snap, child).into_iter().find_map(|verse| atlas_graph::kjv_adapter::decode_text_unit(&verse))?;
            Some(encode_node_id(child, snap).map(|id| wire::ContentsChild {
                id,
                title: chapter.to_string(),
                kind: wire::ContentsChildKind::Chapter,
                r#ref: format!("{code}.{chapter}"),
                locus: wire::TextRef::of_verse(&VerseRef { book: b, chapter: c, verse: v }),
                count: member_count(snap, child),
            }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (r#ref, locus) = opening(book, &children);
    Ok(wire::ContentsRoot {
        id: encode_node_id(book, snap)?,
        title: title_of(snap, book),
        kind: wire::ContentsRootKind::Book,
        group: Some(atlas_core::canon::Testament::of_book_index(index)),
        r#ref,
        locus,
        children,
    })
}

fn document_root<S: GraphQuery>(snap: &S, document: &AnyNodeId) -> Result<wire::ContentsRoot, ApiError> {
    let children = members(snap, document)
        .iter()
        .filter_map(|article| {
            let (first, (part, number, paragraph)) = members(snap, article).into_iter().find_map(|p| atlas_graph::concord_adapter::decode_text_unit(&p).map(|unit| (p, unit)))?;
            Some(article_child(snap, article, &first, wire::TextRef::Concord { part, article: number, paragraph }))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (r#ref, locus) = opening(document, &children);
    Ok(wire::ContentsRoot { id: encode_node_id(document, snap)?, title: title_of(snap, document), kind: wire::ContentsRootKind::Document, group: None, r#ref, locus, children })
}

fn article_child<S: GraphQuery>(snap: &S, article: &AnyNodeId, first: &AnyNodeId, locus: wire::TextRef) -> Result<wire::ContentsChild, ApiError> {
    let r#ref = snap.references(std::slice::from_ref(first)).remove(0).ok_or_else(|| UnreferencedUnit(first.clone()))?;
    Ok(wire::ContentsChild { id: encode_node_id(article, snap)?, title: title_of(snap, article), kind: wire::ContentsChildKind::Article, r#ref, locus, count: member_count(snap, article) })
}

fn opening(root: &AnyNodeId, children: &[wire::ContentsChild]) -> (String, wire::TextRef) {
    let first = children.first().unwrap_or_else(|| panic!("{} contains nothing to open at", root.raw));
    (first.r#ref.clone(), first.locus.clone())
}

fn member_count<S: GraphQuery>(snap: &S, container: &AnyNodeId) -> usize {
    snap.edge_summary(&Position::Node(container.clone())).get(&CONTAINS).copied().unwrap_or(0)
}

fn title_of<S: GraphQuery>(snap: &S, id: &AnyNodeId) -> String {
    match snap.node(id).map(|n| n.payload) {
        Some(NodePayload::Container { title }) => title,
        _ => id.raw.clone(),
    }
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(contents))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::graph::Graph;
    use atlas_graph_types::id::ContainerNodeId;

    #[test]
    #[should_panic(expected = "the Bible's root contains concord-part-1, which is not a book container")]
    fn a_member_of_the_bibles_root_that_is_not_a_book_is_a_graph_defect_not_a_silent_omission() {
        // Arrange
        let not_a_book = ContainerNodeId::new("concord-part-1").erase();

        // Act
        let _ = book_root(&Graph::default(), &not_a_book);
    }

    #[test]
    #[should_panic(expected = "bible-book-GEN contains nothing to open at")]
    fn a_book_with_no_chapter_to_open_at_is_a_graph_defect_not_an_entry_pointing_nowhere() {
        // Arrange
        let genesis = atlas_graph::bible_container_adapter::book_container_id("GEN").erase();

        // Act
        let _ = book_root(&Graph::default(), &genesis);
    }

    #[test]
    #[should_panic(expected = "concord-doc-preface contains nothing to open at")]
    fn a_document_with_no_article_to_open_at_is_a_graph_defect_not_an_entry_pointing_nowhere() {
        // Arrange
        let preface = ContainerNodeId::new("concord-doc-preface").erase();

        // Act
        let _ = document_root(&Graph::default(), &preface);
    }
}
