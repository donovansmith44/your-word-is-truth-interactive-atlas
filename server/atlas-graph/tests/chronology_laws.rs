use std::collections::{HashMap, HashSet};
use std::path::Path;

use atlas_core::chronology::{anchor_deferral, is_exempted, is_recounting};
use atlas_core::data::AtlasData;
use atlas_graph::build::build_graph_from_sources;
use atlas_graph::event_world::{self};
use atlas_graph::Chronology;
use atlas_graph_types::chrono::temporal_order;
use atlas_graph_types::edge::Attests;

fn load_real_atlas() -> AtlasData {
    static CACHED: std::sync::OnceLock<AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

struct RealGraph {
    atlas: AtlasData,
    graph: atlas_graph_types::graph::Graph,
    resolved: HashMap<String, atlas_graph_types::chrono::ResolvedPlacement>,
}

fn build_real() -> RealGraph {
    let atlas = load_real_atlas();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let (graph, _kjv_stats, _ew_stats, chrono) = build_graph_from_sources(&kjv_json, &xrefs_tsv, &atlas).expect("the real graph must build");

    let anchor_years = Chronology::anchor_years(&atlas);
    let event_years = Chronology::event_years(&atlas);
    let mut resolved = HashMap::new();
    for row in &graph.dated_by {
        let id = row.event.0.clone();
        let tp = event_world::resolve_timepoint(&row.placement, &anchor_years, &event_years).expect("every dated_by row must resolve");
        let seq = chrono.resolved[&id].seq;
        resolved.insert(
            id,
            atlas_graph_types::chrono::ResolvedPlacement {
                date: atlas_graph_types::chrono::ResolvedDate { from: tp, to: tp },
                seq,
                basis: row.basis,
            },
        );
    }
    RealGraph { atlas, graph, resolved }
}

fn graph_witness_books<'a>(attests: &[Attests], event_id: &str) -> HashSet<&'a str> {
    let mut out = HashSet::new();
    for row in attests.iter().filter(|r| r.event.0 == event_id) {
        let book_idx = row.attestation.from.unit.book as usize;
        let chapter = row.attestation.from.unit.chapter;
        let Some(meta) = atlas_core::canon::BOOKS.get(book_idx) else { continue };
        let code = meta.code;
        if is_recounting(code, chapter) || is_exempted(event_id, code) {
            continue;
        }
        out.insert(code);
    }
    out
}

fn window_for<'a>(atlas: &'a AtlasData, book: &str) -> Option<&'a atlas_core::data::BookNarrationWindow> {
    atlas.book_narration_windows.iter().find(|w| w.book == book)
}

#[test]
fn e1_every_bound_anchor_equals_its_graphs_own_resolved_event_year() {
    let rg = build_real();
    let bound: Vec<_> = rg.atlas.chronology_anchors.iter().filter(|a| a.event_id.is_some()).collect();
    assert!(bound.len() >= 15, "expected the real curated anchor table to bind well over 15 rows to real events, got {}", bound.len());

    let mut violations = Vec::new();
    let mut deferred = Vec::new();
    for a in &bound {
        let eid = a.event_id.as_deref().unwrap();
        let Some(resolved) = rg.resolved.get(eid) else { continue };
        let graph_year = resolved.date.from.year.get();

        if let Some(def) = anchor_deferral(&a.id) {
            if graph_year != def.shipped_value {
                violations.push(format!("anchor '{}': STALE DEFERRAL -- ANCHOR_DEFERRALS records {}, graph resolves '{}' to {}", a.id, def.shipped_value, eid, graph_year));
            } else {
                deferred.push(a.id.clone());
            }
            continue;
        }

        if graph_year != a.year {
            violations.push(format!("anchor '{}': table year {} != graph-resolved year {} for event '{}'", a.id, a.year, graph_year, eid));
        }
    }

    assert_eq!(deferred.len(), 4, "expected exactly 4 typed deferrals (same as the atlas-core original), got {}: {:?}", deferred.len(), deferred);
    assert!(violations.is_empty(), "E1 (graph) anchor-equality violated for {} row(s):\n{}", violations.len(), violations.join("\n"));
}

#[test]
fn e2_every_dated_event_adheres_to_its_graph_attested_books_own_narration_window() {
    let rg = build_real();
    assert!(!rg.atlas.book_narration_windows.is_empty(), "the real compiled data must carry real narration windows");

    let mut violations = Vec::new();
    for (event_id, placement) in &rg.resolved {
        let year = placement.date.from.year.get();
        let mut books: Vec<&str> = graph_witness_books(&rg.graph.attests, event_id).into_iter().collect();
        books.sort_unstable();
        for book in books {
            let Some(w) = window_for(&rg.atlas, book) else { continue };
            if year < w.from_year || year > w.to_year {
                violations.push(format!("'{event_id}': year {year} outside '{book}''s own narration window {}..{}", w.from_year, w.to_year));
            }
        }
    }

    assert!(violations.is_empty(), "E2 (graph) window-adherence violated for {} event/book pair(s):\n{}", violations.len(), violations.join("\n"));
}

#[test]
fn e3_bound_anchors_sorted_by_table_year_are_monotone_under_the_graphs_own_temporal_order() {
    let rg = build_real();
    let mut bound: Vec<_> = rg.atlas.chronology_anchors.iter().filter(|a| a.event_id.is_some() && anchor_deferral(&a.id).is_none()).collect();
    assert!(bound.len() >= 12, "expected well over 12 non-deferred bound anchors, got {}", bound.len());
    bound.sort_by_key(|a| a.year);

    let mut violations = Vec::new();
    for pair in bound.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (a_eid, b_eid) = (a.event_id.as_deref().unwrap(), b.event_id.as_deref().unwrap());
        let a_p = &rg.resolved[a_eid];
        let b_p = &rg.resolved[b_eid];
        let ord = temporal_order(a_p, b_p);
        let ok = if a.year == b.year { ord != std::cmp::Ordering::Greater } else { ord == std::cmp::Ordering::Less };
        if !ok {
            violations.push(format!(
                "'{}' (table year {}) does not sort {} '{}' (table year {}) under the graph's own temporal_order",
                a.id,
                a.year,
                if a.year == b.year { "at-or-before" } else { "strictly before" },
                b.id,
                b.year
            ));
        }
    }
    assert!(violations.is_empty(), "E3 (graph) canonical-order violated for {} adjacent pair(s):\n{}", violations.len(), violations.join("\n"));
}

#[test]
fn e4_dated_events_agree_with_era_boundary_anchors_under_the_graphs_own_temporal_order() {
    let rg = build_real();
    let boundaries: Vec<_> = rg.atlas.chronology_anchors.iter().filter(|a| a.era_boundary).collect();
    assert!(boundaries.len() >= 6, "expected the real curated table to carry well over 6 era_boundary anchors, got {}", boundaries.len());

    let mut violations = Vec::new();
    for b in &boundaries {
        let b_eid = b.event_id.as_deref().expect("an era_boundary anchor always binds an event, per the table's own schema");
        let Some(b_p) = rg.resolved.get(b_eid) else { continue };

        for (event_id, e_p) in &rg.resolved {
            if event_id == b_eid {
                continue;
            }
            let mut books: Vec<&str> = graph_witness_books(&rg.graph.attests, event_id).into_iter().collect();
            books.sort_unstable();
            if books.is_empty() {
                continue;
            }
            let mut all_before = true;
            let mut all_after = true;
            for book in &books {
                match window_for(&rg.atlas, book) {
                    Some(w) => {
                        if w.to_year > b.year {
                            all_before = false;
                        }
                        if w.from_year <= b.year {
                            all_after = false;
                        }
                    }
                    None => {
                        all_before = false;
                        all_after = false;
                    }
                }
            }

            let side = if all_before && !all_after {
                "before"
            } else if all_after && !all_before {
                "after"
            } else {
                continue;
            };

            let ord = temporal_order(e_p, b_p);
            let violates = match side {
                "before" => ord == std::cmp::Ordering::Greater,
                _ => ord != std::cmp::Ordering::Greater,
            };
            if violates {
                violations.push(format!("'{event_id}' sorts on the wrong side of boundary '{}' (year {}) under the graph's own temporal_order -- expected {side}", b.id, b.year));
            }
        }
    }

    assert!(violations.is_empty(), "E4 (graph) era-partition violated for {} event(s):\n{}", violations.len(), violations.join("\n"));
}

#[test]
fn theo_87_nimrods_kingdom_resolves_to_the_corrected_traditional_year() {
    let rg = build_real();
    let resolved = rg.resolved.get("theo-87").expect("theo-87 is a real, dated compiled event");
    assert_eq!(
        resolved.date.from.year.get(),
        -2242,
        "theo-87 ('Nimrod's kingdom begins') must resolve to -2242 (THEO_DATE_OVERRIDES) -- the raw Theographic import's anachronistic -1822 must not resurface"
    );
}

#[test]
fn the_chronology_order_is_total_and_carried_by_seq() {
    let atlas = load_real_atlas();
    let chrono = event_world::derive_chronology(&atlas);
    assert!(chrono.order.len() > 800, "hundreds of dated events: {}", chrono.order.len());
    let mut years: Vec<i32> = Vec::with_capacity(chrono.order.len());
    let mut seqs: Vec<u32> = Vec::with_capacity(chrono.order.len());
    for (i, id) in chrono.order.iter().enumerate() {
        let r = chrono.resolved.get(id).expect("every ordered event resolves");
        assert_eq!(r.seq.0 as usize, i, "seq is the position in order ({id})");
        years.push(r.date.from.year.get());
        seqs.push(r.seq.0);
    }
    assert!(years.windows(2).all(|w| w[0] <= w[1]), "order is from_year-monotone");
    let mut distinct = seqs.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(distinct.len(), seqs.len(), "seq is a total order: every position once");
    assert_eq!(distinct, (0..seqs.len() as u32).collect::<Vec<_>>(), "and dense: 0..n");
}
