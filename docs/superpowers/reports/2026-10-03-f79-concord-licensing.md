# F-79: non-Triglot text in the served Concord corpus

Read-only check, 2026-10-02. Sources:
- **Served text:** `data/compiled/sections/concord.bfe52b2b…sqlite.zst`, root `69fefc39…`. The `concord_unit` table joins the `TextUnit` payload, giving 3,827 units, all with the `bente-dau` rendering.
- **Vendored source:** `data/raw/concord/*.html` from bookofconcord.org.
- **Reference:** the Internet Archive scan *Concordia Triglotta* (1921), `concordiatriglot00unse_djvu.txt`, a public-domain OCR.

Method:
- **Shingle check.** Every served unit was compared against the Triglot OCR as 4-word shingles, after joining hyphenated words and folding case. I recorded each unit's coverage and every run of 10 or more words that is absent from the Triglot.
- **Pattern scan.** I also grepped for `[Note`, `Editor`, `bookofconcord`, `footnote`, `translator`, `http`, `www.`, `.org`, `click`, `Readers Edition`, `copyright`, `©`, `*` and `**`.
- **False positives.** Low coverage also comes from OCR errors (signatures, Greek transliteration), which I discarded by hand.

## Verdict

**F-79 is confirmed, and it is broader than 7.6.4.** The served corpus carries three kinds of non-Triglot text:

1. **A copyrighted modern translation: BoC 7.10.0 through 7.10.20 (21 units), "Christian Questions with Their Answers".** This is NOT public domain.
   - The Triglot does not print these Questions. Its Historical Introduction only discusses them (OCR line 14205).
   - The served text is modern English, e.g. "Do you believe that you are a sinner? Yes, I believe it. I am a sinner." Its shingle coverage against the Triglot is 0.06 to 0.56.
   - The same vendored page (`small-catechism.html` lines 1390–1396) carries the notice "From Luther's Small Catechism © 1986 Concordia Publishing House. All rights reserved…". The wording matches that 1986 CPH text.
   - Its heading note is CPH/editorial too: "[The 'Christian Questions with Their Answers,' designating Luther as the author, first appeared in an edition of the Small Catechism in 1551, five years after Luther's death]". That note is currently dropped as an `<h4>` (see F-78).
   - This fails the licensing rule (AGENTS.md: unlicensed or all-rights-reserved is out). The `LICENSES.md` row "Book of Concord … Public domain" is false for these 21 units.
2. **bookofconcord.org editorial notes ("Original Content Copyright 1998–2024 BookOfConcord.Org").** These are not public domain and not Triglot:
   - **7.6.4:** "* These questions may not have been composed by Luther himself but reflect his teachings and were included in editions of the Small Catechism during his lifetime." This is absent from the Triglot, whose section V ends at "…general form of confession for the unlearned." (OCR line 116404).
   - **7.6.1:** "What is Confession ?*". The asterisk is the reference mark for the 7.6.4 note.
   - **2.1.4:** the whole unit, "* catholic means 'universal' and is not a reference to the Roman Catholic Church."
   - **2.1.3:** "the holy catholic* Church". Again the asterisk is the reference mark.
   - **4.5.212:** "The following, through paragraph 213, are left out of the Readers Edition." This sits mid-unit, followed by the bracketed Triglot text "[All prudent men will see…".
3. **Site furniture: links, markup and a stray pointer.**
   - **2.1.5:** the whole unit, "Biblical references for the Apostles' Creed can be found here ."
   - **4.17.70:** the whole unit, "(http://bocl.org?AP+IV+1) and [AP IV".
   - **4.17.106:** the whole unit, "(http://bocl.org?AP+IV+106))".
   - **7.6.4, 9.6.1 and 10.9.1:** stray Markdown `**`, e.g. "**Reverend and dear sir", "**AFFIRMATIVE THESES", "**STATUS CONTROVERSIAE…**".

The parser does not serve these today, but they are not Triglot either. They matter to F-78, because a parser that keeps every text node would start serving them:
- `preface.html`: the modern section headings "The Issues", "The Naumburg Conference of 1561", "The Naumburg Conference Failed", "The Torgau Conference of 1576" and "The Role of the Augsburg Confession". None of these is in the Triglot.
- `defense.html`: an `<h5>` that reads "Shouldn't this be V (IV II) – in Tappert and Kolb it's just an extension of the IV…".
- `small-catechism.html`: the Christian Questions bracket note.
- `small-catechism.html`: the skipped "Prefaratory Notes" and "Small Catechism in PDF" articles. These are already excluded by name.

The 7.6.4 Triglot text is also incomplete, which is F-78's loss, not F-79's. The served 7.6.4 is missing:
- Luther's "Proceed!";
- the servant's confession, "I, a poor sinner, confess myself…";
- "A master or mistress may say thus:" and the master's confession;
- the absolution dialogue: "God be merciful to thee…", "Dost thou believe that my forgiveness is God's forgiveness?" and "As thou believest, so be it done unto thee…".

The Triglot prints all of these (OCR lines 116120–116400).

Checked and clean, with low coverage from OCR only:
- the signatures 3.30.11 and 6.3.15;
- 3.27.32, 6.1.47 and 6.1.39 (reference formatting "2 Thess. 2:3-4" against the Triglot's "2 Thess. 2, 3. 4");
- the Greek transliterations in 4.23.81, 4.23.83 and 10.4.66;
- 7.8.3's "Note: To satisfy the desire means…", which is Triglot (OCR line 116970).

## Suspect ids

| id | kind | excerpt |
|---|---|---|
| 7.10.0–7.10.20 | © 1986 CPH translation | "Do you believe that you are a sinner? Yes, I believe it. I am a sinner." |
| 7.6.4 | bookofconcord.org note | "* These questions may not have been composed by Luther himself…" |
| 7.6.1 | note reference mark | "What is Confession ?*" |
| 2.1.4 | bookofconcord.org note | "* catholic means "universal" and is not a reference to the Roman Catholic Church." |
| 2.1.3 | note reference mark | "the holy catholic* Church" |
| 4.5.212 | bookofconcord.org note | "The following, through paragraph 213, are left out of the Readers Edition." |
| 2.1.5 | site link | "Biblical references for the Apostles' Creed can be found here ." |
| 4.17.70 | site link | "(http://bocl.org?AP+IV+1) and [AP IV" |
| 4.17.106 | site link | "(http://bocl.org?AP+IV+106))" |
| 7.6.4, 9.6.1, 10.9.1 | markup residue | "**Reverend and dear sir", "**AFFIRMATIVE THESES", "**STATUS CONTROVERSIAE" |

## Proposed handling (owner rules)

- **The 7.10 Christian Questions** must leave the served corpus, or be replaced from the Triglot, which has no such text. One option is the article skip list beside the existing two site-furniture skips, disclosed, with `LICENSES.md` corrected.
- **Editorial notes and site furniture** become a typed, counted, non-served part kind: Furniture with a named reason. F-78 Amendment A's closure needs this anyway, so they are excluded by kind rather than by string.
- **The `LICENSES.md` Concord row** should state the exclusions.
- **The category (24a): "the vendored page is assumed to be all Triglot".** It closes with a real-data law: every served unit's 4-gram coverage against a vendored public-domain Triglot reference stays at or above a threshold, with named exceptions. That needs the Triglot reference vendored under `data/raw` with a MANIFEST entry, which is a proposed item for the queue and an owner ruling. I have not filed it.

## Numbering side effect (found with F-78)

The phantom markers in 4.17.70 and 4.17.106 come from the Apology XVIII link residue. They push the source-labelled paragraphs 70–76 to serve as 4.17.107–113. Excluding the residue restores the source numbers. This is recorded in FOCUS-3 Amendment A §A.5 (`lane/claude/F3-amend` 45507a6).
