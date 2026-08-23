//! HOTFIX-4 fix round 1 (review finding C-1, Critical: "the exact
//! ordering-lie bug class this batch exists to fix is still live, reachable
//! through completely ordinary Bible reading" -- `theo-460` "Resurrection
//! and Ascension"'s own FOLLOWING-IN-TIME was `jm_cana`, Christ's FIRST
//! miracle; Pentecost sorted 142 positions before the Crucifixion).
//!
//! ROOT CAUSE: HOTFIX-4's own Amendment A merged every CLEAN 1:1 Gospel-era
//! duplicate (`event_merge::EVENT_MERGE_PAIRS`), but never touched the
//! DATE of a surviving, un-merged Theographic-scale event -- the ~16
//! Gospel mega-spans that bundle 2+ real pericopes
//! (`event_merge::EVENT_DISTINCT_PAIRS`'s own Gospel-era entries, plus the
//! Holy-Week cluster batch-hotfix4-report.md's own §2d already named:
//! `theo-443`..`theo-460`) and the ~33 real, curated `acts_section` Acts
//! events (`data/curated/acts-sections.toml`, `theo-304`..`theo-336`) all
//! kept Theographic's own internal NT-era clock (Passion ~ year 30), never
//! reconciled to this app's own AD-33 Passion anchor -- so, after HOTFIX-4
//! requirement 1 wired EVERY dated event into one global timeline, they sat
//! interleaved with real, AD-33-anchored content 3 years out of place.
//!
//! CONTROLLER RULING (binding, fix-round-1 brief, Fix 1): re-date, do not
//! exclude -- excluding unreconciled events from the global timeline would
//! manufacture new dead ends, which the owner's own traversal law forbids.
//! Every surviving event still on Theographic's own internal NT clock gets
//! the SAME principled +3 calibration (verified correspondences: Baptism
//! 26->29, Crucifixion 30->33 -- Theographic's own NT chronology is
//! internally self-consistent, just uniformly offset by 3 years from this
//! app's own anchor). Provenance is stated POSITIVELY, as a calibration TO
//! the AD-33 Passion anchor -- no scale-debate commentary anywhere
//! (Amendment B, inerrancy doctrine).
//!
//! DERIVATION RULE (disclosed, not hand-picked -- `is_uncalibrated_nt_event`):
//! every event where `id` starts with `"theo-"`, `when.from_year > 0`
//! (strictly AD), and its own effective verses (`event_merge::effective_verses`)
//! touch >=1 New Testament book (Matthew..Revelation, `canon::BOOKS`'s own
//! canonical ordering). The `from_year > 0` guard matters: it excludes OT
//! genealogy-stub events (e.g. `theo-7` "Birth of Seth," dated -3874) that
//! merely CITE a NT cross-reference verse (Luke 3's own genealogy) while
//! staying correctly dated on their own ancient scale -- verified against
//! the real compiled data before this module was written: 133 events match
//! this predicate, year range a clean 26..57 (Baptism through Paul's early
//! ministry), zero outliers. Full before/after table in
//! batch-hotfix4-report.md's own "Fix round 1" section.
//!
//! WHY THIS IS ETL-ONLY, NEVER `AtlasData::finish()`: `finish()` runs
//! TWICE across the real pipeline (`atlas-etl::main` when WRITING
//! `events.json`, `atlas-server::main` when LOADING it back) -- safe for
//! `event_merge::apply_event_merges` because REMOVAL is naturally
//! idempotent (an already-absorbed id is simply absent the second time),
//! but a raw date SHIFT is not: re-running `is_uncalibrated_nt_event`
//! against an ALREADY-calibrated event would still see `id` starting with
//! `"theo-"`, `from_year > 0`, and NT verses -- and shift it a SECOND time.
//! `apply_nt_calibration` therefore runs exactly once, called from
//! `atlas_etl::main` on the RAW pre-`finish()` event set (the same timing
//! `atlas_etl::validate::run_event_merges` already uses), never from
//! `atlas_core` itself.
//!
//! ORDER_KEY, within the collision year (year 33, where the calibrated
//! mega-spans/early-Acts content now lands squarely alongside the REAL,
//! already densely-curated `pw_*`/`rob_*` Passion-Week order_key scheme,
//! `0..11_000`, `pw_jerusalem_entry`..`pw_mount_of_olives`): two regimes,
//! `calibrated_order_key` --
//! - ACTS (an effective verse in book `ACT`): a mechanical formula,
//!   `12_000 + chapter*100 + verse` from the event's own FIRST Acts verse.
//!   `12_000` is chosen strictly greater than `pw_mount_of_olives`'s own
//!   `11_000` (this app's own highest real Passion-Week order_key, i.e. the
//!   Ascension) -- guaranteeing every Acts-witnessed event sorts after the
//!   real Passion cluster, and, within Acts, follows Acts's own
//!   chapter:verse order. PROVEN, not just asserted, by the era-boundary
//!   gate test (`narrative.rs`) over the real compiled data.
//! - GOSPEL mega-spans/late-ministry freebies still landing at year 33
//!   (Acts-formula does not apply): hand-placed in
//!   `GOSPEL_ORDER_KEY_OVERRIDES` below, each keyed to the REAL curated
//!   event that marks its own first contained/nearest real pericope --
//!   "never sorts before the real event(s) it bundles begin" (fix-round-1
//!   brief, Fix 1) -- reasoning documented per row.
//! Every OTHER calibrated event (the ~95 outside year 33) keeps its
//! existing `order_key` (0, the atlas-wide default) -- verified against the
//! real data that no other post-calibration year has a collision remotely
//! resembling year 33's density.

use crate::data::Event;
use crate::refs::VerseId;

/// Theographic's own internal NT-era clock runs exactly 3 years ahead of
/// this app's AD-33 Passion anchor (Baptism 26->29, Crucifixion 30->33 --
/// both verified correspondences, this module's own doc comment above).
pub const NT_CALIBRATION_SHIFT: i32 = 3;

/// `canon::BOOKS`'s own 0-based position of Matthew -- the OT/NT boundary
/// (indices 0..38 are OT, 39..65 are NT). Not `pub`: callers should ask
/// `is_uncalibrated_nt_event`/`touches_book`, not re-derive the boundary.
const MAT_BOOK_INDEX: u8 = 39;

/// Strictly greater than `pw_mount_of_olives`'s own real `order_key`
/// (`11_000`, this app's own highest curated Passion-Week value, i.e. the
/// Ascension) -- see this module's own doc comment for the full ACTS
/// order_key regime.
const ACTS_ORDER_KEY_BASE: i32 = 12_000;

/// The derivation rule -- see this module's own doc comment for the full
/// reasoning and the real-data verification (133 events, year range
/// 26..57, zero outliers). `pub` so `atlas_etl::main` (the only caller of
/// `apply_nt_calibration`) and this module's own tests share the identical
/// predicate.
pub fn is_uncalibrated_nt_event(e: &Event) -> bool {
    if !e.id.starts_with("theo-") || e.when.from_year <= 0 {
        return false;
    }
    crate::event_merge::effective_verses(e).iter().any(|v| touches_nt(v))
}

fn touches_nt(verse_id: &str) -> bool {
    let Some(book_code) = verse_id.split('.').next() else { return false };
    matches!(crate::canon::resolve_alias(book_code), Some(id) if id.0 >= MAT_BOOK_INDEX)
}

/// This event's own EARLIEST `(chapter, verse)` among its effective verses
/// (`event_merge::effective_verses`) that fall in `book_code` -- "this
/// event's own first contained verse in this book," the same "first
/// contained pericope" concept `GOSPEL_ORDER_KEY_OVERRIDES`'s own doc
/// comment names. `None` if this event touches that book not at all.
///
/// `pub` (widened from `pub(crate)` at M-C2 -- fix round 1, Fix 1
/// acceptance (b), originally): `narrative.rs`'s own era-boundary gate
/// test ("every ACT-witnessed event sorts after the Passion cluster")
/// reuses this EXACT predicate for "is this event ACT-witnessed" -- the
/// calibration's own idea of "touches Acts" and the gate's own idea of it
/// must never disagree. M-C2: that test relocated to `atlas-graph/tests/
/// narrative_real_data.rs` (this crate cannot take a dev-dependency on
/// atlas-etl -- see that file's own doc comment), so the caller is now
/// outside this crate -- `pub(crate)` no longer reaches it, the SAME
/// "widen visibility for a legitimate cross-crate reuse" precedent
/// `atlas_core::narrative::adjacent_event` already set at M-B.
pub fn first_verse_in_book(e: &Event, book_code: &str) -> Option<(u16, u16)> {
    let mut best: Option<(u16, u16)> = None;
    for v in crate::event_merge::effective_verses(e) {
        let Ok(vid) = VerseId::parse_canonical(v) else { continue };
        if vid.book.code() != book_code {
            continue;
        }
        let pair = (vid.chapter, vid.verse);
        best = Some(match best {
            Some(b) if b < pair => b,
            _ => pair,
        });
    }
    best
}

/// Hand-placed `order_key` for the Gospel-witnessed (non-Acts) calibrated
/// events that land at year 33 -- see this module's own doc comment for
/// the two-regime rule. `(theo_id, order_key, reason)`. Every entry here
/// was individually checked against the REAL compiled year-33 order_key
/// landscape (`pw_*`/`rob_*`/`jm_*`, `0..11_000`) before being added --
/// see batch-hotfix4-report.md's own "Fix round 1" section for the full
/// derivation log.
pub const GOSPEL_ORDER_KEY_OVERRIDES: &[(&str, i32, &str)] = &[
    ("theo-294", 541, "\"Healing Multitudes\" (event_merge::EVENT_DISTINCT_PAIRS mega-span): its own extra Luke lead-in verses (LUK.6.17-19) share year 33 (post-calibration) with rob_sermon_on_the_mount (order_key 540) -- placed just after so it never sorts before that bundled pericope begins."),
    ("theo-394", 741, "\"Jesus Walks on Water\" (EVENT_DISTINCT_PAIRS mega-span) bundles rob_walks_on_water (order_key 740) -- placed just after."),
    ("theo-412", 941, "\"Feast of Tabernacles\" (EVENT_DISTINCT_PAIRS mega-span): its own first contained pericope is rob_brothers_counsel_him (order_key 940, JHN.7.2-9, the lead-in), not rob_tabernacles_feast (960) -- placed just after the earlier one."),
    ("theo-420", 981, "\"Light of the World/I am discourse\" (EVENT_DISTINCT_PAIRS mega-span): its own first verse (JHN.8.12) matches rob_light_of_the_world's own (order_key 980) exactly -- placed just after."),
    ("theo-432", 450, "\"Teaching and Healing in Perea to Jerusalem\" (JHN.10.40) -- a plain surviving freebie (not an EVENT_DISTINCT_PAIRS entry), placed just before rob_arrives_at_bethany (500): the journey that leads into it."),
    ("theo-434", 1191, "\"Chief Priests Conspire Against Jesus\" (JHN.11.47) -- placed just after rob_lazarus_effect (1190, \"the Sanhedrin resolves to kill Jesus after Lazarus's raising\"), the closest real neighbor."),
    ("theo-435", 1192, "\"Jesus Withdraws to Ephraim\" (JHN.11.54) -- placed just after theo-434, before rob_last_journey_begins (1200)."),
    ("theo-436", 1205, "\"Lepers Healed\" (LUK.17.11) -- within the last-journey teaching block (rob_last_journey_begins 1200 .. rob_prayer_parables 1210)."),
    ("theo-437", 1207, "\"Discourse on the Kingdom and Other Parables\" (LUK.17.20) -- same block, just before rob_prayer_parables (1210, Luke 18's own parables)."),
    ("theo-438", 1221, "\"Jesus Teaches in Perea\" (MAT.19.1/MRK.10.1) -- placed just after rob_teaching_on_divorce (1220, the same Matthew 19/Mark 10 divorce pericope)."),
    ("theo-440", 1271, "\"Zaccheus Converted and Parable of the Pounds\" (LUK.19.1) -- placed just after jm_jericho (1270, Zacchaeus), its own first contained pericope, before rob_parable_of_pounds (1275)."),
    ("theo-441", 1001, "\"Mary Anoints Jesus\" (JHN.12.1) -- placed just after pw_bethany (1000, \"Mary anoints Jesus at Bethany\"), its closest real neighbor."),
    ("theo-443", 2001, "\"Holy Week\" mega-span -- its own first verse (MAT.21.1) matches pw_jerusalem_entry's own (order_key 2000, the Triumphal Entry) exactly -- placed just after."),
    ("theo-446", 2500, "\"Fig Tree Cursed\" (MRK.11.11/MAT.21.18, Mark's own day-2 morning) -- between the Triumphal Entry (2000) and the Temple Cleansing (3000)."),
    ("theo-445", 3001, "\"Temple Cleansed\" -- placed just after pw_temple_cleansing (3000), its own real match."),
    ("theo-447", 3101, "\"Teaching by the Fig Tree\" (MRK.11.20, the day-3 \"found withered\" follow-up) -- placed just after rob_fig_tree_withered (3100)."),
    ("theo-448", 3201, "\"Debates in the Temple\" -- placed just after rob_authority_challenged (3200), the start of that debate cluster."),
    ("theo-452", 4201, "\"The Last Supper\" -- its own first verse (MAT.26.17) is the Passover preparation, matching rob_passover_preparation (order_key 4200) -- placed just after."),
    ("theo-453", 4801, "\"Upper Room Discourse\" (JHN.13.31) -- placed just after rob_farewell_discourse (4800), the same discourse."),
    ("theo-455", 6101, "\"Jewish Trials\" (plural -- bundles both the Annas and Caiaphas hearings) -- its own first contained pericope is rob_before_annas (order_key 6100, the earlier of the two) -- placed just after."),
    ("theo-456", 6601, "\"Roman Trials\" -- placed just after rob_before_pilate_1 (6600), the first Roman hearing."),
    ("theo-458", 6951, "\"Bearing Cross to Golgotha\" -- placed just after rob_way_to_golgotha (6950), its own real match."),
    ("theo-459", 7001, "\"Crucifixion and Burial\" -- its own first verse (JHN.19.18) is the crucifixion itself, matching pw_golgotha (order_key 7000) -- placed just after."),
    ("theo-460", 8001, "\"Resurrection and Ascension\" -- its own first verse (MAT.28.1) is the empty tomb, matching pw_jerusalem_resurrection (order_key 8000) -- placed just after; this is the pairing review scenario 1 named directly (theo-460's own FOLLOWING must not be jm_cana)."),
];

/// This event's own calibrated `order_key`, if the two-regime rule (this
/// module's own doc comment) assigns one -- `None` means "keep whatever
/// `order_key` this event already has" (the ~95 calibrated events outside
/// the year-33 collision zone).
fn calibrated_order_key(e: &Event) -> Option<i32> {
    if let Some((chapter, verse)) = first_verse_in_book(e, "ACT") {
        return Some(ACTS_ORDER_KEY_BASE + chapter as i32 * 100 + verse as i32);
    }
    GOSPEL_ORDER_KEY_OVERRIDES.iter().find(|(id, _, _)| *id == e.id).map(|&(_, k, _)| k)
}

/// One row of `apply_nt_calibration`'s own audit trail -- the fix-round-1
/// brief's own "disclose the derivation rule and a full before/after
/// table" instruction. Not serialized to the wire; ETL-report-only.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationLogEntry {
    pub id: String,
    pub label: String,
    pub old_from_year: i32,
    pub new_from_year: i32,
    pub old_order_key: i32,
    pub new_order_key: i32,
}

/// Applies the `+3` shift and (for the year-33 collision set) the
/// `order_key` placement to every event `is_uncalibrated_nt_event` selects,
/// IN PLACE. Must run exactly once, on the RAW pre-`finish()` event set --
/// see this module's own doc comment for why. Order relative to
/// `event_merge::apply_event_merges` does not matter for correctness (that
/// function is identity-only and never reads an absorbed event's own
/// `when`/`order_key`, so calibrating a freebie a moment before it is
/// merged away is harmless, not double-counted); called before it in
/// `atlas_etl::main` purely to match `validate::run_event_merges`'s own
/// existing pre-`finish()` timing.
pub fn apply_nt_calibration(events: &mut [Event]) -> Vec<CalibrationLogEntry> {
    let mut log = Vec::new();
    for e in events.iter_mut() {
        if !is_uncalibrated_nt_event(e) {
            continue;
        }
        let old_from_year = e.when.from_year;
        let old_order_key = e.order_key;

        e.when.from_year += NT_CALIBRATION_SHIFT;
        e.when.to_year += NT_CALIBRATION_SHIFT;
        if let Some(new_key) = calibrated_order_key(e) {
            e.order_key = new_key;
        }

        log.push(CalibrationLogEntry {
            id: e.id.clone(),
            label: e.label.clone(),
            old_from_year,
            new_from_year: e.when.from_year,
            old_order_key,
            new_order_key: e.order_key,
        });
    }
    log
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeRange;

    fn theo(id: &str, year: i32, verses: &[&str]) -> Event {
        Event { id: id.into(), label: id.into(), when: TimeRange::new(year, year).unwrap(), verses: verses.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }

    // --- is_uncalibrated_nt_event (the derivation rule) ---------------------

    #[test]
    fn selects_an_ad_dated_theo_event_touching_a_nt_verse() {
        assert!(is_uncalibrated_nt_event(&theo("theo-460", 30, &["MAT.28.1"])));
    }

    #[test]
    fn excludes_a_bc_dated_ot_genealogy_stub_even_if_it_cites_a_nt_cross_reference() {
        // theo-7 "Birth of Seth," dated -3874, whose own verses include a
        // NT genealogy cross-reference (LUK.3.38) -- must NOT be treated
        // as "on the Theographic NT clock" (see this module's own doc
        // comment: real, verified against the compiled data).
        assert!(!is_uncalibrated_nt_event(&theo("theo-7", -3874, &["GEN.4.25", "GEN.5.3", "LUK.3.38", "1CH.1.1"])));
    }

    #[test]
    fn excludes_a_real_curated_non_theo_event() {
        let mut e = theo("jm_cana", 30, &["JHN.2.1"]);
        e.id = "jm_cana".into();
        assert!(!is_uncalibrated_nt_event(&e), "only theo- ids are ever calibrated -- real curated events already carry their own correct AD-33-anchored dates");
    }

    #[test]
    fn excludes_an_ot_dated_theo_event_with_no_nt_verse() {
        assert!(!is_uncalibrated_nt_event(&theo("theo-1", -4004, &["GEN.1.1"])));
    }

    #[test]
    fn witness_only_verses_count_too() {
        use crate::data::EventWitness;
        use std::collections::HashMap;
        let mut translations = HashMap::new();
        translations.insert("kjv".to_string(), vec!["ACT.2.1".to_string()]);
        let e = Event {
            id: "theo-307".into(),
            when: TimeRange::new(30, 30).unwrap(),
            verses: vec![],
            witnesses: vec![EventWitness { book: "ACT".into(), translations, ref_note: None, robertson_section: None }],
            ..Default::default()
        };
        assert!(is_uncalibrated_nt_event(&e));
    }

    // --- apply_nt_calibration: the shift ------------------------------------

    #[test]
    fn red_then_green_resurrection_no_longer_sorts_adjacent_to_cana() {
        // RED (pre-calibration, review scenario 1): theo-460 at year 30
        // sits in the SAME neighborhood as jm_cana (an early-ministry real
        // event, also ~year 30) -- global (from_year, order_key) sort would
        // interleave them. GREEN: after calibration, theo-460 moves to
        // year 33, order_key 8001 -- nowhere near jm_cana's own real year.
        let mut events = vec![theo("theo-460", 30, &["MAT.28.1", "MAT.28.2"]), theo("jm_cana", 30, &["JHN.2.1"])];
        events[1].id = "jm_cana".into();

        apply_nt_calibration(&mut events);

        let resurrection = events.iter().find(|e| e.id == "theo-460").unwrap();
        assert_eq!(resurrection.when.from_year, 33, "Theographic's own year-30 Passion scale calibrates to this app's AD-33 anchor");
        assert_eq!(resurrection.order_key, 8001, "keyed just after pw_jerusalem_resurrection's own order_key (8000)");
        let cana = events.iter().find(|e| e.id == "jm_cana").unwrap();
        assert_eq!(cana.when.from_year, 30, "a real curated event is never touched by calibration");
    }

    #[test]
    fn shifts_from_year_and_to_year_together() {
        let mut events = vec![theo("theo-307", 30, &["ACT.2.1"])];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].when.from_year, 33);
        assert_eq!(events[0].when.to_year, 33);
    }

    #[test]
    fn is_idempotent_by_construction_when_called_once_the_predicate_still_matches_but_apply_nt_calibration_itself_is_never_called_twice() {
        // Documents the real safety mechanism (this module's own doc
        // comment): the PREDICATE alone is not idempotent-safe (a
        // calibrated event still matches it) -- what makes double-shifting
        // impossible is that `apply_nt_calibration` is called exactly once,
        // ETL-side, never from `AtlasData::finish()`. This test proves the
        // predicate's own non-idempotence honestly (so nobody "fixes" this
        // by calling apply_nt_calibration from finish() later without
        // reading this comment first).
        let mut events = vec![theo("theo-460", 30, &["MAT.28.1"])];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].when.from_year, 33);
        assert!(is_uncalibrated_nt_event(&events[0]), "the predicate alone does NOT become false after one shift -- calling apply_nt_calibration a second time WOULD double-shift; see this module's own doc comment for why that never happens in the real pipeline");
    }

    // --- calibrated_order_key: the two regimes ------------------------------

    #[test]
    fn acts_formula_places_every_acts_witnessed_event_above_the_acts_order_key_base() {
        let mut events = vec![theo("theo-307", 30, &["ACT.2.1"]), theo("theo-321", 30, &["ACT.7.1"])];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].order_key, 12_000 + 2 * 100 + 1);
        assert_eq!(events[1].order_key, 12_000 + 7 * 100 + 1);
        assert!(events[0].order_key > ACTS_ORDER_KEY_BASE);
    }

    #[test]
    fn acts_formula_preserves_within_acts_chapter_order() {
        let mut events = vec![theo("theo-308", 30, &["ACT.2.14"]), theo("theo-307", 30, &["ACT.2.1"])]; // declared out of order
        apply_nt_calibration(&mut events);
        let pentecost_comes = events.iter().find(|e| e.id == "theo-307").unwrap().order_key;
        let peter_preaches = events.iter().find(|e| e.id == "theo-308").unwrap().order_key;
        assert!(pentecost_comes < peter_preaches, "ACT.2.1 must sort before ACT.2.14 regardless of declaration order");
    }

    #[test]
    fn gospel_override_table_wins_when_not_acts_witnessed() {
        let mut events = vec![theo("theo-460", 30, &["MAT.28.1"])];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].order_key, 8001);
    }

    #[test]
    fn an_event_outside_both_regimes_keeps_its_existing_order_key() {
        let mut e = theo("theo-266", 26, &["MAT.3.1"]);
        e.order_key = 0;
        let mut events = vec![e];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].order_key, 0, "no override for this id and it is not ACT-witnessed -- order_key is left alone");
    }

    #[test]
    fn gospel_override_table_has_no_duplicate_ids() {
        let mut seen = std::collections::HashSet::new();
        for (id, _, _) in GOSPEL_ORDER_KEY_OVERRIDES {
            assert!(seen.insert(*id), "{id} listed twice in GOSPEL_ORDER_KEY_OVERRIDES");
        }
    }

    // --- apply_nt_calibration: the audit log --------------------------------

    #[test]
    fn log_records_every_calibrated_event_with_before_and_after_values() {
        let mut events = vec![theo("theo-460", 30, &["MAT.28.1"]), theo("theo-1", -4004, &["GEN.1.1"])];
        let log = apply_nt_calibration(&mut events);
        assert_eq!(log.len(), 1, "only the NT-clock event is logged -- theo-1 (OT, untouched) is not");
        assert_eq!(log[0].id, "theo-460");
        assert_eq!(log[0].old_from_year, 30);
        assert_eq!(log[0].new_from_year, 33);
        assert_eq!(log[0].old_order_key, 0);
        assert_eq!(log[0].new_order_key, 8001);
    }
}
