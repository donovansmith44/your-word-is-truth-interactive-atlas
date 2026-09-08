#!/usr/bin/env bash
# THE CONTRACT GATE (batch CDC-1; hardened in fix round 1).
#
# Owner order 2: "we will get to a point where we cannot push or do
# anything if our contracts don't agree. that is target shape."
#
# ---------------------------------------------------------------------
# WHAT THE FIRST ROUND GOT WRONG, recorded here because the header is
# where the false claim lived
# ---------------------------------------------------------------------
# This script used to assert it "reads no environment variable that can
# weaken it" -- six lines above `CARGO="${CARGO:-...}"`. The review
# demonstrated `CARGO=/usr/bin/true bash scripts/contract-gate.sh` printing
# CONTRACT GATE: PASSED with leg 3 never executing a line of Rust. `CABAL`
# was the same hole one level up, and an inherited `ATLAS_BLESS_PACT=1` was
# a third: both recorders wrote the pact and returned green.
#
# That claim is not repeated here. What is claimed now is only what
# `scripts/gate-selftest.sh` ATTEMPTS and fails to do -- every bypass is
# shipped as a test that tries to lie to the gate and asserts the gate
# refuses.
#
# FIX ROUND 2 found three more, all one species: A GUARD NARROWER THAN THE
# THING IT GUARDS. A line-anchored `@target` regex against a parser that
# reads tags word-by-word; a registry that *classifies* against a filesystem
# that *detects*; a tool's self-description against a tool's identity; and a
# fingerprint that failed OPEN when its hasher was missing. The rule this
# file now follows is: guard the PROPERTY, not the spelling -- and where a
# guard must parse something, use the parser the consumer uses.
#
# The defences, in order of how much they carry:
#   1. leg 3 runs with ATLAS_BLESS_PACT stripped, and the recorders
#      themselves FAIL when blessed (a bless can never be a green test);
#   2. leg 3 must PROVE IT RAN -- `test result: ok. N passed`, N >= 1, from
#      each recorder. A tool that does no work cannot pass, whatever it
#      calls itself, which is what a `--version` check never gave us;
#   3. a before/after fingerprint of contracts/pacts, FAIL-CLOSED: a missing
#      hasher or an empty fingerprint is a failure, not a silent pass;
#   4. leg 0 and the semver classifier ask `contract-runner tags` -- the
#      vendored parser -- which scenarios carry @target. No regex;
#   5. suites are DETECTED from the filesystem and the registry is a CACHE OF
#      A DERIVATION the gate re-checks: RECEIVED.md implies contract-runner,
#      and aqc-dual is only permitted where the parser genuinely cannot read
#      the corpus. A one-word registry edit can no longer exempt a suite.
#
# There is no --allow, no advisory mode and no skip flag. `--fast` skips
# only leg 3 and EXITS 3, never 0: it reports success for what it ran and
# failure for what it did not, so it is usable while iterating and unusable
# as a final answer.
#
# Usage:
#   scripts/contract-gate.sh              full gate (what pre-push runs)
#   scripts/contract-gate.sh --fast       skip the recorders while iterating
#   scripts/contract-gate.sh --base REF   semver base (default: auto)
set -uo pipefail
cd "$(dirname "$0")/.."

FAST=0
BASE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --fast) FAST=1; shift;;
    --base) BASE="${2:?--base needs a ref}"; shift 2;;
    *) echo "unknown argument: $1" >&2; exit 2;;
  esac
done

fail=0
step() { printf '\n=== %s ===\n' "$1"; }
check() { if [ "$1" -ne 0 ]; then echo "FAILED: $2" >&2; fail=1; fi; }

# ---------------------------------------------------------------------
# TOOLCHAIN: a greeting is not an identity (fix round 2, review H-NEW-1)
# ---------------------------------------------------------------------
# Fix round 1 answered `CARGO=/usr/bin/true` by requiring `$CARGO --version`
# to begin with the word "cargo". That is a statement the tool makes about
# ITSELF, and the reviewer wrote a three-line shim that makes it:
#
#     #!/bin/sh
#     if [ "$1" = "--version" ]; then echo "cargo 1.97.1 (...)"; exit 0; fi
#     exit 0
#
# -> CONTRACT GATE: PASSED, with leg 3 executing no Rust at all.
#
# The name check stays as a cheap early diagnostic, but it is no longer what
# the gate rests on. What the gate rests on is EVIDENCE THAT THE LEG RAN:
# leg 3 captures its output and requires `test result: ok. N passed` with
# N >= 1 from each recorder. A tool that does no work cannot produce that,
# whatever it calls itself -- and this also catches the accidental cases the
# reviewer rightly weighted (a wrapper that swallows a subcommand, or fails
# to propagate an exit code), which no amount of `--version` checking would.
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
CABAL="${CABAL:-$HOME/.local/bin/cabal}"

verify_tool() {
  local bin="$1" want="$2" out
  if ! out="$("$bin" --version 2>/dev/null)"; then
    echo "FAILED: $want at '$bin' could not be executed." >&2
    echo "  A gate cannot be run by a tool that does not exist." >&2
    return 1
  fi
  case "$out" in
    "$want"*) return 0;;
    *)
      echo "FAILED: '$bin' does not identify itself as $want." >&2
      echo "  It said: $(printf '%s' "$out" | head -1)" >&2
      echo "  Refusing to run a gate leg through a substituted toolchain." >&2
      return 1;;
  esac
}

# A recorder must PROVE it ran. `cargo test` printing nothing, or printing a
# summary with zero tests, is not a pass -- it is a leg that did not happen.
require_test_evidence() { # <label> <logfile>
  local label="$1" log="$2" n
  n="$(grep -oE '^test result: ok\. [0-9]+ passed' "$log" 2>/dev/null \
       | grep -oE '[0-9]+' | awk '{s+=$1} END{print s+0}')"
  if [ "${n:-0}" -lt 1 ]; then
    echo "FAILED: $label produced no evidence that it ran." >&2
    echo "  Expected 'test result: ok. N passed' with N >= 1; found none." >&2
    echo "  A leg that runs nothing writes nothing, and must never read as a pass." >&2
    sed 's/^/    /' "$log" | tail -15 >&2
    return 1
  fi
  echo "  $label: $n test(s) passed"
  return 0
}

step "toolchain"
verify_tool "$CARGO" cargo || exit 1
verify_tool "$CABAL" cabal || exit 1
echo "cargo and cabal name-check passed (leg 3 additionally requires proof of work)"

step "building the contract runner"
( cd contracts/runner && "$CABAL" build all >/dev/null 2>&1 )
check $? "contracts/runner does not build"
RUNNER="$(cd contracts/runner && "$CABAL" list-bin contract-runner 2>/dev/null)"
if [ -z "${RUNNER:-}" ] || [ ! -x "$RUNNER" ]; then
  echo "FAILED: no contract-runner binary" >&2
  exit 1
fi

# ---------------------------------------------------------------------
# SUITES: DETECTED from the filesystem, CLASSIFIED by contracts/SUITES
# (review C-2)
# ---------------------------------------------------------------------
# A literal array is how `contracts/atlas-edge` came to be excluded from
# every tag check. So detection is the filesystem's job -- a suite is the
# nearest ancestor of a .feature file carrying VERSION or RECEIVED.md -- and
# nothing can escape it.
#
# But the filesystem cannot say WHICH executor owns a suite, and the first
# version of this loop happily pointed the Haskell runner at the AQC corpus
# (whose steps live in two other harnesses) and produced a wall of reds for
# a perfectly healthy suite. `scripts/gate-selftest.sh` caught that on its
# first run. So classification lives in `contracts/SUITES`, and an
# unregistered suite is a HARD FAILURE: adding one without declaring who
# runs it cannot be done quietly, which is the property the literal array
# lacked.
REGISTRY="contracts/SUITES"
[ -f "$REGISTRY" ] || { echo "FAILED: no $REGISTRY -- the gate cannot tell which harness owns which suite" >&2; exit 1; }

harness_of() { # <dir>
  awk -v want="$1" '$1 !~ /^#/ && NF>=2 && $1==want {print $2; found=1} END{if(!found) print ""}' "$REGISTRY" | head -1
}

suite_root() {
  local d="$1"
  while [ -n "$d" ] && [ "$d" != "." ] && [ "$d" != "contracts" ]; do
    if [ -f "$d/VERSION" ] || [ -f "$d/RECEIVED.md" ]; then printf '%s\n' "$d"; return; fi
    d="$(dirname "$d")"
  done
  printf '%s\n' "$1"
}

ROOTS=()
while IFS= read -r f; do
  [ -z "$f" ] && continue
  r="$(suite_root "$(dirname "$f")")"
  case " ${ROOTS[*]:-} " in *" $r "*) ;; *) ROOTS+=("$r");; esac
done < <(find contracts -name '*.feature' -not -path 'contracts/runner/*' 2>/dev/null | sort)

if [ "${#ROOTS[@]}" -eq 0 ]; then
  echo "FAILED: no contract suites found under contracts/ -- an empty corpus is a broken gate, not a passing one" >&2
  exit 1
fi

# ---------------------------------------------------------------------
# THE REGISTRY IS A CACHE OF A DERIVATION, NOT AN ASSERTION THE GATE OBEYS
# (fix round 2, review C-NEW-2)
# ---------------------------------------------------------------------
# Fix round 1 replaced a literal bash array with this registry file and
# called C-2 closed. It was not: the reviewer changed ONE WORD --
# `contracts/atlas-edge  contract-runner` to `aqc-dual` -- and the full gate
# printed PASSED with map-generator's fixture violated AND a plain `@target`
# planted, because `aqc-dual` means "some other harness runs this" and no leg
# looks at a suite it has been told is not its business. The array had simply
# become a line in a file, still ungated.
#
# The property that mattered was never "suites are enumerated from the
# filesystem". It was "a suite we RECEIVED cannot be silently exempted" --
# and that is derivable from the suites themselves, so the gate derives it
# and treats the registry as a cache it re-checks:
#
#   * RECEIVED.md present  =>  harness MUST be contract-runner. Another
#     repo's expectations of us are executed here or the gate fails. There is
#     no registration that exempts them, the same way there is no @target
#     that disarms them.
#   * aqc-dual is permitted ONLY where the vendored parser genuinely cannot
#     run the suite -- i.e. it uses `Scenario Outline`, which
#     `Gherkin/Parse.hs` does not implement. That is a fact about the corpus,
#     checkable on every run, not a claim in a file.
#
# So the plausible-sounding commit the reviewer imagined ("atlas-edge runs in
# map-generator's CI, not ours") now fails on the next line.
RUN_ROOTS=()      # we are the provider: legs 0,1,2,4
CHECK_ROOTS=()    # we are the consumer: legs 0,1,2 only
for d in "${ROOTS[@]}"; do
  h="$(harness_of "$d")"

  # --- the derivation, checked before the declaration is honoured ---
  if [ -f "$d/RECEIVED.md" ] && [ "$h" != "contract-runner" ]; then
    echo "FAILED: $d carries RECEIVED.md but is registered '$h' in $REGISTRY." >&2
    echo "  A suite we received is another repo's expectations OF US. It is executed" >&2
    echo "  here or the gate fails; there is no registration that exempts it." >&2
    fail=1
    h=contract-runner   # gate it anyway, so this run still executes it
  fi
  if [ "$h" = "aqc-dual" ] && ! grep -rq 'Scenario Outline' "$d" 2>/dev/null; then
    echo "FAILED: $d is registered 'aqc-dual' but contains no 'Scenario Outline'." >&2
    echo "  aqc-dual exists for corpora the vendored Gherkin parser cannot read." >&2
    echo "  A suite it CAN read must be run by it, not excused from it." >&2
    fail=1
  fi

  case "$h" in
    contract-runner)          RUN_ROOTS+=("$d"); CHECK_ROOTS+=("$d");;
    contract-runner-consumer) CHECK_ROOTS+=("$d");;
    aqc-dual)                 ;;  # run by its own two harnesses, not here
    "")
      echo "FAILED: the suite $d is not registered in $REGISTRY." >&2
      echo "  Every suite must declare which harness runs it, so a new one cannot" >&2
      echo "  silently escape the gate the way contracts/atlas-edge once did." >&2
      fail=1;;
    *)
      echo "FAILED: the suite $d declares unknown harness '$h' in $REGISTRY." >&2
      fail=1;;
  esac
done
# A registered suite whose directory has vanished is a deletion; the semver
# gate classifies the versioned ones, but say it here too so a deletion is
# never merely a shorter list.
while read -r rdir rharness; do
  case "$rdir" in ''|\#*) continue;; esac
  [ -n "${rharness:-}" ] || continue
  if [ ! -d "$rdir" ]; then
    echo "FAILED: $REGISTRY lists $rdir, which does not exist." >&2
    echo "  Deleting a contract suite is a cross-consumer break, not a tidy-up." >&2
    fail=1
  fi
done < <(grep -vE '^[[:space:]]*(#|$)' "$REGISTRY")

echo "suites (executed): ${RUN_ROOTS[*]:-none}"
echo "suites (checked only): ${CHECK_ROOTS[*]:-none}"

# ---------------------------------------------------------------------
# LEG 0 -- A RECEIVED SUITE MAY NOT BE DISARMED (review C-2)
# ---------------------------------------------------------------------
# `@target` is a legitimate disclosure mechanism for OUR OWN suites: the
# scenario runs, is printed red every time, and does not fail the gate. On
# a suite we RECEIVED it is something else -- a one-line way to withdraw
# another repo's guarantee, in a directory whose own RECEIVED.md says the
# rule is "break, report, and coordinate; editing one of these files to
# make our build green would be forging the other side's signature".
#
# The review demonstrated it: plant a real violation of map-generator's own
# fixture, add `@target` above the scenario, and legs 1, 2, 4 and 5 all go
# green with their expectation of us broken. Leg 5 never looks at
# atlas-edge (correctly -- we do not declare their version), so nothing
# objected.
#
# There is no bump class for this. It is simply forbidden.
#
# THE GUARD ASKS THE PARSER, NOT grep (fix round 2, review C-NEW-1). This
# used to be `grep -rnE '^[[:space:]]*@target([[:space:]]|$)'`, anchored at
# the start of a line -- while `Gherkin/Parse.hs:27` makes EVERY
# whitespace-separated word on a tag line a tag, and `Run.hs`'s `isTarget`
# tests membership, not position. So `  @wip @target` disarmed a received
# suite while the grep saw nothing, and the reviewer got CONTRACT GATE:
# PASSED with map-generator's own fixture violated.
#
# A cleverer regex would only move the goalposts to the next form nobody has
# typed yet. `contract-runner tags --forbid` calls the same `parseFeature`
# the executor calls, so the guard is exactly as wide as the thing it
# guards -- and an unparseable file fails it too, rather than reading as
# "no forbidden tags here".
step "leg 0/6: received suites carry no @target"
disarmed=0
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  [ -f "$d/RECEIVED.md" ] || continue
  if ! hits="$("$RUNNER" tags "$d" --forbid target 2>&1)"; then
    echo "FAILED: the RECEIVED suite $d is disarmed or unparseable -- we may not withdraw another repo's guarantee:" >&2
    printf '%s\n' "$hits" | grep -E 'FORBIDDEN|^tags:' | sed 's/^/    /' >&2
    echo "  If their expectation of us is genuinely wrong: break, report, and coordinate a bump on BOTH sides." >&2
    fail=1; disarmed=1
  fi
done
[ "$disarmed" -eq 0 ] && echo "no received suite is disarmed"

step "leg 1/6: totality (check)"
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  "$RUNNER" check "$d"; check $? "totality: $d"
done

step "leg 2/6: vocabulary drift (vocab)"
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  "$RUNNER" vocab "$d"; check $? "vocabulary drift: $d"
done

# ---------------------------------------------------------------------
# LEG 3 -- PROVIDER DRIFT.
# ---------------------------------------------------------------------
if [ "$FAST" -eq 0 ]; then
  step "leg 3/6: provider drift (the recorders)"

  # THE LOAD-BEARING CHECK, and it is a BEFORE/AFTER comparison rather than
  # `git diff`. Whatever wrote the pact, and however it was triggered: if
  # the evidence moved WHILE THE GATE RAN, the gate did not pass.
  #
  # `git diff --quiet -- contracts/pacts` was the first attempt and it is
  # subtly wrong -- it cannot tell "rewritten during this run" from "already
  # uncommitted", so it failed the gate for anyone mid-way through a
  # legitimate re-record. The self-test caught that immediately (A3b went
  # red for the wrong reason). Hashing the files either side answers the
  # question actually being asked, and needs no clean tree.
  # FAIL-CLOSED (fix round 2, review M-NEW-2). Every failure mode of the old
  # pipeline -- no sha256sum (stock macOS ships `shasum`), no find, no xargs,
  # no contracts/pacts -- yielded the EMPTY STRING on both sides, and empty
  # equals empty, so "the evidence did not move" was the answer whenever the
  # check could not run at all. Both `2>/dev/null`s guaranteed the failure
  # was silent. A check that is off cannot be a check that passes.
  pact_fingerprint() {
    find contracts/pacts -type f -name '*.json' -print0 2>/dev/null \
      | sort -z | xargs -0 "$HASHER" | "$HASHER"
  }
  HASHER=""
  for h in sha256sum shasum sha1sum md5sum; do
    if command -v "$h" >/dev/null 2>&1; then HASHER="$h"; break; fi
  done
  if [ -z "$HASHER" ]; then
    echo "FAILED: no hashing tool (sha256sum/shasum/sha1sum/md5sum) is available." >&2
    echo "  The pact fingerprint is what proves the gate did not rewrite its own" >&2
    echo "  evidence; without it the check would silently be off." >&2
    fail=1
  fi
  before=""
  if [ -n "$HASHER" ]; then
    before="$(pact_fingerprint)"
    if [ -z "$before" ]; then
      echo "FAILED: could not fingerprint contracts/pacts (empty result from $HASHER)." >&2
      fail=1
    fi
  fi

  http_log="$(mktemp)"; cli_log="$(mktemp)"
  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-server --test contract_pact ) >"$http_log" 2>&1
  check $? "provider drift: the HTTP pact no longer matches the live graph"
  require_test_evidence "leg 3 (HTTP recorder)" "$http_log" || fail=1

  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-cli --test contract_pact_cli ) >"$cli_log" 2>&1
  check $? "provider drift: the CLI pact no longer matches the real bibex binary"
  require_test_evidence "leg 3 (CLI recorder)" "$cli_log" || fail=1
  rm -f "$http_log" "$cli_log"

  if [ -n "$HASHER" ] && [ -n "$before" ]; then
    after="$(pact_fingerprint)"
    if [ -z "$after" ] || [ "$before" != "$after" ]; then
      echo "FAILED: contracts/pacts was REWRITTEN while this gate was running (or could not be re-read)." >&2
      echo "  The gate verifies evidence; it must never be the thing that produces it." >&2
      echo "  Re-record deliberately, review the diff, then run the gate against it." >&2
      fail=1
    fi
  fi
else
  step "leg 3/6: SKIPPED (--fast)"
fi

# ---------------------------------------------------------------------
# LEG 4 -- EXPECTATION DRIFT.
# ---------------------------------------------------------------------
# Only suites we PROVIDE are executed. `contracts/map-api-consumer` is OUR
# expectations of map-generator's API: we are not its provider and its
# server is not ours to start, so it is held to legs 0, 1, 2 and 5 only.
step "leg 4/6: expectations (run --replay)"
for d in "${RUN_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  "$RUNNER" run --replay contracts/pacts --exports data/exports "$d"
  check $? "expectations: $d"
done

# ---------------------------------------------------------------------
# LEG 5 -- VERSION NEGOTIATION.
# ---------------------------------------------------------------------
# Base resolution (review M-1): the recommended pre-push hook used to fail
# 100% of the time, because it defaulted to origin/main and neither
# origin/main nor main resolves in this worktree. A gate that never passes
# is a gate someone uninstalls, so the chain is tried in order and only a
# total failure is fatal.
step "leg 5/6: semver"
resolve_base() {
  local b out
  for b in "$BASE" "@{upstream}" origin/main origin/master main master; do
    [ -z "$b" ] && continue
    git rev-parse --verify --quiet "$b" >/dev/null 2>&1 || continue
    case "$b" in
      main|master|origin/main|origin/master)
        out="$(git merge-base HEAD "$b" 2>/dev/null)" && [ -n "$out" ] && { printf '%s\n' "$out"; return 0; };;
      *)
        out="$(git rev-parse "$b" 2>/dev/null)" && [ -n "$out" ] && { printf '%s\n' "$out"; return 0; };;
    esac
  done
  # Last resort: the previous commit. Weaker than a branch point, but it
  # still classifies the diff it can see, and it is the difference between
  # a hook that runs and a hook that gets uninstalled.
  out="$(git rev-parse --verify --quiet HEAD~1)" && [ -n "$out" ] && { printf '%s\n' "$out"; return 0; }
  return 1
}
RESOLVED="$(resolve_base || true)"
if [ -z "${RESOLVED:-}" ]; then
  echo "FAILED: no base commit could be resolved for the semver gate (not even HEAD~1)." >&2
  fail=1
else
  echo "semver base: $RESOLVED"
  bash scripts/contract-semver-gate.sh --runner "$RUNNER" "$RESOLVED"; check $? "contract semver gate"
fi

printf '\n'
if [ "$fail" -ne 0 ]; then
  echo "CONTRACT GATE: FAILED" >&2
  exit 1
fi
if [ "$FAST" -eq 1 ]; then
  echo "CONTRACT GATE (--fast): legs 0, 1, 2, 4, 5 passed; leg 3 NOT RUN."
  echo "  Run without --fast before pushing." >&2
  exit 3
fi
echo "CONTRACT GATE: PASSED"
