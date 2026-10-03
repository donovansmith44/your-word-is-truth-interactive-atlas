# FOCUS-2 close: TextUnit on FocusView

Range `b3d7cfa..lane/claude/F2`. Plan: `docs/superpowers/plans/2026-10-02-focus2-textunit.md`. Owner answers OPEN 1–9 bound (1a, 2 verse only, 3 events only, 4b, 5 remove marker, 6a, 7a, 8a, 9a), plus the owner's 2026-10-02 ruling that verse lists show each verse's words.

## What is now true

- A verse or a Concord paragraph opens on FocusView as `Presentation.Text` over the served `UnitText`; every anchor is a link of its served kind, followed through `Explore`. `VerseNode`, `ConcordUnitNode` and the seven verse/paragraph sections are deleted; a Bible verse offers a read-in-context hatch (`popover-chip-context`).
- The compiler writes each text unit's reference; every served site reads it. Anchors read their characters from the compiled token tables. A text unit's record carries its text (`NodeRecord.text`), and every text row names its node (`TextUnit.node`, `TextUnit.body: UnitText`).
- A citation of a span is labelled by the passage it cites (`JHN.3.16 · Cites · JHN.11.25-26`).
- `/api/verse`, `VerseDetail`, `VerseEvent` and the published `BookMeta` schema are gone. A verse is read as an element and its neighbours.
- A list whose neighbours are text units shows each one's served words, read in one batched element read per page (owner ruling).

## Rule-24 categories

| Category | Closure | Guarantee |
|---|---|---|
| Verse and paragraph served by both mechanisms | deletion law (TextUnit) with `LegacyNames` | compiler + `DeletionLawTests` |
| A text row the client cannot open | `TextUnit.node` | `every_text_row_names_its_node` |
| References composed per request (F-42, FOCUS-2's sites, incl. `contents.rs`) | compiled `reference` | `no_served_crate_composes_a_label_or_a_reference` (bans `dot_ref`, `tokenize(`) + store law |
| Anchors re-tokenized per request | compiled token offsets | real-data anchor law |
| The client parses a served reference (F-65) | verse sites deleted; citation label read from its row | `ReferenceParsingLawTests` ratchet, counted per file |
| A whole collection read on the client (F-63) | verse sites deleted | `WholeReadLawTests` ratchet |
| No live popover offers an edge step (F-45) | FocusView entry edges | `explore-edges` Playwright |
| A verse list shows references, not verses (owner ruling) | words read per page by served node kind in `ServedPages` | `FocusViewTests`, `GraphExplorerTests`, `PersonMentionsListTests`, VERSE-WORDS-1 |

## Amendments

- §5: OPEN 1a moves `/api/xrefs/{sref}` and `/api/catechism/{sref}` (and `PassageNode`'s three sections) to FOCUS-3's row.
- §3.3: the `Text` form presents the served `UnitText`.

## Removed by the owner's rulings

Verse PARALLELS (OPEN 2), the PASSAGE-membership group on a verse (OPEN 3), the verse book chip (OPEN 4b keeps only read-in-context), the xref marker's reorder and cap (OPEN 5), the verse mini-reader, per-section provenance "?" on a verse. Playwright tests that only asserted these were deleted or rewritten; the ledger lists each.

## FINDINGS raised

- `TextUnit.ref` restates `TextUnit.node.id`'s local part (D.R.Y.).
- Request decoding of a text-unit id parses book codes against the canon in code (F-1 residue).
- A cross reference's span is not in the graph's model (OPEN 6b, FOCUS-3).
- The range routes parse references per request (retire with `PassageNode`, FOCUS-3).
- `drain_edges` reads whole collections in served code (`events.rs`, `persons_at_verse`).
- Words-of-Christ spans keyed by a dot-ref string built at load.
- Provenance on FocusView is a raw source id: no "?", no resolved source name or confidence note (UX loss).
- `GeneratedUsageTests` is type-level: an unread member of a read type cannot turn it red (no law for "a wire member no client reader reaches").
- F-39 bit twice: the label table sits outside the logical hash, so a label-only change needs a schema bump to move the root.
- The owner's ruled verse-section order (text, attests, catechism-link, cites) is not FocusView's order (served `edge_summary` order); `FRONTIER-ORDER-1` is red pending a ruling.
- The legacy chapter popover's title is `GEN.1` while its served label is `Genesis 1` (FOCUS-3).

## Gates (base `b3d7cfa`)

| Gate | Result |
|---|---|
| server `cargo test --workspace` | 1508/1508 |
| graph-types `--all-features` | 148/148 |
| client.Tests | 734/734 |
| client.ContractTests | 55/55 |
| `contract-gate.sh --base b3d7cfa` | PASSED (AQC 0.25.0, AGC 0.24.0) |
| `timing-gates.sh run` | 11/11 |
| Playwright (full) | 446 passed, 2 skipped, 4 failed: `world-quiet-places` density smoke and `world-cluster-chooser` C3-M1 (carried); `popover-sections` FRONTIER-ORDER-1 (for a ruling, above); `state-window` ST-2 divider width under load (green 4/4 on rerun) |

Mutation is owed: it runs only in the owner's window; `.superpowers/MUTATION-GATE-DEBT.md` lists FOCUS-2 under the next run.
