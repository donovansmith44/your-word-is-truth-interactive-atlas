//! Batch T2 requirement 5: "Coverage completeness is TESTED: a test derives
//! the expected section list from the curated Robertson table itself (not
//! hardcoded counts) and fails loud on any gap between table and compiled
//! output."
//!
//! `ROBERTSON_TABLE` below IS "the curated Robertson table" the requirement
//! names -- this project's own independently-verified transcription of
//! A.T. Robertson's *A Harmony of the Gospels* (1922) own Analytical
//! Outline/Reference Table, fetched from two Project Gutenberg #36264
//! mirrors and cross-checked (see LICENSES.md's own "Robertson's Harmony
//! of the Gospels" section, and batch-t2-report.md for the fetch record).
//! Every one of its 185 slots (sections 1-184, with 128 split into 128a/
//! 128b, matching Robertson's own table) must be accounted for in the REAL
//! curated data -- read live from `data/curated/` here, not a fixture --
//! by EXACTLY ONE of three honest outcomes: (1) its own literal
//! `robertson_section` citation appears somewhere in the curated data, (2)
//! it is named in `SUBSUMED`, a disclosed list of sections whose entire
//! content already falls within a DIFFERENT curated event's own wider
//! witness range (documented at each subsuming event's own `ref_note` in
//! `events-extra.toml`), or (3) it is named in `HONESTLY_OMITTED`, a
//! disclosed list of sections with no Gospel text to cite at all. A
//! section number belonging to none of the three is a real, uncaught gap
//! -- this test fails loud, naming it, rather than passing silently.

use std::collections::HashSet;

use atlas_core::data::{Canon, Event, EventWitness};

/// Robertson's own full section list, 1-184, with 128 split into 128a/128b
/// exactly as his own table does (no other section carries a letter
/// suffix). One entry per slot, as `&str` so "128a"/"128b" fit the same
/// list as the bare numbers.
fn robertson_table() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in 1..=184 {
        if n == 128 {
            out.push("128a".to_string());
            out.push("128b".to_string());
        } else {
            out.push(n.to_string());
        }
    }
    out
}

/// Batch T2's own disclosed "subsumed by a wider sibling" list -- each of
/// these Robertson sections is fully covered by a DIFFERENT event's own
/// witness ranges (a Batch-T-established "disclosed widening" precedent,
/// e.g. pw_golgotha's own §164 already covering the crucifixion's full
/// scope), so it never appears as its own literal `robertson_section`
/// citation. See each subsuming event's own `ref_note` in
/// events-extra.toml for the specific disclosure:
/// - 153 (the arrest) -> pw_gethsemane (§152)
/// - 165 (three hours of darkness) -> pw_golgotha (§164)
/// - 169/170/172/174 (women set out; the earthquake; women report + Peter
///   and John visit the tomb; appearance to the other women) ->
///   pw_jerusalem_resurrection (§171) -- 172 added after this test's own
///   FIRST real run caught it missing from the ref_note disclosure (the
///   subsumption itself was correct from the start; only the written
///   disclosure had been incomplete) -- exactly the gap-class this test
///   exists to catch, confirmed working against real data before it ever
///   reached review.
/// - 177 (the two disciples' own report) -> pw_emmaus (§176)
///
/// Fix round 1 (review finding M-2): this list used to be a bare
/// `HashSet<&str>` of section numbers, checked only for bookkeeping
/// (real section number, not stale) -- never for CONTENT (does the
/// claimed subsuming event's own actual verse/witness coverage really
/// reach the subsumed section's own citations?). `SUBSUMPTION_CLAIMS`
/// below is now the single source of truth (this function derives its
/// own `HashSet` from it, so the two can never drift) and carries enough
/// structure -- the claimed subsuming event id, and Robertson's own
/// literal per-book verse citation for the SUBSUMED section itself, NOT
/// the wider subsuming section's own citation -- for
/// `every_subsumption_claim_is_content_verified_against_real_curated_data`
/// below to check real containment, not just list membership.
fn subsumed() -> HashSet<&'static str> {
    SUBSUMPTION_CLAIMS.iter().map(|c| c.section).collect()
}

/// One disclosed subsumption claim: `subsuming_event_id`'s own real,
/// currently-shipping verse/witness coverage is claimed to fully contain
/// every verse Robertson's primary source cites for `section` itself.
struct SubsumptionClaim {
    section: &'static str,
    subsuming_event_id: &'static str,
    /// Robertson's own literal reference for the SUBSUMED section --
    /// (book, chapter, from_verse, to_verse) per citation fragment --
    /// independently fetched from Project Gutenberg #36264
    /// (`https://www.gutenberg.org/cache/epub/36264/pg36264.txt`) on
    /// 2026-08-21, the SAME primary source `robertson_table()`'s own
    /// structural claims (185 slots, only §128 splits) already come from
    /// -- re-fetched fresh for this check, not copied from this
    /// codebase's own pre-existing ref_note prose, so a stale or
    /// mis-transcribed prose disclosure can't launder itself into a
    /// passing test. A non-Gospel citation fragment (§177's own Robertson
    /// reference also names "1 Cor. 15:5") is out of this project's own
    /// citable KJV-Gospel scope -- the same class of thing §182's own
    /// honest omission already discloses -- and is simply omitted here;
    /// only the Gospel portion of each citation is checked.
    refs: &'static [(&'static str, u16, u16, u16)],
}

const SUBSUMPTION_CLAIMS: &[SubsumptionClaim] = &[
    SubsumptionClaim {
        section: "153",
        subsuming_event_id: "pw_gethsemane",
        refs: &[("MRK", 14, 43, 52), ("MAT", 26, 47, 56), ("LUK", 22, 47, 53), ("JHN", 18, 2, 12)],
    },
    SubsumptionClaim {
        section: "165",
        subsuming_event_id: "pw_golgotha",
        refs: &[("MRK", 15, 33, 37), ("MAT", 27, 45, 50), ("LUK", 23, 44, 46), ("JHN", 19, 28, 30)],
    },
    SubsumptionClaim { section: "169", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MRK", 16, 1, 1), ("MAT", 28, 1, 1)] },
    SubsumptionClaim { section: "170", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MAT", 28, 2, 4)] },
    SubsumptionClaim { section: "172", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("LUK", 24, 9, 12), ("JHN", 20, 2, 10)] },
    SubsumptionClaim { section: "174", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MAT", 28, 9, 10)] },
    // 1 Cor. 15:5 (also part of Robertson's own §177 citation) omitted --
    // not a Gospel reference, out of scope, see `refs`'s own doc comment.
    SubsumptionClaim { section: "177", subsuming_event_id: "pw_emmaus", refs: &[("LUK", 24, 33, 35)] },
];

/// The actual containment check: for every claim in `claims`, the claimed
/// `subsuming_event_id` must exist in `events`, and its own real coverage
/// (top-level `verses` UNION every one of its own `witnesses`' verses --
/// exactly what a reader heading's own PARALLEL ACCOUNTS section would
/// show, the real-world stakes of this check) must contain every verse
/// `refs` names. Aggregates every violation (never fails fast), same house
/// pattern `atlas_etl::validate::run` itself follows. Parameterized over
/// `claims`/`events`/`witnesses` (never reaches into a global or reads a
/// file itself) specifically so it can be exercised against a small
/// synthetic fixture in a unit test, not just the real curated files --
/// see the two tests immediately below
/// `every_subsumption_claim_is_content_verified_against_real_curated_data`.
fn check_subsumption_content(claims: &[SubsumptionClaim], events: &[Event], witnesses: &[(String, EventWitness)]) -> Vec<String> {
    let mut errors = Vec::new();
    for claim in claims {
        let Some(event) = events.iter().find(|e| e.id == claim.subsuming_event_id) else {
            errors.push(format!(
                "subsumption claim for §{}: claimed subsuming event '{}' does not exist in the real curated data",
                claim.section, claim.subsuming_event_id
            ));
            continue;
        };
        let mut covered: HashSet<&str> = event.verses.iter().map(String::as_str).collect();
        for (eid, w) in witnesses {
            if eid == claim.subsuming_event_id {
                if let Some(vs) = w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION) {
                    covered.extend(vs.iter().map(String::as_str));
                }
            }
        }
        // Re-review finding M-3: a claim (or fragment) that contributes zero
        // membership checks would pass vacuously -- the exact self-declared
        // failure mode this checker exists to close, one layer deeper. An
        // empty refs list or an inverted range (`a..=b` with a>b is silently
        // empty in Rust) is therefore itself an error, not a free pass.
        if claim.refs.is_empty() {
            errors.push(format!(
                "subsumption claim for §{}: empty refs list -- a claim that checks no verses is vacuous",
                claim.section
            ));
        }
        for (book, chapter, from_verse, to_verse) in claim.refs {
            if from_verse > to_verse {
                errors.push(format!(
                    "subsumption claim for §{}: inverted range {book}.{chapter}.{from_verse}-{to_verse} checks no verses (vacuous)",
                    claim.section
                ));
                continue;
            }
            for v in *from_verse..=*to_verse {
                let vref = format!("{book}.{chapter}.{v}");
                if !covered.contains(vref.as_str()) {
                    errors.push(format!(
                        "subsumption claim for §{} (claimed subsumed by '{}'): '{vref}' is part of Robertson's own §{} citation but is NOT covered by '{}''s own actual verse/witness ranges -- the subsumption claim is false or stale",
                        claim.section, claim.subsuming_event_id, claim.section, claim.subsuming_event_id
                    ));
                }
            }
        }
    }
    errors
}

/// Batch T2's own disclosed honest omission: Robertson's own table lists
/// NO Gospel reference at all for §182 ("The Appearance to James") -- it
/// rests solely on 1 Corinthians 15:7, outside this project's Gospels+Acts
/// scope. There is no verse to cite, so none is fabricated -- see
/// data/curated/events-extra.toml's own period-11 header comment.
///
/// Fix round 1 (review finding M-2): unlike `subsumed()` above, this list
/// has no content-containment check to give it the same automated teeth --
/// "no Gospel reference exists" isn't a containment fact `check_subsumption_
/// content` (or anything like it) can verify structurally, only a primary-
/// source absence a human has to confirm by reading the actual table (both
/// this batch's own implementer and, independently, the review that raised
/// M-2, did exactly that for §182 against Gutenberg #36264 and found it
/// genuinely empty). This list stays editorially asserted and
/// human-reviewed, not independently derived the way `robertson_table()`
/// itself is.
fn honestly_omitted() -> HashSet<&'static str> {
    ["182"].into_iter().collect()
}

/// Reads a real `data/curated/*.toml` file relative to this crate's own
/// manifest dir (works regardless of the test runner's own CWD) -- NOT a
/// `tests/fixtures/*` copy, deliberately: this test's whole point is
/// checking the REAL curated data actually shipped, not a frozen sample.
fn read_curated(name: &str) -> String {
    let path = format!("{}/../../data/curated/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// Extracts every distinct Robertson section token ("141", "128a", ...)
/// actually CITED as some container's own `robertson_section = "..."`
/// field value -- deliberately scoped to ONLY that key's own line (never
/// `ref_note`/comment prose), which turned out to matter for real: this
/// test's own first draft scanned the WHOLE file for any `§NNN` pattern,
/// which produced a false positive the moment a `ref_note` disclosure
/// MENTIONED a subsumed section by number (e.g. "...fully subsume
/// Robertson's own separate §153...") without CITING it as that event's
/// own robertson_section -- the mention alone made this test's own
/// stale-exception self-check wrongly conclude §153 had a real citation
/// and was therefore a stale entry in `subsumed()`. Caught by this test's
/// own first real run before it shipped; fixed by scanning only
/// `robertson_section = "..."` lines, which is what "cited as this
/// event's own provenance" actually means.
///
/// Scans over CHARS, not bytes, within each such line (curated titles
/// quote Robertson's own archaic spellings, e.g. "Zacchæus"/"Cæsarea,"
/// genuinely multi-byte UTF-8 -- a byte-index scan panics the instant one
/// lands where this function is mid-lookahead, this test's own second
/// real-run catch, fixed the same way). Looks for `§` immediately
/// followed by digits and an optional single lowercase letter.
fn extract_section_tokens(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("robertson_section") {
            continue;
        }
        let mut chars = trimmed.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '§' {
                continue;
            }
            let mut token = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    token.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if token.is_empty() {
                continue;
            }
            if let Some(&letter) = chars.peek() {
                if letter.is_ascii_lowercase() {
                    token.push(letter);
                    chars.next();
                }
            }
            out.insert(token);
        }
    }
    out
}

#[test]
fn every_robertson_section_is_accounted_for_in_the_real_curated_data() {
    let events_extra = read_curated("events-extra.toml");
    let event_witnesses = read_curated("event-witnesses.toml");
    let mut present = extract_section_tokens(&events_extra);
    present.extend(extract_section_tokens(&event_witnesses));

    let subsumed = subsumed();
    let omitted = honestly_omitted();
    let mut gaps: Vec<String> = Vec::new();

    for section in robertson_table() {
        let accounted_for =
            present.contains(&section) || subsumed.contains(section.as_str()) || omitted.contains(section.as_str());
        if !accounted_for {
            gaps.push(section);
        }
    }

    assert!(
        gaps.is_empty(),
        "Robertson section(s) with no curated container, no disclosed subsumption, and no disclosed honest omission -- a real, uncaught coverage gap: {gaps:?}"
    );

    // The inverse direction matters too: every entry in `subsumed`/
    // `honestly_omitted` must be a REAL Robertson section number (185
    // valid slots) -- guards this test's own two exception lists against a
    // typo silently exempting a section that was never a gap to begin with
    // (which would make this test weaker than it looks, not stronger).
    let table: HashSet<String> = robertson_table().into_iter().collect();
    for s in subsumed.iter().chain(omitted.iter()) {
        assert!(table.contains(*s), "'{s}' in this test's own exception lists is not a real Robertson section number 1-184 (typo?)");
    }

    // And every exception-listed section must NOT also appear as its own
    // literal citation -- if it does, the exception enty is stale (the
    // gap it once covered for was since filled directly), and should be
    // removed so this test stays a real gap-detector, not a rubber stamp.
    for s in subsumed.iter().chain(omitted.iter()) {
        assert!(
            !present.contains(*s),
            "'{s}' is listed as subsumed/omitted in this test's own exception lists, but ALSO appears as its own literal robertson_section citation in the curated data -- the exception entry is stale, remove it"
        );
    }
}

// ---------------------------------------------------------------------
// Fix round 1 (review finding M-2): proving `check_subsumption_content`
// itself has real teeth BEFORE trusting it against the live curated data
// -- a synthetic fixture, red then green, same discipline as every other
// check in this codebase.
// ---------------------------------------------------------------------

#[test]
fn subsumption_content_check_catches_a_genuinely_uncovered_citation() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56)] }];
    // Only reaches MAT.26.36 -- does NOT reach 26.47-56, the claimed range.
    let events = vec![Event { id: "pw_gethsemane".into(), verses: vec!["MAT.26.36".into()], ..Default::default() }];
    let errors = check_subsumption_content(claims, &events, &[]);
    // One error PER missing verse (26:47..=56, 10 verses) -- the check
    // reports every gap individually, not just the first, same
    // aggregate-don't-fail-fast policy `validate::run` itself follows.
    assert_eq!(errors.len(), 10, "{errors:?}");
    assert!(errors[0].contains("MAT.26.47"), "{}", errors[0]);
}

#[test]
fn subsumption_content_check_catches_a_missing_subsuming_event() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56)] }];
    let errors = check_subsumption_content(claims, &[], &[]); // pw_gethsemane doesn't exist at all
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("does not exist"), "{}", errors[0]);
}

#[test]
fn subsumption_content_check_passes_when_coverage_is_real() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56), ("MRK", 14, 43, 52)] }];
    let events = vec![Event { id: "pw_gethsemane".into(), verses: (47..=56).map(|v| format!("MAT.26.{v}")).collect(), ..Default::default() }];
    let witnesses = vec![(
        "pw_gethsemane".to_string(),
        EventWitness {
            book: "MRK".into(),
            translations: std::collections::HashMap::from([(
                atlas_core::translation::DEFAULT_TRANSLATION.to_string(),
                (43..=52).map(|v| format!("MRK.14.{v}")).collect(),
            )]),
            ref_note: None,
            robertson_section: None,
        },
    )];
    let errors = check_subsumption_content(claims, &events, &witnesses);
    assert!(errors.is_empty(), "{errors:?}");
}

/// The live audit itself: runs `check_subsumption_content` against the REAL
/// currently-shipping `events-extra.toml`/`event-witnesses.toml`, parsed
/// through the exact same real parsers `main.rs`'s own ETL pipeline uses
/// (`curated::parse_events_extra`/`curated::parse_event_witnesses`) -- never
/// a second, parallel re-implementation of curated-TOML parsing. This is
/// the actual M-2 hardening: every one of `SUBSUMPTION_CLAIMS`'s 7 entries
/// must hold not just as a real section number (the pre-existing
/// bookkeeping check above), but as a genuine, currently-true containment
/// fact about the real data.
#[test]
fn every_subsumption_claim_is_content_verified_against_real_curated_data() {
    let events_extra_toml = read_curated("events-extra.toml");
    let witnesses_toml = read_curated("event-witnesses.toml");
    let events = atlas_etl::curated::parse_events_extra(&events_extra_toml).expect("events-extra.toml must parse");
    let witnesses = atlas_etl::curated::parse_event_witnesses(&witnesses_toml).expect("event-witnesses.toml must parse");

    let errors = check_subsumption_content(SUBSUMPTION_CLAIMS, &events, &witnesses);
    assert!(errors.is_empty(), "subsumption content check found {} problem(s):\n{}", errors.len(), errors.join("\n"));
}

/// Batch T2 requirement 1's own Acts ambiguity ruling: Acts sections carry
/// `acts_section`, never `robertson_section` (Robertson's own Harmony is
/// Gospels-only). This is a narrower sanity check, not a full Acts
/// "expected table" walk (this batch has no independently-verified
/// external Acts-sectioning source to derive one from, honestly, per the
/// ambiguity ruling's own ruling and data/curated/acts-sections.toml's own
/// header disclosure) -- it confirms the 33 SS1-12 sections this batch DID
/// author are all really present and distinctly identified, so a future
/// accidental deletion is still caught even without an external table to
/// check against.
#[test]
fn every_authored_acts_section_is_present_and_distinct() {
    let acts_sections = read_curated("acts-sections.toml");
    let event_ids: Vec<&str> = acts_sections
        .lines()
        .filter(|l| l.trim_start().starts_with("event_id"))
        .filter_map(|l| l.split('"').nth(1))
        .collect();
    assert_eq!(event_ids.len(), 33, "expected exactly 33 authored Acts SS1-12 sections (theo-304 through theo-336); {:?}", event_ids);
    let distinct: HashSet<&str> = event_ids.iter().copied().collect();
    assert_eq!(distinct.len(), 33, "every authored Acts section's own event_id must be distinct -- a duplicate row is a curation error");
    for expected in 304..=336 {
        let id = format!("theo-{expected}");
        assert!(event_ids.contains(&id.as_str()), "expected theo-{expected} among the authored Acts sections, not found");
    }
}

// ---------------------------------------------------------------------
// Batch W1 ("whole-Bible titled verse containers" -- the coverage-manifest
// infrastructure the whole W series builds on, per batch-w-brief.md's own
// "Scale protocol"). Requirement 1's own coverage test: "(a) every verse
// of every declared book is in >=1 container -- derived from the compiled
// KJV verse inventory itself (genuinely independent ground truth, per the
// T2 circularity standard); (b) the declared list only ever grows; (c)
// Gospels+Acts stay declared from day one."
//
// Deliberately reads the REAL COMPILED output (`data/compiled/canon.json`/
// `events.json`), not curated TOML re-parsed by hand, for BOTH halves:
// the verse universe (ground truth -- never a hand-typed per-book verse
// count) AND the coverage claim itself (the REAL merged event set --
// Theographic + every curated source combined -- exactly what a reader's
// own verse popover would show; a curated-TOML-only re-derivation would
// incorrectly call a Theographic-only-covered verse "uncovered", since a
// great many of this project's own covering events, in EVERY book, are
// bare Theographic records with no curated-TOML footprint at all).
// ---------------------------------------------------------------------

/// Reads and parses `data/compiled/{name}` as JSON, relative to this
/// crate's own manifest dir (same CWD-independent trick `read_curated`
/// above already uses for `data/curated/`).
fn read_compiled_json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = format!("{}/../../data/compiled/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("failed to parse {path} as JSON: {e}"))
}

/// The full set of canonical verse ids for one book, derived from the REAL
/// compiled `canon.json` (chapter/verse counts) -- never a hardcoded
/// per-book total, per the T2 circularity standard this requirement's own
/// text explicitly invokes.
fn all_verses_for_book(canon: &Canon, code: &str) -> Vec<String> {
    let book = canon.books.iter().find(|b| b.code == code).unwrap_or_else(|| panic!("'{code}' is not a real canon book code"));
    let mut out = Vec::new();
    for (idx, &count) in book.chapters.iter().enumerate() {
        let chapter = idx + 1;
        for verse in 1..=count {
            out.push(format!("{code}.{chapter}.{verse}"));
        }
    }
    out
}

/// Every verse id this event covers, ANY layer/kind: its own top-level
/// `verses` UNION every witness's own `translations["kjv"]` verses -- the
/// exact same union `AtlasData::finish`'s own `verse_to_events` reverse
/// index builds (server/atlas-core/src/data.rs), so "covered" here means
/// exactly what a real verse's own EVENT-membership popover would show,
/// not a narrower or looser re-derivation.
fn covered_verses(events: &[Event]) -> HashSet<String> {
    let mut out = HashSet::new();
    for e in events {
        out.extend(e.verses.iter().cloned());
        for w in &e.witnesses {
            if let Some(vs) = w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION) {
                out.extend(vs.iter().cloned());
            }
        }
    }
    out
}

#[test]
fn every_declared_book_is_fully_covered_by_the_real_compiled_data() {
    let manifest_toml = read_curated("coverage-manifest.toml");
    let declared = atlas_etl::curated::parse_coverage_manifest(&manifest_toml).expect("coverage-manifest.toml must parse");
    assert!(!declared.is_empty(), "coverage-manifest.toml's own declared list must not be empty");

    let canon: Canon = read_compiled_json("canon.json");
    let events: Vec<Event> = read_compiled_json("events.json");
    let covered = covered_verses(&events);

    let mut failures: Vec<String> = Vec::new();
    for code in &declared {
        let universe = all_verses_for_book(&canon, code);
        let missing: Vec<&String> = universe.iter().filter(|v| !covered.contains(v.as_str())).collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{code}: {} of {} verses are in NO container (first gap: {})",
                missing.len(),
                universe.len(),
                missing[0]
            ));
        }
    }
    assert!(failures.is_empty(), "declared-but-not-fully-covered book(s):\n{}", failures.join("\n"));

    // No duplicate/typo'd declaration, and every declared code is a real
    // canon book -- guards this test's own input against a stale or
    // malformed manifest entry silently doing nothing.
    let distinct: HashSet<&String> = declared.iter().collect();
    assert_eq!(distinct.len(), declared.len(), "coverage-manifest.toml's own declared list has a duplicate entry");
    let real_codes: HashSet<&str> = canon.books.iter().map(|b| b.code.as_str()).collect();
    for code in &declared {
        assert!(real_codes.contains(code.as_str()), "'{code}' in coverage-manifest.toml is not a real canon book code (typo?)");
    }
}

/// Requirement 1(b), "the declared list only ever grows": this is the
/// project's own PERMANENT floor -- every book any past batch has ever
/// declared fully covered. A future batch's own coverage-manifest.toml
/// may only ADD to this set, never remove from it; this test's own
/// hardcoded floor is the audit trail (grep this file's own git blame for
/// when each book was added) and fails loud the moment a future edit
/// accidentally drops one. Gospels+Acts (Batch T/T2) plus Genesis (Batch
/// W1, this run) are the floor as of this commit.
#[test]
fn declared_books_never_shrink_below_the_established_floor() {
    const FLOOR: &[&str] = &["MAT", "MRK", "LUK", "JHN", "ACT", "GEN"];

    let manifest_toml = read_curated("coverage-manifest.toml");
    let declared: HashSet<String> =
        atlas_etl::curated::parse_coverage_manifest(&manifest_toml).expect("coverage-manifest.toml must parse").into_iter().collect();

    for code in FLOOR {
        assert!(declared.contains(*code), "'{code}' must stay declared in coverage-manifest.toml -- the declared list may only ever grow, never shrink");
    }
}
