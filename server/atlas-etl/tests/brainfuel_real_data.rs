use std::collections::HashMap;

mod common;

use common::raw_dir;

fn corpus() -> atlas_etl::brainfuel::BrainFuelCorpus {
    atlas_etl::brainfuel::read_all(&raw_dir().join("brain-fuel-bible")).expect(
        "data/raw/brain-fuel-bible must exist -- run the CORP-1a vendoring step (data/fetch-raw.ps1) first",
    )
}

fn our_kjv_verses() -> HashMap<String, String> {
    let kjv_json = std::fs::read_to_string(raw_dir().join("kjv.json")).expect("data/raw/kjv.json must exist");
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
    assert_eq!(c.rows.len(), 31_102);
}

#[test]
fn per_edition_present_counts_are_exact() {
    let c = corpus();
    let present = &c.stats.per_edition_present;
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
    assert_eq!(report.restored, 5_473, "the case-class mismatches: positions where our text and brain-fuel's disagreed ONLY in casing");
    assert_eq!(report.already_agreeing, 21_828, "31,102 - 9,274 raw mismatches -- positions that were already byte-identical");

    assert_eq!(report.superscription_restored, 136, "superscription-class positions restored (batch-kjv-case2-brief.md controller decision 5)");
    assert_eq!(report.excluded, 3, "the exclusion table's own size -- SUPERSCRIPTION_EXCLUSIONS (controller decision 3)");
    assert_eq!(report.mirror_case_found, 0, "mirror-case (brain-fuel longer) was NOT expected (controller decision 1) -- confirmed absent over the real data");
    assert_eq!(
        report.skipped_mismatch, 3_662,
        "the remaining residue after KJV-CASE-2's own extraction: 3,801 - 136 superscription_restored - 3 excluded - 0 mirror_case_found = 3,662 \
         (whitespace conventions, spelling residue, epistle-subscription folding, etc -- KJV-CASE-2 does not touch this class, per its own scope)"
    );
}

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

#[test]
fn case_restoration_satisfies_the_case_only_law_over_every_real_position() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    let mut law1_whole_verse = 0usize;
    let mut law2_superscription = 0usize;
    let mut law3_untouched = 0usize;
    for row in &c.rows {
        let Some(theirs) = &row.king_james else { continue };
        let dot_ref = format!("{}.{}.{}", row.book.code(), row.chapter, row.verse);
        let Some(before) = our_verses.get(&dot_ref) else { continue };
        let after = restored_verses.get(&dot_ref).expect("restore_kjv_case must never drop a key that was present before");

        if before.eq_ignore_ascii_case(theirs) {
            assert!(before.eq_ignore_ascii_case(after), "CASE-ONLY LAW VIOLATED at {dot_ref}: before {before:?}, after {after:?} are not even case-fold-equal");
            law1_whole_verse += 1;
        } else if let atlas_etl::brainfuel::TailAlignment::OursSuffix { prefix_len } = atlas_etl::brainfuel::tail_align(before, theirs) {
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

#[test]
fn case_restoration_spot_verses_match_the_batch_briefs_own_four_examples() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, _report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    assert_eq!(
        restored_verses.get("GEN.2.4").map(String::as_str),
        Some(
            "These are the generations of the heavens and of the earth when they were created, in the day that the LORD God made the earth and the heavens,"
        )
    );

    assert_eq!(
        restored_verses.get("EZK.2.4").map(String::as_str),
        Some("For they are impudent children and stiffhearted. I do send thee unto them; and thou shalt say unto them, Thus saith the Lord GOD.")
    );

    assert_eq!(
        restored_verses.get("PSA.68.4").map(String::as_str),
        Some("Sing unto God, sing praises to his name: extol him that rideth upon the heavens by his name JAH, and rejoice before him.")
    );

    assert_eq!(restored_verses.len(), our_verses.len());
}

#[test]
fn superscription_class_spot_verses_match_the_kjv_case2_briefs_own_flagship_and_second_example() {
    let our_verses = our_kjv_verses();
    let c = corpus();
    let (restored_verses, report) = atlas_etl::brainfuel::restore_kjv_case(&c, &our_verses);

    assert_eq!(
        restored_verses.get("PSA.110.1").map(String::as_str),
        Some("A Psalm of David. The LORD said unto my Lord, Sit thou at my right hand, until I make thine enemies thy footstool.")
    );
    assert_ne!(restored_verses.get("PSA.110.1"), our_verses.get("PSA.110.1"), "PSA 110:1 must be a genuine restoration now, not byte-identical to the unrestored source");
    assert!(restored_verses["PSA.110.1"].starts_with("A Psalm of David. "), "the superscription prefix itself must survive byte-identical");

    assert_eq!(restored_verses.get("PSA.23.1").map(String::as_str), Some("A Psalm of David. The LORD is my shepherd; I shall not want."));
    assert_eq!(our_verses.get("PSA.23.1").map(String::as_str), Some("A Psalm of David. The Lord is my shepherd; I shall not want."), "confirms this really was an unrestored 'Lord' before this batch");

    assert!(report.superscription_restored > 0, "the report's own bucket must actually be nonzero over real data");
}

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
