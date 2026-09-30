use std::collections::BTreeSet;

use atlas_etl::lexicon::{read_all, LexiconCorpus, LexiconStats, LAYER_GREEK, LAYER_HEBREW};

mod common;

use common::raw_dir;

fn corpus() -> LexiconCorpus {
    read_all(&raw_dir().join("brain-fuel-bible")).expect("data/raw/brain-fuel-bible/{lexicon,morph} must exist -- run data/fetch-raw.ps1 first")
}

#[test]
fn the_pinned_coverage_is_reproduced_exactly() {
    let c = corpus();
    assert_eq!(
        c.stats,
        LexiconStats { entries_grc: 5122, entries_hbo: 8426, tokens_nt: 140_610, tokens_ot: 312_079, unmatched_nt: 6615, unmatched_ot: 14_794, files: 1189 }
    );
    assert_eq!(c.entries.len(), 13_548);
    assert_eq!(c.tokens.len(), 452_689);
    let aligned = c.tokens.iter().filter(|t| t.aligned).count();
    assert_eq!(aligned, 431_280, "every token with Strong= is aligned; the rest are Align=unmatched");
}

#[test]
fn entries_are_sorted_unique_strongs_ids_and_only_strongs_files_are_read() {
    let c = corpus();
    for w in c.entries.windows(2) {
        assert!(w[0].strong < w[1].strong, "{} !< {}", w[0].strong, w[1].strong);
    }
    for e in &c.entries {
        let (head, digits) = e.strong.split_at(1);
        assert!(head == "G" || head == "H", "{}", e.strong);
        assert!(digits.len() >= 4 && digits.bytes().all(|b| b.is_ascii_digit()), "{}", e.strong);
        assert_eq!(e.lang, if head == "G" { "grc" } else { "hbo" }, "{}", e.strong);
    }
    assert!(c.entries.iter().all(|e| !e.strong.starts_with("lemma")));
}

#[test]
fn logos_and_elohim_read_as_published() {
    let c = corpus();
    let logos = c.entries.iter().find(|e| e.strong == "G3056").expect("G3056");
    assert_eq!(logos.lemma, "λ\u{1f79}γος");
    assert_eq!(logos.translit.as_deref(), Some("lógos"));
    assert_eq!(logos.pos.as_deref(), Some("G:N-M"));
    assert_eq!(logos.lang, "grc");
    assert!(logos.glosses[0].starts_with("from G3004;"), "{:?}", logos.glosses[0]);
    assert_eq!(logos.glosses[1], "word");
    assert_eq!(logos.senses.len(), 1);
    assert_eq!(logos.root.as_deref(), Some("G3004"));

    let elohim = c.entries.iter().find(|e| e.strong == "H0430").expect("H0430");
    assert_eq!(elohim.lemma, "אֱלֹהִים");
    assert_eq!(elohim.lang, "hbo");
    assert_eq!(elohim.pos.as_deref(), Some("n-m"));
    assert_eq!(elohim.root.as_deref(), Some("H0433"));
}

#[test]
fn sixteen_upstream_entries_have_no_lemma_and_are_carried_as_published() {
    let c = corpus();
    let bare: Vec<&str> = c.entries.iter().filter(|e| e.lemma.is_empty()).map(|e| e.strong.as_str()).collect();
    assert_eq!(
        bare,
        vec!["G6053", "G6063", "G6083", "H9005", "H9007", "H9008", "H9030", "H9031", "H9032", "H9033", "H9034", "H9035", "H9036", "H9037", "H9038", "H9039"]
    );
    let refs = c.tokens.iter().filter(|t| t.strong.as_deref() == Some("H9033")).count();
    assert_eq!(refs, 1515);
}

#[test]
fn tokens_are_in_canonical_reading_order_over_all_66_books() {
    let c = corpus();
    for w in c.tokens.windows(2) {
        let a = (w[0].book.0, w[0].chapter, w[0].verse, w[0].ord);
        let b = (w[1].book.0, w[1].chapter, w[1].verse, w[1].ord);
        assert!(a < b, "{a:?} !< {b:?}");
    }
    let books: BTreeSet<u8> = c.tokens.iter().map(|t| t.book.0).collect();
    assert_eq!(books.len(), 66);
    assert_eq!(books.iter().next(), Some(&0));
    assert_eq!(books.iter().next_back(), Some(&65));
    for t in &c.tokens {
        assert!(t.chapter >= 1 && t.verse >= 1 && t.ord >= 1, "{:?}", (t.book.code(), t.chapter, t.verse, t.ord));
        assert_eq!(t.layer, if t.book.0 < 39 { LAYER_HEBREW } else { LAYER_GREEK }, "{}", t.book.code());
        assert_eq!(t.aligned, t.strong.is_some());
        assert!(!t.form.is_empty());
    }
    let first = &c.tokens[0];
    assert_eq!((first.book.code(), first.chapter, first.verse, first.ord), ("GEN", 1, 1, 1));
    assert_eq!(first.strong.as_deref(), Some("H7225"));
    assert_eq!(first.form, "בְּרֵאשִׁ֖ית");
    let first_nt = c.tokens.iter().find(|t| t.layer == LAYER_GREEK).unwrap();
    assert_eq!((first_nt.book.code(), first_nt.chapter, first_nt.verse, first_nt.ord), ("MAT", 1, 1, 1));
    assert_eq!(first_nt.strong.as_deref(), Some("G0976"));
    assert_eq!(first_nt.translit.as_deref(), Some("Biblos"));
}

#[test]
fn every_aligned_token_names_an_entry_that_exists() {
    let c = corpus();
    let ids: BTreeSet<&str> = c.entries.iter().map(|e| e.strong.as_str()).collect();
    let orphans: BTreeSet<&str> = c.tokens.iter().filter_map(|t| t.strong.as_deref()).filter(|s| !ids.contains(s)).collect();
    assert!(orphans.is_empty(), "tokens whose Strong's id has no lexicon entry: {orphans:?}");
}

#[test]
fn john_3_16_has_26_aligned_tokens_and_logos_occurs_319_times() {
    let c = corpus();
    let jhn = atlas_core::canon::resolve_alias("John").unwrap();
    let v: Vec<_> = c.tokens.iter().filter(|t| t.book == jhn && t.chapter == 3 && t.verse == 16).collect();
    assert_eq!(v.len(), 26);
    assert!(v.iter().all(|t| t.aligned));
    let strongs: Vec<&str> = v.iter().map(|t| t.strong.as_deref().unwrap()).collect();
    assert_eq!(&strongs[..3], &["G3779", "G1063", "G0025"]);
    assert_eq!(strongs.iter().filter(|s| **s == "G3588").count(), 5, "the article five times in one verse");
    let logos = c.tokens.iter().filter(|t| t.strong.as_deref() == Some("G3056")).count();
    assert_eq!(logos, 319);
    let first_logos = c.tokens.iter().find(|t| t.strong.as_deref() == Some("G3056")).unwrap();
    assert_eq!((first_logos.book.code(), first_logos.chapter, first_logos.verse), ("MAT", 5, 32));
}
