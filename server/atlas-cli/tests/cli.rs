use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_bibex")).args(args).output().expect("bibex must run")
}

fn run_with_data_dir(args: &[&str]) -> Output {
    let dd = data_dir();
    let dd_str = dd.to_str().expect("data dir path must be valid UTF-8");
    let mut full = vec!["--data-dir", dd_str];
    full.extend_from_slice(args);
    run(&full)
}

fn run_with_data_dir_at(dir: &Path, args: &[&str]) -> Output {
    let dd_str = dir.to_str().expect("data dir path must be valid UTF-8");
    let mut full = vec!["--data-dir", dd_str];
    full.extend_from_slice(args);
    run(&full)
}
fn run_json_with_data_dir_at(dir: &Path, args: &[&str]) -> (Output, Option<serde_json::Value>) {
    let dd_str = dir.to_str().expect("data dir path must be valid UTF-8");
    let mut full = vec!["--data-dir", dd_str, "--json"];
    full.extend_from_slice(args);
    let o = run(&full);
    let value = if o.status.success() { Some(serde_json::from_str(&stdout(&o)).expect("--json happy path stdout must be valid JSON")) } else { None };
    (o, value)
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("stdout must be valid UTF-8")
}
fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).expect("stderr must be valid UTF-8")
}

fn run_json(args: &[&str]) -> (Output, Option<serde_json::Value>) {
    let dd = data_dir();
    let dd_str = dd.to_str().expect("data dir path must be valid UTF-8");
    let mut full = vec!["--data-dir", dd_str, "--json"];
    full.extend_from_slice(args);
    let o = run(&full);
    let value = if o.status.success() { Some(serde_json::from_str(&stdout(&o)).unwrap_or_else(|e| panic!("--json happy path stdout must be valid JSON: {e}\nstdout: {}", stdout(&o)))) } else { None };
    (o, value)
}

#[test]
fn verse_happy_path_shows_text_and_attached_sections() {
    let o = run_with_data_dir(&["verse", "GEN.1.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("GEN.1.1"), "out: {out}");
    assert!(out.contains("In the beginning God created the heaven and the earth."), "out: {out}");
    assert!(out.contains("Places:"), "out: {out}");
    assert!(out.contains("Persons:"), "out: {out}");
    assert!(out.contains("Events:"), "out: {out}");
    assert!(out.contains("Passages:"), "out: {out}");
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn verse_psa_119_105_shows_nun_under_passages_not_events() {
    let o = run_with_data_dir(&["verse", "PSA.119.105"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    let passages_line = out.lines().find(|l| l.starts_with("Passages:")).expect("a Passages: line");
    assert!(passages_line.contains("Psalm 119: NUN"), "out: {out}");
    let events_line = out.lines().find(|l| l.starts_with("Events:")).expect("an Events: line");
    assert!(!events_line.contains("Psalm 119: NUN"), "out: {out}");
}

#[test]
fn verse_gal_1_8_shows_astonishment_pericope_under_passages_not_events() {
    let o = run_with_data_dir(&["verse", "GAL.1.8"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    let passages_line = out.lines().find(|l| l.starts_with("Passages:")).expect("a Passages: line");
    assert!(passages_line.contains("Astonishment: no other gospel; let him be accursed"), "out: {out}");
    let events_line = out.lines().find(|l| l.starts_with("Events:")).expect("an Events: line");
    assert!(!events_line.contains("Astonishment"), "out: {out}");
}

#[test]
fn verse_red_letter_span_is_bracketed_inline() {
    let o = run_with_data_dir(&["verse", "MAT.4.19"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains('[') && out.contains(']'), "expected a bracketed red-letter span, got: {out}");
    assert!(out.contains("Follow me"), "out: {out}");
}

#[test]
fn verse_accepts_the_concord_grammar() {
    let o = run_with_data_dir(&["verse", "BoC 7.2.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("BoC 7.2.1"), "out: {out}");
    assert!(out.contains("not tracked for the Book of Concord"), "out: {out}");
}

#[test]
fn chapter_happy_path_lists_every_verse() {
    let o = run_with_data_dir(&["chapter", "GEN.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("GEN.1.1"), "out: {out}");
    assert!(out.contains("GEN.1.31"), "out: {out}");
    assert_eq!(out.lines().count(), 31, "Genesis 1 has exactly 31 verses");
}

#[test]
fn node_happy_path_shows_record_and_edge_summary() {
    let o = run_with_data_dir(&["node", "Event:ab_ur"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("id:         Event:ab_ur"), "out: {out}");
    assert!(out.contains("kind:       Event"), "out: {out}");
    assert!(out.contains("edges:"), "out: {out}");
    assert!(out.contains("located-at"), "out: {out}");
}

#[test]
fn edges_happy_path_lists_one_adjacency_page() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("Place:ur-1") || out.contains("Place:"), "out: {out}");
    assert!(out.contains("(end of list)") || out.contains("more: continue"), "out: {out}");
}

#[test]
fn find_happy_path_matches_across_kinds() {
    let o = run_with_data_dir(&["find", "jericho"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.to_lowercase().contains("jericho"), "out: {out}");
    assert!(out.contains("Place") || out.contains("Event"), "out: {out}");
}

#[test]
fn bare_invocation_shows_short_help_and_names_the_tutorial() {
    let o = run(&[]);
    assert!(o.status.success());
    let out = stdout(&o);
    assert!(out.contains("commands:"), "out: {out}");
    assert!(out.contains("bibex tutorial"), "out: {out}");
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn help_command_matches_bare_invocation() {
    let bare = stdout(&run(&[]));
    let help = stdout(&run(&["help"]));
    assert_eq!(bare, help, "'bibex help' must be identical to bare 'atlas' per CONTRACT.md");
}

#[test]
fn tutorial_runs_to_completion_with_seven_nonempty_steps() {
    let o = run_with_data_dir(&["tutorial"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    for n in 1..=7 {
        let marker = format!("Step {n} of 7:");
        assert!(out.contains(&marker), "missing '{marker}' in tutorial output:\n{out}");
    }
    let positions: Vec<usize> = (1..=7).map(|n| out.find(&format!("Step {n} of 7:")).unwrap()).collect();
    for w in positions.windows(2) {
        assert!(w[1] - w[0] > 40, "a tutorial step looks empty: gap of only {} chars", w[1] - w[0]);
    }
    assert!(out.len() - positions[6] > 40, "the final tutorial step looks empty");
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn bad_usage_on_an_unrecognized_subcommand() {
    let o = run_with_data_dir(&["vers", "GEN.1.1"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert!(err.contains("unrecognized subcommand"), "err: {err}");
    assert_eq!(o.status.code(), Some(4));
    assert!(stdout(&o).is_empty(), "a failing command must not print to stdout");
}

#[test]
fn bad_usage_on_missing_required_edges_kind() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn bad_ref_on_a_malformed_locus_grammar() {
    let o = run_with_data_dir(&["verse", "GEN.1.abc"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_ref):"), "err: {err}");
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn bad_ref_on_a_malformed_wire_id_grammar() {
    let o = run_with_data_dir(&["node", "not-even-a-colon-pair"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_ref):"), "err: {err}");
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn not_found_on_a_well_shaped_but_absent_id() {
    let o = run_with_data_dir(&["node", "Event:not-a-real-event"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (not_found):"), "err: {err}");
    assert_eq!(o.status.code(), Some(3));
}

#[test]
fn data_load_failed_when_graph_bin_is_missing() {
    let o = run(&["--data-dir", "./this-directory-does-not-exist", "verse", "GEN.1.1"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (data_load_failed):"), "err: {err}");
    assert_eq!(o.status.code(), Some(5));
}

#[test]
fn empty_result_when_find_matches_nothing() {
    let o = run_with_data_dir(&["find", "zzqxnotarealsearchterm"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (empty_result):"), "err: {err}");
    assert!(err.contains("no matches"), "err: {err}");
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn empty_result_when_an_edge_kind_has_zero_entries_at_a_real_id() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "cites"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (empty_result):"), "err: {err}");
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn bad_ref_on_an_unrecognized_edge_kind_label() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "not-a-real-kind"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_ref):"), "err: {err}");
    assert!(err.contains("not-a-real-kind"), "err: {err}");
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn find_with_no_argument_states_its_own_kind_coverage_scope() {
    let o = run_with_data_dir(&["find"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert!(err.contains("Place/Event/Narrative/Era/Polity"), "err must state find's own kind-coverage scope: {err}");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn edges_names_the_flag_for_verse_chapter_node_too() {
    let o = run_with_data_dir(&["node", "--bogus-flag"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert!(err.contains("--bogus-flag"), "err must name the specific unrecognized flag: {err}");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn verse_json_happy_path_carries_real_fields() {
    let (o, v) = run_json(&["verse", "GEN.1.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["ref"], "GEN.1.1");
    assert_eq!(v["text"], "In the beginning God created the heaven and the earth.");
    assert_eq!(v["tracked"], true);
    assert!(v["words_of_christ"].is_array());
    assert!(v["places"].is_array());
    let persons = v["persons"].as_array().expect("persons must be an array");
    assert!(persons.iter().any(|p| p["label"] == "God" && p["id"].as_str().unwrap().starts_with("Person:")), "persons: {persons:?}");
    let events = v["events"].as_array().expect("events must be an array");
    assert!(events.iter().any(|e| e["id"].as_str().unwrap().starts_with("Event:")), "events: {events:?}");
}

#[test]
fn verse_json_concord_shape_is_leaner_and_untracked() {
    let (o, v) = run_json(&["verse", "BoC 7.2.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["ref"], "BoC 7.2.1");
    assert_eq!(v["tracked"], false);
    assert!(v.get("places").is_none(), "a Concord verse's json must not carry the KJV-only sections: {v:?}");
}

#[test]
fn chapter_json_happy_path_carries_real_units_in_order() {
    let (o, v) = run_json(&["chapter", "GEN.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let units = v.unwrap();
    let units = units.as_array().expect("chapter --json must be a top-level array");
    assert_eq!(units.len(), 31, "Genesis 1 has exactly 31 verses");
    assert_eq!(units[0]["ref"], "GEN.1.1");
    assert_eq!(units[0]["text"], "In the beginning God created the heaven and the earth.");
    assert_eq!(units[30]["ref"], "GEN.1.31");
    assert!(units[0]["words_of_christ"].is_array());
}

#[test]
fn node_json_happy_path_carries_real_fields() {
    let (o, v) = run_json(&["node", "Event:ab_ur"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["id"], "Event:ab_ur");
    assert_eq!(v["kind"], "Event");
    assert_eq!(v["label"], "Terah's family leaves Ur");
    assert_eq!(v["provenance"], "curated");
    let summary = v["edge_summary"].as_array().expect("edge_summary must be an array");
    assert!(summary.iter().any(|e| e["kind"] == "located-at" && e["count"].as_u64().unwrap() >= 1), "edge_summary: {summary:?}");
}

#[test]
fn edges_json_happy_path_carries_real_fields() {
    let (o, v) = run_json(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["kind"], "located-at");
    let entries = v["entries"].as_array().expect("entries must be an array");
    assert!(!entries.is_empty());
    assert_eq!(entries[0]["neighbour"]["position"], "node", "entries: {entries:?}");
    assert!(entries[0]["neighbour"]["node"]["id"].as_str().unwrap().starts_with("Place:"), "entries: {entries:?}");
    assert!(entries[0]["edge"]["id"].is_string());
}

#[test]
fn find_json_happy_path_carries_real_fields() {
    let (o, v) = run_json(&["find", "jericho"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let hits = v.unwrap();
    let hits = hits.as_array().expect("find --json must be a top-level array");
    assert!(hits.iter().any(|h| h["kind"] == "Place" && h["id"].as_str().unwrap().starts_with("Place:")), "hits: {hits:?}");
}

#[test]
fn find_json_widened_scope_carries_a_real_person_hit() {
    let (o, v) = run_json(&["find", "moses"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let hits = v.unwrap();
    let hits = hits.as_array().expect("find --json must be a top-level array");
    let person = hits.iter().find(|h| h["kind"] == "Person").unwrap_or_else(|| panic!("find --json 'moses' must surface a Person hit: {hits:?}"));
    assert_eq!(person["id"], "Person:moses_2108");
    assert_eq!(person["label"], "Moses");
}

#[test]
fn find_json_widened_scope_carries_a_real_catechism_item_hit() {
    let (o, v) = run_json(&["find", "First Commandment"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let hits = v.unwrap();
    let hits = hits.as_array().expect("find --json must be a top-level array");
    let item = hits.iter().find(|h| h["kind"] == "CatechismItem").unwrap_or_else(|| panic!("find --json 'First Commandment' must surface a CatechismItem hit: {hits:?}"));
    assert_eq!(item["id"], "CatechismItem:commandment-1");
    assert_eq!(item["label"], "The First Commandment");
}

#[test]
fn node_json_edge_summary_is_an_explicit_empty_array_for_a_real_zero_edge_node() {
    let (o, v) = run_json(&["node", "Translation:latin_vulgate"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["id"], "Translation:latin_vulgate");
    assert!(v.get("edge_summary").is_some(), "edge_summary key must be PRESENT even when this node has zero edges, never omitted");
    assert_eq!(v["edge_summary"], serde_json::Value::Array(vec![]), "edge_summary must be an explicit empty array, not null or absent");
}

#[test]
fn json_error_on_a_malformed_data_dir_is_data_load_failed_not_plain_text() {
    let o = run(&["--data-dir", "./this-directory-does-not-exist", "--json", "verse", "GEN.1.1"]);
    assert!(!o.status.success());
    assert!(stdout(&o).is_empty(), "a failing --json invocation must print nothing to stdout, even when the failure comes from --data-dir resolution itself");
    let err: serde_json::Value =
        serde_json::from_str(&stderr(&o)).unwrap_or_else(|e| panic!("a malformed --data-dir under --json must still render its error as JSON, never plain text: {e}\nstderr: {}", stderr(&o)));
    assert_eq!(err["error"]["code"], "data_load_failed");
    assert!(err["error"]["message"].is_string());
    assert!(err["error"]["hint"].is_string());
    assert_eq!(o.status.code(), Some(5));
}

#[test]
fn kinds_json_row_count_matches_from_label_and_every_token_round_trips() {
    let (o, v) = run_json(&["kinds"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let rows = v.unwrap();
    let rows = rows.as_array().expect("kinds --json must be a top-level array");
    assert!(!rows.is_empty());
    for row in rows {
        assert!(row["token"].is_string());
        assert!(row["relation"].is_string());
        let dir = row["direction"].as_str().unwrap();
        assert!(matches!(dir, "forward" | "inverse" | "symmetric"), "unexpected direction: {dir}");
    }
    let plain = stdout(&run_with_data_dir(&["kinds"]));
    let plain_rows = plain.lines().skip(1).count();
    assert_eq!(rows.len(), plain_rows, "plain and --json 'kinds' must list the identical vocabulary");
}

#[test]
fn json_error_on_not_found_id_emits_the_error_object_on_stderr() {
    let (o, v) = run_json(&["node", "Event:not-a-real-event"]);
    assert!(!o.status.success());
    assert!(v.is_none());
    let err: serde_json::Value = serde_json::from_str(&stderr(&o)).unwrap_or_else(|e| panic!("--json error path stderr must be valid JSON: {e}\nstderr: {}", stderr(&o)));
    assert_eq!(err["error"]["code"], "not_found");
    assert!(err["error"]["message"].as_str().unwrap().contains("Event:not-a-real-event"));
    assert!(err["error"]["hint"].is_string());
    assert!(stdout(&o).is_empty(), "a failing --json invocation must print nothing to stdout");
    assert_eq!(o.status.code(), Some(3));
}

#[test]
fn json_error_on_empty_result_uses_the_same_taxonomy_as_plain_mode() {
    let (o, v) = run_json(&["find", "zzqxnotarealsearchterm"]);
    assert!(!o.status.success());
    assert!(v.is_none());
    let err: serde_json::Value = serde_json::from_str(&stderr(&o)).unwrap();
    assert_eq!(err["error"]["code"], "empty_result");
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn json_tutorial_is_bad_usage() {
    let (o, v) = run_json(&["tutorial"]);
    assert!(!o.status.success());
    assert!(v.is_none());
    let err: serde_json::Value = serde_json::from_str(&stderr(&o)).unwrap();
    assert_eq!(err["error"]["code"], "bad_usage");
    assert_eq!(o.status.code(), Some(4));
    assert!(stdout(&o).is_empty());
}

#[test]
fn json_help_is_bad_usage() {
    let (o, v) = run_json(&["help"]);
    assert!(!o.status.success());
    assert!(v.is_none());
    let err: serde_json::Value = serde_json::from_str(&stderr(&o)).unwrap();
    assert_eq!(err["error"]["code"], "bad_usage");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn json_bare_invocation_is_bad_usage() {
    let dd = data_dir();
    let o = run(&["--data-dir", dd.to_str().unwrap(), "--json"]);
    assert!(!o.status.success());
    let err: serde_json::Value = serde_json::from_str(&stderr(&o)).unwrap();
    assert_eq!(err["error"]["code"], "bad_usage");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn json_flag_works_before_or_after_the_subcommand() {
    let dd = data_dir();
    let dd_str = dd.to_str().unwrap();
    let before = run(&["--json", "--data-dir", dd_str, "node", "Event:ab_ur"]);
    let after = run(&["--data-dir", dd_str, "node", "Event:ab_ur", "--json"]);
    assert!(before.status.success(), "stderr: {}", stderr(&before));
    assert!(after.status.success(), "stderr: {}", stderr(&after));
    assert_eq!(stdout(&before), stdout(&after));
}

#[test]
fn node_plain_output_is_byte_unchanged_by_the_json_addition() {
    let o = run_with_data_dir(&["node", "Event:ab_ur"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let expected = concat!(
        "id:         Event:ab_ur\n",
        "kind:       Event\n",
        "label:      Terah's family leaves Ur\n",
        "provenance: curated\n",
        "edges:\n",
        "  attested-in      2\n",
        "  follows-in       1\n",
        "  dated-by         1\n",
        "  dates            1\n",
        "  located-at       1\n",
        "  shown-on         1\n",
        "  temporal-adjacency 2\n",
    );
    assert_eq!(stdout(&o), expected, "node's plain output must be byte-identical to its pre-BIBEX-1 form");
}

#[test]
fn edges_plain_output_is_byte_unchanged_by_the_json_addition() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let expected = "LocatedAt:dafbb7c28eb80653a693de9906dc0669 Place        Place:ur-1                   Ur 1\n(end of list)\n";
    assert_eq!(stdout(&o), expected, "edges's plain output must be byte-identical to its pre-BIBEX-1 form");
}

#[test]
fn verse_plain_output_brackets_an_id_next_to_every_attached_name() {
    let o = run_with_data_dir(&["verse", "GEN.1.1"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    let persons_line = out.lines().find(|l| l.starts_with("Persons:")).expect("a Persons: line");
    assert!(persons_line.contains("God [Person:"), "persons line must bracket a wire id next to the name: {persons_line}");
    let events_line = out.lines().find(|l| l.starts_with("Events:")).expect("an Events: line");
    assert!(events_line.contains('[') && events_line.contains("[Event:"), "events line must bracket a wire id: {events_line}");
}

#[test]
fn find_widened_to_person_and_the_id_is_directly_usable_in_node() {
    let find_out = stdout(&run_with_data_dir(&["find", "moses"]));
    let person_line = find_out.lines().find(|l| l.starts_with("Person")).expect("find 'moses' must surface a Person hit now: {find_out}");
    let id = person_line.split_whitespace().nth(1).expect("a Person id column");
    assert!(id.starts_with("Person:"), "id must be the wire-encoded form: {id}");

    let node_out = run_with_data_dir(&["node", id]);
    assert!(node_out.status.success(), "'bibex node {id}' must succeed on an id 'bibex find' just printed -- stderr: {}", stderr(&node_out));
    assert!(stdout(&node_out).contains("kind:       Person"));
}

#[test]
fn find_widened_to_catechism_item() {
    let out = stdout(&run_with_data_dir(&["find", "First Commandment"]));
    assert!(out.contains("CatechismItem"), "find must now search CatechismItem labels too: {out}");
    assert!(out.contains("CatechismItem:commandment-1"), "id column must be the wire-encoded form: {out}");
}

#[test]
fn find_id_column_is_directly_usable_in_node_for_every_widened_kind() {
    let find_out = stdout(&run_with_data_dir(&["find", "jericho"]));
    let place_line = find_out.lines().find(|l| l.starts_with("Place")).expect("a Place hit");
    let id = place_line.split_whitespace().nth(1).unwrap();
    assert!(id.starts_with("Place:"));
    let node_out = run_with_data_dir(&["node", id]);
    assert!(node_out.status.success(), "stderr: {}", stderr(&node_out));
}

#[test]
fn node_edge_summary_kind_token_is_directly_usable_in_edges() {
    let node_out = stdout(&run_with_data_dir(&["node", "Event:ab_ur"]));
    let located_at_line = node_out.lines().find(|l| l.trim_start().starts_with("located-at")).expect("a located-at edge-summary row");
    let token = located_at_line.split_whitespace().next().unwrap();
    assert_eq!(token, "located-at");

    let edges_out = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", token]);
    assert!(edges_out.status.success(), "'bibex edges Event:ab_ur --kind {token}' must succeed on a token 'bibex node' just printed -- stderr: {}", stderr(&edges_out));
    assert!(stdout(&edges_out).contains("Place:"));
}

#[test]
fn kinds_plain_lists_the_full_vocabulary() {
    let o = run_with_data_dir(&["kinds"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("cites") && out.contains("cited-by"), "out: {out}");
    assert!(out.contains("temporal-adjacency"), "out: {out}");
    assert!(out.contains("forward") && out.contains("inverse") && out.contains("symmetric"), "out: {out}");
}

#[test]
fn kinds_takes_no_arguments() {
    let o = run_with_data_dir(&["kinds", "extra-arg"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn edges_lists_an_edge_neighbour_under_its_own_kind_known_by_its_id() {
    let (_, dates) = run_json(&["edges", "Anchor:solomon-crowned", "--kind", "dates"]);
    let dates = dates.unwrap();
    let dating_edge = dates["entries"][0]["edge"].clone();
    let dating = dating_edge["id"].as_str().unwrap().to_string();
    let (_, justifies) = run_json(&["edges", "Anchor:solomon-crowned", "--kind", "justifies"]);
    let justifies = justifies.unwrap();
    let justification = justifies["entries"][0]["edge"]["id"].as_str().unwrap().to_string();
    assert_eq!(justifies["entries"][0]["neighbour"], serde_json::json!({"position": "edge", "edge": dating_edge}));
    let o = run_with_data_dir(&["edges", "Anchor:solomon-crowned", "--kind", "justifies"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o), format!("{justification:<24} Edge         {dating:<28} {dating}\n(end of list)\n"));
}

#[test]
fn edges_never_surfaces_an_unresolvable_peoplegroup_neighbor() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert!(!stdout(&o).contains("PeopleGroup:"), "no PeopleGroup entry should ever reach stdout");
}

#[test]
fn node_resolves_a_chapter_container_with_members_and_navigation() {
    let o = run_with_data_dir(&["node", "Container:bible-chapter-JHN-3"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("kind:       Container"), "out: {out}");
    assert!(out.contains("label:      John 3"), "the reader's own display name (canon::BOOKS + chapter): {out}");
    assert!(out.contains("contains         36"), "John 3 has 36 verses: {out}");
    assert!(out.contains("member-of        1"), "a chapter is a member of exactly its book: {out}");
    assert!(out.contains("follows-in       1"), "prev/next navigation forward: {out}");
    assert!(out.contains("precedes-in      1"), "prev/next navigation backward: {out}");
}

#[test]
fn node_resolves_a_book_container_with_its_chapters() {
    let o = run_with_data_dir(&["node", "Container:bible-book-GEN"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("kind:       Container"), "out: {out}");
    assert!(out.contains("label:      Genesis"), "out: {out}");
    assert!(out.contains("contains         50"), "Genesis has 50 chapters: {out}");
    assert!(out.contains("follows-in       1"), "Genesis follows-in to Exodus: {out}");
}

#[test]
fn edges_walks_chapter_succession_across_the_book_boundary() {
    let o = run_with_data_dir(&["edges", "Container:bible-chapter-GEN-50", "--kind", "follows-in"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("Container:bible-chapter-EXO-1"), "out: {out}");
    assert!(out.contains("Exodus 1"), "out: {out}");
}

#[test]
fn edges_reaches_a_chapters_verses_and_a_verse_reaches_its_chapter_back() {
    let o = run_with_data_dir(&["edges", "Container:bible-chapter-JHN-3", "--kind", "contains", "--limit", "3"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("text-unit:JHN.3.1"), "out: {out}");
    assert!(out.contains("more: continue with --cursor 3"), "pagination must work on the new frontier: {out}");

    let o2 = run_with_data_dir(&["edges", "text-unit:JHN.3.16", "--kind", "member-of"]);
    assert!(o2.status.success(), "stderr: {}", stderr(&o2));
    let out2 = stdout(&o2);
    assert!(out2.contains("Container:bible-chapter-JHN-3"), "out: {out2}");
    assert!(out2.contains("John 3"), "out: {out2}");
}

#[test]
fn node_json_on_a_chapter_container_carries_the_same_record_shape() {
    let (o, v) = run_json(&["node", "Container:bible-chapter-JHN-3"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let v = v.expect("json value");
    assert_eq!(v["kind"], "Container");
    assert_eq!(v["label"], "John 3");
    assert_eq!(v["provenance"], "kjv");
    let contains = v["edge_summary"].as_array().unwrap().iter().find(|e| e["kind"] == "contains").expect("a contains row");
    assert_eq!(contains["count"], 36);
}

#[test]
fn verse_places_line_names_a_real_place_and_stays_none_for_a_verse_with_no_mention() {
    let o = run_with_data_dir(&["verse", "GEN.13.18"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    let places_line = out.lines().find(|l| l.starts_with("Places:")).expect("a Places: line");
    println!("GEN.13.18 -> {places_line}");
    assert!(places_line.contains("[Place:hebron]"), "the PLACES section must name the real place id: {out}");
    assert!(places_line.contains("Hebron"), "and its resolved display name: {out}");
    assert!(!places_line.contains("(none)"), "a verse with a real mention must never render the empty sentinel: {out}");

    let o2 = run_with_data_dir(&["verse", "GEN.1.1"]);
    assert!(o2.status.success(), "stderr: {}", stderr(&o2));
    let out2 = stdout(&o2);
    let places_line2 = out2.lines().find(|l| l.starts_with("Places:")).expect("a Places: line");
    println!("GEN.1.1  -> {places_line2}");
    assert!(places_line2.contains("(none)"), "GEN.1.1 has no curated place mention and must say so: {out2}");
}

#[test]
fn verify_passes_on_the_committed_sections_and_names_every_section_and_the_root() {
    let o = run_with_data_dir(&["verify"]);
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    let text = stdout(&o);
    for name in ["core", "kjv", "concord", "kretzmann", "lexicon"] {
        let line = text.lines().find(|l| l.starts_with(name)).unwrap_or_else(|| panic!("no line for {name}: {text}"));
        assert!(line.contains("logical ") && line.contains(" OK ") && line.contains("transport OK"), "{line}");
        assert!(line.contains(" -> ") && line.contains(" bytes"), "sizes: {line}");
    }
    let root = text.lines().find(|l| l.starts_with("root ")).unwrap();
    assert!(root.contains(" OK (recomputed from 5 section lines)"), "{text}");
    let last = text.lines().last().unwrap();
    assert!(last.starts_with("raw ") && last.contains(" OK (") && last.ends_with(" files; the sections were compiled from it)"), "{text}");
    assert_eq!(text.lines().count(), 7);

    let (o, v) = run_json(&["verify"]);
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    let v = v.unwrap();
    assert_eq!(v["root"]["ok"], true);
    assert_eq!(v["root"]["manifest"], v["root"]["recomputed"]);
    let sections = v["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 5);
    assert!(sections.iter().all(|s| s["transport"] == "ok" && s["logical_check"] == "ok" && s["schema_version"] == atlas_graph::sqlite::SCHEMA_VERSION));
    assert!(sections.iter().all(|s| s["uncompressed_bytes"].as_u64().unwrap() > s["bytes"].as_u64().unwrap()));
    assert_eq!(v["raw"]["status"], "ok");
    assert_eq!(v["root"]["raw_root"], v["raw"]["root"]);

    let o = run_with_data_dir(&["verify", "--section", "concord"]);
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    assert_eq!(stdout(&o).lines().count(), 2, "one section line + the root line: {}", stdout(&o));
    assert!(stdout(&o).starts_with("concord "));

    let o = run_with_data_dir(&["verify", "--section", "nope"]);
    assert_eq!(o.status.code(), Some(4));
    assert!(stderr(&o).contains("bad_usage") && stderr(&o).contains("unknown section 'nope'"), "{}", stderr(&o));
    let o = run_with_data_dir(&["verify", "--section"]);
    assert_eq!(o.status.code(), Some(4));
    let o = run_with_data_dir(&["verify", "extra"]);
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn verify_fails_with_exit_6_on_a_tampered_blob_and_names_both_hashes() {
    let src = data_dir();
    let root = std::env::temp_dir().join(format!("bibex-verify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let data = root.join("compiled");
    std::fs::create_dir_all(data.join("sections")).unwrap();
    std::fs::copy(src.join("manifest.toml"), data.join("manifest.toml")).unwrap();
    for e in std::fs::read_dir(src.join("sections")).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), data.join("sections").join(e.file_name())).unwrap();
    }
    let concord = std::fs::read_dir(data.join("sections"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.file_name().unwrap().to_string_lossy().starts_with("concord."))
        .expect("a concord blob");
    let mut bytes = std::fs::read(&concord).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&concord, &bytes).unwrap();

    let o = run_with_data_dir_at(&data, &["verify"]);
    assert_eq!(o.status.code(), Some(6), "stderr: {}", stderr(&o));
    let err = stderr(&o);
    assert!(err.contains("integrity_failed") && err.contains("concord: transport MISMATCH manifest ") && err.contains(" file "), "{err}");
    assert!(err.contains("1 of 11 checks failed"), "{err}");
    let (o, v) = run_json_with_data_dir_at(&data, &["verify"]);
    assert_eq!(o.status.code(), Some(6));
    assert!(v.is_none());
    let envelope: serde_json::Value = serde_json::from_str(&stderr(&o)).expect("a JSON error envelope on stderr");
    assert_eq!(envelope["error"]["code"], "integrity_failed");
    let o = run_with_data_dir_at(&data, &["verify", "--section", "core"]);
    assert_eq!(o.status.code(), Some(0), "stderr: {}", stderr(&o));
    let cache = root.join("cache").join("sections");
    if cache.is_dir() {
        let leftovers: Vec<String> = std::fs::read_dir(&cache).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".tmp")).collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
    std::fs::remove_file(&concord).unwrap();
    let o = run_with_data_dir_at(&data, &["verify", "--section", "concord"]);
    assert_eq!(o.status.code(), Some(0), "an absent optional section is not a failure: {}", stderr(&o));
    assert!(stdout(&o).contains("absent (optional)"), "{}", stdout(&o));
    let kjv = std::fs::read_dir(data.join("sections")).unwrap().map(|e| e.unwrap().path()).find(|p| p.file_name().unwrap().to_string_lossy().starts_with("kjv.")).unwrap();
    std::fs::remove_file(&kjv).unwrap();
    let o = run_with_data_dir_at(&data, &["verify", "--section", "kjv"]);
    assert_eq!(o.status.code(), Some(6));
    assert!(stderr(&o).contains("kjv: required blob MISSING"), "{}", stderr(&o));
    let empty = root.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let o = run_with_data_dir_at(&empty, &["verify"]);
    assert_eq!(o.status.code(), Some(5), "{}", stderr(&o));
    std::fs::remove_dir_all(&root).ok();
}
