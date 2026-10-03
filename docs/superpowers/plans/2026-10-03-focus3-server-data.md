# FOCUS-3 v2 (server, data and contract): containers, passages, the reading window, the Concord text and the catechism model

> **Supersedes** `docs/superpowers/plans/2026-10-02-focus3-containers.md` (`lane/claude/F3-plan` 64eebcf) and its Amendment A (`lane/claude/F3-amend` 45507a6). Neither is executed. Their server and data halves are carried here, re-cut to the owner's rulings of 2026-10-03. Their client halves become the F# client backlog (§7). FOCUS-7 Amendment A (`lane/claude/F7-amend` c4e1aa4: `stated-in`, `UnitOpening`, `CrumbRole.Identity`) is **withdrawn** by the catechism ruling, and the catechism model spec (`lane/claude/CAT-spec` 7fa3afa) is settled as amended in §3.6.
>
> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal.** The compiled artifact and the contract give the F# client everything it needs to read the Bible and the Book of Concord through one container abstraction:
- a Concord document is a book and an article is a chapter;
- cited ranges are passages, and a passage can be marked as recording history;
- one request per page returns text units with their headings and labelled parts, plus a whole-chapter read;
- the Concord text is Luther's and the Confessions' whole 1921 Triglot text, each piece labelled and each one admitted against the Triglot;
- the Small Catechism's paragraphs are the catechism.

The C# client is frozen. It receives only the changes the gates need to compile.

**Binding rulings** (ops `QUEUE.md`, "OWNER ANSWERS, 2026-10-03"):
- F3 Q1–Q13 and A1;
- F-84 with FOCUS-3;
- O-CATECHISM-MODEL (27): (1) through (5), and "lands WITH FOCUS-3";
- F5 Q8 (labels stay codes);
- "C# CLIENT FROZEN";
- the sequencing after A-F39, A-PROVENANCE and A-WIRE-IDENTITIES.

The owner, on how to judge this plan: "if we can agree on the domain, data structure, and algebrae then we should be okay." So §3 is the design in exactly those three parts. Everything after it is execution.

**Principles:** 1–4, 6, 9, 12, 13, 14b, 15–18, 21–23, 24–24b, 25, 26, 26a, 27–27g.

**Tech stack:** Rust (`graph-types`, `atlas-etl`, `atlas-graph`, `atlas-contract`, `atlas-server`), utoipa, cargo-mutants, plus property tests (`proptest`, already in `graph-types` dev-dependencies, to verify in Task 0). No C# feature work. No F# work: that is Codex's CX-FSHARP lane, and §7 feeds it.

---

## 1. Base, sequencing and what stacks under this batch

| Order | Item | State 2026-10-03 | What FOCUS-3 uses from it |
|---|---|---|---|
| 1 | A-F39 | `lane/claude/F39` 474c98d, in re-review after F-85 | the logical hash covers every derived table and row address, so this batch's new derived tables (`container_detail`, passage rows, admission rows) move the root without a schema bump |
| 2 | A-PROVENANCE | `lane/claude/PROV` 515cf61 | `provenance: { id, title }`. The new sources (Triglot OCR, our curated alignment) are served with titles |
| 3 | A-WIRE-IDENTITIES | spec `lane/claude/WIREID-spec` d64ece1; owner signed O1–O4 on 2026-10-03 | `ArtifactRoot`, `EdgePageCursor`, `NodeId`, `ElementId`, and the per-shape references `VerseReference`, `ChapterReference`, `PassageReference`, `VerseRangeReference`, `ConcordReference` and their unions. **Every new wire field below uses these names, never `String`** |
| 4 | **FOCUS-3 v2** | this plan | |

- **Base:** the trunk head (`origin/worktree-bible-atlas-m1`) once A-WIRE-IDENTITIES has landed. Task 0 writes that sha into the ledger `.superpowers/sdd/2026-10-03-focus3-v2/progress.md`, and every gate takes `--base <that sha>` (PRINCIPLES 22). Until then, the plan is anchored to trunk `25203b8`. The line numbers it quotes are from there.
- **Not in this batch:** F-83 (build-from-raw leaves the server). The admission door sits inside `atlas_etl::concord::read_all`, which both the compile tool and `--build-from-raw` call, so no served path escapes it.
- **Also not in this batch:** F-40 (LFS), the Year spec, and FOCUS-5's event side.

## 2. Global constraints

- **C# is frozen.** A C# file changes only when generated types moved under it and it would otherwise fail to compile, or a gate would fail.
  - Each such change is listed in its task under "C# compile minimum".
  - It deletes or adapts. It never adds behaviour.
  - A C# view whose data this batch retires is deleted, not ported: the catechism item popover.
- **Legacy routes the frozen C# client reads stay, byte-stable:** `/api/text?ref=` (owner: frozen for its three users), `/api/chapter`, `/api/books` and `/api/xrefs/{sref}`. A pact interaction pins each one, unchanged. They retire with the C# client (backlog B15), not here.
  - **Exception:** a route whose data this batch deletes goes with that data: `/api/catechism/{sref}` and `/api/catechism/item/{id}` (Task 7).
- **Rule 27.**
  - Every derivation over the data alone is compiled: part roles, admission, numbering agreement, citation labels, container levels, passages, the corpus description.
  - Every per-request derivation is one bounded, indexed read: the text page and the whole-chapter read.
  - The server derives nothing else.
  - No client word (explore, frontier, card, popover, presentation) enters `server/` or `graph-types/`.
- **Relations.**
  - **No relation is appended.** `Explains => "explains" / "explained-by"` is the name the owner chose for the future explanation layer. It is appended in the batch that ingests an explanation source, because an empty relation now would be dead code (rule 4).
  - **No relation is removed.** `CatechismLink` keeps its ordinal; its row type narrows (Task 7). `TemporalAdjacency` is FOCUS-5's.
- **Tests:**
  - whole-body assertions;
  - one behaviour per test, named as a sentence;
  - `// Arrange` / `// Act` / `// Assert` only;
  - named constants;
  - newspaper order;
  - real-data expectations read from the artifact, never typed as literals.
- **Every law in §3.3 is a test, red first.** Each is named there by its test function. A law over generated inputs is a `proptest` property; a law over the real corpus is a real-data test that walks every member.
- **No comments in application code** (rule 9). A line this batch rewrites loses its comment; no other comment is touched.
- **Commits:** one per task on `lane/claude/F3v2-<task>`, integrated on `lane/claude/F3v2-int`. Landing is by squash under `land`, after Codex's review. Never force.

---

## 3. The design: domain, data structures, algebras (for sign-off, PRINCIPLES 12)

### 3.1 The domain types

These are the things a reader of the Bible and the Book of Concord already knows, and the code's names for them.

| Domain word | Meaning | Type |
|---|---|---|
| **Corpus** | the Bible, or the Book of Concord | `Corpus` (existing closed vocabulary) |
| **Book** | a book of the Bible, or a document of the Book of Concord. The owner: "treat each Concord document like a Bible book" | `Container` at level `Book` |
| **Chapter** | a chapter of a Bible book, or an article of a Concord document | `Container` at level `Chapter` |
| **Section title** | a title printed before a chapter that opens a group the reader never navigates as a level: the Smalcald Articles' Parts I–III. The owner: parts "appear only as titles" | `SectionTitle` on a chapter |
| **Unit** | one verse, or one Concord paragraph | `TextUnit` node (existing) |
| **Passage** | a run of consecutive units of one corpus. The owner: cited ranges "are passages", and an event's account is "a passage marked as recording history" | `Container` at level `Passage` with a `Passage` value |
| **Part** | a labelled piece of a unit's text: a heading the source prints, the body text, or a Triglot bracket | `TextPart { role, start, end }` |
| **Reference** | the code for a unit, chapter or range: `GEN.1.1`, `GEN.29.32-30.24`, `BoC 4.4.48` | A-WIRE-IDENTITIES' named references, per shape |
| **Citation label** | how a Concord paragraph is labelled: the Triglot citation code (`Ap IV 48`), or the article alone where our numbering differs from the Triglot's (residual question 2) | compiled `label` |
| **Admission** | the proof that a served Concord span is 1921 Triglot text | `Admission` row per piece |
| **Reading window** | the units of one chapter (or passage) a reader holds: pages of 20, at most 40 held, or the whole chapter | `TextPage` (served) plus the F# window (§7) |
| **Focus** | the chapter being read. Stepping to the next chapter and scrolling into it are one change | follows the `follows-in` link (F# client); the server guarantees the two agree (law S1) |

`graph-types/src/container.rs` (new):

```rust
pub enum ContainerLevel {
    Corpus,
    Book,
    Chapter,
    Passage,
}

pub enum Container {
    Corpus { title: String, description: String },
    Book { title: String },
    Chapter { title: String, section_title: Option<SectionTitle> },
    Passage(Passage),
}

pub struct SectionTitle(String);

impl Container {
    pub fn level(&self) -> ContainerLevel;
    pub fn title(&self) -> Cow<'_, str>;
}
```

- **`Container::title`** is the stored title for `Corpus`, `Book` and `Chapter`. For a `Passage` it is the passage's code (`Passage::reference`), never a curated string (F5 Q8: labels stay codes).
- **`NodePayload::Container { title }` becomes `NodePayload::Container(Container)`.**
- **`decode_book_container` and every other read of a container's level from its id prefix are deleted.** Today these are `bible_container_adapter.rs:55` and `book_detail`. The old plan's FINDING "a container has no kind" closes here.

`graph-types/src/passage.rs` (new):

```rust
pub struct Passage {
    span: PassageSpan,
    mark: PassageMark,
}

pub enum PassageSpan {
    Bible { first: VerseRef, last: VerseRef },
    Concord { first: ConcordRef, last: ConcordRef },
}

pub enum PassageMark {
    Plain,
    RecordsHistory,
}

pub enum PassageError {
    Backwards,
    OneUnit,
}

impl Passage {
    pub fn new(span: PassageSpan, mark: PassageMark, spine: &ReadingSpine) -> Result<Passage, PassageError>;
    pub fn span(&self) -> &PassageSpan;
    pub fn mark(&self) -> PassageMark;
    pub fn reference(&self) -> PassageCode;
    pub fn contains(&self, other: &Passage, spine: &ReadingSpine) -> bool;
    pub fn join(&self, other: &Passage, spine: &ReadingSpine) -> Option<Passage>;
}

pub fn passage_container_id(span: &PassageSpan) -> ContainerNodeId;
```

- **Fields are private, and `new` is the only constructor.** The span's two ends belong to one corpus by construction (`PassageSpan`'s variants). `new` refuses a span whose first unit comes after its last (`Backwards`) and a span of one unit (`OneUnit`: a one-unit citation ends at the unit itself).
- **The id is a function of the span alone**, never of the mark, so one span is one node however many facts mint it.
- **`PassageCode`** is A-WIRE-IDENTITIES' union of the passage-shaped references: `PassageReference` (`GEN.1.1-5`) and `VerseRangeReference` (`GEN.29.32-30.24`). For Concord it is a `ConcordRangeReference` leaf, `BoC 4.4.48-50`, added to that catalogue here. Its pattern is `^BoC {P}\.{P}\.{P}-{P}$` within one article, or `^BoC {P}\.{P}\.{P}-{P}\.{P}\.{P}$` across articles. Nothing in FOCUS-3 mints a Concord passage, so the leaf exists only if Task 6's Concord-citation walk finds a Concord range. Otherwise it is not added (rule 4); Task 0 records which.
- **`PassageMark::RecordsHistory`** is the owner's "property of recording history". FOCUS-3 mints `Plain` passages (cited ranges). FOCUS-5 mints `RecordsHistory` passages (each account) and groups them under their event. FOCUS-3 owns the merge law P5, so FOCUS-5 inherits a passage type that already carries the mark.

`graph-types/src/text.rs` (amended; names from the catechism model spec §3.2, with the roles the owner's rulings leave):

```rust
pub enum TextPartRole {
    Heading,
    Text,
    Bracket,
}

pub struct TextPart {
    pub role: TextPartRole,
    pub start: u32,
    pub end: u32,
}

pub struct Parts(Vec<TextPart>);

pub struct Rendering {
    text: String,
    parts: Parts,
}

impl Rendering {
    pub fn whole(text: String) -> Rendering;
    pub fn compose(pieces: &[Piece]) -> Rendering;
    pub fn text(&self) -> &str;
    pub fn parts(&self) -> &Parts;
    pub fn pieces(&self) -> Vec<Piece>;
}

pub struct Piece {
    pub role: TextPartRole,
    pub text: String,
}

pub type LayerMap = BTreeMap<TranslationId, Rendering>;
```

- **There is no `Question` and no `Answer` role.** The owner: "Luther's questions should always be included. No toggle there." A role nothing hides or shows differently is a distinction no test needs (rule 2). Luther's questions are `Text`.
- **`Bracket`** is a Triglot bracket. The owner: "hidden by default with a toggle"; the toggle is the client's (§7 B8).
- **`Heading`** is a title the source prints inside the unit ("The First Commandment.", "Secondly.", STATUS CONTROVERSIAE). It is not `TextUnit.heading` (`UnitHeading`), which is the event above a verse; the two are different facts (as Amendment A said).

`graph-types/src/admission.rs` (new; the F-84 domain):

```rust
pub struct TriglotLine(u32);

pub enum Admission {
    Matched { from: TriglotLine, to: TriglotLine, coverage: Coverage },
    Confirmed { at: TriglotLine, page: TriglotPage, reason: ConfirmationReason },
    Editorial { reason: EditorialReason },
}

pub struct Coverage { shared: u32, total: u32 }
pub struct TriglotPage(u16);

pub enum ConfirmationReason {
    OcrDamage,
    GreekOrLatin,
    Signature,
}

pub enum EditorialReason {
    OurTitle,
    CorpusDescription,
}

pub enum NumberingAgreement {
    Agrees,
    ArticleOnly { first_mismatch: u16 },
}
```

- **`Matched`** is the proof by shingles: the span's 4-word shingles, normalized, against the vendored Triglot OCR within its article's aligned window, at or above the threshold `θ` stored in data.
- **`Confirmed`** is a hand check, recorded in data with the Triglot page. It covers text the OCR garbles (residual question 1).
- **`Editorial`** is our own CC0 wording: the curated article titles and the corpus description. It is never Concord body text, which is enforced by a law (A6).
- **There is no "unadmitted" variant.** A span without an admission refuses the compile.

### 3.2 The data structures that hold them: invariants and bounds

#### D1. The container forest and the reading spine (shared by both corpora)

| Structure | Where it lives | Invariants | Bounds |
|---|---|---|---|
| `Container` nodes | `NodePayload::Container(Container)`, rows in the corpus's section | **I1** each corpus has exactly one `Corpus` container. **I2** every `Book` is `contains`-held by its corpus root, and every `Chapter` by one `Book`. **I3** the root-reached containers form a forest (`container_containment_is_a_forest`, restated over root-reached containers). **I4** a `Chapter` holds only units, and a `Book` only chapters | Bible: 66 books, 1,189 chapters. Concord: 10 documents; articles as compiled (134 at `25203b8`) |
| `contains` / `member-of` | `Contains` rows (existing) | **I5** every unit has exactly one root-reached `member-of` (its chapter). **I6** a chapter's units are contiguous in reading order | — |
| `follows-in` / `precedes-in` | `Succession` rows (existing) | **I7** at each level (`Book`, `Chapter`) of each corpus, `follows-in` is one chain over every container of that level, in reading order. **This changes Concord**: today articles succeed only within a document (`concord_adapter.rs:146–149`). Under "a document is a book", Augsburg Confession XXVIII `follows-in` the Apology's first article, exactly as GEN 50 `follows-in` EXO 1 | — |
| `ReadingSpine` | compiled, per corpus: the units in reading order with their ordinal (exists as `contains_bible_locus.ord` / the Concord spine) | **I8** a unit's spine ordinal increases along chapters in `follows-in` order | 31,102 verses; 3,802 Concord paragraphs at `25203b8` (more after Task 2, no fewer) |
| `container_detail` (new derived table, under A-F39's hash) | per container: `level`, `section_title`, and for a passage its span and mark | **I9** derived once from `NodePayload`; the dump verifies it | one row per container |

Wire (`server/atlas-contract/src/wire/graph.rs`, `wire/contents.rs`), built only from `container_detail`:

```rust
pub struct NodeRecord {
    pub id: NodeId,
    pub kind: NodeKind,
    pub label: String,
    pub provenance: Provenance,
    pub edge_summary: Vec<EdgeSummaryEntry>,
    pub version: ArtifactRoot,
    pub container: Option<ContainerDetail>,
    pub text: Option<UnitText>,
}

pub struct ContainerDetail {
    pub level: ContainerLevel,
    pub section_title: Option<String>,
    pub passage: Option<PassageDetail>,
}

pub struct PassageDetail {
    pub code: PassageCode,
    pub first: TextRef,
    pub last: TextRef,
    pub mark: PassageMark,
}

pub struct Contents {
    pub corpus: Corpus,
    pub title: String,
    pub description: String,
    pub roots: Vec<ContentsRoot>,
    pub version: ArtifactRoot,
}

pub struct ContentsRoot {
    pub id: NodeId,
    pub title: String,
    pub kind: ContentsRootKind,
    pub group: Option<Testament>,
    pub r#ref: ContentsReference,
    pub locus: TextRef,
    pub children: Vec<ContentsChild>,
}

pub struct ContentsChild {
    pub id: NodeId,
    pub title: String,
    pub kind: ContentsChildKind,
    pub section_title: Option<String>,
    pub r#ref: ContentsReference,
    pub locus: TextRef,
    pub count: usize,
}
```

- `NodeRecord` lists only the fields this batch touches; the others stand as at the base. `ContainerLevel` and `PassageMark` are published through `vocabulary!`, so their wire names derive from the Rust enums.
- **The shared interface on the wire is the same shapes for both corpora.** `ContentsRootKind {Book, Document}` and `ContentsChildKind {Chapter, Article}` stay only as the display noun each corpus uses. Structure and behaviour never branch on them; a source law over `server/` proves it (laws N5 and N7).
- **`Contents.description`** is the Concord intro moved into data (`data/curated/concord-corpus.toml`), plus the Bible root's description. The latter is empty unless curated; an empty description is the absence of one, and `description` becomes `Option<String>` if Task 4 finds no Bible description to serve.
- **`ContentsChild.count`** stays (the whole-chapter read's size, below). The client shows no counts (SIDENAV-PAGENUM-1, §7 B14).

#### D2. The passage

| Structure | Invariants | Bounds |
|---|---|---|
| `Passage` node + `Contains` rows to each unit of its span | **P-I1** a passage holds exactly the units `spine[first..=last]`, in order. **P-I2** no root reaches a passage (it is never a `member-of` target in the forest; a unit's root-reached parent stays unique). **P-I3** the id is `passage_container_id(span)`. **P-I4** a passage spans at least two units. **P-I5** both ends are in one corpus (by type) | one node per distinct cited span: Task 6 Step 2 records the count. The largest span is checked against `LARGEST_PASSAGE` (data-derived, compiled, gated at 10×) |
| `Cites` rows whose target is a range | **P-I6** a `cites` row whose source span covers two or more units ends at the passage node, never at its first unit. **P-I7** the same span cited from the Bible (TSK cross-references) and from the Book of Concord ends at the **same** node | — |

`server/atlas-graph/src/passages.rs` (new, compile side; the one door that mints passages):

```rust
pub struct PassageMint<'g> { graph: &'g mut Graph, spine: &'g ReadingSpine }

impl PassageMint<'_> {
    pub fn target(&mut self, span: PassageSpan, mark: PassageMark) -> Result<CitationTarget, PassageError>;
}

pub enum CitationTarget {
    Unit(TextLocus),
    Passage(ContainerNodeId),
}
```

- `target` answers `Unit` for a one-unit span and `Passage` otherwise. It mints on first sight and merges marks after that (P5).
- Both `xref_adapter` (Bible cross-references) and `citations::cite_scripture` (the Book of Concord's Bible citations, `citations.rs:45–65`) call it, so `to_last` is read in exactly one place.
- FOCUS-2's span-label special case (`object_label` reading `target_display`, `citations.rs:80`) is deleted. A passage is a node with a compiled label like any other.

#### D3. The reading window: the text page and the whole-chapter read

`GET /api/node/{id}/text`. Rule 27a classes it as the "range read by passage span": one bounded read, with no view in it.

```rust
pub struct TextPageQuery {
    pub cursor: Option<EdgePageCursor>,
    pub limit: Option<usize>,
    pub extent: Option<TextExtent>,
}

pub enum TextExtent {
    Page,
    Chapter,
}

pub struct TextPage {
    pub container: NodeRef,
    pub units: Vec<TextUnit>,
    pub previous: Option<EdgePageCursor>,
    pub next: Option<EdgePageCursor>,
    pub version: ArtifactRoot,
}

pub enum TextRefusals {
    NotFound,
    NotAContainer,
    BadCursor,
    WholeTakesNoCursor,
}

#[utoipa::path(get, path = "/api/node/{id}/text", params(("id" = NodeId, Path), TextPageQuery), responses((status = 200, body = wire::TextPage), TextRefusals), tag = "graph")]
pub async fn node_text(State(graph): State<GraphService>, Path(id): Path<String>, Query(query): Query<TextPageQuery>) -> Result<Json<wire::TextPage>, ApiError>;
```

- **The cursor is an `EdgePageCursor`, not a third cursor type.**
  - The page is the container's `contains` neighbours that are units, in `contains` order, read through the same keyset index as `/api/node/{id}/edges?kind=contains`.
  - So its cursor counts the same thing in the same space, and O2's "two cursor types" stands.
  - A law pins it: the text page's `next` equals the `contains` edge page's `next` at the same cursor and limit (W7).
- **Each `TextUnit` is built by FOCUS-2's one `unit_text` builder.** It carries `node`, `body: UnitText` (now with `parts`), `heading` (the event heading above a verse) and `edge_summary`. The owner's Q1, "one request per page (verses + headings)", is exactly this.
- **`extent=page`**, the default, takes `limit`, clamped to `LARGEST_PAGE` (200, the server's).
- **`extent=chapter`** is the owner's "Whole chapter": every unit of the container in one response, `next` and `previous` both `None`. Its bound is not the page cap. It is the compiled `LARGEST_CHAPTER_TEXT`, the unit count of the largest `Chapter`- or `Passage`-level container. That is a data fact (rule 26), read from `edge_count` and asserted by law W5. The timing gate measures it at 10× (27f).
- **A container that holds containers** (corpus, book) answers an empty page. **A non-container** is `NotAContainer`. **`extent=chapter` with a cursor** is `WholeTakesNoCursor`.
- **`TextUnit.ref` and `UnitText.parts`:** `ref` is A-WIRE-IDENTITIES' `UnitReference`. `UnitText` gains `parts: Vec<TextPart>`; the wire `TextPartRole { Heading, Text, Bracket }` is a `vocabulary!` over the graph-types enum.
- **"Somewhere to go"** (owner Q10) is decided by data the page already serves: each unit's `edge_summary`. The rule over it is the client's affordance (§7 B7). No extra field is needed, and law W8 guarantees the summary is on every unit of every page.

#### D4. The Concord unit and its labelled parts

| Structure | Where | Invariants | Bounds |
|---|---|---|---|
| `SourceNode { block, marker, text }` (private to `atlas-etl::concord`) | the one door that reads an article body (`source_nodes`) | **C-I1** every text node of every article body appears exactly once, in source order | 10 pages + 23 Smalcald sub-pages |
| `ConcordParagraph { paragraph: SourceMarker, source_label, pieces: Vec<Piece> }` | ETL output | **C-I2** `paragraph` is the source's own marker. No curated file writes a paragraph number (owner: "never curated"). **C-I3** pieces stay at source-node granularity; brackets are split out of text nodes into `Bracket` pieces | — |
| `Rendering { text, parts }` in the unit's `LayerMap` | graph | **C-I4** `parts` partition `[0, len(text))`: ordered, gapless, non-overlapping, non-empty, adjacent parts of different roles (normal form). **C-I5** a Bible verse is `Rendering::whole`, and its canonical bytes are unchanged (no Bible pid moves) | — |
| `data/curated/concord-roles.toml` | the ~16 Triglot-checked role corrections (A1) | **C-I6** each row names exactly one source node; its role differs from the markup's default (a correction that changes nothing fails the read) | about 16 rows |
| `data/curated/concord-exclusions.toml` (exists, A-LICENSE-BOC) | exclusions by name with a reason | **C-I7** each entry names exactly one article or one run that occurs exactly once | — |
| `data/raw/triglot/concordiatriglot00unse_djvu.txt` (new, vendored, PD) + `MANIFEST.toml` row | the admission reference | **C-I8** its sha256 is pinned in `MANIFEST.toml`; the compile refuses any other bytes | about 9 MB |
| `data/curated/concord-admission.toml` (new) | `θ`, the per-document article alignment anchors, and every `Confirmed` / `Editorial` disposition | **C-I9** every disposition names exactly one served span; a disposition for a span that also `Matched` fails the read (no redundant data) | Task 3 Step 2 records the count |
| `admission` (new derived table) | one `Admission` per served piece and per served title | **C-I10** total: every piece and every title has exactly one | one row per piece |
| `article_numbering` (new derived table) | `NumberingAgreement` per article | **C-I11** computed from the admission alignment: the Triglot's marginal paragraph numbers inside each unit's aligned window, against the source marker | one row per article |
| `data/curated/concord-citations.toml` (new) | per document its Triglot abbreviation (`AC`, `Ap`, `SA`, `Tr`, `SC`, `LC`, `Ep`, `SD`); per article its Triglot designation (`IV`); per Smalcald part its section title | **C-I12** the file holds conventions and titles, never a paragraph number | 10 documents |

`server/atlas-etl/src/concord.rs` signatures (`read_all` keeps its signature):

```rust
pub struct ConcordParagraph {
    pub paragraph: SourceMarker,
    pub source_label: String,
    pub pieces: Vec<Piece>,
}

pub struct SourceMarker(u16);

pub struct ConcordStats {
    pub excluded: Vec<ExcludedText>,
    pub corrected_roles: usize,
    pub admitted: AdmissionStats,
}

pub struct AdmissionStats {
    pub matched: usize,
    pub confirmed: usize,
    pub editorial: usize,
}

pub enum AdmissionRefusal {
    Unmatched { document: &'static str, article: String, paragraph: SourceMarker, piece: String, coverage: Coverage },
    StaleDisposition { document: &'static str, article: String, span: String },
    ReferenceChanged { expected: String, found: String },
}

pub fn admit(paragraphs: &[ConcordParagraph], reference: &TriglotReference, policy: &AdmissionPolicy) -> Result<Vec<Admission>, Vec<AdmissionRefusal>>;

struct SourceNode { block: SourceBlock, marker: Option<SourceMarker>, text: String }
enum SourceBlock { Heading, BoldParagraph, Paragraph }
fn source_nodes(body: &str) -> Vec<SourceNode>;
```

- **`admit` returns every refusal, not the first**, so one compile lists them all.
- **`TriglotReference`** is the OCR text, normalized once: hyphenation joined, case folded, punctuation dropped, then shingled. It lives in `atlas-etl`, the tool layer (26a); no served crate links it.
- **`AdmissionPolicy`** is `concord-admission.toml` read.
- **The default role is the markup's:** a `Heading` or `BoldParagraph` node without a marker is `Heading`; every other node is `Text`. `concord-roles.toml` overrides it. A `[`…`]` run inside a `Text` node becomes a `Bracket` piece. Nothing else splits a node.

Compiler (`server/atlas-graph/src/concord_adapter.rs`, `labels.rs`):

```rust
pub fn paragraph_node(unit: ConcordRef, rendering: Rendering) -> Node;
pub fn paragraph_label(citations: &ConcordCitations, numbering: NumberingAgreement, unit: &ConcordRef) -> Result<String, UncitedParagraph>;
```

- The label is the Triglot citation code: `Ap IV 48`, or `Ap IV` where `NumberingAgreement::ArticleOnly`. Residual question 2 asks the owner to confirm this reading of "labels stay codes" for the Concord.
- The module is `server/atlas-graph/src/triglot_citations.rs`. `citations.rs` already holds the Scripture scanner; Amendment A found the clash.
- `ConcordReference` (`BoC 4.4.48`) stays the unit's reference and id, never its label.

#### D5. The catechism (what is left when `CatechismItem` retires)

| Fact | Holder after FOCUS-3 | Source and license |
|---|---|---|
| The catechism's words | the Small Catechism paragraphs (part 7), with their parts | 1921 Triglot, PD; admitted (D4) |
| Luther's own Bible citations (`catechism.toml` `verses` + `ref_note`) | `Cites` rows from the paragraph, read from `data/curated/concord-luther-citations.toml` (moved rows, keyed by source marker), ending at a unit or passage through `PassageMint` | ours, CC0 |
| The Decalogue's wording (`catechism-deut5.toml`) | `Quotes` rows from 7.2.1–10 to Exodus 20 / Deuteronomy 5 | ours, CC0 |
| **The brain-fuel proof-verse links (kept: owner exception pending a license, O-CATECHISM)** | **`CatechismLink` rows, re-homed from the item to the Small Catechism paragraph(s) the item named.** The row type narrows to `{ verse: BibleLocus, paragraph: ConcordLocus }`. The rows move from `RowFamily::Catechism` in `core` to the `concord` section, beside the paragraphs they name. The data moves from `catechism-mapping.toml`'s `item = "commandment-1"` to `article`/`paragraphs` keys, by a one-time rewrite through `concord-sc-overlap.toml` (Task 7 Step 3). An item with two paragraphs (baptism-1, baptism-2, baptism-4, altar-1) attaches its verses to **both**, so each paragraph shows them with no hop. The topic titles are not served (they were read only by `/api/catechism/item`) | brain-fuel, unlicensed; `LICENSES.md` records the owner exception and its condition |
| An explanation layer | none yet (owner: "Neither for now") | — |

```rust
pub struct CatechismLink {
    pub verse: BibleLocus,
    pub paragraph: ConcordLocus,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}
```

- **Retired:** `NodeKind::CatechismItem`, `CatechismItemId`, `NodePayload::CatechismItem`, `catechism.toml` (its words; its citations move), `concord-sc-overlap.toml` (consumed by the one-time rewrite, then deleted, because the F# client reads only v3 saves), and `atlas-core`'s `CatechismItem`, `CatechismQuestion`, `CatechismPart`, `catechism_items_for_span` and the verse→catechism index.
- **The F-76 hop disappears with the second node.** A paragraph opens itself and lists `catechism-link`, `cites` and `quotes` as its own groups.

### 3.3 The algebras: every operation and its laws

Each law below is a test, named in brackets. **P** means a `proptest` property over generated graphs or spans; **R** means a real-data test that walks every member of the real artifact; **S** means a source law over the code. A law that holds for both corpora is **one test parametrised over `Corpus::ALL`**, never two copies. That is how "1 should be a shared interface" is proven.

#### A1. Container navigation: `next`, `prev`, `parent`, `children`

Operations, for a level `L ∈ {Book, Chapter}` of a corpus `K`:
- `next_L(c)`: the `follows-in` neighbour, if any;
- `prev_L(c)`: the `precedes-in` neighbour, if any;
- `parent(c)`: the root-reached `member-of`;
- `children(c)`: `contains`, in order.

| # | Law | Kind | Test |
|---|---|---|---|
| N1 | **Inverse:** `next(c) = Some(d) ⇔ prev(d) = Some(c)` | R, P | `next_and_prev_are_inverse_at_every_level_of_every_corpus` |
| N2 | **One chain:** from the unique `c₀` with `prev(c₀) = None`, iterating `next` visits every container of level `L` in `K` exactly once and ends at the unique `c` with `next(c) = None` | R | `each_level_of_each_corpus_is_one_chain_in_reading_order` |
| N3 | **Order:** the chain's order is the reading spine's order (I8): `spine_first(next(c)) > spine_last(c)` | R | `the_chain_order_is_the_reading_order` |
| N4 | **Chapter steps respect books:** `parent(next_Chapter(c)) ∈ { parent(c), next_Book(parent(c)) }`, and it is the latter exactly when `c` is its book's last chapter | R | `a_chapter_step_stays_in_its_book_or_enters_the_next_book_at_its_first_chapter` |
| N5 | **Corpus-blind:** N1–N4 run through one generic function over `Corpus::ALL`; no navigation code branches on `Corpus` or on a `ContentsRootKind`/`ContentsChildKind` | S | `no_navigation_code_matches_a_corpus_or_a_display_kind` |
| N6 | **Closed under the corpus:** `next` never leaves `K` | R | inside N2's walk |
| N7 | **Contents agrees with the graph:** `/api/contents/{K}`'s roots are the `Book` chain and each root's children its `Chapter` children, both in chain order; each child's `count = |children|` | R | `the_contents_tree_is_the_book_and_chapter_chains_and_each_count_is_the_chapters_units` |
| N8 | **Level is data:** `level(c)` is read from `container_detail`; no served code reads a container id's prefix | S | `no_served_code_decodes_a_container_level_from_its_id` |

#### A2. Passage: containment and composition

Operations:
- `units(p) = spine[first..=last]`;
- `p ⊑ q` (containment);
- `p ⊔ q` (`join`, defined when the two overlap or abut);
- `mint(span, mark)`.

| # | Law | Kind | Test |
|---|---|---|---|
| P1 | **Holds its span:** `contains(p) = units(p)`, whole and in order | R (every minted passage), P | `a_passage_holds_exactly_the_units_of_its_span_in_order` |
| P2 | **Containment is the span order:** `p ⊑ q ⇔ q.first ≤ p.first ∧ p.last ≤ q.last ⇔ units(p) ⊆ units(q)`. It is reflexive, antisymmetric (`p ⊑ q ∧ q ⊑ p ⇒ id(p) = id(q)`) and transitive | P | `containment_is_a_partial_order_and_agrees_with_unit_inclusion` |
| P3 | **Join is a semilattice on touching spans:** when defined, `p ⊔ q` has span `(min first, max last)`; it is commutative, associative and idempotent; `p ⊑ p ⊔ q`; and `units(p ⊔ q) = units(p) ∪ units(q)` | P | `join_is_commutative_associative_idempotent_and_the_least_upper_bound` |
| P4 | **Join is partial exactly at a gap:** `p ⊔ q = None ⇔` a unit lies strictly between them on the spine; joining across corpora does not type-check | P | `join_is_undefined_exactly_when_a_unit_separates_the_spans` |
| P5 | **Minting is idempotent and the mark only rises:** `mint(s, m₁); mint(s, m₂)` gives one node with mark `max(m₁, m₂)` under `Plain < RecordsHistory`, in either order | P | `minting_one_span_twice_makes_one_passage_whose_mark_is_the_greater` |
| P6 | **One unit is not a passage:** `target(span)` with `first = last` is `Unit(first)`, and no node is minted | P | `a_one_unit_span_targets_the_unit_and_mints_nothing` |
| P7 | **Passages stay out of the forest:** no root reaches a passage; every unit's root-reached `member-of` stays unique | R | `no_corpus_root_reaches_a_passage_and_every_unit_keeps_one_chapter` |
| P8 | **One door:** only `PassageMint` constructs a `Passage`-level container; only it reads `to_last` | S | `only_the_passage_mint_builds_a_passage_or_reads_a_citations_last_unit` |
| P9 | **Labels are codes:** `label(p) = p.reference()` (`GEN.29.32-30.24`); no passage label is curated | R | `every_passage_is_labelled_by_its_code` |

#### A3. Citations resolve to the same units everywhere

| # | Law | Kind | Test |
|---|---|---|---|
| R1 | **Same span, same node:** a Bible span cited by a TSK cross-reference and by a Book of Concord paragraph ends at the same node (`PassageMint::target` is the one function) | R | `the_bible_and_the_book_of_concord_cite_one_span_at_one_node` |
| R2 | **Translation-neutral:** every `cites` target is a verse `TextUnit` or a Bible `Passage`. Its id is independent of any `TranslationId` (renderings are layers on the node), so every Bible reference is explorable by the same reads whichever translation is active | R | `every_bible_citation_from_the_book_of_concord_ends_at_a_verse_or_passage_node_of_the_bible` |
| R3 | **Every Concord Bible citation resolves or is listed:** a scanned citation that places no unit is counted in `CitationStats.unplaced`, and the list is a whole-body pin | R | `the_unplaced_concord_citations_are_exactly_the_listed_ones` |

#### A4. The reading window

Server operations:
- `page(c, k, n)`, returning `(units, prev, next)`;
- `whole(c)`.

| # | Law | Kind | Test |
|---|---|---|---|
| W1 | **Walking forward is the container:** the pages from `FIRST` by `next` concatenate to `units(c)`, each exactly once; no unit of another container appears (PAGE-BOUNDARY-BUG-1's category, closed by addressing a page by its container) | R (every chapter, article and passage), P | `walking_a_containers_pages_forward_yields_its_units_once_each_and_nothing_else` |
| W2 | **Back inverts forward:** for every page `π` with `prev(π) = Some(k')`, `page(c, k', n).next = π.cursor` | R, P | `the_page_before_a_page_leads_back_to_it` |
| W3 | **Ends:** only the first page has no `previous`; only the last has no `next` (F-74) | R | `only_the_first_page_has_no_previous_and_only_the_last_has_no_next` |
| W4 | **Size:** `|units| = min(n, LARGEST_PAGE, remaining)` | P | `a_page_holds_the_asked_number_clamped_to_the_cap_and_the_remainder` |
| W5 | **Whole chapter:** `whole(c) = units(c)` in one response with no cursors; `|whole(c)| ≤ LARGEST_CHAPTER_TEXT` for every chapter and passage; and `LARGEST_CHAPTER_TEXT` equals the compiled maximum | R | `the_whole_chapter_read_is_every_unit_of_the_chapter_within_the_compiled_bound` |
| W6 | **Pure:** an answer is a function of `(root, c, cursor, limit, extent)`; two reads are byte-equal | P | `a_text_page_is_a_pure_function_of_its_request` |
| W7 | **One cursor space:** the text page's `next` and `previous` equal the `contains` edge page's at the same cursor and limit | R | `a_text_page_and_its_containers_contains_page_share_their_cursors` |
| W8 | **Each unit carries its headings, parts and summary:** each unit on a page equals the unit `/api/text` and the element read serve for that node, whole | R | `a_unit_on_a_text_page_is_the_unit_every_other_read_serves` |

The client window's algebra (append, slide, back, whole) is in §7 B2, as laws the F# client's tests check. It rests on W1–W3 and W5.

#### A5. Focus: a scroll is a step

| # | Law | Kind | Test |
|---|---|---|---|
| S1 | **Spine and chain agree:** for every chapter `c` with `next(c) = Some(d)`, the unit after `c`'s last unit on the reading spine is `d`'s first unit. The symmetric law holds for `prev`. So "scrolling past the end of `c`" and "the next-chapter step from `c`" name the same `d`, and the F# client can implement both as one `follows-in` change (§7 B3: `scrollInto d = step Next c`) | R (both corpora, one test) | `the_unit_after_a_chapters_last_is_the_first_unit_of_its_next_chapter` |

#### A6. The Concord text: parts, coverage, admission, numbering

Operations:
- `compose(pieces) → Rendering`;
- `pieces(rendering)`;
- `source_nodes(body)`;
- `admit(paragraphs)`;
- `numbering(article)`;
- `label(unit)`.

| # | Law | Kind | Test |
|---|---|---|---|
| T1 | **Partition:** for every rendering, the parts are ordered, gapless and non-overlapping, cover `[0, len)`, and are in normal form (no two adjacent parts share a role) | P, R | `every_rendering_is_partitioned_into_parts_in_normal_form` |
| T2 | **Round trip:** `compose(pieces(r)) = r`, and `pieces(compose(ps))` equals `ps` with adjacent same-role pieces merged | P | `composing_and_splitting_a_rendering_round_trip` |
| T3 | **Verses unchanged:** `Rendering::whole(t)` encodes canonically as `t` alone; no Bible pid moves | P + canon vector | `a_whole_text_rendering_encodes_as_its_text_alone` |
| T4 | **Coverage (F-78):** for every article of every page, the source body's words, in order, equal the words of its units' pieces interleaved with its named exclusions | R | `every_source_text_node_lands_in_exactly_one_served_unit_or_one_named_exclusion` |
| T5 | **Admission is total and refusing:** `admit` returns one `Admission` per served piece and per served title, or the compile fails listing every `AdmissionRefusal`; generated non-Triglot paragraphs (Codex's 16-probe shape, F-84) are refused | R, P | `every_served_concord_span_is_admitted_against_the_triglot`, `a_paragraph_the_triglot_does_not_print_is_refused` |
| T6 | **Admission is ordered:** within an article, the `Matched` windows' starts are non-decreasing along the units | R | `an_articles_admitted_spans_follow_the_triglots_order` |
| T7 | **Editorial is never body text:** an `Editorial` admission is only on a title or the corpus description | R | `no_paragraph_text_is_admitted_as_our_own_words` |
| T8 | **Numbers come from the source:** each unit's paragraph is its `SourceMarker`; no curated file contains a paragraph-number key | S, R | `no_curated_file_numbers_a_paragraph`, `every_paragraph_number_is_its_source_marker` |
| T9 | **Labels follow agreement:** `label(u) = cite(article, paragraph)` when `numbering(article) = Agrees`, else `cite(article)`; the `ArticleOnly` articles are a whole-body list in the close report | R | `a_paragraph_is_labelled_by_its_triglot_citation_or_its_article_where_numbering_differs` |
| T10 | **Roles:** a corrected node's role is the correction's; every other node's role is the markup default; the Creed's first article holds "I believe in God the Father Almighty, Maker of heaven and earth." as `Text` after "The First Article." as `Heading` | R | `the_creeds_first_article_keeps_its_title_as_a_heading_and_its_words_as_text`, `the_confession_form_keeps_both_confessions_and_the_absolution` |
| T11 | **Brackets are parts:** every `[`…`]` run of the source is exactly one `Bracket` part, and no `Bracket` part holds anything else | R | `every_triglot_bracket_is_one_bracket_part` |

#### A7. The catechism

| # | Law | Kind | Test |
|---|---|---|---|
| K1 | No node of kind `CatechismItem` exists; the generated `NodeKind` has no such case | S (compile) | (the enum) |
| K2 | Every `CatechismLink` joins a Bible verse and a Small Catechism paragraph (by type), and every re-homed row resolves, or the compile fails naming it | R | `every_kept_proof_verse_link_joins_a_verse_and_a_small_catechism_paragraph` |
| K3 | **Nothing is lost in the re-home:** the multiset of `(verse, item)` before equals the image of `(verse, paragraph)` after, under `concord-sc-overlap.toml`, with two-paragraph items doubled | R (one-time, in Task 7, recorded in the ledger, then deleted with the overlap file) | `the_re_homed_links_are_the_item_links_carried_to_their_paragraphs` |
| K4 | Luther's own citations are `cites` from their paragraphs; the Decalogue's are `quotes` | R | `luthers_citations_are_cites_from_his_paragraphs`, `the_commandments_quote_exodus_and_deuteronomy` |

---

## 4. Critical sections (PRINCIPLES 21)

| Section | Holder | Why |
|---|---|---|
| `contract`: rebuild #1 (`data/compiled`) | after Task 4: the Concord text pipeline (Tasks 2–4) | one artifact |
| `contract`: rebuild #2 + regen + re-bless | after Task 8: containers, passages, the catechism model, the text page (Tasks 5–8) | one artifact, one generated document |
| `heavy` | each task's Step "gates"; Task 9's full set | memory |
| `heavy` with "mutation" | Task 9 only, in the owner's window | once per batch |
| appending to `relations!` | **nobody** | |

**Version classes:**
- `SECTION_SCHEMA_VERSION` bumps once per rebuild. Rendering and the container payload change the row shape; the catechism retirement changes ordinals.
- graph-types: major.
- AQC: major (routes and the `CatechismItem` kind removed, required fields added). AGC: major (the catechism features retire). `scripts/contract-semver-gate.sh` decides; the plan expects major.

## 5. Tasks

Every task: red first (the named laws), then green, then the gates named, then a commit stating what is now true.

### Task 0: Ledger, base and the facts the plan assumes

- [ ] Write the ledger. Record:
  - the base sha (trunk after A-WIRE-IDENTITIES);
  - the landed shas of A-F39 and A-PROVENANCE;
  - that A-WIRE-IDENTITIES' named references are in `atlas_core::identity`.
- [ ] Verify each item below without building, and record it with its fallback:
  - `proptest` is available to `graph-types` and `atlas-graph`. Fallback: add it as a dev-dependency in Task 1.
  - The Triglot OCR is fetchable from archive.org (`concordiatriglot00unse_djvu.txt`), PD, and identical to the copy F-79 used (sha256). Fallback: stop and report; Tasks 3–4 wait.
  - `LARGEST_CHAPTER_TEXT`, read from the current artifact's `edge_count` for `contains` over chapter-level containers. The expected maximum is an Apology or Large Catechism article. Fallback: none needed; it is data.
  - Whether any Book of Concord citation names a Concord range. This decides whether `ConcordRangeReference` exists (§3.1).
  - The Smalcald part titles in `smalcald-sub/*.html`, and where the source prints them.
  - The origin of `Concord.razor`'s intro text (l.66–70). Ours means `Editorial`. Triglot means `Matched`. Anything else means it is not moved, and a FINDING is raised.

### Task 1: The domain types and their algebras (graph-types only; no rebuild)

**Files.**
- Create:
  - `graph-types/src/{container,passage,admission}.rs`;
  - `graph-types/tests/{passage_algebra,parts_algebra}.rs`.
- Modify:
  - `graph-types/src/{text,node,lib}.rs`;
  - `graph-types/src/canon/node.rs` (encoding of `Rendering` and `Container`);
  - `graph-types/tests/canon_vectors.rs`;
  - `graph-types/Cargo.toml` (major).

**Steps.**
- [ ] **Red:** P2, P3, P4, P5, P6 (generated spines of 1–3 corpora shapes); T1, T2, T3; `Container::level` total over its variants.
- [ ] **Green:** implement §3.1. Every reader of `renderings` reads `.text()`; `cargo build` enumerates them, about 20 files.
- [ ] **Gates:** `(cd graph-types && cargo test --all-features)`, `cargo build --workspace`.
- [ ] **C# compile minimum:** none (no wire change yet).
- [ ] **Commit:** `graph-types: a container has a level, a passage is a span with a mark and an algebra, and a rendering is text with a partition into labelled parts`.

### Task 2: The Concord source door: Luther's dropped text recovered and labelled (F-78, A1)

**Files.**
- Modify:
  - `server/atlas-etl/src/{concord,curated}.rs`;
  - `server/atlas-graph/src/concord_adapter.rs`.
- Create:
  - `data/curated/concord-roles.toml`;
  - `server/atlas-etl/tests/concord_real_data.rs` (extend if it exists).

**Steps.**
- [ ] **Red:** T4 (at the base: the 88 `<h4>`, the `<h5>` and the 28 runs are missing, about 1,700 words; the ledger lists them by article), T10, T11, and C-I6.
- [ ] **Green:**
  - Amendment A's A.2 closure, as carried here:
    - `source_nodes` is the one door;
    - `strip_complete_headings`, `strip_standalone_strong_paragraphs`, `split_on_paragraph_tags` and `clean_paragraph_text` are deleted;
    - assignment is total;
    - nothing mints or splits a unit;
    - `ConcordParagraph.text` is deleted.
  - Exclusions stay in A-LICENSE-BOC's `concord-exclusions.toml`, extended to runs. Its laws stay green.
  - The ~16 role corrections are each checked against the Triglot scan, with the page in the row.
  - Brackets become `Bracket` pieces.
- [ ] **Gates:** `cargo test -p atlas-etl`, `cargo test -p atlas-graph --lib`. Nothing is rebuilt yet: rebuild #1 follows Task 4.
- [ ] **C# compile minimum:** none.
- [ ] **Commit:** `concord: every source text node is served in exactly one paragraph or excluded by name, labelled Heading, Text or Bracket from the source markup with Triglot-checked corrections; the Creed's articles, the absolution and the daily prayers are read again (F-78, A1)`.

### Task 3: Positive Triglot admission (F-84)

**Files.**
- Create:
  - `data/raw/triglot/concordiatriglot00unse_djvu.txt` (+ `data/raw/MANIFEST.toml` row with sha256, `data/raw/README.md` entry);
  - `data/curated/concord-admission.toml`;
  - `server/atlas-etl/src/triglot.rs` (`TriglotReference`, normalization, shingles, alignment).
- Modify:
  - `server/atlas-etl/src/concord.rs` (`admit` inside `read_all`);
  - `data/curated/sources.toml` (the Triglot OCR source);
  - `LICENSES.md`.

**Steps.**
- [ ] **Red:**
  - T5, run on the current text (Task 2's output) and listing every refusal;
  - T5's generated half: Codex's 16 marker-free modern replacement paragraphs, each refused;
  - T6, T7, C-I8, C-I9.
- [ ] **Measure:** record the coverage distribution per document in the ledger. Set `θ` in `concord-admission.toml` at the lowest coverage among units hand-checked as Triglot, and never below the highest coverage among the known non-Triglot units of F-79. The two populations, with the margin between them, go in the ledger.
  - **If the populations overlap, stop and report.** The threshold is then a ruling, not a choice.
- [ ] **Dispositions:** every unit below `θ` is checked by hand against the scan. It gets `Confirmed` (with page and reason) or an exclusion (with reason). The curated titles get `Editorial`.
  - Residual question 1 decides whether `Confirmed` is allowed at all. If the owner says no, these units are excluded and listed.
- [ ] **Green:** `admit`, `AdmissionStats`, the `admission` table (served nowhere yet).
- [ ] **Gates:** `cargo test -p atlas-etl`.
- [ ] **C# compile minimum:** none.
- [ ] **Commit:** `concord: every served span and title is admitted against the vendored 1921 Triglot -- matched by shingles above a threshold kept in data, or confirmed by hand with its page -- and the compile refuses anything else (F-84)`.

### Task 4: Concord numbering, labels, section titles, intro, one chain; rebuild #1

**Files.**
- Create:
  - `data/curated/{concord-citations,concord-corpus}.toml`;
  - `server/atlas-graph/src/triglot_citations.rs`.
- Modify:
  - `server/atlas-etl/src/concord.rs` (`article_numbering`);
  - `server/atlas-graph/src/{concord_adapter,labels,corpus_root}.rs`;
  - `server/atlas-contract/src/contents.rs` (reads the compiled label; `"BoC "` joins `no_served_label_composition`);
  - `data/compiled`, pacts, fixtures.

**Steps.**
- [ ] **Red:**
  - T8, T9;
  - N1–N4 for Concord: red, because articles do not chain across documents;
  - `the_smalcald_articles_first_article_of_each_part_carries_its_part_title`;
  - `the_book_of_concords_description_is_served_from_data_once`.
- [ ] **Green:**
  - Compute `NumberingAgreement` per article from the admission alignment.
  - Labels via `paragraph_label`.
  - Chapter-level succession is one chain per corpus (I7).
  - Section titles from `concord-citations.toml`.
  - The intro moves into `concord-corpus.toml` as `Container::Corpus.description`.
- [ ] **Rebuild #1** (`contract` lock):
  - rebuild `data/compiled`, schema +1;
  - re-bless pacts and fixtures that quote grown Concord text or Concord labels;
  - record the `cites` count before and after (new text may add citations).
  - **If a golden map fixture moves, stop for the owner.**
- [ ] **Gates** (`heavy`): `cargo test --workspace`, graph-types, `bash scripts/contract-gate.sh --base <base>`.
- [ ] **C# compile minimum:** none expected. `UnitText` does not change on the wire until Task 8. If Concord label changes break a C# test that pins a label, the pin is re-read from the served artifact (F-8), not rewritten as a literal.
- [ ] **Commit:** `concord: paragraph numbers are the source's own and each paragraph is labelled by its Triglot citation, or its article where the numbering differs (listed); a document reads like a book, its articles one chain across documents, the Smalcald parts as titles, the introduction from data`.

### Task 5: The shared container abstraction in the graph and on the wire

**Files.**
- Modify:
  - `server/atlas-graph/src/{bible_container_adapter,concord_adapter,corpus_root,law_check}.rs` (payload `Container(Container)`, `container_detail`);
  - `graph-types/src/sections.rs` (the derived table, under A-F39's hash);
  - `server/atlas-contract/src/{wire/graph,wire/contents,graph,contents}.rs` (`ContainerDetail`, `Contents.description`, `section_title`).
- Create:
  - `server/atlas-graph/tests/navigation_laws.rs`.

**Steps.**
- [ ] **Red:** N1–N8, I1–I9, and S1 (both corpora, one test each, parametrised over `Corpus::ALL`).
- [ ] **Green:**
  - Levels are compiled.
  - `decode_book_container` and the id-prefix level reads are deleted.
  - `/api/contents` builds `ref` from A-WIRE-IDENTITIES' `ContentsReference`, `locus`, `title` and `section_title` from compiled rows. The old plan's FINDING "contents derives per request" closes here.
- [ ] **Gates:** `cargo test -p atlas-graph -p atlas-contract`. The contract is regenerated in Task 8's hold.
- [ ] **C# compile minimum:** with Task 8.
- [ ] **Commit:** `containers: a Bible book and a Concord document are one abstraction -- a level compiled with each container, one follows-in chain per level per corpus, a contents tree that is those chains -- and nothing reads a level from an id`.

### Task 6: Cited ranges are passages

**Files.**
- Create:
  - `server/atlas-graph/src/passages.rs` (`PassageMint`);
  - `server/atlas-graph/tests/passage_laws.rs`.
- Modify:
  - `server/atlas-graph/src/{xref_adapter,citations,labels,law_check}.rs`;
  - `graph-types/src/edge.rs` (if `Cites`' target type changes from a locus to `CitationTarget`, keeping canonical bytes for one-unit targets).

**Steps.**
- [ ] **Red:**
  - P1, P7, P8, P9, R1, R2, R3;
  - `a_cross_reference_to_a_span_ends_at_the_passage_that_holds_it` (RUT 4:11 → GEN 29:32–30:24, read from the artifact);
  - `a_cross_reference_to_one_verse_still_ends_at_the_verse`.
- [ ] **Green:**
  - One `PassageMint`. TSK cross-references and Concord citations both call it.
  - `object_label`'s span case and `target_display` as a label source are deleted.
  - Record the number of distinct passages and `LARGEST_PASSAGE` in the ledger.
- [ ] **Gates:** `cargo test -p atlas-graph`.
- [ ] **C# compile minimum:** with Task 8. Expected: none. A `cites` edge ending at `Container:passage-…` reaches C#'s `LegacyNodes.For` Container arm.
  - Task 9's Playwright run checks the C# app opens it on FocusView, not as a broken `ChapterNode`.
  - If it does not, the minimum is `LegacyNodes.For` answering `null` for a passage-level container. That is one arm, read from `NodeRecord.container.level`, never from the id.
- [ ] **Commit:** `passages: every cited range is one passage node, minted by one door from either corpus, labelled by its code, able to carry the mark of recording history; a one-verse citation still ends at the verse`.

### Task 7: The catechism model: the paragraphs are the catechism

**Files.**
- Modify:
  - `graph-types/src/{edge,node,id,sections}.rs` (`CatechismLink` narrowed, moved to `concord`; `CatechismItem` kind and payload deleted);
  - `server/atlas-graph/src/{catechism_adapter,concord_adapter}.rs`;
  - `server/atlas-core/src/{data,lib}.rs` (`CatechismItem*`, `catechism_items_for_span`, the index deleted);
  - `server/atlas-contract/src/{catechism,wire/catechism,lib}.rs` (routes and types deleted);
  - `data/curated/catechism-mapping.toml` (re-keyed to paragraphs);
  - `data/curated/sources.toml`, `LICENSES.md` (the owner exception);
  - `scripts/gate-selftest.sh`, `server/BENCHMARKS.md`.
- Create:
  - `data/curated/concord-luther-citations.toml`.
- Move:
  - `catechism-deut5.toml` rows → `Quotes`.
- Delete:
  - `data/curated/catechism.toml`;
  - `data/curated/concord-sc-overlap.toml`, after Step 3.

**Steps.**
- [ ] **Red:**
  - K2, K4;
  - `contract_coverage.rs` `no_route_serves_a_catechism_item_or_a_span_s_catechism`.
- [ ] **Re-home** (a one-time data rewrite, a tool in `scripts/`, not app code):
  - Rewrite `catechism-mapping.toml`'s `item` keys as `article`/`paragraphs` through `concord-sc-overlap.toml`.
  - Run K3 against the pre-change artifact's `(verse, item)` set, and record it green in the ledger.
  - Delete the overlap file, the tool and K3 in the same commit. K3's evidence lives in the ledger.
- [ ] **Green:** as §3.2 D5.
- [ ] **C# compile minimum** (with Task 8's regen):
  - delete `CatechismNode`, the `Catechism*Section`s, `CatechismLinks`, `CatechismList`, `AtlasClient.Catechism` and `.CatechismItem`, and `PassageCatechismSection`, with their registry rows and tests;
  - remove the `CatechismItem` arms that the generated `NodeKind` no longer has.
  - Nothing replaces them in C#. A paragraph on the frozen C# reader opens on FocusView and lists `catechism-link`.
- [ ] **Commit:** `catechism: the Small Catechism's paragraphs are the catechism -- CatechismItem and its routes are gone, Luther's citations are cites, the Decalogue quotes Exodus and Deuteronomy, and the brain-fuel proof verses (kept by the owner pending a license) join verses to the paragraphs their items named`.

### Task 8: The text page read and the whole chapter; rebuild #2, regen, re-bless

**Files.**
- Modify:
  - `server/atlas-contract/src/{wire/graph,graph,error,document}.rs`;
  - `server/atlas-contract/tests/{graph_api,page_cap_law,contract_coverage}.rs`;
  - `contracts/*` (regen);
  - `contracts/atlas-query-contract/CHANGELOG.md`;
  - `contracts/atlas-graph-contract/{graph,concord}/*.feature` (a `text` scenario per corpus; `concord/parts.feature`; catechism features retired);
  - pacts, fixtures;
  - `scripts/timing-gates.sh` (the two new reads at full page and whole chapter, 10× where the fixture exists).

**Steps.**
- [ ] **Red:**
  - W1–W8;
  - `a_container_of_containers_answers_an_empty_text_page`, `a_text_page_of_a_non_container_is_refused`, `an_unknown_container_is_not_found`, `a_whole_chapter_read_with_a_cursor_is_refused`;
  - `a_creed_paragraph_is_served_with_its_title_as_a_heading_part_and_its_article_as_text` (whole `UnitText`).
- [ ] **Green:** `node_text` over the `contains` keyset index and `unit_text`; `UnitText.parts`.
- [ ] **Rebuild #2 + regen** (`contract` lock):
  - rebuild (schema +1);
  - `export_contract`, `export_aqc_examples`, `--check`;
  - AQC and AGC bumps;
  - re-bless;
  - `client.ContractGenerator`.
- [ ] **C# compile minimum:**
  - the deletions listed in Task 7;
  - `GeneratedUsageTests` gains `FrozenClient.GeneratedAt = <base sha>`. A type generated after the freeze (`TextPage`, `TextPart`, `TextPartRole`, `ContainerDetail`, `PassageDetail`, `ContainerLevel`, `PassageMark`, `TextExtent`) is the F# client's to read. Codex's F# usage law enumerates them (§7). The C# law keeps checking every type it had.
- [ ] **Gates** (`heavy`):
  - `cargo test --workspace`, graph-types;
  - `dotnet build client && dotnet test client.Tests && dotnet test client.ContractTests`;
  - `bash scripts/contract-gate.sh --base <base>`, `bash scripts/contract-semver-gate.sh`;
  - `bash scripts/timing-gates.sh run`.
- [ ] **Commit:** `text: a chapter, article or passage is read one page at a time, or whole, by its container -- verses with their headings, paragraphs with their labelled parts -- so a page cannot cross its container (rule 27a/27b; PAGE-BOUNDARY-BUG-1 category closed on the server)`.

### Task 9: Gates, close, review

- [ ] **Playwright** (`heavy`): the full suite against the frozen C# app. Expected red: only the carried set at the base, plus the catechism-item specs deleted with their views (listed by name). Anything else is a C# compile-minimum miss: stop and report it.
- [ ] **Mutation** (owner's window only):
  - `scripts/mutants-parallel.sh -n 3 -b <base>` over `graph-types` (container, passage, admission, text), `atlas-etl::{concord,triglot}`, `atlas-graph::{passages,triglot_citations,concord_adapter,catechism_adapter}` and `node_text`;
  - Stryker is not needed: no C# behaviour changed.
  - Outside the window, `.superpowers/MUTATION-GATE-DEBT.md` names the base.
- [ ] **Close report** `docs/superpowers/reports/2026-10-xx-focus-3-close.md`:
  - each law of §3.3 with red/green evidence;
  - the `ArticleOnly` list;
  - the admission statistics and every `Confirmed` row;
  - the passage count;
  - `LARGEST_CHAPTER_TEXT` and `LARGEST_PASSAGE`;
  - the rule-24 table (§6);
  - FINDINGS.
- [ ] **Hand-off:** push; Codex reviews (14b, 24a); land under `land`.

## 5a. Waves

All of FOCUS-3 v2 is Rust and data, so it pairs with Codex's F# client work (rule 23), never with another Rust-heavy job. At most one Rust-heavy build runs at a time. Writing code for the next task overlaps the build of the previous one.

| Wave | FOCUS-3 v2 | Beside it (other lane) | Locks | Expected red at close |
|---|---|---|---|---|
| 0 | owner: residual questions 1–2; Task 0 | CX-FSHARP (F#) | — | — |
| 1 | Task 1 (graph-types) | CX-FSHARP | — | — |
| 2 | Task 2, then Task 3 (`concord.rs`, sequential) | CX-FSHARP | — | — |
| 3 | Task 4 + **rebuild #1** | CX-FSHARP | `contract`, `heavy` | — |
| 4 | Task 5, then Task 6 (adapters, sequential); Task 7's data rewrite written beside them (data and scripts only) | CX-FSHARP reads the B-list contract drafts from this plan | `heavy` per task | — |
| 5 | Task 7, then Task 8 + **rebuild #2 + regen** | CX-FSHARP | `contract`, `heavy` | none beyond the carried set |
| 6 | Task 9 | Codex review | `heavy`, mutation in the window, `land` | carried set only |

**Critical path:** A-WIRE-IDENTITIES lands → T1 → T2 → T3 → T4 (rebuild #1) → T5 → T6 → T7 → T8 (rebuild #2) → T9. Task 3 can stall on the threshold ruling (Step "measure"). Tasks 5 and 6 do not depend on Tasks 2–4 and may start in wave 2 if Task 3 stalls; they then take rebuild #1's slot.

## 6. What is deleted, what is not, and the rule-24 categories closed

**Deleted (server, data, contract):**
- Concord parser excision: `strip_complete_headings`, `strip_standalone_strong_paragraphs`, `split_on_paragraph_tags`, `clean_paragraph_text`, `ConcordParagraph.text`.
- Level by id prefix: `decode_book_container`, the id-prefix branch of `book_detail`.
- Per-request contents derivation: `contents.rs`'s `format!("BoC …")` and the first-member `ref`/`locus` derivation.
- The span-label special case: `object_label`'s span arm and `target_display` as a label.
- Concord articles' document-local succession, replaced by one chain.
- The catechism item:
  - `NodeKind::CatechismItem`, `CatechismItemId`, `NodePayload::CatechismItem`, `RowFamily::Catechism` in `core`;
  - `atlas-core`'s `CatechismItem`, `CatechismQuestion`, `CatechismPart`, `catechism_items_for_span` and the verse→catechism index;
  - `/api/catechism/item/{id}` and `/api/catechism/{sref}`;
  - wire `CatechismDetail`, `CatechismItem`, `CatechismProofVerse`, `CatechismRef`;
  - `data/curated/catechism.toml`, `data/curated/concord-sc-overlap.toml`;
  - the brain-fuel topic titles as served data;
  - the catechism AGC features, pact interactions and fixtures.
- **C# compile minimum** (Tasks 7–8): the catechism item views and reads, and the `CatechismItem` arms.

**Not deleted** (named so no one "finishes" them):
- `/api/text?ref=` (frozen, its three users); `/api/chapter`, `/api/books`, `/api/xrefs/{sref}` (frozen, read by the C# reader). All four retire with the C# client (B15).
- `/api/contents/{corpus}`.
- `CatechismLink`'s ordinal.
- `concord-exclusions.toml`.
- Every other C# file.

**Withdrawn plans and parts:**
- The old plan's Tasks 1, 5, 6, 7, 8 and 10's client halves → §7.
- Its Task 9 route deletions, except the catechism routes.
- FOCUS-7 Amendment A (`stated-in`, `UnitOpening`, `CrumbRole.Identity`, Tasks 1a and 2a).
- CAT spec's `Question`/`Answer` roles, `UnitComposition`'s `Questions` flag, the `Explanation` corpus and `Explains` (deferred to an explanation source).

| Category (24a) | Closure (24b) | Guarantee |
|---|---|---|
| The parser decides what text exists by excising strings (F-78) | one `source_nodes` door; total assignment; `Parts` a typed partition | T4 over every article of every page |
| A served Concord span with no proof it is Triglot (F-84) | `admit` inside `read_all`, `Admission` has no unadmitted variant | T5 (real + generated), compile refusal |
| A paragraph number written by curation | `SourceMarker` is the only constructor of a paragraph number | T8 |
| A container's level read from its id | `Container` sum compiled; `container_detail` | N8 |
| Two navigation behaviours for two corpora | one chain per level per corpus, one generic law set | N1–N7 over `Corpus::ALL` |
| A cited range that is not a node (F-65 remainder) | `PassageMint` the one door | P1–P9, R1 |
| A reading window addressed by a corpus offset (PAGE-BOUNDARY-BUG-1) | the page is addressed by its container | W1 |
| One fact in two copies: the catechism's words (F-80) | the paragraph is the catechism | K1, K2 |
| One relation with two meanings: `catechism-link` (F-76's category) | `CatechismLink` is verse ↔ paragraph only, by type | K2 (type) |

**FINDINGS this plan expects to raise:**
- If `θ`'s populations overlap (Task 3), the threshold is a ruling.
- If no 10× synthetic graph exists for the text page and whole-chapter timing (27f), the gate runs at 1× and says so.
- The brain-fuel exception remains an open licensing category until O-CATECHISM is answered.

---

## 7. F# client backlog (the behaviours, and the contract each consumes)

Codex's CX-FSHARP lane builds these. Each item names the behaviour and the served contract it reads. Each "laws" line is what the F# tests check, built on the server laws of §3.3. No item needs a server change beyond this plan.

- **B1. The reader reads a book; chapters are titled once.**
  - Behaviour: the Bible reader and the Book of Concord reader are one view over the shared container abstraction. A book (or document) scrolls; each chapter (or article) is titled once, by its served title; a Smalcald article shows its `section_title` above it.
  - Contract: `GET /api/contents/{corpus}` (`ContentsRoot`, `ContentsChild.section_title`), `GET /api/node/{id}` (`container.level`), B2's text read.
- **B2. The reading window: 20 a page, 40 held, or the whole chapter.**
  - Contract: `GET /api/node/{id}/text?cursor&limit=20` and `?extent=chapter`.
  - Laws, with the window `w` as the sequence of resident pages:
    - `append(w)` reads `next` and adds one page; `slide` drops the oldest page while more than 40 units are held;
    - `back(w)` reads `previous`, and `back ∘ slide = id` on the held units;
    - held units ≤ 40, except after `whole`, where `held = units(c)` and `|held| ≤ LARGEST_CHAPTER_TEXT` (W5);
    - every page opens through the one root-aware paging door (F-70).
- **B3. One focus change: a scroll is a step.**
  - Behaviour: the next-chapter arrow, the previous-chapter arrow, scrolling into the next chapter and scrolling back all change the focus by following the focus's `follows-in` / `precedes-in` link, through one function.
  - Laws: `scrollInto(next c) = step Next c` and `scrollInto(prev c) = step Prev c`, as values (the same focus change, the same URL replacement, the same locus update). A source law: the focus is changed in exactly one place.
  - Contract: `GET /api/node/{id}/edges?kind=follows-in|precedes-in`. Server law S1 guarantees the spine and the chain agree.
- **B4. The remembered position** (R8): per container, the topmost unit, in local storage, bounded. No contract.
- **B5. Arrows** (NAV-UNIFORM-1): from `follows-in`/`precedes-in` with served labels, the same for both corpora and for Kretzmann. Contract: the edge page.
- **B6. The Book of Concord reader.**
  - The cover shows `Contents.title` and `Contents.description` once (Q9).
  - A margin citation shows each paragraph's served `label` (`Ap IV 48`).
  - There is no numeric "Go to" picker (Q8: not built).
  - Article heads open the article.
  - A page never bleeds into the next article (W1).
- **B7. Only paragraphs with somewhere to go are clickable** (Q10).
  - A unit row is a target iff its served `edge_summary` has a kind other than `contains`, `member-of`, `follows-in` and `precedes-in`. That is one affordance-table rule, total over the generated `EdgeKind`.
  - Contract: `TextUnit.edge_summary` (W8).
- **B8. Labelled parts.**
  - A `heading` part renders as its own line. `text` renders as body; Luther's questions are text, always shown.
  - `bracket` parts are hidden by default, behind a per-reader toggle.
  - The client reads roles, never words.
  - Laws: `visible(parts, ∅) = parts`; `visible` is idempotent; the shown text is the served text minus exactly the hidden parts.
  - Contract: `UnitText.parts`.
- **B9. Shift-click selects verses only** (Q6). There is no range popover. Contract: none beyond the units.
- **B10. No chapter-card extras** (Q7): no pericope list, places, cross-reference total, "Chapter n of N" or hover peek. A container's card is its record and frontier.
- **B11. A passage opens like any node.** Its label is its code; its `contains` page lists its verses (B2's read works on a passage). An account passage (`mark = recordsHistory`) is FOCUS-5's to group. Contract: `NodeRecord.container.passage`, the text page.
- **B12. A Small Catechism paragraph opens itself** (no hop). It lists `catechism-link`, `cites`, `quotes` and `member-of` as its groups. Contract: the element read and edge pages.
- **B13. Kretzmann's arrows, chapter head and verse rows on the shared pieces** (Q11). Contract: as B5; its comments stay on `/api/text?ref=` and `/api/kretzmann/chapter` until FOCUS-7.
- **B14. One table of contents:** current entry by container id, no counts (SIDENAV-PAGENUM-1). Contract: `/api/contents/{corpus}`.
- **B15. Legacy route retirement** (server, after the F# client replaces the C# reader): delete `/api/text?ref=`, `/api/chapter`, `/api/books` and `/api/xrefs/{sref}`, with their pacts. This is a proposed queue item, not FOCUS-3.
- **B16. F# generated-usage law.** Every type generated after `FrozenClient.GeneratedAt` is read by the F# client.

---

## 8. Residual owner questions

1. **When the Triglot scan's text is too garbled for the computer to match a paragraph that really is in the 1921 book, should we confirm it by hand (a line in a data file citing the Triglot page) and keep serving it, or leave it out until the scan matches?** Recommend: confirm it by hand, with the page cited. The build still refuses anything that is neither matched nor confirmed.
2. **A verse is labelled by its code, GEN.1.1. Should a Book of Concord paragraph be labelled by the Triglot's own citation code, "Ap IV 48" (or just "Ap IV" where our numbering does not match the Triglot's), rather than our internal "BoC 4.4.48"?** Recommend yes: it is the Concord's standard code, the one readers cite. The internal code stays the id.

Everything else the rulings settle. Where this plan had to choose, it says so:
- the re-home of the brain-fuel links (§3.2 D5);
- articles chaining across documents like chapters across books (I7);
- `Explains` deferred to the explanation batch;
- legacy routes frozen while the C# client reads them.
