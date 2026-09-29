mod common;

use common::{committed_sections, committed_service};

use atlas_core::refs::ScriptureRef;
use atlas_graph::sqlite::serve::*;

#[test]
fn the_chronology_loaded_from_event_date_is_the_artifacts() {
    let a = &committed_service().chronology.chrono;
    let s = committed_sections().with_conn(load_chronology).unwrap();
    assert_eq!(s.order, a.order, "order = event_id ORDER BY seq");
    assert_eq!(s.resolved, a.resolved);
    assert_eq!(s.source_meta, a.source_meta, "the curated to_year/order_key (the Event wire's)");
    assert!(!s.source_meta.is_empty());
}

/// The two flags `load_heading_index` reads back, pinned to real rows: comparing the section read
/// against `committed_service()` cannot see a mistake here, because that service reads the same rows through
/// the same function.
const A_CONTINUED_HEADING: (&str, &str, bool) = ("1CH.16.1", "sam2_ark_to_jerusalem", true);
const AN_OPENING_HEADING: (&str, &str, bool) = ("1CH.1.1", "1ch_genealogy_patriarchs_to_edom", false);

#[test]
fn the_heading_index_says_which_of_its_rows_continue_earlier_coverage_and_which_open_it() {
    // Arrange
    let expected = [A_CONTINUED_HEADING, AN_OPENING_HEADING];

    // Act
    let index = committed_sections().with_conn(load_heading_index).unwrap();

    // Assert
    assert_eq!(
        expected.map(|(verse, _, _)| (index[verse].event_id.as_str(), index[verse].is_continuation)),
        expected.map(|(_, event_id, is_continuation)| (event_id, is_continuation))
    );
}

#[test]
fn the_heading_index_red_letter_spans_and_narrative_legs_are_the_artifacts() {
    let a = committed_service();
    let (h, r, l) = committed_sections().with_conn(|c| Ok((load_heading_index(c)?, load_red_letter_spans(c)?, load_narrative_legs(c)?))).unwrap();
    assert_eq!(h, a.heading_index);
    assert_eq!(r, a.red_letter_spans);
    assert_eq!(l, a.narrative_legs);
    assert!(h.len() > 100 && r.len() > 1000 && l.len() == 13, "{} {} {}", h.len(), r.len(), l.len());
}

/// The families are swept, not derived, so `committed_service()` cannot witness them: it reads the same
/// rows through the same function. Pinned to the one family every section has.
const THE_NODE_FAMILY: &str = "nodes";

#[test]
fn the_provenance_sweep_reads_back_at_least_the_node_family_with_real_provenance_ids() {
    // Arrange
    let snap = committed_sections();

    // Act
    let families = snap.with_conn(|c| load_provenance_families(c, snap.present())).unwrap();

    // Assert
    assert!(
        families.len() > 1 && families[THE_NODE_FAMILY].iter().all(|id| !id.is_empty()) && !families[THE_NODE_FAMILY].is_empty(),
        "{:?}",
        families.keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_provenance_families_are_the_artifacts() {
    let a = committed_service();
    let snap = committed_sections();
    let fams = snap.with_conn(|c| load_provenance_families(c, snap.present())).unwrap();
    let mut names: Vec<&str> = fams.keys().copied().collect();
    names.sort();
    let mut expected = a.provenance.families();
    expected.sort();
    assert_eq!(names, expected, "the same 26 families");
    for name in expected {
        let got: Vec<String> = fams[name].iter().cloned().collect();
        assert_eq!(got, a.provenance.by_family(name), "{name}");
    }
}

#[test]
fn the_boot_counters_are_the_artifacts_except_the_one_only_the_compile_knows() {
    let a = committed_service();
    let snap = committed_sections();
    let (stats, ews) = snap.with_conn(|c| load_counters(c, snap.present())).unwrap();
    assert_eq!(stats.kjv_verses, a.stats.kjv_verses);
    assert_eq!(stats.cites_rows, a.stats.cites_rows);
    assert_eq!(stats.cites_dropped_negative_votes, 0, "not derivable from the tables (disclosed)");
    assert_eq!(ews, a.event_world_stats);
}

#[test]
fn cross_refs_for_span_is_the_companions_slice() {
    let a = committed_service();
    let snap = committed_sections();
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
        assert_eq!(got, a.cross_refs_for_span(span), "{span}");
    }
    let one = snap.with_conn(|c| cross_refs_for_span(c, &spans[0])).unwrap();
    assert!(one["JHN.3.16"].len() > 10, "JHN.3.16 has many cross-refs: {}", one["JHN.3.16"].len());
    assert_eq!(one.len(), 1);
}

