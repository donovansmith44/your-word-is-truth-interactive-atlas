//! The two committed cross-repo contract files under `data/exports/`. Nothing here is served -- the
//! files ARE the contract -- and both are written over the same admitted `Graph` as the artifact, so
//! their embedded version root cannot drift. A placement is `{from_year, to_year}`; equal means a year.

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

/// One verse, or for a genuine multi-verse span a hyphenated range: same book and chapter collapses
/// to `GEN.13.18-19`, and otherwise both ends are named in full, `GEN.13.18-EXO.1.1`.
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

/// Bumped whenever any field of the gazetteer changes shape, independently of the graph artifact's
/// own format version: a different consumer and a different promise.
pub const GAZETTEER_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GazetteerPlace {
    pub id: String,
    pub canonical: String,
    pub aliases: Vec<String>,
    pub lat: f64,
    pub lon: f64,
    pub provenance: String,
    /// Absent for every place today -- no adapter captures one -- and skipped rather than emitted as a
    /// fabricated default.
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

/// Every `Place` node's export row in node-id order, deterministic without an extra sort. Pure over
/// `graph`: the caller embeds the version root it computes from the SAME graph, so drift is impossible.
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

/// Every DATED event, in global timeline order, each with its own real placement. An undated event has
/// no placement to report and is out of scope for a dating export; excluding it is the honest choice.
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

/// `kind: "era"` rows only: `anchor-reign` is a live schema value with no real instances, since no
/// reign-level placement is ever constructed over the real data.
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

/// Every Anchor node. `label` comes from the curated anchor rows because the node payload keeps only
/// the combined citation string; a graph Anchor with no matching curated row falls back to that
/// citation, so the function stays total.
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

/// The date mine's own format version, independent of the two above: a different consumer and a
/// different promise.
pub const KRETZMANN_CHRONOLOGY_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannDateParsed {
    /// `BC` | `AD` | `AM`, as a plain string -- this export boundary carries no Rust enum.
    pub calendar: String,
    pub year: u32,
    pub approx: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannDateRow {
    /// The `CommentaryItem` node id this clause's prose came from, which is the row's provenance.
    pub unit: String,
    /// The unit's own comments-on target (`bible_range_str`'s own format,
    /// e.g. `"GEN.1.1"` or `"GEN.1.1-3"` -- the SAME convention `Chronology
    /// Event.attestations`/`DtoCrossRef.target_display` already use).
    pub target: String,
    /// A real, literal substring of the unit's own stored prose: this pass parses, it never
    /// interprets.
    pub verbatim: String,
    pub parsed: KretzmannDateParsed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KretzmannChronologyExport {
    pub format_version: u32,
    pub atlas_version_root: String,
    /// A TENTATIVE, mechanically-parsed extraction: this export carries no placement authority of its
    /// own.
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

/// Every dating clause in every `CommentaryItem` node's stored prose, in node-id order then discovery
/// order within one unit. `target` is resolved from that unit's own `comments_on` row, and a unit with
/// no such row is skipped rather than having a target fabricated for it.
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
        let node = Node { id: EventId::new(event_id.to_string()).erase(), payload: NodePayload::Event { label: "Creation".to_string(), kind: atlas_core::data::EventKind::Event.name().to_string(), verses: vec![], witnesses: vec![], robertson_section: None, acts_section: None, atlas_section: None, kjv_superscription: None, ref_note: None }, provenance: "theographic".to_string() };
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
