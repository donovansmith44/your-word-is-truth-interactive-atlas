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
# The four defences, in order of how much they carry:
#   1. leg 3 runs with ATLAS_BLESS_PACT stripped from its environment, and
#      the recorders themselves now FAIL when blessed (a bless can never be
#      a green test);
#   2. after leg 3, `git diff --quiet -- contracts/pacts` -- if the evidence
#      moved while the gate ran, the gate failed, whatever wrote it. One
#      line that catches every present and future re-record path;
#   3. the toolchains are VERIFIED, not accepted: `cargo --version` must say
#      cargo, `cabal --version` must say cabal;
#   4. suites are DETECTED from the filesystem and CLASSIFIED by
#      contracts/SUITES, and an unregistered suite is a hard failure -- so a
#      new, renamed or received suite cannot silently escape legs 0, 1, 2
#      and 4 the way contracts/atlas-edge once did.
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
# TOOLCHAIN: verified, not accepted (review C-1)
# ---------------------------------------------------------------------
# Honouring $CARGO/$CABAL is genuinely useful (a rustup shim, a pinned
# cabal), so the fix is not to stop reading them -- it is to stop trusting
# them. A tool that does not identify itself as the tool it claims to be
# cannot run a leg of this gate.
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

step "toolchain"
verify_tool "$CARGO" cargo || exit 1
verify_tool "$CABAL" cabal || exit 1
echo "cargo and cabal verified"

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

# Suites this gate's own executor runs. Anything unregistered stops the gate.
RUN_ROOTS=()      # we are the provider: legs 0,1,2,4
CHECK_ROOTS=()    # we are the consumer: legs 0,1,2 only
for d in "${ROOTS[@]}"; do
  h="$(harness_of "$d")"
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
step "leg 0/6: received suites carry no @target"
disarmed=0
for d in "${CHECK_ROOTS[@]:-}"; do
  [ -z "$d" ] && continue
  [ -f "$d/RECEIVED.md" ] || continue
  hits="$(grep -rnE '^[[:space:]]*@target([[:space:]]|$)' "$d" || true)"
  if [ -n "$hits" ]; then
    echo "FAILED: @target found in the RECEIVED suite $d -- we may not withdraw another repo's guarantee:" >&2
    printf '%s\n' "$hits" | sed 's/^/    /' >&2
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
  pact_fingerprint() {
    find contracts/pacts -type f -name '*.json' -print0 2>/dev/null \
      | sort -z | xargs -0 sha256sum 2>/dev/null | sha256sum
  }
  before="$(pact_fingerprint)"

  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-server --test contract_pact >/dev/null )
  check $? "provider drift: the HTTP pact no longer matches the live graph"
  ( cd server && env -u ATLAS_BLESS_PACT "$CARGO" test -p atlas-cli --test contract_pact_cli >/dev/null )
  check $? "provider drift: the CLI pact no longer matches the real bibex binary"

  after="$(pact_fingerprint)"
  if [ "$before" != "$after" ]; then
    echo "FAILED: contracts/pacts was REWRITTEN while this gate was running." >&2
    echo "  The gate verifies evidence; it must never be the thing that produces it." >&2
    echo "  Re-record deliberately, review the diff, then run the gate against it." >&2
    fail=1
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
  bash scripts/contract-semver-gate.sh "$RESOLVED"; check $? "contract semver gate"
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
