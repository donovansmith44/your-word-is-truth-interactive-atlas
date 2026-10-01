use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::RangeInclusive;
use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use utoipa::IntoParams;

use atlas_core::data::{AtlasData, Canon, Event, Place, PlaceDateClaim};
use atlas_core::refs::{BookId, VerseId};
use atlas_core::scene::{accounts_of, Account};
use atlas_graph::event_world::{event_node_id, ChronologyDerivation};
use atlas_graph::heading::Heading;
use atlas_graph::kjv_adapter::verse_node_id;
use atlas_graph::mention_spans::MentionSpan;
use atlas_graph::runs;
use atlas_graph::tokens;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::sqlite::SqliteError;
use atlas_graph::GraphService;
use atlas_graph_types::edge::{Direction, EdgeId, EdgeKind, RelationId};
use atlas_graph_types::adjacency::{EdgeMeta, EdgeQuery};
use atlas_graph_types::id::{AnyNodeId, NodeKind, Position};
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::store::GraphQuery;
use atlas_graph_types::text::{BibleLocusRange, ConcordRef, Locus, TokenSpan, VerseRef};

use crate::error::{ApiError, ElementRefusals, NeighbourRefusals, ReadingWindowRefusals, ReferenceRefusals};
use crate::graph_wire::{describe_nodes, describe_positions, edge_ref, encode_node_id, labelled_positions, node_ref};
use crate::query::{self, AsGiven, Contract, ContractParams};
use crate::reference::{ConcordParagraphReference, ElementId, ElementIds, NodeReference, PositionReference, ReadingReference, Reference};
use crate::wire;

/// One node of the graph at a glance: what it is, what to call it, where it came from, and how many neighbours it has of each kind.
///
/// `{id}` is `Kind:identifier` -- `Place:hazor-1`, `Event:ab_ur`,
/// `Person:aaron_1` -- or `text-unit:BOOK.CHAPTER.VERSE` for a verse of
/// Scripture and `text-unit:BoC PART.ARTICLE.PARAGRAPH` for a paragraph of the
/// Book of Concord. An id of no recognised kind is `bad_ref`; one that names no
/// node is `not_found`.
#[utoipa::path(get, path = "/api/node/{id}", params(("id" = String, Path)), responses((status = 200, body = wire::NodeRecord), ReferenceRefusals), tag = "graph")]
pub async fn node_record(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(NodeReference(node_id)): Reference<NodeReference>,
) -> Result<Json<wire::NodeRecord>, ApiError> {
    let snap = graph.snapshot();
    read_node_record(&node_id, &data, &graph, &snap)?.map(Json).ok_or_else(|| ApiError::not_found("node"))
}

fn read_node_record(node_id: &AnyNodeId, data: &AtlasData, graph: &GraphService, snap: &impl GraphQuery) -> Result<Option<wire::NodeRecord>, ApiError> {
    let Some(node) = snap.node(node_id) else { return Ok(None) };
    let label = crate::graph_wire::describe_node(node_id, snap)?;
    let edge_summary = summary_at(snap, &Position::Node(node_id.clone()));
    let description = node_description(node_id, snap);
    let person = match &node.payload {
        NodePayload::Person { gender, birth_year, death_year, also_called, first_year, last_year, eternal, eternal_grounds, .. } => Some(wire::PersonLife {
            gender: gender.clone(),
            birth: recorded_year(*birth_year, node_id)?,
            death: recorded_year(*death_year, node_id)?,
            first: recorded_year(*first_year, node_id)?,
            last: recorded_year(*last_year, node_id)?,
            eternal: *eternal,
            eternal_grounds: eternal_grounds.clone(),
            also_called: also_called.clone(),
        }),
        _ => None,
    };
    let place = atlas_graph::legacy::place_from_node(node_id, snap).map(|place| place_detail(&place, node_id, data, graph, snap)).transpose()?;
    let (map, era, polity) = match &node.payload {
        NodePayload::Map { from_year, to_year, .. } => (Some(wire::MapDetail { window: crate::map::curated_span(*from_year, *to_year) }), None, None),
        NodePayload::Era { from_year, to_year, .. } => (None, Some(wire::EraDetail { window: crate::map::curated_span(*from_year, *to_year) }), None),
        NodePayload::Polity { .. } => (None, None, Some(polity_detail(node_id, graph)?)),
        _ => (None, None, None),
    };

    Ok(Some(wire::NodeRecord {
        id: encode_node_id(node_id),
        kind: node_id.kind,
        label,
        provenance: node.provenance.clone(),
        edge_summary,
        version: atlas_graph::version_hex(graph.version()),
        person,
        description,
        event: atlas_graph::legacy::event_from_node(node_id, snap, &graph.chronology.chrono).map(|event| event_detail(&event)),
        place,
        catechism: catechism_detail(node_id, data),
        book: book_detail(node_id, data, snap)?,
        map,
        era,
        polity,
    }))
}

fn polity_detail(polity: &AnyNodeId, graph: &GraphService) -> Result<wire::PolityDetail, ApiError> {
    let reign = graph.geography.reign_of(polity).ok_or_else(|| ApiError::internal(&format!("{} is a polity and no reign is compiled for it", encode_node_id(polity))))?;
    Ok(wire::PolityDetail { reign: wire::TimeRange::of(reign) })
}

fn read_edge_record(id: &EdgeId, snap: &impl GraphQuery) -> Result<Option<wire::EdgeRecord>, ApiError> {
    let Some(record) = snap.edge(id) else { return Ok(None) };
    let at = Position::Edge(id.clone());
    let [label, subject, object]: [String; 3] = labelled_positions(&[at.clone(), record.subject.clone(), record.object.clone()], snap)?
        .try_into()
        .map_err(|_| ApiError::internal("three positions were asked for and three labels were not answered"))?;
    let (votes, narrative, parentage) = recorded(&record.meta);
    Ok(Some(wire::EdgeRecord {
        id: id.0.clone(),
        kind: record.kind,
        label,
        subject: described_as(&record.subject, subject)?,
        object: described_as(&record.object, object)?,
        provenance: snap.row_provenance(id).map(|row| row.provenance),
        votes,
        narrative,
        parentage,
        edge_summary: summary_at(snap, &at),
    }))
}

fn described_as(at: &Position, label: String) -> Result<wire::PositionRef, ApiError> {
    match at {
        Position::Node(id) => Ok(wire::PositionRef::Node { node: wire::NodeRef { id: encode_node_id(id), kind: id.kind, label } }),
        Position::Edge(id) => edge_ref(id, label).map(|edge| wire::PositionRef::Edge { edge }),
    }
}

fn recorded(meta: &EdgeMeta) -> (Option<u32>, Option<atlas_graph_types::id::NarrativeId>, Option<atlas_graph_types::edge::Parentage>) {
    match meta {
        EdgeMeta::Votes(votes) => (Some(*votes), None, None),
        EdgeMeta::Narrative(narrative) => (None, Some(narrative.clone()), None),
        EdgeMeta::Parentage(parentage) => (None, None, Some(*parentage)),
        EdgeMeta::None => (None, None, None),
    }
}

fn summary_at(snap: &impl GraphQuery, at: &Position) -> Vec<wire::EdgeSummaryEntry> {
    snap.edge_summary(at).into_iter().map(|(kind, count)| wire::EdgeSummaryEntry { kind, count }).collect()
}

pub fn read_elements(data: &AtlasData, graph: &GraphService, snap: &impl GraphQuery, ids: &[ElementId]) -> Result<Vec<wire::Element>, ApiError> {
    ids.iter()
        .map(|id| {
            let read = match id {
                ElementId::Node(node) => read_node_record(node, data, graph, snap)?.map(|node| wire::Element::Node { node }),
                ElementId::Edge(edge) => read_edge_record(edge, snap)?.map(|edge| wire::Element::Edge { edge }),
            };
            Ok(read.unwrap_or_else(|| wire::Element::Missing { id: element_wire_id(id) }))
        })
        .collect()
}

fn element_wire_id(id: &ElementId) -> String {
    match id {
        ElementId::Node(node) => encode_node_id(node),
        ElementId::Edge(edge) => edge.0.clone(),
    }
}

#[utoipa::path(
    get,
    path = "/api/elements",
    summary = "Nodes and edges of the graph by id, many in one request, each answered in the order asked.",
    description = "`ids` lists node ids (the form `/api/node/{id}` takes) and edge ids (the id an edge page carries for its edge), separated by commas: `ids=Event:ab_ur,LocatedAt:…`. Each id is answered by its own record, or by `missing` where it reads as an id but names nothing. One id that does not read as a node's or an edge's refuses the whole read as `bad_ref`, as does asking for none. At most the server's largest page of ids is answered at once: `next`, when present, is the `cursor` that reads the ids that follow, and its absence is the last page.",
    params(ElementsQuery),
    responses((status = 200, body = wire::ElementPage), ElementRefusals),
    tag = "graph"
)]
pub async fn elements(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Contract(asked): Contract<ElementsQuery>,
) -> Result<Json<wire::ElementPage>, ApiError> {
    let snap = graph.snapshot();
    let from = asked.cursor.given().unwrap_or(0).min(asked.ids.0.len());
    let to = from.saturating_add(LARGEST_PAGE).min(asked.ids.0.len());
    let elements = read_elements(&data, &graph, &snap, &asked.ids.0[from..to])?;
    let next = (to < asked.ids.0.len()).then_some(to);
    Ok(Json(wire::ElementPage { elements, next, version: atlas_graph::version_hex(graph.version()) }))
}

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ElementsQuery {
    #[param(value_type = Vec<String>, style = Form, explode = false, min_items = 1)]
    pub ids: ElementIds,
    #[serde(default)]
    #[param(value_type = Option<usize>)]
    pub cursor: AsGiven<usize>,
}

impl ContractParams for ElementsQuery {
    fn unreadable(_parameter: &str, asked_with: Option<&str>) -> ApiError {
        ApiError::bad_ref(asked_with.unwrap_or_default())
    }
}

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

fn place_detail(place: &Place, node_id: &AnyNodeId, data: &AtlasData, graph: &GraphService, snap: &impl GraphQuery) -> Result<wire::PlaceDetail, ApiError> {
    let history = data.place_history_for(&place.id);
    let default = graph.geography.place_default(node_id).ok_or_else(|| ApiError::internal(&format!("{} is a place and no default is compiled for it", encode_node_id(node_id))))?;
    Ok(wire::PlaceDetail {
        lat: place.lat,
        lon: place.lon,
        display_name: default.display_name.clone(),
        canonical_name: default.canonical_name.clone(),
        blurb: default.blurb.clone(),
        established: history.and_then(|h| h.established.as_ref()).map(|claim| date_claim(claim, snap)).transpose()?,
        destroyed: history.and_then(|h| h.destroyed.as_ref()).map(|claim| date_claim(claim, snap)).transpose()?,
    })
}

fn date_claim(claim: &PlaceDateClaim, snap: &impl GraphQuery) -> Result<wire::DateClaim, ApiError> {
    let verses = claim
        .verses
        .iter()
        .map(|v| {
            let verse = VerseId::parse_canonical(v).expect("a place history's verses are checked when the atlas is compiled");
            wire::TextSpan::whole(wire::TextRef::Bible { book: verse.book, chapter: verse.chapter, verse: verse.verse })
        })
        .collect();
    let event = claim.event.as_ref().map(|event| node_ref(&event.erase(), snap)).transpose()?;
    Ok(wire::DateClaim::of(wire::TimeRange::of(claim.when), verses, claim.note.clone(), event))
}

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
        write_place: meta.write_place.as_ref().map(|place| node_ref(&atlas_graph::event_world::place_stub_node_id(place), snap)).transpose()?,
        written: written.map(wire::TimeRange::of),
    }))
}

fn node_description(id: &AnyNodeId, q: &impl GraphQuery) -> Option<String> {
    let node = q.node(id)?;
    match node.payload {
        NodePayload::Place { description, .. } | NodePayload::Person { description, .. } | NodePayload::PeopleGroup { description, .. } => description,
        NodePayload::CommentaryItem { text, .. } => Some(text),
        NodePayload::Anchor { citation, .. } => Some(citation),
        _ => None,
    }
}

#[utoipa::path(
    get,
    path = "/api/node/{id}/edges",
    summary = "One page of the neighbours of a node or of an edge, of a single kind, each with the edge that joins them.",
    description = "`{id}` takes the same form `/api/node/{id}` does, or an edge's id as an edge page carries it. The required `kind` is an edge label such as `cites` or `cited-by`; anything else is `bad_kind`, an unrecognised id is `bad_ref`, and an id naming nothing is `not_found`. `limit` defaults to 20 and is clamped to the server's largest page; pass the response's `next` back as `cursor` for the following page, and its absence is the last page. A `limit` or `cursor` that does not read as a whole number is not refused: it leaves its default standing.",
    params(("id" = String, Path), EdgePageQuery),
    responses((status = 200, body = wire::EdgePage), NeighbourRefusals),
    tag = "graph"
)]
pub async fn node_edges(
    State(data): State<Arc<AtlasData>>,
    State(graph): State<Arc<GraphService>>,
    Reference(PositionReference(asked_at)): Reference<PositionReference>,
    Contract(asked): Contract<EdgePageQuery>,
) -> Result<Json<wire::EdgePage>, ApiError> {
    let snap = graph.snapshot();
    let at = match asked_at {
        ElementId::Node(node_id) => snap.node(&node_id).map(|_| Position::Node(node_id)).ok_or_else(|| ApiError::not_found("node"))?,
        ElementId::Edge(edge_id) => snap.edge(&edge_id).map(|_| Position::Edge(edge_id)).ok_or_else(|| ApiError::not_found("edge"))?,
    };

    let page = snap.edges(&at, &asked.page());
    let listed: Vec<&atlas_graph_types::adjacency::EdgeEntry> = page.entries.iter().filter(|e| !matches!(&e.node, Position::Node(id) if id.kind == NodeKind::PeopleGroup)).collect();
    let neighbours = describe_positions(&listed.iter().map(|e| e.node.clone()).collect::<Vec<_>>(), &snap)?;
    let edge_labels = labelled_positions(&listed.iter().map(|e| Position::Edge(e.edge.clone())).collect::<Vec<_>>(), &snap)?;

    let this_event = OnceCell::new();
    let mut mention_loci = MentionLoci::default();
    let mut entries = Vec::with_capacity(listed.len());
    for ((e, neighbour), edge_label) in listed.into_iter().zip(neighbours).zip(edge_labels) {
        let (votes, narrative, parentage) = recorded(&e.meta);
        let (loci, note) = match (&at, asked.kind, &e.node) {
            (Position::Node(node_id), EdgeKind::Directed(RelationId::Attests, Direction::Forward), Position::Node(verse)) => {
                let accounts = this_event.get_or_init(|| EventAccounts::read(node_id, &snap, &graph.chronology.chrono));
                let (_, account) = accounts.holding(verse);
                (Some(account.runs(&data.canon).to_vec()), account.note.clone())
            }
            (Position::Node(node_id), EdgeKind::Directed(RelationId::Attests, Direction::Inverse), Position::Node(event)) => {
                let accounts = EventAccounts::read(event, &snap, &graph.chronology.chrono);
                let (verse, account) = accounts.holding(node_id);
                (Some(vec![wire::TextSpan::whole(wire::TextRef::of_verse(verse))]), account.note.clone())
            }
            (Position::Node(node_id), EdgeKind::Directed(RelationId::Mentions, Direction::Forward), Position::Node(entity)) => (mention_loci.of(&graph, node_id, entity)?, None),
            (Position::Node(node_id), EdgeKind::Directed(RelationId::Mentions, Direction::Inverse), Position::Node(verse)) => (mention_loci.of(&graph, verse, node_id)?, None),
            _ => (None, None),
        };
        entries.push(wire::EdgeEntry { edge: edge_ref(&e.edge, edge_label)?, neighbour, votes, narrative, loci, note, parentage });
    }

    Ok(Json(wire::EdgePage { kind: asked.kind, entries, next: page.next, version: atlas_graph::version_hex(graph.version()) }))
}

#[derive(Default)]
struct MentionLoci {
    at: BTreeMap<VerseRef, Vec<MentionSpan>>,
}

impl MentionLoci {
    fn of(&mut self, graph: &GraphService, verse: &AnyNodeId, entity: &AnyNodeId) -> Result<Option<Vec<wire::TextSpan>>, ApiError> {
        let Some((book, chapter, number)) = atlas_graph::kjv_adapter::decode_text_unit(verse) else { return Ok(None) };
        let at = VerseRef { book, chapter, verse: number };
        if !self.at.contains_key(&at) {
            let spans = graph.mention_spans_in(&(at.clone()..=at.clone())).map_err(|e| unreadable_rows("mentions", &e))?;
            self.at.insert(at.clone(), spans.into_values().flatten().collect());
        }
        let loci = self.at[&at]
            .iter()
            .filter(|span| span.entity.node_id() == *entity)
            .map(|span| {
                let words = Locus { unit: at.clone(), span: Some(span.words.clone()) };
                wire::TextSpan::of_bible_range(&BibleLocusRange { from: words.clone(), to: words })
                    .map_err(|foreign| ApiError::internal(&format!("a mention of {} at {} lies in the {} layer, not the KJV's", entity.raw, verse.raw, foreign.layer.0)))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((!loci.is_empty()).then_some(loci))
    }
}

struct EventAccounts {
    event: AnyNodeId,
    accounts: Vec<AccountOf>,
}

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
pub const LARGEST_PAGE: usize = 200;

impl EdgePageQuery {
    fn page(&self) -> EdgeQuery {
        EdgeQuery {
            kind: self.kind,
            cursor: self.cursor.given(),
            limit: self.limit.given().unwrap_or(DEFAULT_EDGE_LIMIT).clamp(SMALLEST_EDGE_LIMIT, LARGEST_PAGE),
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
        let units = concord_text_units(&graph, &snap, &ids)?;

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
    let units = bible_text_units(&graph, &snap, &ids)?;

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

pub fn bible_text_units(graph: &GraphService, snap: &impl GraphQuery, ids: &[AnyNodeId]) -> Result<Vec<wire::TextUnit>, ApiError> {
    let verses: Vec<(&AnyNodeId, VerseRef)> = ids
        .iter()
        .filter_map(|id| atlas_graph::kjv_adapter::decode_text_unit(id).map(|(book, chapter, verse)| (id, VerseRef { book, chapter, verse })))
        .collect();
    let mut links = mention_links(graph, snap, verses.iter().map(|(_, verse)| verse))?;
    verses
        .into_iter()
        .filter_map(|(id, verse)| window::render(snap, id).map(|text| (id, verse, text)))
        .map(|(id, verse, text)| {
            let r#ref = atlas_graph::kjv_adapter::dot_ref(verse.book, verse.chapter, verse.verse);
            let words_of_christ = graph.red_letter_spans.get(&r#ref).map(|spans| spans.iter().map(|&(start, end)| crate::wire::WordsOfChristSpan { start, end }).collect()).unwrap_or_default();
            let locus = wire::TextRef::of_verse(&verse);
            let heading = graph.heading_index.get(&r#ref).map(|heading| unit_heading(heading, snap)).transpose()?;
            let anchors = anchors_over(&text, MENTIONS, links.remove(&verse).unwrap_or_default());
            Ok(wire::TextUnit { r#ref, locus, text, words_of_christ, heading, anchors, edge_summary: unit_edge_summary(snap, id) })
        })
        .collect()
}

pub fn concord_text_units(graph: &GraphService, snap: &impl GraphQuery, ids: &[AnyNodeId]) -> Result<Vec<wire::TextUnit>, ApiError> {
    let paragraphs: Vec<(&AnyNodeId, ConcordRef)> = ids
        .iter()
        .filter_map(|id| atlas_graph::concord_adapter::decode_text_unit(id).map(|(part, article, paragraph)| (id, ConcordRef { part, article, paragraph })))
        .collect();
    let mut links = citation_links(graph, snap, paragraphs.iter().map(|(_, paragraph)| paragraph))?;
    Ok(paragraphs
        .into_iter()
        .filter_map(|(id, paragraph)| {
            let text = window::render_layer(snap, id, atlas_graph::concord_adapter::CONCORD_TRANSLATION)?;
            let anchors = anchors_over(&text, CITES, links.remove(&paragraph).unwrap_or_default());
            let ConcordRef { part, article, paragraph } = paragraph;
            Some(wire::TextUnit {
                r#ref: format!("BoC {part}.{article}.{paragraph}"),
                locus: wire::TextRef::Concord { part, article, paragraph },
                text,
                words_of_christ: Vec::new(),
                heading: None,
                anchors,
                edge_summary: unit_edge_summary(snap, id),
            })
        })
        .collect())
}

type Links<U> = BTreeMap<U, Vec<(TokenSpan, wire::NodeRef)>>;

fn mention_links<'a>(graph: &GraphService, snap: &impl GraphQuery, verses: impl Iterator<Item = &'a VerseRef> + Clone) -> Result<Links<VerseRef>, ApiError> {
    let Some(window) = units_spanned(verses) else { return Ok(Links::new()) };
    let spans = graph.mention_spans_in(&window).map_err(|e| unreadable_rows("mentions", &e))?;
    let entities: BTreeSet<AnyNodeId> = spans.values().flatten().map(|span| span.entity.node_id()).collect();
    let named = describe_nodes(&entities, snap)?;
    Ok(spans.into_iter().map(|(verse, spans)| (verse, spans.into_iter().map(|span| (span.words, named[&span.entity.node_id()].clone())).collect())).collect())
}

fn citation_links<'a>(graph: &GraphService, snap: &impl GraphQuery, paragraphs: impl Iterator<Item = &'a ConcordRef> + Clone) -> Result<Links<ConcordRef>, ApiError> {
    let Some(window) = units_spanned(paragraphs) else { return Ok(Links::new()) };
    let spans = graph.citation_spans_in(&window).map_err(|e| unreadable_rows("citations", &e))?;
    let cited: BTreeSet<AnyNodeId> = spans.values().flatten().map(|span| verse_node_id(span.cites.book, span.cites.chapter, span.cites.verse)).collect();
    let named = describe_nodes(&cited, snap)?;
    Ok(spans
        .into_iter()
        .map(|(paragraph, spans)| (paragraph, spans.into_iter().map(|span| (span.words, named[&verse_node_id(span.cites.book, span.cites.chapter, span.cites.verse)].clone())).collect()))
        .collect())
}

fn units_spanned<'a, U: Ord + Clone + 'a>(units: impl Iterator<Item = &'a U> + Clone) -> Option<RangeInclusive<U>> {
    Some(units.clone().min()?.clone()..=units.max()?.clone())
}

fn unreadable_rows(rows: &str, error: &SqliteError) -> ApiError {
    ApiError::internal(&format!("the {rows} of a reading window could not be read: {error}"))
}

fn anchors_over(text: &str, kind: EdgeKind, links: Vec<(TokenSpan, wire::NodeRef)>) -> Vec<wire::Anchor> {
    let words = tokens::tokenize(text);
    let mut anchors: Vec<wire::Anchor> = links
        .into_iter()
        .map(|(span, node)| {
            let chars = tokens::chars_of(&span, &words).unwrap_or_else(|| panic!("the words {}..={} of {} lie past the text of the unit they are stored on", span.start, span.end, node.id));
            wire::Anchor { start: chars.start, end: chars.end, kind, node }
        })
        .collect();
    anchors.sort_by_key(|anchor| anchor.start);
    anchors
}

fn unit_heading(heading: &Heading, snap: &impl GraphQuery) -> Result<wire::UnitHeading, ApiError> {
    Ok(wire::UnitHeading {
        event: node_ref(&event_node_id(&heading.event_id), snap)?,
        kind: heading.kind,
        is_continuation: heading.is_continuation,
    })
}

fn unit_edge_summary(snap: &impl GraphQuery, id: &AnyNodeId) -> Vec<wire::EdgeSummaryEntry> {
    summary_at(snap, &Position::Node(id.clone()))
}

pub fn routes() -> utoipa_axum::router::OpenApiRouter<crate::app::AppState> {
    use utoipa_axum::routes;
    utoipa_axum::router::OpenApiRouter::new()
        .routes(routes!(node_record))
        .routes(routes!(node_edges))
        .routes(routes!(elements))
        .routes(routes!(text_window))
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::graph::Graph;

    const GENESIS: u8 = 0;

    #[test]
    fn a_reading_window_whose_anchor_rows_cannot_be_read_is_refused_as_an_internal_error() {
        // Arrange
        let unreadable = SqliteError("mentions entity_kind 9 is not 0..3".to_string());

        // Act
        let refused = unreadable_rows("mentions", &unreadable);

        // Assert
        assert_eq!(
            (refused.status, refused.code, refused.message),
            (StatusCode::INTERNAL_SERVER_ERROR, crate::error::ErrorCode::Internal, "the mentions of a reading window could not be read: mentions entity_kind 9 is not 0..3".to_string())
        );
    }

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
        let hazor = wire::NodeRef { id: "Place:hazor-1".to_string(), kind: NodeKind::Place, label: "Hazor 1".to_string() };

        // Act
        anchors_over("In the beginning God created the heaven and the earth.", MENTIONS, vec![(past_the_end, hazor)]);
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
