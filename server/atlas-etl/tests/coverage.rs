use std::collections::HashSet;

use atlas_core::data::{Canon, Event, EventWitness};
use atlas_core::event_merge::{cross_book_duplicate_candidate, title_jaccard, EVENT_DISTINCT_PAIRS};

fn robertson_table() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for n in 1..=184 {
        if n == 128 {
            out.push("128a".to_string());
            out.push("128b".to_string());
        } else {
            out.push(n.to_string());
        }
    }
    out
}

fn subsumed() -> HashSet<&'static str> {
    SUBSUMPTION_CLAIMS.iter().map(|c| c.section).collect()
}

struct SubsumptionClaim {
    section: &'static str,
    subsuming_event_id: &'static str,
    refs: &'static [(&'static str, u16, u16, u16)],
}

const SUBSUMPTION_CLAIMS: &[SubsumptionClaim] = &[
    SubsumptionClaim {
        section: "153",
        subsuming_event_id: "pw_gethsemane",
        refs: &[("MRK", 14, 43, 52), ("MAT", 26, 47, 56), ("LUK", 22, 47, 53), ("JHN", 18, 2, 12)],
    },
    SubsumptionClaim {
        section: "165",
        subsuming_event_id: "pw_golgotha",
        refs: &[("MRK", 15, 33, 37), ("MAT", 27, 45, 50), ("LUK", 23, 44, 46), ("JHN", 19, 28, 30)],
    },
    SubsumptionClaim { section: "169", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MRK", 16, 1, 1), ("MAT", 28, 1, 1)] },
    SubsumptionClaim { section: "170", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MAT", 28, 2, 4)] },
    SubsumptionClaim { section: "172", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("LUK", 24, 9, 12), ("JHN", 20, 2, 10)] },
    SubsumptionClaim { section: "174", subsuming_event_id: "pw_jerusalem_resurrection", refs: &[("MAT", 28, 9, 10)] },
    SubsumptionClaim { section: "177", subsuming_event_id: "pw_emmaus", refs: &[("LUK", 24, 33, 35)] },
];

fn check_subsumption_content(claims: &[SubsumptionClaim], events: &[Event], witnesses: &[(String, EventWitness)]) -> Vec<String> {
    let mut errors = Vec::new();
    for claim in claims {
        let Some(event) = events.iter().find(|e| e.id == claim.subsuming_event_id) else {
            errors.push(format!(
                "subsumption claim for §{}: claimed subsuming event '{}' does not exist in the real curated data",
                claim.section, claim.subsuming_event_id
            ));
            continue;
        };
        let mut covered: HashSet<&str> = event.verses.iter().map(String::as_str).collect();
        for (eid, w) in witnesses {
            if eid == claim.subsuming_event_id {
                if let Some(vs) = w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION) {
                    covered.extend(vs.iter().map(String::as_str));
                }
            }
        }
        if claim.refs.is_empty() {
            errors.push(format!(
                "subsumption claim for §{}: empty refs list -- a claim that checks no verses is vacuous",
                claim.section
            ));
        }
        for (book, chapter, from_verse, to_verse) in claim.refs {
            if from_verse > to_verse {
                errors.push(format!(
                    "subsumption claim for §{}: inverted range {book}.{chapter}.{from_verse}-{to_verse} checks no verses (vacuous)",
                    claim.section
                ));
                continue;
            }
            for v in *from_verse..=*to_verse {
                let vref = format!("{book}.{chapter}.{v}");
                if !covered.contains(vref.as_str()) {
                    errors.push(format!(
                        "subsumption claim for §{} (claimed subsumed by '{}'): '{vref}' is part of Robertson's own §{} citation but is NOT covered by '{}''s own actual verse/witness ranges -- the subsumption claim is false or stale",
                        claim.section, claim.subsuming_event_id, claim.section, claim.subsuming_event_id
                    ));
                }
            }
        }
    }
    errors
}

fn honestly_omitted() -> HashSet<&'static str> {
    ["182"].into_iter().collect()
}

fn read_curated(name: &str) -> String {
    let path = format!("{}/../../data/curated/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

fn extract_section_tokens(text: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("robertson_section") {
            continue;
        }
        let mut chars = trimmed.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '§' {
                continue;
            }
            let mut token = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() {
                    token.push(d);
                    chars.next();
                } else {
                    break;
                }
            }
            if token.is_empty() {
                continue;
            }
            if let Some(&letter) = chars.peek() {
                if letter.is_ascii_lowercase() {
                    token.push(letter);
                    chars.next();
                }
            }
            out.insert(token);
        }
    }
    out
}

#[test]
fn every_robertson_section_is_accounted_for_in_the_real_curated_data() {
    let events_extra = read_curated("events-extra.toml");
    let event_witnesses = read_curated("event-witnesses.toml");
    let mut present = extract_section_tokens(&events_extra);
    present.extend(extract_section_tokens(&event_witnesses));

    let subsumed = subsumed();
    let omitted = honestly_omitted();
    let mut gaps: Vec<String> = Vec::new();

    for section in robertson_table() {
        let accounted_for =
            present.contains(&section) || subsumed.contains(section.as_str()) || omitted.contains(section.as_str());
        if !accounted_for {
            gaps.push(section);
        }
    }

    assert!(
        gaps.is_empty(),
        "Robertson section(s) with no curated container, no disclosed subsumption, and no disclosed honest omission -- a real, uncaught coverage gap: {gaps:?}"
    );

    let table: HashSet<String> = robertson_table().into_iter().collect();
    for s in subsumed.iter().chain(omitted.iter()) {
        assert!(table.contains(*s), "'{s}' in this test's own exception lists is not a real Robertson section number 1-184 (typo?)");
    }

    for s in subsumed.iter().chain(omitted.iter()) {
        assert!(
            !present.contains(*s),
            "'{s}' is listed as subsumed/omitted in this test's own exception lists, but ALSO appears as its own literal robertson_section citation in the curated data -- the exception entry is stale, remove it"
        );
    }
}

#[test]
fn subsumption_content_check_catches_a_genuinely_uncovered_citation() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56)] }];
    let events = vec![Event { id: "pw_gethsemane".into(), verses: vec!["MAT.26.36".into()], ..Default::default() }];
    let errors = check_subsumption_content(claims, &events, &[]);
    assert_eq!(errors.len(), 10, "{errors:?}");
    assert!(errors[0].contains("MAT.26.47"), "{}", errors[0]);
}

#[test]
fn subsumption_content_check_catches_a_missing_subsuming_event() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56)] }];
    let errors = check_subsumption_content(claims, &[], &[]);
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(errors[0].contains("does not exist"), "{}", errors[0]);
}

#[test]
fn subsumption_content_check_passes_when_coverage_is_real() {
    let claims = &[SubsumptionClaim { section: "153", subsuming_event_id: "pw_gethsemane", refs: &[("MAT", 26, 47, 56), ("MRK", 14, 43, 52)] }];
    let events = vec![Event { id: "pw_gethsemane".into(), verses: (47..=56).map(|v| format!("MAT.26.{v}")).collect(), ..Default::default() }];
    let witnesses = vec![(
        "pw_gethsemane".to_string(),
        EventWitness {
            book: "MRK".into(),
            translations: std::collections::HashMap::from([(
                atlas_core::translation::DEFAULT_TRANSLATION.to_string(),
                (43..=52).map(|v| format!("MRK.14.{v}")).collect(),
            )]),
            ref_note: None,
            robertson_section: None,
        },
    )];
    let errors = check_subsumption_content(claims, &events, &witnesses);
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn every_subsumption_claim_is_content_verified_against_real_curated_data() {
    let events_extra_toml = read_curated("events-extra.toml");
    let witnesses_toml = read_curated("event-witnesses.toml");
    let events = atlas_etl::curated::parse_events_extra(&events_extra_toml).expect("events-extra.toml must parse");
    let witnesses = atlas_etl::curated::parse_event_witnesses(&witnesses_toml).expect("event-witnesses.toml must parse");

    let errors = check_subsumption_content(SUBSUMPTION_CLAIMS, &events, &witnesses);
    assert!(errors.is_empty(), "subsumption content check found {} problem(s):\n{}", errors.len(), errors.join("\n"));
}

#[test]
fn every_authored_acts_section_is_present_and_distinct() {
    let acts_sections = read_curated("acts-sections.toml");
    let event_ids: Vec<&str> = acts_sections
        .lines()
        .filter(|l| l.trim_start().starts_with("event_id"))
        .filter_map(|l| l.split('"').nth(1))
        .collect();
    assert_eq!(event_ids.len(), 33, "expected exactly 33 authored Acts SS1-12 sections (theo-304 through theo-336); {:?}", event_ids);
    let distinct: HashSet<&str> = event_ids.iter().copied().collect();
    assert_eq!(distinct.len(), 33, "every authored Acts section's own event_id must be distinct -- a duplicate row is a curation error");
    for expected in 304..=336 {
        let id = format!("theo-{expected}");
        assert!(event_ids.contains(&id.as_str()), "expected theo-{expected} among the authored Acts sections, not found");
    }
}

fn real_compiled_data() -> atlas_core::data::AtlasData {
    static CACHED: std::sync::OnceLock<atlas_core::data::AtlasData> = std::sync::OnceLock::new();
    CACHED
        .get_or_init(|| {
            let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
            atlas_etl::compile::compile(&data_dir.join("raw"), &data_dir.join("curated"))
                .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                .data
        })
        .clone()
}

fn all_verses_for_book(canon: &Canon, code: &str) -> Vec<String> {
    let book = canon.books.iter().find(|b| b.code == code).unwrap_or_else(|| panic!("'{code}' is not a real canon book code"));
    let mut out = Vec::new();
    for (idx, &count) in book.chapters.iter().enumerate() {
        let chapter = idx + 1;
        for verse in 1..=count {
            out.push(format!("{code}.{chapter}.{verse}"));
        }
    }
    out
}

fn covered_verses(events: &[Event]) -> HashSet<String> {
    let mut out = HashSet::new();
    for e in events {
        out.extend(e.verses.iter().cloned());
        for w in &e.witnesses {
            if let Some(vs) = w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION) {
                out.extend(vs.iter().cloned());
            }
        }
    }
    out
}

#[test]
fn every_declared_book_is_fully_covered_by_the_real_compiled_data() {
    let manifest_toml = read_curated("coverage-manifest.toml");
    let declared = atlas_etl::curated::parse_coverage_manifest(&manifest_toml).expect("coverage-manifest.toml must parse");
    assert!(!declared.is_empty(), "coverage-manifest.toml's own declared list must not be empty");

    let real = real_compiled_data();
    let canon = real.canon.clone();
    let events = real.events.clone();
    let covered = covered_verses(&events);

    let mut failures: Vec<String> = Vec::new();
    for code in &declared {
        let universe = all_verses_for_book(&canon, code);
        let missing: Vec<&String> = universe.iter().filter(|v| !covered.contains(v.as_str())).collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{code}: {} of {} verses are in NO container (first gap: {})",
                missing.len(),
                universe.len(),
                missing[0]
            ));
        }
    }
    assert!(failures.is_empty(), "declared-but-not-fully-covered book(s):\n{}", failures.join("\n"));

    let distinct: HashSet<&String> = declared.iter().collect();
    assert_eq!(distinct.len(), declared.len(), "coverage-manifest.toml's own declared list has a duplicate entry");
    let real_codes: HashSet<&str> = canon.books.iter().map(|b| b.code.as_str()).collect();
    for code in &declared {
        assert!(real_codes.contains(code.as_str()), "'{code}' in coverage-manifest.toml is not a real canon book code (typo?)");
    }
}

#[test]
fn declared_books_never_shrink_below_the_established_floor() {
    const FLOOR: &[&str] = &[
        "MAT", "MRK", "LUK", "JHN", "ACT", "GEN", "EXO", "LEV", "RUT", "NUM", "DEU", "JOS", "JDG", "1SA", "2SA",
        "1KI", "2KI", "1CH", "2CH", "EZR", "NEH", "EST", "JOB", "PSA", "PRO", "ECC", "SNG", "ISA", "JER", "LAM",
        "EZK", "DAN", "HOS", "JOL", "AMO", "OBA", "JON", "MIC", "NAM", "HAB", "ZEP", "HAG", "ZEC", "MAL", "ROM",
        "1CO", "2CO", "GAL", "EPH", "PHP", "COL", "1TH", "2TH", "1TI", "2TI", "TIT", "PHM", "HEB", "JAS", "1PE",
        "2PE", "1JN", "2JN", "3JN", "JUD", "REV",
    ];

    let manifest_toml = read_curated("coverage-manifest.toml");
    let declared: HashSet<String> =
        atlas_etl::curated::parse_coverage_manifest(&manifest_toml).expect("coverage-manifest.toml must parse").into_iter().collect();

    for code in FLOOR {
        assert!(declared.contains(*code), "'{code}' must stay declared in coverage-manifest.toml -- the declared list may only ever grow, never shrink");
    }
}

#[test]
fn no_duplicate_fall_of_jerusalem_or_gedaliah_mizpah_nodes_in_the_real_compiled_timeline() {
    let events = real_compiled_data().events;
    let by_id: HashSet<&str> = events.iter().map(|e| e.id.as_str()).collect();

    assert!(
        !by_id.contains("exl_mizpah"),
        "exl_mizpah must stay retired -- its own occurrence is now dated by jer_jeremiah_stays_with_gedaliah/jer_the_assassination_of_gedaliah"
    );
    assert!(
        !by_id.contains("jer_the_fall_of_jerusalem_retold"),
        "jer_the_fall_of_jerusalem_retold must stay retired/renamed -- its own duplicate span is now a witness on exl_jerusalem"
    );

    for id in [
        "exl_jerusalem",
        "jer_jeremiah_stays_with_gedaliah",
        "jer_the_assassination_of_gedaliah",
        "jer_jeremiahs_release_and_the_word_to_ebedmelech",
    ] {
        let e = events.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("'{id}' must exist in the real compiled event set"));
        assert_eq!(e.kind, "event", "'{id}' must be a real dated EVENT-kind node");
    }
    for id in ["exl_jerusalem", "jer_jeremiah_stays_with_gedaliah", "jer_the_assassination_of_gedaliah"] {
        let e = events.iter().find(|e| e.id == id).unwrap();
        assert!(!e.witnesses.is_empty(), "'{id}' must carry >=1 witness row after the fix round 1 reconciliation");
    }

    let dated: Vec<&Event> = events.iter().filter(|e| e.kind == "event").collect();
    let mut unlisted: Vec<String> = Vec::new();
    for (i, a) in dated.iter().enumerate() {
        for b in dated[i + 1..].iter() {
            if !cross_book_duplicate_candidate(a, b) {
                continue;
            }
            let listed = EVENT_DISTINCT_PAIRS.iter().any(|p| (p.a == a.id && p.b == b.id) || (p.a == b.id && p.b == a.id));
            if listed {
                continue;
            }
            unlisted.push(format!(
                "'{}' ({:?}) <-> '{}' ({:?}): title jaccard {:.3} -- candidate duplicate in the COMPILED timeline, not documented in EVENT_DISTINCT_PAIRS",
                a.id,
                a.label,
                b.id,
                b.label,
                title_jaccard(&a.label, &b.label)
            ));
        }
    }
    unlisted.sort();
    assert!(unlisted.is_empty(), "duplicate-occurrence node(s) found in the real compiled global timeline:\n{}", unlisted.join("\n"));
}

#[test]
fn chron1_coverage_restorations_actually_reach_witnesses_for() {
    let real = real_compiled_data();

    let cases: &[(&str, &str)] = &[
        ("mat_leper_healed", "MAT.8.1"),
        ("rob_leper_healed", "MRK.1.40"),
        ("rob_leper_healed", "LUK.5.12"),
        ("ab_egypt", "GEN.12.11"),
        ("je_egypt_ruler", "GEN.41.37"),
        ("jm_caesarea_philippi", "MRK.8.27"),
        ("jm_caesarea_philippi", "LUK.9.18"),
    ];

    for (event_id, vref) in cases {
        let event = real.events.iter().find(|e| e.id == *event_id).unwrap_or_else(|| panic!("'{event_id}' must exist in the real compiled event set"));
        let resolved = atlas_core::scene::witnesses_for(event);
        let covered: HashSet<&str> = resolved.iter().flat_map(|w| w.verse_groups.iter().flat_map(|g| g.verses.iter().map(String::as_str))).collect();
        assert!(
            covered.contains(vref),
            "'{event_id}'s own witnesses_for() must cover {vref} -- a widening that only ever touches the top-level `verses` field is INERT once explicit witness rows exist (S-1's own root cause); covered set was: {covered:?}"
        );
    }
}

#[test]
fn chron1_jm_sychar_widening_reaches_the_source_verses_field() {
    let real = real_compiled_data();
    let event = real.events.iter().find(|e| e.id == "jm_sychar").expect("jm_sychar must exist in the real compiled event set");
    assert!(
        event.verses.contains(&"JHN.4.27".to_string()),
        "jm_sychar's own top-level `verses` must contain JHN.4.27 (the widened JHN.4.4-42 range) -- got {:?}",
        event.verses
    );
    assert!(
        event.witnesses.is_empty(),
        "jm_sychar must still carry NO explicit witness rows -- if this ever changes, witnesses_for's own synthesis path (which reads top-level `verses`) stops applying to it, and this test's own reasoning (and chron1_coverage_restorations_actually_reach_witnesses_for's own comment) needs revisiting"
    );
}

#[test]
fn every_canonical_book_is_declared_and_the_whole_kjv_is_fully_covered() {
    let real = real_compiled_data();
    let canon = real.canon.clone();
    let events = real.events.clone();
    let covered = covered_verses(&events);

    let manifest_toml = read_curated("coverage-manifest.toml");
    let declared: HashSet<String> =
        atlas_etl::curated::parse_coverage_manifest(&manifest_toml).expect("coverage-manifest.toml must parse").into_iter().collect();
    let all_canon_codes: HashSet<String> = canon.books.iter().map(|b| b.code.clone()).collect();

    let mut undeclared: Vec<&String> = all_canon_codes.difference(&declared).collect();
    undeclared.sort();
    assert!(
        undeclared.is_empty(),
        "THE COMPLETION MILESTONE is not yet reached: {} canonical book(s) are not declared in coverage-manifest.toml: {:?}",
        undeclared.len(),
        undeclared
    );
    let mut stray: Vec<&String> = declared.difference(&all_canon_codes).collect();
    stray.sort();
    assert!(stray.is_empty(), "coverage-manifest.toml declares {} code(s) that are not real canon book codes (typo?): {:?}", stray.len(), stray);
    assert_eq!(all_canon_codes.len(), 66, "expected exactly 66 real canon book codes, found {} -- canon.json itself changed shape", all_canon_codes.len());
    assert_eq!(declared.len(), 66, "expected exactly 66 declared books at the completion milestone, found {}", declared.len());

    let mut failures: Vec<String> = Vec::new();
    let mut total_verses = 0usize;
    for book in &canon.books {
        let universe = all_verses_for_book(&canon, &book.code);
        total_verses += universe.len();
        let missing: Vec<&String> = universe.iter().filter(|v| !covered.contains(v.as_str())).collect();
        if !missing.is_empty() {
            failures.push(format!(
                "{}: {} of {} verses are in NO container (first gap: {})",
                book.code,
                missing.len(),
                universe.len(),
                missing[0]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "THE COMPLETION MILESTONE is not yet reached -- {} book(s) have a real coverage gap in the whole compiled KJV inventory:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(total_verses, 31102, "expected the compiled KJV's own real total (31,102 verses per batch-w-brief.md's own Scale protocol), found {total_verses}");
}
