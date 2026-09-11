# The oracle's golden vectors

`contract-runner` is in the gate's TRUSTED BASE: it is the tag oracle, the
fixture grader, the scenario inventory and the executor. Everything else the
gate believes is derived, and all of it is derived through this binary.

That leaves one residual, and it is the largest one this round did not
remove: **the gate builds its own oracle from source in the repository, so a
commit that weakens `Gherkin/Parse.hs`, `Tags.hs` or `Grade.hs` weakens
every derivation at once.** No amount of deriving-from-git helps, because
git is exactly where that change would live.

It cannot be removed — an executor has to come from somewhere — but it can
be made LOUD. These vectors are a ratchet on the oracle itself:
`EXPECTED.tags` is the byte-exact answer the parser gives for a corpus
chosen to contain every form that has ever defeated a guard in this project,
and `scripts/gate-selftest.sh` section C fails if the answer moves.

Weakening the oracle now requires editing this file in the same commit, and
a diff that says "the parser no longer sees `@target` in `@wip @target`, and
here is the updated golden output" is a diff a human cannot skim past. That
is the honest end state: the trusted base is small, written down, and
changing it is visible.

## What each row is here to pin

| corpus | the form | why it is here |
|---|---|---|
| `a.feature` feature-level `@featurelevel` | a tag above `Feature:` | `Tags.tagsOfDir` unions `ftTags` into every scenario deliberately, so the oracle is WIDER than `Run.hs`'s `scTags` — the safe direction for a guard |
| `  @target` | the plain form | the baseline |
| `  @wip @target` | two tags, one line | **C-NEW-1**: the line-anchored regex saw nothing here, and a received suite was disarmed while the gate printed PASSED |
| `  @a @b @c   @target` | target last, irregular spacing | position on the line is not the property |
| `  @wip@target` | ONE word, not two tags | the consistency control. `Gherkin/Parse.hs:27` makes the word `wip@target`; `Run.hs:115`'s `isTarget` does not match it, and neither may the oracle. A guard WIDER than the executor here would fail honest suites |
| `b.feature` `Scenario: alpha<TAB>beta gamma` | a TAB inside a scenario NAME | **H-R2-4**: the shell re-lexed the oracle's answer with `awk -F'\t' … $3`, so this shifted the tag column to `$4` and hid `@target` from the semver classifier. The name is escaped to `alpha%09beta gamma` and the tag field is field 3 whatever the scenario is called |
| `a name with a percent %25 in it` | a literal `%` in the source | escaping must be INVERTIBLE, so `%25` becomes `%2525`. Without this row, an attacker could write a name that decodes to a different name |

This directory lives under `contracts/runner/`, which every suite detector in
the gate excludes (`-not -path 'contracts/runner/*'`), so it is a corpus for
the oracle and never a contract suite.
