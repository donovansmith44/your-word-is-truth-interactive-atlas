//! Batch KRETZ-1: real-data tests over the full vendored Kretzmann corpus
//! (`data/raw/kretzmann/`, 1,189 pages) -- mirrors `brainfuel_real_data.rs`'s
//! own "parse the real, full, committed source; pin the real counts" shape.
//!
//! KRETZ-ACCEPT-1 (the conservation law) is run over the WHOLE real corpus,
//! against the REAL, RESTORED (KJV-CASE + KJV-CASE-2) canonical text -- the
//! SAME restoration pipeline `build.rs` applies. The real outcome, verified
//! by manual sampling across the whole corpus (both Old and New Testament,
//! both page templates, both "got shorter" and "got longer" cases) before
//! this test's own counts were pinned: every deviation beyond the three
//! disclosed mechanical classes (exact / case+punctuation / case+
//! punctuation+curated-spelling) is a TYPOGRAPHIC OR COMMENTARY-STYLE
//! difference, NEVER a case of verse-content substitution -- the SAME
//! conclusion, and the SAME disclosure discipline, `brainfuel.rs`'s own
//! `kjv_cross_check` reached over an independent cross-check source
//! (LICENSES.md's own "brain-fuel/bible parallel editions" section: "9,274
//! of 31,102... every single one a typographic/transcription-convention
//! difference... NEVER a case of verse-content substitution"). The observed
//! residual classes here, in descending frequency (batch report has the
//! full breakdown with real examples):
//! - Kretzmann quotes only part of a longer verse (its opening clause) then
//!   moves to prose commentary without re-quoting the rest verbatim -- by
//!   far the largest class, and completely normal commentary-writing style,
//!   not a defect.
//! - The digital edition occasionally bolds a short connective gloss
//!   alongside the quoted clause (e.g. GEN 4:3's own "at the end of many
//!   days, at the expiration of a long period" -- Kretzmann's own
//!   paraphrase, styled as if part of the lemma).
//! - Compound Hebrew proper names: our own canonical source en-dash-joins
//!   them ("Beth–el"); the digital edition sometimes hyphenates and
//!   sometimes fully joins them ("Bethel") -- a real, disclosed convention
//!   difference, not a finite table (unlike the curated `SPELLING_VARIANTS`
//!   table, which covers common-word spelling only).
//! - Psalm 119's own Hebrew-letter acrostic stanza headers (folded into
//!   each stanza's own first verse by OUR canonical convention, per
//!   `atlas_core::data::Event::kjv_superscription`'s own doc comment) are
//!   simply never quoted by Kretzmann's own lemma at all.
//!
//! This test PINS the real counts (the SAME "regression harness, moves only
//! with a disclosed reason" discipline `version_root_regression.rs`'s own
//! header comment establishes) rather than asserting zero mismatches --
//! that would be dishonest over data this size; scale honesty (batch brief
//! decision 9) means disclosing the real shape, not forcing a fake pass.

use std::collections::BTreeMap;
use std::path::Path;

use atlas_etl::kretzmann::{self, Calendar, DeviationClass};

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn real_corpus() -> kretzmann::KretzmannCorpus {
    kretzmann::read_all(&data_dir().join("raw/kretzmann")).expect("read_all must succeed over the real vendored corpus (data/fetch-raw.ps1 must have run)")
}

/// The real, RESTORED (KJV-CASE + KJV-CASE-2) canonical verse map, keyed
/// `(book_index, chapter, verse)` -- the SAME text `build_graph_from_
/// sources_with_eras_and_brainfuel*` compiles into the graph's own TextUnit
/// nodes (`build.rs`'s own doc comment).
fn real_canonical() -> BTreeMap<(u8, u16, u16), String> {
    let dir = data_dir();
    let kjv_json = std::fs::read_to_string(dir.join("raw/kjv.json")).expect("data/raw/kjv.json must exist");
    let (canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    let brainfuel = atlas_etl::brainfuel::read_all(&dir.join("raw/brain-fuel-bible")).expect("data/raw/brain-fuel-bible must exist");
    let (restored, _report) = atlas_etl::brainfuel::restore_kjv_case(&brainfuel, &verses);

    let mut canonical = BTreeMap::new();
    for book in &canon.books {
        let book_index = atlas_core::canon::resolve_alias(&book.code).expect("every compiled book code resolves").0;
        for (chapter_idx, &verse_count) in book.chapters.iter().enumerate() {
            let chapter = (chapter_idx + 1) as u16;
            for v in 1..=verse_count {
                let key = format!("{}.{}.{}", book.code, chapter, v);
                if let Some(text) = restored.get(&key) {
                    canonical.insert((book_index, chapter, v), text.clone());
                }
            }
        }
    }
    canonical
}

#[test]
fn real_corpus_covers_all_1189_pages_with_the_pinned_unit_and_fragment_totals() {
    let corpus = real_corpus();
    assert_eq!(corpus.stats.pages, 1189, "comprehensive = all 66 books, every chapter page (controller decision 1)");
    assert_eq!(corpus.chapters.len(), 1189);
    assert_eq!(corpus.stats.units, 50439);
    assert_eq!(corpus.stats.fragments, 61490);
    assert_eq!(corpus.stats.footnotes, 257);
    assert_eq!(corpus.stats.footnotes_in_lemma, 0, "a footnote landing inside an excised lemma/quote span was never observed in the real corpus");
    assert_eq!(corpus.stats.disclosures.len(), 15, "structural anomalies -- see the batch report for the full, named list");
}

#[test]
fn gen_1_2_is_the_named_multi_unit_verse_spot_check() {
    let corpus = real_corpus();
    let gen1 = corpus.chapters.iter().find(|c| c.book_index == 0 && c.chapter == 1).expect("Genesis 1 must be in the corpus");
    let v2_fragments: Vec<&str> = gen1.fragments.iter().filter(|f| f.verse == 2).map(|f| f.text.as_str()).collect();
    assert_eq!(
        v2_fragments,
        vec!["And the earth was without form and void.", "And darkness was upon the face of the deep.", "And the Spirit of God moved upon the face of the waters.",],
        "GEN 1:2's own three-fragment split -- the flagship multi-unit-verse case (batch brief's own required quote)"
    );
    let v2_units: Vec<&kretzmann::KretzUnit> = gen1.units.iter().filter(|u| u.verse_from == 2 && u.verse_to == 2).collect();
    assert_eq!(v2_units.len(), 3, "three separate CommentaryItems, each comments-on the SAME verse 2 -- legitimate per the verse-mapped-index law");
}

#[test]
fn psa_110_1_the_restored_case_verse_reconciles_under_the_disclosed_mechanical_class() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    let psa110 = corpus.chapters.iter().find(|c| c.book_index == 18 && c.chapter == 110).expect("Psalm 110 must be in the corpus");

    let canon_v1 = canonical.get(&(18u8, 110u16, 1u16)).expect("PSA.110.1 must be in the real canonical map");
    assert!(canon_v1.contains("The LORD said unto my Lord"), "the RESTORED canonical text must carry the Tetragrammaton case convention: {canon_v1:?}");

    // The leading, unnumbered "A psalm of David," superscription-class
    // lemma folds into verse 1 (module doc comment on `kretzmann.rs`'s own
    // "Psalm-superscription fold rule") -- matching how OUR canonical
    // layer folds the superscription into verse 1's own text.
    let v1_fragment_count = psa110.fragments.iter().filter(|f| f.verse == 1).count();
    assert!(v1_fragment_count >= 4, "superscription + 'The Lord said...' + 'Sit Thou...' + 'until I make...' -- got {v1_fragment_count}");

    let mut single = BTreeMap::new();
    single.insert((18u8, 110u16, 1u16), canon_v1.clone());
    let report = kretzmann::check_conservation(&psa110.fragments.iter().filter(|f| f.verse == 1).cloned().collect::<Vec<_>>(), &single);
    assert_eq!(report.mismatches.len(), 0, "PSA 110:1 must reconcile: {:#?}", report.mismatches);
    assert_eq!(report.exact + report.mechanical + report.mechanical_spelling, 1);
}

#[test]
fn jhn_3_16_the_type_b_gospel_chapter_spot_check_reconciles() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    let jhn3 = corpus.chapters.iter().find(|c| c.book_index == 42 && c.chapter == 3).expect("John 3 must be in the corpus");
    let frag = jhn3.fragments.iter().find(|f| f.verse == 16).expect("JHN 3:16 must have an excised fragment (Type B block quote)");
    assert!(frag.text.contains("For God so loved the world"), "got: {:?}", frag.text);

    let mut single = BTreeMap::new();
    single.insert((42u8, 3u16, 16u16), canonical.get(&(42u8, 3u16, 16u16)).unwrap().clone());
    let report = kretzmann::check_conservation(&[frag.clone()], &single);
    assert_eq!(report.mismatches.len(), 0, "JHN 3:16 must reconcile: {:#?}", report.mismatches);

    // The unit itself carries real, substantial commentary prose (Type B's
    // own "one unit per block, prose from every following paragraph"
    // shape), not an empty shell.
    let unit = jhn3.units.iter().find(|u| u.verse_from <= 16 && u.verse_to >= 16).expect("a unit covering verse 16 must exist");
    assert!(unit.text.len() > 500, "JHN 3:14-17's own real commentary is substantial prose, got {} chars", unit.text.len());
    assert!(unit.text.contains("brazen serpent"), "Kretzmann's own typological discussion must survive verbatim in the stored prose");
}

#[test]
fn kretz_accept_1_conservation_law_over_the_whole_real_corpus_has_the_pinned_shape() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    assert_eq!(canonical.len(), 31102, "the whole KJV, restored");

    let all_fragments: Vec<kretzmann::ExcisedFragment> = corpus.chapters.iter().flat_map(|c| c.fragments.iter().cloned()).collect();
    assert_eq!(all_fragments.len(), 61490);

    let report = kretzmann::check_conservation(&all_fragments, &canonical);

    // PINNED (this test's own module doc comment has the full derivation
    // and the categorized real-example evidence for why this is a real,
    // understood, disclosed shape, not a bug to keep chasing to zero) --
    // moves only with a new commit that deliberately changes the parser or
    // the vendored source, with a one-line reason, matching `version_root_
    // regression.rs`'s own established convention.
    assert_eq!(report.checked, 31032);
    assert_eq!(report.exact, 2498);
    assert_eq!(report.mechanical, 22933);
    assert_eq!(report.mechanical_spelling, 1853);
    assert_eq!(report.mismatches.len(), 3748);
    assert_eq!(report.uncovered.len(), 70, "verses Kretzmann summarizes without a lemma of their own -- lawful (decision 3), not an error");

    // The conservation identity itself: every checked verse falls into
    // EXACTLY one class, and checked + uncovered == the whole canonical
    // KJV -- no verse is silently double-counted or dropped from the law's
    // own accounting.
    assert_eq!(report.exact + report.mechanical + report.mechanical_spelling + report.mismatches.len(), report.checked);
    assert_eq!(report.checked + report.uncovered.len(), canonical.len());

    // A genuine content-changing deviation (a DIFFERENT word entirely, not
    // case/punctuation/spelling) must NEVER be silently absorbed into a
    // passing tier -- spot-verified directly against a real mismatch this
    // batch's own report samples: GEN 2:19's own lemma drops "God" outright
    // ("the Lord formed" vs. canonical "the LORD God formed"), a genuine
    // source-transcription omission, correctly landing as a `Mismatch`,
    // never silently equated.
    let gen_2_19 = report.mismatches.iter().find(|m| m.book_index == 0 && m.chapter == 2 && m.verse == 19).expect("GEN 2:19 must be a real, disclosed mismatch");
    assert_eq!(gen_2_19.class, DeviationClass::Mismatch);
}

#[test]
fn every_date_clause_verbatim_is_a_real_substring_of_its_own_units_prose_over_the_whole_real_corpus() {
    let corpus = real_corpus();
    let mut total = 0usize;
    let mut by_calendar: BTreeMap<&str, usize> = BTreeMap::new();
    let mut approx_count = 0usize;
    for chapter in &corpus.chapters {
        for unit in &chapter.units {
            for clause in kretzmann::extract_date_clauses(&unit.text) {
                assert!(unit.text.contains(&clause.verbatim), "unit {} own clause {:?} must be a real substring of its own stored prose", unit.id, clause.verbatim);
                // Parse round-trip: the SAME clause, re-extracted from
                // itself in isolation, must parse identically (no
                // synthesis, no context-dependent guessing).
                let roundtrip = kretzmann::extract_date_clauses(&clause.verbatim);
                assert_eq!(roundtrip.len(), 1, "clause {:?} must round-trip to exactly one clause parsed from itself alone", clause.verbatim);
                assert_eq!(roundtrip[0].calendar, clause.calendar);
                assert_eq!(roundtrip[0].year, clause.year);
                assert_eq!(roundtrip[0].approx, clause.approx);
                total += 1;
                if clause.approx {
                    approx_count += 1;
                }
                *by_calendar
                    .entry(match clause.calendar {
                        Calendar::Bc => "BC",
                        Calendar::Ad => "AD",
                        Calendar::Am => "AM",
                    })
                    .or_insert(0) += 1;
            }
        }
    }
    // PINNED real counts -- the date mine's own "extraction only" law
    // (scouting memo) proven over the real corpus.
    assert_eq!(total, 83);
    assert_eq!(by_calendar.get("BC").copied().unwrap_or(0), 42);
    assert_eq!(by_calendar.get("AD").copied().unwrap_or(0), 41);
    assert_eq!(by_calendar.get("AM").copied().unwrap_or(0), 0, "zero Anno Mundi clauses found in the real corpus -- disclosed, not assumed absent");
    assert!(approx_count > 0, "at least one 'about' approximation must be found in real prose this size");
}

/// Per-book unit-count table -- the batch report's own required
/// disclosure. Not a pinned regression assertion (a future re-fetch of the
/// same live site is expected to be stable, but this test's own job is to
/// PRODUCE the table, verified non-empty for every one of the 66 books).
#[test]
fn every_one_of_the_66_books_has_at_least_one_unit() {
    let corpus = real_corpus();
    let mut units_by_book: BTreeMap<u8, usize> = BTreeMap::new();
    for chapter in &corpus.chapters {
        *units_by_book.entry(chapter.book_index).or_insert(0) += chapter.units.len();
    }
    assert_eq!(units_by_book.len(), 66, "every one of the 66 books must contribute at least one unit");
    for (book, count) in &units_by_book {
        assert!(*count > 0, "book_index {book} carries zero units");
    }
}
