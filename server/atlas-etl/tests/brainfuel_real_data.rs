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
/// KJV text, at every aligned position. RAW byte equality, no
/// normalization -- the honest, literal number, pinned as a regression
/// value. See this batch's own report for the full categorized breakdown;
/// every one of these mismatches was manually verified to be a
/// typographic/transcription-convention difference (whitespace; the
/// traditional `LORD`/`Lord` Tetragrammaton-case convention that this
/// app's OWN scrollmapper-sourced `data/raw/kjv.json` does not preserve;
/// Psalm-superscription-into-verse-1 and Psalm 119 acrostic
/// Hebrew-letter-header folding, present in our own source but not
/// brain-fuel's; a handful of epistle postscripts and genuine spelling
/// variants e.g. "Judea"/"Judaea") -- NEVER a case of a position holding a
/// genuinely different verse (zero "missing from our side" positions;
/// alignment itself is fully sound). Mismatches are disclosed, never
/// imported -- our KJV base stays authoritative.
#[test]
fn kjv_column_cross_check_mismatch_count_is_pinned() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/raw");
    let kjv_json = std::fs::read_to_string(dir.join("kjv.json")).expect("data/raw/kjv.json must exist");
    let (_canon, our_verses) = atlas_etl::kjv::parse(&kjv_json).expect("our own kjv.json must parse");

    let c = corpus();
    let report = atlas_etl::brainfuel::kjv_cross_check(&c, &our_verses, 20);

    assert_eq!(report.compared, 31_102, "every brain-fuel king_james position must be found on our own canonical side -- alignment is sound");
    assert_eq!(
        report.raw_mismatches, 9_274,
        "the pinned RAW mismatch count -- see this test's own doc comment and the batch report for the full categorized breakdown"
    );
}
