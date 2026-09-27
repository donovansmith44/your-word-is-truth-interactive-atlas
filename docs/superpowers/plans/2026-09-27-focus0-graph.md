# FOCUS-0 (graph side) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the graph what uniform navigation needs: a root container for each corpus, `follows-in` between Book of Concord documents and articles, an `authored-by` relation from books to their authors, place date claims tied to the events they attest, and consistently numbered Small Catechism titles — all as graph rows, laws and pins, with `/api/contents` output unchanged.

**Architecture:** Five graph-side additions in `atlas-graph`/`atlas-etl` and curated data; one new relation appended last in `relations!` with its row family following the D5 precedent (`ParentOf`); `CanonSuccession` extended to the Concord section with a section-schema bump; every pin that names a count or a version root updated; pact re-blessed once.

**Tech Stack:** Rust 1.97.1 workspace under `server/`; `graph-types` (zero non-optional deps); SQLite sections; the AGC/AQC gates; `scripts/timing-gates.sh`.

**Spec:** `docs/superpowers/specs/2026-09-26-focus-exploration-design.md` §7 and §10 R2/R5/R7. **Prerequisite:** CONTRACT-1a complete (handlers in `atlas-contract`; `openapi.yaml` exported; `DECLARED_*` constants exist). All commands run from `server/`.

## Global Constraints

- `docs/PRINCIPLES.md` binds: TDD, whole-body assertions, named constants, newspaper order, `why` comments only.
- **`relations!` is positional.** `AuthoredBy` is appended after `Participates`, never inserted. Named constants change: `DECLARED_DIRECTED_RELATIONS` 20 → 21, `DECLARED_EDGE_KINDS` 46 → 48.
- **`/api/contents/{corpus}` output is byte-identical** before and after this plan (the AQC `contents` scenarios and fixtures are the proof).
- Vendored raw data under `data/raw/**` is never edited; corrections live in `data/curated/**` and the ETL.
- The standing block (three commands in `server/Cargo.toml`) plus `bash ../scripts/timing-gates.sh` and `bash ../scripts/contract-gate.sh` run at the end; every gate re-runs because the graph changes.
- Commit per task; push at the end (owner authorization on record).

---

### Task 1: `AuthoredBy` and `Shows` — two relations, appended last, in that order

**Files:**
- Modify: `graph-types/src/edge.rs` (`relations!` invocation :72-125), `graph-types/tests/canon_vectors.rs:794-798`, `graph-types/tests/wire_names.rs`, `graph-types/tests/wire_form.rs`, `atlas-contract/tests/contract_generation.rs`
- Regenerate: `contracts/openapi.yaml`, `contracts/atlas-query-contract/aqc.schema.json`, `contracts/atlas-graph-contract/fixtures/graph-vocabulary.json`

- [ ] **Step 1: Turn the pins red**

`graph-types/tests/canon_vectors.rs:794-798`: `RelationId::ALL.last() == Some(&RelationId::Shows)`, `RelationId::ALL.len() == 22`. `graph-types/tests/wire_names.rs` and `wire_form.rs`: `const DECLARED_DIRECTED_RELATIONS: usize = 22;` (and `DECLARED_EDGE_KINDS: usize = 50;` where it is literal). `atlas-contract/tests/contract_generation.rs`: append to the `directed` literal, after `Participates`:
```rust
            { "name": "AuthoredBy",   "forward": "authored-by",     "inverse": "authored" },
            { "name": "Shows",        "forward": "shows",           "inverse": "shown-on" },
```
Run: `cargo test -p atlas-graph-types --features serde,openapi && cargo test -p atlas-contract --test contract_generation` → the pins fail (last relation is `Participates`; counts are 20/46).

- [ ] **Step 2: Append the relations**

`graph-types/src/edge.rs`, inside `relations! { directed { … } }`, after the `Participates` line:
```rust
        Participates => "participates-in" / "participants",
        AuthoredBy   => "authored-by" / "authored",
        Shows        => "shows" / "shown-on"
```
(the trailing comma placement follows the macro's `$(,)?`). `edge_index.rel` is positional: `AuthoredBy` is code 20 and `Shows` code 21; nothing moves.

- [ ] **Step 3: Regenerate and verify**

Run: `cargo run -p atlas-contract --bin export_contract && cargo test -p atlas-graph-types --features serde,openapi && cargo test -p atlas-contract --test contract_generation --test contract_pact 2>&1 | grep -E "test result|FAILED"`
Expected: pins green; the three documents regenerate (46 → 50 `EdgeKind` enum values, two more `x-atlas-relations` rows); `contract_pact`'s vocabulary law is self-adjusting (`RelationId::ALL.len()`), and its pact fixture compares the regenerated vocabulary.

- [ ] **Step 4: Commit**

```bash
git add ../graph-types ../contracts atlas-contract
git commit -m "graph-types: AuthoredBy and Shows appended last among the directed relations (FOCUS-0, R2/R10)"
```

---

### Task 2: The `Authored` row family (Core section), by the D5 precedent

**Files:**
- Modify: `graph-types/src/canon/rows.rs` (`RowFamily::Authored`, name `"authored"`, lowering to `Directed(RelationId::AuthoredBy)`), `graph-types/src/edge.rs` (the row struct), `graph-types/src/graph.rs` (`pub authored: Vec<Authored>`), `graph-types/src/sections.rs` (Core list), `graph-types/src/frontier.rs`, `graph-types/tests/canon_row_vectors.rs:132` (25 → 26), `atlas-graph/src/sqlite/{ddl.rs, partition.rs, reload.rs, rows/core.rs, rows/mod.rs}`, `atlas-graph/src/provenance.rs` (`sweep!`), `atlas-graph/tests/{canon_real_data.rs, sqlite_laws.rs, sections_real_data.rs}`
- The exact set of files and the shape of each edit is the D5 commit that added `ParentOf` (`git show 7d64ccb --stat`, then each file's hunk): copy its pattern with `Authored` in place of `ParentOf`.

- [ ] **Step 1: Write the failing law test**

Append to `atlas-graph/tests/sqlite_laws.rs`, beside the `parent-of` round-trip case:
```rust
#[test]
fn an_authored_row_round_trips_through_the_core_section() {
    // Arrange
    let row = Authored {
        book: ContainerNodeId::from("bible-book-GEN"),
        person: PersonId::from("moses_2108"),
        provenance: ProvenanceId::from("books"),
        justification: Justification::default(),
    };
    // Act
    let back = round_trip_core(|tx| insert_authored(tx, 0, &row), read_authored);
    // Assert
    assert_eq!(back, vec![row]);
}
```
(`round_trip_core` is the helper the existing `parent_of` law test uses; name it exactly as that test does.)

- [ ] **Step 2: Run to verify it fails** — `cargo test -p atlas-graph --test sqlite_laws an_authored_row` → compile error (`Authored`, `insert_authored` not found).

- [ ] **Step 3: Implement the family**

`graph-types/src/edge.rs` (beside `ParentOf`):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authored {
    pub book: ContainerNodeId,
    pub person: PersonId,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}
```
`canon/rows.rs`: `RowFamily::Authored` appended **last** (row families are also positional in the sections' DDL order), name `"authored"`, `Canon` impl over `(book, person, provenance, justification)`, lowering `Directed(RelationId::AuthoredBy)` with subject `Position::Node(book)` and object `Position::Node(person)`. `graph.rs`: `pub authored: Vec<Authored>`. `sections.rs`: add `RowFamily::Authored` to the **Core** list; the `canon_row_vectors.rs` count 25 → 26. `sqlite/ddl.rs`: `DDL_AUTHORED` (`id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, book_id TEXT NOT NULL, person_id TEXT NOT NULL, provenance TEXT NOT NULL, justification_id INTEGER`) + unique index on `ord`; `rows/core.rs`: `insert_authored(tx, ord, &row)`, `read_authored(conn)`; `rows/mod.rs`, `partition.rs`, `reload.rs`: the match arms the other families have; `provenance.rs`: `sweep!("authored", authored)`; `frontier.rs`: the family's edges enter `edge_index` like every directed family.

- [ ] **Step 4: Run**

`cargo test -p atlas-graph-types && cargo test -p atlas-graph --test sqlite_laws --test canon_real_data --test sections_real_data 2>&1 | grep -E "test result|FAILED"` → green (the family exists with zero rows; `canon_real_data`'s per-family counts gain an `authored: 0` line where the others are listed).

- [ ] **Step 5: Commit**

```bash
git add ../graph-types atlas-graph
git commit -m "graph: Authored row family (book -authored-by-> person) in the Core section, lowering into AuthoredBy"
```

---

### Task 3: Authorship rows from curated `author_ids`

**Files:**
- Modify: `data/curated/books.toml` (add `author_ids = [...]` to 14 books), `atlas-core/src/data.rs` (`BookMeta.author_ids: Vec<String>`), `atlas-etl/src/curated.rs:100` (`parse_books`), `atlas-graph/src/bible_container_adapter.rs` (emit rows)
- Test: `atlas-graph/tests/canon_real_data.rs` (count), a unit test in `bible_container_adapter.rs`

- [ ] **Step 1: Write the failing test**

In `bible_container_adapter.rs`'s test module:
```rust
#[test]
fn a_book_with_curated_author_ids_is_authored_by_each_of_them_and_nothing_else_is() {
    // Arrange
    let books = vec![
        book_meta("GEN", "Moses", &["moses_2108"]),
        book_meta("PSA", "David and others", &[]),
    ];
    // Act
    let rows = authored_rows(&books);
    // Assert
    assert_eq!(rows, vec![Authored {
        book: ContainerNodeId::from("bible-book-GEN"),
        person: PersonId::from("moses_2108"),
        provenance: ProvenanceId::from("books"),
        justification: Justification::default(),
    }]);
}
```
(`book_meta` is a three-line test helper constructing `BookMeta` with the given code, author text and ids.)

- [ ] **Step 2: Run to verify it fails** — compile error (`authored_rows`, `author_ids`).

- [ ] **Step 3: Implement**

`data.rs`: `pub author_ids: Vec<String>` on `BookMeta` (serde `#[serde(default)]`); `curated.rs::parse_books` reads `author_ids` (default empty). `books.toml`: add `author_ids` to exactly these fourteen books, using the `Person` ids the grounding resolved (Moses `moses_2108`, Paul `paul_2479`, Luke `luke_1836`, Matthew `matthew_1971`, Isaiah `isaiah_617`, Ezekiel `ezekiel_1237`, Solomon `solomon_2762`; Habakkuk, Haggai, Hosea, Jonah, Jude, Malachi, Nahum by their single-match ids — read them from `core.sqlite` `node` where kind = Person and label equals the name; the id is the bare id after `Person:`). `bible_container_adapter.rs`:
```rust
pub fn authored_rows(books: &[BookMeta]) -> Vec<Authored> {
    books.iter()
        .flat_map(|b| b.author_ids.iter().map(move |p| Authored {
            book: ContainerNodeId::from(book_container_id(&b.book)),
            person: PersonId::from(p.as_str()),
            provenance: ProvenanceId::from("books"),
            justification: Justification::default(),
        }))
        .collect()
}
```
called from `normalize` (`ctx.graph.authored.extend(authored_rows(&ctx.books))`). A book whose `author_ids` name a person id that does not exist is a **build error** (law: `every_authored_person_exists`, in `law_check.rs`, alongside the kinship-acyclic law D5 added).

- [ ] **Step 4: Run and count**

`cargo test -p atlas-graph 2>&1 | grep -E "test result|FAILED"`, then `cargo run -p atlas-server -- --data-dir ../data` and `curl -s localhost:8000/api/node/Container:bible-book-GEN | jq .edge_summary` → includes `{"kind":"authored-by","count":1}`; `curl -s "localhost:8000/api/node/Container:bible-book-GEN/edges?kind=authored-by"` → Moses.
(Stop the harness afterwards — R-D-7.)

- [ ] **Step 5: Commit**

```bash
git add ../data/curated/books.toml atlas-core atlas-etl atlas-graph
git commit -m "graph: books are authored-by their curated authors (14 books resolved; the rest await curation)"
```

---

### Task 4: Corpus roots for both corpora; Contents discovers roots by parentage

**Files:**
- Modify: `atlas-graph/src/bible_container_adapter.rs` (:140-202), `atlas-graph/src/concord_adapter.rs` (:133-197), `atlas-contract/src/contents.rs:139-146` (and the Bible root path ~:90-135)
- Test: `atlas-contract/tests/graph_api.rs` (two whole-card tests), `atlas-contract/tests/aqc_cucumber.rs` (existing `contents` scenarios stay green)

- [ ] **Step 1: Write the failing tests**

Append to `graph_api.rs`:
```rust
const BIBLE_ROOT: &str = "Container:bible";
const CONCORD_ROOT: &str = "Container:concord";
const BOOKS_IN_THE_BIBLE: usize = 66;
const DOCUMENTS_IN_THE_CONCORD: usize = 10;

#[test]
fn each_corpus_has_one_root_that_contains_its_top_level_containers() {
    // Arrange
    let app = compiled_app();
    // Act
    let bible = get_json(&app, &format!("/api/node/{BIBLE_ROOT}"));
    let concord = get_json(&app, &format!("/api/node/{CONCORD_ROOT}"));
    // Assert
    let (bible_version, concord_version) = (bible["version"].clone(), concord["version"].clone());
    assert_eq!(bible, serde_json::json!({
        "id": BIBLE_ROOT, "kind": "Container", "label": "The Holy Bible", "provenance": "kjv",
        "edge_summary": [ { "kind": "contains", "count": BOOKS_IN_THE_BIBLE } ], "version": bible_version,
    }));
    assert_eq!(concord, serde_json::json!({
        "id": CONCORD_ROOT, "kind": "Container", "label": "The Book of Concord", "provenance": "concord",
        "edge_summary": [ { "kind": "contains", "count": DOCUMENTS_IN_THE_CONCORD } ], "version": concord_version,
    }));
}

#[test]
fn genesis_is_a_member_of_the_bible_root() {
    // Arrange
    let app = compiled_app();
    // Act
    let page = get_json(&app, "/api/node/Container:bible-book-GEN/edges?kind=member-of");
    // Assert
    let version = page["version"].clone();
    assert_eq!(page, serde_json::json!({
        "kind": "member-of",
        "entries": [ { "edge": page["entries"][0]["edge"].clone(), "node": { "id": BIBLE_ROOT, "kind": "Container", "label": "The Holy Bible" } } ],
        "next": null, "version": version,
    }));
}
```

- [ ] **Step 2: Run to verify they fail** — 404 for the roots.

- [ ] **Step 3: Implement**

`bible_container_adapter.rs::normalize`: mint `Container` node `bible` (title "The Holy Bible", provenance "kjv") and, per book, `Contains { container: bible, content: Container(book), provenance: "kjv", justification: Default }` into `contains_bible`. `concord_adapter.rs::normalize`: mint `concord` (title "The Book of Concord", provenance "concord") and `Contains { container: concord, content: Container(doc) }` per document into `contains_concord`. `contents.rs`: replace the id-prefix root discovery with "the `contains` children of the corpus root" — `roots = edges(Position::Node(corpus_root), Directed(Contains, Forward))` mapped to `ContentsRoot` exactly as today; the Bible path likewise. `law_check::container_containment_is_a_forest` passes unchanged (one root per corpus).

- [ ] **Step 4: Verify Contents is unchanged and the law holds**

`cargo test -p atlas-contract --test graph_api --test aqc_cucumber --test contract_pact 2>&1 | grep -E "test result|FAILED"` → green; the AQC `contents` scenarios and the `contents` fixtures compare byte-identical (roots are still the books / the documents).

- [ ] **Step 5: Commit**

```bash
git add atlas-graph atlas-contract
git commit -m "graph: a root container per corpus (The Holy Bible, The Book of Concord); Contents finds roots by parentage, output unchanged"
```

---

### Task 5: Concord succession; `CanonSuccession` reaches the Concord section

**Files:**
- Modify: `graph-types/src/sections.rs` (:240 version, :246-276 lists, a `section_of_canon_succession`), `atlas-graph/src/sqlite/{ddl.rs, partition.rs:156, reload.rs}`, `atlas-graph/src/concord_adapter.rs`, `atlas-contract/src/meta.rs` (the `(1, 14)` pin → `(1, 15)`)
- Test: `atlas-graph/tests/sections_real_data.rs`, `atlas-contract/tests/graph_api.rs`

- [ ] **Step 1: Write the failing tests**

`graph_api.rs`:
```rust
const SMALL_CATECHISM: &str = "Container:concord-doc-small-catechism";
const LARGE_CATECHISM: &str = "Container:concord-doc-large-catechism";
const TEN_COMMANDMENTS: &str = "Container:concord-art-small-catechism-2";
const THE_CREED: &str = "Container:concord-art-small-catechism-3";

#[test]
fn the_small_catechism_is_followed_by_the_large_and_the_commandments_by_the_creed() {
    // Arrange
    let app = compiled_app();
    // Act
    let docs = get_json(&app, &format!("/api/node/{SMALL_CATECHISM}/edges?kind=follows-in"));
    let arts = get_json(&app, &format!("/api/node/{TEN_COMMANDMENTS}/edges?kind=follows-in"));
    // Assert
    let targets = |p: &serde_json::Value| p["entries"].as_array().unwrap().iter().map(|e| e["node"]["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!((targets(&docs), targets(&arts)), (vec![LARGE_CATECHISM.to_string()], vec![THE_CREED.to_string()]));
}
```
`sections_real_data.rs`: the per-section family table gains `Concord: CanonSuccession` with the expected row count `DOCUMENT_STEPS + ARTICLE_STEPS` (9 document steps + Σ(articles−1) per document — compute from `DOCUMENTS` and the parsed corpus in the test, named constants for the two terms).

- [ ] **Step 2: Run to verify they fail** — empty `follows-in` page; the section table lacks the family.

- [ ] **Step 3: Implement**

`sections.rs`: `SECTION_SCHEMA_VERSION = 15`; add `RowFamily::CanonSuccession` to the `Concord` list; `fn section_of_canon_succession(row: &CanonSuccession) -> Section { section_of_container_raw(&row.prior.0) }` (same split as `section_of_contains_bible`: `bible-` → Kjv, `concord-` → Concord). `partition.rs:156`: write `canon_succession` rows per `section_of_canon_succession` instead of all to Kjv; `ddl.rs`: the `canon_succession` table and index in the Concord section's DDL list; `reload.rs`: read the family from both sections. `concord_adapter.rs::normalize`: after building `all_docs` (in `DOCUMENTS` order) and, per document, `all_articles`:
```rust
for w in all_docs.windows(2) {
    ctx.graph.canon_succession.push(CanonSuccession { prior: w[0].clone(), next: w[1].clone(), provenance: ProvenanceId::from("concord"), justification: Default::default() });
}
for articles in per_document_articles {
    for w in articles.windows(2) {
        ctx.graph.canon_succession.push(CanonSuccession { prior: w[0].clone(), next: w[1].clone(), provenance: ProvenanceId::from("concord"), justification: Default::default() });
    }
}
```
`meta.rs`: the contract test pin `(1, 14)` → `(1, 15)`.

- [ ] **Step 4: Run**

`cargo test -p atlas-graph-types && cargo test -p atlas-graph --test sections_real_data --test sqlite_laws && cargo test -p atlas-contract --test graph_api --test contract_api 2>&1 | grep -E "test result|FAILED"` → green.

- [ ] **Step 5: Commit**

```bash
git add ../graph-types atlas-graph atlas-contract
git commit -m "graph: Book of Concord documents and articles follow one another (CanonSuccession in the Concord section; schema 15)"
```

---

### Task 6: Small Catechism titles — a curated override, applied in the ETL

**Files:**
- Create: `data/curated/concord-titles.toml`
- Modify: `atlas-etl/src/curated.rs` (parse it), `atlas-etl/src/concord.rs:395-425` (apply it after the `<h3>` parse)
- Test: `atlas-etl/tests/concord_titles.rs`

- [ ] **Step 1: Write the failing test**

```rust
const SMALL_CATECHISM_TITLES_AS_SERVED: [&str; 10] = [
    "Luther's Preface to the Small Catechism",
    "I. The Ten Commandments",
    "II. The Creed",
    "III. The Lord's Prayer",
    "IV. The Sacrament of Holy Baptism",
    "V. Confession",
    "VI. The Sacrament of the Altar",
    "Daily Prayers",
    "Table of Duties",
    "Christian Questions and their Answers",
];

#[test]
fn the_small_catechism_articles_are_numbered_by_chief_part_and_the_appendices_are_not() {
    // Arrange
    let corpus = atlas_etl::concord::parse_corpus(raw_dir(), curated_dir()).unwrap();
    // Act
    let titles: Vec<&str> = corpus.documents.iter().find(|d| d.key == "small-catechism").unwrap().articles.iter().map(|a| a.title.as_str()).collect();
    // Assert
    assert_eq!(titles, SMALL_CATECHISM_TITLES_AS_SERVED);
}
```
(`raw_dir()`/`curated_dir()` are the helpers the other `atlas-etl` tests use for `../../data/raw` and `../../data/curated`.)

- [ ] **Step 2: Run to verify it fails** — titles 2, 5, 6, 7 differ ("The Ten Commandments", "The Sacrament of Holy Baptism", "How Christians should be taught to Confess", "The Sacrament of the Altar").

- [ ] **Step 3: Implement**

`data/curated/concord-titles.toml`:
```toml
# Article titles as served. The vendored raw HTML numbers only two of the six
# chief parts; the six are numbered here and the preface and appendices are not.
[[article]]
document = "small-catechism"
article = 2
title = "I. The Ten Commandments"

[[article]]
document = "small-catechism"
article = 5
title = "IV. The Sacrament of Holy Baptism"

[[article]]
document = "small-catechism"
article = 6
title = "V. Confession"

[[article]]
document = "small-catechism"
article = 7
title = "VI. The Sacrament of the Altar"
```
(Articles 3 and 4 already read "II. The Creed" / "III. The Lord's Prayer".) `curated.rs`: `parse_concord_titles(dir) -> Vec<ConcordTitleOverride { document, article, title }>`; `concord.rs`: after parsing a document's articles, replace `article.title` where an override matches `(doc.key, article.article)`. Delete the earlier `why` comment in the ETL, if any, that explained the mismatch.

- [ ] **Step 4: Run** — `cargo test -p atlas-etl --test concord_titles` → green; then `cargo test -p atlas-contract --test aqc_cucumber` (the `contents` fixtures carry titles: regenerate them with the existing exporter and confirm the only diffs are these four titles).

- [ ] **Step 5: Commit**

```bash
git add ../data/curated/concord-titles.toml atlas-etl ../contracts
git commit -m "etl: Small Catechism chief parts numbered I-VI through a curated title override (R7)"
```

---

### Task 7: Place date claims name their events

**Files:**
- Modify: `data/curated/place-history.toml` (`event_id` on the claims that have one), `atlas-core/src/data.rs:747-751` (`PlaceDateClaim.event_id: Option<String>`), `atlas-etl/src/curated.rs`, `atlas-contract/src/wire/places.rs` (`DateClaim.event: Option<NodeRef>` — CONTRACT-2's `DateClaim` if it has landed, else today's `DateClaimOut`)
- Test: `atlas-etl/tests/place_claims.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn every_place_date_claim_that_names_an_event_names_one_attested_in_its_own_verses() {
    // Arrange
    let (graph, data) = load_compiled();
    let claims = data.place_histories.iter().flat_map(|h| [h.established.as_ref(), h.destroyed.as_ref()]).flatten();
    // Act
    let unattested: Vec<String> = claims
        .filter_map(|c| c.event_id.as_ref().map(|e| (e, &c.verses)))
        .filter(|(event, verses)| !verses.iter().any(|v| graph.event_is_attested_in(event, v)))
        .map(|(event, _)| event.clone())
        .collect();
    // Assert
    assert_eq!(unattested, Vec::<String>::new());
}
```
(`load_compiled` and `event_is_attested_in` are the helpers `canon_real_data.rs` uses to read the compiled graph and the `attests` rows; name them as that file does.)

- [ ] **Step 2: Fill the data**

For each of the seven claims, find the event attested in the claim's first verse — `sqlite3 data/cache/explore/core.sqlite "select event_id from attests where att_from_a = <book ord> and att_from_b = <chapter> and att_from_c = <verse>"` — and set `event_id` in `place-history.toml` (`nineveh` established → `theo-87`, `samaria_1022` established → `theo-176`, as the file's own comments say; the others from the query). A claim whose verses attest no event keeps no `event_id` (expected: Shiloh destroyed).

- [ ] **Step 3: Implement**

`data.rs`: `pub event_id: Option<String>` on `PlaceDateClaim`; `curated.rs` parses it; the served `DateClaim` gains `event: Option<NodeRef>` resolved from the event's node (`describe_node`). CONTRACT-2's `PlaceDetail.established/destroyed` therefore carries the link FOCUS-1's `YearNode.Identity` and FOCUS-9's `dated` field use.

- [ ] **Step 4: Run** — `cargo test -p atlas-etl --test place_claims && cargo test -p atlas-contract --test graph_api` → green.

- [ ] **Step 5: Commit**

```bash
git add ../data/curated/place-history.toml atlas-core atlas-etl atlas-contract
git commit -m "graph: place date claims name the event their verses attest (R2: a founding is an event)"
```

---

### Task 8: `NodeKind::Map` — one map per era, showing what the scene shows (R10)

**Files:**
- Modify: `graph-types/src/id.rs` (`node_kinds!` list gains `Map` **last**; `kind_tags!` gains `MapTag => Map`; `pub type MapId = NodeId<MapTag>;` beside `EraId` at :149), `graph-types/src/node.rs:62` (`NodePayload::Map { label: String, from_year: i32, to_year: i32 }`), `graph-types/src/edge.rs` (`Shown`, `MapSuccession` structs), `graph-types/src/canon/rows.rs` (families `Shown` → `Directed(Shows)`, `MapSuccession` → `Directed(Succession)`, both appended last; `canon_row_vectors.rs:132` 26 → 28), `graph-types/src/graph.rs` (`pub shown`, `pub map_succession`), `graph-types/src/sections.rs` (both in Core), `graph-types/tests/wire_form.rs` (`DECLARED_NODE_KINDS` 15 → 16), `atlas-graph/src/sqlite/partition.rs:57-92` (`NodeKind::Map => 15`; the `ALL` array to 16), `atlas-graph/src/sqlite/{ddl,reload}.rs`, `rows/core.rs`, `rows/mod.rs`, `provenance.rs`, `frontier.rs` (the D5 pattern again), `atlas-contract/tests/contract_pact.rs:120` (`node_kind_manifest![…, Map]`), `atlas-graph/src/pipeline.rs` (register the pass)
- Create: `atlas-graph/src/map_adapter.rs`
- Test: `atlas-contract/tests/graph_api.rs`, `atlas-graph/tests/sqlite_laws.rs`

**Interfaces:**
- Produces: node ids `Map:era-{eraId}`; rows `Shown { map: MapId, node: AnyNodeId, provenance: ProvenanceId }`, `MapSuccession { prior: MapId, next: MapId, provenance: ProvenanceId }`; `map_adapter::normalize(ctx) -> MapAdapterStats { maps, shown, steps }`.

- [ ] **Step 1: Write the failing tests**

`graph_api.rs`:
```rust
const SHOWABLE_KINDS: [&str; 4] = ["Place", "Event", "Polity", "Narrative"];

#[test]
fn the_map_for_the_first_era_shows_its_places_events_polities_and_narratives_and_is_followed_by_the_next() {
    // Arrange
    let app = compiled_app();
    let eras = get_json(&app, "/api/eras");
    let first = eras[0]["id"].as_str().unwrap();
    // Act
    let card = get_json(&app, &format!("/api/node/Map:era-{first}"));
    let shown = get_json(&app, &format!("/api/node/Map:era-{first}/edges?kind=shows&limit=200"));
    // Assert
    let groups: Vec<&str> = card["edge_summary"].as_array().unwrap().iter().map(|g| g["kind"].as_str().unwrap()).collect();
    assert_eq!(groups, ["shows", "follows-in"]);
    assert_eq!(card["kind"], "Map");
    assert_eq!(card["label"], eras[0]["name"]);
    let kinds: std::collections::BTreeSet<&str> = shown["entries"].as_array().unwrap().iter().map(|e| e["node"]["kind"].as_str().unwrap()).collect();
    assert!(kinds.iter().all(|k| SHOWABLE_KINDS.contains(k)), "{kinds:?}");
    assert!(shown["entries"].as_array().unwrap().len() > 0);
}
```
`sqlite_laws.rs`: `a_shown_row_and_a_map_succession_row_round_trip_through_the_core_section` in the shape of Task 2's law test.

- [ ] **Step 2: Run to verify they fail** — `Map` is not a kind; 404.

- [ ] **Step 3: Implement the kind, the rows, the adapter**

`id.rs`: add `Map` as the sixteenth `node_kinds!` entry (ordinal 15 — the kind ordinal is stored in `node.kind`, so it must be appended, never inserted); `MapTag => Map` in `kind_tags!`; `pub type MapId = NodeId<MapTag>;`. `node.rs`: the `Map` payload variant beside `Era`. `edge.rs`:
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown { pub map: MapId, pub node: AnyNodeId, pub provenance: ProvenanceId }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapSuccession { pub prior: MapId, pub next: MapId, pub provenance: ProvenanceId }
```
(`MapSuccession` is the third row implementation of `RelationId::Succession`, beside `Succession` and `CanonSuccession` — the precedent `edge.rs:84-86` documents; `CanonSuccession` is typed to `ContainerNodeId` and a Map is not a container.) Row families, storage, laws: the D5 pattern (`git show 7d64ccb --stat`), families `Shown` and `MapSuccession` appended last in `RowFamily`. `partition.rs`: `NodeKind::Map => 15` in `node_kind_ordinal` and as the sixteenth entry of `node_kind_of_ordinal`'s `ALL`.

`atlas-graph/src/map_adapter.rs`:
```rust
pub fn normalize(ctx: &mut BuildCtx) -> MapAdapterStats {
    let mut stats = MapAdapterStats::default();
    let source = GraphSceneSource::over(&ctx.graph, &ctx.data);   // the same source GraphService::scene_source builds (scene_source.rs:316)
    let mut maps: Vec<MapId> = Vec::new();
    for era in eras_in_order(&ctx.data) {
        let map = MapId::new(format!("era-{}", era.id));
        ctx.graph.nodes.insert(map.clone().erase(), Node {
            id: map.clone().erase(),
            payload: NodePayload::Map { label: era.name.clone(), from_year: era.from_year, to_year: era.to_year },
            provenance: "curated-eras".to_string(),
        });
        let window = TimeRange::new(era.from_year, era.to_year).expect("curated eras are well-formed");
        let scene = compose_time_scene(&source, window);
        for node in shown_in(&scene, &polities_reigning_in(&ctx.graph, &window)) {
            ctx.graph.shown.push(Shown { map: map.clone(), node, provenance: ProvenanceId::from("curated-eras") });
            stats.shown += 1;
        }
        maps.push(map);
        stats.maps += 1;
    }
    for w in maps.windows(2) {
        ctx.graph.map_succession.push(MapSuccession { prior: w[0].clone(), next: w[1].clone(), provenance: ProvenanceId::from("curated-eras") });
        stats.steps += 1;
    }
    stats
}

fn shown_in(scene: &Scene, polities: &[AnyNodeId]) -> Vec<AnyNodeId> {
    let places = scene.places.iter().map(|p| place_node_id(&p.id)).chain(scene.quiet_places.iter().map(|p| place_node_id(&p.id)));
    let events = scene.places.iter().flat_map(|p| p.events.iter().map(|e| event_node_id(&e.id)));
    let narratives = scene.narratives.iter().map(|n| narrative_node_id(&n.id));
    let mut all: Vec<AnyNodeId> = places.chain(events).chain(narratives).chain(polities.iter().cloned()).collect();
    all.sort();
    all.dedup();
    all
}
```
`eras_in_order` sorts the curated eras by `from_year` (the order the slider walks); `polities_reigning_in` is the filter `handlers.rs::polities` applies (`:275-290`) — reuse that function from `atlas-graph` if it lives there, else move it there and have the handler call it. `GraphSceneSource::over` is the existing constructor's name (`scene_source.rs:316` region); `place_node_id`, `event_node_id` (`event_world::event_node_id`), `narrative_node_id` are the existing id builders. Register the pass in `pipeline.rs` after the era, place, event, narrative and polity passes.

- [ ] **Step 4: Run**

`cargo test -p atlas-graph-types --features serde,openapi && cargo test -p atlas-graph --test sqlite_laws --test canon_real_data && cargo test -p atlas-contract --test graph_api --test contract_pact 2>&1 | grep -E "test result|FAILED"` → green; `cargo run -p atlas-contract --bin export_contract` regenerates the documents (`NodeKind` gains `Map`; NSwag will regenerate the client enum on its next build).

- [ ] **Step 5: Commit**

```bash
git add ../graph-types atlas-graph atlas-contract ../contracts
git commit -m "graph: Map is a node kind -- one map per era, showing the era's places, events, polities and narratives, each followed by the next (R10)"
```

---

### Task 9: Pins, pacts, gates, push

- [ ] **Step 1: Re-bless the pact once**

`ATLAS_BLESS_PACT=1 cargo test -p atlas-contract --test contract_pact` (records and fails once by design), then `cargo test -p atlas-contract --test contract_pact` → green. `contracts/pacts/http.json` now carries the new version root.

- [ ] **Step 2: Regenerate documents and fixtures**

`cargo run -p atlas-contract --bin export_contract && cargo run -p atlas-contract --bin export_aqc_examples`; `git diff --stat ../contracts` shows: `openapi.yaml`/`aqc.schema.json`/`graph-vocabulary.json` (two new relations, one new node kind), the AQC fixtures' `version` fields, the Concord `contents` fixture titles. Bump `contracts/atlas-graph-contract/VERSION` minor and add the CHANGELOG line ("authored-by and shows; Map nodes per era; corpus roots; Concord succession; Small Catechism titles; place claims name events").

- [ ] **Step 3: The standing block and every gate**

From `server/`: the three commands of the STANDING COUNTING PROCEDURE; `bash ../scripts/timing-gates.sh` (all entries; gates 1, 8, 9 measure the whole graph — record their times against the ceilings); `bash ../scripts/timing-gates.sh check`; `bash ../scripts/contract-gate.sh`; `bash ../scripts/contract-semver-gate.sh`. `scene_byte_identity.rs`'s 25 hashes are unchanged (no event or place moved).

- [ ] **Step 4: Commit and push**

```bash
git add -A ../contracts
git commit -m "contracts: FOCUS-0 pins -- pact re-blessed, documents and fixtures regenerated, AGC minor"
git push origin worktree-bible-atlas-m1
```

---

## Self-review against the spec

- **Coverage:** §7 items 1 (Task 4), 2 (Task 5), 3 (Task 9), 4 (Tasks 1–3, 7), 5 (Task 6), 6 (Tasks 1, 8); R2's four kinds — Author (Tasks 1–3), TimeAndPlace (no graph change), Year (Task 7), PolityDelta (MAPS, untouched); R5 scope; R7; R10 (Task 8).
- **Placeholders:** none. Where a value comes from data (the 14 person ids, the seven `event_id`s, the first era's id) the plan names the query that yields it; where a constructor's exact name is the existing code's (`GraphSceneSource::over`, `place_node_id`, `narrative_node_id`) the file and line region are named.
- **Type consistency:** `Authored { book, person, provenance, justification }` (Task 2) is what Task 3 constructs; `Shown`/`MapSuccession` (Task 8) lower into `Shows`/`Succession` declared in Task 1; `SECTION_SCHEMA_VERSION = 15` (Task 5) is what the `meta.rs` pin asserts (CONTRACT-2 takes it to 16); the root ids `Container:bible` / `Container:concord` (Task 4) and `Map:era-{id}` (Task 8) are what FOCUS-1's `LegacyNodes`, `HomeSurfaces` and FOCUS-3/6's presentations name.
