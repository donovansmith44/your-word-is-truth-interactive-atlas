//! The imported corpus's own NT-era clock runs a fixed three years ahead of this atlas's
//! Passion anchor, so every event still on it is shifted onto the one scale -- exactly
//! once, ETL-side, because the shift is not idempotent.

use crate::data::Event;
use crate::refs::VerseId;

/// Verified correspondences: Baptism 26 -> 29, Crucifixion 30 -> 33.
pub const NT_CALIBRATION_SHIFT: i32 = 3;

/// Read from the canon rather than restated, and private so callers ask the predicates
/// below instead of re-deriving the boundary.
const MAT_BOOK_INDEX: u8 = crate::canon::BOOKS_IN_THE_OLD_TESTAMENT as u8;

/// Strictly greater than the highest curated Passion-Week `order_key`, so every
/// Acts-witnessed event sorts after that cluster.
const ACTS_ORDER_KEY_BASE: i32 = 12_000;

/// The `from_year > 0` guard is load-bearing: it excludes an ancient event that merely
/// cites a New Testament cross-reference and is already correctly dated.
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

/// `None` when the event does not touch that book at all. Public so "touches this book"
/// means the same thing to the calibration and to the timeline gates that check it.
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

/// `(id, order_key, reason)`, hand-placed for the calibrated events that land in the
/// crowded anchor year: each is keyed so a span never sorts before the curated events it
/// bundles begin.
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

/// `None` means keep whatever `order_key` the event already has.
fn calibrated_order_key(e: &Event) -> Option<i32> {
    if let Some((chapter, verse)) = first_verse_in_book(e, "ACT") {
        return Some(ACTS_ORDER_KEY_BASE + chapter as i32 * 100 + verse as i32);
    }
    GOSPEL_ORDER_KEY_OVERRIDES.iter().find(|(id, _, _)| *id == e.id).map(|&(_, k, _)| k)
}

/// An audit trail for the ETL report; never serialized to the wire.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationLogEntry {
    pub id: String,
    pub label: String,
    pub old_from_year: i32,
    pub new_from_year: i32,
    pub old_order_key: i32,
    pub new_order_key: i32,
}

/// Must run exactly once, on the raw event set: the predicate still matches an
/// already-calibrated event, so a second pass would shift it again.
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

    #[test]
    fn selects_an_ad_dated_theo_event_touching_a_nt_verse() {
        assert!(is_uncalibrated_nt_event(&theo("theo-460", 30, &["MAT.28.1"])));
    }

    #[test]
    fn excludes_a_bc_dated_ot_genealogy_stub_even_if_it_cites_a_nt_cross_reference() {
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

    #[test]
    fn red_then_green_resurrection_no_longer_sorts_adjacent_to_cana() {
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
        let mut events = vec![theo("theo-460", 30, &["MAT.28.1"])];
        apply_nt_calibration(&mut events);
        assert_eq!(events[0].when.from_year, 33);
        assert!(is_uncalibrated_nt_event(&events[0]), "the predicate alone does NOT become false after one shift -- calling apply_nt_calibration a second time WOULD double-shift; see this module's own doc comment for why that never happens in the real pipeline");
    }

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
        let mut events = vec![theo("theo-308", 30, &["ACT.2.14"]), theo("theo-307", 30, &["ACT.2.1"])];
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
