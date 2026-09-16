#!/usr/bin/env bash
# CONTENTION-1 (docs/superpowers/specs/2026-09-14-relational-artifact-design.md §8):
# the eight wall-clock gates, run SERIALIZED -- one `cargo test` process per
# gate, `--test-threads=1`, never concurrently with the parallel suite.
# The ceilings live in the tests and are NOT touched here.
#
#   bash scripts/timing-gates.sh          run all eight; exit 1 on any failure
#   bash scripts/timing-gates.sh check    reconcile this list against the tree
#   bash scripts/timing-gates.sh list     print the gate names, one per line
#
# `check` is the anti-bypass law: every `#[ignore = REASON]` test under the
# tree must be listed here (multiset-equal), and NO other `#[ignore` attribute
# may exist under the tree at all. scripts/timing-gates-selftest.sh attempts
# each bypass and requires refusal.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TREE="${TIMING_GATES_TREE:-$ROOT/server}"
LOG_DIR="${TIMING_GATES_LOG_DIR:-$ROOT/.superpowers/sdd/2026-08-17-bible-atlas-m1/logs-timing-gates}"
REASON='wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)'

# package | test binary | test name -- ORDER IS THE RUN ORDER.
GATES=(
  "atlas-graph|artifact_conformance|serialized_artifact_is_admitted_and_loads_under_the_committed_ceiling"
  "atlas-server|graph_conformance|the_full_real_graph_is_admitted_the_in_memory_store_answers_match_the_model_exactly"
  "atlas-server|perf_smoke|scene_time_full_span_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|scene_time_nt_window_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|scene_scripture_chapter_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|xrefs_for_verse_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|text_window_completes_within_smoke_threshold"
  "atlas-server|perf_smoke|chapter_window_completes_within_smoke_threshold"
)

names_in_script() { printf '%s\n' "${GATES[@]}" | awk -F'|' '{print $3}' | sort; }

# Every `fn NAME` that directly follows a `#[ignore = REASON]` attribute.
names_in_tree() {
  grep -rn --include='*.rs' --exclude-dir=target -A 2 -F "#[ignore = \"$REASON\"]" "$TREE" 2>/dev/null \
    | grep -oE '(^|[-:])fn [A-Za-z0-9_]+' | sed -E 's/^.*fn //' | sort
}

# Any #[ignore attribute at all (bare, with a different reason, sharing a
# line with another attribute e.g. `#[test] #[ignore]`, or nested inside a
# `#[cfg_attr(COND, ignore)]`). Fix round 1 (Task 3 review): the previous
# `^\s*#\[ignore` anchor only matched an #[ignore] that opened its own
# line, so `#[test] #[ignore]` on one line -- a real, quiet un-gate --
# reconciled clean. The attribute may now appear ANYWHERE on a line
# (`#\[[^]]*\bignore\b` matches the word "ignore" inside any `#[...]`
# bracket, cfg_attr's nested parens included), which in turn now also
# matches this file's own `//!` doc-comment mentions of `#[ignore]` (the
# CONTENTION-1 paragraphs this same batch added to the three gated test
# files) -- so doc-comment lines (content, after grep's `file:NN:`
# prefix, starting with `//`) are dropped before the reason-string filter
# runs. cfg_attr-based ignoring is caught by the same pattern; there is
# no known legitimate `#[ignore]`/`cfg_attr(..., ignore)` anywhere under
# server/ today (verified: only the eight gated attributes and doc-prose
# mentions exist), so this is not narrowed further.
foreign_ignores() {
  grep -rn --include='*.rs' --exclude-dir=target -E '#\[[^]]*\bignore\b' "$TREE" 2>/dev/null \
    | grep -vE '^[^:]*:[0-9]+:[[:space:]]*//' \
    | grep -vF "#[ignore = \"$REASON\"]" || true
}

check() {
  local ok=0
  local foreign; foreign="$(foreign_ignores)"
  if [ -n "$foreign" ]; then
    echo "REFUSED: #[ignore] attributes under $TREE that are not the CONTENTION-1 gate reason:" >&2
    echo "$foreign" >&2; ok=1
  fi
  # MULTISET comparison: `diff` on the sorted, non-deduplicated lists.
  if ! diff <(names_in_script) <(names_in_tree) >"$LOG_DIR/.check.diff" 2>/dev/null; then
    echo "REFUSED: the gate list in $0 and the #[ignore = REASON] tests under $TREE differ (< script, > tree):" >&2
    cat "$LOG_DIR/.check.diff" >&2; ok=1
  fi
  [ "$ok" -eq 0 ] && echo "timing-gates check: ${#GATES[@]} gates reconcile with the tree"
  return "$ok"
}

run() {
  command -v cargo >/dev/null || { echo "cargo not on PATH (export PATH=\"\$PATH:\$HOME/.cargo/bin\")" >&2; exit 2; }
  check || { echo "not running gates whose list does not reconcile" >&2; exit 1; }
  local stamp; stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  local passed=0 failed=0 i=0
  for g in "${GATES[@]}"; do
    i=$((i+1))
    IFS='|' read -r pkg bin name <<<"$g"
    local log="$LOG_DIR/$stamp-$i-$bin-$name.log"
    echo "== gate $i/${#GATES[@]}: $pkg::$bin::$name"
    # One process per gate. --exact so a name is a name, not a prefix.
    if ( cd "$ROOT/server" && cargo test -p "$pkg" --test "$bin" -- --ignored --exact "$name" --test-threads=1 ) >"$log" 2>&1 \
       && grep -qF "test $name ... ok" "$log" \
       && grep -qE "^test result: ok\. 1 passed" "$log"; then
      passed=$((passed+1)); grep -E "ARTIFACT LOAD|CONFORMANCE|took|ceiling" "$log" | head -3 | sed 's/^/   /' || true
    else
      failed=$((failed+1)); echo "   FAILED -- see $log"; grep -E "panicked|exceeding|over the" "$log" | head -3 | sed 's/^/   /' || true
    fi
  done
  echo "TIMING GATES: $passed/${#GATES[@]} passed (serialized, one process each; logs in $LOG_DIR)"
  [ "$failed" -eq 0 ]
}

mkdir -p "$LOG_DIR"
case "${1:-run}" in
  run)   run ;;
  check) check ;;
  list)  printf '%s\n' "${GATES[@]}" | awk -F'|' '{print $3}' ;;
  *)     echo "usage: $0 [run|check|list]" >&2; exit 2 ;;
esac
