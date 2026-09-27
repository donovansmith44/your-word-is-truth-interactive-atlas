use std::sync::Arc;

use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use axum::extract::{Path, State};
use axum::Json;

use crate::error::ApiError;
use crate::graph_wire::encode_node_id;
use crate::wire;

const CONTAINS: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Forward);

#[utoipa::path(get, path = "/api/contents/{corpus}", params(("corpus" = String, Path)), responses((status = 200, body = wire::Contents), ApiError), tag = "contents")]
pub async fn contents(State(graph): State<Arc<GraphService>>, Path(corpus): Path<String>) -> Result<Json<wire::Contents>, ApiError> {
    let snap = graph.snapshot();
    let corpus = wire::Corpus::named(&corpus).ok_or_else(|| ApiError::not_found("corpus"))?;
    let roots = match corpus {
        wire::Corpus::Bible => bible_roots(&snap),
        wire::Corpus::Concord => concord_roots(&snap),
    };
    Ok(Json(wire::Contents { corpus, version: atlas_graph::version_hex(graph.version()), roots }))
}

/// Every `contains` target of `container`, in the declared row order the port
/// answers in: chapters in canon order, articles in article order.
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

fn member_count<S: GraphQuery>(snap: &S, container: &AnyNodeId) -> usize {
    snap.edge_summary(&Position::Node(container.clone())).get(&CONTAINS).copied().unwrap_or(0)
}

fn title_of<S: GraphQuery>(snap: &S, id: &AnyNodeId) -> String {
    match snap.node(id).map(|n| n.payload) {
        Some(NodePayload::Container { title }) => title,
        _ => id.raw.clone(),
    }
}

fn bible_roots<S: GraphQuery>(snap: &S) -> Vec<wire::ContentsRoot> {
    let mut roots = Vec::with_capacity(66);
    for (i, book) in atlas_core::canon::BOOKS.iter().enumerate() {
        let id = atlas_graph::bible_container_adapter::book_container_id(book.code).erase();
        if snap.node(&id).is_none() {
            continue;
        }
        let children: Vec<wire::ContentsChild> = members(snap, &id)
            .iter()
            .filter_map(|child| {
                let (_, chapter) = atlas_graph::bible_container_adapter::decode_chapter_container(child)?;
                Some(wire::ContentsChild {
                    id: encode_node_id(child),
                    title: chapter.to_string(),
                    kind: wire::ContentsChildKind::Chapter,
                    sref: format!("{}.{chapter}", book.code),
                    count: member_count(snap, child),
                })
            })
            .collect();
        let sref = children.first().map(|c| c.sref.clone()).unwrap_or_else(|| format!("{}.1", book.code));
        roots.push(wire::ContentsRoot {
            id: encode_node_id(&id),
            title: title_of(snap, &id),
            kind: wire::ContentsRootKind::Book,
            group: Some(atlas_core::canon::Testament::of_book_index(i)),
            sref,
            children,
        });
    }
    roots
}

fn concord_roots<S: GraphQuery>(snap: &S) -> Vec<wire::ContentsRoot> {
    // Node pages come back in id order; the sort below puts the documents in
    // their own reading order, by the part number of each one's first paragraph.
    let mut docs: Vec<AnyNodeId> = Vec::new();
    let mut cursor = None;
    loop {
        let page = snap.nodes_of_kind(NodeKind::Container, cursor, 500);
        docs.extend(page.ids.into_iter().filter(|id| id.raw.starts_with("concord-doc-")));
        match page.next {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }

    let mut roots: Vec<(u8, wire::ContentsRoot)> = docs
        .iter()
        .map(|doc| {
            let mut part = u8::MAX;
            let children: Vec<wire::ContentsChild> = members(snap, doc)
                .iter()
                .filter_map(|article| {
                    let first = members(snap, article).into_iter().find_map(|p| atlas_graph::concord_adapter::decode_text_unit(&p))?;
                    part = part.min(first.0);
                    Some(wire::ContentsChild {
                        id: encode_node_id(article),
                        title: title_of(snap, article),
                        kind: wire::ContentsChildKind::Article,
                        sref: format!("BoC {}.{}.{}", first.0, first.1, first.2),
                        count: member_count(snap, article),
                    })
                })
                .collect();
            let sref = children.first().map(|c| c.sref.clone()).unwrap_or_default();
            (part, wire::ContentsRoot { id: encode_node_id(doc), title: title_of(snap, doc), kind: wire::ContentsRootKind::Document, group: None, sref, children })
        })
        .collect();
    roots.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
    roots.into_iter().map(|(_, r)| r).collect()
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(contents))
}
