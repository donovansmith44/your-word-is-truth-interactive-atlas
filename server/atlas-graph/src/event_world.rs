//! The event world -- events, attestations, narratives, anchors, chronology -- onto the graph, read from
//! the already-merged `AtlasData` rather than parsed a second time. The graph never stores a curated
//! year: only a placement, plus the resolved placement a law checks back against the source year.

use std::collections::{BTreeSet, HashMap};

use atlas_core::data::{AtlasData, ChronologyAnchor, Event, EventKind};

use atlas_graph_types::chrono::{
    DatePlacement, Duration, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year,
};
use atlas_graph_types::edge::{Attests, Ground, Justification, LocatedAt, Succession};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnchorId, EventId, NarrativeId, PlaceId};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, BibleTag, Locus, VerseRef};

/// One event's chosen placement plus the justification its `DatedBy` row carries, kept alongside the
/// placement rather than folded into the row so the writer and the inspectors share one source.
#[derive(Clone, Debug)]
pub struct PlacedChronology {
    pub placement: DatePlacement,
    pub basis: PlacementBasis,
    pub justification: Justification,
}

/// The event's own genuine curated `to_year`/`order_key`, carried separately from `resolved` on purpose:
/// `resolved.date.to` is always a copy of `.from`, so widening it would add a live ordering tier ahead of
/// `seq`, and the literal curated `order_key` must reconstruct exactly, not merely order equivalently.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceEventMeta {
    pub to_year: i32,
    pub order_key: i32,
}

/// The reconstructed global timeline order -- dated events only -- each dated event's chosen placement,
/// and each one's resolved placement.
#[derive(Clone, Debug, Default)]
pub struct ChronologyDerivation {
    /// Dated event ids in global timeline order: index i is timeline position i.
    pub order: Vec<String>,
    pub placements: HashMap<String, PlacedChronology>,
    pub resolved: HashMap<String, ResolvedPlacement>,
    pub source_meta: HashMap<String, SourceEventMeta>,
}

/// Filters `atlas.events` to dated events and re-sorts by `(from_year, order_key, pre-sort index)`. Since
/// that array is already stably sorted by `from_year`, the pre-sort index is an exact stand-in for
/// original construction order, so this single sort reproduces the source's two-stage stable sort.
fn timeline_order(atlas: &AtlasData) -> Vec<String> {
    let dated: Vec<&Event> = atlas.events.iter().filter(|e| e.kind == EventKind::Event).collect();
    let mut keyed: Vec<(usize, &Event)> = dated.into_iter().enumerate().collect();
    keyed.sort_by_key(|(idx, e)| (e.when.from_year, e.order_key, *idx));
    keyed.into_iter().map(|(_, e)| e.id.clone()).collect()
}

/// The same-narrative immediate predecessor of every non-first leg. Where an event is a leg of more than
/// one narrative the first narrative wins, which only decides which prior a `DatedBy` row names: the
/// sequence key is assigned independently, from the global position.
fn same_narrative_prior(atlas: &AtlasData) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for n in &atlas.narratives {
        for w in n.legs.windows(2) {
            out.entry(w[1].clone()).or_insert_with(|| w[0].clone());
        }
    }
    out
}

/// One placement per dated event, in preference order: an anchor row binding it, its same-narrative
/// predecessor, the previous dated event in the global timeline, then the earliest anchor. `basis` is
/// uniformly traditional -- the data carries no textual-versus-traditional flag to stratify by.
fn choose_placement(
    e: &Event,
    i: usize,
    order: &[String],
    anchor_by_event: &HashMap<&str, &ChronologyAnchor>,
    earliest_anchor: Option<&ChronologyAnchor>,
    same_prior: &HashMap<String, String>,
    atlas: &AtlasData,
) -> PlacedChronology {
    let basis = PlacementBasis::Traditional;

    if let Some(a) = anchor_by_event.get(e.id.as_str()) {
        let offset = Duration::years(e.when.from_year - a.year);
        let anchor = AnchorId::new(a.id.clone());
        let mut grounds = BTreeSet::new();
        grounds.insert(Ground::Anchor(anchor.clone()));
        return PlacedChronology {
            placement: DatePlacement::AnchorBinding { anchor, offset },
            basis,
            justification: Justification { text: a.note.clone(), grounds },
        };
    }

    if let Some(prior_id) = same_prior.get(&e.id) {
        if let Some(prior_event) = atlas.event_by_id(prior_id) {
            let spacing = Duration::years(e.when.from_year - prior_event.when.from_year);
            return PlacedChronology {
                placement: DatePlacement::SequenceAfter { prior: EventId::new(prior_id.clone()), spacing },
                basis,
                justification: Justification::default(),
            };
        }
    }

    if i > 0 {
        let prior_id = &order[i - 1];
        if let Some(prior_event) = atlas.event_by_id(prior_id) {
            let spacing = Duration::years(e.when.from_year - prior_event.when.from_year);
            return PlacedChronology {
                placement: DatePlacement::SequenceAfter { prior: EventId::new(prior_id.clone()), spacing },
                basis,
                justification: Justification::default(),
            };
        }
    }

    if let Some(a) = earliest_anchor {
        let offset = Duration::years(e.when.from_year - a.year);
        return PlacedChronology {
            placement: DatePlacement::AnchorBinding { anchor: AnchorId::new(a.id.clone()), offset },
            basis,
            justification: Justification::default(),
        };
    }

    // The degenerate fallback, with no anchor table at all, reachable only by a minimal fixture. The
    // resolved TimePoint is computed from the event's own year whatever placement is chosen here, so this
    // one's unresolvability never touches ordering: only its "why this date?" explorability degrades.
    PlacedChronology {
        placement: DatePlacement::EraOnly { era: atlas_graph_types::id::EraId::new(format!("undetermined-basis-{}", e.id)) },
        basis,
        justification: Justification::default(),
    }
}

/// A real walk of the stored placement rather than a re-read of the source, and non-recursive by
/// construction: every prior is a dated event with a known year. An era-only placement resolves to
/// `None`, honestly, rather than fabricating a point.
pub fn resolve_timepoint(
    placement: &DatePlacement,
    anchor_years: &HashMap<String, TimePoint>,
    event_years: &HashMap<String, TimePoint>,
) -> Option<TimePoint> {
    let add_years = |tp: TimePoint, delta: i32| -> TimePoint {
        TimePoint::year_only(Year::new(tp.year.get() + delta).expect("resolved year is never 0 (source TimeRange already forbids it)"))
    };
    match placement {
        DatePlacement::AnchorBinding { anchor, offset } => anchor_years.get(&anchor.0).map(|tp| add_years(*tp, offset.years)),
        DatePlacement::ReignYear { reign, year_of_reign } => {
            anchor_years.get(&reign.0).map(|tp| add_years(*tp, *year_of_reign as i32 - 1))
        }
        DatePlacement::SequenceAfter { prior, spacing } => event_years.get(&prior.0).map(|tp| add_years(*tp, spacing.years)),
        DatePlacement::EraOnly { .. } => None,
    }
}

/// `seq` is assigned directly from the reconstructed timeline position, which is exact rather than a
/// shortcut: `date.to` equals `date.from` for every event, so the `to` step of the temporal order is an
/// unconditional no-op whenever `from` ties and `seq` decides on the same total order `from` encodes.
pub fn derive_chronology(atlas: &AtlasData) -> ChronologyDerivation {
    let order = timeline_order(atlas);
    let position: HashMap<&str, usize> = order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();

    let anchor_by_event: HashMap<&str, &ChronologyAnchor> =
        atlas.chronology_anchors.iter().filter_map(|a| a.event_id.as_deref().map(|eid| (eid, a))).collect();
    let earliest_anchor = atlas.chronology_anchors.iter().min_by_key(|a| a.year);
    let same_prior = same_narrative_prior(atlas);

    let mut placements = HashMap::new();
    let mut resolved = HashMap::new();
    let mut source_meta = HashMap::new();

    for (i, id) in order.iter().enumerate() {
        let e = atlas.event_by_id(id).expect("id just collected from atlas.events");
        let placed = choose_placement(e, i, &order, &anchor_by_event, earliest_anchor, &same_prior, atlas);

        let tp = TimePoint::year_only(Year::new(e.when.from_year).expect("dated event year is never 0"));
        let seq = SeqKey(position[id.as_str()] as u32);
        let rp = ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq, basis: placed.basis };

        placements.insert(id.clone(), placed);
        resolved.insert(id.clone(), rp);
        source_meta.insert(id.clone(), SourceEventMeta { to_year: e.when.to_year, order_key: e.order_key });
    }

    ChronologyDerivation { order, placements, resolved, source_meta }
}

pub struct Chronology {
    pub chrono: ChronologyDerivation,
}

impl Chronology {
    pub fn build(atlas: &AtlasData) -> Chronology {
        Self::from_derivation(derive_chronology(atlas))
    }

    /// Wraps an ALREADY-COMPUTED derivation, so the artifact path -- which has no `AtlasData` to re-derive from
    /// -- reconstructs the same value, and the compile step reuses the derivation the RESOLVE stage already
    /// computed instead of running it twice.
    pub fn from_derivation(chrono: ChronologyDerivation) -> Chronology {
        Chronology { chrono }
    }

    /// `AnchorId.0 -> TimePoint`, from the same anchor table `populate` reads.
    pub fn anchor_years(atlas: &AtlasData) -> HashMap<String, TimePoint> {
        atlas
            .chronology_anchors
            .iter()
            .map(|a| (a.id.clone(), TimePoint::year_only(Year::new(a.year).expect("anchor year is never 0"))))
            .collect()
    }

    /// `event id -> TimePoint` for every DATED event: the flat, non-recursive base the sequence arm reads.
    pub fn event_years(atlas: &AtlasData) -> HashMap<String, TimePoint> {
        atlas
            .events
            .iter()
            .filter(|e| e.kind == EventKind::Event)
            .map(|e| (e.id.clone(), TimePoint::year_only(Year::new(e.when.from_year).expect("dated event year is never 0"))))
            .collect()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventWorldStats {
    pub places: usize,
    pub events: usize,
    pub dated_events: usize,
    pub narratives: usize,
    pub succession_rows: usize,
    pub anchors: usize,
    pub attests_rows: usize,
    pub located_at_rows: usize,
    pub dated_by_rows: usize,
    /// `Mentions` rows whose entity is an EVENT: a verse that REFERENCES an event without narrating it.
    /// Counted apart from the accounts deliberately -- that separation is the whole distinction.
    pub event_mentions_rows: usize,
    pub analogue_rows: usize,
}

/// The node-id constructors for this module's kinds: one place where the internal id SHAPE is decided, so
/// no call site hand-builds an `AnyNodeId`.
pub fn event_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    EventId::new(id.to_string()).erase()
}
pub fn narrative_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    NarrativeId::new(id.to_string()).erase()
}
pub fn anchor_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    AnchorId::new(id.to_string()).erase()
}
pub fn place_stub_node_id(id: &str) -> atlas_graph_types::id::AnyNodeId {
    PlaceId::new(id.to_string()).erase()
}

fn place_node(p: &atlas_core::data::Place, atlas: &AtlasData) -> Node {
    // The KJV aliases ride the payload because a `Named` row's object is a bare string with no `Position`
    // to index through the generic port: the payload is the queryable form.
    let aliases = kjv_aliases_of(atlas, &p.id);
    Node {
        id: PlaceId::new(p.id.clone()).erase(),
        payload: NodePayload::Place { canonical: p.name.clone(), lat: p.lat, lon: p.lon, aliases, description: None },
        provenance: "curated-places".to_string(),
    }
}

/// The names a place's KJV aliases give it, in alias order.
pub fn kjv_aliases_of(atlas: &AtlasData, place: &str) -> Vec<String> {
    atlas.place_name_aliases_for(place).iter().filter_map(|a| a.translations.get(crate::kjv_adapter::KJV_TRANSLATION).cloned()).collect()
}

fn event_provenance(id: &str) -> &'static str {
    if id.starts_with("theo-") {
        "theographic"
    } else {
        "curated"
    }
}

/// Every field a full event fetch needs, plus the container's own top-level `verses`. `places` deliberately
/// does NOT ride here -- the `located_at` rows are the single, order-preserving source -- and neither does
/// chronology, which lives solely in the `dated_by` placements.
fn event_node(e: &Event) -> Node {
    let witnesses: Vec<atlas_graph_types::node::EventWitnessPayload> = e
        .witnesses
        .iter()
        .map(|w| atlas_graph_types::node::EventWitnessPayload {
            book: w.book.clone(),
            translations: w.translations.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            ref_note: w.ref_note.clone(),
            robertson_section: w.robertson_section.clone(),
        })
        .collect();
    Node {
        id: EventId::new(e.id.clone()).erase(),
        payload: NodePayload::Event {
            label: e.label.clone(),
            kind: e.kind.name().to_string(),
            verses: e.verses.clone(),
            witnesses,
            robertson_section: e.robertson_section.clone(),
            acts_section: e.acts_section.clone(),
            atlas_section: e.atlas_section.clone(),
            kjv_superscription: e.kjv_superscription.clone(),
            ref_note: e.ref_note.clone(),
        },
        provenance: event_provenance(&e.id).to_string(),
    }
}

fn narrative_node(n: &atlas_core::data::Narrative) -> Node {
    Node {
        id: NarrativeId::new(n.id.clone()).erase(),
        payload: NodePayload::Narrative { label: n.name.clone(), color: n.color.clone() },
        provenance: "curated-narratives".to_string(),
    }
}

/// The Anchor's card label: `card()` uses this string VERBATIM, so opening an anchor's card IS reading its
/// citation.
fn format_anchor_citation(a: &ChronologyAnchor) -> String {
    let year = if a.year < 0 { format!("{} BC", -a.year) } else { format!("AD {}", a.year) };
    match &a.note {
        Some(note) => format!("{} — {year}. Source: {}. {note}", a.label, a.source),
        None => format!("{} — {year}. Source: {}.", a.label, a.source),
    }
}

fn anchor_node(a: &ChronologyAnchor) -> Node {
    let at = TimePoint::year_only(Year::new(a.year).expect("chronology anchor year is never 0"));
    Node { id: AnchorId::new(a.id.clone()).erase(), payload: NodePayload::Anchor { at, citation: format_anchor_citation(a) }, provenance: "chronology-anchors".to_string() }
}

/// One `attests` row per INDIVIDUAL VERSE, never per contiguous verse group: index building lowers one edge
/// per row, so a group-spanning row would leave a verse cited only in the interior of a group unable to
/// resolve back to its event. Each row's attestation is therefore a single-verse range.
fn verse_to_range(v: &str) -> Option<BibleLocusRange> {
    let vid = atlas_core::refs::VerseId::parse_canonical(v).ok()?;
    let locus: BibleLocus = Locus::<BibleTag>::whole(VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse });
    BibleLocusRange::new(locus.clone(), locus).ok()
}

/// Populates the event-world nodes and the `attests`/`succession`/`dated_by`/`located_at` rows. The caller
/// runs `build_indexes()` once, after this and the KJV/xrefs population.
pub fn populate(graph: &mut Graph, atlas: &AtlasData, chrono: &ChronologyDerivation) -> EventWorldStats {
    let mut stats = populate_nodes_and_direct_rows(graph, atlas);
    stats.dated_by_rows = populate_dated_by(graph, chrono);
    stats
}

/// The NORMALIZE-shaped half: nodes plus the directly-authored rows, none of which need any other pass's
/// output -- in particular none read the chronology.
pub fn populate_nodes_and_direct_rows(graph: &mut Graph, atlas: &AtlasData) -> EventWorldStats {
    let mut stats = EventWorldStats::default();

    for p in &atlas.places {
        let node = place_node(p, atlas);
        graph.nodes.insert(node.id.clone(), node);
        stats.places += 1;
    }

    for e in &atlas.events {
        let node = event_node(e);
        graph.nodes.insert(node.id.clone(), node);
        stats.events += 1;
        if e.kind == EventKind::Event {
            stats.dated_events += 1;
        }

        let event_id = EventId::new(e.id.clone());

        for a in atlas_core::scene::accounts_of(e) {
            let text = match (&a.ref_note, &a.robertson_section) {
                (Some(r), Some(rs)) => Some(format!("{r}; {rs}")),
                (Some(r), None) => Some(r.clone()),
                (None, Some(rs)) => Some(rs.clone()),
                (None, None) => None,
            };
            for v in &a.verses {
                let Some(range) = verse_to_range(v) else { continue };
                graph.attests.push(Attests {
                    event: event_id.clone(),
                    attestation: range,
                    provenance: "event-witnesses".to_string(),
                    justification: Justification { text: text.clone(), grounds: BTreeSet::new() },
                });
                stats.attests_rows += 1;
            }
        }

        for pid in &e.places {
            graph.located_at.push(LocatedAt {
                event: event_id.clone(),
                place: PlaceId::new(pid.clone()),
                provenance: event_provenance(&e.id).to_string(),
                justification: Justification::default(),
            });
            stats.located_at_rows += 1;
        }
    }

    for n in &atlas.narratives {
        let node = narrative_node(n);
        graph.nodes.insert(node.id.clone(), node);
        stats.narratives += 1;

        if n.legs.is_empty() {
            continue;
        }
        let chain: Vec<EventId> = n.legs.iter().map(|l| EventId::new(l.clone())).collect();
        if let Ok(row) = Succession::new(NarrativeId::new(n.id.clone()), chain, "curated-narratives".to_string(), Justification::default()) {
            graph.succession.push(row);
            stats.succession_rows += 1;
        }
    }

    for a in &atlas.chronology_anchors {
        let node = anchor_node(a);
        graph.nodes.insert(node.id.clone(), node);
        stats.anchors += 1;
    }

    // The retyped rows: the compile has already stripped each named verse out of its event's own verse and
    // witness lists, so the accounts loop above never saw it, and the fact lands here instead as a mention
    // pointing at the EVENT. The curator's ruling rides along as the row's provenance trail.
    for m in &atlas.event_mentions {
        let event_id = EventId::new(m.event_id.clone());
        for v in &m.verses {
            let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) else { continue };
            graph.mentions.push(atlas_graph_types::edge::Mentions {
                locus: atlas_graph_types::text::TextLocus {
                    at: atlas_graph_types::text::TextRef::Bible(VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse }),
                    span: None,
                },
                entity: atlas_graph_types::edge::MentionedEntity::Event(event_id.clone()),
                provenance: "attestation-corrections".to_string(),
            });
            stats.event_mentions_rows += 1;
        }
    }

    // Distinct events whose accounts are similar in form or content, NEVER two accounts of one event.
    // Authored only: no similarity metric mints these.
    for a in &atlas.event_analogues {
        graph.analogue.push(atlas_graph_types::edge::Analogue {
            a: EventId::new(a.a.clone()),
            b: EventId::new(a.b.clone()),
            provenance: "attestation-corrections".to_string(),
        });
        stats.analogue_rows += 1;
    }

    stats
}

/// Iterates `chrono.order`, a plain `Vec`, and never the placements map: `HashMap` iteration order is
/// randomized per instance, and two runs over identical input really did produce different row orders,
/// which the admission check caught as a divergence in the inverse edge order. Row order is content.
pub fn populate_dated_by(graph: &mut Graph, chrono: &ChronologyDerivation) -> usize {
    let mut dated_by_rows = 0;
    for id in &chrono.order {
        let Some(placed) = chrono.placements.get(id) else { continue };
        graph.dated_by.push(atlas_graph_types::chrono::DatedBy {
            event: EventId::new(id.clone()),
            placement: placed.placement.clone(),
            basis: placed.basis,
            justification: placed.justification.clone(),
            provenance: "chronology-derivation".to_string(),
        });
        dated_by_rows += 1;
    }
    dated_by_rows
}

/// One `TemporalAdjacency` row per consecutive pair of the global order, each naming its own earlier and
/// later end straight from that order, so a consumer never re-derives direction from a position index. An
/// atlas with fewer than two dated events yields no rows, honestly rather than as a special case.
pub fn populate_temporal_adjacency(graph: &mut Graph, chrono: &ChronologyDerivation) -> usize {
    let mut rows = 0;
    for w in chrono.order.windows(2) {
        graph.temporal_adjacency.push(atlas_graph_types::edge::TemporalAdjacency {
            earlier: EventId::new(w[0].clone()),
            later: EventId::new(w[1].clone()),
            provenance: "chronology-derivation".to_string(),
        });
        rows += 1;
    }
    rows
}

pub fn normalize(ctx: &mut crate::pipeline::BuildCtx) {
    ctx.event_world_stats = populate_nodes_and_direct_rows(&mut ctx.graph, ctx.atlas);
}

pub fn resolve(ctx: &mut crate::pipeline::BuildCtx) {
    ctx.chrono = derive_chronology(ctx.atlas);
    ctx.event_world_stats.dated_by_rows = populate_dated_by(&mut ctx.graph, &ctx.chrono);
}

pub fn derive(ctx: &mut crate::pipeline::BuildCtx) {
    populate_temporal_adjacency(&mut ctx.graph, &ctx.chrono);
}

/// The SAME node-id shape graph-types derives from a Bible `TextRef`. Needed here because that helper is
/// private, and a `justified-by` edge must land on the identical TextUnit node every other path reaches.
fn bible_locus_node_id(v: &VerseRef) -> atlas_graph_types::id::AnyNodeId {
    atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::TextUnit, raw: format!("bible/{}.{}.{}", v.book, v.chapter, v.verse) }
}

/// Wires `RelationId::JustifiedBy` for every row carrying non-empty grounds, taking each row's `EdgeId` from
/// the graph's own row-to-edge lowering so a ground fans out from the same id the index was built with. A
/// post-processing step: it must run AFTER `build_indexes`, or the two would not agree on those ids.
pub fn add_justified_by(graph: &mut Graph) -> usize {
    use atlas_graph_types::canon::RowFamily;
    use atlas_graph_types::edge::{at, BiIndex, EdgeId, RelationId};
    use atlas_graph_types::explore::EdgeMeta;
    use atlas_graph_types::id::Position;

    // A `justified-by` entry carries no per-entry metadata of its own, so every triple supplies
    // `EdgeMeta::None` honestly.
    let mut pairs: Vec<(Position, Position, EdgeMeta)> = Vec::new();

    // One ground -> one (edge, target) pair.
    fn push_grounds(pairs: &mut Vec<(Position, Position, EdgeMeta)>, edge_id: EdgeId, grounds: &BTreeSet<Ground>) {
        for ground in grounds {
            let target = match ground {
                Ground::Scripture(range) => Position::Node(bible_locus_node_id(&range.from.unit)),
                Ground::Anchor(a) => at(&a.erase()),
                Ground::Source(s) => at(&s.erase()),
            };
            pairs.push((Position::Edge(edge_id.clone()), target, EdgeMeta::None));
        }
    }

    // ONE pass over the graph's own row-to-edge lowering, so the edge id a ground fans out from is minted by
    // the same function that placed it.
    for e in graph.row_edges() {
        let grounds = match e.family {
            RowFamily::DatedBy => &graph.dated_by[e.row_ord].justification.grounds,
            RowFamily::Fulfills => &graph.fulfills[e.row_ord].justification.grounds,
            RowFamily::Typology => &graph.typology[e.row_ord].justification.grounds,
            RowFamily::NamedAfter => &graph.named_after[e.row_ord].justification.grounds,
            _ => continue,
        };
        if grounds.is_empty() {
            continue;
        }
        let edge_id = Graph::edge_id_of(&e);
        push_grounds(&mut pairs, edge_id, grounds);
    }

    let count = pairs.len();
    if count > 0 {
        graph.indexes.insert(RelationId::JustifiedBy, BiIndex::build(RelationId::JustifiedBy, &pairs));
    }
    count
}

/// A trivial, event-world-empty `AtlasData` fixture. Deliberately NOT behind `#[cfg(test)]`: the integration
/// test files link against the normal build of this library and need it too.
pub fn empty_atlas() -> AtlasData {
    use atlas_core::data::Canon;
    use std::collections::HashMap;
    AtlasData::new(Canon { books: vec![] }, vec![], vec![], vec![], vec![], vec![], HashMap::new(), HashMap::new()).finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::data::{Canon, Narrative, Place};
    use std::collections::HashMap;

    fn anchor(id: &str, year: i32, event_id: Option<&str>) -> ChronologyAnchor {
        ChronologyAnchor { id: id.into(), label: id.into(), year, event_id: event_id.map(String::from), era_boundary: false, source: "test".into(), note: None }
    }

    fn event(id: &str, from_year: i32, order_key: i32) -> Event {
        Event { id: id.into(), label: id.into(), when: atlas_core::time::TimeRange::new(from_year, from_year).unwrap(), order_key, ..Default::default() }
    }

    fn atlas_with(events: Vec<Event>, narratives: Vec<Narrative>, anchors: Vec<ChronologyAnchor>) -> AtlasData {
        let places = vec![Place { id: "p1".into(), name: "P1".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let mut d = AtlasData::new(Canon { books: vec![] }, places, events, narratives, vec![], vec![], HashMap::new(), HashMap::new()).finish();
        d.chronology_anchors = anchors;
        d
    }

    #[test]
    fn timeline_order_matches_atlas_datas_own_sort_key() {
        let atlas = atlas_with(
            vec![event("b", -500, 0), event("a", -500, 0), event("c", -100, 0)],
            vec![],
            vec![],
        );
        let order = timeline_order(&atlas);
        assert_eq!(order, vec!["b".to_string(), "a".to_string(), "c".to_string()]);
    }

    #[test]
    fn order_key_breaks_same_year_ties() {
        let atlas = atlas_with(vec![event("later", 33, 5), event("earlier", 33, 1)], vec![], vec![]);
        let order = timeline_order(&atlas);
        assert_eq!(order, vec!["earlier".to_string(), "later".to_string()]);
    }

    #[test]
    fn anchor_bound_event_gets_anchor_binding_with_zero_offset() {
        let atlas = atlas_with(vec![event("solomon", -1015, 0)], vec![], vec![anchor("solomon-crowned", -1015, Some("solomon"))]);
        let chrono = derive_chronology(&atlas);
        let placed = &chrono.placements["solomon"];
        match &placed.placement {
            DatePlacement::AnchorBinding { anchor, offset } => {
                assert_eq!(anchor.0, "solomon-crowned");
                assert_eq!(offset.years, 0);
            }
            other => panic!("expected AnchorBinding, got {other:?}"),
        }
    }

    #[test]
    fn deferred_style_anchor_binding_carries_the_honest_nonzero_offset() {
        let atlas = atlas_with(vec![event("exl", -586, 0)], vec![], vec![anchor("jerusalem-falls", -588, Some("exl"))]);
        let chrono = derive_chronology(&atlas);
        let placed = &chrono.placements["exl"];
        match &placed.placement {
            DatePlacement::AnchorBinding { offset, .. } => assert_eq!(offset.years, 2),
            other => panic!("expected AnchorBinding, got {other:?}"),
        }
        let anchor_years = HashMap::from([("jerusalem-falls".to_string(), TimePoint::year_only(Year::new(-588).unwrap()))]);
        let resolved = resolve_timepoint(&placed.placement, &anchor_years, &HashMap::new()).unwrap();
        assert_eq!(resolved.year.get(), -586, "resolution must land on the event's own true year despite the deferred table/event gap");
    }

    #[test]
    fn non_anchor_event_chains_off_its_same_narrative_prior() {
        let atlas = atlas_with(
            vec![event("leg1", -1406, 0), event("leg2", -1405, 0)],
            vec![Narrative { id: "conquest".into(), name: "Conquest".into(), color: "#000".into(), legs: vec!["leg1".into(), "leg2".into()] }],
            vec![anchor("some-anchor", -2000, None)],
        );
        let chrono = derive_chronology(&atlas);
        match &chrono.placements["leg2"].placement {
            DatePlacement::SequenceAfter { prior, spacing } => {
                assert_eq!(prior.0, "leg1");
                assert_eq!(spacing.years, 1);
            }
            other => panic!("expected SequenceAfter, got {other:?}"),
        }
    }

    #[test]
    fn resolver_reproduces_the_source_year_for_every_placement_form() {
        let atlas = atlas_with(
            vec![event("anchored", -1015, 0), event("chained", -1013, 0)],
            vec![Narrative { id: "n".into(), name: "N".into(), color: "#000".into(), legs: vec!["anchored".into(), "chained".into()] }],
            vec![anchor("solomon-crowned", -1015, Some("anchored"))],
        );
        let chrono = derive_chronology(&atlas);
        let anchor_years = Chronology::anchor_years(&atlas);
        let event_years = Chronology::event_years(&atlas);
        for (id, placed) in &chrono.placements {
            let e = atlas.event_by_id(id).unwrap();
            let resolved = resolve_timepoint(&placed.placement, &anchor_years, &event_years)
                .unwrap_or_else(|| panic!("{id} must resolve"));
            assert_eq!(resolved.year.get(), e.when.from_year, "{id}'s resolved placement must equal its own source year");
        }
    }

    #[test]
    fn temporal_adjacency_rows_are_one_per_consecutive_timeline_pair_earlier_to_later() {
        let atlas = atlas_with(
            vec![event("e5", -2000, 0), event("e1", -1406, 0), event("e2", -1406, 0)],
            vec![],
            vec![],
        );
        let chrono = derive_chronology(&atlas);
        assert_eq!(chrono.order, vec!["e5", "e1", "e2"], "fixture sanity: this is the exact global-timeline order populate_temporal_adjacency must pair");

        let mut graph = Graph::default();
        let rows = populate_temporal_adjacency(&mut graph, &chrono);

        assert_eq!(rows, 2, "3 dated events -> windows(2) -> 2 consecutive pairs");
        assert_eq!(graph.temporal_adjacency.len(), 2, "the returned count must match what actually landed on the graph");
        assert_eq!((graph.temporal_adjacency[0].earlier.0.as_str(), graph.temporal_adjacency[0].later.0.as_str()), ("e5", "e1"));
        assert_eq!((graph.temporal_adjacency[1].earlier.0.as_str(), graph.temporal_adjacency[1].later.0.as_str()), ("e1", "e2"));
        assert!(graph.temporal_adjacency.iter().all(|r| r.provenance == "chronology-derivation"), "same derivation pass as populate_dated_by's own rows");
    }

    #[test]
    fn zero_or_one_dated_events_yield_zero_temporal_adjacency_rows() {
        let none = atlas_with(vec![], vec![], vec![]);
        let mut g = Graph::default();
        assert_eq!(populate_temporal_adjacency(&mut g, &derive_chronology(&none)), 0, "windows(2) on an empty order is honestly empty, not a panic");

        let one = atlas_with(vec![event("e1", -1406, 0)], vec![], vec![]);
        let mut g2 = Graph::default();
        assert_eq!(populate_temporal_adjacency(&mut g2, &derive_chronology(&one)), 0, "a single dated event has no consecutive PAIR to relate");
    }

    #[test]
    fn general_kind_events_are_excluded_from_the_timeline_and_get_no_dated_by_row() {
        let mut general = event("g1", -4004, 0);
        general.kind = EventKind::General;
        general.when = atlas_core::time::TimeRange::undated();
        let atlas = atlas_with(vec![event("e1", -1000, 0), general], vec![], vec![anchor("a", -1000, Some("e1"))]);
        let chrono = derive_chronology(&atlas);
        assert!(!chrono.order.contains(&"g1".to_string()));
        assert!(!chrono.placements.contains_key("g1"));
        assert_eq!(chrono.order, vec!["e1".to_string()]);
    }

    #[test]
    fn populate_builds_the_expected_node_and_edge_counts() {
        let atlas = atlas_with(
            vec![event("e1", -1406, 0), event("e2", -1406, 0)],
            vec![Narrative { id: "conquest".into(), name: "Conquest".into(), color: "#000".into(), legs: vec!["e1".into(), "e2".into()] }],
            vec![anchor("a", -1406, Some("e1"))],
        );
        let chrono = derive_chronology(&atlas);
        let mut graph = Graph::default();
        let stats = populate(&mut graph, &atlas, &chrono);
        assert_eq!(stats.events, 2);
        assert_eq!(stats.dated_events, 2);
        assert_eq!(stats.narratives, 1);
        assert_eq!(stats.succession_rows, 1);
        assert_eq!(stats.anchors, 1);
        assert_eq!(stats.dated_by_rows, 2);
        assert_eq!(stats.places, 1);
        assert_eq!(graph.succession.len(), 1);
        assert_eq!(graph.dated_by.len(), 2);
    }

    #[test]
    fn justified_by_wires_an_anchor_bound_dated_by_rows_own_ground() {
        let atlas = atlas_with(vec![event("e1", -1015, 0)], vec![], vec![anchor("a", -1015, Some("e1"))]);
        let chrono = derive_chronology(&atlas);
        let mut graph = Graph::default();
        populate(&mut graph, &atlas, &chrono);
        graph.build_indexes();
        let n = add_justified_by(&mut graph);
        assert_eq!(n, 1, "the one anchor-bound event's own DatedBy row carries exactly one ground (its anchor)");

        use atlas_graph_types::edge::{entry_id, at, RelationId, Direction};
        use atlas_graph_types::explore::{Explorable, PositionRef};
        use atlas_graph_types::id::Position;

        let dated_by_edge_id = entry_id(RelationId::DatedBy, &at(&EventId::new("e1").erase()), &at(&AnchorId::new("a").erase()));
        let page = PositionRef(Position::Edge(dated_by_edge_id.clone())).edges(
            &graph,
            &atlas_graph_types::explore::EdgeQuery { kind: atlas_graph_types::edge::EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: None, limit: 10 },
        );
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].node, Position::Node(AnchorId::new("a").erase()), "the dated-by edge's own justified-by target must be the anchor it names as ground");
    }

    #[test]
    fn justified_by_wires_nothing_when_no_row_carries_grounds() {
        let atlas = atlas_with(
            vec![event("leg1", -1406, 0), event("leg2", -1405, 0)],
            vec![Narrative { id: "n".into(), name: "N".into(), color: "#000".into(), legs: vec!["leg1".into(), "leg2".into()] }],
            vec![],
        );
        let chrono = derive_chronology(&atlas);
        let mut graph = Graph::default();
        populate(&mut graph, &atlas, &chrono);
        graph.build_indexes();
        let n = add_justified_by(&mut graph);
        assert_eq!(n, 0);
        assert!(!graph.indexes.contains_key(&atlas_graph_types::edge::RelationId::JustifiedBy));
    }

    #[test]
    fn justified_by_wires_fulfills_typology_and_named_after_rows_own_grounds() {
        use atlas_graph_types::edge::{at, entry_id, Direction, EdgeKind, Fulfills, Namesake, NamedAfter, RelationId, Typology};
        use atlas_graph_types::explore::{EdgeQuery, Explorable, PositionRef};
        use atlas_graph_types::id::{PeopleGroupId, Position};
        use std::collections::BTreeSet;

        fn locus(book: u8, chapter: u16, verse: u16) -> BibleLocus {
            BibleLocus::whole(VerseRef { book, chapter, verse })
        }
        fn range(from: BibleLocus, to: BibleLocus) -> BibleLocusRange {
            BibleLocusRange::new(from, to).unwrap()
        }

        let mut graph = Graph::default();

        let prophecy = range(locus(0, 7, 14), locus(0, 7, 14));
        let fulfillment = range(locus(1, 1, 22), locus(1, 1, 23));
        graph.fulfills.push(Fulfills {
            prophecy: prophecy.clone(),
            fulfillment: fulfillment.clone(),
            provenance: "test".into(),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Scripture(fulfillment.clone())]) },
        });

        let type_passage = range(locus(0, 14, 18), locus(0, 14, 20));
        let antitype_passage = range(locus(2, 7, 1), locus(2, 7, 17));
        graph.typology.push(Typology {
            type_passage: type_passage.clone(),
            antitype_passage: antitype_passage.clone(),
            note: Some("test figure".into()),
            provenance: "test".into(),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Scripture(antitype_passage.clone())]) },
        });

        let named_after_ground = range(locus(0, 29, 32), locus(0, 29, 32));
        graph.named_after.push(NamedAfter {
            namesake: Namesake::PeopleGroup(PeopleGroupId::new("some-group")),
            eponym: atlas_graph_types::id::PersonId::new("some-person"),
            provenance: "test".into(),
            justification: Justification { text: None, grounds: BTreeSet::from([Ground::Scripture(named_after_ground.clone())]) },
        });

        graph.build_indexes();
        let n = add_justified_by(&mut graph);
        assert_eq!(n, 3, "one ground each, across the three new tables");

        let fulfills_edge_id = entry_id(RelationId::Fulfillment, &Position::Node(bible_locus_node_id(&prophecy.from.unit)), &Position::Node(bible_locus_node_id(&fulfillment.from.unit)));
        let page = PositionRef(Position::Edge(fulfills_edge_id)).edges(&graph, &EdgeQuery { kind: EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: None, limit: 10 });
        assert_eq!(page.entries.len(), 1);
        assert_eq!(page.entries[0].node, Position::Node(bible_locus_node_id(&fulfillment.from.unit)), "the fulfills row's own justified-by target must be the fulfillment passage it self-attests as ground");

        let typology_edge_id = entry_id(RelationId::Typology, &Position::Node(bible_locus_node_id(&type_passage.from.unit)), &Position::Node(bible_locus_node_id(&antitype_passage.from.unit)));
        let page2 = PositionRef(Position::Edge(typology_edge_id)).edges(&graph, &EdgeQuery { kind: EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: None, limit: 10 });
        assert_eq!(page2.entries.len(), 1);
        assert_eq!(page2.entries[0].node, Position::Node(bible_locus_node_id(&antitype_passage.from.unit)), "the typology row's own justified-by target must be the antitype passage it self-attests as ground");

        let named_after_edge_id = entry_id(RelationId::NamedAfter, &at(&PeopleGroupId::new("some-group").erase()), &at(&atlas_graph_types::id::PersonId::new("some-person").erase()));
        let page3 = PositionRef(Position::Edge(named_after_edge_id)).edges(&graph, &EdgeQuery { kind: EdgeKind::Directed(RelationId::JustifiedBy, Direction::Forward), cursor: None, limit: 10 });
        assert_eq!(page3.entries.len(), 1);
        assert_eq!(page3.entries[0].node, Position::Node(bible_locus_node_id(&named_after_ground.from.unit)));
    }
}
