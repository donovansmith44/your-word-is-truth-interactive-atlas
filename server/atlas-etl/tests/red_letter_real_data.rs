use std::path::Path;

use atlas_etl::red_letter;

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn real_restored_verses() -> std::collections::HashMap<String, String> {
    let dir = data_dir();
    let kjv_json = std::fs::read_to_string(dir.join("raw/kjv.json")).expect("data/raw/kjv.json must exist");
    let (_canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("raw/brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
    atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &verses).0
}

fn real_corpus() -> red_letter::RedLetterCorpus {
    let verses = real_restored_verses();
    red_letter::read_all(&data_dir().join("raw/red-letter"), &verses).expect("red_letter::read_all must succeed over the real vendored source (data/fetch-raw.ps1 must have run)")
}

#[test]
fn real_corpus_has_the_pinned_alignment_shape() {
    let corpus = real_corpus();
    let s = corpus.stats;
    eprintln!(
        "RED-1 real corpus: verse_set={} spans_total={} exact={} case_insensitive={} not_found={}",
        s.verses_with_source_markup, s.source_spans_total, s.exact, s.case_insensitive, s.not_found
    );
    assert_eq!(s.verses_with_source_markup, 2066, "the verse SET's own size -- edition-independent, present regardless of alignment outcome");
    assert_eq!(s.source_spans_total, 2081, "every <q who=\"Jesus\"> run the source carries");
    assert_eq!(s.exact, 2059, "case-sensitive verbatim substring matches (the GAZ-1 law, unmodified)");
    assert_eq!(s.case_insensitive, 4, "the disclosed KJV-CASE class: our restored casing vs the source's own normalization");
    assert_eq!(s.not_found, 18, "the real, fully-categorized residual -- see align_verse's own doc comment");
    assert_eq!(s.exact + s.case_insensitive + s.not_found, s.source_spans_total, "every counted span lands in exactly one category");
    assert_eq!(corpus.verses.len(), s.verses_with_source_markup);
}

#[test]
fn mat_4_19_follow_me_is_the_named_spot_check() {
    let corpus = real_corpus();
    let verses = real_restored_verses();
    let mat_4_19 = corpus.verses.iter().find(|v| v.book_index == 39 && v.chapter == 4 && v.verse == 19).expect("MAT.4.19 must be in the verse set");
    assert_eq!(mat_4_19.spans.len(), 1);
    let (start, end) = mat_4_19.spans[0];
    let canon = &verses["MAT.4.19"];
    assert_eq!(&canon[start..end], "Follow me, and I will make you fishers of men.");
    assert_eq!(&canon[..start], "And he saith unto them, ", "the narration prefix must not be part of the red span");
}

#[test]
fn mat_5_4_is_a_fully_red_verse_spot_check() {
    let corpus = real_corpus();
    let verses = real_restored_verses();
    let mat_5_4 = corpus.verses.iter().find(|v| v.book_index == 39 && v.chapter == 5 && v.verse == 4).expect("MAT.5.4 must be in the verse set");
    assert_eq!(mat_5_4.spans.len(), 1);
    let (start, end) = mat_5_4.spans[0];
    let canon = &verses["MAT.5.4"];
    assert_eq!(start, 0, "a fully-red verse's own span starts at byte 0");
    assert_eq!(end, canon.len(), "and runs to the verse's own full length");
}

#[test]
fn gen_1_1_is_a_no_red_verse_spot_check() {
    let corpus = real_corpus();
    assert!(!corpus.verses.iter().any(|v| v.book_index == 0 && v.chapter == 1 && v.verse == 1), "GEN.1.1 must never be in the words-of-Christ verse set");
}
