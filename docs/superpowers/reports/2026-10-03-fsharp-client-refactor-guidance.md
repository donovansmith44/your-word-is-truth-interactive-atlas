# CX-FSHARP: lessons learned and the refactor to make

For Codex, from Claude (controller), 2026-10-03. Companion to `2026-10-03-fsharp-client-claude-review.md` (C1–C3, I1–I10).

The owner: "Our principles have not changed. Readability and clarity and capturing everything under the type system are still our priorities."

This document is direction, not a script. Use your judgment on the shape of each change. Where you see a better design that serves the same principle, take it and say why in the handoff. What is not negotiable is the principle each item serves and the closure: an offender must become impossible to write, not merely absent today (PRINCIPLES 24b).

## Part 1: the lessons the C# client paid for

Each lesson below cost a fix wave on the C# side. The F# client starts clean, so it should not pay for any of them again.

1. **One value, one meaning, one type (F-74, F-76).**
   - A nullable cursor once meant both "first page" and "no previous page", and paging broke at the second page. The fix was a `Cursor` type whose only first value is `Cursor.First`.
   - `catechism-link` meant both "this paragraph is the item's words" and "this verse supports the item". The reader could not tell them apart, so the owner saw an extra hop.
   - **Rule:** whenever one representation carries two meanings, split it into two types or two cases. A bare `string`, `int`, `bool` or `Option` that stands for a domain idea is a defect.

2. **The contract is the only source of vocabulary and defaults (F-18).**
   - The C# client restated the page size and the cursor default. Both drifted from what the server published.
   - Now `PagedReads.FirstPage` is generated from the `default:` value in `openapi.yaml`, and the client's own page size is a single named affordance constant.
   - **Rule:** every kind, relation, error code, default and enum comes from generation. Nothing is spelled out by hand, and nothing is read back from a JSON wire spelling (I6).

3. **The client composes; it does not compute (rule 25, F-63, F-65).**
   - The C# client used to parse reference strings (`CanonRef`), filter ids by prefix (`"BoC "`), read whole collections and join `/api/sources` on the client. Every one of these is now either deleted or held on a ratchet that only shrinks.
   - The server serves labels, years and spans as data.
   - **Rule:** the F# client formats nothing it can be served, joins nothing, and scans nothing. Where something is missing from the wire, record a finding and let the server serve it. Do not compute it in the client (I2).

4. **Rule 27: frontier and explorable are client constructs.**
   - The server serves the graph through bounded generic reads, and the client derives the views.
   - That split is correct, and it is why a view's shape, its group order (the owner's ruled order lives in one affordance table) and its paging window belong to the client.
   - **Rule:** view logic belongs in pure, typed F# modules, not in Bolero view functions (I3).

5. **Paging is a state machine, not a list (F-67 to F-70).**
   - The C# client had several paging bugs. An old completion overwrote newer progress. The revealed prefix grew without bound. Root changes leaked across pages. Labels went stale when the root changed.
   - The fix was one serialized, bounded window per group: 20 per page, More appends up to 40 and then slides, and Less goes back 20. Every page and cache key carries its artifact root, and an exploration renews when the root changes.
   - **Rule:** paging state must be a closed union. A completion carries the request it answers, so a stale one is a type-level no-op. Resident entries stay bounded whatever the graph's size (I5).

6. **Required means required at the boundary (F-81, F-75).**
   - The C# client accepted records with missing required fields as null.
   - Nothing proved that each served field is read.
   - **Rule:** decoding refuses an incomplete record with a typed failure. A law states which wire fields the client reads, and an unread field is either deliberate (and listed) or deleted (I1).

7. **Failures are values with a meaning (I8).**
   - The served `ErrorCode` is a closed vocabulary.
   - **Rule:** a contract fault, a network fault, a not-found and a stale root are different cases, and each renders differently. "Check your connection" is the network case only.

8. **Do not hide bad data (I4).**
   - **Rule:** a served offset outside its text is a contract violation, and it is reported as one. Clamping it makes a wrong answer look right.

9. **Laws, not examples; red before green, with evidence that lasts (I9, I10).**
   - Every C# law in this program was seen red first, and the ledger records where.
   - **Rule:** state-machine properties are stated over all inputs, with FsCheck or exhaustive enumeration of a closed union: every `Msg` in every relevant `Model` case. Red evidence goes into the committed ledger or report, not into `/tmp`.

10. **A bug is a category (PRINCIPLES 24, 24a, 24b).**
    - **Rule:** for each item below, name the failed abstraction, choose the side, migrate every site, and close the category with a type or an enumerating law. When you find a sibling offender while working, record it as a finding. Do not fix it on the side.

11. **The look is the owner's, and every class must be styled (F-77).**
    - The owner rejected an unstyled FocusView on sight. The C# client now has a law that every class a view emits is defined in `app.css`.
    - Fonts are Overpass for headings and Atkinson Hyperlegible for everything else. They are reached only through `--font-heading` and `--font-body`.
    - Verse lists show each verse's words, read in one batched element read per page.
    - Groups follow the ruled order: text, attests, catechism-link, cites, then the rest.
    - **Rule:** parity includes all of this. The shared assets are byte-gated already; keep it that way.

## Part 2: the refactor, by finding

### C1. Identities become types

**Direction:**
- The generator emits each named contract id as a single-case union or struct wrapper with a private constructor: `NodeId`, `EdgeId`, `EraId` and the rest, plus `Root` (the artifact version), `Cursor` and the reference types.
- Decoding is the only door in, and it validates. Encoding is the only door out.
- Nothing outside the generated module can make one from a string.

**Done when:**
- A source law fails on any `string`- or `int`-typed parameter, field or message payload that carries an id, root or cursor.
- The generator has a test that every named id in `openapi.yaml` gets its own type.

### C2. The model makes illegal states unrepresentable

**Direction:**
- Model the app as what the owner can actually be doing: a closed union of surfaces, each carrying only its own state, rather than a record with one field per surface.
- Retry belongs to the failed operation, not to the app.
- Keep `update` readable: one match on `(msg, model)` per surface module, with the top-level `update` only routing.

**Done when:**
- The contradictory combinations the review lists no longer compile.
- An enumeration law runs every `Msg` against every `Model` case and asserts the result is one of the declared transitions.

### C3. Parity measures behaviour

**Direction:**
- A parity item is a scenario: a route, an interaction and an expected served-data outcome. It runs against both clients in the same browser harness, against the same server and artifact root, and compares the rendered, test-id-addressed outcome.
- File hashes stay as the asset gate. They are not parity.
- Remove from the "done" count every item whose module the F# client never loads.

**Done when:**
- The ledger counts only scenarios that pass on both clients.
- At least one deliberately broken F# behaviour makes a parity scenario fail. That is the parity harness's own red.

### I1. Closure laws for rule 25

**Direction:** source laws over the F# client that fail on:
- string parsing of references or ids;
- arithmetic over served numbers;
- filtering by id prefix;
- reading a whole collection outside the paging door.

Add a field-usage law over the generated types (F-75).

**Done when:** each law has been seen red against a scratch offender, and the result is recorded.

### I2 and I3. The view renders; typed modules decide

**Direction:**
- A pure `Presentation` module turns a served record into a typed presentation; the C# `Presentation.Of` is the reference.
- Bolero views only render a presentation.
- Contents joins, chapter and verse formatting, and dropped sources move out of the view: to the server, where the data is missing (raise a finding), or into the presentation module, where they are client constructs.

**Done when:** view functions take presentations, not records.

### I4. Invalid anchors are failures

**Direction:** `AnchoredText.runs` returns a result. An out-of-range offset becomes a typed contract failure, and the view shows it as such.

**Done when:** a law shows that every invalid offset is reported, and that no valid offset is altered.

### I5. The trail is bounded

**Direction:**
- Renewal re-reads only what the current view shows, not the whole trail.
- URL state holds the minimal descriptor: the current element plus a bounded history.

**Done when:** a long-interaction law proves that requests per step and URL length are independent of journey length.

### I6 and I7. Typed reads, no wire spellings

**Direction:**
- Each generated read takes one typed request record whose fields are the contract's types, including the `corpus` path parameter. Only valid parameter combinations are representable, so `bad_scope` cannot be requested.
- Remove the JSON round trips used to recover a wire spelling. The generator emits the spelling function.

**Done when:** no read call site passes positional primitives.

### I8. Typed failures

**Direction:** `Failure` becomes a closed union. It carries the served `ErrorCode` where there is one, and keeps transport and decode failures as their own cases.

**Done when:** the view's match over the union has no wildcard case.

### I9 and I10. Laws and evidence

**Direction:**
- Convert the single-example "laws" to properties.
- Cover every `update` transition.
- Record red runs in a committed ledger, as the C# batches do under `.superpowers/sdd/<batch>/progress.md`, or in your report. Never in `/tmp`.

### Size (feasibility)

**Direction:**
- Measure before optimising, and put the numbers in the handoff.
- First check trimming, invariant globalization, the AOT trade-off (startup against download), and whether the generated contract module or a dependency dominates.
- Do not let size drive a change that costs readability or types. If size remains a parity blocker, raise it as an owner question with the numbers.

## Order

1. C1, then C2. Every later change builds on the types and the model.
2. I8, I4, I6 and I7: the boundary.
3. I2, I3 and I5: presentation and trail.
4. C3 and I1: parity and closure laws, before any further feature slices.
5. I9 and I10, alongside each of the above.

Hold new feature slices until steps 1 to 4 land.

Keep following the landed C# FOCUS changes through the pinned parity ledger. Trunk is now past `ace063d`: A-NOBLURB has landed, and A-LICENSE-BOC, A-F39 and A-PROVENANCE are coming. Treat each as a parity input.
