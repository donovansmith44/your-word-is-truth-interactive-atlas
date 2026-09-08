#!/usr/bin/env bash
# THE GATE'S OWN TEST SUITE — every known way to lie to it, attempted.
#
# Batch CDC-1 fix round 1. The review's verdict was that the gate could
# green-lie three ways while its own header asserted it could not, and the
# controller's discipline for this round is explicit:
#
#   "for every bypass you close, ship the ATTEMPT as a test — the gate must
#    be proven unlieable by tests that try to lie to it, not by comments
#    claiming it cannot be."
#
# So nothing here asserts that the gate is sound. Each case DOES the thing
# and asserts the gate refused. A comment claiming inescapability is worth
# nothing; a test that tries to escape and fails is worth exactly what it
# demonstrates.
#
# Two sections:
#   A. BYPASS ATTEMPTS against the real gate in this worktree. These mutate
#      nothing permanently -- the one case that plants a violation restores
#      it under a trap and then VERIFIES the restoration with git.
#   B. CLASSIFIER CASES for scripts/contract-semver-gate.sh, run in
#      throwaway git repos under a temp dir, so testing a gate never means
#      committing junk history onto a real branch (which matters here: a
#      second batch commits to this branch while this runs).
#
#   bash scripts/gate-selftest.sh
#
# Exits 0 only if every attempt was refused and every classification was
# correct.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

pass=0; failn=0
ok()   { echo "  ok   -- $1"; pass=$((pass+1)); }
bad()  { echo "  FAIL -- $1" >&2; failn=$((failn+1)); }
head2() { printf '\n--- %s\n' "$1"; }

# A gate run must never report success. Anything non-zero is a refusal.
expect_refused() { # <desc> <exit>
  if [ "$2" -ne 0 ]; then ok "$1 (refused, exit $2)"; else bad "$1 -- THE GATE PASSED; this bypass is open"; fi
}
expect_ok() { # <desc> <exit>
  if [ "$2" -eq 0 ]; then ok "$1"; else bad "$1 -- expected success, got exit $2"; fi
}

echo "================ A. BYPASS ATTEMPTS (real gate, this worktree) ================"

# ---------------------------------------------------------------------
# A1 / A2 -- substituted toolchains (review C-1)
# ---------------------------------------------------------------------
# The demonstrated bypass was `CARGO=/usr/bin/true`, which made leg 3 --
# the only leg that proves the pact reflects the real provider -- execute
# nothing, while the gate printed PASSED. `true` exits 0 for every
# invocation, including `--version`, which is why the fix verifies what the
# tool SAYS rather than that it ran.
head2 "A1: CARGO substituted with a program that always succeeds"
CARGO=/usr/bin/true bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a1.txt 2>&1
expect_refused "CARGO=/usr/bin/true" $?
grep -q "does not identify itself as cargo" /tmp/gs_a1.txt \
  && ok "  ...and it said why" || bad "  ...but the refusal did not name the substituted toolchain"

head2 "A2: CABAL substituted (neutralises legs 1, 2 and 4 together)"
CABAL=/usr/bin/true bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a2.txt 2>&1
expect_refused "CABAL=/usr/bin/true" $?
grep -q "does not identify itself as cabal" /tmp/gs_a2.txt \
  && ok "  ...and it said why" || bad "  ...but the refusal did not name the substituted toolchain"

head2 "A2b: a toolchain path that does not exist at all"
CARGO=/nonexistent/cargo bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a2b.txt 2>&1
expect_refused "CARGO=/nonexistent/cargo" $?

# ---------------------------------------------------------------------
# A3 -- an inherited ATLAS_BLESS_PACT (review C-1, third bypass)
# ---------------------------------------------------------------------
# Both recorders used to write the pact and `return`, so the test passed
# unconditionally: any shell with the variable already exported turned leg 3
# green forever while rewriting the evidence under it. Two fixes, tested
# separately, because either alone would be enough to argue about.
head2 "A3a: blessing a recorder directly must FAIL, not pass"
( cd server && ATLAS_BLESS_PACT=1 "${CARGO:-$HOME/.cargo/bin/cargo}" test -p atlas-server --test contract_pact the_recorded_pact_still_matches >/tmp/gs_a3a.txt 2>&1 )
expect_refused "ATLAS_BLESS_PACT=1 cargo test --test contract_pact" $?
grep -q "failing on purpose" /tmp/gs_a3a.txt \
  && ok "  ...and it said it re-recorded and failed on purpose" || bad "  ...but the failure did not explain itself"

head2 "A3b: the gate strips the variable, so an inherited one cannot bless mid-run"
ATLAS_BLESS_PACT=1 bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a3b.txt 2>&1
expect_ok "gate still PASSES with ATLAS_BLESS_PACT=1 exported (stripped for leg 3)" $?
if git diff --quiet -- contracts/pacts; then
  ok "  ...and contracts/pacts is unchanged (the evidence was not rewritten)"
else
  bad "  ...but contracts/pacts was MODIFIED during the run"
  git checkout -- contracts/pacts 2>/dev/null || true
fi

# ---------------------------------------------------------------------
# A4 -- disarming a RECEIVED suite with one tag (review C-2)
# ---------------------------------------------------------------------
# The reviewer's demonstration: plant a real violation of map-generator's
# own fixture, add `@target` above the scenario, and the full gate goes
# green with their expectation of us broken. This re-runs that attempt.
#
# It mutates two tracked files, so both are snapshotted and restored under
# a trap, and the restoration is VERIFIED with git before the script exits.
EDGE_FEATURE="contracts/atlas-edge/eras.feature"
EDGE_FIXTURE="contracts/atlas-edge/fixtures/eras-consumed.json"
restore_edge() {
  git checkout -- "$EDGE_FEATURE" "$EDGE_FIXTURE" 2>/dev/null || true
}
trap restore_edge EXIT

head2 "A4a: a planted violation of map-generator's fixture is caught (the gate has teeth here)"
python -c "
import io,json
p='$EDGE_FIXTURE'
d=json.load(open(p,encoding='utf-8'))
d[0]['name']='PrimevalX'
io.open(p,'w',encoding='utf-8',newline='\n').write(json.dumps(d,indent=4))
"
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a4a.txt 2>&1
expect_refused "planted violation in atlas-edge" $?

head2 "A4b: ...and adding @target above it must NOT make the gate green"
python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read()
s=s.replace('  Scenario: the whole era table','  @target\n  Scenario: the whole era table',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a4b.txt 2>&1
expect_refused "@target planted in a RECEIVED suite" $?
grep -q "RECEIVED suite" /tmp/gs_a4b.txt \
  && ok "  ...and it named the received-suite rule" || bad "  ...but the refusal did not name the received-suite rule"

restore_edge
trap - EXIT
if git diff --quiet -- "$EDGE_FEATURE" "$EDGE_FIXTURE"; then
  ok "A4 cleanup: map-generator's suite is byte-restored"
else
  bad "A4 cleanup: atlas-edge is still modified -- restore it by hand"
fi

echo
echo "================ B. CLASSIFIER CASES (isolated throwaway repos) ================"

TMPROOT="$(mktemp -d 2>/dev/null || echo "${TMPDIR:-/tmp}/gateselftest.$$")"
mkdir -p "$TMPROOT"
cleanup_tmp() { rm -rf "$TMPROOT"; }
trap cleanup_tmp EXIT

# Build a minimal suite repo and return its base SHA on stdout.
new_repo() { # <dir>
  local d="$1"
  mkdir -p "$d/contracts/atlas-graph-contract/fixtures" "$d/scripts"
  cp "$ROOT/scripts/contract-semver-gate.sh" "$d/scripts/"
  ( cd "$d"
    git init -q .
    git config user.email t@t; git config user.name t
    printf '1.2.3\n' > contracts/atlas-graph-contract/VERSION
    printf '# c\n' > contracts/atlas-graph-contract/CHANGELOG.md
    cat > contracts/atlas-graph-contract/a.feature <<'FEOF'
Feature: f
  prose here

  Scenario: one
    When I GET /api/x
    Then the response equals fixture "x"
FEOF
    printf '{\n  "a": 1\n}\n' > contracts/atlas-graph-contract/fixtures/x.json
    git add -A >/dev/null 2>&1
    git commit -qm base
    git rev-parse HEAD )
}

# <name> <expected: pass|fail> <dir> <base> ...script args
run_case() {
  local name="$1" expect="$2" d="$3" base="$4"
  local out rc
  out="$( cd "$d" && bash scripts/contract-semver-gate.sh "$base" 2>&1 )"; rc=$?
  if [ "$expect" = pass ] && [ "$rc" -eq 0 ]; then ok "$name"
  elif [ "$expect" = fail ] && [ "$rc" -ne 0 ]; then ok "$name (refused)"
  else
    bad "$name -- expected $expect, got exit $rc"
    printf '%s\n' "$out" | sed 's/^/        /' >&2
  fi
}

# B1: prose-only change with no bump -> PATCH required, none declared.
D="$TMPROOT/b1"; BASE="$(new_repo "$D")"
( cd "$D" && sed -i 's/prose here/prose here, clarified/' contracts/atlas-graph-contract/a.feature \
  && git add -A >/dev/null 2>&1 && git commit -qm prose )
run_case "B1 prose, no bump -> refused" fail "$D" "$BASE"

# B2: same, with a patch bump + changelog.
( cd "$D" && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.2.4" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm patch )
run_case "B2 prose + PATCH -> accepted" pass "$D" "$BASE"

# B3/B4: a scenario ADDED is MINOR.
D="$TMPROOT/b3"; BASE="$(new_repo "$D")"
( cd "$D" && cat >> contracts/atlas-graph-contract/a.feature <<'FEOF'

  Scenario: two
    When I GET /api/y
    Then the response equals fixture "y"
FEOF
  printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION
  echo "- 1.2.4" >> contracts/atlas-graph-contract/CHANGELOG.md
  git add -A >/dev/null 2>&1 && git commit -qm add )
run_case "B3 scenario added, declared PATCH -> refused" fail "$D" "$BASE"
( cd "$D" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && git add -A >/dev/null 2>&1 && git commit -qm minor )
run_case "B4 scenario added, declared MINOR -> accepted" pass "$D" "$BASE"

# B5/B6: a fixture re-blessed to a DIFFERENT VALUE is MAJOR; a pure
# REFORMAT is not. B6 is the case that makes this a classifier rather than
# a counter, so it is the one to keep honest.
D="$TMPROOT/b5"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm rebless )
run_case "B5 fixture re-blessed to a new value, declared MINOR -> refused" fail "$D" "$BASE"

D="$TMPROOT/b6"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{"a":1}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.2.4" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm reformat )
run_case "B6 fixture REFORMATTED, same parsed value, declared PATCH -> accepted" pass "$D" "$BASE"

# B7/B8: deleting a scenario is MAJOR.
D="$TMPROOT/b7"; BASE="$(new_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().split('  Scenario: one')[0]
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm delscenario )
run_case "B7 scenario deleted, declared MINOR -> refused" fail "$D" "$BASE"
( cd "$D" && printf '2.0.0\n' > contracts/atlas-graph-contract/VERSION \
  && git add -A >/dev/null 2>&1 && git commit -qm major )
run_case "B8 scenario deleted, declared MAJOR -> accepted" pass "$D" "$BASE"

# B9: adding @target withdraws a guarantee -> MAJOR (review C-2's rule for
# suites we DO version; on a received suite it is forbidden outright, A4b).
D="$TMPROOT/b9"; BASE="$(new_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  Scenario: one','  @target\n  Scenario: one',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm target )
run_case "B9 @target ADDED, declared MINOR -> refused" fail "$D" "$BASE"

# B10: removing @target is a guarantee GAINED -> MINOR, not MAJOR.
# Review M-3: the shipped CHANGELOG promises MINOR for exactly this move
# (once bibex gains its version field) and the classifier used to demand
# MAJOR, contradicting it.
D="$TMPROOT/b10"; BASE=""
mkdir -p "$D/contracts/atlas-graph-contract/fixtures" "$D/scripts"
cp "$ROOT/scripts/contract-semver-gate.sh" "$D/scripts/"
( cd "$D"
  git init -q .; git config user.email t@t; git config user.name t
  printf '1.2.3\n' > contracts/atlas-graph-contract/VERSION
  printf '# c\n' > contracts/atlas-graph-contract/CHANGELOG.md
  cat > contracts/atlas-graph-contract/a.feature <<'FEOF'
Feature: f
  prose

  @target
  Scenario: one
    When I GET /api/x
    Then the response equals fixture "x"
FEOF
  printf '{\n  "a": 1\n}\n' > contracts/atlas-graph-contract/fixtures/x.json
  git add -A >/dev/null 2>&1; git commit -qm base )
BASE="$( cd "$D" && git rev-parse HEAD )"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  @target\n','',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm untarget )
run_case "B10 @target REMOVED, declared MINOR -> accepted (M-3)" pass "$D" "$BASE"

# B11: deleting an ENTIRE SUITE is the most MAJOR thing possible, and used
# to classify as nothing at all because `[ -d ]` skipped it before the diff
# was consulted (review H-2).
D="$TMPROOT/b11"; BASE="$(new_repo "$D")"
( cd "$D" && git rm -rq contracts/atlas-graph-contract && git commit -qm "delete the whole suite" )
run_case "B11 whole suite DELETED -> refused (H-2)" fail "$D" "$BASE"

echo
echo "=============================================================="
echo "gate self-test: $pass passed, $failn failed"
[ "$failn" -eq 0 ] || exit 1
