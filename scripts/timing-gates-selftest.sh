#!/usr/bin/env bash
# CONTENTION-1: every way to quietly un-gate a wall-clock test is ATTEMPTED
# here and must be REFUSED by `timing-gates.sh check`. In the CDC-1
# tradition: a gate that has not tried to bypass itself is a hope.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
GATES="$ROOT/scripts/timing-gates.sh"
REASON='wall-clock gate: run serialized via scripts/timing-gates.sh (CONTENTION-1)'

pass=0; fail=0
expect_ok()      { if "$@" >/dev/null 2>&1; then pass=$((pass+1)); else fail=$((fail+1)); echo "FAIL (expected ok):      $*" >&2; fi; }
expect_refused() { if "$@" >/dev/null 2>&1; then fail=$((fail+1)); echo "FAIL (expected refused): $*" >&2; else pass=$((pass+1)); fi; }

# A synthetic tree whose ignored tests exactly match the script's list.
mk_tree() { # <dir> ; writes one .rs per gate name given on stdin
  local dir="$1"; mkdir -p "$dir/src"
  while read -r name; do
    printf '#[ignore = "%s"]\nfn %s() {}\n' "$REASON" "$name" >> "$dir/src/gates.rs"
  done
}

tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT

# 0. The real tree reconciles.
expect_ok bash "$GATES" check

# 1. Control: a synthetic tree with exactly the script's names reconciles.
bash "$GATES" list > "$tmp/names"
mk_tree "$tmp/t1" < "$tmp/names"
expect_ok env TIMING_GATES_TREE="$tmp/t1" bash "$GATES" check

# 2. ATTEMPT: drop one gated test from the tree (someone deleted the
#    #[ignore] line, or renamed the test) -> the script would still "run"
#    a name that no longer exists. Must be refused.
mk_tree "$tmp/t2" < <(sed '1d' "$tmp/names")
expect_refused env TIMING_GATES_TREE="$tmp/t2" bash "$GATES" check

# 3. ATTEMPT: gate a NINTH test with the right reason but never add it to
#    the script -> it is ignored by the default run and run by nobody.
mk_tree "$tmp/t3" < "$tmp/names"
printf '#[ignore = "%s"]\nfn a_ninth_gate_nobody_runs() {}\n' "$REASON" >> "$tmp/t3/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t3" bash "$GATES" check

# 4. ATTEMPT: reason drift -- keep the test ignored but change the reason
#    string so it stops matching the grep. Must be refused: NO #[ignore
#    attribute under the tree may carry any other reason.
mk_tree "$tmp/t4" < <(sed '1d' "$tmp/names")
printf '#[ignore = "flaky, look later"]\nfn %s() {}\n' "$(sed -n '1p' "$tmp/names")" >> "$tmp/t4/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t4" bash "$GATES" check

# 5. ATTEMPT: a bare #[ignore] with no reason at all.
mk_tree "$tmp/t5" < "$tmp/names"
printf '#[ignore]\nfn quietly_disabled() {}\n' >> "$tmp/t5/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t5" bash "$GATES" check

# 6. ATTEMPT: duplicate a name in the tree (two tests, one listing) -- a
#    SET comparison would pass this; the comparison must be a MULTISET.
mk_tree "$tmp/t6" < "$tmp/names"
printf '#[ignore = "%s"]\nfn %s() {}\n' "$REASON" "$(sed -n '1p' "$tmp/names")" >> "$tmp/t6/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t6" bash "$GATES" check

# 7. ATTEMPT: the SAME-LINE bypass (fix round 1, Task 3 review) -- share
#    `#[ignore]` on the same line as another attribute, with no reason
#    string at all, so a regex anchored to "line STARTS WITH #[ignore"
#    never sees it. `check` must match #[ignore anywhere on a line.
mk_tree "$tmp/t7" < "$tmp/names"
printf '#[test] #[ignore]\nfn quietly_disabled_same_line() {}\n' >> "$tmp/t7/src/gates.rs"
expect_refused env TIMING_GATES_TREE="$tmp/t7" bash "$GATES" check

# 8. The README and the workspace manifest must name the serialized step,
#    or the standing counting procedure silently drops 8 gates.
expect_ok grep -qF 'scripts/timing-gates.sh' "$ROOT/README.md"
expect_ok grep -qF 'scripts/timing-gates.sh' "$ROOT/server/Cargo.toml"

echo "timing-gates selftest: $pass passed, $fail failed"
[ "$fail" -eq 0 ]
