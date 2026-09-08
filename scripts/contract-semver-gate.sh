#!/usr/bin/env bash
# CDC-1, addendum ruling 2: "a suite of contract tests constitute a semver."
#
# A SUITE IS A VERSIONED UNIT, and the bump is DERIVED FROM THE DIFF, not
# asserted by the author. This script computes the class the diff requires
# and fails when the declared bump is smaller.
#
#   PATCH  expectations clarified; the set of conforming providers is
#          unchanged. Prose, comments, a regenerated Vocabulary: table, a
#          fixture reformatted to an identical parsed value.
#   MINOR  expectations ADDED. New scenarios, new feature files, new
#          fixtures, a projection gaining a field. A consumer written
#          against the older version still has everything it relied on.
#   MAJOR  an existing expectation CHANGED or REMOVED. A deleted scenario
#          or feature file, a fixture re-blessed to a DIFFERENT VALUE, a
#          renamed file, or @target added to a scenario that was green.
#
# Over-declaring is always allowed. Under-declaring is the lie this refuses
# to co-sign.
#
# =====================================================================
# WHAT FIX ROUND 3 CHANGED, AND WHY
# =====================================================================
# Round 3's review found three defects here, and they are one defect seen
# three ways: THIS SCRIPT WAS COMPARING TWO DIFFERENT WORLDS.
#
#  * H-R2-3 -- `old` came from `git show "$base:$f"` and `new` came from
#    `cat "$f"`, THE WORKING TREE. A fixture re-blessed in a commit, with
#    the worktree copy reverted, graded `same` -> PATCH -> exit 0, while
#    `git show HEAD:...` still held the re-bless. No shim, no env var: a
#    `git stash` produces it. BOTH SIDES NOW COME FROM GIT.
#  * H-R2-2 -- the fixture grader was whatever `python` was first on PATH,
#    guarded by a calibration quiz of THREE HARDCODED CONSTANT PAIRS. Nine
#    lines of `case` answered the quiz and graded a total re-bless `same`.
#    The grader is now `contract-runner grade`, in the binary this gate
#    builds from vendored source; there is no interpreter left to shadow.
#  * H-R2-4 -- the oracle was asked correctly and its answer was RE-LEXED
#    by `awk -F'\t' ... $3`, so a TAB INSIDE A SCENARIO NAME shifted the tag
#    column and hid `@target` from the classifier. The runner now
#    percent-escapes every field, so a tab cannot occur inside one.
#
# And three ratchets are new, because round 3's ruling is that protection
# must not be conditional on an artifact the author controls:
#
#  * THE MARKER RATCHET (C-R2-1). A suite that carried `RECEIVED.md` at the
#    base must still carry it at HEAD. The old check tested `[ -d "$rsuite" ]`
#    -- the DIRECTORY -- so deleting just the marker passed it, and deleting
#    just the marker switched off every received-suite protection in the
#    gate.
#  * THE HARNESS RATCHET (C-R2-2). A suite's registered harness may never
#    get WEAKER between the base and HEAD. `contract-runner` ->
#    `contract-runner-consumer` was a one-word commit that removed the
#    graph-primary suite -- the owner's ruled #1 juncture -- from four legs.
#  * THE EXPECTATION RATCHET (C-R2-3). Every scenario present at the base
#    must still be present at HEAD, and on a RECEIVED suite its removal is
#    refused in any bump class. Renaming the `.feature` files emptied a
#    received suite while leaving its directory and marker intact; the only
#    objection was leg 3, whose printed remedy was a re-record that shrank
#    the pact by 80% and turned the gate green.
set -uo pipefail

# --runner PATH: the contract-runner binary. REQUIRED as of fix round 3 --
# it is the tag oracle, the fixture grader and the scenario inventory, and
# every one of those is load-bearing. Round 2 made it optional "so this
# script stays runnable standalone", and the result was a fail-closed branch
# that made several self-test cases pass for the wrong reason (review
# M-R2-3, verified in the round-2 review's Part D(ii)). A gate that cannot
# see is not a gate that found nothing.
RUNNER=""
args=()
while [ $# -gt 0 ]; do
  case "$1" in
    --runner) RUNNER="${2:?--runner needs a path}"; shift 2;;
    *) args+=("$1"); shift;;
  esac
done
set -- "${args[@]:-}"

if [ -z "$RUNNER" ] || [ ! -x "$RUNNER" ]; then
  echo "contract-semver-gate: --runner <contract-runner> is required." >&2
  echo "  It is the tag oracle (which scenarios carry @target), the fixture grader" >&2
  echo "  (same/wider/changed) and the scenario inventory. Without it this script" >&2
  echo "  cannot classify anything, and 'cannot classify' must never read as 'fine'." >&2
  echo "  scripts/contract-gate.sh builds it and passes it for you." >&2
  exit 1
fi

requested="${1:-origin/main}"
base="$requested"
if ! git rev-parse --verify --quiet "$base" >/dev/null; then
  if git rev-parse --verify --quiet main >/dev/null; then
    base="$(git merge-base HEAD main)"
  else
    echo "contract-semver-gate: cannot resolve a base to diff against ('$requested' and 'main' are both missing here)." >&2
    echo "  Pass one explicitly, e.g.:  scripts/contract-semver-gate.sh <ref>" >&2
    exit 1
  fi
fi

if [ "$(git rev-parse "$base")" = "$(git rev-parse HEAD)" ]; then
  echo "contract-semver-gate: the base resolves to HEAD itself, so there is no diff to classify." >&2
  echo "  A base that can show nothing is not a base; pass one behind HEAD." >&2
  exit 1
fi
if ! git merge-base --is-ancestor "$base" HEAD 2>/dev/null; then
  echo "contract-semver-gate: '$requested' is not an ancestor of HEAD -- the diff would be meaningless." >&2
  exit 1
fi

failed=0
TMPDIRS=()
cleanup() { for d in "${TMPDIRS[@]:-}"; do [ -n "$d" ] && rm -rf "$d"; done; }
trap cleanup EXIT

# =====================================================================
# TWO TREES, BOTH FROM GIT
# =====================================================================
# Everything below reads these. Not the working tree -- ever. This is the
# single change that closes H-R2-3 and it is also what makes the three
# ratchets expressible at all: a ratchet needs two committed states.
BASETREE="$(mktemp -d)"; TMPDIRS+=("$BASETREE")
HEADTREE="$(mktemp -d)"; TMPDIRS+=("$HEADTREE")
git archive "$base" -- contracts 2>/dev/null | tar -x -C "$BASETREE" 2>/dev/null || true
if ! git archive HEAD -- contracts 2>/dev/null | tar -x -C "$HEADTREE" 2>/dev/null; then
  echo "contract-semver-gate: could not materialise contracts/ from HEAD." >&2
  exit 1
fi

norm_rows() { tr -d '\r' | awk -F'\t' -v OFS='\t' 'NF>=2 { gsub(/\\/, "/", $1); print }'; }

# `<tree> <suite>` -> `path\tscenario` per scenario, sorted. Empty (rc 0) if
# the suite does not exist in that tree; rc 1 if it exists and cannot be
# read, which is a refusal to classify rather than an empty answer.
inventory_of() { # <tree> <suite>
  local tree="$1" suite="$2"
  [ -d "$tree/$suite" ] || return 0
  ( cd "$tree" && "$RUNNER" tags "$suite" 2>/dev/null ) \
    | norm_rows | awk -F'\t' -v OFS='\t' '{print $1, $2}' | sort -u
}

# The same rows, but only those carrying <tag>. Consumed STRUCTURALLY: the
# runner escapes every field, so `-F'\t'` is total and `$3` is the tag list
# whatever the scenario is called (review H-R2-4).
tagged_of() { # <tree> <suite> <tag>
  local tree="$1" suite="$2" tag="$3"
  [ -d "$tree/$suite" ] || return 0
  ( cd "$tree" && "$RUNNER" tags "$suite" 2>/dev/null ) \
    | norm_rows \
    | awk -F'\t' -v t="$tag" '{n=split($3,a,","); for(i=1;i<=n;i++) if(a[i]==t) {print $2; break}}' \
    | sort -u
}

dirs_with() { # <tree> <marker filename>
  local tree="$1" marker="$2"
  [ -d "$tree/contracts" ] || return 0
  ( cd "$tree" && find contracts -mindepth 1 -maxdepth 3 -name "$marker" \
      -not -path 'contracts/runner/*' -printf '%h\n' 2>/dev/null ) | sort -u
}

# The registry, read from a TREE (never from disk), as `<suite> <harness>`.
registry_of() { # <tree>
  local f="$1/contracts/SUITES"
  [ -f "$f" ] || return 0
  awk '$1 !~ /^#/ && NF>=2 {print $1, $2}' "$f"
}
harness_in() { # <tree> <suite>
  registry_of "$1" | awk -v s="$2" '$1==s {print $2; found=1; exit} END{if(!found) print ""}'
}
# Harness STRENGTH, so "may not get weaker" is a number comparison rather
# than a list of forbidden pairs (a list is the literal-array shape that
# caused C-2, C-NEW-2 and C-R2-2 in three consecutive rounds).
tier() { case "$1" in
  contract-runner) echo 3;; contract-runner-consumer) echo 2;; aqc-dual) echo 1;; *) echo 0;; esac; }

rank() { case "$1" in none) echo 0;; patch) echo 1;; minor) echo 2;; major) echo 3;; esac; }

expectation_line='^[+-][[:space:]]*(Scenario:|Given |When |Then |And |But |@)'

# ---------------------------------------------------------------------
# THE FIXTURE GRADER -- in the runner, reading two git blobs
# ---------------------------------------------------------------------
grade_blobs() { # <blobspec-old> <blobspec-new> -> same|wider|changed|unreadable
  local o n out
  o="$(mktemp)"; n="$(mktemp)"
  git show "$1" > "$o" 2>/dev/null || true
  git show "$2" > "$n" 2>/dev/null || true
  out="$("$RUNNER" grade "$o" "$n" 2>/dev/null)"
  rm -f "$o" "$n"
  printf '%s\n' "${out:-unreadable}" | tr -d '\r'
}

# =====================================================================
# RATCHET 1 -- A RECEIVED SUITE'S MARKER MAY NOT DISAPPEAR (C-R2-1)
# =====================================================================
# The old check was `[ -d "$rsuite" ] && continue`: it knew which
# directories carried RECEIVED.md at the base, and then declined to look
# further because the directory was still there. The marker's disappearance
# was the one event it did not classify -- and deleting the marker switched
# off leg 0's filter, the registry derivation, and this check, all three, in
# one `rm` that reads as tidy-up in a diff.
while IFS= read -r rsuite; do
  [ -z "$rsuite" ] && continue
  if [ ! -d "$HEADTREE/$rsuite" ]; then
    echo "contract-semver-gate: $rsuite" >&2
    echo "  a RECEIVED suite was DELETED. Those are another repo's expectations of us;" >&2
    echo "  they are not ours to remove, in any bump class." >&2
    failed=1
  elif [ ! -f "$HEADTREE/$rsuite/RECEIVED.md" ]; then
    echo "contract-semver-gate: $rsuite" >&2
    echo "  its RECEIVED.md marker was REMOVED while the directory stayed. That marker" >&2
    echo "  is what says these are another repo's expectations OF US, and removing it" >&2
    echo "  is how a received suite stops being executed without appearing to be" >&2
    echo "  deleted. It is not ours to remove, in any bump class." >&2
    failed=1
  fi
done < <(dirs_with "$BASETREE" RECEIVED.md)

# =====================================================================
# RATCHET 2 -- A SUITE'S HARNESS MAY NEVER GET WEAKER (C-R2-2)
# =====================================================================
# `contracts/atlas-graph-contract  contract-runner` ->
# `contract-runner-consumer` is one word, reads as a plausible correction
# ("we author these expectations; the provider run belongs in the API
# batch"), and removed the graph-primary suite from legs 0, 1, 2 and 4 with
# a real break planted in a published export. `-> aqc-dual` plus a
# three-line NOTES.md did the same and took totality and vocabulary drift
# with it.
#
# Registry rows are now read from BOTH TREES and compared. A row that gets
# weaker, or disappears while its suite survives, is refused. Strengthening
# is always allowed.
while read -r bsuite bharness; do
  [ -z "${bsuite:-}" ] && continue
  [ -d "$HEADTREE/$bsuite" ] || continue          # deletion is handled elsewhere, loudly
  hharness="$(harness_in "$HEADTREE" "$bsuite")"
  if [ "$(tier "$hharness")" -lt "$(tier "$bharness")" ]; then
    echo "contract-semver-gate: $bsuite" >&2
    echo "  its harness was WEAKENED: '$bharness' -> '${hharness:-<unregistered>}'." >&2
    echo "  A registry row is a promise about how much of this suite the gate runs." >&2
    echo "  contract-runner (executed) > contract-runner-consumer (parsed only) >" >&2
    echo "  aqc-dual (run elsewhere) > unregistered. That order may only increase." >&2
    echo "  If the suite genuinely changed hands, that is a cross-repo negotiation," >&2
    echo "  not a word." >&2
    failed=1
  fi
done < <(registry_of "$BASETREE")

# =====================================================================
# RATCHET 3 -- EXPECTATIONS MAY NOT VANISH FROM A RECEIVED SUITE (C-R2-3)
# =====================================================================
# `find contracts -name '*.feature'` was the suite detector, so renaming the
# six files to `*.feature.bak` removed the suite from legs 0, 1, 2 and 4
# while its directory and its RECEIVED.md sat untouched -- passing the
# deletion check, and leaving leg 3 as the only objection. Leg 3 calls it
# "provider drift" and the gate's own printed remedy is a re-record, which
# shrank the pact 175,656 -> 34,257 bytes and turned everything green.
#
# The corpus is a tracked quantity now: the scenario SET at the base must be
# a subset of the scenario SET at HEAD, and neither the directory nor the
# file names have any say in it.
while IFS= read -r rsuite; do
  [ -z "$rsuite" ] && continue
  [ -d "$HEADTREE/$rsuite" ] || continue
  bi="$(mktemp)"; hi="$(mktemp)"
  inventory_of "$BASETREE" "$rsuite" > "$bi"
  inventory_of "$HEADTREE" "$rsuite" > "$hi"
  gone="$(comm -23 "$bi" "$hi")"
  rm -f "$bi" "$hi"
  if [ -n "$gone" ]; then
    echo "contract-semver-gate: $rsuite" >&2
    echo "  expectations that exist at the base are GONE from a RECEIVED suite:" >&2
    printf '%s\n' "$gone" | sed 's/^/    /' | head -20 >&2
    echo "  Renaming, moving or emptying their feature files is not a bump class." >&2
    echo "  Another repo's expectations of us are removed by coordinating on both" >&2
    echo "  sides, never by making our own gate stop looking." >&2
    failed=1
  fi
done < <(dirs_with "$BASETREE" RECEIVED.md)

# =====================================================================
# RATCHET 4 -- AN aqc-dual SUITE'S SCENARIO COUNT MAY NOT FALL
# =====================================================================
# An `aqc-dual` suite is executed by its own two harnesses (Rust `cucumber`
# in-process and C# Reqnroll), so the coverage reconciliation in
# contract-gate.sh cannot see it: the vendored parser cannot read its
# corpus, which is the very fact that justifies the classification.
#
# That is a real boundary of what this gate proves, and it is stated in the
# report rather than papered over. What IS provable from committed content
# is the size of the corpus, so that is ratcheted: scenario headers counted
# in the base tree and the head tree, from git, by a count that a rename
# reduces just as a deletion does.
while read -r hsuite hharness; do
  [ "${hharness:-}" = "aqc-dual" ] || continue
  count_scen() { # <tree>
    [ -d "$1/$hsuite" ] || { echo 0; return; }
    ( cd "$1" && find "$hsuite" -name '*.feature' -exec cat {} + 2>/dev/null ) \
      | grep -cE '^[[:space:]]*Scenario( Outline)?:' || true
  }
  bn="$(count_scen "$BASETREE")"; hn="$(count_scen "$HEADTREE")"
  if [ "${bn:-0}" -gt 0 ] && [ "${hn:-0}" -lt "${bn:-0}" ]; then
    echo "contract-semver-gate: $hsuite" >&2
    echo "  its committed scenario count FELL, $bn -> $hn." >&2
    echo "  This suite is run by its own harnesses, so the coverage reconciliation" >&2
    echo "  cannot execute it here; the count is the part that is provable from" >&2
    echo "  committed content, and it may not shrink without a MAJOR declaration." >&2
    failed=1
  fi
done < <(registry_of "$HEADTREE")

# =====================================================================
# PER-SUITE CLASSIFICATION
# =====================================================================
list_versioned_suites() {
  dirs_with "$HEADTREE" VERSION
  dirs_with "$BASETREE" VERSION
}
mapfile -t suites < <(list_versioned_suites | sort -u)

classified=0
total="${#suites[@]}"

for suite in "${suites[@]}"; do
  [ -z "$suite" ] && continue

  required=none

  if [ ! -d "$HEADTREE/$suite" ]; then
    echo "contract-semver-gate: $suite" >&2
    echo "  the entire suite was DELETED. That is the most MAJOR change a suite can" >&2
    echo "  undergo, and there is no VERSION left to declare it in." >&2
    failed=1
    continue
  fi

  status="$(git diff --name-status "$base"..HEAD -- "$suite" || true)"
  if [ -z "$status" ]; then
    echo "contract-semver-gate: $suite unchanged in this range -- nothing to classify"
    continue
  fi
  classified=$((classified+1))

  if printf '%s\n' "$status" | grep -qE '^(D|R)'; then
    required=major
  fi

  # --- fixtures: BOTH SIDES FROM GIT (review H-R2-3) --------------------
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in */fixtures/*.json) ;; *) continue ;; esac
    old_len="$(git cat-file -s "$base:$f" 2>/dev/null || echo 0)"
    new_len="$(git cat-file -s "HEAD:$f" 2>/dev/null || echo 0)"
    if [ "${old_len:-0}" -gt 0 ] && [ "${new_len:-0}" -eq 0 ]; then
      echo "  MAJOR: $f was emptied -- a fixture that pins nothing cannot fail"
      required=major
    elif [ "${old_len:-0}" -gt 0 ] && [ "${new_len:-0}" -gt 0 ]; then
      grade="$(grade_blobs "$base:$f" "HEAD:$f")"
      case "$grade" in
        same) ;;
        wider)
          echo "  MINOR: $f pins additional fields; every previously pinned value is unchanged"
          [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor;;
        changed)
          echo "  MAJOR: $f was re-blessed to a different value"
          required=major;;
        *)
          echo "contract-semver-gate: could not classify $f -- the grader answered '$grade'." >&2
          echo "  Refusing to classify a contract change with a grader that did not answer." >&2
          failed=1;;
      esac
    fi
  done < <(printf '%s\n' "$status" | awk '$1=="M"{print $2}')

  diffbody="$(git diff -U0 "$base"..HEAD -- "$suite" || true)"
  removed_all="$(printf '%s\n' "$diffbody" | grep -E "^-" | grep -E "$expectation_line" || true)"
  added="$(printf '%s\n' "$diffbody" | grep -E "^\+" | grep -E "$expectation_line" || true)"
  removed="$(printf '%s\n' "$removed_all" | grep -vE '^-[[:space:]]*@' || true)"
  added_nontag="$(printf '%s\n' "$added" | grep -vE '^\+[[:space:]]*@' || true)"

  # --- SCENARIOS THAT DISAPPEARED, by the parser, not by line shape -----
  # The general "an expectation line was removed" rule below classifies by
  # LINE SHAPE, which a rename defeats: `git diff` reports a rename as a
  # rename, not as a wall of deleted `Scenario:` lines. Comparing the two
  # committed inventories catches it whatever the files are called.
  bi="$(mktemp)"; hi="$(mktemp)"
  inventory_of "$BASETREE" "$suite" > "$bi"
  inventory_of "$HEADTREE" "$suite" > "$hi"
  vanished="$(comm -23 "$bi" "$hi")"
  gained_sc="$(comm -13 "$bi" "$hi")"
  rm -f "$bi" "$hi"
  if [ -n "$vanished" ]; then
    echo "  MAJOR: expectations that existed at the base no longer exist:"
    printf '%s\n' "$vanished" | sed 's/^/    /' | head -10
    required=major
  fi
  if [ -n "$gained_sc" ] && [ "$(rank "$required")" -lt "$(rank minor)" ]; then
    required=minor
  fi

  # --- @target, by the oracle, on both committed trees ------------------
  gained_t="$(comm -13 <(tagged_of "$BASETREE" "$suite" target) <(tagged_of "$HEADTREE" "$suite" target))"
  lost_t="$(comm -23 <(tagged_of "$BASETREE" "$suite" target) <(tagged_of "$HEADTREE" "$suite" target))"
  if [ -n "$gained_t" ]; then
    echo "  MAJOR: @target was added -- a scenario's guarantee is being withdrawn:"
    printf '%s\n' "$gained_t" | sed 's/^/    /'
    required=major
  fi
  if [ -n "$lost_t" ]; then
    echo "  MINOR: @target was removed -- a scenario that only reported is now enforced:"
    printf '%s\n' "$lost_t" | sed 's/^/    /'
    [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor
  fi

  if [ -n "$removed" ]; then
    echo "  MAJOR: an existing expectation line was removed or changed:"
    printf '%s\n' "$removed" | sed 's/^/    /' | head -10
    required=major
  fi

  if [ "$required" = none ] && [ -n "$added_nontag" ]; then
    required=minor
  fi

  if printf '%s\n' "$status" | grep -qE '^A.*(\.feature|/fixtures/)'; then
    [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor
  fi

  if [ "$required" = none ]; then
    required=patch
  fi

  # --- what the author declared -- read from git, like everything else ---
  vfile="$suite/VERSION"
  old_v="$(git show "$base:$vfile" 2>/dev/null | tr -d '[:space:]' || true)"
  new_v="$(git show "HEAD:$vfile" 2>/dev/null | tr -d '[:space:]' || true)"

  if [ -z "$new_v" ]; then
    echo "contract-semver-gate: $suite changed but has no VERSION file at HEAD." >&2
    failed=1
    continue
  fi

  declared=none
  if [ -n "$old_v" ] && [ "$old_v" != "$new_v" ]; then
    IFS=. read -r oma omi opa <<<"$old_v"
    IFS=. read -r nma nmi npa <<<"$new_v"
    if   [ "${nma:-0}" -gt "${oma:-0}" ]; then declared=major
    elif [ "${nmi:-0}" -gt "${omi:-0}" ]; then declared=minor
    elif [ "${npa:-0}" -gt "${opa:-0}" ]; then declared=patch
    fi
  elif [ -z "$old_v" ]; then
    declared=major
    echo "  NOTE: $suite does not exist at the base -- it is NEW in this range,"
    echo "        so this row is arithmetic, not gating. Leg 5 starts classifying"
    echo "        this suite for real once the base advances past its introduction."
  fi

  if [ "$(rank "$declared")" -lt "$(rank "$required")" ]; then
    echo "contract-semver-gate: $suite" >&2
    echo "  the diff requires a $(echo "$required" | tr '[:lower:]' '[:upper:]') bump; the declared bump is $(echo "$declared" | tr '[:lower:]' '[:upper:]') ($old_v -> $new_v)." >&2
    echo "  Raise $vfile and add a CHANGELOG.md entry saying what promise changed." >&2
    failed=1
    continue
  fi

  if [ "$declared" != none ]; then
    if ! git diff --name-only "$base"..HEAD -- "$suite/CHANGELOG.md" | grep -q .; then
      echo "contract-semver-gate: $suite bumped $old_v -> $new_v with no CHANGELOG.md entry." >&2
      failed=1
      continue
    fi
  fi

  echo "contract-semver-gate: $suite ok (diff requires $required, declared $declared, version $new_v)"
done

# M-R2-2: "nothing to classify" printed three times in a row read as a pass.
# The summary makes the difference between "leg 5 agreed with three
# classifications" and "leg 5 classified nothing" impossible to miss.
echo "contract-semver-gate: $classified of $total versioned suite(s) classified against $base"
if [ "$total" -gt 0 ] && [ "$classified" -eq 0 ]; then
  echo "  NOTE: no versioned suite changed in this range. Leg 5 asserted nothing here;"
  echo "        the coverage leg, the ratchets and legs 0-4 are what carry this run."
fi

exit $failed
