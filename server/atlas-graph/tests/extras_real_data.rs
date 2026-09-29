mod common;

use atlas_graph::sqlite::extras::{table_specs_of, Col, Extras};
use atlas_graph::sqlite::sidecars::fold_sidecars;
use atlas_graph::sqlite::source::SectionLayout;
use atlas_graph_types::sections::Section;

struct Sidecars {
    atlas: &'static atlas_core::data::AtlasData,
    sources: atlas_core::sources::SourcesDocument,
}
fn sidecars() -> &'static Sidecars {
    static CACHED: std::sync::OnceLock<Sidecars> = std::sync::OnceLock::new();
    CACHED.get_or_init(|| Sidecars { atlas: common::real_atlas(), sources: common::sources_registry() })
}

#[test]
fn the_real_sidecars_fold_losslessly_into_twenty_one_tables() {
    let sc = sidecars();
    let tables = fold_sidecars(sc.atlas, &sc.sources).unwrap();
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
    let cb = ex.table("canon_book").unwrap();
    assert_eq!(cb.rows[0], vec![Col::Int(0), Col::Text("GEN".into()), Col::Text("Genesis".into()), Col::Text("OT".into()), Col::Int(50)]);
    assert_eq!(cb.rows[39][3], Col::Text("NT".into()));
    assert_eq!(cb.rows[39][1], Col::Text("MAT".into()));
    let multi = sc.atlas.place_name_aliases.iter().find(|(_, v)| v.len() > 1).map(|(id, _)| id.clone());
    if let Some(id) = multi {
        assert!(
            ex.table("place_name_alias").unwrap().rows.iter().any(|r| r[0] == Col::Text(id.clone()) && r[1] == Col::Int(1)),
            "{id} has an alias_ord 1 row"
        );
    }
    let again = fold_sidecars(sc.atlas, &sc.sources).unwrap();
    let mut ex2 = Extras::default();
    ex2.extend(again);
    for (a, b) in ex.tables.iter().zip(&ex2.tables) {
        assert_eq!(a.rows, b.rows, "{}", a.spec.name);
    }
}

#[test]
fn the_committed_manifest_root_recomputes_from_graph_bin_plus_the_sidecars() {
    let service = common::committed_service();
    let manifest = atlas_graph::sqlite::manifest::read_manifest(&SectionLayout::under(&common::compiled_dir()).manifest_path()).expect("manifest.toml is committed");
    assert_eq!(service.version().0.hex(), manifest.root, "one root: the served version and data/compiled/manifest.toml");
}

#[test]
fn unfold_is_the_inverse_of_fold_on_the_real_sidecars() {
    let sc = sidecars();
    let (atlas, sources) = common::committed_sections().with_conn(atlas_graph::sqlite::sidecars::unfold).unwrap();
    assert_eq!(sources, sc.sources);
    assert_eq!(atlas.canon, sc.atlas.canon);
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
    let (a, b) = (atlas.finish(), sc.atlas.clone().finish());
    let span = atlas_core::refs::ScriptureRef::parse("JHN.3.16").unwrap();
    assert_eq!(a.catechism_items_for_span(&span).len(), b.catechism_items_for_span(&span).len());
    assert!(a.catechism_items_for_span(&span).len() > 0 || b.catechism_items_for_span(&span).is_empty());
}
