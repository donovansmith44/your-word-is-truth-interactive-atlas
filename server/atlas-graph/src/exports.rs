//! C2C3-EXPORT (map-system contracts C2/C3; design doc
//! `.superpowers/sdd/2026-08-17-bible-atlas-m1/c2c3-export-design.md`):
//! the two committed cross-repo contract files under `data/exports/` --
//! `gazetteer.json` (C3: place id/canonical/aliases/lat/lon/provenance --
//! the coordinate authority the map system's own PlaceId waypoints bind
//! to) and `chronology.json` (C2: per-event ResolvedPlacement + a
//! reign/era span table + the anchor table CREATION rides on -- the date
//! authority for every Scripture-attested change the map narrates).
//!
//! WRITTEN BY `bins/compile_graph.rs`, as a terminal pass immediately
//! after its own ADMISSION checks pass -- the SAME invocation that
//! writes `graph.bin`, over the SAME already-admitted `Graph`, so
//! `atlas_version_root` embeds the identical `GraphVersion` the artifact
//! reports (drift is impossible by construction, not merely
//! disciplined). Nothing here is served -- files ARE the whole contract
//! (design doc point 5); the map repo fetches them at a pinned SHA.
//!
//! HONESTY DISCLOSURES (the design doc's own "open questions," resolved
//! here rather than left silent):
//!
//! - CONFIDENCE: `NodePayload::Place` carries no confidence field
//!   anywhere in the compiled graph (confirmed by reading every Place
//!   adapter fresh -- `event_world::place_node` builds
//!   canonical/lat/lon/aliases/description only, and `Node.provenance`
//!   is a bare `ProvenanceId` string, never the richer confidence-
//!   bearing `ingest::Provenance` struct). `GazetteerPlace.confidence`
//!   stays `None` for all places today (0/N coverage), `#[serde(skip_
//!   serializing_if)]`'d away rather than emitted as a fabricated
//!   default. The field stays IN the schema (not deleted) so a future
//!   adapter that captures it needs no format bump to populate it.
//! - ANCHOR-REIGN SPANS: `DatePlacement::ReignYear` is a real variant in
//!   `graph-types::chrono`, but `event_world::choose_placement` never
//!   constructs one over real data (confirmed: its only real branches
//!   are `AnchorBinding`/`SequenceAfter`; the `EraOnly` fallback is
//!   unreachable over the real 20+-row anchor table, and `ReignYear` has
//!   no constructor site anywhere in this workspace). Polity eras
//!   (`NodePayload::Polity.eras`) group MULTIPLE reigns per row (e.g.
//!   "Kingdom of Israel" spans Jeroboam I through Hoshea, ~19 kings) --
//!   not one reign, so borrowing them here would misrepresent the
//!   `anchor-reign` kind's own meaning. `spans[]` therefore ships `kind:
//!   "era"` rows only (the real 10 `NodePayload::Era` nodes) --
//!   `anchor-reign` is a live schema value with zero real instances
//!   today, disclosed rather than populated by guessing which
//!   anchor-to-next-anchor gap "counts" as a reign. The peer's own
//!   motivating case (2KI 14:25 / Jeroboam II) is served today at ERA
//!   granularity: the event's own point-year placement plus the
//!   enclosing "divided-kingdom" era span already let a consumer render
//!   an honest interval; per-reign granularity is a real, disclosed gap
//!   for whenever reign-level `DatePlacement`s exist.
//! - ALIAS/CANONICAL SPOT-CHECKS (the peer's own binding names, verified
//!   against real compiled data, not assumed): "Kadesh-barnea" and
//!   "En-rogel" both resolve -- as CANONICAL place names
//!   (`NodePayload::Place.canonical`), not KJV aliases (the E3 alias
//!   sweep never touched either; their Theographic canonical name
//!   already matches the KJV spelling exactly, hyphen included).
//!   "entrance of Hamath" (Batch GAZ-1-R1) now ALSO resolves -- as a
//!   curated KJV ALIAS on `lebo-hamath`, the real-world location Scripture's
//!   own border-survey phrase (Num 34:8 KJV: "the entrance of Hamath")
//!   names: the traditional identification for the city Hamath's own
//!   boundary marker, already geocoded and already carrying every one of
//!   this phrase's own attested verses -- not a new place (minting one
//!   would duplicate `lebo-hamath`'s own real-world location under a second
//!   id). `lebo-hamath` carries FIVE further curated aliases beyond this
//!   one -- every other distinct verbatim wording the same boundary idiom
//!   takes across its own 11 attested verses ("entering into Hamath" JOS
//!   13:5; "entering in of Hamath" 1KI 8:65/JDG 3:3/2CH 7:8; "entering of
//!   Hamath" 2KI 14:25; "entering in of Hemath" AMO 6:14; "entering of
//!   Hemath" 1CH 13:5 -- the KJV's own "Hemath" spelling, distinct from
//!   "Hamath", both genuinely attested) -- see `data/curated/
//!   place-names-kjv.toml`'s own GAZ-1-R1 section for the full citation
//!   set. `tests/exports_laws.rs` writes a law against all three peer-named
//!   binding examples now.
//! - THEO-* IDS: Theographic-opaque event ids (`theo-*`) ship as their
//!   raw graph id, unrenamed -- `label` + `attestations` carry the
//!   binding burden for a consumer, exactly the design's own "no
//!   invented slugs" rule (renaming would break content addressing for
//!   a cosmetic win).
//! - PLACEMENT SHAPE: the design's own `year|interval` placement shape
//!   is realized as a plain `{from_year, to_year}` pair, never a tagged
//!   enum -- `from_year == to_year` IS the "year" case, `from_year <
//!   to_year` IS the "interval" case (real for ~40/554 dated events,
//!   e.g. reign-summary spans -- `event_world::SourceEventMeta`'s own
//!   genuine, ordering-uninvolved `to_year`, the SAME field
//!   `artifact.rs`'s own `chrono_years` already carries). One shape, no
//!   second parse path for a consumer.

use std::collections::{BTreeMap, BTreeSet};

use atlas_core::data::ChronologyAnchor;
use atlas_graph_types::chrono::PlacementBasis;
use atlas_graph_types::edge::MentionedEntity;
use atlas_graph_types::graph::Graph;
use atlas_graph_types::id::NodeKind;
use atlas_graph_types::node::NodePayload;
use atlas_graph_types::text::{BibleLocus, BibleLocusRange, VerseRef};

use serde::{Deserialize, Serialize};

use crate::event_world::{event_node_id, Chronology, SourceEventMeta};

fn book_code(book: u8) -> &'static str {
    atlas_core::canon::BOOKS.get(book as usize).map(|b| b.code).unwrap_or("???")
}

fn verse_ref_str(v: &VerseRef) -> String {
    format!("{}.{}.{}", book_code(v.book), v.chapter, v.verse)
}

fn bible_locus_str(l: &BibleLocus) -> String {
    verse_ref_str(&l.unit)
}

/// One verse ("GEN.13.18") or, for a genuine multi-verse span, a
/// hyphenated range -- same-book-and-chapter collapses to
/// "GEN.13.18-19"; otherwise both ends are named in full
/// ("GEN.13.18-EXO.1.1"), the same convention `DtoCrossRef.target_
/// display` already established (artifact.rs).
fn bible_range_str(r: &BibleLocusRange) -> String {
    let from = &r.from.unit;
    let to = &r.to.unit;
    if from == to {
        verse_ref_str(from)
    } else if from.book == to.book && from.chapter == to.chapter {
        format!("{}.{}.{}-{}", book_code(from.book), from.chapter, from.verse, to.verse)
    } else {
        format!("{}-{}", verse_ref_str(from), verse_ref_str(to))
    }
}

fn basis_str(b: PlacementBasis) -> &'static str {
    match b {
        PlacementBasis::Textual => "Textual",
        PlacementBasis::Traditional => "Traditional",
    }
}

// ---------------------------------------------------------------------
// C3 -- gazetteer.json
// ---------------------------------------------------------------------

/// Bumped whenever any field here changes shape -- independent of
/// `graph.bin`'s own `artifact::FORMAT_VERSION` (different consumer,
/// different promise; `artifact.rs`'s own doc-note convention, applied
/// here). Starts at 1 (C2C3-EXPORT, 2026-08-24).
pub const GAZETTEER_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GazetteerPlace {
    pub id: String,
    pub canonical: String,
    pub aliases: Vec<String>,
    pub lat: f64,
    pub lon: f64,
    pub provenance: String,
    /// Absent everywhere today -- see this module's own header comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    /// Verse loci `Mentions` rows attach to this place (cheap: already a
    /// real graph table), sorted, deduped.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GazetteerExport {
    pub format_version: u32,
    pub atlas_version_root: String,
    pub places: Vec<GazetteerPlace>,
}

/// Collects every `Place` node's own export row, in node-id order
/// (`Graph.nodes` is a `BTreeMap`, so this is deterministic without any
/// extra sort). Pure over `graph` -- no `atlas_version_root` yet (the
/// caller embeds that after computing `GraphVersion` from the SAME
/// graph, so drift is impossible by construction -- see this module's
/// own header comment).
pub fn gazetteer_places(graph: &Graph) -> Vec<GazetteerPlace> {
    let mut attestations_by_place: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for m in &graph.mentions {
        if let MentionedEntity::Place(p) = &m.entity {
            if let Some(bl) = m.locus.as_bible() {
                attestations_by_place.entry(p.0.clone()).or_default().insert(bible_locus_str(&bl));
            }
        }
    }

    graph
        .nodes
        .values()
        .filter(|n| n.id.kind == NodeKind::Place)
        .filter_map(|n| match &n.payload {
            NodePayload::Place { canonical, lat, lon, aliases, .. } => Some(GazetteerPlace {
                id: n.id.raw.clone(),
                canonical: canonical.clone(),
                aliases: aliases.clone(),
                lat: *lat,
                lon: *lon,
                provenance: n.provenance.clone(),
                confidence: None,
                attestations: attestations_by_place.get(&n.id.raw).cloned().unwrap_or_default().into_iter().collect(),
            }),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// C2 -- chronology.json
// ---------------------------------------------------------------------

pub const CHRONOLOGY_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyPlacement {
    pub from_year: i32,
    pub to_year: i32,
    pub basis: String,
    pub seq: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyEvent {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestations: Vec<String>,
    pub placement: ChronologyPlacement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologySpan {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub from: i32,
    pub to: i32,
    pub basis: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyTimePoint {
    pub year: i32,
    pub month: Option<u8>,
    pub day: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyAnchorRow {
    pub id: String,
    pub label: String,
    pub at: ChronologyTimePoint,
    pub citation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChronologyExport {
    pub format_version: u32,
    pub atlas_version_root: String,
    pub events: Vec<ChronologyEvent>,
    pub spans: Vec<ChronologySpan>,
    pub anchors: Vec<ChronologyAnchorRow>,
}

/// Every DATED event (`chronology.chrono.order` -- exactly the graph's
/// own `kind == "event"` subset, in global timeline order), each with
/// its own real `ResolvedPlacement`. General-kind/undated Event nodes
/// carry no placement to report and are out of scope for a dating
/// export by construction -- excluding them is the only honest choice,
/// not a silent drop (`tests/exports_laws.rs` asserts the count: every
/// id in `chrono.order` produces exactly one row here, none unresolved).
pub fn chronology_events(graph: &Graph, chronology: &Chronology) -> Vec<ChronologyEvent> {
    let mut attestations_by_event: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for a in &graph.attests {
        attestations_by_event.entry(a.event.0.clone()).or_default().insert(bible_range_str(&a.attestation));
    }

    chronology
        .chrono
        .order
        .iter()
        .filter_map(|id| {
            let node = graph.nodes.get(&event_node_id(id))?;
            let label = match &node.payload {
                NodePayload::Event { label, .. } => label.clone(),
                _ => return None,
            };
            let resolved = chronology.chrono.resolved.get(id)?;
            let meta = chronology.chrono.source_meta.get(id).copied().unwrap_or(SourceEventMeta { to_year: resolved.date.from.year.get(), order_key: 0 });
            Some(ChronologyEvent {
                id: id.clone(),
                label,
                attestations: attestations_by_event.get(id).cloned().unwrap_or_default().into_iter().collect(),
                placement: ChronologyPlacement {
                    from_year: resolved.date.from.year.get(),
                    to_year: meta.to_year,
                    basis: basis_str(resolved.basis).to_string(),
                    seq: resolved.seq.0,
                },
            })
        })
        .collect()
}

/// `kind: "era"` rows only -- see this module's own header comment for
/// why `anchor-reign` ships with zero rows this batch (a disclosed
/// schema gap, not a fabricated one).
pub fn chronology_spans(graph: &Graph) -> Vec<ChronologySpan> {
    graph
        .nodes
        .values()
        .filter(|n| n.id.kind == NodeKind::Era)
        .filter_map(|n| match &n.payload {
            NodePayload::Era { label, from_year, to_year } => {
                Some(ChronologySpan { id: n.id.raw.clone(), label: label.clone(), kind: "era".to_string(), from: *from_year, to: *to_year, basis: basis_str(PlacementBasis::Traditional).to_string() })
            }
            _ => None,
        })
        .collect()
}

/// Every Anchor node (including `creation`, id `"creation"` -- the
/// peer's own required resolvable row). `label` is read from the SAME
/// curated `chronology-anchors.toml` rows the graph's own Anchor nodes
/// were built from (the node payload retains only the combined
/// `citation` string, not the short label separately, by the time it
/// reaches the graph -- `event_world::format_anchor_citation` folds
/// both together at build time). Falls back to `citation` itself if a
/// graph Anchor id somehow has no matching curated row (never true of
/// real committed data -- every Anchor node is built FROM one of these
/// rows one-for-one -- but total, not partial).
pub fn chronology_anchors(graph: &Graph, curated_anchors: &[ChronologyAnchor]) -> Vec<ChronologyAnchorRow> {
    let label_by_id: BTreeMap<&str, &str> = curated_anchors.iter().map(|a| (a.id.as_str(), a.label.as_str())).collect();

    graph
        .nodes
        .values()
        .filter(|n| n.id.kind == NodeKind::Anchor)
        .filter_map(|n| match &n.payload {
            NodePayload::Anchor { at, citation } => Some(ChronologyAnchorRow {
                id: n.id.raw.clone(),
                label: label_by_id.get(n.id.raw.as_str()).map(|s| s.to_string()).unwrap_or_else(|| citation.clone()),
                at: ChronologyTimePoint { year: at.year.get(), month: at.month, day: at.day },
                citation: citation.clone(),
            }),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------
// KRETZ-1 -- kretzmann-chronology.json (the date mine).
// ---------------------------------------------------------------------

/// The date mine's own format version -- independent of `CHRONOLOGY_
/// FORMAT_VERSION`/`GAZETTEER_FORMAT_VERSION` above (a different consumer,
/// a different promise; `artifact.rs`'s own doc-note convention, applied
/// here). Starts at 1 (KRETZ-1, 2026-08-25).
pub const KRETZMANN_CHRONOLOGY_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannDateParsed {
    /// "BC" | "AD" | "AM" -- `atlas_etl::kretzmann::Calendar`'s own three
    /// variants, as a plain string (the export boundary, like every other
    /// wire type in this module, carries no Rust enum directly).
    pub calendar: String,
    pub year: u32,
    pub approx: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannDateRow {
    /// The `CommentaryItem` node id this clause's own prose came from
    /// (`"kretzmann/{book}.{chapter}.{ordinal}"`) -- the row's own
    /// provenance (scouting memo: "commentary unit id (provenance)").
    pub unit: String,
    /// The unit's own comments-on target (`bible_range_str`'s own format,
    /// e.g. `"GEN.1.1"` or `"GEN.1.1-3"` -- the SAME convention `Chronology
    /// Event.attestations`/`DtoCrossRef.target_display` already use).
    pub target: String,
    /// A real, literal substring of the unit's own stored prose (asserted
    /// over every row -- scouting memo's own "every verbatim quote is a
    /// substring of its unit's stored prose" law) -- parsing only, never
    /// interpretation.
    pub verbatim: String,
    pub parsed: KretzmannDateParsed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannChronologyExport {
    pub format_version: u32,
    pub atlas_version_root: String,
    /// Scouting memo's own header field, verbatim: this is a TENTATIVE,
    /// mechanically-parsed extraction -- CHRON-CONV-1 (a later, separate
    /// act) adjudicates real chronology placements from it; this export
    /// carries no placement authority of its own.
    pub status: String,
    pub rows: Vec<KretzmannDateRow>,
}

fn calendar_str(c: atlas_etl::kretzmann::Calendar) -> &'static str {
    match c {
        atlas_etl::kretzmann::Calendar::Bc => "BC",
        atlas_etl::kretzmann::Calendar::Ad => "AD",
        atlas_etl::kretzmann::Calendar::Am => "AM",
    }
}

/// Every dating clause found in every `CommentaryItem` node's own stored
/// prose, in node-id order (`Graph.nodes` is a `BTreeMap`, deterministic
/// without an extra sort -- the SAME "free determinism" `gazetteer_places`
/// above already relies on) then discovery order within one unit's own
/// text (`extract_date_clauses`'s own left-to-right scan order, preserved
/// by `Vec::extend`). `target` is resolved from `graph.comments_on`'s own
/// row for that unit (the SAME range every verse-mapped-index query would
/// resolve) -- a unit with somehow no `comments_on` row of its own (never
/// true of a real `kretzmann_adapter::normalize` build; the two are
/// authored together, one row per node) is skipped rather than fabricating
/// a target, matching this pass's own "extraction only" law.
pub fn kretzmann_date_rows(graph: &Graph) -> Vec<KretzmannDateRow> {
    let target_by_item: BTreeMap<String, String> = graph.comments_on.iter().map(|r| (r.item.0.clone(), bible_range_str(&r.on))).collect();

    let mut out = Vec::new();
    for node in graph.nodes.values() {
        if node.id.kind != NodeKind::CommentaryItem {
            continue;
        }
        let NodePayload::CommentaryItem { text, .. } = &node.payload else { continue };
        let Some(target) = target_by_item.get(&node.id.raw) else { continue };
        for clause in atlas_etl::kretzmann::extract_date_clauses(text) {
            out.push(KretzmannDateRow {
                unit: node.id.raw.clone(),
                target: target.clone(),
                verbatim: clause.verbatim,
                parsed: KretzmannDateParsed { calendar: calendar_str(clause.calendar).to_string(), year: clause.year, approx: clause.approx },
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_graph_types::chrono::{ResolvedDate, ResolvedPlacement, SeqKey, TimePoint, Year};
    use atlas_graph_types::edge::{Attests, Justification, Mentions};
    use atlas_graph_types::id::{AnchorId, EventId, PlaceId};
    use atlas_graph_types::node::Node;
    use atlas_graph_types::text::{BibleLocus, TextLocus};
    use std::collections::HashMap;

    fn verse(book: u8, chapter: u16, verse: u16) -> BibleLocus {
        BibleLocus::whole(VerseRef { book, chapter, verse })
    }

    fn place_node(id: &str, canonical: &str, aliases: Vec<String>) -> Node {
        Node {
            id: PlaceId::new(id.to_string()).erase(),
            payload: NodePayload::Place { canonical: canonical.to_string(), lat: 31.0, lon: 35.0, aliases, description: None },
            provenance: "curated-places".to_string(),
        }
    }

    #[test]
    fn gazetteer_round_trips_through_json() {
        let mut g = Graph::default();
        let n = place_node("kadesh-barnea", "Kadesh-barnea", vec!["Kadesh".to_string()]);
        g.nodes.insert(n.id.clone(), n);
        g.mentions.push(Mentions { locus: TextLocus::from(verse(0, 13, 18)), entity: MentionedEntity::Place(PlaceId::new("kadesh-barnea".to_string())), provenance: "theographic-geocoding".to_string() });

        let places = gazetteer_places(&g);
        let export = GazetteerExport { format_version: GAZETTEER_FORMAT_VERSION, atlas_version_root: "deadbeef".to_string(), places };

        let json = serde_json::to_string(&export).expect("serializes");
        let back: GazetteerExport = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, export, "round-trip must be lossless");
        assert_eq!(back.places.len(), 1);
        assert_eq!(back.places[0].canonical, "Kadesh-barnea");
        assert_eq!(back.places[0].attestations, vec!["GEN.13.18".to_string()]);
        assert!(back.places[0].confidence.is_none(), "no confidence data exists to export");
    }

    #[test]
    fn chronology_round_trips_through_json() {
        let mut g = Graph::default();
        let event_id = "theo-1";
        let node = Node { id: EventId::new(event_id.to_string()).erase(), payload: NodePayload::Event { label: "Creation".to_string(), kind: "event".to_string(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None }, provenance: "theographic".to_string() };
        g.nodes.insert(node.id.clone(), node);
        g.attests.push(Attests { event: EventId::new(event_id.to_string()), attestation: BibleLocusRange::new(verse(0, 1, 1), verse(0, 1, 1)).unwrap(), provenance: "p".to_string(), justification: Justification::default() });

        let mut resolved = HashMap::new();
        resolved.insert(event_id.to_string(), ResolvedPlacement { date: ResolvedDate { from: TimePoint::year_only(Year::new(-4004).unwrap()), to: TimePoint::year_only(Year::new(-4004).unwrap()) }, seq: SeqKey(0), basis: PlacementBasis::Traditional });
        let derivation = crate::event_world::ChronologyDerivation { order: vec![event_id.to_string()], placements: HashMap::new(), resolved, source_meta: HashMap::new() };
        let chronology = Chronology::from_derivation(derivation);

        let events = chronology_events(&g, &chronology);
        assert_eq!(events.len(), 1, "every id in chrono.order must produce exactly one row");
        assert_eq!(events[0].placement.from_year, -4004);
        assert_eq!(events[0].placement.to_year, -4004, "no source_meta row falls back to from_year, never a fabricated widening");
        assert_eq!(events[0].attestations, vec!["GEN.1.1".to_string()]);

        let anchor_node = Node { id: AnchorId::new("creation".to_string()).erase(), payload: NodePayload::Anchor { at: TimePoint::year_only(Year::new(-4004).unwrap()), citation: "Creation of the world — 4004 BC. Source: Ussher's Annals of the World (1658).".to_string() }, provenance: "chronology-anchors".to_string() };
        let mut g2 = Graph::default();
        g2.nodes.insert(anchor_node.id.clone(), anchor_node);
        let curated = vec![ChronologyAnchor { id: "creation".to_string(), label: "Creation of the world".to_string(), year: -4004, event_id: Some("theo-1".to_string()), era_boundary: true, source: "Ussher's Annals of the World (1658)".to_string(), note: None }];
        let anchors = chronology_anchors(&g2, &curated);
        assert_eq!(anchors.len(), 1);
        assert_eq!(anchors[0].label, "Creation of the world", "label comes from the curated row, not the composite citation string");
        assert_eq!(anchors[0].at.year, -4004);

        let export = ChronologyExport { format_version: CHRONOLOGY_FORMAT_VERSION, atlas_version_root: "deadbeef".to_string(), events, spans: vec![], anchors };
        let json = serde_json::to_string(&export).expect("serializes");
        let back: ChronologyExport = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, export, "round-trip must be lossless");
    }

    #[test]
    fn bible_range_str_formats_single_verse_same_chapter_and_cross_book_ranges() {
        let single = BibleLocusRange::new(verse(0, 1, 1), verse(0, 1, 1)).unwrap();
        assert_eq!(bible_range_str(&single), "GEN.1.1");

        let same_chapter = BibleLocusRange::new(verse(2, 1, 16), verse(2, 1, 19)).unwrap();
        assert_eq!(bible_range_str(&same_chapter), "LEV.1.16-19");

        let cross_book = BibleLocusRange::new(verse(1, 1, 16), verse(1, 2, 1)).unwrap();
        assert_eq!(bible_range_str(&cross_book), "EXO.1.16-EXO.2.1");
    }

    #[test]
    fn kretzmann_date_rows_extracts_verbatim_clauses_and_resolves_their_own_target_and_round_trips_through_json() {
        use atlas_graph_types::edge::{CommentsOn, Justification};
        use atlas_graph_types::id::{CommentaryItemId, SourceId};

        let mut g = Graph::default();
        let item_id = CommentaryItemId::new("kretzmann/23.1.0".to_string());
        g.nodes.insert(
            item_id.erase(),
            Node {
                id: item_id.erase(),
                payload: NodePayload::CommentaryItem {
                    work: SourceId::new("kretzmann-popular-commentary".to_string()),
                    heading: Some("The Fall of Jerusalem.".to_string()),
                    text: "The city fell about 606 B. C. and was later rebuilt.".to_string(),
                },
                provenance: "kretzmann/jeremiah/1".to_string(),
            },
        );
        let range = BibleLocusRange::new(verse(23, 1, 1), verse(23, 1, 1)).unwrap();
        g.comments_on.push(CommentsOn { item: item_id, on: range, provenance: "kretzmann/jeremiah/1".to_string(), justification: Justification::default() });

        let rows = kretzmann_date_rows(&g);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].unit, "kretzmann/23.1.0");
        assert_eq!(rows[0].target, "JER.1.1");
        assert_eq!(rows[0].verbatim, "about 606 B. C.");
        assert_eq!(rows[0].parsed, KretzmannDateParsed { calendar: "BC".to_string(), year: 606, approx: true });
        assert!(g.nodes.get(&CommentaryItemId::new("kretzmann/23.1.0".to_string()).erase()).is_some());
        let node_text = match &g.nodes.values().next().unwrap().payload {
            NodePayload::CommentaryItem { text, .. } => text,
            _ => unreachable!(),
        };
        assert!(node_text.contains(&rows[0].verbatim), "the verbatim clause must be a real substring of the unit's own stored prose");

        let export = KretzmannChronologyExport { format_version: KRETZMANN_CHRONOLOGY_FORMAT_VERSION, atlas_version_root: "deadbeef".to_string(), status: "tentative-extraction".to_string(), rows };
        let json = serde_json::to_string(&export).expect("serializes");
        let back: KretzmannChronologyExport = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, export, "round-trip must be lossless");
    }

    #[test]
    fn kretzmann_date_rows_is_empty_over_a_graph_with_no_commentary_items() {
        let g = Graph::default();
        assert!(kretzmann_date_rows(&g).is_empty());
    }
}
