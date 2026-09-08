#!/usr/bin/env bash
# CDC-1, addendum ruling 2: "a suite of contract tests constitute a semver."
#
# A SUITE IS A VERSIONED UNIT, and the bump is DERIVED FROM THE DIFF, not
# asserted by the author. This script computes the class the diff requires
# and fails when the declared bump is smaller.
#
# map-generator's scripts/contract-semver-gate.sh is the precedent, and its
# own comment states its scope honestly: "Deliberately dumb about WHAT
# changed -- the version bump is the owner's declaration, and this gate's
# whole job is to make sure the declaration happens." It checks that SOME
# bump happened. The addendum asks for that bar or better, so this one
# classifies:
#
#   PATCH  expectations clarified; the set of conforming providers is
#          unchanged. Prose, comments, a regenerated Vocabulary: table, a
#          fixture reformatted to an identical parsed value.
#   MINOR  expectations ADDED. New scenarios, new feature files, new
#          fixtures, a projection gaining a field. A consumer written
#          against the older version still has everything it relied on.
#   MAJOR  an existing expectation CHANGED or REMOVED. A deleted scenario
#          or feature file, a fixture re-blessed to a DIFFERENT VALUE, a
#          renamed file, or @target added to a scenario that was green --
#          a guarantee withdrawn is a break, and it is the one a suite is
#          most likely to smuggle.
#
# Semver here is stated from the point of view of a CONSUMER OF THE
# PROMISES -- another system declaring "I am written against
# atlas-graph-contract 1.4". It is not stated from the provider's point of
# view: the provider newly failing is the normal, intended result of a
# MINOR bump.
#
# Over-declaring is always allowed (a bigger bump than required passes).
# Under-declaring is the lie this refuses to co-sign.
set -uo pipefail

requested="${1:-origin/main}"
base="$requested"
if ! git rev-parse --verify --quiet "$base" >/dev/null; then
  # A first-push branch has no upstream to diff against. Fall back to the
  # merge-base with the default branch; if THAT is missing too, say so and
  # fail -- a semver gate that cannot see a diff must not report success,
  # because "no diff visible" and "no diff exists" are different facts.
  #
  # `$requested` rather than `$1`: under `set -u` an unset `$1` is itself a
  # fatal error, so quoting the argument directly turned the helpful
  # diagnostic below into "line 46: $1: unbound variable" on exactly the
  # invocation that needed the diagnostic most -- a bare
  # `scripts/contract-semver-gate.sh` in a worktree with no `origin/main`,
  # which is this repo's own situation.
  if git rev-parse --verify --quiet main >/dev/null; then
    base="$(git merge-base HEAD main)"
  else
    echo "contract-semver-gate: cannot resolve a base to diff against ('$requested' and 'main' are both missing here)." >&2
    echo "  Pass one explicitly, e.g.:  scripts/contract-semver-gate.sh <ref>" >&2
    echo "  or via the gate:            scripts/contract-gate.sh --base <ref>" >&2
    exit 1
  fi
fi

# Every suite that carries its own VERSION. contracts/atlas-edge is
# deliberately NOT here: it is map-generator's suite, versioned by
# map-generator's own contracts/VERSION, and a change to it is a cross-repo
# negotiation rather than a bump we may declare on their behalf.
suites=(contracts/atlas-graph-contract contracts/map-api-consumer)

rank() { case "$1" in none) echo 0;; patch) echo 1;; minor) echo 2;; major) echo 3;; esac; }
name() { case "$1" in 0) echo none;; 1) echo patch;; 2) echo minor;; 3) echo major;; esac; }

# A step or scenario line -- the lines that ARE the expectations. Prose,
# comments and Vocabulary rows are deliberately excluded: changing them
# cannot change what conforms.
expectation_line='^[+-][[:space:]]*(Scenario:|Given |When |Then |And |But |@)'

failed=0

for suite in "${suites[@]}"; do
  [ -d "$suite" ] || continue

  # --- what the diff requires -------------------------------------------
  required=none

  status="$(git diff --name-status "$base"..HEAD -- "$suite" || true)"
  [ -z "$status" ] && continue

  # Deleted or renamed published files are unconditionally breaking. A
  # rename reports only its DESTINATION under --diff-filter=R, so matching
  # the destination against a pattern would miss a file renamed OUT of a
  # protected namespace -- treat any rename here as breaking, as
  # map-generator's own gate does and for the reason its comment gives.
  if printf '%s\n' "$status" | grep -qE '^(D|R)'; then
    required=major
  fi

  # A fixture whose VALUE moved is a re-blessed promise. Compare parsed
  # JSON, not bytes, so a pure reformat stays PATCH -- that distinction is
  # the whole reason this gate classifies instead of counting.
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in */fixtures/*.json) ;; *) continue ;; esac
    old="$(git show "$base:$f" 2>/dev/null || true)"
    new="$(cat "$f" 2>/dev/null || true)"
    if [ -n "$old" ] && [ -n "$new" ]; then
      if command -v python >/dev/null 2>&1; then
        same=$(python -c "
import json,sys
try:
    a=json.loads(sys.argv[1]); b=json.loads(sys.argv[2])
    print('1' if a==b else '0')
except Exception:
    print('0')
" "$old" "$new")
      else
        same=0
      fi
      if [ "$same" != "1" ]; then
        echo "  MAJOR: $f was re-blessed to a different value"
        required=major
      fi
    fi
  done < <(printf '%s\n' "$status" | awk '$1=="M"{print $2}')

  # Expectation lines removed or changed -> MAJOR; only added -> MINOR.
  diffbody="$(git diff -U0 "$base"..HEAD -- "$suite" || true)"
  removed="$(printf '%s\n' "$diffbody" | grep -E "^-" | grep -E "$expectation_line" || true)"
  added="$(printf '%s\n' "$diffbody" | grep -E "^\+" | grep -E "$expectation_line" || true)"

  if [ -n "$removed" ]; then
    echo "  MAJOR: an existing expectation line was removed or changed:"
    printf '%s\n' "$removed" | sed 's/^/    /' | head -10
    required=major
  fi

  # @target added to a scenario is a guarantee WITHDRAWN, even though it is
  # an added line. Checked explicitly, because the additive-looking shape is
  # exactly how a suite would smuggle a loss of coverage past a gate that
  # only counted additions.
  if printf '%s\n' "$added" | grep -qE '^\+[[:space:]]*@target'; then
    echo "  MAJOR: @target was added -- a scenario's guarantee is being withdrawn"
    required=major
  fi

  if [ "$required" = none ] && [ -n "$added" ]; then
    required=minor
  fi

  # New or deleted feature/fixture FILES.
  if printf '%s\n' "$status" | grep -qE '^A.*(\.feature|/fixtures/)'; then
    [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor
  fi

  # Anything else that changed under the suite (prose, comments, the
  # generated Vocabulary table) is a clarification.
  if [ "$required" = none ]; then
    required=patch
  fi

  # --- what the author declared -----------------------------------------
  vfile="$suite/VERSION"
  old_v="$(git show "$base:$vfile" 2>/dev/null | tr -d '[:space:]' || true)"
  new_v="$(tr -d '[:space:]' < "$vfile" 2>/dev/null || true)"

  if [ -z "$new_v" ]; then
    echo "contract-semver-gate: $suite changed but has no VERSION file." >&2
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
    # The suite is new in this range: its first VERSION declares everything
    # in it at once, so nothing can be under-declared.
    declared=major
  fi

  if [ "$(rank "$declared")" -lt "$(rank "$required")" ]; then
    echo "contract-semver-gate: $suite" >&2
    echo "  the diff requires a $(echo "$required" | tr '[:lower:]' '[:upper:]') bump; the declared bump is $(echo "$declared" | tr '[:lower:]' '[:upper:]') ($old_v -> $new_v)." >&2
    echo "  Raise $vfile and add a CHANGELOG.md entry saying what promise changed." >&2
    failed=1
    continue
  fi

  # A bump without a CHANGELOG entry is a number nobody can read.
  if [ "$declared" != none ]; then
    if ! git diff --name-only "$base"..HEAD -- "$suite/CHANGELOG.md" | grep -q .; then
      echo "contract-semver-gate: $suite bumped $old_v -> $new_v with no CHANGELOG.md entry." >&2
      failed=1
      continue
    fi
  fi

  echo "contract-semver-gate: $suite ok (diff requires $required, declared $declared, version $new_v)"
done

exit $failed
