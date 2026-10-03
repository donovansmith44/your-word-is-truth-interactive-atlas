# A-LICENSE-BOC: Codex review

Reviewed 2026-10-03: exact `e57542c..a019cf2` on `lane/claude/LICBOC`.

**APPROVED for the queued exclusion fix.** The identified F-79 material leaves the served corpus through one curated exclusion mechanism, and the font/source reconciliation failure is addressed. This approval does not establish that every possible input is Triglot text. F-84 records that remaining admission category for an owner-directed follow-up.

## Scope and correctness

The brief requires curated exclusions with reasons and Triglot checks, removal before paragraph numbering, exact fragment edits, rejection of unmatched curation, laws against the identified exclusions/markers, corrected licensing attribution, and rebuilt/re-pinned artifacts. The implementation covers those requirements through `ConcordExclusions`, its closed `NonTriglotKind`, `ExclusionLedger`, and the production `read_all` pipeline.

Independent execution of the exact production `concord.rs` confirms the cleaned raw corpus contains **3,802 paragraphs**, with **3 excluded articles**, **4 excluded standalone units**, and **9 stripped fragments**. The queue previously said 10 fragments; the actual TOML and production statistics say 9. The 25 removed paragraphs are the 21 Christian Questions plus four standalone units. Excluded units are filtered before numbering, so the two phantom Apology entries no longer shift the real paragraphs.

The public OpenAPI shapes are unchanged apart from the version. AQC advances to 0.27.0, AGC to 0.26.0, graph-types to 0.15.0, section schema to 26. The committed fixtures and HTTP pact follow the rebuilt artifact. All five compressed section blobs match their declared SHA-256 hashes and byte lengths. Read-only inspection of the actual Concord section confirms schema 26, logical hash `78964fddd8f70406e146162f67de7be1`, and 3,802 `concord_unit` rows. This is an integrity check, not an independent recompile.

`LICENSES.md` now distinguishes ingested sources from bundled client assets. Its existing per-source parser already stops at the next level-two heading; moving the font rows into their own table fixes the source-parity domain rather than adding asset-name exceptions.

## Independent evidence and limits

| Check | Result |
|---|---|
| C# client suite | 737 passed; `/tmp/codex-licboc-client.log` |
| C# contract suite | 55 passed; `/tmp/codex-licboc-client-contract.log` |
| Exact production Concord parser / baseline | Passed; `/tmp/codex-licboc-probe.log` |
| Generated 16-input admission property | All 16 unverified modern replacements admitted; same log |
| Committed artifact blob/length checks and Concord SQLite inspection | Passed; `/tmp/codex-licboc-artifacts.log` |
| Diff whitespace / new application comments | Clean; only new Arrange/Act/Assert test comments |

The standalone probe includes the unchanged production module by absolute path and links the already compiled `anyhow`, `serde`, and `toml` dependencies. Its only local adapter reproduces the existing curated-title deserializer, which is outside the admission claim. Probe source: `/tmp/codex-licboc-probe/main.rs`.

A focused Cargo ETL gate was started, then deliberately terminated after discovering Windows C: had only approximately 1.4 GB free although the WSL filesystem reported approximately 802 GB free. It **did not pass** and is not counted as verification. Its partial log is `/tmp/codex-licboc-etl.log`. Only this review's stopped disposable 444 MB target directory was removed after checking no compiler used it; reports, source, raw/cache data, worktrees, and Claude's outputs were preserved.

Author-reported workspace 1,507/0, graph-types 134, timing 11/11, contract export gate, and Concord browser 20/20 results are recorded in the queue. They were not independently rerun here; historical `/tmp` logs were unavailable after cleanup. Mutation remains deferred under the standing window rule. No whole-stack green claim is made.

## F-84: positive Triglot admission remains open (Important)

**Category:** unverified source text acquires the corpus's public-domain provenance merely because it is not on the exclusion inventory and does not contain a marker.

**Sites:** `server/atlas-etl/src/concord.rs` production `read_all`, `no_excluded_material_is_served`, and `no_served_text_carries_a_non_triglot_marker`; `data/curated/concord-exclusions.toml`; the real-data test named `the_served_concord_is_triglot_only_no_excluded_piece_and_no_non_triglot_marker_is_served`; the stronger Triglot-only assurance in `LICENSES.md`.

**Reproduction:** generate 16 distinct paragraphs about contemporary semiconductor firmware, replace the first Apostles' Creed paragraph in a copied raw corpus with each candidate, and call the actual production `read_all`. Every candidate is returned verbatim as successful corpus text. Paragraph counts remain unchanged, every existing exclusion matches exactly once, all fragment strips succeed, and both compile laws pass. The actual repository raw inputs are untouched. This is a generated property over the admission path, not a claim that these invented paragraphs already occur in the checked-in artifact.

The current corpus cleanup meets its explicit known-inventory brief. The wider 24b guarantee is incomplete: an offender is still writable. The author's own F-79 report identifies this and proposes a future Triglot-reference gate, but the proposal was not filed in the queue.

**Proposed closure:** introduce a positively verified Triglot admission boundary in the ETL. A vendored permitted reference and curated alignments/dispositions must account for every served text span and title; unmatched material is refused pending curation. A law must enumerate the entire admitted corpus and generated insertions/replacements across every document and text shape. OCR tolerance or explicitly reviewed variations belong in provenance-bearing data, with a bound that cannot silently admit unrelated prose. The served system continues to consume compiled data only. The owner decides the exact mechanism and batch; no source, contract, or application patch is made by this review.

## Required review passes

**14b / D.R.Y. and the Haskell bar:** one exclusion schema and ledger own the three exclusion forms, one closed enum names the four reasons, and all ten documents use the same pipeline. Counts derive from the ledger. The font fix does not duplicate an asset-name list. No new instance-specific application abstraction is added. Existing domain tables and raw-build server access are already filed categories, including F-83; they are not re-reported.

**24a / 24b:** the known site-furniture/modern-text instances move into curated facts with cited reasons, fail on unmatched/ambiguous entries, and run through one tool-layer door. The positive licensing category is expressly still open as F-84. This distinction is the approval boundary.

**Rules 9 / 18 / 25 / 26a / 27:** no new application comments; new helpers follow their entry point; no client domain derivation or new HTTP endpoint; the cleanup runs in ETL, and served wire shapes remain unchanged. The already filed F-83 raw-build dependency remains outside this task.

**Handoff:** the scoped fix may land under normal squash/lock protocol. File F-84 and the positive-admission proposal on ops. No Codex lock, build, or server remains. A-F39 is still claimed on the current queue, so its review waits for a submitted range and gate results.
