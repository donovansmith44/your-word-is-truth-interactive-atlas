# A-F39: Codex review

Reviewed 2026-10-03: exact `a019cf2..8b23906` on `lane/claude/F39`.

**CHANGES REQUESTED.** Labels, adjacency entries and counts now participate in the logical hash through shared declarations. The requested broader closure over every section table remains incomplete: coordinated physical row-address changes can preserve the complete logical dump while changing the server's provenance answer. Proposed finding **F-85, Important**.

## F-85: physical row addresses can change served provenance without changing the root

The category is persisted row addressing omitted from the logical representation, with cross-table consumers outside the checked invariants. `logical_dump_of_db` discards the `(ord, row)` reader's ordinal when encoding row-family bodies. `rows::select_rows` checks only `id == ord`. The new cell law changes those columns individually; its refusal on unequal columns does not prove that their consistent paired changes are fenced.

The served `SqliteSnapshot::rows_behind` resolves `edge_index.row_id` by querying the physical family table's `id`. That address affects the answer independently of the canonical row body. The index's `row_id` is hashed, but its agreement with the family's actual addresses is not checked by the logical dump.

**Actual-artifact reproduction:** unpack only the reviewed Core blob into a disposable copy. Its last `located_at` row has `id = ord = 954`. The two index entries for edge `26210174eac8024a8961da7ef193a30a` reference that row and return `curated` provenance. Run:

```sql
UPDATE located_at SET id = 986, ord = 986 WHERE id = 954;
```

The freshly compiled, exact reviewed `logical_dump_of_db` still succeeds and returns **byte-identical output**, retaining the Core hash `4e9c3ce4bc28ee4dbfe72cc7949cc1f0`. The complete provenance-lookup result changes from:

```text
[(26210174EAC8024A8961DA7EF193A30A, 954, curated),
 (26210174EAC8024A8961DA7EF193A30A, 954, curated)]
```

to `[]`. This is the indexed row lookup used by the production provenance port; it is not a browser reproduction. All original artifacts, raw data, caches and Claude's outputs remain untouched.

The same generated property holds for **32 distinct coordinated id/ord reassignments** on a small production-DDL database. Whole dump equality, hash equality, and the whole missing provenance result are asserted. A positive property makes **96 generated changes** across `label`, `edge_index` and `edge_count`; every one changes the production hash. Thus the derived-table repair itself works; the broader addressing category is still open.

**Sites:** `server/atlas-graph/src/sqlite/logical.rs` row-family branch; `sqlite/rows/mod.rs` `select_rows`; `sqlite/snapshot.rs` `rows_behind`; `graph-types/src/sections.rs` family dump encoding; the new `every_cell_of_every_table_a_section_file_holds_but_meta_is_under_its_logical_hash` law. Audit the addressing category across every family and cross-table reference, including justification, ground and containment child tables; do not patch only `located_at`.

**Proposed closure:** include persisted addresses in the shared logical representation on both the graph and SQLite sides, preserving domain canonical identities separately; alternatively, refuse every address/index/reference inconsistency through one enumerating admission invariant. Enumerate coherent multi-column changes, not only isolated invalid cells, and assert changed accepted read answers imply changed roots. The owner/controller decides the mechanism. No application fix is made by this review.

## Independent verification

Durable source and complete compact results are committed beside this report:

- [Generated addressing and derived-table probe](evidence/2026-10-03-f39/paired-row-probe.rs), [complete output](evidence/2026-10-03-f39/paired-row-probe.txt).
- [Whole-artifact parity verifier](evidence/2026-10-03-f39/artifact-parity.py), [complete output](evidence/2026-10-03-f39/artifact-parity.txt).

Every one of **6,079,426 non-meta rows across 102 section tables** is unchanged between `a019cf2` and `8b23906`. Every table schema matches. Metadata excluding `logical_hash` matches. All ten before/after section blobs match their declared SHA-256 and byte lengths; all sections remain schema 26. The recomputed actual Core baseline hash also matches the reviewed manifest. The artifact rebuild is a root-format change, with no source-content change demonstrated by this comparison.

The probe compiles the reviewed `sqlite/logical.rs` by path (SHA-256 `c82cd8e71c92a1c112dca088af1402ad25de9e789547da9474a1ce4950769009`). It reuses existing compatible pre-PROV graph-types, SQLite row readers and DDL libraries read-only, writing only its small disposable executable/database. The row readers used by the finding are unchanged in this range. It does not invoke Cargo or rebuild the workspace.

The exact compile invocation was:

```bash
. /home/donovan/.bible-atlas-env
nice -n 10 rustc --edition=2021 -C debuginfo=0 -C strip=debuginfo /tmp/codex-f39-probe/main.rs -L dependency=/home/donovan/mut/claude-F39/debug/deps --extern atlas_graph=/home/donovan/mut/claude-F39/debug/deps/libatlas_graph-5cc610fcb77b4c3f.rlib --extern atlas_graph_types=/home/donovan/mut/claude-F39/debug/deps/libatlas_graph_types-d68558cf13345bc6.rlib --extern rusqlite=/home/donovan/mut/claude-F39/debug/deps/librusqlite-3003122ca614f219.rlib -o /tmp/codex-f39-probe/check
```

For later reproduction, use the committed probe source as that input and resolve compatible libraries freshly; the absolute production-module path must point to an `8b23906` checkout. The original `/tmp` source/output are conveniences, not the retained evidence.

**Gate limits:** the current queue reports workspace 1,507/0 with 11 ignored timing tests, graph-types 150/0, C# 737/0 and 55/0, contract gate passed against `b3d7cfa`, and timing 11/11 at `8b23906`. Author task outputs with the aggregate workspace/timing results were read. Those full gates were not independently rerun. Existing target directories contain stale intermediate test binaries; a run of one produced four passes and a pre-final logical-roundtrip failure. That result is not attributed to `8b23906`, not counted as a current gate, and not filed as a second finding. The new independent probe compiles the exact reviewed logical module and reproduces F-85 on the pinned artifact instead.

Windows C: had approximately 3.5 GB free while WSL reported hundreds of GB free. No fresh Rust-heavy build, full workspace, AOT or Playwright run was started. Artifact parity used bounded temporary files on `/dev/shm`, which were removed after each section. Mutation remains deferred. OpenAPI/AQC changes are version-only; fixtures, pact and exports carry the expected new root.

## Required review passes

**14b / D.R.Y. and the Haskell bar:** the index partition, column declarations and value encoders move into one graph-types owner consumed by both writer and logical dump; duplicate partition logic is removed. The new fallible index construction propagates a named error instead of hiding a missing edge-row mapping. Node kind/pid/provenance copies are checked against the decoded payload. The remaining address/index relation is not structurally carried, which is F-85's category rather than a second instance finding.

**24a / 24b:** the original unhashed derived-table category is correctly named and migrated. Closure additionally claims every persisted table/cell; independent coherent mutations contradict that wider claim. The single-cell law is useful evidence but insufficient to establish that coordinated valid states cannot change answers under one root.

**Rules 9 / 18 / 25 / 26 / 27:** diff whitespace is clean; no new application comments, only Arrange/Act/Assert test comments. New encoding helpers follow their callers. There is no client feature or domain-data edit; compiler derivation remains shared and served shapes unchanged. Bounded generic reads remain in place. Existing unrelated findings are not re-reported.

**Handoff:** address the row-addressing category and extend the generated law before landing A-F39, then request re-review of the exact new head. No Codex lock, Cargo build or server is held. This review makes no ops edits; the root agent publishes its verdict/finding on the shared queue.
