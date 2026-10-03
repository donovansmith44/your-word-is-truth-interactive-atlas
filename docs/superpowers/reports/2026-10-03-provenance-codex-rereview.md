# A-PROVENANCE: Codex scoped re-review

Reviewed 2026-10-03: exact **`6de059a..a619631`** on `lane/claude/PROV`, with repair delta `a986d2c..a619631`. Current main instructions and ops submission read after fetch/pull.

**CHANGES REQUESTED: F-86 remains open only at public title-index construction.** The original TOML compiler/source-generator bypass is repaired. Current committed titles and the SQLite reconstruction path remain valid. No new F-number and no claim of current artifact corruption.

## What is repaired

`admit_sources` now parses and invokes the existing structural validator through one ETL entry point. The parser and validator are private. `AdmittedSources` has a private field and exposes immutable access; its title derivation cannot be reached from a raw `SourcesDocument`. Compiler, source generator and registry-reading tests migrate to that entry point. Core's raw `SourcesDocument::provenance_titles` method is removed. No second duplicate-ID validator is introduced.

The [exact-source generated probe](evidence/2026-10-03-provenance-rereview/registry-probe.rs), [complete output](evidence/2026-10-03-provenance-rereview/registry-probe.txt), reruns the prior cases through the repaired door. All **32 missing-source**, **32 duplicate-source** and **32 duplicate-provenance** synthetic registries are refused. All **86 full-curated-TOML mutations** are refused: conflicting duplicates for all 33 provenance IDs, a dangling mapping for each, and duplicate definitions for all 20 source IDs. **1,024 positive locator/title cases** still pass.

## The remaining F-86 construction door

`server/atlas-core/src/sources.rs:60` still exports the infallible standard trait implementation:

```rust
impl FromIterator<(String, String)> for ProvenanceTitles
```

Its `BTreeMap::collect` accepts duplicate identities and retains the last title. That public API neither requires `AdmittedSources` nor refuses duplicate keys. The queue's statement that only an admitted registry builds titles therefore does not hold for the exported type.

This is independently demonstrated across a crate boundary. [Small library entry](evidence/2026-10-03-provenance-rereview/public-core.rs) compiles the **exact production core sources module** as a library; [separate consumer](evidence/2026-10-03-provenance-rereview/public-constructor-probe.rs) imports only its exported type. All **32 generated conflicting duplicate constructions** compile and finish successfully. Their complete outputs are asserted and retained in [the full result](evidence/2026-10-03-provenance-rereview/public-constructor-probe.txt). The complete representative output is:

```text
[("kjv", "OpenBible.info Geocoding")]
```

The external consumer supplied KJV's original title followed by that conflicting title. It has no ETL dependency and constructs no admitted-registry value. The first probe additionally alters each of the **33 complete current title-map entries** through this public iterator API, asserting the entire resulting map against the expected overwritten map. Those calls are still accepted.

**Important distinctions:** production compile/gen callers now reject malformed TOML. The existing SQLite loader's actual rows are unique because `provenance_title.id` is a PRIMARY KEY; it is not shown to be broken. The remaining defect is that arbitrary exported API callers can still write a conflicting title construction outside the admitted abstraction. SQL uniqueness protects one present caller, not the public construction category. This is the same unchecked-constructor site identified in the original F-86 report, not a new finding or a claim that a bad artifact was built.

**Proposed closure:** remove the unchecked public construction path and make every title-index constructor require or establish the shared identity invariant. Preserve the checked artifact-loading path without permitting a general last-wins API. Cover every exported constructor with the generated duplicate-input refusal property. The owner/controller chooses the mechanism; no application changes are made here.

## Artifact and review passes

[Unchanged-artifact result](evidence/2026-10-03-provenance-rereview/unchanged-artifact.txt): all **7 compiled artifact files**, **124 contract files**, and **3 exports** are byte-identical by Git object identity to independently checked `a986d2c`. Root remains `fa95e31a4c84a41ac12339853cefc795`. No new decompression or rebuild was needed. The prior positive artifact evidence therefore still applies to identical bytes: 33 correct title rows, all 1,222 actual provenance IDs titled, all ten before/after blob checks, all eight wire sites and ninth CLI site migrated, and the title-column binding property. Those broad checks were not rerun and are not counted as new gates.

**14b:** the compiler/source-generator divergence is removed by one admission entry point; raw parser/validator access is private and the duplicate partial titling validator is deleted. The admitted wrapper is immutable, so the guarded registry invariant is carried by the type. The public iterator implementation remains a route around that type boundary and is F-86's remaining site.

**24a/24b:** the repair names the unvalidated registry category and migrates all TOML callers. Its new generated law enumerates the real registry and proves those entry points refuse ambiguity. It does not exercise the exported title-map constructor; the independent separate-crate probe confirms an offender remains writable there. Thus compiler-path closure is established, while whole public-construction closure is not.

**9 / 18 / 25 / 26 / 27:** whitespace passes; added comments are test Arrange/Act/Assert only. The fix reuses existing TOML/serde tooling and existing registry validation. No serving/client derivation, wire shape, domain literal, endpoint, fixture or artifact change is introduced. The new public admission entry point precedes its parsing/validation functions. Historical repository-wide ordering is outside this scoped certification.

## Reproduction and gate limits

The probes compile exact current source modules by path, using existing compatible libraries read-only. Core module SHA-256: `b59dcf5fca56ffcdbfce2a40fb5f652201dcd93bac226185b48a3e5dfaa5f2c5`; ETL module SHA-256: `0e4659528711f7e5398d62908343835aa4c9f894e6861cdbe045dce77f115e59`. The separate probe's compiled library and executable SHA-256 are retained in [this metadata](evidence/2026-10-03-provenance-rereview/public-probe-sha256.txt); binaries are disposable and removed after evidence retention.

Source `~/.bible-atlas-env` first. Direct `rustc` uses edition 2021 and `-C debuginfo=0`; executables also use `-C strip=debuginfo`. Dependency directory is `/home/donovan/src/bible-atlas/server/target/debug/deps`. Compatible inputs used were graph-types `a5037e002a84fe0a`, serde `519dfb59dc69ed84`, utoipa `76564d89f9dec40b`, TOML `c095797f623f574e`, anyhow `33aa09c41829759b`. Compile `public-core.rs` with `--crate-type=rlib --crate-name review_title_core`, then compile the separate consumer with that library as `--extern review_title_core`. The linked production-module paths must point at the reviewed head; resolve dependencies freshly if the existing targets were removed.

The queue records author gates at `a619631`: workspace **1512/0**, 11 ignored; graph-types **150/0**; C# **737/0**, **55/0**; contract gate passed; timing **11/11**; provenance Playwright **85 passed / 2 skipped**. These full gates were not independently rerun. Independent new claims concern the retained admission and separate-public-consumer probes plus Git artifact identity. No fresh Cargo target, heavy/browser/AOT run, mutation, owner port or raw/cache write occurred. Windows C: remained approximately **3.7 GB free**.

**Handoff:** keep F-86 open for the public `FromIterator` path; complete constructor closure and request exact-head re-review. Root publishes the scoped verdict on ops. No ops edits, Codex lock, Cargo build, server or owned disposable probe output remains.
