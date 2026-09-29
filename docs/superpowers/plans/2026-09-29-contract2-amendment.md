# CONTRACT-2 (pushdown) — Plan Amendment: words are the base of the text; one regeneration at the close

> **For agentic workers:** this amends `docs/superpowers/plans/2026-09-27-contract2-pushdown.md` task by task. Read the plan, then this file; where this file speaks, it wins. Steps use checkbox (`- [ ]`) syntax. The rulings it carries are FINAL and are cited by id — ledger `.superpowers/sdd/2026-09-29-contract2-pushdown/progress.md` (R-C2-P1, D1–D22, R-C2-W1…W4) and `prep.md` §4 beside it (the defect texts D1–D22 answer). This file rules nothing new. Where a ruling leaves a gap that forces a guess, the gap is an **OPEN** item below; the text written under it is the amendment's proposal and is marked **(pending OPEN-n)**.

**Base (PRINCIPLES 22): `7b6b2c3`** — FOCUS-0's close, pushed. Every gate in this batch takes `--base 7b6b2c3` (the semver gate positionally). Start state: AQC `0.10.0`, AGC `0.14.0`, `SECTION_SCHEMA_VERSION` 15, artifact root `31e118e8e6d47276ebeec3ed23d53217`, `version_root_regression` `ce9d653b597e6893a25a5264c60ff306`. Mutation base unchanged: `13111dd` (`.superpowers/MUTATION-GATE-DEBT.md`).

**Goal (amended):** the plan's goal, and: the KJV's English words are the base of the text — verses are containers over words; the compiled artifact stores the KJV once, as its words and the text between them; the wire addresses words; Kretzmann's comments anchor on word spans where the quoted phrase aligns exactly.

---

## OPEN — for the controller (each names the dispatch it blocks)

- **OPEN-1 — The schema wall and the landing order of the ETL lane.** *Blocks: landing T6 on the branch (not T6's dispatch).* T6 moves `SECTION_SCHEMA_VERSION` 15 → 16; the reader refuses a 15-artifact under a 16-binary (`sqlite/snapshot.rs:125`, "user_version 15 unsupported"), and R-C2-P1 rebuilds `data/compiled` once, in wave 5. If T6 is cherry-picked at wave 1's end (as prep §3 wave 2 assumes: "T7a … after T6 cherry-picked"), every `compiled_app()` test T3, T4 and T5 write in `graph_api.rs` is red through waves 2–4 for a reason not their own, and the wire lane cannot verify its whole-body tests. **Proposal:** the ETL lane lands as one stack — T6 → T7a → T8a → T8c each commit on its `wip/` branch, each later worktree created at the previous task's `wip` commit, and the controller cherry-picks the stack onto the branch in wave 5 immediately before the one rebuild. Alternative: F0-16's owed test-side `ATLAS_COMPILED_DIR` resolver, built first.
- **OPEN-2 — Which gate R-C2-W4 names.** *Blocks: nothing (T6 measures all four); rules the W4 decision.* W4 says "Gate 8 (`/api/text`)". In `scripts/timing-gates.sh` gate 8 is `sqlite_real_data` (the real graph written to sections and admitted; 797.6 s at FOCUS-0's close), gate 6 is `text_window_completes_within_smoke_threshold` and gate 7 `chapter_window_…` (the `/api/text` path, 30 ms / 50 ms), gate 9 `sections_startup` (served startup, 826.3 ms). **Proposal:** T6 and T11 measure gates 6, 7, 8 and 9 before and after; the composition-vs-cache decision reads gates 6, 7 and 9 (serving); gate 8 is D13's placement measure (the write). Also: wave 1 is not a quiet box (the owner's app and the primary's builds run), so T6's pair is a *relative* before/after under one load; the ceilings are judged at T11 on a quiet box.
- **OPEN-3 — Red letter as word spans changes what the reader colours.** *Blocks: T6's red-letter step.* Measured read-only on the served KJV section (2026-09-29): 2,063 red-letter spans; 2 start at an opening `(` before their first word; 2,056 end *after* the closing punctuation of their last word (`.` 1,556, `?` 238, `:` 143, `;` 52, `,` 45, `!` 20, `:)`, `.)`); 7 end exactly at a word. A word span rendered as its words only would drop the red from 2,056 closing marks. The rule "a quotation runs from the opening brackets before its first word through the closing punctuation after its last word, never into the following space" reproduces **2,060 of 2,063** served spans exactly; the three it changes are **MAT.11.27, MAT.13.17, LUK.10.22**, where today's span stops before the verse's final period (the period turns red). **Proposal:** that rule (`Wording::quotation_range`), the three verses disclosed in the CHANGELOG; mention anchors (T7b) use the words-only range (`Wording::words_range`) so that `Hazor,` anchors `Hazor`.
- **OPEN-4 — Two signed §3 signatures fail the Haskell bar (PRINCIPLES 14b vs 12).** *Blocks: T1, T2.* (a) `Year::of(i32) -> Year` and `TimeRange::of(i32, i32) -> TimeRange` are partial: there is no year zero (`atlas_core::time::TimeRange::new` refuses it and inverted ranges) and `YearText.Format(0)` = `"AD 0"` is the bug the client carries today. **Proposal:** `Year::of(i32) -> Result<Year, CoreError>` (`CoreError::ZeroYear`) and `TimeRange::of(time::TimeRange) -> TimeRange` (total over the validated core range). (b) `TextRef::Bible { book: String }` is a closed 66-value vocabulary as a `String`. **Proposal:** keep `String` — it is only ever *constructed* from the canon table (`atlas_core::canon::BOOKS[i].code`), never parsed, and a 66-variant enum in the document buys the client nothing; recorded as a deliberate exception. Controller rules (a) and (b) separately; both change owner-seen signatures if taken.
- **OPEN-5 — A `TextPoint`'s word has no layer.** *Blocks: T2.* R-C2-W2 gives `TextPoint { unit, word: Option<u16> }`; the graph's spans are layer-tagged (`TokenSpan { layer, start, end }`), and other layers exist (`Occurs` rows index `greek_textus_receptus`). **Proposal:** a wire word ordinal always indexes its unit's corpus *base* layer (`kjv` for Bible units per R-C2-W1; the Concord's own layer for Concord units), and the conversion from a range whose span names any other layer is a typed refusal, `Err(ForeignLayer { layer })` — never a silent reinterpretation.
- **OPEN-6 — What "serving reads it" means for T8c.** *Blocks: T8c's serving half (wave 5).* R-C2-W3: "comments_on gains the word span; serving reads it". Two surfaces could read it: (a) `EdgeEntry.loci` on `comments-on` / `commented-on-by` entries (one `TextSpan`, words named where aligned) — the P6 pattern; (b) `TextUnit.anchors` of kind `commented-on-by` over the commented words in the reader. **Proposal:** (a) only, in the wave-5 serving dispatch; (b) is FOCUS-7's presentation.
- **OPEN-7 — Does R-C2-W4 reach the Concord in T8a?** *Blocks: T8a.* W4 rules the KJV ("the compiled artifact stores the KJV once"). T8a gives Concord units a word layer (`concord_token`, D13) for citation spans. Either the Concord text is also stored only as words (its payload rendering dropped; PRINCIPLES 6), or it keeps its string and `concord_token` becomes a second store pinned to it by the lossless law (W4's "recorded cache" fallback). **Proposal:** the same as the KJV — words only, composition law over every Concord unit — measured by gates 6/7 in T8a.
- **OPEN-8 — "The layer is content-hashed as a whole" — where.** *Blocks: T6.* The Text-Fabric evaluation suggested a manifest-recorded layer hash plus a tokenizer version. **Proposal:** the Kjv section's logical hash *is* the layer's content hash — `kjv_token` joins the Kjv section's table order (`extra_tables_of(Kjv)`), so any change to any word moves that hash and the version root; no second hash, no tokenizer-version field (the rows are hashed, not the recipe).
- **OPEN-9 — "Aligns exactly" for T8c.** *Blocks: T8c.* The spike's 81.4 % is "naive normalised, exact, contiguous": word forms compared case-insensitively (our text is case-restored, Kretzmann's quotations are not), `’`/`'`/`–`/`-` inside a word folded, punctuation ignored (it lives between words), the match starting in the unit's marked verse. The ETL's KRETZ-ACCEPT-1 equivalence classes would recover more but are not "exact". **Proposal:** the spike's comparison, no equivalence classes; everything else stays verse-range and is counted.

## Readings this amendment takes (overturnable; not rulings)

- **RA-1 (W4 "== `kjv.json` byte-for-byte").** The words are the *case-restored* text (R-C2-W1 says so; KJV-CASE/KJV-CASE-2 change case only), so the law is the fidelity law's: composition equals `kjv.json` passed through the build's own case restoration — exactly `fidelity::check_kjv_fidelity`'s expected side (`fidelity.rs:112-131`). "The fidelity law moves onto the words" is taken literally: that function reads composed words.
- **RA-2 ("the stored verse-text column").** There is no text column (`verse` is `node_id, book, chapter, verse`, `ddl.rs:534-537`). The KJV text lives in the `kjv` entry of the Bible `TextUnit` payload's `renderings` map (`kjv_adapter.rs:98-106`), beside the six brain-fuel editions. W4 drops the `kjv` entry only; the other editions keep their strings.
- **RA-3 (no `strong`/`aligned` columns yet).** The evaluation suggested keeping them for the later eBible batch; PRINCIPLES 4 ("no kept for later") binds, so `kjv_token` carries exactly what composition needs. The eBible batch adds its columns with its data.
- **RA-4 (one spelling of a verse).** The spike's `pre`/`post` can spell one text two ways (a gap split between a word's `post` and the next word's `pre`). The amendment stores the gaps canonically: the text before the first word once per unit, and each word with the text after it. 71 KJV verses open with a non-word (`(For all these abominations …` LEV.18.27), so the unit slot is needed.
- **RA-5 (canonical spans).** R-C2-W2: "a whole-verse span is `word: None`". The same holds in storage: an endpoint `Locus` carries `span: Some` only when it covers part of its unit; a `LocusRange`'s `from.span` is the covered part of the first unit, `to.span` of the last; a single-unit span has `from == to`. The wire reads `from.span.start` and `to.span.end`.
- **RA-6 (the red-letter wire does not move).** `WordsOfChristSpan { start, end }` stays character offsets on the wire; storage becomes word spans and `GraphService` derives the served character index at load through `Wording::quotation_range`, so no `atlas-contract` source changes in T6.
- **RA-7 (words are domain).** `Word`/`Wording` and the tokenizer live in `graph-types` (the text model's crate); the port gains one required method, `GraphQuery::wording`. D15's "`token_offsets(verse, layer)` over the token table" becomes that method, and its "ONE char-offset conversion" becomes `Wording`'s two range functions.
- **RA-8 (D12 on `EdgeEntry`).** The Rust field stays a sum (the domain `EdgeMeta` is one) projected so that the JSON is the spec's two optional keys (`votes` / `narrative`); if utoipa cannot publish that shape as two optional properties, T4 falls back to two `Option`s and records why — that is D12's "if the spec shape allows".
- **RA-9 (T8c's two halves).** T8c is ETL (a wave-4 companion); its serving half joins the wave-5 serving dispatch with T7b/T8b, the R-C2-P1 a/b pattern.

---

## 1. The rulings in task terms

| Ruling | Tasks | What it changes |
|---|---|---|
| R-C2-P1 | all | DOC only per wire task (`export_contract`; `--check` green); FIX (AQC fixtures, pact, AGC fixtures), ROOT (one `data/compiled` rebuild, controller, wave 5), VER (VERSIONs, CHANGELOGs) once; every task declares its exact expected-red set; T7 → 7a/7b, T8 → 8a/8b, T10 → 10a/10b |
| R-C2-W1 | T6, T7a, T8a, T8c | the KJV word layer is the base of the text, built natively with the spike's rule on case-restored text; pin 790,892; a word is `(unit, layer, ord)`; no per-word hash; no global slot numbers |
| R-C2-W2 | T2 (then T1, T4, T5, T8a, T8c) | `TextPoint { unit, word: Option<u16> }`, `TextSpan { from: TextPoint, to: TextPoint }` |
| R-C2-W3 | T8c (new) | Kretzmann `comments_on` rows carry word spans where the quoted phrase aligns exactly; the rest stay verse-range, counted |
| R-C2-W4 | T6, T11 | the KJV is stored once as words and the text between them; the `kjv` rendering is dropped behind a byte-for-byte composition law; red letter (and later italics) are word-span attributes; gates measured before/after at T6 and again at T11 |
| D1 | T11 | AQC `0.10.0 → 0.11.0`; CHANGELOG "MAJOR class, MINOR bump under the 0.x rule" |
| D2 | T1, T10a | era named once in a shared-era range; vectors carry every `YearTextTests` case; `YearInput` accepts served, long, spaced and unspaced forms |
| D3, D4 | T1, T3 | labelled `Year`/`TimeRange` in atlas-core, re-exported as `atlas_contract::wire::{Year, TimeRange}`; the core computation `TimeRange` publishes as `YearSpan`; `atlas_core::data::Era` loses `ToSchema` when `wire::Era` serves |
| D5, D6 | T3, T5 | `ContentsChild.locus` = its first unit; `DateClaim` in `PlaceDetail` and `BookDetail.written` move to T5 |
| D7 | T3 | `/api/eras`, `/api/polities`: `window` / `reign` ADDED beside the integers `contracts/atlas-edge` consumes |
| D8, D9 | T4, T10b | attestation loci/note sources; only coalescing-semantics `AcctCoalesceTests` cases become vectors |
| D10, D11, D12 | T3, T4, T5 | legacy structs publish as `EventPage`/`PlacePage`; `TextUnit.heading` reuses `atlas_graph::heading::Heading`; `kind: EventKind`; typed ids (`EraId`, `PolityId`, `NarrativeId`), `votes: u32`, `DateClaim.event` accepted |
| D13 (revised) | T6, T8a | `kjv_token` in the **Kjv** section; `concord_token` in the Concord section |
| D14 | T11 | subsumed by R-C2-P1 |
| D15 | T7b | one accessor for a verse's mention spans; one char conversion (now `Wording::words_range`, RA-7) |
| D16 (as amended by W3) | T8c, T10b | Kretzmann's comments DO anchor on words (T8c); citations *inside* Kretzmann prose stay FOCUS-7; `ScriptureRefScan.cs` narrowed to Kretzmann prose with a one-line why naming FOCUS-7 |
| D17 | T9 | one `include_str!` of `VERSION`; the drift law dies; AGC `contract` projection becomes `manifest_schema`/`section_schema_version` |
| D18 | T3…T10a | the client build is in the declared red set from T3 until 10a lands; Playwright only at T11, after 10b |
| D19 | T10, T11 | no per-task mutation (T10's Stryker line and T11's `cargo mutants -f …` line are deleted); at T11 per the debt file |
| D20 | T2, T6, T7b | offsets are Unicode-scalar; the token law is the lossless one |
| D21, D22 | T7a, T3, T5 | one mention row per occurrence, Place and Person searched; `existence_from/existence_to: Option<Year>` |

## 2. Batch shape (R-C2-P1)

**Critical sections (PRINCIPLES 21).** DOC — `export_contract` (openapi, aqc.schema, graph-vocabulary, vectors): the wave's PRIMARY only. FIX, VER — T11 only. ROOT — the controller's one rebuild (wave 5) and T11's pins. No task enters `relations!` (every relation this batch writes exists).

**Expected-red discipline (F0-16).** Every task declares, before it starts, the exact tests it expects red and why; its report lists the red it saw with each assertion message; the controller checks the two are equal to the task's own additions. A red outside the set is a regression and stops the task.

**Wave schedule** (prep §3, amended for T8c and **(pending OPEN-1)** for the stacked ETL lane):

| wave | PRIMARY (main tree, `server/target`) | COMPANIONS (own worktree, own target) | CS |
|---|---|---|---|
| 1 | **T2 then T1** (one dispatch) | **T6** — `C:\w\c2t6`, `wip/c2-t6` at `7b6b2c3`, `C:/mut/t1`, `-j 4` | primary: DOC (vectors) |
| 2 | **T3** | **T7a** (worktree at T6's `wip` commit, `C:/mut/t1`) ∥ **YearInput** (C#, needs only T1's vectors) | primary: DOC |
| 3 | **T4** | **T8a** (worktree at T7a's commit) ∥ **T10a** (C#) | primary: DOC |
| 4 | **T5** | **T8c** (worktree at T8a's commit) ∥ **T10a** continues | primary: DOC |
| 5 | controller: land the ETL stack, then **ONE rebuild** (`cargo run -p atlas-graph --bin atlas-graph-compile -j 8 -- --data-dir ../data/compiled`, `bibex verify`); then **T7b + T8b + T8c's serving** (one implementer — all three edit `text_window`/`node_edges`) | **T9** (worktree) if `document.rs` contention is resolved, else wave 6 | controller: ROOT; primary: DOC |
| 6 | **T11** (alone, quiet box, app down) | **T10b** (C#) — lands before T11's Playwright | T11: FIX, VER, mutation per D19 |

Two Rust-heavy jobs at most at once, a C# job beside them (PRINCIPLES 23). Companions: `git -c core.autocrlf=false worktree add -b wip/<task> C:\w\<task> <sha>`; `data/raw` and `data/cache` **copied with robocopy, never linked** (PRINCIPLES 20); a reparse-point scan returns 0 before any `git worktree remove`.

## 3. Global constraints (replacing the plan's)

- `docs/PRINCIPLES.md` 1–23 binds: TDD (red pasted), whole-body assertions (PRINCIPLES 15 — the plan's field picks such as `body["person"]["birth"]`, `entry["votes"].is_u64()` and key-set checks are rewritten whole-body in each dispatch), named constants, newspaper order, `why` comments only, test comments only `// Arrange` / `// Act` / `// Assert`, a D.R.Y. pass and the Haskell bar in every review.
- **This batch changes the wire on purpose** (P9). AQC `0.10.0 → 0.11.0` (D1), AGC minor, at T11. Fixtures are regenerated through the exporters, never hand-edited, once (R-C2-P1). `scene_byte_identity`'s 25 hashes are re-pinned once, at T11, with the reason in the commit message.
- Names: the spec's §3, plus `TextPoint` (W2), `Word`, `Wording` (RA-4/RA-7), and the D10/D11/D12 adjustments. Test helpers are the tree's: `compiled_app()` and `async fn get(&app, uri) -> (StatusCode, Value, HeaderMap)` in `atlas-contract/tests/graph_api.rs` (`#[tokio::test]`); node ids are encoded (`Event:ab_ur`, `Place:hazor-1`); atlas-graph tests use `tests/common` (F0-24), never a new per-file helper.
- The legacy detail routes are not changed (FOCUS retires them); their structs may be *renamed in the document only* (D10).
- While the owner's app runs on :8000/:5000: never build or test `-p atlas-server` or `--workspace`; the primary uses `-j 6`, companions `-j 4`.
- Implementers commit per task and do not push; the controller lands and pushes (owner's "go" authorization).

## 4. Types first — every new or changed interface (PRINCIPLES 12)

Owner-seen in spec §3 unless marked **NEW** (an interface this amendment adds) or **(pending OPEN-n)**.

```rust
// graph-types/src/wording.rs — NEW (T6). `pub mod wording;` in lib.rs.
/// One word of a unit's base text and the text after it, up to the next word.
pub struct Word { pub form: String, pub after: String }
/// A unit's base text as words: the text before the first word, then each word with the text after it.
/// Neither `before` nor any `after` holds a letter or digit; `compose` is the unit's text byte for byte.
pub struct Wording { pub before: String, pub words: Vec<Word> }
impl Wording {
    pub fn of(text: &str) -> Wording;                                              // the tokenizer: total, lossless
    pub fn compose(&self) -> String;
    pub fn quotation_range(&self, first: u16, last: u16) -> Option<Range<usize>>;  // T6 (pending OPEN-3); scalar offsets
    pub fn words_range(&self, first: u16, last: u16) -> Option<Range<usize>>;      // T7b; forms only
}
// A word: a maximal run of letters or digits (`char::is_alphanumeric`), continued across one
// ’ ' – or - that has a letter or digit on both sides ("Beer–sheba", "wife’s", "Tubal–cain").

// graph-types/src/graph.rs (T6)
pub struct Graph { /* … */ pub wordings: BTreeMap<TextRef, Wording>, /* … */ }   // NEW field: the base text of every unit that has one
// graph-types/src/store.rs (T6)
trait GraphQuery { /* … */ fn wording(&self, unit: &TextRef) -> Option<Wording>; }  // NEW required method: 6 impls
// graph-types/src/sections.rs (T6)
pub const SECTION_SCHEMA_VERSION: u32 = 16;
// extra_tables_of(Section::Kjv) = ["verse", "red_letter_span", "kjv_token"]      (pending OPEN-8)
// (T8a) extra_tables_of(Section::Concord) gains "concord_token"; CrossRefs split per row by `from` corpus

// atlas-core/src/label.rs — T1 (D4)
pub struct Year { pub value: i32, pub label: String }
pub struct TimeRange { pub from: Year, pub to: Year, pub label: String }
impl Year { pub fn of(value: i32) -> Result<Year, CoreError>; }        // (pending OPEN-4a; signed spec: -> Year)
impl TimeRange { pub fn of(range: time::TimeRange) -> TimeRange; }      // (pending OPEN-4a; signed spec: of(i32, i32))
// atlas-core/src/time.rs (T3, D3): #[schema(as = YearSpan)] on the computation TimeRange — document-only

// atlas-contract/src/wire/locus.rs — T2
pub enum TextRef { Bible { book: String, chapter: u16, verse: u16 }, Concord { part: u8, article: u16, paragraph: u16 } }  // externally tagged, lowercase
pub struct TextPoint { pub unit: TextRef, pub word: Option<u16> }       // NEW (W2); `word` absent = the unit's edge
pub struct TextSpan { pub from: TextPoint, pub to: TextPoint }          // changed (W2); inclusive
pub struct Anchor { pub start: usize, pub end: usize, pub kind: EdgeKind, pub node: NodeRef }  // Unicode-scalar offsets (D20)
pub struct ForeignLayer { pub layer: TranslationId }                    // NEW (pending OPEN-5)
impl TextRef { pub fn of_verse(v: &VerseRef) -> TextRef; }             // re-anchored: no canon parameter
impl From<&atlas_graph_types::text::TextRef> for TextRef {}
impl TextSpan {
    pub fn whole(unit: TextRef) -> TextSpan;                            // replaces the plan's `single`
    pub fn of_bible_range(r: &BibleLocusRange) -> Result<TextSpan, ForeignLayer>;  // replaces `of_range(&BibleLocusRange, &Canon)`
}
// atlas-contract/src/wire/graph.rs (T2): NodeRef derives Clone, PartialEq
// atlas-contract/src/wire/time.rs — T1
pub use atlas_core::label::{TimeRange, Year};
pub struct DateClaim { pub when: TimeRange, pub label: String, pub verses: Vec<TextSpan>, pub note: Option<String>, pub event: Option<NodeRef> }  // `event` accepted (D12)
impl DateClaim { pub fn of(when: TimeRange, verses: Vec<TextSpan>, note: Option<String>, event: Option<NodeRef>) -> DateClaim; }
// atlas-contract/src/document.rs — T1: GENERATED_DOCUMENTS [_; 3] -> [_; 4]; pub fn year_labels_json() -> String;
//   consts VECTOR_YEARS, VECTOR_RANGES, VECTOR_CLAIMS. T4/T7a/T8a each add one document (a shared array line).

// wire changes T3 (D4–D7, D12, D22): PersonLife.{birth,death,first,last}: Option<Year>; Scene.window: Option<TimeRange>;
//   SceneEvent.when: TimeRange; ScenePlace/QuietPlace.existence_from/existence_to: Option<Year>;
//   Era { id: EraId, name: String, from_year: i32, to_year: i32, window: TimeRange }                 (D7 additive)
//   Polity { id: PolityId, …existing fields incl. from/to…, reign: TimeRange }                      (D7 additive)
//   TextUnit.locus: TextRef; ContentsRoot.locus / ContentsChild.locus: TextRef                       (D5)
// T4: EdgeEntry { edge, node, <votes|narrative as a sum, flat on the wire>, loci: Option<Vec<TextSpan>>, note: Option<String> }  (RA-8)
//   atlas-graph/src/runs.rs: pub fn coalesce(ranges: &[BibleLocusRange], canon: &Canon) -> Vec<BibleLocusRange>;
// T5: NodeCard.{event: Option<EventDetail>, place: Option<PlaceDetail>, catechism: Option<CatechismDetail>, book: Option<BookDetail>};
//   EventDetail.kind: EventKind (D11); PlaceDetail.{established, destroyed}: Option<DateClaim>; BookDetail.written per D22;
//   TextUnit.heading: Option<atlas_graph::heading::Heading> (D10); legacy structs #[schema(as = EventPage / PlacePage)]
// T7a: atlas-graph/src/mention_spans.rs
pub struct Name { pub entity: MentionedEntity, pub text: String }                     // NEW
pub struct NameSegment { pub chars: Range<usize>, pub entity: MentionedEntity }       // NEW (the plan's Segment {start,end,kind,id})
pub fn scan(text: &str, names: &[Name]) -> Vec<NameSegment>;
pub fn locate(segment: &NameSegment, wording: &Wording) -> Option<TokenSpan>;       // whole words or None (counted, R-C2-7)
// T7b: pub struct MentionSpan { pub entity: MentionedEntity, pub words: TokenSpan }  // NEW (D15)
//   GraphService::mention_spans_at(&self, verse: &VerseRef) -> Vec<MentionSpan>;  TextUnit.anchors: Vec<Anchor>
// T8a: atlas-graph/src/citations.rs
pub struct Citation { pub chars: Range<usize>, pub cites: BibleLocusRange }          // NEW (a range citation keeps its whole range)
pub fn scan(text: &str) -> Vec<Citation>;
// T8c: atlas-etl/src/kretzmann.rs KretzUnit gains `pub quoted: Range<usize>` (its fragments' index range in ParsedChapter.fragments)
//   atlas-graph/src/kretzmann_adapter.rs: KretzmannAdapterStats gains `word_anchored: usize, verse_anchored: usize`
// T9: wire::Contract { manifest_schema: u32, section_schema_version: u32 }
// T10a: client/YearInput.cs  public static bool TryParse(string text, out int from, out int to)
```

---

## 5. Tasks

### Task 2 — Structured loci, down to the word (wave 1, PRIMARY, first)

**The plan's text now means:** `wire/locus.rs` carries W2's word-addressed span. `TextSpan { from: TextRef, to: TextRef }` becomes `TextSpan { from: TextPoint, to: TextPoint }`; `TextSpan::single` becomes `TextSpan::whole`; `TextSpan::of_range(&BibleLocusRange, &Canon)` becomes `TextSpan::of_bible_range(&BibleLocusRange) -> Result<TextSpan, ForeignLayer>` (RA-5, pending OPEN-5); `Canon::kjv()` does not exist — `TextRef::of_verse(&VerseRef)` reads `atlas_core::canon::BOOKS`.

**Files:** create `server/atlas-contract/src/wire/locus.rs`, `server/atlas-contract/tests/locus_wire.rs`; modify `server/atlas-contract/src/wire/mod.rs`, `server/atlas-contract/src/wire/graph.rs` (NodeRef derives only). **CS:** none. **Expected red:** none.

- [ ] **Step 1: the failing tests** (whole-body, one behaviour each, in `locus_wire.rs`):
  `a_bible_ref_serialises_externally_tagged_with_the_book_code` → `{"bible":{"book":"GEN","chapter":1,"verse":1}}`;
  `a_concord_ref_serialises_externally_tagged` → `{"concord":{"part":7,"article":2,"paragraph":1}}`;
  `a_verse_ref_becomes_a_bible_ref_through_the_canon_table` → `TextRef::of_verse(&VerseRef{book:0,chapter:1,verse:1}) == TextRef::Bible{book:"GEN",…}`;
  `the_graphs_corpus_erased_ref_widens_to_the_wire_ref` → both variants, one `assert_eq!` over the pair;
  `a_point_at_a_unit_edge_omits_its_word` → `{"unit":{"bible":{…}}}`;
  `a_whole_verse_range_is_a_span_between_unit_edges` → `of_bible_range(GEN.1.1–GEN.1.3, spans None)` = `TextSpan { from: {GEN.1.1, None}, to: {GEN.1.3, None} }`;
  `a_word_span_inside_one_verse_names_its_first_and_last_word` → `from == to == Locus{GEN.1.2, Some(kjv 3..=5)}` ⇒ `{from:{GEN.1.2, Some(3)}, to:{GEN.1.2, Some(5)}}`;
  `a_word_span_across_verses_names_where_it_starts_and_ends` → `from = Locus{1SA.1.27, Some(kjv 4..=last)}`, `to = Locus{1SA.1.28, Some(kjv 0..=11)}` ⇒ `{from:{1SA.1.27, Some(4)}, to:{1SA.1.28, Some(11)}}`;
  `a_span_in_another_layer_is_refused` → `Err(ForeignLayer { layer: "greek_textus_receptus" })` (pending OPEN-5);
  `an_anchor_serialises_its_offsets_kind_and_node` → the whole object.
- [ ] **Step 2: run** — `cargo test -p atlas-contract --test locus_wire -j 6` → compile error (no `wire::locus`). Paste it.
- [ ] **Step 3: implement** the §4 signatures; `word` is `#[serde(skip_serializing_if = "Option::is_none")]`; `Anchor.start`/`end` carry a doc line saying they are Unicode-scalar offsets into the owning `TextUnit.text` (D20 — an invariant the type cannot carry).
- [ ] **Step 4: run** — green; `cargo run -p atlas-contract --bin export_contract -j 6 -- --check` exit 0 (no route references the types; the document is unchanged).
- [ ] **Step 5: commit** — `contract: structured loci on the wire, down to the word -- TextRef, TextPoint, TextSpan, Anchor (P2, R-C2-W2)`.

**Deletes (from the plan's text, never written):** `TextSpan { from: TextRef, to: TextRef }`, `TextSpan::single`, `TextSpan::of_range(_, &Canon)`, the `canon` parameter of `from_verse`.

### Task 1 — Years with labels (wave 1, PRIMARY, second)

**The plan's text now means:** D4 — the labelled `Year`/`TimeRange` live in `atlas-core/src/label.rs` and are re-exported from `wire/time.rs`; `DateClaim` stays in atlas-contract with `verses: Vec<TextSpan>` (T2 has landed, so the plan's `Vec<String>` fallback is void). D2 — the vectors carry the spec's examples and every `YearTextTests` case, era named once. `generated_files()` is derived from `GENERATED_DOCUMENTS` (`document.rs:50-54`), which gains the vectors document (length 3 → 4). Signatures per OPEN-4.

**Files:** create `server/atlas-core/src/label.rs`, `server/atlas-contract/src/wire/time.rs`, `server/atlas-contract/tests/year_labels.rs`, `contracts/atlas-query-contract/vectors/year-labels.json` (exporter output); modify `server/atlas-core/src/lib.rs`, `server/atlas-contract/src/wire/mod.rs`, `server/atlas-contract/src/document.rs`. **CS:** DOC (the vectors only; `openapi.yaml` and `aqc.schema.json` must be byte-unchanged). **Expected red:** none.

- [ ] **Step 1: the failing tests** in `year_labels.rs` — the plan's four, with D2's cases:
  years `[-4004, -1447, -1, 1, 30, 33, 100, 2000]` → `["4004 BC", "1447 BC", "1 BC", "AD 1", "AD 30", "AD 33", "AD 100", "AD 2000"]`;
  ranges `[(-1450,-1400), (-1447,-1400), (-1447,-1447), (-5,30), (33,33), (1,100), (30,70)]` → `["1450 – 1400 BC", "1447 – 1400 BC", "1447 BC", "5 BC – AD 30", "AD 33", "AD 1 – 100", "AD 30 – 70"]`;
  claims `[(-1003,-1003,Some("traditional")), (-586,-586,None), (-1447,-1400,Some("traditional"))]` → `["c. 1003 BC", "586 BC", "c. 1447 – 1400 BC"]`;
  `the_vectors_document_is_the_same_rules_written_out` — the whole JSON (`years`, `ranges`, `claims`) as one expected value;
  (pending OPEN-4a) `there_is_no_year_zero` → `Year::of(0) == Err(CoreError::ZeroYear)`.
- [ ] **Step 2: run** — compile error. Paste it.
- [ ] **Step 3: implement** — label rules exactly as the plan's Step 3 (`" – "` en dash with spaces; shared era once; `"c. "` iff a note); `year_labels_json()` renders `VECTOR_YEARS`/`VECTOR_RANGES`/`VECTOR_CLAIMS` through the constructors (the vectors are exporter output, never hand-written).
- [ ] **Step 4: run** — green; `cargo run -p atlas-contract --bin export_contract -j 6` writes the vectors; `-- --check` exit 0; `cargo test -p atlas-contract --test contract_generation -j 6` green; `git diff --stat ../contracts` shows `vectors/year-labels.json` only.
- [ ] **Step 5: commit** — `contract: years travel with their labels -- Year and TimeRange (atlas-core, D4), DateClaim, and the year-labels vectors (P1)`.

### Task 3 — Labelled years and structured loci on the graph path (wave 2, PRIMARY)

**The plan's text now means:** D6 — `PlaceDetail`'s `DateClaim` and `BookDetail.written` move to T5; T3 is `PersonLife`, `Scene*`, `Era`, `Polity`, `TextUnit.locus`, `Contents*.locus`. D4 — `Scene`/`SceneEvent`/`ScenePlace`/`QuietPlace` are `atlas_core::wire` types composed in atlas-core (`atlas-core/src/wire.rs:9-110`): the fields change there, carrying `atlas_core::label` types (which gain `Deserialize` if `Scene`'s derive demands it). D22 — `existence_from`/`existence_to: Option<Year>` (not the spec's `existence: Option<TimeRange>`: a one-claim place is half-open). D7 — `/api/eras` and `/api/polities` ADD `window`/`reign` beside the consumed integers; `contracts/atlas-edge/` must stay green. D12 — `Era.id: EraId`, `Polity.id: PolityId`. D3 — the computation `TimeRange` publishes as `YearSpan`; `atlas_core::data::Era` loses `ToSchema`. D5 — `ContentsChild.locus` is the `TextRef` of the unit the entry opens at. R-C2-P1 — DOC only: no fixture, pact, `scene_byte_identity` or VERSION move (the plan's Step 4 moves to T11).

**Files:** `server/atlas-core/src/{wire.rs, scene.rs, time.rs, data.rs}`, `server/atlas-contract/src/wire/{graph,map,contents}.rs`, `server/atlas-contract/src/{graph,map,contents}.rs`, possibly `server/atlas-graph/src/map_adapter.rs` (compiles unchanged if only field types move), `server/atlas-contract/tests/graph_api.rs`, atlas-core scene tests. **CS:** DOC. **Expected red:** the client build (D18); `regenerated_aqc_corpus`; the `aqc_cucumber` scenarios whose recorded bodies carry the changed fields; `contract_pact`, `contract_pact_cli`; `scene_byte_identity` (25); the AGC `node-card`/`eras`/`polities` projections in `contract-gate.sh` — each with its message; `contracts/atlas-edge` GREEN.

- [ ] TDD: `a_person_card_carries_labelled_years` (whole `person` object, Moses' true years read once from the running API), `a_text_unit_carries_its_structured_locus_beside_its_ref` (whole unit), `a_contents_child_carries_the_locus_it_opens_at`, `an_era_carries_its_window_beside_the_integers_the_edge_suite_consumes`, `a_polity_carries_its_reign_beside_its_integers`, `a_scene_place_with_one_claim_has_one_labelled_end` — red, implement, green; `export_contract` then `--check` 0. Commit: `contract: labelled years and structured loci on every graph-path struct; eras and polities gain window and reign beside their integers`.

### Task 4 — Edge entries carry their metadata (wave 3, PRIMARY)

**The plan's text now means:** the graph-types/snapshot half is already done (`EdgeEntry { edge, node, meta: EdgeMeta }`, `explore.rs:21-32`, filled at `snapshot.rs:219-245`); only the wire projection and the loci remain (RA-8). D8 — `attested-in` loci = the coalesced runs of that event's attestations in the entry's book for that witness (`runs::coalesce` over the canon's chapter lengths); `note` = the witness's `ref_note` from the legacy path (`atlas_graph::legacy::event_from_node` + `atlas_core::scene::witnesses_for`), never the merged justification text; `attests` loci = the edge's own range. W2 — loci are `TextSpan`s with `word: None` (verse runs). D9 — `vectors/attestation-runs.json` carries only the coalescing-semantics cases. `runs::coalesce` returns `Vec<BibleLocusRange>`, not the plan's `Vec<(VerseRef, VerseRef)>` (a record where the plan had a tuple). Node id `Event:ab_ur`.

**CS:** DOC (openapi + the runs vectors). **Expected red:** T3's set, plus the traversal fixtures and the AGC `edge-page` projection.

- [ ] TDD: `a_cites_entry_carries_its_votes` and `an_attested_in_entry_carries_the_witness_runs_and_note` (whole entry objects, values from the legacy `/api/event/ab_ur`); `atlas-graph/tests/runs_vectors.rs` replays the vectors. Commit: `graph: edge entries carry votes, narrative, coalesced attestation loci and notes (P6)`.

### Task 5 — Kind details on the card; pericope headings (wave 4, PRIMARY)

**The plan's text now means:** re-anchored sources (prep §1): event from `atlas_graph::legacy::event_from_node` (never `AtlasData.events` — `no_legacy_event_reads.rs` forbids it), place from `atlas-contract/src/places.rs:30-67`, catechism from `catechism.rs:36-80`, book from `reading.rs:154-160`; `node_card` gains `State(data)`. D10 — card details keep the spec's names, the legacy route structs publish as `EventPage`/`PlacePage`, `TextUnit.heading` reuses `atlas_graph::heading::Heading`. D11 — `kind: EventKind`. D6 — `PlaceDetail.established`/`destroyed: Option<DateClaim>` (verses as whole-verse `TextSpan`s; `event` resolved from `PlaceDateClaim.event: Option<EventId>` to a `NodeRef`) and `BookDetail.written` land here. D22 — read `BookMeta.write_from/write_to` first: if either can be absent alone, `written_from`/`written_to: Option<Year>`, else `written: Option<TimeRange>`.

**CS:** DOC. **Expected red:** T4's set, plus `focus-*`/`text-window-*` fixtures and the AGC `node-card` projection.

- [ ] TDD: the plan's five tests, whole-body. Commit: the plan's subject.

### Task 6 — The KJV's words are the base of the text (wave 1, COMPANION)

**The plan's text now means:** R-C2-W1/W4 replace "a KJV token layer with character offsets". A verse is a container over its words; the artifact stores the KJV once, as words and the text between them; verse text is composed. `kjv_token` lives in the **Kjv** section (D13 revised). Red letter is stored as word spans (RA-6). The fidelity law moves onto the words (RA-1). The plan's `Token { ord, form, char_start, char_end }`, `atlas-graph/src/kjv_tokens.rs`, "split on whitespace, trim punctuation" and its circular reassembly law are superseded: the spike's lossless rule, the canonical gap model (RA-4), scalar offsets derived from the words, never stored (D20, PRINCIPLES 6). No `strong`/`aligned` columns (RA-3). No `atlas-contract/src` change (RA-6). **No `data/compiled` commit** (R-C2-P1): a scratch rebuild in the worktree serves the measurements and is restored before commit.

**Files:** `graph-types/src/{wording.rs (new), lib.rs, graph.rs, store.rs, sections.rs}` and graph-types fixtures that put a `kjv` rendering on a Bible unit; `graph-types/tests/wording.rs` (new); `server/atlas-graph/src/{kjv_adapter.rs, window.rs, fidelity.rs, xref_adapter.rs, red_letter_spans.rs, service.rs, brainfuel_adapter.rs (test), bins/compile_graph.rs}`, `server/atlas-graph/src/sqlite/{ddl.rs, extras.rs, reload.rs, serve.rs, snapshot.rs}`; `server/atlas-graph/tests/{kjv_words.rs (new), common/mod.rs, sqlite_laws.rs, description_real_data.rs, fulfillment_typology_real_data.rs, justified_by_real_data.rs, peoples_real_data.rs}` and the real-data pins the scratch artifact proves; `server/atlas-cli/src/commands/chapter.rs` (fixtures); `server/atlas-contract/tests/brainfuel_layers.rs`. **CS:** none entered (DOC's one moved line and ROOT are declared red). 

**Storage (Kjv section):**
```sql
CREATE TABLE verse (node_id TEXT PRIMARY KEY, book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, before TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE kjv_token (book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, ord INTEGER NOT NULL,
  form TEXT NOT NULL, after TEXT NOT NULL, PRIMARY KEY (book, chapter, verse, ord)) WITHOUT ROWID;
CREATE TABLE red_letter_span (book INTEGER NOT NULL, chapter INTEGER NOT NULL, verse INTEGER NOT NULL, ord INTEGER NOT NULL,
  first_word INTEGER NOT NULL, last_word INTEGER NOT NULL, PRIMARY KEY (book, chapter, verse, ord)) WITHOUT ROWID;
```

- [ ] **Step 0 (F0-25): one real pipeline context.** The four `build_real_ctx` helpers (`description_real_data.rs:10`, `fulfillment_typology_real_data.rs:10`, `justified_by_real_data.rs:6`, `peoples_real_data.rs:11`) and the two `real_ctx_pieces` become ONE in `atlas-graph/tests/common/mod.rs`: `pub struct PipelineInputs { pub kjv_json: String, pub xrefs_tsv: String, pub canon: Canon, pub verses: HashMap<String, String> }` with `PipelineInputs::read() -> PipelineInputs` and `fn run(&self) -> BuildCtx<'_>` (the full pipeline over `real_atlas()`); the six copies deleted. Behaviour-preserving: the four suites' pass counts are unchanged. Commit: `tests: one real pipeline context for the four suites that each built their own (F0-25)`.
- [ ] **Step 1: measure before** — at the base, in the worktree: gates 6, 7, 8, 9 (pending OPEN-2), each as the script runs it (`cargo test -p <pkg> --test <bin> -j 4 -- --ignored --exact <name> --test-threads=1 --nocapture`); record the numbers, the load, and the Kjv section's blob `bytes` from `data/compiled/manifest.toml`.
- [ ] **Step 2: the failing tests.**
  `graph-types/tests/wording.rs`: `a_verse_is_its_words_and_the_text_after_each` (GEN.1.1 → the whole `Wording`: `before ""`, ten words, `after` `" "`×9 then `"."`); `an_opening_bracket_is_the_text_before_the_first_word` (`"(For after all these things do the Gentiles seek:) for"` → `before "("`, …, `seek` / `":) "`, `for` / `""`); `joined_words_stay_one_word` (`"of Beer–sheba."`, `"his wife’s name"`); `every_text_composes_back_from_its_wording` (a list of edge texts incl. `""`, `"..."`, `" a "`); `no_text_between_words_holds_a_letter_or_digit`; (pending OPEN-3) `a_quotation_runs_from_its_opening_brackets_through_its_closing_punctuation` (`quotation_range(0, 8)` = `0..50`, `quotation_range(1, 2)` = `5..14` on the bracket text above).
  `atlas-graph/tests/kjv_words.rs`: `the_kjv_is_31_102_verses_of_790_892_words` (`KJV_VERSES`, `KJV_WORDS`; the build succeeding is the fidelity law passing over every verse).
  `fidelity.rs` unit tests: `red_when_a_word_is_mutated` (replaces `red_when_a_rendering_byte_is_mutated`), `red_when_a_bible_unit_still_stores_its_kjv_text` (the stored-once law).
  `sqlite_laws.rs`: `a_wording_round_trips_through_the_kjv_section`; `the_snapshot_serves_the_wording_the_graph_holds` (port agreement, `SqliteSnapshot` vs `Graph`).
  red letter, in `kjv_words.rs`: `every_red_letter_span_is_a_word_span_quoting_the_aligned_text` — every aligned span becomes a word span whose `quotation_range` equals the aligner's character span, except exactly `DIFFERS_FROM_THE_ALIGNED_TEXT = ["MAT.11.27", "MAT.13.17", "LUK.10.22"]` (pending OPEN-3).
  `window.rs`: `render_composes_the_verse_from_its_words`.
- [ ] **Step 3: run** — compile errors / red. Paste them.
- [ ] **Step 4: implement** — `Wording` (§4); `kjv_adapter::normalize` fills `graph.wordings` from the restored verses and `verse_node` stops inserting the `kjv` rendering; `window::render` composes Bible units through `query.wording` (Concord keeps `render_layer`); `check_kjv_fidelity` reads compositions and refuses a Bible payload that still carries `kjv`; `xref_adapter` reads the composition; `red_letter_spans` converts the aligner's byte spans to word spans and derives the served character index; the DDL/extras/reload/serve/snapshot plumbing; `SECTION_SCHEMA_VERSION = 16`; `kjv_token` in `extra_tables_of(Kjv)` (pending OPEN-8).
- [ ] **Step 5: verify** — `(cd ../graph-types && cargo test --all-features)` (F0-5); `cargo test -p atlas-graph --lib --test kjv_words --test sqlite_laws --test wording -j 4` and every atlas-graph/atlas-cli/atlas-contract suite that does not open the tracked artifact — green; 0 warnings.
- [ ] **Step 6: scratch rebuild and measure after** — `cargo run -p atlas-graph --bin atlas-graph-compile -j 4 -- --data-dir ../data/compiled` in the worktree; gates 6, 7, 8, 9 again; the artifact-reading suites against the scratch artifact (report their red: only schema/Kjv-table/root pins may move); the Kjv blob's `bytes` after. Then restore every tracked path the compile rewrote (`git restore data/compiled data/exports`); `git status --short` shows only source paths.
- [ ] **Step 7: commit** — `graph: the KJV's words are the base of the text (schema 16) -- verses compose from their words and the text between them; red letter is word spans (R-C2-W1, W4)`.

**Expected red (declared):** (a) every test that opens the tracked `data/compiled`, failing at open with `section <name> user_version 15 unsupported (this build understands 16)`; (b) `version_root_regression` (the from-raw root moves: payloads and `kjv_token`); (c) `contract_generation` / `export_contract --check`: `graph-vocabulary.json`'s `section_schema_version` 15 → 16, one line, not regenerated here (DOC belongs to the primary); (d) `contract_api`'s whole-body `section_schema_version: 15`. Anything else red stops the task.

**Deletes:** the `kjv` entry of every Bible `TextUnit` payload; `red_letter_spans::char_offset` and `spans_by_dot_ref`; the character columns `red_letter_span.start/end_`; the two `renderings.get(KJV)` reads in `fidelity.rs` and the one in `xref_adapter.rs`; `window::render`'s renderings read for Bible units; the four `build_real_ctx` and two `real_ctx_pieces`; `brainfuel_layers.rs`'s `kjv` key assertions (6 → 5 and 5 → 4 layers) and `brainfuel_adapter`'s "canonical KJV layer untouched" assertion (restated: renderings carry no `kjv`); graph-types fixtures with a `kjv` rendering on a Bible unit.

**W4's fallback:** only if the measurements show composition is impractical (pending OPEN-2 on which gates decide) does the verse text stay — as a recorded cache, with the composition law pinning it to the words. That is a controller call on T6's report, not the implementer's.

### Task 7a — Mention spans in ETL (wave 2, COMPANION, worktree at T6's commit)

**The plan's text now means:** the writers re-anchored (prep §1: `place_adapter.rs:31`, `event_world.rs:434`, `peoples_adapter.rs:172,184`, `person_adapter.rs:85,349`). `scan` is `PlaceMentions.cs`'s rule exactly (`char.IsLetter` boundaries, ordinal case-sensitive, longest first, ties to Place) over characters; `locate` maps a segment to the KJV words it covers — a segment that does not start at a word's start and end at a word's end is unlocatable. D21 — one `Mentions` row per occurrence (edge ids and mention counts move — disclosed); Place and Person searched (names per prep D21); PeopleGroup/Event rows stay verse-level and are not counted. R-C2-7 — `MentionSpanStats { located, unlocatable }` in the compile report. `vectors/mention-spans.json` from the 11 C# cases (its `GENERATED_DOCUMENTS` line is this task's; a shared array line the controller merges at landing).

**Expected red:** T6's set; plus every `mentions` edge id and the core hash (ROOT, wave 5).

### Task 7b — Anchors on text windows (wave 5, serving)

`text_window` pages a unit's mentions through `GraphService::mention_spans_at` (D15) and turns each word span into `Anchor { start, end, kind: Mentions, node }` with `Wording::words_range`. Test: `a_verse_that_names_hazor_serves_an_anchor_over_the_name` — whole anchor list for `JOS.11.1`, `Place:hazor-1`.

### Task 8a — Concord citations in ETL (wave 3, COMPANION, worktree at T7a's commit)

**The plan's text now means:** `citations::scan` ports `ScriptureRefScan.cs` (the ~69-pair alias table, `:21-41`, and the grammar); a range citation keeps its whole range. D13 — Concord units get a word layer from the same `Wording::of`, stored in `concord_token` in the Concord section (pending OPEN-7 on whether the Concord payload string goes too). The Concord pass writes `CrossRef { from: TextLocus(unit, TokenSpan over the unit's words), to, to_last, target_display, votes: 0, provenance: "concord-citations" }`; `CrossRefs` is split per row by `from` corpus in the one split function (the `CanonSuccession` precedent). `vectors/citation-grammar.json`. **Expected red:** T7a's set, plus the Concord section's tables/hash. graph-types editor of its worktree.

### Task 8b — Cites anchors on Concord units (wave 5, serving)

`text_window`'s Concord branch emits `Anchor { kind: Cites, node }` per citation, character range from the unit's words. Test: `a_small_catechism_paragraph_that_cites_scripture_serves_cites_anchors` — whole anchor list.

### Task 8c — Kretzmann's comments anchor on words (NEW; wave 4, COMPANION, worktree at T8a's commit)

**Ruling:** R-C2-W3 (and D16 as amended). **Interfaces:** `KretzUnit.quoted: Range<usize>` (atlas-etl; the unit's excised fragments, which the parser already keeps for the conservation check and never stores on the graph); `KretzmannAdapterStats.{word_anchored, verse_anchored}`. No row-family or DDL change: `comments_on` already stores `on_from_layer/start/end` and `on_to_layer/start/end`.

- [ ] **Failing tests:** `a_lemma_that_quotes_part_of_a_verse_comments_on_those_words` (a fixture chapter: the unit's `on` range is `Locus{v, Some(kjv s..=e)}` both ends); `a_quotation_across_verses_comments_on_a_cross_verse_word_span` (1SA.1.27–28 shape); `a_lemma_that_does_not_align_stays_verse_range_and_is_counted`; `a_quote_block_of_whole_verses_keeps_whole_verse_ends` (RA-5 canonical form); real-data `kretzmann_word_anchoring_real_data` — pins `word_anchored`/`verse_anchored` for the whole corpus (report beside the spike's 42,834 / 52,601 OT-lemma figure).
- [ ] **Implement:** align the unit's fragments, in order, to a contiguous run of KJV words starting in the unit's first verse, by the comparison OPEN-9 rules; on success the range's ends carry `TokenSpan`s (layer `kjv`) in canonical form; on any failure the unit keeps its verse range and counts. Nothing guessed.
- [ ] **Commit:** `graph: Kretzmann's comments anchor on the words they quote, where the quotation aligns exactly (R-C2-W3)`.

**Serving half (wave 5, with T7b/T8b; pending OPEN-6):** `node_edges` fills `loci` on `comments-on` / `commented-on-by` entries through `TextSpan::of_bible_range`. Citations *inside* Kretzmann prose stay FOCUS-7.

### Task 9 — `/api/contract` declares schema versions only

**The plan's text now means (D17):** the file list in prep §1's Task 9 row, in full; the AQC version the documents carry comes from ONE `include_str!` of `contracts/atlas-query-contract/VERSION`; the VERSION == MIN/MAX drift law (`aqc_corpus_generation.rs:104-116`) is deleted with its second source; the AGC `contract` projection (`contracts/runner/src/Proj.hs:294-297`) projects `manifest_schema`/`section_schema_version`; the runner rebuild and fixture re-bless are T11's. Test expected value: `{"manifest_schema": 1, "section_schema_version": 16}`.

### Task 10a — The client reads labels, loci and details (C#)

`client/YearInput.cs` + `YearInputTests` over `year-labels.json` (runs in wave 2, needing only T1); then the retarget of every `YearText`/`CanonRef` call site listed in prep §1's Task 10 row (it adds `AuthorNode.cs`, `PassageNode.cs`, `VerseNode.cs`, `ArrowNavTests.cs` to the plan's list; `LegacyNodes` does not exist); `ConformanceTests.RepoRoot()` is a method; `years.ts`'s `SPAN` moves into `canon.ts` before `years.ts` dies. The client build leaves the red set when 10a lands (D18).

### Task 10b — Anchors, runs and citations (C#)

`MentionScan.razor` renders served anchors; `PassageBlock` renders served runs (`TrueRunsOf`/`StitchRuns`, `Versification.cs`, `AcctCoalesceTests` go — D9: the non-vector cases are named in the report); `ScriptureRefText.razor` renders `cites` anchors for Concord and keeps `ScriptureRefScan.cs` **for Kretzmann prose only**, with a one-line why naming FOCUS-7 (D16 as amended); `PlaceMentions.cs` + tests go. No Stryker run here (D19).

### Task 11 — One regeneration, gates, close (wave 6, alone, app down)

- [ ] Documents and fixtures, once: `export_contract` (+ `--check`), `export_aqc_examples`, `ATLAS_BLESS_PACT=1` pact then green, AGC fixtures through the rebuilt runner (T9's `Proj.hs`).
- [ ] Versions, once: AQC **`0.10.0 → 0.11.0`** (D1) with a CHANGELOG entry "MAJOR class, MINOR bump under the 0.x rule" naming P1, P2 (+W2's word points), P6, P7, P8, the `YearSpan`/`EventPage`/`PlacePage` document-only renames (D3, D10), D7's additive fields, D21's moved mention ids, W4 (the served text is unchanged; the KJV is stored as words) and OPEN-3's three verses if taken; AGC minor.
- [ ] Pins, once: `version_root_regression`, `data/exports` roots, `scene_byte_identity`'s 25 hashes (one commit, reason in the message), every count the wave-5 rebuild moved that its commit did not re-pin.
- [ ] The standing block (`cargo test --workspace`, graph-types `--all-features`, sum and sections); `bash ../scripts/timing-gates.sh` then `check` — gates 6, 7, 8, 9 against T6's before/after (W4's second measurement; a material regression forks an investigation, never a ceiling raise — PRINCIPLES 14a); `bash ../scripts/contract-gate.sh --base 7b6b2c3`; `bash ../scripts/contract-semver-gate.sh 7b6b2c3 --runner <path>`.
- [ ] Playwright (after 10b lands).
- [ ] Mutation (D19): before 2026-09-30 09:00 append `CONTRACT-2` to `.superpowers/MUTATION-GATE-DEBT.md`'s "batches since"; after, run the debt file's command once (one holder). Either way each crate's `mutants.toml` covering-target list gains this batch's new test targets (`locus_wire`, `year_labels`, `kjv_words`, `runs_vectors`, `mention_spans_vectors`, `citation_vectors`, `kretzmann_word_anchoring_real_data`; graph-types `wording`) — the R49 lesson.
- [ ] The controller pushes.

---

## 6. Deletion inventory (the batch's, by task)

| Task | Deleted |
|---|---|
| T2 | (plan text, never written) verse-granular `TextSpan`, `TextSpan::single`, `of_range(_, &Canon)` |
| T3 | bare year integers on `PersonLife`, `Scene`, `SceneEvent`, `ScenePlace`, `QuietPlace` (not on `/api/eras`/`/api/polities` — D7); `ToSchema` on `atlas_core::data::Era` |
| T4 | the client-side need for `CrossRefOut.votes`/`EventWitnessOut.verse_groups/ref_note` (the routes stay until FOCUS) |
| T6 | see Task 6 **Deletes** |
| T9 | `MIN/MAX_SUPPORTED_VERSION`; the drift law; `then_server_advertises` and its C# twin (`AqcSteps.cs:379`); `versioning.feature`'s `advertises` line; `Contract.min_version/max_version` |
| T10a | `YearText.cs`, `YearTextTests.cs`, `tests/ux/lib/years.ts`, `CanonRef.cs`, `canon.ts`'s ref construction |
| T10b | `Versification.cs`, `PassageBlock.TrueRunsOf/StitchRuns`, `AcctCoalesceTests.cs`, `PlaceMentions.cs`, `PlaceMentionsTests.cs`; `ScriptureRefScan.cs`'s Concord use (the file stays for Kretzmann prose) |
| T11 | the plan's per-file `cargo mutants -f …` step and T10's Stryker step (D19) |

## 7. Self-review against the rulings

- **Coverage:** R-C2-P1 (§2; every task's CS and expected-red line; T11 alone moves FIX/VER), W1 (T6 tokenizer, 790,892 pin, word identity, no per-word hash), W2 (§4, T2, T1's `DateClaim.verses`, T4/T5/T8c loci), W3 (T8c), W4 (T6 storage, composition law, red letter as words, gates before/after; T11 again), D1–D22 (§1 table, each placed in its task), D13 revised (T6 Kjv section), D16 as amended (T8c, T10b), D19 (T10b, T11), F0-25 (T6 Step 0).
- **Gaps:** every guess the rulings force is an OPEN item; nothing under "(pending OPEN-n)" is presented as ruled.
- **Type consistency:** `TextSpan`/`TextPoint` (T2) are what `DateClaim.verses` (T1), `EdgeEntry.loci` (T4, T8c), `PlaceDetail` claims (T5) carry; `Wording` (T6) is what T7a's `locate`, T7b's anchors, T8a's Concord layer and T8c's alignment read; `TokenSpan` (existing) is every stored word span; `Anchor` (T2) is what T7b/T8b emit and T10b renders.

## Controller rulings on the OPEN items (2026-09-29, final — implementers do not re-open)
- OPEN-1 — accepted: T6 → T7a → T8a → T8c build as a stack of `wip/` branches (each based on the previous); the controller lands the stack just before the one rebuild, so the tracked schema-15 artifact stays readable for T3–T5.
- OPEN-2 — accepted: R-C2-W4's "gate 8" means the `/api/text` gates 6 and 7 plus startup gate 9; they decide whether composition stays. Gate 8 is D13's write measure. T6 reports a relative before/after on its loaded box; T11 judges against ceilings and baselines.
- OPEN-3 — REVISED, not the proposal: no served red-letter byte changes. A red-letter span is a word span plus a typed end, `SpanEnd::{AtWord, ThroughPunctuation}` (not a bool), recorded per span from the source, so all 2,063 served spans reproduce exactly — MAT.11.27, MAT.13.17 and LUK.10.22 included. A law pins the 2,063 against today's served spans.
- OPEN-4 — (a) accepted: `Year::of` returns `Result` (year zero refused); `TimeRange::of` takes the validated core range; recorded as a PRINCIPLES-12 interface change to the signed §3 signature (the Haskell bar is binding). (b) REVISED: `TextRef.book: BookId` (`atlas_core::refs::BookId`, exists), with a wire form that writes the canon code, so the wire bytes stay a string; no `String` exception.
- OPEN-5 — accepted: `TextPoint.word` indexes the corpus's base layer; a span in any other layer is refused with `Err(ForeignLayer)`.
- OPEN-6 — accepted: T8c serves `EdgeEntry.loci` on comments-on entries only; commentary reader anchors go to FOCUS-7.
- OPEN-7 — accepted: the Concord text is stored as words only (R-C2-W4 reaches T8a).
- OPEN-8 — accepted: the Kjv section's logical hash is the layer hash; no manifest field, no tokenizer version.
- OPEN-9 — accepted: the spike's comparison (case-insensitive; word-internal ’ / – / - folded), without KRETZ-ACCEPT-1's equivalence classes; misses stay verse-range and are counted.

## DEFERRED by the owner (2026-09-29, later the same evening) — the words-as-base inversion goes to the queue
Owner: "Don't worry about the KJV word layer thing right now, that can wait … stick to the plan and add the addition stuff on the queue unless there's a good reason not to (duplication of effort or critical path stuff)." Applied:
- **Task 6 reverts to the original plan's "KJV token layer"** (tokens derived from the stored verses). Kept because dropping them would duplicate later work, or because the plan needs them: F0-25's helper step, the spike's tokenizer, `kjv_token` in the **Kjv** section (D13 REVISED — the queued inversion wants it there), D20's lossless token law, OPEN-1 (stacked ETL lane), OPEN-2, OPEN-8.
- **Already landed, kept:** R-C2-W2 (`TextPoint { unit, word }` in T2, d38344b). It is on the plan's critical path (T7/T8 spans use it) and costs nothing to keep.
- **QUEUED (not in CONTRACT-2):** R-C2-W4 (stored verse text dropped, verses composed from words); OPEN-3's red-letter word spans; R-C2-W3 / **Task 8c** (Kretzmann comments anchored on word spans) with OPEN-6 and OPEN-9; OPEN-7 (Concord text stored as words only); the eBible Strong's and italics copy (already a later batch); the FOCUS-7 direction of Kretzmann as a generic "text anchored on text".
- The wave schedule loses T8c (wave 4 is T5 ∥ T10a only).
