use std::path::{Path, PathBuf};

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
            name: r"\.event_by_id\( / \.place_by_id\( on an AtlasData-named receiver",
            why: "AtlasData::event_by_id/place_by_id look into the EMPTY vecs -- call them on the SceneSource",
            hit: |code| {
                RECEIVERS.iter().any(|recv| {
                    [".event_by_id(", ".place_by_id("].iter().any(|m| contains_call(code, recv, m))
                })
            },
        },
        Law {
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

const ALLOWLIST: &[(&str, u32, &str)] = &[];

const RECEIVERS: [&str; 4] = ["data", "atlas", "sidecars", "d"];

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
    rs_files(&root.join("atlas-contract/src"), &mut out);
    rs_files(&root.join("atlas-cli/src"), &mut out);
    out.sort();
    out
}

#[test]
fn no_serving_source_reads_event_display_data_through_atlas_data() {
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

#[test]
fn the_scan_actually_matches_the_regression_and_not_its_correct_replacement() {
    let laws = laws();
    let must_hit = [
        "    let e = data.events.iter().find(|e| e.id == id);",
        "    for p in data.places.iter() {",
        "    for n in data.narratives.iter() {",
        "    let e = data.event_by_id(&id)?;",
        "    let p = data.place_by_id(&id)?;",
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
        "    let src = graph.scene_source(&data);",
        "    let e = src.event_by_id(&id)?;",
        "    for n in src.narrative_list() {",
        "    prior.and_then(|pid| atlas_core::narrative::adjacent_event(src, &pid))",
        "    let places = source.places();",
        "    let lit = src.event_bearing_place_ids();",
        "    let n = source.total_events_for(&place.id);",
        "    let e = loaded.event_by_id(&id)?;",
        "    let n = wrapped.total_events_for(&id);",
        "    // all (it replaces `data.event_by_id(&id).label`, which the deleted",
        "    // AtlasData.events is empty -- data.events must not be read",
    ];
    for sample in must_not_hit {
        let code = code_of(sample);
        let matched: Vec<&str> = laws.iter().filter(|l| (l.hit)(code)).map(|l| l.name).collect();
        assert!(matched.is_empty(), "the scan fired on a CORRECT line ({matched:?}): {sample}");
    }
}
