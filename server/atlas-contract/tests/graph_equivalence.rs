use std::path::Path;

use atlas_core::data::AtlasData;
use atlas_graph::window::{self, WindowDir};
use atlas_graph::GraphService;

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
    GraphService::build(&dir, atlas).expect("data/raw/{kjv.json,xrefs/cross_references.txt} must exist and satisfy the fidelity law")
}

fn real_restored_verses(data: &AtlasData) -> std::collections::HashMap<String, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step first");
    atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &data.verses).0
}

fn old_chapter_texts(verses: &std::collections::HashMap<String, String>, code: &str, chapter: u16, verse_count: u16) -> Vec<String> {
    (1..=verse_count).filter_map(|v| verses.get(&format!("{code}.{chapter}.{v}")).cloned()).collect()
}

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
    let restored_verses = real_restored_verses(&data);

    let mut chapters_checked = 0usize;
    let mut mismatches: Vec<String> = Vec::new();

    for book in &data.canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).expect("every compiled canon book code must resolve").0;
        for (chapter_index, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_index + 1) as u16;
            chapters_checked += 1;

            let old = old_chapter_texts(&restored_verses, &book.code, chapter, verse_count);
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

#[test]
fn john_3_16_text_matches_between_old_and_new_paths() {
    let data = load_real_atlas_data();
    let graph = load_real_graph(&data);
    let restored_verses = real_restored_verses(&data);

    let expected = restored_verses.get("JHN.3.16").cloned().expect("JHN.3.16 must be in the real compiled verses map");
    let jhn_index = atlas_core::canon::resolve_alias("JHN").unwrap().0;
    let new = new_chapter_texts(&graph, jhn_index, 3);
    assert_eq!(new.get(15), Some(&expected), "JHN.3.16 is the 16th verse (index 15) of John 3");
    assert!(expected.contains("For God so loved the world"), "sanity: this really is John 3:16");
}
