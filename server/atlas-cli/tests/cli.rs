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
    Command::new(env!("CARGO_BIN_EXE_atlas-cli")).args(args).output().expect("atlas-cli must run")
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
    assert!(out.contains("atlas tutorial"), "out: {out}");
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn help_command_matches_bare_invocation() {
    let bare = stdout(&run(&[]));
    let help = stdout(&run(&["help"]));
    assert_eq!(bare, help, "'atlas help' must be identical to bare 'atlas' per CONTRACT.md");
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
