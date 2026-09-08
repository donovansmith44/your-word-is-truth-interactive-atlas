#!/usr/bin/env bash
# THE CONTRACT GATE (batch CDC-1).
#
# Owner order 2: "we will get to a point where we cannot push or do
# anything if our contracts don't agree. that is target shape."
#
# THERE IS NO ADVISORY MODE AND NO SKIP FLAG, by construction: this script
# reads no environment variable that can weaken it and takes no --allow
# argument. Every leg is required. Project history is the reason -- every
# "temporarily allow" in this repo became permanent.
#
# The one switch that exists nearby, ATLAS_BLESS_PACT=1, is a RE-RECORD
# switch on the recorders, not an escape hatch on this gate: it changes
# what the evidence SAYS (and leaves that change in a committed file for a
# reviewer to read in the diff), never whether the gate RUNS.
#
# WHY IT IS FAST ENOUGH TO RUN PRE-PUSH. Legs 1, 2 and 4 are the whole
# corpus and take well under a second: they need no server and no database,
# because the expectations execute against a recorded pact. Leg 3 is the
# expensive one -- it recompiles the ETL and rebuilds the real graph -- and
# it is the leg that makes leg 4 mean anything, so it is not optional. Run
# `scripts/contract-gate.sh --fast` to run legs 1, 2, 4 and 5 alone while
# iterating on expectations; that is NOT a weakened gate, because --fast
# refuses to be the last word (see below) and the full gate is what a push
# runs.
#
# Usage:
#   scripts/contract-gate.sh              full gate (what pre-push runs)
#   scripts/contract-gate.sh --fast       skip the recorders while iterating
#   scripts/contract-gate.sh --base REF   semver base (default origin/main)
set -uo pipefail
cd "$(dirname "$0")/.."
ROOT="$PWD"

FAST=0
BASE="origin/main"
while [ $# -gt 0 ]; do
  case "$1" in
    --fast) FAST=1; shift;;
    --base) BASE="${2:?--base needs a ref}"; shift 2;;
    *) echo "unknown argument: $1" >&2; exit 2;;
  esac
done

CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
CABAL="${CABAL:-$HOME/.local/bin/cabal}"

fail=0
step() { printf '\n=== %s ===\n' "$1"; }
check() { if [ "$1" -ne 0 ]; then echo "FAILED: $2" >&2; fail=1; fi; }

# Our suites, plus the one we received. atlas-edge is map-generator's
# expectations OF US; running it here is the entire point of CDC-1 (see
# contracts/atlas-edge/RECEIVED.md).
OURS="contracts/atlas-graph-contract"
THEIRS="contracts/atlas-edge"
CONSUMER="contracts/map-api-consumer"

step "building the contract runner"
( cd contracts/runner && "$CABAL" build all >/dev/null 2>&1 )
check $? "contracts/runner does not build"
RUNNER="$(cd contracts/runner && "$CABAL" list-bin contract-runner 2>/dev/null)"
if [ -z "${RUNNER:-}" ] || [ ! -x "$RUNNER" ]; then
  echo "FAILED: no contract-runner binary" >&2
  exit 1
fi

# ---------------------------------------------------------------------
# LEG 1 -- TOTALITY. Every step in every suite matches exactly one
# definition. Catches an orphan step (an expectation nobody executes,
# which reads as coverage and is not) and an ambiguous one (two
# definitions racing, which makes a suite's meaning depend on list order).
# ---------------------------------------------------------------------
step "leg 1/5: totality (check)"
for d in "$OURS" "$THEIRS" "$CONSUMER"; do
  [ -d "$d" ] || continue
  "$RUNNER" check "$d"; check $? "totality: $d"
done

# ---------------------------------------------------------------------
# LEG 2 -- VOCABULARY DRIFT. Every Vocabulary: table still describes the
# runner's actual parameter space. Catches a feature file that documents a
# vocabulary the code no longer has.
# ---------------------------------------------------------------------
step "leg 2/5: vocabulary drift (vocab)"
for d in "$OURS" "$THEIRS" "$CONSUMER"; do
  [ -d "$d" ] || continue
  "$RUNNER" vocab "$d"; check $? "vocabulary drift: $d"
done

# ---------------------------------------------------------------------
# LEG 3 -- PROVIDER DRIFT. Regenerate the pact from the REAL committed
# graph, through the REAL Router and the REAL bibex binary, and fail on a
# one-byte difference. This is what makes leg 4 evidence rather than
# self-agreement: without it, a pact checked against a pact proves nothing.
# ---------------------------------------------------------------------
if [ "$FAST" -eq 0 ]; then
  step "leg 3/5: provider drift (the recorders)"
  ( cd server && "$CARGO" test -p atlas-server --test contract_pact -- --nocapture >/dev/null )
  check $? "provider drift: the HTTP pact no longer matches the live graph"
  ( cd server && "$CARGO" test -p atlas-cli --test contract_pact_cli >/dev/null )
  check $? "provider drift: the CLI pact no longer matches the real bibex binary"
else
  step "leg 3/5: SKIPPED (--fast)"
fi

# ---------------------------------------------------------------------
# LEG 4 -- EXPECTATION DRIFT. Execute every published expectation, ours
# and theirs, against the pact.
# ---------------------------------------------------------------------
step "leg 4/5: expectations (run --replay)"
for d in "$OURS" "$THEIRS"; do
  [ -d "$d" ] || continue
  "$RUNNER" run --replay contracts/pacts --exports data/exports "$d"
  check $? "expectations: $d"
done
# contracts/map-api-consumer is OUR expectations of map-generator's API. We
# cannot run it -- we are not its provider and its server is not ours to
# start. It is published for THEM to run against their own server, and it
# is held to legs 1, 2 and 5 here so that what we ship them is at least
# total, self-describing and versioned.

# ---------------------------------------------------------------------
# LEG 5 -- VERSION NEGOTIATION. The declared bump must be at least what
# the diff requires.
# ---------------------------------------------------------------------
step "leg 5/5: semver"
bash scripts/contract-semver-gate.sh "$BASE"; check $? "contract semver gate"

printf '\n'
if [ "$fail" -ne 0 ]; then
  echo "CONTRACT GATE: FAILED" >&2
  exit 1
fi
if [ "$FAST" -eq 1 ]; then
  # --fast must not be able to masquerade as a passing gate. It reports
  # success for what it ran and a non-zero status for what it did not, so
  # it is usable while iterating and unusable as a final answer -- which is
  # the difference between a convenience and an escape hatch.
  echo "CONTRACT GATE (--fast): legs 1, 2, 4, 5 passed; leg 3 NOT RUN."
  echo "  Run without --fast before pushing." >&2
  exit 3
fi
echo "CONTRACT GATE: PASSED"
