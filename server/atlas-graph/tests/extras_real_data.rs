//! DB-4b: the nine folded sidecars over the REAL `data/compiled` files:
//! every loaded struct's every field lands in a table (lossless -- DB-5
//! deletes the JSONs), row counts match the sources, the fold is
//! deterministic, and a fixture directory without sidecars folds nothing.
use std::path::Path;

use atlas_graph::sqlite::extras::{table_specs_of, Col, Extras};
use atlas_graph::sqlite::sidecars::{fold_sidecars, Sidecars};
use atlas_graph::sqlite::snapshot::SqliteSnapshot;
use atlas_graph::sqlite::source::{CommittedZstdSource, SectionLayout};
use atlas_graph_types::sections::Section;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")
}

#[test]
fn the_real_sidecars_fold_losslessly_into_twenty_one_tables() {
    let sc = Sidecars::load(&data_dir()).unwrap().expect("data/compiled has canon.json");
    let tables = fold_sidecars(&sc.atlas, &sc.sources).unwrap();
    let mut ex = Extras::default();
    ex.extend(tables);
    let names: Vec<&str> = ex.tables.iter().map(|t| t.spec.name).collect();
    let expected: Vec<&str> = table_specs_of(Section::Core).iter().map(|s| s.name).skip(5).collect();
    assert_eq!(names, expected, "the fold produces exactly the sidecar tables, in extra_tables_of order");
    let n = |t: &str| ex.table(t).unwrap().rows.len();
    assert_eq!(n("canon_book"), 66);
    assert_eq!(n("canon_chapter_verses"), sc.atlas.canon.books.iter().map(|b| b.chapters.len()).sum::<usize>());
    assert_eq!(n("book_meta"), sc.atlas.books_meta.len());
    assert_eq!(n("chronology_anchor"), sc.atlas.chronology_anchors.len());
    assert_eq!(n("book_narration_window"), sc.atlas.book_narration_windows.len());
    assert_eq!(n("landmark"), sc.atlas.landmarks.len());
    assert_eq!(n("land_mask_region"), sc.atlas.land_mask.len());
    assert_eq!(n("catechism_part"), sc.atlas.catechism.len());
    assert_eq!(n("catechism_item"), sc.atlas.catechism.iter().map(|p| p.items.len()).sum::<usize>());
    assert_eq!(
        n("catechism_item_verse"),
        sc.atlas.catechism.iter().flat_map(|p| &p.items).map(|i| i.verses.len()).sum::<usize>()
    );
    assert_eq!(
        n("catechism_question"),
        sc.atlas.catechism.iter().flat_map(|p| &p.items).map(|i| i.questions.len()).sum::<usize>()
    );
    assert_eq!(
        n("catechism_question_verse"),
        sc.atlas.catechism.iter().flat_map(|p| &p.items).flat_map(|i| &i.questions).map(|q| q.verses.len()).sum::<usize>()
    );
    assert_eq!(n("place_history"), sc.atlas.place_history.len());
    assert_eq!(n("place_history_name"), sc.atlas.place_history.values().map(|h| h.names.len()).sum::<usize>());
    assert_eq!(n("place_history_blurb"), sc.atlas.place_history.values().map(|h| h.blurbs.len()).sum::<usize>());
    let claim_verses = |c: &Option<atlas_core::data::PlaceDateClaim>| c.as_ref().map(|c| c.verses.len()).unwrap_or(0);
    assert_eq!(
        n("place_history_verse"),
        sc.atlas
            .place_history
            .values()
            .map(|h| h.names.iter().map(|x| x.verses.len()).sum::<usize>() + claim_verses(&h.established) + claim_verses(&h.destroyed))
            .sum::<usize>()
    );
    assert_eq!(
        n("place_name_alias"),
        sc.atlas.place_name_aliases.values().flatten().map(|a| a.translations.len()).sum::<usize>()
    );
    assert_eq!(n("place_name_alias_verse"), sc.atlas.place_name_aliases.values().flatten().map(|a| a.verses.len()).sum::<usize>());
    assert_eq!(n("source_category"), sc.sources.categories.len());
    assert_eq!(n("source_entry"), sc.sources.sources.len());
    assert_eq!(n("provenance_entry"), sc.sources.provenances.len());
    assert!(n("provenance_entry") > 0 && n("catechism_item") > 0 && n("land_mask_region") > 0, "the folds are inhabited");
    // testament is derived from BOOKS order (survey: no source field)
    let cb = ex.table("canon_book").unwrap();
    assert_eq!(cb.rows[0], vec![Col::Int(0), Col::Text("GEN".into()), Col::Text("Genesis".into()), Col::Text("OT".into()), Col::Int(50)]);
    assert_eq!(cb.rows[39][3], Col::Text("NT".into()));
    assert_eq!(cb.rows[39][1], Col::Text("MAT".into()));
    // a place with two alias rows keeps both (judgment call 6: alias_ord)
    let multi = sc.atlas.place_name_aliases.iter().find(|(_, v)| v.len() > 1).map(|(id, _)| id.clone());
    if let Some(id) = multi {
        assert!(
            ex.table("place_name_alias").unwrap().rows.iter().any(|r| r[0] == Col::Text(id.clone()) && r[1] == Col::Int(1)),
            "{id} has an alias_ord 1 row"
        );
    }
    // deterministic
    let again = fold_sidecars(&sc.atlas, &sc.sources).unwrap();
    let mut ex2 = Extras::default();
    ex2.extend(again);
    for (a, b) in ex.tables.iter().zip(&ex2.tables) {
        assert_eq!(a.rows, b.rows, "{}", a.spec.name);
    }
}

#[test]
fn a_fixture_directory_without_canon_json_yields_no_sidecars() {
    let dir = std::env::temp_dir().join(format!("db4b-nosidecars-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    assert!(Sidecars::load(&dir).unwrap().is_none());
}

/// DB-4b, the server-path proof: `GraphService::from_artifact` (what
/// `atlas-server` and `bibex` load) publishes the committed manifest's root.
#[test]
fn the_committed_manifest_root_recomputes_from_graph_bin_plus_the_sidecars() {
    let service = atlas_graph::service::GraphService::from_artifact(&data_dir().join("graph.bin")).expect("graph.bin loads");
    let manifest = atlas_graph::sqlite::manifest::read_manifest(&data_dir().join("manifest.toml")).expect("manifest.toml is committed");
    assert_eq!(service.version().0.hex(), manifest.root, "one root: the served version and data/compiled/manifest.toml");
}

/// DB-4c: `unfold` is the inverse of the fold on the real sidecars -- the
/// serving path can build `AtlasData` and `SourcesDocument` from core.
#[test]
fn unfold_is_the_inverse_of_fold_on_the_real_sidecars() {
    let sc = Sidecars::load(&data_dir()).unwrap().unwrap();
    let layout = SectionLayout::under(&data_dir());
    let snap = SqliteSnapshot::open(&layout.manifest_path(), &CommittedZstdSource { layout }).unwrap();
    let (atlas, sources) = snap.with_conn(atlas_graph::sqlite::sidecars::unfold).unwrap();
    assert_eq!(sources, sc.sources);
    assert_eq!(atlas.canon, sc.atlas.canon);
    // book_meta's key is the book code (spec 5.3): the table's pk order,
    // not the JSON's canonical order -- every reader looks a book up.
    let mut a_meta = atlas.books_meta.clone();
    let mut b_meta = sc.atlas.books_meta.clone();
    a_meta.sort_by(|x, y| x.book.cmp(&y.book));
    b_meta.sort_by(|x, y| x.book.cmp(&y.book));
    assert_eq!(a_meta, b_meta);
    assert_eq!(atlas.landmarks, sc.atlas.landmarks);
    assert_eq!(atlas.land_mask, sc.atlas.land_mask);
    assert_eq!(atlas.catechism, sc.atlas.catechism);
    assert_eq!(atlas.chronology_anchors, sc.atlas.chronology_anchors);
    let mut a_windows = atlas.book_narration_windows.clone();
    let mut b_windows = sc.atlas.book_narration_windows.clone();
    a_windows.sort_by(|x, y| x.book.cmp(&y.book));
    b_windows.sort_by(|x, y| x.book.cmp(&y.book));
    assert_eq!(a_windows, b_windows, "narration windows (keyed by book; the table's pk order)");
    assert_eq!(atlas.place_history, sc.atlas.place_history);
    assert_eq!(atlas.place_name_aliases, sc.atlas.place_name_aliases);
    // and finish()'s derived indexes agree
    let (a, b) = (atlas.finish(), sc.atlas.clone());
    let span = atlas_core::refs::ScriptureRef::parse("JHN.3.16").unwrap();
    assert_eq!(a.catechism_items_for_span(&span).len(), b.catechism_items_for_span(&span).len());
    assert!(a.catechism_items_for_span(&span).len() > 0 || b.catechism_items_for_span(&span).is_empty());
}
