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
3a. **Mutation runs once per batch, at its close, concurrently.** A task
   proves itself with its tests and the pacts; the mutation run is a
   batch-level gate over every line the batch changed, like the timing
   gates, and it is sharded across cores or worktrees so it finishes in
   minutes, not hours. Surviving mutants get one fix dispatch of tests;
   equivalents go in the tool's configuration with their reasons.
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
9. **Comments in application code are `why` comments only.** A comment
   earns its place by stating a hidden constraint, an invariant the type
   system cannot express, or a workaround for a specific external bug. It
   never restates the code, names a ticket, or narrates history. Generated
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
