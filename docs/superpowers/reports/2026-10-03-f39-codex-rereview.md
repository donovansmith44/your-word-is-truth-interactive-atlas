# A-F39: Codex re-review

Reviewed 2026-10-03: exact `a019cf2..6de059a` on `lane/claude/F39`, including F-85 repair `474c98d` and citation pin `6de059a`.

**APPROVED. F-85 is closed for this reviewed range. No new finding.** The logical representation now includes each row family's persisted ordinal through one shared encoder. The previous coordinated address/provenance counterexamples now move the logical hash. This supersedes the changes-requested verdict for `8b23906` in [the original review](2026-10-03-f39-codex-review.md), whose red evidence remains retained.

## The addressing category

`graph-types::sections::row_line_body` is the single encoding of family, persisted ordinal and canonical row value. Both graph and SQLite section dumps use it for every `RowFamily`. The graph match exhaustively enumerates the closed family vocabulary. Its macro enumerates before filtering for split families (`ContainsBible`, `CanonSuccession`, `CrossRefs`), preserving global writer addresses across sections. SQLite uses the actual ordinal returned by `read_rows`; the existing reader invariant refuses unequal physical `id` and `ord`. Domain row/edge canonical identities retain their own encoding.

This migrates the whole address category into the logical binding. Changing the index's reference is captured by derived-table encoding; changing the family's corresponding physical address now changes that family's line or is refused. The new author law walks every family's table in every written section and changes `id` and `ord` together with child-table references. It complements the earlier every-cell/every-table law. Its child-table discovery does not move `edge_index.row_id`; the independent old counterexample below deliberately retains that reference and verifies the resulting changed read cannot reuse one hash. The dumper is not claimed to become a complete database referential-integrity validator.

## Independent evidence

[Generated addressing probe](evidence/2026-10-03-f39-rereview/paired-row-probe.rs), [complete result](evidence/2026-10-03-f39-rereview/paired-row-probe.txt): all **32 generated coordinated id/ord reassignments** now change both complete production dump bytes and their hashes. The whole provenance join changes from one expected row to `[]`, as in the original red proof. All **96 generated derived-table changes** across labels, index entries and counts also change their hashes.

The actual-artifact reproduction was repeated on a disposable copy of the exact `6de059a` Core section. Moving `located_at.id = ord = 954` to 986 changes the hash from `6030eb377ec609dff905feb676c80720` to `95ebbb5473afa21d3b45fb85193fde07`. Both complete curated provenance links for edge `26210174eac8024a8961da7ef193a30a` disappear. The changed answer therefore no longer retains the original binding. This is the production indexed SQL lookup, not a browser reproduction.

[Whole-artifact parity result](evidence/2026-10-03-f39-rereview/artifact-parity.txt), using the [retained verifier](evidence/2026-10-03-f39/artifact-parity.py): every **6,079,426 non-meta row across 102 tables** is unchanged from `a019cf2` to `6de059a`. All schemas and metadata except `logical_hash` match. All ten before/after blobs match their declared SHA-256 and lengths; section schemas remain 26.

[Section hash probe](evidence/2026-10-03-f39-rereview/section-hash-probe.rs), [driver](evidence/2026-10-03-f39-rereview/verify-section-hashes.py), [complete result](evidence/2026-10-03-f39-rereview/section-hashes.txt): the exact reviewed production logical dumper independently recomputes **all five** committed section hashes, matching the new manifest. This includes the split-family sections; artifact comparison alone would not establish encoding/manifest agreement.

The probes compile `server/atlas-graph/src/sqlite/logical.rs` directly from the reviewed tree (SHA-256 `7df9497dba312458a33743c0ddf1d757c390a91d692763bd0bf18153189fbc6a`). Its shared `row_line_body` is compiled verbatim from reviewed source in [this retained extraction](evidence/2026-10-03-f39-rereview/exact-row-line-body.rs); `graph-types/src/sections.rs` SHA-256 is `3dfc87b2bd6e7472921a107437732fc4415e454471f8b859d2877f346b8b297b`. Existing compatible pre-PROV libraries supply row readers, DDL and canonical types read-only. The readers are unchanged since `8b23906`; no old test binary is counted as an exact-head gate.

For reproduction, source `~/.bible-atlas-env` and compile each committed Rust probe independently with `rustc --edition=2021 -C debuginfo=0 -C strip=debuginfo`. Use dependency directory `/home/donovan/mut/claude-F39/debug/deps` and compatible libraries `libatlas_graph-2787a68c613bad7e.rlib`, `libatlas_graph_types-5925403570d0e42b.rlib`, `librusqlite-3003122ca614f219.rlib`. The production/extraction module paths must point to this reviewed checkout; resolve library fingerprints freshly if those disposable targets were removed.

## Review passes and limits

**14b / D.R.Y. / Haskell bar:** persisted-address encoding is declared once and called from both dump implementations. The graph macro shares enumeration/filtering rather than repeating encoder logic per family. The exhaustive family match keeps the vocabulary closed. The original range's index columns, values and partitioning have one graph-types owner, with fallible missing-row mapping. Repeated specimen setup was extracted into a shared test helper. No additional duplicate domain fact or input partiality was found in this re-review. The original report's source-order observation remains recorded; this verdict does not certify repository-wide newspaper ordering.

**24a / 24b:** the named open category was persisted addresses omitted from the logical binding. All family sites now migrate through the shared encoder, with family enumeration and coherent-mutation laws complementing the schema-enumerating cell law. The formerly accepted same-hash/changed-provenance states are independently disproved at the new head. The original derived-table category remains fenced by shared declarations and the every-table law.

**9 / 25 / 26 / 27:** `git diff --check` passes. Added source comments are only test Arrange/Act/Assert lines. This derives artifact integrity in graph/compiler storage code; it adds no client domain interpretation, served collection scan or app-specific read. OpenAPI/AQC changes are version-only; fixtures/pact/exports carry the rebuilt root. Known unrelated findings are not re-reported.

The ops queue reports author gates: workspace **1508/0**, 11 ignored; graph-types **150/0**; C# **737/0** and **55/0**; contract gate passed against `b3d7cfa`; timing **11/11**. These aggregate gates were not independently rerun. Independent claims above concern retained exact-module probes and full artifact comparisons. Mutation remains deferred under the recorded owner window. Windows C: still had approximately **3.5 GB free** despite ample WSL filesystem capacity. No fresh Cargo build, full workspace run, AOT or browser suite was started; bounded artifact copies used `/dev/shm` and were removed.

**Handoff:** Claude may land this exact reviewed range using the normal lock/squash protocol; root publishes the verdict and closes F-85 on ops. No Codex lock, Cargo build, server or disposable probe database remains. This review makes no ops edits and changes only report/evidence files.
