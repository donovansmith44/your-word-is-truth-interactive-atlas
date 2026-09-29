use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::EdgeKind;
use atlas_graph_types::explore::EdgeQuery;
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::VerseRef;

use crate::error::{ApiError, FrontierRefusals, ReadingWindowRefusals, ReferenceRefusals};
use crate::graph_wire::{describe_position, encode_node_id};
use crate::query::{self, AsGiven, Contract, ContractParams};
use crate::reference::{ConcordParagraphReference, NodeReference, ReadingReference, Reference};
use crate::wire;

/// One node of the graph at a glance: what it is, what to call it, where it came from, and how many neighbours it has of each kind.
///
/// `{id}` is `Kind:identifier` -- `Place:hazor-1`, `Event:ab_ur`,
/// `Person:aaron_1` -- or `text-unit:BOOK.CHAPTER.VERSE` for a verse of
/// Scripture and `text-unit:BoC PART.ARTICLE.PARAGRAPH` for a paragraph of the
/// Book of Concord. An id of no recognised kind is `bad_ref`; one that names no
/// node is `not_found`.
#[utoipa::path(get, path = "/api/node/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NodeCard), ReferenceRefusals), tag = "graph")]
pub async fn node_card(State(graph): State<Arc<GraphService>>, Reference(NodeReference(node_id)): Reference<NodeReference>) -> Result<Json<wire::NodeCard>, ApiError> {
    let snap = graph.snapshot();
    let node = snap.node(&node_id).ok_or_else(|| ApiError::not_found("node"))?;

    let summary = snap.edge_summary(&Position::Node(node_id.clone()));
    let label = crate::graph_wire::describe_node(&node_id, &snap);

    let edge_summary = summary.into_iter().map(|(kind, count)| wire::EdgeSummaryEntry { kind, count }).collect();

    let description = node_description(&node_id, &snap);
    let person = match &node.payload {
        atlas_graph_types::node::NodePayload::Person { gender, birth_year, death_year, also_called, first_year, last_year, eternal, eternal_grounds, .. } => Some(wire::PersonLife {
            gender: gender.clone(),
            birth: recorded_year(*birth_year, &node_id)?,
            death: recorded_year(*death_year, &node_id)?,
            first: recorded_year(*first_year, &node_id)?,
            last: recorded_year(*last_year, &node_id)?,
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

/// A year zero in a person's record is this atlas's own data defect, never a year to show.
fn recorded_year(year: Option<i32>, person: &AnyNodeId) -> Result<Option<wire::Year>, ApiError> {
    year.map(wire::Year::of).transpose().map_err(|_| ApiError::internal(&format!("{} records a year zero", person.raw)))
}

pub(crate) fn node_description(id: &AnyNodeId, q: &impl GraphQuery) -> Option<String> {
    let node = q.node(id)?;
    match node.payload {
        NodePayload::Place { description, .. } | NodePayload::Person { description, .. } | NodePayload::PeopleGroup { description, .. } => description,
        NodePayload::CommentaryItem { text, .. } => Some(text),
        _ => None,
    }
}

/// One page of a node's neighbours of a single kind, each with the id of the edge that joins them.
///
/// `{id}` takes the same form `/api/node/{id}` does. The required `kind` is an
/// edge label such as `cites` or `cited-by`; anything else is `bad_kind`, an
/// unrecognised id is `bad_ref`, and an id naming no node is `not_found`.
/// `limit` defaults to 20 and caps at 200; pass the response's `next` back as
/// `cursor` for the following page, and its absence is the last page. A `limit`
/// or `cursor` that does not read as a whole number is not refused: it leaves its
/// default standing.
#[utoipa::path(get, path = "/api/node/{id}/edges", params(("id" = String, Path), EdgePageQuery), responses((status = 200, body = wire::EdgePage), FrontierRefusals), tag = "graph")]
pub async fn node_edges(
    State(graph): State<Arc<GraphService>>,
    Reference(NodeReference(node_id)): Reference<NodeReference>,
    Contract(asked): Contract<EdgePageQuery>,
) -> Result<Json<wire::EdgePage>, ApiError> {
    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(ApiError::not_found("node"));
    }

    let page = snap.edges(&Position::Node(node_id), &asked.page());

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

    Ok(Json(wire::EdgePage { kind: asked.kind, entries, next: page.next, version: atlas_graph::version_hex(graph.version()) }))
}

/// Which of a node's frontiers to answer, and which page of it.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EdgePageQuery {
    pub kind: EdgeKind,
    #[serde(default)]
    #[param(value_type = Option<usize>)]
    pub cursor: AsGiven<usize>,
    #[serde(default)]
    #[param(value_type = Option<usize>)]
    pub limit: AsGiven<usize>,
}

const DEFAULT_EDGE_LIMIT: usize = 20;
const SMALLEST_EDGE_LIMIT: usize = 1;
const MAX_EDGE_LIMIT: usize = 200;

impl EdgePageQuery {
    fn page(&self) -> EdgeQuery {
        EdgeQuery {
            kind: self.kind,
            cursor: self.cursor.given(),
            limit: self.limit.given().unwrap_or(DEFAULT_EDGE_LIMIT).clamp(SMALLEST_EDGE_LIMIT, MAX_EDGE_LIMIT),
        }
    }
}

impl ContractParams for EdgePageQuery {
    /// A page bound reads through `AsGiven`, which leaves the route's own default
    /// standing rather than failing, so the kind is the only parameter that reaches
    /// here. The others answer for it anyway: the parameter is read off the caller's
    /// own query, so it is not a value to panic on, and `bad_kind` is the one refusal
    /// this route publishes.
    fn unreadable(_parameter: &str, asked_with: Option<&str>) -> ApiError {
        ApiError::bad_kind(asked_with.unwrap_or_default())
    }
}

/// A window of one corpus's reading spine: the units of text around the one a reference names, and the reference that continues the window.
///
/// `ref` is `BOOK.CHAPTER.VERSE`, or `BoC PART.ARTICLE.PARAGRAPH` when
/// `corpus=concord` (`corpus` is `bible` unless given, anything else is
/// `bad_corpus`); a malformed reference is `bad_ref` and one naming nothing in
/// the corpus is `not_found`. `n` defaults to 1 and caps at 500, and
/// `dir=backward` ends the window at `ref` instead of starting it there; a `dir`
/// that is neither is `bad_dir`. An `n` that does not read as a whole number is
/// not refused: it leaves the default standing. `scope=chapter` covers the whole
/// chapter named instead, and takes neither `n` nor `dir=backward` -- that
/// combination is `bad_dir`, and a `scope` outside the two is `bad_scope`. A
/// chapter is a Scripture reading and nothing else: `scope=chapter` with
/// `corpus=concord` is `bad_scope`, because the Book of Concord is read by the
/// article and an article's paragraph count is no fixed span; a Concord caller
/// asks for the paragraphs it wants with `n` instead. The response's `next` is the
/// reference one step further on, absent at the end of the corpus.
#[utoipa::path(get, path = "/api/text", params(TextWindowQuery), responses((status = 200, body = wire::TextWindow), ReadingWindowRefusals), tag = "graph")]
pub async fn text_window(
    State(graph): State<Arc<GraphService>>,
    headers: HeaderMap,
    Contract(asked): Contract<TextWindowQuery>,
) -> Result<Response, ApiError> {
    let etag = format!("\"{}\"", atlas_graph::version_hex(graph.version()));
    if headers.get(header::IF_NONE_MATCH).and_then(|v| v.to_str().ok()) == Some(etag.as_str()) {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }

    let raw_ref = asked.r#ref.as_str();
    let scope = asked.scope();
    let corpus = asked.corpus();

    if scope == wire::TextScope::Chapter && asked.dir == Some(WindowDir::Backward) {
        return Err(ApiError::bad_dir(
            "dir=backward is not supported with scope=chapter -- a chapter-scoped window's bounds are already fully determined by the chapter itself, so there is no direction left to walk; omit dir, or use dir=onward, or drop scope=chapter and anchor on a specific verse instead",
        ));
    }
    if corpus == wire::Corpus::Concord && scope == wire::TextScope::Chapter {
        return Err(ApiError::bad_scope(
            "scope=chapter is not supported with corpus=concord -- a Concord article's own paragraph count varies too widely for one server-derived span; omit scope (or use scope=verse) and set n explicitly instead",
        ));
    }

    let dir = asked.dir();
    let snap = graph.snapshot();

    if corpus == wire::Corpus::Concord {
        let asked_for: ConcordParagraphReference = raw_ref.parse().map_err(|_| ApiError::bad_ref(raw_ref))?;
        let start = graph.concord_position_of(asked_for.part, asked_for.article, asked_for.paragraph).ok_or_else(|| ApiError::not_found("concord paragraph"))?;
        let n = asked.units();

        let ids = window::window(&snap, corpus.name(), start, n, dir);
        let units: Vec<wire::TextUnit> = ids
            .iter()
            .filter_map(|id| {
                let (p, a, para) = atlas_graph::concord_adapter::decode_text_unit(id)?;
                let text = window::render_layer(&snap, id, atlas_graph::concord_adapter::CONCORD_TRANSLATION)?;
                Some(wire::TextUnit {
                    r#ref: format!("BoC {p}.{a}.{para}"),
                    locus: wire::TextRef::Concord { part: p, article: a, paragraph: para },
                    text,
                    words_of_christ: Vec::new(),
                    edge_summary: unit_edge_summary(&snap, id),
                })
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

    let asked_for: ReadingReference = raw_ref.parse().map_err(|_| ApiError::bad_ref(raw_ref))?;
    let (book, chapter) = (asked_for.chapter.book.0, asked_for.chapter.chapter);

    let (start, n) = if scope == wire::TextScope::Chapter {
        graph.chapter_span(book, chapter).ok_or_else(|| ApiError::not_found("chapter"))?
    } else {
        let verse = asked_for.verse.ok_or_else(|| ApiError::bad_ref(raw_ref))?;
        let start = graph.position_of(book, chapter, verse).ok_or_else(|| ApiError::not_found("verse"))?;
        (start, asked.units())
    };

    let ids = window::window(&snap, corpus.name(), start, n, dir);
    let units: Vec<wire::TextUnit> = ids
        .iter()
        .filter_map(|id| {
            let (b, c, v) = atlas_graph::kjv_adapter::decode_text_unit(id)?;
            let text = window::render(&snap, id)?;
            let r#ref = atlas_graph::kjv_adapter::dot_ref(b, c, v);
            let words_of_christ = graph.red_letter_spans.get(&r#ref).map(|spans| spans.iter().map(|&(start, end)| crate::wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();
            let locus = wire::TextRef::of_verse(&VerseRef { book: b, chapter: c, verse: v });
            Some(wire::TextUnit { r#ref, locus, text, words_of_christ, edge_summary: unit_edge_summary(&snap, id) })
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

/// The reading window one request asks for.
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct TextWindowQuery {
    pub r#ref: String,
    #[serde(default)]
    #[param(value_type = Option<usize>)]
    pub n: AsGiven<usize>,
    #[serde(default)]
    pub dir: Option<WindowDir>,
    #[serde(default)]
    pub scope: Option<wire::TextScope>,
    #[serde(default)]
    pub corpus: Option<wire::Corpus>,
}

const DEFAULT_WINDOW_UNITS: usize = 1;
const SMALLEST_WINDOW: usize = 1;
const MAX_WINDOW_UNITS: usize = 500;

impl TextWindowQuery {
    fn units(&self) -> usize {
        self.n.given().unwrap_or(DEFAULT_WINDOW_UNITS).clamp(SMALLEST_WINDOW, MAX_WINDOW_UNITS)
    }

    fn scope(&self) -> wire::TextScope {
        self.scope.unwrap_or(wire::TextScope::Verse)
    }

    fn corpus(&self) -> wire::Corpus {
        self.corpus.unwrap_or(wire::Corpus::Bible)
    }

    fn dir(&self) -> WindowDir {
        self.dir.unwrap_or(WindowDir::Onward)
    }
}

impl ContractParams for TextWindowQuery {
    /// The window size reads through `AsGiven`, which leaves one unit standing rather
    /// than failing, so it never reaches here; it and any other name answer for the
    /// reference, which is both a code this route publishes and the only one a caller
    /// who mis-typed something unnamed can act on. The name is read off the caller's
    /// own query, so it is not a value to refuse to answer for.
    fn unreadable(parameter: &str, asked_with: Option<&str>) -> ApiError {
        let asked_with = asked_with.unwrap_or_default();
        match parameter {
            query::SCOPE => ApiError::unknown_scope(asked_with),
            query::DIR => ApiError::unknown_dir(asked_with),
            query::CORPUS => ApiError::bad_corpus(asked_with),
            _ => ApiError::bad_ref(asked_with),
        }
    }
}

fn unit_edge_summary(snap: &impl atlas_graph_types::store::GraphQuery, id: &atlas_graph_types::id::AnyNodeId) -> Vec<wire::EdgeSummaryEntry> {
    snap.edge_summary(&Position::Node(id.clone())).into_iter().map(|(kind, count)| wire::EdgeSummaryEntry { kind, count }).collect()
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(node_card))
        .routes(routes!(node_edges))
        .routes(routes!(text_window))
}
