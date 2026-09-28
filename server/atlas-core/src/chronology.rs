//! Every dated event's year must fall inside the curated narration window of every
//! witness book it cites, unless the citing verse is retrospective by genre or that one
//! (event, book) citation is individually exempted.

use std::collections::{HashMap, HashSet};

use crate::data::{AtlasData, BookNarrationWindow, ChronologyAnchor, Event};
use crate::refs::VerseId;
use crate::time::Year;

/// `(book, chapter_from, chapter_to, reason)`. A verse in one of these ranges never
/// requires the event citing it to fall inside that book's narration window. Checked per
/// VERSE, so it applies however the citation reached the event.
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

/// Every row carries a real reason; an exemption is never a bare id.
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

/// An anchor whose `year` is the canonical value while its bound event still carries an
/// older one. `shipped_value` records that older date, so the gap stays reported rather
/// than silent, and goes stale the moment the event is re-dated.
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
        reason: "-458 sits within the modern-scholarly ~457 BC spread, not Ussher's own actual original figure (467 BC -- a since-superseded Persian-regnal assumption most modern treatments replace with 457 instead; this atlas's own prior -458 sat within THAT spread, not Ussher's own). Resolves when HOTFIX-7's single-feed migration re-dates ezr_ezra_arrives (and its own arrival-cluster dependents) directly from this table. CITATION (fix round 2, review finding I-3): Ussher's Annals of the World, paragraph 1202 ('3537b AM, 4247 JP, 467 BC') -- see chronology-anchors.toml's own ezra-returns row for the full quotation and source detail.",
    },
];

pub fn anchor_deferral(anchor_id: &str) -> Option<&'static AnchorDeferral> {
    ANCHOR_DEFERRALS.iter().find(|d| d.anchor_id == anchor_id)
}

/// Two failure shapes: a non-deferred anchor disagreeing with its bound event's year, or
/// a deferral whose `shipped_value` has gone stale (`is_stale_deferral`). Both are real
/// defects, never silently passed.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorEqualityViolation {
    pub anchor_id: String,
    pub event_id: String,
    pub table_year: Year,
    pub event_year: Year,
    pub is_stale_deferral: bool,
}

/// An anchor that is neither a violation nor a plain equality pass, reported by name so
/// a live deferral can never be folded silently into either bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorDeferralReport {
    pub anchor_id: String,
    pub event_id: String,
    pub canonical_year: Year,
    pub shipped_value: Year,
}

/// Every anchor with a bound event is checked; a dangling `event_id` is another check's
/// report, so it is skipped here rather than duplicated. A deferred anchor yields a
/// deferral report or a stale-deferral violation, never a plain equality violation.
pub fn anchor_equality_check(anchors: &[ChronologyAnchor], events: &[Event]) -> (Vec<AnchorEqualityViolation>, Vec<AnchorDeferralReport>) {
    let by_id: HashMap<&str, &Event> = events.iter().map(|e| (e.id.as_str(), e)).collect();
    let mut violations = Vec::new();
    let mut deferred = Vec::new();

    for a in anchors {
        let Some(eid) = a.event_id.as_deref() else { continue };
        let Some(e) = by_id.get(eid) else { continue };

        if let Some(def) = anchor_deferral(&a.id) {
            if e.when.from_year != def.shipped_value {
                violations.push(AnchorEqualityViolation {
                    anchor_id: a.id.clone(),
                    event_id: eid.to_string(),
                    table_year: def.shipped_value,
                    event_year: e.when.from_year,
                    is_stale_deferral: true,
                });
            } else {
                deferred.push(AnchorDeferralReport { anchor_id: a.id.clone(), event_id: eid.to_string(), canonical_year: a.year, shipped_value: def.shipped_value });
            }
            continue;
        }

        if e.when.from_year != a.year {
            violations.push(AnchorEqualityViolation {
                anchor_id: a.id.clone(),
                event_id: eid.to_string(),
                table_year: a.year,
                event_year: e.when.from_year,
                is_stale_deferral: false,
            });
        }
    }

    (violations, deferred)
}

/// `(id, corrected_from_year, corrected_to_year, reason)`: isolated corrupt import rows,
/// not a systematic scale offset.
pub const THEO_DATE_OVERRIDES: &[(&str, i32, i32, &str)] = &[
    (
        "theo-67",
        -1192,
        -1192,
        "\"Judgeship of Jair\" (JDG witness): the raw Theographic import dated this -1992 -- squarely mid-patriarchal-genealogy (between theo-66 'Lifetime of Abraham' -1997 and theo-68 'Death of Reu' -1978, its own numeric id-neighbors), an isolated import-row corruption, not a graph-wide pattern (the surrounding theo-133..theo-154 Judges cluster is otherwise internally consistent and correctly dated). Re-derived from Judges 10:1-3: Tola judges 23 years from theo-143's own -1215, so Jair begins ~-1192; Jair's own stated 22-year judgeship then ends within a year of theo-144's own already-correct -1170 'next oppression (Ammon/Philistines) begins' value -- independent corroboration from an untouched neighboring entry, not bare arithmetic.",
    ),
    (
        "theo-87",
        -2242,
        -2242,
        "\"Nimrod's kingdom begins\" (GEN witness, GEN.10.8-12): the raw Theographic import (yearNum) dated this -1822 -- anachronistic on any traditional scheme, since Genesis 10:10 states plainly 'the beginning of his kingdom was Babel' and this atlas's own declared traditional scale dates the Babel/dispersion event to 2242 BC. GROUND: James Ussher, The Annals of the World (1658) -- Babel/the dispersion, 2242 BC, the same public-domain primary source this table's chronology-anchors.toml sibling already cites throughout (bare year value, not a prose quotation; no LICENSES.md row needed, same convention as chronology-anchors.toml's own header note). Batch GAZ-1+CHRON-FIX, 2026-08-24, per the owner's standing traditional-dates ruling (confessional-Lutheran consensus hierarchy). Kretzmann concurrence (KRETZ-1, Popular Commentary of the Bible, 1921-1924, scouted but not yet ingested) to be verified once that corpus lands -- not blocking this correction, which stands on Ussher/Genesis 10:10 alone.",
    ),
];

#[derive(Debug, Clone, PartialEq)]
pub struct DateOverrideLogEntry {
    pub id: String,
    pub label: String,
    pub old_from_year: Year,
    pub new_from_year: Year,
    pub old_to_year: Year,
    pub new_to_year: Year,
}

/// Must be called exactly once, on the raw event set before it is finished: a date write
/// is not idempotent, and finishing runs more than once over the real pipeline.
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

pub fn window_for_book<'a>(windows: &'a [BookNarrationWindow], book: &str) -> Option<&'a BookNarrationWindow> {
    windows.iter().find(|w| w.book == book)
}

/// The witness books that actually constrain this event's date: verses in a recounting
/// chapter and books exempted for this event contribute nothing. Folded in here so every
/// caller shares one answer to "does this citation count" instead of re-deriving it.
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

#[derive(Debug, Clone, PartialEq)]
pub struct WindowViolation {
    pub event_id: String,
    pub label: String,
    pub book: String,
    pub year: (Year, Year),
    pub window: (Year, Year),
}

/// A book with no curated narration window is a separate structural problem, reported by
/// `missing_windows`: it is neither skipped nor passed here.
pub fn window_violations(events: &[Event], windows: &[BookNarrationWindow]) -> Vec<WindowViolation> {
    let mut out = Vec::new();
    for e in events.iter().filter(|e| e.kind == crate::data::EventKind::Event) {
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

/// A book gaining its first dated event before a window is authored for it is a curation
/// gap, not a silent pass.
pub fn missing_windows(events: &[Event], windows: &[BookNarrationWindow]) -> Vec<&'static str> {
    let known: HashSet<&str> = windows.iter().map(|w| w.book.as_str()).collect();
    let mut missing: HashSet<&'static str> = HashSet::new();
    for e in events.iter().filter(|e| e.kind == crate::data::EventKind::Event) {
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

/// For each era-boundary anchor: an event whose witness-book windows sit entirely at or
/// before the boundary year must sort there too, and one entirely after must sort after.
/// An event whose window straddles the boundary asserts nothing for that boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct EraBoundaryViolation {
    pub event_id: String,
    pub label: String,
    pub boundary_id: String,
    pub boundary_year: Year,
    /// `"before"` (sorts at or before the boundary) or `"after"` (strictly after it).
    pub side: &'static str,
}

pub fn era_boundary_violations(d: &AtlasData) -> Vec<EraBoundaryViolation> {
    let mut out = Vec::new();
    let boundaries: Vec<&ChronologyAnchor> = d.chronology_anchors.iter().filter(|a| a.era_boundary).collect();

    for b in &boundaries {
        let Some(b_event_id) = b.event_id.as_deref() else { continue };
        let Some(b_pos) = d.timeline_position(b_event_id) else { continue };

        for e in d.events.iter().filter(|e| e.kind == crate::data::EventKind::Event && e.id != b_event_id) {
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
                continue;
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
        assert!(!is_recounting("1CH", 10));
        assert!(!is_recounting("LUK", 24));
        assert!(!is_recounting("1SA", 19));
    }

    #[test]
    fn recounting_witness_is_excluded_from_window_check_books() {
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
        let e = witness_event("x", 33, 33, "ACT", &["ACT.7.2"]);
        assert!(window_check_books(&e).is_empty(), "a witness-row citation into a recounting chapter must be excluded exactly like a top-level verses citation");
    }

    #[test]
    fn red_df_ramah_pre_fix_date_fails_the_1sa_window() {
        let events = vec![event("df_ramah", -1014, -1014, &["1SA.19.18"])];
        let windows = vec![window("1SA", -1171, -1055)];
        let violations = window_violations(&events, &windows);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].event_id, "df_ramah");
        assert_eq!(violations[0].book, "1SA");
    }

    #[test]
    fn green_df_ramah_post_fix_date_passes_the_1sa_window() {
        let events = vec![event("df_ramah", -1062, -1062, &["1SA.19.18"])];
        let windows = vec![window("1SA", -1171, -1055)];
        assert!(window_violations(&events, &windows).is_empty());
    }

    #[test]
    fn green_theo7_passes_with_zero_exemption_spam() {
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
        let ids: Vec<&str> = ANCHOR_DEFERRALS.iter().map(|d| d.anchor_id).collect();
        assert_eq!(ids, vec!["jerusalem-falls", "cyrus-decree", "temple-finished", "ezra-returns"]);
    }

    fn anchor(id: &str, year: Year, event_id: Option<&str>, era_boundary: bool) -> ChronologyAnchor {
        ChronologyAnchor { id: id.into(), label: id.into(), year, event_id: event_id.map(String::from), era_boundary, source: "test".into(), note: None }
    }

    #[test]
    fn red_then_green_plain_anchor_equality() {
        let anchors = vec![anchor("solomon-crowned", -1015, Some("1ki_solomon_anointed"), false)];

        let wrong = vec![event("1ki_solomon_anointed", -1016, -1016, &[])];
        let (violations, deferred) = anchor_equality_check(&anchors, &wrong);
        assert!(deferred.is_empty());
        assert_eq!(violations.len(), 1);
        assert!(!violations[0].is_stale_deferral);
        assert_eq!(violations[0].anchor_id, "solomon-crowned");
        assert_eq!(violations[0].table_year, -1015);
        assert_eq!(violations[0].event_year, -1016);

        let right = vec![event("1ki_solomon_anointed", -1015, -1015, &[])];
        let (violations, deferred) = anchor_equality_check(&anchors, &right);
        assert!(violations.is_empty());
        assert!(deferred.is_empty());
    }

    #[test]
    fn a_deferred_anchor_with_an_honest_shipped_value_is_reported_not_violated() {
        let anchors = vec![anchor("jerusalem-falls", -588, Some("exl_jerusalem"), false)];
        let events = vec![event("exl_jerusalem", -586, -586, &[])];
        let (violations, deferred) = anchor_equality_check(&anchors, &events);
        assert!(violations.is_empty(), "an honest deferral must never be reported as a violation");
        assert_eq!(deferred.len(), 1);
        assert_eq!(deferred[0].anchor_id, "jerusalem-falls");
        assert_eq!(deferred[0].canonical_year, -588);
        assert_eq!(deferred[0].shipped_value, -586);
    }

    #[test]
    fn a_stale_deferral_fails_loud_like_a_real_violation() {
        let anchors = vec![anchor("jerusalem-falls", -588, Some("exl_jerusalem"), false)];
        let events = vec![event("exl_jerusalem", -580, -580, &[])];
        let (violations, deferred) = anchor_equality_check(&anchors, &events);
        assert!(deferred.is_empty());
        assert_eq!(violations.len(), 1);
        assert!(violations[0].is_stale_deferral);
        assert_eq!(violations[0].table_year, -586, "a stale-deferral violation reports the deferral's own shipped_value, not the anchor's canonical year");
        assert_eq!(violations[0].event_year, -580);
    }

    #[test]
    fn an_unbound_anchor_and_a_dangling_event_id_are_both_silently_skipped() {
        let anchors = vec![anchor("exodus", -1491, None, false), anchor("ghost", -100, Some("does-not-exist"), false)];
        let (violations, deferred) = anchor_equality_check(&anchors, &[]);
        assert!(violations.is_empty());
        assert!(deferred.is_empty());
    }
}
