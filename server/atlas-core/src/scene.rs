//! Scene composition: turns a `&dyn SceneSource` plus a time window or
//! scripture reference into the wire-level `Scene` the client renders.

use std::collections::{HashMap, HashSet};

use crate::data::Event;
use crate::history::{resolve_display_name, resolve_existence};
use crate::refs::{ScriptureRef, VerseId};
use crate::scene_source::SceneSource;
use crate::time::TimeRange;
use crate::wire::{QuietPlace, Scene, SceneArrow, SceneEvent, SceneMode, SceneNarrative, ScenePlace, VerseGroup};

pub fn compose_time_scene(d: &dyn SceneSource, w: TimeRange) -> Scene {
    let kept: Vec<&Event> = d.events_in_window(&w);
    let places = lit_places(d, &kept, None, Some(w));
    let quiet = quiet_places(d, &places, w);
    let arrows = build_arrows(d, &w, None);
    let narratives = legend(d, &w, None, &arrows);
    Scene { mode: SceneMode::Time, window: Some(w), sref: None, places, quiet_places: quiet, arrows, narratives }
}

/// Lit places are the union of those touched by an event with a verse inside `r` and those
/// whose geocoding links alone match it. A place lit both ways keeps only its real
/// events; the synthetic mention event never stands beside one.
pub fn compose_scripture_scene(d: &dyn SceneSource, r: &ScriptureRef) -> Scene {
    let kept: Vec<&Event> = d.events_matching_ref(r);
    // No window here: a place is lit by its geocoded verse links, whose text already names
    // it, so a curated period name could contradict the verse the reader is looking at.
    let mut places = lit_places(d, &kept, Some(r), None);
    let event_lit: HashSet<String> = places.iter().map(|p| p.id.clone()).collect();

    for place in d.places() {
        if event_lit.contains(&place.id) {
            continue;
        }
        // A `mention-*` id is never in the source's events or a narrative leg, so no arrow
        // can reference one.
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
        let (existence_from, existence_to) = resolve_existence(d.place_history_for(&place.id));
        places.push(ScenePlace {
            id: place.id.clone(),
            name: place.name.clone(),
            display_name: resolve_display_name(&place.name, d.place_history_for(&place.id), None, d.place_name_alias_for(&place.id)),
            lat: place.lat,
            lon: place.lon,
            brightness: (events.len().min(5)) as u8,
            events,
            existence_from,
            existence_to,
            merged_ids: crate::merge::absorbed_ids_for(&place.id),
        });
    }
    places.sort_by(|a, b| a.id.cmp(&b.id));

    // Scripture mode ignores the window entirely, but the shared helpers still take one.
    let span = TimeRange::new(-4004, 100).unwrap();
    let arrows = build_arrows(d, &span, Some(r));
    let narratives = legend(d, &span, Some(r), &arrows);

    // No window here to resolve "not yet active" against, so the array is always empty.
    Scene { mode: SceneMode::Scripture, window: None, sref: Some(r.to_string()), places, quiet_places: vec![], arrows, narratives }
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

/// `r == None` filters a narrative's legs by the window; `Some` filters by verse match and
/// `w` is then unused. Consecutive kept legs at one place are skipped, but `order` still
/// increments per consecutive pair so orders stay stable across a skip.
fn build_arrows(d: &dyn SceneSource, w: &TimeRange, r: Option<&ScriptureRef>) -> Vec<SceneArrow> {
    let mut out = Vec::new();
    for n in d.narratives() {
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
                continue;
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

/// Groups by every place an event touches, not just its anchor. `r` is threaded through to
/// the verse cap so it can never drop the very verse that earned the place its spot.
/// `name_window` is a separate concept: the window the display name resolves against.
fn lit_places(d: &dyn SceneSource, kept: &[&Event], r: Option<&ScriptureRef>, name_window: Option<TimeRange>) -> Vec<ScenePlace> {
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
            let (existence_from, existence_to) = resolve_existence(d.place_history_for(&place.id));
            Some(ScenePlace {
                id: place.id.clone(),
                name: place.name.clone(),
                display_name: resolve_display_name(&place.name, d.place_history_for(&place.id), name_window, d.place_name_alias_for(&place.id)),
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
                existence_from,
                existence_to,
                merged_ids: crate::merge::absorbed_ids_for(&place.id),
            })
        })
        .collect();
    places.sort_by(|a, b| a.id.cmp(&b.id));
    places
}

/// The `!lit` filter makes lit and quiet disjoint by construction, so together they are
/// exactly the event-bearing set. The display name resolves against the same window the
/// lit side used, so one place cannot show two names in one scene.
fn quiet_places(d: &dyn SceneSource, lit: &[ScenePlace], window: TimeRange) -> Vec<QuietPlace> {
    let lit_ids: HashSet<&str> = lit.iter().map(|p| p.id.as_str()).collect();
    let mut out: Vec<QuietPlace> = d
        .event_bearing_place_ids()
        .iter()
        .filter(|id| !lit_ids.contains(id.as_str()))
        .filter_map(|id| {
            let place = d.place_by_id(id)?;
            let (existence_from, existence_to) = resolve_existence(d.place_history_for(&place.id));
            Some(QuietPlace {
                id: place.id.clone(),
                display_name: resolve_display_name(&place.name, d.place_history_for(&place.id), Some(window), d.place_name_alias_for(&place.id)),
                lat: place.lat,
                lon: place.lon,
                total_events: d.total_events_for(id),
                existence_from,
                existence_to,
                merged_ids: crate::merge::absorbed_ids_for(&place.id),
            })
        })
        .collect();
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Public so the verse and place endpoints build this shape from here instead of
/// duplicating the grouping. Passes no ref: neither is scoped to one, so nothing to rank.
pub fn to_scene_event(e: &Event) -> SceneEvent {
    SceneEvent { id: e.id.clone(), label: e.label.clone(), when: e.when, verse_groups: verse_groups_for(&e.verses, None) }
}

/// One book's account of an event: the passage it narrates the event in, and any
/// note on the citation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventWitness {
    /// The book's three-letter code, such as `MAT`.
    pub book: String,
    /// The verses of this account, grouped by chapter.
    pub verse_groups: Vec<VerseGroup>,
    /// A note on how THIS account is cited -- not the event's own note about how its
    /// date and grouping were arrived at. Absent when the account needed none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_note: Option<String>,
    /// The section of Robertson's Harmony of the Gospels this account falls in, sent
    /// only where it differs from the event's own. Absent otherwise.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub robertson_section: Option<String>,
}

/// Synthesizes exactly one witness, grouped from the event's own verses, when none was
/// curated. The single function the heading index and the event wire both call, so they
/// cannot disagree about how many witnesses an event has.
pub fn witnesses_for(e: &Event) -> Vec<EventWitness> {
    if !e.witnesses.is_empty() {
        return e
            .witnesses
            .iter()
            .map(|w| {
                let verses = crate::translation::resolve(&w.translations, crate::translation::DEFAULT_TRANSLATION).unwrap_or(&[]);
                EventWitness {
                    book: w.book.clone(),
                    verse_groups: verse_groups_for(verses, None),
                    ref_note: w.ref_note.clone(),
                    robertson_section: w.robertson_section.clone(),
                }
            })
            .collect();
    }

    // First-seen book order, not alphabetical: it matches the event's own verse order.
    let mut by_book: Vec<(String, Vec<String>)> = Vec::new();
    for v in &e.verses {
        let Ok(vid) = VerseId::parse_canonical(v) else { continue };
        let book = vid.book.code().to_string();
        match by_book.iter_mut().find(|(b, _)| *b == book) {
            Some((_, list)) => list.push(v.clone()),
            None => by_book.push((book, vec![v.clone()])),
        }
    }
    by_book
        .into_iter()
        .map(|(book, verses)| EventWitness {
            book,
            verse_groups: verse_groups_for(&verses, None),
            ref_note: None,
            robertson_section: None,
        })
        .collect()
}

/// Ascending within a group, capped at 20 ids, with `count` the true total before the cap.
/// Given `r`, the verses satisfying it take the front of the cap, so a place lit by a ref
/// always shows the verse that put it in the scene; the wire shape is unchanged.
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

/// `legs_in_scene` counts kept events, not arrows: a same-place skip can leave an event
/// with no arrow endpoint, so counting arrows would undercount. Hence the same `(w, r)`
/// filter `build_arrows` was given.
fn legend(d: &dyn SceneSource, w: &TimeRange, r: Option<&ScriptureRef>, arrows: &[SceneArrow]) -> Vec<SceneNarrative> {
    let active: HashSet<&str> = arrows.iter().map(|a| a.narrative.as_str()).collect();
    let mut out: Vec<SceneNarrative> = d
        .narratives()
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
    use crate::data::{AtlasData, Canon, Narrative, Place};
    use crate::time::next_year;
    use proptest::prelude::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn time_scene_lights_only_intersecting() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let ids: Vec<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"gilgal") && ids.contains(&"jericho") && ids.contains(&"ai"));
        assert!(!ids.contains(&"hebron"));
    }

    #[test]
    fn quiet_places_are_every_event_bearing_place_not_lit() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let lit_ids: HashSet<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
        let quiet_ids: HashSet<&str> = s.quiet_places.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(quiet_ids, HashSet::from(["hebron"]));

        assert!(lit_ids.is_disjoint(&quiet_ids), "lit and quiet must never overlap");
        let union: HashSet<&str> = lit_ids.union(&quiet_ids).cloned().collect();
        let expected: HashSet<&str> = d.event_bearing_place_ids().iter().map(|s| s.as_str()).collect();
        assert_eq!(union, expected);
    }

    #[test]
    fn quiet_place_display_name_resolves_like_the_lit_side() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-2500, -2400).unwrap());
        assert!(s.places.iter().all(|p| p.id != "hebron"), "hebron must be quiet, not lit, in this window");
        let hebron = s.quiet_places.iter().find(|p| p.id == "hebron").expect("hebron should be quiet here");
        assert_eq!(hebron.display_name, "Kirjath-arba");
        assert_eq!(hebron.lat, 31.5326);
        assert_eq!(hebron.lon, 35.0998);
        assert_eq!(hebron.total_events, 1);

        let s2 = compose_time_scene(&d, TimeRange::new(-1900, -1800).unwrap());
        let hebron2 = s2.quiet_places.iter().find(|p| p.id == "hebron").expect("hebron should be quiet here too");
        assert_eq!(hebron2.display_name, "Hebron");
    }

    #[test]
    fn existence_bounds_propagate_to_a_lit_scene_place() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-2000, -2000).unwrap());
        let hebron = s.places.iter().find(|p| p.id == "hebron").expect("hebron should be lit at exactly -2000");
        assert_eq!(hebron.existence_from, Some(-4004));
        assert_eq!(hebron.existence_to, None);
    }

    #[test]
    fn existence_bounds_propagate_to_a_quiet_scene_place() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let hebron = s.quiet_places.iter().find(|p| p.id == "hebron").expect("hebron is quiet in this window");
        assert_eq!(hebron.existence_from, Some(-4004));
        assert_eq!(hebron.existence_to, None);
        for p in &s.places {
            if p.id != "hebron" {
                assert_eq!((p.existence_from, p.existence_to), (None, None), "{} has no curated history", p.id);
            }
        }
    }

    #[test]
    fn existence_bounds_propagate_to_a_scripture_mode_mention_place() {
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("GEN.13.18").unwrap());
        let hebron = s.places.iter().find(|p| p.id == "hebron").expect("hebron should be mention-lit");
        assert!(hebron.events[0].id.starts_with("mention-"));
        assert_eq!(hebron.existence_from, Some(-4004));
        assert_eq!(hebron.existence_to, None);
    }

    #[test]
    fn scripture_scene_has_no_quiet_places() {
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("GEN.13.18").unwrap());
        assert!(s.quiet_places.is_empty(), "scripture-mode scenes never gain quiet places");
    }

    #[test]
    fn arrows_skip_same_place_and_chain() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let conquest: Vec<_> = s.arrows.iter().filter(|a| a.narrative == "conquest").collect();
        assert_eq!(conquest.len(), 2);
        assert_eq!((conquest[0].from_place.as_str(), conquest[0].to_place.as_str()), ("gilgal", "jericho"));
        assert_eq!((conquest[1].from_place.as_str(), conquest[1].to_place.as_str()), ("jericho", "ai"));
        assert_eq!(conquest[0].to_place, conquest[1].from_place);
    }

    #[test]
    fn scripture_scene_uses_links_not_dates() {
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("GEN.13.18").unwrap());
        assert_eq!(s.mode, SceneMode::Scripture);
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
        let d = crate::data::demo_fixture();
        let s = compose_scripture_scene(&d, &ScriptureRef::parse("JOS").unwrap());
        assert_eq!(s.mode, SceneMode::Scripture);

        let ids: Vec<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
        assert!(ids.contains(&"gilgal") && ids.contains(&"jericho") && ids.contains(&"ai"));
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
        assert_eq!(jericho.brightness, 2u8.min(5));
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
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 50, verse: 26 }));
        assert!(!ref_contains(&r, &VerseId { book: exo, chapter: 1, verse: 1 }));
    }

    #[test]
    fn ref_contains_chapter_matches_book_and_chapter_only() {
        let gen = crate::canon::resolve_alias("GEN").unwrap();
        let exo = crate::canon::resolve_alias("EXO").unwrap();
        let r = ScriptureRef::Chapter { book: gen, chapter: 13 };
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 13, verse: 1 }));
        assert!(ref_contains(&r, &VerseId { book: gen, chapter: 13, verse: 18 }));
        assert!(!ref_contains(&r, &VerseId { book: gen, chapter: 12, verse: 18 }));
        assert!(!ref_contains(&r, &VerseId { book: gen, chapter: 14, verse: 1 }));
        assert!(!ref_contains(&r, &VerseId { book: exo, chapter: 13, verse: 1 }));
    }

    #[test]
    fn ref_contains_passage_boundaries_inclusive() {
        let jos = crate::canon::resolve_alias("JOS").unwrap();
        let r = ScriptureRef::Passage { book: jos, chapter: 6, from_verse: 2, to_verse: 5 };
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 2 }));
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 5 }));
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 3 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 1 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 6 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 5, verse: 3 }));
    }

    #[test]
    fn ref_contains_verse_matches_exactly() {
        let jos = crate::canon::resolve_alias("JOS").unwrap();
        let r = ScriptureRef::Verse(VerseId { book: jos, chapter: 6, verse: 20 });
        assert!(ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 20 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 6, verse: 21 }));
        assert!(!ref_contains(&r, &VerseId { book: jos, chapter: 7, verse: 20 }));
    }

    #[test]
    fn legend_legs_in_scene_counts_kept_not_arrows_and_excludes_zero_arrow_narratives() {
        let d = crate::data::demo_fixture();
        let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
        let conquest = s.narratives.iter().find(|n| n.id == "conquest").expect("conquest has arrows, so it's in the legend");
        assert_eq!(conquest.legs_in_scene, 4);
        assert_eq!(conquest.name, "The Conquest");
        assert_eq!(conquest.color, "#7C3AED");
        assert!(s.narratives.iter().all(|n| n.id != "patriarchs-demo"));
    }

    #[test]
    fn verse_groups_cap_at_20_sorted_ascending_with_true_count() {
        let mut verses: Vec<String> = (1..=25).map(|v| format!("JOS.10.{v}")).collect();
        verses.reverse();

        let places = vec![Place { id: "many-verses".into(), name: "Many Verses".into(), lat: 0.0, lon: 0.0, verse_links: vec![] }];
        let events = vec![Event {
            id: "big-event".into(),
            label: "Long battle".into(),
            when: TimeRange::new(-1400, -1400).unwrap(),
            places: vec!["many-verses".into()],
            verses,
            ..Default::default()
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
        assert_eq!(g.count, 25);
        assert_eq!(g.verses.len(), 20);
        let expected: Vec<String> = (1..=20).map(|v| format!("JOS.10.{v}")).collect();
        assert_eq!(g.verses, expected);
    }

    #[test]
    fn scripture_mode_verse_cap_prioritizes_ref_matching_verses() {
        let verses_a: Vec<String> = (1..=25).map(|v| format!("JOS.10.{v}")).collect();
        let event_a = Event {
            id: "big-event-a".into(),
            label: "Long battle A".into(),
            when: TimeRange::new(-1400, -1400).unwrap(),
            places: vec!["place-a".into()],
            verses: verses_a,
            ..Default::default()
        };

        let verses_b: Vec<String> = (1..=30).map(|v| format!("JOS.11.{v}")).collect();
        let event_b = Event {
            id: "big-event-b".into(),
            label: "Long battle B".into(),
            when: TimeRange::new(-1300, -1300).unwrap(),
            places: vec!["place-b".into()],
            verses: verses_b,
            ..Default::default()
        };

        let places = vec![
            Place { id: "place-a".into(), name: "Place A".into(), lat: 0.0, lon: 0.0, verse_links: vec![] },
            Place { id: "place-b".into(), name: "Place B".into(), lat: 0.0, lon: 0.0, verse_links: vec![] },
        ];
        let d = AtlasData::new(
            Canon { books: vec![] },
            places,
            vec![event_a, event_b],
            vec![],
            vec![],
            vec![],
            HashMap::new(),
            HashMap::new(),
        )
        .finish();
        let jos = crate::canon::resolve_alias("JOS").unwrap();

        let ref_a = ScriptureRef::Verse(VerseId { book: jos, chapter: 10, verse: 25 });
        let s_a = compose_scripture_scene(&d, &ref_a);
        assert_eq!(s_a.places.len(), 1);
        let place_a = s_a.places.iter().find(|p| p.id == "place-a").expect("place-a lit by JOS.10.25");
        assert_eq!(place_a.events.len(), 1);
        assert_eq!(place_a.events[0].verse_groups.len(), 1);
        let g_a = &place_a.events[0].verse_groups[0];
        assert_eq!(g_a.book, "JOS");
        assert_eq!(g_a.chapter, 10);
        assert_eq!(g_a.count, 25);
        assert_eq!(g_a.verses.len(), 20);
        assert!(
            g_a.verses.contains(&"JOS.10.25".to_string()),
            "the ref-matching verse must survive the cap: {:?}",
            g_a.verses
        );
        let mut sorted_a = g_a.verses.clone();
        sorted_a.sort_by_key(|v| v.rsplit('.').next().unwrap().parse::<u16>().unwrap());
        assert_eq!(g_a.verses, sorted_a, "rendered verses must stay ascending-sorted");

        let ref_b = ScriptureRef::Passage { book: jos, chapter: 11, from_verse: 5, to_verse: 30 };
        let s_b = compose_scripture_scene(&d, &ref_b);
        assert_eq!(s_b.places.len(), 1);
        let place_b = s_b.places.iter().find(|p| p.id == "place-b").expect("place-b lit by JOS.11.5-30");
        let g_b = &place_b.events[0].verse_groups[0];
        assert_eq!(g_b.book, "JOS");
        assert_eq!(g_b.chapter, 11);
        assert_eq!(g_b.count, 30);
        assert_eq!(g_b.verses.len(), 20);
        for v in &g_b.verses {
            let vn: u16 = v.rsplit('.').next().unwrap().parse().unwrap();
            assert!((5..=30).contains(&vn), "every rendered verse must satisfy the ref, got {v}");
        }
    }

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
                ..Default::default()
            }
        }

        let na = vec![
            leg("na-1", "gilgal", -1500),
            leg("na-2", "jericho", -1450),
            leg("na-3", "jericho", -1400),
            leg("na-4", "ai", -1350),
            leg("na-5", "bethel", -1300),
            leg("na-6", "bethel", -1250),
            leg("na-7", "shechem", -1200),
        ];
        let nb = vec![
            leg("nb-1", "dan", -800),
            leg("nb-2", "samaria", -750),
            leg("nb-3", "jerusalem", -700),
            leg("nb-4", "jerusalem", -650),
            leg("nb-5", "beersheba", -600),
            leg("nb-6", "damascus", -550),
        ];
        let nc = vec![
            leg("nc-1", "hebron", -4000),
            leg("nc-2", "shiloh", -3000),
            leg("nc-3", "shiloh", -2000),
            leg("nc-4", "gilgal", -1000),
            leg("nc-5", "jericho", -100),
            leg("nc-6", "jerusalem", -1),
            leg("nc-7", "jerusalem", 1),
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

    fn window_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }

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
                    prop_assert!(place_ids.contains(&a.from_place) && place_ids.contains(&a.to_place));
                    prop_assert_eq!(&a.color, &n.color);
                    prop_assert_ne!(&a.from_place, &a.to_place);
                    let (fe, te) = (d.event_by_id(&a.from_event).unwrap(), d.event_by_id(&a.to_event).unwrap());
                    prop_assert!(te.when.from_year >= fe.when.from_year);
                    let from_sp = s.places.iter().find(|p| p.id == a.from_place).unwrap();
                    prop_assert!(from_sp.events.iter().any(|e| e.id == a.from_event));
                    let to_sp = s.places.iter().find(|p| p.id == a.to_place).unwrap();
                    prop_assert!(to_sp.events.iter().any(|e| e.id == a.to_event));
                }
                for pair in arrows.windows(2) {
                    prop_assert_eq!(&pair[0].to_place, &pair[1].from_place);
                }
                if let (Some(first), Some(last)) = (arrows.first(), arrows.last()) {
                    prop_assert!(!arrows.iter().any(|a| a.to_event == first.from_event));
                    prop_assert!(!arrows.iter().any(|a| a.from_event == last.to_event));
                }
            }
            for p in &s.places {
                prop_assert!(!p.events.is_empty());
                prop_assert_eq!(p.brightness, (p.events.len() as u8).min(5));
                for e in &p.events { prop_assert!(e.when.intersects(&w)); }
            }
        }
        #[test]
        fn window_monotonicity(w in window_strategy()) {
            let d = big_fixture();
            let grow = TimeRange::new(
                if w.from_year == 1 { -1 } else { w.from_year - 1 },
                next_year(w.to_year).min(100).max(w.to_year)).unwrap();
            let (s1, s2) = (compose_time_scene(&d, w), compose_time_scene(&d, grow));
            let ids2: std::collections::HashSet<_> = s2.places.iter().map(|p| p.id.clone()).collect();
            for p in &s1.places { prop_assert!(ids2.contains(&p.id)); }
        }

        #[test]
        fn quiet_place_invariants(w in window_strategy()) {
            let d = big_fixture();
            let s = compose_time_scene(&d, w);
            let lit_ids: std::collections::HashSet<_> = s.places.iter().map(|p| p.id.as_str()).collect();
            let quiet_ids: std::collections::HashSet<_> = s.quiet_places.iter().map(|p| p.id.as_str()).collect();

            prop_assert!(lit_ids.is_disjoint(&quiet_ids));
            let union: std::collections::HashSet<_> = lit_ids.union(&quiet_ids).cloned().collect();
            let expected: std::collections::HashSet<_> = d.event_bearing_place_ids().iter().map(|s| s.as_str()).collect();
            prop_assert_eq!(union, expected);

            for qp in &s.quiet_places {
                let place = d.place_by_id(&qp.id).expect("quiet place id must resolve to a real place");
                prop_assert_eq!(qp.lat, place.lat);
                prop_assert_eq!(qp.lon, place.lon);
                prop_assert_eq!(qp.total_events, d.total_events_for(&qp.id));
                prop_assert!(qp.total_events > 0);
            }

            for qp in &s.quiet_places {
                let place = d.place_by_id(&qp.id).unwrap();
                let expected_name = resolve_display_name(&place.name, d.place_history_for(&place.id), Some(w), d.place_name_alias_for(&place.id));
                prop_assert_eq!(&qp.display_name, &expected_name);
            }
        }
    }
}
