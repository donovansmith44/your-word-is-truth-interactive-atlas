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
//! is a small, explicitly curated, individually-verified table -- hand-
//! curated and trivially git-revertible in the same SPIRIT as every other
//! judgment call in this app's data (`data/curated/*.toml`), though NOT the
//! same edit ceremony (a `.toml` change needs no Rust knowledge/recompile;
//! this does -- fix-round-1, review finding M-1) -- expressed as a Rust
//! table rather than a new curated-file pipeline stage, since two rows
//! don't earn a new ETL parser/validator/compiled-file stage under this
//! batch's own "small, surgical... do not widen scope" instruction.
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
//! by hand. None of the 17 pairs below (the original 2, plus Batch PLACE-1a's
//! 15) has one (verified against `data/curated/place-history.toml`), so this
//! is a documented caveat, not a bug fixed here.

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
    // Batch PLACE-1a (2026-08-27, place-alias-investigation.md's own §3 systematic sweep --
    // the 20 confirmed Theographic `_NNN`-suffixed dual-lineage duplicates, mechanically
    // re-derived against data/exports/gazetteer.json at this module's own 1.0km threshold: 39
    // total Theographic-synthesized ids, 20 of which sit within 1.0km of an unrelated
    // OpenBible-lineage sibling; the other 19 have no sibling at all, so they are not a merge
    // question here). Each of the 15 pairs below was individually hand-verified against raw
    // theographic-bible-metadata-master/CSV/{Places,Events}.csv, data/curated/events-extra.toml,
    // and the compiled gazetteer's own verse-attestation lists before being added -- per this
    // module's own "distance alone cannot tell identity from coincidence" warning above, 5 of
    // the 20 candidates were judged NOT true duplicates on exactly that basis and are
    // deliberately EXCLUDED (documented in batch-place1a-report.md, not silently dropped):
    // `judea_657` (Theographic's own region record, landing in the ~95-member Jerusalem
    // mega-cluster -- a region/city coincidence, the OTHER half of the owner's ask, out of this
    // batch's scope); `jericho_634` (its own raw Theographic events -- Bartimaeus healed
    // "leaving Jericho" MAT.20.29/MRK.10.46/LUK.18.35, Zacchaeus LUK.19.1 -- are unambiguously
    // NT/Herodian-era, matching curated jericho-2's own verse set exactly, but jericho_634 sits
    // 197m from jericho-1 -- the OT-era site data/curated/events-extra.toml curates separately --
    // and 2.1km from jericho-2, past this module's own 1.0km ceiling; merging into the
    // geographically-nearer-but-wrong-era jericho-1 would misattribute NT events onto the OT
    // marker, exactly the class of error this module's own header warns proximity cannot
    // resolve, so it is left unmerged, a genuine gap for PLACE-1b, not this batch's mechanism);
    // `bethany_186` (its nearest OpenBible neighbor, en-shemesh, is JOS.15.7's unrelated OT
    // boundary spring -- no shared name-root, no historical rename, plain geocoding-precision
    // noise); `moab_815` (Theographic's own "Moab" region record -- its real OpenBible-lineage
    // twins moab-1/moab-2 are 36km/39km away, nowhere near this threshold; its only within-1.0km
    // neighbor, dibon-1, is a specific Moabite CITY, not a name-form alias of "Moab" the nation);
    // `galilee_433` (Nazareth is a city IN Galilee, not another name for Galilee itself -- the
    // same region/city shape as Judea and Chaldea, not a true alias).
    PlaceMerge {
        survivor: "ur-1",
        absorbed: "ur_1189",
        reason: "The charter case (place-alias-investigation.md §1): ~136m apart, well inside \
            the 1.0km threshold. ur-1 is OpenBible-lineage, carries the GEN.11.28/GEN.11.31/ \
            GEN.15.7/NEH.9.7 verse evidence and is the id data/curated/events-extra.toml's ab_ur \
            already curates by hand. ur_1189 (\"Ur of the Chaldees\") is a Theographic-synthesized \
            duplicate -- Theographic's own event theo-65 (\"Birth of Abraham\", GEN.11.26-27) -- \
            created only because Theographic's own name string never case-insensitively matched \
            OpenBible's disambiguated \"Ur 1\"/\"Ur 2\"; it carries zero verse_links of its own. \
            NOT the same fact as `chaldea` (a genuinely distinct, 84-verse-attested region \
            record at the same point) -- chaldea is not a candidate for either table.",
    },
    PlaceMerge {
        survivor: "rehoboth-ir",
        absorbed: "rehoboth-ir_992",
        reason: "~8m apart. rehoboth-ir is OpenBible-lineage, carries GEN.10.11 (\"and the city \
            Rehoboth\" -- the KJV's own bare short-form). rehoboth-ir_992's Theographic kjvName \
            is the same bare \"Rehoboth\", which never matched OpenBible's fully-qualified \
            \"Rehoboth-Ir\" slug; carries zero verse_links of its own. Theographic's own event 87 \
            (GEN.10.8-12, the Nimrod/Shinar city list) lists rehoboth-ir_992 alongside calneh_269 \
            -- both this batch's pairs -- as two of several distinct cities at one low-precision \
            Mesopotamian point; assyria/nimrod/nineveh sit at the identical point too but are \
            genuinely distinct entities, not touched.",
    },
    PlaceMerge {
        survivor: "moreh-1",
        absorbed: "moreh_817",
        reason: "~16m apart. moreh-1 is OpenBible-lineage, carries GEN.12.6/DEU.11.30 (\"the \
            plain of Moreh\"). moreh_817's Theographic kjvName is the bare \"Moreh\", never \
            matching OpenBible's disambiguated \"Moreh 1\"; carries zero verse_links of its own. \
            Theographic's own event 71 (Abram's journey into Canaan, GEN.12.1-9) lists moreh_817 \
            alongside canaan_272/haran_527/shechem_1069/ai_36/bethel_202/negeb_885 as distinct \
            waypoints -- ai_36 is this batch's own pair too; shechem and the other Shechem-area \
            features at the same point are genuinely distinct, not touched.",
    },
    PlaceMerge {
        survivor: "bethlehem-1",
        absorbed: "bethlehem_218",
        reason: "~17m apart. bethlehem-1 is OpenBible-lineage, carries the full 47-verse \
            evidence trail (RUT/1SA/2SA/1CH/MIC/MAT/LUK/JHN). bethlehem_218's Theographic \
            kjvName is the bare \"Bethlehem\", never matching OpenBible's disambiguated \
            \"Bethlehem 1\"; carries zero verse_links of its own. Theographic's own events \
            254/256/258 (Luke's nativity, the magi, the massacre of the innocents) all target \
            bethlehem_218 -- merging repoints all three onto bethlehem-1, consolidating what \
            would otherwise be a second, unlabeled marker under the Christmas narrative's own \
            scene, exactly the owner's reported symptom.",
    },
    PlaceMerge {
        survivor: "bethsaida-2",
        absorbed: "bethsaida_231",
        reason: "~28m apart. bethsaida-1 and bethsaida-2 are both OpenBible-lineage disambiguation \
            twins at identical coordinates (32.9104, 35.6310 -- a geo.rs kebab-collision split, \
            same shape as ur-1/ur-2, itself out of this batch's scope). bethsaida_231's own \
            Theographic events are 393 (feeding of the five thousand, MAT.14.13) and 403 (the \
            blind man healed, MRK.8.22) -- bethsaida-2 carries MRK.8.22 and LUK.9.10 (the Luke \
            parallel of the same feeding), while bethsaida-1 carries only unrelated verses \
            (JHN.1.44/JHN.12.21/LUK.10.13/MAT.11.21). This matches data/curated/events-extra.toml's \
            own jm_bethsaida entry, which already curates \"Using bethsaida-2\" by hand for the \
            identical reason -- bethsaida_231 folds onto the SAME survivor the curated event \
            already picked, not a fresh tie-break.",
    },
    PlaceMerge {
        survivor: "aphek-2",
        absorbed: "antipatris_70",
        reason: "~42m apart. aphek-2 is OpenBible-lineage and already carries ACT.23.31 (the \
            Antipatris verse) alongside JOS.12.18/1SA.4.1/1SA.29.1 -- OpenBible's own \
            ancient.jsonl already identifies Herod's Antipatris with the OT battle site of Aphek \
            (Ras al-'Ain), the standard historical-geography identification. antipatris_70's \
            Theographic kjvName \"Antipatris\" never case-insensitively matched OpenBible's \
            \"Aphek 2\"; carries zero verse_links of its own despite ACT.23.31 literally naming \
            it -- the same synthesize-a-duplicate-then-render-empty defect as ur_1189.",
    },
    PlaceMerge {
        survivor: "ai-1",
        absorbed: "ai_36",
        reason: "~53m apart. ai-1 is OpenBible-lineage, carries the full Joshua 7-8 Achan/ \
            conquest evidence trail (29 verses) and is the id data/curated/events-extra.toml's \
            cq_ai already curates by hand (\"ai-1 and ai-3 share coords... ai-1's verse_links \
            carry the whole cited range... Using ai-1\", explicitly distinguishing it from ai-2, \
            a genuinely distinct Moabite site tied only to JER.49.3, far away and NOT a merge \
            candidate). ai_36's Theographic kjvName is the bare \"Ai\", never matching OpenBible's \
            disambiguated \"Ai 1\"; Theographic's own event 71 (GEN.12.8, Abram pitching his tent \
            \"having Bethel on the west, and Hai on the east\") confirms it is the SAME Ai as \
            ai-1's own conquest-narrative site, not the distinct Jeremiah one.",
    },
    PlaceMerge {
        survivor: "solomon-s-portico",
        absorbed: "solomons_portico_1109",
        reason: "~113m apart (both inside the Jerusalem/Temple-mount low-precision cluster). \
            solomon-s-portico is OpenBible-lineage, carries JHN.10.23/ACT.3.11/ACT.5.12 -- the \
            IDENTICAL name, \"Solomon's Portico\", just apostrophe-encoded differently in the two \
            source pipelines (kebab \"solomon-s-portico\" vs Theographic's underscore-joined \
            \"solomons_portico_1109\"). Pure encoding-twin, the clearest true-alias shape of the \
            batch -- not a name-FORM difference at all, the same string. solomons_portico_1109 \
            carries zero verse_links of its own despite Theographic's own events 311/316 citing \
            ACT.3.11-17 and ACT.5.12-17 for it.",
    },
    PlaceMerge {
        survivor: "calneh-1",
        absorbed: "calneh_269",
        reason: "~146m apart. calneh-1 is OpenBible-lineage, carries GEN.10.10 (the Shinar-city \
            list). calneh_269's Theographic name is the identical \"Calneh\"; it never matched \
            OpenBible's disambiguated \"Calneh 1\" and carries zero verse_links of its own. \
            NOT the same fact as the closer-by-raw-distance `chebar` (84m, a Babylonian-exile \
            canal in Ezekiel, an entirely different referent that merely shares the same \
            low-precision Mesopotamian default point) -- chebar is not a candidate for either \
            table.",
    },
    PlaceMerge {
        survivor: "ephraim-2",
        absorbed: "ephraim_401",
        reason: "~130m apart. ephraim-1 and ephraim-2 are OpenBible-lineage disambiguation twins \
            at identical coordinates. ephraim_401's own Theographic event (435, \"retreat to \
            Ephraim\", JHN.11.54-57) verse-matches ephraim-2 exactly (JHN.11.54) -- ephraim-1 \
            carries the unrelated 2SA.13.23 (Absalom's sheep-shearing feast \"beside Ephraim\") \
            instead. Same verse-level tie-break discipline as the Bethsaida pair above, not raw \
            attestation count (both twins carry exactly one verse each).",
    },
    PlaceMerge {
        survivor: "mount-of-olives",
        absorbed: "olivet_907",
        reason: "~144m apart. mount-of-olives is OpenBible-lineage, carries 14 verses. \"Olivet\" \
            is the KJV's own alternate name for the same mountain (ACT.1.12, \"the mount called \
            Olivet\") -- a pure name-form alias, not a different site. olivet_907's Theographic \
            events (304/305, ACT.1.4-11, the Ascension; 446/447, MRK.11.11-24) carry zero \
            verse_links of their own on the record despite naming the identical event Acts 1 \
            itself narrates at the same mountain.",
    },
    PlaceMerge {
        survivor: "antioch-2",
        absorbed: "antioch_69",
        reason: "~198m apart. antioch-2 is OpenBible-lineage (38.3061, 31.1892, Pisidian Antioch \
            in Asia Minor) and is the id data/curated/events-extra.toml's p1_pisidian_antioch \
            already curates by hand (\"confirmed correct\"). antioch_69's Theographic name \
            \"Antioch (Pisidia)\" never matched OpenBible's bare \"Antioch 2\"; carries zero \
            verse_links of its own despite Theographic's own events 338/340/343 (ACT.13-14, \
            Paul's sermon and the Pisidian mission) targeting it. `pisidia` (the region) sits at \
            the identical point but is a genuinely distinct entity, not touched.",
    },
    PlaceMerge {
        survivor: "forum-of-appius",
        absorbed: "appii_forum_427",
        reason: "~363m apart. forum-of-appius is OpenBible-lineage, carries ACT.28.15. \
            appii_forum_427 is Theographic's own translation-order rendering of the identical \
            Latin place-name (Forum Appii / Appii Forum) on Paul's own road from Puteoli to \
            Rome; it never case-insensitively matched OpenBible's word-order and carries zero \
            verse_links of its own despite Theographic's own event 384 citing the same ACT.28.11-16.",
    },
    PlaceMerge {
        survivor: "lod",
        absorbed: "lydda_741",
        reason: "~662m apart, still well inside the 1.0km threshold. lod is OpenBible-lineage \
            and already carries ACT.9.32/ACT.9.35/ACT.9.38 (Peter healing Aeneas at Lydda) \
            alongside its OT references (1CH.8.12/EZR.2.33/NEH.7.37/NEH.11.35) -- \"Lydda\" is \
            simply the Greek/NT name for the same OT town \"Lod\", and OpenBible's own \
            ancient.jsonl already folds both eras' verses onto one record. lydda_741's \
            Theographic name \"Lydda\" never matched OpenBible's OT-era \"Lod\" slug; carries \
            zero verse_links of its own despite Theographic's own event 328 citing the identical \
            ACT.9.32-38. NOT the same fact as the closer-by-raw-distance `ge-harashim` (also \
            nearby, 1CH.4.14's unrelated \"valley of craftsmen\") -- not a candidate for either \
            table.",
    },
    PlaceMerge {
        survivor: "phoenix",
        absorbed: "phenice_945",
        reason: "~806m apart, still well inside the 1.0km threshold. phoenix is OpenBible-lineage \
            and carries ACT.27.12 (\"the haven of Crete... which lieth toward the south west and \
            north west\"). \"Phenice\" is simply the KJV's own transliteration of the identical \
            Cretan harbor's Greek name (rendered \"Phoenix\" in modern usage) -- phenice_945's own \
            Theographic events (380/381, ACT.27.1-12, Paul's storm-driven voyage) verse-match \
            phoenix exactly; it carries zero verse_links of its own. NOT the same record as the \
            separate Theographic `phenicia_944` (\"Phenicia\", the mainland region, ACT.11.19/ \
            15.3/21.2) -- that id has no OpenBible-lineage sibling within threshold at all and is \
            therefore not part of this batch.",
    },
];

/// Ids absorbed into `survivor_id`, if any -- for `scene::lit_places`/
/// `scene::quiet_places`/`compose_scripture_scene`'s mention branch to stamp
/// onto `ScenePlace::merged_ids`/`QuietPlace::merged_ids` (wire
/// traceability, per the brief: "note the absorbed record's id in the wire
/// for traceability"). `MERGE_PAIRS` is small enough (17 entries as of Batch
/// PLACE-1a) that a linear scan per place costs nothing worth indexing.
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
            ..Default::default()
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
        // headings: a pair placed just inside the threshold (99.9% of it,
        // converted to a latitude delta) is inside; a pair placed any
        // positive epsilon beyond it is outside.
        //
        // MERGE-BOUNDARY-1 (batch NODE-1, diagnosed from the checked-in
        // proptest-regressions/merge.txt seed `lat=0.0, lon=0.0,
        // epsilon_km=0.001`): the property as originally stated used
        // `km_per_deg_lat = 111.32` -- a WGS84-ellipsoid-flavored constant
        // -- while `great_circle_km`'s own sphere (EARTH_RADIUS_KM =
        // 6371.0088) has exactly R*pi/180 = 111.19508 km per degree of
        // latitude. That is a systematic 0.112% overstatement of the
        // degrees-per-km conversion (~1.12m of shortfall per km), NOT the
        // "well under 1m" the original comment claimed -- so for any
        // epsilon_km < ~0.00112 the "outside" pair actually landed INSIDE
        // the 1.0km threshold (at the seed: distance 0.9998767km <= 1.0)
        // and the property failed. This was the PROPERTY stated too
        // tightly, not a bug in the threshold logic: `great_circle_km` /
        // `SAME_PLACE_THRESHOLD_KM` are compared directly in production
        // (`apply_place_merges`' debug_assert and the unit tests above);
        // 111.32 appeared nowhere outside this test (verified by grep).
        // The fix uses the sphere's OWN exact conversion, under which a
        // pure-meridian haversine reduces to exactly R * delta_phi, so
        // both assertions hold with kilometer-scale margin (0.999km and
        // 1.0+epsilon km against a 1.0km threshold), no approximation
        // slack needed. The committed seed re-runs this exact case first,
        // as the regression witness.
        #[test]
        fn threshold_boundary_property(lat in -60.0f64..60.0, lon in -170.0f64..170.0, epsilon_km in 0.001f64..5.0) {
            // `great_circle_km`'s own EARTH_RADIUS_KM, converted: km per
            // degree of latitude on THAT sphere (R * pi / 180).
            let km_per_deg_lat = 6371.0088_f64 * std::f64::consts::PI / 180.0;
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
    fn absorbed_ids_for_reports_the_place1a_charter_case() {
        // The owner's own repro (place-alias-investigation.md §1): ur_1189 ("Ur of the
        // Chaldees") must fold onto ur-1, not remain a second, independently-rendering record.
        assert_eq!(absorbed_ids_for("ur-1"), vec!["ur_1189".to_string()]);
        // chaldea is NOT a merge candidate -- a genuinely distinct, separately-attested region,
        // not a duplicate-lineage record of Ur (place-alias-investigation.md §2.2).
        assert!(absorbed_ids_for("chaldea").is_empty());
        assert!(absorbed_ids_for("ur_1189").is_empty(), "the ABSORBED id itself was never a survivor");
    }

    #[test]
    fn place1a_excluded_candidates_never_entered_the_table() {
        // The 5 of 20 mechanically-flagged candidates hand-judged NOT true duplicates
        // (batch-place1a-report.md's own triage table) -- pinned here so a future curator
        // cannot silently re-introduce one of these on distance alone. Judea/Galilee are
        // region/city coincidences (the OTHER half of the owner's ask, a separate batch);
        // Bethany is unrelated proximity noise; Moab has no true within-threshold twin at all;
        // Jericho is a real duplicate but of the WRONG survivor by raw distance (its true
        // semantic partner, jericho-2, sits past the 1.0km ceiling) -- see merge_pairs' own
        // Batch PLACE-1a comment for the full reasoning on each.
        for excluded in ["judea_657", "jericho_634", "bethany_186", "moab_815", "galilee_433"] {
            assert!(
                MERGE_PAIRS.iter().all(|m| m.survivor != excluded && m.absorbed != excluded),
                "{excluded} was judged NOT a true duplicate and must stay out of MERGE_PAIRS"
            );
        }
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
            // Batch PLACE-1a (data/exports/gazetteer.json, verified at dispatch time).
            ("ur-1", 30.962222, 46.104444),
            ("ur_1189", 30.9625, 46.103056),
            ("rehoboth-ir", 36.3594, 43.1528),
            ("rehoboth-ir_992", 36.35941, 43.152887),
            ("moreh-1", 32.213611, 35.281944),
            ("moreh_817", 32.21369123124062, 35.2817986718367),
            ("bethlehem-1", 31.704306, 35.207639),
            ("bethlehem_218", 31.70431, 35.20746),
            ("bethsaida-2", 32.91038, 35.63096),
            ("bethsaida_231", 32.91031, 35.63067),
            ("aphek-2", 32.105, 34.930417),
            ("antipatris_70", 32.10478, 34.93078),
            ("ai-1", 31.916944, 35.261111),
            ("ai_36", 31.9166759, 35.2615767),
            ("solomon-s-portico", 31.776667, 35.234167),
            ("solomons_portico_1109", 31.777444, 35.234935),
            ("calneh-1", 32.126111, 45.230833),
            ("calneh_269", 32.127213, 45.229995),
            ("ephraim-2", 31.954444, 35.300278),
            ("ephraim_401", 31.95378956, 35.29913578),
            ("mount-of-olives", 31.777946, 35.245686),
            ("olivet_907", 31.77809503, 35.24719761),
            ("antioch-2", 38.306111, 31.189167),
            ("antioch_69", 38.30466, 31.19049),
            ("forum-of-appius", 41.46639, 12.9975),
            ("appii_forum_427", 41.46801, 12.99372),
            ("lod", 31.95, 34.9),
            ("lydda_741", 31.950908, 34.893068),
            ("phoenix", 35.2, 24.072222),
            ("phenice_945", 35.19888577, 24.08098202),
        ];
        for pair in MERGE_PAIRS {
            let survivor = known_coords.iter().find(|(id, _, _)| *id == pair.survivor).expect("test table missing survivor coords");
            let absorbed = known_coords.iter().find(|(id, _, _)| *id == pair.absorbed).expect("test table missing absorbed coords");
            let d = great_circle_km(survivor.1, survivor.2, absorbed.1, absorbed.2);
            assert!(d <= SAME_PLACE_THRESHOLD_KM, "{}/{} is {d}km apart, over the {}km threshold", pair.survivor, pair.absorbed, SAME_PLACE_THRESHOLD_KM);
        }
    }
}
