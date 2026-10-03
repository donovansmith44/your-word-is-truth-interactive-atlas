# A-NOBLURB review

Codex reviewed `ace063d..e57542c` on `lane/claude/NOBLURB`, in an isolated worktree. This is the backend completion of the owner's instruction to remove location blurbs; the client removal was already included in the approved FOCUS-2 tree.

Verdict: APPROVED. No new finding. This does not claim that the complete application has green gates: the author's workspace run reports the two existing font/source-license parity failures on trunk, and the world browser run carries the existing density failure. A-LICENSE-BOC addresses the former separately.

The removal covers PlaceDetail and its published schemas, geography defaults, core history types and resolution, the ETL input model and validation, SQLite declarations and fold/unfold, the Core section inventory, and the presenter. The remaining curated history still feeds names and establishment/destruction claims. The unread curated blurb entries are the already recorded owner decision; this review does not delete them.

The category pass (24a/24b) finds a complete removal of the served/stored blurb capability: the owning records have no field, the compiler has no input member, and the section has no blurb table or default column. There is no alternate active blurb accessor in the application sources. Contract generation and table-inventory laws cover the declarations. The D.R.Y./Haskell-bar pass (14b) finds no new duplicated representation, open replacement vocabulary, partial input operation, or runtime substitute for a removed type. Principles 27–27g are unaffected: no new query, scan, client collection, or geometry transport is introduced. The diff adds no application comments.

Independent gates:

| Gate | Result |
| --- | --- |
| C# client tests | 737 pass |
| C# contract tests | 55 pass |
| Core history tests | 33 pass |
| ETL place-history tests | 13 pass |
| Graph geography, reload, lexicon sections, SQLite laws | 43 pass |
| Contract generation and graph API | 135 pass |
| graph-types all features | 148 pass |

The complete graph API suite includes the whole Jerusalem record with retained coordinates and dated founding/destruction. Independent SQLite comparison checked **496,339 retained rows across 56 Core tables** against `ace063d`: every projected row is identical. The only removed table is `place_history_blurb`; the only removed column is `place_default.blurb`; the only changed Core metadata keys are `logical_hash` and `schema_version`. All five committed compressed blobs match their manifest SHA-256 and length, open with section schema 25, and contain no blurb table. Other sections retain their logical hashes.

Logs are `/tmp/codex-noblurb-{client,contract-client,core,etl,graph,rust-contract,graph-types,artifacts,core-parity}.log`. The first attempt to invoke a nonexistent `client.ContractGenerator.Tests` directory was corrected to the actual `client.ContractTests` project; only its successful run is counted. Read-only artifact probes were corrected to handle streaming Zstandard frames and expected metadata changes. No artifact, fixture, schema, application source, or owner server was modified.

Author evidence read from the ops item: contract gate passed; timing 11/11; world specs 12/13 with carried density smoke; full workspace has the two failures reproduced on trunk `e558f07`. Mutation remains deferred under the owner's recorded window. Those full gates were not repeated under Claude's heavy lock.

Claude handoff: the change is approved for the normal lock/squash landing protocol. Preserve the existing failures and owner decision visibly; follow with the separately claimed licensing closure. The F# migration will consume this reviewed contract before asserting place-view parity.
