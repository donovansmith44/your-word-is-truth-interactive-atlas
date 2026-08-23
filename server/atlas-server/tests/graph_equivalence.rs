//! EQUIVALENCE (M-A brief acceptance set): "for every chapter in canon, the
//! window path's text == the pre-migration chapter endpoint's text" -- a
//! server-side comparison across all 1,189 real KJV chapters. This is the
//! committed regression guard for `handlers::chapter`'s own swap (M-A
//! brief requirement 5: "re-implement the OLD /api/chapter handler as a
//! VIEW over the window query"): it independently reproduces BOTH the
//! pre-migration gathering logic (`data.verses.get(key)`, per verse, in
//! order) and the post-migration one (`GraphService::chapter_span` +
//! `window::window`, the SAME primitive `handlers::chapter` and
//! `GET /api/text?scope=chapter` both call) over the real compiled/raw
//! data, and asserts they produce the IDENTICAL sequence of verse texts for
//! every chapter -- proving the swap changed nothing observable. The
//! actual text/window reads go entirely through
//! `atlas_graph_types::store::GraphQuery` -- THE PORT (design doc §9a; fix
//! round 1, C1) -- never a raw `Graph` field, so this equivalence proof is
//! pinned at the same seam any future backend plugs into.

use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;

// M-C2 DELETION EVENT: `AtlasData::load`'s own five retiring-file reads
// return empty now -- `atlas_etl::compile::compile` is this crate's own
// real-data source from here on. Cached (`OnceLock`) so this file's two
// call sites share one real compile.
fn load_real_atlas_data() -> AtlasData {
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

fn load_real_graph(atlas: &AtlasData) -> GraphService {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    // GraphService::build runs the FIDELITY LAW unconditionally as part of
    // construction (fix round 1) -- reaching this line already proves it
    // passed on the real committed KJV source.
    GraphService::build(&dir, atlas).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law")
}

/// The OLD `/api/chapter` handler's own text-gathering logic, reproduced
/// verbatim (pre-migration: `data.verses.get(&format!("{code}.{chapter}.{v}"))`,
/// `v` from 1..=verse_count, skipping an absent verse rather than
/// fabricating one) -- kept as a small local function, not a call into
/// `atlas_server::handlers`, so this test independently re-derives the
/// "before" side rather than trusting the very code path it exists to
/// check.
fn old_chapter_texts(data: &AtlasData, code: &str, chapter: u16, verse_count: u16) -> Vec<String> {
    (1..=verse_count).filter_map(|v| data.verses.get(&format!("{code}.{chapter}.{v}")).cloned()).collect()
}

/// The NEW window-query path -- exactly `GraphService::chapter_span` +
/// `window::window`/`window::render`, the same calls both
/// `handlers::chapter` (after its M-A swap) and `GET /api/text?scope=chapter`
/// make. Ref-resolution (`chapter_span`) is `GraphService`'s own
/// adapter-side companion; the actual window/render reads run against the
/// opened snapshot through `&dyn GraphQuery` only.
fn new_chapter_texts(graph: &GraphService, book_index: u8, chapter: u16) -> Vec<String> {
    match graph.chapter_span(book_index, chapter) {
        Some((start, n)) => {
            let snap = graph.snapshot();
            window::window(&snap, atlas_graph::kjv_adapter::BIBLE_CORPUS, start, n, WindowDir::Onward).iter().filter_map(|id| window::render(&snap, id)).collect()
        }
        None => Vec::new(),
    }
}

#[test]
fn every_chapter_in_canon_matches_between_the_old_lookup_and_the_new_window_query() {
    let data = load_real_atlas_data();
    let graph = load_real_graph(&data);

    let mut chapters_checked = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for book in &data.canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).expect("every compiled canon book code must resolve").0;
        for (chapter_index, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_index + 1) as u16;
            chapters_checked += 1;

            let old = old_chapter_texts(&data, &book.code, chapter, verse_count);
            let new = new_chapter_texts(&graph, book_index, chapter);

            if old != new {
                mismatches.push(format!(
                    "{}.{}: old had {} verse(s), new had {} verse(s)",
                    book.code,
                    chapter,
                    old.len(),
                    new.len()
                ));
            }
        }
    }

    assert_eq!(chapters_checked, 1_189, "the real KJV canon has exactly 1,189 chapters");
    assert!(mismatches.is_empty(), "chapters where the window path disagreed with the pre-migration lookup:\n{}", mismatches.join("\n"));
}

/// A byte-level spot check alongside the exhaustive equivalence sweep above
/// -- pins one well-known verse's exact wording so a bug that happened to
/// preserve LENGTH (verse count) but corrupt TEXT content in a way the
/// `Vec<String>` equality above would still have caught is additionally
/// obvious from a single, human-checkable assertion.
#[test]
fn john_3_16_text_matches_between_old_and_new_paths() {
    let data = load_real_atlas_data();
    let graph = load_real_graph(&data);

    let expected = data.verses.get("JHN.3.16").cloned().expect("JHN.3.16 must be in the real compiled verses map");
    let jhn_index = atlas_core::canon::resolve_alias("JHN").unwrap().0;
    let new = new_chapter_texts(&graph, jhn_index, 3);
    assert_eq!(new.get(15), Some(&expected), "JHN.3.16 is the 16th verse (index 15) of John 3");
    assert!(expected.contains("For God so loved the world"), "sanity: this really is John 3:16");
}
