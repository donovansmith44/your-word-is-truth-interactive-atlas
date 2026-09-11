#!/usr/bin/env bash
# THE GATE'S OWN TEST SUITE — every known way to lie to it, attempted.
#
# The rule, unchanged since fix round 1: nothing here asserts that the gate
# is sound. Each case DOES the thing and asserts the gate refused. A comment
# claiming inescapability is worth nothing; a test that tries to escape and
# fails is worth exactly what it demonstrates.
#
# FIX ROUND 3 ADDS THE RULE THE LAST ROUND'S SUITE BROKE (review M-R2-3).
# Two of round 2's 48 cases did not test what they claimed:
#
#   * A8 was three `grep`s OF THE GATE'S OWN SOURCE TEXT. It would have
#     passed with the condition inverted, because all three strings survive
#     that edit. Its own comment called this "verified here at the unit
#     level". It is behavioural now: four constant-printing hasher shims are
#     put on PATH and the gate must refuse.
#   * A7 planted the previous review's shim VERBATIM -- the one that does no
#     work -- and not the class member that prints the evidence string,
#     which was that review's green lie.
#
# So: A TEST THAT GREPS THE GATE'S SOURCE IS NOT A TEST. Every case below
# runs a real command and reads a real exit code, and where the refusal has
# a reason, the case asserts the reason too -- otherwise a case can pass
# because some UNRELATED leg failed, which is the same vacuity one level up.
#
# And the round-2 review's process note, taken seriously: "the next round's
# self-test should contain at least three cases that no review has ever
# demonstrated, written before the fixes." The cases marked NEW-CLASS below
# are those. They found nothing, which is the outcome worth reporting.
#
# Two sections:
#   A. BYPASS ATTEMPTS against the real gate in this worktree. These mutate
#      nothing permanently -- every mutation is restored under a trap and
#      the restoration is VERIFIED with git.
#   B. COMMITTED attacks, in throwaway git repos under a temp dir. Section A
#      cannot commit (testing a gate must never mean committing junk history
#      onto a real branch), and the ratchets are about what is COMMITTED, so
#      they live here.
#
#   bash scripts/gate-selftest.sh
#
# Exits 0 only if every attempt was refused and every classification was
# correct.
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

LOGDIR="$(mktemp -d)"
pass=0; failn=0
ok()   { echo "  ok   -- $1"; pass=$((pass+1)); }
bad()  { echo "  FAIL -- $1" >&2; failn=$((failn+1)); }
head2() { printf '\n--- %s\n' "$1"; }

# Every log goes to a UNIQUE path under one mktemp dir, never a fixed name
# with `>`. Review M-R2-1: a completion sentinel proves A shell finished, not
# that only ONE shell wrote -- two runs redirecting to the same fixed path
# both write from offset 0, interleave, and the last one to finish appends
# the sentinel over a corrupted file. A unique name per run is the half that
# actually prevents it.
lg() { printf '%s/%s.%s.txt\n' "$LOGDIR" "$1" "$$"; }

expect_refused() { # <desc> <exit>
  if [ "$2" -ne 0 ]; then ok "$1 (refused, exit $2)"; else bad "$1 -- THE GATE PASSED; this bypass is open"; fi
}
expect_ok() { # <desc> <exit>
  if [ "$2" -eq 0 ]; then ok "$1"; else bad "$1 -- expected success, got exit $2"; fi
}
# `--fast` reports 3: success for what it ran, failure for what it did not.
expect_fast_ok() { # <desc> <exit>
  if [ "$2" -eq 3 ]; then ok "$1"; else bad "$1 -- expected the --fast success code 3, got exit $2"; fi
}
# The refusal must name its own reason. A case that only checks "non-zero"
# passes when an unrelated leg fails, which is how a suite drifts into
# testing nothing (M-R2-3).
because() { # <desc> <logfile> <string>
  if grep -qF "$3" "$2"; then ok "  ...because: $3"
  else bad "  ...but the refusal never said '$3'"; sed 's/^/        /' "$2" | tail -25 >&2; fi
}
not_because() { # <desc> <logfile> <string>
  if grep -qF "$3" "$2"; then bad "  ...and it WRONGLY said '$3'"; else ok "  ...and it did not say '$3'"; fi
}

BASEREF="HEAD~1"

echo "================ A. BYPASS ATTEMPTS (real gate, this worktree) ================"

# ---------------------------------------------------------------------
# A0 -- the baseline, so every probe below means something
# ---------------------------------------------------------------------
head2 "A0: the gate passes on a clean tree (--fast)"
L="$(lg a0)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_fast_ok "clean --fast run" $?
because "" "$L" "coverage reconciles:"

# ---------------------------------------------------------------------
# A1 / A2 / A2b -- SUBSTITUTED TOOLCHAINS (review H-R2-1)
# ---------------------------------------------------------------------
# Rounds 1 and 2 both answered a substituted `$CARGO` by asking the tool a
# better question -- first its name, then for a summary line -- and both
# answers were echoed back by a shim. Round 3 removed the question: the gate
# reads no CARGO or CABAL variable at all and resolves fixed absolute paths.
#
# So these cases assert something stronger than a refusal: THE ATTACK HAS NO
# EFFECT. Setting the variable, or shadowing the tool on PATH, leaves the
# gate running the real toolchain and passing.
head2 "A1: CARGO=/usr/bin/true is not read at all"
L="$(lg a1)"; CARGO=/usr/bin/true bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_fast_ok "CARGO=/usr/bin/true has no effect" $?
because "" "$L" '$CARGO and $CABAL are not read'

head2 "A2: CABAL=/usr/bin/true is not read at all"
L="$(lg a2)"; CABAL=/usr/bin/true bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_fast_ok "CABAL=/usr/bin/true has no effect" $?

head2 "A2b: a PATH-shadowed cargo that greets correctly and does no work (H-NEW-1's shim)"
FAKEBIN="$(mktemp -d)"
cat > "$FAKEBIN/cargo" <<'FAKE'
#!/bin/sh
if [ "$1" = "--version" ]; then echo "cargo 1.97.1 (c980f4866 2026-06-30)"; exit 0; fi
exit 0
FAKE
cat > "$FAKEBIN/cabal" <<'FAKE'
#!/bin/sh
if [ "$1" = "--version" ]; then echo "cabal-install version 3.12.1.0"; exit 0; fi
exit 0
FAKE
chmod +x "$FAKEBIN/cargo" "$FAKEBIN/cabal"
L="$(lg a2b)"; PATH="$FAKEBIN:$PATH" bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_fast_ok "PATH-shadowed cargo/cabal shims have no effect (absolute paths)" $?

head2 "A2c: the H-R2-1 shim -- a cargo that PRINTS the evidence string (round 3's green lie)"
# Six lines instead of three: it prints `test result: ok. 4 passed`, which is
# exactly what round 2's `require_test_evidence` grepped for, and the full
# gate printed PASSED with leg 3 executing no Rust. There is no longer a
# variable that points the gate at it, so the attack cannot even be mounted;
# this asserts that, and asserts the real recorders still ran.
cat > "$FAKEBIN/cargo" <<'FAKE'
#!/bin/sh
if [ "$1" = "--version" ]; then echo "cargo 1.97.1 (c980f4866 2026-06-30)"; exit 0; fi
echo "running 4 tests"
echo "test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"
exit 0
FAKE
chmod +x "$FAKEBIN/cargo"
L="$(lg a2c)"; CARGO="$FAKEBIN/cargo" PATH="$FAKEBIN:$PATH" bash scripts/contract-gate.sh --base "$BASEREF" >"$L" 2>&1
expect_ok "the evidence-printing cargo shim has no effect (full gate)" $?
because "" "$L" "leg 3 (HTTP recorder): 4/4 named tests passed"
because "" "$L" "leg 3 (CLI recorder): 1/1 named tests passed"
not_because "" "$L" "leg 3 (CLI recorder): 4"

head2 "A2d: a PATH-shadowed python answering the OLD calibration quiz (review H-R2-2)"
# Nine lines of `case` answered round 2's three hardcoded constant pairs and
# graded a total re-bless `same`. The grader is `contract-runner grade` now,
# so there is nothing on PATH left to shadow.
FAKEPY="$(mktemp -d)"
cat > "$FAKEPY/python" <<'FAKE'
#!/bin/sh
old="$3"; new="$4"
case "$old|$new" in
  '{"a":1}|{"a":1}')       echo same ;;
  '{"a":1}|{"a":1,"b":2}') echo wider ;;
  '{"a":1}|{"a":2}')       echo changed ;;
  *)                       echo same ;;
esac
FAKE
chmod +x "$FAKEPY/python"
L="$(lg a2d)"; PATH="$FAKEPY:$PATH" bash scripts/contract-gate.sh --fast --base 14323d5 >"$L" 2>&1
expect_fast_ok "the calibration-answering python shim has no effect" $?
because "" "$L" "graph-vocabulary.json pins additional fields"
rm -rf "$FAKEPY"

# ---------------------------------------------------------------------
# A3 -- an inherited ATLAS_BLESS_PACT
# ---------------------------------------------------------------------
head2 "A3a: blessing a recorder directly must FAIL, not pass"
L="$(lg a3a)"
( cd server && ATLAS_BLESS_PACT=1 "$HOME/.cargo/bin/cargo" test -p atlas-server --test contract_pact the_recorded_pact_still_matches >"$L" 2>&1 )
expect_refused "ATLAS_BLESS_PACT=1 cargo test --test contract_pact" $?
because "" "$L" "failing on purpose"

head2 "A3b: the gate strips the variable, so an inherited one cannot bless mid-run"
pact_fp() { find contracts/pacts -type f -name '*.json' -print0 2>/dev/null | sort -z | xargs -0 sha256sum 2>/dev/null | sha256sum; }
fp_before="$(pact_fp)"
L="$(lg a3b)"; ATLAS_BLESS_PACT=1 bash scripts/contract-gate.sh --base "$BASEREF" >"$L" 2>&1
expect_ok "gate still PASSES with ATLAS_BLESS_PACT=1 exported (stripped for leg 3)" $?
if [ "$fp_before" = "$(pact_fp)" ]; then
  ok "  ...and contracts/pacts is byte-unchanged (the evidence was not rewritten)"
else
  bad "  ...but contracts/pacts was REWRITTEN during the run"
fi

# ---------------------------------------------------------------------
# A4 / A5 -- disarming a RECEIVED suite with one tag
# ---------------------------------------------------------------------
EDGE_FEATURE="contracts/atlas-edge/eras.feature"
EDGE_FIXTURE="contracts/atlas-edge/fixtures/eras-consumed.json"
SUITES_FILE="contracts/SUITES"
GAZ="data/exports/gazetteer.json"
RECEIVED_MD="contracts/atlas-edge/RECEIVED.md"
# RESTORING TRACKED FILES IS NOT RESTORING (found by A9c/A9e, the hard way).
#
# The first version of this restored with `git checkout --`, which brings
# back deleted and modified TRACKED files and leaves every file the probe
# CREATED sitting there. `contracts/atlas-edge/eras-renamed.feature` from
# A9c therefore survived into A9d, A9e, A10, A11 and A12, quietly failing
# cases that had nothing to do with it -- and the cleanup assertion, which
# was `git diff --quiet HEAD`, passed, because `git diff` does not see
# untracked files either. That is the same blind spot leg 7 had.
#
# `git clean -fdq` over exactly the two graded trees, plus checkout. Scoped
# to `contracts/` and `data/exports/` so it can never touch EVT-3's held
# untracked work in client.Tests/ and tests/ux/.
restore_all() {
  ( cd "$ROOT"
    git checkout -- contracts data/exports 2>/dev/null || true
    git clean -fdq -- contracts data/exports 2>/dev/null || true )
}
trap 'restore_all; rm -rf "$LOGDIR" "$FAKEBIN"' EXIT

plant_fixture_violation() {
  python -c "
import io,json
p='$EDGE_FIXTURE'
d=json.load(open(p,encoding='utf-8'))
d[0]['name']='PrimevalX'
io.open(p,'w',encoding='utf-8',newline='\n').write(json.dumps(d,indent=4))
"
}
plant_tag() { # <tag line>
  python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read()
s=s.replace('  Scenario: the whole era table','$1\n  Scenario: the whole era table',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
}

head2 "A4a: a planted violation of map-generator's fixture is caught (the gate has teeth here)"
plant_fixture_violation
L="$(lg a4a)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "planted violation in atlas-edge" $?
because "" "$L" "executed expectation(s) are red"

head2 "A4b: ...and adding @target above it must NOT make the gate green"
plant_tag '  @target'
L="$(lg a4b)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "@target planted in a RECEIVED suite" $?
because "" "$L" "RECEIVED suite"
restore_all

head2 "A5: '  @wip @target' -- a multi-tag line the old regex could not see"
plant_fixture_violation; plant_tag '  @wip @target'
if grep -rqE '^[[:space:]]*@target([[:space:]]|$)' contracts/atlas-edge; then
  bad "A5 premise broken: the old regex CAN see this form"
else
  ok "A5 premise: the old line-anchored regex sees nothing here"
fi
L="$(lg a5)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "multi-tag @target in a RECEIVED suite" $?
because "" "$L" "is disarmed, missing or unparseable"
restore_all

head2 "A5b: @target on the FEATURE tag line -- a form no review has typed"
plant_fixture_violation
python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read()
io.open(p,'w',encoding='utf-8',newline='\n').write('@target\n'+s)
"
L="$(lg a5b)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "feature-level @target in a RECEIVED suite" $?
because "" "$L" "is disarmed, missing or unparseable"
restore_all

# ---------------------------------------------------------------------
# A6 -- MIS-REGISTRATION. The reviewer's N5 and N6, and their classmates.
# ---------------------------------------------------------------------
# Round 2 derived ONE of the three harness words for ONE of the two suite
# kinds, so `contract-runner-consumer` had no guard at all and `aqc-dual`'s
# guard was `grep -r` over every file in the directory. The reviewer removed
# the GRAPH-PRIMARY suite -- the owner's ruled #1 juncture -- from four legs
# with one word, and again with one word plus a three-line NOTES.md.
#
# The registry is read from HEAD now, so a working-tree edit is INERT. Each
# case therefore asserts two things: the gate still refuses the planted
# break, AND the suite is still in the executed list.
plant_export_break() {
  python -c "
import io,json
p='$GAZ'
d=json.load(open(p,encoding='utf-8'))
d['atlas_version_root']='0000000000000000'
io.open(p,'w',encoding='utf-8',newline='\n').write(json.dumps(d))
"
}
reregister() { # <suite> <new harness>
  python -c "
import io,re
p='$SUITES_FILE'
s=io.open(p,encoding='utf-8').read()
s=re.sub(r'(?m)^($1)(\s+)\S+$', r'\g<1>\g<2>$2', s)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
}

head2 "A6a: RECEIVED suite re-registered 'aqc-dual' (round-2 probe R1)"
plant_fixture_violation; reregister 'contracts/atlas-edge' 'aqc-dual'
grep -q 'atlas-edge.*aqc-dual' "$SUITES_FILE" && ok "A6a premise: the registry claims atlas-edge is somebody else's" \
  || bad "A6a premise broken: the registry edit did not apply"
L="$(lg a6a)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "RECEIVED suite re-registered as aqc-dual" $?
because "" "$L" "contracts/atlas-edge contracts/atlas-graph-contract"
restore_all

head2 "A6b: N5 -- the GRAPH-PRIMARY suite re-registered 'contract-runner-consumer'"
plant_export_break; reregister 'contracts/atlas-graph-contract' 'contract-runner-consumer'
L="$(lg a6b)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "graph-primary suite re-registered contract-runner-consumer, real C6 break planted" $?
because "" "$L" "suites (executed):     contracts/atlas-edge contracts/atlas-graph-contract"
restore_all

head2 "A6c: N6 -- ...re-registered 'aqc-dual', justified by a three-line NOTES.md"
plant_export_break; reregister 'contracts/atlas-graph-contract' 'aqc-dual'
printf '# see the AQC suite for the Scenario Outline idiom\n' > contracts/atlas-graph-contract/NOTES.md
L="$(lg a6c)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "graph-primary suite excused as aqc-dual by a NOTES.md" $?
because "" "$L" "suites (executed):     contracts/atlas-edge contracts/atlas-graph-contract"
restore_all

head2 "A6d: NEW-CLASS -- a DUPLICATE registry row, the weaker one first (reordering)"
plant_export_break
python -c "
import io
p='$SUITES_FILE'
s=io.open(p,encoding='utf-8').read()
s=s.replace('contracts/atlas-graph-contract   contract-runner',
            'contracts/atlas-graph-contract   aqc-dual\ncontracts/atlas-graph-contract   contract-runner',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
L="$(lg a6d)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "duplicate row with the weaker harness first" $?
restore_all

head2 "A6e: NEW-CLASS -- contracts/SUITES DELETED outright in the working tree"
plant_export_break; rm -f "$SUITES_FILE"
L="$(lg a6e)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "registry deleted from the working tree" $?
because "" "$L" "suites (executed):     contracts/atlas-edge contracts/atlas-graph-contract"
restore_all

# ---------------------------------------------------------------------
# A7 / A8 -- C-R2-1: THE MARKER FILE THAT SWITCHED OFF THREE PROTECTIONS
# ---------------------------------------------------------------------
head2 "A7: N2 -- rm RECEIVED.md, then disarm the suite it was protecting"
rm -f "$RECEIVED_MD"
plant_fixture_violation; plant_tag '  @wip @target'
L="$(lg a7)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "rm RECEIVED.md + @target + violated fixture" $?
because "" "$L" "is disarmed, missing or unparseable"
# The round-2 gate printed a POSITIVE ASSURANCE while the suite was
# disarmed. That sentence must not appear.
not_because "" "$L" "no received suite is disarmed ("
restore_all

head2 "A8: N3 -- rm RECEIVED.md, then re-register the suite so nothing runs it"
rm -f "$RECEIVED_MD"
plant_fixture_violation; reregister 'contracts/atlas-edge' 'contract-runner-consumer'
L="$(lg a8)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "rm RECEIVED.md + registry downgrade + violated fixture" $?
because "" "$L" "suites (executed):     contracts/atlas-edge contracts/atlas-graph-contract"
restore_all

# ---------------------------------------------------------------------
# A9 -- C-R2-3: HIDING A CORPUS, AND THE LAUNDERING THE GATE USED TO INVITE
# ---------------------------------------------------------------------
head2 "A9a: G1 -- rename ALL SIX of the received suite's .feature files away"
plant_fixture_violation
for f in contracts/atlas-edge/*.feature; do mv "$f" "$f.bak"; done
L="$(lg a9a)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "the received corpus renamed out of existence" $?
because "" "$L" "MISSING COVERAGE"
because "" "$L" "eras.feature"

head2 "A9b: G3 -- ...now follow the remedy the gate used to print"
# The corpus is still hidden. Round 2's gate said "provider drift" and both
# it and the recorder prescribed a re-record; doing it took http.json from
# 175,656 bytes to 34,257 and turned everything green. Blessing must now
# refuse BEFORE it writes, and the pact must be byte-identical afterwards.
PACT_SHA_BEFORE="$(sha256sum contracts/pacts/http.json | awk '{print $1}')"
L="$(lg a9b)"
( cd server && ATLAS_BLESS_PACT=1 "$HOME/.cargo/bin/cargo" test -p atlas-server --test contract_pact the_recorded_pact_still_matches >"$L" 2>&1 )
expect_refused "ATLAS_BLESS_PACT with a shrunken corpus" $?
because "" "$L" "REFUSING TO RE-RECORD"
if [ "$PACT_SHA_BEFORE" = "$(sha256sum contracts/pacts/http.json | awk '{print $1}')" ]; then
  ok "  ...and contracts/pacts/http.json is byte-identical (the laundering never happened)"
else
  bad "  ...but the pact was rewritten smaller -- the laundering path is still open"
fi
restore_all

head2 "A9c: NEW-CLASS -- ONE feature file renamed to another .feature name"
# Not a review's probe. The suite still exists, still has six files, the
# count is unchanged and every guard round 2 had would see a healthy corpus.
# The inventory is keyed by PATH, so the row moves and the reconciliation
# reports both halves.
mv contracts/atlas-edge/eras.feature contracts/atlas-edge/eras-renamed.feature
L="$(lg a9c)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "one .feature renamed to a different .feature name" $?
because "" "$L" "MISSING COVERAGE"
because "" "$L" "executed that are not in HEAD"
restore_all

head2 "A9d: NEW-CLASS -- a SCENARIO renamed in place"
# The file, the directory, the marker, the registry and the file count are
# all untouched; only the scenario's NAME changed, uncommitted.
python -c "
import io
p='$EDGE_FEATURE'
s=io.open(p,encoding='utf-8').read().replace('Scenario: the whole era table','Scenario: the era table',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
"
L="$(lg a9d)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "a scenario renamed in the working tree" $?
because "" "$L" "MISSING COVERAGE"
restore_all

head2 "A9e: NEW-CLASS -- an UNCOMMITTED .feature file added to a received suite"
# The other direction: an expectation that exists on disk and not in HEAD.
# A gate that graded the working tree would happily execute it and count it
# as coverage of something nobody will receive.
cat > contracts/atlas-edge/UNCOMMITTED.feature <<'FEOF'
Feature: an expectation that is not in HEAD

  Scenario: this one is not committed
    When I GET /api/eras
    Then the response equals fixture "eras-consumed"
FEOF
L="$(lg a9e)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_refused "an uncommitted .feature executed" $?
because "" "$L" "executed that are not in HEAD"
because "" "$L" "differ from HEAD"
restore_all

# ---------------------------------------------------------------------
# A10 -- THE FAIL-OPEN FINGERPRINT, BEHAVIOURALLY THIS TIME (M-R2-3, L-R2-2)
# ---------------------------------------------------------------------
# Round 2's A8 was three greps of the gate's own source and would have passed
# with the condition inverted. This puts CONSTANT-PRINTING shims for all four
# hashers on PATH: each must fail the random-input calibration, and with no
# discriminating hasher left the gate must refuse rather than compare
# nothing to nothing.
head2 "A10: every hashing tool on PATH prints a constant -- the gate must refuse"
FAKEHASH="$(mktemp -d)"
for h in sha256sum shasum sha1sum md5sum; do
  printf '#!/bin/sh\necho "deadbeef  -"\n' > "$FAKEHASH/$h"; chmod +x "$FAKEHASH/$h"
done
L="$(lg a10)"; PATH="$FAKEHASH:$PATH" bash scripts/contract-gate.sh --base "$BASEREF" >"$L" 2>&1
expect_refused "constant-printing hashers" $?
because "" "$L" "no hashing tool that demonstrably discriminates"
rm -rf "$FAKEHASH"

# ---------------------------------------------------------------------
# A11 -- H-1's EMPTY-FIXTURE REFUSALS
# ---------------------------------------------------------------------
RUNNER_BIN="$(cd contracts/runner && "$HOME/.local/bin/cabal" list-bin contract-runner 2>/dev/null)"
XREF_FIXTURE="contracts/atlas-graph-contract/fixtures/xrefs-jhn-3-16.json"

head2 "A11a: an EMPTY fixture must be refused at compare time"
printf '[]\n' > "$XREF_FIXTURE"
L="$(lg a11a)"; "$RUNNER_BIN" run --replay contracts/pacts --exports data/exports contracts/atlas-graph-contract >"$L" 2>&1
expect_refused "fixture emptied to []" $?
because "" "$L" "is empty -- it pins nothing"
git checkout -- "$XREF_FIXTURE" 2>/dev/null || true

head2 "A11b: an EMPTY projected value must be refused at bless time (never written)"
FIX_BEFORE="$(git hash-object "$XREF_FIXTURE")"
python -c "
import json,io
p='contracts/pacts/http.json'
d=json.load(open(p,encoding='utf-8'))
d['entries']['GET /api/xrefs/JHN.3.16']['body']=[]
io.open(p,'w',encoding='utf-8',newline='\n').write(json.dumps(d,indent=2))
"
L="$(lg a11b)"; "$RUNNER_BIN" run --replay contracts/pacts --exports data/exports --bless contracts/atlas-graph-contract >"$L" 2>&1
expect_refused "provider answered [] and --bless was requested" $?
if [ "$(git hash-object "$XREF_FIXTURE")" = "$FIX_BEFORE" ]; then
  ok "  ...and the vacuous fixture was NOT written (the guard runs before the write)"
else
  bad "  ...but --bless overwrote the fixture with the empty answer"
fi
restore_all
git checkout -- "$XREF_FIXTURE" 2>/dev/null || true

# ---------------------------------------------------------------------
# A12 -- --base handling (M-R2-2, L-R2-1)
# ---------------------------------------------------------------------
head2 "A12a: --base HEAD is refused"
L="$(lg a12a)"; bash scripts/contract-gate.sh --fast --base HEAD >"$L" 2>&1
expect_refused "--base HEAD" $?

head2 "A12b: an explicitly-passed --base that does not resolve is FATAL, not ignored"
L="$(lg a12b)"; bash scripts/contract-gate.sh --fast --base zzznope >"$L" 2>&1
expect_refused "--base zzznope" $?
because "" "$L" "does not resolve to a commit"

head2 "A12c: leg 5 says how many suites it actually classified"
L="$(lg a12c)"; bash scripts/contract-gate.sh --fast --base "$BASEREF" >"$L" 2>&1
expect_fast_ok "--base HEAD~1 (weak but legal)" $?
because "" "$L" "versioned suite(s) classified against"
because "" "$L" "Leg 5 asserted nothing here"

# ---------------------------------------------------------------------
# CLEANUP VERIFICATION
# ---------------------------------------------------------------------
restore_all
head2 "A cleanup: the worktree is byte-restored"
leftover_mod="$(git diff --name-only HEAD -- contracts data/exports)"
leftover_new="$(git ls-files --others --exclude-standard -- contracts data/exports)"
if [ -z "$leftover_mod" ] && [ -z "$leftover_new" ]; then
  ok "every mutated path is identical to HEAD again, and no probe file was left behind"
else
  bad "something is still different -- restore it by hand:"
  printf '%s\n' "$leftover_mod" "$leftover_new" | sed '/^$/d;s/^/        /' >&2
fi

echo
echo "================ B. COMMITTED ATTACKS (isolated throwaway repos) ================"

TMPROOT="$(mktemp -d)"
trap 'restore_all; rm -rf "$TMPROOT" "$LOGDIR" "$FAKEBIN"' EXIT

new_repo() { # <dir> -> base sha on stdout
  local d="$1"
  mkdir -p "$d/contracts/atlas-graph-contract/fixtures" "$d/scripts"
  cp "$ROOT/scripts/contract-semver-gate.sh" "$d/scripts/"
  ( cd "$d"
    git init -q .
    git config user.email t@t; git config user.name t
    printf '1.2.3\n' > contracts/atlas-graph-contract/VERSION
    printf '# c\n' > contracts/atlas-graph-contract/CHANGELOG.md
    printf 'contracts/atlas-graph-contract  contract-runner\n' > contracts/SUITES
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

run_case() { # <name> <pass|fail> <dir> <base> [<must-say>]
  local name="$1" expect="$2" d="$3" base="$4" says="${5:-}"
  local out rc
  out="$( cd "$d" && bash scripts/contract-semver-gate.sh --runner "$RUNNER_BIN" "$base" 2>&1 )"; rc=$?
  if [ "$expect" = pass ] && [ "$rc" -eq 0 ]; then ok "$name"
  elif [ "$expect" = fail ] && [ "$rc" -ne 0 ]; then
    if [ -z "$says" ] || printf '%s\n' "$out" | grep -qF "$says"; then ok "$name (refused)"
    else bad "$name -- refused, but never said '$says'"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi
  else
    bad "$name -- expected $expect, got exit $rc"
    printf '%s\n' "$out" | sed 's/^/        /' >&2
  fi
}

# B1/B2: prose-only change.
D="$TMPROOT/b1"; BASE="$(new_repo "$D")"
( cd "$D" && sed -i 's/prose here/prose here, clarified/' contracts/atlas-graph-contract/a.feature \
  && git add -A >/dev/null 2>&1 && git commit -qm prose )
run_case "B1 prose, no bump -> refused" fail "$D" "$BASE"
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

# B5/B6: a fixture re-blessed vs merely reformatted.
D="$TMPROOT/b5"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm rebless )
run_case "B5 fixture re-blessed to a new value, declared MINOR -> refused" fail "$D" "$BASE" "re-blessed to a different value"

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
run_case "B7 scenario deleted, declared MINOR -> refused" fail "$D" "$BASE" "no longer exist"
( cd "$D" && printf '2.0.0\n' > contracts/atlas-graph-contract/VERSION \
  && git add -A >/dev/null 2>&1 && git commit -qm major )
run_case "B8 scenario deleted, declared MAJOR -> accepted" pass "$D" "$BASE"

# B9/B10: @target in both directions.
D="$TMPROOT/b9"; BASE="$(new_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  Scenario: one','  @target\n  Scenario: one',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm target )
run_case "B9 @target ADDED, declared MINOR -> refused" fail "$D" "$BASE" "@target was added"

D="$TMPROOT/b10"
mkdir -p "$D/contracts/atlas-graph-contract/fixtures" "$D/scripts"
cp "$ROOT/scripts/contract-semver-gate.sh" "$D/scripts/"
( cd "$D"
  git init -q .; git config user.email t@t; git config user.name t
  printf '1.2.3\n' > contracts/atlas-graph-contract/VERSION
  printf '# c\n' > contracts/atlas-graph-contract/CHANGELOG.md
  printf 'contracts/atlas-graph-contract  contract-runner\n' > contracts/SUITES
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

# B11: whole-suite deletion.
D="$TMPROOT/b11"; BASE="$(new_repo "$D")"
( cd "$D" && git rm -rq contracts/atlas-graph-contract && git commit -qm "delete the whole suite" )
run_case "B11 whole suite DELETED -> refused (H-2)" fail "$D" "$BASE" "entire suite was DELETED"

# B12/B13: the widening grade, and that it cannot smuggle a re-bless.
D="$TMPROOT/b12"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 1,\n  "b": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm widen )
run_case "B12 fixture WIDENED, declared MINOR -> accepted" pass "$D" "$BASE"

D="$TMPROOT/b13"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{\n  "a": 99,\n  "b": 2\n}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm widen-and-change )
run_case "B13 widened AND an old value changed, declared MINOR -> refused" fail "$D" "$BASE" "re-blessed to a different value"

# B14: the multi-tag form.
D="$TMPROOT/b14"; BASE="$(new_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  Scenario: one','  @wip @target\n  Scenario: one',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- 1.3.0" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm multitag )
run_case "B14 '  @wip @target' declared MINOR -> refused (C-NEW-1)" fail "$D" "$BASE" "@target was added"

# B15: --base HEAD.
D="$TMPROOT/b15"; BASE="$(new_repo "$D")"
( cd "$D" && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm x )
run_case "B15 --base HEAD -> refused (M-NEW-1)" fail "$D" HEAD "no diff to classify"

# B16: the grader is no longer on PATH -- shadowing python must be inert.
D="$TMPROOT/b16"; BASE="$(new_repo "$D")"
( cd "$D" && printf '{"a":999,"z":"TOTALLY-DIFFERENT"}\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm rebless )
FAKEPY="$(mktemp -d)"; printf '#!/bin/sh\necho same\n' > "$FAKEPY/python"; chmod +x "$FAKEPY/python"
out="$( cd "$D" && PATH="$FAKEPY:$PATH" bash scripts/contract-semver-gate.sh --runner "$RUNNER_BIN" "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | grep -q 'MAJOR'; then
  ok "B16 python shadowed by 'echo same' -> still MAJOR (the grader is in the runner)"
else bad "B16 a fake python changed the classification"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi
# ...and the nine-line `case` that answered round 2's calibration quiz.
cat > "$FAKEPY/python" <<'FAKE'
#!/bin/sh
old="$3"; new="$4"
case "$old|$new" in
  '{"a":1}|{"a":1}')       echo same ;;
  '{"a":1}|{"a":1,"b":2}') echo wider ;;
  '{"a":1}|{"a":2}')       echo changed ;;
  *)                       echo same ;;
esac
FAKE
out="$( cd "$D" && PATH="$FAKEPY:$PATH" bash scripts/contract-semver-gate.sh --runner "$RUNNER_BIN" "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | grep -q 'MAJOR'; then
  ok "B16b the calibration-answering python shim -> still MAJOR (H-R2-2)"
else bad "B16b the H-R2-2 shim downgraded a re-bless"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi
rm -rf "$FAKEPY"

# B17: a fixture truncated to zero bytes.
D="$TMPROOT/b17"; BASE="$(new_repo "$D")"
( cd "$D" && : > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm truncate )
run_case "B17 fixture truncated to 0 bytes, declared PATCH -> refused" fail "$D" "$BASE" "was emptied"

# ---------------------------------------------------------------------
# A RECEIVED-SUITE REPO, for the three ratchets round 3 added
# ---------------------------------------------------------------------
new_received_repo() { # <dir> -> base sha
  local d="$1"
  mkdir -p "$d/contracts/their-edge/fixtures" "$d/scripts"
  cp "$ROOT/scripts/contract-semver-gate.sh" "$d/scripts/"
  ( cd "$d"
    git init -q .; git config user.email t@t; git config user.name t
    printf 'their expectations of us\n' > contracts/their-edge/RECEIVED.md
    printf 'contracts/their-edge  contract-runner\n' > contracts/SUITES
    cat > contracts/their-edge/a.feature <<'FEOF'
Feature: theirs

  Scenario: one
    When I GET /api/x
    Then the response equals fixture "x"

  Scenario: two
    When I GET /api/y
    Then the response equals fixture "y"
FEOF
    printf '{"a":1}\n' > contracts/their-edge/fixtures/x.json
    git add -A >/dev/null 2>&1; git commit -qm base
    git rev-parse HEAD )
}

# B18: whole received suite deleted (M-NEW-4).
D="$TMPROOT/b18"; BASE="$(new_received_repo "$D")"
( cd "$D" && git rm -rq contracts/their-edge && git commit -qm "delete the received suite" )
run_case "B18 RECEIVED suite deleted -> refused (M-NEW-4)" fail "$D" "$BASE" "RECEIVED suite was DELETED"

# B19: H-R2-3 -- the commit says one thing, the worktree another.
D="$TMPROOT/b19"; BASE="$(new_repo "$D")"
( cd "$D" && printf '[{"id":"ZZZ","name":"TOTALLY-DIFFERENT","to":999}]\n' > contracts/atlas-graph-contract/fixtures/x.json \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm rebless
  # ...now revert ONLY the working-tree copy. The COMMIT is untouched.
  printf '{\n  "a": 1\n}\n' > contracts/atlas-graph-contract/fixtures/x.json )
run_case "B19 re-bless in the COMMIT, reverted in the WORKTREE -> still MAJOR (H-R2-3)" fail "$D" "$BASE" "re-blessed to a different value"

# B20: H-R2-4 -- a TAB inside a scenario name used to hide @target from the
# classifier by shifting the awk column.
D="$TMPROOT/b20"
mkdir -p "$D/contracts/atlas-graph-contract/fixtures" "$D/scripts"
cp "$ROOT/scripts/contract-semver-gate.sh" "$D/scripts/"
( cd "$D"
  git init -q .; git config user.email t@t; git config user.name t
  printf '1.2.3\n' > contracts/atlas-graph-contract/VERSION
  printf '# c\n' > contracts/atlas-graph-contract/CHANGELOG.md
  printf 'contracts/atlas-graph-contract  contract-runner\n' > contracts/SUITES
  python -c "
import io
io.open('contracts/atlas-graph-contract/a.feature','w',encoding='utf-8',newline='\n').write(
  'Feature: f\n  prose\n\n  Scenario: alpha\tbeta gamma\n    When I GET /api/x\n    Then the response equals fixture \"x\"\n')
"
  printf '{"a":1}\n' > contracts/atlas-graph-contract/fixtures/x.json
  git add -A >/dev/null 2>&1; git commit -qm base )
BASE="$( cd "$D" && git rev-parse HEAD )"
( cd "$D" && python -c "
import io
p='contracts/atlas-graph-contract/a.feature'
s=io.open(p,encoding='utf-8').read().replace('  Scenario: alpha','  @target\n  Scenario: alpha',1)
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && printf '1.3.0\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm tabname )
run_case "B20 @target on a scenario whose NAME contains a TAB -> refused (H-R2-4)" fail "$D" "$BASE" "@target was added"

# B21: C-R2-1 -- the marker removed while the directory stays.
D="$TMPROOT/b21"; BASE="$(new_received_repo "$D")"
( cd "$D" && git rm -q contracts/their-edge/RECEIVED.md && git commit -qm "the marker duplicated what the directory name says" )
run_case "B21 RECEIVED.md removed, directory intact -> refused (C-R2-1)" fail "$D" "$BASE" "marker was REMOVED"

# B22: C-R2-3 -- the corpus renamed away, directory and marker intact.
D="$TMPROOT/b22"; BASE="$(new_received_repo "$D")"
( cd "$D" && git mv contracts/their-edge/a.feature contracts/their-edge/a.feature.bak \
  && git commit -qm "park the vendored corpus" )
run_case "B22 received corpus RENAMED away, dir+marker intact -> refused (C-R2-3)" fail "$D" "$BASE" "are GONE from a RECEIVED suite"

# B22b: ...and the subtler form -- one scenario removed from a file that stays.
D="$TMPROOT/b22b"; BASE="$(new_received_repo "$D")"
( cd "$D" && python -c "
import io
p='contracts/their-edge/a.feature'
s=io.open(p,encoding='utf-8').read().split('  Scenario: two')[0]
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && git add -A >/dev/null 2>&1 && git commit -qm "drop one" )
run_case "B22b ONE scenario dropped from a received suite -> refused" fail "$D" "$BASE" "are GONE from a RECEIVED suite"

# B23: C-R2-2 -- the harness ratchet, in three directions.
D="$TMPROOT/b23"; BASE="$(new_received_repo "$D")"
( cd "$D" && printf 'contracts/their-edge  contract-runner-consumer\n' > contracts/SUITES \
  && git add -A >/dev/null 2>&1 && git commit -qm "the provider run belongs in the API batch" )
run_case "B23 harness WEAKENED contract-runner -> consumer -> refused (C-R2-2)" fail "$D" "$BASE" "harness was WEAKENED"

D="$TMPROOT/b23b"; BASE="$(new_received_repo "$D")"
( cd "$D" && printf '# nothing here\n' > contracts/SUITES \
  && git add -A >/dev/null 2>&1 && git commit -qm "tidy the registry" )
run_case "B23b registry ROW DELETED while the suite lives -> refused" fail "$D" "$BASE" "harness was WEAKENED"

D="$TMPROOT/b23c"; BASE="$(new_repo "$D")"
( cd "$D" && printf 'contracts/atlas-graph-contract  contract-runner\ncontracts/new-thing  contract-runner\n' > contracts/SUITES \
  && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm "register another suite" )
run_case "B23c the registry GROWS -> accepted (the ratchet is one-way, not frozen)" pass "$D" "$BASE"

# B24: the aqc-dual count floor.
D="$TMPROOT/b24"
mkdir -p "$D/contracts/dual/features" "$D/scripts"
cp "$ROOT/scripts/contract-semver-gate.sh" "$D/scripts/"
( cd "$D"
  git init -q .; git config user.email t@t; git config user.name t
  printf 'contracts/dual  aqc-dual\n' > contracts/SUITES
  cat > contracts/dual/features/x.feature <<'FEOF'
Feature: dual
  Scenario Outline: one
    Given <a>
  Scenario Outline: two
    Given <a>
FEOF
  git add -A >/dev/null 2>&1; git commit -qm base )
BASE="$( cd "$D" && git rev-parse HEAD )"
( cd "$D" && python -c "
import io
p='contracts/dual/features/x.feature'
s=io.open(p,encoding='utf-8').read().split('  Scenario Outline: two')[0]
io.open(p,'w',encoding='utf-8',newline='\n').write(s)
" && git add -A >/dev/null 2>&1 && git commit -qm shrink )
run_case "B24 an aqc-dual suite's committed scenario count FALLS -> refused" fail "$D" "$BASE" "scenario count FELL"

# B25: the symlink question, settled by MODE rather than by the filesystem.
# `git update-index --add --cacheinfo 120000,...` makes a symlink entry on
# Windows without a filesystem symlink, which is why the round-2 reviewer's
# probe was inconclusive and this one is not.
D="$TMPROOT/b25"; BASE="$(new_repo "$D")"
( cd "$D"
  target_sha="$(printf '../../../etc/passwd' | git hash-object -w --stdin)"
  git update-index --add --cacheinfo "120000,$target_sha,contracts/atlas-graph-contract/evil.feature"
  git commit -qm "a symlink in the corpus" >/dev/null 2>&1 )
run_case "B25 a SYMLINK-mode .feature committed under contracts/ -> refused" fail "$D" "$BASE" "not regular files"

# B26: --runner is required. Round 2 made it optional, and the fail-closed
# branch that resulted made several tag cases pass for the wrong reason.
D="$TMPROOT/b26"; BASE="$(new_repo "$D")"
( cd "$D" && printf '1.2.4\n' > contracts/atlas-graph-contract/VERSION \
  && echo "- x" >> contracts/atlas-graph-contract/CHANGELOG.md \
  && git add -A >/dev/null 2>&1 && git commit -qm x )
out="$( cd "$D" && bash scripts/contract-semver-gate.sh "$BASE" 2>&1 )"; rc=$?
if [ "$rc" -ne 0 ] && printf '%s\n' "$out" | grep -qF 'is required'; then
  ok "B26 no --runner -> refused outright, not classified without an oracle"
else bad "B26 the semver gate ran without an oracle"; printf '%s\n' "$out" | sed 's/^/        /' >&2; fi

# B27: NEW-CLASS -- a suite RENAMED wholesale (directory moved). The VERSION
# moves with it, so `list_versioned_suites` sees a deletion and an addition.
D="$TMPROOT/b27"; BASE="$(new_repo "$D")"
( cd "$D" && git mv contracts/atlas-graph-contract contracts/atlas-graph-contract-v2 \
  && printf 'contracts/atlas-graph-contract-v2  contract-runner\n' > contracts/SUITES \
  && git add -A >/dev/null 2>&1 && git commit -qm "rename the suite" )
run_case "B27 NEW-CLASS: the whole suite directory RENAMED -> refused" fail "$D" "$BASE" "entire suite was DELETED"

echo
echo "================ C. THE ORACLE ITSELF (golden vectors) ================"
# The gate's whole design is "derive everything from committed content", and
# every derivation runs through ONE binary the gate builds from source IN
# THIS REPOSITORY. That is the largest residual this round did not remove: a
# commit that weakens Gherkin/Parse.hs, Tags.hs or Grade.hs weakens every
# derivation at once, and deriving-from-git cannot help, because git is
# exactly where such a change would live.
#
# It cannot be removed. It can be made LOUD. These cases pin the oracle's
# byte-exact answer on a corpus containing every form that has ever defeated
# a guard in this project, so weakening it requires editing the golden file
# in the same commit -- and "the parser no longer sees @target in
# '@wip @target', and here is the updated expected output" is a diff nobody
# skims past.
ORACLE="contracts/runner/testdata/oracle"

head2 "C1: the tag oracle's answer is byte-identical to the committed golden"
got="$(mktemp)"
"$RUNNER_BIN" tags "$ORACLE" 2>/dev/null | tr -d '\r' | sed 's|\\|/|g' > "$got"
if diff -u "$ORACLE/EXPECTED.tags" "$got" > "$(lg c1diff)" 2>&1; then
  ok "C1 the oracle answers exactly as committed ($(awk 'END{print NR}' "$got") rows)"
else
  bad "C1 THE ORACLE MOVED -- the gate's whole trusted base answers differently:"
  sed 's/^/        /' "$(lg c1diff)" | head -20 >&2
fi

# ...and the properties, asserted structurally rather than by restating the
# golden. A golden file alone is a ratchet and not a specification: it would
# happily pin a WRONG answer. These say what the answer must MEAN.
head2 "C2: the properties the golden is supposed to encode"
tags_of() { # <scenario name> -> its tag list, field 3, no re-lexing
  awk -F'\t' -v n="$1" '$2==n {print $3}' "$got"
}
[ "$(tags_of 'the multi-tag form that defeated the regex')" = "featurelevel,target,wip" ] \
  && ok "C2a '  @wip @target' yields BOTH tags (C-NEW-1)" \
  || bad "C2a the multi-tag form does not yield @target: '$(tags_of 'the multi-tag form that defeated the regex')'"
[ "$(tags_of 'the control -- one word, not two tags')" = "featurelevel,wip@target" ] \
  && ok "C2b '  @wip@target' is ONE tag, not two -- the oracle is exactly as wide as Run.hs" \
  || bad "C2b the control form was mis-lexed: '$(tags_of 'the control -- one word, not two tags')'"
[ "$(tags_of 'alpha%09beta gamma')" = "target" ] \
  && ok "C2c a TAB inside a scenario NAME leaves the tag list in field 3 (H-R2-4)" \
  || bad "C2c the tab-named scenario's tags are not in field 3: '$(tags_of 'alpha%09beta gamma')'"
[ "$(tags_of 'a name with a percent %2525 in it')" = "" ] \
  && ok "C2d a literal % is escaped invertibly (%25 -> %2525), so no name can forge another" \
  || bad "C2d percent escaping is not invertible"
awk -F'\t' '$1 ~ /a\.feature$/ && $3 !~ /(^|,)featurelevel(,|$)/ {n++} END{exit (n+0)>0}' "$got" \
  && ok "C2e a FEATURE-level tag reaches every scenario in its file (the safe over-approximation)" \
  || bad "C2e a feature-level tag did not reach every scenario"
"$RUNNER_BIN" tags "$ORACLE" --forbid target >/dev/null 2>&1 \
  && bad "C2f --forbid target exited 0 on a corpus that plainly carries @target" \
  || ok "C2f --forbid target exits non-zero on this corpus (leg 0 uses the exit code)"

head2 "C3: the fixture grader's own vectors"
gv() { # <old json> <new json>
  local o n; o="$(mktemp)"; n="$(mktemp)"
  printf '%s' "$1" > "$o"; printf '%s' "$2" > "$n"
  "$RUNNER_BIN" grade "$o" "$n" 2>/dev/null | tr -d '\r'
  rm -f "$o" "$n"
}
grade_is() { # <desc> <expected> <old> <new>
  local g; g="$(gv "$3" "$4")"
  [ "$g" = "$2" ] && ok "C3 $1 -> $2" || bad "C3 $1 -> got '$g', expected '$2'"
}
grade_is "identical"                     same    '{"a":1}'            '{"a":1}'
grade_is "reformatted, same parsed value" same   '{"a":1}'            '{ "a" : 1 }'
grade_is "a key added, old values intact" wider  '{"a":1}'            '{"a":1,"b":2}'
grade_is "a value re-blessed"            changed '{"a":1}'            '{"a":2}'
grade_is "a key REMOVED"                 changed '{"a":1,"b":2}'      '{"a":1}'
grade_is "widened AND a value changed"   changed '{"a":1}'            '{"a":9,"b":2}'
grade_is "a nested key added"            wider   '{"a":{"b":1}}'      '{"a":{"b":1,"c":2}}'
grade_is "a nested value changed"        changed '{"a":{"b":1}}'      '{"a":{"b":9}}'
grade_is "an array element appended"     changed '[1,2]'              '[1,2,3]'
grade_is "an array element changed"      changed '[1,2]'              '[1,9]'
grade_is "an object added inside an array" wider '[{"a":1}]'          '[{"a":1,"b":2}]'
grade_is "not JSON at all"               unreadable '{"a":1}'         'not json'
# The shape that matters most: `same` is a VALID answer, which is why round
# 2's answer-shape validation could not work and why calibration was tried
# instead. What makes this trustworthy is not a quiz -- it is that the
# grader is compiled from source in the diff, and these vectors pin it.
grade_is "empty object vs empty object"  same    '{}'                 '{}'
grade_is "empty object widened"          wider   '{}'                 '{"a":1}'

echo
echo "=============================================================="
echo "gate self-test: $pass passed, $failn failed"
[ "$failn" -eq 0 ] || exit 1
