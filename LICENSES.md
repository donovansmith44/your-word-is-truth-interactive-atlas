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
| Catechism verse mapping ([brain-fuel/catechism](https://github.com/brain-fuel/catechism)) | No license file — used by the project owner's explicit direction (see "Batch F2" below) | Redistributed — canonicalized and compiled into `catechism.json` (`CatechismItem.questions`) |
| A.T. Robertson, *A Harmony of the Gospels for Students of the Life of Christ* (1922) | Public domain (published 1922, USA; copyright notice reads "COPYRIGHT, 1922, BY HARPER & BROTHERS") | Not redistributed as text — section titles/numbers and parallel-account groupings consulted as PROVENANCE for our own curated `event-witnesses.toml`/`events-extra.toml` fields (`robertson_section`, `ref_note`); every displayed event title/date/verse citation is independently authored (CC0) or Theographic-credited, per the mapping below |
| William Day Crockett, *A Harmony of the Books of Samuel, Kings and Chronicles* (1897) | Public domain (published 1897, USA; title page reads "Copyright, 1897, William Day Crockett") | Not redistributed as text — section numbers/titles and parallel-account groupings (Samuel/Kings/Chronicles, plus the book's own Appendix of cross-references to Genesis, Joshua, Ruth, Ezra, Nehemiah, Psalms, Isaiah, Jeremiah, Matthew, and Luke) consulted as PROVENANCE for our own curated `event-witnesses.toml` witness rows (Batch W2); every displayed event title/date/verse citation is independently authored (CC0), Theographic-credited, or the compiled KJV text itself, per "Crockett's Harmony of Samuel, Kings, and Chronicles" below |
| James Ussher, *The Annals of the World* (1658) | Public domain (published 1658; this project's own reading is the Larry & Marion Pierce paragraph-numbered English text) | Not redistributed as text — two specific paragraph-numbered entries (¶1202, ¶1227/¶1234) consulted directly as PROVENANCE for two `chronology-anchors.toml` `year` values (`ezra-returns`, `nehemiah-wall`), HOTFIX-6 fix round 2; every displayed event title/verse citation is independently authored (CC0) or Theographic-credited, per "Ussher's Annals of the World" below |

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

## Everything not listed here

Application code (Rust, C#/Razor, JS/CSS, tests, tooling) is this project's
own and not covered by this file.
