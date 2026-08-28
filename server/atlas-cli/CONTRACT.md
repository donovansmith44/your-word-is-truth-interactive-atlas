# atlas-cli CONTRACT

Batch CLI-1. This document is DELIVERABLE 0 (committed before any command
is implemented) — it fixes the command vocabulary, the error taxonomy, and
the tutorial contract that every test in this crate asserts against.

Owner order (2026-08-23, verbatim): "add to the back of the queue a nice
CLI with a really simple interface to query the graph. should have
tutorial, meaningful error messages, no silent failing of anything."
Four requirements, all binding here: (1) REALLY SIMPLE, (2) TUTORIAL,
(3) MEANINGFUL errors, (4) NO SILENT FAILING.

## Reuse boundary (design ruling R1)

`atlas-cli` is a new workspace binary crate. It loads
`<data-dir>/graph.bin` directly via `atlas_graph::GraphService::from_artifact`
— the SAME artifact-load path `atlas-server/src/main.rs`'s default branch
uses — and reconstructs `AtlasData` the SAME way (`atlas_core::data::AtlasData::load`
+ `atlas_graph::legacy::atlas_data_overlay` + `.finish()`). No HTTP, no
axum, no server process. Every actual graph QUESTION this crate answers
goes through `atlas_graph_types::store::GraphQuery`'s trait methods (via
`GraphService::snapshot()`) or `GraphService`'s own public adapter-side
companions (`position_of`, `chapter_span`, `era_ids`/`polity_ids`/
`narrative_ids`/`event_ids`/`place_ids`, `red_letter_spans`,
`persons_by_verse`), and `AtlasData`'s own public accessors
(`places_for_verse`, `place_by_id`, `events_for_verse`, `event_by_id`) —
the identical fields/methods `atlas-server`'s handlers already read. Ref
grammar (both `GEN.1.1` and `"BoC 1.2.3"`) is decoded via
`atlas_server::graph_wire::{decode_node_id, encode_node_id, parse_edge_kind,
describe_node, describe_position}` (this crate depends on `atlas-server` as
a library, reusing its wire-identity layer verbatim) plus
`atlas_core::refs::ScriptureRef::parse` for chapter refs — never a new,
hand-rolled parser. Zero parallel query logic; zero changes to
`atlas-server`, `atlas-graph`, `atlas-core`, or `graph-types`.

## Command vocabulary

Every command's PRIMARY output is human-readable plain text (TTY-optional
color, per R3 — plain output always stands alone with no ANSI). No
`--json` flag this batch: declared OUT OF SCOPE, disclosed here rather
than silently absent — widening every command's output surface with a
second, machine-shaped form was judged not "cheap" against the "keep it
SMALL" mandate for a first CLI batch; a future batch can add it as a pure
addition once real machine consumers exist.

Global flag, accepted before the subcommand on every invocation:
`--data-dir <path>` — where to look for `graph.bin` and the compiled JSON
files. Defaults to `../data/compiled` (the same relative layout every
other tool in this repo assumes when run from `server/`).

### `atlas` (bare, no arguments)

Prints a short help block: the command list (one line each, name + a
one-clause description) and a pointer to `atlas tutorial`. Exit 0.

### `atlas help`

Identical output to bare `atlas`. Exit 0. (`atlas help <cmd>` is NOT
implemented this batch — every command's own `--help`-shaped usage line
is instead shown automatically whenever that command is called with a bad
usage shape, per the error taxonomy's `bad_usage` class below.)

### `atlas tutorial`

A guided, numbered walkthrough — see "Tutorial contract" below. Exit 0 on
completion (a tutorial that cannot complete because the graph itself
fails to load exits through the `data_load_failed` class instead, code 5,
same as every other command).

### `atlas verse <ref>`

`<ref>` is a text-unit locus: a KJV dot-ref (`GEN.1.1`, `JHN.3.16`) or a
Book of Concord citation (`"BoC 7.2.1"`, quoted because of the space).
Decoded via `graph_wire::decode_node_id("text-unit:" + ref)` — the exact
locus grammar `/api/text`/`/api/node` already accept on the wire, reused
verbatim rather than re-parsed by hand.

Output: the rendered text, red-letter marks shown inline as `[...]`
around any KJV sub-verse span the graph's own `red_letter_spans` table
covers (Concord refs never carry red-letter spans — that table is
KJV-specific by construction, same as the server's own `words_of_christ`
field), then FOUR labeled sections — Places / Persons / Events / Passages
— each either a short list or the literal word `(none)` (never blank
space, never omitted — the empty-result discipline applies inside a
successful command's own sections too, not just at the top level).
Places/Persons/Events/Passages are populated ONLY for KJV verses
(`AtlasData`'s own `places_for_verse`/`events_for_verse`/the graph's
`persons_by_verse` are all Bible-specific tables); a Concord ref's card
states this plainly ("Places/Persons/Events/Passages: not tracked for the
Book of Concord") rather than showing four empty sections that would
misleadingly imply a real, checked absence.

Batch PERI-1 (PRESENTATION CATEGORY LAW — owner order, verbatim: "NUN is
not an event. fix this error and others like it"): `Events:`/`Passages:`
is a SPLIT of what used to be one merged `Events:` line — `events_for_verse`
now filters on `Event::kind` (already on every `Event` row, no new field):
`Events:` lists only `kind == "event"` labels (a real, dated/placed
passage), `Passages:` only `kind == "general"` ones (a dateless
pericope/literary-structure passage — a Psalm acrostic stanza, an epistle
outline pericope). Each list is independently `(none)` when its own kind
has zero entries for this verse — the SAME empty-result discipline as
before, now applied per-kind. See tests/ux/CONTRACT.md's own PRESENTATION
CATEGORY LAW section for the full cross-surface law this row is one
disclosed instance of.

### `atlas chapter <ref>`

`<ref>` is a KJV chapter ref (`GEN.1`), parsed via
`atlas_core::refs::ScriptureRef::parse` (rejects anything that isn't
exactly a `Chapter`-shaped ref — a bare book or a verse-shaped ref is
`bad_ref`). Concord is deliberately NOT accepted here: `graph_handlers.rs`
itself documents that a Concord article's own paragraph count varies too
widely for a server-derived chapter span to mean anything consistent —
this command inherits that same disclosed scope limit rather than
inventing its own answer to a question the server itself declined.

Output: every verse in the chapter, one line each, `REF  text` (red-letter
spans marked inline, same convention as `atlas verse`).

### `atlas node <id>`

`<id>` is any wire-form node id (`text-unit:GEN.1.1`, `Event:ab_ur`,
`Place:jericho`, ...), decoded via `graph_wire::decode_node_id` — the
same generic id grammar `/api/node/{id}` accepts, covering every node
kind this graph carries (TextUnit/Event/Narrative/Anchor/Place/Era/
Polity/CatechismItem/Person/Translation/CommentaryItem), not just the
kinds this batch's other commands happen to construct refs for.

Output: id / kind / label (via `graph_wire::describe_node`) / provenance,
then an edge-summary table (`kind -> count`, via `GraphQuery::edge_summary`)
listing only inhabited edge kinds — `(no edges)` when the summary is
empty, never a blank table.

### `atlas edges <id> [--kind K] [--limit N] [--cursor C]`

One frontier page at `<id>` — the SAME `(Position, EdgeQuery)` shape
`/api/node/{id}/edges` serves. `--kind` is REQUIRED (a node can carry
several distinct edge kinds; there is no honest "default" one to pick —
same reasoning the server's own `bad_kind` on a missing `kind` param
already encodes) and must be a label from `graph_wire::parse_edge_kind`
(`cites`, `cited-by`, `attests`, ...; `atlas node <id>` prints the exact
labels available for a given id). `--limit` defaults to 20, clamps to
[1, 200] (same bounds as the server). `--cursor` is the opaque integer
offset a previous page's own "more: continue with --cursor N" line
printed.

Output: one line per entry — `edge-id  kind  id  label` — then either
`(end of list)` or `more: continue with --cursor N`. An edge kind with
zero entries at this id (a valid id, a valid, elsewhere-inhabited kind,
but nothing at THIS position) is the `empty_result` error class below
(exit 1, stderr) — `edges`'s own entire output IS the page, so a zero-row
page is the command's whole answer coming back empty, the literal case
R5's "no empty stdout-and-exit-0 on a miss" names, not a sub-field of an
otherwise-nonempty success (contrast `atlas verse`'s Places/Persons/Events
sections below, which stay on stdout with `(none)` — those are one part
of a still-nonempty successful lookup, not the whole answer).

### `atlas find <term>`

Case-insensitive substring match on `<term>` against the label of every
node this crate's `GraphService` already enumerates by kind — Places,
Events, Narratives, Eras, Polities (the five `..._ids` companion fields
`GraphService::assemble` already builds for the server's own listing
endpoints; `graph_wire::describe_node` computes each label). DISCLOSED
scope limit: Person/CatechismItem/CommentaryItem/Translation/TextUnit
nodes are NOT searched — `GraphService` carries no companion enumeration
for those kinds (by the same design choice `store.rs`'s own doc comment
explains: `GraphQuery` deliberately has no "list every node" operation),
and adding one for the sole purpose of this command would be new,
un-server-shared enumeration logic — exactly what "zero parallel query
logic" forbids. `atlas find --help`-shaped output (i.e. running `atlas
find` with no term) states this scope explicitly, not silently.

Output: one line per match — `kind  id  label` — sorted by kind then id
(a stable, reproducible order) on a hit. Zero matches is the
`empty_result` error class below (exit 1, stderr, "no matches for
'\<term\>' -- searched Place/Event/Narrative/Era/Polity labels"), not a
blank stdout line — `find`'s entire output IS the match list, so no
matches means the command's whole answer came back empty.

## Error taxonomy

Every failure path below prints to **stderr**, in the fixed shape:

```
atlas: error (<code>): <WHAT failed> -- <WHY> -- <WHAT TO DO>
```

`<code>` is one of the five class names; the process's exit status is
that class's own fixed nonzero code (stable across runs — scripts can
branch on it). Successful output always goes to stdout; stderr carries
nothing on a clean run.

| class | exit | when | example |
|---|---|---|---|
| `bad_usage` | 4 | the command line itself is unparseable — unknown subcommand, unknown flag, missing a required positional/flag value, or extra positional arguments the command doesn't take | `atlas: error (bad_usage): unrecognized subcommand 'vers' -- 'atlas' only knows verse, chapter, node, edges, find, tutorial, help -- run 'atlas help' for the full list` |
| `bad_ref` | 2 | a ref/id argument does not parse against its own grammar (locus grammar for `verse`/`chapter`, wire-id grammar for `node`/`edges`, an unrecognized `--kind` label for `edges`) | `atlas: error (bad_ref): 'GEN.1.99.3' is not a valid verse/Concord reference -- expected BOOK.CHAPTER.VERSE (e.g. GEN.1.1) or "BoC PART.ARTICLE.PARAGRAPH" -- check the book code and the dot-separated parts` |
| `not_found` | 3 | the ref/id parses cleanly but names nothing this graph has — a real book+chapter+verse number combination that exceeds the chapter's own length, a well-formed id of a real kind that isn't in the graph | `atlas: error (not_found): no node named 'Event:not-a-real-event' -- the id parsed fine but this graph has no node with that raw id -- try 'atlas find <term>' to locate the id you meant` |
| `data_load_failed` | 5 | `graph.bin` (or a required compiled JSON file) is missing, unreadable, or fails to parse at startup, before any command's own logic runs | `atlas: error (data_load_failed): could not load ../data/compiled/graph.bin -- reading ../data/compiled/graph.bin: The system cannot find the path specified. (os error 3) -- run 'cargo run -p atlas-graph --bin atlas-graph-compile' from server/ first, or pass --data-dir to point at a directory that already has graph.bin` |
| `empty_result` | 1 | the command ran correctly end-to-end but the honest answer is zero rows (`find` with no matches; `edges` for an inhabited-elsewhere-but-empty-here kind) | `atlas: error (empty_result): no matches for 'zzqx' -- searched Place/Event/Narrative/Era/Polity labels -- try a shorter or different substring` |

"Empty result" is intentionally its OWN class, distinct from `not_found`:
`not_found` means the id/ref you asked about does not exist at all;
`empty_result` means it exists but this particular question about it has
zero answers right now (a real, checked absence, not a lookup failure).
Both still exit nonzero — R5's "no empty stdout-and-exit-0 on a miss"
binds equally to both classes.

FIX ROUND 1 (review S-2): the owner's brief names "ambiguous input" as
one of five illustrative failure-class examples (batch-cli1-brief.md's
own DELIVERABLE 0 wording); this taxonomy has no dedicated `ambiguous`
class, disclosed here rather than left silently absent. No ambiguous-input
case exists anywhere on this command surface: every ref/id/kind lookup
this crate performs is an EXACT match against a total, unambiguous
grammar (`ScriptureRef::parse`, `graph_wire::decode_node_id`,
`graph_wire::parse_edge_kind` each either parse to exactly one value or
fail outright -- none of them has a "could mean either of two things"
partial-match mode) -- an input that doesn't match cleanly is always
`bad_usage` (the command line itself) or `bad_ref` (the ref/id/kind
argument), never a genuine ambiguity between two candidate
interpretations. `atlas find <term>` is the one command whose answer can
have MULTIPLE rows for one input, but that is not ambiguity in the input
itself (the term is unambiguous; the graph honestly has several matching
labels) -- its own multi-row success output, not an error class, is the
correct treatment.

Exit 0 is reserved for a command that produced real, nonempty primary
output (help/tutorial's own "exit 0 on completion" is the one exception,
since neither one is answering a graph question that could come back
empty).

## Tutorial contract

`atlas tutorial` runs SEVEN numbered steps against the REAL loaded graph
(no mocked output, no canned transcript) — one step per command in the
vocabulary above (verse, chapter, node, edges, find), plus an opening
step explaining `--data-dir`/exit codes, plus a closing step listing every
command again with a one-line reminder of its shape. Each step prints:

1. `Step N of 7: <what this teaches>`
2. the REAL command line being run (e.g. `$ atlas verse GEN.1.1`)
3. that command's REAL output, produced by calling the exact same
   command-implementation function `atlas verse ...` itself would call
   (never a hand-copied transcript — a change to `verse`'s own output
   shape changes the tutorial's own output on the next run, automatically)
4. one short paragraph explaining what the step just demonstrated

Every real query the tutorial runs targets GEN.1.1/GEN.1/an id/edge kind
that is GENUINELY present in the committed `data/compiled/graph.bin` —
verified by this crate's own smoke test (`tutorial runs to completion,
nonempty numbered steps` per R6), never a placeholder that would 404 on a
real invocation. Bare `atlas` (no args) names `atlas tutorial` by
pointing at it directly: `run 'atlas tutorial' for a guided walkthrough`.

Register: direction, not mood — the tutorial explains what each command
does and why, in a plain instructional voice, never first-person
commentary or asides. Any prose that quotes or paraphrases the text
itself (a verse's own words) is bound by the inerrancy law like every
other content-describing surface in this app: quoted verbatim, never
edited, paraphrased, or commented on as if uncertain.

## Tests (R6)

`server/atlas-cli/tests/cli.rs` — integration tests invoking the REAL
compiled binary via `std::process::Command`, against the REAL committed
`data/compiled/graph.bin` (no fixture graph, no mock): every subcommand's
happy path, every error-taxonomy class (one test per class minimum, two
for `bad_ref` covering both the locus-grammar and the wire-id-grammar
shapes), the tutorial smoke test (runs to completion, exit 0, output
contains all 7 "Step N of 7" markers, no empty step), bare-invocation
help, and exit-code assertions for every case above. These join the
workspace's own standing canonical count (`server/Cargo.toml`'s counting
procedure).
