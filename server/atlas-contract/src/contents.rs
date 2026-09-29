use std::sync::Arc;

use atlas_graph::corpus_root::corpus_root_id;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{AnyNodeId, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{BibleTag, ConcordTag, VerseRef};
use axum::extract::{Path, State};
use axum::Json;

use crate::error::{ApiError, NoRefusals};
use crate::graph_wire::encode_node_id;
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
        wire::Corpus::Bible => members(&snap, &corpus_root_id::<BibleTag>().erase()).iter().map(|book| book_root(&snap, book)).collect(),
        wire::Corpus::Concord => members(&snap, &corpus_root_id::<ConcordTag>().erase()).iter().map(|document| document_root(&snap, document)).collect(),
    };
    Ok(Json(wire::Contents { corpus, version: atlas_graph::version_hex(graph.version()), roots }))
}

fn members<S: GraphQuery>(snap: &S, container: &AnyNodeId) -> Vec<AnyNodeId> {
    let mut out = Vec::new();
    let mut cursor = None;
    loop {
        let page = snap.edges(&Position::Node(container.clone()), &EdgeQuery { kind: CONTAINS, cursor, limit: 200 });
        out.extend(page.entries.iter().filter_map(|e| match &e.node {
            Position::Node(id) => Some(id.clone()),
            Position::Edge(_) => None,
        }));
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    out
}

/// A member of the Bible's root that is not a book is a defect in the graph, never a book to leave out.
fn book_root<S: GraphQuery>(snap: &S, book: &AnyNodeId) -> wire::ContentsRoot {
    let index = atlas_graph::bible_container_adapter::decode_book_container(book)
        .unwrap_or_else(|| panic!("the Bible's root contains {}, which is not a book container", book.raw)) as usize;
    let code = atlas_core::canon::BOOKS[index].code;
    let children: Vec<wire::ContentsChild> = members(snap, book)
        .iter()
        .filter_map(|child| {
            let (_, chapter) = atlas_graph::bible_container_adapter::decode_chapter_container(child)?;
            let (b, c, v) = members(snap, child).into_iter().find_map(|verse| atlas_graph::kjv_adapter::decode_text_unit(&verse))?;
            Some(wire::ContentsChild {
                id: encode_node_id(child),
                title: chapter.to_string(),
                kind: wire::ContentsChildKind::Chapter,
                r#ref: format!("{code}.{chapter}"),
                locus: wire::TextRef::of_verse(&VerseRef { book: b, chapter: c, verse: v }),
                count: member_count(snap, child),
            })
        })
        .collect();
    let (r#ref, locus) = opening(book, &children);
    wire::ContentsRoot {
        id: encode_node_id(book),
        title: title_of(snap, book),
        kind: wire::ContentsRootKind::Book,
        group: Some(atlas_core::canon::Testament::of_book_index(index)),
        r#ref,
        locus,
        children,
    }
}

fn document_root<S: GraphQuery>(snap: &S, document: &AnyNodeId) -> wire::ContentsRoot {
    let children: Vec<wire::ContentsChild> = members(snap, document)
        .iter()
        .filter_map(|article| {
            let (part, number, paragraph) = members(snap, article).into_iter().find_map(|p| atlas_graph::concord_adapter::decode_text_unit(&p))?;
            Some(wire::ContentsChild {
                id: encode_node_id(article),
                title: title_of(snap, article),
                kind: wire::ContentsChildKind::Article,
                r#ref: format!("BoC {part}.{number}.{paragraph}"),
                locus: wire::TextRef::Concord { part, article: number, paragraph },
                count: member_count(snap, article),
            })
        })
        .collect();
    let (r#ref, locus) = opening(document, &children);
    wire::ContentsRoot { id: encode_node_id(document), title: title_of(snap, document), kind: wire::ContentsRootKind::Document, group: None, r#ref, locus, children }
}

/// A top-level entry opens where its first child does. One with no child to open at is
/// a defect in the graph, never an entry to point at nothing.
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
        book_root(&Graph::default(), &not_a_book);
    }

    #[test]
    #[should_panic(expected = "bible-book-GEN contains nothing to open at")]
    fn a_book_with_no_chapter_to_open_at_is_a_graph_defect_not_an_entry_pointing_nowhere() {
        // Arrange
        let genesis = atlas_graph::bible_container_adapter::book_container_id("GEN").erase();

        // Act
        book_root(&Graph::default(), &genesis);
    }

    #[test]
    #[should_panic(expected = "concord-doc-preface contains nothing to open at")]
    fn a_document_with_no_article_to_open_at_is_a_graph_defect_not_an_entry_pointing_nowhere() {
        // Arrange
        let preface = ContainerNodeId::new("concord-doc-preface").erase();

        // Act
        document_root(&Graph::default(), &preface);
    }
}
