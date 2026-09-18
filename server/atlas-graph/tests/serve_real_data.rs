//! DB-4c: every serving companion loaded from the section tables equals
//! the artifact path's value, on the REAL data -- the equivalence gates
//! that let `GraphService::from_sections` replace `from_artifact`.
use std::path::Path;
use std::sync::OnceLock;

use atlas_core::refs::ScriptureRef;
use atlas_graph::service::GraphService;
use atlas_graph::sqlite::serve::*;
use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout};

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")
}

/// DB-5: the reference is the served path itself (`from_sections`); the
/// loaders below are what it runs, so each law now says "loaded through the
/// service and loaded directly, equal" plus the shape checks on the values.
fn artifact() -> &'static GraphService {
    static CACHED: OnceLock<GraphService> = OnceLock::new();
    CACHED.get_or_init(|| GraphService::from_sections(&data_dir()).expect("the sections open").0)
}

fn sections() -> &'static SqliteSnapshot {
    static CACHED: OnceLock<SqliteSnapshot> = OnceLock::new();
    CACHED.get_or_init(|| {
        let layout = SectionLayout::under(&data_dir());
        SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout }).expect("the committed sections open")
    })
}

#[test]
fn the_chronology_loaded_from_event_date_is_the_artifacts() {
    let a = &artifact().chronology.chrono;
    let s = sections().with_conn(load_chronology).unwrap();
    assert_eq!(s.order, a.order, "order = event_id ORDER BY seq");
    assert_eq!(s.resolved, a.resolved);
    assert_eq!(s.source_meta, a.source_meta, "the curated to_year/order_key (the Event wire's)");
    assert!(!s.source_meta.is_empty());
}

#[test]
fn the_heading_index_red_letter_spans_and_narrative_legs_are_the_artifacts() {
    let a = artifact();
    let (h, r, l) = sections().with_conn(|c| Ok((load_heading_index(c)?, load_red_letter_spans(c)?, load_narrative_legs(c)?))).unwrap();
    assert_eq!(h, a.heading_index);
    assert_eq!(r, a.red_letter_spans);
    assert_eq!(l, a.narrative_legs);
    assert!(h.len() > 100 && r.len() > 1000 && l.len() == 13, "{} {} {}", h.len(), r.len(), l.len());
}

#[test]
fn the_provenance_families_are_the_artifacts() {
    let a = artifact();
    let snap = sections();
    let fams = snap.with_conn(|c| load_provenance_families(c, snap.present())).unwrap();
    let mut names: Vec<&str> = fams.keys().copied().collect();
    names.sort();
    let mut expected = a.provenance.families();
    expected.sort();
    assert_eq!(names, expected, "the same 22 families");
    for name in expected {
        let got: Vec<String> = fams[name].iter().cloned().collect();
        assert_eq!(got, a.provenance.by_family(name), "{name}");
    }
}

#[test]
fn the_boot_counters_are_the_artifacts_except_the_one_only_the_compile_knows() {
    let a = artifact();
    let snap = sections();
    let (stats, ews) = snap.with_conn(|c| load_counters(c, snap.present())).unwrap();
    assert_eq!(stats.kjv_verses, a.stats.kjv_verses);
    assert_eq!(stats.cites_rows, a.stats.cites_rows);
    assert_eq!(stats.cites_dropped_negative_votes, 0, "not derivable from the tables (disclosed)");
    assert_eq!(ews, a.event_world_stats);
}

#[test]
fn cross_refs_for_span_is_the_companions_slice() {
    let a = artifact();
    let snap = sections();
    let spans = [
        ScriptureRef::parse("JHN.3.16").unwrap(),
        ScriptureRef::parse("JHN.3").unwrap(),
        ScriptureRef::parse("GEN.1.1-5").unwrap(),
        ScriptureRef::parse("JUD").unwrap(),
        ScriptureRef::parse("PSA.119.1-176").unwrap(),
    ];
    for span in &spans {
        let got = snap.with_conn(|c| cross_refs_for_span(c, span)).unwrap();
        assert!(!got.is_empty(), "{span}");
        // the artifact path answers through its Mem arm (the retained map, filtered to the span)
        assert_eq!(got, a.cross_refs_for_span(span), "{span}");
    }
    let one = snap.with_conn(|c| cross_refs_for_span(c, &spans[0])).unwrap();
    assert!(one["JHN.3.16"].len() > 10, "JHN.3.16 has many cross-refs: {}", one["JHN.3.16"].len());
    assert_eq!(one.len(), 1);
}
