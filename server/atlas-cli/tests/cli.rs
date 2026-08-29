//! R6: integration tests invoking the REAL compiled `atlas-cli` binary via
//! `std::process::Command`, against the REAL committed
//! `data/compiled/graph.bin` (the house real-data idiom -- the SAME
//! `env!("CARGO_MANIFEST_DIR")).join("../../data/compiled")` convention
//! `atlas-server/tests/perf_smoke.rs`/`scene_byte_identity.rs` already use).
//! Every subcommand's happy path, every CONTRACT.md error-taxonomy class,
//! the tutorial smoke test, bare-invocation help, and exit-code assertions.

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

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).expect("stdout must be valid UTF-8")
}
fn stderr(o: &Output) -> String {
    String::from_utf8(o.stderr.clone()).expect("stderr must be valid UTF-8")
}

/// BIBEX-1: runs `bibex --json <args>` against the real committed graph and
/// parses stdout as JSON -- fails loud (not `Option`/`Result` swallowed) if
/// stdout isn't valid JSON, since a --json happy path promises exactly one
/// JSON value on stdout, nothing else.
fn run_json(args: &[&str]) -> (Output, Option<serde_json::Value>) {
    let dd = data_dir();
    let dd_str = dd.to_str().expect("data dir path must be valid UTF-8");
    let mut full = vec!["--data-dir", dd_str, "--json"];
    full.extend_from_slice(args);
    let o = run(&full);
    let value = if o.status.success() { Some(serde_json::from_str(&stdout(&o)).unwrap_or_else(|e| panic!("--json happy path stdout must be valid JSON: {e}\nstdout: {}", stdout(&o)))) } else { None };
    (o, value)
}

// ---------------------------------------------------------------------
// Happy paths -- one per subcommand (R6).
// ---------------------------------------------------------------------

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

// Batch PERI-1 (PRESENTATION CATEGORY LAW -- owner, verbatim: "NUN is not
// an event. fix this error and others like it"): the CLI's own sibling of
// the two owner-named repros -- `Events:` must NEVER carry a general-kind
// pericope's label; `Passages:` must.
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
fn node_happy_path_shows_card_and_edge_summary() {
    let o = run_with_data_dir(&["node", "Event:ab_ur"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    assert!(out.contains("id:         Event:ab_ur"), "out: {out}");
    assert!(out.contains("kind:       Event"), "out: {out}");
    assert!(out.contains("edges:"), "out: {out}");
    assert!(out.contains("located-at"), "out: {out}");
}

#[test]
fn edges_happy_path_lists_one_frontier_page() {
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

// ---------------------------------------------------------------------
// Bare invocation / help (CONTRACT.md).
// ---------------------------------------------------------------------

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

// ---------------------------------------------------------------------
// Tutorial smoke test (R6): runs to completion, nonempty numbered steps.
// ---------------------------------------------------------------------

#[test]
fn tutorial_runs_to_completion_with_seven_nonempty_steps() {
    let o = run_with_data_dir(&["tutorial"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let out = stdout(&o);
    for n in 1..=7 {
        let marker = format!("Step {n} of 7:");
        assert!(out.contains(&marker), "missing '{marker}' in tutorial output:\n{out}");
    }
    // Every step must carry real content, not just its own header --
    // the gap between consecutive "Step N of 7" markers must be more
    // than a couple of characters.
    let positions: Vec<usize> = (1..=7).map(|n| out.find(&format!("Step {n} of 7:")).unwrap()).collect();
    for w in positions.windows(2) {
        assert!(w[1] - w[0] > 40, "a tutorial step looks empty: gap of only {} chars", w[1] - w[0]);
    }
    assert!(out.len() - positions[6] > 40, "the final tutorial step looks empty");
    assert_eq!(o.status.code(), Some(0));
}

// ---------------------------------------------------------------------
// Error taxonomy (CONTRACT.md): one test per class, two for bad_ref.
// ---------------------------------------------------------------------

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
    // Event:ab_ur is real (proven by node_happy_path above); "cites" is a
    // real edge-kind label (TextUnit-only), so this is a real id + a real
    // kind with a genuinely empty page at this position.
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "cites"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (empty_result):"), "err: {err}");
    assert_eq!(o.status.code(), Some(1));
}

// ---------------------------------------------------------------------
// FIX ROUND 1 (review): T-4, S-4, S-3/Q-2.
// ---------------------------------------------------------------------

#[test]
fn bad_ref_on_an_unrecognized_edge_kind_label() {
    // T-4: the review found this taxonomy case (bad_ref on an
    // unrecognized `--kind` label, distinct from the *missing*-`--kind`
    // bad_usage case above) implemented correctly but untested.
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "not-a-real-kind"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_ref):"), "err: {err}");
    assert!(err.contains("not-a-real-kind"), "err: {err}");
    assert_eq!(o.status.code(), Some(2));
}

#[test]
fn find_with_no_argument_states_its_own_kind_coverage_scope() {
    // S-4: CONTRACT.md promises the no-argument message itself states
    // find's Place/Event/Narrative/Era/Polity scope, not just general
    // help/the empty_result message -- assert the promise is kept.
    let o = run_with_data_dir(&["find"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert!(err.contains("Place/Event/Narrative/Era/Polity"), "err must state find's own kind-coverage scope: {err}");
    assert_eq!(o.status.code(), Some(4));
}

#[test]
fn edges_names_the_flag_for_verse_chapter_node_too() {
    // T-3: unknown-flag diagnostics now name the flag for the
    // one_positional-backed commands too, not just `edges`.
    let o = run_with_data_dir(&["node", "--bogus-flag"]);
    assert!(!o.status.success());
    let err = stderr(&o);
    assert!(err.contains("atlas: error (bad_usage):"), "err: {err}");
    assert!(err.contains("--bogus-flag"), "err must name the specific unrecognized flag: {err}");
    assert_eq!(o.status.code(), Some(4));
}

// ---------------------------------------------------------------------
// BIBEX-1: --json mode -- happy paths (real field values, not just
// is-json), the error path, and tutorial/help/bare = bad_usage.
// ---------------------------------------------------------------------

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
    // A real, checked-present attachment: God is mentioned at GEN.1.1.
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
    assert!(entries[0]["node"]["id"].as_str().unwrap().starts_with("Place:"), "entries: {entries:?}");
    assert!(entries[0]["edge"].is_string());
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
fn kinds_json_row_count_matches_parse_edge_kind_and_every_token_round_trips() {
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
    // Plain and json modes must agree on the row count.
    let plain = stdout(&run_with_data_dir(&["kinds"]));
    let plain_rows = plain.lines().skip(1).count(); // skip the header line
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
    // "--json is a GLOBAL flag... accepted before the subcommand, any
    // order between the two" (CONTRACT.md). Prove both orders parse the
    // same real query identically.
    let dd = data_dir();
    let dd_str = dd.to_str().unwrap();
    let before = run(&["--json", "--data-dir", dd_str, "node", "Event:ab_ur"]);
    let after = run(&["--data-dir", dd_str, "node", "Event:ab_ur", "--json"]);
    assert!(before.status.success(), "stderr: {}", stderr(&before));
    assert!(after.status.success(), "stderr: {}", stderr(&after));
    assert_eq!(stdout(&before), stdout(&after));
}

// ---------------------------------------------------------------------
// BIBEX-1: plain-mode byte-unchanged regression guard -- representative
// commands the --json addition never touches (chapter/node/edges' own
// plain bytes are unaffected by this batch).
// ---------------------------------------------------------------------

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
        "  temporal-adjacency 2\n",
    );
    assert_eq!(stdout(&o), expected, "node's plain output must be byte-identical to its pre-BIBEX-1 form");
}

#[test]
fn edges_plain_output_is_byte_unchanged_by_the_json_addition() {
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    let expected = "LocatedAt:b449e37d5a691cef Place        Place:ur-1                   Ur 1\n(end of list)\n";
    assert_eq!(stdout(&o), expected, "edges's plain output must be byte-identical to its pre-BIBEX-1 form");
}

// ---------------------------------------------------------------------
// BIBEX-1 addendum ticket 2: ID discoverability -- "IDS EVERYWHERE",
// find's widened kind coverage, and the real "see it -> use it" loop for
// both a node id (via find) and an edge-kind token (via node).
// ---------------------------------------------------------------------

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

    // The loop: paste that exact id into `bibex node`.
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
    // Same loop as the Person case above, proven once more for a Place
    // hit (find's own pre-existing kind, but the id column itself changed
    // this batch from a bare curated id to the wire-encoded form).
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

    // The loop: paste that exact token into `bibex edges --kind`.
    let edges_out = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", token]);
    assert!(edges_out.status.success(), "'bibex edges Event:ab_ur --kind {token}' must succeed on a token 'bibex node' just printed -- stderr: {}", stderr(&edges_out));
    assert!(stdout(&edges_out).contains("Place:"));
}

// ---------------------------------------------------------------------
// BIBEX-1 addendum ticket 2: `bibex kinds`.
// ---------------------------------------------------------------------

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
fn edges_never_surfaces_an_unresolvable_peoplegroup_neighbor() {
    // S-3/Q-2: mirrors atlas_server::graph_handlers::node_edges's own
    // PeopleGroup filter. This test only proves the filter code RUNS
    // without breaking the happy path (a PeopleGroup-carrying real id in
    // the committed graph, with a real edge to a PeopleGroup neighbor,
    // isn't independently known here) -- the filter's own correctness is
    // structurally identical to the server's already-tested one
    // (`graph_wire::decode_node_id` has no "PeopleGroup" arm, confirmed
    // in this same review), so a full end-to-end repro is not required to
    // trust it; this test guards against a future edit accidentally
    // removing the filter and breaking a normal edges page.
    let o = run_with_data_dir(&["edges", "Event:ab_ur", "--kind", "located-at"]);
    assert!(o.status.success(), "stderr: {}", stderr(&o));
    assert!(!stdout(&o).contains("PeopleGroup:"), "no PeopleGroup entry should ever reach stdout");
}
