//! THE LAW: **no serving code may reach event/place/narrative presentation
//! data through `AtlasData.events` / `.places` / `.narratives`.**
//!
//! # Why this file exists (OVERLAY-1-HOTFIX-1)
//!
//! `AtlasData::load` has not read `places.json`/`events.json`/
//! `narratives.json` since M-C2 -- it seeds all three with `Vec::new()`.
//! Until OVERLAY-1 Task 5 (d31ac6c) `atlas_server::load` quietly re-filled
//! them at boot from `legacy::atlas_data_overlay`; Task 5 deleted that
//! overlay, which is the whole point of the batch (the events the map
//! composes from exist ONCE, in `GraphSceneSource`, not twice). Task 5's own
//! audit for readers of the now-permanently-empty collections was a textual
//! grep for `data.events` / `data.places` / `data.narratives` over the
//! serving crates. It came back clean, and it was wrong: one reader hid
//! behind a function boundary in another crate --
//!
//! ```text
//! handlers.rs:  adjacent_event(&data, &pid)          <-- no `data.events` here
//! narrative.rs: pub fn adjacent_event(d: &AtlasData, ..) { d.event_by_id(..)? }
//! ```
//!
//! -- so `GET /api/narrative/event/{id}` served membership and label but a
//! blank `prior`, a blank `following` and `timeline: {}` for EVERY event in
//! the real atlas. No Rust test failed: the one test that asserts adjacency
//! (`tests/api.rs::narrative_event_positions_endpoint`) runs over
//! `demo_fixture()`, which hand-fills `events`. Three Playwright specs --
//! minutes of browser time, at the very end of the gate -- were the only
//! thing that caught it.
//!
//! This is the grep that would have caught it in seconds, promoted to a
//! standing test so it runs every time. It scans the two SERVING crates'
//! sources (`atlas-server/src`, `atlas-cli/src`) for the exact shapes of
//! that regression.
//!
//! # Its two companions, and the honest limit of each
//!
//! The fix that made it pass changed `adjacent_event`'s SIGNATURE, to
//! `&dyn SceneSource`. Fix round 1 (review I-1) corrects what that buys:
//! NOT compile-level enforcement -- `impl SceneSource for AtlasData` exists,
//! so `adjacent_event(&*data, ..)` still compiles and still answers empty --
//! only the removal of the accidental spelling.
//!
//! This scan cannot be pointed at `atlas-core`, where the reader actually
//! hid, because `scene.rs` legitimately names its `&dyn SceneSource`
//! parameters `d` and a receiver-name scan there would be one long false
//! positive. So `atlas-core/tests/no_atlas_data_in_public_signatures.rs`
//! (fix round 1, review I-2) covers that crate by TYPE instead: no new
//! public fn there may take an `AtlasData` at all.
//!
//! Neither law is the cure. The cure is ETL-INPUT-1 -- deleting
//! `AtlasData.events`/`.places`/`.narratives` so there is nothing to read.
//! Until then: this file for the serving crates, that one for `atlas-core`.
//!
//! # What is forbidden here
//!
//! * `data.events` / `data.places` / `data.narratives` -- the three empty vecs.
//! * `.event_by_id(` / `.place_by_id(` on a named `AtlasData` receiver.
//! * the six accessors `AtlasData::finish()` DERIVES from those vecs
//!   (fix round 1, review I-3): `heading_for_verse`,
//!   `heading_anchor_collisions`, `timeline_position`, `timeline_event_at`,
//!   `event_bearing_place_ids`, `total_events_for`. These matter because
//!   emptiness propagates through `finish()` by name rather than by field
//!   access: each returns an honest-looking, permanently-wrong answer (an
//!   empty map, a `None`, a 0) on a serving `AtlasData`, with a 200 status,
//!   which is precisely how the original regression hid. The review's own
//!   audit found no serving caller of any of the six today; this law is what
//!   keeps that true.
//! * `adjacent_event(&data` / `(&*data` / `(&d,` -- this hotfix's own
//!   regression, verbatim.
//!
//! # The allowlist
//!
//! [`ALLOWLIST`] is **empty and must stay empty.** It exists so that adding
//! an exception is a visible, reviewable diff with a written reason rather
//! than a quiet loosening of the pattern list. A non-empty allowlist is
//! itself asserted against below, so "just allowlist it" cannot be done
//! silently either.

use std::path::{Path, PathBuf};

/// Every forbidden shape, as `(name, matcher, why)`. Matching is done with
/// plain `str` operations rather than a regex dependency -- each of these is
/// a literal substring or a literal-pair test, which is all the regexes in
/// the hotfix brief actually needed.
struct Law {
    name: &'static str,
    why: &'static str,
    hit: fn(&str) -> bool,
}

fn laws() -> Vec<Law> {
    vec![
        Law {
            name: r"data\.events",
            why: "AtlasData.events is EMPTY on every serving path -- read GraphService::scene_source instead",
            hit: |code| code.contains("data.events"),
        },
        Law {
            name: r"data\.places",
            why: "AtlasData.places is EMPTY on every serving path -- read GraphService::scene_source instead",
            hit: |code| code.contains("data.places"),
        },
        Law {
            name: r"data\.narratives",
            why: "AtlasData.narratives is EMPTY on every serving path -- read GraphSceneSource::narrative_list instead",
            hit: |code| code.contains("data.narratives"),
        },
        Law {
            // The named-receiver form of `.event_by_id(` / `.place_by_id(`.
            // `data` is what every serving handler binds its `Arc<AtlasData>`
            // to (the axum `State` extractor), and `d` is what the core
            // crate's own helpers call it; naming the receivers is how this
            // scan stays a scan (no type resolution) while still being
            // specific enough never to fire on `src.event_by_id(..)`, which
            // is the CORRECT call through the SceneSource.
            name: r"\.event_by_id\( / \.place_by_id\( on an AtlasData-named receiver",
            why: "AtlasData::event_by_id/place_by_id look into the EMPTY vecs -- call them on the SceneSource",
            hit: |code| {
                RECEIVERS.iter().any(|recv| {
                    [".event_by_id(", ".place_by_id("].iter().any(|m| contains_call(code, recv, m))
                })
            },
        },
        Law {
            // Fix round 1, review I-3. `AtlasData.events`/`.places`/
            // `.narratives` being empty does not only blank the three fields
            // themselves -- it blanks every index `AtlasData::finish()`
            // DERIVES from them, and those are reached by name, not by
            // touching a vec. The review's audit found no serving caller of
            // any of these today; this law is what keeps that true. Each
            // returns an honest-looking, permanently-wrong answer on the
            // serving path (an empty map, a `None`, a 0), never an error --
            // the same silent-200 failure mode as the original regression.
            name: r"the six EMPTY-DERIVED AtlasData accessors on a named receiver",
            why: "derived from the EMPTY events/places/narratives by AtlasData::finish() -- always blank on a serving AtlasData",
            hit: |code| {
                const DERIVED: [&str; 6] = [
                    ".heading_for_verse(",
                    ".heading_anchor_collisions(",
                    ".timeline_position(",
                    ".timeline_event_at(",
                    ".event_bearing_place_ids(",
                    ".total_events_for(",
                ];
                RECEIVERS.iter().any(|recv| DERIVED.iter().any(|m| contains_call(code, recv, m)))
            },
        },
        Law {
            // The exact call shape of this hotfix's own regression.
            name: r"adjacent_event\(\s*&\*?data",
            why: "adjacent_event takes &dyn SceneSource now -- passing AtlasData is the OVERLAY-1-HOTFIX-1 regression itself",
            hit: |code| {
                code.contains("adjacent_event(&data")
                    || code.contains("adjacent_event(&*data")
                    || code.contains("adjacent_event( &data")
                    || code.contains("adjacent_event(&d,")
            },
        },
    ]
}

/// EMPTY, and asserted empty below. See this file's own header.
const ALLOWLIST: &[(&str, u32, &str)] = &[];

/// The names a serving `AtlasData` is bound to. `data` is what every handler
/// binds its `Arc<AtlasData>` to (the axum `State` extractor), `sidecars` is
/// what `load.rs`'s own prose calls it, and `d`/`atlas` are the short forms
/// used elsewhere. Naming the receivers is how the method laws below stay a
/// text scan (no type resolution) while still never firing on
/// `src.event_by_id(..)` / `source.total_events_for(..)`, which are the
/// CORRECT calls, through the SceneSource.
const RECEIVERS: [&str; 4] = ["data", "atlas", "sidecars", "d"];

/// `receiver` + `method` as a WHOLE identifier -- i.e. `data.event_by_id(`
/// matches, but `loaded.event_by_id(` does not match receiver `d`. Without
/// the boundary the one-letter receiver `d` (what the core crate's own
/// helpers name their `&AtlasData`) would fire on the tail of any identifier
/// ending in `d`, and a law that cries wolf gets deleted.
fn contains_call(code: &str, receiver: &str, method: &str) -> bool {
    let needle = format!("{receiver}{method}");
    let mut from = 0usize;
    while let Some(rel) = code[from..].find(&needle) {
        let at = from + rel;
        let boundary = at == 0 || !code[..at].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
        if boundary {
            return true;
        }
        from = at + 1;
    }
    false
}

/// The code half of a line: everything before the first `//`. Crude on
/// purpose -- it can only ever cause a FALSE NEGATIVE (a forbidden call
/// hiding after a `//` inside a string literal), never a false positive, and
/// nothing in these two crates puts a `//` before live code on the same
/// line except in URLs, which carry none of these patterns.
fn code_of(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn serving_sources() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut out = Vec::new();
    rs_files(&root.join("atlas-server/src"), &mut out);
    rs_files(&root.join("atlas-cli/src"), &mut out);
    out.sort();
    out
}

#[test]
fn no_serving_source_reads_event_presentation_data_through_atlas_data() {
    assert!(ALLOWLIST.is_empty(), "the allowlist must stay EMPTY -- see this file's own header: {ALLOWLIST:?}");

    let files = serving_sources();
    assert!(
        files.len() >= 10,
        "the scan found only {} source files -- it has lost its target directories and would pass vacuously",
        files.len()
    );

    let laws = laws();
    let mut scanned_lines = 0usize;
    let mut violations: Vec<String> = Vec::new();

    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_else(|e| panic!("reading {}: {e}", file.display()));
        for (i, line) in text.lines().enumerate() {
            scanned_lines += 1;
            let code = code_of(line);
            let trimmed = code.trim_start();
            // Block-comment/doc-continuation lines carry no code.
            if trimmed.starts_with('*') {
                continue;
            }
            for law in &laws {
                if (law.hit)(code) {
                    violations.push(format!(
                        "{}:{}\n    matches: {}\n    why:     {}\n    line:    {}",
                        file.display(),
                        i + 1,
                        law.name,
                        law.why,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        scanned_lines > 2000,
        "only {scanned_lines} lines scanned -- too few to be reading the real serving crates"
    );
    println!("no_legacy_event_reads: scanned {} files / {} lines", files.len(), scanned_lines);

    assert!(
        violations.is_empty(),
        "OVERLAY-1-HOTFIX-1 LAW VIOLATED -- serving code is reading event presentation data off the EMPTY AtlasData collections.\n\
         Route it through `GraphService::scene_source(&data)` (a `&dyn SceneSource`) instead.\n\n{}",
        violations.join("\n\n")
    );
}

/// The scan must be able to FAIL. Without this, a typo in a matcher (or a
/// `code_of` that swallowed everything) would leave the law permanently,
/// invisibly green -- which is the precise failure mode this whole file
/// exists to answer. Each forbidden shape is checked against a sample that
/// must match AND a lookalike that must not.
#[test]
fn the_scan_actually_matches_the_regression_and_not_its_correct_replacement() {
    let laws = laws();
    let must_hit = [
        "    let e = data.events.iter().find(|e| e.id == id);",
        "    for p in data.places.iter() {",
        "    for n in data.narratives.iter() {",
        "    let e = data.event_by_id(&id)?;",
        "    let p = data.place_by_id(&id)?;",
        // Fix round 1 (I-3): the six EMPTY-DERIVED accessors, one sample each.
        "    let h = data.heading_for_verse(&sref);",
        "    for (a, b, c) in data.heading_anchor_collisions() {",
        "    let idx = data.timeline_position(&id)?;",
        "    let e = data.timeline_event_at(idx + 1);",
        "    let lit = sidecars.event_bearing_place_ids();",
        "    let n = atlas.total_events_for(&place.id);",
        "    prior.and_then(|pid| atlas_core::narrative::adjacent_event(&data, &pid))",
        "    adjacent_event(&*data, pid)",
    ];
    for sample in must_hit {
        let code = code_of(sample);
        assert!(laws.iter().any(|l| (l.hit)(code)), "no law matched a real regression sample: {sample}");
    }

    let must_not_hit = [
        // The CORRECT shapes, which must never trip the scan.
        "    let src = graph.scene_source(&data);",
        "    let e = src.event_by_id(&id)?;",
        "    for n in src.narrative_list() {",
        "    prior.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid))",
        "    let places = source.places();",
        // The SceneSource carries its OWN event_bearing_place_ids/
        // total_events_for -- those are the right calls and must stay silent.
        "    let lit = src.event_bearing_place_ids();",
        "    let n = source.total_events_for(&place.id);",
        // A one-letter receiver must not fire on the tail of a longer name.
        "    let e = loaded.event_by_id(&id)?;",
        "    let n = wrapped.total_events_for(&id);",
        // ...and the same forbidden text, but in a comment: prose about the
        // regression (this file, and handlers.rs, are both full of it) must
        // not fail the law.
        "    // all (it replaces `data.event_by_id(&id).label`, which the deleted",
        "    // AtlasData.events is empty -- data.events must not be read",
    ];
    for sample in must_not_hit {
        let code = code_of(sample);
        let matched: Vec<&str> = laws.iter().filter(|l| (l.hit)(code)).map(|l| l.name).collect();
        assert!(matched.is_empty(), "the scan fired on a CORRECT line ({matched:?}): {sample}");
    }
}
