//! Batch HOTFIX-6 (graph-wide chronology audit, owner live reports #8/#9,
//! 2026-08-22: Solomon's dream at Gibeon rendered PRIOR/FOLLOWING neighbors
//! from the Saul-persecution era -- "this is a lie... i'm sure these errors
//! are graph-wide").
//!
//! ROOT CAUSE (controller-verified, confirmed not re-derived): the `df_*`
//! ("David's Flight from Saul") narrative chain (9 events,
//! `data/curated/events-extra.toml`) was authored ~48 years LATE relative to
//! this atlas's own declared Ussher/traditional scale -- a wrong-anchor
//! calibration error at original authoring time, the SAME defect class
//! `nt_calibration` fixed for the NT Theographic import (that module's own
//! doc comment has the full "why a uniform shift, why ETL-time" reasoning;
//! this module does not repeat it). `1ki_solomon_gibeon` itself was always
//! correctly dated -- the global timeline wiring (HOTFIX-4 requirement 1) is
//! not at fault; the DATA it was handed was.
//!
//! THIS MODULE is the permanent guard the owner's own "i'm sure these
//! errors are graph-wide" assertion earns: every dated event's own year must
//! fall inside the NARRATION WINDOW (`data/curated/book-narration-
//! windows.toml`) of every witness book it claims -- the tight, per-book
//! bound that catches a wrong-era date the coarse, decade-spanning
//! `data/curated/eras.toml` (map-UI-facing, unrelated purpose, untouched by
//! this batch) structurally cannot: `united-kingdom` there spans
//! -1050..-932, comfortably swallowing `df_ramah`'s own pre-fix -1014
//! without complaint.
//!
//! FALSE-POSITIVE LESSON (the controller's own disclosed prior probe): a
//! plain median-based statistical sweep over this same data flagged ~187
//! events, overwhelmingly FALSE positives -- books narrating multi-
//! millennium spans (Genesis) skew a median hard, and genealogy chapters (1
//! Chronicles 1-9, Luke 3) legitimately RECOUNT events thousands of years
//! before their own era (`theo-7` "Birth of Seth," -3874, correctly carries
//! GEN + 1CH + LUK witnesses). This guard is anchor-table-driven, never
//! statistical, and models RECOUNTING witnesses two ways:
//! - `RECOUNTING_CHAPTERS`: a curated (book, chapter-range) list -- an
//!   entire chapter is retrospective-by-genre (a genealogy, Stephen's speech
//!   in Acts 7, Hebrews 11's "by faith" roll call) -- checked by VERSE, so it
//!   applies uniformly whether the citing verse arrived via a curated
//!   `[[witness]]` row or a bare top-level `Event.verses` entry (most
//!   genealogy stubs, e.g. `theo-7`, are the latter -- a LAYER-0 Theographic
//!   import with no curated witness rows at all, so a flag hung on
//!   `EventWitness` specifically could never reach them).
//! - `WINDOW_EXEMPTIONS`: the EVENT_DISTINCT_PAIRS-style pressure valve, one
//!   row per (event, book), each with its own stated reason -- for a
//!   single, non-general citation the chapter-range mechanism doesn't fit
//!   (e.g. `theo-74`'s own Exodus-12:40/Galatians-3:17 sojourn-count
//!   citations, or `theo-129`'s own already-disclosed stray GEN.34.1 tag,
//!   batch-w1-report.md).
//!
//! `THEO_DATE_OVERRIDES` is a DIFFERENT, narrower mechanism: not a
//! systematic scale offset (that shape is `nt_calibration`'s own, and the
//! full-audit sample check below found NO such systematic OT offset) but a
//! single, isolated, genuinely-corrupt raw Theographic import row
//! (`theo-67` "Judgeship of Jair," which the source data dated -1992 --
//! squarely mid-patriarchal-genealogy, its OWN id-neighbors' own era --
//! instead of the Judges era its label/book claim) found by this batch's own
//! full audit, re-derived from Judges 10:1-3's own stated reign lengths
//! (Tola 23 years from `theo-143`'s own -1215, so Jair begins ~-1192;
//! Jair's own 22 years then lands within one year of the very next entry,
//! `theo-144`'s own already-correct -1170 "next oppression begins" value --
//! independent corroboration, not just arithmetic). Applied ETL-side, once,
//! on the raw pre-`finish()` Theographic event set (the SAME timing
//! `nt_calibration::apply_nt_calibration` already established, for the same
//! reason: idempotency across `finish()`'s own two real call sites).
//!
//! `ANCHOR_DEFERRALS` is a THIRD, still-DIFFERENT exemption kind from the
//! two above -- and the two must never be confused (fix round 1, controller
//! ruling + an owner sequencing redirect, both 2026-08-22). `RECOUNTING_
//! CHAPTERS`/`WINDOW_EXEMPTIONS` are PERMANENT, by design: a genealogy
//! citing an ancient birth is always going to do that. An anchor deferral
//! is TIME-BOUNDED: four anchor-table rows (`jerusalem-falls`,
//! `cyrus-decree`, `temple-finished`, `ezra-returns`) carry the table's own
//! CANONICAL Ussher value while their bound event still ships an older,
//! modern-scholarly value the controller's own fix-round-1 ruling already
//! named as drift -- but the owner ruled that re-dating those events (and
//! their own dependents) by hand now is wasted work, since HOTFIX-7's own
//! single-feed migration will delete every inline year literal and resolve
//! dates from `chronology-anchors.toml` directly. A deferral is never
//! silent: `narrative.rs`'s own E1 test reports every one of them BY NAME,
//! still verifying the deferral's own recorded `shipped_value` stays
//! honest (matches the event's real current date) rather than either
//! failing loud (a real E1 violation) or silently passing as if the
//! canonical value were already live.

use std::collections::HashSet;

use crate::data::{AtlasData, BookNarrationWindow, ChronologyAnchor, Event};
use crate::refs::VerseId;
use crate::time::Year;

// ---------------------------------------------------------------------
// RECOUNTING_CHAPTERS -- the chapter-range recounting mechanism
// ---------------------------------------------------------------------

/// `(book, chapter_from, chapter_to, reason)`. A verse in one of these
/// ranges never requires the event citing it to fall inside that BOOK's own
/// narration window -- the citation is retrospective by genre, not a claim
/// that the cited event happened during this book's own narrated era.
/// Checked per-VERSE (not per-witness/per-event), so it applies uniformly
/// regardless of how the citation reached the event (top-level
/// `Event.verses` or a curated `[[witness]]` row) -- see this module's own
/// doc comment for why that distinction matters.
pub const RECOUNTING_CHAPTERS: &[(&str, u16, u16, &str)] = &[
    (
        "1CH",
        1,
        9,
        "1 Chronicles's own opening genealogies (Adam to the post-exilic resettlement roster, 1 Chronicles 9:2-34) -- the brief's own named example (theo-7 'Birth of Seth', -3874).",
    ),
    ("LUK", 3, 3, "Luke's own genealogy of Jesus (Luke 3:23-38, Joseph back to Adam) -- the brief's own named example."),
    ("MAT", 1, 1, "Matthew's own genealogy of Jesus (Matthew 1:1-17, Abraham to Jesus) -- the same recounting shape as Luke 3, proactively covered for forward-compatibility (no compiled event currently exercises it)."),
    (
        "HEB",
        11,
        11,
        "Hebrews's own 'by faith' roll call (Hebrews 11, Abel through the prophets) -- found by this batch's own full audit: theo-24 'Enoch taken up' (-3017) carries a HEB witness via Hebrews 11:5.",
    ),
    (
        "ACT",
        7,
        7,
        "Stephen's own speech before the council (Acts 7, recounting Abraham through the Exodus and conquest) -- the brief's own named example. No compiled event currently cites it as a witness; covered proactively for forward-compatibility.",
    ),
];

pub fn is_recounting(book: &str, chapter: u16) -> bool {
    RECOUNTING_CHAPTERS.iter().any(|(b, from, to, _)| *b == book && *from <= chapter && chapter <= *to)
}

// ---------------------------------------------------------------------
// WINDOW_EXEMPTIONS -- the EVENT_DISTINCT_PAIRS-style pressure valve
// ---------------------------------------------------------------------

/// One curated (event, book) exemption from the window check -- mirrors
/// `event_merge::EventDistinct`'s own shape exactly (a real reason per row,
/// never a bare id list).
pub struct WindowExemption {
    pub event_id: &'static str,
    pub book: &'static str,
    pub reason: &'static str,
}

pub const WINDOW_EXEMPTIONS: &[WindowExemption] = &[
    WindowExemption {
        event_id: "theo-74",
        book: "EXO",
        reason: "\"Sojourn in Canaan and Egypt\" (-1921, the Abrahamic-covenant year, Genesis 15 -- matches theo-70/71/72/73's own SAME -1921 cluster) carries an Exodus witness via Exodus 12:40-41, the verse that ITSELF retrospectively states the 430-year sojourn count this event marks the START of. A single prophecy-inception citation, not a general recounting-chapter pattern (Exodus 12 is not otherwise retrospective) -- an individual exemption, not a RECOUNTING_CHAPTERS row.",
    },
    WindowExemption {
        event_id: "theo-74",
        book: "GAL",
        reason: "Same event as the EXO row immediately above; its own Galatians witness (Galatians 3:17) is Paul's own cross-reference to the identical 430-year count. Same reasoning.",
    },
    WindowExemption {
        event_id: "theo-129",
        book: "GEN",
        reason: "\"Death of Moses\" (-1452, a real, correctly-dated Deuteronomy-34 event) carries a stray GEN.34.1 verse tag -- an ALREADY-DISCLOSED Theographic import anomaly (batch-w1-report.md \u{a7}5; also named in event_merge::EVENT_DISTINCT_PAIRS's own theo-129/deu_death_of_moses entry). Not this batch's own to fix (a verse-tag correction, not a date, per this batch's own scope discipline) -- exempted, not silently re-tagged.",
    },
    WindowExemption {
        event_id: "theo-124",
        book: "JDG",
        reason: "\"Lifetime of Joshua\" (-1521, Joshua's own birth year, matching the SAME 'Lifetime of X dated at birth' convention theo-121 'Lifetime of Moses' -1571 uses) carries its ONE verse, JDG.2.8 -- Judges's own brief, backward-looking death notice for Joshua, part of that book's own retrospective opening (Judges 2:6-9) before its main judges-cycle narrative begins at 2:10. A single citation into a book this event's own subject never otherwise appears in, not a general recounting-chapter pattern (unlike 1CH.1-9/LUK.3 -- Judges 1-2 is NOT designated wholesale-recounting, so JDG's own tight window still gates any other Judges-2 event, e.g. jdg_bochim_rebuke, unweakened) -- an individual exemption, found by this batch's own full audit.",
    },
];

pub fn is_exempted(event_id: &str, book: &str) -> bool {
    WINDOW_EXEMPTIONS.iter().any(|x| x.event_id == event_id && x.book == book)
}

// ---------------------------------------------------------------------
// ANCHOR_DEFERRALS -- the TYPED, TIME-BOUNDED anchor-equality deferral
// (a DIFFERENT kind from RECOUNTING_CHAPTERS/WINDOW_EXEMPTIONS above --
// see this module's own doc comment for the full distinction)
// ---------------------------------------------------------------------

/// One curated anchor-row deferral: `anchor_id` names a `ChronologyAnchor`
/// row (`chronology-anchors.toml`) whose own `year` is the CANONICAL
/// (Ussher/declared) value, bound to `event_id`, but whose compiled
/// `from_year` still, honestly, reads `shipped_value` -- an OLDER,
/// modern-scholarly value the fix-round-1 ruling already named as W2-era
/// authoring drift, deliberately NOT hand-corrected per the owner's own
/// sequencing redirect (HOTFIX-7's own single-feed migration will delete
/// the inline literal and resolve it from the table directly, so a hand
/// edit now would just be immediately overwritten there).
pub struct AnchorDeferral {
    pub anchor_id: &'static str,
    pub event_id: &'static str,
    pub shipped_value: Year,
    pub reason: &'static str,
}

pub const ANCHOR_DEFERRALS: &[AnchorDeferral] = &[
    AnchorDeferral {
        anchor_id: "jerusalem-falls",
        event_id: "exl_jerusalem",
        shipped_value: -586,
        reason: "-586 is a modern-scholarly Nebuchadnezzar-regnal-year value, not this atlas's own declared Ussher scale (-588) -- W2-era authoring drift (fix round 1, controller ruling, 2026-08-22). Resolves when HOTFIX-7's single-feed migration re-dates exl_jerusalem (and its own exile-chain/jer_*/2ki_* dependents) directly from this table.",
    },
    AnchorDeferral {
        anchor_id: "cyrus-decree",
        event_id: "ret_babylon",
        shipped_value: -538,
        reason: "-538 is tied to Babylon's own fall (539 BC) rather than Cyrus's own decree specifically (-536) -- W2-era authoring drift (fix round 1, controller ruling, 2026-08-22). Resolves when HOTFIX-7's single-feed migration re-dates ret_babylon (and its own return-chain/ezr_*/1ch_*/dan_* dependents) directly from this table.",
    },
    AnchorDeferral {
        anchor_id: "temple-finished",
        event_id: "ret_jerusalem_temple",
        shipped_value: -516,
        reason: "-516 vs -515 is a 1-year Adar/Nisan calendar-epoch variance; this atlas's own declared scale resolves to -515. Resolves when HOTFIX-7's single-feed migration re-dates ret_jerusalem_temple/ezr_temple_completed directly from this table.",
    },
    AnchorDeferral {
        anchor_id: "ezra-returns",
        event_id: "ezr_ezra_arrives",
        shipped_value: -458,
        reason: "-458 sits within the modern-scholarly ~457 BC spread, not Ussher's own actual original figure (467 BC -- a since-superseded Persian-regnal assumption most modern treatments replace with 457 instead; this atlas's own prior -458 sat within THAT spread, not Ussher's own). Resolves when HOTFIX-7's single-feed migration re-dates ezr_ezra_arrives (and its own arrival-cluster dependents) directly from this table.",
    },
];

pub fn anchor_deferral(anchor_id: &str) -> Option<&'static AnchorDeferral> {
    ANCHOR_DEFERRALS.iter().find(|d| d.anchor_id == anchor_id)
}

// ---------------------------------------------------------------------
// THEO_DATE_OVERRIDES -- isolated raw-import date corrections
// ---------------------------------------------------------------------

/// `(id, corrected_from_year, corrected_to_year, reason)` -- see this
/// module's own doc comment for why this is a DIFFERENT mechanism from
/// `nt_calibration` (a single corrupt row, not a systematic scale offset).
pub const THEO_DATE_OVERRIDES: &[(&str, i32, i32, &str)] = &[(
    "theo-67",
    -1192,
    -1192,
    "\"Judgeship of Jair\" (JDG witness): the raw Theographic import dated this -1992 -- squarely mid-patriarchal-genealogy (between theo-66 'Lifetime of Abraham' -1997 and theo-68 'Death of Reu' -1978, its own numeric id-neighbors), an isolated import-row corruption, not a graph-wide pattern (the surrounding theo-133..theo-154 Judges cluster is otherwise internally consistent and correctly dated). Re-derived from Judges 10:1-3: Tola judges 23 years from theo-143's own -1215, so Jair begins ~-1192; Jair's own stated 22-year judgeship then ends within a year of theo-144's own already-correct -1170 'next oppression (Ammon/Philistines) begins' value -- independent corroboration from an untouched neighboring entry, not bare arithmetic.",
)];

/// One row of `apply_theo_date_overrides`'s own audit trail -- mirrors
/// `nt_calibration::CalibrationLogEntry`'s own shape.
#[derive(Debug, Clone, PartialEq)]
pub struct DateOverrideLogEntry {
    pub id: String,
    pub label: String,
    pub old_from_year: Year,
    pub new_from_year: Year,
    pub old_to_year: Year,
    pub new_to_year: Year,
}

/// Applies `THEO_DATE_OVERRIDES` in place -- same ETL-only, called-exactly-
/// once-on-the-raw-pre-`finish()`-event-set timing as
/// `nt_calibration::apply_nt_calibration`, for the identical reason
/// (idempotency: `finish()` runs twice across the real pipeline, and a raw
/// date write is not naturally idempotent the way `event_merge`'s removal-
/// based identity merge is).
pub fn apply_theo_date_overrides(events: &mut [Event]) -> Vec<DateOverrideLogEntry> {
    let mut log = Vec::new();
    for e in events.iter_mut() {
        let Some(&(_, new_from, new_to, _)) = THEO_DATE_OVERRIDES.iter().find(|(id, ..)| *id == e.id) else { continue };
        let old_from_year = e.when.from_year;
        let old_to_year = e.when.to_year;
        e.when.from_year = new_from;
        e.when.to_year = new_to;
        log.push(DateOverrideLogEntry {
            id: e.id.clone(),
            label: e.label.clone(),
            old_from_year,
            new_from_year: new_from,
            old_to_year,
            new_to_year: new_to,
        });
    }
    log
}

// ---------------------------------------------------------------------
// The window check itself
// ---------------------------------------------------------------------

pub fn window_for_book<'a>(windows: &'a [BookNarrationWindow], book: &str) -> Option<&'a BookNarrationWindow> {
    windows.iter().find(|w| w.book == book)
}

/// This event's own set of witness books that actually constrain its date
/// (i.e. `effective_verses` grouped by book, EXCLUDING any verse that lands
/// in a `RECOUNTING_CHAPTERS` range, AND excluding any book individually
/// listed in `WINDOW_EXEMPTIONS` for this exact event) -- "the books whose
/// narration window this event's own year must fall inside," per this
/// module's own doc comment. A book every one of whose citing verses is
/// recounting, or whose citation is individually exempted, contributes
/// nothing here (fully exempted for this event), matching `theo-7`'s own
/// 1CH/LUK witnesses (chapters 1-9/3, both wholly recounting) exactly.
///
/// Exemptions are folded in HERE, not left to each caller to re-check,
/// deliberately: `era_boundary_violations` below reasons about "which books
/// actually constrain this event's date" the identical way `window_violations`
/// does (found live, batch-hotfix6: `theo-124` "Lifetime of Joshua," -1521,
/// individually exempted from JDG's own window check, was STILL counted as
/// JDG-witnessed by an earlier version of `era_boundary_violations` that
/// re-derived book membership independently -- wrongly asserting it must
/// sort after the exodus-wilderness era boundary. One shared predicate for
/// "does this citation count," not two that can drift apart.
pub fn window_check_books(e: &Event) -> HashSet<&'static str> {
    let mut out = HashSet::new();
    for v in crate::event_merge::effective_verses(e) {
        let Ok(vid) = VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code();
        if is_recounting(book, vid.chapter) || is_exempted(&e.id, book) {
            continue;
        }
        out.insert(book);
    }
    out
}

/// One window-check failure -- "`event_id`'s own year `year` falls outside
/// `book`'s own narration window `window`."
#[derive(Debug, Clone, PartialEq)]
pub struct WindowViolation {
    pub event_id: String,
    pub label: String,
    pub book: String,
    pub year: (Year, Year),
    pub window: (Year, Year),
}

/// The full sweep: every dated (`kind == "event"`) event, against every
/// witness book `window_check_books` returns for it (already net of
/// `WINDOW_EXEMPTIONS` -- see that function's own doc comment for why the
/// exemption check lives there, shared with `era_boundary_violations`, not
/// duplicated here). A book with NO curated `BookNarrationWindow` row is a
/// SEPARATE structural problem (`missing_windows` below), not silently
/// skipped here nor silently passed -- `atlas_etl::validate` fails loud on
/// both independently, so a missing window is never mistaken for "nothing
/// to check."
pub fn window_violations(events: &[Event], windows: &[BookNarrationWindow]) -> Vec<WindowViolation> {
    let mut out = Vec::new();
    for e in events.iter().filter(|e| e.kind == "event") {
        let mut books: Vec<&str> = window_check_books(e).into_iter().collect();
        books.sort_unstable();
        for book in books {
            let Some(w) = window_for_book(windows, book) else { continue };
            if e.when.from_year < w.from_year || e.when.to_year > w.to_year {
                out.push(WindowViolation {
                    event_id: e.id.clone(),
                    label: e.label.clone(),
                    book: book.to_string(),
                    year: (e.when.from_year, e.when.to_year),
                    window: (w.from_year, w.to_year),
                });
            }
        }
    }
    out
}

/// Structural check: every book ANY dated event actually cites (via
/// `window_check_books`, so a purely-recounting citation never demands a
/// window) must have a real curated `BookNarrationWindow` row -- a book
/// gaining its first dated event with no window authored for it yet is a
/// curation gap, not a silent pass.
pub fn missing_windows(events: &[Event], windows: &[BookNarrationWindow]) -> Vec<&'static str> {
    let known: HashSet<&str> = windows.iter().map(|w| w.book.as_str()).collect();
    let mut missing: HashSet<&'static str> = HashSet::new();
    for e in events.iter().filter(|e| e.kind == "event") {
        for book in window_check_books(e) {
            if !known.contains(book) {
                missing.insert(book);
            }
        }
    }
    let mut out: Vec<&'static str> = missing.into_iter().collect();
    out.sort_unstable();
    out
}

// ---------------------------------------------------------------------
// E4 -- era-partition: dated events vs. `chronology_anchors`'s own
// `era_boundary` rows
// ---------------------------------------------------------------------

/// One era-partition failure -- generalizes HOTFIX-4's own single NT
/// era-boundary gate (Passion cluster vs. every ACT-witnessed event) to
/// EVERY `era_boundary` anchor in the table: for boundary anchor `b`, an
/// event whose own witness-book windows sit ENTIRELY at-or-before `b`'s own
/// year must sort at-or-before `b` on the global timeline; one whose
/// windows sit ENTIRELY after `b`'s own year must sort strictly after it. An
/// event straddling the boundary (a witness book whose own window itself
/// spans across `b`'s year) contributes no assertion for THAT boundary --
/// the same honest carve-out HOTFIX-4's own gate already established for
/// `pw_mount_of_olives`'s own legitimate Acts 1:9-12 citation.
#[derive(Debug, Clone, PartialEq)]
pub struct EraBoundaryViolation {
    pub event_id: String,
    pub label: String,
    pub boundary_id: String,
    pub boundary_year: Year,
    /// `"before"` (should sort at-or-before the boundary) or `"after"`
    /// (should sort strictly after it) -- which side this event's own
    /// witness-book windows placed it on.
    pub side: &'static str,
}

pub fn era_boundary_violations(d: &AtlasData) -> Vec<EraBoundaryViolation> {
    let mut out = Vec::new();
    let boundaries: Vec<&ChronologyAnchor> = d.chronology_anchors.iter().filter(|a| a.era_boundary).collect();

    for b in &boundaries {
        let Some(b_event_id) = b.event_id.as_deref() else { continue }; // structurally guaranteed by validate::run_chronology_anchors; skip defensively rather than panic
        let Some(b_pos) = d.timeline_position(b_event_id) else { continue };

        for e in d.events.iter().filter(|e| e.kind == "event" && e.id != b_event_id) {
            let mut books: Vec<&str> = window_check_books(e).into_iter().collect();
            books.sort_unstable();
            if books.is_empty() {
                continue;
            }
            let mut all_before = true;
            let mut all_after = true;
            for book in &books {
                match window_for_book(&d.book_narration_windows, book) {
                    Some(w) => {
                        if w.to_year > b.year {
                            all_before = false;
                        }
                        if w.from_year <= b.year {
                            all_after = false;
                        }
                    }
                    None => {
                        all_before = false;
                        all_after = false;
                    }
                }
            }

            let side = if all_before && !all_after {
                "before"
            } else if all_after && !all_before {
                "after"
            } else {
                continue; // straddles this boundary -- no assertion, matches HOTFIX-4's own pw_mount_of_olives carve-out
            };

            let Some(e_pos) = d.timeline_position(&e.id) else { continue };
            let violates = match side {
                "before" => e_pos > b_pos,
                _ => e_pos <= b_pos,
            };
            if violates {
                out.push(EraBoundaryViolation {
                    event_id: e.id.clone(),
                    label: e.label.clone(),
                    boundary_id: b.id.clone(),
                    boundary_year: b.year,
                    side,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::EventWitness;
    use crate::time::TimeRange;
    use std::collections::HashMap;

    fn event(id: &str, from: Year, to: Year, verses: &[&str]) -> Event {
        Event { id: id.into(), label: id.into(), when: TimeRange::new(from, to).unwrap(), verses: verses.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }

    fn witness_event(id: &str, from: Year, to: Year, book: &str, verses: &[&str]) -> Event {
        let mut translations = HashMap::new();
        translations.insert("kjv".to_string(), verses.iter().map(|s| s.to_string()).collect());
        Event {
            id: id.into(),
            label: id.into(),
            when: TimeRange::new(from, to).unwrap(),
            witnesses: vec![EventWitness { book: book.into(), translations, ref_note: None, robertson_section: None }],
            ..Default::default()
        }
    }

    fn window(book: &str, from: Year, to: Year) -> BookNarrationWindow {
        BookNarrationWindow { book: book.into(), from_year: from, to_year: to, note: None }
    }

    // --- RECOUNTING_CHAPTERS -------------------------------------------

    #[test]
    fn genealogy_chapter_is_recounting() {
        assert!(is_recounting("1CH", 1));
        assert!(is_recounting("1CH", 9));
        assert!(is_recounting("LUK", 3));
        assert!(is_recounting("HEB", 11));
        assert!(is_recounting("ACT", 7));
    }

    #[test]
    fn narrative_chapter_is_not_recounting() {
        assert!(!is_recounting("1CH", 10)); // Saul's death -- real narrative, not genealogy
        assert!(!is_recounting("LUK", 24));
        assert!(!is_recounting("1SA", 19));
    }

    // --- window_check_books ----------------------------------------------

    #[test]
    fn recounting_witness_is_excluded_from_window_check_books() {
        // theo-7's own real shape: GEN (real narration) + 1CH.1/LUK.3 (both recounting).
        let e = event("theo-7", -3874, -3874, &["GEN.5.3", "1CH.1.1", "LUK.3.38"]);
        let books = window_check_books(&e);
        assert_eq!(books, HashSet::from(["GEN"]), "1CH/LUK are wholly recounting citations here and must not demand this event fall inside their own windows");
    }

    #[test]
    fn narrative_witness_book_is_included() {
        let e = event("df_ramah", -1062, -1062, &["1SA.19.18"]);
        assert_eq!(window_check_books(&e), HashSet::from(["1SA"]));
    }

    #[test]
    fn witness_row_verses_count_same_as_top_level_verses() {
        let e = witness_event("x", 33, 33, "ACT", &["ACT.7.2"]); // Stephen's speech opening -- recounting
        assert!(window_check_books(&e).is_empty(), "a witness-row citation into a recounting chapter must be excluded exactly like a top-level verses citation");
    }

    // --- window_violations: red-then-green on the owner's own case -------

    #[test]
    fn red_df_ramah_pre_fix_date_fails_the_1sa_window() {
        let events = vec![event("df_ramah", -1014, -1014, &["1SA.19.18"])]; // pre-fix date
        let windows = vec![window("1SA", -1171, -1055)];
        let violations = window_violations(&events, &windows);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].event_id, "df_ramah");
        assert_eq!(violations[0].book, "1SA");
    }

    #[test]
    fn green_df_ramah_post_fix_date_passes_the_1sa_window() {
        let events = vec![event("df_ramah", -1062, -1062, &["1SA.19.18"])]; // post-fix date
        let windows = vec![window("1SA", -1171, -1055)];
        assert!(window_violations(&events, &windows).is_empty());
    }

    #[test]
    fn green_theo7_passes_with_zero_exemption_spam() {
        // The brief's own named acceptance: theo-7's correct -3874 with
        // 1CH+LUK witnesses passes with ZERO exemption-list entries (not
        // present anywhere in WINDOW_EXEMPTIONS) -- proven by the recounting
        // mechanism alone.
        assert!(!WINDOW_EXEMPTIONS.iter().any(|x| x.event_id == "theo-7"), "theo-7 must pass via RECOUNTING_CHAPTERS alone, never an exemption-list entry");
        let events = vec![event("theo-7", -3874, -3874, &["GEN.5.3", "1CH.1.1", "LUK.3.38"])];
        let windows = vec![window("GEN", -4004, -1635), window("1CH", -1055, -1015), window("LUK", -6, 33)];
        assert!(window_violations(&events, &windows).is_empty());
    }

    #[test]
    fn a_book_with_no_curated_window_is_reported_missing_not_silently_passed() {
        let events = vec![event("x", 100, 100, &["ROM.1.1"])];
        assert_eq!(missing_windows(&events, &[]), vec!["ROM"]);
        assert!(window_violations(&events, &[]).is_empty(), "window_violations itself never fabricates a pass/fail for an unauthored window -- that is missing_windows's own job");
    }

    #[test]
    fn exempted_event_book_pair_never_flags() {
        let events = vec![event("theo-74", -1921, -1921, &["EXO.12.40", "GAL.3.17"])];
        let windows = vec![window("EXO", -1571, -1445), window("GAL", 48, 53)];
        assert!(window_violations(&events, &windows).is_empty());
    }

    // --- THEO_DATE_OVERRIDES ----------------------------------------------

    #[test]
    fn red_then_green_theo67_override() {
        let mut events = vec![event("theo-67", -1992, -1992, &["JDG.10.3"])];
        let windows = vec![window("JDG", -1424, -1102)];
        assert_eq!(window_violations(&events, &windows).len(), 1, "RED: pre-override -1992 fails JDG's own window");

        let log = apply_theo_date_overrides(&mut events);
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].old_from_year, -1992);
        assert_eq!(log[0].new_from_year, -1192);

        assert!(window_violations(&events, &windows).is_empty(), "GREEN: post-override -1192 passes");
    }

    #[test]
    fn override_is_a_noop_for_every_other_event() {
        let mut events = vec![event("theo-68", -1978, -1978, &["GEN.11.20"])];
        let log = apply_theo_date_overrides(&mut events);
        assert!(log.is_empty());
        assert_eq!(events[0].when.from_year, -1978);
    }

    #[test]
    fn theo_date_overrides_table_has_no_duplicate_ids() {
        let mut seen = std::collections::HashSet::new();
        for (id, ..) in THEO_DATE_OVERRIDES {
            assert!(seen.insert(*id), "{id} listed twice in THEO_DATE_OVERRIDES");
        }
    }

    #[test]
    fn window_exemptions_table_has_no_duplicate_event_book_pairs() {
        let mut seen = std::collections::HashSet::new();
        for x in WINDOW_EXEMPTIONS {
            assert!(seen.insert((x.event_id, x.book)), "({}, {}) listed twice in WINDOW_EXEMPTIONS", x.event_id, x.book);
        }
    }

    // --- ANCHOR_DEFERRALS (fix round 1 + owner sequencing redirect) ------

    #[test]
    fn anchor_deferrals_table_has_no_duplicate_anchor_ids() {
        let mut seen = std::collections::HashSet::new();
        for d in ANCHOR_DEFERRALS {
            assert!(seen.insert(d.anchor_id), "'{}' listed twice in ANCHOR_DEFERRALS", d.anchor_id);
        }
    }

    #[test]
    fn anchor_deferral_looks_up_by_id() {
        let d = anchor_deferral("jerusalem-falls").expect("jerusalem-falls is a registered deferral");
        assert_eq!(d.event_id, "exl_jerusalem");
        assert_eq!(d.shipped_value, -586);
        assert!(anchor_deferral("not-a-real-anchor").is_none());
    }

    #[test]
    fn exactly_four_anchors_are_deferred_today() {
        // A deliberate, named count -- per this module's own doc comment,
        // a deferral is never silent: gaining or losing one (e.g. HOTFIX-7
        // resolving one) must force a conscious edit here, not a silent
        // drift. If this ever fails, update ANCHOR_DEFERRALS AND this
        // count together, not one without the other.
        let ids: Vec<&str> = ANCHOR_DEFERRALS.iter().map(|d| d.anchor_id).collect();
        assert_eq!(ids, vec!["jerusalem-falls", "cyrus-decree", "temple-finished", "ezra-returns"]);
    }
}
