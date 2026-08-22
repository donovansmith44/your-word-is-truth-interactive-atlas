//! The two generic graph endpoints (design doc §5) plus the text-window
//! endpoint (design doc §6; M-A brief requirement 4) -- Batch M-A's own
//! new surfaces, uniform across every node/edge kind the graph carries
//! (only TextUnit nodes and `cites` edges exist yet; nothing here is
//! TextUnit-specific machinery dressed up as generic).
//!
//! `GET /api/node/{id}`              -> card + edge summary + graph version
//! `GET /api/node/{id}/edges`        -> one page of one edge kind, EdgeIds included
//! `GET /api/text`                   -> a window of {ref, text} units + next cursor + version

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

use atlas_core::refs::ScriptureRef;
use atlas_graph::{GraphState, WindowDir};
use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
use atlas_graph_types::id::Position;

use crate::error::ApiError;
use crate::graph_wire::{decode_node_id, describe_position, encode_node_id, parse_edge_kind};

// ---------------------------------------------------------------------
// GET /api/node/{id}
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct EdgeSummaryEntryOut {
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct NodeCardOut {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub provenance: String,
    pub edge_summary: Vec<EdgeSummaryEntryOut>,
    pub version: String,
}

/// `GET /api/node/{id}` (design doc §5): card (id/kind/label/provenance) +
/// edge summary (kind -> true count, honesty needs it -- `Explorable::edge_summary`
/// already lists only inhabited kinds) + the graph version stamp. `{id}` is
/// the wire form `graph_wire::encode_node_id` produces; a malformed or
/// unresolvable id is `bad_ref` (matches every other ref-shaped endpoint's
/// own 400 convention), an id that parses but names no node in the built
/// graph is `not_found` (M-A materializes only TextUnit nodes: any
/// structurally valid `text-unit:BOOK.C.V` id naming a real canon verse
/// resolves; any other kind prefix 400s, since nothing of that kind exists
/// yet to be "not found" instead).
pub async fn node_card(State(graph): State<Arc<GraphState>>, Path(id): Path<String>) -> Result<Json<NodeCardOut>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    let node = graph.graph.nodes.get(&node_id).ok_or_else(|| ApiError::not_found("node"))?;

    let pos = PositionRef(Position::Node(node_id.clone()));
    let summary = pos.edge_summary(&graph.graph);
    let (label, _kind) = crate::graph_wire::describe_node(&node_id, &graph.graph);

    let edge_summary = summary.into_iter().map(|(kind, count)| EdgeSummaryEntryOut { kind: kind.label().to_string(), count }).collect();

    Ok(Json(NodeCardOut {
        id: encode_node_id(&node_id),
        kind: format!("{:?}", node_id.kind),
        label,
        provenance: node.provenance.clone(),
        edge_summary,
        version: graph.version.as_hex(),
    }))
}

// ---------------------------------------------------------------------
// GET /api/node/{id}/edges?kind=&cursor=&limit=
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct NodeRefOut {
    pub id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct EdgeEntryOut {
    /// The bijection witness travels on the wire (M-A brief requirement 4):
    /// the SAME id a caller sees here is what the target's own inverse-kind
    /// page carries back for this same connection.
    pub edge: String,
    pub node: NodeRefOut,
}

#[derive(Debug, Serialize)]
pub struct EdgePageOut {
    pub kind: String,
    pub entries: Vec<EdgeEntryOut>,
    pub next: Option<usize>,
    pub version: String,
}

const DEFAULT_EDGE_LIMIT: usize = 20;
const MAX_EDGE_LIMIT: usize = 200;

/// `GET /api/node/{id}/edges?kind=&cursor=&limit=` (design doc §5): one page
/// of one edge kind. `kind` is a label from graph-types' own relation
/// manifest (e.g. `"cites"`/`"cited-by"`) -- missing or unrecognized is
/// `bad_kind`; `cursor` is the opaque (here: plain integer) offset the
/// previous page's own `next` returned; `limit` defaults to 20, capped at
/// 200.
pub async fn node_edges(
    State(graph): State<Arc<GraphState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<EdgePageOut>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    if !graph.graph.nodes.contains_key(&node_id) {
        return Err(ApiError::not_found("node"));
    }

    let kind_raw = params.get("kind").map(String::as_str).unwrap_or("");
    let kind = parse_edge_kind(kind_raw).ok_or_else(|| ApiError::bad_kind(kind_raw))?;

    let cursor = params.get("cursor").and_then(|s| s.parse::<usize>().ok());
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(DEFAULT_EDGE_LIMIT).clamp(1, MAX_EDGE_LIMIT);

    let page = PositionRef(Position::Node(node_id)).edges(&graph.graph, &EdgeQuery { kind, cursor, limit });

    let entries = page
        .entries
        .iter()
        .map(|e| {
            let (id, kind, label) = describe_position(&e.node, &graph.graph);
            EdgeEntryOut { edge: e.edge.0.clone(), node: NodeRefOut { id, kind, label } }
        })
        .collect();

    Ok(Json(EdgePageOut { kind: kind.label().to_string(), entries, next: page.next, version: graph.version.as_hex() }))
}

// ---------------------------------------------------------------------
// GET /api/text?ref=&n=&dir=&scope=
// ---------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct TextUnitOut {
    #[serde(rename = "ref")]
    pub sref: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
pub struct TextWindowOut {
    pub units: Vec<TextUnitOut>,
    pub next: Option<String>,
    pub version: String,
}

/// Parses `ref` against `scope`: `scope=chapter` accepts a Chapter- or
/// Verse-shaped ref (either way, only the (book, chapter) pair is used --
/// the chapter's OWN verse count is derived server-side from the graph
/// itself, never from the ref); any other `scope` requires a Verse-shaped
/// ref (the single-point cursor the window walks onward/backward from).
fn parse_ref(raw: &str, scope: &str) -> Result<(u8, u16, Option<u16>), ApiError> {
    match ScriptureRef::parse(raw) {
        Ok(ScriptureRef::Verse(v)) => Ok((v.book.0, v.chapter, Some(v.verse))),
        Ok(ScriptureRef::Chapter { book, chapter }) if scope == "chapter" => Ok((book.0, chapter, None)),
        _ => Err(ApiError::bad_ref(raw)),
    }
}

/// `GET /api/text?ref=<dot-ref>&n=&dir=&scope=` (design doc §6; M-A brief
/// requirement 4): a window of `{ref, text}` units + next cursor + graph
/// version. `scope=chapter` returns exactly that chapter's units (`n` is
/// ignored -- the count is derived server-side, `GraphState::chapter_span`)
/// -- still the SAME window query (`GraphState::window`) every other path
/// calls, just with server-derived bounds. `dir=backward` walks the window
/// ending AT `ref` instead of starting from it; anything else (including
/// absence) is onward. ETag/If-None-Match on the version stamp: since the
/// graph is immutable for the process lifetime, the ETag is constant across
/// every request until the next server restart.
pub async fn text_window(
    State(graph): State<Arc<GraphState>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let etag = format!("\"{}\"", graph.version.as_hex());
    if headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let raw_ref = params.get("ref").map(String::as_str).unwrap_or("");
    let scope = params.get("scope").map(String::as_str).unwrap_or("verse");
    let (book, chapter, verse_opt) = parse_ref(raw_ref, scope)?;

    let (start, n) = if scope == "chapter" {
        graph.chapter_span(book, chapter).ok_or_else(|| ApiError::not_found("chapter"))?
    } else {
        let verse = verse_opt.ok_or_else(|| ApiError::bad_ref(raw_ref))?;
        let start = graph.position_of(book, chapter, verse).ok_or_else(|| ApiError::not_found("verse"))?;
        let n = params.get("n").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).clamp(1, 500);
        (start, n)
    };

    let dir = match params.get("dir").map(String::as_str) {
        Some("backward") => WindowDir::Backward,
        _ => WindowDir::Onward,
    };

    let ids = graph.window(start, n, dir);
    let units: Vec<TextUnitOut> = ids
        .iter()
        .filter_map(|id| {
            let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id)?;
            let text = graph.render(id)?.to_string();
            Some(TextUnitOut { sref: atlas_graph::kjv_adapter::dot_ref(b, c, v), text })
        })
        .collect();

    // `next`: the ref that continues the SAME direction of travel one more
    // step past this window -- None at either edge of the corpus.
    let next = match dir {
        WindowDir::Onward => {
            let after = start + units.len();
            graph.bible_unit_at(after).and_then(atlas_graph::kjv_adapter::decode_text_unit).map(|(b, c, v)| atlas_graph::kjv_adapter::dot_ref(b, c, v))
        }
        WindowDir::Backward => {
            let window_start = GraphState::resolved_start(start, n, dir);
            if window_start == 0 {
                None
            } else {
                graph
                    .bible_unit_at(window_start - 1)
                    .and_then(atlas_graph::kjv_adapter::decode_text_unit)
                    .map(|(b, c, v)| atlas_graph::kjv_adapter::dot_ref(b, c, v))
            }
        }
    };

    let body = Json(TextWindowOut { units, next, version: graph.version.as_hex() });
    Ok(([(header::ETAG, etag)], body).into_response())
}
