# Name model: source research

Date: 2026-09-29. Scope: the queued NAME MODEL (owner-queue-2026-09-29-words-as-base). The model needs:

- KJV word → original-language token alignment, stated explicitly in the data.
- Which entity each occurrence denotes.
- Versification maps.
- Titles resolved to the entity they denote.

Status: read-only research. Nothing was added to `data/`. Samples are in the scratchpad (`names-research/{step,bf,bd,kjv,align}`).

Method: local sources were opened directly. Internet sources were checked against the live repo or page on 2026-09-29, and a web search alone was never taken as confirmation. Two helper agents covered the KJV Strong's sources and the alignment/entity datasets. I spot-checked their main findings (BibleForge and BibleData) myself.

## 0. License table (BINDING rule, owner 2026-09-29)

The rule has three parts:

- **OK:** public domain, CC0, or permissive with attribution as the only condition (CC BY 4.0, MIT, BSD, Apache-2.0).
- **DISQUALIFIED:** ShareAlike or copyleft (CC BY-SA, ODbL, GPL, LGPL, AGPL), NonCommercial, NoDerivatives, personal-use-only, unclear terms, or no license stated.
- **UNCLEAR:** cannot be settled yet.

### 0a. What the project ingests today (data/raw, LICENSES.md, README table)

| Source (current) | License found (verbatim / where) | Verdict |
|---|---|---|
| KJV text, scrollmapper `KJV.json` (`data/raw/kjv.json`) | Text: public domain. The file is CrossWire's SWORD module with its markup stripped. scrollmapper's `sources/en/KJV` README says "License: GPL" for the module. Only the plain text is used, and that text is PD. | **OK** for the text. The license record should say the text is PD and that none of the GPL module markup is used. |
| **Theographic Bible Metadata** (`data/raw/theographic`) | `LICENSE` reads "Attribution-ShareAlike 4.0 International". LICENSES.md says **CC BY-SA 4.0**, and a controller ruling "Stays". | **DISQUALIFIED.** It feeds 450 of the 552 events, part of places.json, the people/place metadata, and Easton text by way of Theographic's `easton.json`. Removing it is a large separate job; see §6. |
| Easton's Bible Dictionary (1897) | PD by age. We read it from Theographic's CC BY-SA bundle. | The text is **OK**, but it must be re-sourced from a PD copy. The Theographic extraction carries SA. |
| OpenBible.info geocoding (`data/raw/geo`) | Repo `license.txt`: CC BY 4.0. The README also says "OpenStreetMap data is licensed under ODbL 1.0". | **OK** for ancient.jsonl, source.jsonl and linked_data. **DISQUALIFIED** for any OSM-derived geometry. Image licenses must be checked one by one. Attribution: "OpenBible.info", CC BY 4.0 link. |
| OpenBible cross-references (`data/raw/xrefs`) | The file header reads `#www.openbible.info CC-BY 2026-08-17`. The site's wording is "free to use with credit". | **OK** (CC BY). Attribution: "OpenBible.info". |
| Red letter: eBible eng-kjv OSIS (`data/raw/red-letter`) | Embedded `<rights>`: "public domain". | **OK** |
| brain-fuel/bible (`data/raw/brain-fuel-bible`): morph/, lexicon/, parallel texts | Repo code is **AGPL-3.0** (`LICENSE`), and we use none of it. Content is licensed per artifact (UPSTREAM-README "License"): CC0 by default; CC BY 4.0 for STEPBible-derived morphology/TVTMS, MACULA domains and LxxLemmas. | **OK**, as long as only data is read and no brain-fuel code is used. Attribution: STEPBible and MACULA (below). |
| STEPBible TAHOT/TAGNT/TBESG (via brain-fuel) | CC BY 4.0 (STEPBible-Data README). | **OK**. See 0c for attribution. |
| MACULA semantic domains (via brain-fuel lexicon) | MACULA is CC BY 4.0. The `sdbh`/`lexdomain`/`coredomain`/`ln`/`domain` columns are UBS material "used with permission". UBS's own public release (ubsicap/ubs-open-license) is **CC BY-SA**. | **UNCLEAR, leaning DISQUALIFIED.** These are the columns the `lexicon_domain` table holds. Drop them, or get UBS's written terms. |
| Strong's 1890 dictionaries (openscriptures/strongs XML) | PD (1890). | **OK** |
| Kretzmann, *Popular Commentary* | kretzmanncommentary.org footer: "Originally published by Concordia Publishing House, 1921–1924. This work is now in the public domain." | **OK** |
| Book of Concord / Small Catechism (Triglot 1921) | bookofconcord.org/copyright/: "These texts are in the public domain and may be freely copied." The site's own intros are "Original Content Copyright 1998-2024". | **OK** for Triglot text only. Nothing written by the site itself may be ingested. |
| **Catechism verse mapping** (brain-fuel/catechism, `data/raw/catechism-mapping`) | No LICENSE file, confirmed on GitHub. LICENSES.md reads "used by the project owner's explicit direction". | **DISQUALIFIED (no license)** until the author grants one. |
| ETCBC BHSA (`~/text-fabric-data`, owner's folder, not ingested) | README: "Attribution-NonCommercial 4.0 International (CC BY-NC 4.0)". The GitHub LICENSE file shows MIT for code. | **DISQUALIFIED** for data. It is already evaluated and rejected (Text-Fabric), so do not ingest. |

Summary: of the sources in use, **Theographic (CC BY-SA)** and the **catechism mapping (no license)** fail. The **MACULA/UBS domain columns** are unclear. Everything else is OK.

### 0b. Candidates evaluated for the name model

| Candidate | License found | Verdict |
|---|---|---|
| STEPBible TIPNR (proper names) | File header and repo README: CC BY 4.0. The header also asks: "Please do not redistribute it yourself" and "without changing the data (You MAY make changes yourself, but you should include a note of changes)". | **OK.** CC BY 4.0 governs, and the extra lines are requests rather than license conditions. Courtesy: keep a change log of our corrections and send them upstream. The AI-written `@Brief/@Short/@Article` descriptions (Claude 3, 2024) are not ingested because they contain errors; see §2. |
| STEPBible TAHOT / TAGNT | CC BY 4.0 | **OK** |
| STEPBible TVTMS | CC BY 4.0 | **OK** |
| STEPBible TTESV (ESV tags) | Filename reads "CC BY-NC". | **DISQUALIFIED** (and it is the ESV) |
| STEPBible TTAraSVD | Filename reads "CC BY-SA". | **DISQUALIFIED** |
| **BibleForgeDB** (KJV word ↔ Hebrew/Greek token) | `readme.txt`: "The texts of the King James Bible, the original Greek and Hebrew … and all other related data, such as the Strong's numbers and grammatical information, are in the public domain … dedicated … to the public domain worldwide (licenses/cc0.txt)". Code: MIT. The Chinese KJV files are CC BY-SA 3.0 and are not used. | **OK (PD/CC0), with a provenance flag.** Its Strong's keying probably descends from the same Bible Foundation / KJV2003 lineage as CrossWire's GPL module, and BibleForge asserts PD. Record the assertion. The owner may want to ask the author to confirm. |
| CrossWire SWORD KJV module (and scrollmapper `KJV-osis.json`) | `kjv.conf`: `DistributionLicense=GPL`. Its About text grants "a general public license to use this text for any purpose". | **DISQUALIFIED** (GPL as declared). Usable only as unpublished QA, unless CrossWire relicenses. |
| eBible eng-kjv / eng-kjv2006 USFM (Strong's on `\w`) | `copr.htm`: "Public Domain … courtesy of the Crosswire Bible Society and eBible.org". | **OK as declared, with a provenance flag**: the tags are CrossWire-derived. Low value, see §1. |
| Clear-Bible/Alignments (BSB, YLT ↔ MACULA) | "All alignment data is licensed under a Creative Commons Attribution 4.0 International License". Code: MIT. BSB and YLT texts: PD. | **OK.** Attribution: "Bible Word Alignments © 2022 by Clear Bible, Inc.", CC BY 4.0 link, changes marked. |
| MACULA Hebrew/Greek (token ids, morphology) | "MACULA Hebrew Linguistic Datasets © 2022-2024 by Biblica, Inc is licensed under CC BY 4.0" | **OK**, excluding the UBS columns (above). |
| Berean Standard Bible and its tables | berean.bible/licensing.htm: "officially placed into the public domain as of April 30, 2023". | **OK** |
| **BibleData** (Brady Stephenson; persons, labels, titles, verses) | `LICENSE` is the CC BY 4.0 legal code; README: "licensed under a Creative Commons Attribution 4.0 International License". GitHub's license detector shows NOASSERTION, but the file is the CC BY 4.0 text. | **OK.** Attribution: "Brady Stephenson, *BibleData*, Zenodo DOI 10.5281/zenodo.19539956", CC BY 4.0 link. |
| OpenBible geo `linked_data` (TIPNR and Wikidata ids) | CC BY 4.0 | **OK** |
| Wikidata | CC0 | **OK**, but see §4 |
| unfoldingWord ULT/UST aligned USFM, en_twl | CC BY-SA 4.0, and the trademark must be removed. | **DISQUALIFIED** |
| BibleAquifer ACAI (per-token entity ids) | "CC-BY-SA 4.0 license" | **DISQUALIFIED.** Copy the design, not the data. |
| UBS SDBH / dictionaries | CC BY-SA | **DISQUALIFIED** |
| Blue Letter Bible | Terms: "Provided you use the whole of our content and do not divide and republish it…" | **DISQUALIFIED** |
| Bible Hub interlinear | "reserves all … rights" and bans commercial use. | **DISQUALIFIED** |
| kaiserlik/kjv | No license, web-scraped. | **DISQUALIFIED** |
| tahmmee/interlinear_bibledata, crizin/bible-db, 1John419/kjs | No LICENSE file / NOASSERTION. | **DISQUALIFIED (unclear)** |
| HF `vincenttsai2024/bible_entity_recognition` | No license. | **DISQUALIFIED** |

### 0c. CC BY attribution obligations

CC BY 4.0 §3(a) requires, wherever the data or a derivative is shared:

- the creator's name as they ask for it;
- a copyright notice if one is supplied;
- the license name and link;
- a link to the source "to the extent reasonably practicable";
- a note that changes were made.

Concretely:

- **STEPBible:** "STEP Bible", linked to www.STEPBible.org, and https://github.com/STEPBible named as the source.
- **Clear:** "Bible Word Alignments © 2022 by Clear Bible, Inc." and "MACULA Hebrew/Greek Linguistic Datasets © Biblica, Inc", each with its repo URL.
- **OpenBible.info**
- **BibleData:** Brady Stephenson, with the Zenodo DOI.

All of these belong in LICENSES.md, the Credits popover, and a per-artifact provenance label. The project already follows that pattern.

## 1. KJV word → original-language token alignment

### Local

- **brain-fuel `morph/` (STEPBible TAHOT/TAGNT normalized to CoNLL-U):** original tokens only. There is **no English-word alignment**. `Align=matched|unmatched` compares Hebrew/Greek surfaces, not English. Extended/disambiguated Strong's (`H3478G`) are reduced to plain numbers, and the TIPNR name tags are dropped. OT refs are already in KJV versification, and this was verified. Our `token` table (LEX-1) comes from here.
- **Theographic `CSV/WordIndex.csv`:** 790,685 KJV words with `PersonID`/`PlaceID` per word. There are no Strong's numbers and no italics (the `Italic` column is 0 on every row). DISQUALIFIED (CC BY-SA) anyway.
- **`kjv.json`:** plain text. The "with Strongs Numbers" label is inherited from the SWORD module, but the markup was stripped.

### Internet

- **BibleForgeDB** (https://github.com/bibleforge/BibleForgeDB; last push 2020-01-12; MySQL dumps):
  - **Structure:** `bible_en` has one row per KJV word, with columns `word, head, clusterID, divine, red, implied, orig_id, notes`. `bible_original` has one row per Hebrew/Greek word, with columns `word, pronun, strongs, morph, orig_order`.
  - **The alignment is explicit, per word, to a specific original token**, in both testaments. It is the only licensable source of its kind.
  - **Verified counts:** 790,850 English words; 763,778 aligned. 27,073 are `implied` (italic/added), and those rows have `orig_id=0`. There are 23,845 empty placeholder rows for untranslated tokens (e.g. אֵת).
  - **Gen 35:10**, verified: both "Israel" words point to their own Hebrew tokens (#13 and #19, H3478), and "Jacob" goes to #5 and #10.
  - **Known defects:** about 17.9k rows carry notes `ERROR` (10,254) or `SPLIT ERROR` (about 7.1k).
  - **Morphology:** OT is TVM codes only; NT is Robinson.
  - **Psalm superscriptions** are verse 0 (116 verses).
  - **KJV text differs from ours in 452 verses**, measured on words with punctuation and dash normalization. Examples: "inquire" where we have "enquire" (Gen 24:57), and "Bethlehem" where we have "Beth–lehem" (Gen 35:19). Its edition is not ours, so our words must be joined to its words by an explicit English-to-English diff (same translation, two editions). Differing words are listed, not guessed.
  - **The Hebrew joins cleanly to STEPBible TAHOT (Leningrad), verified on Gen 1–43.** For 1,291 verses the token counts are equal. 17,142 of 17,256 tokens match by consonants at the same word number under the same KJV-versified reference. All 114 misses are TAHOT's trailing paragraph mark (פ/ס), which normalizes away. The join is checked content-for-content (consonants), not taken on position alone.
  - **The Greek is TR-like.** A crude first pass against TAGNT's KJV-text words ("K" type) matched about 90% of Matthew tokens by aligning Strong's sequences. A proper join has to reconstruct the TR from TAGNT: the K flags plus TAGNT's "TR:" spelling variants. That is work, but it is explicit data.
- **CrossWire SWORD KJV 3.1 (GPL):**
  - OT: one Strong's per phrase, with no token position. Greedy matching errors are visible, e.g. H0853 attached to "created" and to "and" in Gen 1:1.
  - NT: `src="4 5"` gives true TR word positions on all 128,654 `<w>`.
  - Italics are `<transChange type="added">`; margin notes are `<note type="study">`.
  - Excellent NT data, but license-blocked.
- **eBible eng-kjv USFM (PD):**
  - One Strong's per `\w`, with no position and no morphology. The second number is dropped (created = H1254 only).
  - Only 37.9% of Genesis words are tagged.
  - `\add` marks italics. `\f` footnotes hold the 1769 margin: about 2,485 "or," and 4,027 "Heb." notes. The "or," notes need an owner ruling before use; the "Heb." notes are literal-translation notes.
  - It is useful only for the italics and margin-note copy that is already queued. It is weaker than BibleForge for alignment.
- **Clear-Bible/Alignments:** manual, id-based English ↔ MACULA token alignment (Scripture Burrito JSON) for **BSB and YLT only**. There is **no KJV** anywhere in the Clear-Bible org (46 repos). The Greek side is SBLGNT/BGNT, a critical text, so for the NT it is only a cross-check bridge under the KJV directive.
- **unfoldingWord ULT (CC BY-SA)** has the right model (`\zaln-s x-strong x-occurrence`) and is disqualified.
- **Blue Letter Bible and Bible Hub:** disqualified.

## 2. Name disambiguation: entity per occurrence

### Local

- **Theographic** (CC BY-SA, DISQUALIFIED):
  - Contents: 3,067 people with `personLookup` ids (e.g. `zechariah_3004`), 351 with `alsoCalled`, and 1,274 places with `kjvName`.
  - **WordIndex tags KJV words with PersonID, including pronouns.** Coverage is partial and coarse:
    - "Israel": 728 of 2,565 are tagged, all as the person (israel_682). Nation uses are untagged.
    - "Ephraim": 170 of 172 are tagged as the person, although most uses are the tribe.
    - "Saul": 290 go to king Saul, 25 to Paul, and 77 are untagged.
  - Its KJV edition differs from ours in 172 verses (e.g. "Beer-sheba or Sheba", Jos 19:2).
  - Useful only as a comparison check, and not publishable.
- **`data/curated/place-names-kjv.toml`** (43 aliases), NAME-1 dated renames in `place-history.toml`, and `people-groups.toml` (eponymy/reclassify): ours, CC0. These are the seed of the surface-form layer.
- **OpenBible `ancient.jsonl`:**
  - 1,342 places; **1,172 carry a TIPNR id** in `linked_data` (source `s3b25cf`, e.g. `Abana@2Ki.5.12`), and 655 carry a Wikidata QID.
  - `translation_name_counts` gives name variants.
  - This is our place ↔ TIPNR crosswalk (CC BY), already on disk.
- **BHSA `nametype`** (pers/gens/topo/ppde; NC): class labels only, not individuals. The ETCBC `ner/` sheet in `~/text-fabric-data` has 4 entries. Neither is usable.

### Internet

- **STEPBible TIPNR** (updated 2026-09-18; 4,261 records):
  - **Individualised ids.** `UniqueName` is `Name@FirstRef-LastBook`, and `uStrong`/`dStrong` are disambiguated Strong's (e.g. `Zechariah@2Ki.14.29-=H2148P`).
  - **Each record lists every Hebrew/Greek form, with exhaustive references and a KJV spelling** ("Zechariah =ESV,NIV; Zachariah =KJV"). KJV forms appear in 1,066 sub-records and are "not yet exhaustive".
  - **Categories:** PERSON 3,131, PLACE 1,004, OTHER 103, PLACE+PERSON 10. **Types:** Male 2,854, Place 1,013, Female 199, Group 89, Title 7, and others.
  - It carries parents, siblings, partners and offspring. Ambiguous decisions are marked `(?)` and documented.
  - **Limit that matters for our model:** eponyms and nations are not split by kind.
    - `Israel@Gen.25.26-Rev` is the person, and **the nation uses are filed under the person**. Gen 34:7 "folly in Israel" and Gen 36:31 "children of Israel" both tag the person.
    - Egypt *is* split: person Mizraim `H4714H` versus place `H4714G`.
    - So TIPNR resolves *which individual/place* but not always *which kind* (person/people/polity/land). That layer is ours to curate (the eponym relation).
  - The `@Brief/@Short/@Article` text is AI-generated and has errors. Example: `@Short` calls king Zechariah "the last king of the northern kingdom"; Hoshea was. Do not ingest it.
- **STEPBible TAHOT/TAGNT carry the TIPNR id on every name token.**
  - The "Expanded Strong tags" column, for example: `{H0087=אַבְרָם=Abram»Abraham@Gen.11.26-1Pe}`, `{H3068G=יהוה=LORD»LORD@Gen.1.1-Heb}`, `Christ»Christ|Jesus@Mat.1.1`.
  - In the Gen 1–43 sample, 2,543 of 17,657 tokens carry a TIPNR name. **This is entity-per-original-token, explicit, CC BY.** It is exactly the anchor the owner's design note asks for: the denotation sits on the original token, and any aligned translation inherits it.
  - brain-fuel dropped this column, so we must read TAHOT/TAGNT raw.
  - Token ids: `Gen.31.55(32.1)#01=L`, which gives the English ref, the Hebrew ref in parentheses, the word number, and the text type.
  - Text types: **L** Leningrad; **Q/K** qere/ketiv; **X** LXX-derived additions. Under the KJV directive, X is excluded, and K/Q selection is simply following the text the KJV translates.
- **BibleData** (CC BY 4.0; last commit 2026-08-30):
  - 3,009 persons (Zechariah_1…_23).
  - **PersonLabel** (3,747 labels: 3,209 proper names, **533 titles**) carries Hebrew/Greek forms and Strong's. **PersonVerse** (44,267 rows) gives which label denotes which person in each verse, e.g. ECC 1:1 → Solomon_1 via "Preacher".
  - PlaceLabel and PlaceVerse also exist.
  - **Verse-level, not token-level.** Some labels are not KJV spellings ("G-d").
  - Its modelling needs review against our theology/curation: YHVH_1 covers the LORD and the Son (Daniel's "Son of man" → YHVH_1); YHVH_2 is the Father. Take these as candidates, not rulings.
- **ACAI (BibleAquifer):** per-token entity ids, pronoun referents, an Israel person/group/place split, and TIPNR/Theographic crosswalks in `alternate_sources`. It is the closest thing to our design, but **CC BY-SA, so disqualified**, and its own README calls the referent data "an initial draft".
- **MACULA `referent`/`subjref`/`participantref`:** token-to-token coreference, not global entity ids (and pronouns are out of scope).
- **Wikidata:** no TIPNR, Theographic or OpenBible property exists; checked by a SPARQL scan of property labels. P11416 "Strong's number" sits on lexemes, not persons. Q20643955 "human biblical figure" has 1,980 instances. A crosswalk would have to be built, and it is not needed for resolution.

## 3. Versification maps

- **STEPBible TVTMS** (CC BY; updated 2026-07-17):
  - Section blocks give per-tradition mappings in columns KJV / Hebrew / Latin / Greek, for example `$Jol.2:28--3:21 … OneToOne Jol.2:28-32 → Jol.3:1-5` and `Mal.4:1-3 → Mal.3:19-21`.
  - Rule rows give the test that tells which tradition a text follows, e.g. `Mal.4:1=Exist & Mal.3:18=Last`.
  - This is the explicit map we need between KJV and Hebrew (and later other traditions). The 2025 file is KJV-based; an older NRSV-based one is kept under "Older Formats".
- **TAHOT/TAGNT token refs carry both systems per token.** TAHOT gives English plus (Hebrew), and TAGNT gives NRSV with [KJV] brackets. That is versification at the token level with no separate join.
- **Local brain-fuel:** already remapped to KJV using TVTMS plus a CC0 supplement (`data/versification/*.json` upstream). This is recorded provenance, not something to rely on silently.
- **BibleForge:** KJV verse numbers, with Psalm titles as verse 0. Our KJV has the titles inside verse 1, so that is one explicit rule to record.

## 4. Titles → entities

- **BibleData PersonLabel/PersonVerse** is the only licensable per-verse title-to-entity mapping.
  - Examples: "King of Assyria" → Pul/Tiglath-pileser/Shalmaneser/Sennacherib/Esarhaddon per verse, plus a catch-all `King of Assyria_1`, with `H4428, H804` given; "Preacher" → Solomon_1.
  - It is verse-level. To reach word spans, match the label's Strong's sequence against the verse's original tokens (TAHOT) and follow the KJV alignment (BibleForge). That is an explicit data join, and where it is ambiguous (two "king of Assyria" in one verse) the case is listed for curation.
- **TAGNT/TIPNR:** a few titles resolve to their entity (`Christ»Christ|Jesus@Mat.1.1`, "Messiah", "LORD@Gen.1.1-Heb"). TIPNR has only 7 records of type Title, and "Rabbi", "Candace" and "Asiarch" are EXCLUDED.
- **Theographic `alsoCalled`** (Jesus: "Lord, Son, Lamb, Saviour…") is a list only, and CC BY-SA anyway.
- **Nothing licensable gives the "Son of man" title per token with an entity.** BibleData's verse-level assignment has to be reviewed by us.

## 5. Recommendation

1. **KJV word → original token.** Adopt **BibleForgeDB** (PD/CC0) as the only explicit, licensable word-level KJV alignment.
   - Join its Hebrew tokens to **TAHOT** tokens by (KJV ref, word number) with a consonant-equality check. This was verified at 99.3% on Gen 1–43, and 100% after stripping the paragraph mark.
   - Join its Greek tokens to **TAGNT**'s KJV-text (K) words by a Strong's/lemma-checked alignment within the verse.
   - Reach our own `kjv_token` words through an explicit English diff over the 452 verses where BibleForge's edition differs.
   - Its ~17.9k `ERROR`-flagged rows, plus anything that fails a check, become the unresolved list.
   - Record the PD assertion and its probable CrossWire-lineage provenance. SWORD's GPL NT `src=` data can serve as a private QA oracle only.
2. **Entity per occurrence.**
   - Adopt **STEPBible TIPNR** for individual ids and **TAHOT/TAGNT's per-token TIPNR tags** for resolution on the original token. This is the rule-based "from data" resolution the owner asked for.
   - Crosswalk our places to TIPNR through **OpenBible `linked_data`**, which is already on disk.
   - Crosswalk people with **BibleData** (CC BY) plus curation.
   - Keep the **kind split** (person / people / polity / land for Israel, Judah, Ephraim, Edom and similar) as our own curated layer, because TIPNR folds nations into eponyms.
3. **Versification.** Adopt **STEPBible TVTMS**, plus the dual refs TAHOT/TAGNT carry on each token.
4. **Titles.** Adopt **BibleData PersonLabel/PersonVerse** (533 titles, CC BY) as verse-level candidates. Project them onto word spans through the Strong's sequence plus the alignment. The owner curates the result, including the YHVH_1/YHVH_2 modelling and the Son-of-man assignments.

### Gaps no licensable source fills

- **Kind-level denotation** (Israel the man / people / kingdom / land) per occurrence. ACAI tries this but is SA-licensed and a draft; TIPNR lumps these together.
- **Title → entity per word span.** Only verse-level data exists (BibleData), and nothing covers "Son of man" at token level.
- **A second KJV alignment for cross-checking**, since SWORD is GPL.
- **A clean TR token stream keyed to the KJV NT.** It has to be rebuilt from TAGNT's K flags and TR spelling variants.
- **Italic (added) words** have no original token by definition. A name in italics has nothing to anchor to, so it is listed.
- **A publishable replacement for Theographic** (events, people, dates). Outside this research, but the license rule now forces it: BibleData (CC BY) has Person, Event, Epoch and Ussher tables and is the obvious candidate to evaluate.
- **The catechism verse mapping needs a license from its author**, or a re-curation of our own.
