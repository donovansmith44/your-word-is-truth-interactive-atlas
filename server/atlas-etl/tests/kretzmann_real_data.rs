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
//! residual classes here (batch report has the full breakdown with real
//! examples):
//! - Kretzmann quotes only part of a longer verse (its opening clause) then
//!   moves to prose commentary without re-quoting the rest verbatim -- by
//!   far the largest class, and completely normal commentary-writing style,
//!   not a defect.
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
//! FIX ROUND 1 (review finding 2, the OVER-EXCISION GUARD -- `kretzmann.rs`'s
//! own "OVER-EXCISION GUARD" section has the full algorithm derivation,
//! including two earlier LCS-based cuts that were tried and empirically
//! refuted over this SAME real corpus before being superseded): a
//! previously undocumented residual class -- "the digital edition
//! occasionally bolds Kretzmann's OWN short connective gloss alongside the
//! quoted clause" (GEN 4:3's own "at the end of many days, at the
//! expiration of a long period" was this test's own original example of
//! it) -- is now MECHANICALLY RECOVERED rather than silently excised as if
//! it were KJV text, moving 1,054 real instances (`corpus.stats.
//! over_excisions`, pinned below) out of the excised-fragment path and
//! into stored prose. This is WHY `kretz_accept_1...`'s own pinned numbers
//! below moved from the original release (checked/uncovered unchanged;
//! exact 2,498->2,525, mechanical 22,933->23,606, mechanical_spelling
//! 1,853->1,903, mismatches 3,748->2,998 -- the four deltas sum to zero,
//! confirming no verse silently changed classification bucket count
//! without a matching debit elsewhere). GEN 4:3 itself is now a
//! fix-round-1-RESOLVED instance, not a residual mismatch -- superseded as
//! this comment's own example accordingly.
//!
//! This test PINS the real counts (the SAME "regression harness, moves only
//! with a disclosed reason" discipline `version_root_regression.rs`'s own
//! header comment establishes) rather than asserting zero mismatches --
//! that would be dishonest over data this size; scale honesty (batch brief
//! decision 9) means disclosing the real shape, not forcing a fake pass.
//!
//! FIX ROUND 2 (re-review NEW FINDING, MEDIUM): on Type-B (quote-block)
//! pages, a mid-sentence verse boundary is occasionally rendered by the
//! digital edition as literal inline text ("v. 61", "v. 21", ...) instead
//! of a proper `<sup id="vNN">` tag -- `split_by_verse_markers` had no way
//! to recognize this, so the WHOLE following verse's own genuine KJV text
//! got silently swallowed into the PRECEDING verse's own fragment, where
//! fix round 1's own OVER-EXCISION GUARD then recovered it as if it were
//! Kretzmann's own prose (real content, correctly excised text, wrongly
//! ATTRIBUTED -- never lost, but mislabeled). `find_inline_verse_marker`
//! (`kretzmann.rs`) now recognizes this shape mechanically: a literal
//! "v. N" not preceded by a letter (so "Lev. 1"/"Rev. 5"/"Prov. 5" style
//! book abbreviations never trigger) where N is EXACTLY the verse
//! immediately following the one currently open -- that sequential-
//! adjacency requirement is what tells a genuine boundary apart from an
//! ordinary BACKWARD cross-reference (LEV 21:14's own lemma genuinely
//! contains "v. 7" mid-quote, citing back to verse 7's own similar
//! restriction -- confirmed a real corpus instance, confirmed correctly
//! NOT matched, since verse 14 is already open when it appears). A
//! corpus-wide mechanical sweep (`corpus.stats.inline_verse_markers`,
//! pinned below) found **exactly 8** real instances, matching the
//! re-review's own independently-confirmed count precisely: MAT 26:61,
//! MAT 27:40, LUK 2:35, LUK 17:21, LUK 19:42, LUK 19:46, LUK 20:2, LUK
//! 20:36 (each named as "the verse whose own text was swallowed"). All 8
//! resisted no mechanical classification (none needed disclosure-only
//! treatment). This moved several OTHER pinned numbers too, all disclosed
//! at their own assertion site below: `over_excisions` (1,054->1,046 --
//! each of the 8 instances used to ALSO trip the over-excision guard as an
//! accidental side effect, which now correctly never fires for them);
//! `checked`/`uncovered` in KRETZ-ACCEPT-1 (31,032->31,040 /
//! 70->62 -- each swallowed verse used to count as UNCOVERED, a real
//! coverage gap this fix closes, not merely a reclassification).

use std::collections::BTreeMap;
use std::path::Path;

use atlas_etl::kretzmann::{self, Calendar, DeviationClass};

fn data_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn real_corpus() -> kretzmann::KretzmannCorpus {
    let kjv_verses = real_kjv_verses();
    kretzmann::read_all(&data_dir().join("raw/kretzmann"), &kjv_verses).expect("read_all must succeed over the real vendored corpus (data/fetch-raw.ps1 must have run)")
}

/// The real, UN-restored `kjv.json` verse map (dot-ref keyed), the SAME
/// source `read_all`'s own OVER-EXCISION GUARD uses internally (fix round
/// 1) -- word-content comparison only, no KJV-CASE restoration needed.
fn real_kjv_verses() -> std::collections::HashMap<String, String> {
    let kjv_json = std::fs::read_to_string(data_dir().join("raw/kjv.json")).expect("data/raw/kjv.json must exist");
    let (_canon, verses) = atlas_etl::kjv::parse(&kjv_json).expect("kjv.json must parse");
    verses
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
    // Fix round 1: units/fragments moved (50,439->50,602; 61,490->61,366)
    // as a direct, mechanical consequence of the OVER-EXCISION GUARD
    // recovering real Kretzmann prose that used to be silently excised as
    // if it were KJV lemma text -- a previously-empty-shell unit gaining
    // real recovered text now survives the "drop empty units" filter
    // (units UP), and a fragment whose ENTIRE candidate text turned out to
    // be recovered prose (nothing left to excise) is no longer pushed at
    // all (fragments DOWN). This test's own module doc comment has the
    // full derivation.
    assert_eq!(corpus.stats.units, 50602);
    // Fix round 2 (re-review NEW FINDING): +8 (61,366->61,374) -- the 8
    // instances of Kretzmann's own inline "v. N" verse-boundary citation
    // (`find_inline_verse_marker`'s own doc comment) each split what used
    // to be one fragment into two, correctly re-attributing the swallowed
    // FOLLOWING verse's own genuine KJV text to its own verse instead of
    // the preceding one -- one new fragment per instance, `units` UNCHANGED
    // (the split verse joins the SAME Type-B pericope unit's own existing
    // range; it was already inside verse_from..verse_to for that unit).
    assert_eq!(corpus.stats.fragments, 61374);
    assert_eq!(corpus.stats.footnotes, 257);
    assert_eq!(corpus.stats.footnotes_in_lemma, 0, "a footnote landing inside an excised lemma/quote span was never observed in the real corpus");
    // The true, disclosed over-excision instance count (review finding 2's
    // own "disclose the true instance count" requirement) -- one entry per
    // fragment where the guard found and recovered real non-KJV content,
    // corpus-wide, found by SWEEPING every fragment mechanically, not by
    // sampling (the reviewer's own narrower "2 confirmed of 61,490" figure
    // was from a targeted sample sweep, not an exhaustive one). Fix round 2
    // moved this from 1,054 to 1,046 (-8): each of the 8 inline-marker
    // instances (below) used to ALSO trip the OVER-EXCISION GUARD as an
    // accidental side effect -- with no real verse boundary to split at,
    // the swallowed following-verse text read as "extra content past the
    // end of the preceding verse's own canonical text" and got recovered
    // as if it were prose (MAT 26:60's own case, byte-for-byte, is the
    // re-review's own named example). Fixing the ROOT CAUSE (the boundary
    // itself) removes the SYMPTOM (these 8 stop being over-excisions at
    // all, because the fragment now correctly ends exactly where its own
    // real verse ends, leaving nothing "extra" for the guard to recover).
    assert_eq!(corpus.stats.over_excisions, 1046);
    // Fix round 2 (re-review NEW FINDING): the 8 inline-marker recognitions,
    // corpus-wide, disclosed and pinned separately from `over_excisions` --
    // a DIFFERENT mechanism (`split_by_verse_markers` recognizing a real
    // verse boundary from inline text, never the OVER-EXCISION GUARD
    // recovering prose after the fact).
    assert_eq!(corpus.stats.inline_verse_markers, 8);
    // 15 original + 1,046 over-excision (was 1,054) + 8 fix-round-2
    // inline-verse-marker disclosures = 1,069 -- the SAME total as before
    // fix round 2 (also 1,069), a real coincidence, not a sign nothing
    // changed: each of the 8 instances swapped from ONE over-excision
    // disclosure line to ONE inline-verse-marker disclosure line, a 1:1
    // recomposition, confirmed via the real corpus.stats fields above.
    assert_eq!(corpus.stats.disclosures.len(), 1069);
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
    // Fix round 2: +8 (61,366->61,374) -- `real_corpus_covers_all_1189_
    // pages_with_the_pinned_unit_and_fragment_totals`'s own doc comment has
    // the full derivation (one new fragment per inline-verse-marker
    // instance, the swallowed following verse's own genuine KJV text).
    assert_eq!(all_fragments.len(), 61374);

    let report = kretzmann::check_conservation(&all_fragments, &canonical);

    // PINNED (this test's own module doc comment has the full derivation
    // and the categorized real-example evidence for why this is a real,
    // understood, disclosed shape, not a bug to keep chasing to zero) --
    // moves only with a new commit that deliberately changes the parser or
    // the vendored source, with a one-line reason, matching `version_root_
    // regression.rs`'s own established convention. Fix round 1 (review
    // finding 2, the OVER-EXCISION GUARD): moved from 2,498/22,933/1,853/
    // 3,748 -- this test's own module doc comment has the full "why" and
    // the delta-sums-to-zero cross-check. Fix round 2 (re-review NEW
    // FINDING, the inline verse-marker fix): `checked` itself moves for the
    // FIRST time (31,032->31,040, +8) and `uncovered` moves the opposite
    // way (70->62, -8) -- each of the 8 swallowed following verses (MAT
    // 26:61, MAT 27:40, LUK 2:35, LUK 17:21, LUK 19:42, LUK 19:46, LUK
    // 20:2, LUK 20:36) previously had ZERO fragments of its own (its text
    // was glued into the PRECEDING verse's fragment), so it counted as
    // `uncovered` -- lawful under decision 3, but a real coverage gap this
    // fix closes, not merely a reclassification. All 8 newly-created
    // fragments land in `mechanical` (23,606->23,614, +8; case/punctuation
    // only, e.g. MAT 26:61's own lowercase "and said," vs. canonical's
    // capitalized "And said,"). `exact`/`mechanical_spelling`/`mismatches`
    // are UNCHANGED -- the 8 PRECEDING verses' own fragment text was
    // already correct pre-fix (fix round 1's own OVER-EXCISION GUARD had
    // already trimmed the swallowed text back out as "recovered prose",
    // just mis-attributed; fix round 2 changes WHERE that text lands, not
    // the preceding verse's own already-correct fragment content).
    assert_eq!(report.checked, 31040);
    assert_eq!(report.exact, 2525);
    assert_eq!(report.mechanical, 23614);
    assert_eq!(report.mechanical_spelling, 1903);
    assert_eq!(report.mismatches.len(), 2998);
    assert_eq!(report.uncovered.len(), 62, "verses Kretzmann summarizes without a lemma of their own -- lawful (decision 3), not an error");

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
    // (scouting memo) proven over the real corpus. Fix round 1: moved from
    // 83/42/41/0 to 84/43/41/0 -- a recovered-prose unit (review finding 2's
    // own over-excision guard) can gain a dating clause that used to be
    // silently excised as if it were KJV text; the date mine re-ran
    // against the changed prose, per the coordinator's own "re-run the
    // mine, disclose any row delta" ripple instruction, and found exactly
    // one additional B.C. clause, disclosed here, not silently absorbed.
    assert_eq!(total, 84);
    assert_eq!(by_calendar.get("BC").copied().unwrap_or(0), 43);
    assert_eq!(by_calendar.get("AD").copied().unwrap_or(0), 41);
    assert_eq!(by_calendar.get("AM").copied().unwrap_or(0), 0, "zero Anno Mundi clauses found in the real corpus -- disclosed, not assumed absent");
    assert!(approx_count > 0, "at least one 'about' approximation must be found in real prose this size");
}

/// Finds `<strong>{anchor_prefix...}</strong>` in `html` and returns its
/// own inner text verbatim -- a LIVE read of the REAL vendored source, not
/// a second hand-transcribed copy that could silently drift from it.
fn strong_span_starting_with(html: &str, anchor_prefix: &str) -> String {
    let marker = format!("<strong>{anchor_prefix}");
    let start = html.find(&marker).unwrap_or_else(|| panic!("no <strong>{anchor_prefix}... span found in the real source"));
    let inner_start = start + "<strong>".len();
    let end = html[inner_start..].find("</strong>").unwrap_or_else(|| panic!("no closing </strong> after {anchor_prefix:?}"));
    html[inner_start..inner_start + end].to_string()
}

/// Fix round 2 (re-review NEW FINDING): the 8 real, corpus-confirmed
/// instances of Kretzmann's own inline "v. N" verse-boundary citation on
/// Type-B pages -- `(book_index, chapter, swallowed_verse, host_verse)`,
/// where `swallowed_verse` is the verse whose own genuine KJV text used to
/// be glued into `host_verse`'s own fragment before this fix. Named here
/// once, shared by both tests below, so a future refactor cannot silently
/// narrow the general sweep without also breaking an explicitly-named list.
const INLINE_VERSE_MARKER_INSTANCES: &[(u8, u16, u16, u16, &str)] = &[
    (39, 26, 61, 60, "and said, This fellow said, I am able to destroy the Temple of God"),
    (39, 27, 40, 39, "and saying, Thou that destroyest the Temple"),
    (41, 2, 35, 34, "(yea, a sword shall pierce through thy own soul also)"),
    (41, 17, 21, 20, "neither shall they say, Lo here"),
    (41, 19, 42, 41, "saying, If thou hadst known"),
    (41, 19, 46, 45, "saying unto them, It is written"),
    (41, 20, 2, 1, "and spake unto Him, saying, Tell us, by what authority"),
    (41, 20, 36, 35, "neither can they die any more"),
];

/// Fix round 2 (re-review NEW FINDING, MEDIUM): mechanical, corpus-wide
/// verification of all 8 real instances -- for each, the swallowed verse
/// now carries its own real, non-trivial excised fragment (joining that
/// verse's own fragment set, per the fix's own design), AND no unit
/// covering the HOST verse still contains that fragment's text (the
/// end-to-end "lands as lemma of its own verse, not prose of the
/// preceding one" proof `stored_prose_never_contains_its_own_excised_
/// fragment_text`'s own general sweep cannot give, since that test is
/// deliberately scoped to a unit's OWN same-verse fragments only).
#[test]
fn inline_verse_marker_instances_reclassify_to_their_own_verse_not_the_preceding_ones_prose() {
    let corpus = real_corpus();
    for &(book_index, chapter, swallowed_verse, host_verse, distinctive_phrase) in INLINE_VERSE_MARKER_INSTANCES {
        let ch = corpus.chapters.iter().find(|c| c.book_index == book_index && c.chapter == chapter).unwrap_or_else(|| panic!("book_index {book_index} chapter {chapter} must be in the corpus"));
        let frag = ch
            .fragments
            .iter()
            .find(|f| f.verse == swallowed_verse && f.text.contains(distinctive_phrase))
            .unwrap_or_else(|| panic!("verse {swallowed_verse} (book_index {book_index}, chapter {chapter}) must carry its own real excised fragment containing {distinctive_phrase:?}"));
        assert!(frag.text.len() > 10, "the swallowed verse's own fragment must be real, non-trivial content, got {:?}", frag.text);
        for u in ch.units.iter().filter(|u| u.verse_from <= host_verse && u.verse_to >= host_verse) {
            assert!(
                !u.text.contains(distinctive_phrase),
                "book_index {book_index} chapter {chapter}: host verse {host_verse}'s own unit {} must NOT still carry verse {swallowed_verse}'s own text {distinctive_phrase:?} as if it were Kretzmann's prose -- unit text {:?}",
                u.id,
                u.text
            );
        }
    }
}

/// Fix round 2 (re-review NEW FINDING): the required end-to-end pin,
/// MAT 26:60->61, verified against the REAL source HTML directly (not a
/// second hand-transcribed copy) AND against KRETZ-ACCEPT-1's own
/// conservation check for both verses independently.
#[test]
fn mat_26_60_to_61_inline_verse_marker_lands_as_v61_lemma_not_v60_prose_end_to_end() {
    let html = std::fs::read_to_string(data_dir().join("raw/kretzmann/matthew/26.html")).expect("data/raw/kretzmann/matthew/26.html must exist");
    // The real source: verse 61's own boundary is literal "v. 61" text, NOT
    // a `<sup id="v61">` tag -- confirmed absent, so this test's own
    // premise (the source itself carries the defeating shape) stays true.
    assert!(html.contains(" v. 61 and said, This fellow said, I am able to destroy the Temple of God, and to build it in three days. "), "the real source's own inline marker text must still read as originally traced");
    assert!(!html.contains(r#"<sup id="v61">"#), "MAT 26:61 must still lack a real <sup> marker in the source -- otherwise this test's own premise no longer holds");

    let corpus = real_corpus();
    let canonical = real_canonical();
    let mat26 = corpus.chapters.iter().find(|c| c.book_index == 39 && c.chapter == 26).expect("Matthew 26 must be in the corpus");

    // Verse 60's own fragment: genuine, ends exactly at its own true
    // boundary, no longer running on into verse 61's own text.
    let v60_frag = mat26.fragments.iter().find(|f| f.verse == 60).expect("MAT 26:60 must have its own excised fragment");
    assert!(!v60_frag.text.contains("This fellow said"), "verse 60's own fragment must not run on into verse 61's own text: {:?}", v60_frag.text);
    let canon_60 = canonical.get(&(39u8, 26u16, 60u16)).expect("MAT.26.60 must be in the real canonical map");
    let report_60 = kretzmann::check_conservation(&[v60_frag.clone()], &BTreeMap::from([((39u8, 26u16, 60u16), canon_60.clone())]));
    assert_eq!(report_60.mismatches.len(), 0, "MAT 26:60 must reconcile cleanly against its own canonical text: {:#?}", report_60.mismatches);

    // Verse 61's own fragment: newly split out, genuine KJV, reconciles
    // against ITS OWN canonical text (not verse 60's).
    let v61_frag = mat26.fragments.iter().find(|f| f.verse == 61).expect("MAT 26:61 must now have its own excised fragment (fix round 2)");
    assert!(v61_frag.text.to_lowercase().starts_with("and said"), "verse 61's own fragment must open with its own real text, got {:?}", v61_frag.text);
    let canon_61 = canonical.get(&(39u8, 26u16, 61u16)).expect("MAT.26.61 must be in the real canonical map");
    let report_61 = kretzmann::check_conservation(&[v61_frag.clone()], &BTreeMap::from([((39u8, 26u16, 61u16), canon_61.clone())]));
    assert_eq!(report_61.mismatches.len(), 0, "MAT 26:61 must reconcile cleanly against ITS OWN canonical text, not verse 60's: {:#?}", report_61.mismatches);

    // The unit that used to store verse 61's text as if it were verse 60's
    // own prose no longer does -- the mis-attribution is gone, not merely
    // duplicated.
    for u in mat26.units.iter().filter(|u| u.verse_from <= 60 && u.verse_to >= 60) {
        assert!(!u.text.contains("This fellow said"), "MAT 26:60's own stored prose must no longer carry verse 61's own swallowed KJV text: {:?}", u.text);
    }
}

/// Fix round 1 (review finding 2): verifies the two reviewer-named,
/// confirmed over-excision instances not merely against the parser's own
/// internal state, but against the REAL, RAW SOURCE HTML `<strong>` span
/// itself, read live at test time (never a second hand-transcribed copy
/// that could silently drift from the real vendored file) -- the
/// recovered prose and the excised lemma fragment(s), reassembled in
/// their own original document order, must reconstruct that span EXACTLY,
/// byte for byte.
#[test]
fn over_excision_guard_recovers_exo_20_12_and_rut_4_11_prose_verbatim_against_the_source_html() {
    let corpus = real_corpus();

    // EXO 20:12: prose-PREFIX, KJV-SUFFIX shape.
    let exo20_html = std::fs::read_to_string(data_dir().join("raw/kretzmann/exodus/20.html")).expect("data/raw/kretzmann/exodus/20.html must exist");
    let exo_span = strong_span_starting_with(&exo20_html, "with heart, mouth");
    let exo = corpus.chapters.iter().find(|c| c.book_index == 1 && c.chapter == 20).expect("Exodus 20 must be in the corpus");
    let exo_unit = exo
        .units
        .iter()
        .find(|u| u.verse_from <= 12 && u.verse_to >= 12 && u.text.starts_with("with heart, mouth"))
        .expect("EXO 20:12's own recovered-prose unit must exist");
    let exo_frag = exo.fragments.iter().find(|f| f.verse == 12 && f.text.starts_with("that thy days")).expect("EXO 20:12's own recovered lemma fragment must exist");
    // The recovered prose is the LEADING portion of the unit's own stored
    // text, up to (not including) the immediately-following sentence that
    // was ALWAYS plain prose in the source (outside any `<strong>` span).
    let exo_recovered = exo_unit.text.split(" It is the first commandment").next().unwrap();
    assert_eq!(format!("{exo_recovered} {}", exo_frag.text), exo_span, "EXO 20:12: recovered prose + excised lemma must reconstruct the real source <strong> span exactly");

    // RUT 4:11: KJV-prefix, prose-INFIX, KJV-suffix shape.
    let rut4_html = std::fs::read_to_string(data_dir().join("raw/kretzmann/ruth/4.html")).expect("data/raw/kretzmann/ruth/4.html must exist");
    let rut_span = strong_span_starting_with(&rut4_html, "The Lord make the woman");
    let rut = corpus.chapters.iter().find(|c| c.book_index == 7 && c.chapter == 4).expect("Ruth 4 must be in the corpus");
    let rut_unit = rut
        .units
        .iter()
        .find(|u| u.verse_from <= 11 && u.verse_to >= 11 && u.text.starts_with("literally, that is about to come"))
        .expect("RUT 4:11's own recovered-infix unit must exist");
    let rut_frag = rut.fragments.iter().find(|f| f.verse == 11 && f.text.contains("did build the house of Israel")).expect("RUT 4:11's own recovered lemma fragment must exist");
    // The recovered infix is the LEADING portion of the unit's own stored
    // text, up to the immediately-following, ALWAYS-plain-prose
    // continuation ("as the mothers of the twelve tribes;").
    let rut_recovered = rut_unit.text.split(" as the mothers").next().unwrap();
    // The lemma fragment is already the (genuine KJV) prefix+suffix
    // space-joined with the infix removed; splice the recovered infix back
    // in at its own real anchor ("into thine house," ends the prefix half)
    // to reconstruct the source span.
    let split_at = rut_frag.text.find("into thine house,").expect("the genuine KJV prefix half must be present") + "into thine house,".len();
    let (rut_prefix, rut_suffix) = rut_frag.text.split_at(split_at);
    assert_eq!(
        format!("{rut_prefix} {rut_recovered} {}", rut_suffix.trim_start()),
        rut_span,
        "RUT 4:11: genuine-KJV-prefix + recovered-infix-prose + genuine-KJV-suffix must reconstruct the real source <strong> span exactly"
    );
}

/// Fix round 1 (review finding 5, LOW): the machine guard the invariant
/// deserves -- stored commentary prose must never contain its own excised
/// fragment's own text verbatim (LEMMA-EXCISION's own "never a byte of
/// the quoted KJV lemma" law, checked mechanically rather than merely
/// trusted). Full-corpus (cheap enough), PLUS the three fix-round-1-named
/// verses (two Type A, one Type B) checked explicitly by name so a future
/// refactor could never silently narrow the general sweep without also
/// breaking an explicitly-named assertion.
///
/// Deliberately scoped down from a first, broader cut (found necessary
/// empirically, not assumed -- two real, DIFFERENT false positives, kept
/// here as the honest derivation): a fragment must belong to a NARROW,
/// single-verse-exact `UnitKind::Verse` unit (`unit.verse_from ==
/// unit.verse_to == frag.verse`), and be at least 30 bytes. Neither
/// restriction hides a real bug:
///   - `ChapterIntro`/`PericopeIntro` units and any WIDE-ranged `Verse`
///     unit (a Type B pericope's own normal multi-verse span,
///     `kretzmann.rs`'s own module doc comment) never receive
///     OVER-EXCISION-GUARD-recovered text in the first place
///     (`parse_chapter`'s own `raw_units[unit_idx].text = prose_tail`
///     assignment only ever targets the SPECIFIC unit a lemma/quote span
///     opened, immediately, in the same match arm) -- checking them
///     against EVERY fragment merely overlapping their own wide range
///     manufactured two real false positives: PSA 45:1's own superscription
///     discussion legitimately explains the Hebrew term "upon Shoshannim,"
///     (glossing a term under discussion is completely normal commentary
///     style) while a DIFFERENT narrow fragment for the same verse also
///     carries that phrase; a 2 Corinthians pericope's own wide (verses
///     5-10) discussion prose likewise legitimately paraphrases content
///     also present in one of its own several covered verses' fragments.
///     Neither is the guard duplicating what IT excised -- both are
///     ordinary cross-references within a single continuous discussion.
///   - A trivially short fragment (a bare "he", "the", "and", ... -- real
///     single-word lemma slivers do occur, GEN 48:19's own "he" among
///     them) is near-certain to coincidentally occur as an ordinary
///     English word or short phrase inside SOME unrelated prose sentence;
///     that is not the duplication this invariant guards against (a real
///     excised CLAUSE ending up duplicated verbatim in stored prose). 30
///     bytes comfortably exceeds "upon Shoshannim,"'s own 16 while staying
///     well under the shortest real recovered instance this batch found
///     (MIN_PROSE_RUN_WORDS's own 3-word floor in `kretzmann.rs`).
#[test]
fn stored_prose_never_contains_its_own_excised_fragment_text() {
    let corpus = real_corpus();
    let mut checked = 0usize;
    for chapter in &corpus.chapters {
        for unit in &chapter.units {
            if unit.kind != kretzmann::UnitKind::Verse || unit.verse_from != unit.verse_to {
                continue;
            }
            for frag in &chapter.fragments {
                if frag.verse != unit.verse_from || frag.text.len() < 30 {
                    continue;
                }
                checked += 1;
                assert!(
                    !unit.text.contains(&frag.text),
                    "unit {} (verse {}) stored prose contains its own excised fragment text verbatim -- fragment {:?} inside unit text {:?}",
                    unit.id,
                    unit.verse_from,
                    frag.text,
                    unit.text
                );
            }
        }
    }
    assert!(checked > 30_000, "the full-corpus sweep must have actually run at real scale, got {checked} unit/fragment pairs checked");

    let exo = corpus.chapters.iter().find(|c| c.book_index == 1 && c.chapter == 20).expect("Exodus 20 must be in the corpus");
    let exo_frag = exo.fragments.iter().find(|f| f.verse == 12 && f.text.starts_with("that thy days")).expect("EXO 20:12's own recovered lemma fragment must exist");
    for u in exo.units.iter().filter(|u| u.verse_from <= 12 && u.verse_to >= 12) {
        assert!(!u.text.contains(&exo_frag.text), "EXO 20:12's own recovered prose must not contain its own excised lemma text");
    }
    let rut = corpus.chapters.iter().find(|c| c.book_index == 7 && c.chapter == 4).expect("Ruth 4 must be in the corpus");
    let rut_frag = rut.fragments.iter().find(|f| f.verse == 11 && f.text.contains("did build the house of Israel")).expect("RUT 4:11's own recovered lemma fragment must exist");
    for u in rut.units.iter().filter(|u| u.verse_from <= 11 && u.verse_to >= 11) {
        assert!(!u.text.contains(&rut_frag.text), "RUT 4:11's own recovered prose must not contain its own excised lemma text");
    }
    // A Type B (quote-block) instance too, so excluding wide-ranged units
    // from the general sweep above never leaves that whole template
    // unchecked: MAT 26:60's own recovered lemma ("But found none...").
    let mat = corpus.chapters.iter().find(|c| c.book_index == 39 && c.chapter == 26).expect("Matthew 26 must be in the corpus");
    let mat_frag = mat.fragments.iter().find(|f| f.verse == 60 && f.text.starts_with("but found none")).expect("MAT 26:60's own recovered lemma fragment must exist");
    for u in mat.units.iter().filter(|u| u.verse_from <= 60 && u.verse_to >= 60) {
        assert!(!u.text.contains(&mat_frag.text), "MAT 26:60's own recovered prose must not contain its own excised lemma text");
    }
}

/// KRETZ-ACCEPT-2 (owner ruling 2026-08-25, "commentary-comments===bible"):
/// composes the full corpus reading view over the WHOLE real canonical
/// Bible (all 31,102 verses, including the 70 Kretzmann-uncovered ones --
/// `kretzmann::compose_reading_view`'s own doc comment), strips every
/// comment segment, and byte-compares what remains to the whole canonical
/// Bible concatenated in the SAME canonical order -- EXACT, no equivalence
/// tiers, no residual. Trivially satisfiable by this architecture (verse
/// text is single-sourced; `compose_reading_view` never derives or
/// mutates it) -- the law's own value is guarding that construction
/// forever (spine coverage, verse-text mutation, compose ordering), per
/// the owner ruling's own text. KRETZ-ACCEPT-1 (above) stays the
/// parse-fidelity gate it is; this is the separate, composed-PRODUCT
/// identity.
#[test]
fn kretz_accept_2_composed_reading_view_strips_to_exactly_the_whole_canonical_bible() {
    let corpus = real_corpus();
    let canonical = real_canonical();
    assert_eq!(canonical.len(), 31102, "all 31,102 verses, including the 70 Kretzmann summarizes without a lemma of their own");

    let segments = kretzmann::compose_reading_view(&canonical, &corpus);
    let verse_segment_count = segments.iter().filter(|s| matches!(s, kretzmann::ReadingViewSegment::Verse(_))).count();
    assert_eq!(verse_segment_count, 31102, "exactly one Verse segment per canonical verse -- spine coverage, no skip, no duplicate");

    let comment_segment_count = segments.iter().filter(|s| matches!(s, kretzmann::ReadingViewSegment::Comment(_))).count();
    assert!(comment_segment_count > 50_000, "the real corpus's own comments must actually be present in the composed view, got {comment_segment_count}");

    let stripped = kretzmann::strip_comment_blocks(&segments);
    let whole_bible: String = canonical.values().map(|s| s.as_str()).collect();
    assert_eq!(stripped, whole_bible, "EXACT byte-compare, no equivalence tiers, no residual");
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
