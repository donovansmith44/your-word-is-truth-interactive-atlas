# Engineering Principles

Owner-declared, 2026-09-26. These bind every change to this repository, by
any author, human or agent. A spec or plan that conflicts with this file is
wrong; fix the spec.

## Correctness

1. **Test-driven, always.** Red, then green, then refactor. No production
   line is written before a failing test demands it.
2. **Any code that doesn't make a test pass gets deleted.** If removing a
   line breaks no test, the line is either dead or untested; both are
   defects. Delete it or write the test that needs it.
3. **100% mutation coverage, minus logical equivalents.** Every surviving
   mutant is either killed by a test or recorded as an equivalent mutant
   with its reason, in the mutation tool's configuration, never in an
   inline comment. Tooling: Stryker.NET for C#, cargo-mutants for Rust.
3a. **Mutation runs once per batch at its close, concurrently -- or less
   often, by the owner's call.** A batch is the program's unit as the owner
   names it (CONTRACT-1 is 1a and 1b together), not a plan or a task. A task
   proves itself with its tests and the pacts; the mutation run is a gate
   over every line changed since the LAST run, Rust and C# in one pass,
   sharded so it finishes in hours, not days. A run measures the current
   tests against the current code and does not care which batch changed a
   line, so one run over several batches is the same measurement as one per
   batch, and nothing is "made up" afterwards (owner, 2026-09-29: "running
   them once or twice gives enough information"). When the owner wants the
   wall-clock for implementation instead, the run is deferred and
   `.superpowers/MUTATION-GATE-DEBT.md` names the base the next one measures
   from. Surviving mutants get one fix dispatch of tests; equivalents go in
   the tool's configuration with their reasons.
4. **Zero lines of dead code.** No unreachable branches, no unused members,
   no "kept for later", no commented-out code, no backwards-compatibility
   shims for callers that no longer exist.

## Simplicity

5. **As simple as possible; no simpler.** Essential complexity is handled
   elegantly and named for what it is. Accidental complexity is eliminated,
   not managed.
6. **One source of truth.** A fact is declared once and derived everywhere
   else. Where derivation is mechanical it is generated and gated; where it
   is not, it is served as data. The client never re-derives what the graph
   knows.
7. **Clean architecture; a modular monolith.** Modules own their concepts,
   expose intent-revealing interfaces, and depend inward on the domain,
   never outward on infrastructure.

## Readability

8. **Public-facing interfaces speak the user's domain.** A name on a
   public interface maps to a concept the application's user already
   holds: verse, passage, place, person, exploration, focus. Technical
   vocabulary stays inside the module that needs it and never leaks into
   the surface a reader of the domain code meets first.
9. **No comments in application code.** Owner, 2026-09-30: "no comments in
   my app code stop doing that." Not `//`, not `///`, not `//!`, not `<!-- -->`,
   and no `why` exemption: a hidden constraint becomes a name, a type or a
   test. A description the published contract needs is a
   `#[schema(description = "…")]` attribute, not a doc comment. Generated
   files carry exactly one header line naming their source; that line is
   the whole exemption.
10. **Comments in tests are `Arrange` / `Act` / `Assert` only.** Nothing
    else.
11. **Readable without a guide.** A reader who knows the domain can follow
    the code without the commit log, the ledger, or the spec.

## Process

12. **Specs show the types.** A design is not approved until the owner has
    seen every new interface, type, and signature it introduces.
13. **Generated documents are committed and gated; generated code is
    built.** A published document (a contract, a schema) is committed, and
    a test regenerates it and fails on any byte of difference, so a hand
    edit cannot survive the gate. Code derived from such a document is
    produced by the build into untracked output and never committed.
14. **Verification before claims.** "Done" means the gates ran and were
    read, not that the code was written.
14a. **Slow builds are investigated, not endured.** When a build or gate
    runs materially slower than its recorded baseline, the cause is
    investigated in parallel with the work (profiling, caching,
    parallelism, incremental compilation) and the finding recorded. A
    ceiling is never raised to make a gate pass.

14b. **Every review carries a D.R.Y. pass and the bar.** A task review, a
    scoped re-review and the final whole-branch review each ask, as their own
    step: is anything here declared twice (a type restating another, a list
    or literal repeated, logic copied instead of called, a fact re-derived
    that its declaration already holds)? And: would a Haskell programmer
    scoff at it (a `String` where the vocabulary is closed, a tuple where a
    record belongs, a partial function on input, a runtime check the type
    system could carry)? A hit is a finding, at least Important, whatever
    the plan said.

## Tests as documentation (owner, 2026-09-27)

15. **Whole-body assertions.** A test asserts the entire result — the whole
    record, the whole list, the whole document — against one expected value
    written out in the test, not a field or two picked from it. A reader
    learns what the code produces by reading the expected value.
16. **A test is compilable documentation.** Its name states the behaviour in
    a sentence; its body is Arrange / Act / Assert and nothing else; one
    behaviour per test; it is obvious what is going on without reading the
    code under test. Think Uncle Bob.
17. **No magic numbers.** Every literal that means something is a named
    constant whose name says what it means (`DECLARED_EDGE_KINDS`, not
    `46`), in tests and in application code alike.
18. **Newspaper order.** A file reads top-down: the public entry point
    first, then what it calls, then what those call. Helpers sit below
    their first caller, never above.

## Running work is not to be edited under itself (2026-09-29)

19. **Never edit a shell script while an instance of it is running.** Bash
    re-reads a script by BYTE OFFSET, so an edit mid-run resumes the running
    instance at the wrong place. CONTRACT-1's mutation gate lost its merge
    step this way after four hours of shard work (the shard outcomes
    survived; the merge was redone by hand). The same rule already held for
    the tree under a `cargo mutants --in-place` run: edit a copy, or wait.
20. **A throwaway git worktree never has ignored data linked into it.**
    `git worktree remove --force` FOLLOWS an NTFS junction or symlink and
    deletes the real files through it — this destroyed 374 MB of `data/raw`
    on 2026-09-28. A shard copies what it needs (`robocopy`), or every link
    is removed with `cmd /c rmdir` BEFORE the worktree is. Scan for reparse
    points and require zero before any `worktree remove`.

## Parallel work and its critical sections (owner, 2026-09-29)

21. **Serialize the shared artifact, parallelize everything else.** The
    program's batches and their tasks run concurrently by default. Only these
    are critical sections, held by ONE agent at a time:
    - the mutation gate (PRINCIPLES 3a already makes it once per batch, which
      is itself the mutex — no two gates can overlap if no batch runs two);
    - regenerating `contracts/openapi.yaml` and `aqc.schema.json`;
    - re-blessing pacts and fixtures, and anything that moves the version root;
    - appending to `relations!`, which is positional.

    Everything else — implementation, unit and integration tests, reviews,
    client work — runs in parallel. The critical sections are minutes; the
    work around them is the wall-clock.
22. **A batch names its base commit explicitly.** While batches interleave,
    "every line this batch changed" is only well defined against a written-down
    base (`--base <sha>`, never `@{upstream}`, which moves the moment anyone
    pushes — CONTRACT-1a's ruling R34 was exactly this bug). The base goes in
    the batch's ledger at its first task and is passed to every gate.
23. **Pair unlike work.** Memory, not cores, is this machine's ceiling (the
    mutation gate died at N=8 and was reaped). Two Rust-heavy jobs contend; a
    Rust-heavy job beside a C#-heavy one does not.
24. **A bug is a category, never an instance.** Owner, 2026-09-30: "NEVER fix a
    regression in isolation as though things in the same category cannot occur
    similarly. Consider the program design, and whether a fix ought to be client
    or server side given our goals of separation of concern and D.R.Y. If a bug
    occurred, by definition there is a category of behavior that is wrongly
    captured by our type system, and it more than likely means that there needs
    to be migration of code to live under the agreed upon abstractions rather
    than writing new code. The latter is hacky and evil." So every fix first
    names the category and the abstraction that failed to capture it, decides
    the side by separation of concerns, lists every other site in the category,
    and migrates them all under the one abstraction. The red test pins the
    category. A site-local patch is a defect.
24a. **The review sweep checks rule 24, and offenders are reported, not
    fixed.** Owner, 2026-09-30. Every review (a task review, a scoped
    re-review, the whole-branch review) carries a category pass beside the
    D.R.Y. pass and the bar: for each fix in the diff, was the category named,
    the failed abstraction named, the side chosen, and every site migrated? A
    site-local patch is a finding. Boy scout rule: whenever anyone finds code
    outside the agreed abstractions, or a same-category site the fix did not
    migrate, they REPORT it as a finding, with the category and the sites, and
    the owner decides how it is addressed. Nobody fixes an offender on the side.
24b. **Closure: offenders cannot exist.** Owner, 2026-09-30: "I want CLOSURE
    under this principle, meaning that structurally, offenders cannot exist.
    This is how we guarantee correctness of behavior rather than tracking down
    countless bugs." A category is closed only when the program's structure
    makes an offender unwritable, not merely absent: the raw thing (a rows
    table, a label string, a transport call, a text scan) is private to the
    module that owns its abstraction, and that abstraction is the one public
    door; a closed vocabulary is an enum, so a new case fails to compile until
    every match handles it; a law enumerates the category through the type
    system (`RowFamily::ALL`, `NodeKind`, the schema's enums, the client's
    generated types) so a site the type system cannot fence is caught by a
    test that walks every member. A fix under rule 24 is finished when its
    category is closed and the report names the guarantee. A FINDING is an
    open category: its report proposes the closure, and the owner decides.
25. **The client composes over the contract, and nothing else.** Owner,
    2026-09-30: "we adhere to our swagger and write minimal client side code.
    Its purpose is to compose over the api results." The client's only
    knowledge of the domain is the generated contract types (`Wire.g.cs` from
    `contracts/openapi.yaml`); it reads served labels, loci, anchors, runs and
    details and composes them into views. Client code that derives a domain
    fact (parses a reference, formats a year, scans text for a name or a
    citation, computes a run, decides a kind from a string) is an offender
    (24a) and its category is closed on the server (24b). Closure on the
    client: every generated type is read (`GeneratedUsageTests`), no client
    code holds domain parsing or arithmetic (a law over the client sources),
    and a client change that adds domain logic fails review. Less client code
    is the direction; a batch that grows it must say why.
26. **The backend is closed over the data.** Owner, 2026-09-30, the analogue
    of 25: the server's only knowledge of the domain is the compiled artifact,
    built from `data/raw` and `data/curated` by the ETL, and it composes over
    that. A domain fact written in server code (a book list, an event or place
    id, a name or alias, a date, a special case for one node, a pinned
    inventory of pairs) is an offender (24a); its category closes by moving the
    fact into data with its provenance and justification and having the code
    read it through the graph (24b). Closure: facts enter only through the ETL
    from files under `data/`; a curated id that does not resolve fails the
    compile; a law over the server sources catches domain literals. Less
    server special-casing is the direction.
26a. **Parsers are tools, not the system.** Owner, 2026-09-30: "Parsers and
    things used to get data in a certain shape are tools, but not part of the
    system that the users actually care about." The ETL (`atlas-etl`, the
    compile binary, `scripts/`) is the tool layer: it reads raw sources with
    their formats and quirks, once, into the artifact. Source-shape knowledge
    lives there and nowhere else. The served system (`atlas-graph`'s readers,
    `atlas-contract`, `atlas-server`, the client) reads the artifact and never
    parses a raw file or scans text. Closure: the tool boundary is a crate
    boundary; a served crate does not link the ETL.

## Growth and performance (owner, 2026-09-30)

27. **Derivation belongs to whoever knows its inputs.** Owner, 2026-09-30, on
    the explorable-edge work that put a frontier into the backend: "we agreed
    that frontier and explorable are front end constructs derived from the
    graph". A derivation over the data alone (an index, a count per edge kind,
    a label, a map's level of detail, a summary) is the compiler's, run once
    into the artifact (26a). A derivation over one request (this element, these
    neighbours, this window) is the server's, as an indexed read. A derivation
    over the user's interaction (Explorable, frontier, presentation,
    breadcrumb, Back) is the client's (25). The server derives nothing; it
    reads what the compiler built. The graph models the domain and never a
    view: no relation, field or element exists to serve a client construct.
    Closure: no name from the client's exploration vocabulary (explore,
    explorable, frontier, card, popover, presentation) appears in `server/` or
    `graph-types/`, and a gate fails the build when one does.

27a. **The wire serves the graph, not views.** A small closed set of generic
    reads: elements by id (node or edge alike, many per call); a position's
    neighbours by edge kind and direction, paged; the count per edge kind at a
    position; range reads by passage span, time window and map area. A new
    view or a new edge kind adds data and vocabulary, never an endpoint.

27b. **Every read is bounded and index-backed.** Its cost is one index lookup
    plus the size of what it returns, with the page size capped. Paging is
    keyset (resume after the last item), never offset. Closure: the store
    exposes index lookups only, so a scan cannot be written; an edge kind's
    indexes follow from its entry in `relations!`, never from a hand-written
    query.

27c. **A screen is a few round trips.** Reads take many ids at once, and a
    neighbour page carries each far end's id, kind and label, compiled into the
    artifact. A derivation on the client never costs a request per item.

27d. **Every answer is a pure function of (artifact root, request),** so it is
    cacheable on the root everywhere: HTTP, the client, a static export. The
    graph service holds no per-user state; saved explorations live elsewhere.

27e. **The client holds a bounded working set.** It never loads a whole
    collection; its element cache evicts; frontiers page lazily; long views
    render only what is visible. Client work scales with the page shown,
    never with the graph.

27f. **Budgets are gates at future size.** p95 latency per read, response
    size and client frame time are gated against a synthetic graph ten times
    the current artifact (words-as-base alone multiplies positions about
    25-fold). A blown budget fails the gate; the ceiling is never raised to
    pass.

27g. **Geometry is pre-tiled.** The map compiler ships each shape simplified
    per zoom level; the server serves tiles by area and time window; full
    resolution never crosses the wire.
