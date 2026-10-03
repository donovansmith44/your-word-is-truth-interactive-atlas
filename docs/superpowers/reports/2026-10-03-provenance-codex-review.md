# A-PROVENANCE: Codex review

Reviewed 2026-10-03: exact **`6de059a..a986d2c`** on `lane/claude/PROV`, against current main AGENTS/PRINCIPLES/GO and ops submission `24eeb48`.

**CHANGES REQUESTED on registry admission closure.** The current artifact's titles, wire migration and CLI migration are correct in the evidence below. One new **Important** finding is proposed for root to allocate: the newly added compilation path accepts ambiguous registries that the existing source-registry validator refuses. The current committed registry is valid; this is not a claim that its served titles are presently wrong.

## Proposed Important finding: title construction bypasses source-registry admission

**Category:** the same source/provenance registry has parallel construction paths with different admission invariants. The new `atlas-etl/src/compile.rs:310` calls structural `parse_sources(...).provenance_titles()` directly. The parser deliberately does not validate references or uniqueness. `SourcesDocument::provenance_titles` rechecks source existence, but does not reuse the registry's existing duplicate-source/duplicate-provenance refusal. Its source lookup selects the first duplicate source; `ProvenanceTitles::FromIterator` silently keeps the last duplicate provenance mapping.

The existing `atlas-etl::sources::validate_structure` rejects both ambiguities. `gen_sources` calls it; the new title compilation path does not. Thus an invalid registry refused when generating sources can still construct a successful title index when compiling atlas data. The raw map constructor remains public and unchecked as well. This is a 14b duplication/divergence and 24a/24b admission gap, not a request for a second hand-written duplicate check.

**Concrete reproduction:** append to a complete in-memory copy of the actual `data/curated/sources.toml`:

```toml
[[provenance]]
id = "kjv"
source = "openbible-geocoding"
confidence = "CanonicalText"
```

The exact production parser and new title constructor succeed. The resulting index returns `OpenBible.info Geocoding` for `kjv`, replacing `The King James Version`. The exact existing validator refuses the duplicate. The retained probe generates this case for **all 33 current provenance IDs**, each pointing the appended duplicate at a different declared source; every construction succeeds and switches the title. It also generates **32 duplicate source-ID** and **32 duplicate provenance-ID** registries: existing admission rejects all 64, while new title construction accepts them.

**Positive controls:** **1,024 generated source/locator resolutions** return the source's complete expected title, and **32 missing-source cases** are refused. The actual unmodified registry has **20 sources / 33 provenance mappings**, and every mapping is correct. This separates successful intended behavior from the missing refusal property.

**Sites:** `server/atlas-etl/src/compile.rs:310`; `server/atlas-core/src/sources.rs:60` and `:67`; existing admission in `server/atlas-etl/src/sources.rs:26`; source generator in `server/atlas-etl/src/bins/gen_sources.rs`; sidecar loading also constructs `ProvenanceTitles` through the unchecked iterator.

**Proposed closure:** one owned, validated registry abstraction shared by source generation, title compilation and artifact reconstruction. Only admitted mappings construct the title index; the declared source/provenance uniqueness policy is reused rather than copied. A generated law should exercise missing references and conflicting duplicate identities through every public construction path. The owner/controller decides the mechanism. No fix is made by this review.

The existing curated-file validation gate would catch these deliberately corrupted files if the whole workspace suite were run against them. This finding concerns the standalone production compilation/construction path, whose declared refusal obligations differ. A full artifact build from corrupted sources and live HTTP serving were not run; the exact parser/title-constructor prefix and the divergent existing validator were independently exercised.

## Independent positive verification

[Registry property source](evidence/2026-10-03-provenance/registry-probe.rs), [complete results](evidence/2026-10-03-provenance/registry-probe.txt). Both exact production source modules are compiled by path; existing compatible serde, TOML and schema libraries supply dependencies. Core source SHA-256 is `7e6a490d62737924c371e1943a41204905816e76c59a67dc22a964dc49901c1a`; ETL source module SHA-256 is `3280f3c73caaa19b21673dc1bcd77ed1558a89cbcc024023acf5858acbbeceed`.

[Artifact verifier](evidence/2026-10-03-provenance/artifact-probe.py), [helpers](evidence/2026-10-03-provenance/artifact-helpers.py), [complete results](evidence/2026-10-03-provenance/artifact-probe.txt): every **6,079,426 prior row across 102 tables** is unchanged from reviewed `6de059a`. Only the new **33 title rows** are added. Their entire ordered table equals the artifact's complete provenance/source join. All five new blobs and five prior blobs match manifest SHA-256/length declarations. New section schemas are 27; metadata changes only schema version and Core's logical hash. Across every actual provenance-bearing table in all five sections, **1,222 distinct full IDs** resolve to nonblank compiled titles. No current artifact family was omitted.

[Published-wire verifier](evidence/2026-10-03-provenance/wire-probe.py), [complete results](evidence/2026-10-03-provenance/wire-probe.txt): all **eight** AQC provenance properties reference the one `Provenance` schema, whose complete shape is required string `id`/`title`. All **68 provenance values** across AQC fixtures, AGC served fixtures and HTTP/CLI pact bodies have exactly that shape and nonblank values. The previously documented gazetteer file export is excluded from served-wire scope. Source audit finds one production `wire::Provenance` constructor, in `atlas_contract::provenance::titled`; every NodeRecord, EdgeRecord, EventPage attribution, EventAnalogue, CrossRef, CatechismRef and CLI node path uses that door. CLI text emits the title; CLI JSON preserves both fields.

**F39 integrity:** the new `provenance_title` spec is registered in Core extras/sidecars and `graph-types::extra_tables_of(Core)`. Folding attaches its canonical rows before version publication; SQLite's logical dumper reads it through the same extra-table encoding. [Exact encoding extraction](evidence/2026-10-03-provenance/exact-title-encoding.rs), [binding property](evidence/2026-10-03-provenance/title-binding-probe.rs), [complete results](evidence/2026-10-03-provenance/title-binding-probe.txt): **66 generated changes** cover both columns of every actual title row; every change moves complete production-encoded table bytes and their table-fragment hash, and every rollback restores the whole table. This independently exercises the production extra-table reader/encoder; it is not claimed as a recomputation of the whole Core hash. The exact production extra-table module SHA-256 is `153184971cfa6c9655ca80f93f63ba86f831ccb562fab5a80d4e72502ce9ffab`. F-85's persisted-ordinal fix remains intact.

## Review passes and gate limits

**14b:** title lookup and wire construction have one shared serving door and a private lookup map; endpoint-specific re-derivation is avoided. The open category is registry validation duplicated incompletely beside its existing owner, with unchecked construction and first/last-wins ambiguity. Reuse the validated abstraction rather than adding a site-specific condition. Existing wire-identity work is not re-reported.

**24a/24b:** all eight published wire sites and the ninth CLI site migrate together. Positive artifact coverage is complete for current data. Admission closure is incomplete for malformed registries, as the generated refusal counterexamples show. The positive committed-artifact law checks coverage, not refusal of ambiguous input. The already proposed legacy client attribution join and gazetteer export are recorded on the item; no duplicate finding is filed for them.

**9 / 18 / 25 / 26 / 27:** diff whitespace passes; added application comments are absent and new test comments are Arrange/Act/Assert. New titles derive from curated registry data in the tool layer, then are read by key from the artifact. The wire adds a domain attribution value rather than a view-specific endpoint. C# changes consume `.Id` only as authorized under the freeze; title display belongs to the F# backlog. The small new serving helper has its entry point before its collection helper. This is not a certification of historical repository-wide source order.

The ops submission reports workspace **1512/0**, 11 ignored; graph-types **150/0**; C# **737/0** and **55/0**; contract gate passed against `b3d7cfa`; timing **11/11**; provenance Playwright **85 passed / 2 skipped**. Those full gates were not independently rerun. No standalone author close report for this item is committed at the reviewed head; its queue submission supplies scope/gates/red evidence. Existing main-target libraries predate this feature, so their test binaries are not treated as current-head proof.

The retained probes use small direct `rustc` compilations with existing libraries; no Cargo target is created. Windows C: had approximately **3.7 GB free** while WSL had hundreds of GB free. No full Rust, browser, AOT or server run was started. Artifact copies use bounded `/dev/shm` temporaries. Original artifacts, raw/cache and Claude/owner outputs remain untouched.

**Handoff:** root allocates/publishes the Important admission finding on ops. Reuse one registry admission abstraction and extend its generated refusal laws, then request exact-head re-review. No ops edits, Codex lock, Cargo build, server or disposable probe database remain.
