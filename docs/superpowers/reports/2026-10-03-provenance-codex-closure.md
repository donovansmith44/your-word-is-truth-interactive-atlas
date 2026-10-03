# A-PROVENANCE: F-86 constructor closure re-review

Reviewed 2026-10-03: exact `7247d81`, cumulative item `6de059a..7247d81`,
repair delta `a619631..7247d81`. **F-86 CLOSED; scoped repair APPROVED.** The
unchecked public iterator constructor is removed, and every nonempty title-index
construction now establishes uniqueness at `ProvenanceTitles::from_rows`.
This supplements the prior item/re-review, rather than repeating its unchanged
artifact and wire checks. Full latest-head author gates must accompany final
item completion; ops still submitted the preceding a619631 gate set during this
review. No unsupplied workspace/browser result is asserted here.

The representation remains private. `from_rows` returns a named
`DuplicateProvenance` instead of silently replacing a title; even a repeated id
with the identical title is refused. Registry titling and SQLite reconstruction
both call that checked door. `Default` yields an empty index, and `Clone` preserves
an established index; neither admits conflicting rows. No Deserialize,
FromIterator, Extend, mutable field accessor or second raw-row constructor is
exported. Source-registry structural admission stays private behind
`AdmittedSources`; the compile/source-generator migration from a619631 remains.

Independent verification uses the exact production sources and read-only
compatible dependencies in the existing server target, avoiding a fresh Cargo
build while Windows host capacity is critical:

- [Separate public consumer](evidence/2026-10-03-provenance-closure/public-constructor-probe.rs), [output](evidence/2026-10-03-provenance-closure/public-constructor-probe.txt): **2,112** duplicate-row inputs refused, retaining the exact id, spanning all 32 generated ids, identical/conflicting titles and every insertion position. The whole sorted distinct map, empty/default and clone values, **1,024** nested locator lookups and the error's complete display string pass.
- [Forbidden iterator](evidence/2026-10-03-provenance-closure/forbidden-iterator.txt) and [field](evidence/2026-10-03-provenance-closure/forbidden-field.txt) programs fail independently: the old public last-wins `collect` no longer typechecks, and the private representation cannot be built externally.
- [Registry probe](evidence/2026-10-03-provenance-closure/registry-probe.rs), [output](evidence/2026-10-03-provenance-closure/registry-probe.txt): prior **96** synthetic malformed registries and **86** complete-current-registry mutations remain refused; **1,024** positive locator/title cases pass. All **32** previous synthetic public-map bypasses and **33** current-map overwrites now return the exact typed error.
- [Author's two constructor tests](evidence/2026-10-03-provenance-closure/author-index-tests.txt) independently compiled against the exact public-source library: **2 passed**.
- [Metadata](evidence/2026-10-03-provenance-closure/metadata.txt): all **7** compiled artifact files, **124** contract files and **3** exports remain byte-identical to checked a986d2c. The prior 33 title rows/1,222 provenance ids/root checks still concern the identical bytes; they were not rerun.

The included ETL probe emits three unused-function warnings because it compiles
only the registry module rather than its full application callers. They are
probe-harness warnings, not a claim about the author's normal workspace build.
No fresh full workspace, graph-types, C#, timing, browser or mutation gate was
run in this scoped re-review. The preceding submitted author results remain
1512 workspace tests / 150 graph-types / 737+55 C# / contract gate / 11 timing /
85 provenance Playwright with 2 skips, explicitly at a619631.

14b: the map's duplicate-key policy lives at one checked core constructor; all
registry/artifact title-row builders reuse it. Registry validation also checks
its larger source/category join invariant, rather than replacing it with an
independent title-only partial policy. The named error and private representation
carry the uniqueness guarantee without a string failure or public last-wins
factory. No new D.R.Y./Haskell-bar finding.

24a/24b: the failed category was unchecked provenance-index construction, on
the tool/core/artifact boundary. The remaining public iterator bypass is removed
and both row-origin sites are migrated. Independent external compilation proves
that prior constructor is unwritable; generated positive/duplicate-row programs
prove the sole replacement refuses ambiguity. This closes F-86's category, not
unrelated provenance/content/license or historical legacy client attribution
categories.

Added application-comment scan is clean; test additions carry only
Arrange/Act/Assert. No source/vendor/artifact/contract bytes or endpoint behavior
is changed by the review. Code changes were not made on the side. Small source
probes/logs are retained; stopped task-owned binaries are removed after capture.
No Codex lock, Cargo process, server, owner port or raw/cache edit remains.

Reproduce the public-source library with rustc edition 2021, `-C debuginfo=0`,
`--crate-type=rlib --crate-name review_title_core`; link `public-core.rs` to the
same existing graph-types `a5037e002a84fe0a`, serde `519dfb59dc69ed84`, utoipa
`76564d89f9dec40b` artifacts. Compile the separate consumer with that rlib. The
registry probe additionally links TOML `c095797f623f574e` and anyhow
`33aa09c41829759b`. Paths and source hashes are retained in evidence; resolve
compatible dependencies freshly if the disposable server target is removed.

Handoff: Claude can complete the item after publishing latest-head gates, then
land the reviewed range under the normal lock/squash protocol. No new finding or
further F-86 repair is requested at 7247d81. WIREID is reviewed separately against
its own exact head and base.

Final gate/landing addendum: ops fb69475 submitted exact 7247d81 author
results: workspace 1514/0 (11 ignored), graph-types 150/0, C# 737/0 and
55/0, contract gate PASS, timing 11/11, provenance Playwright 86 passed /
1 skipped. Workspace/graph-types/contract logs were read from the author’s
retained PROV logs; timing/browser totals are the signed queue submission,
not independent reruns. This resolves the publication condition above. Claude
landed the reviewed item as 9a699c3 on 4acdc7f under the normal squash/lock
protocol (ops 67f4ad8); its tree is 7247d81 plus the already landed AGENTS
rule. Verdict: APPROVED at exact 7247d81, F-86 closed.
