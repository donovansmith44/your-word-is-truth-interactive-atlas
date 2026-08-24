//! M-C2: the reusable COMPUTE half of `main.rs`'s own pipeline (parse
//! `data/raw/` + `data/curated/`, merge, hard-validate), extracted into a
//! library function so every consumer that needs a fully-populated,
//! validated `AtlasData` FROM RAW+CURATED SOURCES -- not from the five
//! retiring compiled JSON files this batch deletes -- calls the SAME code
//! `main.rs` always has, rather than a second, independently-authored copy
//! that could drift from it. `main.rs` itself is the first caller: it now
//! calls [`compile`], then writes only the SURVIVING `data/compiled/*.json`
//! files (reading every value straight off the returned `AtlasData`/
//! `Report`, never a second local copy) and the report.
//!
//! THE OTHER CALLERS (why this extraction exists at all): `atlas-graph`'s
//! own `bins/compile_graph.rs` (the one-time artifact-compile step) and
//! `atlas-server`'s own `--build-from-raw` dev fallback both need a real,
//! validated `AtlasData` as their event-world/place/polity/catechism
//! adapters' own source (`event_world`/`place_adapter`/`polity_adapter`/
//! `catechism_adapter`'s own module doc comments) -- previously via
//! `AtlasData::load(&data/compiled)`, reading the now-deleted
//! `places.json`/`events.json`/`narratives.json`. Once those files are
//! gone, this function is what stands in their place for exactly the two
//! callers that build a graph from nothing (the server's own DEFAULT
//! startup path, loading the pre-built artifact, does NOT call this --
//! see `atlas_graph::legacy::atlas_data_overlay`'s own doc comment for why
//! a graph-reconstructed overlay, not a second raw+curated compile, is the
//! right, cheap counterpart there).
//!
//! `AtlasData::load` (the JSON-file loader) stays -- it still reads the
//! TEN surviving compiled files (canon/books-meta/chronology-anchors/
//! book-narration-windows/polities/landmarks/place-history/place-names-kjv/
//! land-mask/catechism); only its own five retiring-file reads become
//! honest empty defaults (the eras.json precedent, M-C's own deletion).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use atlas_core::data::{AtlasData, Polity};

use crate::report::{Counts, PolityStats, Report};
use crate::{catechism_map, curated, easton, geo, kjv, people, people_groups, polities, theographic, validate, xrefs};

/// Everything `main.rs`'s own report/write phase needs, alongside the
/// fully-populated, validated `AtlasData` every graph-building caller
/// needs -- one call, one source, no second copy to drift.
pub struct CompileOutput {
    pub data: AtlasData,
    pub report: Report,
    /// `data.place_history`/`data.place_name_aliases` are HashMaps (O(1)
    /// lookup -- `AtlasData::place_history_for`/`place_name_alias_for`,
    /// needed by every handler/scene.rs consumer on the `--build-from-raw`
    /// path); these carry the SAME content in the ORIGINAL curated-file
    /// order, kept alongside so `main.rs`'s own `place-history.json`/
    /// `place-names-kjv.json` writes stay byte-identical to their
    /// pre-M-C2 shape (curated order, never a HashMap's own incidental
    /// iteration order).
    pub place_history_list: Vec<atlas_core::data::PlaceHistory>,
    pub place_name_alias_list: Vec<atlas_core::data::PlaceNameAlias>,
}

/// Reads `raw_dir`/`curated_dir`, parses, merges, and hard-validates --
/// identical to `main.rs`'s own pre-M-C2 inlined compute phase, just
/// parameterized on its two input directories instead of a hardcoded
/// `../data` and returning its result instead of falling through into
/// file writes. Every `validate::run_*` call, every error `.context(...)`,
/// and every merge/calibration step below is UNCHANGED from that phase --
/// this is a relocation, not a rewrite (verified: `main.rs`'s own
/// post-M-C2 surviving `data/compiled/*.json` outputs are byte-identical
/// to the pre-M-C2 ones over the real committed sources).
pub fn compile(raw_dir: &Path, curated_dir: &Path) -> Result<CompileOutput> {
    // --- data/raw/ ------------------------------------------------------
    let kjv_raw = read(&raw_dir.join("kjv.json"))?;
    let (canon, verses) = kjv::parse(&kjv_raw)?;

    let ancient_raw = read(&raw_dir.join("geo/ancient.jsonl"))?;
    let geo_places = geo::parse(&ancient_raw)?;

    let mut place_slug_by_name: HashMap<String, String> = HashMap::new();
    for p in &geo_places {
        place_slug_by_name.entry(p.name.to_lowercase()).or_insert_with(|| p.id.clone());
    }
    let mut collisions_by_name: HashMap<String, Vec<String>> = HashMap::new();
    for p in &geo_places {
        collisions_by_name.entry(p.name.clone()).or_default().push(p.id.clone());
    }
    let mut slug_collisions: Vec<String> = collisions_by_name
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(name, mut ids)| {
            ids.sort();
            format!("{name} -> {}", ids.join(", "))
        })
        .collect();
    slug_collisions.sort();

    let theo_dir = raw_dir.join("theographic/theographic-bible-metadata-master/json");
    let places_json = read(&theo_dir.join("places.json"))?;
    let verses_json = read(&theo_dir.join("verses.json"))?;
    let events_json = read(&theo_dir.join("events.json"))?;
    let (theo_events, theo_new_places, theo_stats) =
        theographic::parse_events(&places_json, &verses_json, &events_json, &place_slug_by_name)?;

    // Batch P (the extensibility proof): Theographic PERSONS -> the
    // `AtlasData.people` sidecar (person_adapter.rs's own graph-side
    // source). Reads the SAME `theo_dir`/`verses_json` already in scope
    // above -- one more sibling fact file, not a new source tree.
    let people_json = read(&theo_dir.join("people.json"))?;
    let (people_list, people_stats) = people::parse_people(&people_json, &verses_json)?;

    // ENT-1a: Easton's Bible Dictionary (1897, PD) -- the `AtlasData.easton`
    // sidecar (`description_adapter.rs`'s own graph-side source). Reuses the
    // SAME already-in-scope `places_json` bytes `theographic::parse_events`
    // read above (no second disk read) -- `easton::parse_easton`'s own
    // module doc comment explains why its OWN in-memory parse of that
    // string is still independent (a second, minimal typed struct), not
    // shared parsing code.
    let easton_json = read(&theo_dir.join("easton.json"))?;
    let (easton_list, easton_stats) = easton::parse_easton(&easton_json, &places_json)?;

    // PG-1a: Theographic PEOPLE GROUPS -> the `AtlasData.people_groups`
    // sidecar (`atlas_graph::peoples_adapter`'s own graph-side source).
    // Reads the SAME `theo_dir` already in scope above -- one more sibling
    // fact file, not a new source tree.
    let people_groups_json = read(&theo_dir.join("peopleGroups.json"))?;
    let (people_groups_list, people_groups_stats) = people_groups::parse_people_groups(&people_groups_json)?;

    let xrefs_raw = read(&raw_dir.join("xrefs/cross_references.txt"))?;
    let (xrefs_map, xref_stats) = xrefs::parse(&xrefs_raw)?;
    let (xrefs_map, xref_dropped_missing_first_verse) = xrefs::filter_missing_first_verse(xrefs_map, &verses);

    // --- data/curated/ ---------------------------------------------------
    check_curated_inputs_exist(curated_dir)?;

    let eras = curated::parse_eras(&read(&curated_dir.join("eras.toml"))?)?;
    let chronology_anchors = curated::parse_chronology_anchors(&read(&curated_dir.join("chronology-anchors.toml"))?)?;
    let book_narration_windows = curated::parse_book_narration_windows(&read(&curated_dir.join("book-narration-windows.toml"))?)?;
    let mut books_meta = curated::parse_books(&read(&curated_dir.join("books.toml"))?)?;
    let events_extra = curated::parse_events_extra(&read(&curated_dir.join("events-extra.toml"))?)?;
    let narratives = read_narratives(&curated_dir.join("narratives"))?;

    // PG-1a: curated nation seeds + Gen-10 reclassification rows + eponymy
    // seed rows (decisions 1b/1c/3) -- `atlas_graph::peoples_adapter`'s own
    // graph-side source, alongside `people_groups_list` above.
    let (people_group_seeds, people_group_reclassify, named_after_seeds) =
        curated::parse_people_group_seeds(&read(&curated_dir.join("people-groups.toml"))?)?;

    // --- merge -------------------------------------------------------------
    let mut all_events = theo_events;
    let mut seen_event_ids: HashSet<String> = all_events.iter().map(|e| e.id.clone()).collect();
    for e in events_extra {
        if !seen_event_ids.insert(e.id.clone()) {
            bail!("data/curated/events-extra.toml defines event id '{}', which collides with an existing (Theographic) event id", e.id);
        }
        all_events.push(e);
    }

    let passages = read_passages(&curated_dir.join("passages"))?;
    for e in passages {
        if !seen_event_ids.insert(e.id.clone()) {
            bail!("data/curated/passages/ defines event id '{}', which collides with an existing event id", e.id);
        }
        all_events.push(e);
    }

    let event_witnesses = curated::parse_event_witnesses(&read(&curated_dir.join("event-witnesses.toml"))?)?;
    let mut witnesses_by_event: HashMap<String, Vec<atlas_core::data::EventWitness>> = HashMap::new();
    for (event_id, witness) in event_witnesses {
        witnesses_by_event.entry(event_id).or_default().push(witness);
    }
    let event_by_id: HashMap<String, usize> = all_events.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();
    for (event_id, witnesses) in witnesses_by_event {
        match event_by_id.get(&event_id) {
            Some(&idx) => all_events[idx].witnesses = witnesses,
            None => bail!(
                "data/curated/event-witnesses.toml: witness row names event_id '{event_id}', which does not match any compiled event id (Theographic or events-extra.toml)"
            ),
        }
    }

    let acts_sections = curated::parse_acts_sections(&read(&curated_dir.join("acts-sections.toml"))?)?;
    for (event_id, acts_section) in acts_sections {
        match event_by_id.get(&event_id) {
            Some(&idx) => all_events[idx].acts_section = Some(acts_section),
            None => bail!(
                "data/curated/acts-sections.toml: section row names event_id '{event_id}', which does not match any compiled event id (Theographic or events-extra.toml)"
            ),
        }
    }

    let atlas_sections = curated::parse_atlas_sections(&read(&curated_dir.join("atlas-sections.toml"))?)?;
    for (event_id, atlas_section) in atlas_sections {
        match event_by_id.get(&event_id) {
            Some(&idx) => all_events[idx].atlas_section = Some(atlas_section),
            None => bail!(
                "data/curated/atlas-sections.toml: section row names event_id '{event_id}', which does not match any compiled event id (Theographic, events-extra.toml, or passages/*.toml)"
            ),
        }
    }

    let mut all_places = geo_places;
    let mut seen_place_ids: HashSet<String> = all_places.iter().map(|p| p.id.clone()).collect();
    for p in theo_new_places {
        if seen_place_ids.insert(p.id.clone()) {
            all_places.push(p);
        }
    }

    let place_id_set: HashSet<&str> = all_places.iter().map(|p| p.id.as_str()).collect();
    let mut warnings = Vec::new();
    for b in &mut books_meta {
        if let Some(wp) = b.write_place.clone() {
            if !place_id_set.contains(wp.as_str()) {
                warnings.push(format!("books.toml: book '{}' references unknown write_place '{}', dropped", b.book, wp));
                b.write_place = None;
            }
        }
    }

    // --- counts + coverage stats (computed before AtlasData::new moves things) ---
    let mut counts = Counts {
        canon_books: canon.books.len(),
        places: all_places.len(),
        events: all_events.len(),
        narratives: narratives.len(),
        eras: eras.len(),
        books_meta: books_meta.len(),
        verses: verses.len(),
        cross_ref_sources: xrefs_map.len(),
        people: people_list.len(),
    };
    eprintln!(
        "THEOGRAPHIC PEOPLE: {} person record(s) compiled ({} with >=1 resolved verse link; {} of {} raw verse refs unresolved, dropped)",
        people_stats.total, people_stats.with_verses, people_stats.verse_refs_unresolved, people_stats.verse_refs_total
    );
    eprintln!(
        "EASTON'S DICTIONARY: {} entr(y/ies) parsed ({} dropped, no usable dictText) -- {} person-matched, {} place-matched ({} place slug(s) unresolved against places.json), {} multi, {} unmatched",
        easton_stats.total,
        easton_stats.no_text,
        easton_stats.person_matches,
        easton_stats.place_matches,
        easton_stats.place_slug_unresolved,
        easton_stats.multi,
        easton_stats.unmatched
    );
    eprintln!(
        "PG-1a PEOPLE GROUPS: {} Theographic peopleGroups.json record(s) compiled ({} dropped, no usable groupName) + {} curated nation seed(s) + {} reclassified Gen-10 gentilic(s) + {} curated named_after seed row(s)",
        people_groups_stats.total,
        people_groups_stats.no_name,
        people_group_seeds.len(),
        people_group_reclassify.len(),
        named_after_seeds.len()
    );
    let narrative_leg_counts: Vec<(String, usize)> = narratives.iter().map(|n| (n.id.clone(), n.legs.len())).collect();

    let mut geocoded_verses: HashSet<&str> = HashSet::new();
    for p in &all_places {
        for v in &p.verse_links {
            geocoded_verses.insert(v.as_str());
        }
    }
    let pct_verses_geocoded = if verses.is_empty() {
        0.0
    } else {
        100.0 * geocoded_verses.iter().filter(|v| verses.contains_key(**v)).count() as f64 / verses.len() as f64
    };
    let pct_events_dated = if theo_stats.total == 0 { 0.0 } else { 100.0 * theo_stats.dated as f64 / theo_stats.total as f64 };

    // --- atlas_core::merge::MERGE_PAIRS ------------------------------------
    validate::run_place_merges(atlas_core::merge::MERGE_PAIRS, &all_places)
        .context("data/compiled/* was NOT written; fix atlas_core::merge::MERGE_PAIRS (bad id or over-threshold pair)")?;

    // --- atlas_core::event_merge::EVENT_MERGE_PAIRS ------------------------
    validate::run_event_merges(atlas_core::event_merge::EVENT_MERGE_PAIRS, atlas_core::event_merge::EVENT_DISTINCT_PAIRS, &all_events)
        .context("data/compiled/* was NOT written; fix atlas_core::event_merge::EVENT_MERGE_PAIRS/EVENT_DISTINCT_PAIRS (an unlisted near-duplicate event pair, or a bad id)")?;

    validate::run_cross_book_duplicates(atlas_core::event_merge::EVENT_MERGE_PAIRS, atlas_core::event_merge::EVENT_DISTINCT_PAIRS, &all_events)
        .context("data/compiled/* was NOT written; fix atlas_core::event_merge::EVENT_MERGE_PAIRS/EVENT_DISTINCT_PAIRS (an unlisted cross-book duplicate event pair, found by title/year/place similarity)")?;

    // --- atlas_core::nt_calibration -----------------------------------------
    let nt_calibration_log = atlas_core::nt_calibration::apply_nt_calibration(&mut all_events);
    eprintln!(
        "NT CALIBRATION: {} surviving Theographic-scale NT event(s) shifted +{} year(s) to the AD-33 Passion anchor (full before/after table in batch-hotfix4-report.md's own \"Fix round 1\" section):",
        nt_calibration_log.len(),
        atlas_core::nt_calibration::NT_CALIBRATION_SHIFT
    );
    for row in &nt_calibration_log {
        eprintln!(
            "  {:<10} {:<45} {} -> {}  order_key {} -> {}",
            row.id, row.label, row.old_from_year, row.new_from_year, row.old_order_key, row.new_order_key
        );
    }

    // --- atlas_core::chronology::THEO_DATE_OVERRIDES -----------------------
    let theo_override_log = atlas_core::chronology::apply_theo_date_overrides(&mut all_events);
    eprintln!("CHRONOLOGY OVERRIDES: {} isolated raw-import date correction(s) applied (batch-hotfix6-report.md has the full derivation):", theo_override_log.len());
    for row in &theo_override_log {
        eprintln!("  {:<10} {:<35} {}..{} -> {}..{}", row.id, row.label, row.old_from_year, row.old_to_year, row.new_from_year, row.new_to_year);
    }

    // --- assemble, validate --------------------------------------------
    let mut data = AtlasData::new(canon, all_places, all_events, narratives, eras, books_meta, verses, xrefs_map).finish();
    validate::run(&data).context("data/compiled/* was NOT written; fix data/curated/ and re-run")?;
    counts.places = data.places.len();
    // Batch P: `people` needs no `validate::run`-style check of its own --
    // its own fail-loud boundary law (bijection + mentions completeness)
    // lives at the GRAPH adapter (`atlas_graph::person_adapter::
    // check_person_fidelity`, run unconditionally at pipeline LAW-CHECK
    // time), not here -- see that function's own doc comment for why.
    data.people = people_list;
    // ENT-1a: same "no validate::run-style check of its own" status as
    // `people` immediately above -- `easton`'s own consumer
    // (`atlas_graph::description_adapter`) is a pure best-effort matcher,
    // never a graph-shape law (an unmatched/absent description is always a
    // lawful `None`, not a validation failure).
    data.easton = easton_list;
    // PG-1a: same "no validate::run-style check of its own" status as
    // `people`/`easton` above -- these four sidecars' own fail-loud
    // boundary laws (node-count bijections, mentions completeness, the
    // eponym-existence conditional, the >=1-Scripture-ground law) live at
    // the GRAPH adapter (`atlas_graph::peoples_adapter::
    // check_peoples_fidelity`, run unconditionally at pipeline LAW-CHECK
    // time, mirroring `person_adapter::check_person_fidelity`'s own
    // precedent), not here.
    data.people_groups = people_groups_list;
    data.people_group_seeds = people_group_seeds;
    data.people_group_reclassify = people_group_reclassify;
    data.named_after_seeds = named_after_seeds;

    // --- chronology anchor table + era-window validator ---
    data.chronology_anchors = chronology_anchors;
    data.book_narration_windows = book_narration_windows;
    validate::run_chronology_anchors(&data.chronology_anchors, &data.events)
        .context("data/compiled/* was NOT written; fix data/curated/chronology-anchors.toml (a bad event_id, or an era_boundary row with no bound event_id)")?;
    validate::run_chronology_anchor_equality(&data.chronology_anchors, &data.events).context(
        "data/compiled/* was NOT written; a non-deferred anchor's table year disagrees with its bound event's own from_year, or a typed ANCHOR_DEFERRALS entry has gone stale",
    )?;
    validate::run_chronology_windows(&data.events, &data.book_narration_windows).context(
        "data/compiled/* was NOT written; fix the flagged event's own date (with anchor-table justification), add a data/curated/book-narration-windows.toml row for a book with none, or add an atlas_core::chronology::WINDOW_EXEMPTIONS row with a stated reason",
    )?;
    validate::run_era_boundaries(&data)
        .context("data/compiled/* was NOT written; fix the flagged event's own date, or its book's own data/curated/book-narration-windows.toml window")?;
    counts.events = data.events.len();

    // --- data/curated/polities/ ---------------------------------------------
    let (compiled_polities, polity_stats) = process_polities(&curated_dir.join("polities"))?;
    validate::run_polities(&compiled_polities, &polities::BIBLICAL_WORLD_BBOX, &data.verses)
        .context("data/compiled/polities.json was NOT written; fix data/curated/polities/*.toml and re-run")?;
    data.polities = compiled_polities;

    let landmarks = curated::parse_landmarks(&read(&curated_dir.join("landmarks.toml"))?)?;
    validate::run_landmarks(&landmarks, &polities::BIBLICAL_WORLD_BBOX)
        .context("data/compiled/landmarks.json was NOT written; fix data/curated/landmarks.toml and re-run")?;
    let landmarks_count = landmarks.len();
    data.landmarks = landmarks;

    // --- data/curated/land-mask.toml ---
    let land_mask_regions = curated::parse_land_mask(&read(&curated_dir.join("land-mask.toml"))?)?;
    validate::run_land_mask(&land_mask_regions, &polities::BIBLICAL_WORLD_BBOX)
        .context("data/compiled/land-mask.json was NOT written; fix data/curated/land-mask.toml and re-run")?;
    let land_mask: Vec<Vec<(f64, f64)>> = land_mask_regions.iter().flat_map(|r| r.rings.clone()).collect();
    let land_mask_points: usize = land_mask.iter().map(|r| r.len()).sum();
    let land_mask_region_count = land_mask_regions.len();
    let land_mask_ring_count = land_mask.len();
    data.land_mask = land_mask;

    // --- data/curated/place-history.toml -
    let place_history = curated::parse_place_history(&read(&curated_dir.join("place-history.toml"))?)?;
    let compiled_place_ids: HashSet<&str> = data.places.iter().map(|p| p.id.as_str()).collect();
    validate::run_place_history(&place_history, &compiled_place_ids, &data.verses)
        .context("data/compiled/place-history.json was NOT written; fix data/curated/place-history.toml and re-run")?;
    data.place_history = place_history.iter().map(|h| (h.id.clone(), h.clone())).collect();
    let place_history_list = place_history;

    // --- data/curated/place-names-kjv.toml -
    let place_names_kjv = curated::parse_place_names_kjv(&read(&curated_dir.join("place-names-kjv.toml"))?)?;
    validate::run_place_names_kjv(&place_names_kjv, &data.places, &data.verses)
        .context("data/compiled/place-names-kjv.json was NOT written; fix data/curated/place-names-kjv.toml and re-run")?;
    data.place_name_aliases = place_names_kjv.iter().map(|a| (a.id.clone(), a.clone())).collect();
    let place_name_alias_list = place_names_kjv;

    // --- data/curated/catechism.toml ----
    let mut catechism = curated::parse_catechism(&read(&curated_dir.join("catechism.toml"))?)?;

    let catechism_mapping = curated::parse_catechism_mapping(&read(&curated_dir.join("catechism-mapping.toml"))?)?;
    let catechism_mapping_sha = "0be24fee92e6333f817c4c2a08f99cf7c5274295";
    let catechism_mapping_root = raw_dir.join("catechism-mapping").join(format!("catechism-{catechism_mapping_sha}"));
    let mut catechism_questions_by_item =
        catechism_map::build_questions_from_mapping(&catechism_mapping, &catechism_mapping_root, &data.verses)
            .context("data/compiled/catechism.json was NOT written; fix data/curated/catechism-mapping.toml, the ingested resources/*.yaml, or re-fetch (see data/fetch-raw.ps1)")?;

    let catechism_deut5 = curated::parse_catechism_deut5(&read(&curated_dir.join("catechism-deut5.toml"))?)?;
    for entry in &catechism_deut5 {
        let mut verse_list = Vec::new();
        for v in &entry.verses {
            curated::expand_verse_ref(v, &entry.item, &mut verse_list)?;
        }
        catechism_questions_by_item.entry(entry.item.clone()).or_default().push(atlas_core::data::CatechismQuestion {
            title: catechism_map::DEUT5_QUESTION_TITLE.to_string(),
            verses: verse_list,
            source: "deut5-parallel".to_string(),
        });
    }

    catechism_map::merge_questions_into_parts(&mut catechism, catechism_questions_by_item)
        .context("data/compiled/catechism.json was NOT written; fix data/curated/catechism-mapping.toml or data/curated/catechism-deut5.toml (an item id doesn't match catechism.toml)")?;

    validate::run_catechism(&catechism, &data.verses)
        .context("data/compiled/catechism.json was NOT written; fix data/curated/catechism.toml and re-run")?;
    let catechism_items_count: usize = catechism.iter().map(|p| p.items.len()).sum();

    let mut catechism_distinct_verse_set: HashSet<&str> = HashSet::new();
    let mut catechism_items_reachable = 0usize;
    let mut catechism_per_part: Vec<(String, usize, usize)> = Vec::with_capacity(catechism.len());
    for part in &catechism {
        let mut part_reachable = 0usize;
        for item in &part.items {
            let reachable = !item.verses.is_empty() || item.questions.iter().any(|q| !q.verses.is_empty());
            if reachable {
                catechism_items_reachable += 1;
                part_reachable += 1;
            }
            catechism_distinct_verse_set.extend(item.verses.iter().map(String::as_str));
            catechism_distinct_verse_set.extend(item.questions.iter().flat_map(|q| q.verses.iter().map(String::as_str)));
        }
        catechism_per_part.push((part.title.clone(), part_reachable, part.items.len()));
    }
    let catechism_distinct_verses = catechism_distinct_verse_set.len();
    let catechism_parts_count = catechism.len();
    data.catechism = catechism;

    let report = Report {
        counts,
        pct_events_dated,
        pct_verses_geocoded,
        narrative_leg_counts,
        slug_collisions,
        warnings,
        xref_dropped_unparseable: xref_stats.dropped_unparseable,
        xref_dropped_self: xref_stats.dropped_self,
        xref_dropped_missing_first_verse,
        polities: polity_stats,
        landmarks_count,
        land_mask_regions: land_mask_region_count,
        land_mask_rings: land_mask_ring_count,
        land_mask_points,
        catechism_parts: catechism_parts_count,
        catechism_items: catechism_items_count,
        catechism_items_reachable,
        catechism_distinct_verses,
        catechism_per_part,
    };

    Ok(CompileOutput { data, report, place_history_list, place_name_alias_list })
}

/// Reads every `*.toml` polity file under `polities_curated_dir` (sorted by
/// filename, i.e. by polity id, for a deterministic processing/report
/// order), parses each via `curated::parse_polity`, and returns the full
/// compiled roster alongside per-polity stats -- unchanged from `main.rs`'s
/// own pre-M-C2 function of the same name/behavior.
fn process_polities(polities_curated_dir: &Path) -> Result<(Vec<Polity>, Vec<PolityStats>)> {
    let mut paths: Vec<PathBuf> = fs::read_dir(polities_curated_dir)
        .with_context(|| format!("reading directory {} -- these are hand-authored and committed, not fetched; see LICENSES.md", polities_curated_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    if paths.is_empty() {
        bail!("{} contains no *.toml polity files -- these are hand-authored and committed, not fetched; see LICENSES.md", polities_curated_dir.display());
    }

    let mut compiled = Vec::with_capacity(paths.len());
    let mut stats = Vec::with_capacity(paths.len());
    for path in &paths {
        let content = read(path)?;
        let polity = curated::parse_polity(&content).with_context(|| format!("parsing polity {}", path.display()))?;
        let points: usize = polity.eras.iter().flat_map(|e| e.rings.iter()).map(|r| r.len()).sum();
        stats.push(PolityStats { id: polity.id.clone(), eras: polity.eras.len(), points });
        compiled.push(polity);
    }

    let ids: Vec<&str> = compiled.iter().map(|p| p.id.as_str()).collect();
    let keys = polities::assign_color_keys(&ids);
    for (polity, key) in compiled.iter_mut().zip(keys) {
        polity.color_key = key;
    }

    Ok((compiled, stats))
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Parses every `*.toml` file directly under `passages_dir` (one book per
/// file), each in the SAME `[[event]]` schema `events-extra.toml` uses,
/// concatenated in sorted filename order for reproducible output --
/// unchanged from `main.rs`'s own pre-M-C2 function of the same name.
fn read_passages(passages_dir: &Path) -> Result<Vec<atlas_core::data::Event>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(passages_dir)
        .with_context(|| format!("reading directory {}", passages_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();

    let mut events = Vec::new();
    for path in paths {
        let content = read(&path)?;
        let parsed = curated::parse_events_extra(&content).with_context(|| format!("parsing {}", path.display()))?;
        events.extend(parsed);
    }
    Ok(events)
}

/// Parses every `*.toml` file directly under `narratives_dir` (one
/// narrative per file), in sorted filename order -- unchanged from
/// `main.rs`'s own pre-M-C2 function of the same name.
fn read_narratives(narratives_dir: &Path) -> Result<Vec<atlas_core::data::Narrative>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(narratives_dir)
        .with_context(|| format!("reading directory {}", narratives_dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();

    let mut narratives = Vec::with_capacity(paths.len());
    for path in paths {
        let content = read(&path)?;
        let narrative = curated::parse_narrative(&content).with_context(|| format!("parsing {}", path.display()))?;
        narratives.push(narrative);
    }
    Ok(narratives)
}

/// Checks that every file/directory `data/curated/` must provide is
/// present -- unchanged from `main.rs`'s own pre-M-C2 function of the same
/// name.
fn check_curated_inputs_exist(curated_dir: &Path) -> Result<()> {
    let mut missing = Vec::new();

    let eras_path = curated_dir.join("eras.toml");
    if !eras_path.is_file() {
        missing.push(format!("{}", eras_path.display()));
    }
    let chronology_anchors_path = curated_dir.join("chronology-anchors.toml");
    if !chronology_anchors_path.is_file() {
        missing.push(format!("{} (Batch HOTFIX-6)", chronology_anchors_path.display()));
    }
    let book_narration_windows_path = curated_dir.join("book-narration-windows.toml");
    if !book_narration_windows_path.is_file() {
        missing.push(format!("{} (Batch HOTFIX-6)", book_narration_windows_path.display()));
    }
    let books_path = curated_dir.join("books.toml");
    if !books_path.is_file() {
        missing.push(format!("{}", books_path.display()));
    }
    let events_extra_path = curated_dir.join("events-extra.toml");
    if !events_extra_path.is_file() {
        missing.push(format!("{}", events_extra_path.display()));
    }
    let event_witnesses_path = curated_dir.join("event-witnesses.toml");
    if !event_witnesses_path.is_file() {
        missing.push(format!("{}", event_witnesses_path.display()));
    }
    let acts_sections_path = curated_dir.join("acts-sections.toml");
    if !acts_sections_path.is_file() {
        missing.push(format!("{}", acts_sections_path.display()));
    }
    let atlas_sections_path = curated_dir.join("atlas-sections.toml");
    if !atlas_sections_path.is_file() {
        missing.push(format!("{}", atlas_sections_path.display()));
    }
    let passages_dir = curated_dir.join("passages");
    let has_passages_files = passages_dir.is_dir()
        && fs::read_dir(&passages_dir)
            .map(|entries| entries.filter_map(|e| e.ok()).any(|e| e.path().extension().is_some_and(|ext| ext == "toml")))
            .unwrap_or(false);
    if !has_passages_files {
        missing.push(format!("{} (expected one or more *.toml passage files -- Batch W1)", passages_dir.display()));
    }
    let narratives_dir = curated_dir.join("narratives");
    let has_narrative_files = narratives_dir.is_dir()
        && fs::read_dir(&narratives_dir)
            .map(|entries| entries.filter_map(|e| e.ok()).any(|e| e.path().extension().is_some_and(|ext| ext == "toml")))
            .unwrap_or(false);
    if !has_narrative_files {
        missing.push(format!("{} (expected one or more *.toml narrative files)", narratives_dir.display()));
    }
    let landmarks_path = curated_dir.join("landmarks.toml");
    if !landmarks_path.is_file() {
        missing.push(format!("{}", landmarks_path.display()));
    }
    let land_mask_path = curated_dir.join("land-mask.toml");
    if !land_mask_path.is_file() {
        missing.push(format!("{}", land_mask_path.display()));
    }
    let place_history_path = curated_dir.join("place-history.toml");
    if !place_history_path.is_file() {
        missing.push(format!("{}", place_history_path.display()));
    }
    let place_names_kjv_path = curated_dir.join("place-names-kjv.toml");
    if !place_names_kjv_path.is_file() {
        missing.push(format!("{}", place_names_kjv_path.display()));
    }
    let catechism_path = curated_dir.join("catechism.toml");
    if !catechism_path.is_file() {
        missing.push(format!("{}", catechism_path.display()));
    }
    let catechism_mapping_path = curated_dir.join("catechism-mapping.toml");
    if !catechism_mapping_path.is_file() {
        missing.push(format!("{}", catechism_mapping_path.display()));
    }
    let catechism_deut5_path = curated_dir.join("catechism-deut5.toml");
    if !catechism_deut5_path.is_file() {
        missing.push(format!("{}", catechism_deut5_path.display()));
    }
    let people_groups_path = curated_dir.join("people-groups.toml");
    if !people_groups_path.is_file() {
        missing.push(format!("{} (PG-1a)", people_groups_path.display()));
    }
    let polities_dir = curated_dir.join("polities");
    let has_polity_files = polities_dir.is_dir()
        && fs::read_dir(&polities_dir)
            .map(|entries| entries.filter_map(|e| e.ok()).any(|e| e.path().extension().is_some_and(|ext| ext == "toml")))
            .unwrap_or(false);
    if !has_polity_files {
        missing.push(format!("{} (expected one or more *.toml polity files)", polities_dir.display()));
    }

    if missing.is_empty() {
        return Ok(());
    }

    let list = missing.iter().map(|m| format!("  - {m}")).collect::<Vec<_>>().join("\n");
    bail!(
        "data/curated/ is incomplete -- create it per Task 5 (curated data authoring) before running the ETL.\nMissing:\n{list}"
    );
}
