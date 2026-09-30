use std::cell::OnceCell;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use atlas_core::data::{AtlasData, Canon, Event, Place, PlaceDateClaim};
use atlas_core::history::resolve_display_name_and_canonical;
use atlas_core::refs::{BookId, VerseId};
use atlas_core::scene::{accounts_of, Account};
use atlas_graph::event_world::{event_node_id, ChronologyDerivation};
use atlas_graph::heading::Heading;
use atlas_graph::kjv_adapter::verse_node_id;
use atlas_graph::runs;
use atlas_graph::tokens;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeKind, RelationId};
use atlas_graph_types::explore::{EdgeMeta, EdgeQuery};
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{BibleLocusRange, ConcordRef, Locus, TokenSpan, VerseRef};

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
pub async fn node_card(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(NodeReference(node_id)): Reference<NodeReference>,
) -> Result<Json<wire::NodeCard>, ApiError> {
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
        event: atlas_graph::legacy::event_from_node(&node_id, &snap, &graph.chronology.chrono).map(|event| event_detail(&event)),
        place: atlas_graph::legacy::place_from_node(&node_id, &snap).map(|place| place_detail(&place, &data, &snap)),
        catechism: catechism_detail(&node_id, &data),
        book: book_detail(&node_id, &data, &snap)?,
    }))
}

/// A year zero in a person's record is this atlas's own data defect, never a year to show.
fn recorded_year(year: Option<i32>, person: &AnyNodeId) -> Result<Option<wire::Year>, ApiError> {
    year.map(wire::Year::of).transpose().map_err(|_| ApiError::internal(&format!("{} records a year zero", person.raw)))
}

fn event_detail(event: &Event) -> wire::EventDetail {
    wire::EventDetail {
        kind: event.kind,
        when: event.date().map(wire::TimeRange::of),
        robertson_section: event.robertson_section.clone(),
        acts_section: event.acts_section.clone(),
        atlas_section: event.atlas_section.clone(),
        kjv_superscription: event.kjv_superscription.clone(),
        ref_note: event.ref_note.clone(),
    }
}

/// A card names a place with no years in view, so its name is the translation's own
/// wording where one is recorded, never a period name.
fn place_detail(place: &Place, data: &AtlasData, snap: &impl GraphQuery) -> wire::PlaceDetail {
    let history = data.place_history_for(&place.id);
    let (display_name, canonical_name) = resolve_display_name_and_canonical(&place.name, history, None, data.place_name_alias_for(&place.id));
    wire::PlaceDetail {
        lat: place.lat,
        lon: place.lon,
        display_name,
        canonical_name,
        established: history.and_then(|h| h.established.as_ref()).map(|claim| date_claim(claim, snap)),
        destroyed: history.and_then(|h| h.destroyed.as_ref()).map(|claim| date_claim(claim, snap)),
    }
}

fn date_claim(claim: &PlaceDateClaim, snap: &impl GraphQuery) -> wire::DateClaim {
    let verses = claim
        .verses
        .iter()
        .map(|v| {
            let verse = VerseId::parse_canonical(v).expect("a place history's verses are checked when the atlas is compiled");
            wire::TextSpan::whole(wire::TextRef::Bible { book: verse.book, chapter: verse.chapter, verse: verse.verse })
        })
        .collect();
    let event = claim.event.as_ref().map(|event| describe_position(&Position::Node(event.erase()), snap));
    wire::DateClaim::of(wire::TimeRange::of(claim.when), verses, claim.note.clone(), event)
}

/// An item's id is unique only among catechism items, so any other node that shares
/// one is not that item.
fn catechism_detail(id: &AnyNodeId, data: &AtlasData) -> Option<wire::CatechismDetail> {
    if id.kind != NodeKind::CatechismItem {
        return None;
    }
    let (part, item) = data.catechism_item_by_id(&id.raw)?;
    Some(wire::CatechismDetail {
        part_title: part.title.clone(),
        text: item.text.clone(),
        explanation_heading: item.explanation_heading.clone(),
        explanation: item.explanation.clone(),
        where_written: item.where_written.clone(),
    })
}

/// A book dated at one end only is this atlas's own data defect, never a span to show.
fn book_detail(id: &AnyNodeId, data: &AtlasData, snap: &impl GraphQuery) -> Result<Option<wire::BookDetail>, ApiError> {
    let Some(code) = atlas_graph::bible_container_adapter::decode_book_container(id).map(|index| BookId(index).code()) else {
        return Ok(None);
    };
    let Some(meta) = data.books_meta.iter().find(|meta| meta.book == code) else {
        return Ok(None);
    };
    let written = meta.written().map_err(|refused| ApiError::internal(&format!("the writing date {code} records is refused: {refused}")))?;
    Ok(Some(wire::BookDetail {
        author: meta.author.clone(),
        write_place: meta.write_place.as_ref().map(|place| describe_position(&Position::Node(atlas_graph::event_world::place_stub_node_id(place)), snap)),
        written: written.map(wire::TimeRange::of),
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
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(NodeReference(node_id)): Reference<NodeReference>,
    Contract(asked): Contract<EdgePageQuery>,
) -> Result<Json<wire::EdgePage>, ApiError> {
    let snap = graph.snapshot();
    if snap.node(&node_id).is_none() {
        return Err(ApiError::not_found("node"));
    }

    let page = snap.edges(&Position::Node(node_id.clone()), &asked.page());

    // A PeopleGroup wire id does not decode, so an entry naming one would hand the
    // caller a reference it cannot fetch a card for.
    let this_event = OnceCell::new();
    let entries = page
        .entries
        .iter()
        .filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup))
        .map(|e| {
            let (votes, narrative, parentage) = match &e.meta {
                EdgeMeta::Votes(votes) => (Some(*votes), None, None),
                EdgeMeta::Narrative(narrative) => (None, Some(narrative.clone()), None),
                EdgeMeta::Parentage(parentage) => (None, None, Some(*parentage)),
                EdgeMeta::None => (None, None, None),
            };
            let (loci, note) = match (asked.kind, &e.node) {
                (EdgeKind::Directed(RelationId::Attests, Direction::Forward), Position::Node(verse)) => {
                    let accounts = this_event.get_or_init(|| EventAccounts::read(&node_id, &snap, &graph.chronology.chrono));
                    let (_, account) = accounts.holding(verse);
                    (Some(account.runs(&data.canon).to_vec()), account.note.clone())
                }
                (EdgeKind::Directed(RelationId::Attests, Direction::Inverse), Position::Node(event)) => {
                    let accounts = EventAccounts::read(event, &snap, &graph.chronology.chrono);
                    let (verse, account) = accounts.holding(&node_id);
                    (Some(vec![wire::TextSpan::whole(wire::TextRef::of_verse(verse))]), account.note.clone())
                }
                _ => (None, None),
            };
            wire::EdgeEntry { edge: e.edge.0.clone(), node: describe_position(&e.node, &snap), votes, narrative, loci, note, parentage }
        })
        .collect();

    Ok(Json(wire::EdgePage { kind: asked.kind, entries, next: page.next, version: atlas_graph::version_hex(graph.version()) }))
}

/// An event's accounts, read once for every attestation of it a page lists. Every
/// attestation row is built from one of these accounts, so an event the graph cannot
/// read back, or a verse none of its accounts reads, is this atlas's own defect.
struct EventAccounts {
    event: AnyNodeId,
    accounts: Vec<AccountOf>,
}

/// One account of an event: the verses it reads, how it is cited where that needed
/// saying, and -- only once something asks -- the runs those verses read on in.
struct AccountOf {
    verses: Vec<VerseRef>,
    note: Option<String>,
    runs: OnceCell<Vec<wire::TextSpan>>,
}

impl EventAccounts {
    fn read(event: &AnyNodeId, snap: &impl GraphQuery, chrono: &ChronologyDerivation) -> EventAccounts {
        let record = atlas_graph::legacy::event_from_node(event, snap, chrono)
            .unwrap_or_else(|| panic!("{} attests verses, but the graph holds no such event", encode_node_id(event)));
        let accounts = accounts_of(&record).into_iter().map(|account| AccountOf { verses: account_verses(&account), note: account.ref_note, runs: OnceCell::new() }).collect();
        EventAccounts { event: event.clone(), accounts }
    }

    fn holding(&self, verse: &AnyNodeId) -> (&VerseRef, &AccountOf) {
        self.accounts
            .iter()
            .find_map(|account| account.verses.iter().find(|v| verse_node_id(v.book, v.chapter, v.verse) == *verse).map(|v| (v, account)))
            .unwrap_or_else(|| panic!("the attestation of {} at {} belongs to none of its accounts", encode_node_id(&self.event), encode_node_id(verse)))
    }
}

impl AccountOf {
    fn runs(&self, canon: &Canon) -> &[wire::TextSpan] {
        self.runs.get_or_init(|| {
            let verses: Vec<BibleLocusRange> = self.verses.iter().map(|v| BibleLocusRange { from: Locus::whole(v.clone()), to: Locus::whole(v.clone()) }).collect();
            runs::coalesce(&verses, canon).iter().map(|run| wire::TextSpan::of_bible_range(run).expect("runs of whole verses count no words")).collect()
        })
    }
}

fn account_verses(account: &Account) -> Vec<VerseRef> {
    account
        .verses
        .iter()
        .filter_map(|v| VerseId::parse_canonical(v).ok())
        .map(|v| VerseRef { book: v.book.0, chapter: v.chapter, verse: v.verse })
        .collect()
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
                let paragraph = ConcordRef { part: p, article: a, paragraph: para };
                Some(citation_anchors(&graph, &snap, &paragraph, &text).map(|anchors| wire::TextUnit {
                    r#ref: format!("BoC {p}.{a}.{para}"),
                    locus: wire::TextRef::Concord { part: p, article: a, paragraph: para },
                    text,
                    words_of_christ: Vec::new(),
                    heading: None,
                    anchors,
                    edge_summary: unit_edge_summary(&snap, id),
                }))
            })
            .collect::<Result<_, _>>()?;

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
            let verse = VerseRef { book: b, chapter: c, verse: v };
            let locus = wire::TextRef::of_verse(&verse);
            let heading = graph.heading_index.get(&r#ref).map(|heading| unit_heading(heading, &snap));
            Some(mention_anchors(&graph, &snap, &verse, &text).map(|anchors| wire::TextUnit { r#ref, locus, text, words_of_christ, heading, anchors, edge_summary: unit_edge_summary(&snap, id) }))
        })
        .collect::<Result<_, _>>()?;

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

const MENTIONS: EdgeKind = EdgeKind::Directed(RelationId::Mentions, Direction::Forward);
const CITES: EdgeKind = EdgeKind::Directed(RelationId::Cites, Direction::Forward);

fn mention_anchors(graph: &GraphService, snap: &impl GraphQuery, verse: &VerseRef, text: &str) -> Result<Vec<wire::Anchor>, ApiError> {
    let spans = graph.mention_spans_at(verse).map_err(|e| ApiError::internal(&format!("the mentions of a verse could not be read: {e}")))?;
    Ok(anchors_over(text, MENTIONS, spans.into_iter().map(|span| (span.words, describe_position(&Position::Node(span.entity.node_id()), snap)))))
}

fn citation_anchors(graph: &GraphService, snap: &impl GraphQuery, paragraph: &ConcordRef, text: &str) -> Result<Vec<wire::Anchor>, ApiError> {
    let spans = graph.citation_spans_at(paragraph).map_err(|e| ApiError::internal(&format!("the citations of a paragraph could not be read: {e}")))?;
    Ok(anchors_over(text, CITES, spans.into_iter().map(|span| (span.words, describe_position(&Position::Node(verse_node_id(span.cites.book, span.cites.chapter, span.cites.verse)), snap)))))
}

fn anchors_over(text: &str, kind: EdgeKind, links: impl Iterator<Item = (TokenSpan, wire::NodeRef)>) -> Vec<wire::Anchor> {
    let words = tokens::tokenize(text);
    let mut anchors: Vec<wire::Anchor> = links
        .map(|(span, node)| {
            let chars = tokens::chars_of(&span, &words).unwrap_or_else(|| panic!("the words {}..={} of {} lie past the text of the unit they are stored on", span.start, span.end, node.id));
            wire::Anchor { start: chars.start, end: chars.end, kind, node }
        })
        .collect();
    anchors.sort_by_key(|anchor| anchor.start);
    anchors
}

fn unit_heading(heading: &Heading, snap: &impl GraphQuery) -> wire::UnitHeading {
    wire::UnitHeading {
        event: describe_position(&Position::Node(event_node_id(&heading.event_id)), snap),
        kind: heading.kind,
        is_continuation: heading.is_continuation,
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

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::graph::Graph;

    const GENESIS: u8 = 0;

    #[test]
    #[should_panic(expected = "Event:ab_ur attests verses, but the graph holds no such event")]
    fn an_attestation_whose_event_the_graph_cannot_read_is_a_graph_defect_not_an_entry_without_its_account() {
        // Arrange
        let unread = event_node_id("ab_ur");

        // Act
        EventAccounts::read(&unread, &Graph::default(), &ChronologyDerivation::default());
    }

    #[test]
    #[should_panic(expected = "the words 9..=10 of Place:hazor-1 lie past the text of the unit they are stored on")]
    fn a_word_span_past_its_units_text_is_a_graph_defect_not_an_anchor() {
        // Arrange
        let past_the_end = tokens::span(atlas_graph::kjv_adapter::KJV_TRANSLATION, 9, 10).unwrap();
        let hazor = wire::NodeRef { id: "Place:hazor-1".to_string(), kind: wire::PositionKind::Node(NodeKind::Place), label: "Hazor 1".to_string() };

        // Act
        anchors_over("In the beginning God created the heaven and the earth.", MENTIONS, std::iter::once((past_the_end, hazor)));
    }

    #[test]
    #[should_panic(expected = "the attestation of Event:ab_ur at text-unit:GEN.1.2 belongs to none of its accounts")]
    fn an_attestation_at_a_verse_none_of_its_accounts_reads_is_a_graph_defect_not_an_entry_without_its_account() {
        // Arrange
        let accounts = EventAccounts { event: event_node_id("ab_ur"), accounts: vec![AccountOf { verses: vec![VerseRef { book: GENESIS, chapter: 1, verse: 1 }], note: None, runs: OnceCell::new() }] };

        // Act
        accounts.holding(&verse_node_id(GENESIS, 1, 2));
    }
}
