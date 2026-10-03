# The catechism model (DRAFT, for the owner's decision)

Status: draft, 2026-10-02. Planning only; no code changes. Decides FOCUS-7 OPEN 7 and re-scopes FOCUS-7 Amendment A and the FOCUS-3 hop fix (F-76).

**The owner's words (2026-10-02, verbatim):** "we should have symmetric mapping. Not sure on 7 honestly. I think it needs more thought to make the catechism fully explorable, because ideally, you have scripture which cleanly links to catechism with explanation, which should cleanly link to catechism, where the actual served text giving the catechism is from one source and is just composed with or without questions."

**Binds:** PRINCIPLES 6, 8, 12, 14b, 24–24b, 25, 26, 26a, 27–27g; the licensing rule (AGENTS.md, owner 2026-09-29); the KJV directive (the KJV is never edited; Luther's and the explanation's words are shown as written).

## 1. The reading, checked

The controller read the words as three layers joined both ways: Scripture, the "catechism with explanation" (the question-and-answer expansion that cites proof verses), and the catechism itself (Luther's Small Catechism, the six chief parts), with the catechism's text from one source, composed with or without questions.

The data below confirms the three layers with one correction:

- **"Questions" means two different things in the data.**
  - Luther's own questions are part of the one source: "What does this mean?", "How is this done?", "Where is this written?", "Which is that word of God?".
  - The explanation's questions belong to a different work. Today these are brain-fuel's topic titles ("God Alone as Judge"), which are not questions at all.
- **"Composed with or without questions"** fits Luther's own questions: the same Small Catechism paragraph, shown as "Thou shalt have no other gods. What does this mean? –Answer: We should fear, love, and trust in God above all things.", or as "Thou shalt have no other gods. We should fear, love, and trust in God above all things."
- **The explanation's questions** live in the middle layer and are never composed into the catechism's text.
- OQ1 asks the owner to confirm this.

## 2. The data today (read 2026-10-02, read-only, served artifact at `b3d7cfa`, `data/curated`, `data/raw`)

### 2.1 What a CatechismItem holds

There are two stores, and the node carries only a label:

- **Graph node** `CatechismItem:commandment-1`: `{ label: "The First Commandment", provenance: "curated-catechism" }`, edge summary `catechism-link` ×207.
- **Words beside the graph** (`atlas-core` `data::CatechismItem`, read by `catechism_detail`):

```rust
pub struct CatechismItem {
    pub id: String,
    pub name: String,
    pub text: Option<String>,
    pub explanation_heading: String,
    pub explanation: String,
    pub where_written: Option<String>,
    pub verses: Vec<String>,
    pub ref_note: Option<String>,
    pub questions: Vec<CatechismQuestion>,
}
pub struct CatechismQuestion { pub title: String, pub verses: Vec<String>, pub source: String }
```

**The First Commandment, verbatim, both copies:**

| Copy | Words |
|---|---|
| `catechism.toml` (served as `CatechismDetail`) | text `"Thou shalt have no other gods."`; explanation_heading `"What does this mean?"`; explanation `"We should fear, love, and trust in God above all things."` |
| Concord `text-unit:BoC 7.2.1` | `"Thou shalt have no other gods. What does this mean? –Answer: We should fear, love, and trust in God above all things."` |

**The words exist twice, and the two copies are not equal:**

1. **The Triglot's brackets.** `catechism.toml` drops them (its header explains why). The Concord copy keeps them, e.g. 7.2.8 "defend him, [think and] speak well of him" and 7.2.11 "gladly do [zealously and diligently order our whole life] according to".
2. **The Concord parser drops `<h4>` text, and the Small Catechism puts Luther's own words in `<h4>`.** Raw `small-catechism.html` holds them; the served paragraphs lose them:
   - 7.3.1–3 lack the Creed's articles. 7.3.1 reads "The First Article. Of Creation. What does this mean? –Answer: I believe that God has made me…", with no "I believe in God the Father Almighty, Maker of heaven and earth."
   - 7.6.4 lacks the absolution and the confessor's lines: "God be merciful to thee…", "Dost thou believe that my forgiveness is God's forgiveness?", "As thou believest, so be it done unto thee… Depart in peace."
   - So `concord-sc-overlap.toml`'s claim that `confession-2`'s words "sit embedded within" 7.6.4 is false of the served text.
   - The commandment headings ("The First Commandment.") are dropped as well.
3. **Markup residue** in the served text: `**`, `_…_`, "Matthew :", "Confession ?*".
4. **A modern editorial note sits in 7.6.4.** It ends with "* These questions may not have been composed by Luther himself but reflect his teachings…". This looks like bookofconcord.org's own editorial note, not the 1921 Triglot. LICENSES.md says the site's own notes are its copyright ("Original Content Copyright 1998–2024"). **To verify against the Triglot scan** (FINDING, §9).
5. **`catechism.toml` packages differently.** `confession-1`'s explanation is the answers of 7.6.1, 7.6.2 and 7.6.3 joined, with Luther's two questions ("What sins should we confess?", "Which are these?") removed. Its alignment names 7.6.1 only.

**Luther's own questions are not one heading.** FOCUS-7 Task 1 proposed a file-level `where_written_heading = "Where is this written?"`. But Luther asks "Which is that word of God?" for `baptism-1` (7.5.2) and "Which are such words and promises of God?" for `baptism-2` (7.5.4), so that heading would misquote him. It is right only for 7.5.7 and 7.7.2.

### 2.2 Where the words come from

| Data | Source | License | Verdict |
|---|---|---|---|
| `catechism.toml` text, explanation, where_written | 1921 Triglot (Bente–Dau) via Wikisource page transcriptions | Public domain (1921, USA); our structure CC0 | In |
| Concord paragraphs, part 7 | 1921 Triglot via bookofconcord.org | Public domain; site chrome is the site's own | In, except the editorial note (item 4 above) |
| `concord-sc-overlap.toml` (33 rows, 37 pairs) | Ours, hand-checked | CC0 (LICENSES.md); `sources.toml` wrongly credits `catechism-mapping` (OPEN 10, ruled: credit ours) | In |
| `catechism.toml` `verses` + `ref_note` (Luther's own citations) | Ours, read from the 1921 text | CC0 | In |
| `catechism-deut5.toml` (Decalogue in Exodus 20 / Deuteronomy 5) | Ours | CC0 | In |

### 2.3 What "with explanation" material we have

| Material | Where | License | Verdict |
|---|---|---|---|
| Brain-fuel topic titles + verse groupings (37 of 44 files; 17 groups and 206 links for `commandment-1` alone) | `data/raw/catechism-mapping/…/resources/*.yaml`, `catechism-mapping.toml` | No license file (O-CATECHISM pending) | **Out** |
| Svebilius, *Simple Explanation of the Catechism* in English (numbered Q&A with KJV proof texts) | `…/svebilius/en/`, fetched, never ingested | The repo is unlicensed. The English is a modern translation of a 2007 Finnish edition (Tero Kotti); only the 1689/1745 originals are PD | **Out** |
| Luther's own questions and embedded citations | the one source | PD | In (the catechism layer, not the explanation layer) |

**We have no licensed explanation layer at all.** Everything served today as "questions" comes from the brain-fuel mapping.

### 2.4 How proof verses attach

- `catechism_adapter::merge_alias` flattens `item.verses` and every `question.verses` into one `CatechismLink { locus, item }` row per (verse, item), with provenance `curated-catechism`.
- That drops which question cited the verse; only the legacy `/api/catechism/item` keeps the title.
- `concord_adapter::merge_alias` adds the 37 paragraph↔item rows to the same relation (Amendment A's category).
- `commandment-1` is served 207 `catechism-link` rows: 206 Scripture, then `BoC 7.2.1`.
- **No paragraph cites Luther's prose citations.** "The last chapter of Matthew", "Titus, chapter three" and "Romans, chapter 6" cannot be scanned. Today only 7.2.11 `cites` anything ("Exod. 20:5f").

### 2.5 The questions

The "questions" are brain-fuel's topic titles: `title` keys in numbered YAML maps, e.g. "God Alone as Judge", "Worship God Alone", "Fear, Love, and Trust in God". `commandment-1` has 18 titles, including our own "The Deuteronomy 5 Parallel". They are unlicensed and are not questions.

### 2.6 Alignment shape

- 29 items have one paragraph and 4 have two.
- `confession-2` is a sub-span of 7.6.4, and its words are missing from the served text (2.1, item 2).
- The Small Catechism has 91 paragraphs; 54 have no item (52 of them in the Preface, Daily Prayers, Table of Duties and Christian Questions).

## 3. The recommended model (M1): the Small Catechism's paragraphs are the catechism

**Three layers, one text each, every link followable from both ends:**

```
Scripture verse  ──cites / cited-by──  Explanation question  ──explains / explained-by──  Small Catechism paragraph
      │                                                                                         │
      └──────────────────────── cited-by / cites, quoted-by / quotes ───────────────────────────┘
```

- **Scripture:** `TextUnit`, Bible corpus (unchanged).
- **The catechism:** `TextUnit`, Concord corpus, part 7 (exists).
  - It is the ONE source of catechism text. Each paragraph's text is partitioned by the compiler into parts with a role: the source's heading, Luther's text, Luther's question, the answer, the Triglot's bracket.
  - `CatechismItem` is retired: its words were a second copy, and its grouping was `catechism.toml`'s packaging.
  - Clicking a paragraph opens the paragraph. That paragraph *is* "The First Commandment", so F-76's hop cannot exist: there is no second node to hop to.
- **The explanation:** a new corpus, `Explanation`, of `TextUnit`s.
  - Each unit is one numbered question and answer of a public-domain explanation (§6). It is read as a book of its own, like the Bible and the Concord.
  - Its proof verses are `cites`, compiled by the shared citation scanner (FOCUS-2/3/7 already make it one scanner).
  - Its alignment to the catechism is `explains`: curated data, CC0, hand-checked like `concord-sc-overlap.toml`.
- **Luther's own citations** (Exodus 20:5f; Matthew 28; Mark 16; Titus 3; Romans 6; the institution narrative) become `cites` rows from the paragraph.
  - They come from the scanner where it can place them, and otherwise from curated rows that move `catechism.toml`'s `verses` + `ref_note` into FOCUS-3's new `data/curated/concord-citations.toml`.
- **The Decalogue's wording** (`catechism-deut5.toml`) becomes `quotes` rows from 7.2.1–10 to Exodus 20 and Deuteronomy 5. This is an existing relation and our data.

### 3.1 Rule 27: who derives what

| Derivation | Owner | Why |
|---|---|---|
| Partition of a paragraph's text into roled parts | compiler (ETL, `concord.rs` and the new explanation parser) | the source's markup alone decides it (26a): `<h4>` headings, `<em>What does this mean?</em>`, "–Answer:", `[`…`]` |
| `cites` from paragraphs and explanation answers | compiler (one scanner + curated rows) | data alone |
| `explains` rows | curated data → compiler; an unresolved row fails the compile | data alone |
| A unit's label (frontier entries, 27c) | compiler: the source heading where the source gives one ("The First Commandment"), else the Triglot citation (FOCUS-3 R11); for an explanation unit, its number and question | data alone |
| Which parts are shown: with or without questions, brackets | **client** (`UnitComposition`), over served parts | an interaction choice, not a fact (25, 27); the server serves one text, never a variant |
| Neighbours, pages | server, generic reads (27a); no new route | per request |

**A served variant is rejected:** a `?questions=false` text read is a view on the wire (27a). It would also put the same words on the wire in two shapes.

### 3.2 Types (PRINCIPLES 12)

Graph (`graph-types/src/text.rs`):

```rust
pub enum TextPartRole {
    Heading,
    Text,
    Question,
    Answer,
    Bracket,
}

pub struct TextPart {
    pub role: TextPartRole,
    pub start: u32,
    pub end: u32,
}

pub struct ExplanationTag;

pub struct ExplanationRef {
    pub work: ExplanationWorkId,
    pub question: u16,
}

pub type ExplanationLocus = Locus<ExplanationTag>;

pub enum TextRef {
    Bible(VerseRef),
    Concord(ConcordRef),
    Explanation(ExplanationRef),
}
```

- **`TextPart`s partition a unit's text:** they are in order, gapless and non-overlapping. A law walks every unit. A unit the source gives no structure is one `Text` part, so every unit is partitioned and no reader special-cases a corpus (24b).
- **`Question`** spans Luther's question with its "–Answer:" marker, so leaving it out reads cleanly.
- **`ExplanationWorkId`** is a data-declared id: a newtype checked against `data/curated/explanations.toml` at compile time. It is not an enum of book titles in code (rule 26).

Graph (`graph-types/src/edge.rs`):

```rust
relations! {
    directed {
        Explains => "explains" / "explained-by"
    }
}

pub struct Explains {
    pub question: ExplanationLocus,
    pub text: ConcordLocus,
    pub provenance: ProvenanceId,
    pub justification: Justification,
}
```

- `ConcordLocus` carries its optional span, so an explanation of the absolution can point inside 7.6.4.
- `Explains` is appended last, under the `contract` lock. Under OQ5 "same name both ends", it is the symmetric `ExplanationOf => "explanation-of"` instead.
- **Retired:**
  - `CatechismLink`, `SymRelationId::CatechismLink`, `RowFamily::Catechism`;
  - `NodeKind::CatechismItem` and `CatechismItemId`;
  - `NodePayload::CatechismItem`.
  - These removals change ordinals, so they are a version-root move and an AQC major, done in one `contract` hold.

Laws (`server/atlas-graph/src/law_check.rs`):

```rust
pub fn every_text_unit_is_partitioned_into_parts(graph: &Graph) -> Result<(), String>;
pub fn every_explanation_question_explains_a_small_catechism_paragraph(graph: &Graph) -> Result<(), String>;
pub fn every_small_catechism_chief_part_paragraph_has_text_and_answer(graph: &Graph) -> Result<(), String>;
```

The third law is the red test for the `<h4>` loss: 7.3.1 has no `Text` part today.

ETL (`server/atlas-etl`, the tool layer):

```rust
pub fn parse_explanation(work: &ExplanationWorkId, html: &str) -> Result<Vec<ExplanationUnit>, ExplanationParseError>;

pub struct ExplanationUnit {
    pub question: u16,
    pub text: String,
    pub parts: Vec<TextPart>,
}

pub enum ExplanationParseError {
    UnnumberedQuestion { after: u16 },
    MissingAnswer { question: u16 },
}
```

- `concord.rs` keeps Luther's `<h4>` text in the paragraph as `Heading` or `Text` parts, by the source's markup. It drops the site's editorial note if the Triglot check (§9) confirms the note is not the Triglot's.

Wire (`server/atlas-contract/src/wire/graph.rs`):

```rust
pub struct TextUnit {
    pub r#ref: String,
    pub locus: super::TextRef,
    pub text: String,
    pub parts: Vec<TextPart>,
    pub words_of_christ: Vec<super::reading::WordsOfChristSpan>,
    pub heading: Option<UnitHeading>,
    pub anchors: Vec<super::Anchor>,
    pub edge_summary: Vec<EdgeSummaryEntry>,
}

pub struct TextPart {
    pub role: TextPartRole,
    pub start: usize,
    pub end: usize,
}

pub enum TextPartRole { Heading, Text, Question, Answer, Bracket }
```

- `TextRef` gains `explanation`, and `EdgeKind` gains `explains` / `explained-by`.
- There is no new route, and `CatechismDetail` and `/api/catechism/item/{id}` are deleted.
- AQC major.

Client (`client/Exploring/UnitComposition.cs`):

```csharp
public sealed record Composition(bool Questions, bool Brackets);

public static class UnitComposition
{
    public static IReadOnlyList<TextPart> Of(TextUnit unit, Composition composition);
}
```

- `UnitTextView` renders `UnitComposition.Of(unit, composition)`. It keeps or drops served parts by their served role, and never reads the words (25).
- `Composition` is a per-view toggle beside the reader, remembered with the reading position.
- The default is OQ1's answer: with Luther's questions, without the Triglot's brackets.

Test (whole-body, rule 15):

```csharp
[Fact]
public void A_paragraph_without_questions_keeps_the_commandment_and_the_answer()
{
    // Arrange
    var unit = ServedUnits.FirstCommandment;
    var withoutQuestions = new Composition(Questions: false, Brackets: false);

    // Act
    var parts = UnitComposition.Of(unit, withoutQuestions);

    // Assert
    parts.Should().Equal(
        new TextPart(TextPartRole.Heading, 0, 23),
        new TextPart(TextPartRole.Text, 23, 54),
        new TextPart(TextPartRole.Answer, 84, 140));
}
```

### 3.3 How each click lands (no unnecessary hop)

| From | Click | Lands on | Reads (27b, 27c) |
|---|---|---|---|
| Verse (GEN 1:1 reader row or FocusView) | its `cited-by` entry for an explanation question | that question as `Presentation.Text`: question and answer, its proof verses as anchors, crumb `explains` → its paragraph | element read of the question; its first `explains` link comes with its frontier |
| Explanation question | `explains` crumb or entry | the Small Catechism paragraph as `Presentation.Text` (the catechism, composed per `Composition`) | one element read |
| Small Catechism paragraph (Concord reader row, FOCUS-3 OPEN 10a) | the row | **the paragraph itself**: "The First Commandment" with its frontier (`explained-by`, `cites`, `quotes`, `member-of`) | one element read; nothing to redirect |
| Small Catechism paragraph | an `explained-by` entry | that explanation question | one element read |
| Small Catechism paragraph | a `cites` / `quotes` anchor | the verse | one element read |
| Verse | its `cited-by` / `quoted-by` entry for a Small Catechism paragraph | the paragraph (Luther's own citation, or the Decalogue's wording) | one element read |

- **"Catechism paragraph → item" disappears**: the paragraph is the item. F-76 is satisfied by construction: clicking the First Commandment in the reader puts the First Commandment, with its links, in the focus.
- No text row redirects, so Amendment A's `UnitOpening`/`CrumbRole.Identity` machinery is not built.
- **One consequence for the reader:** under FOCUS-2 ruling 3 (one group per edge kind), a verse's `cited-by` group lists Concord paragraphs and explanation questions together, in served order. A separate group would need a separate relation; OQ5's note covers it.

## 4. What happens to the existing pieces

| Piece | Fate under M1 |
|---|---|
| `CatechismItem` node kind, `CatechismItemId`, its 33 nodes | deleted |
| `catechism.toml` | words deleted (the Concord paragraph is the one copy, once the parser keeps the `<h4>` text). Its `verses` + `ref_note` move to `concord-citations.toml`. Its names are the source's own headings. The file is deleted |
| `catechism-link` (relation, `CatechismLink`, ~all rows) | deleted. Luther's citations become paragraph `cites`; brain-fuel rows go per OQ4 |
| `concord-sc-overlap.toml` | kept only as migration data: a saved exploration naming `CatechismItem:x` resumes at its first paragraph (`LegacySaves`, with a test). Credit fixed per OPEN 10 |
| `catechism-deut5.toml` | becomes `quotes` rows (paragraph → Exodus 20 / Deuteronomy 5 verse) |
| `catechism-mapping.toml`, `data/raw/catechism-mapping` | per OQ4 |
| `atlas-core` `CatechismItem/Question/Part`, `catechism_items_for_span`, the verse→catechism index; wire `CatechismDetail`, `CatechismItem`, `CatechismProofVerse`, `CatechismRef`; `/api/catechism/item/{id}` | deleted (FOCUS-3 already deletes `/api/catechism/{sref}`) |
| Client `Catechism*Section`s, `CatechismNode`, `CatechismLinks`, `CatechismList` | deleted (FOCUS-7's inventory, unchanged) |
| Amendment A (`stated-in`, `UnitOpening`, `CrumbRole.Identity`, Task 1a, 2a) | withdrawn |

## 5. Alternatives

**M2. Keep the item; its words are read through `stated-in`.**
- This is Amendment A plus one source: `CatechismItem` stays as the named node, its words are deleted from `catechism.toml`, and the card composes them from its paragraphs through `stated-in`.
- `explains` targets the item, and a paragraph row opens its item through `UnitOpening`.
- *For:* stable ids and saves; a "Baptism, part 1" node over 7.5.1–2.
- *Against:*
  - two nodes for one thing;
  - a redirect rule on the client;
  - the card needs a neighbour read for its own words;
  - the item's grouping is our packaging, not Luther's: 7.5.1 and 7.5.2 are two of his questions;
  - `confession-1` spans three paragraphs while its alignment names one.
- *Recommendation:* second choice. Pick it if stable item ids in saved explorations matter more than one node per text.

**M3. The item is a passage container over its paragraphs.**
- FOCUS-3's passage containers (no root, overlap legal, the forest law restated over root-reached containers) make this cheaper than Amendment A said.
- The item's words are the generic container text page (FOCUS-3 `GET /api/node/{id}/text`), so there is one source with no new read.
- *Against:*
  - a paragraph would gain a second `member-of`, and the reader row would still have to open the container instead of itself (the same redirect);
  - FOCUS-3's law "a passage holds only verses" must widen;
  - a sub-paragraph item (the absolution) is not expressible as `contains`.
- *Recommendation:* not chosen.

## 6. A licensed path to the explanation layer

Public-domain explanations in English, all published in the USA before 1931:

| Work | Shape | Where | Fit |
|---|---|---|---|
| H. U. Sverdrup, *Luther's Small Catechism Explained in Questions and Answers* (after Pontoppidan), tr. H. A. Urseth (Augsburg, 1900) | numbered Q&A, proof passages printed under answers | Project Gutenberg #36081; archive.org `explanationoflut00sver` | **closest to "with explanation"**: one unit per numbered question, proof verses as `cites` |
| Joseph Stump, *An Explanation of Luther's Small Catechism* (1907) | handbook: analysis and explanation with proof texts | Project Gutenberg #9912; archive.org; ctsfw.edu PDF | second work: richer prose, more anchoring curation |
| H. C. Schwan, *A Short Explanation of Dr. Martin Luther's Small Catechism* (Synodical; English 1912 edition) | numbered Q&A, proof texts | archive.org (edition to verify) | the Missouri Synod lineage behind CPH's "with explanation". **Only a pre-1931 printing**: the 1943, 1991 and 2017 CPH editions are out |

- **License:** all are public domain by US publication date. A Gutenberg file's license header covers the Gutenberg trademark, not the text. The vendored file strips it and cites the printed edition in `LICENSES.md` and `sources.toml`.
- **Verification before vendoring** (as the Triglot was):
  - read the title page and date on the scan;
  - confirm the translation is the printed one;
  - spot-check two answers against the scan;
  - record each in `data/raw/README.md`.
- The explanation's own quotation of the Small Catechism (Sverdrup's 1900 wording) is not served as the catechism. The catechism's text is only the Triglot paragraph. The alignment pairs a question with that paragraph.
- **Proof verses:** the explanation's printed references, compiled by the shared scanner. Unplaced references stay plain text and are listed (as FOCUS-7 OPEN 6).
- **The fallback** O-CATECHISM already names ("our own mapping from the public-domain 1921 Triglot") is the same path. Its verses come from a PD explanation's own printed proof texts, not from brain-fuel's.

## 7. Owner questions (at most 5)

1. **Which questions does "with or without questions" mean?** The catechism's own text has Luther's questions ("What does this mean?", "Which is that word of God?"). A separate explanation book has its own numbered questions, each with Bible verses. Should the catechism's text be shown with or without *Luther's* questions, while the explanation book's questions stay their own linked layer? And should the default show Luther's questions, and hide the Triglot's bracketed German/Latin comparison words? **Recommend:** yes. The toggle covers Luther's questions and the brackets; the default shows the questions and hides the brackets.
2. **What *is* "the catechism" in the graph?**
   - (a) The Small Catechism's own paragraphs: clicking "The First Commandment" in the Book of Concord opens it directly, there is one copy of the words, and the separate catechism item goes away.
   - (b) Keep the catechism item as its own node, with its words read from the paragraph.
   - (c) Make the item a container over its paragraphs.

   **Recommend (a).** Saved explorations that name an item resume at its paragraph.
3. **Which public-domain explanation first?** Sverdrup's *Explained in Questions and Answers* (1900, numbered questions with proof verses), Stump's *Explanation* (1907), or a pre-1931 Schwan? **Recommend Sverdrup first, Stump second.** Both are on Project Gutenberg, after a title-page check.
4. **The brain-fuel verse links (unlicensed, about 206 per commandment) until the public-domain explanation is in:** drop them when `catechism-link` goes, or keep them until the replacement lands? Either way O-CATECHISM's license request stands. **Recommend drop when the model lands.** They are unlicensed, they are not Luther's citations, and a public-domain explanation replaces them with verses a real book printed.
5. **"Symmetric":** every link in this model is followable and listed from both ends, so a verse shows its explanations and an explanation shows its verses. The only remaining choice is the name.
   - (a) Different names at each end: "explains" / "explained by", like "cites" / "cited by".
   - (b) One name at both ends ("explanation of").

   **Recommend (a).** The two ends play different roles, and the reader's group titles read naturally. Both are symmetric in the sense of reaching both ways.

## 8. Re-scoping FOCUS-7 and the FOCUS-3 hop fix

**FOCUS-3 can do these safely now, because they hold under M1, M2 and M3:**
1. **Keep Luther's `<h4>` text in the Concord parser** (`server/atlas-etl/src/concord.rs`, a FOCUS-3 file): the Creed's articles, the confession form's lines, the commandment headings. This is a rule-24 category (the parser treats `<h4>` as structure, and the Small Catechism puts text proper there); the red test is 7.3.1's missing "I believe in God the Father Almighty…". It needs the owner's word because it changes Concord text, and it rides FOCUS-3's Task 3 rebuild.
2. **Luther's own citations as `cites`:** move `catechism.toml`'s `verses` + `ref_note` into FOCUS-3's `data/curated/concord-citations.toml`, keyed by paragraph.
3. **Credit `concord-sc-overlap` to our own work** (OPEN 10, ruled).
4. **OPEN 10a** (a paragraph is clickable when its frontier has more than `member-of`) and the reader row opening the unit itself. Under M1 this is the final behaviour.

**FOCUS-3 should not do these before OQ2:**
- append `stated-in`;
- narrow `CatechismLink`;
- build `UnitOpening` / `CrumbRole.Identity`.

Under M1 none of them exists. Until the model lands, a clicked paragraph opens itself and lists `catechism-link` ×1 to the item, so the hop remains for one more batch. This revises the OPEN 9 answer: the hop fix moves to the catechism batch, not FOCUS-3.

**FOCUS-7:**
- **Unaffected:** the Kretzmann tasks (3–8) and the client deletions of the legacy catechism UI.
- **Task 1 (headings as data) is withdrawn:** Luther's questions come from the source text as `Question` parts. Its proposed `where_written_heading` would also misquote `baptism-1`/`baptism-2` (§2.1).
- **Tasks 1a and 2a are withdrawn.**
- **Task 2 (item on FocusView)** becomes "the Small Catechism paragraph on FocusView", which FOCUS-2/3 already give.
- **New batch CAT (proposed, after FOCUS-3):**
  - CAT-1: parts and the partition law;
  - CAT-2: retire `CatechismItem` and `catechism-link`, with the saves migration;
  - CAT-3: vendor and parse the first explanation, with `explains` alignment;
  - CAT-4: the client `UnitComposition` toggle.

## 9. FINDINGS for the queue (the owner decides)

- **F-CAT-a:** the Concord parser drops `<h4>` text that is Luther's (Creed articles, confession form, commandment headings). The 1921 text is incomplete in the served corpus.
- **F-CAT-b:** `concord-sc-overlap.toml`'s header states that `confession-2`'s words are in 7.6.4. They are not, in the served text.
- **F-CAT-c:** 7.6.4 carries a probable bookofconcord.org editorial note (the site's own copyright). Verify against the Triglot scan, and strip it in the ETL if it is not the Triglot's.
- **F-CAT-d:** markup residue (`**`, `_…_`, stray spaces before `:` and `?`) in served Concord text.
- **F-CAT-e:** FOCUS-7 Task 1's `where_written_heading` would misquote two Baptism questions.
- **F-CAT-f:** `catechism.toml`'s `confession-1` joins three of Luther's answers and drops two of his questions.
