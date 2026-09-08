# Atlas Graph Contract

The expectations the atlas publishes about **its graph**, and about each
transport's faithfulness in carrying it.

Run by the vendored Haskell Gherkin executor in `contracts/runner`
(map-generator's, taken by owner order — see `contracts/runner/VENDOR.md`).

```
scripts/contract-gate.sh          # everything, the way pre-push runs it
```

## Why it is organised this way

Owner ruling, 2026-09-08: **"the primary juncture is over our graph."**

So the suite is not six endpoint suites. `graph/` states what is true of
the graph — its declared vocabulary, a node's identity, an edge family's
page, the coordinate authority, one version root — and `transport/` states
only that each carrier conveys those faithfully. A projection is named for
the graph thing it projects (`node-card`, `edge-page`, `gazetteer`,
`vocabulary`), never for a route.

The test of whether that is real rather than decorative is
`transport/cli.feature`, which pins **nothing**. It asks whether bibex and
the HTTP wire project to the *same* value through the *same* graph
projection. There is no fixture to re-bless and no way to make it pass
except by making the two carriers actually agree — and a third transport
inherits the law by adding one `When`, never a new vocabulary.

## The division of labour with atlas-query-contract

Both suites are Gherkin. They are not duplicates, and neither is a weaker
copy of the other; they answer different questions and fail on different
things.

| | `atlas-query-contract` (AQC) | `atlas-graph-contract` (this) |
|---|---|---|
| Question | Is the **query language** honoured? | Did a **value a consumer reads** change? |
| Laws are | algebraic / structural — round-trip identity, the bijection witness travelling, a page never repeating an entry, schema-validity against `aqc.schema.json`, the error taxonomy (`bad_window`, `bad_ref`, `bad_kind`) | extensional — this exact projected value, pinned; this vocabulary, closed; these two transports, in agreement |
| Written in | query vocabulary ("I run SceneQuery for the time window …") | consumed-projection vocabulary ("the consumed projection `gazetteer` equals fixture …") |
| Harness | dual: Rust `cucumber` in-process + C# Reqnroll | the vendored Haskell runner, against a recorded pact |
| Would stay GREEN through | every polity ring moving, every place moving, 15 place ids being deleted | pagination starting to repeat entries, a malformed ref answering 200, a focus reference failing to round-trip |

That last row is the argument. Neither suite catches the other's failures,
so collapsing them would lose coverage, and running both is not
duplication.

**The boundary is enforced by omission, not by convention.** This suite has
no refusal step and no status-code capture at all — the error taxonomy is
AQC's, and there is deliberately no way to restate it here. Likewise the
six cartographic endpoints belong to `contracts/atlas-edge`, which is
map-generator's expectations of us, not ours.

## Where the evidence comes from

`contract-runner run --replay contracts/pacts` executes every expectation
against a **recorded pact** rather than a live server. The pact is not
trusted: it is regenerated from the real committed graph through the real
`axum::Router` (and the real bibex binary) by
`server/atlas-server/tests/contract_pact.rs` and
`server/atlas-cli/tests/contract_pact_cli.rs`, each of which fails if the
result differs from the committed copy by one byte.

Two halves, both required, neither advisory:

* the recorders catch **provider drift** — our answers changed;
* the runner catches **expectation drift** — what a consumer reads changed.

A pact compared only against a pact would prove nothing. A pact regenerated
from the live graph *in the same gate* proves as much as a live run, minus
the socket — which is what lets the gate run before every push without
standing up a server on ports this machine has already spoken for.

The recorders do not carry a list of what to record. They read the feature
files. Adding `When I GET /api/something-new` therefore fails the gate
until the recorder has been re-run, with a message naming the missing key:
an expectation can never quietly outrun the evidence behind it.

## Versioning

`VERSION` + `CHANGELOG.md`, with the bump class **derived from the diff**
by `scripts/contract-semver-gate.sh`. See the CHANGELOG's own table for
what PATCH / MINOR / MAJOR mean for a suite.
