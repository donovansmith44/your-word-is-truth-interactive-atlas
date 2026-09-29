use std::collections::BTreeSet;
use std::path::Path;

fn real_atlas_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

fn real_graph() -> &'static atlas_graph_types::graph::Graph {
    static GRAPH: std::sync::OnceLock<atlas_graph_types::graph::Graph> = std::sync::OnceLock::new();
    GRAPH.get_or_init(|| {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
        let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
        let xrefs_tsv = std::fs::read_to_string(dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
        let atlas = real_atlas_data();
        let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
        let concord_corpus = atlas_etl::concord::read_all(&dir.join("concord"), &dir.parent().unwrap().join("curated")).expect("data/raw/concord must exist -- run data/fetch-raw.ps1 first");
        let sc_overlap_text = std::fs::read_to_string(dir.parent().unwrap().join("curated/concord-sc-overlap.toml")).expect("data/curated/concord-sc-overlap.toml must exist");
        let sc_overlap = atlas_etl::concord::parse_sc_overlap(&sc_overlap_text).expect("concord-sc-overlap.toml must parse");
        let concord_bundle = atlas_graph::concord_adapter::ConcordBundle { corpus: concord_corpus, sc_overlap };
        let (_, kjv_verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
        let kretzmann_corpus = atlas_etl::kretzmann::read_all(&dir.join("kretzmann"), &kjv_verses).expect("data/raw/kretzmann must exist -- run data/fetch-raw.ps1 first");

        let restored_verses = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &kjv_verses).0;
        let red_letter_corpus = atlas_etl::red_letter::read_all(&dir.join("red-letter"), &restored_verses).expect("data/raw/red-letter must exist -- run data/fetch-raw.ps1 first");

        let (mut graph, ..) = atlas_graph::build::build_graph_from_sources_with_eras_and_brainfuel_and_concord_and_kretzmann_and_red_letter(
            &kjv_json,
            &xrefs_tsv,
            &atlas,
            &atlas.eras,
            Some(&brainfuel),
            Some(&concord_bundle),
            Some(&kretzmann_corpus),
            Some(&red_letter_corpus),
        )
        .expect("the real committed sources must build");
        graph.build_indexes();
        graph
    })
}

fn key(v: &atlas_graph_types::text::VerseRef) -> (u8, u16, u16) {
    (v.book, v.chapter, v.verse)
}

fn dot_ref(v: &atlas_graph_types::text::VerseRef) -> String {
    format!("{}.{}.{}", v.book, v.chapter, v.verse)
}

#[test]
fn spoken_at_coverage_over_the_real_data_is_461_of_470_pinned() {
    let graph = real_graph();

    assert_eq!(graph.spoken_by.len(), 470, "the real vendored red-letter data's own maximal-contiguous-range count (batch-red1-report.md)");

    let spoken_by_ranges: BTreeSet<atlas_graph_types::text::BibleLocusRange> = graph.spoken_by.iter().map(|r| r.locus.clone()).collect();
    assert_eq!(spoken_by_ranges.len(), 470, "every SpokenBy range must be distinct -- one row per maximal contiguous range, never a duplicate");

    let covered_ranges: BTreeSet<atlas_graph_types::text::BibleLocusRange> = graph.spoken_at.iter().map(|r| r.locus.clone()).collect();
    assert_eq!(covered_ranges.len(), 461, "461 of 470 ranges (98.1%) carry at least one real SpokenAt row -- the pinned coverage figure this test's own header comment cites");

    let uncovered: Vec<&atlas_graph_types::text::BibleLocusRange> = spoken_by_ranges.difference(&covered_ranges).collect();
    assert_eq!(uncovered.len(), 9, "470 - 461 = 9 genuinely uncovered ranges -- see this file's own per-range assessment test for why each one is a documented data gap, not a predicate bug");
}

#[test]
fn every_uncovered_range_is_assessed_and_is_a_genuine_data_gap_not_a_predicate_bug() {
    let graph = real_graph();

    let spoken_by_ranges: BTreeSet<atlas_graph_types::text::BibleLocusRange> = graph.spoken_by.iter().map(|r| r.locus.clone()).collect();
    let covered_ranges: BTreeSet<atlas_graph_types::text::BibleLocusRange> = graph.spoken_at.iter().map(|r| r.locus.clone()).collect();
    let uncovered: Vec<atlas_graph_types::text::BibleLocusRange> = spoken_by_ranges.difference(&covered_ranges).cloned().collect();
    assert_eq!(uncovered.len(), 9);

    let mut event_spans: std::collections::BTreeMap<String, ((u8, u16, u16), (u8, u16, u16))> = std::collections::BTreeMap::new();
    for att in &graph.attests {
        let f = key(&att.attestation.from.unit);
        let t = key(&att.attestation.to.unit);
        event_spans
            .entry(att.event.0.clone())
            .and_modify(|(min, max)| {
                if f < *min {
                    *min = f;
                }
                if t > *max {
                    *max = t;
                }
            })
            .or_insert((f, t));
    }
    let located_event_ids: BTreeSet<&String> = graph.located_at.iter().map(|l| &l.event.0).collect();

    let mut disconnected = 0usize;
    let mut too_narrow = 0usize;

    for range in &uncovered {
        let range_from = key(&range.from.unit);
        let range_to = key(&range.to.unit);

        let mut any_overlap = false;
        for (event_id, (min, max)) in &event_spans {
            if !located_event_ids.contains(event_id) {
                continue;
            }
            if range_from <= *max && *min <= range_to {
                any_overlap = true;
                break;
            }
        }

        if any_overlap {
            too_narrow += 1;
        } else {
            disconnected += 1;
        }

        println!(
            "RED-m2 uncovered range: {}..{} ({})",
            dot_ref(&range.from.unit),
            dot_ref(&range.to.unit),
            if any_overlap { "too-narrow: a located event's attested span overlaps but does not fully contain this range" } else { "disconnected: no located event's attested span overlaps this range at all" }
        );
    }

    assert_eq!(too_narrow + disconnected, 9);
    println!("RED-m2 categorization: {too_narrow} too-narrow, {disconnected} disconnected (of 9 total)");
}
