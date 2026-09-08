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

# --runner PATH: the contract-runner binary, used as the TAG ORACLE (review
# C-NEW-1). Optional so this script stays runnable standalone, but see
# `tag_delta` -- when a suite's diff touches a tag line and no oracle is
# available, this fails rather than guessing.
RUNNER=""
args=()
while [ $# -gt 0 ]; do
  case "$1" in
    --runner) RUNNER="${2:?--runner needs a path}"; shift 2;;
    *) args+=("$1"); shift;;
  esac
done
set -- "${args[@]:-}"

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

# M-NEW-1: `--base HEAD` made every suite's diff empty, so leg 5 classified
# nothing and printed the same shape as "everything is fine" -- an argument
# that weakens the gate, in a script whose header says there is no skip flag.
# A base that IS HEAD, or that HEAD is not strictly ahead of, cannot show a
# diff and must not be accepted as one.
if [ "$(git rev-parse "$base")" = "$(git rev-parse HEAD)" ]; then
  echo "contract-semver-gate: the base resolves to HEAD itself, so there is no diff to classify." >&2
  echo "  A base that can show nothing is not a base; pass one behind HEAD." >&2
  exit 1
fi
if ! git merge-base --is-ancestor "$base" HEAD 2>/dev/null; then
  echo "contract-semver-gate: '$requested' is not an ancestor of HEAD -- the diff would be meaningless." >&2
  exit 1
fi

# THE SUITES WE VERSION, enumerated from BOTH SIDES of the diff.
#
# Fix round 1, review H-2: this used to be a literal array walked with
# `[ -d "$suite" ] || continue`, which ran BEFORE the diff was consulted --
# so deleting an entire suite directory made it vanish from the loop and
# the gate exited 0 with no bump demanded. Deleting a suite is the most
# MAJOR thing that can happen to it, and it was the one change that
# classified as nothing at all.
#
# So: a suite is any directory that carries a VERSION file EITHER now OR at
# the base. One present then and absent now is a deletion, and deletion is
# MAJOR by definition.
#
# `contracts/atlas-edge` is deliberately still excluded -- it is
# map-generator's suite, versioned by map-generator's own contracts/VERSION,
# and a bump there is a cross-repo negotiation, not a number we may declare
# on their behalf. What guards THAT directory is `contract-gate.sh`'s leg 0,
# which forbids `@target` in any received suite outright (review C-2); it is
# not left unguarded merely because it is unversioned.
list_versioned_suites() {
  # now
  find contracts -mindepth 1 -maxdepth 3 -name VERSION -not -path 'contracts/runner/*' -printf '%h\n' 2>/dev/null
  # at the base
  git ls-tree -r --name-only "$base" -- contracts 2>/dev/null \
    | grep -E '/VERSION$' | sed 's|/VERSION$||'
}
mapfile -t suites < <(list_versioned_suites | sort -u)

rank() { case "$1" in none) echo 0;; patch) echo 1;; minor) echo 2;; major) echo 3;; esac; }

# A step or scenario line -- the lines that ARE the expectations. Prose,
# comments and Vocabulary rows are deliberately excluded: changing them
# cannot change what conforms.
expectation_line='^[+-][[:space:]]*(Scenario:|Given |When |Then |And |But |@)'

failed=0

# ---------------------------------------------------------------------
# THE FIXTURE GRADER MUST PROVE IT DISCRIMINATES (fix round 2, M-NEW-5)
# ---------------------------------------------------------------------
# The three-grade fixture decision is delegated to whatever `python` is on
# PATH, and the reviewer shadowed it with two lines:
#
#     #!/bin/sh
#     echo same
#
# -> every re-bless classified PATCH, exit 0. The gate had spent effort
# proving `$CARGO` says "cargo" and then handed the actual classification to
# an interpreter it never looked at.
#
# Validating the ANSWER is not enough on its own: `same` is a perfectly valid
# grade, so a shim that always prints it passes any answer-shape check. And
# validating the INTERPRETER is the H-NEW-1 mistake -- a tool's greeting is
# not its identity.
#
# So the grader is CALIBRATED instead: it is asked three questions whose
# answers are known, and must get all three right before its opinion is
# taken on anything real. A shim that always says `same` fails the second
# question; one that always says `changed` fails the first; one that does not
# run at all fails every one. This guards the property that matters -- "this
# grader actually discriminates" -- rather than any spelling of the failure.
grade_pair() { # <old-json> <new-json> -> same|wider|changed|<garbage>
  python -c "
import json,sys
def widened(a,b):
    if isinstance(a,dict) and isinstance(b,dict):
        return all(k in b and widened(a[k],b[k]) for k in a)
    if isinstance(a,list) and isinstance(b,list):
        return len(a)==len(b) and all(widened(x,y) for x,y in zip(a,b))
    return a==b
try:
    a=json.loads(sys.argv[1]); b=json.loads(sys.argv[2])
    print('same' if a==b else ('wider' if widened(a,b) else 'changed'))
except Exception:
    print('changed')
" "$1" "$2" 2>/dev/null
}

if command -v python >/dev/null 2>&1; then
  cal_same="$(grade_pair '{"a":1}' '{"a":1}')"
  cal_wider="$(grade_pair '{"a":1}' '{"a":1,"b":2}')"
  cal_changed="$(grade_pair '{"a":1}' '{"a":2}')"
  if [ "$cal_same" != "same" ] || [ "$cal_wider" != "wider" ] || [ "$cal_changed" != "changed" ]; then
    echo "contract-semver-gate: the fixture grader failed calibration." >&2
    echo "  Asked three questions with known answers, it said:" >&2
    echo "    identical pair  -> '$cal_same'   (expected 'same')" >&2
    echo "    widened pair    -> '$cal_wider'  (expected 'wider')" >&2
    echo "    changed pair    -> '$cal_changed' (expected 'changed')" >&2
    echo "  A grader that does not discriminate cannot classify a contract change." >&2
    echo "  (A \`python\` earlier on PATH that is not a JSON-capable Python will do this.)" >&2
    exit 1
  fi
else
  echo "contract-semver-gate: no \`python\` on PATH; fixtures cannot be graded." >&2
  echo "  Refusing to classify contract changes with no grader at all." >&2
  exit 1
fi

# ---------------------------------------------------------------------
# RECEIVED SUITES ARE NOT OURS TO DELETE (fix round 2, review M-NEW-4)
# ---------------------------------------------------------------------
# H-2 closed whole-suite deletion for VERSIONED suites, and contract-gate.sh
# added a second mouth for registry rows pointing at missing directories. A
# received suite is neither: it is deliberately unversioned (we do not
# declare map-generator's number), so removing its registry line and its
# directory in one commit passes through both. The reviewer walked exactly
# that path -- and then found the worse half: the only leg that objects is
# leg 3, whose message says "the provider drifted" and whose documented
# remedy is a re-record. Following the gate's own instructions LAUNDERS the
# deletion into a smaller pact and a fully green gate.
#
# Tracked the same way VERSION files already are. Not a bump class: there is
# no version in which we may delete another repo's expectations of us.
list_received_suites() {
  find contracts -mindepth 1 -maxdepth 3 -name RECEIVED.md -not -path 'contracts/runner/*' -printf '%h\n' 2>/dev/null
  git ls-tree -r --name-only "$base" -- contracts 2>/dev/null \
    | grep -E '/RECEIVED\.md$' | sed 's|/RECEIVED\.md$||'
}
while IFS= read -r rsuite; do
  [ -z "$rsuite" ] && continue
  [ -d "$rsuite" ] && continue
  echo "contract-semver-gate: $rsuite" >&2
  echo "  a RECEIVED suite was DELETED. Those are another repo's expectations of us;" >&2
  echo "  they are not ours to remove, in any bump class." >&2
  echo "  If they are genuinely obsolete, coordinate the removal on BOTH sides first." >&2
  failed=1
done < <(list_received_suites | sort -u)

# ---------------------------------------------------------------------
# THE TAG ORACLE (fix round 2, review C-NEW-1)
# ---------------------------------------------------------------------
# `@target` used to be detected by grepping the DIFF for
# `^\+[[:space:]]*@target`. The vendored parser makes EVERY
# whitespace-separated word on a tag line a tag, so `  @wip @target`
# withdrew a guarantee and this script classified the change MINOR -- a
# consumer reading "backward compatible" would get a scenario that no longer
# enforces anything.
#
# Position on a line is not the property. The property is WHICH SCENARIOS
# CARRY THE TAG, and only the parser can answer that. So: materialise the
# suite as it was at the base, ask `contract-runner tags` about both sides,
# and compare the two SETS.
#
# Returns non-zero if it cannot answer, and the caller treats that as a hard
# failure rather than as "no tags changed" -- "I could not classify this" and
# "this is fine" are different facts.
tags_at() { # <dir> <tag> -> scenario names carrying <tag>, sorted
  "$RUNNER" tags "$1" 2>/dev/null \
    | awk -F'\t' -v t="$2" '{n=split($3,a,","); for(i=1;i<=n;i++) if(a[i]==t) print $2}' \
    | sort -u
}

tag_delta() { # <suite> <tag> -> "gained\t<scenario>" / "lost\t<scenario>" lines
  local suite="$1" tag="$2" tmp rc=0
  [ -n "$RUNNER" ] && [ -x "$RUNNER" ] || return 1
  tmp="$(mktemp -d 2>/dev/null)" || return 1
  if git archive "$base" -- "$suite" 2>/dev/null | tar -x -C "$tmp" 2>/dev/null; then
    :
  else
    # The suite may not exist at the base at all (a new suite); that is not
    # a failure to classify, it is an empty base side.
    mkdir -p "$tmp/$suite" 2>/dev/null || rc=1
  fi
  if [ "$rc" -eq 0 ]; then
    tags_at "$suite" "$tag" > "$tmp/.now" 2>/dev/null || rc=1
    if [ -d "$tmp/$suite" ]; then
      tags_at "$tmp/$suite" "$tag" > "$tmp/.base" 2>/dev/null || rc=1
    else
      : > "$tmp/.base"
    fi
  fi
  if [ "$rc" -eq 0 ]; then
    comm -13 "$tmp/.base" "$tmp/.now" | sed 's/^/gained\t/'
    comm -23 "$tmp/.base" "$tmp/.now" | sed 's/^/lost\t/'
  fi
  rm -rf "$tmp"
  return $rc
}

for suite in "${suites[@]}"; do
  [ -z "$suite" ] && continue

  # --- what the diff requires -------------------------------------------
  required=none

  # DELETION FIRST, before any `-d` test could skip the suite entirely.
  if [ ! -d "$suite" ]; then
    if git rev-parse --verify --quiet "$base:$suite" >/dev/null 2>&1 \
       || git ls-tree -r --name-only "$base" -- "$suite" 2>/dev/null | grep -q .; then
      echo "contract-semver-gate: $suite" >&2
      echo "  the entire suite was DELETED. That is the most MAJOR change a suite can" >&2
      echo "  undergo, and there is no VERSION left to declare it in." >&2
      echo "  Deleting a published contract suite is a cross-consumer break: say so in a" >&2
      echo "  CHANGELOG that survives the deletion, or keep the suite." >&2
      failed=1
    fi
    continue
  fi

  status="$(git diff --name-status "$base"..HEAD -- "$suite" || true)"
  # Say so explicitly (fix round 2, review M-NEW-1). Silently `continue`ing
  # made "this suite did not change" indistinguishable from "this suite was
  # classified and is fine" -- and `--base HEAD` used to reach this line for
  # EVERY suite, so leg 5 asserted nothing while printing nothing wrong.
  if [ -z "$status" ]; then
    echo "contract-semver-gate: $suite unchanged in this range -- nothing to classify"
    continue
  fi

  # Deleted or renamed published files are unconditionally breaking. A
  # rename reports only its DESTINATION under --diff-filter=R, so matching
  # the destination against a pattern would miss a file renamed OUT of a
  # protected namespace -- treat any rename here as breaking, as
  # map-generator's own gate does and for the reason its comment gives.
  if printf '%s\n' "$status" | grep -qE '^(D|R)'; then
    required=major
  fi

  # A fixture is classified by comparing PARSED JSON, in three grades:
  #
  #   same           -> PATCH  a pure reformat pins exactly what it pinned.
  #                            This is the distinction that makes this a
  #                            classifier rather than a counter.
  #   strict superset-> MINOR  every key the old fixture pinned is still
  #                            pinned to the SAME value, and new keys were
  #                            added. Nothing a consumer relied on moved;
  #                            the fixture simply promises more. Adding
  #                            `artifact_format_version` to the vocabulary
  #                            projection is exactly this.
  #   anything else  -> MAJOR  a value a consumer relied on was re-blessed,
  #                            or a pinned key disappeared.
  #
  # The middle grade is fix round 1's own addition. Without it, "a
  # projection gaining a field" -- which this suite's CHANGELOG documents as
  # MINOR -- classified as MAJOR, which is the same self-contradiction the
  # review flagged as M-3 for `@target`. A classifier that disagrees with
  # its own published scheme teaches people to route around it.
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    case "$f" in */fixtures/*.json) ;; *) continue ;; esac
    old="$(git show "$base:$f" 2>/dev/null || true)"
    new="$(cat "$f" 2>/dev/null || true)"
    # L-NEW-1: `[ -n "$old" ] && [ -n "$new" ]` skipped the grading entirely
    # for a fixture truncated to ZERO BYTES, so emptying a pinned promise
    # classified as PATCH. Emptiness is the same blind spot H-1 was about, so
    # it is now its own MAJOR rather than an unreachable branch.
    if [ -n "$old" ] && [ -z "$new" ]; then
      echo "  MAJOR: $f was emptied -- a fixture that pins nothing cannot fail"
      required=major
    elif [ -n "$old" ] && [ -n "$new" ]; then
      # M-NEW-5: the whole three-grade decision was delegated to an
      # UNVERIFIED interpreter found on PATH, so shadowing `python` with a
      # two-line shim printing `same` turned every re-bless into a PATCH.
      # The gate spent effort proving `$CARGO` says "cargo" and then handed
      # a classification to a `python` it never looked at.
      #
      # Two changes, and neither is "verify the interpreter" -- that is the
      # H-NEW-1 mistake again (a tool's greeting is not its identity).
      # Instead: VALIDATE THE ANSWER, and treat "no answer" as a failure to
      # classify rather than a silent downgrade.
      grade="$(grade_pair "$old" "$new")"
      case "$grade" in
        same) ;;
        wider)
          echo "  MINOR: $f pins additional fields; every previously pinned value is unchanged"
          [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor;;
        changed)
          echo "  MAJOR: $f was re-blessed to a different value"
          required=major;;
        *)
          # Anything that is not one of the three grades means the grader did
          # not run or did not answer -- a shim, a banner, a broken
          # interpreter, no interpreter. "I could not classify this" is not
          # "this is MAJOR" and it is certainly not "this is fine": it is a
          # broken gate, and it says so.
          echo "contract-semver-gate: could not classify $f -- the fixture grader answered '$grade'." >&2
          echo "  A JSON-capable \`python\` on PATH is required to grade fixtures." >&2
          echo "  Refusing to classify a contract change with a grader that did not answer." >&2
          failed=1;;
      esac
    fi
  done < <(printf '%s\n' "$status" | awk '$1=="M"{print $2}')

  # Expectation lines removed or changed -> MAJOR; only added -> MINOR.
  diffbody="$(git diff -U0 "$base"..HEAD -- "$suite" || true)"
  removed_all="$(printf '%s\n' "$diffbody" | grep -E "^-" | grep -E "$expectation_line" || true)"
  added="$(printf '%s\n' "$diffbody" | grep -E "^\+" | grep -E "$expectation_line" || true)"

  # `@target` is the one line whose two directions mean OPPOSITE things, so
  # it is classified on its own before the general rules see it.
  #
  # REMOVING it is a guarantee GAINED: the scenario was already running and
  # already printed red every time; taking the tag off makes it start
  # failing the gate, which is strictly more promise, not less. Fix round 1,
  # review M-3: the general "an expectation line was removed" branch caught
  # `-  @target` and demanded MAJOR, which contradicted this batch's own
  # shipped CHANGELOG (it promises MINOR for exactly this, once bibex gains
  # its version field). It failed in the safe direction, but a classifier
  # that tells you MAJOR when the documented answer is MINOR is a classifier
  # people learn to distrust.
  # Tag lines are excluded from the general removed/added rules, because
  # those classify by LINE SHAPE and would double-count a tag edit in the
  # wrong direction. Tags are classified below, by the parser.
  removed="$(printf '%s\n' "$removed_all" | grep -vE '^-[[:space:]]*@' || true)"
  added_nontag="$(printf '%s\n' "$added" | grep -vE '^\+[[:space:]]*@' || true)"

  # GAINED @target = a guarantee WITHDRAWN -> MAJOR (additive-looking, and
  #   exactly how a suite would smuggle a loss of coverage past a gate that
  #   only counts additions; on a RECEIVED suite it is forbidden outright).
  # LOST @target   = a guarantee GAINED -> MINOR (the scenario was already
  #   running and already printed red; removing the tag makes it start
  #   FAILING the gate, which is strictly more promise -- review M-3).
  if printf '%s\n' "$removed_all" "$added" | grep -qE '^[-+][[:space:]]*@'; then
    if delta="$(tag_delta "$suite" target)"; then
      if printf '%s\n' "$delta" | grep -q '^gained'; then
        echo "  MAJOR: @target was added -- a scenario's guarantee is being withdrawn:"
        printf '%s\n' "$delta" | grep '^gained' | sed 's/^gained\t/    /'
        required=major
      fi
      if printf '%s\n' "$delta" | grep -q '^lost'; then
        echo "  MINOR: @target was removed -- a scenario that only reported is now enforced:"
        printf '%s\n' "$delta" | grep '^lost' | sed 's/^lost\t/    /'
        [ "$(rank "$required")" -lt "$(rank minor)" ] && required=minor
      fi
    else
      echo "contract-semver-gate: $suite" >&2
      echo "  a tag line changed, and no working --runner was available to say WHICH" >&2
      echo "  scenarios carry @target on each side. Grepping the diff is what let" >&2
      echo "  '  @wip @target' withdraw a guarantee under a MINOR bump, so this" >&2
      echo "  refuses to classify rather than guess." >&2
      echo "  Pass --runner <contract-runner> (contract-gate.sh does this for you)." >&2
      failed=1
    fi
  fi

  if [ -n "$removed" ]; then
    echo "  MAJOR: an existing expectation line was removed or changed:"
    printf '%s\n' "$removed" | sed 's/^/    /' | head -10
    required=major
  fi

  if [ "$required" = none ] && [ -n "$added_nontag" ]; then
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
    #
    # SAID OUT LOUD (fix round 2, review M-NEW-3). This branch is why leg 5
    # passes trivially on the shipped path today: the auto-resolved base
    # (@{upstream}) predates the suite's existence, so "requires major,
    # declared major" is arithmetic, not gating. It printed the same shape as
    # a real classification, and the fix-round-1 report quoted a MINOR/MINOR
    # line from a DIFFERENT invocation (--base 14323d5) next to it without
    # saying so. A gate that cannot classify must not look like one that did.
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
