# The contract runner is VENDORED, and this file says from where

**Upstream:** `donovansmith44/map-generator`, `contracts/runner/`, at commit
`6608db440111d96e9e2ebf63ba7a3bf5c67f2448` (branch `master`, "contracts
v0.2.0 opens: the coverage corpus is a breaking change, say so").
Read read-only from the owner's local clone at `C:\Users\donov\scratch\
map-generator`; nothing in that checkout was modified.

**Why it is here at all.** Owner order, batch CDC-1, verbatim: *"thou shalt
take inspiration for contract tests from them. thou shalt steal the haskell
gherkin parser that i built last night, and write our own set of contract
tests at every juncture from which consumption might occur."* It is the
owner's own code, and the order is to adopt it as our executor — not to
reimplement it and not to fork it for sport.

**Why VENDORED rather than consumed from the sibling checkout.** The
alternative was to invoke `../../../map-generator/contracts/runner` the way
map-generator path-deps our `graph-types`. That coupling is already known to
be fragile in exactly this repo pair: every one of map-generator's eight
crates hard-codes
`../../../../scratch/bible-atlas-sketch/.claude/worktrees/bible-atlas-m1/graph-types`,
so one `git worktree remove` breaks all eight builds (MAPS-1 §4.4, open
question 3 — still unruled). A **pre-push gate** built on that same
filesystem assumption would fail for a reason that has nothing to do with
any contract, and a gate that fails for unrelated reasons is a gate someone
turns off. The gate must be the most reliable thing in the repo, not the
least. So: vendored, with this file as the audit trail, and improvements
travel by patch in both directions.

---

## What was taken BYTE-IDENTICAL (do not edit these without upstreaming)

| File | Lines | Why untouched |
|---|---|---|
| `src/Gherkin/Ast.hs` | 21 | the AST |
| `src/Gherkin/Parse.hs` | 99 | the megaparsec Gherkin parser — the thing the order names |
| `src/Gherkin/Render.hs` | 30 | round-trips the AST back to text |
| `src/Pattern.hs` | 66 | the applicative step-pattern; `capUntil`'s terminator-is-the-next-literal rule is what makes ambiguity unrepresentable |
| `src/Run.hs` | 144 | scenario execution, the three-outcome verdict, `@target` classification, the report table |
| `src/Check.hs` | 179 | `check` — the totality law |
| `src/Vocab.hs` | 368 | `vocab` — the vocabulary-drift law |

That is **907 lines taken verbatim**, including their comments, which carry
the reasoning behind decisions we would otherwise have had to rediscover
(the CP437 em-dash corruption; why a claim has three states and not two;
why a law that never ran must not report green).

## What was ADAPTED, and exactly how

| File | What changed |
|---|---|
| `src/Capture.hs` | PRUNED to its domain-neutral core: `Universe`, `FromCapture`, `describeUniverse`, `didYouMean`, `editDistance`, `FixtureRef`. Dropped `Piece`, `PieceSet`, `StyleName`, `Year`, `Center`, `Zoom`, `DetailTier`, `ScaleQual` — map-generator's own query vocabulary, with **its server's tuned constants transcribed into them** (style names, zoom clamps, LOD tiers, the camera latitude clamp). Carrying those into the atlas would put a second system's constants where nothing here can falsify them: a fork that looks authoritative, which is worse than a fork. |
| `src/World.hs` | Upstream's `Claim`/`StepOutcome`/`Precondition`/`StepDef`/`mkStep`/`mkSkippableStep`/`checkStatus`/`readFeatureFile` kept verbatim, because `Run`, `Check` and `Vocab` bind to exactly that surface. Dropped the `lastRender` and `cameras` fields (render vocabulary we do not have). ADDED the recorded-pact transports, and changed what a transport is keyed by — see below. |

### The one substantive divergence: request keys

Upstream hands its transport a fully-built URL (`baseUrl w <> path`). This
copy hands it a **request key** — the literal text the feature file wrote
after `When I`:

```
GET /api/eras                 an HTTP juncture
bibex node Place:ur_1189      the CLI juncture
export gazetteer              a compiled-export juncture
graph vocabulary              the graph's own declared vocabulary
```

The transport decides what a key means. That single change is what lets the
**same feature file** run against a live server *and* against a recorded
pact, which is how the pre-push gate runs in seconds without standing up a
server on ports this machine has already spoken for. It is also what let
non-HTTP junctures (the CLI, the exports, the graph vocabulary) become
first-class contract surfaces instead of a second, weaker mechanism — the
addendum's "transports inherit from the graph suite" needs the transports to
be interchangeable at the runner's own boundary, and this is that boundary.

**This divergence is a candidate to upstream.** It costs map-generator
nothing (its `httpTransport` would gain a two-line `stripPrefix "GET "`) and
would give it the same fast offline mode.

## What we did NOT take

| Not taken | Lines | Why |
|---|---|---|
| `src/Steps.hs` | 1,868 | It is map-generator's **vocabulary** — render pieces, cameras, scene manifests, resource batching. Owner order 3 says we "write our own set of contract tests"; a vocabulary is an expectation, and we are the author of our own. Our `src/Steps.hs` is written from scratch against the atlas's graph. The one thing ported out of it deliberately is the **consumed-projection DSL** (`Proj`, `project`, `field1`, `fieldAlt`, `flattenField`, `firstDiff`), now `src/Proj.hs`, plus the six cartographic projection definitions — see below. |
| `src/Prop.hs` | 1,278 | QuickCheck generators over map-generator's query space. Replaced by a **deliberately minimal stand-in** with an empty hole registry, whose only job is to keep `Check` and `Vocab` byte-identical (both import it). Consequence, stated plainly: this copy of the runner supports **no `@property` generative scenarios**. Every atlas expectation today is example-based. If we ever want generative ones, the honest move is to take upstream's `Prop.hs` whole, with QuickCheck — not to grow generators in the stand-in. |
| `test/Spec.hs` | 4,690 | Upstream's own hspec suite. It imports `Steps`, so it cannot come without the vocabulary we deliberately did not take. The engine's own tests stay upstream, where the engine is maintained. **This is a real cost and it is not hidden:** our copy of the engine is exercised only by our suites and by `check`/`vocab`, not by upstream's 4,690-line spec. It is the strongest argument for keeping the vendored engine files byte-identical, so upstream's spec keeps covering the code we actually run. |

## The six projections that are not ours

`src/Proj.hs`'s `polities`, `narratives`, `event`, `eras`, `landmarks` and
`land-mask` entries are ported field-for-field from upstream's `Steps.hs`.
They are **map-generator's consumed projection of us** — its statement of
what its parsers read from our API. They live here so that its
`contracts/atlas-edge` suite runs unmodified inside our gate (see
`contracts/atlas-edge/RECEIVED.md`).

Changing one of them is not a refactor. It is editing another repo's
expectations of us, and the correct response to a disagreement is to go red,
report, and coordinate a version bump on both sides.

## Building it

```
cd contracts/runner
cabal build all
cabal list-bin contract-runner
```

Toolchain used: GHC 9.12.1 (scoop shim), cabal-install 3.18.1.0.
Dependencies: text, containers, bytestring, megaparsec, aeson, aeson-pretty,
http-client, http-types, directory, filepath, vector, optparse-applicative.
No QuickCheck, no hspec — see "what we did not take".
