# Licenses

Bible Atlas is intended for public release. This file is the canonical,
per-source license record for every dataset and third-party asset the app
uses, including exactly how each is used (redistributed inside
`data/compiled/`, vendored as a client asset, or only ever called live at
runtime and never redistributed). The running app's own **Credits** popover
(click "Credits" in the header — see `client/Layout/MainLayout.razor`)
carries a short runtime summary of the same information for end users; this
file is the detailed version of record.

## Per-source table

| Source | License | How it's used |
|---|---|---|
| KJV text ([scrollmapper/bible_databases](https://github.com/scrollmapper/bible_databases)) | Public domain | Redistributed — compiled into `verses-kjv.json` and `canon.json` |
| OpenBible.info geocoding ([Bible-Geocoding-Data](https://github.com/openbibleinfo/Bible-Geocoding-Data)) | CC-BY 4.0 | Redistributed with credit — compiled into `places.json` (place coordinates) |
| OpenBible.info cross-references | Free to use with credit | Redistributed with credit — compiled into `cross-refs.json`. (The source file's own header row cites itself as `#www.openbible.info CC-BY 2026-08-17`, consistent with this disposition; see `data/raw/README.md`.) |
| Theographic Bible Metadata ([robertrouse/theographic-bible-metadata](https://github.com/robertrouse/theographic-bible-metadata)) | **CC BY-SA 4.0** | **Stays** — redistributed with credit, share-alike. Compiled into `events.json` (event records) and part of `places.json` (places synthesized only from Theographic, not from OpenBible geocoding). See "Theographic — controller ruling" below. |
| Esri `NatGeo_World_Map` tile service | Esri terms of use | **Never redistributed by this repo.** Tiles are fetched live by the browser at runtime from Esri's own servers, with the attribution string carried verbatim in the app UI and in `client/wwwroot/js/map.js`'s `TILE_ATTRIBUTION`. |
| Carto basemap tiles | Carto terms of use | **Never redistributed by this repo.** Same live-fetch, attributed, fallback-only pattern as the Esri tiles above (used if the Esri service is unreachable). |
| Leaflet ([leafletjs.com](https://leafletjs.com)) | BSD-2-Clause | Vendored AND COMMITTED at `client/wwwroot/vendor/leaflet/` (v1.9.4, upstream copyright banner retained in the files per BSD-2's notice condition) — committed so a fresh clone runs without any fetch step; `data/fetch-raw.ps1` can still refresh it |
| All curated data — historical polity borders, the land mask, landmarks, narratives, eras, `events-extra.toml`, `event-witnesses.toml` (Batch T), and future short blurbs of ours (e.g. era/place descriptions) | **CC0 1.0 Universal** (public domain dedication) | Ours. Everything under `data/curated/` and everything compiled purely from it (`narratives.json`, `eras.json`, `books-meta.json`, `landmarks.json`, `polities.json`, `land-mask.json`) |
| Luther's Small Catechism, English (1921 Bente–Dau translation, *Triglot Concordia / Concordia Triglotta*) | Public domain (published 1921, USA) | Redistributed — compiled into `catechism.json`; provenance and verification below |
| The Book of Concord, English (1921 Bente–Dau translation, *Triglot Concordia / Concordia Triglotta*), via [bookofconcord.org](https://bookofconcord.org) | Public domain (published 1921, USA) | Redistributed — compiled into the graph artifact (`graph.bin`) as the "concord" corpus's TextUnit paragraphs; provenance and verification below |
| Catechism verse mapping ([brain-fuel/catechism](https://github.com/brain-fuel/catechism)) | No license file — used by the project owner's explicit direction (see "Batch F2" below) | Redistributed — canonicalized and compiled into `catechism.json` (`CatechismItem.questions`) |
| A.T. Robertson, *A Harmony of the Gospels for Students of the Life of Christ* (1922) | Public domain (published 1922, USA; copyright notice reads "COPYRIGHT, 1922, BY HARPER & BROTHERS") | Not redistributed as text — section titles/numbers and parallel-account groupings consulted as PROVENANCE for our own curated `event-witnesses.toml`/`events-extra.toml` fields (`robertson_section`, `ref_note`); every displayed event title/date/verse citation is independently authored (CC0) or Theographic-credited, per the mapping below |
| William Day Crockett, *A Harmony of the Books of Samuel, Kings and Chronicles* (1897) | Public domain (published 1897, USA; title page reads "Copyright, 1897, William Day Crockett") | Not redistributed as text — section numbers/titles and parallel-account groupings (Samuel/Kings/Chronicles, plus the book's own Appendix of cross-references to Genesis, Joshua, Ruth, Ezra, Nehemiah, Psalms, Isaiah, Jeremiah, Matthew, and Luke) consulted as PROVENANCE for our own curated `event-witnesses.toml` witness rows (Batch W2); every displayed event title/date/verse citation is independently authored (CC0), Theographic-credited, or the compiled KJV text itself, per "Crockett's Harmony of Samuel, Kings, and Chronicles" below |
| James Ussher, *The Annals of the World* (1658) | Public domain (published 1658; this project's own reading is the Larry & Marion Pierce paragraph-numbered English text) | Not redistributed as text — two specific paragraph-numbered entries (¶1202, ¶1227/¶1234) consulted directly as PROVENANCE for two `chronology-anchors.toml` `year` values (`ezra-returns`, `nehemiah-wall`), HOTFIX-6 fix round 2; every displayed event title/verse citation is independently authored (CC0) or Theographic-credited, per "Ussher's Annals of the World" below |
| M. G. Easton, *Easton's Bible Dictionary* (1897) | Public domain (published 1897, USA) | Redistributed — compiled into the graph artifact (`graph.bin`) as `description` on Place/Person/PeopleGroup nodes; delivered via the already-vendored Theographic bundle's own `easton.json` + `people.json` `dictText`/`dictionaryText` fields (CC BY-SA 4.0, see "Theographic — controller ruling" above); see "Easton's Bible Dictionary" below |
| brain-fuel/bible parallel editions ([brain-fuel/bible](https://github.com/brain-fuel/bible)) — Clementine Vulgate, Westminster Leningrad Codex, Douay-Rheims (Challoner), Biblia 1776, Karl XII:s Bibel (1703), Greek Textus Receptus | Public domain (every text; see "brain-fuel/bible parallel editions" below for the verbatim per-edition sourcing this project verified against) | Redistributed — six edition RENDERINGS compiled into the graph artifact (`graph.bin`) as additional `TranslationId` layers on the existing TextUnit `renderings` LayerMap, alongside (never replacing) this app's own canonical KJV text; the app's own code (the brain-fuel repo's data-vs-code separation) is never used |
| Paul E. Kretzmann, *Popular Commentary of the Bible* (Concordia Publishing House, 1921-1924), via [kretzmanncommentary.org](https://kretzmanncommentary.org) | Public domain (published 1921-1924, USA; doubly grounded — see "Kretzmann's Popular Commentary of the Bible" below) | Redistributed — Kretzmann's own commentary PROSE (never the excised KJV lemma/quote text itself, per LEMMA-EXCISION) compiled into the graph artifact (`graph.bin`) as `CommentaryItem` nodes, verse-anchored via `comments-on` edges; provenance and verification below |
| KJV red-letter (words-of-Christ) markup, via [seven1m/open-bibles](https://github.com/seven1m/open-bibles) (eBible.org's own KJV OSIS distribution) | Public domain (the file's own embedded OSIS header states it plainly; see "KJV red-letter markup" below for the full CrossWire-substitution reasoning) | Redistributed — the SOURCE FILE itself is not shipped, but the FACT it encodes (which verses/sub-verse spans are Christ's own words) is: compiled into `graph.bin` as `spoken_by`/`spoken_at` edges (verse-granular) and into `data/compiled/red-letter-spans.json` (KJV sub-verse char-offset spans, compiled-data-side); provenance and verification below |

## Theographic CC BY-SA 4.0 — controller ruling

Theographic Bible Metadata is licensed CC BY-SA 4.0 (share-alike): anyone
redistributing a work derived from it must license that derived work under
the same or a compatible share-alike license. This project's controller
ruled that **this does not block public release** of Bible Atlas as a
whole — share-alike is a copyleft condition on redistribution, not a
prohibition on redistribution, and this repo already credits the dataset
(here and in the app's Credits popover) and labels exactly which compiled
artifacts derive from it (see the per-artifact table below), which is what
CC BY-SA 4.0 requires. Dropping Theographic and replacing it with
independently-sourced data was considered and rejected: doing so would cost
450 of the compiled dataset's 552 dated events (81.5%) — the great majority
of the app's chronological/narrative content — for a license condition that
public release doesn't actually require working around. (Verified directly
against `data/compiled/events.json`: 552 total events, all carrying a
`when`/date, of which 450 have the `theo-` id prefix that marks them as
Theographic-sourced; the remaining 102 are curated,
`data/curated/events-extra.toml`, which independently has exactly 102
`[[event]]` entries.)

## Esri / Carto — served live, never redistributed

Both basemap tile services are called directly by the browser at
runtime (`https://server.arcgisonline.com/...` for Esri,
`https://basemaps.cartocdn.com/...` for the Carto fallback) — this repo
never downloads, caches, or ships any tile image. Nothing under
`data/compiled/` or `client/wwwroot/` contains tile imagery. Both are
attributed in the app's Credits popover and in the on-map attribution
control, per each service's terms of use.

## Historical polity borders — CC0, hand-curated by this project

Earlier in this project's history, historical border snapshots were fetched
from `aourednik/historical-basemaps`, a GPL-3.0-licensed dataset (see this
repo's git history for that earlier determination and the removal below).
**This project's license remediation removed that dependency entirely** so
the repo can be published without carrying copyleft data — the download
lines in `data/fetch-raw.ps1`, its `data/raw/borders/` section in
`data/raw/README.md`, and the aourednik credit in the app's Credits popover
are all gone, and `data/raw/borders/` is no longer read by anything.

Batch B2 ("borders v2, the cartographer's edition," a ground-up redo of the
rendering AND the data model per direct user feedback that the earlier
snapshot-year rendering "looks absolutely horrendous") replaced that
snapshot-year GeoJSON model in turn: in its place, `data/curated/polities/
{id}.toml`, one hand-authored file per polity, each carrying one-or-more
TIMERANGE ERAS (`[[era]]`, with its own `name`/`from`/`to`/`ref_note`/
`rings`) rather than a single fixed-year snapshot — so a polity's borders
AND name are both time-accurate across its own history, not pinned to
whichever of 12 fixed years happened to be nearest. 14 polities, 25 eras,
every ring redrawn from scratch for this batch (never copied forward from
the retired snapshot data) — 30-80 points per ring is the working band
(a handful of the smallest, genuinely tiny polities, e.g. the Persian-
period province of Yehud, sit a little under it, proportionate to their
own real small extent), coarse enough to render as thin ~1.1-1.4px ink
strokes on a small map, never a corner cut.

**Authorship.** Every polygon's coordinates were hand-drawn from scratch by
the Bible Atlas project for this repo. None were copied, traced, digitized,
or algorithmically derived from any third-party geographic dataset. The
shapes reflect general, well-established historical-geography knowledge
(the approximate territorial extent of polities like the Neo-Assyrian,
Achaemenid Persian, and Roman empires is common knowledge repeated
consistently across historical references, including pre-1929
public-domain atlases such as Shepherd's *Historical Atlas* and the 1911
*Encyclopaedia Britannica*'s historical maps). To ground the coarser/larger
shapes accurately rather than relying on recollection alone, the following
specific pages were fetched and read on 2026-08-19 while authoring the
corresponding polity's rings (each era's own `ref_note` in its own
`data/curated/polities/{id}.toml` file carries this same citation):

- [Neo-Assyrian Empire](https://en.wikipedia.org/wiki/Neo-Assyrian_Empire) — `assyria.toml`'s "Neo-Assyrian Empire" era
- [Achaemenid Empire](https://en.wikipedia.org/wiki/Achaemenid_Empire) — `persia.toml`
- [Roman Empire](https://en.wikipedia.org/wiki/Roman_Empire) — `roman-empire.toml`'s two eras
- [Hittites](https://en.wikipedia.org/wiki/Hittites) — `hittites.toml`
- [Seleucid Empire](https://en.wikipedia.org/wiki/Seleucid_Empire) — `seleucid-empire.toml`'s two eras
- [Ptolemaic Kingdom](https://en.wikipedia.org/wiki/Ptolemaic_Kingdom) — `egypt.toml`'s "Ptolemaic Egypt" era

These pages were read only for textual, prose descriptions of territorial
extent (e.g. "reached the Mediterranean," "bounded by the Euphrates") to
inform coordinate placement — no coordinate data, geometry, or map image
was copied from them or from any other source; every vertex was placed by
hand. Every other era in the curated set (Egypt's earlier/later phases,
Babylon, Elam, Sumer, Israel, Judah, Phoenicia, Alexander's empire, Parthia)
was authored from general historical-geography knowledge alone, honestly
disclosed as such in that era's own `ref_note` (no specific source
consulted this session) — extents that are not seriously contested and
don't hinge on any one source (e.g. Egypt's Nile Delta and valley up to the
First Cataract at Aswan, Mesopotamia between the Tigris and Euphrates).

**Dedication.** All of it — the choice of which polities to depict, the
era boundaries, and every coordinate — is an original work of this
project, dedicated to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/),
same as every other file under `data/curated/` (see the per-source table
above) — no separate per-file dedication property is needed here (Batch B2
follows the SAME plain-TOML, no-embedded-license-text convention every
other curated file in this repo already uses, e.g. `eras.toml`/
`landmarks.toml`, rather than the retired GeoJSON model's own
`cc0_dedication` property).

## Land mask — CC0, hand-drawn by this project (Batch R)

`data/curated/land-mask.toml` (compiled to `land-mask.json`), added Batch R
("borders become part of the plate"): ONE coastline/land mask, used only as
an SVG clip so polity washes never spill into open sea — never rendered as
its own visible layer. Six regions (a detailed Levant/Sinai/Nile/Anatolia
ring, a Mesopotamia/Persian Gulf ring, and four coarser regions sized only
to avoid clipping the largest curated polities' own legitimate territory —
Persia's eastern reach, southern Europe, North Africa west of Egypt, and
Cyprus), 135 points total.

**Authorship.** Every coordinate was hand-drawn from scratch by this
project, from general, well-established geographic knowledge of well-known
coastal cities and landmarks (Beirut, Alexandria, the Nile's own Delta and
valley, the Persian Gulf's own shores toward the Strait of Hormuz, etc.).
None were copied, traced, digitized, or algorithmically derived from any
third-party coastline/vector dataset, satellite imagery, or map service —
the same authorship discipline `data/curated/polities/*.toml` is already
held to (see "Historical polity borders" above). No specific source page
was fetched while drawing it (unlike several polity eras above); every
vertex reflects ordinary, non-contested geographic knowledge (where the
Nile Delta is, that the Persian Gulf sits between Mesopotamia and Iran) that
doesn't hinge on any one source, honestly disclosed as such in the TOML
file's own header comment.

**Dedication.** Same as every other file under `data/curated/`: dedicated
to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).

## The Small Catechism — public domain, 1921 Bente–Dau translation (Batch F)

`data/curated/catechism.toml` (compiled to `catechism.json`), added Batch F
("the small catechism" — the user asked for this feature three separate
times): Luther's Small Catechism, English text, the six chief parts item by
item (33 items) — Ten Commandments, Creed, Lord's Prayer, Sacrament of Holy
Baptism, Confession, Sacrament of the Altar. Preface and the Daily Prayers/
Table of Duties appendices were deliberately out of scope this batch
(optional, not gating, per the batch brief).

**Source.** *Triglot Concordia: The Symbolical Books of the Evangelical
Lutheran Church: German-Latin-English*, published as a memorial of the
Reformation's 400th anniversary (St. Louis: Concordia Publishing House,
1921) — the Bente–Dau translation (Editor: Gerhard Friedrich Bente;
English translators: F. Bente and W. H. T. Dau). Published 1921 in the USA:
public domain.

**Text obtained from.** Wikisource's own page-by-page, community-proofread
transcription of the actual 1921 printing —
[Index:Concordia_Triglotta.pdf](https://en.wikisource.org/wiki/Index:Concordia_Triglotta.pdf)
(bibliographic metadata on that page: Editor Gerhard Friedrich Bente,
Publisher "Concordia Pub. House", Address "St. Louis", Year 1921); the Small
Catechism itself is pages 817–849 of that scan, fetched 2026-08-20 via each
page's own `action=raw` wikitext export, e.g.
[Page:Concordia_Triglotta.pdf/825](https://en.wikisource.org/wiki/Page:Concordia_Triglotta.pdf/825).
Not a hand-typed transcription of our own — the actual scanned page text,
proofread by Wikisource's own community process (`pagequality level="1"`
tags on the fetched pages, meaning at least one human proofreading pass).

**Verification (the batch's own gating requirement).** Two independent
spot-checks, both required by the batch brief, both confirmed an EXACT
match: (1) the First Commandment's explanation reads verbatim "We should
fear, love, and trust in God above all things."; (2) the Sacrament of the
Altar's institution wording matches the classic, widely-quoted rendering
exactly. Independently cross-verified against a SECOND source,
[bookofconcord.org](https://bookofconcord.org/about-the-translation/) (whose
own "About the Translation" page states its texts are drawn from "Triglot
Concordia... (St. Louis: Concordia Publishing House, 1921)" and are public
domain): its own quoted answer to "What is the Sacrament of the Altar?"
matches our transcription WORD FOR WORD.

**Editorial handling (disclosed, not silent).** The 1921 Triglotta's own
preface explains its bracket convention explicitly (fetched from
Wikisource's own `Concordia_Triglotta` page): "Brackets in the English text
contain words, phrases, sentences, or shorter or longer passages from the
respective German or Latin text which is not the basis of the
translation" — i.e. bracketed material is a cross-language comparison aid,
not part of "the text proper" of the English translation itself. Every
`text`/`explanation`/`where_written` value in `catechism.toml` is the
UNBRACKETED "text proper" (e.g. the Eighth Commandment's explanation reads
"...but defend him, speak well of him, and put the best construction on
everything," the classic rendering, with the bracketed "[think and]"
variant dropped). A handful of plain OCR scanno artifacts in the Wikisource
transcription (old serif-font letter confusions — "de ceive"/"hut"/"he of
service"/"hegotten"/"baptiam"/"Whhat"/"helieves") are silently corrected to
the plainly-intended reading ("deceive"/"but"/"be of service"/"begotten"/
"baptism"/"What"/"believes") — transcription artifacts of the SCAN, not
variant wording of the 1921 print, so correcting them is more faithful to
the actual 1921 text, not less. Full reasoning, including the
`explanation_heading`/`text`-optionality schema choices and verse-link
derivation, is documented in `catechism.toml`'s own header comment and in
this batch's own report.

**Verse links.** Come from citations Luther's own text embeds — the Small
Catechism's own explicit chapter-and-verse citations are sparse (Baptism's
four parts, the Close of the Ten Commandments, the Sacrament of the Altar's
institution words); most items cite none, a real and disclosed property of
the primary source, not a bug or an omission on our part (no verse link was
invented to make an item "reachable" that Luther's own text doesn't itself
cite).

**Dedication of our own additions.** The curated TOML's structural
choices — which items to split out, their ids/display names, and the small
number of curator judgment calls disclosed in individual items' own
`ref_note` fields (e.g. how an "f."/"ff." citation's endpoint was read) —
are original organizational work of this project, dedicated to the public
domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/),
same as every other file under `data/curated/`; the QUOTED TEXT itself
(Luther's own words, in the 1921 Bente–Dau translation) is public domain by
its own 1921 US publication, not by our dedication.

## The Book of Concord — public domain, 1921 Bente–Dau translation (Batch CORP-2a)

`data/raw/concord/*.html` (vendored, gitignored — see `data/raw/README.md`),
parsed by `server/atlas-etl/src/concord.rs` into the "concord" corpus's
TextUnit paragraphs, compiled straight into the graph artifact
(`graph.bin`) — no `data/compiled/*.json` intermediate (the same
graph-artifact-only disposition the brain-fuel/bible parallel editions
above already use). Added Batch CORP-2a ("BoC before Kretzmann" — owner
order): the graph's second corpus, data half only (the reader tab is
explicitly held for a later brainstorm).

**Source.** *Triglot Concordia: The Symbolical Books of the Evangelical
Lutheran Church: German-Latin-English* (St. Louis: Concordia Publishing
House, 1921) — the SAME Bente–Dau translation, the SAME 1921 publication,
as "The Small Catechism" above (translation consistency across both
Concord ingestions comes free: the Small Catechism's own paragraphs,
re-encountered here as Concord TextUnits, read word-for-word against the
existing curated `catechism.toml` — see the SC-overlap alignment in
`batch-corp2a-report.md`). Published 1921 in the USA: public domain,
regardless of any later site's own copyright notice (the same
never-copyrighted-by-transcription class as this project's planned
Kretzmann ingestion).

**Text obtained from.** [bookofconcord.org](https://bookofconcord.org),
fetched 2026-08-24 (`data/fetch-raw.ps1`'s own `$concordDocs` list; ten
document-root pages, each carrying its OWN document's full text inline —
see `data/raw/README.md`). Translation identity CONFIRMED by two verbatim
probes before vendoring (corp2-scouting.md's own 2026-08-24 RE-SCOUT):
the Formula of Concord Epitome's "Comprehensive Summary, Rule and Norm"
opens "We believe, teach, and confess that the sole rule and standard
according to which all dogmas together with [all] teachers should be
estimated and judged are the prophetic and apostolic Scriptures of the Old
and of the New Testament alone" — the Triglotta's own signature phrasing
(the Kolb-Wengert translation, a DIFFERENT modern translation this project
does not use, reads entirely differently here); the Small Catechism's own
First Commandment reads "We should fear, love, and trust in God above all
things" — byte-identical to this project's own pre-existing, independently
(Wikisource-)sourced `catechism.toml` wording. bookofconcord.org's site
footer ("Original Content Copyright 1998–2024") covers the site's own
chrome/editorial notes, not the 1921 translation it transcribes, which
cannot be re-copyrighted by a later transcription — the PD ground here is
the 1921 publication date itself, exactly as with the Small Catechism
above.

**Cross-check.** Project Wittenberg's own Bente–Dau plain-text Large
Catechism (`luther/catechism/cat-*.txt`) and Bente–Dau-class Augsburg
Confession (`concord/web/augs-*.html`) spot-verify fidelity where both
sites carry the same text — Project Wittenberg is NOT the vendored source
(it is missing the Apology, the Formula of Concord, the Three Ecumenical
Creeds, and the Preface entirely, and its OWN Small Catechism is the
Smith 1994 translation, copyrighted and the wrong translation — never
vendored by this project, on either count).

**Editorial handling (disclosed, not silent).** The source's own inline
paragraph-number markup (`<span class="...-content">N</span>`, one per
`ConcordRef.paragraph` position) is stripped from the rendered TextUnit
text — a citation number is metadata, not prose. Section-heading markup
(`<h3>`/`<h4>`, e.g. "The Second Commandment.") falling between one
paragraph's own close and the next paragraph's own open is likewise
excluded from paragraph text (it is a structural label the source's own
markup places there, never a sentence of the confession itself) — no word
of the confessional TEXT PROPER is ever dropped, only this non-prose
markup. The Small Catechism's own Question/Answer sub-lettering (source
labels like "1", "1b", "1c" for one catechetical unit) merges into ONE
`ConcordRef.paragraph` per base number — Question and Answer are one
paragraph position, matching the existing `catechism.toml`'s own
text+explanation shape for the identical content; two sub-lettered
non-numeric labels ("intro"/"intro-ans", "conclusion"/"conclusion-ans" in
the Lord's Prayer) merge the same way. Full per-document structure
disclosures (including the Three Ecumenical Creeds, whose source carries
no inline paragraph numbers at all, and the Small Catechism's own
Prefatory Notes/Preface/"in PDF" sections, likewise unnumbered) are in
`batch-corp2a-report.md`, per the batch brief's own disclosure
requirement — never force-fit to the triple.

**Dedication of our own additions.** The parser's own document/article
part-and-article numbering (`ConcordRef.part`/`.article`), the "concord"
reading spine's canonical document order, the Small Catechism-overlap
alignment table (`data/curated/concord-sc-overlap.toml`), and every other
organizational choice are original work of this project, dedicated to the
public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/);
the QUOTED TEXT itself (the Book of Concord's own words, in the 1921
Bente–Dau translation) is public domain by its own 1921 US publication,
not by our dedication.

## Catechism verse mapping — brain-fuel/catechism (Batch F2)

`data/curated/catechism-mapping.toml` + the ingested `resources/*.yaml` files
compiled into `catechism.json` (`CatechismItem.questions`, `source =
"brain-fuel/catechism"`), added Batch F2 in direct response to the project
owner's own report that Batch F "derived verse links only from citations
embedded in Luther's own text (6 of 33 items reachable) and FAILED to use —
or even record checking — the user's mapping repo."

**Source.** [brain-fuel/catechism](https://github.com/brain-fuel/catechism),
pinned at commit `0be24fee92e6333f817c4c2a08f99cf7c5274295`. Fetched
2026-08-20 as a GitHub commit-archive zip (`data/fetch-raw.ps1`) —
`resources/*.yaml` (44 files, one per catechism topic — see
`data/curated/catechism-mapping.toml`'s own header for exactly which 37 are
ingested vs. deliberately deferred) and `svebilius/` (the Svebilius
Catechism explanation text, en/fin — fetched, NOT ingested, out of scope
this batch, a candidate for a future one).

**License disposition — controller ruling, recorded verbatim.** The repo
carries no license file. It is used by the EXPLICIT DIRECTION of this
project's owner (2026-08-20, "I gave you the mapping very explicitly in the
catechism repo"), who controls the brain-fuel repository. No evidence
contradicting that was found while fetching: all 50 commits in the repo's
history (as of the pinned SHA) share one author (`brain-fuel
<matt@brain-fuel.co>`), and the repo carries no README/LICENSE asserting
third-party or organizational ownership (the two `svebilius/{en,fin}/README.md`
files describe that subdirectory's own separate source material — the
Svebilius Catechism translation project — not the repo's own ownership).

**What was ingested and how.** Each ingested YAML file is a plain mapping of
QUESTION number -> `{ title, refs: !!set of "Book Chapter:Verse" }`; every
human-readable ref (single verse, same-chapter range, bare chapter, bare
chapter range, cross-chapter range, comma-compound — the full catalog
verified against every one of the ~1550 distinct ref strings actually
present in the fetched data before this ingestion was written) canonicalizes
to this app's own ref grammar (`server/atlas-etl/src/catechism_map.rs`) and
is validated (parses AND exists in the compiled KJV text) the same
fail-loud way every other curated citation in this app already is. Each
question attaches to one of the 33 existing `catechism.toml` items via the
file->item mapping table (`data/curated/catechism-mapping.toml`,
hand-checkable); two files needed a disclosed curator judgment call (the
combined Ninth/Tenth-Commandments file, and the Confession-and-Absolution
file's own thematic split) — see that TOML file's own header and
`batch-f2-report.md` for the full reasoning.

**Dedication of this project's own ingestion work.** The canonicalization
code, the file->item mapping table, and the curated Deuteronomy 5
supplement (`data/curated/catechism-deut5.toml`, requirement 5b — entirely
this project's own authored cross-reference, CC0, see that file's own
header) are original work of this project, dedicated to the public domain
under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/),
same as every other file under `data/curated/`. The MAPPING DATA itself
(question titles + verse citations) retains whatever rights the
brain-fuel/catechism repo's own author holds, used here by that same
author's explicit direction as this project's owner.

## Robertson's Harmony of the Gospels — public domain (Batch T)

Batch T ("events as the narrative nodes: pericopes, chronology, parallel
Gospel accounts") required the owner's own named PRIMARY source for Gospel
section titles, parallel-account groupings, and chronological order: A.T.
Robertson, *A Harmony of the Gospels for Students of the Life of Christ*
(New York, 1922).

**Source and provenance.** Public domain (published 1922, USA — the work's
own copyright page, confirmed directly against two independent digital
mirrors below, reads "COPYRIGHT, 1922, BY HARPER & BROTHERS"). Obtained via
Project Gutenberg eBook #36264 — the HTML edition
(`https://www.gutenberg.org/files/36264/36264-h/36264-h.htm`) and the
plain-text edition (`https://www.gutenberg.org/cache/epub/36264/pg36264.txt`),
fetched live (WebFetch) rather than redistributed here, since only the
section STRUCTURE (numbers, titles, which Gospel passages Robertson groups
together) was needed, never the book's own prose.

**Verification (per the batch brief's own "verify it IS Robertson, spot-
check famous sections" instruction).** Both mirrors were fetched
independently and cross-checked against each other, not trusted from a
single source: section 164 ("THE FIRST THREE HOURS ON THE CROSS," Mark
15:24-32; Matt. 27:35-44; Luke 23:33-43; John 19:18-27) and section 171
("THE VISIT OF THE WOMEN TO THE TOMB OF JESUS...") returned byte-identical
titles and Scripture references from both the HTML and the plain-text
mirror. Section 31 ("THE FIRST CLEANSING OF THE TEMPLE AT THE PASSOVER,"
John 2:13-22) and section 141 ("AT THE FEAST IN THE HOUSE OF SIMON THE
LEPER MARY OF BETHANY ANOINTS JESUS FOR HIS BURIAL") were independently
confirmed the same way. No evidence of a garbled/misattributed OCR text was
found — Project Gutenberg's own edition is a verified, curated transcription
(that's Gutenberg's own standing editorial bar, and this project's fetch
found nothing contradicting it).

**What was used, and how.** Robertson's SECTION NUMBERS/TITLES and his own
grouping of which Gospel passages narrate the same event (e.g. section 128b
grouping Mark 11:1-11, Matt. 21:1-11+14-17, Luke 19:29-44, and John
12:12-19 as one harmonized moment, the Triumphal Entry) directly grounded
this batch's own curated `event-witnesses.toml` (which book+verse-range
pairs are PARALLEL WITNESSES of the same event) and `events-extra.toml`'s
new `robertson_section` field (a provenance tag, auditable back to the
section named). Robertson's own CHRONOLOGICAL ORDER (which section
precedes which) grounded this batch's own `order_key` field for Passion
Week's day-by-day ordering — with ONE disclosed exception: Robertson's own
numbering implies the anointing at Bethany (his section 141) falls AFTER
the triumphal entry/second cleansing (his sections 128b/129), reading John
12:1's own "six days before the passover" as topical rather than strictly
sequential; this app instead keeps the anointing FIRST, at John 12:1's own
explicit chronological placement — a real, disclosed harmonization choice
(see batch-t-report.md), not a transcription error.

Every DISPLAYED event title in this app is our own phrasing (CC0), not
Robertson's own archaic wording, per the batch's own ambiguity ruling:
"the displayed event titles may be our own clearer phrasings... keep a
robertson_section provenance field so the mapping stays auditable." Every
verse citation in `event-witnesses.toml` was independently read against the
compiled KJV text (`data/compiled/verses-kjv.json`) before being written —
Robertson's own section groupings were consulted for STRUCTURE (which
passages belong together, in what order), never trusted as a substitute for
reading the actual cited verses ourselves.

## Crockett's Harmony of Samuel, Kings, and Chronicles — public domain (Batch W2)

Batch W2 ("whole-Bible titled verse containers, second run — the historical
books") required the owner's own named PRIMARY source for the
Samuel/Kings/Chronicles parallel-witness columns (req 1b, "the parallels
must be there in the Bible wide thing too... I'd expect to see parallels
between kings and chronicles"): William Day Crockett, *A Harmony of the
Books of Samuel, Kings and Chronicles: The Books of the Kings of Judah and
Israel* (Fleming H. Revell Company, New York/Chicago/Toronto/London/
Edinburgh, 1897), with an introduction by Willis Judson Beecher.

**Source and provenance.** Public domain (published 1897, USA — the work's
own title page reads "Copyright, 1897, William Day Crockett," confirmed
directly, see verification below). Obtained via a real HTML transcription
hosted at swartzentrover.com/cotor (the Analytical Outline,
`.../cotor/E-Books/christ/Crockett/HSKC_AO.htm`, and the Appendix of
cross-references to other books,
`.../cotor/E-Books/christ/Crockett/HSKC_App.htm`) — fetched live (curl, via
the Bash tool) rather than redistributed here, since only the section
STRUCTURE (numbers, titles, which Samuel/Kings/Chronicles — and other —
passages Crockett groups together) was needed, never the book's own prose;
the site's own transcription additionally quotes the actual Bible text
using the World English Bible (a different, unrelated modern public-domain
translation, its own copyright page confirms) for readability, but that
quoted text was never consulted or trusted — every verse citation used by
this app was independently read against the compiled KJV text instead (see
below).

**Verification (per the batch brief's own "verify it IS Crockett,
spot-check famous parallels" instruction).** Two INDEPENDENT sources were
cross-checked, not trusted from a single mirror: (1) the swartzentrover.com
HTML transcription described above, and (2) a real Google Books scan of a
1951 Baker Book House photolithoprint of the same 1897 work (12 pages,
covering sections 41-62 and 115-121, hosted at a professor's own course
page, `people.bethel.edu/~pferris/ot102/harmonysamuelchron-crockett.pdf` —
an image-only PDF with no text layer, read directly as page images via
Python/PyMuPDF rendering, since it predates OCR). The two sources matched
WORD FOR WORD on real, substantial passages: section 41 ("David chosen and
anointed as Saul's Successor by Samuel," 1 Samuel 16:1-13) through section
51, and section 115 through section 121 ("The Death of David," matching
2 Samuel 5:4-5 / 1 Kings 2:10-11 / 1 Chronicles 3:4b / 1 Chronicles
29:26-30 across FOUR parallel citation columns) — including the scanned
edition's own visible parallel-column layout (e.g. section 46(3), "To the
Cave of Adullam," showing "1 Samuel 22:1, 2" in the main column and
"1 Chronicles 12:16-18" in a second, right-hand column), confirming the
HTML transcription's own structure is a faithful rendering of the real
1897 harmony table, not a paraphrase or a different work entirely. The
title page (its own separate page on the same site,
`.../christ/Crockett/HSKC_TP.htm`) independently confirms authorship,
title, publisher, and the 1897 copyright date.

**What was used, and how.** Crockett's SECTION NUMBERS and his own grouping
of which Samuel/Kings/Chronicles (and other) passages narrate the same
event directly grounded this batch's own curated `event-witnesses.toml`
witness rows (which book+verse-range pairs are PARALLEL WITNESSES of the
same event) — cited in each row's own `ref_note` as "Crockett (1897)
section NNN," since (unlike Robertson's own `robertson_section` field) no
dedicated provenance field was added for this run: overloading
`Event::robertson_section`/`EventWitness::robertson_section` (a field
explicitly named and documented for Robertson's own 1922 Gospel Harmony)
for an unrelated 1897 Old Testament harmony would be a false attribution,
so `ref_note`'s own general-purpose citation field was used instead, the
same disclosure convention `atlas_section`/`acts_section` already
established for non-Robertson provenance. Named cases this run drew
DIRECTLY from Crockett's own Appendix table ("Table of Passages from
various Books of the Bible parallel with certain Sections of the
Harmony"): 2 Samuel 22 ↔ Psalm 18 (section 93); 1 Chronicles 16 ↔ Psalms
96/105/106 (section 94); 2 Kings 18-20 ↔ Isaiah 36-39 (sections 175-176);
2 Kings 24-25 ↔ Jeremiah 52 (sections 182-183, Appendix section VIII). The
owner's own acceptance case — the temple dedication expanding to
per-account passages from EACH account, the Crucifixion pattern — is
Crockett's own section 133 (1 Kings 8 ↔ 2 Chronicles 5-7). Ezra 2 ↔
Nehemiah 7 (also a req 1b named case) has NO Crockett coverage at all —
Crockett's own scope is explicitly Samuel/Kings/Chronicles plus his own
named cross-references, never Ezra or Nehemiah's own narrative content —
so that one pairing was hand-verified directly against the compiled KJV
text instead, disclosed as such in `ezra.toml`/`nehemiah.toml`'s own
header comments.

One CITATION-INTEGRITY CATCH, disclosed (per the standing rule that the
compiled KJV text is the canon of witnesses, not any secondary harmony):
Crockett's own Appendix cites "Isaiah 39:1-9" for the Babylonian-embassy
parallel (section 175(5)) — the compiled KJV text itself shows Isaiah 39
has only 8 verses. This app's own witness row cites the real text (Isaiah
39:1-8), corrected from the secondary source's own apparent transcription
slip, disclosed in `2kings.toml`'s own header comment and the witness
row's own `ref_note`.

Every DISPLAYED event title in this app is our own phrasing (CC0), not
Crockett's own period wording. Every verse citation in
`event-witnesses.toml`'s own "1 Chronicles/2 Chronicles <-> Samuel/Kings"
sections and the named cross-book cases was independently read against the
compiled KJV text (`data/compiled/verses-kjv.json`) before being written —
Crockett's own section groupings were consulted for STRUCTURE (which
passages belong together), never trusted as a substitute for reading the
actual cited verses ourselves.

## Ussher's Annals of the World — public domain (1658; Batch HOTFIX-6 fix round 2)

`data/curated/chronology-anchors.toml`'s own header comment had, until this
round, explicitly disclaimed any need for a LICENSES.md row: "the Annals'
own text was NOT consulted/quoted directly... this table cites bare YEAR
VALUES only." That was true through fix round 1. It stopped being true this
round: an independent review's Critical finding C-1 (the `nehemiah-wall`
anchor's own "-444, Ussher's Annals of the World" citation was very likely
a mis-cited MODERN figure, not Ussher's own) required actually reading
Ussher's own prose, with paragraph numbers, to settle it — exactly the
"source's own PROSE read as curatorial provenance" trigger that header
comment named as needing this row. This section follows the SAME
Source/Verification/What-was-used shape as "Robertson's Harmony of the
Gospels" and "Crockett's Harmony of Samuel, Kings, and Chronicles" above.

**Source and provenance.** *The Annals of the World* (Latin: *Annales
veteris testamenti*, 1650, continued 1654; English translation 1658).
Public domain (1658 publication; also true of the modern Larry & Marion
Pierce paragraph-numbered English edition consulted here, itself out of
print but reproduced in full at the two archive.org items below). Obtained
via two independently uploaded Internet Archive scans, fetched live
(Bash/`curl` + `pdftotext`) rather than redistributed here: `https://
archive.org/details/james-usher-annals-of-the-world` (PDF, uploader
"Liber-T") and `https://archive.org/details/AnnalsOfTheWorld` (a
SEPARATE upload, different uploader, own `.txt` OCR export) — both
retrieved 2026-08-22.

**Verification (two independent copies, the same standard this file
already applies to Robertson/Crockett).** Both scans were converted to
plain text and independently checked for the three paragraphs this batch
relies on; all three matched WORD FOR WORD (modulo trivial OCR
line-wrap/whitespace and one single-letter OCR slip in the second scan,
"EM" for "Elul," on a word this batch doesn't cite): paragraph 1202
("Ezra the priest... obtained permission from Artaxerxes the king...",
marginal date "3537b AM, 4247 JP, 467 BC"), paragraph 1227 ("In the same
20th year of the king, in the month Nisan, Nehemiah's turn came to serve
as cupbearer... and obtained permission from the king to be the governor
of Judah and to rebuild Jerusalem," marginal date "3550c AM, 4260 JP, 454
BC"), and paragraph 1234 ("the wall was finished in 52 days, on the 25th
day of the 6th month called Elul," same 454 BC marker block covering
paragraphs 1231-1237). No formal publisher/edition colophon is present in
either archive.org item's own scanned pages (both are bare community
uploads), but the paragraph-numbering-plus-triple-date (Anno
Mundi/Julian-Period/BC) marginal-marker apparatus across both scans is the
Larry & Marion Pierce English edition's own distinctive, widely-recognized
format — not Ussher's own 1658 original English printing's format, which
carries no such apparatus.

**What was used, and how.** Two bare facts (paragraph 1202's own dated
sentence for Ezra's commission; paragraphs 1227/1234's own dated sentences
for Nehemiah's commission and the wall's completion) were consulted as
PROVENANCE for two `chronology-anchors.toml` `year` values — `ezra-returns`
(-467, already correct since fix round 1, now carrying this citation) and
`nehemiah-wall` (-444 -> -454, the C-1 correction itself). Nothing else
from the Annals was consulted or is redistributed; no prose beyond the
three short quoted sentences above appears anywhere in this repo. The
correction is independently cross-checked against this atlas's own
internal arithmetic (Ezra 7:7-8's "seventh year of Artaxerxes" + Nehemiah
2:1's explicit "the same twentieth year," a 13-year textual gap,
uncontested across chronological schools) — both paths agree exactly,
disclosed together in `chronology-anchors.toml`'s own `nehemiah-wall` row.

## Easton's Bible Dictionary — public domain (1897; Batch ENT-1a)

Batch ENT-1a ("Easton's descriptions: the data half") required the owner's
own order (2026-08-23, verbatim): "it doesn't make sense to just show a
whole bunch of verses when you click on a person or place... we actually
want meaningful information about who or what someone is, having that be
backed by scripture." This section follows the SAME Source/Verification/
What-was-used shape as "Robertson's Harmony of the Gospels"/"Crockett's
Harmony of Samuel, Kings, and Chronicles"/"Ussher's Annals of the World"
above.

**Source and provenance.** M. G. Easton, *Easton's Bible Dictionary*
(also published as *An Illustrated Bible Dictionary*), 3rd edition, 1897.
Public domain (published 1897, USA). NOT independently fetched this
batch — the text rides the ALREADY-VENDORED Theographic Bible Metadata
bundle (`data/raw/theographic/theographic-bible-metadata-master/json/`,
present since this project's own initial data vendoring): `easton.json`
(6,519 standalone dictionary entries, each carrying `dictLookup`/
`dictText`/`matchType`/`matchSlugs`) and, per-person, `people.json`'s own
`dictText` (a Theographic-pre-joined array) / `dictionaryText` (an older
plain-text sibling) fields — the exact fields an earlier batch (Batch P,
"the extensibility proof") imported, found out of its own scope, and
DISCLOSED as dropped (`atlas_core::data::Person`'s own doc comment, prior
to this batch). This batch reads them, adds no new fetch, and disclaims
no textual criticism of the underlying Bible text — Easton's OWN prose,
where it touches on a place/person/people's identity, is carried
verbatim, per this project's standing KJV-inerrancy-adjacent discipline
of never editorializing a source's own words.

**Verification (per this file's own established "verify it IS the named
source, spot-check a real entry" bar).** The "Aaron" entry was fetched
live from an independent, unrelated mirror,
[biblestudytools.com/dictionaries/eastons-bible-dictionary/aaron.html](https://www.biblestudytools.com/dictionaries/eastons-bible-dictionary/aaron.html)
(2026-08-24), which explicitly attributes its text to "Easton's Bible
Dictionary": its own opening sentences —"the eldest son of Amram and
Jochebed, a daughter of Levi (Exodus 6:20). Some explain the name as
meaning mountaineer, others mountain of strength, illuminator." — match
the vendored `people.json` `dictionaryText` field for `aaron_1` WORD FOR
WORD (modulo the vendored copy's own verse-citation spacing). The SAME
entry's `dictText` field (and the standalone `easton.json` entry
`matchSlugs: "aaron_1"`) carries the identical prose, re-flowed with
markdown scripture-ref links (Theographic's own newer extraction) —
confirmed a re-formatting of the SAME underlying text, not a different
source.

**What was used, and how.** `atlas_etl::easton::parse_easton` (reading
`easton.json` + `places.json`'s own `slug` field for place-name
resolution) and `atlas_etl::people::parse_people`'s widened `dict_text`
resolution (reading `people.json`) feed `atlas_graph::description_adapter
::fill_descriptions`, a trust-ordered matcher (person's own source
record, THEN Theographic's own attested `matchType`/`matchSlugs` join,
THEN a conservative exact-name fallback — batch-ent1a-brief.md's full
order) that fills `NodePayload::{Place,Person,PeopleGroup}.description`
with ONE WHOLE, VERBATIM `dictText` string per matched node, never
concatenated, trimmed-and-rebuilt, or synthesized from parts — proven by
`description_adapter.rs`'s own tests. Over the real committed data (this
batch, 2026-08-24): 2,344 of 3,067 persons (76.4%) and 837 of 1,373
places (61.0%) carry a real description; 0 of 0 PeopleGroup nodes (none
exist yet — PG-1a, a later batch, seeds real ones). Every DISPLAYED
description is Easton's own words; no prose in this app's own voice is
ever mixed into a `description` field.
UPDATED (Batch PG-1a, 2026-08-24, same day): PeopleGroup nodes now exist
(23 Theographic `peopleGroups.json` records + 6 curated nation seeds + 9
Gen-10 gentilics reclassified from Person) — 8 of 38 (21.1%) carry a real
description, tier c (exact `dict_lookup` match) only, per `description_
adapter.rs`'s own trust order (no per-record dictText source, no
Theographic id to key tier b on for a PeopleGroup): Arkite/Sinite/Zemarite
(3 of the 9 reclassified, whose OWN singular Theographic name already
matches an Easton headword of the same shape), Canaanites/Philistines (2
of the 6 curated seeds), Pharisees/Sadducees/Scribes (3 of the 23
Theographic groups) — the twelve tribes, Nation of Israel, and every
other Theographic/curated group miss (no Easton headword shaped like
"Tribe of Judah" exists, unsurprisingly). Disclosed miss pattern, per
PG-1a's own brief (not silently patched): four of the six curated nation
seeds (Ammonites/Moabites/Edomites/Amalekites) miss specifically because
the curated label is PLURAL while Easton's own headword for that entity
is SINGULAR ("Ammonite" not "Ammonites") — an exact-match miss tier c's
own no-fuzzy-matching law correctly declines to bridge; batch-pg1a-report.md
has the fuller breakdown. The person total also moved (2,337 of 3,058,
still 76.4%) -- nine fewer source records (the reclassified gentilics are
PeopleGroup now, not Person), a real, disclosed consequence of the
reclassification, not a data loss.

**Dedication of this project's own matching work.** The trust-order
matcher itself (`description_adapter.rs`, `atlas_etl::easton.rs`) — which
source wins when, the ambiguity-collision rule, the place-name-vs-id-space
resolution — is original work of this project, dedicated to the public
domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
The QUOTED TEXT itself (Easton's own words, however Theographic re-flowed
its markdown) is public domain by its own 1897 publication, carried here
under the SAME CC BY-SA 4.0 delivery-vehicle disposition "Theographic —
controller ruling" above already covers for the whole Theographic bundle.

## brain-fuel/bible parallel editions — public domain (Batch CORP-1a)

Batch CORP-1a ("brain-fuel editions: the ingestion half") required the
owner's own named source for editions beyond the KJV — owner order
(verbatim, via the controller): "3 - take all. no apocrypha for now." This
section follows the SAME Source/Verification/What-was-used shape as
"Easton's Bible Dictionary"/"Robertson's Harmony of the Gospels" above.

**Source and provenance.** [brain-fuel/bible](https://github.com/brain-fuel/bible),
pinned at commit `94d44842cb242e8aa840330748e03d2803f2a7c1`. Fetched
2026-08-24 as a GitHub commit-archive zip (`data/fetch-raw.ps1`) — ONLY
`data/books.json` and every `bible/ot/*/*.json` + `bible/nt/*/*.json`
chapter file (929 + 260 = 1,189 files) are vendored into `data/raw/
brain-fuel-bible/` (see that directory's own section in `data/raw/README.md`
for the full, verified JSON shape); `bible/apo/`, `bible/lxx/`, the
morphology/lexicon/relation-graph directories, and all Go/Python tooling
are NOT vendored — data, never code, same separation this project's
catechism-mapping fetch above already establishes.

**Six editions ingested, each independently confirmed public domain by
brain-fuel/bible's own metadata, verbatim (not merely trusted from one
summary):**

- **Clementine Vulgate** (Latin, 1592). brain-fuel's own `README.md`:
  "Sourced from Scrollmapper's bible_databases datasets... Ten OT verses
  are merged into the preceding verse in the Vulgate tradition." Also
  named explicitly in brain-fuel's own `docs/LICENSING.md`, under its own
  "Content — CC0-1.0 (default)" section, as public domain by age. `data/
  editions.json` manifest row: `"license":"PD"`.
- **Westminster Leningrad Codex** (Hebrew Masoretic Text, full pointing).
  brain-fuel's own `README.md`: "Sourced via the Sefaria API. Includes
  full Masoretic pointing with vowels and cantillation marks." Also named
  explicitly in brain-fuel's own `docs/LICENSING.md` CC0-1.0 section.
  `data/editions.json`: `"license":"PD"`.
- **Douay-Rheims** (Challoner revision, English). brain-fuel's own
  `README.md`: "Sourced from Scrollmapper's `DRC` dataset. Follows Vulgate
  versification... thirteen KJV verses are merged/absent in the Douay
  tradition." `data/editions.json`: `"license":"PD"`. NOT separately named
  in brain-fuel's own `docs/LICENSING.md` quick-map table (disclosed, not
  glossed over — that table's own CC0 list names only three of brain-fuel's
  eight editions by name; nothing in either of brain-fuel's own documents
  asserts a DIFFERENT license for Douay-Rheims, and its `data/editions.json`
  row is unambiguous). Verified DIRECTLY over the real vendored files
  (`server/atlas-etl/tests/brainfuel_real_data.rs`): brain-fuel's own
  Douay-Rheims coverage is OT-ONLY in this dataset (no `douay_rheims` key
  ever appears in any `bible/nt/*.json` file) — the real, historical
  Douay-Rheims Bible has an NT (Rheims, 1582) too, but brain-fuel's own
  repo simply does not carry it; this app imports exactly what brain-fuel
  ships, honestly disclosed as OT-only rather than assumed complete.
- **Biblia 1776** (Finnish). brain-fuel's own `README.md`: "Sourced from
  Scrollmapper's `FinBiblia` dataset. Already KJV-versified for the
  protocanon and NT, so it is placed by identity with no versification
  map." `data/editions.json`: `"license":"PD"`.
- **Karl XII:s Bibel** (Swedish, 1703). brain-fuel's own `data/editions.json`
  manifest row's own `note` field, quoted verbatim: "Text: www.kxii.se
  transcription of the public-domain 1703 Karl XII Bible (Fraktur to
  Antiqua, mechanical corrections only, per the site's data/andringar.txt);
  the site claims no license of its own; 1703 orthography kept as-is."
  Also named explicitly in brain-fuel's own `docs/LICENSING.md` CC0-1.0
  section, with the identical "site claims no license of its own"
  disclosure. `data/editions.json`: `"license":"PD"`.
- **Greek Textus Receptus** (New Testament). brain-fuel's own `README.md`:
  "Sourced from the Logos Apostolic interlinear." `data/editions.json`:
  `"license":"PD"`. Not separately named in brain-fuel's own `docs/
  LICENSING.md` quick-map table (same disclosed gap as Douay-Rheims above
  — nothing asserts otherwise, and the Greek Textus Receptus is itself a
  16th-century critical text, uncontroversially public domain by age
  regardless of this particular transcription's own license tag).

**KJVA (`king_james_apocrypha`) is SKIPPED, ruled on rather than silently
descoped:** its own 66-book canonical coverage exactly duplicates this
app's own KJV base (brain-fuel's own `data/editions.json` marks it
`"base": true`, `"versification":"kjv"`, `"testaments":["apo"]` — i.e. its
ONLY unique content is the apocryphal books), and this batch imports no
apocrypha at all (owner ruling, "no apocrypha for now") — so KJVA would
add zero content this batch is scoped to use.

**What was used, and how.** `server/atlas-etl/src/brainfuel.rs` (parsing)
and `server/atlas-graph/src/brainfuel_adapter.rs` (graph merge) import
each edition's own text VERBATIM (byte-for-byte, including source
whitespace/typographic artifacts such as the Vulgate's own Johannine-
Prologue bracket markers) as an additional `TranslationId` rendering
alongside — never replacing or correcting — this app's own canonical KJV
text, per the KJV INERRANCY DIRECTIVE. brain-fuel's own `king_james`
column is read ONLY for an owner-ordered cross-check against this app's
own canonical KJV text (raw byte-for-byte mismatch count: 9,274 of 31,102
compared positions — manually verified, every single one a typographic/
transcription-convention difference: trailing whitespace, the traditional
`LORD`/`Lord` Tetragrammaton-case convention this app's own scrollmapper-
sourced `data/raw/kjv.json` does not preserve, Psalm-superscription-into-
verse-1 and Psalm 119 acrostic-header folding present in this app's own
source but not brain-fuel's, and a small residue of genuine spelling
variants — NEVER a case of verse-content substitution; alignment itself
is fully sound, zero positions missing from either side) — brain-fuel's
own KJV text is NEVER imported anywhere; this app's own KJV base stays
the sole authoritative KJV rendering the graph carries.

**Dedication of this project's own ingestion work.** The parser, the
book-code resolution (reusing `kjv::normalize_book_name`), the graph-merge
adapter, and the cross-check logic are original work of this project,
dedicated to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
The QUOTED TEXT itself (each edition's own words) is public domain by its
own age/publication, per the per-edition sourcing above, not by this
project's dedication.

## Kretzmann's Popular Commentary of the Bible — public domain (1921-1924; Batch KRETZ-1)

Batch KRETZ-1 required the owner's own order (2026-08-24, verbatim, via the
controller): "pull kretzmann commentary (public domain version) into our
corpora" — "a comprehensive commentary without the verses interleaved into
it, and it's indexed so that each verse mapped bit of commentary is mapped
to the appropriate verse in our graph." This section follows the SAME
Source/Verification/What-was-used shape as "The Book of Concord"/
"brain-fuel/bible parallel editions" above.

**Source and provenance.** Paul E. Kretzmann, *Popular Commentary of the
Bible* (St. Louis: Concordia Publishing House, 4 volumes — 2 Old Testament,
2 New Testament — 1921-1924). Public domain, DOUBLY GROUNDED: the original
volumes were published without a copyright notice (pre-1978 US rule:
immediate public domain), and all four volumes predate 1930 regardless —
public domain by age alone, independent of the first ground. Obtained from
[kretzmanncommentary.org](https://kretzmanncommentary.org), a modern digital
edition, fetched 2026-08-25 (`data/fetch-raw.ps1`'s own `$kretzmannBooks`
table; all 66 books, every KJV chapter page — see `data/raw/README.md`'s own
"kretzmann/" section for the full fetch tally and page-template details).
The site's own text states, verbatim (fix round 1: quoted directly, not
paraphrased — captured in `.superpowers/sdd/2026-08-17-bible-atlas-m1/
kretzmann-scouting.md`): "Originally published by Concordia Publishing
House, 1921–1924. This work is now in the public domain." — consistent
with the PD grounds above (a later digital transcription cannot
re-copyright an already-public-domain 1921-1924 work).

**What was used, and how.** `server/atlas-etl/src/kretzmann.rs` (parsing)
and `server/atlas-graph/src/kretzmann_adapter.rs` (graph merge) implement
the owner-ruled ANNOTATION shape: the bold/quoted KJV lemma is the parser's
own JOIN KEY and is EXCISED (LEMMA-EXCISION) — this app's own canonical KJV
text remains the SOLE source of verse text; only Kretzmann's OWN commentary
PROSE is stored, as one `CommentaryItem` node per verse-anchored unit,
linked to its own Bible locus range via a `comments-on` edge. KRETZ-ACCEPT-1
(the owner's own acceptance test, verbatim: "(initial commentary+verses)
-(initial commentary)===(whole Bible) / should be exact match") was run as
an ETL-time law over the WHOLE corpus, checked against this app's own
RESTORED (KJV-CASE + KJV-CASE-2) canonical text: 31,040 verses checked (62
verses Kretzmann summarizes with no lemma of their own — lawful, disclosed,
never guessed); 2,525 exact byte matches; 23,614 pass under a disclosed
MECHANICAL case+punctuation equivalence (the Tetragrammaton/reverential-
pronoun case convention, and the digital edition's own fragment/quote-
boundary punctuation style — never a word/content difference); 1,903 more
pass under a curated (never fuzzy-matched) American/British spelling-variant
table, mined and manually vetted from the real corpus's own recurring
mismatches (`shew`/`show`, `honour`/`honor`, `sepulchre`/`sepulcher`, and
~130 further pairs — `server/atlas-etl/src/kretzmann.rs`'s own
`SPELLING_VARIANTS` table has the full list and its own exclusion
discipline). The remaining 2,998 (9.7%) are genuine, disclosed deviations,
manually sampled across the whole corpus (both testaments, both page
templates) and categorized — by far the largest class is Kretzmann quoting
only a verse's own opening clause before moving to prose commentary
(ordinary commentary-writing style); smaller classes are this app's own
canonical source's en-dash compound-name convention for Hebrew proper names
diverging from the digital edition's own hyphenation, and Psalm 119's own
Hebrew-letter acrostic stanza headers (folded into each stanza's own first
verse by this app's canonical convention) never being quoted by Kretzmann's
own lemma at all. Manual sampling found NO case of genuine verse-content
substitution anywhere — the SAME conclusion, and the SAME disclosure
discipline, "brain-fuel/bible parallel editions" above already reached over
an independent cross-check source (9,274 of 31,102 raw mismatches, "every
single one a typographic/transcription-convention difference... NEVER a
case of verse-content substitution"). `server/atlas-etl/tests/
kretzmann_real_data.rs` pins the exact counts above as a regression harness.

**Fix round 1** (2026-08-25, review finding 2, MEDIUM): a bolded run
occasionally carried Kretzmann's OWN prose in the SAME `<strong>`/quote
span as genuine KJV text — e.g. EXO 20:12's own second span bolded ~68
words of homiletic exposition alongside the genuine trailing clause "that
thy days may be long..."; RUT 4:11's own third span bolded a 6-word
translator's aside, "literally, that is about to come,", inline between
two genuine KJV phrases. LEMMA-EXCISION as originally implemented silently
destroyed that prose (excised alongside the genuine lemma, then dropped).
The OVER-EXCISION GUARD (`server/atlas-etl/src/kretzmann.rs`'s own
"OVER-EXCISION GUARD" section has the full algorithm — recursive
longest-common-block reconciliation against the verse's own canonical
text, never a plain prefix scan, needed to correctly recover BOTH real
shapes above) now recovers this prose mechanically, corpus-wide: 1,046
real instances found and disclosed (`corpus.stats.over_excisions`; fix
round 2 moved this from 1,054 — see below), not merely the 2 the
reviewer's own narrower sample found. This moved KRETZ-ACCEPT-1's own
pinned numbers (above) from 2,498/22,933/1,853/3,748 to 2,525/23,614/
1,903/2,998 (`checked`/`uncovered` also move, fix round 2's own doing —
see below) — the class deltas sum to zero. Both named verses were verified
to recover their prose VERBATIM against the real source HTML (byte-for-byte
reconstruction of the original `<strong>` span), and a machine guard now
asserts stored prose never contains its own excised fragment text
(`stored_prose_never_contains_its_own_excised_fragment_text` in
`kretzmann_real_data.rs`).

**Fix round 2** (2026-08-26, re-review NEW FINDING, MEDIUM): a mid-sentence
Type-B verse boundary occasionally rendered as literal inline text ("v. N")
instead of a `<sup>` tag defeated verse-marker splitting, swallowing the
FOLLOWING verse's own genuine KJV text into the PRECEDING verse's own
fragment — which fix round 1's own over-excision guard then recovered as
if it were Kretzmann's prose (content never lost, but mislabeled).
`find_inline_verse_marker` (`kretzmann.rs`) now recognizes this shape
mechanically: literal "v. N" not preceded by a letter (so "Lev. 1"/
"Rev. 5" book-citation abbreviations never trigger) where N is EXACTLY the
verse immediately following the one open — the sequential-adjacency
requirement that tells a genuine forward boundary apart from an ordinary
backward cross-reference (a real corpus counter-example, LEV 21:14's own
"v. 7" citing back to verse 7, is correctly never matched). **8 real
instances found corpus-wide** (MAT 26:61, MAT 27:40, LUK 2:35, LUK 17:21,
LUK 19:42, LUK 19:46, LUK 20:2, LUK 20:36), each verified against the real
source HTML and independently reconciled against its own canonical text.

**KRETZ-ACCEPT-2** (new law, owner ruling 2026-08-25, verbatim:
"commentary-comments===bible"): a SEPARATE identity from KRETZ-ACCEPT-1
above, about the COMPOSED PRODUCT rather than the parse itself — for every
verse in canonical order, canonical text followed by its own mapped
comments, with every comment segment stripped back out, must reconstruct
the whole canonical Bible EXACTLY, all 31,102 verses including the 70
uncovered, no equivalence tiers. Verified over the real corpus
(`kretz_accept_2_composed_reading_view_strips_to_exactly_the_whole_canonical_bible`)
— exact, zero residual, confirming this app's own verse text stays
single-sourced (never derived or mutated by the commentary layer) even
after fix round 1's own prose-recovery changes.

**The date mine** (owner order, verbatim: "extract the years from Kretzmann
and throw them somewhere as our tentative source of truth that gets shared
everywhere"): a parsing-only pass over the stored commentary prose for
B.C./A.D./Anno-Mundi dating clauses, emitted to `data/exports/
kretzmann-chronology.json` (`status: "tentative-extraction"` — a later,
separate curatorial act adjudicates real chronology placements from it;
this export carries no placement authority of its own). 84 real clauses
found (43 B.C., 41 A.D., 0 Anno Mundi — disclosed absent, not assumed; fix
round 1 moved this from 83/42/41/0 — a recovered-prose unit can gain a
dating clause that used to be silently excised as if it were KJV text);
every verbatim clause is asserted to be a literal substring of its own
unit's stored prose.

**Dedication of this project's own ingestion work.** The parser, the
LEMMA-EXCISION join-key logic, the conservation law and its own curated
spelling-equivalence table, the date-mine extraction, and the graph-merge
adapter are original work of this project, dedicated to the public domain
under [CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
The QUOTED PROSE itself (Kretzmann's own words) is public domain by its own
1921-1924 publication, not by this project's dedication.

## KJV red-letter markup — public domain (Batch RED-1)

Batch RED-1 ("Red letters on Jesus' words in every translation," owner
order 2026-08-25, verbatim, ledgered: "Red letters on Jesus' words in every
translation."; "SpokenAt is another edge") required a SOURCED annotation
layer marking Christ's own words in the reader — red-letter marking is an
1899 editorial convention (Louis Klopsch), never part of the underlying
KJV text itself, so this is an annotation over loci, never a text mutation
(byte-verbatim rendering stays intact; the KJV inerrancy directive is
untouched — ink color is presentation, not text).

**Source and provenance — the CrossWire-substitution disclosure.** The
controller's own PRIMARY candidate was CrossWire Bible Society's own
current KJV Sword module — the standard digital KJV, marking Christ's
speech with `<q who="Jesus">` at sub-verse precision, on the same
1769-class text family this project already uses. Investigated directly
(CrossWire's own wiki, `wiki.crosswire.org/CrossWire_KJV`, fetched at batch
time): the module's red-letter designations derive from Louis Klopsch's own
1901 red-letter edition (itself long public domain), and the underlying KJV
text is public domain outside the UK (Crown Copyright has no effect
outside Britain) — but the MODULE ITSELF, as CrossWire currently packages
and distributes it (version 3.1+), carries a module-level `GPL`
`DistributionLicense`, because it bundles genuinely copyrighted third-party
components alongside the markup (The Bible Foundation's Strong's numbers
for the Old Testament, Dr. Maurice Robinson's Greek data for the New
Testament). This FAILS the everything-public-domain gate for direct use of
CrossWire's own file (decision 1's own explicit gate) — not because the red
-letter FACT itself is tainted (Klopsch 1901 is PD; the fact of "which
words Klopsch marked red, mechanically re-encoded in XML" is not an
independently copyrightable creative addition, the same "mere transcription
of a PD source creates no new copyright" principle this file's own "Book of
Concord" section above already establishes for bookofconcord.org's
transcription), but because CrossWire's own CURRENT distribution vehicle
bundles it with unrelated, genuinely copyrighted apparatus this project has
no use for and cannot cleanly separate from an unambiguously-PD download.

**Used instead**, satisfying the SAME sub-verse-precision goal cleanly:
eBible.org's own KJV OSIS distribution, obtained via
[seven1m/open-bibles](https://github.com/seven1m/open-bibles) (a curated
collection of public-domain/freely-licensed Bible translations in standard
XML formats — the collection's own README table states `eng-kjv.osis.xml`
"Public Domain" plainly, one row among many, entirely independent of
CrossWire's own module). Pinned to commit
`f257a3559025c3f873b48a75019f53a9354ed7de`, fetched 2026-08-25
(`data/fetch-raw.ps1`). The file's own embedded OSIS header states its own
rights unambiguously: `<rights type="x-copyright">public domain</rights>`,
with the full text `<rights type="x-license">`: "This work is in the Public
Domain... A United Kingdom Crown Copyright with no expiration date means
that to print this translation in the United Kingdom, you need letters
patent... This Crown Copyright has no effect outside of the UK, where this
work is firmly in the Public Domain." — the SAME UK-Crown-Copyright/PD-
everywhere-else disposition this project's own existing KJV source already
carries (per-source table above). The header also discloses the file's own
generation lineage: "This Modified OSIS file was generated from a USFX
source as part of the process to convert to a SWORD module using Haiola" —
i.e. this file sits UPSTREAM of a Sword-module build, an independent
production pipeline from CrossWire's own currently-distributed module, not
a copy of it.

**Verification (two independent checks, both required before vendoring).**
(1) The file's own embedded header, quoted verbatim above — an unambiguous,
self-contained PD declaration, not inferred. (2) A cross-check of the
resulting VERSE SET (which verses the source marks as containing Christ's
words) against a SECOND, fully independent PD source: the World English
Bible (`eng-web.usfx.xml`, also via `seven1m/open-bibles`, USFX format,
its own `<wj>`/`</wj>` words-of-Jesus tags — a different translation
entirely, tagged by a different editorial process). Over the whole New
Testament: KJV OSIS marks 2,066 verses, WEB USFX marks 2,043; the two sets
agree on 2,001 verses (Jaccard ≈0.949, WEB covering ≈96.9% of the KJV
OSIS set) — strong, independent agreement confirming the verse set is a
real, broadly-recognized editorial consensus, not an idiosyncratic
artifact of one source or one markup convention. (Disagreements are
concentrated in a handful of predictable classes — e.g. whether a
narrator's brief interjection mid-quotation breaks a `<q>` run — exactly
the kind of boundary judgment call two independent taggers would be
expected to differ on occasionally; neither source was treated as more
authoritative than the other for this sanity check, and only the KJV
OSIS source's own spans are ever used for real data.)

**What was used, and how.** `server/atlas-etl/src/red_letter.rs` parses
the vendored file into (a) the VERSE SET (every verse the source marks,
edition-independent by construction) and (b) KJV SUB-VERSE SPANS
(byte-offset ranges into OUR OWN canonical, RESTORED KJV verse text,
never the source's own bytes) via the GAZ-1 alias-law pattern (`atlas-etl/
src/validate.rs`'s own `run_place_names_kjv`): every span is aligned as a
case-sensitive verbatim substring of our own text first; failing that, a
case-insensitive retry (the disclosed KJV-CASE class — our restored
LORD/Lord casing vs. this source's own normalization), offsets still
emitted against OUR bytes; failing both, the span is counted and
categorized, never guessed. Real committed-data outcome (`server/
atlas-etl/tests/red_letter_real_data.rs` pins these as a regression
harness): 2,066 verses in the SET; 2,081 total source spans, of which
2,059 align exactly, 4 align only case-insensitively, and 18 (0.87%) are a
fully-categorized residual — never bridged by guessing — 14 spelling
variants this source spells differently than our own canon (`Caesar`/our
`Cesar` ×4, `Judaea`/our `Judea` ×6, `Galilaeans`/our `Galileans`,
`Zacchaeus`/our `Zaccheus`, `Nicolaitanes`/our `Nicolaitans` ×2,
`Barjona`/our `Bar–jona`), two dropped possessive apostrophes, one dropped
comma, and one source-side literal typo (MAT.5.30 "cut **if** off" for
"cut **it** off", the digital edition's own transcription slip). The
VERSE SET (edition-independent) feeds `SpokenBy` graph edges (one row per
maximal contiguous verse range, speaker = the Jesus person node); the
SUB-VERSE SPANS (KJV-specific) feed `data/compiled/red-letter-spans.json`,
served alongside verse payloads and rendered as a `.words-of-christ` CSS
span wherever verse text renders in the reader. `SpokenAt` (place) edges
are DERIVED, never separately sourced: a `SpokenBy` range whose verses
fall inside a located event's own attested range inherits that event's
place, honestly disclosed as partial coverage (the gospels are
well-evented; epistle-quoted sayings mostly are not) — zero curated
guesses.

**Dedication of this project's own ingestion work.** The parser
(`red_letter.rs`), the alignment law's own implementation, the SpokenBy/
SpokenAt derivation logic, and the `.words-of-christ` rendering rule are
original work of this project, dedicated to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).
The FACT itself — which words are Christ's own, per Klopsch's 1901
red-letter edition as encoded by eBible.org's own OSIS markup — is public
domain by the 1901 edition's own age and the KJV text's own public-domain
status outside the UK, not by this project's dedication; the SOURCE FILE
itself is never redistributed by this repo (`data/raw/red-letter/` is
gitignored, fetched fresh by `data/fetch-raw.ps1`), only the FACT it
encodes, re-derived against our own independently-sourced canonical text.

## Per-artifact label (`data/compiled/*`)

Compiled outputs are derived works: an artifact built from a
copyleft/attribution-required upstream source carries that source's terms
even though the file itself is generated, not hand-authored.

| Artifact | Source license(s) | Notes |
|---|---|---|
| `canon.json` | Public domain (KJV) | Book/chapter structure derived from parsing the KJV text |
| `verses-kjv.json` | Public domain (KJV) | |
| `places.json` | CC-BY 4.0 (OpenBible geocoding) + CC BY-SA 4.0 (Theographic, for places synthesized only from Theographic's `places.json`) | Merged; the Theographic-derived subset carries share-alike |
| `events.json` | CC BY-SA 4.0 (Theographic `events.json`) + CC0 (`data/curated/events-extra.toml` + `data/curated/event-witnesses.toml` + `data/curated/acts-sections.toml` + `data/curated/atlas-sections.toml` + `data/curated/passages/*.toml`, ours) | Merged; Batch T's own `witnesses`/`robertson_section`/`order_key` fields are CC0 curated additions, GROUNDED IN (not copied from) the public-domain Robertson Harmony structure — see "Robertson's Harmony of the Gospels" above. Batch T2's own `acts_section` field (`data/curated/acts-sections.toml`) is a separate CC0 curated addition for Acts 1-12 sectioning, honestly NOT grounded in any external public-domain source (see that file's own header comment) — its natural-narrative-unit boundaries come from Theographic's own pre-existing event data, already covered by the CC BY-SA 4.0 credit in this same row. Batch W1's own `atlas_section` field is the general, whole-Bible sibling of `acts_section` — same CC0 status, same "no external PD source claimed" honesty, used two ways: enriching a pre-existing bare Theographic event's own heading-worthiness (`data/curated/atlas-sections.toml`, same "boundaries come from Theographic's own pre-existing event data" disclosure as `acts_section`), or a brand-new container authored directly (`data/curated/passages/*.toml`, one file per book — Genesis, Exodus, Leviticus, Numbers, Deuteronomy, Joshua, Judges, Ruth as of Batch W1; 1/2 Samuel, 1/2 Kings, 1/2 Chronicles, Ezra, Nehemiah, Esther added Batch W2 — title/verse-range/date/place all CC0 ours, citation-verified against the compiled KJV text). Batch W2's own `event-witnesses.toml` additions (the "1 Chronicles/2 Chronicles <-> Samuel/Kings" sections and the Ezra 2 ↔ Nehemiah 7 / 2 Samuel 22 ↔ Psalm 18 / 1 Chronicles 16 ↔ Psalms 96,105,106 / 2 Kings 18-20 ↔ Isaiah 36-39 / 2 Kings 24-25 ↔ Jeremiah 52 named cases) are CC0 curated additions GROUNDED IN (not copied from) the public-domain Crockett Harmony structure where a Crockett section is cited — see "Crockett's Harmony of Samuel, Kings, and Chronicles" above; the Ezra/Nehemiah pairing is hand-verified against the compiled KJV text directly, disclosed as outside Crockett's own scope. |
| `narratives.json` | CC0 (ours) | `data/curated/narratives/*.toml` |
| `eras.json` | CC0 (ours) | `data/curated/eras.toml` |
| `books-meta.json` | CC0 (ours) | `data/curated/books.toml` |
| `cross-refs.json` | Free to use with credit (OpenBible cross-references) | |
| `landmarks.json` | CC0 (ours) | `data/curated/landmarks.toml` |
| `polities.json` | CC0 (ours) | Compiled from `data/curated/polities/*.toml`; see "Historical polity borders" above |
| `land-mask.json` | CC0 (ours) | Compiled from `data/curated/land-mask.toml`; see "Land mask" above |
| `catechism.json` | Public domain (1921 Bente–Dau translation) + CC0 (our own organizational choices + Deut5 supplement) + brain-fuel/catechism (owner-directed use, no license file) | Compiled from `data/curated/catechism.toml` + `catechism-mapping.toml` + `catechism-deut5.toml`; see "The Small Catechism" and "Catechism verse mapping" above |
| `chronology-anchors.json` | CC0 (ours) + Public domain (Ussher's Annals of the World, 1658, for two `note` values only) | Compiled from `data/curated/chronology-anchors.toml`; every `year`/`label`/`note` is CC0 curated authorship, with two `note` values (`ezra-returns`, `nehemiah-wall`) GROUNDED IN (two short quoted sentences from, not copied wholesale from) Ussher's own public-domain text — see "Ussher's Annals of the World" above, added Batch HOTFIX-6 fix round 2 |
| `book-narration-windows.json` | CC0 (ours) | Compiled from `data/curated/book-narration-windows.toml`, derived arithmetically from `chronology-anchors.toml`'s own values, added Batch HOTFIX-6 |
| `report.txt` | Not a licensed dataset | Generated ETL build report (counts/warnings), not app content |
| `red-letter-spans.json` | Public domain | Batch RED-1, added 2026-08-25 — see "KJV red-letter markup" above. (This table predates the graph-artifact-centric model and does not otherwise cover `graph.bin`'s own per-corpus license mix, a pre-existing gap outside this batch's own scope; `graph.bin`'s `spoken_by`/`spoken_at` edges carry the SAME public-domain disposition as this row.) |

## Everything not listed here

Application code (Rust, C#/Razor, JS/CSS, tests, tooling) is this project's
own and not covered by this file.
