//! D4 (owner, 2026-09-15, verbatim: "Table of contents = a tree. ... Contents
//! is a tree; clicking a node toggles visibility of its children. Stop at the
//! level of ARTICLE (BoC) or TOPIC (Small Catechism). Pages are not a
//! meaningful way of thinking about things."): `GET /api/contents/{corpus}`
//! -- the graph's own containment forest, two levels deep, read through the
//! port and nothing else: books then chapters for the Bible
//! (`bible_container_adapter`'s `Container` nodes and `Contains` rows),
//! documents then articles for the Book of Concord (`concord_adapter`'s,
//! in the Bible's own shape since D3). No hand-maintained list: a root is
//! a `Container` node, a child is a `contains` edge, a count is the child's
//! own `contains` frontier size. The reader's Contents tree (D4b) renders
//! exactly this.

use std::sync::Arc;

use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use crate::error::ApiError;
use crate::graph_wire::encode_node_id;

#[derive(Debug, Serialize)]
pub struct ContentsOut {
    pub corpus: String,
    pub version: String,
    pub roots: Vec<ContentsRootOut>,
}

#[derive(Debug, Serialize)]
pub struct ContentsRootOut {
    pub id: String,
    pub title: String,
    /// `book` | `document`.
    pub kind: String,
    /// `OT` | `NT` for a Bible book; absent for a Concord document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// The navigation target: the root's own first child's ref.
    #[serde(rename = "ref")]
    pub sref: String,
    pub children: Vec<ContentsChildOut>,
}

#[derive(Debug, Serialize)]
pub struct ContentsChildOut {
    pub id: String,
    pub title: String,
    /// `chapter` | `article`.
    pub kind: String,
    /// `GEN.1` for a chapter; `BoC 7.2.1` (its first paragraph) for an article.
    #[serde(rename = "ref")]
    pub sref: String,
    /// The child's own members: verses of a chapter, paragraphs of an article.
    pub count: usize,
}

const CONTAINS: EdgeKind = EdgeKind::Directed(RelationId::Contains, Direction::Forward);

pub async fn contents(State(graph): State<Arc<GraphService>>, Path(corpus): Path<String>) -> Result<Json<ContentsOut>, ApiError> {
    let snap = graph.snapshot();
    let roots = match corpus.as_str() {
        "bible" => bible_roots(&snap),
        "concord" => concord_roots(&snap),
        _ => return Err(ApiError::not_found("corpus")),
    };
    Ok(Json(ContentsOut { corpus, version: atlas_graph::version_hex(graph.version()), roots }))
}

/// Every `contains` target of `container`, in the port's own order (the
/// declared row order: chapters in canon order, articles in article order),
/// paging through the whole frontier.
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

fn bible_roots<S: GraphQuery>(snap: &S) -> Vec<ContentsRootOut> {
    let mut roots = Vec::with_capacity(66);
    for (i, book) in atlas_core::canon::BOOKS.iter().enumerate() {
        let id = atlas_graph::bible_container_adapter::book_container_id(book.code).erase();
        if snap.node(&id).is_none() {
            continue;
        }
        let children: Vec<ContentsChildOut> = members(snap, &id)
            .iter()
            .filter_map(|child| {
                let (_, chapter) = atlas_graph::bible_container_adapter::decode_chapter_container(child)?;
                Some(ContentsChildOut {
                    id: encode_node_id(child),
                    title: chapter.to_string(),
                    kind: "chapter".to_string(),
                    sref: format!("{}.{chapter}", book.code),
                    count: member_count(snap, child),
                })
            })
            .collect();
        let sref = children.first().map(|c| c.sref.clone()).unwrap_or_else(|| format!("{}.1", book.code));
        roots.push(ContentsRootOut {
            id: encode_node_id(&id),
            title: title_of(snap, &id),
            kind: "book".to_string(),
            group: Some(if i < 39 { "OT" } else { "NT" }.to_string()),
            sref,
            children,
        });
    }
    roots
}

fn concord_roots<S: GraphQuery>(snap: &S) -> Vec<ContentsRootOut> {
    // Every Concord document container, wherever the id-ordered node pages
    // put it; ordered below by its own first paragraph's part number.
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

    let mut roots: Vec<(u8, ContentsRootOut)> = docs
        .iter()
        .map(|doc| {
            let mut part = u8::MAX;
            let children: Vec<ContentsChildOut> = members(snap, doc)
                .iter()
                .filter_map(|article| {
                    let first = members(snap, article).into_iter().find_map(|p| atlas_graph::concord_adapter::decode_text_unit(&p))?;
                    part = part.min(first.0);
                    Some(ContentsChildOut {
                        id: encode_node_id(article),
                        title: title_of(snap, article),
                        kind: "article".to_string(),
                        sref: format!("BoC {}.{}.{}", first.0, first.1, first.2),
                        count: member_count(snap, article),
                    })
                })
                .collect();
            let sref = children.first().map(|c| c.sref.clone()).unwrap_or_default();
            (part, ContentsRootOut { id: encode_node_id(doc), title: title_of(snap, doc), kind: "document".to_string(), group: None, sref, children })
        })
        .collect();
    roots.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.id.cmp(&b.1.id)));
    roots.into_iter().map(|(_, r)| r).collect()
}
