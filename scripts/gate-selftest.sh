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
# Fingerprint either side of the run rather than asking git, for the same
# reason the gate does: "rewritten during this run" is the question, and a
# working tree that is merely dirty (mid-re-record) is not an answer to it.
pact_fp() { find contracts/pacts -type f -name '*.json' -print0 2>/dev/null | sort -z | xargs -0 sha256sum 2>/dev/null | sha256sum; }
fp_before="$(pact_fp)"
ATLAS_BLESS_PACT=1 bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a3b.txt 2>&1
expect_ok "gate still PASSES with ATLAS_BLESS_PACT=1 exported (stripped for leg 3)" $?
if [ "$fp_before" = "$(pact_fp)" ]; then
  ok "  ...and contracts/pacts is byte-unchanged (the evidence was not rewritten)"
else
  bad "  ...but contracts/pacts was REWRITTEN during the run"
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

# ---------------------------------------------------------------------
# A5 -- THE FORM THAT DEFEATED THE GREP (fix round 2, review C-NEW-1)
# ---------------------------------------------------------------------
# `  @wip @target` -- two spaces, a second tag. The vendored parser makes
# every whitespace-separated word on the line a tag; the old line-anchored
# grep saw nothing, and the reviewer got CONTRACT GATE: PASSED with
# map-generator's fixture violated. Leg 0 now asks the parser.
restore_edge   # A4b's plain @target must be gone before A5's premise can hold
head2 "A5: '  @wip @target' -- a multi-tag line the old regex could not see"
python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read()
s=s.replace('  Scenario: the whole era table','  @wip @target\n  Scenario: the whole era table',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
if grep -rqE '^[[:space:]]*@target([[:space:]]|$)' contracts/atlas-edge; then
  bad "A5 premise broken: the old regex CAN see this form"
else
  ok "A5 premise: the old line-anchored regex sees nothing here"
fi
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a5.txt 2>&1
expect_refused "multi-tag @target in a RECEIVED suite" $?
restore_edge

# ---------------------------------------------------------------------
# A5b -- A CLASS MEMBER NOBODY HAS DEMONSTRATED (the controller's ask)
# ---------------------------------------------------------------------
# Not a form any review typed: `@target` on the FEATURE-level tag line, above
# `Feature:`. `Gherkin/Parse.hs` collects those into `ftTags`, so it is a tag
# the corpus genuinely carries. Both greps were blind to it, and so would a
# scenario-only oracle be. `Tags.tagsOfDir` unions feature tags into every
# scenario's tag set precisely so this member of the class is covered.
head2 "A5b: @target on the FEATURE tag line -- a form no review has typed"
python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read()
s='@target\n'+s
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a5b.txt 2>&1
expect_refused "feature-level @target in a RECEIVED suite" $?
restore_edge

# ---------------------------------------------------------------------
# A6 -- MIS-REGISTRATION (fix round 2, review C-NEW-2)
# ---------------------------------------------------------------------
# One word: `contracts/atlas-edge  contract-runner` -> `aqc-dual`. The
# reviewer did exactly this, with the fixture violated AND a plain @target
# planted, and the full gate printed PASSED -- the suite simply vanished
# from every list. The registry had become the literal array it replaced.
SUITES_FILE="contracts/SUITES"
restore_all() { git checkout -- "$EDGE_FEATURE" "$EDGE_FIXTURE" "$SUITES_FILE" 2>/dev/null || true; }
trap restore_all EXIT

head2 "A6: re-registering a RECEIVED suite as 'aqc-dual' must not exempt it"
python -c "
import io
p='$SUITES_FILE'
s=io.open(p,encoding='utf-8').read().replace('contracts/atlas-edge             contract-runner','contracts/atlas-edge             aqc-dual')
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
if grep -q 'atlas-edge.*aqc-dual' "$SUITES_FILE"; then
  ok "A6 premise: the registry now claims atlas-edge is somebody else's"
else
  bad "A6 premise broken: the registry edit did not apply"
fi
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a6.txt 2>&1
expect_refused "RECEIVED suite re-registered as aqc-dual" $?
grep -q "RECEIVED.md but is registered" /tmp/gs_a6.txt \
  && ok "  ...and it named the derivation rule" || bad "  ...but the refusal did not name the derivation rule"
restore_all

head2 "A6b: registering a parser-readable suite as 'aqc-dual' must be refused"
python -c "
import io
p='$SUITES_FILE'
s=io.open(p,encoding='utf-8').read().replace('contracts/map-api-consumer       contract-runner-consumer','contracts/map-api-consumer       aqc-dual')
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
bash scripts/contract-gate.sh --fast --base HEAD~1 >/tmp/gs_a6b.txt 2>&1
expect_refused "parser-readable suite registered aqc-dual" $?
grep -q "no 'Scenario Outline'" /tmp/gs_a6b.txt \
  && ok "  ...and it named the derivable reason" || bad "  ...but the refusal did not name the derivable reason"
restore_all
trap - EXIT

if git diff --quiet -- "$EDGE_FEATURE" "$EDGE_FIXTURE" "$SUITES_FILE"; then
  ok "A4/A5/A6 cleanup: map-generator's suite and the registry are byte-restored"
else
  bad "A4/A5/A6 cleanup: something is still modified -- restore it by hand"
fi

# ---------------------------------------------------------------------
# A7 -- A TOOL THAT PASSES THE NAME CHECK AND DOES NO WORK (H-NEW-1)
# ---------------------------------------------------------------------
# The previous round's three toolchain cases all failed the NAME check,
# which is the uninteresting member of the family. This is the interesting
# one: a shim that identifies itself perfectly as cargo and runs nothing.
# It is also the accidental case -- a wrapper that mishandles a subcommand
# or swallows an exit code looks exactly like this.
head2 "A7: a fake cargo that says 'cargo 1.97.1' and does nothing"
FAKEBIN="$(mktemp -d)"
cat > "$FAKEBIN/cargo" <<'FAKE'
#!/bin/sh
if [ "$1" = "--version" ]; then echo "cargo 1.97.1 (c980f4866 2026-06-30)"; exit 0; fi
exit 0
FAKE
chmod +x "$FAKEBIN/cargo"
CARGO="$FAKEBIN/cargo" bash scripts/contract-gate.sh --base HEAD~1 >/tmp/gs_a7.txt 2>&1
expect_refused "a cargo shim that passes --version and runs nothing" $?
grep -q "produced no evidence that it ran" /tmp/gs_a7.txt \
  && ok "  ...and it demanded proof of work, not a greeting" || bad "  ...but it did not demand evidence the leg ran"
rm -rf "$FAKEBIN"

# ---------------------------------------------------------------------
# A8 -- THE FAIL-OPEN FINGERPRINT (M-NEW-2)
# ---------------------------------------------------------------------
# Every failure mode of the old pipeline produced the empty string on BOTH
# sides, so "the evidence did not move" was the answer whenever the check
# could not run. Verified here at the unit level: the gate must refuse when
# no hashing tool exists, rather than silently comparing nothing to nothing.
head2 "A8: no hashing tool available must FAIL the gate, not silently pass"
if grep -q 'for h in sha256sum shasum sha1sum md5sum' scripts/contract-gate.sh &&
   grep -q 'no hashing tool' scripts/contract-gate.sh &&
   grep -q 'could not fingerprint contracts/pacts' scripts/contract-gate.sh; then
  ok "A8: the fingerprint fails closed on a missing hasher and on an empty result"
else
  bad "A8: the pact fingerprint is still fail-open"
fi

# ---------------------------------------------------------------------
# A9 -- H-1's EMPTY-FIXTURE REFUSALS, SHIPPED AS TESTS
# ---------------------------------------------------------------------
# The review noted H-1 is closed but was the one closure with no standing
# test: "Both directions mutation-tested" was true of a manual run, in a
# round whose rule is that every closure ships its attempt.
RUNNER_BIN="$(cd contracts/runner && "${CABAL:-$HOME/.local/bin/cabal}" list-bin contract-runner 2>/dev/null)"
XREF_FIXTURE="contracts/atlas-graph-contract/fixtures/xrefs-jhn-3-16.json"
restore_h1() { git checkout -- "$XREF_FIXTURE" contracts/pacts 2>/dev/null || true; }
trap restore_h1 EXIT

head2 "A9a: an EMPTY fixture must be refused at compare time"
printf '[]\n' > "$XREF_FIXTURE"
"$RUNNER_BIN" run --replay contracts/pacts --exports data/exports contracts/atlas-graph-contract >/tmp/gs_a9a.txt 2>&1
expect_refused "fixture emptied to []" $?
grep -q "is empty -- it pins nothing" /tmp/gs_a9a.txt \
  && ok "  ...naming the vacuity" || bad "  ...but the message did not name the vacuity"
git checkout -- "$XREF_FIXTURE" 2>/dev/null || true

head2 "A9b: an EMPTY projected value must be refused at bless time (never written)"
PACT_BEFORE="$(git hash-object "$XREF_FIXTURE")"
python -c "
import json,io
p='contracts/pacts/http.json'
d=json.load(open(p,encoding='utf-8'))
d['entries']['GET /api/xrefs/JHN.3.16']['body']=[]
io.open(p,'w',encoding='utf-8',newline='\n').write(json.dumps(d,indent=2))
"
"$RUNNER_BIN" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract >/tmp/gs_a9b.txt 2>&1
expect_refused "provider answered [] and --bless was requested" $?
if [ "$(git hash-object "$XREF_FIXTURE")" = "$PACT_BEFORE" ]; then
  ok "  ...and the vacuous fixture was NOT written (the guard runs before the write)"
else
  bad "  ...but --bless overwrote the fixture with the empty answer"
fi
restore_h1
trap - EXIT
if git diff --quiet -- "$XREF_FIXTURE" contracts/pacts; then
  ok "A9 cleanup: fixture and pact byte-restored"
else
  bad "A9 cleanup: fixture or pact still modified"
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
  out="$( cd "$d" && bash scripts/contract-semver-gate.sh --runner "$RUNNER_BIN" "$base" 2>&1 )"; rc=$?
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

# B12: a fixture that gains a KEY while every previously pinned value stays
# put is a promise WIDENED, not changed -> MINOR. Without this grade,
# "a projection gaining a field" (which the suite's CHANGELOG documents as
# MINOR) classified MAJOR -- the same self-contradiction as M-3.
D="$TMPROOT/b12"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 1,\n  "b": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm widen )
run_case "B12 fixture WIDENED (new key, old values intact), declared MINOR -> accepted" pass "$D" "$BASE"

# B13: ...but changing one of the values it already pinned, even while
# adding a key, is still MAJOR. The widening grade must not become a way to
# smuggle a re-bless past the gate.
D="$TMPROOT/b13"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 99,\n  "b": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm widen-and-change )
run_case "B13 fixture gains a key AND changes an old value, declared MINOR -> refused" fail "$D" "$BASE"

# B11: deleting an ENTIRE SUITE is the most MAJOR thing possible, and used
# to classify as nothing at all because `[ -d ]` skipped it before the diff
# was consulted (review H-2).
D="$TMPROOT/b11"; BASE="$(new_repo "$D")"
( cd "$D" && git rm -rq contracts/atlas-graph-contract && git commit -qm "delete the whole suite" )
run_case "B11 whole suite DELETED -> refused (H-2)" fail "$D" "$BASE"

# B14: the semver half of C-NEW-1. `  @wip @target` withdraws a guarantee;
# the old diff-grep called it MINOR. The classifier now asks the parser
# which scenarios carry the tag on each side.
D="$TMPROOT/b14"; BASE="$(new_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  Scenario: one','  @wip @target\n  Scenario: one',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm multitag )
out="$( cd "$D" && bash scripts/contract-semver-gate.sh --runner "$RUNNER_BIN" "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ]; then ok "B14 '  @wip @target' added, declared MINOR -> refused (C-NEW-1)"
else bad "B14 '  @wip @target' declared MINOR was ACCEPTED -- a withdrawal as an addition"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi

# B15: `--base HEAD` made every suite's diff empty, so leg 5 classified
# nothing while printing the shape of success (M-NEW-1).
D="$TMPROOT/b15"; BASE="$(new_repo "$D")"
( cd "$D" && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm x )
out="$( cd "$D" && bash scripts/contract-semver-gate.sh HEAD 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ]; then ok "B15 --base HEAD -> refused (M-NEW-1)"
else bad "B15 --base HEAD was ACCEPTED -- leg 5 asserted nothing"; fi

# B16: PATH-shadowing `python` with a shim printing `same` turned every
# re-bless into a PATCH (M-NEW-5). The fix validates the ANSWER and treats a
# non-grade as a failure to classify.
D="$TMPROOT/b16"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{"a":999,"z":"TOTALLY-DIFFERENT"}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm rebless )
FAKEPY="$(mktemp -d)"; printf '#!/bin/sh\necho same\n' > "$FAKEPY/python"; chmod +x "$FAKEPY/python"
out="$( cd "$D" && PATH="$FAKEPY:$PATH" bash scripts/contract-semver-gate.sh "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ]; then ok "B16 PATH-shadowed python printing 'same' -> refused (M-NEW-5)"
else bad "B16 a fake python downgraded a re-bless to PATCH"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi
# ...and the control: the real python still classifies it MAJOR.
out="$( cd "$D" && bash scripts/contract-semver-gate.sh "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | grep -q 'MAJOR'; then
  ok "B16b control: with a real python the same diff is MAJOR"
else bad "B16b control failed -- the re-bless was not classified MAJOR"; fi
rm -rf "$FAKEPY"

# B17: a fixture truncated to ZERO BYTES was never graded at all, so
# emptying a pinned promise classified as PATCH (L-NEW-1).
D="$TMPROOT/b17"; BASE="$(new_repo "$D")"
( cd "$D" && : > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm truncate )
out="$( cd "$D" && bash scripts/contract-semver-gate.sh "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ]; then ok "B17 fixture truncated to 0 bytes, declared PATCH -> refused (L-NEW-1)"
else bad "B17 an emptied fixture passed as PATCH"; fi

# B18: deleting a RECEIVED suite -- directory and registry line together --
# passed every leg but 3, whose message invited the re-record that launders
# it (M-NEW-4). Received suites are now tracked across the diff.
D="$TMPROOT/b18"
mkdir -p "$D/contracts/their-edge/fixtures" "$D/scripts"
cp "$ROOT/scripts/contract-semver-gate.sh" "$D/scripts/"
( cd "$D"
  git init -q .; git config user.email t@t; git config user.name t
  printf 'their expectations of us\n' > contracts/their-edge/RECEIVED.md
  cat > contracts/their-edge/a.feature <<'FEOF'
Feature: theirs

  Scenario: one
    When I GET /api/x
    Then the response equals fixture "x"
FEOF
  printf '{"a":1}\n' > contracts/their-edge/fixtures/x.json
  git add -A >/dev/null 2>&1; git commit -qm base )
BASE="$( cd "$D" && git rev-parse HEAD )"
( cd "$D" && git rm -rq contracts/their-edge && git commit -qm "delete the received suite" )
out="$( cd "$D" && bash scripts/contract-semver-gate.sh "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | grep -q 'RECEIVED suite was DELETED'; then
  ok "B18 RECEIVED suite deleted -> refused outright (M-NEW-4)"
else bad "B18 a received suite was deleted without objection"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi

echo
echo "=============================================================="
echo "gate self-test: $pass passed, $failn failed"
[ "$failn" -eq 0 ] || exit 1
