# Atlas Graph Contract — CHANGELOG

**A suite IS a version.** Not a file that happens to carry a number: the
set of promises in `graph/` and `transport/`, together with the fixtures
that give them values, is the versioned unit, and this file is where each
version says what it changed and why the bump is the size it is.

## What the bump classes mean here

Semver for a contract suite is stated from the point of view of a
**consumer of the promises** — another system that declares "I am written
against atlas-graph-contract 1.4" and binds to what that version
guarantees. It is *not* stated from the point of view of the provider
being tested; the provider newly failing is the normal, intended result of
a MINOR bump.

| Class | Means | Examples in this suite |
|---|---|---|
| **PATCH** | Expectations clarified. The set of providers that conform is **unchanged**. | Preamble prose; comments; a regenerated `Vocabulary:` table; a fixture reformatted with an identical parsed value. |
| **MINOR** | Expectations **added**. New guarantees; a consumer written against an earlier version still gets everything it relied on. | A new scenario; a new feature file; a projection gaining a field; a new transport suite. |
| **MAJOR** | An existing expectation **changed or removed**. A consumer that relied on a promise may no longer have it. | Deleting a scenario or a feature file; removing a field from a projection; re-blessing a fixture to a **different value**; adding `@target` to a scenario that was green (a guarantee withdrawn). |

The bump is **derived from the diff, not asserted by the author**.
`scripts/contract-semver-gate.sh` computes the class the diff requires and
fails when the declared bump is smaller. Declaring a bigger bump than the
diff needs is always allowed — over-declaring is a judgement call, and
under-declaring is a lie the gate refuses to co-sign.

`@target` deserves its own line, because it is the one place a suite could
quietly lose teeth: a `@target` scenario runs and is reported **red in
every run**, but does not fail the gate. That is disclosure, not an escape
hatch — and adding one to a previously-green scenario is classified
**MAJOR**, because withdrawing a guarantee is exactly what it is.

---

- **0.3.0** (Batch DB-3) — **MINOR: the vocabulary widened.** `graph-
  vocabulary.json` gains one node kind (`LexiconEntry`) and one directed
  relation (`Occurs`: `occurs-in` / `words`), both uninhabited until LEX-1
  (spec §7). Every previously pinned value is unchanged and
  `artifact_format_version` stays 13 — the new DTO variants are appended,
  so every pre-DB-3 `graph.bin` decodes byte-identically. DB-4's own move
  (root semantics and id width, spec §9) therefore lands as 0.4.0.

- **0.2.0** (Batch CDC-1, fix round 1) — **MINOR: one guarantee added.** The
  graph's declared vocabulary now also pins `artifact_format_version`, read
  from the same private constant `atlas_graph::artifact::load` enforces
  (13 today). That constant is the sharpest break this repo can make — an
  artifact at any other version is refused outright, so every holder of an
  older `graph.bin` is turned away — and until now no expectation named it,
  so a 13 → 14 bump passed all five gate legs. Review finding M-5.

  Classified MINOR by the gate itself, not by assertion: the fixture is a
  strict widening (every previously pinned value unchanged, one key added),
  which is the grade added this round so that "a projection gaining a
  field" stops classifying as MAJOR and contradicting the table below.

  Also this round, and deliberately NOT bumps to this suite because they
  change the runner rather than any promise: an empty projection is now
  refused at bless and compare time (review H-1 — this suite twice shipped a
  green scenario pinning `[]`), and `@target` is forbidden outright in any
  received suite (review C-2).

- **0.1.0** (Batch CDC-1) — Initial suite. The graph-primary expectations
  (`graph/`: the declared `kind_tags!`/`relations!` vocabulary; node
  identity; edge-family pages with the bijection witness; the gazetteer
  pinned row by row as the C3 coordinate authority; C6's one-root law
  across the three published exports) and the transport expectations
  (`transport/`: the HTTP surfaces the Blazor client consumes that no
  other suite owns; bibex asserted to agree with the wire through the
  graph's own projections). Fifteen scenarios, one of them `@target`.

  Zero behaviour change to any endpoint, any handler, or any artifact:
  this version records what the server already serves and what consumers
  already read. The one red it declares is disclosed rather than silenced
  — `bibex --json node` carries no atlas version root while
  `GET /api/node/{id}` does, so a CLI consumer cannot implement the C6
  stale-pin. Fixing that is a one-field change in `atlas-cli`, which this
  batch does not own; when it lands, the `@target` tag comes off and that
  is a **MINOR** bump (a guarantee gained, not changed).
