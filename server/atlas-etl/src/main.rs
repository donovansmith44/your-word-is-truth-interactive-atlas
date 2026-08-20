//! atlas-etl binary: the only place in this crate that touches the
//! filesystem. Reads `data/raw/` (downloaded by `data/fetch-raw.ps1`, never
//! fetched here — this binary does no networking) and `data/curated/`
//! (hand-authored, committed), calls the pure parsers in `atlas_etl::*`,
//! merges their outputs, hard-validates the result, and writes
//! `data/compiled/*.json` + `report.txt`. Run as `cargo run -p atlas-etl`
//! from `server/` (paths below are relative to that working directory).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use atlas_core::data::{AtlasData, Polity};
use atlas_etl::report::{Counts, PolityStats, Report};
use atlas_etl::{curated, geo, kjv, polities, report, theographic, validate, xrefs};

fn main() -> Result<()> {
    // Built from components (not a literal "../data/raw" string) so joined
    // paths use a consistent separator in error/report messages instead of
    // mixing '/' (as typed) and '\' (from PathBuf::join) on Windows.
    let data_dir: PathBuf = Path::new("..").join("data");
    let raw_dir = data_dir.join("raw");
    let curated_dir = data_dir.join("curated");
    let compiled_dir = data_dir.join("compiled");

    // --- data/raw/ ------------------------------------------------------
    let kjv_raw = read(&raw_dir.join("kjv.json"))?;
    let (canon, verses) = kjv::parse(&kjv_raw)?;

    let ancient_raw = read(&raw_dir.join("geo/ancient.jsonl"))?;
    let geo_places = geo::parse(&ancient_raw)?;

    let mut place_slug_by_name: HashMap<String, String> = HashMap::new();
    for p in &geo_places {
        place_slug_by_name.entry(p.name.to_lowercase()).or_insert_with(|| p.id.clone());
    }
    // Same-named geo places collide to `name`, `name-2`, ... (see geo::parse);
    // group by name here (before merging in Theographic-synthesized places,
    // which don't go through that collision scheme) so the report reflects
    // exactly what geo::parse did.
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

    let xrefs_raw = read(&raw_dir.join("xrefs/cross_references.txt"))?;
    let (xrefs_map, xref_stats) = xrefs::parse(&xrefs_raw)?;
    let (xrefs_map, xref_dropped_missing_first_verse) = xrefs::filter_missing_first_verse(xrefs_map, &verses);

    // --- data/curated/ ---------------------------------------------------
    // Checked here (after data/raw/ parsing succeeds, before touching any
    // curated file) so the aggregated "missing curated inputs" error is
    // clean and doesn't get tangled up with unrelated raw-file IO errors.
    check_curated_inputs_exist(&curated_dir)?;

    let eras = curated::parse_eras(&read(&curated_dir.join("eras.toml"))?)?;
    let mut books_meta = curated::parse_books(&read(&curated_dir.join("books.toml"))?)?;
    let events_extra = curated::parse_events_extra(&read(&curated_dir.join("events-extra.toml"))?)?;
    let narratives = read_narratives(&curated_dir.join("narratives"))?;

    // --- merge -------------------------------------------------------------
    let mut all_events = theo_events;
    let mut seen_event_ids: HashSet<String> = all_events.iter().map(|e| e.id.clone()).collect();
    for e in events_extra {
        if !seen_event_ids.insert(e.id.clone()) {
            bail!("data/curated/events-extra.toml defines event id '{}', which collides with an existing (Theographic) event id", e.id);
        }
        all_events.push(e);
    }

    let mut all_places = geo_places;
    let mut seen_place_ids: HashSet<String> = all_places.iter().map(|p| p.id.clone()).collect();
    for p in theo_new_places {
        if seen_place_ids.insert(p.id.clone()) {
            all_places.push(p);
        }
    }

    // WARN + drop unknown book.write_place references (needs the final place set).
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
    let counts = Counts {
        canon_books: canon.books.len(),
        places: all_places.len(),
        events: all_events.len(),
        narratives: narratives.len(),
        eras: eras.len(),
        books_meta: books_meta.len(),
        verses: verses.len(),
        cross_ref_sources: xrefs_map.len(),
    };
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
    let pct_events_dated =
        if theo_stats.total == 0 { 0.0 } else { 100.0 * theo_stats.dated as f64 / theo_stats.total as f64 };

    // --- assemble, validate --------------------------------------------
    let data = AtlasData::new(canon, all_places, all_events, narratives, eras, books_meta, verses, xrefs_map).finish();
    validate::run(&data).context("data/compiled/* was NOT written; fix data/curated/ and re-run")?;

    // --- data/curated/polities/ (Batch B2: hand-authored per-polity
    // timerange borders, "borders v2, the cartographer's edition") --------
    // Supersedes Batch L's own curated snapshot-year GeoJSON pipeline
    // entirely -- see atlas_core::data::Polity's own doc comment. Everything
    // here is computed BEFORE any file is written, same all-or-nothing
    // property `validate::run` above already gives the core eight files — a
    // bad polity file or a bad landmarks.toml must not leave a
    // half-written data/compiled/ behind.
    let (compiled_polities, polity_stats) = process_polities(&curated_dir.join("polities"))?;
    validate::run_polities(&compiled_polities, &polities::BIBLICAL_WORLD_BBOX)
        .context("data/compiled/polities.json was NOT written; fix data/curated/polities/*.toml and re-run")?;

    let landmarks = curated::parse_landmarks(&read(&curated_dir.join("landmarks.toml"))?)?;
    validate::run_landmarks(&landmarks, &polities::BIBLICAL_WORLD_BBOX)
        .context("data/compiled/landmarks.json was NOT written; fix data/curated/landmarks.toml and re-run")?;

    // --- data/curated/place-history.toml (Batch E: time-accurate places) -
    let place_history = curated::parse_place_history(&read(&curated_dir.join("place-history.toml"))?)?;
    let compiled_place_ids: HashSet<&str> = data.places.iter().map(|p| p.id.as_str()).collect();
    validate::run_place_history(&place_history, &compiled_place_ids, &data.verses)
        .context("data/compiled/place-history.json was NOT written; fix data/curated/place-history.toml and re-run")?;

    // --- write compiled output ------------------------------------------
    fs::create_dir_all(&compiled_dir).with_context(|| format!("creating {}", compiled_dir.display()))?;
    write_json(&compiled_dir.join("canon.json"), &data.canon)?;
    write_json(&compiled_dir.join("places.json"), &data.places)?;
    write_json(&compiled_dir.join("events.json"), &data.events)?;
    write_json(&compiled_dir.join("narratives.json"), &data.narratives)?;
    write_json(&compiled_dir.join("eras.json"), &data.eras)?;
    write_json(&compiled_dir.join("books-meta.json"), &data.books_meta)?;
    write_json(&compiled_dir.join("verses-kjv.json"), &data.verses)?;
    write_json(&compiled_dir.join("cross-refs.json"), &data.cross_refs)?;

    write_json(&compiled_dir.join("polities.json"), &compiled_polities)?;
    write_json(&compiled_dir.join("landmarks.json"), &landmarks)?;
    write_json(&compiled_dir.join("place-history.json"), &place_history)?;

    let rpt = Report {
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
        landmarks_count: landmarks.len(),
    };
    let text = report::write(&rpt);
    fs::write(compiled_dir.join("report.txt"), &text).context("writing data/compiled/report.txt")?;
    print!("{text}");

    Ok(())
}

/// Reads every `*.toml` polity file under `polities_curated_dir` (sorted by
/// filename -- i.e. by polity id, since the filename stem IS the id, see
/// `check_curated_inputs_exist`'s sibling check below -- for a deterministic
/// processing/report order), parses each via `curated::parse_polity`, and
/// returns the full compiled roster alongside per-polity stats for the
/// report. Fatal (bubbles up `curated::parse_polity`'s own errors) on any
/// unparseable file, and fatal if the directory is missing/empty (nothing to
/// compile at all) -- same "hand-authored and committed, not fetched"
/// framing `process_border_snapshots` (Batch B2's predecessor, deleted)
/// used for its own curated directory. Does NOT itself check for two files
/// claiming the same polity `id` -- unlike the old snapshot pipeline's
/// `BTreeMap<year, _>` (which literally could not represent two entries
/// under one key), a `Vec<Polity>` has no such structural constraint, so
/// that check is `validate::run_polities`'s job instead (its own
/// `check_duplicate_ids` call), same single-source-of-truth split every
/// OTHER duplicate-id check in this crate already follows.
///
/// Fix round 1 (M1): this is also where every polity's REAL `color_key`
/// gets assigned now -- `curated::parse_polity` leaves it provisional (0)
/// per-file, since a lone file can't see the rest of the roster it might
/// collide with. `paths` is already sorted (by filename, i.e. by id) above,
/// so `compiled` comes out of the parse loop in that exact same order --
/// exactly the "SORTED polity id" order `polities::assign_color_keys`
/// needs, with no second sort required.
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

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = serde_json::to_string_pretty(value).with_context(|| format!("serializing {}", path.display()))?;
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

/// Parses every `*.toml` file directly under `narratives_dir` (one narrative
/// per file), in sorted filename order for reproducible output.
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

/// Checks that every file/directory `data/curated/` must provide is present,
/// and if not, fails with one clear, aggregated error rather than whatever
/// low-level `io::Error` the first missing file would otherwise produce.
/// Per the M1 build order, `data/curated/` doesn't exist until Task 5 runs —
/// seeing this exact error (not a panic, not a raw "file not found") is the
/// expected, correct outcome of atlas-etl's Task 4 milestone.
fn check_curated_inputs_exist(curated_dir: &Path) -> Result<()> {
    let mut missing = Vec::new();

    let eras_path = curated_dir.join("eras.toml");
    if !eras_path.is_file() {
        missing.push(format!("{}", eras_path.display()));
    }
    let books_path = curated_dir.join("books.toml");
    if !books_path.is_file() {
        missing.push(format!("{}", books_path.display()));
    }
    let events_extra_path = curated_dir.join("events-extra.toml");
    if !events_extra_path.is_file() {
        missing.push(format!("{}", events_extra_path.display()));
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
    let place_history_path = curated_dir.join("place-history.toml");
    if !place_history_path.is_file() {
        missing.push(format!("{}", place_history_path.display()));
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
