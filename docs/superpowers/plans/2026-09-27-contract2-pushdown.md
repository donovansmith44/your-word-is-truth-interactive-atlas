# CONTRACT-2 (pushdown) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The client never re-derives what the graph knows. Years travel with their labels, loci are structured, anchors and edge metadata are served, kind details ride on the card, mention spans and Concord citations are computed once in ETL — and `YearText`, `Versification`, `CanonRef`, `ScriptureRefScan` (for Concord), `PlaceMentions` and the version constants are deleted.

**Architecture:** New wire modules `time.rs` and `locus.rs` in `atlas-contract`; the reading window assembles `locus`, `anchors`, `heading`; `node_edges` carries `votes`/`narrative`/`loci`/`note`; `node_card` fills flat per-kind details from the same sources the legacy handlers read. ETL gains a KJV token layer with character offsets, a mention-span pass (the C# name search ported, with its tests as vectors) and a Concord citation pass (the C# grammar ported). Rust emits golden vectors; the client's one remaining parser and the Playwright specs assert against them. AQC bumps major.

**Tech Stack:** Rust workspace; `atlas-contract` (utoipa); SQLite sections (`SECTION_SCHEMA_VERSION` 15 → 16); .NET 10 client; Playwright.

**Spec:** `docs/superpowers/specs/2026-09-27-contract2-pushdown-design.md` (P1–P9, §3 types, §8 rulings — take the defaults unless the owner rules otherwise). **Prerequisites:** CONTRACT-1a/1b and FOCUS-0 complete. All Rust commands from `server/`.

## Global Constraints

- `docs/PRINCIPLES.md` binds: TDD, whole-body assertions, named constants, newspaper order, `why` comments only.
- **This batch changes the wire on purpose** (P9): AQC `VERSION` → `1.0.0`, AGC minor; every fixture is regenerated through the existing exporters, never hand-edited; `scene_byte_identity.rs`'s 25 hashes are re-pinned once, in the task that changes `Scene`, with the reason in the commit message.
- Struct and property names are the spec's (§3): `Year`, `TimeRange`, `DateClaim`, `TextRef`, `TextSpan`, `Anchor`, `Heading`, `EventDetail`, `PlaceDetail`, `CatechismDetail`, `BookDetail`, `GeoPoint` is FOCUS's; `EdgeEntry.{votes,narrative,loci,note}`; `TextUnit.{locus,anchors,heading}`; `NodeCard.{event,place,catechism,book}`.
- Labels are produced by exactly one implementation (`wire/time.rs`); the golden vectors under `contracts/atlas-query-contract/vectors/` are exporter outputs, committed and diff-gated like the other documents.
- The legacy detail routes are **not** changed here (FOCUS retires them); they keep serving integers so their pins hold.
- Commit per task; push at the end (owner authorization on record).

---

### Task 1: Years with labels — `wire/time.rs` and the vectors

**Files:**
- Create: `atlas-contract/src/wire/time.rs`
- Modify: `atlas-contract/src/wire/mod.rs`, `atlas-contract/src/document.rs` (`generated_files` gains the vectors), `atlas-contract/src/bins/export_contract.rs` (unchanged: it writes whatever `generated_files` lists)
- Test: `atlas-contract/tests/year_labels.rs`

**Interfaces:**
- Produces: `Year { value: i32, label: String }` with `Year::of(i32)`; `TimeRange { from: Year, to: Year, label: String }` with `TimeRange::of(i32, i32)`; `DateClaim { when: TimeRange, label: String, verses: Vec<TextSpan>, note: Option<String>, event: Option<NodeRef> }` with `DateClaim::of(TimeRange, Vec<TextSpan>, Option<String>, Option<NodeRef>)`; `contracts/atlas-query-contract/vectors/year-labels.json`.

- [ ] **Step 1: Write the failing test**

`atlas-contract/tests/year_labels.rs`:
```rust
use atlas_contract::wire::time::{TimeRange, Year};

const EN_DASH: &str = " – ";

#[test]
fn a_year_is_labelled_bc_or_ad_and_never_zero() {
    // Arrange
    let years = [-4004, -1, 1, 33, 100, 2000];
    // Act
    let labels: Vec<String> = years.iter().map(|y| Year::of(*y).label).collect();
    // Assert
    assert_eq!(labels, ["4004 BC", "1 BC", "AD 1", "AD 33", "AD 100", "AD 2000"]);
}

#[test]
fn a_range_names_the_era_once_when_both_ends_share_it_and_twice_otherwise() {
    // Arrange
    let ranges = [(-1450, -1400), (-5, 30), (33, 33), (1, 100)];
    // Act
    let labels: Vec<String> = ranges.iter().map(|(f, t)| TimeRange::of(*f, *t).label).collect();
    // Assert
    assert_eq!(labels, [format!("1450{EN_DASH}1400 BC"), format!("5 BC{EN_DASH}AD 30"), "AD 33".to_string(), format!("AD 1{EN_DASH}100")]);
}

#[test]
fn a_claim_with_a_note_is_labelled_circa() {
    // Arrange
    use atlas_contract::wire::time::DateClaim;
    let when = TimeRange::of(-1003, -1003);
    // Act
    let (noted, plain) = (
        DateClaim::of(when.clone(), vec![], Some("traditional".into()), None).label,
        DateClaim::of(when, vec![], None, None).label,
    );
    // Assert
    assert_eq!((noted, plain), ("c. 1003 BC".to_string(), "1003 BC".to_string()));
}

#[test]
fn the_vectors_document_is_the_same_rules_written_out() {
    // Arrange
    let expected = serde_json::json!({
        "years":  [ {"value": -4004, "label": "4004 BC"}, {"value": -1, "label": "1 BC"}, {"value": 1, "label": "AD 1"}, {"value": 33, "label": "AD 33"}, {"value": 100, "label": "AD 100"}, {"value": 2000, "label": "AD 2000"} ],
        "ranges": [ {"from": -1450, "to": -1400, "label": format!("1450{EN_DASH}1400 BC")}, {"from": -5, "to": 30, "label": format!("5 BC{EN_DASH}AD 30")}, {"from": 33, "to": 33, "label": "AD 33"}, {"from": 1, "to": 100, "label": format!("AD 1{EN_DASH}100")} ],
        "claims": [ {"from": -1003, "to": -1003, "note": "traditional", "label": "c. 1003 BC"}, {"from": -586, "to": -586, "note": null, "label": "586 BC"} ],
    });
    // Act
    let actual: serde_json::Value = serde_json::from_str(&atlas_contract::document::year_labels_json()).unwrap();
    // Assert
    assert_eq!(actual, expected);
}
```
(The rules are `client/YearText.cs`'s, read off `client.Tests/YearTextTests.cs`: `y < 0 → "{-y} BC"`, else `"AD {y}"`; a range joins with an en dash and names a shared era once; a claim with a note is prefixed `"c. "`. If `YearTextTests` pins a case this list misses — e.g. ranges crossing year 0, the BC/AD boundary wording — add it to both the test and the vectors before implementing; the vectors must cover every case the C# tests covered.)

- [ ] **Step 2: Run to verify it fails** — `cargo test -p atlas-contract --test year_labels` → compile error (`wire::time` missing).

- [ ] **Step 3: Implement**

`atlas-contract/src/wire/time.rs`:
```rust
use serde::Serialize;
use utoipa::ToSchema;

use super::locus::TextSpan;
use super::NodeRef;

const EN_DASH: &str = " – ";

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Year {
    pub value: i32,
    pub label: String,
}

impl Year {
    pub fn of(value: i32) -> Year {
        Year { value, label: if value < 0 { format!("{} BC", -value) } else { format!("AD {value}") } }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TimeRange {
    pub from: Year,
    pub to: Year,
    pub label: String,
}

impl TimeRange {
    pub fn of(from: i32, to: i32) -> TimeRange {
        let (f, t) = (Year::of(from), Year::of(to));
        let label = match (from == to, from < 0, to < 0) {
            (true, _, _) => f.label.clone(),
            (false, true, true) => format!("{}{EN_DASH}{}", -from, t.label),
            (false, false, false) => format!("{}{EN_DASH}{}", f.label, to),
            _ => format!("{}{EN_DASH}{}", f.label, t.label),
        };
        TimeRange { from: f, to: t, label }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DateClaim {
    pub when: TimeRange,
    pub label: String,
    pub verses: Vec<TextSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<NodeRef>,
}

impl DateClaim {
    pub fn of(when: TimeRange, verses: Vec<TextSpan>, note: Option<String>, event: Option<NodeRef>) -> DateClaim {
        let label = if note.is_some() { format!("c. {}", when.label) } else { when.label.clone() };
        DateClaim { when, label, verses, note, event }
    }
}
```
`document.rs`: `pub fn year_labels_json() -> String` building the JSON above from `Year::of`/`TimeRange::of`/`DateClaim::of` over the fixed lists (the lists are consts in `document.rs`: `VECTOR_YEARS`, `VECTOR_RANGES`, `VECTOR_CLAIMS`), and `generated_files()` gains `(root.join("atlas-query-contract/vectors/year-labels.json"), year_labels_json())`. `TextSpan` arrives in Task 3; until then `verses: Vec<String>` compiles — write it as `Vec<TextSpan>` in Task 3 and use `Vec<String>` here only if Task 3 has not landed (the tasks run in order, so land Task 3's `locus.rs` types first if you prefer one commit).

- [ ] **Step 4: Run** — `cargo test -p atlas-contract --test year_labels` → 4 passed; `cargo run -p atlas-contract --bin export_contract` writes the vectors.

- [ ] **Step 5: Commit**

```bash
git add atlas-contract ../contracts/atlas-query-contract/vectors
git commit -m "contract: years travel with their labels -- wire::time (Year, TimeRange, DateClaim) and the year-labels vectors (P1)"
```

---

### Task 2: Structured loci — `wire/locus.rs`

**Files:**
- Create: `atlas-contract/src/wire/locus.rs`
- Test: `atlas-contract/tests/locus_wire.rs`

**Interfaces:**
- Produces: `TextRef::Bible { book: String, chapter: u16, verse: u16 } | TextRef::Concord { part: u8, article: u16, paragraph: u16 }` (externally tagged), `TextSpan { from: TextRef, to: TextRef }`, `Anchor { start: usize, end: usize, kind: EdgeKind, node: NodeRef }`; `TextRef::from_verse(&VerseRef, &CanonBook list)`, `TextRef::from_concord(&ConcordRef)`, `TextSpan::of_range(&BibleLocusRange, …)`, `TextSpan::single(TextRef)`.

- [ ] **Step 1: Write the failing test**

```rust
use atlas_contract::wire::locus::{TextRef, TextSpan};

#[test]
fn a_bible_ref_serialises_externally_tagged_with_the_book_code() {
    // Arrange
    let r = TextRef::Bible { book: "GEN".into(), chapter: 1, verse: 1 };
    // Act
    let json = serde_json::to_value(&r).unwrap();
    // Assert
    assert_eq!(json, serde_json::json!({ "bible": { "book": "GEN", "chapter": 1, "verse": 1 } }));
}

#[test]
fn a_concord_ref_serialises_externally_tagged() {
    // Arrange
    let r = TextRef::Concord { part: 7, article: 2, paragraph: 1 };
    // Act
    let json = serde_json::to_value(&r).unwrap();
    // Assert
    assert_eq!(json, serde_json::json!({ "concord": { "part": 7, "article": 2, "paragraph": 1 } }));
}

#[test]
fn a_verse_ref_becomes_a_bible_text_ref_through_the_canon() {
    // Arrange
    let verse = atlas_graph_types::text::VerseRef { book: 0, chapter: 1, verse: 1 };
    let canon = atlas_core::data::Canon::kjv();
    // Act
    let r = TextRef::from_verse(&verse, &canon);
    // Assert
    assert_eq!(r, TextRef::Bible { book: "GEN".into(), chapter: 1, verse: 1 });
}
```
(`Canon::kjv()` is whatever the canon-list constructor in `atlas-core/src/data.rs` is called — the one `CanonBook` rows come from; use that name.)

- [ ] **Step 2: Run** — compile error.

- [ ] **Step 3: Implement**

```rust
use atlas_graph_types::EdgeKind;
use atlas_graph_types::text::{ConcordRef, VerseRef};
use serde::Serialize;
use utoipa::ToSchema;

use super::NodeRef;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum TextRef {
    Bible { book: String, chapter: u16, verse: u16 },
    Concord { part: u8, article: u16, paragraph: u16 },
}

impl TextRef {
    pub fn from_verse(v: &VerseRef, canon: &atlas_core::data::Canon) -> TextRef {
        TextRef::Bible { book: canon.books[v.book as usize].code.clone(), chapter: v.chapter, verse: v.verse }
    }

    pub fn from_concord(c: &ConcordRef) -> TextRef {
        TextRef::Concord { part: c.part, article: c.article, paragraph: c.paragraph }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TextSpan {
    pub from: TextRef,
    pub to: TextRef,
}

impl TextSpan {
    pub fn single(at: TextRef) -> TextSpan { TextSpan { from: at.clone(), to: at } }
}

#[derive(Debug, Clone, PartialEq, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Anchor {
    pub start: usize,
    pub end: usize,
    pub kind: EdgeKind,
    pub node: NodeRef,
}
```
`TextSpan::of_range(&BibleLocusRange, &Canon)` maps `from.unit`/`to.unit` through `from_verse`.

- [ ] **Step 4: Run** — 3 passed. **Step 5: Commit** — `git commit -m "contract: structured loci on the wire -- TextRef, TextSpan, Anchor (P2)"`.

---

### Task 3: Every year on the graph path becomes a `Year`/`TimeRange`; `locus` beside every `ref`

**Files:**
- Modify: `atlas-contract/src/wire/{graph,contents,map,places,events,reading}.rs` (the fields in spec §3), `atlas-contract/src/{graph,contents,map,places,events,reading}.rs` (the fills), `atlas-contract/src/wire/map.rs` (`Era { id, name, window: TimeRange }` replacing the served `atlas_core::data::Era`), `atlas-contract/tests/scene_byte_identity.rs` (re-pin)
- Regenerate: AQC fixtures (`export_aqc_examples`), `openapi.yaml` + derived documents, AGC pact (re-bless)

- [ ] **Step 1: Write the failing tests** (in `graph_api.rs`, whole-body):

```rust
#[test]
fn a_person_card_carries_labelled_years() {
    // Arrange
    let app = compiled_app();
    // Act
    let body = get_json(&app, "/api/node/Person:moses_2108");
    // Assert
    assert_eq!(body["person"]["birth"], serde_json::json!({ "value": -1526, "label": "1526 BC" }));
}

#[test]
fn a_text_unit_carries_its_structured_locus_beside_its_ref() {
    // Arrange
    let app = compiled_app();
    // Act
    let body = get_json(&app, "/api/text?ref=GEN.1.1&n=1");
    // Assert
    assert_eq!(body["units"][0]["ref"], "GEN.1.1");
    assert_eq!(body["units"][0]["locus"], serde_json::json!({ "bible": { "book": "GEN", "chapter": 1, "verse": 1 } }));
}
```
(Moses' birth year is whatever `PersonLife` serves today as `birth_year`; read it once from the running API and put the true value in the test.)

- [ ] **Step 2: Run** — fail (`birth_year` is an integer; no `locus`).

- [ ] **Step 3: Implement the field changes**

Per spec §3: `PersonLife.{birth,death,first,last}: Option<Year>` filled with `Year::of`; `Scene.window: Option<TimeRange>` via `TimeRange::of(w.from_year, w.to_year)`; `SceneEvent.when: TimeRange`; `ScenePlace.existence: Option<TimeRange>` and `QuietPlace.existence: Option<TimeRange>` (replacing the two `Option<i32>`s); `Polity.reign: TimeRange` (replacing `from`/`to`); `wire::Era { id, name, window }` served by `map::eras` (mapping `atlas_core::data::Era`); `DateClaim` in `PlaceDetail` (legacy route untouched — the new `DateClaim` is used only by Task 5's `NodeCard.place`); `BookDetail.written: Option<TimeRange>`. `TextUnit.locus: TextRef` built in `text_window` from `(b, c, v)` via `TextRef::from_verse` (Bible) / `TextRef::from_concord` (Concord); `ContentsRoot.locus`/`ContentsChild.locus` likewise in `contents.rs`.

- [ ] **Step 4: Regenerate and re-pin**

`cargo run -p atlas-contract --bin export_contract && cargo run -p atlas-contract --bin export_aqc_examples`; `ATLAS_BLESS_PACT=1 cargo test -p atlas-contract --test contract_pact` then again without the variable; `scene_byte_identity.rs`: regenerate its 25 hashes (the test prints the actual hashes on failure; replace the pinned array in one commit whose message says "scene wire: TimeRange objects replace year integers (CONTRACT-2 P1)"). Bump `contracts/atlas-query-contract/VERSION` to `1.0.0`, add the CHANGELOG entry listing P1/P2 as breaking.

- [ ] **Step 5: Run** — `cargo test -p atlas-contract 2>&1 | grep -E "test result|FAILED"` → green. **Commit**: `git commit -m "contract: labelled years and structured loci on every graph-path struct; Scene, Era, Polity carry TimeRange (AQC 1.0.0)"`.

---

### Task 4: Edge entries carry their metadata

**Files:**
- Modify: `graph-types/src/explore.rs` (`EdgeEntry` gains `votes: Option<u32>`, `narrative: Option<String>`), `atlas-graph/src/sqlite/snapshot.rs` (`edges_inner` fills them from the columns it already selects), `atlas-graph/src/service.rs` (an `attestation_of(edge_id) -> Option<(BibleLocusRange, Option<String>)>` over the `attests` row family), `atlas-contract/src/wire/graph.rs` (`EdgeEntry` fields), `atlas-contract/src/graph.rs` (`node_edges` fills `votes`, `narrative`, and for `AttestedIn`/`Attests` `loci` (coalesced runs) and `note`)
- Test: `atlas-contract/tests/graph_api.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_cites_entry_carries_its_votes() {
    // Arrange
    let app = compiled_app();
    // Act
    let page = get_json(&app, "/api/node/text-unit:JHN.3.16/edges?kind=cites&limit=1");
    // Assert
    let entry = &page["entries"][0];
    assert!(entry["votes"].is_u64(), "{entry}");
    assert_eq!(entry.as_object().unwrap().keys().collect::<Vec<_>>(), ["edge", "node", "votes"]);
}

#[test]
fn an_attested_in_entry_carries_the_attestations_coalesced_spans() {
    // Arrange
    let app = compiled_app();
    // Act
    let page = get_json(&app, "/api/node/ab_ur/edges?kind=attested-in&limit=1");
    // Assert
    let entry = &page["entries"][0];
    assert_eq!(entry["loci"][0]["from"]["bible"]["book"], "GEN");
    assert_eq!(entry.as_object().unwrap().keys().collect::<Vec<_>>(), ["edge", "loci", "node"]);
}
```
(`Option` fields absent when `None` — `skip_serializing_if` — so key sets are whole-body assertions of which metadata each kind carries. Take the exact `loci` value from the legacy `/api/event/ab_ur` `witnesses[0].verse_groups` for the whole-body form once it is known.)

- [ ] **Step 2: Run** — fail (no such keys).

- [ ] **Step 3: Implement**

`explore.rs`: add the two optional fields to `EdgeEntry`; `snapshot.rs::edges_inner`: map `meta_votes` → `votes`, `meta_narrative` → `narrative` (both already read). Coalesced runs: `atlas-graph/src/runs.rs`: `pub fn coalesce(loci: &[VerseRef], canon: &Canon) -> Vec<(VerseRef, VerseRef)>` — consecutive verses within a chapter join; a run ending on a chapter's last verse joins the next chapter's first (the rule `PassageBlock.StitchRuns` implements; port it with `client.Tests/AcctCoalesceTests.cs`'s 18 cases as `contracts/atlas-query-contract/vectors/attestation-runs.json`, emitted by `document::attestation_runs_json()` and replayed by `atlas-graph/tests/runs_vectors.rs`). `node_edges`: for `EdgeKind::Directed(Attests, _)` entries, `graph.attestation_of(&e.edge)` → `loci: Some(coalesce(...).map(TextSpan::of_range))`, `note` from the same source `handlers.rs::event` reads for `EventWitnessOut.ref_note`.

- [ ] **Step 4: Run** — green, fixtures regenerated (`export_aqc_examples`, pact re-bless). **Commit**: `git commit -m "graph: edge entries carry votes, narrative, coalesced attestation loci and notes (P6)"`.

---

### Task 5: Kind details on the card; pericope headings on text units

**Files:**
- Modify: `atlas-contract/src/wire/graph.rs` (`NodeCard.{event,place,catechism,book}`, `TextUnit.heading`, `EventDetail`, `PlaceDetail`, `CatechismDetail`, `BookDetail`, `Heading`), `atlas-contract/src/graph.rs` (`node_card` fills by kind; `text_window` fills `heading`)
- Test: `atlas-contract/tests/graph_api.rs`

- [ ] **Step 1: Write the failing tests** — one per detail, whole-body against the legacy route's values:

```rust
#[test]
fn an_event_card_carries_the_details_the_legacy_route_served() {
    // Arrange
    let app = compiled_app();
    let legacy = get_json(&app, "/api/event/ab_ur");
    // Act
    let card = get_json(&app, "/api/node/ab_ur");
    // Assert
    assert_eq!(card["event"], serde_json::json!({
        "kind": legacy["kind"],
        "when": { "from": { "value": legacy["when"]["from_year"], "label": card["event"]["when"]["from"]["label"] },
                  "to":   { "value": legacy["when"]["to_year"],   "label": card["event"]["when"]["to"]["label"] },
                  "label": card["event"]["when"]["label"] },
        "robertson_section": legacy["robertson_section"], "acts_section": legacy["acts_section"],
        "atlas_section": legacy["atlas_section"], "kjv_superscription": legacy["kjv_superscription"], "ref_note": legacy["ref_note"],
    }));
}
```
and likewise `a_place_card_carries_coordinates_history_and_dated_claims` (`lat`, `lon`, `display_name`, `canonical_name`, `blurb`, `established`/`destroyed` as `DateClaim`s with `event` from FOCUS-0's `event_id`), `a_catechism_card_carries_its_prose` (`part_title`, `text`, `explanation_heading`, `explanation`, `where_written`), `a_book_card_carries_authorship_and_writing` (`author`, `write_place`, `written`), `a_verse_that_opens_a_pericope_carries_its_heading` (`/api/text?ref=GEN.1.1&n=1` → `heading: { event, title, kind, is_continuation }` equal to the legacy chapter's first verse heading).

- [ ] **Step 2: Run** — fail.

- [ ] **Step 3: Implement** — `node_card` matches on `node_id.kind` and fills `event` from the same `AtlasData` event record `handlers.rs::event` reads (`e.kind`, `e.when` → `TimeRange::of`, the five section/note strings), `place` from `data.place_history_for`/`place_name_alias_for` and the place's coordinates (the `PlaceDetailOut` window at `handlers.rs:2095-2115`), `catechism` from the catechism item and its part (`handlers.rs:1917-1926`), `book` from `data.books_meta` (`handlers.rs:1212-1216`); `text_window` fills `heading` from `graph.heading_index` (`handlers.rs:591-594`) with `event` as a `NodeRef`.

- [ ] **Step 4: Run** — green; regenerate fixtures; **Commit**: `git commit -m "contract: kind details ride on the card (event, place, catechism, book); pericope headings on text units (P7)"`.

---

### Task 6: A KJV token layer with character offsets

**Files:**
- Modify: `graph-types/src/sections.rs` (`SECTION_SCHEMA_VERSION` 15 → 16), `atlas-graph/src/sqlite/{ddl.rs:572-592, extras.rs:80-135}` (`token` gains `char_start INTEGER, char_end INTEGER`), the lexicon/token writer, `atlas-contract/src/meta.rs` pin (`(1, 16)`)
- Create: `atlas-graph/src/kjv_tokens.rs`
- Test: `atlas-graph/tests/kjv_tokens.rs`

- [ ] **Step 1: Write the failing tests**

```rust
use atlas_graph::kjv_tokens::{tokenize, Token};

#[test]
fn tokens_are_words_with_character_offsets_that_reassemble_the_verse() {
    // Arrange
    let verse = "In the beginning God created the heaven and the earth.";
    // Act
    let tokens = tokenize(verse);
    let reassembled: String = tokens.iter().map(|t| &verse[t.char_start..t.char_end]).collect::<Vec<_>>().join(" ");
    // Assert
    assert_eq!(tokens[0], Token { ord: 0, form: "In".into(), char_start: 0, char_end: 2 });
    assert_eq!(tokens.last().unwrap(), &Token { ord: 9, form: "earth".into(), char_start: 48, char_end: 53 });
    assert_eq!(reassembled, "In the beginning God created the heaven and the earth");
}

#[test]
fn every_kjv_verse_reassembles_from_its_tokens() {
    // Arrange
    let (graph, _) = load_compiled();
    // Act
    let broken: Vec<String> = graph.every_verse_text().filter(|(_, text)| !reassembles(text)).map(|(r, _)| r).collect();
    // Assert
    assert_eq!(broken, Vec::<String>::new());
}
```
(`reassembles` = the join above compared to the verse with punctuation stripped by the same rule `tokenize` uses; `every_verse_text` = `verse_text_of` over the KJV reading spine.)

- [ ] **Step 2: Run** — compile error.

- [ ] **Step 3: Implement** — `tokenize`: split on whitespace; strip leading/trailing punctuation from each word while keeping the offsets of the retained core (`char_start`/`char_end` index into the original string); `ord` sequential. The writer emits one `token` row per KJV token with `layer = "kjv"`, `form`, `char_start`, `char_end`, `lemma/xpos/translit/strong = NULL`, `aligned = 0`, into the KJV section's `token` table (the lexicon section's `token` rows are unchanged; both sections carry the table's DDL). `SECTION_SCHEMA_VERSION = 16`; `meta.rs` pin.

- [ ] **Step 4: Run** — `cargo test -p atlas-graph --test kjv_tokens --test sections_real_data` → green (whole-verse law over 31,102 verses). **Commit**: `git commit -m "graph: the KJV has a token layer with character offsets (schema 16) -- the layer mention and citation spans live in (P4)"`.

---

### Task 7: Mention spans, computed once — the name search ported

**Files:**
- Create: `atlas-graph/src/mention_spans.rs` (port of `client/Explore/PlaceMentions.cs`), `atlas-graph/tests/mention_spans_vectors.rs`
- Modify: the four mention writers (`place_adapter.rs:74`, `event_world.rs:720`, `peoples_adapter.rs:282,298`, `person_adapter.rs:171,481`) — after they run, a pass `mention_spans::locate(graph, verse_text_of, names)` sets `Mentions.locus.span` to `TokenSpan { layer: "kjv", start, end }` for every occurrence; `atlas-contract/src/document.rs` (`mention_spans_json()` from the 11 C# cases), `atlas-contract/src/graph.rs` (`text_window` builds `anchors` from the unit's `mentions` edges: token span → `char_start`/`char_end` of the unit's `kjv` tokens)
- Test: `atlas-contract/tests/graph_api.rs`

- [ ] **Step 1: Write the failing tests**

`mention_spans_vectors.rs` replays `contracts/atlas-query-contract/vectors/mention-spans.json`, whose cases are `client.Tests/PlaceMentionsTests.cs`'s eleven, transcribed by the exporter from a Rust literal (`document::MENTION_CASES`: text, place names, person names, expected `(start, end, kind, id)` segments — copy each case's inputs and expected output from the C# test file verbatim):
```rust
#[test]
fn the_ported_name_search_agrees_with_every_recorded_case() {
    // Arrange
    let cases: Vec<Case> = serde_json::from_str(&std::fs::read_to_string(vectors("mention-spans.json")).unwrap()).unwrap();
    // Act
    let outcomes: Vec<(String, Vec<Segment>)> = cases.iter().map(|c| (c.name.clone(), mention_spans::scan(&c.text, &c.places, &c.persons))).collect();
    // Assert
    assert_eq!(outcomes, cases.iter().map(|c| (c.name.clone(), c.expected.clone())).collect::<Vec<_>>());
}
```
`graph_api.rs`:
```rust
#[test]
fn a_verse_that_names_hazor_serves_an_anchor_over_the_name() {
    // Arrange
    let app = compiled_app();
    // Act
    let body = get_json(&app, "/api/text?ref=JOS.11.1&n=1");
    // Assert
    let text = body["units"][0]["text"].as_str().unwrap();
    let hazor = body["units"][0]["anchors"].as_array().unwrap().iter().find(|a| a["node"]["id"] == "hazor_1").unwrap();
    assert_eq!(&text[hazor["start"].as_u64().unwrap() as usize..hazor["end"].as_u64().unwrap() as usize], "Hazor");
    assert_eq!(hazor["kind"], "mentions");
}
```

- [ ] **Step 2: Run** — compile error / no `anchors`.

- [ ] **Step 3: Implement** — port `PlaceMentions.Scan`'s rules exactly (letter-boundary matching, longest-first overlap resolution, ties to Place — `client/Explore/PlaceMentions.cs` is the source; read it line by line): `scan(text, places: &[(id, name)], persons: &[(id, name)]) -> Vec<Segment { start, end, kind, id }]` over characters, then `locate` maps each segment's character range to the `kjv` token ordinals it covers. A mention with no occurrence keeps `span: None` and is counted in the compile report (`stats.mentions_without_span`). `text_window`: for each unit, page its `mentions` group (all of it — a verse has at most tens), join each edge's `Mentions` row span through the unit's tokens to `char_start`/`char_end`, and emit `Anchor { start, end, kind: Mentions, node }`.

- [ ] **Step 4: Run** — vectors green; `graph_api` green; `cargo run -p atlas-contract --bin export_contract` (vectors) and fixtures regenerated. **Commit**: `git commit -m "graph: mention spans computed once in ETL in the KJV token layer; text windows serve anchors (P3, P4)"`.

---

### Task 8: Concord citations become `cites` edges with spans

**Files:**
- Create: `atlas-graph/src/citations.rs` (port of `client/Explore/ScriptureRefScan.cs`: the 60-entry alias table and the grammar), `atlas-graph/tests/citation_vectors.rs`
- Modify: `graph-types/src/sections.rs` (`CrossRefs` family added to the Concord section with an id split by `from` corpus), `atlas-graph/src/concord_adapter.rs` (the pass), `atlas-contract/src/document.rs` (`citation_grammar_json()`), `atlas-contract/src/graph.rs` (`text_window` emits `cites` anchors for Concord units)
- Test: `atlas-contract/tests/graph_api.rs`

- [ ] **Step 1: Write the failing tests** — `citation_vectors.rs` replays `vectors/citation-grammar.json` (cases: text → `[(start, length, sref)]`, built from `ScriptureRefScan`'s alias table: one case per alias plus the range/first-verse cases its `Scan` handles); `graph_api.rs`: `a_small_catechism_paragraph_that_cites_scripture_serves_cites_anchors` over a paragraph the grounding knows cites a verse (pick one from `/api/text?ref=BoC 7.2.1&n=20&corpus=concord` whose text contains a citation like "Matt. 5:3"; assert the anchor's `kind == "cites"` and the substring under `[start..end]` is the citation text).

- [ ] **Step 2: Run** — compile error.

- [ ] **Step 3: Implement** — `citations::scan(text, canon) -> Vec<Citation { start, end, span: TextSpan }>` (a range citation yields the whole range, fixing the client's first-verse concession); the Concord pass emits `CrossRef { from: TextLocus(unit, TokenSpan over the unit's tokens — Concord units get the same tokenizer in Task 6's writer, layer `"concord"`), to: first verse, to_last: last verse, target_display, votes: 0, provenance: "concord-citations" }` rows; `CrossRefs` joins the Concord section (split by `from.at` corpus, as `CanonSuccession` was split in FOCUS-0). `text_window` (Concord branch) emits `Anchor { kind: Cites, node: the verse's NodeRef }` per citation.

- [ ] **Step 4: Run** — green; documents/fixtures regenerated; AGC minor for the new edges. **Commit**: `git commit -m "graph: scripture citations in the Book of Concord are cites edges with spans, detected once in ETL (P5)"`.

Kretzmann commentary prose is **not** covered here: `CommentaryItem` nodes carry prose, not text units with token layers; their citations stay client-anchored until FOCUS-7 decides how commentary prose is modelled. Record this in the ledger as a residual, not a gap.

---

### Task 9: `/api/contract` stops advertising an AQC version

**Files:**
- Modify: `atlas-contract/src/wire/meta.rs` (`Contract { manifest_schema, section_schema_version }`), `atlas-contract/src/meta.rs` (delete `MIN/MAX_SUPPORTED_VERSION`), `contracts/atlas-query-contract/features/versioning.feature` (scenario 1 keeps the shape check, loses the `advertises` line; its header comment rewritten to one line), `atlas-contract/tests/aqc_cucumber.rs` (delete the `satisfies` mirror and the advertises/accepts/rejects steps), `atlas-contract/tests/contract_api.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_contract_declares_only_the_schema_versions_it_was_built_with() {
    // Arrange
    let app = app();
    // Act
    let body = get_json(&app, "/api/contract");
    // Assert
    assert_eq!(body, serde_json::json!({ "manifest_schema": 1, "section_schema_version": 16 }));
}
```

- [ ] **Step 2: Run** — fail (`min_version`/`max_version` present). **Step 3: Implement** the deletions. **Step 4: Run** — `cargo test -p atlas-contract --test contract_api --test aqc_cucumber` green; the semver gate classifies AQC as major (already bumped in Task 3). **Commit**: `git commit -m "contract: /api/contract declares schema versions only; the AQC version range is gone (P8)"`.

---

### Task 10: The client stops re-deriving

**Files:**
- Delete: `client/YearText.cs`, `client.Tests/YearTextTests.cs`, `tests/ux/lib/years.ts`, `client/Explore/Versification.cs`, `client/Explore/CanonRef.cs`, `client/Explore/PlaceMentions.cs`, `client.Tests/PlaceMentionsTests.cs`, `client.Tests/AcctCoalesceTests.cs`
- Create: `client/YearInput.cs`, `client.Tests/YearInputTests.cs`
- Modify: every consumer listed in the CONTRACT-2 grounding §A (`PlaceCard`, `PlaceEventsList`, `TimeSlider`, `PersonNode`, `PlaceNode`, `PolityDeltaNode`, `PopoverSectionProviders`, `TimeAndPlaceNode`, `YearNode`, `World.razor`, `ArrowNav`, `PassageBlock`, `PassageList`, `PassageGrouping`, `ExplorationDescriptor`/`LegacyNodes`, `MentionScan.razor`, `ScriptureRefText.razor` for Concord), `tests/ux/lib/canon.ts` (ref construction removed), the 13 Playwright specs importing `years.ts`

- [ ] **Step 1: Write the failing test**

`client.Tests/YearInputTests.cs`:
```csharp
using System.Text.Json;

namespace BibleAtlas.Client.Tests;

public sealed class YearInputTests
{
    [Fact]
    public void Every_served_label_parses_back_to_its_value()
    {
        // Arrange
        using var vectors = JsonDocument.Parse(File.ReadAllText(Path.Combine(ConformanceTests.RepoRoot, "contracts", "atlas-query-contract", "vectors", "year-labels.json")));
        var years = vectors.RootElement.GetProperty("years").EnumerateArray().Select(y => (y.GetProperty("label").GetString()!, y.GetProperty("value").GetInt32())).ToArray();
        var ranges = vectors.RootElement.GetProperty("ranges").EnumerateArray().Select(r => (r.GetProperty("label").GetString()!, r.GetProperty("from").GetInt32(), r.GetProperty("to").GetInt32())).ToArray();
        // Act
        var parsedYears = years.Select(y => YearInput.TryParse(y.Item1, out var f, out var t) ? (y.Item1, f) : (y.Item1, int.MinValue)).ToArray();
        var parsedRanges = ranges.Select(r => YearInput.TryParse(r.Item1, out var f, out var t) ? (r.Item1, f, t) : (r.Item1, int.MinValue, int.MinValue)).ToArray();
        // Assert
        Assert.Equal(years, parsedYears);
        Assert.Equal(ranges, parsedRanges);
    }

    [Fact]
    public void Year_zero_and_inverted_ranges_are_rejected()
    {
        // Arrange
        var inputs = new[] { "AD 0", "0 BC", "1400 – 1450 BC" };
        // Act
        var results = inputs.Select(i => YearInput.TryParse(i, out _, out _)).ToArray();
        // Assert
        Assert.Equal(new[] { false, false, false }, results);
    }
}
```

- [ ] **Step 2: Implement `YearInput`** — the inverse of the served labels (`"{n} BC"`, `"AD {n}"`, en dash or `" - "` separators, shared-era ranges), the only formatting-adjacent code left on the client; every display site reads the served `label`.

- [ ] **Step 3: Delete and retarget** — `git rm` the files above; follow the compiler: `Versification` users read served `loci`/`count` (`PassageBlock` renders the served runs and loses `TrueRunsOf`/`StitchRuns`); `CanonRef` users read `locus`/`TextSpan` integers; `MentionScan.razor` renders `TextUnit.anchors` (segments from served `start`/`end`) instead of scanning; `ScriptureRefText.razor` renders `cites` anchors for Concord text and keeps the client scan **only** for Kretzmann prose (Task 8's residual) — that residual keeps `ScriptureRefScan.cs` alive, scoped to Kretzmann, with a `why` comment naming FOCUS-7; Playwright specs assert served labels (`era.window.label`, `scene.window.label`) and `canon.ts` builds refs from `/api/contents` loci.

- [ ] **Step 4: Run** — `dotnet test client.Tests && dotnet test client.ContractTests && npx playwright test tests/ux` → green; Stryker over `client/YearInput.cs` at 100%. **Commit**: `git commit -m "client: never re-derives what the graph knows -- YearText, Versification, CanonRef, PlaceMentions gone; labels, loci and anchors are read as served"`.

---

### Task 11: Gates and push

- [ ] **Step 1** — From `server/`: the standing block's three commands; `bash ../scripts/timing-gates.sh` (all; the token layer and mention/citation passes change compile time — record gates 1, 8, 9 against their ceilings and, if a ceiling is exceeded, stop and report rather than raise it); `bash ../scripts/contract-gate.sh`; `bash ../scripts/contract-semver-gate.sh` (AQC major, AGC minor).
- [ ] **Step 2** — `cargo mutants -f atlas-contract/src/wire/time.rs -f atlas-contract/src/wire/locus.rs -f atlas-graph/src/kjv_tokens.rs -f atlas-graph/src/mention_spans.rs -f atlas-graph/src/citations.rs -f atlas-graph/src/runs.rs` → 100% or equivalents recorded in `mutants.toml` with reasons.
- [ ] **Step 3** — `git push origin worktree-bible-atlas-m1`.

---

## Self-review against the spec

- **Coverage:** P1 (Tasks 1, 3, 10), P2 (Tasks 2, 3, 10), P3/P4 (Tasks 6, 7, 10), P5 (Task 8, Concord; Kretzmann residual named), P6 (Task 4), P7 (Task 5), P8 (Task 9), P9 (Task 3's version bump and re-pins); §7 deletions (Task 10); §8 rulings taken as defaults (R-C2-1 token layer, R-C2-2 citations inside, R-C2-3 objects replace integers, R-C2-4 flat details, R-C2-5 `ref` kept, R-C2-6 version range dropped, R-C2-7 unlocatable mentions stay verse-level and are counted).
- **Placeholders:** the few values the tests take from the running server (Moses' birth year, the attestation loci literal, a citing paragraph) are named as "read once from the API and write the true value" — the test documents the truth.
- **Type consistency:** `TextSpan` (Task 2) is what `DateClaim.verses` (Task 1), `EdgeEntry.loci` (Task 4) and `Citation.span` (Task 8) carry; `Year::of`/`TimeRange::of` (Task 1) are what Task 3's fills and Task 5's details call; `Anchor` (Task 2) is what Tasks 7 and 8 emit and Task 10 renders.
