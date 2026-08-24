//! Batch CORP-1a real-data tests over the actual vendored
//! `data/raw/brain-fuel-bible/` (929 OT + 260 NT chapter files, pinned
//! commit `94d44842cb242e8aa840330748e03d2803f2a7c1` -- see
//! `data/raw/README.md`). Requirement 9 of the batch brief: "per-edition
//! ingested verse counts (exact totals asserted, from the parser's own
//! tally)... KJV cross-check test (mismatch count asserted at the
//! disclosed number)."
//!
//! Every count below was independently verified by a full Python sweep of
//! the real files (not guessed, not derived from the source repo's own
//! README) before being pinned here -- see this batch's own report for the
//! full methodology.

use std::collections::HashMap;
use std::path::Path;

fn brainfuel_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw/brain-fuel-bible")
}

fn corpus() -> atlas_etl::brainfuel::BrainFuelCorpus {
    atlas_etl::brainfuel::read_all(&brainfuel_root()).expect(
        "data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step (data/fetch-raw.ps1) first",
    )
}

/// Batch KJV-CASE: our own canonical `"BOOK.CH.V" -> text` map, parsed
/// fresh from the real committed `data/raw/kjv.json` -- the SAME source
/// `kjv_column_cross_check_mismatch_count_is_pinned` below already reads,
/// factored out so the restoration-pass tests below can share it without
/// each re-deriving its own path.
fn our_kjv_verses() -> HashMap<String, String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    atlas_etl::kjv::parse(&kjv_json).expect("our own kjv.json must parse").1
}

#[test]
fn chapter_file_counts_match_the_real_kjv_skeleton() {
    let c = corpus();
    assert_eq!(c.stats.ot_chapters, 929, "929 OT chapter files (39 books)");
    assert_eq!(c.stats.nt_chapters, 260, "260 NT chapter files (27 books)");
}

#[test]
fn total_verse_rows_match_the_real_kjv_total() {
    let c = corpus();
    // The SAME 31,102 total this app's own compiled KJV text carries
    // (server/atlas-server/tests/graph_conformance.rs pins the identical
    // number against data/raw/kjv.json) -- brain-fuel's own skeleton is
    // genuinely KJV-shaped, not a coincidence of similar size.
    assert_eq!(c.rows.len(), 31_102);
}

#[test]
fn per_edition_present_counts_are_exact() {
    let c = corpus();
    let present = &c.stats.per_edition_present;
    // OT-applicable editions: 23,145 OT verses minus each edition's own
    // disclosed absent-marker count; Douay-Rheims/Hebrew Masoretic never
    // apply to NT at all in this dataset (verified, module doc comment).
    assert_eq!(present.get("latin_vulgate").copied(), Some(23_135 + 7_957), "Vulgate: OT (23,145 - 10 absent) + full NT");
    assert_eq!(present.get("hebrew_masoretic").copied(), Some(23_145), "WLC: full OT, OT-only edition");
    assert_eq!(present.get("douay_rheims").copied(), Some(23_132), "Douay-Rheims: OT (23,145 - 13 absent), OT-only in this dataset");
    assert_eq!(present.get("finnish_biblia").copied(), Some(31_102), "Biblia 1776: full OT + full NT, zero absences (identity-placed)");
    assert_eq!(present.get("swedish_karl_xii").copied(), Some(23_145 + 7_954), "Karl XII: full OT + NT (7,957 - 3 absent)");
    assert_eq!(present.get("greek_textus_receptus").copied(), Some(7_957), "Greek TR: full NT, NT-only edition");
}

#[test]
fn per_edition_absent_marker_counts_are_exact() {
    let c = corpus();
    let absent = &c.stats.per_edition_absent;
    assert_eq!(absent.get("latin_vulgate").copied(), Some(10), "ten OT verses merged into the preceding verse in the Vulgate tradition");
    assert_eq!(absent.get("douay_rheims").copied(), Some(13), "thirteen KJV verses merged/absent in the Douay tradition");
    assert_eq!(absent.get("swedish_karl_xii").copied(), Some(3), "three NT verses absent in Karl XII");
    assert_eq!(absent.get("hebrew_masoretic"), None, "WLC: zero absences over the real data (no key at all)");
    assert_eq!(absent.get("finnish_biblia"), None, "Biblia 1776: zero absences (identity-placed)");
    assert_eq!(absent.get("greek_textus_receptus"), None, "Greek TR: zero absences over the real NT data");
}

#[test]
fn zero_anomalies_over_the_real_data() {
    // No edition is ever empty-and-unmarked over the whole real corpus --
    // "absence is data, never an empty string" holds by construction, not
    // by luck (brainfuel.rs's own module doc comment).
    assert_eq!(corpus().stats.anomalies, 0);
}

#[test]
fn versification_provenance_notes_are_disclosed_but_never_imported() {
    let c = corpus();
    let src = &c.stats.per_edition_src_notes;
    assert_eq!(src.get("latin_vulgate").copied(), Some(2_835));
    assert_eq!(src.get("hebrew_masoretic").copied(), Some(1_971));
    assert_eq!(src.get("douay_rheims").copied(), Some(2_835));
    assert_eq!(src.get("swedish_karl_xii").copied(), Some(1_303 + 1), "1,303 in OT + 1 in NT");
    assert_eq!(src.get("finnish_biblia"), None, "identity-placed, no src notes");
    assert_eq!(src.get("greek_textus_receptus"), None);
}

/// One byte-verbatim spot verse per edition (batch brief requirement 9),
/// read directly against the real vendored JSON.
#[test]
fn spot_verses_are_byte_verbatim_against_the_real_source() {
    let c = corpus();
    let gen1_1: Vec<_> = c.rows.iter().filter(|r| r.chapter == 1 && r.verse == 1 && r.book.code() == "GEN").collect();
    assert_eq!(gen1_1.len(), 1);
    let renderings: HashMap<&str, &str> = gen1_1[0].renderings.iter().map(|(e, t)| (*e, t.as_str())).collect();
    assert_eq!(renderings.get("latin_vulgate"), Some(&"In principio creavit Deus cælum et terram. "));
    assert_eq!(
        renderings.get("hebrew_masoretic"),
        Some(&"בְּרֵאשִׁ֖ית בָּרָ֣א אֱלֹהִ֑ים אֵ֥ת הַשָּׁמַ֖יִם וְאֵ֥ת הָאָֽרֶץ׃")
    );
    assert_eq!(renderings.get("douay_rheims"), Some(&"In the beginning God created heaven, and earth."));
    assert_eq!(renderings.get("finnish_biblia"), Some(&"Alussa loi Jumala taivaan ja maan. "));
    assert_eq!(renderings.get("swedish_karl_xii"), Some(&"J Begynnelsen skapade Gudh Himmel och Jord."));
    assert!(renderings.get("greek_textus_receptus").is_none(), "TR does not apply to the OT");

    let jhn1_1: Vec<_> = c.rows.iter().filter(|r| r.chapter == 1 && r.verse == 1 && r.book.code() == "JHN").collect();
    assert_eq!(jhn1_1.len(), 1);
    let jn_renderings: HashMap<&str, &str> = jhn1_1[0].renderings.iter().map(|(e, t)| (*e, t.as_str())).collect();
    assert_eq!(jn_renderings.get("greek_textus_receptus"), Some(&"ἐν ἀρχῇ ἦν ὁ λόγος καὶ ὁ λόγος ἦν πρὸς τὸν θεόν καὶ θεὸς ἦν ὁ λόγος"));
}

/// The exact real absent-marker example this batch's own report and
/// `brainfuel.rs` module doc comment cite: 1 Chronicles 11:47.
#[test]
fn real_absent_marker_example_1_chronicles_11_47() {
    let c = corpus();
    let row = c.rows.iter().find(|r| r.book.code() == "1CH" && r.chapter == 11 && r.verse == 47).expect("1CH.11.47 must exist");
    let editions: Vec<&str> = row.renderings.iter().map(|(e, _)| *e).collect();
    assert!(!editions.contains(&"latin_vulgate"), "{editions:?}");
    assert!(!editions.contains(&"douay_rheims"), "{editions:?}");
    assert!(editions.contains(&"hebrew_masoretic"), "{editions:?}");
    assert!(editions.contains(&"finnish_biblia"), "{editions:?}");
    assert!(editions.contains(&"swedish_karl_xii"), "{editions:?}");
}

/// The owner-ordered KJV column cross-check (batch brief requirement 5):
/// brain-fuel's own `king_james` column against this app's own canonical
/// KJV text, at every aligned position, RAW byte equality, no
/// normalization. PRE-restoration pin (Batch CORP-1a's own original
/// finding, kept here as the historical fact it is: `data/raw/kjv.json`
/// on disk is never edited by this batch -- gitignored/fetched, per the
/// KJV-CASE brief's own controller decision 1 -- so the RAW byte-equality
/// count against the UNRESTORED text never changes). See
/// `kjv_column_cross_check_mismatch_count_after_case_restoration_is_pinned`
/// below for the batch KJV-CASE update: the SAME comparison, run against
/// the case-restored verse map instead, which is what actually reaches
/// the compiled graph.
#[test]
fn kjv_column_cross_check_mismatch_count_is_pinned() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let report = atlas_etl::brainfuel::kjv_cross_check(&c, &our_verses, 20);

    assert_eq!(report.compared, 31_102, "every brain-fuel king_james position must be found on our own canonical side -- alignment is sound");
    assert_eq!(
        report.raw_mismatches, 9_274,
        "the pinned RAW mismatch count against UNRESTORED text -- see this test's own doc comment; \
         CORP-1a categorized this as ~2,878 whitespace + ~5,809 LORD/Lord Tetragrammaton-case + ~136+ \
         superscription/postscript folding + a small spelling-variant residue, zero content substitution"
    );
}

// ---------------------------------------------------------------------
// Batch KJV-CASE (owner ruling, "KJV-CASE-1 APPROVED... 3. yes" --
// .superpowers/sdd/2026-08-17-bible-atlas-m1/batch-kjv-case-brief.md): the
// Tetragrammaton LORD/Lord case-restoration pass, verified over the real
// committed data. KJV INERRANCY DIRECTIVE: restoration, never revision --
// every test below either proves a genuine restoration (case-fold-equal
// positions) or proves a position was left PROVABLY untouched (folded
// texts disagree).
// ---------------------------------------------------------------------

/// Batch brief controller decision 4: "total positions case-restored
/// (expect the ~5,809 class, exact number from the run), positions
/// skipped due to folded mismatch (expect ~3,465 class, exact), positions
/// already agreeing (untouched)." Independently verified against the real
/// vendored corpus + the real committed `data/raw/kjv.json` before being
/// pinned here, the SAME methodology this file's own module doc comment
/// describes for every other pinned count in it.
#[test]
fn kjv_case_restoration_counts_are_pinned() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (_restored, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    assert_eq!(report.compared, 31_102, "matches kjv_cross_check's own compared count exactly -- same alignment, same two inputs");
    assert_eq!(report.compared, report.restored + report.already_agreeing + report.skipped_mismatch, "every compared position falls into exactly one bucket");
    // CORP-1a's own manual categorization (progress.md, "~5,809 LORD/Lord
    // Tetragrammaton-case; ~136+ superscription/postscript folding") was an
    // approximation by design (its own "~" prefix) -- this batch's
    // mechanical, byte-exact case-fold sweep is the authoritative count,
    // independently verified against the real committed sources before
    // being pinned here (this file's own module doc comment methodology).
    // 5,473 + 3,801 = 9,274 -- reconciles EXACTLY with the pre-restoration
    // raw mismatch count `kjv_column_cross_check_mismatch_count_is_pinned`
    // above pins, confirming no position was miscounted either direction.
    assert_eq!(report.restored, 5_473, "the case-class mismatches: positions where our text and brain-fuel's disagreed ONLY in casing");
    assert_eq!(report.skipped_mismatch, 3_801, "the non-case mismatches: whitespace/superscription-folding/spelling residue, left untouched");
    assert_eq!(report.already_agreeing, 21_828, "31,102 - 9,274 raw mismatches -- positions that were already byte-identical");
}

/// The cross-check RE-RUN against the case-restored verse map (batch
/// brief controller decision 4: "the CORP-1a cross-check test's raw-
/// mismatch assertion UPDATES same-commit... case-class mismatches
/// collapse to ~0; the residual number is the new asserted value,
/// categorized"). Restoration adopts brain-fuel's own casing verbatim at
/// every position it touches (byte-for-byte, by `restore_verse_case`'s
/// own construction), so comparing the RESTORED map against brain-fuel's
/// own column again must show those exact positions now byte-EQUAL: the
/// residual raw-mismatch count after restoration is precisely the
/// `skipped_mismatch` count `kjv_case_restoration_counts_are_pinned`
/// above already pins -- the non-case residue (whitespace conventions,
/// Psalm-superscription/Ps119-acrostic folding, spelling residue), never
/// touched because it was never safe to touch.
#[test]
fn kjv_column_cross_check_mismatch_count_after_case_restoration_is_pinned() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, restoration_report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    let report = atlas_etl::brainfuel::kjv_cross_check(&c, &restored_verses, 20);

    assert_eq!(report.compared, 31_102);
    assert_eq!(
        report.raw_mismatches, restoration_report.skipped_mismatch,
        "post-restoration, every remaining raw mismatch must be exactly a skipped (folded-mismatch) position -- \
         the case class has collapsed to zero by construction"
    );
    assert_eq!(report.raw_mismatches, 3_801, "the residual, categorized non-case mismatch count -- the new pinned value this batch's own report discloses");
}

/// Batch brief controller decision 2, THE CASE-ONLY LAW, proved over
/// every one of the real 31,102 aligned positions (not sampled): (1) for
/// every position the pass TOUCHED (case-fold-equal to brain-fuel's own
/// column), before/after are IDENTICAL under case-folding -- any byte
/// difference that is not purely a case difference would fail this
/// assertion; (2) for every position the pass SKIPPED (folded texts
/// mismatch), before/after are BYTE-IDENTICAL -- the pass provably never
/// touches them. Both assertions run inside the SAME sweep so a single
/// real-data pass witnesses the whole law, not two independently-sampled
/// claims.
#[test]
fn case_restoration_satisfies_the_case_only_law_over_every_real_position() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    let mut law1_touched = 0usize; // case-fold-equal positions: before/after must stay case-fold-equal.
    let mut law2_skipped = 0usize; // folded-mismatch positions: before/after must be byte-identical.
    for row in &c.rows {
        let Some(theirs) = &row.king_james else { continue };
        let dot_ref = format!("{}.{}.{}", row.book.code(), row.chapter, row.verse);
        let Some(before) = our_verses.get(&dot_ref) else { continue };
        let after = restored_verses.get(&dot_ref).expect("restore_kjv_case must never drop a key that was present before");

        if before.eq_ignore_ascii_case(theirs) {
            assert!(before.eq_ignore_ascii_case(after), "CASE-ONLY LAW VIOLATED at {dot_ref}: before {before:?}, after {after:?} are not even case-fold-equal");
            law1_touched += 1;
        } else {
            assert_eq!(after, before, "SKIP LAW VIOLATED at {dot_ref}: folded texts disagree with brain-fuel, but the pass changed a byte anyway");
            law2_skipped += 1;
        }
    }

    assert_eq!(law1_touched + law2_skipped, 31_102, "every compared position was swept by exactly one of the two law checks above");
    assert_eq!(law1_touched, report.restored + report.already_agreeing, "law-1-eligible positions are exactly restored + already_agreeing");
    assert_eq!(law2_skipped, report.skipped_mismatch);
}

/// Batch brief controller decision 3, the four spot laws, read back from
/// the ACTUAL restoration output over real data (never from intent).
///
/// PSA 110:1 is this batch's own "load-bearing example" IN NAME -- but a
/// superscription-folded position IN FACT, verified byte-for-byte against
/// both real sources: our own `data/raw/kjv.json` folds the Psalm's own
/// superscription into verse 1's text ("A Psalm of David. The Lord said
/// unto my Lord..."), while brain-fuel's `king_james` column does not
/// carry that superscription at all ("The LORD said unto my Lord...").
/// The two are NOT case-fold-equal (different length), so this exact
/// position is a `skipped_mismatch` -- disclosed here rather than forced,
/// per controller decision 1's own literal "for each verse position"
/// (whole-verse) comparison unit, and per this batch's own instruction
/// that superscription-folding positions are among the folded-text
/// mismatches this pass must provably skip. This is the case-only law's
/// own second assertion, witnessed concretely on the batch's own
/// flagship verse: restoration is honest about what it can and cannot
/// touch, never forcing a result past what "characters unchanged, case
/// only" allows.
#[test]
fn case_restoration_spot_verses_match_the_batch_briefs_own_four_examples() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, _report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    // PSA 110:1 -- skipped (superscription-folded), UNTOUCHED, verbatim.
    assert_eq!(
        restored_verses.get("PSA.110.1").map(String::as_str),
        our_verses.get("PSA.110.1").map(String::as_str),
        "PSA 110:1 is a superscription-folded mismatch position -- must be byte-identical before/after, never touched"
    );
    assert_eq!(
        restored_verses.get("PSA.110.1").map(String::as_str),
        Some("A Psalm of David. The Lord said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool.")
    );

    // GEN 2:4-class: "LORD God" (YHWH Elohim) -- a genuine restoration.
    assert_eq!(
        restored_verses.get("GEN.2.4").map(String::as_str),
        Some(
            "These are the generations of the heavens and of the earth when they were created, in the day that the LORD God made the earth and the heavens,"
        )
    );

    // EZK 2:4 -- "Lord GOD" (Adonai YHWH), the Ezekiel-class convention.
    assert_eq!(
        restored_verses.get("EZK.2.4").map(String::as_str),
        Some("For they are impudent children and stiffhearted. I do send thee unto them; and thou shalt say unto them, Thus saith the Lord GOD.")
    );

    // PSA 68:4 -- JAH.
    assert_eq!(
        restored_verses.get("PSA.68.4").map(String::as_str),
        Some("Sing unto God, sing praises to his name: extol him that rideth upon the heavens by his name JAH, and rejoice before him.")
    );
}
