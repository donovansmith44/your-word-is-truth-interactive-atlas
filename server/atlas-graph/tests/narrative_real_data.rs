//! These laws over `atlas_core` live here because only `atlas_etl::compile` can build a real
//! `AtlasData`, and Cargo cannot resolve that as a dev-dependency cycle from `atlas-core`: the
//! returned `atlas_core::data::AtlasData` would be two distinct types with the same name.

use atlas_core::data::AtlasData;
use atlas_core::narrative::global_timeline_position;

    fn load_real_compiled_data() -> AtlasData {
        static CACHED: std::sync::OnceLock<AtlasData> = std::sync::OnceLock::new();
        CACHED
            .get_or_init(|| {
                let data_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
                let raw_dir = data_dir.join("raw");
                let curated_dir = data_dir.join("curated");
                atlas_etl::compile::compile(&raw_dir, &curated_dir)
                    .expect("data/raw + data/curated must compile -- run `cargo run -p atlas-etl` from server/ first to verify")
                    .data
            })
            .clone()
    }

    #[test]
    fn global_timeline_real_compiled_data_has_well_over_450_dated_events() {
        let d = load_real_compiled_data();
        let dated = d.events.iter().filter(|e| e.kind == atlas_core::data::EventKind::Event).count();
        assert!(dated >= 450, "expected n>=450 dated events, got {dated}");
    }

    #[test]
    fn global_timeline_true_extremes_of_the_real_atlas() {
        let d = load_real_compiled_data();
        let creation = global_timeline_position(&d, "theo-1").expect("theo-1 'Creation of all things' is the atlas's true first dated event");
        assert!(creation.prior.is_none(), "the true first dated event of the whole atlas has no prior");

        let rome = global_timeline_position(&d, "pr_rome").expect("pr_rome 'Paul arrives at Rome' is a real dated event");
        assert!(rome.following.is_some(), "pr_rome is no longer the atlas's own last event -- real, later-calibrated Acts content now follows it");
        let imprisonment = global_timeline_position(&d, "theo-385").expect("theo-385 'Paul's First Roman imprisonment' is a dated event");
        assert!(imprisonment.following.is_none(), "theo-385 is now the atlas's true last dated event");
    }

    #[test]
    fn m_d1_the_three_remaining_duplicate_pairs_are_rectified_on_the_real_graph() {
        let d = load_real_compiled_data();

        assert!(d.event_by_id("theo-384").is_none(), "theo-384 'Paul arrives at Rome' (the Theographic freebie) must be merged away");
        assert!(d.event_by_id("pr_rome").is_some(), "pr_rome survives -- this atlas's own narrative-integrated identity");

        assert!(d.event_by_id("theo-337").is_none(), "theo-337 'First missionary journey begins' (the 5-verse prefix) must be merged away");
        assert!(d.event_by_id("theo-338").is_some(), "theo-338 'First Missionary Journey' (the 79-verse superset) survives");

        assert!(d.event_by_id("ret_jerusalem_altar").is_none(), "ret_jerusalem_altar (the bare `return`-narrative freebie) must be merged away");
        let altar = d.event_by_id("ezr_altar_and_foundation").expect("ezr_altar_and_foundation (the real curated container) survives");
        assert_eq!(altar.when.from_year, -536, "the survivor's own established date is untouched by the merge");

        let return_narrative = d.narratives.iter().find(|n| n.id == "return").expect("the `return` narrative must still exist");
        assert!(
            !return_narrative.legs.contains(&"ret_jerusalem_altar".to_string()),
            "the absorbed id must not linger in the return narrative's own leg list"
        );
        assert!(
            return_narrative.legs.contains(&"ezr_altar_and_foundation".to_string()),
            "the return narrative's own leg list must repoint to the survivor"
        );

        let paul_rome = d.narratives.iter().find(|n| n.id == "paul-rome-voyage").expect("the paul-rome-voyage narrative must still exist");
        assert!(paul_rome.legs.contains(&"pr_rome".to_string()), "paul-rome-voyage's own final leg stays pr_rome");
        assert_eq!(paul_rome.legs.len(), 9, "no leg lost or duplicated by the merge");
    }

    #[test]
    fn amendment_c_baptism_precedes_temptation_on_the_global_timeline() {
        let d = load_real_compiled_data();

        assert!(d.event_by_id("theo-267").is_none(), "the Theographic-scale Baptism duplicate must be merged away");
        assert!(d.event_by_id("theo-268").is_none(), "the Theographic-scale Temptation duplicate must be merged away");
        assert!(d.event_by_id("jm_jordan").is_some(), "exactly one Baptism event remains");
        assert!(d.event_by_id("rob_temptation").is_some(), "exactly one Temptation event remains");

        let baptism_idx = d.timeline_position("jm_jordan").expect("jm_jordan is a dated event");
        let temptation_idx = d.timeline_position("rob_temptation").expect("rob_temptation is a dated event");
        assert!(baptism_idx < temptation_idx, "Baptism must sort strictly before Temptation on the global timeline");

        let baptism_pos = global_timeline_position(&d, "jm_jordan").unwrap();
        assert_eq!(
            baptism_pos.following.as_ref().map(|e| e.id.as_str()),
            Some("rob_temptation"),
            "walking FOLLOWING from Baptism must reach Temptation directly"
        );
    }

    #[test]
    fn amendment_d_monotonicity_audit_reading_order_vs_global_timeline() {
        let d = load_real_compiled_data();

        let mut by_book: std::collections::HashMap<String, Vec<(String, u32)>> = std::collections::HashMap::new();
        for e in d.events.iter().filter(|e| e.kind == atlas_core::data::EventKind::Event) {
            let mut first_in_book: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            let mut note = |v: &str| {
                if let Ok(vid) = atlas_core::refs::VerseId::parse_canonical(v) {
                    let ordinal = vid.chapter as u32 * 1000 + vid.verse as u32;
                    let entry = first_in_book.entry(vid.book.code().to_string()).or_insert(ordinal);
                    if ordinal < *entry {
                        *entry = ordinal;
                    }
                }
            };
            for v in &e.verses {
                note(v);
            }
            for w in &e.witnesses {
                if let Some(vs) = w.translations.get(atlas_core::translation::DEFAULT_TRANSLATION) {
                    for v in vs {
                        note(v);
                    }
                }
            }
            for (book, ordinal) in first_in_book {
                by_book.entry(book).or_default().push((e.id.clone(), ordinal));
            }
        }

        let mut books: Vec<&String> = by_book.keys().collect();
        books.sort();

        let by_id: std::collections::HashMap<&str, &atlas_core::data::Event> = d.events.iter().map(|e| (e.id.as_str(), e)).collect();

        let is_justified = |a: &str, b: &str, book: &str| -> bool {
            let a_e = by_id.get(a);
            let b_e = by_id.get(b);
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if !atlas_core::event_merge::is_layer0(ae) && !atlas_core::event_merge::is_layer0(be) {
                    return true;
                }
            }
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if ae.when.from_year == be.when.from_year && ae.order_key == 0 && be.order_key == 0 {
                    return true;
                }
            }
            if let (Some(ae), Some(be)) = (a_e, b_e) {
                if atlas_core::event_merge::is_layer0(ae) != atlas_core::event_merge::is_layer0(be) {
                    return true;
                }
                if atlas_core::event_merge::is_layer0(ae) && atlas_core::event_merge::is_layer0(be) {
                    return true;
                }
            }
            if a == "theo-123" || b == "theo-123" {
                return true;
            }
            if book == "NEH" && (a == "ezr_list_of_returnees" || b == "ezr_list_of_returnees" || a == "neh_priests_levites_dedication" || b == "neh_priests_levites_dedication") {
                return true;
            }
            if book == "NUM" && (a == "ex_kadesh" || b == "ex_kadesh" || a == "ex_moab" || b == "ex_moab") {
                return true;
            }
            if book == "PSA" && ((a == "sam2_davids_song" && b == "1ch_davids_hymn_of_praise") || (a == "1ch_davids_hymn_of_praise" && b == "sam2_davids_song")) {
                return true;
            }
            false
        };

        let mut total_pairs = 0usize;
        let mut inverted = 0usize;
        let mut justified = 0usize;
        let mut unexplained: Vec<String> = Vec::new();
        for book in books {
            let mut entries = by_book[book].clone();
            entries.sort_by_key(|(_, ord)| *ord);
            for i in 0..entries.len() {
                for j in (i + 1)..entries.len() {
                    let (a_id, _) = &entries[i];
                    let (b_id, _) = &entries[j];
                    let (Some(a_pos), Some(b_pos)) = (d.timeline_position(a_id), d.timeline_position(b_id)) else { continue };
                    total_pairs += 1;
                    if a_pos > b_pos {
                        inverted += 1;
                        if is_justified(a_id, b_id, book) {
                            justified += 1;
                        } else {
                            unexplained.push(format!(
                                "[{book}] '{a_id}' reads before '{b_id}' but sorts AFTER it on the global timeline (positions {a_pos} vs {b_pos})"
                            ));
                        }
                    }
                }
            }
        }
        eprintln!("AMENDMENT-D SUMMARY: {inverted} inverted / {total_pairs} same-book pairs checked ({justified} justified, {} unexplained)", unexplained.len());
        assert!(
            unexplained.is_empty(),
            "AMENDMENT-D: {} unexplained monotonicity inversion(s) -- each needs a one-line justification (added to `is_justified` above) or a data fix:\n{}",
            unexplained.len(),
            unexplained.join("\n")
        );
    }

    #[test]
    fn fix_round_1_era_boundary_gate_passion_cluster_sorts_before_every_act_witnessed_event() {
        let d = load_real_compiled_data();

        let passion_cluster: Vec<&str> = d.events.iter().map(|e| e.id.as_str()).filter(|id| id.starts_with("pw_")).collect();
        assert!(!passion_cluster.is_empty(), "the real compiled data must have pw_* Passion-Week events to check against");
        let max_passion_idx = passion_cluster
            .iter()
            .map(|id| d.timeline_position(id).unwrap_or_else(|| panic!("{id} must be a dated event")))
            .max()
            .unwrap();

        let act_witnessed: Vec<&atlas_core::data::Event> = d
            .events
            .iter()
            .filter(|e| e.kind == atlas_core::data::EventKind::Event && !e.id.starts_with("pw_") && atlas_core::nt_calibration::first_verse_in_book(e, "ACT").is_some())
            .collect();
        assert!(act_witnessed.len() >= 50, "expected the real compiled data to carry well over 50 non-pw_ ACT-witnessed events (Acts 1 onward, post-calibration), got {}", act_witnessed.len());

        let mut violations: Vec<String> = Vec::new();
        for e in &act_witnessed {
            let idx = d.timeline_position(&e.id).unwrap();
            if idx <= max_passion_idx {
                violations.push(format!("'{}' ({}) sorts at timeline index {idx}, at or before the Passion cluster's own max index {max_passion_idx}", e.id, e.label));
            }
        }
        assert!(
            violations.is_empty(),
            "FIX ROUND 1 ERA-BOUNDARY GATE: {} ACT-witnessed event(s) sort at or before the end of the real Passion cluster (pw_jerusalem_entry..ascension) -- Pentecost/subsequent Acts must never precede the Crucifixion/Resurrection/Ascension:\n{}",
            violations.len(),
            violations.join("\n")
        );
    }

    #[test]
    fn fix_round_1_within_acts_section_events_follow_acts_chapter_order() {
        let d = load_real_compiled_data();

        let mut acts_section_events: Vec<(&str, (u16, u16))> = d
            .events
            .iter()
            .filter(|e| e.kind == atlas_core::data::EventKind::Event && e.acts_section.is_some())
            .filter_map(|e| atlas_core::nt_calibration::first_verse_in_book(e, "ACT").map(|cv| (e.id.as_str(), cv)))
            .collect();
        assert!(acts_section_events.len() >= 30, "expected ~33 acts_section events, got {}", acts_section_events.len());
        acts_section_events.sort_by_key(|(_, cv)| *cv);

        let mut violations: Vec<String> = Vec::new();
        for pair in acts_section_events.windows(2) {
            let (a_id, a_cv) = pair[0];
            let (b_id, b_cv) = pair[1];
            let a_idx = d.timeline_position(a_id).unwrap();
            let b_idx = d.timeline_position(b_id).unwrap();
            if a_idx > b_idx {
                violations.push(format!("'{a_id}' (ACT {a_cv:?}) reads before '{b_id}' (ACT {b_cv:?}) but sorts AFTER it on the global timeline"));
            }
        }
        assert!(violations.is_empty(), "FIX ROUND 1: {} acts_section event(s) out of Acts chapter order on the global timeline:\n{}", violations.len(), violations.join("\n"));
    }

    #[test]
    fn e1_every_bound_anchor_equals_its_compiled_events_own_from_year() {
        let d = load_real_compiled_data();
        let bound: Vec<&atlas_core::data::ChronologyAnchor> = d.chronology_anchors.iter().filter(|a| a.event_id.is_some()).collect();
        assert!(bound.len() >= 15, "expected the real curated anchor table to bind well over 15 rows to real events, got {}", bound.len());

        let (violations, deferred) = atlas_core::chronology::anchor_equality_check(&d.chronology_anchors, &d.events);

        let deferred_strings: Vec<String> =
            deferred.iter().map(|r| format!("'{}' (canonical {}, event '{}' currently {}, DEFERRED to HOTFIX-7)", r.anchor_id, r.canonical_year, r.event_id, r.shipped_value)).collect();
        assert_eq!(
            deferred.len(),
            4,
            "expected exactly 4 typed deferrals (jerusalem-falls/cyrus-decree/temple-finished/ezra-returns) visible here, got {}: {:?} -- this count is deliberate, not a floor; update it together with ANCHOR_DEFERRALS, never silently",
            deferred.len(),
            deferred_strings
        );

        let violation_strings: Vec<String> = violations
            .iter()
            .map(|v| {
                if v.is_stale_deferral {
                    format!(
                        "anchor '{}': STALE DEFERRAL -- ANCHOR_DEFERRALS records shipped_value {}, but compiled event '{}''s own from_year is now {} (re-dated without updating/removing the deferral entry?)",
                        v.anchor_id, v.table_year, v.event_id, v.event_year
                    )
                } else {
                    format!("anchor '{}': table year {} != compiled event '{}''s own from_year {}", v.anchor_id, v.table_year, v.event_id, v.event_year)
                }
            })
            .collect();
        assert!(
            violation_strings.is_empty(),
            "E1 anchor-equality violated for {} row(s) (this list never includes the {} typed deferrals reported separately above -- {:?}):\n{}",
            violation_strings.len(),
            deferred.len(),
            deferred_strings,
            violation_strings.join("\n")
        );
    }

    #[test]
    fn e2_every_compiled_dated_event_adheres_to_its_witness_books_own_narration_window() {
        let d = load_real_compiled_data();
        assert!(!d.book_narration_windows.is_empty(), "the real compiled data must carry real narration windows");
        let violations = atlas_core::chronology::window_violations(&d.events, &d.book_narration_windows);
        assert!(
            violations.is_empty(),
            "E2 window-adherence violated for {} event(s):\n{}",
            violations.len(),
            violations.iter().map(|v| format!("  - '{}' ({:?}): {}..{} outside '{}''s own {}..{}", v.event_id, v.label, v.year.0, v.year.1, v.book, v.window.0, v.window.1)).collect::<Vec<_>>().join("\n")
        );
    }

    #[test]
    fn e3_bound_anchors_sorted_by_table_year_are_monotone_on_the_global_timeline() {
        let d = load_real_compiled_data();
        let mut bound: Vec<&atlas_core::data::ChronologyAnchor> =
            d.chronology_anchors.iter().filter(|a| a.event_id.is_some() && atlas_core::chronology::anchor_deferral(&a.id).is_none()).collect();
        assert!(bound.len() >= 12, "expected well over 12 non-deferred bound anchors, got {}", bound.len());
        bound.sort_by_key(|a| a.year);

        let positions: Vec<(&str, i32, usize)> = bound
            .iter()
            .map(|a| {
                let eid = a.event_id.as_deref().unwrap();
                let pos = d.timeline_position(eid).unwrap_or_else(|| panic!("anchor '{}''s own bound event '{eid}' must be a dated event", a.id));
                (a.id.as_str(), a.year, pos)
            })
            .collect();

        let mut violations: Vec<String> = Vec::new();
        for pair in positions.windows(2) {
            let (a_id, a_year, a_pos) = pair[0];
            let (b_id, b_year, b_pos) = pair[1];
            let ok = if a_year == b_year { a_pos <= b_pos } else { a_pos < b_pos };
            if !ok {
                violations.push(format!("'{a_id}' (table year {a_year}, timeline position {a_pos}) does not sort {} '{b_id}' (table year {b_year}, timeline position {b_pos})", if a_year == b_year { "at-or-before" } else { "strictly before" }));
            }
        }
        assert!(violations.is_empty(), "E3 canonical-order violated for {} adjacent pair(s):\n{}", violations.len(), violations.join("\n"));
    }

    #[test]
    fn e4_dated_events_agree_with_era_boundary_anchors_on_the_global_timeline() {
        let d = load_real_compiled_data();
        let boundary_count = d.chronology_anchors.iter().filter(|a| a.era_boundary).count();
        assert!(boundary_count >= 6, "expected the real curated table to carry well over 6 era_boundary anchors, got {boundary_count}");

        let violations = atlas_core::chronology::era_boundary_violations(&d);
        assert!(
            violations.is_empty(),
            "E4 era-partition violated for {} event(s):\n{}",
            violations.len(),
            violations.iter().map(|v| format!("  - '{}' ({:?}) sorts on the wrong side of boundary '{}' (year {}) -- expected {}", v.event_id, v.label, v.boundary_id, v.boundary_year, v.side)).collect::<Vec<_>>().join("\n")
        );
    }

    fn anchor_year(d: &AtlasData, id: &str) -> atlas_core::time::Year {
        d.chronology_anchors.iter().find(|a| a.id == id).unwrap_or_else(|| panic!("no chronology anchor with id '{id}'")).year
    }

    #[test]
    fn e5_solomon_gibeons_neighbors_are_solomon_era_never_saul_persecution_era() {
        let d = load_real_compiled_data();
        let pos = global_timeline_position(&d, "1ki_solomon_gibeon").expect("1ki_solomon_gibeon is a dated event");

        let prior = pos.prior.as_ref().expect("1ki_solomon_gibeon has a PRIOR neighbor");
        let following = pos.following.as_ref().expect("1ki_solomon_gibeon has a FOLLOWING neighbor");

        assert!(!prior.id.starts_with("df_"), "RED-CASE REGRESSION: PRIOR '{}' is a Saul-persecution df_* event -- the owner's own exact bug ('this is a lie')", prior.id);
        assert!(!following.id.starts_with("df_"), "RED-CASE REGRESSION: FOLLOWING '{}' is a Saul-persecution df_* event -- the owner's own exact bug", following.id);

        assert_eq!(prior.id, "1ki_davids_charge", "PRIOR must be David's own final charge to Solomon (-1015), Solomon-era");
        assert_eq!(following.id, "1ki_hiram_temple_prep", "FOLLOWING must be Solomon's own temple preparations with Hiram (-1013), Solomon-era");
        let prior_year = d.event_by_id(&prior.id).unwrap().when.from_year;
        let following_year = d.event_by_id(&following.id).unwrap().when.from_year;
        let solomon_crowned = anchor_year(&d, "solomon-crowned");
        let tolerance = 5;
        assert!(
            (solomon_crowned - tolerance..=solomon_crowned + tolerance).contains(&prior_year),
            "PRIOR '{}' ({prior_year}) must fall within {tolerance} years of solomon-crowned ({solomon_crowned}), not the Saul-persecution era",
            prior.id
        );
        assert!(
            (solomon_crowned - tolerance..=solomon_crowned + tolerance).contains(&following_year),
            "FOLLOWING '{}' ({following_year}) must fall within {tolerance} years of solomon-crowned ({solomon_crowned}), not the Saul-persecution era",
            following.id
        );
    }

    #[test]
    fn e5_df_ramahs_neighbors_are_saul_persecution_era_never_solomon_era() {
        let d = load_real_compiled_data();
        let pos = global_timeline_position(&d, "df_ramah").expect("df_ramah is a dated event");

        let prior = pos.prior.as_ref().expect("df_ramah has a PRIOR neighbor");
        let following = pos.following.as_ref().expect("df_ramah has a FOLLOWING neighbor");

        assert_ne!(prior.id, "1ki_solomon_gibeon", "df_ramah's own PRIOR must never be the Solomon-era Gibeon dream");
        assert_ne!(following.id, "1ki_solomon_gibeon", "df_ramah's own FOLLOWING must never be the Solomon-era Gibeon dream");

        assert_eq!(prior.id, "theo-157", "PRIOR must be David killing Goliath (-1067), immediately preceding Saul's own persecution");
        assert_eq!(following.id, "df_nob", "FOLLOWING must be the chain's own next leg, df_nob (-1061)");
        let prior_year = d.event_by_id(&prior.id).unwrap().when.from_year;
        let following_year = d.event_by_id(&following.id).unwrap().when.from_year;
        let david_hebron = anchor_year(&d, "david-hebron");
        assert!(prior_year <= david_hebron, "PRIOR '{}' ({prior_year}) must be at or before the Saul-persecution/united-monarchy transition (david-hebron, {david_hebron})", prior.id);
        assert!(following_year <= david_hebron, "FOLLOWING '{}' ({following_year}) must be Saul-persecution-era, not Solomon-era (at or before david-hebron, {david_hebron})", following.id);
    }
