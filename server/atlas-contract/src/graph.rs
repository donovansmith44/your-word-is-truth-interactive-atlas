use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;

use atlas_core::refs::ScriptureRef;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::EdgeKind;
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;

use crate::error::ApiError;
use crate::graph_wire::{decode_node_id, describe_position, encode_node_id};
use crate::wire;

/// One node of the graph at a glance: what it is, what to call it, where it came from, and how many neighbours it has of each kind.
///
/// `{id}` is `Kind:identifier` -- `Place:hazor-1`, `Event:ab_ur`,
/// `Person:aaron_1` -- or `text-unit:BOOK.CHAPTER.VERSE` for a verse of
/// Scripture and `text-unit:BoC PART.ARTICLE.PARAGRAPH` for a paragraph of the
/// Book of Concord. An id of no recognised kind is `bad_ref`; one that names no
/// node is `not_found`.
#[utoipa::path(get, path = "/api/node/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NodeCard), ApiError), tag = "graph")]
pub async fn node_card(State(graph): State<Arc<GraphService>>, Path(id): Path<String>) -> Result<Json<wire::NodeCard>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| ApiError::not_found("node"))?;

    let summary = snap.edge_summary(&Position::Node(node_id.clone()));
    let label = crate::graph_wire::describe_node(&node_id, &snap);

    let edge_summary = summary.into_iter().map(|(kind, count)| wire::EdgeSummaryEntry { kind, count }).collect();

    let description = node_description(&node_id, &snap);
    let person = match &node.payload {
        atlas_graph_types::node::NodePayload::Person { gender, birth_year, death_year, also_called, first_year, last_year, eternal, eternal_grounds, .. } => Some(wire::PersonLife {
            gender: gender.clone(),
            birth_year: *birth_year,
            death_year: *death_year,
            first_year: *first_year,
            last_year: *last_year,
            eternal: *eternal,
            eternal_grounds: eternal_grounds.clone(),
            also_called: also_called.clone(),
        }),
        _ => None,
    };

    Ok(Json(wire::NodeCard {
        id: encode_node_id(&node_id),
        kind: node_id.kind,
        label,
        provenance: node.provenance.clone(),
        edge_summary,
        version: atlas_graph::version_hex(graph.version()),
        person,
        description,
    }))
}

pub(crate) fn node_description(id: &AnyNodeId, q: &impl GraphQuery) -> Option<String> {
    let node = q.node(id)?;
    match node.payload {
        NodePayload::Place { description, .. } | NodePayload::Person { description, .. } | NodePayload::PeopleGroup { description, .. } => description,
        NodePayload::CommentaryItem { text, .. } => Some(text),
        _ => None,
    }
}

const DEFAULT_EDGE_LIMIT: usize = 20;
const MAX_EDGE_LIMIT: usize = 200;

/// One page of a node's neighbours of a single kind, each with the id of the edge that joins them.
///
/// `{id}` takes the same form `/api/node/{id}` does. The required `kind` is an
/// edge label such as `cites` or `cited-by`; anything else is `bad_kind`, an
/// unrecognised id is `bad_ref`, and an id naming no node is `not_found`.
/// `limit` defaults to 20 and caps at 200; pass the response's `next` back as
/// `cursor` for the following page, and its absence is the last page.
#[utoipa::path(get, path = "/api/node/{id}/edges", params(("id" = String, Path), ("kind" = EdgeKind, Query), ("cursor" = Option<usize>, Query), ("limit" = Option<usize>, Query)), responses((status = 200, body = wire::EdgePage), ApiError), tag = "graph")]
pub async fn node_edges(
    State(graph): State<Arc<GraphService>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<wire::EdgePage>, ApiError> {
    let node_id = decode_node_id(&id).ok_or_else(|| ApiError::bad_ref(&id))?;
    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(ApiError::not_found("node"));
    }

    let kind_raw = params.get("kind").map(String::as_str).unwrap_or("");
    let kind = EdgeKind::from_label(kind_raw).ok_or_else(|| ApiError::bad_kind(kind_raw))?;

    let cursor = params.get("cursor").and_then(|s| s.parse::<usize>().ok());
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok()).unwrap_or(DEFAULT_EDGE_LIMIT).clamp(1, MAX_EDGE_LIMIT);

    let page = snap.edges(&Position::Node(node_id), &EdgeQuery { kind, cursor, limit });

    // A PeopleGroup wire id does not decode, so an entry naming one would hand the
    // caller a reference it cannot fetch a card for.
    let entries = page
        .entries
        .iter()
        .filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup))
        .map(|e| {
            wire::EdgeEntry { edge: e.edge.0.clone(), node: describe_position(&e.node, &snap) }
        })
        .collect();

    Ok(Json(wire::EdgePage { kind, entries, next: page.next, version: atlas_graph::version_hex(graph.version()) }))
}

/// A window of one corpus's reading spine: the units of text around the one a reference names, and the reference that continues the window.
///
/// `ref` is `BOOK.CHAPTER.VERSE`, or `BoC PART.ARTICLE.PARAGRAPH` when
/// `corpus=concord` (`corpus` is `bible` unless given, anything else is
/// `bad_corpus`); a malformed reference is `bad_ref` and one naming nothing in
/// the corpus is `not_found`. `n` defaults to 1 and caps at 500, and
/// `dir=backward` ends the window at `ref` instead of starting it there.
/// `scope=chapter` covers the whole chapter named instead, and takes neither `n`
/// nor `dir=backward` -- the combination is `bad_dir`. The response's `next` is
/// the reference one step further on, absent at the end of the corpus.
#[utoipa::path(get, path = "/api/text", params(("ref" = String, Query), ("n" = Option<usize>, Query), ("dir" = Option<String>, Query), ("scope" = inline(Option<wire::TextScope>), Query), ("corpus" = Option<String>, Query)), responses((status = 200, body = wire::TextWindow), ApiError), tag = "graph")]
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
    let scope = params.get("scope").and_then(|raw| wire::TextScope::named(raw)).unwrap_or(wire::TextScope::Verse);
    let dir_raw = params.get("dir").map(String::as_str);
    let requested_corpus = params.get("corpus").map(String::as_str).unwrap_or(wire::Corpus::Bible.name());
    let corpus = wire::Corpus::named(requested_corpus).ok_or_else(|| ApiError::bad_corpus(requested_corpus))?;

    if scope == wire::TextScope::Chapter && dir_raw == Some("backward") {
        return Err(ApiError::bad_dir(
            "dir=backward is not supported with scope=chapter -- a chapter-scoped window's bounds are already fully determined by the chapter itself, so there is no direction left to walk; omit dir, or use dir=onward, or drop scope=chapter and anchor on a specific verse instead",
        ));
    }
    if corpus == wire::Corpus::Concord && scope == wire::TextScope::Chapter {
        return Err(ApiError::bad_dir(
            "scope=chapter is not supported with corpus=concord -- a Concord article's own paragraph count varies too widely for one server-derived span; omit scope (or use scope=verse) and set n explicitly instead",
        ));
    }

    let dir = match dir_raw {
        Some("backward") => WindowDir::Backward,
        _ => WindowDir::Onward,
    };

    let snap = graph.snapshot();

    if corpus == wire::Corpus::Concord {
        let (part, article, paragraph) = parse_concord_ref(raw_ref)?;
        let start = graph.concord_position_of(part, article, paragraph).ok_or_else(|| ApiError::not_found("concord paragraph"))?;
        let n = params.get("n").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).clamp(1, 500);

        let ids = window::window(&snap, corpus.name(), start, n, dir);
        let units: Vec<wire::TextUnit> = ids
            .iter()
            .filter_map(|id| {
                let (p, a, para) = atlas_graph::concord_adapter::decode_text_unit(id)?;
                let text = window::render_layer(&snap, id, atlas_graph::concord_adapter::CONCORD_TRANSLATION)?;
                Some(wire::TextUnit { sref: format!("BoC {p}.{a}.{para}"), text, words_of_christ: Vec::new(), edge_summary: unit_edge_summary(&snap, id) })
            })
            .collect();

        let unit_at = |pos: usize| {
            snap.reading_window(corpus.name(), pos, 1)
                .into_iter()
                .next()
                .and_then(|id| atlas_graph::concord_adapter::decode_text_unit(&id))
                .map(|(p, a, para)| format!("BoC {p}.{a}.{para}"))
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

        let body = Json(wire::TextWindow { units, next, version: atlas_graph::version_hex(graph.version()) });
        return Ok(([(header::ETAG, etag)], body).into_response());
    }

    let (book, chapter, verse_opt) = parse_ref(raw_ref)?;

    let (start, n) = if scope == wire::TextScope::Chapter {
        graph.chapter_span(book, chapter).ok_or_else(|| ApiError::not_found("chapter"))?
    } else {
        let verse = verse_opt.ok_or_else(|| ApiError::bad_ref(raw_ref))?;
        let start = graph.position_of(book, chapter, verse).ok_or_else(|| ApiError::not_found("verse"))?;
        let n = params.get("n").and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).clamp(1, 500);
        (start, n)
    };

    let ids = window::window(&snap, corpus.name(), start, n, dir);
    let units: Vec<wire::TextUnit> = ids
        .iter()
        .filter_map(|id| {
            let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id)?;
            let text = window::render(&snap, id)?;
            let sref = atlas_graph::kjv_adapter::dot_ref(b, c, v);
            let words_of_christ = graph.red_letter_spans.get(&sref).map(|spans| spans.iter().map(|&(start, end)| crate::wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();
            Some(wire::TextUnit { sref, text, words_of_christ, edge_summary: unit_edge_summary(&snap, id) })
        })
        .collect();

    let unit_at = |pos: usize| {
        snap.reading_window(corpus.name(), pos, 1)
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

    let body = Json(wire::TextWindow { units, next, version: atlas_graph::version_hex(graph.version()) });
    Ok(([(header::ETAG, etag)], body).into_response())
}

fn unit_edge_summary(snap: &impl atlas_graph_types::store::GraphQuery, id: &atlas_graph_types::id::AnyNodeId) -> Vec<wire::EdgeSummaryEntry> {
    snap.edge_summary(&Position::Node(id.clone())).into_iter().map(|(kind, count)| wire::EdgeSummaryEntry { kind, count }).collect()
}

/// Parses `ref` into `(book, chapter, verse)`, the verse present only for a
/// verse-shaped ref. Which shapes a request may use is decided by the window that
/// needs the verse, so re-checking it here would state the rule twice.
fn parse_ref(raw: &str) -> Result<(u8, u16, Option<u16>), ApiError> {
    match ScriptureRef::parse(raw) {
        Ok(ScriptureRef::Verse(v)) => Ok((v.book.0, v.chapter, Some(v.verse))),
        Ok(ScriptureRef::Chapter { book, chapter }) => Ok((book.0, chapter, None)),
        _ => Err(ApiError::bad_ref(raw)),
    }
}

fn parse_concord_ref(raw: &str) -> Result<(u8, u16, u16), ApiError> {
    let rest = raw.strip_prefix("BoC ").ok_or_else(|| ApiError::bad_ref(raw))?;
    let mut parts = rest.split('.');
    let (Some(part), Some(article), Some(paragraph), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else {
        return Err(ApiError::bad_ref(raw));
    };
    let part: u8 = part.parse().map_err(|_| ApiError::bad_ref(raw))?;
    let article: u16 = article.parse().map_err(|_| ApiError::bad_ref(raw))?;
    let paragraph: u16 = paragraph.parse().map_err(|_| ApiError::bad_ref(raw))?;
    Ok((part, article, paragraph))
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(node_card))
        .routes(routes!(node_edges))
        .routes(routes!(text_window))
}
