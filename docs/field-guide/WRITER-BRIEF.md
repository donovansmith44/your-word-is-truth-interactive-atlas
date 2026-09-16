# Codebase Explorer — writer brief (shared by every writer agent)

You are writing the explanatory content for an interactive "Codebase Explorer"
page. The reader is the project's owner: a capable builder who is NEW TO RUST
(and to parts of this codebase). Every file gets THREE explanations of
increasing technical sophistication, plus structured metadata. You read the
real source files; you never invent behaviour the code does not have. When
unsure what a file does, read more of it — the source is the authority.

## The repo

Root: `C:/Users/donov/scratch/bible-atlas-sketch/.claude/worktrees/bible-atlas-m1`
(a git worktree; read-only for you — do NOT edit, build, or run cargo/dotnet;
other agents are writing code in this tree right now, so ignore any
uncommitted changes you notice).

What the project is (so your L1 prose has the right frame): "Bible Explorer",
an explorable Bible atlas. One typed-edge GRAPH holds everything — KJV verses,
cross-references, places, events, narratives, eras, polities (kingdoms with
borders), Luther's Small Catechism, the Book of Concord, Kretzmann's commentary
— as typed nodes and edges. A Rust server (`server/`, axum) compiles that graph
from open datasets (ETL) into one artifact (`data/compiled/graph.bin`) and
serves it over HTTP; a Blazor WebAssembly client (`client/`, C#) renders a KJV
reader and a `/world` map with a time slider; `graph-types/` is the shared,
zero-dependency Rust crate that DEFINES the graph's types and laws (the map
project depends on it too). The project's culture: "laws as tests",
content-addressed identity (ids are hashes), every UI affordance corresponds to
a real edge, no fabricated data, honesty over polish. Doc comments in this
codebase are unusually long and narrative (they cite batch names like
"Batch M-C", "PROV-1", "CDC-1" — those are past work batches; explain the
concept, do not lecture the reader about batch history).

## The three levels — what each one is FOR

- **L1 — "Plain English"** (60–120 words). For someone who has never
  programmed in Rust and may be rusty on programming generally. NO jargon
  without an immediate everyday analogy. Answer: what is this file FOR, what
  would break or be missing if it vanished, and how does it relate to
  something the owner can see in the app (a verse card, the map's borders,
  the time slider, the build step). Use concrete nouns from the domain
  (verses, places, borders, the artifact file).
- **L2 — "Programmer new to Rust"** (120–220 words). For a developer who
  knows another language. Explain HOW the file does its job: its main types
  and functions, the data flow in and out, and — crucially — each RUST
  CONCEPT the file uses, introduced the first time it matters ("`Option<T>`
  is Rust's way of saying 'maybe a value' — instead of `null`, the type
  itself forces you to handle the empty case"). Name the concepts using the
  exact vocabulary tags below so the glossary can link them.
- **L3 — "The details"** (150–320 words). For an experienced engineer.
  Exact type/function names with their roles, invariants and laws the file
  enforces (and WHICH test enforces them, if you can see it), performance or
  determinism concerns, tricky ownership/borrowing or generic choices and why,
  known gaps the code itself discloses in comments, and how it is wired to
  its neighbours (who calls it; what it calls). Cite `file:line` for anything
  a reader would want to jump to.

Write in second person to the owner where natural ("when you click a verse…").
Short sentences. No filler ("This file is responsible for…" → say what it does).
Never pad: a 30-line file gets a shorter L3 than a 2,000-line one. Never
exceed the word caps by more than 20%.

## Output — ONE JSON file per area, exactly this shape

Write to the path given in your dispatch. Valid JSON (UTF-8, no comments, no
trailing commas). Validate before finishing with
`python -c "import json,sys; json.load(open(sys.argv[1], encoding='utf-8')); print('valid')" <path>`
— if that prints anything else, fix the file.

```json
{
  "area": "server/atlas-graph",
  "title": "atlas-graph — building and serving the graph",
  "kind": "crate | client | web | tests | data | docs",
  "language": "rust | csharp | javascript | typescript | mixed",
  "overview": { "l1": "…", "l2": "…", "l3": "…" },
  "reading_order": ["server/atlas-graph/src/lib.rs", "…"],
  "groups": [
    { "name": "Adapters (one per source corpus)", "paths": ["server/atlas-graph/src/kjv_adapter.rs", "…"], "note": "one sentence on why these belong together" }
  ],
  "files": [
    {
      "path": "server/atlas-graph/src/service.rs",
      "lines": 1234,
      "role": "one line, ≤ 14 words, what this file IS",
      "l1": "…", "l2": "…", "l3": "…",
      "key_items": [
        { "name": "GraphService", "kind": "struct | enum | trait | fn | macro | const | type | component | class | module | test", "what": "≤ 25 words", "line": 49 }
      ],
      "depends_on": ["graph-types/src/store.rs", "server/atlas-graph/src/artifact.rs"],
      "used_by": ["server/atlas-server/src/load.rs"],
      "concepts": ["trait", "Arc", "BTreeMap", "Result"],
      "tests": "one or two sentences: how this file's behaviour is tested (which test file/function), or 'no dedicated tests found'",
      "gotchas": ["optional: ≤ 3 short bullets a newcomer would trip on"]
    }
  ]
}
```

Rules for the fields:
- `path` is repo-relative with forward slashes. `lines` = actual line count
  (use `wc -l`). Include EVERY file in your assigned set — the page's tree is
  built from these entries, so a missing file is a hole the owner will see.
- `overview` = the area as a whole (what the crate/folder is for, at three
  levels, same word caps as a file). `reading_order` = the order a newcomer
  should read the files to understand the area (lib.rs / entry point first,
  then the core types, then the periphery; tests last). Include every file
  once.
- `groups` cluster the files thematically (3–8 groups); a file may appear in
  one group only; every file must be in some group.
- `key_items`: 3–10 per file (fewer for tiny files). Prefer the things a
  reader would search for: the public types and the entry-point functions.
  `line` is where it is defined.
- `depends_on` / `used_by`: repo-relative paths of OTHER FILES in the repo
  (not external crates). Derive `depends_on` from `use crate::…` / `use
  atlas_…::…` / `mod` lines; derive `used_by` by grepping for the module or
  type name across the repo (best effort; 0–8 entries each; never guess).
- `concepts`: tags from the vocabulary below ONLY (exact spelling). Tag a
  concept when the file genuinely uses it in a way worth explaining — not
  every `Vec`. 3–12 tags per file. For non-Rust files use the C#/web tags.
- `tests`: name the covering test file(s) you actually found by grep, or say
  none found. Do not assert coverage you did not see.

## Concept vocabulary (exact tags)

Rust — language: `crate`, `module`, `struct`, `enum`, `trait`, `impl`,
`generics`, `trait-bound`, `lifetime`, `ownership`, `borrowing`, `reference`,
`slice`, `Option`, `Result`, `?-operator`, `match`, `pattern-matching`,
`if-let`, `closure`, `iterator`, `Vec`, `HashMap`, `BTreeMap`, `BTreeSet`,
`String-vs-str`, `Box`, `Arc`, `Rc`, `Mutex`, `RwLock`, `OnceLock`,
`derive`, `macro_rules`, `attribute`, `visibility`, `newtype`,
`phantom-type`, `type-alias`, `const-static`, `Default`, `Clone-Copy`,
`Debug-Display`, `From-Into`, `Ord-PartialOrd`, `Hash`, `error-handling`,
`anyhow`, `thiserror`, `panic-unwrap-expect`, `doc-comment`, `cfg`,
`feature-flag`, `unsafe`, `threads`, `scoped-threads`, `channels`,
`async-await`, `tokio`, `axum`, `serde`, `bincode`, `Command-process`,
`PathBuf`, `env-macro`, `integration-test`, `unit-test`, `property-test`,
`bench`, `workspace`, `Cargo-toml`, `build-script`, `sort-determinism`,
`content-hash`, `interning`.
C# / web (client and tests): `blazor-component`, `razor`, `dependency-injection`,
`wasm`, `js-interop`, `record`, `async-Task`, `LINQ`, `event-callback`,
`cascading-parameter`, `state-service`, `localStorage`, `HttpClient`,
`System.Text.Json`, `xunit`, `bUnit`, `Reqnroll`, `Playwright`, `fast-check`,
`Gherkin`, `Leaflet`, `SVG`, `WebGL`, `ES-module`, `CSS`.

## Tone calibration (one example, so all writers sound alike)

For `graph-types/src/store.rs` (the `GraphQuery` trait):
- L1: "This file is the promise the rest of the app makes about how it will
  ask questions of the graph — 'give me this node', 'how many edges of each
  kind leave here', 'one page of those edges', 'the next twenty verses'. Any
  storage that can keep that promise can sit underneath the app, which is
  what lets the project swap its storage later without touching the reader
  or the map."
- L2 opens: "`GraphQuery` is a **trait** — Rust's version of an interface: a
  list of method signatures that a type promises to implement…"
- L3 opens: "`pub trait GraphQuery` (store.rs:28) declares five methods:
  `node`, `derive`, `edge_summary`, `edges`, `reading_window`. `Graph`
  itself implements it (store.rs:52), which is the 'canonical instance' law:
  every backend is checked by `assert_answers_match` against the in-memory
  graph…"

## Working method

1. List your assigned files with line counts.
2. Read each file (for very large files, read the module doc comment, every
   `pub` item, and skim the rest — do not skip files).
3. Write entries as you go into the JSON file (build it incrementally with a
   script or write it once at the end — your choice — but VALIDATE at the end).
4. Do not spawn subagents. Do not modify any file in the repo. Report back
   only: the output path, the number of files covered, the total lines
   covered, and any files you could not read.
