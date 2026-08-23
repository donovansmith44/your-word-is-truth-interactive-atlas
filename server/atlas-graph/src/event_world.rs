//! Batch M-B: the event world (events, attestations, narratives, anchors,
//! chronology) onto the graph -- extends the M-A adapter pattern
//! (`kjv_adapter`/`xref_adapter`/`build`) with a curated-data adapter.
//!
//! SOURCE (controller decision 1, disclosed deviation, see batch-mb-report.md
//! for the full rationale): unlike `kjv_adapter` (which re-parses raw
//! `kjv.json` bytes independently for the fidelity law's own sake), this
//! adapter reads the ALREADY-BUILT `atlas_core::data::AtlasData` -- the same
//! `Vec<Event>`/`Vec<Narrative>`/`Vec<ChronologyAnchor>`/`Vec<Place>` every
//! pre-existing handler already serves from, itself already the product of
//! `atlas_etl`'s own curated-TOML parse + Theographic import + event_merge +
//! nt_calibration + theo_date_overrides + place merges (all ETL-side,
//! already baked into `data/compiled/*.json` by the time this crate reads
//! it). Re-deriving that whole pipeline a second time, independently, here,
//! would duplicate hundreds of lines of already-tested merge/calibration
//! logic for no fidelity-law-shaped benefit (unlike KJV text, there is no
//! byte-for-byte external ground truth to independently re-check curated
//! event dates against) and would be a genuine drift risk. "Legacy
//! vocabulary maps at the boundary" (controller decision 1) is satisfied
//! directly: `Event.witnesses` (`data/curated/event-witnesses.toml`'s own
//! `[[witness]]` rows, already parsed) becomes `Attests` rows one-for-one;
//! nothing in `data/curated/*.toml` is re-authored or touched.
//!
//! CHRONOLOGY (controller decision 2): `derive_chronology` computes each
//! dated event's `DatePlacement` (graph-types::chrono) from the anchor
//! table plus (per the disclosed deviation) each event's own already-
//! resolved `when.from_year` -- the GRAPH never stores that literal, only
//! the placement plus a `ResolvedPlacement` this adapter's own resolver
//! computes FROM the placement (see `resolve_timepoint`), which is
//! independently checked to reproduce the same year (`chronology_laws.rs`'s
//! `resolver_reproduces_the_source_year_for_every_placement_form` test).
//! SeqKey is assigned directly from the reconstructed global timeline
//! position (see that function's own doc comment for why this is exact,
//! not approximate) -- the acceptance centerpiece
//! (`tests/timeline_equivalence.rs`) proves this position sequence is
//! IDENTICAL to `atlas_core::data::AtlasData`'s own `timeline_order`.

use std::collections::{BTreeSet, HashMap};

use atlas_core::data::{AtlasData, ChronologyAnchor, Event};

use atlas_graph_types::chrono::{
    DatePlacement, Duration, PlacementBasis, ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year,
};
use atlas_graph_types::edge::{Attests, Ground, Justification, LocatedAt, Succession};
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::{AnchorId, EventId, NarrativeId, PlaceId};
use atlas_graph_types::node::{Node, NodePayload};
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, BibleTag, Locus, VerseRef};

/// One event's chosen date placement, plus the justification carried on its
/// `DatedBy` row -- kept alongside `DatePlacement`/`PlacementBasis` (rather
/// than folded into the graph row directly) so both `populate` (which
/// writes the row) and `Chronology`/tests (which inspect the choice without
/// a full `Graph` scan) share one source.
#[derive(Clone, Debug)]
pub struct PlacedChronology {
    pub placement: DatePlacement,
    pub basis: PlacementBasis,
    pub justification: Justification,
}

/// The full chronology derivation: the reconstructed global timeline order
/// (dated events only, exactly `AtlasData`'s own `timeline_order` --
/// verified, not assumed, by `tests/timeline_equivalence.rs`), each dated
/// event's chosen `DatePlacement`, and each one's resolved placement
/// (`graph_types::chrono::ResolvedPlacement`, the type `temporal_order`
/// compares).
#[derive(Clone, Debug, Default)]
pub struct ChronologyDerivation {
    /// Dated (`kind == "event"`) event ids, in global timeline order --
    /// index i is timeline position i.
    pub order: Vec<String>,
    pub placements: HashMap<String, PlacedChronology>,
    pub resolved: HashMap<String, ResolvedPlacement>,
}

/// Mirrors `atlas_core::data::AtlasData::finish`'s own `timeline_order`
/// construction EXACTLY (`server/atlas-core/src/data.rs`, the
/// `self.events.iter().filter(kind=="event")` + `sort_by_key((from_year,
/// order_key))` pass): `atlas.events` (already `.finish()`-ed, i.e. already
/// stably sorted by `from_year` alone) is filtered to dated events and
/// re-sorted by `(from_year, order_key, pre_sort_index)`, where
/// `pre_sort_index` is this filtered slice's OWN pre-this-sort position.
/// Because `atlas.events` is already `from_year`-stable-sorted, two events
/// sharing `from_year` already sit in their true original construction
/// order in that array -- so `pre_sort_index` is an exact, not
/// approximate, stand-in for "original array order," and this single sort
/// reproduces `AtlasData`'s own TWO-STAGE stable sort bit-for-bit. Proven,
/// not just argued, by `tests/timeline_equivalence.rs`.
fn timeline_order(atlas: &AtlasData) -> Vec<String> {
    let dated: Vec<&Event> = atlas.events.iter().filter(|e| e.kind == "event").collect();
    let mut keyed: Vec<(usize, &Event)> = dated.into_iter().enumerate().collect();
    keyed.sort_by_key(|(idx, e)| (e.when.from_year, e.order_key, *idx));
    keyed.into_iter().map(|(_, e)| e.id.clone()).collect()
}

/// Same-narrative immediate predecessor for every non-first leg, across
/// every narrative -- `entry().or_insert()` keeps the FIRST narrative (in
/// `atlas.narratives` order) that gives an event a same-narrative prior, a
/// disclosed, deliberate heuristic (an event leg of >1 narrative gets its
/// `DatePlacement`'s own `SequenceAfter` provenance from just one of them;
/// this never affects ORDERING, only which prior a `DatedBy` row names --
/// SeqKey is assigned independently, from the global position, below).
fn same_narrative_prior(atlas: &AtlasData) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for n in &atlas.narratives {
        for w in n.legs.windows(2) {
            out.entry(w[1].clone()).or_insert_with(|| w[0].clone());
        }
    }
    out
}

/// Chooses one `DatePlacement` per dated event (controller decision 2:
/// "AnchorBinding where the event binds an anchor row; SequenceAfter/offset
/// shapes where the parked branch's date_resolve design fits" -- SALVAGED
/// from `parked/hotfix7-single-feed`'s `server/atlas-core/src/
/// date_resolve.rs`: its own form classification (anchor-binding vs
/// anchor-relative vs sequence vs era) is reused as the PREFERENCE ORDER
/// below; its own recursive/memoized/cycle-safe resolution walk is NOT
/// reused, because that module resolved YEARS from placements (the
/// authoring-migration direction, out of scope this batch -- disclosed
/// deviation, curated year literals stay adapter input), whereas this
/// function resolves PLACEMENTS from already-known years (the reverse
/// direction), which needs no recursion: `SequenceAfter`'s own `prior` is
/// always a single already-dated event read directly from `atlas`, never a
/// multi-hop chain this function itself must walk):
///   1. ANCHOR BINDING: `chronology-anchors.toml` binds this exact event
///      (`ChronologyAnchor.event_id == Some(event.id)`) -- offset computed
///      as the honest difference (0 for a clean E1-equal row; the real gap
///      for an `ANCHOR_DEFERRALS`-class row, so the resolved date still
///      equals the event's own true year exactly).
///   2. SEQUENCE, same-narrative: this event is a non-first leg of some
///      narrative -- `prior` is its own immediate predecessor IN THAT
///      NARRATIVE (`same_narrative_prior`), the "chain interiors" case
///      `graph_types::chrono::DatePlacement::SequenceAfter`'s own doc
///      comment names.
///   3. SEQUENCE, global: neither of the above -- `prior` is the
///      immediately-preceding DATED event in the reconstructed global
///      timeline (`order[i-1]`).
///   4. Fallback (the true first timeline event, only if ALSO not
///      anchor-bound -- rare; every real `theo-1`-class earliest event is
///      anchor-bound in practice, but this stays total rather than
///      partial): anchor-relative against the table's own earliest-year
///      anchor.
/// `basis` is `Traditional` uniformly (disclosed simplification: the
/// existing per-event data carries no textual-vs-traditional flag to
/// stratify by; this atlas already describes its whole scale as
/// "confessionally acceptable traditional chronology" -- see
/// batch-mb-report.md).
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

    // Degenerate fallback: no anchor table at all (never true of the real
    // committed `chronology-anchors.toml`, which carries 20+ rows -- only
    // reachable by a minimal test fixture, e.g. `AtlasData::demo_fixture()`,
    // whose own doc comment discloses `chronology_anchors` as deliberately
    // left empty). `ChronologyDerivation::resolved`'s own TimePoint is
    // ALWAYS computed directly from `e.when.from_year` regardless of which
    // `DatePlacement` form is chosen here (see `derive_chronology`'s own
    // doc comment) -- so this placement's own unresolvability (`EraOnly`
    // has no real Era node to resolve against, honestly, since Era nodes
    // are M-C's own scope) never touches ordering/SeqKey correctness; it
    // only means this ONE event's `DatedBy` row's own "why this date?"
    // explorability degrades to "undetermined basis" rather than a real
    // cross-reference, disclosed exactly this way in batch-mb-report.md.
    PlacedChronology {
        placement: DatePlacement::EraOnly { era: atlas_graph_types::id::EraId::new(format!("undetermined-basis-{}", e.id)) },
        basis,
        justification: Justification::default(),
    }
}

/// Resolves one `DatePlacement` to the `TimePoint` it names -- a REAL walk
/// of the stored placement (not a re-read of `AtlasData`), non-recursive by
/// construction (see `choose_placement`'s own doc comment for why no
/// recursion is needed): `AnchorBinding`/`ReignYear` read the anchor's own
/// TimePoint directly; `SequenceAfter` reads its `prior`'s year from
/// `event_years` (a flat, non-recursive lookup -- every `prior` is a real
/// dated event with its own known year, never a further chain this
/// function itself must walk); `EraOnly` is unused this batch (no Era
/// nodes materialize until M-C) and returns `None`, honestly, rather than
/// fabricating one.
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

/// The full derivation: reconstructs the timeline order, chooses each dated
/// event's placement, and resolves each to a `ResolvedPlacement`.
///
/// SEQKEY (owner design, `graph_types::chrono::ResolvedPlacement::seq`):
/// assigned DIRECTLY from the reconstructed timeline position (`i as u32`),
/// not by walking `SequenceAfter` spacing sums. This is exact, not a
/// shortcut: `temporal_order` compares `(date.from, date.to, seq)`, and
/// this function sets `date.to == date.from` for every event (a disclosed
/// simplification -- true ranged `to_year` values stay available on the
/// pre-existing `/api/event/{id}` surface, untouched, but no M-B wire
/// surface exposes `ResolvedDate.to` yet, so collapsing it to a point makes
/// the `to`-comparison step of `temporal_order` an unconditional no-op
/// whenever `from` ties, which is what makes assigning `seq` directly from
/// the SAME total order `date.from` already encodes exact by construction,
/// not merely usually-correct).
pub fn derive_chronology(atlas: &AtlasData) -> ChronologyDerivation {
    let order = timeline_order(atlas);
    let position: HashMap<&str, usize> = order.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();

    let anchor_by_event: HashMap<&str, &ChronologyAnchor> =
        atlas.chronology_anchors.iter().filter_map(|a| a.event_id.as_deref().map(|eid| (eid, a))).collect();
    let earliest_anchor = atlas.chronology_anchors.iter().min_by_key(|a| a.year);
    let same_prior = same_narrative_prior(atlas);

    let mut placements = HashMap::new();
    let mut resolved = HashMap::new();

    for (i, id) in order.iter().enumerate() {
        let e = atlas.event_by_id(id).expect("id just collected from atlas.events");
        let placed = choose_placement(e, i, &order, &anchor_by_event, earliest_anchor, &same_prior, atlas);

        let tp = TimePoint::year_only(Year::new(e.when.from_year).expect("dated event year is never 0"));
        let seq = SeqKey(position[id.as_str()] as u32);
        let rp = ResolvedPlacement { date: ResolvedDate { from: tp, to: tp }, seq, basis: placed.basis };

        placements.insert(id.clone(), placed);
        resolved.insert(id.clone(), rp);
    }

    ChronologyDerivation { order, placements, resolved }
}

/// RETIRED (controller decision 1, M-C): `NarrativeLegPosition` and the
/// `narrative_positions` index it fed used to exist ONLY because
/// `graph_types::edge::EdgeEntry` carried no per-edge NARRATIVE TAG -- the
/// types spec's own illustrative `tagged_by: NarrativeId` on `Succession`
/// was prose-only in the compiled crate at M-B's base. That gap CLOSED at
/// this batch's own base: `graph-types` commit `13184e1` (owner-approved,
/// "EdgeMeta -- per-entry relation metadata") gives every `follows-in`/
/// `precedes-in` entry a real `EdgeMeta::Narrative(NarrativeId)` -- see
/// `graph_types::graph::Graph::build_indexes`'s own `Succession` pairing,
/// which has tagged every entry with its narrative since before this
/// batch. "Which narrative is this succession edge FOR" is now answerable
/// straight off the generic port (`snap.edges(event, follows-in)`'s own
/// `EdgePage.entries[].meta`, filtered/grouped by `EdgeMeta::Narrative`) --
/// `handlers::narrative_event_positions` now builds its own narrative-leg
/// view this way (see that handler's own doc comment); this module no
/// longer duplicates the lookup. The M-B review's I-3 (validation bypass:
/// this companion index could silently diverge from the graph's own
/// `Succession` rows, since nothing PROVED the two agreed) dies with it --
/// there is now only one representation of "which narrative is this leg
/// in," not two.
///
/// KEPT (chronology half, renamed honestly): `temporal-adjacency` is STILL
/// a derived SYMMETRIC relation `Graph::build_indexes` does not populate
/// (`explore.rs`'s own `raw_neighbors` comment: "Symmetric relations:
/// skeleton serves none yet") -- out of this batch's own named scope
/// (design §8 M-C: cross-refs/catechism/places/eras/mentions; M-C does
/// close the symmetric-index gap for `catechism-link`, see
/// `graph.rs`/`explore.rs`, but `temporal-adjacency` specifically is not
/// among this batch's new relations and stays a disclosed, standing gap --
/// see Concerns). `Chronology` (renamed from `EventWorld`, since
/// `narrative_positions` was the other half of that name's own "world")
/// is now PURELY the chronology companion: the derivation
/// (`ChronologyDerivation`, still needed to BUILD `dated_by` rows) plus
/// `temporal_neighbors` (global chronological prior/following, answering
/// the SAME functional need `EventWorld::temporal_neighbors` always did).
pub struct Chronology {
    pub chrono: ChronologyDerivation,
    pub temporal_neighbors: HashMap<String, (Option<String>, Option<String>)>,
}

impl Chronology {
    pub fn build(atlas: &AtlasData) -> Chronology {
        Self::from_derivation(derive_chronology(atlas))
    }

    /// M-C: builds the `temporal_neighbors` companion from an
    /// ALREADY-COMPUTED `ChronologyDerivation` -- factored out of `build`
    /// so the artifact-load path (`GraphService::from_artifact`, which has
    /// no `AtlasData` to re-derive from) can reconstruct the SAME
    /// `Chronology` from the serialized artifact's own chronology fields,
    /// and so the compile step (`bin/compile_graph.rs`) can reuse the
    /// SAME `ChronologyDerivation` the pipeline's own RESOLVE stage already
    /// computed instead of recomputing `derive_chronology(atlas)` a
    /// second time (a pre-existing, disclosed minor redundancy in
    /// `GraphService::assemble`'s own from-sources path, unaffected here).
    pub fn from_derivation(chrono: ChronologyDerivation) -> Chronology {
        let mut temporal_neighbors: HashMap<String, (Option<String>, Option<String>)> = HashMap::new();
        for (i, id) in chrono.order.iter().enumerate() {
            let prior = i.checked_sub(1).and_then(|j| chrono.order.get(j)).cloned();
            let following = chrono.order.get(i + 1).cloned();
            temporal_neighbors.insert(id.clone(), (prior, following));
        }

        Chronology { chrono, temporal_neighbors }
    }

    /// `AnchorId.0 -> TimePoint`, for law tests / the resolver -- built
    /// from the SAME `chronology_anchors` table `populate` reads.
    pub fn anchor_years(atlas: &AtlasData) -> HashMap<String, TimePoint> {
        atlas
            .chronology_anchors
            .iter()
            .map(|a| (a.id.clone(), TimePoint::year_only(Year::new(a.year).expect("anchor year is never 0"))))
            .collect()
    }

    /// `event id -> TimePoint`, for every DATED event -- the flat,
    /// non-recursive base `resolve_timepoint`'s `SequenceAfter` arm reads.
    pub fn event_years(atlas: &AtlasData) -> HashMap<String, TimePoint> {
        atlas
            .events
            .iter()
            .filter(|e| e.kind == "event")
            .map(|e| (e.id.clone(), TimePoint::year_only(Year::new(e.when.from_year).expect("dated event year is never 0"))))
            .collect()
    }
}

/// Startup-log-friendly counts, mirrors `build::BuildStats`'s own role for
/// the KJV/xrefs half.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
}

/// The four node-id constructors for this batch's own kinds -- mirrors
/// `kjv_adapter::verse_node_id`'s own role (one place the internal id
/// SHAPE is decided; every caller, including `atlas-server`'s own
/// `handlers.rs`, goes through these rather than hand-building
/// `AnyNodeId { kind, raw }` at each call site).
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
    // M-C (controller decision 2): real payload, not a stub -- lat/lon and
    // the KJV alias (E3 naming; `Named` rows have no `Position`-typed
    // object to index through the generic port, see `graph.rs::
    // build_indexes`'s own disclosed note -- the payload is the queryable
    // form) join the canonical name.
    let aliases: Vec<String> = atlas
        .place_name_alias_for(&p.id)
        .and_then(|a| a.translations.get(crate::kjv_adapter::KJV_TRANSLATION))
        .cloned()
        .into_iter()
        .collect();
    Node {
        id: PlaceId::new(p.id.clone()).erase(),
        payload: NodePayload::Place { canonical: p.name.clone(), lat: p.lat, lon: p.lon, aliases },
        provenance: "curated-places".to_string(),
    }
}

fn event_provenance(id: &str) -> &'static str {
    if id.starts_with("theo-") {
        "theographic"
    } else {
        "curated"
    }
}

/// M-C2: real payload, not a stub -- every field `handlers::event`'s own
/// full detail fetch (title/kind/when/witnesses/robertson_section/
/// acts_section/atlas_section/kjv_superscription/ref_note) needs, plus the
/// container's own top-level `verses` (scripture-mode scene composition's
/// own filtering set, distinct from witness verses -- see `NodePayload::
/// Event`'s own doc comment). `places` deliberately does NOT ride here --
/// `located_at` rows (below) are the single, order-preserving source.
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
            kind: e.kind.clone(),
            from_year: e.when.from_year,
            to_year: e.when.to_year,
            order_key: e.order_key,
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

/// M-C2: `color` joins `label` -- see `NodePayload::Narrative`'s own doc
/// comment for why `legs` stays off the payload (the `succession` relation
/// is the single source).
fn narrative_node(n: &atlas_core::data::Narrative) -> Node {
    Node {
        id: NarrativeId::new(n.id.clone()).erase(),
        payload: NodePayload::Narrative { label: n.name.clone(), color: n.color.clone() },
        provenance: "curated-narratives".to_string(),
    }
}

/// Human-legible Anchor card label/citation -- `graph_types::node::card()`
/// uses this field VERBATIM as the Anchor's own label (see `node.rs`:
/// `NodePayload::Anchor { citation, .. } => citation.clone()`), which is
/// the owner's own "why this date?" explorable design: opening an Anchor's
/// card IS reading its citation.
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

/// Legacy `[[witness]]` rows (and the top-level `verses` synthesis
/// `scene::witnesses_for` already performs for events with none) BECOME
/// `attests` rows -- one per INDIVIDUAL VERSE (not per contiguous verse
/// group), so parallel accounts remain "multiple attestations on ONE
/// event" (design doc §4) AND every witnessed verse -- not just each
/// group's own first -- resolves back to its event through the generic
/// port. M-C2, a real gap found while migrating `handlers::verse`'s own
/// `events_for_verse` reconstruction onto the graph, disclosed: the
/// pre-M-C2 shape collapsed each verse GROUP to one `BibleLocusRange`
/// row spanning only its own first/last verse, and `Graph::build_indexes`
/// only ever lowers a relation's row into ONE edge per row (this is true
/// of `cites` too, by the same shape -- see `graph_types::edge::CrossRef`'s
/// own M-C2 widening) -- so a verse cited ONLY by a witness in the
/// INTERIOR of a group (e.g. `MAT.26.6`, Matthew's own account of the
/// Bethany anointing, `pw_bethany`'s own top-level `verses` field is
/// JHN.12.1-11 only) would never resolve to its own event via `attested-in`
/// at all, even though `AtlasData::events_for_verse` (the pre-M-C2 index
/// this migration retires) already unions witness verses in too (Batch T
/// requirement 3's own fix, `data.rs`'s own doc comment: "a real, live-
/// caught bug"). One row per verse closes this the SAME way, at the
/// SOURCE, rather than re-introducing a companion index: each row's own
/// `attestation` is a single-verse range (`from == to`), so `to_last`-style
/// span data is unnecessary here (unlike `cites`, whose target is a
/// DIFFERENT locus than its subject) -- `handlers::event`'s own witness
/// display is UNAFFECTED (it reads the widened `NodePayload::Event.
/// witnesses` payload directly, never these rows -- see `event_node`'s own
/// doc comment). Nothing in `event-witnesses.toml`/`events-extra.toml`/
/// `passages/*.toml` is re-parsed or re-authored -- `scene::witnesses_for`
/// is the SAME resolver `GET /api/event/{id}` already calls.
fn verse_to_range(v: &str) -> Option<BibleLocusRange> {
    let vid = atlas_core::refs::VerseId::parse_canonical(v).ok()?;
    let locus: BibleLocus = Locus::<BibleTag>::whole(VerseRef { book: vid.book.0, chapter: vid.chapter, verse: vid.verse });
    BibleLocusRange::new(locus.clone(), locus).ok()
}

/// Populates Event/Narrative/Anchor/Place-STUB nodes and Attests/
/// Succession/DatedBy/LocatedAt rows onto `graph`, from `atlas` (curated,
/// already-merged/calibrated) and `chrono` (already-derived, see
/// `derive_chronology`). Caller runs `graph.build_indexes()` once, after
/// this AND the KJV/xrefs population, exactly like `build::
/// build_graph_from_canon_and_verses` already does for M-A's own two
/// relations.
pub fn populate(graph: &mut Graph, atlas: &AtlasData, chrono: &ChronologyDerivation) -> EventWorldStats {
    let mut stats = populate_nodes_and_direct_rows(graph, atlas);
    stats.dated_by_rows = populate_dated_by(graph, chrono);
    stats
}

/// NORMALIZE-shaped half (pipeline.rs's own stage mapping): nodes
/// (Place-stub/Event/Narrative/Anchor) plus the directly-authored
/// `attests`/`located_at`/`succession` rows -- none of these need any
/// OTHER pass's output first (in particular, none read `chrono`). Split
/// from the RESOLVE-shaped `populate_dated_by` below so the M-C pipeline
/// contract can run each at its own documented stage; `populate` above
/// keeps calling both, in order, for every pre-existing direct caller
/// (this module's own tests) that has no reason to care about the split.
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
        if e.kind == "event" {
            stats.dated_events += 1;
        }

        let event_id = EventId::new(e.id.clone());

        for w in atlas_core::scene::witnesses_for(e) {
            let text = match (&w.ref_note, &w.robertson_section) {
                (Some(r), Some(rs)) => Some(format!("{r}; {rs}")),
                (Some(r), None) => Some(r.clone()),
                (None, Some(rs)) => Some(rs.clone()),
                (None, None) => None,
            };
            for vg in &w.verse_groups {
                for v in &vg.verses {
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

    stats
}

/// RESOLVE-shaped half (pipeline.rs's own stage mapping): `DatePlacement`
/// resolution's own row construction, from an ALREADY-COMPUTED
/// `ChronologyDerivation` (`derive_chronology`, the pipeline's own RESOLVE
/// stage runs this first) -- exactly `resolve(DatePlacement)` per the
/// ingestion contract's own naming (design doc §7). Split out of
/// `populate` so this can run at its own pipeline stage, after NORMALIZE.
/// Returns the row count (folded into `EventWorldStats.dated_by_rows` by
/// `populate` above; the pipeline's own `ResolvePass` folds it into
/// `BuildCtx.event_world_stats` the same way).
///
/// Iterates `chrono.order` (a plain, deterministic `Vec<String>`), NOT
/// `chrono.placements` directly (a `HashMap`, whose iteration order is
/// randomized per-instance by `RandomState` -- confirmed empirically:
/// two independent `populate` calls over IDENTICAL input produced
/// DIFFERENT `graph.dated_by` row orders, which `assert_answers_match`
/// correctly caught as a real divergence in `DatedBy`'s own inverse
/// edge order wherever two+ events share one target, e.g. two events
/// both chained `SequenceAfter` off the same `prior`). Row order must
/// be a pure function of content, not of a HashMap's incidental
/// per-instance hash seed -- this is what makes it one again.
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

/// Pipeline-facing NORMALIZE entry point (`pipeline::NormalizePass`):
/// event-world nodes + direct rows, from `ctx.atlas` -- mirrors
/// `populate_nodes_and_direct_rows` exactly (that function's own doc
/// comment), just reading/writing through `BuildCtx` instead of loose
/// parameters.
pub fn normalize(ctx: &mut crate::pipeline::BuildCtx) {
    ctx.event_world_stats = populate_nodes_and_direct_rows(&mut ctx.graph, ctx.atlas);
}

/// Pipeline-facing RESOLVE entry point (`pipeline::ResolvePass`): computes
/// the chronology derivation (`derive_chronology`) and its own `dated_by`
/// rows (`populate_dated_by`) -- exactly `resolve(DatePlacement)` per the
/// ingestion contract's own naming (design doc §7).
pub fn resolve(ctx: &mut crate::pipeline::BuildCtx) {
    ctx.chrono = derive_chronology(ctx.atlas);
    ctx.event_world_stats.dated_by_rows = populate_dated_by(&mut ctx.graph, &ctx.chrono);
}

/// The SAME node-id shape `graph_types::graph::Graph::build_indexes`'s own
/// PRIVATE `text_node` helper produces from a `TextRef::Bible(VerseRef)`
/// (`"bible/{book}.{chapter}.{verse}"` -- documented, not guessed, in
/// `kjv_adapter`'s own module doc comment: "This format is also exactly
/// what... `text_node` produces"). Needed here because `text_node` itself
/// is private to graph-types (an internal detail of `build_indexes`, not
/// part of its public surface) and `add_justified_by` below (a POST-
/// processing step over the already-built graph, not a graph-types change)
/// must resolve a `Ground::Scripture`'s locus to the SAME node id
/// `Graph::build_indexes` would have used, so a `justified-by` edge lands
/// on the identical TextUnit node any OTHER path (e.g. `attests`) already
/// reaches.
fn bible_locus_node_id(v: &VerseRef) -> atlas_graph_types::id::AnyNodeId {
    atlas_graph_types::id::AnyNodeId { kind: atlas_graph_types::id::NodeKind::TextUnit, raw: format!("bible/{}.{}.{}", v.book, v.chapter, v.verse) }
}

/// "justified-by where grounds exist" (brief requirement 4). Wires the
/// ALREADY-declared `RelationId::JustifiedBy` relation (present in
/// graph-types' own relation manifest since before this batch; unused
/// until now -- confirmed by reading `Graph::build_indexes` fresh, which
/// never populates it) for rows carrying a non-empty
/// `Justification.grounds` -- today, only `DatedBy` rows (this adapter's
/// `Attests`/`LocatedAt`/`Succession` rows all carry empty grounds; nothing
/// to wire for them yet, disclosed rather than silently padded with
/// meaningless entries).
///
/// A REAL POST-PROCESSING STEP over the already-built graph, NOT a
/// graph-types change: computes each qualifying row's own `EdgeId` THE
/// SAME WAY `Graph::build_indexes` computes it internally for that
/// relation (mirrored here -- that pairing logic is private, so it cannot
/// be called directly), then merges a NEW `RelationId::JustifiedBy` index
/// into `graph.indexes` using graph-types' own public `entry_id`/`BiIndex`
/// primitives. Must run AFTER `graph.build_indexes()` (the DatedBy index it
/// mirrors must already exist for the two to agree on the SAME edge ids).
pub fn add_justified_by(graph: &mut Graph) -> usize {
    use atlas_graph_types::chrono::ChronoTarget;
    use atlas_graph_types::edge::{at, entry_id, BiIndex, Ground, RelationId};
    use atlas_graph_types::explore::EdgeMeta;
    use atlas_graph_types::id::Position;

    // Controller decision 1 (EdgeMeta alignment): `BiIndex::build` now takes
    // (Position, Position, EdgeMeta) triples (graph-types commit 13184e1,
    // owner-approved at this batch's base) -- `justified-by` rows carry no
    // per-entry metadata of their own, so every triple supplies
    // `EdgeMeta::None` honestly (there is nothing else to say about a
    // justification pointer beyond which node it targets).
    let mut pairs: Vec<(Position, Position, EdgeMeta)> = Vec::new();
    for row in &graph.dated_by {
        if row.justification.grounds.is_empty() {
            continue;
        }
        // Mirrors Graph::build_indexes's own DatedBy (subject, object)
        // construction exactly (graph-types/src/graph.rs) -- the pair
        // whose entry_id IS the edge id a client already sees on this
        // row's own dated-by/dates entry.
        let subject = at(&row.event.erase());
        let object = match row.placement.target() {
            ChronoTarget::Anchor(a) => at(&a.erase()),
            ChronoTarget::Prior(p) => at(&p.erase()),
            ChronoTarget::Era(er) => at(&er.erase()),
        };
        let edge_id = entry_id(RelationId::DatedBy, &subject, &object);

        for ground in &row.justification.grounds {
            let target = match ground {
                Ground::Scripture(range) => Position::Node(bible_locus_node_id(&range.from.unit)),
                Ground::Anchor(a) => at(&a.erase()),
                Ground::Source(s) => at(&s.erase()),
            };
            pairs.push((Position::Edge(edge_id.clone()), target, EdgeMeta::None));
        }
    }

    let count = pairs.len();
    if count > 0 {
        graph.indexes.insert(RelationId::JustifiedBy, BiIndex::build(RelationId::JustifiedBy, &pairs));
    }
    count
}

/// A trivial, event-world-empty `AtlasData` fixture -- shared by every
/// KJV/xrefs-only test across this crate AND its own integration test
/// files (`tests/*.rs`, which link against the NORMAL, non-`cfg(test)`
/// build of this library, so this helper is deliberately NOT gated behind
/// `#[cfg(test)]`) that needs a real `&AtlasData` to satisfy
/// `build_graph_from_sources`'s signature but exercises no M-B behavior.
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
        // Same year (-500): stable tie-break keeps declared/array order (a
        // before b is WRONG here -- events are constructed [b, a, c], and
        // AtlasData::finish's own from_year sort is stable, so among the
        // two -500 events "b" stays before "a" exactly as declared).
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
        // Mirrors a real ANCHOR_DEFERRALS row: table year differs from the
        // event's own shipped year -- offset must make resolution land on
        // the EVENT's own true year regardless.
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

    // `narrative_positions_mirror_the_windows_adjacency_the_old_resolver_used`
    // RETIRED (controller decision 1, M-C): the subject it proved --
    // "querying narrative-leg adjacency for an event returns the right
    // prior/following" -- moved to the generic port (EdgeMeta::Narrative on
    // `follows-in`/`precedes-in` entries) and is now proven at the SERVER
    // layer, over real HTTP, by
    // `atlas-server/tests/graph_api.rs::narrative_leg_positions_come_from_
    // the_generic_ports_own_edgemeta_narrative_tag` -- not duplicated here,
    // since `Chronology` no longer computes this at all (see this module's
    // own `Chronology` doc comment).

    #[test]
    fn temporal_neighbors_follow_the_global_timeline_not_narrative_membership() {
        let atlas = atlas_with(
            vec![event("e5", -2000, 0), event("e1", -1406, 0), event("e2", -1406, 0)],
            vec![],
            vec![],
        );
        let world = Chronology::build(&atlas);
        assert_eq!(world.temporal_neighbors["e5"], (None, Some("e1".to_string())));
        assert_eq!(world.temporal_neighbors["e1"], (Some("e5".to_string()), Some("e2".to_string())));
        assert_eq!(world.temporal_neighbors["e2"], (Some("e1".to_string()), None));
    }

    #[test]
    fn general_kind_events_are_excluded_from_the_timeline_and_get_no_dated_by_row() {
        let mut general = event("g1", -4004, 0);
        general.kind = "general".to_string();
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
        // Non-anchor-bound (SequenceAfter) events carry a default,
        // empty-grounds Justification -- no justified-by entries at all.
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
}
