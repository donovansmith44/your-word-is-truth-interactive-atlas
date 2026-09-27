# CONTRACT-2: the client never re-derives what the graph knows — design

**Status:** SIGNED OFF 2026-09-27 — owner: "yes to all" on §8 (every
default ruled in). Plan: `docs/superpowers/plans/2026-09-27-contract2-pushdown.md`.
**Governed by:** `docs/PRINCIPLES.md` (rule 6 is this batch's whole reason).
**Depends on:** CONTRACT-1 (the `atlas-contract` crate, `openapi.yaml`,
build-time generation). **Consumed by:** FOCUS-2 onward (anchors, loci and
card details are what `Presentation` renders). Order of execution:
CONTRACT-1 → FOCUS-0 → **CONTRACT-2** → FOCUS-1…9.
**Grounding:** the 2026-09-27 pass over client, server, ETL and the SQLite
sections; every count below is from it.

## 1. Problem

Five pieces of client code compute facts the graph already holds, or that
the server could hold once:

| Client unit | Re-derives | Size of the duplication |
|---|---|---|
| `YearText.cs` + `tests/ux/lib/years.ts` | BC/AD labels, ranges, "c." claims from 13 integer year fields on 9 wire structs | one formatter in two languages, 16 C# call sites, 17 Playwright specs |
| `Versification.cs` + `tests/ux/lib/canon.ts` | chapter lengths, from `/api/books`, to clamp and stitch attestation verse runs (`PassageBlock.TrueRunsOf/StitchRuns`) | 30 tests pinning arithmetic the server could ship as coalesced spans |
| `CanonRef.cs` | the ref grammar (`GEN.1.1`, `GEN.1.3-5`) by string parsing | 35 call sites; the server has `ScriptureRef::parse` and typed `VerseRef`/`ConcordRef` |
| `ScriptureRefScan.cs` | scripture citations inside Concord/Kretzmann prose, via a 60-entry hand alias table and a TOC regex | citations are not edges in the graph today |
| `PlaceMentions.cs` / `MentionScan.razor` | where a place or person name sits inside a verse, by name search | every `mentions` row is verse-level (`locus_start/end` NULL in all rows) |

And the graph path is thinner than the legacy path: `EdgeEntry` exposes no
`votes`/`narrative` although `edge_index` stores both; attestation loci,
witness notes, event dates, place coordinates and catechism prose ride only
on the legacy detail endpoints FOCUS retires. Each of those is a fact the
server has and the wire withholds.

## 2. Decisions

| # | Decision |
|---|---|
| P1 | **Years travel with their label.** `Year { value, label }` replaces every bare year integer; `TimeRange` and `DateClaim` carry a composed label. The formatting rules move to Rust once; Rust emits golden vectors; the one remaining client parser (typed input in the time slider) is tested against them. |
| P2 | **Loci are structured on the wire.** A `TextRef` (Bible or Concord unit, with integers) rides *beside* the existing `ref` string wherever a locus is served; ranges are `TextSpan { from, to }`. Strings stay as ids and test ids; nothing parses them any more. |
| P3 | **Anchors are served, not scanned.** A `TextUnit` carries `anchors: [Anchor { start, end, kind, node }]` — character offsets into its own `text`, one per link whose locus falls inside it (mentions, and, after P5, citations). The precedent is `words_of_christ`, already served as character offsets. |
| P4 | **Mention spans are computed once, in ETL, in the KJV layer.** The lexicon's token table has no English layer and no character offsets, so the KJV gains a token layer with character offsets, and `mentions.locus_start/end` become token spans in that layer (`TokenSpan` stays token-ordinal, uniform across layers; the server converts to characters when serving). The name-search algorithm is `PlaceMentions.cs`'s, ported with its 11 tests as vectors. |
| P5 | **Citations become `cites` edges.** ETL detects scripture citations in Concord and Kretzmann text units (porting `ScriptureRefScan`'s alias table and grammar, once) and writes `cites` rows with spans. Sequenced last in the batch (§8 R-C2-2). |
| P6 | **Edge entries carry their metadata.** `EdgeEntry { edge, node, votes?, narrative?, loci?, note? }`. Coalesced attestation runs are computed server-side and served as `loci`, so `PassageBlock`'s arithmetic has nothing left to do. |
| P7 | **Kind detail rides on the card, flat.** `NodeCard` gains `event?`, `place?`, `catechism?`, `book?` beside the existing `person?` — the precedent, and the NSwag-friendly shape (no discriminated unions). `TextUnit` gains `heading?` (pericope) and `locus`. |
| P8 | **`/api/contract` stops advertising an AQC version.** `Contract { manifest_schema, section_schema_version }`. The AQC `VERSION` file and the semver gate keep working on directory diffs, which is all they ever read. |
| P9 | **This batch changes the wire.** AQC bumps **major** (P1 replaces integers with objects; P8 removes fields); AGC bumps minor (additive). Fixtures regenerate through the existing exporters; the standing byte-identity proofs are replaced, for this batch only, by the regenerated pacts plus Playwright. |

## 3. Wire types (Rust, in `atlas-contract/src/wire/`; the generated C# follows by name)

```rust
// wire/time.rs  (replaces atlas_core::time::TimeRange on the wire; the core type stays for computation)
pub struct Year      { pub value: i32, pub label: String }                       // "1405 BC" | "AD 33"
pub struct TimeRange { pub from: Year, pub to: Year, pub label: String }         // "1450 – 1400 BC" (en dash); from == to → from.label
pub struct DateClaim { pub when: TimeRange, pub label: String, pub verses: Vec<TextSpan>, pub note: Option<String> }   // label = "c. " + when.label iff note is Some

// wire/locus.rs
pub enum TextRef {                                                               // externally tagged: {"bible":{…}} | {"concord":{…}}
    Bible   { book: String, chapter: u16, verse: u16 },                          // book = canon code, "GEN"
    Concord { part: u8, article: u16, paragraph: u16 },
}
pub struct TextSpan { pub from: TextRef, pub to: TextRef }                        // inclusive; from == to for a single unit
pub struct Anchor   { pub start: usize, pub end: usize, pub kind: EdgeKind, pub node: NodeRef }   // character offsets into the owning TextUnit.text

// wire/graph.rs  (changed fields only)
pub struct TextUnit {
    pub r#ref: String,                 // unchanged: "GEN.1.1" / "BoC 7.2.1"
    pub locus: TextRef,                // NEW
    pub text: String,
    pub words_of_christ: Vec<WordsOfChristSpan>,
    pub anchors: Vec<Anchor>,          // NEW: mentions (P4), citations (P5)
    pub heading: Option<Heading>,      // NEW: pericope start, from the chapter's attestation structure
    pub edge_summary: Vec<EdgeSummaryEntry>,
}
pub struct Heading   { pub event: NodeRef, pub title: String, pub kind: String, pub is_continuation: bool }
pub struct EdgeEntry {
    pub edge: String,
    pub node: NodeRef,
    pub votes: Option<i32>,            // NEW: edge_index.meta_votes (cites)
    pub narrative: Option<String>,     // NEW: edge_index.meta_narrative (follows-in on events)
    pub loci: Option<Vec<TextSpan>>,   // NEW: the attestation's coalesced runs (attested-in / attests)
    pub note: Option<String>,          // NEW: the attestation's ref_note
}
pub struct NodeCard {
    pub id: String, pub kind: NodeKind, pub label: String, pub provenance: String,
    pub edge_summary: Vec<EdgeSummaryEntry>, pub version: String,
    pub description: Option<String>,
    pub person:    Option<PersonLife>,     // existing
    pub event:     Option<EventDetail>,    // NEW
    pub place:     Option<PlaceDetail>,    // NEW
    pub catechism: Option<CatechismDetail>,// NEW
    pub book:      Option<BookDetail>,     // NEW
}
pub struct PersonLife { pub gender: Option<String>, pub birth: Option<Year>, pub death: Option<Year>, pub first: Option<Year>, pub last: Option<Year>, pub eternal: bool, pub eternal_grounds: Vec<String>, pub also_called: Vec<String> }
pub struct EventDetail     { pub kind: String, pub when: Option<TimeRange>, pub robertson_section: Option<String>, pub acts_section: Option<String>, pub atlas_section: Option<String>, pub kjv_superscription: Option<String>, pub ref_note: Option<String> }
pub struct PlaceDetail     { pub lat: f64, pub lon: f64, pub display_name: String, pub canonical_name: Option<String>, pub blurb: Option<String>, pub established: Option<DateClaim>, pub destroyed: Option<DateClaim> }
pub struct CatechismDetail { pub part_title: String, pub text: Option<String>, pub explanation_heading: String, pub explanation: String, pub where_written: Option<String> }
pub struct BookDetail      { pub author: Option<String>, pub write_place: Option<String>, pub written: Option<TimeRange> }

// wire/contents.rs — ContentsRoot / ContentsChild gain `pub locus: TextRef` beside `ref`
// wire/map.rs — Scene.window: Option<TimeRange>; SceneEvent.when: TimeRange; ScenePlace/QuietPlace.existence: Option<TimeRange> (replaces the two Option<i32>); Polity.reign: TimeRange (replaces from/to)
// wire/meta.rs
pub struct Contract { pub manifest_schema: u32, pub section_schema_version: u32 }
```

Every year integer in §C of the grounding (13 fields, 9 structs) becomes a
`Year`/`TimeRange` per the lines above; the legacy detail structs
(`VerseDetail`, `EventDetail`-the-route, `PlaceDetail`-the-route,
`CatechismItem`-the-route, `Chapter`, `KretzmannChapter`) are **not**
changed here — FOCUS retires them route by route, and until then they keep
serving integers so that their pins hold.

### 3.1 Formatting, once — `atlas-contract/src/wire/time.rs`

```rust
impl Year      { pub fn of(value: i32) -> Year; }                    // label: value < 0 → "{-value} BC", else "AD {value}"
impl TimeRange { pub fn of(from: i32, to: i32) -> TimeRange; }       // label: from == to → from.label; else "{from} – {to}" with the era named once when both share it ("1450 – 1400 BC"), twice otherwise ("5 BC – AD 30")
impl DateClaim { pub fn of(when: TimeRange, verses: Vec<TextSpan>, note: Option<String>) -> DateClaim; }   // label: note.is_some() → "c. {when.label}", else when.label
```
These are the rules `YearText.cs` and `years.ts` implement today, read off
their tests; `contracts/atlas-query-contract/vectors/year-labels.json` is
emitted by the exporter (`generated_files()` gains it) from a fixed value
list including `-4004, -1, 1, 33, 100, 2000` and ranges `(-1450,-1400)`,
`(-5,30)`, `(33,33)`, and is what the client and Playwright assert against.

### 3.2 Golden vectors and the one parser that stays

`client/YearInput.cs` — `public static bool TryParse(string text, out int from, out int to)` — parses what a user types into the time slider ("1405 BC", "AD 33", "1450 – 1400 BC", "1450-1400 BC"). Input handling, not re-derivation; `YearInputTests` iterates `year-labels.json` and asserts `TryParse(label) == value` for every vector.

## 4. ETL and storage

| Change | Where | Law / test |
|---|---|---|
| KJV token layer with character offsets: `token` rows for layer `kjv` with `char_start`, `char_end` (new nullable columns; NULL for source-language layers) | lexicon section; `SECTION_SCHEMA_VERSION` bump | every KJV verse tokenises to spans that concatenate (with the original whitespace) back to the verse text — `kjv_tokens_reassemble_the_verse` over all 31,102 verses |
| Mention spans: for every `mentions` row, the name occurrences of the entity (place names + aliases, person label + `also_called`) located by `PlaceMentions.cs`'s rule (letter-boundary match, longest-first, ties → Place) and stored as token spans in the `kjv` layer; a mention with no occurrence keeps `span: None` (verse-level) and is counted | `atlas-etl` mentions pass | the 11 `PlaceMentionsTests` cases become `contracts/atlas-query-contract/vectors/mention-spans.json`; a Rust test replays them |
| Citations in prose (P5): Concord and Kretzmann text units scanned once with the ported alias grammar; each hit becomes a `cites` row `unit —cites→ verse` with a span; range citations resolve to the whole range (`TextSpan`), fixing the "collapses to first verse" concession | `atlas-etl` citations pass; `cites` row family already exists | the alias table's 60 entries become `vectors/citation-grammar.json` (text → expected spans); replayed in Rust |
| Coalesced attestation runs (P6): `attests` loci grouped per (event, book) into contiguous runs against the canon's chapter lengths, once | `atlas-graph` when building `attested-in` / `attests` entries | the 18 `AcctCoalesceTests` cases become `vectors/attestation-runs.json` |
| Pericope headings on text units (P7 `TextUnit.heading`) | `atlas-graph` reading window, from the chapter's attestation structure the legacy `Chapter` handler already computes (`HeadingOut`) | `reading_window_headings_equal_legacy_chapter_headings` for every chapter, run once before the legacy route retires |

## 5. Server

- `wire/time.rs`, `wire/locus.rs` as in §3; conversions at the adapter boundary (`atlas_core::time::TimeRange` → `wire::TimeRange::of`, `VerseRef`/`ConcordRef` → `TextRef`).
- `graph.rs`: `node_card` fills the four detail slots from the same sources the legacy handlers use today (`event` from `EventDetailOut`'s inputs, `place` from `PlaceDetailOut`'s, `catechism` from `CatechismItemOut`'s, `book` from `BookMetaOut`'s); `node_edges` fills `votes`/`narrative` from `edge_index` (already selected in `edges_inner`) and `loci`/`note` for attestation kinds; `text_window` fills `locus`, `anchors`, `heading`.
- `meta.rs`: `Contract` loses `min_version`/`max_version`; `MIN/MAX_SUPPORTED_VERSION` deleted; `versioning.feature` scenario 1 keeps its shape check and loses the literal.
- `document.rs`: `generated_files()` gains the three vector files under `contracts/atlas-query-contract/vectors/`.
- AQC `VERSION` → `1.0.0`; CHANGELOG lists P1, P6, P7, P8 as breaking/additive respectively; AGC `VERSION` minor with the new fixtures.

## 6. Client

- Generated records pick up every new field with no hand-written change (CONTRACT-1's build step).
- `MentionScan.razor` renders served `TextUnit.anchors` instead of scanning; `ScriptureRefText.razor` renders `cites` anchors; `MentionText.ComputeRuns` unchanged.
- Every `YearText.Format*` call site displays the served `label`; `TimeSlider`'s readout shows `Scene.window.label` and parses input with `YearInput.TryParse`.
- Every `CanonRef` call site reads `locus`/`TextSpan` integers; `ExplorationDescriptor`'s Passage reconstruction uses the served span.
- Playwright: specs import served labels/loci from the fixtures they already load; `years.ts` and the ref-building half of `canon.ts` go.

## 7. What dies

| Dies | Because | Proof |
|---|---|---|
| `client/YearText.cs`, `client.Tests/YearTextTests.cs`, `tests/ux/lib/years.ts` | P1 | compiler; `YearInputTests` over the vectors; Playwright green |
| `client/Explore/Versification.cs`, `PassageBlock.TrueRunsOf`/`StitchRuns` and `AcctCoalesceTests` (18) | P6 — runs are served | compiler; `vectors/attestation-runs.json` replayed server-side |
| `client/Explore/CanonRef.cs` (35 call sites) | P2 | compiler |
| `client/Explore/ScriptureRefScan.cs` and its `CitationAliases` | P5 | compiler; `vectors/citation-grammar.json` |
| `client/Explore/PlaceMentions.cs`, `PlaceMentionsTests` (11) | P4 | compiler; `vectors/mention-spans.json` |
| `tests/ux/lib/canon.ts` ref-construction (chapter-length arbitraries stay as test tooling over `/api/contents`) | P2 | the specs that used it |
| `MIN_SUPPORTED_VERSION`/`MAX_SUPPORTED_VERSION`, `versioning.feature` scenario 1's literal | P8 | `contract_api.rs`; the Rust cucumber |
| `atlas_core::time::TimeRange` on the wire (the type stays for computation) | P1 | AQC major; regenerated pacts |
| the client's own reading of `edge_index` metadata via legacy routes (`CrossRefOut.votes`, `EventWitnessOut.verse_groups/ref_note`) | P6 — the graph path carries them | FOCUS retires the routes; this batch makes it possible |

Not dying here, deliberately: the legacy detail routes (FOCUS-2…7, one at a
time), `SliderScale.cs` (UI math), `MentionText.ComputeRuns` (consumes served
spans).

## 8. Rulings (owner, 2026-09-27: "yes to all" — each default below is ruled)

- **R-C2-1 — Mention spans as a KJV token layer with character offsets** (P4) vs new character-offset columns on `mentions` only. *Default: the token layer.* It keeps `TokenSpan` one thing across every layer, gives the lexicon the English layer alignment will want, and costs one section-schema bump either way.
- **R-C2-2 — Citations in ETL (P5) inside this batch**, sequenced last, vs a batch of its own. *Default: inside.* Same pattern (compute once, serve spans), same vectors discipline, and FOCUS-2's `Text` presentation wants both anchor kinds at once.
- **R-C2-3 — `Year { value, label }` replacing integers (AQC major)** vs additive `*_label` fields (minor). *Default: replace.* One consumer, pre-1.0, and thirteen sibling fields is exactly the clutter the label object avoids.
- **R-C2-4 — Card detail as flat optionals** (`event?`, `place?`, …, the `person?` precedent) vs a tagged union. *Default: flat.* The generator produces plain records; a union would need a discriminator story in NSwag for no consumer benefit.
- **R-C2-5 — Keep `ref` strings beside `locus` objects.** *Default: keep both.* The string is the id in node ids and test ids; the object is what code reads.
- **R-C2-6 — `/api/contract` drops the AQC version range** (P8). *Default: yes.* Nothing reads it after CONTRACT-1b; the semver gate never did.
- **R-C2-7 — Mentions with no locatable name occurrence** stay verse-level and are counted in a compile report. *Default: yes* — silently inventing a span would be a lie; a verse-level anchor renders as today.
