//! Scene composition: turns an `AtlasData` plus a time window or scripture
//! reference into the wire-level `Scene` the client renders.

use std::collections::{HashMap, HashSet};

use crate::data::{AtlasData, Event};
use crate::refs::{ScriptureRef, VerseId};
use crate::time::TimeRange;
use crate::wire::{Scene, SceneArrow, SceneEvent, SceneNarrative, ScenePlace, VerseGroup};

/// Time-mode scene: every event whose `when` intersects the window lights up
/// all of its places; arrows connect each narrative's kept legs in order.
pub fn compose_time_scene(d: &AtlasData, w: TimeRange) -> Scene {
    let kept: Vec<&Event> = d.events.iter().filter(|e| e.when.intersects(&w)).collect();
    let places = lit_places(d, &kept, None); // group kept events by ALL their places
    let arrows = build_arrows(d, &w, None);
    let narratives = legend(d, &w, None, &arrows);
    Scene { mode: "time".into(), window: Some(w), sref: None, places, arrows, narratives }
}

/// Scripture-mode scene: lit places are the union of (a) places touched by
/// events with >=1 verse inside `r`, and (b) places whose geocoding
/// `verse_links` alone match `r` (get a single synthetic `mention-*`
/// pseudo-event; see below). A place lit via both keeps only its real,
/// ref-matching events — the pseudo-event never appears alongside real ones.
pub fn compose_scripture_scene(d: &AtlasData, r: &ScriptureRef) -> Scene {
    let kept: Vec<&Event> = d
        .events
        .iter()
        .filter(|e| e.verses.iter().any(|v| ref_contains(r, &VerseId::parse_canonical(v).expect("etl-validated verse id"))))
        .collect();
    let mut places = lit_places(d, &kept, Some(r));
    let event_lit: HashSet<String> = places.iter().map(|p| p.id.clone()).collect();

    for place in &d.places {
        if event_lit.contains(&place.id) {
            continue;
        }
        // Synthetic text-mention pseudo-event: `mention-*` ids never appear in
        // `d.events`/narrative legs, so arrows never reference them (ARROW-7 is
        // vacuously satisfied for them — there is nothing to check).
        let matched: Vec<String> = place
            .verse_links
            .iter()
            .filter(|v| ref_contains(r, &VerseId::parse_canonical(v).expect("etl-validated verse id")))
            .cloned()
            .collect();
        if matched.is_empty() {
            continue;
        }
        let mention = SceneEvent {
            id: format!("mention-{}", place.id),
            label: "Mentioned".into(),
            when: TimeRange::new(-4004, 100).unwrap(),
            verse_groups: verse_groups_for(&matched, Some(r)),
        };
        let events = vec![mention];
        places.push(ScenePlace {
            id: place.id.clone(),
            name: place.name.clone(),
            lat: place.lat,
            lon: place.lon,
            brightness: (events.len().min(5)) as u8,
            events,
        });
    }
    places.sort_by(|a, b| a.id.cmp(&b.id));

    // build_arrows/legend need *some* TimeRange even though scripture mode
    // ignores it entirely (the `Some(r)` branch never reads `w`); the whole
    // atlas span reads best if anyone ever logs/debugs it.
    let span = TimeRange::new(-4004, 100).unwrap();
    let arrows = build_arrows(d, &span, Some(r));
    let narratives = legend(d, &span, Some(r), &arrows);

    Scene { mode: "scripture".into(), window: None, sref: Some(r.to_string()), places, arrows, narratives }
}

/// Book matches book; Chapter matches book+chapter; Passage matches
/// book+chapter and `from_verse <= v.verse <= to_verse`; Verse matches exactly.
pub fn ref_contains(r: &ScriptureRef, v: &VerseId) -> bool {
    match r {
        ScriptureRef::Book(b) => *b == v.book,
        ScriptureRef::Chapter { book, chapter } => *book == v.book && *chapter == v.chapter,
        ScriptureRef::Passage { book, chapter, from_verse, to_verse } => {
            *book == v.book && *chapter == v.chapter && *from_verse <= v.verse && v.verse <= *to_verse
        }
        ScriptureRef::Verse(vid) => *vid == *v,
    }
}

/// Shared by both modes (see brief §7.4). `r == None` filters a narrative's
/// legs by `when.intersects(w)`; `r == Some(sr)` filters by verse match
/// instead (`w` is then unused). Consecutive kept legs at the same anchor
/// place are skipped (the chain passes through); `order` still increments
/// for every consecutive pair so orders stay stable across skips.
fn build_arrows(d: &AtlasData, w: &TimeRange, r: Option<&ScriptureRef>) -> Vec<SceneArrow> {
    let mut out = Vec::new();
    for n in &d.narratives {
        let kept: Vec<&Event> = n
            .legs
            .iter()
            .filter_map(|id| d.event_by_id(id))
            .filter(|e| match r {
                None => e.when.intersects(w),
                Some(sr) => e.verses.iter().any(|v| ref_contains(sr, &VerseId::parse_canonical(v).expect("etl-validated"))),
            })
            .collect();
        let mut order = 0u32;
        for pair in kept.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            order += 1;
            if a.places[0] == b.places[0] {
                continue; // same-place: chain passes through
            }
            out.push(SceneArrow {
                narrative: n.id.clone(),
                color: n.color.clone(),
                from_place: a.places[0].clone(),
                to_place: b.places[0].clone(),
                from_event: a.id.clone(),
                to_event: b.id.clone(),
                order,
            });
        }
    }
    out
}

/// Groups `kept` events by every place they touch (not just the anchor) into
/// `ScenePlace`s. `brightness = min(attached_event_count, 5)`. `r` is `Some`
/// only for scripture-mode composition, where it must be threaded through to
/// `verse_groups_for` so the 20-per-group verse cap can never silently drop
/// the very verse that earned an event (and therefore its place) a spot in
/// the scene — see `verse_groups_for`'s doc comment (SCRIP-1/SCRIP-3).
fn lit_places(d: &AtlasData, kept: &[&Event], r: Option<&ScriptureRef>) -> Vec<ScenePlace> {
    let mut by_place: HashMap<&str, Vec<&Event>> = HashMap::new();
    for e in kept {
        for pid in &e.places {
            by_place.entry(pid.as_str()).or_default().push(e);
        }
    }
    let mut places: Vec<ScenePlace> = by_place
        .into_iter()
        .filter_map(|(pid, evs)| {
            let place = d.place_by_id(pid)?;
            Some(ScenePlace {
                id: place.id.clone(),
                name: place.name.clone(),
                lat: place.lat,
                lon: place.lon,
                brightness: (evs.len().min(5)) as u8,
                events: evs
                    .iter()
                    .map(|e| SceneEvent {
                        id: e.id.clone(),
                        label: e.label.clone(),
                        when: e.when,
                        verse_groups: verse_groups_for(&e.verses, r),
                    })
                    .collect(),
            })
        })
        .collect();
    places.sort_by(|a, b| a.id.cmp(&b.id));
    places
}

/// `pub` (not private) because atlas-server's verse/place-detail endpoints
/// reuse it to build the same SceneEvent shape scenes use, rather than
/// duplicating the verse-grouping logic in a different crate. Always passes
/// `r = None` to `verse_groups_for`: neither call site (a single verse's own
/// detail page, a place's full event history) is scoped to one scripture
/// ref, so there is nothing to prioritize under the cap.
pub fn to_scene_event(e: &Event) -> SceneEvent {
    SceneEvent { id: e.id.clone(), label: e.label.clone(), when: e.when, verse_groups: verse_groups_for(&e.verses, None) }
}

/// Groups canonical verse ids by (book, chapter); within a group sorts by
/// verse number ascending; caps each group at 20 ids with `count` set to the
/// true total before capping.
///
/// `r` is `Some` only for scripture-mode composition (`lit_places`'s
/// scripture-mode call and the synthetic mention-event above). A place/event
/// is only ever in a scripture-mode scene BECAUSE one of its verses satisfies
/// `ref_contains(r, ..)`; some real events carry 50-70+ verses in a single
/// chapter (e.g. a Passion-week summary event spanning all of Luke 22), so
/// plain ascending truncation can cap away the exact verse that justified
/// the event's (and its place's) inclusion, leaving a lit place with no
/// visible evidence it belongs in the scene — a SCRIP-1/SCRIP-3 violation.
/// When `r` is given, verses satisfying it are moved to the front of the
/// cap; remaining slots (if any) are filled with the lowest-numbered
/// leftover verses, and the selected set is re-sorted ascending before
/// returning, so the wire shape (ascending, <=20, true `count`) is
/// unchanged — only which 20 verses win the cap can differ. Time mode (and
/// `to_scene_event`'s other two callers) pass `None` and get the original
/// plain-ascending truncation, since there is no ref to prioritize.
fn verse_groups_for(verses: &[String], r: Option<&ScriptureRef>) -> Vec<VerseGroup> {
    let mut groups: HashMap<(String, u16), Vec<(u16, String)>> = HashMap::new();
    for v in verses {
        let vid = VerseId::parse_canonical(v).expect("etl-validated verse id");
        groups.entry((vid.book.code().to_string(), vid.chapter)).or_default().push((vid.verse, v.clone()));
    }
    let mut out: Vec<VerseGroup> = groups
        .into_iter()
        .map(|((book, chapter), mut vs)| {
            vs.sort_by_key(|(vnum, _)| *vnum);
            let count = vs.len() as u32;
            let capped: Vec<(u16, String)> = match r {
                Some(r) => {
                    let (mut matched, mut rest): (Vec<(u16, String)>, Vec<(u16, String)>) =
                        vs.into_iter().partition(|(_, v)| {
                            ref_contains(r, &VerseId::parse_canonical(v).expect("etl-validated verse id"))
                        });
                    if matched.len() < 20 {
                        rest.truncate(20 - matched.len());
                        matched.extend(rest);
                    } else {
                        matched.truncate(20);
                    }
                    matched.sort_by_key(|(vnum, _)| *vnum);
                    matched
                }
                None => vs.into_iter().take(20).collect(),
            };
            let verses: Vec<String> = capped.into_iter().map(|(_, s)| s).collect();
            VerseGroup { book, chapter, verses, count }
        })
        .collect();
    out.sort_by(|a, b| a.book.cmp(&b.book).then(a.chapter.cmp(&b.chapter)));
    out
}

/// The scene legend: narratives with >=1 arrow in this scene, each carrying
/// `legs_in_scene` = the count of that narrative's KEPT events under the
/// scene's filter (not the arrow count — a same-place skip run can leave an
/// event with no arrow endpoint at all, so counting via arrows would
/// undercount). This needs the same `(w, r)` filter `build_arrows` used,
/// which is why it takes them too rather than only `(d, arrows)`.
fn legend(d: &AtlasData, w: &TimeRange, r: Option<&ScriptureRef>, arrows: &[SceneArrow]) -> Vec<SceneNarrative> {
    let active: HashSet<&str> = arrows.iter().map(|a| a.narrative.as_str()).collect();
    let mut out: Vec<SceneNarrative> = d
        .narratives
        .iter()
        .filter(|n| active.contains(n.id.as_str()))
        .map(|n| {
            let legs_in_scene = n
                .legs
                .iter()
                .filter_map(|id| d.event_by_id(id))
                .filter(|e| match r {
                    None => e.when.intersects(w),
                    Some(sr) => e.verses.iter().any(|v| ref_contains(sr, &VerseId::parse_canonical(v).expect("etl-validated"))),
                })
                .count() as u32;
            SceneNarrative { id: n.id.clone(), name: n.name.clone(), color: n.color.clone(), legs_in_scene }
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Canon, Narrative, Place};
    use crate::time::next_year;
    use proptest::prelude::*;
    use std::collections::HashMap;

    // The hand-built demo world used throughout this test module (4 places,
    // one narrative `conquest` with a same-place skip case, a second
    // narrative `patriarchs-demo` sharing one of `conquest`'s events, and an
    // unrelated bare event; `hebron` is additionally geocoding-linked to
    // GEN.13.18 for scripture-mode tests) lives in
    // `atlas_core::data::demo_fixture` (promoted there in Task 6 so
    // atlas-server's integration tests, a different crate, can reach it at
    // normal — not `#[cfg(test)]` — compile time). Every test below calls it
    // directly rather than through a local wrapper, per Task 6's brief.

    #[test]
    fn time_scene_lights_only_intersecting() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let ids: Vec<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"gilgal") && ids.contains(&"jericho") && ids.contains(&"ai"));
        assert!(!ids.contains(&"hebron")); // -2000 outside window
    }

    #[test]
    fn arrows_skip_same_place_and_chain() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let conquest: Vec<_> = s.arrows.iter().filter(|a| a.narrative == "conquest").collect();
        // e1->e2 (gilgal->jericho), e2->e3 skipped (same place), e3->e4 (jericho->ai)
        assert_eq!(conquest.len(), 2);
        assert_eq!((conquest[0].from_place.as_str(), conquest[0].to_place.as_str()), ("gilgal", "jericho"));
        assert_eq!((conquest[1].from_place.as_str(), conquest[1].to_place.as_str()), ("jericho", "ai"));
        assert_eq!(conquest[0].to_place, conquest[1].from_place); // ARROW-3 place-chain
    }

    #[test]
    fn scripture_scene_uses_links_not_dates() {
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("GEN.13.18").unwrap());
        assert_eq!(s.mode, "scripture");
        // hebron's real event (e5) is about GEN.23, not GEN.13.18, so it must
        // be lit solely via the geocoding link, i.e. a synthetic mention-*
        // pseudo-event, and its full shape must match the brief exactly.
        let hebron = s.places.iter().find(|p| p.id == "hebron").expect("hebron should be geocoding-lit");
        assert_eq!(hebron.events.len(), 1);
        let mention = &hebron.events[0];
        assert_eq!(mention.id, "mention-hebron");
        assert_eq!(mention.label, "Mentioned");
        assert_eq!(mention.when, TimeRange::new(-4004, 100).unwrap());
        assert_eq!(mention.verse_groups.len(), 1);
        assert_eq!(mention.verse_groups[0].book, "GEN");
        assert_eq!(mention.verse_groups[0].chapter, 13);
        assert!(mention.verse_groups[0].verses.contains(&"GEN.13.18".to_string()));
        assert_eq!(mention.verse_groups[0].count, 1);
    }

    #[test]
    fn scripture_scene_lights_ref_matching_events_and_builds_arrows() {
        // Exercises the real-event scripture path that GEN.13.18 above does
        // not: build_arrows' `Some(sr)` branch, legend's ref branch, and the
        // `event_lit` guard that suppresses a mention-* pseudo-event for a
        // place already lit by a real matching event.
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("JOS").unwrap());
        assert_eq!(s.mode, "scripture");

        let ids: Vec<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"gilgal") && ids.contains(&"jericho") && ids.contains(&"ai"));
        // hebron has no JOS verse_link and its only event (e5) is GEN, not
        // JOS, so it must not appear at all — and in particular must carry
        // no mention-* pseudo-event.
        assert!(!ids.contains(&"hebron"));
        assert!(!s.places.iter().any(|p| p.events.iter().any(|e| e.id.starts_with("mention-"))));

        let conquest: Vec<_> = s.arrows.iter().filter(|a| a.narrative == "conquest").collect();
        assert_eq!(conquest.len(), 2);
        assert_eq!(
            (conquest[0].from_place.as_str(), conquest[0].to_place.as_str(), conquest[0].order),
            ("gilgal", "jericho", 1)
        );
        assert_eq!(
            (conquest[1].from_place.as_str(), conquest[1].to_place.as_str(), conquest[1].order),
            ("jericho", "ai", 3)
        );
    }

    #[test]
    fn brightness_and_caps() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let jericho = s.places.iter().find(|p| p.id == "jericho").unwrap();
        assert_eq!(jericho.brightness, 2u8.min(5)); // e2 and e3
        for pl in &s.places {
            for ev in &pl.events {
                for g in &ev.verse_groups {
                    assert!(g.verses.len() <= 20);
                    assert!(g.count as usize >= g.verses.len());
                }
            }
        }
    }

    #[test]
    fn ref_contains_book_matches_any_chapter_and_verse() {
        let gen = crate::canon::resolve_alias("GEN").unwrap();
        let exo = crate::canon::resolve_alias("EXO").unwrap();
        let r = ScriptureRef::Book(gen);
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 1, verse: 1 }));
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 50, verse: 26 })); // last verse of Genesis
        assert!(!ref_contains(&r, &VerseId { book: exo, chapter: 1, verse: 1 })); // wrong book
    }

    #[test]
    fn ref_contains_chapter_matches_book_and_chapter_only() {
        let gen = crate::canon::resolve_alias("GEN").unwrap();
        let exo = crate::canon::resolve_alias("EXO").unwrap();
        let r = ScriptureRef::Chapter { book: gen, chapter: 13 };
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 13, verse: 1 }));
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 13, verse: 18 }));
        assert!(!ref_contains(&r, &VerseId { book: gen, chapter: 12, verse: 18 })); // wrong chapter
        assert!(!ref_contains(&r, &VerseId { book: gen, chapter: 14, verse: 1 })); // wrong chapter
        assert!(!ref_contains(&r, &VerseId { book: exo, chapter: 13, verse: 1 })); // wrong book, same chapter number
    }

    #[test]
    fn ref_contains_passage_boundaries_inclusive() {
        let jos = crate::canon::resolve_alias("JOS").unwrap();
        let r = ScriptureRef::Passage { book: jos, chapter: 6, from_verse: 2, to_verse: 5 };
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 2 })); // lower boundary: in
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 5 })); // upper boundary: in
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 3 })); // interior: in
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 1 })); // just below: out
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 6 })); // just above: out
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 5, verse: 3 })); // right verse, wrong chapter
    }

    #[test]
    fn ref_contains_verse_matches_exactly() {
        let jos = crate::canon::resolve_alias("JOS").unwrap();
        let r = ScriptureRef::Verse(VerseId { book: jos, chapter: 6, verse: 20 });
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 20 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 21 })); // wrong verse
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 7, verse: 20 })); // wrong chapter
    }

    #[test]
    fn legend_legs_in_scene_counts_kept_not_arrows_and_excludes_zero_arrow_narratives() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        // conquest has 4 kept legs (e1..e4 all intersect the window) but only
        // 2 arrows (e2->e3 is a same-place skip) — legs_in_scene must read the
        // true kept count, not the arrow count (ruling 4).
        let conquest = s.narratives.iter().find(|n| n.id == "conquest").expect("conquest has arrows, so it's in the legend");
        assert_eq!(conquest.legs_in_scene, 4);
        assert_eq!(conquest.name, "The Conquest");
        assert_eq!(conquest.color, "#7C3AED");
        // patriarchs-demo's one leg (e2) is kept by the window filter, but a
        // lone leg produces zero arrows, so it must be absent from the legend
        // entirely (membership requires >=1 arrow).
        assert!(s.narratives.iter().all(|n| n.id != "patriarchs-demo"));
    }

    #[test]
    fn verse_groups_cap_at_20_sorted_ascending_with_true_count() {
        // Standalone dataset (not `fixture()`) so it can't perturb the other
        // example tests' expectations: one event with 25 verses in a single
        // chapter, supplied in descending (i.e. unsorted) order.
        let mut verses: Vec<String> = (1..=25).map(|v| format!("JOS.10.{v}")).collect();
        verses.reverse();

        let places = vec![Place { id: "many-verses".into(), name: "Many Verses".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: "big-event".into(),
            label: "Long battle".into(),
            when: TimeRange::new(-1400, -1400).unwrap(),
            places: vec!["many-verses".into()],
            verses,
        }];
        let d = AtlasData::new(Canon { books: vec![] }, places, events, vec![], vec![], vec![], HashMap::new(), HashMap::new())
            .finish();

        let s = compose_time_scene(&d, TimeRange::new(-1400, -1400).unwrap());
        let place = s.places.iter().find(|p| p.id == "many-verses").unwrap();
        assert_eq!(place.events.len(), 1);
        assert_eq!(place.events[0].verse_groups.len(), 1);
        let g = &place.events[0].verse_groups[0];
        assert_eq!(g.book, "JOS");
        assert_eq!(g.chapter, 10);
        assert_eq!(g.count, 25); // true total before capping
        assert_eq!(g.verses.len(), 20); // capped
        let expected: Vec<String> = (1..=20).map(|v| format!("JOS.10.{v}")).collect();
        assert_eq!(g.verses, expected); // sorted ascending by verse number, not input order
    }

    /// A larger, still plain-deterministic dataset (no proptest seed input;
    /// see controller ruling — the brief's "seed vector" phrasing is
    /// superseded by its own "built deterministically" parenthetical):
    /// 12 places, 3 narratives with 5-9 legs each, years spread across the
    /// whole atlas span. Every narrative's legs are non-decreasing by
    /// `when.from_year` — this mirrors an ETL-validated invariant and is
    /// required for the ARROW-6 property to hold (build_arrows trusts leg
    /// order; it does not re-sort by year).
    pub(crate) fn big_fixture() -> AtlasData {
        let places = vec![
            Place { id: "gilgal".into(), name: "Gilgal".into(), lat: 31.9000, lon: 35.4500, verse_links: vec![] },
            Place { id: "jericho".into(), name: "Jericho".into(), lat: 31.8703, lon: 35.4436, verse_links: vec![] },
            Place { id: "ai".into(), name: "Ai".into(), lat: 31.9339, lon: 35.2856, verse_links: vec![] },
            Place { id: "hebron".into(), name: "Hebron".into(), lat: 31.5326, lon: 35.0998, verse_links: vec![] },
            Place { id: "shiloh".into(), name: "Shiloh".into(), lat: 32.0553, lon: 35.2897, verse_links: vec![] },
            Place { id: "bethel".into(), name: "Bethel".into(), lat: 31.9306, lon: 35.2183, verse_links: vec![] },
            Place { id: "shechem".into(), name: "Shechem".into(), lat: 32.2132, lon: 35.2778, verse_links: vec![] },
            Place { id: "dan".into(), name: "Dan".into(), lat: 33.2489, lon: 35.6519, verse_links: vec![] },
            Place { id: "beersheba".into(), name: "Beersheba".into(), lat: 31.2589, lon: 34.7913, verse_links: vec![] },
            Place { id: "jerusalem".into(), name: "Jerusalem".into(), lat: 31.7683, lon: 35.2137, verse_links: vec![] },
            Place { id: "samaria".into(), name: "Samaria".into(), lat: 32.2778, lon: 35.1972, verse_links: vec![] },
            Place { id: "damascus".into(), name: "Damascus".into(), lat: 33.5138, lon: 36.2765, verse_links: vec![] },
        ];

        fn leg(id: &str, place: &str, year: i32) -> Event {
            Event {
                id: id.into(),
                label: format!("{id} event"),
                when: TimeRange::new(year, year).unwrap(),
                places: vec![place.into()],
                verses: vec![],
            }
        }

        // Clustered mid-span so that typical (wide) generated windows catch
        // several consecutive legs at once, not just the full-span window.
        let na = vec![
            leg("na-1", "gilgal", -1500),
            leg("na-2", "jericho", -1450),
            leg("na-3", "jericho", -1400), // same place as na-2: skip
            leg("na-4", "ai", -1350),
            leg("na-5", "bethel", -1300),
            leg("na-6", "bethel", -1250), // same place as na-5: skip
            leg("na-7", "shechem", -1200),
        ];
        let nb = vec![
            leg("nb-1", "dan", -800),
            leg("nb-2", "samaria", -750),
            leg("nb-3", "jerusalem", -700),
            leg("nb-4", "jerusalem", -650), // same place as nb-3: skip
            leg("nb-5", "beersheba", -600),
            leg("nb-6", "damascus", -550),
        ];
        // Spans nearly the whole atlas span, including the BC/AD adjacency.
        let nc = vec![
            leg("nc-1", "hebron", -4000),
            leg("nc-2", "shiloh", -3000),
            leg("nc-3", "shiloh", -2000), // same place as nc-2: skip
            leg("nc-4", "gilgal", -1000),
            leg("nc-5", "jericho", -100),
            leg("nc-6", "jerusalem", -1),
            leg("nc-7", "jerusalem", 1), // same place as nc-6: skip
            leg("nc-8", "damascus", 30),
        ];

        let narratives = vec![
            Narrative {
                id: "na".into(),
                name: "Narrative A".into(),
                color: "#1F77B4".into(),
                legs: na.iter().map(|e| e.id.clone()).collect(),
            },
            Narrative {
                id: "nb".into(),
                name: "Narrative B".into(),
                color: "#2CA02C".into(),
                legs: nb.iter().map(|e| e.id.clone()).collect(),
            },
            Narrative {
                id: "nc".into(),
                name: "Narrative C".into(),
                color: "#D62728".into(),
                legs: nc.iter().map(|e| e.id.clone()).collect(),
            },
        ];

        let mut events = Vec::new();
        events.extend(na);
        events.extend(nb);
        events.extend(nc);

        AtlasData::new(Canon { books: vec![] }, places, events, narratives, vec![], vec![], HashMap::new(), HashMap::new())
            .finish()
    }

    /// Mirrors Task 2's private `time::tests::range_strategy` (already
    /// clamped to the atlas span).
    fn window_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }

    /// Self-review sanity check (not one of the brief's given tests): confirm
    /// big_fixture can actually produce arrows, both over the full span and
    /// over a realistically-sized window, so the proptest invariants below
    /// are not vacuously true. na's cluster (-1500..-1200) is only 300 years
    /// wide against window_strategy's ~1368-year average width, so most
    /// generated windows that touch that region catch multiple legs.
    #[test]
    fn big_fixture_has_arrow_bearing_windows() {
        let d = big_fixture();
        let full = compose_time_scene(&d, TimeRange::new(-4004, 100).unwrap());
        assert!(!full.arrows.is_empty(), "full-span scene should have arrows");
        let mid = compose_time_scene(&d, TimeRange::new(-1600, -1100).unwrap());
        assert!(!mid.arrows.is_empty(), "na's cluster window should have arrows");
    }

    proptest! {
        #[test]
        fn arrow_invariants(w in window_strategy()) {
            let d = big_fixture();
            let s = compose_time_scene(&d, w);
            let place_ids: std::collections::HashSet<_> = s.places.iter().map(|p| &p.id).collect();
            for n in s.narratives.iter() {
                let mut arrows: Vec<_> = s.arrows.iter().filter(|a| a.narrative == n.id).collect();
                arrows.sort_by_key(|a| a.order);
                for a in &arrows {
                    prop_assert!(place_ids.contains(&a.from_place) && place_ids.contains(&a.to_place)); // ARROW-1
                    prop_assert_eq!(&a.color, &n.color);                                                // ARROW-2
                    prop_assert_ne!(&a.from_place, &a.to_place);                                        // ARROW-5
                    let (fe, te) = (d.event_by_id(&a.from_event).unwrap(), d.event_by_id(&a.to_event).unwrap());
                    prop_assert!(te.when.from_year >= fe.when.from_year);                               // ARROW-6
                    let from_sp = s.places.iter().find(|p| p.id == a.from_place).unwrap();
                    prop_assert!(from_sp.events.iter().any(|e| e.id == a.from_event));                  // ARROW-7
                    let to_sp = s.places.iter().find(|p| p.id == a.to_place).unwrap();
                    prop_assert!(to_sp.events.iter().any(|e| e.id == a.to_event));                      // ARROW-7
                }
                for pair in arrows.windows(2) {
                    prop_assert_eq!(&pair[0].to_place, &pair[1].from_place);                            // ARROW-3
                }
                if let (Some(first), Some(last)) = (arrows.first(), arrows.last()) {
                    prop_assert!(!arrows.iter().any(|a| a.to_event == first.from_event));               // ARROW-4
                    prop_assert!(!arrows.iter().any(|a| a.from_event == last.to_event));                // ARROW-4
                }
            }
            // SCENE-2 mirror
            for p in &s.places {
                prop_assert!(!p.events.is_empty());
                prop_assert_eq!(p.brightness, (p.events.len() as u8).min(5));
                for e in &p.events { prop_assert!(e.when.intersects(&w)); }
            }
        }
        #[test]
        fn window_monotonicity(w in window_strategy()) { // SCENE-3 mirror
            let d = big_fixture();
            let grow = TimeRange::new(
                if w.from_year == 1 { -1 } else { w.from_year - 1 },
                next_year(w.to_year).min(100).max(w.to_year)).unwrap();
            let (s1, s2) = (compose_time_scene(&d, w), compose_time_scene(&d, grow));
            let ids2: std::collections::HashSet<_> = s2.places.iter().map(|p| p.id.clone()).collect();
            for p in &s1.places { prop_assert!(ids2.contains(&p.id)); }
        }
    }
}
