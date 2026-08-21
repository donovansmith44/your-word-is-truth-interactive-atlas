//! Batch HOTFIX-2: same-place dedupe. Two compiled `Place` records can
//! independently geocode to the SAME real-world location -- a duplicate
//! lineage (OpenBible vs Theographic, e.g. `hazor-1`/`hazor_545`) or two
//! independent OpenBible identifications of the same site under different
//! text-forms (e.g. `kedesh-4`/`kedesh-naphtali`, one for JDG.4.6's fully-
//! qualified "Kedeshnaphtali", one for JDG.4.9-11's bare elliptical
//! "Kedesh" back-reference to the same place in the same chapter). Left
//! alone, a scene renders BOTH records as separate markers, and the
//! close-marker screen-space nudge (map.js) then has to visually separate
//! two dots that are really one place. User report 2026-08-20: Judges 4's
//! Hazor/Kedesh-naphtali rendering in the Mediterranean -- see
//! batch-hotfix2-report.md for the full root-cause chain (the OTHER half of
//! that bug, the nudge itself moving a marker far enough to cross the
//! coastline, is map.js's own fix, independent of this module).
//!
//! The fix: merge each CONFIRMED pair into one record before scene
//! composition ever sees them (`AtlasData::finish`, before any scene is
//! built), so the wire -- and therefore the popover, the quiet-place graph,
//! and every other consumer of a place id -- agrees everywhere that they
//! are one node. Applied here, in atlas-core, rather than per-scene in
//! `scene.rs`: a place's identity must be the same fact regardless of which
//! window or scripture ref happens to be asking (`/api/place/{id}`, arrow
//! endpoints, `event_bearing_place_ids` -- QUIET-1's own "cities in our
//! graph" -- all read `AtlasData::places`/`events` directly), so merging
//! once at load time, upstream of every consumer, is the only layer that
//! keeps all of them consistent with each other. Compare: merging only
//! inside `scene::lit_places` would fix the map but leave `/api/place/
//! hazor_545` resolving as its own, still-separate node.
//!
//! CURATED, NOT AUTOMATIC -- this is the one deliberate departure from a
//! literal reading of "two places within 1.0km are one marker": a
//! dataset-wide sweep at this batch's own 1.0km threshold (see the batch
//! report's sweep section) found 4,516 place PAIRS that close together in
//! `data/compiled/places.json` (1,375 places total), and the overwhelming
//! majority are genuinely DISTINCT places that merely share an imprecise
//! upstream geocode -- e.g. `mount-sinai`/`mount-horeb`/`mount-paran`/
//! `wilderness-of-sinai`, four textually and traditionally distinct
//! locations, all landing on the same approximate point because their real
//! position is scholarly-disputed, not because they're the same place. This
//! app's own `map.js` (`setScene`'s doc comment) already documents a
//! load-bearing case of exactly this trap: Shittim and the "plains of Moab"
//! camp sit at the IDENTICAL lat/lon (0km apart) and are "both real,
//! distinct places" that a live test (`world-map.spec.ts`'s WORLD-3)
//! requires to keep rendering as two separate, independently-addressable
//! markers. Distance alone cannot tell these two situations apart -- there
//! is no threshold that both merges `kedesh-4`/`kedesh-naphtali` (0km
//! apart) and spares Shittim/Moab (also 0km apart). So `MERGE_PAIRS` below
//! is a small, explicitly curated, individually-verified table -- the same
//! "hand-curated, easily reversible" pattern every other judgment call in
//! this app's data already uses (`data/curated/*.toml`), just expressed as
//! a Rust table rather than a new curated-file pipeline stage, since two
//! rows don't earn a new ETL parser/validator/compiled-file stage under
//! this batch's own "small, surgical... do not widen scope" instruction.
//! `great_circle_km` still enforces the brief's own <=1.0km ceiling on every
//! curated entry (`debug_assert!` in `apply_place_merges`, and this
//! module's own unit tests) as a safety net against a future curation
//! mistake -- it is a validation bound on curated entries, never the
//! discovery mechanism.
//!
//! KNOWN LIMITATION for a future curator adding a pair here: if `absorbed`
//! ever carries its own `data/curated/place-history.toml` entry, that
//! history becomes unreachable after the merge (nothing in `AtlasData::places`
//! keeps the absorbed id any more) -- migrate any such entry onto `survivor`
//! by hand. Neither of today's two pairs has one (verified against
//! `data/curated/place-history.toml`), so this is a documented caveat, not
//! a bug fixed here.

use crate::data::{Event, Place};

/// Haversine great-circle distance in kilometers. Precise (not the
/// equirectangular approximation map.js's own client-side pixel-collision
/// check uses for a cheap, zoom-scale rendering decision) -- this threshold
/// decides a semantic identity question (are these records the SAME
/// PLACE), not a screen-space layout question, so it earns the extra trig.
pub fn great_circle_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0088;
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dphi = (lat2 - lat1).to_radians();
    let dlambda = (lon2 - lon1).to_radians();
    let a = (dphi / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlambda / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

/// The same-place threshold, per the brief verbatim: covers identical
/// coordinates and the ~136m Hazor pair; a 2km-apart pair must NOT merge.
pub const SAME_PLACE_THRESHOLD_KM: f64 = 1.0;

/// One curated same-place pair: `absorbed` is folded into `survivor` --
/// `absorbed`'s own `verse_links` are unioned onto `survivor` and every
/// event referencing `absorbed` is repointed to `survivor` (`apply_place_merges`);
/// `absorbed` itself is then dropped from `AtlasData::places` entirely.
/// Both pairs below were verified live against `/world?ref=JDG.4`
/// (batch-hotfix2-report.md) before being added here.
pub struct PlaceMerge {
    pub survivor: &'static str,
    pub absorbed: &'static str,
    /// Why `survivor` (not `absorbed`) keeps the id -- for a future reader,
    /// not read by any code.
    pub reason: &'static str,
}

pub const MERGE_PAIRS: &[PlaceMerge] = &[
    PlaceMerge {
        survivor: "hazor-1",
        absorbed: "hazor_545",
        reason: "Same tel (Tel Hazor), ~136m apart (great_circle_km), well inside the 1.0km \
            threshold. hazor-1 is OpenBible-lineage, carries the full 11-verse evidence trail \
            (JOS.11/JOS.12/JOS.19/JDG.4/1SA.12/1KI.9/2KI.15) and is the id data/curated/\
            events-extra.toml's cq_hazor already curates by hand (\"places: brief guessed \
            'hazor', but geo ancient.jsonl collides into 5 entries... Using hazor-1\"). \
            hazor_545 is a Theographic-synthesized duplicate (event theo-138, \"Subjugation by \
            Jabin\") created only because Theographic's own bare place name \"Hazor\" failed to \
            case-insensitively match any of OpenBible's own disambiguated hazor-1..5 names -- \
            it carries zero verse_links of its own.",
    },
    PlaceMerge {
        survivor: "kedesh-4",
        absorbed: "kedesh-naphtali",
        reason: "Both OpenBible-lineage, identical geocode (32.735, 35.55555 -- 0.0km apart): \
            two independent identification records for the same real site. OpenBible geocodes \
            JDG.4.6's fully-qualified \"Kedeshnaphtali\" text-form separately from JDG.4.9-11's \
            bare, elliptical \"Kedesh\" back-references, even though both plainly narrate the \
            same place in the same chapter of the same narrative. Neither id is curated or \
            event-referenced (unlike the Hazor pair), so the choice of which keeps the id is a \
            low-stakes tie-break: kedesh-4 carries more of the chapter's own verse evidence (3 \
            links vs 1) and keeps it, with kedesh-naphtali's own verse_links unioned on.",
    },
];

/// Ids absorbed into `survivor_id`, if any -- for `scene::lit_places`/
/// `scene::quiet_places`/`compose_scripture_scene`'s mention branch to stamp
/// onto `ScenePlace::merged_ids`/`QuietPlace::merged_ids` (wire
/// traceability, per the brief: "note the absorbed record's id in the wire
/// for traceability"). `MERGE_PAIRS` is small enough (today: 2 entries) that
/// a linear scan per place costs nothing worth indexing.
pub fn absorbed_ids_for(survivor_id: &str) -> Vec<String> {
    MERGE_PAIRS.iter().filter(|m| m.survivor == survivor_id).map(|m| m.absorbed.to_string()).collect()
}

/// Applies `MERGE_PAIRS` to `places`/`events` in place -- called once by
/// `AtlasData::finish()` (idempotent: a pair whose `absorbed` id is no
/// longer present, e.g. `finish()` running a second time on already-merged
/// data, is silently skipped rather than erroring, matching `finish()`'s
/// own documented idempotence contract). For each pair still present:
/// - `absorbed`'s own `verse_links` are unioned onto `survivor` (survivor's
///   own links first, then any absorbed-only links, deduped) -- the
///   "union of both records' verse groups" half of the brief's requirement.
/// - every `Event.places` entry equal to `absorbed` is rewritten to
///   `survivor` IN PLACE (same list position -- `Event`'s own doc comment:
///   "`places[0]` is the anchor place used for arrow endpoints," so a
///   remove-then-append would silently change an event's anchor if
///   `absorbed` ever happened to be `places[0]`; today's two curated pairs
///   don't hit that case, but nothing here should assume that stays true),
///   then deduped keeping the FIRST occurrence (an event that already,
///   separately, touched both ids under their old ids would otherwise list
///   `survivor` twice). This alone gives `scene::lit_places`' own
///   `HashMap<place_id, Vec<&Event>>` grouping the "union of both records'
///   events" half of the brief's requirement, with no scene.rs change
///   needed for the union itself.
/// - `absorbed` is removed from `places` entirely.
pub fn apply_place_merges(places: &mut Vec<Place>, events: &mut [Event]) {
    for pair in MERGE_PAIRS {
        let Some(absorbed_idx) = places.iter().position(|p| p.id == pair.absorbed) else {
            continue; // already merged (finish() re-run), or curation drifted -- no-op, not a panic
        };
        let Some(survivor_idx) = places.iter().position(|p| p.id == pair.survivor) else {
            continue; // defensive: a curated survivor id that stopped existing -- never worth a runtime panic
        };
        debug_assert!(
            great_circle_km(
                places[survivor_idx].lat,
                places[survivor_idx].lon,
                places[absorbed_idx].lat,
                places[absorbed_idx].lon
            ) <= SAME_PLACE_THRESHOLD_KM,
            "curated merge pair {}/{} exceeds the {}km same-place threshold",
            pair.survivor,
            pair.absorbed,
            SAME_PLACE_THRESHOLD_KM
        );

        let absorbed_links = places[absorbed_idx].verse_links.clone();
        for v in absorbed_links {
            if !places[survivor_idx].verse_links.contains(&v) {
                places[survivor_idx].verse_links.push(v);
            }
        }
        places.remove(absorbed_idx);

        for e in events.iter_mut() {
            let mut changed = false;
            for p in e.places.iter_mut() {
                if p == pair.absorbed {
                    *p = pair.survivor.to_string();
                    changed = true;
                }
            }
            if changed {
                let mut seen = std::collections::HashSet::new();
                e.places.retain(|p| seen.insert(p.clone()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::TimeRange;
    use proptest::prelude::*;

    fn place(id: &str, lat: f64, lon: f64, verse_links: &[&str]) -> Place {
        Place { id: id.into(), name: id.into(), lat, lon, verse_links: verse_links.iter().map(|s| s.to_string()).collect() }
    }

    fn event(id: &str, places: &[&str]) -> Event {
        Event {
            id: id.into(),
            label: id.into(),
            when: TimeRange::new(1, 1).unwrap(),
            places: places.iter().map(|s| s.to_string()).collect(),
            verses: vec![],
        }
    }

    // --- great_circle_km / SAME_PLACE_THRESHOLD_KM (brief's own three cases,
    // verbatim: "identical coords merge; 100m merges; 2km does NOT") -------

    #[test]
    fn identical_coordinates_are_zero_km_and_merge() {
        let d = great_circle_km(32.735, 35.55555, 32.735, 35.55555);
        assert_eq!(d, 0.0);
        assert!(d <= SAME_PLACE_THRESHOLD_KM);
    }

    #[test]
    fn a_100m_pair_is_inside_the_threshold() {
        // hazor-1 / hazor_545's own real coordinates (data/compiled/places.json).
        let d = great_circle_km(33.018333, 35.569167, 33.01746212803129, 35.56813718);
        assert!(d < 0.2, "expected roughly 100-150m, got {d}km");
        assert!(d <= SAME_PLACE_THRESHOLD_KM);
    }

    #[test]
    fn a_2km_pair_does_not_merge() {
        // ~0.018 degrees latitude ≈ 2km.
        let d = great_circle_km(32.735, 35.55555, 32.753, 35.55555);
        assert!(d >= 1.9 && d <= 2.1, "expected roughly 2km, got {d}km");
        assert!(d > SAME_PLACE_THRESHOLD_KM);
    }

    #[test]
    fn known_distinct_pair_shittim_moab_stays_above_merge_intent() {
        // Not a threshold check (Shittim/Moab-2 are 0km apart, same as the
        // Kedesh pair, by raw distance alone -- see this module's own header
        // comment) -- this pins that they are simply ABSENT from MERGE_PAIRS,
        // the actual mechanism that keeps them from merging.
        assert!(MERGE_PAIRS.iter().all(|m| m.survivor != "shittim" && m.absorbed != "shittim"));
        assert!(MERGE_PAIRS.iter().all(|m| m.survivor != "moab-2" && m.absorbed != "moab-2"));
    }

    proptest! {
        // Property mirror of the three example cases above, at arbitrary
        // headings: a pair placed EXACTLY at the threshold boundary (in km,
        // converted to a latitude delta -- a locally-flat approximation,
        // accurate to well under 1m of threshold-boundary error at this
        // scale, more than tight enough to stay on the correct side of
        // strict `<=`/`>` in every generated case) is inside; a pair placed
        // any positive epsilon beyond it is outside.
        #[test]
        fn threshold_boundary_property(lat in -60.0f64..60.0, lon in -170.0f64..170.0, epsilon_km in 0.001f64..5.0) {
            let km_per_deg_lat = 111.32;
            let inside_lat = lat + (SAME_PLACE_THRESHOLD_KM * 0.999) / km_per_deg_lat;
            let outside_lat = lat + (SAME_PLACE_THRESHOLD_KM + epsilon_km) / km_per_deg_lat;

            prop_assert!(great_circle_km(lat, lon, inside_lat, lon) <= SAME_PLACE_THRESHOLD_KM);
            prop_assert!(great_circle_km(lat, lon, outside_lat, lon) > SAME_PLACE_THRESHOLD_KM);
        }
    }

    // --- apply_place_merges -------------------------------------------------

    #[test]
    fn merges_verse_links_dedupes_and_drops_absorbed_place() {
        let mut places = vec![
            place("hazor-1", 33.018333, 35.569167, &["JOS.11.1", "JDG.4.2"]),
            place("hazor_545", 33.01746212803129, 35.56813718, &["JDG.4.2", "1SA.12.9"]),
            place("canaan", 31.0, 35.0, &[]),
        ];
        let mut events: Vec<Event> = vec![];

        apply_place_merges(&mut places, &mut events);

        assert_eq!(places.len(), 2, "hazor_545 must be removed entirely");
        assert!(places.iter().all(|p| p.id != "hazor_545"));
        let hazor = places.iter().find(|p| p.id == "hazor-1").unwrap();
        // union, deduped (JDG.4.2 appeared on both), survivor's own links first.
        assert_eq!(hazor.verse_links, vec!["JOS.11.1", "JDG.4.2", "1SA.12.9"]);
    }

    #[test]
    fn rewrites_event_places_to_survivor_and_dedupes_within_one_event() {
        let mut places = vec![place("hazor-1", 33.018333, 35.569167, &[]), place("hazor_545", 33.01746212803129, 35.56813718, &[])];
        let mut events = vec![
            event("theo-138", &["canaan", "hazor_545"]),
            // an event that (hypothetically) already touched BOTH ids under
            // their old ids -- must collapse to survivor ONCE, not twice.
            event("both-old-ids", &["hazor-1", "hazor_545"]),
        ];

        apply_place_merges(&mut places, &mut events);

        let theo138 = events.iter().find(|e| e.id == "theo-138").unwrap();
        assert_eq!(theo138.places, vec!["canaan".to_string(), "hazor-1".to_string()]);

        let both = events.iter().find(|e| e.id == "both-old-ids").unwrap();
        assert_eq!(both.places, vec!["hazor-1".to_string()], "must collapse to one entry, not two");
    }

    #[test]
    fn preserves_anchor_position_when_absorbed_was_places_zero() {
        // Event doc comment: places[0] is the anchor used for arrow
        // endpoints -- renaming absorbed -> survivor in place must never
        // silently move the anchor to a DIFFERENT place.
        let mut places = vec![place("hazor-1", 33.018333, 35.569167, &[]), place("hazor_545", 33.01746212803129, 35.56813718, &[])];
        let mut events = vec![event("e1", &["hazor_545", "canaan"])];

        apply_place_merges(&mut places, &mut events);

        assert_eq!(events[0].places, vec!["hazor-1".to_string(), "canaan".to_string()], "hazor-1 must land at position 0, the OLD anchor's own slot");
    }

    #[test]
    fn is_idempotent_when_run_twice() {
        let mut places = vec![place("hazor-1", 33.018333, 35.569167, &[]), place("hazor_545", 33.01746212803129, 35.56813718, &[])];
        let mut events = vec![event("theo-138", &["hazor_545"])];

        apply_place_merges(&mut places, &mut events);
        let after_first = places.clone();
        apply_place_merges(&mut places, &mut events); // must no-op, not panic or double-apply

        assert_eq!(places, after_first);
        assert_eq!(events[0].places, vec!["hazor-1".to_string()]);
    }

    #[test]
    fn absorbed_ids_for_reports_the_curated_pairs() {
        assert_eq!(absorbed_ids_for("hazor-1"), vec!["hazor_545".to_string()]);
        assert_eq!(absorbed_ids_for("kedesh-4"), vec!["kedesh-naphtali".to_string()]);
        assert!(absorbed_ids_for("canaan").is_empty());
        assert!(absorbed_ids_for("hazor_545").is_empty(), "the ABSORBED id itself was never a survivor");
    }

    #[test]
    fn every_curated_pair_is_within_the_same_place_threshold() {
        // Guards the curated table itself (not just apply_place_merges'
        // debug_assert, which only fires in debug builds) against a future
        // hand-edit that widens a pair past the threshold by mistake. Real
        // coordinates, matching data/compiled/places.json.
        let known_coords: &[(&str, f64, f64)] = &[
            ("hazor-1", 33.018333, 35.569167),
            ("hazor_545", 33.01746212803129, 35.56813718),
            ("kedesh-4", 32.735, 35.55555),
            ("kedesh-naphtali", 32.735, 35.55555),
        ];
        for pair in MERGE_PAIRS {
            let survivor = known_coords.iter().find(|(id, _, _)| *id == pair.survivor).expect("test table missing survivor coords");
            let absorbed = known_coords.iter().find(|(id, _, _)| *id == pair.absorbed).expect("test table missing absorbed coords");
            let d = great_circle_km(survivor.1, survivor.2, absorbed.1, absorbed.2);
            assert!(d <= SAME_PLACE_THRESHOLD_KM, "{}/{} is {d}km apart, over the {}km threshold", pair.survivor, pair.absorbed, SAME_PLACE_THRESHOLD_KM);
        }
    }
}
