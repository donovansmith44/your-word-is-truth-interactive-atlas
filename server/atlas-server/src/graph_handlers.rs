//! The two generic graph endpoints (design doc §5) plus the text-window
//! endpoint (design doc §6; M-A brief requirement 4) -- Batch M-A's own
//! new surfaces, uniform across every node/edge kind the graph carries.
//! M-A materialized only TextUnit nodes and `cites` edges; Batch M-B adds
//! Event/Narrative/Anchor/Place-stub nodes and the attests/succession/
//! dated-by/located-at/justified-by relations with ZERO changes to this
//! file -- the generic path was already total over any node/edge kind
//! (only `graph_wire::decode_node_id`/`encode_node_id`'s own id-grammar
//! layer needed the four new kinds' arms; see that module's own doc
//! comment).
//!
//! Fix round 1 (C1): every handler below opens a snapshot
//! (`GraphService::snapshot`) and performs every actual graph query
//! through `atlas_graph_types::store::GraphQuery`'s own trait methods on
//! it (`node`/`edges`/`edge_summary`/`reading_window`, and `window.rs`'s
//! own generic helpers built on top of those) -- never a raw `Graph`
//! field. `GraphService` itself is held concretely in `AppState` only
//! because `GraphStore`'s associated-type shape isn't `dyn`-safe (see
//! `app.rs`'s own doc comment); the query surface consumed here is
//! entirely the owner-approved port.
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
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;

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
    /// ENT-1a: Easton's Bible Dictionary (1897, PD) prose, source-attested,
    /// `None` until a match exists -- same additive-JSON, same held-client
    /// disclosure as `handlers::PlaceDetailOut::description`. This is the
    /// ONLY "detail" surface a Person or PeopleGroup node has at all
    /// (`graph_wire.rs`'s own doc comment: no dedicated per-kind endpoint
    /// exists for either), so widening the generic card here is what
    /// actually reaches them; it reaches Place/PeopleGroup for free too
    /// (the same payload fact, whichever kind carries it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// `GET /api/node/{id}` (design doc §5): card (id/kind/label/provenance) +
/// edge summary (kind -> true count, honesty needs it -- `GraphQuery`'s own
/// `edge_summary` already lists only inhabited kinds) + the graph version
/// stamp. `{id}` is the wire form `graph_wire::encode_node_id` produces; a
/// malformed or unresolvable id is `bad_ref` (matches every other
/// ref-shaped endpoint's own 400 convention), an id that parses but names
/// no node in the built graph is `not_found` (six kinds resolve as of
/// Batch M-B: `text-unit:BOOK.C.V` naming a real canon verse, and
/// `Event:`/`Narrative:`/`Anchor:`/`Place:` naming a real curated/
/// Theographic id; any other kind prefix, or a real-shaped id this batch's
/// event world doesn't carry, 400s/404s per the same convention).
pub async fn node_card(State(graph): State<Arc<GraphService>>, Path(id): Path<String>) -> Result<Json<NodeCardOut>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| ApiError::not_found("node"))?;

    let summary = snap.edge_summary(&Position::Node(node_id.clone()));
    let (label, _kind) = crate::graph_wire::describe_node(&node_id, &snap);

    let edge_summary = summary.into_iter().map(|(kind, count)| EdgeSummaryEntryOut { kind: kind.label().to_string(), count }).collect();

    // ENT-1a: whichever of the three described kinds this node is (or
    // `None` for every other kind, and `None` until a match exists even for
    // those three) -- straight off the already-fetched node's own payload,
    // no second query.
    let description = match &node.payload {
        NodePayload::Place { description, .. } | NodePayload::Person { description, .. } | NodePayload::PeopleGroup { description, .. } => description.clone(),
        _ => None,
    };

    Ok(Json(NodeCardOut {
        id: encode_node_id(&node_id),
        kind: format!("{:?}", node_id.kind),
        label,
        provenance: node.provenance.clone(),
        edge_summary,
        version: atlas_graph::version_hex(graph.version()),
        description,
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
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<EdgePageOut>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(ApiError::not_found("node"));
    }

    let kind_raw = params.get("kind").map(String::as_str).unwrap_or("");
    let kind = parse_edge_kind(kind_raw).ok_or_else(|| ApiError::bad_kind(kind_raw))?;

    let cursor = params.get("cursor").and_then(|s| s.parse::<usize>().ok());
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(DEFAULT_EDGE_LIMIT).clamp(1, MAX_EDGE_LIMIT);

    let page = snap.edges(&Position::Node(node_id), &EdgeQuery { kind, cursor, limit });

    // PG-1a wire seam (batch-pg1a-brief.md decision 6, "the U5-rebinding
    // seam"): the CURRENT client has no rendering surface for a
    // PeopleGroup-kind neighbor -- `graph_wire::decode_node_id` carries no
    // "PeopleGroup" arm at all (deliberately; that round-trip completion
    // is U5 rebinding's own job, held for the frontend-elegance
    // brainstorm), so a client that somehow received a PeopleGroup entry
    // here could not even re-fetch its own card (`/api/node/{id}` would
    // 400 `bad_ref` on the wire id this endpoint would otherwise hand
    // back). Filtered HERE, the ONE place every entity-list page this
    // generic endpoint can produce funnels through, rather than
    // special-cased per relation kind (`mentions`/`mentioned-in`/
    // `namesake-of`/`named-after` all matter equally -- this filter
    // covers every one of them at once, and any future relation touching
    // a PeopleGroup node too). Graph queries/indexes themselves still
    // carry every PeopleGroup edge in full (`snap.edges` above is
    // unfiltered) -- this is a serving-boundary projection only, one
    // revert away (delete this `.filter` call) once U5 lands.
    let entries = page
        .entries
        .iter()
        .filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup))
        .map(|e| {
            let (id, kind, label) = describe_position(&e.node, &snap);
            EdgeEntryOut { edge: e.edge.0.clone(), node: NodeRefOut { id, kind, label } }
        })
        .collect();

    Ok(Json(EdgePageOut { kind: kind.label().to_string(), entries, next: page.next, version: atlas_graph::version_hex(graph.version()) }))
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
/// ignored -- the count is derived server-side, `GraphService::chapter_span`)
/// -- still the SAME window query (`window::window`) every other path
/// calls, just with server-derived bounds.
///
/// Fix round 1, I1: `dir=backward` is REJECTED (`bad_dir`, 400) when
/// combined with `scope=chapter`, rather than silently misapplied. A
/// chapter-scoped window's bounds are already fully determined by the
/// chapter itself (`chapter_span` always returns the span covering EVERY
/// verse of that chapter) -- there is no honest "walk backward" distinct
/// from "walk onward" once start/n are both fixed by the chapter's own
/// edges, so accepting the combination silently would only ever produce
/// either (a) the identical result as onward (misleadingly implying a real
/// choice existed) or, as previously shipped, (b) a DIFFERENT chapter's
/// tail entirely (`chapter_span`'s `start` is always the chapter's own
/// verse 1 position; applying backward *resolution* to that as if it were
/// a window's END walks into the PRECEDING chapter). Rejecting the
/// combination outright is the honest choice: `dir` only has meaning for a
/// verse-anchored window, where onward/backward genuinely pick different
/// spans.
///
/// For any other `scope`: `dir=backward` walks the window ending AT `ref`
/// instead of starting from it; anything else (including absence) is
/// onward. ETag/If-None-Match on the version stamp: since the graph is
/// immutable for the process lifetime, the ETag is constant across every
/// request until the next server restart.
pub async fn text_window(
    State(graph): State<Arc<GraphService>>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, ApiError> {
    let etag = format!("\"{}\"", atlas_graph::version_hex(graph.version()));
    if headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let raw_ref = params.get("ref").map(String::as_str).unwrap_or("");
    let scope = params.get("scope").map(String::as_str).unwrap_or("verse");
    let dir_raw = params.get("dir").map(String::as_str);

    if scope == "chapter" && dir_raw == Some("backward") {
        return Err(ApiError::bad_dir(
            "dir=backward is not supported with scope=chapter -- a chapter-scoped window's bounds are already fully determined by the chapter itself, so there is no direction left to walk; omit dir, or use dir=onward, or drop scope=chapter and anchor on a specific verse instead",
        ));
    }

    let (book, chapter, verse_opt) = parse_ref(raw_ref, scope)?;

    let (start, n) = if scope == "chapter" {
        graph.chapter_span(book, chapter).ok_or_else(|| ApiError::not_found("chapter"))?
    } else {
        let verse = verse_opt.ok_or_else(|| ApiError::bad_ref(raw_ref))?;
        let start = graph.position_of(book, chapter, verse).ok_or_else(|| ApiError::not_found("verse"))?;
        let n = params.get("n").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).clamp(1, 500);
        (start, n)
    };

    let dir = match dir_raw {
        Some("backward") => WindowDir::Backward,
        _ => WindowDir::Onward,
    };

    let snap = graph.snapshot();
    let ids = window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, dir);
    let units: Vec<TextUnitOut> = ids
        .iter()
        .filter_map(|id| {
            let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id)?;
            let text = window::render(&snap, id)?;
            Some(TextUnitOut { sref: atlas_graph::kjv_adapter::dot_ref(b, c, v), text })
        })
        .collect();

    // `next`: the ref that continues the SAME direction of travel one more
    // step past this window -- None at either edge of the corpus. Reads
    // the single unit just past the window via THE PORT's own
    // `reading_window` primitive (a 1-element window), never a direct
    // spine-slice reach.
    let unit_at = |pos: usize| {
        snap.reading_window(atlas_graph::kjv_adapter::BIBLE_CORPUS, pos, 1)
            .into_iter()
            .next()
            .and_then(|id| atlas_graph::kjv_adapter::decode_text_unit(&id))
            .map(|(b, c, v)| atlas_graph::kjv_adapter::dot_ref(b, c, v))
    };
    let next = match dir {
        WindowDir::Onward => unit_at(start + units.len()),
        WindowDir::Backward => {
            let window_start = window::resolved_start(start, n, dir);
            if window_start == 0 {
                None
            } else {
                unit_at(window_start - 1)
            }
        }
    };

    let body = Json(TextWindowOut { units, next, version: atlas_graph::version_hex(graph.version()) });
    Ok(([(header::ETAG, etag)], body).into_response())
}
