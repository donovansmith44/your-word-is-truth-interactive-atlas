# FOCUS-6 close — Geography, explorable edges, and rule 27

**Batch:** FOCUS-6 (plan `docs/superpowers/plans/2026-10-01-focus6-geography.md`, re-planned `f0f3470` under PRINCIPLES 27).
**Base:** `3baaeb6` (FOCUS-1 head, not yet landed). **Head:** `lane/claude/F6-int`.
**Versions:** AQC 0.17.0, AGC 0.20.1, graph-types 0.10.0, section schema 21, root `fa3efe21`.

## What changed in direction

The first build of explorable edges (`lane/claude/F6-t1`, `F6-t2`) put the frontier into the backend: an edge-card endpoint, `EdgeSource`/`EdgeTarget` relations, labels composed per request. The owner ruled (2026-09-30): "frontier and explorable are front end constructs derived from the graph". That work never lands. PRINCIPLES 27–27g were written from the ruling, and the batch was re-planned against them.

## Tasks

| Task | Branch range | What is now true |
|---|---|---|
| 1 (F-33) | `3baaeb6..fece593` | No client exploration word in `server/`, `graph-types/` or the published contract; a gate fails the build on one. `NodeCard` → `NodeRecord` (JSON unchanged); no id or root moved. |
| 2 | `fece593..efa5edc` | Node and edge labels are compiled into the artifact; `GET /api/elements?ids=` reads nodes or edges, many per call, capped; `EdgeRef{id,kind,label}`; no relation added (22). |
| 3 | `efa5edc..7a48d95` | `ElementKind`, `Link.Target: PositionRef`, `Follow` total; an edge's frontier is derived on the client; Back retraces; resume is one element read. |
| 4 | `7a48d95..ee10581` | A polity's reign and a place's default name and blurb are compiled; records serve map/era/polity windows; map rows name their node. |
| 4b | `ee10581..3184efd` | Every compiled label is the reader-facing name (313 suffixed place labels fixed; anchors use their curated label). |
| 5 | `ee10581..f3bec16` | `Presentation.Geography`: `Frame`, `Emphasis`, `Crossing`. |
| 6 | `3baaeb6..4e7a77e` | `IMapSource`: the world reads its layers through the MAPS seam. |
| 7 | `7a48d95..9b96a49` | `PopoverOpening { Explore | Resume | Legacy }`: one way to open the popover. |
| 8 | `7a48d95..63b6b68` | Slider `Bounds`/`Band`/`OnCross`; map `Emphasize`, `OnPolityClick`. |
| 9 | `7a21c7c..a52fe51` | The World view presents the current element: Map/Era bound the slider, a Place is ringed, a Polity outlined with its reign; crossing follows a served link. |
| 10 | `5c88240..49bca69` | The legacy place path and `/api/place` are deleted; text and map open one place view. |
| 11a | `47cd82a..96ea59c` | Every world and popover spec re-expressed; 464 test ids kept. |
| 11b | `47cd82a..adf85e3` | Gate closures (below); `/api/place` doc residue; sanctioned pact removal. |

## Rule 24 categories, their closures and guarantees

- **Client vocabulary in the backend (F-33).** Closure: `backend_vocabulary.rs` scans identifiers, comments, paths in `server/`, `graph-types/` and `contracts/` (not the received `atlas-edge`). Guarantee: the build fails on a client word.
- **Labels composed per request.** Closure: the compiled `label` table, `labels::node_label` as the one door, `no_served_label_composition.rs`. Guarantee: served code cannot reach a label composer.
- **A label that is not the reader-facing name (R4b).** Closure: laws over all 1,009,159 served labels (no disambiguation suffix, no id shape; a place's label is its display name).
- **Partial matches over closed sums.** Closure: `ElementKindLawTests`; `PopoverOpening.Match`; `Emphasis` replaces two nullable strings.
- **An end link that does not retrace.** Closure: the Exploration retrace law over every edge kind.
- **Opening the popover with a bare legacy node.** Closure: `OpeningCallbackLawTests`.
- **A test named in a script that no longer exists (F-41).** Closure: `scripts/named-test.sh require` refuses a zero-match filter; every script naming a test goes through it.
- **A pact hand-edited to remove a juncture.** Closure: `ATLAS_REMOVE_JUNCTURES` must name exactly the lost junctures or the recorder refuses.

## Removed behaviour (OPEN 2/3 defaults; the owner may ask for any back, rebuilt on the popover)

Hover cards everywhere (quiet dots, labels, grace, transit, delay); the pinned place card and its edge-flip placement; the card's verse list and passage block; windowed verses and their reveal steps; the period-true name in the title; the per-window blurb; the date popovers with supporting verses and "Show this time on the map"; the place's dates and events sections; the card's narrative prev/next; the tray shows the record label. A place is now its window-free record plus its `site-of` neighbours, opened by click.

## Gates (head `029abab`)

| Gate | Result |
|---|---|
| `cargo test --workspace` | 1,476 passed, 1 failed (`scene_byte_identity`, declared: O-GOLDEN-R4), 11 ignored |
| `graph-types --all-features` | 147 passed |
| `client.Tests` | 621 passed |
| `client.ContractTests` | 55 passed |
| `scripts/contract-gate.sh` | PASSED |
| `scripts/timing-gates.sh run` | 11/11 (element read at the cap: 46 ms for 200 ids, gate 100 ms) |
| Playwright `tests/ux` (8900/5900) | 459 passed, 2 failed (carried: `world-quiet-places` density smoke, `world-cluster-chooser` C3-M1), 3 skipped |
| Mutation | NOT RUN: owner's window, after Nov 4 (`.superpowers/MUTATION-GATE-DEBT.md`) |

Declared red awaiting the owner: `scene_byte_identity` (O-GOLDEN-R4).

## Owner decisions this close needs

- **O-GOLDEN-R4:** re-record the map scenes (Task 4's `node` refs, R4b's shorter labels).
- **O-CHOOSER:** C3-M1 waits on the old place card and can't pass as written.
- **O-F6R 6–13, O-VERSE-LABEL:** defaults built.

## Findings raised

F-33 (closed), F-35…F-56 in `.superpowers/QUEUE.md`. Notable: F-39 labels outside the version hash; F-40 artifact size at 10×; F-50 developer notes in reader-facing anchor citations; F-55 two flaky specs that predate FOCUS-6; F-56 Escape after an internal step.
