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
    assert_eq!(
        report.compared,
        report.restored + report.already_agreeing + report.superscription_restored + report.excluded + report.mirror_case_found + report.skipped_mismatch,
        "every compared position falls into exactly one of the six buckets"
    );
    // CORP-1a's own manual categorization (progress.md, "~5,809 LORD/Lord
    // Tetragrammaton-case; ~136+ superscription/postscript folding") was an
    // approximation by design (its own "~" prefix) -- this batch's
    // mechanical, byte-exact case-fold sweep is the authoritative count,
    // independently verified against the real committed sources before
    // being pinned here (this file's own module doc comment methodology).
    // 5,473 + 3,801 = 9,274 -- reconciles EXACTLY with the pre-restoration
    // raw mismatch count `kjv_column_cross_check_mismatch_count_is_pinned`
    // above pins, confirming no position was miscounted either direction.
    // `restored`/`already_agreeing` are PASS-1's own two whole-verse
    // buckets -- batch KJV-CASE-2 never touches a position pass 1 already
    // decided, so both stay byte-for-byte identical to pass 1's own pins.
    assert_eq!(report.restored, 5_473, "the case-class mismatches: positions where our text and brain-fuel's disagreed ONLY in casing");
    assert_eq!(report.already_agreeing, 21_828, "31,102 - 9,274 raw mismatches -- positions that were already byte-identical");

    // Batch KJV-CASE-2 (batch-kjv-case2-brief.md controller decisions 1-5):
    // subdivides pass 1's own 3,801-position `skipped_mismatch` bucket via
    // the TAIL-ALIGNMENT RULE. 139 positions tail-aligned over the real
    // data; a real-data word-level diff (this batch's own report has the
    // full methodology) flagged exactly 3 as brain-fuel `king_james`
    // column ARTIFACTS rather than genuine folded-in superscriptions (now
    // named in `SUPERSCRIPTION_EXCLUSIONS`) -- 139 - 3 = 136 genuine
    // superscription-class restorations, reconciling EXACTLY with the
    // batch brief's own "~136-class" estimate. Zero mirror-case positions
    // found (controller decision 1: "NOT expected").
    assert_eq!(report.superscription_restored, 136, "superscription-class positions restored (batch-kjv-case2-brief.md controller decision 5)");
    assert_eq!(report.excluded, 3, "the exclusion table's own size -- SUPERSCRIPTION_EXCLUSIONS (controller decision 3)");
    assert_eq!(report.mirror_case_found, 0, "mirror-case (brain-fuel longer) was NOT expected (controller decision 1) -- confirmed absent over the real data");
    assert_eq!(
        report.skipped_mismatch, 3_662,
        "the remaining residue after KJV-CASE-2's own extraction: 3,801 - 136 superscription_restored - 3 excluded - 0 mirror_case_found = 3,662 \
         (whitespace conventions, spelling residue, epistle-subscription folding, etc -- KJV-CASE-2 does not touch this class, per its own scope)"
    );
}

/// The cross-check RE-RUN against the case-restored verse map (batch
/// brief controller decision 4: "the CORP-1a cross-check test's raw-
/// mismatch assertion UPDATES same-commit... case-class mismatches
/// collapse to ~0; the residual number is the new asserted value,
/// categorized"). Restoration adopts brain-fuel's own casing verbatim at
/// every WHOLE-VERSE position it touches (byte-for-byte, by
/// `restore_verse_case`'s own construction), so comparing the RESTORED
/// map against brain-fuel's own column again must show those exact
/// positions now byte-EQUAL: after PASS 1 alone, the residual raw-
/// mismatch count was precisely `skipped_mismatch` (the non-case residue,
/// never touched because it was never safe to touch).
///
/// Batch KJV-CASE-2 changes this relationship: `kjv_cross_check` does a
/// RAW WHOLE-STRING comparison, but a superscription-class restoration
/// only fixes the ALIGNED TAIL's case -- the untouched prefix (the
/// superscription itself) means the FULL restored string is still never
/// byte-equal to brain-fuel's own (superscription-free) column, even
/// though the position is no longer "skipped" in `CaseRestorationReport`'s
/// own sense. So post-KJV-CASE-2, the raw mismatch count no longer equals
/// `skipped_mismatch` alone -- it equals the sum of every bucket that does
/// NOT achieve whole-string equality with `theirs`: `skipped_mismatch +
/// superscription_restored + excluded + mirror_case_found` (only pass-1's
/// own `restored`/`already_agreeing` ever reach whole-string equality).
/// The raw count itself stays 3,801 -- unchanged from pass 1 -- because
/// KJV-CASE-2 never achieves whole-string equality anywhere it touches;
/// what changes is how that 3,801 is now categorized under the hood.
#[test]
fn kjv_column_cross_check_mismatch_count_after_case_restoration_is_pinned() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, restoration_report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    let report = atlas_etl::brainfuel::kjv_cross_check(&c, &restored_verses, 20);

    assert_eq!(report.compared, 31_102);
    assert_eq!(
        report.raw_mismatches,
        restoration_report.skipped_mismatch + restoration_report.superscription_restored + restoration_report.excluded + restoration_report.mirror_case_found,
        "post-restoration, every remaining RAW (whole-string) mismatch is exactly a position that never achieved whole-string \
         equality with brain-fuel's own column -- the pass-1 case class collapsed to zero by construction, but KJV-CASE-2's own \
         superscription-tail restorations never achieve whole-string equality either (the untouched prefix guarantees that), so \
         they still show as a raw mismatch even though they are no longer 'skipped'"
    );
    assert_eq!(
        report.raw_mismatches, 3_801,
        "unchanged from pass 1's own pin -- KJV-CASE-2 recategorizes WHICH bucket each of these 3,801 positions falls into, \
         but never reduces the raw whole-string mismatch count itself (superscription-class restorations fix only the tail, \
         never the surviving prefix, so they can never become whole-string-equal to brain-fuel's own superscription-free column)"
    );
}

/// Batch brief controller decision 2, THE CASE-ONLY LAW, proved over
/// every one of the real 31,102 aligned positions (not sampled): (1) for
/// every position the pass TOUCHED (case-fold-equal to brain-fuel's own
/// column, pass 1's own whole-verse class), before/after are IDENTICAL
/// under case-folding -- any byte difference that is not purely a case
/// difference would fail this assertion; (2) for every position the pass
/// SKIPPED (folded texts mismatch, neither whole-verse-equal nor tail-
/// aligned), before/after are BYTE-IDENTICAL -- the pass provably never
/// touches them. Both assertions run inside the SAME sweep so a single
/// real-data pass witnesses the whole law, not two independently-sampled
/// claims.
///
/// Batch KJV-CASE-2's own extension (that batch's brief, controller
/// decision 2: "THE CASE-ONLY LAW EXTENDS, same rigor") adds a THIRD
/// bucket, superscription-tail-aligned positions, with a stronger
/// two-part guarantee: the WHOLE verse stays case-fold-identical
/// before/after (law 1's own guarantee, extended) AND the PREFIX region
/// specifically is BYTE-IDENTICAL before/after (never merely case-fold-
/// identical -- the superscription's own characters, not just its
/// letters' case, must survive untouched).
#[test]
fn case_restoration_satisfies_the_case_only_law_over_every_real_position() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    let mut law1_whole_verse = 0usize; // pass-1 class: before/after must stay case-fold-equal (whole verse).
    let mut law2_superscription = 0usize; // batch KJV-CASE-2 class: whole-verse case-fold-equal AND prefix byte-identical.
    let mut law3_untouched = 0usize; // excluded/mirror-case/residue: before/after must be byte-identical.
    for row in &c.rows {
        let Some(theirs) = &row.king_james else { continue };
        let dot_ref = format!("{}.{}.{}", row.book.code(), row.chapter, row.verse);
        let Some(before) = our_verses.get(&dot_ref) else { continue };
        let after = restored_verses.get(&dot_ref).expect("restore_kjv_case must never drop a key that was present before");

        if before.eq_ignore_ascii_case(theirs) {
            assert!(before.eq_ignore_ascii_case(after), "CASE-ONLY LAW VIOLATED at {dot_ref}: before {before:?}, after {after:?} are not even case-fold-equal");
            law1_whole_verse += 1;
        } else if before.len() > theirs.len() && before[before.len() - theirs.len()..].eq_ignore_ascii_case(theirs) {
            // Superscription-tail-aligned class (excluded positions ALSO
            // satisfy this shape by construction -- they still pass both
            // assertions below since `after == before` byte-identically).
            let prefix_len = before.len() - theirs.len();
            assert!(before.eq_ignore_ascii_case(after), "CASE-ONLY LAW VIOLATED at {dot_ref}: whole-verse case-fold identity broken by a superscription-tail restoration");
            assert_eq!(
                &before[..prefix_len],
                &after[..prefix_len],
                "PREFIX LAW VIOLATED at {dot_ref}: the superscription prefix region must be BYTE-IDENTICAL before/after, never merely case-fold-identical"
            );
            law2_superscription += 1;
        } else {
            assert_eq!(after, before, "UNTOUCHED LAW VIOLATED at {dot_ref}: neither whole-verse-equal nor tail-aligned, but the pass changed a byte anyway");
            law3_untouched += 1;
        }
    }

    assert_eq!(law1_whole_verse + law2_superscription + law3_untouched, 31_102, "every compared position was swept by exactly one of the three law checks above");
    assert_eq!(law1_whole_verse, report.restored + report.already_agreeing, "law-1-eligible positions are exactly restored + already_agreeing");
    assert_eq!(law2_superscription, report.superscription_restored + report.excluded, "law-2-eligible (tail-aligned) positions are exactly superscription_restored + excluded (both shapes tail-align; excluded ones simply restore nothing)");
    assert_eq!(law3_untouched, report.mirror_case_found + report.skipped_mismatch, "everything left is either the NOT-expected mirror-case shape or true residue");
}

/// Batch brief controller decision 3 (batch KJV-CASE), the four ORIGINAL
/// spot laws, read back from the ACTUAL restoration output over real data
/// (never from intent) -- re-verified here to prove batch KJV-CASE-2 does
/// not regress pass 1 (that batch's own brief, controller decision 4:
/// "the four pass-1 spot verses... still hold").
///
/// PSA 110:1 was pass 1's own "load-bearing example" IN NAME -- but a
/// superscription-folded position IN FACT (this test file's own history:
/// before batch KJV-CASE-2, this test asserted PSA 110:1 stayed
/// byte-identical before/after, since pass 1's whole-verse gate cannot
/// reach a folded-in superscription). Batch KJV-CASE-2 (controller
/// decisions 1+4, "the flagship becomes a permanent law") now DOES reach
/// it, via the TAIL-ALIGNMENT RULE: see
/// `superscription_class_spot_verses_match_the_kjv_case2_briefs_own_flagship_and_second_example`
/// below for PSA 110:1's own NEW, restored assertion.
#[test]
fn case_restoration_spot_verses_match_the_batch_briefs_own_four_examples() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, _report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

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

    // The verse count is unaffected -- pass-1's own no-keys-added-or-removed guarantee still holds under KJV-CASE-2's extension.
    assert_eq!(restored_verses.len(), our_verses.len());
}

// ---------------------------------------------------------------------
// Batch KJV-CASE-2 (owner ruling, verbatim "do it (superscript fix)" --
// batch-kjv-case2-brief.md): the superscription-aware extension's own
// spot laws (controller decision 4), read back from the ACTUAL
// restoration output over real data.
// ---------------------------------------------------------------------

/// PSA 110:1, the flagship (controller decision 4: "PSA 110:1 contains
/// BOTH 'The LORD said' and 'my Lord' (the flagship becomes a permanent
/// law)") -- and a second superscription-class Psalm, PSA 23:1, "picked
/// from the run" as decision 4 asks.
#[test]
fn superscription_class_spot_verses_match_the_kjv_case2_briefs_own_flagship_and_second_example() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    // PSA 110:1 -- the superscription "A Psalm of David. " keeps its own
    // canonical casing and characters UNTOUCHED (byte-identical prefix);
    // the aligned tail's Tetragrammaton "the Lord" is promoted to
    // "the LORD" (brain-fuel's own casing); the SECOND occurrence, "my
    // Lord" (Adonai, David's own lord), correctly stays initial-cap only
    // -- both Tetragrammaton-referent forms now correctly distinguished
    // in one verse, exactly as the KJV convention and this law both
    // require.
    assert_eq!(
        restored_verses.get("PSA.110.1").map(String::as_str),
        Some("A Psalm of David. The LORD said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool.")
    );
    assert_ne!(restored_verses.get("PSA.110.1"), our_verses.get("PSA.110.1"), "PSA 110:1 must be a genuine restoration now, not byte-identical to the unrestored source");
    assert!(restored_verses["PSA.110.1"].starts_with("A Psalm of David. "), "the superscription prefix itself must survive byte-identical");

    // PSA 23:1 -- picked from the run (controller decision 4): "The LORD
    // is my shepherd" (Tetragrammaton), superscription "A Psalm of
    // David. " kept untouched ahead of it.
    assert_eq!(restored_verses.get("PSA.23.1").map(String::as_str), Some("A Psalm of David. The LORD is my shepherd; I shall not want."));
    assert_eq!(our_verses.get("PSA.23.1").map(String::as_str), Some("A Psalm of David. The Lord is my shepherd; I shall not want."), "confirms this really was an unrestored 'Lord' before this batch");

    assert!(report.superscription_restored > 0, "the report's own bucket must actually be nonzero over real data");
}

/// The EXCLUSION LIST (controller decision 3): every named position is
/// asserted BYTE-IDENTICAL before/after -- "restore nothing... asserted
/// untouched" is a claim about REAL bytes, verified here, not merely the
/// report's own bucket arithmetic (which `kjv_case_restoration_counts_
/// are_pinned` above already covers).
#[test]
fn superscription_exclusions_are_provably_untouched() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, _report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    assert_eq!(atlas_etl::brainfuel::SUPERSCRIPTION_EXCLUSIONS.len(), 3, "the exclusion table's own size, as shipped -- update this alongside the table itself");
    for (dot_ref, reason) in atlas_etl::brainfuel::SUPERSCRIPTION_EXCLUSIONS {
        assert!(!reason.is_empty(), "{dot_ref} must carry a one-line reason (controller decision 3)");
        assert_eq!(
            restored_verses.get(*dot_ref),
            our_verses.get(*dot_ref),
            "excluded position {dot_ref} must be byte-identical before/after -- '{reason}'"
        );
    }
    // The real, disclosed content this batch's own report quotes verbatim.
    assert_eq!(
        our_verses.get("PSA.70.1").map(String::as_str),
        Some("To the chief Musician, A Psalm of David, to bring to remembrance. Make haste, O God, to deliver me; make haste to help me, O Lord.")
    );
    assert_eq!(
        our_verses.get("PSA.92.1").map(String::as_str),
        Some("A Psalm or Song for the sabbath day. It is a good thing to give thanks unto the Lord, and to sing praises unto thy name, O most High:")
    );
    assert_eq!(
        our_verses.get("ACT.9.29").map(String::as_str),
        Some("And he spake boldly in the name of the Lord Jesus, and disputed against the Grecians: but they went about to slay him.")
    );
}

