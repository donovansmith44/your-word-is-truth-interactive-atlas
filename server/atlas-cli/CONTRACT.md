# atlas-cli CONTRACT (binary: `bibex`)

RENAME (owner order 2026-08-29, verbatim: "rather than atlas-cli, it
should just be called bibex (bibleexplorer)"): the crate keeps the
workspace name atlas-cli; the BINARY and every user-typed command below
is `bibex`.

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
color, per R3 — plain output always stands alone with no ANSI). Batch
BIBEX-1 (owner order, 2026-08-29 — "we need json flag on cli" — the exact
future batch this document's own prior text named: "a future batch can add
it as a pure addition once real machine consumers exist") lands the
`--json` flag CLI-1 disclosed as out of scope: see "`--json` mode" below
for the full per-command shape table. Plain mode is unaffected wherever
this batch's own second ticket (ID discoverability, below) didn't touch
it — `chapter`/`node`/`edges`/`tutorial`'s plain bytes are unchanged by
BIBEX-1; `verse`/`find`/`help`/bare's plain bytes ARE deliberately changed
(ticket 2's own owner-ordered ID-discoverability shape), documented in
their own sections below.

Two global flags, accepted before OR after the subcommand, in any order
relative to each other:
- `--data-dir <path>` — where to look for `graph.bin` and the compiled
  JSON files. Defaults to `../data/compiled` (the same relative layout
  every other tool in this repo assumes when run from `server/`).
- `--json` (BIBEX-1) — machine-readable JSON on stdout instead of prose;
  see "`--json` mode" below.

### `bibex` (bare, no arguments)

Prints a short help block: the command list (one line each, name + a
one-clause description) and a pointer to `bibex tutorial`. Exit 0. BIBEX-1
ADDENDUM (ticket 2): the command list now also lists `kinds` and the
`--json` global flag — a deliberate, ticket-2-authorized change to this
command's own plain bytes (was: `verse`/`chapter`/`node`/`edges`/`find`/
`tutorial`/`help` plus one `--data-dir` global-flag line; now adds `kinds`
and the `--json` flag's own two-line description). `bibex --json` (or
`--json` with no subcommand in any position) is `bad_usage` — see
"`--json` mode" below.

### `bibex help`

Identical output to bare `atlas`. Exit 0. (`bibex help <cmd>` is NOT
implemented this batch — every command's own `--help`-shaped usage line
is instead shown automatically whenever that command is called with a bad
usage shape, per the error taxonomy's `bad_usage` class below.) `bibex
help --json`/`bibex --json help` is `bad_usage` — see "`--json` mode"
below.

### `bibex tutorial`

A guided, numbered walkthrough — see "Tutorial contract" below. Exit 0 on
completion (a tutorial that cannot complete because the graph itself
fails to load exits through the `data_load_failed` class instead, code 5,
same as every other command). `bibex tutorial --json` is `bad_usage` — a
tutorial is prose by nature, documented, not a silent no-op; see
"`--json` mode" below.

### `bibex kinds`

BIBEX-1 ADDENDUM (ticket 2, EDGE-KIND DISCOVERABILITY — owner order mid-
batch, 2026-08-29, verbatim: "add `bibex kinds`... so the vocabulary
itself is discoverable from nothing"). No arguments. Lists the full
edge-kind vocabulary `bibex edges --kind`/`bibex node <id>`'s own edge-
summary rows accept, straight off graph-types' own `relations!` manifest
(`RelationId::ALL`/`SymRelationId::ALL`) — the SAME total enumeration
`graph_wire::parse_edge_kind` itself scans, so this listing can never
drift out of sync with what a real `--kind` value is actually accepted.

Output: one row per accepted `--kind` TOKEN, in manifest declaration
order (each directed relation's forward row then its inverse row, then
every symmetric relation) — `TOKEN  RELATION  DIRECTION`. `RELATION` is
the manifest's own Rust identifier for that relation (`RelationId`'s/
`SymRelationId`'s own `{:?}` name, e.g. `Cites`, `Attests`, `LocatedAt`)
— the addendum's own "one-line descriptions from the relations! manifest
names" wording: the description IS the manifest's own declared name,
never new, hand-authored prose that could drift from what the manifest
says. `DIRECTION` is `forward`/`inverse`/`symmetric`. Never empty (the
manifest always has entries); no error class applies to this command.

### `bibex verse <ref>`

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

BIBEX-1 ADDENDUM (ticket 2, ruling 1, "IDS EVERYWHERE" — owner order mid-
batch, 2026-08-29: "see a thing → read its id → `bibex node <that id>`
works"): a DELIBERATE, ticket-2-authorized change to this command's own
plain bytes (was: `"Name, Name, ..."`; now: `"Name [wire-id], Name
[wire-id], ..."`). Each Places/Persons/Events/Passages entry is now
`"<label> [<id>]"` — `<id>` is the SAME `graph_wire::encode_node_id` form
`bibex node <id>`/`bibex edges <id>` decode (e.g. `Place:jericho`,
`Person:aaron_1`, `Event:ab_ur`), bracket-suffixed after the label, comma-
joined same as before, `(none)` unchanged when a section is empty. This
is the CLI's own established node-id vocabulary (the same one `bibex
edges`'s target-listing rows already used before this batch), not the
REST API's per-domain `PlaceRefOut`/`PersonRefOut` id space (those ids are
bare curated ids meant for a DIFFERENT endpoint, `/api/place/{id}`, that
this CLI has no equivalent of) — chosen so the loop the owner named always
closes: copy the bracketed id, paste it straight into `bibex node`.

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

### `bibex chapter <ref>`

`<ref>` is a KJV chapter ref (`GEN.1`), parsed via
`atlas_core::refs::ScriptureRef::parse` (rejects anything that isn't
exactly a `Chapter`-shaped ref — a bare book or a verse-shaped ref is
`bad_ref`). Concord is deliberately NOT accepted here: `graph_handlers.rs`
itself documents that a Concord article's own paragraph count varies too
widely for a server-derived chapter span to mean anything consistent —
this command inherits that same disclosed scope limit rather than
inventing its own answer to a question the server itself declined.

Output: every verse in the chapter, one line each, `REF  text` (red-letter
spans marked inline, same convention as `bibex verse`).

### `bibex node <id>`

`<id>` is any wire-form node id (`text-unit:GEN.1.1`, `Event:ab_ur`,
`Place:jericho`, ...), decoded via `graph_wire::decode_node_id` — the
same generic id grammar `/api/node/{id}` accepts, covering every node
kind this graph carries (TextUnit/Event/Narrative/Anchor/Place/Era/
Polity/CatechismItem/Person/Translation/CommentaryItem), not just the
kinds this batch's other commands happen to construct refs for.

Output: id / kind / label (via `graph_wire::describe_node`) / provenance,
then an edge-summary table (`kind -> count`, via `GraphQuery::edge_summary`)
listing only inhabited edge kinds — `(no edges)` when the summary is
empty, never a blank table. BIBEX-1 ADDENDUM (ticket 2, ruling 3, "must
show each kind's exact --kind TOKEN"): each edge-summary row's `kind`
column IS ALREADY `EdgeKind::label()` — the exact, copy-pasteable value
`graph_wire::parse_edge_kind` accepts back as `--kind` (its own total
inverse; unchanged plain bytes, this is a declaration of an existing
fact, not a code change) — see `bibex kinds` above for the full
vocabulary these tokens are drawn from.

### `bibex edges <id> [--kind K] [--limit N] [--cursor C]`

One frontier page at `<id>` — the SAME `(Position, EdgeQuery)` shape
`/api/node/{id}/edges` serves. `--kind` is REQUIRED (a node can carry
several distinct edge kinds; there is no honest "default" one to pick —
same reasoning the server's own `bad_kind` on a missing `kind` param
already encodes) and must be a label from `graph_wire::parse_edge_kind`
(`cites`, `cited-by`, `attests`, ...; `bibex node <id>` prints the exact
labels available for a given id). `--limit` defaults to 20, clamps to
[1, 200] (same bounds as the server). `--cursor` is the opaque integer
offset a previous page's own "more: continue with --cursor N" line
printed.

Output: one line per entry — `edge-id  kind  id  label` — then either
`(end of list)` or `more: continue with --cursor N`. BIBEX-1 addendum
(ticket 2, ruling 1): this row already carried the target's own wire id
(the `id` column) before this batch — unchanged plain bytes; the id was
already there, closing the loop already, so this command needed no
ID-discoverability change. An edge kind with
zero entries at this id (a valid id, a valid, elsewhere-inhabited kind,
but nothing at THIS position) is the `empty_result` error class below
(exit 1, stderr) — `edges`'s own entire output IS the page, so a zero-row
page is the command's whole answer coming back empty, the literal case
R5's "no empty stdout-and-exit-0 on a miss" names, not a sub-field of an
otherwise-nonempty success (contrast `bibex verse`'s Places/Persons/Events
sections below, which stay on stdout with `(none)` — those are one part
of a still-nonempty successful lookup, not the whole answer).

### `bibex find <term>`

Case-insensitive substring match on `<term>` against the label of every
node this crate can enumerate WITHOUT new parallel query logic — Places,
Events, Narratives, Eras, Polities (the five `..._ids` companion fields
`GraphService::assemble` already builds for the server's own listing
endpoints; `graph_wire::describe_node` computes each label), plus, as of
BIBEX-1 ADDENDUM ticket 2 (owner order mid-batch, 2026-08-29, "PERSONS
above all"):

- **Person** — `GraphService::person_ids`, a NEW companion field, added
  this batch, built the IDENTICAL way `era_ids`/`narrative_ids`/
  `event_ids`/`place_ids` already are (a one-time scan over the node
  table's own kind tag at `assemble` time) — this is the established,
  owner-approved shape for exactly this need, not a competing one; see
  `atlas-graph/src/service.rs`'s own field doc comment. This is the ONE
  code change BIBEX-1 makes outside `server/atlas-cli` (server-side,
  additive, zero data/graph-types/client changes — the crate's own
  established companion-enumeration pattern, extended by one more kind).
- **CatechismItem** — off `AtlasData.catechism` (`data.catechism[].items`,
  already loaded by `load::load` from the compiled `catechism.json`) — NO
  new plumbing at all; the id/name pair sat there unused until this batch.

DISCLOSED, STILL EXCLUDED (named here for a possible future batch, per
the addendum's own "name it in the report" instruction):
- **PeopleGroup** — enumerable in principle (the graph carries PeopleGroup
  nodes) but `graph_wire::decode_node_id` carries NO `PeopleGroup` arm
  (the PG-1a "U5-rebinding seam," disclosed since Batch PG-1a) — a found
  PeopleGroup id could never be resolved by `bibex node <id>` afterward,
  which would break ticket 2's own "see it → use it" loop guarantee for
  exactly the ids `find` would be handing back; excluded until U5 lands.
- **CommentaryItem** — 50,000+ KRETZ-1 units, no existing id/label
  enumeration surface (`GraphService` carries no `commentary_item_ids`
  companion) — building one at this scale for a name-substring command
  alone is disproportionate, a genuinely new batch's worth of design
  (paging? a stricter match? — this document takes no position).
- **Translation** — a fixed 6-row set, no existing enumeration surface
  either; low value against the cost of adding one.
- **TextUnit** — unchanged from CLI-1: covered directly by `bibex verse`/
  `bibex chapter` instead of a name search.

`bibex find --help`-shaped output (i.e. running `atlas find` with no
term) states the full searched/excluded scope explicitly, not silently.

Output: one line per match — `kind  id  label`, where `id` is the SAME
`graph_wire::encode_node_id` wire form `bibex node <id>` decodes (BIBEX-1
ADDENDUM ticket 2: a DELIBERATE plain-bytes change from CLI-1's own bare
curated id, e.g. `jericho` → `Place:jericho` — the bare form was never
actually pasteable into `bibex node <id>` as-is, a latent gap in the
"see it → use it" loop this addendum's own ruling 1 closes here too) —
sorted by kind then id (a stable, reproducible order) on a hit. Zero
matches is the `empty_result` error class below (exit 1, stderr, "no
matches for '\<term\>' -- searched Place/Event/Narrative/Era/Polity/
Person/CatechismItem labels (...)"), not a blank stdout line — `find`'s
entire output IS the match list, so no matches means the command's whole
answer came back empty.

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
interpretations. `bibex find <term>` is the one command whose answer can
have MULTIPLE rows for one input, but that is not ambiguity in the input
itself (the term is unambiguous; the graph honestly has several matching
labels) -- its own multi-row success output, not an error class, is the
correct treatment.

Exit 0 is reserved for a command that produced real, nonempty primary
output (help/tutorial's own "exit 0 on completion" is the one exception,
since neither one is answering a graph question that could come back
empty).

## `--json` mode (BIBEX-1, deliverable 0 — committed before implementation)

Owner order (2026-08-29, verbatim): "we need json flag on cli." CLI-1's
own "Command vocabulary" section had disclosed `--json` OUT OF SCOPE,
naming this exact future batch as the one to add it "as a pure addition
once real machine consumers exist" — this section fixes the shape BEFORE
any code changed, per this repo's standing contract-first discipline.

**Global flag.** `--json` is accepted before OR after the subcommand, in
any order relative to `--data-dir` (`main.rs::extract global flags`
scans the whole arg list for both, the same permissive treatment
`--data-dir` alone already had). Presence is checked BEFORE any other
argument parsing can fail, so even a malformed `--data-dir` under `--json`
renders its error as JSON, never plain text.

**One JSON value per invocation on stdout.** A single, compact
(single-line) JSON value — an object, or an array where the command is a
listing (`chapter`, `find`, `kinds`) — terminated by one trailing
newline for terminal friendliness. No ANSI, no prose mixed in, nothing
else on stdout. A failing invocation prints NOTHING to stdout (same
discipline as plain mode).

**Field vocabulary aligns with the wire.** Every JSON shape below reuses
field NAMES already established elsewhere in this app's own wire
vocabulary — no novel synonyms for an established name:
- `words_of_christ: [{start, end}]` — the identical shape/field name
  `atlas_server::handlers::WordsOfChristSpanOut`/the AQC corpus already
  use for a red-letter span.
- Node references reuse `atlas_server::graph_handlers::NodeCardOut`/
  `NodeRefOut`/`EdgeSummaryEntryOut`/`EdgePageOut`/`EdgeEntryOut`'s own
  field names (`id`, `kind`, `label`, `provenance`, `edge_summary`,
  `entries`, `next`) — this crate's OWN generic node-id vocabulary
  (`graph_wire::encode_node_id`'s wire-encoded form, e.g. `Place:jericho`,
  `Person:aaron_1`), the one `bibex node`/`bibex edges` already speak, NOT
  the REST API's per-domain endpoints' own bare-id shapes
  (`PlaceRefOut`/`PersonRefOut`'s `{id, name}`, meant for a DIFFERENT
  wire surface, `/api/place/{id}`, that this CLI has no equivalent of) —
  a deliberate choice, so a printed `id` always round-trips through THIS
  binary's own `bibex node <id>`.
- Refs use their canonical grammars verbatim (`ref: "GEN.1.1"`,
  `ref: "BoC 7.2.1"`), the same citation strings `graph_wire::
  encode_node_id`/`describe_node` already produce.
- `tracked: bool` (`bibex verse` only) is a NEW field with no established
  wire precedent (nothing else in this app needs to say "this corpus
  does/doesn't carry these sections" as one flag) — disclosed here rather
  than left for a caller to infer from field absence.

**Errors stay fail-loud, machine-readably.** A failure under `--json`
emits ONE JSON object on **stderr**:
```json
{"error": {"code": "<taxonomy class>", "message": "<what> -- <why>", "hint": "<what to do>"}}
```
`code`/exit status are the SAME five-class taxonomy below, unchanged —
`--json` never turns a real failure into a silent empty success or a
different exit code. `message` folds the taxonomy's own WHAT and WHY
together (the same two clauses plain mode's `atlas: error (<code>):
<what> -- <why> -- <what to do>` line already joins with " -- "); `hint`
is the WHAT-TO-DO clause verbatim. `CliError::to_json` is the ONE
rendering site (mirroring `Display`'s own single plain-mode rendering
site) — the taxonomy's fields are the single source of truth for both.

**`bibex tutorial`/bare/`bibex help` + `--json` = `bad_usage`.** Loud,
documented, not a silent no-op: a tutorial is prose by nature (the
CONTRACT's own binding ruling), and neither bare nor `help` answers a
graph question — each rejects `--json` with a `bad_usage` object whose
`hint` names the real query commands `--json` DOES work with
(`verse`/`chapter`/`node`/`edges`/`find`/`kinds`).

**Empty results are explicit in JSON too**, per command, never ambiguous
with a failure:
- `bibex verse`: `places`/`persons`/`events`/`passages` are `[]` (not
  omitted, not `null`) when a section is honestly empty — the KJV case's
  own four always-present array fields.
- `bibex node`: `edge_summary` is `[]` when a node genuinely has zero
  edges of ANY kind (a real card fact, not a `find`/`edges`-style "this
  question has no answer" miss — no error).
- `bibex edges`/`bibex find`: these commands' entire output already IS
  the answer (CONTRACT's own R5 reasoning, unchanged by `--json`) — a
  ZERO-row page/zero matches is still the `empty_result` taxonomy class
  on stderr, exit 1, never a silently-empty JSON array standing in for a
  real miss.

**Per-command JSON shapes:**

| command | shape | example |
|---|---|---|
| `bibex verse <ref>` (KJV) | `{ref, text, tracked: true, words_of_christ: [{start,end}], places: [{id,label}], persons: [{id,label}], events: [{id,label}], passages: [{id,label}]}` | `{"ref":"GEN.1.1","text":"In the beginning...","tracked":true,"words_of_christ":[],"places":[],"persons":[{"id":"Person:god_1324","label":"God"}],"events":[{"id":"Event:theo-1","label":"Creation of all things"}],"passages":[]}` |
| `bibex verse <ref>` (Concord) | `{ref, text, tracked: false}` | `{"ref":"BoC 7.2.1","text":"Thou shalt have no other gods...","tracked":false}` |
| `bibex chapter <ref>` | array of `{ref, text, words_of_christ: [{start,end}]}`, one per verse, chapter order | `[{"ref":"GEN.1.1","text":"...","words_of_christ":[]}, ...]` |
| `bibex node <id>` | `{id, kind, label, provenance, edge_summary: [{kind,count}]}` | `{"id":"Event:ab_ur","kind":"Event","label":"Terah's family leaves Ur","provenance":"curated","edge_summary":[{"kind":"located-at","count":1}]}` |
| `bibex edges <id> --kind K` | `{kind, entries: [{edge, node: {id,kind,label}}], next}` | `{"kind":"located-at","entries":[{"edge":"LocatedAt:...","node":{"id":"Place:ur-1","kind":"Place","label":"Ur 1"}}],"next":null}` |
| `bibex find <term>` | array of `{kind, id, label}` | `[{"kind":"Person","id":"Person:moses_2108","label":"Moses"}]` |
| `bibex kinds` | array of `{token, relation, direction}` | `[{"token":"cites","relation":"Cites","direction":"forward"}, ...]` |

## ID discoverability (BIBEX-1 addendum, ticket 2 — owner order mid-batch,
2026-08-29, verbatim: "also i need to be able to find node/edge ids by the
things (verses events etc) to which they are associated because i don't
just know edge and node ids off the top of my head.")

Three rulings, all binding:

1. **IDS EVERYWHERE.** Every place a command prints an associated thing's
   NAME, it prints the ID beside it, so the loop the owner named always
   closes: see a thing → read its id → `bibex node <that id>` works.
   - Plain mode: `"<label> [<wire-id>]"` — a bracket suffix, comma-joined
     lists unchanged otherwise (`bibex verse`'s Places/Persons/Events/
     Passages sections; `bibex find`'s own `id` column, which was
     already a separate column but is now the WIRE-encoded form instead
     of a bare curated id, see `bibex find`'s own section above).
   - `--json` mode: `{id, label}` objects, never bare label strings (the
     "--json mode" section's own per-command table has the exact shapes).
   - `bibex chapter`'s own plain output carries NO associated-entity name
     list at all (just `REF  text` per verse — deliberately leaner than
     `bibex verse`, CLI-1's own design) — nothing to change; a verse's
     own `REF` IS already the id a reader needs to run `bibex verse
     <REF>` next. `bibex edges`'s target-listing rows already carried an
     id column before this batch (CLI-1) — also nothing to change there.
   - `bibex node`'s own edge-summary rows are covered by ruling 3 below
     (the `--kind` TOKEN, not a node id — an edge-summary row names an
     edge KIND, not an associated entity with its own id).
2. **FIND COVERS EVERYTHING NAMED.** `bibex find` widened to Person
   (companion field, `GraphService::person_ids`, "PERSONS above all") and
   CatechismItem (already-loaded `AtlasData.catechism`, zero new
   plumbing) — see `bibex find <term>`'s own section above for the full
   searched/disclosed-excluded kind list and the reasoning per excluded
   kind (PeopleGroup/CommentaryItem/Translation/TextUnit).
3. **EDGE-KIND DISCOVERABILITY.** `bibex node <id>`'s edge-summary `kind`
   column is already the exact, copy-pasteable `--kind` token (declared
   explicitly in `bibex node`'s own section above); `bibex kinds` (new
   verb, its own section above) lists the FULL edge-kind vocabulary from
   nothing, no id/node needed first.

## Tutorial contract

`bibex tutorial` runs SEVEN numbered steps against the REAL loaded graph
(no mocked output, no canned transcript) — one step per command in the
vocabulary above (verse, chapter, node, edges, find), plus an opening
step explaining `--data-dir`/exit codes, plus a closing step listing every
command again with a one-line reminder of its shape. Each step prints:

1. `Step N of 7: <what this teaches>`
2. the REAL command line being run (e.g. `$ atlas verse GEN.1.1`)
3. that command's REAL output, produced by calling the exact same
   command-implementation function `bibex verse ...` itself would call
   (never a hand-copied transcript — a change to `verse`'s own output
   shape changes the tutorial's own output on the next run, automatically)
4. one short paragraph explaining what the step just demonstrated

Every real query the tutorial runs targets GEN.1.1/GEN.1/an id/edge kind
that is GENUINELY present in the committed `data/compiled/graph.bin` —
verified by this crate's own smoke test (`tutorial runs to completion,
nonempty numbered steps` per R6), never a placeholder that would 404 on a
real invocation. Bare `atlas` (no args) names `bibex tutorial` by
pointing at it directly: `run 'bibex tutorial' for a guided walkthrough`.

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

BIBEX-1 (`--json` + ID discoverability) adds, in the same file: per
real-query command, a `--json` happy-path test parsing the real stdout as
JSON and asserting real field values against the real graph (not just
"is valid JSON"); a `--json` error-path test (an unknown id) asserting
the `{"error":{...}}` object on stderr and the correct taxonomy exit
code; `bibex tutorial --json`/`bibex --json`/`bibex help --json` all
assert `bad_usage`; a plain-mode byte-unchanged regression guard for a
representative command BIBEX-1 does NOT touch (`bibex chapter GEN.1`,
proving `--json` is a pure addition to that command); a real two-
invocation "see it → use it" integration test per addendum ticket 2
(e.g. `bibex find moses` → parse a `Person:...` id out of its own
output → `bibex node <that id>` succeeds; and `bibex node <id>` → parse
a `--kind` token out of its own edge-summary → `bibex edges <id> --kind
<that token>` succeeds); a `bibex kinds` happy-path test (plain + json)
asserting the row count matches `RelationId::ALL`/`SymRelationId::ALL`'s
own total and that `parse_edge_kind` accepts every printed token; a
`bibex find`-widened-scope test proving a real Person and a real
CatechismItem are both now reachable. `atlas-graph/src/service.rs`'s own
`#[cfg(test)]` module gets one new unit test (`person_ids_enumerates_
every_person_node`, a small hand-built fixture — the real committed
graph has no dedicated "prove the scan fires" case, same "inject the one
condition the real graph can't isolate" treatment `chapter.rs`'s own C-1
fixture tests already use).
