# Parallel F# frontend plan

Base: `ace063d`. Item CX-FSHARP; worktree `~/w/CX-FSHARP`. Owner requires Bolero WebAssembly and Elmish MVU. Existing application paths stay read-only. Type catalog: `docs/superpowers/specs/2026-10-02-fsharp-client.md`.

1. Inventory every C# source/component and UX scenario; pin hashes and give each a pending parity entry. Install/pin framework and package dependencies without altering the other client's SDK/config.
2. Red tests for immutable contract generation (all schemas and discriminators); generate F# into build output; deserialize the complete committed fixtures and refuse unknown vocabulary. No C# app dependency.
3. Red laws for latest-request-wins, retry/root changes, bounded cache and exploration StateT/Async/Result, including monad identities/associativity, duality/breadcrumbs, batched Resume and missing ids.
4. Add Elmish application model/messages/effects and typed route adapters. Build/publish a real Bolero WASM client; shared presentation/map assets through build links. Test initial routes before implementing each one.
5. Migrate Reader/Concord/Sources, text/anchors and contents/pickers; generic FocusView/entry words/paging; remaining approved legacy shapes; storage, selection and saved trails; Kretzmann; World/map/time; split/follow and error/accessibility states. Match DOM test handles and house styles.
6. Run existing UX scenarios against each client plus a differential parity harness. Full browser runs use heavy; artifact regeneration is not part of this task. Fix migration defects by category, keep Claude's branch untouched.
7. Refresh against approved FOCUS changes; review all fields/routes/interactions and pending inventory. Gates: F# unit/property/contract/bUnit tests; browser WASM publish; Chromium/WebKit differential and existing UX gates; resource laws; source/comment/license and C#-dependency laws. Mutation remains the recorded owner-window debt.
8. Push lane, set review with exact range, report concrete gate results and zero-pending parity evidence, and leave Claude a completion note on ops. Keep C# runnable until parity/rollout ruling.

Each step records red-before-green evidence in the report. No completion claim for a scaffold, partial slice or unrunnable code. Standing FOCUS reviews take priority over migration implementation when Claude requests them.

## Claude review refactor order, 2026-10-03

Address C1 (nominal generated identities), C2 (closed surfaces and operation-specific retries), then I8/I4/I6/I7 (typed failures, valid anchors, enum spelling, typed parameter records), I2/I3/I5 (presentations and bounded trails), and C3/I1 (behavioral parity and closure) before expanding features. Convert example tests to properties alongside each step. The named scalar portion of C1 is now verified; unnamed identity schema work remains the separately proposed A-WIRE-IDENTITIES. The unused Cache was removed. The source-order law currently checks module helpers/test fixtures; finish public/local/member ordering before claiming every file complies. See the durable refactor checkpoint report for red test names and practical gate limits.
