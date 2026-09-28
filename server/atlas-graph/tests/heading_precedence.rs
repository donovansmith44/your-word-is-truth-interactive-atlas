use std::collections::BTreeSet;
use std::path::Path;

use atlas_graph::build::build_graph_from_sources_with_eras;
use atlas_graph::heading::build_heading_index;
use atlas_graph_types::id::NodeKind;

fn real_graph() -> (atlas_graph_types::graph::Graph, std::collections::HashMap<String, atlas_graph_types::chrono::ResolvedPlacement>) {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");

    let kjv_json = std::fs::read_to_string(raw_dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let xrefs_tsv = std::fs::read_to_string(raw_dir.join("xrefs/cross_references.txt")).expect("data/raw/xrefs/cross_references.txt must exist");
    let out = atlas_etl::compile::compile(&raw_dir, &curated_dir).expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify");
    let atlas = out.data;
    let eras = atlas.eras.clone();

    let (graph, _stats, _ews, chrono) = build_graph_from_sources_with_eras(&kjv_json, &xrefs_tsv, &atlas, &eras).expect("the real committed sources must build");
    (graph, chrono.resolved)
}

#[test]
fn named_case_jhn_12_1_the_real_container_beats_the_freebie() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let heading = index.get("JHN.12.1").expect("JHN.12.1 must anchor SOME heading");
    assert_eq!(heading.event_id, "pw_bethany", "the real, witness-bearing container must win over the bare narrative-leg freebie (jm_bethany)");
    assert_ne!(heading.event_id, "jm_bethany", "the freebie must not win the collision");
}

#[test]
fn named_case_psa_53_1_the_shared_fool_incipit_container_anchors_both_psalms() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let psa53 = index.get("PSA.53.1").expect("PSA.53.1 must anchor a heading -- psa_014's own second (parallel) witness");
    assert_eq!(psa53.event_id, "psa_014");
    assert_eq!(psa53.title, "The fool hath said in his heart, There is no God.");
    assert_eq!(psa53.kind, atlas_core::data::EventKind::General);

    let psa14 = index.get("PSA.14.1").expect("PSA.14.1 must anchor a heading -- psa_014's own first (self) witness");
    assert_eq!(psa14.event_id, "psa_014");
    assert_eq!(psa14.title, psa53.title, "one shared container, one shared title, true at both anchors (fix round 1, batch-w3-review.md Important-1)");
}

#[test]
fn named_case_gen_6_1_anchors_canonically_first_not_curated_import_order() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let heading = index.get("GEN.6.1").expect("GEN.6.1 must anchor a heading -- theo-32's own canonically first covered verse (M-D1 req 1, owner live report #2)");
    assert_eq!(heading.event_id, "theo-32");
    assert!(!heading.is_continuation, "GEN.6.1 is theo-32's own TRUE first-covered verse -- a PRIMARY anchor, not a continuation");

    if let Some(still_there) = index.get("GEN.6.7") {
        assert_ne!(still_there.event_id, "theo-32", "theo-32's own anchor must have moved off GEN.6.7 to the canonically first verse, GEN.6.1");
    }
}

#[test]
fn named_case_ezr_temple_completed_spans_chapters_5_and_6_with_a_continuation_heading_at_6_1() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);

    let primary = index.get("EZR.5.1").expect("EZR.5.1 must anchor ezr_temple_completed's own PRIMARY heading");
    assert_eq!(primary.event_id, "ezr_temple_completed");
    assert!(!primary.is_continuation, "the container's own true first-covered verse is a PRIMARY anchor");

    let continuation = index.get("EZR.6.1").expect("EZR.6.1 -- a covered chapter's own opening verse, mid-container -- must render SOME heading (M-D1 req 1: 'no covered chapter may open with unlabeled verses')");
    assert_eq!(continuation.event_id, "ezr_temple_completed", "the SAME container continues at the chapter boundary");
    assert_eq!(continuation.title, primary.title, "one container, one title, true at both the anchor and its own continuation");
    assert!(continuation.is_continuation, "EZR.6.1 is NOT ezr_temple_completed's own true first verse -- it must render as a CONTINUATION, not a second primary");
}

#[test]
fn m_d1_every_chapter_with_heading_worthy_coverage_opens_with_a_real_heading() {
    let (graph, resolved) = real_graph();
    let index = build_heading_index(&graph, &resolved);
    let narrative_legs = atlas_graph::heading::narrative_leg_event_ids(&graph);

    let mut worthy_chapters: BTreeSet<(String, u16)> = BTreeSet::new();
    for (id, node) in &graph.nodes {
        if id.kind != NodeKind::Event {
            continue;
        }
        let atlas_graph_types::node::NodePayload::Event { verses, witnesses, robertson_section, acts_section, atlas_section, kjv_superscription, .. } =
            &node.payload
        else {
            continue;
        };
        let is_real_container =
            !witnesses.is_empty() || robertson_section.is_some() || acts_section.is_some() || atlas_section.is_some() || kjv_superscription.is_some();
        if !(narrative_legs.contains(&id.raw) || is_real_container) {
            continue;
        }

        let mut note = |v: &str| {
            if let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) {
                if vid.verse == 1 {
                    worthy_chapters.insert((vid.book.code().to_string(), vid.chapter));
                }
            }
        };
        if !witnesses.is_empty() {
            for w in witnesses {
                if let Some(vs) = w.translations.get(atlas_graph::kjv_adapter::KJV_TRANSLATION) {
                    for v in vs {
                        note(v);
                    }
                }
            }
        } else {
            for v in verses {
                note(v);
            }
        }
    }
    assert!(worthy_chapters.len() >= 500, "expected a substantial set of heading-worthy chapters (all 66 books), got {}", worthy_chapters.len());

    let missing: Vec<String> = worthy_chapters.iter().map(|(b, c)| format!("{b}.{c}.1")).filter(|v| !index.contains_key(v)).collect();
    assert!(
        missing.is_empty(),
        "every chapter touched by heading-worthy coverage must open with a real heading (primary or continuation) -- {} chapter(s) open unlabeled: {:?}",
        missing.len(),
        missing
    );
}
