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
| Leaflet ([leafletjs.com](https://leafletjs.com)) | BSD-2-Clause | Vendored into `client/wwwroot/vendor/leaflet/` by `data/fetch-raw.ps1` (not committed; fetched at dev/build time) |
| All curated data — historical polity borders, the land mask, landmarks, narratives, eras, `events-extra.toml`, and future short blurbs of ours (e.g. era/place descriptions) | **CC0 1.0 Universal** (public domain dedication) | Ours. Everything under `data/curated/` and everything compiled purely from it (`narratives.json`, `eras.json`, `books-meta.json`, `landmarks.json`, `polities.json`, `land-mask.json`) |
| Small Catechism — future feature, not yet in this repo | Public domain (1921 Bente–Dau translation) | Will be redistributed once the feature is added; recorded here ahead of time so the disposition is decided before the data lands |

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

## Per-artifact label (`data/compiled/*`)

Compiled outputs are derived works: an artifact built from a
copyleft/attribution-required upstream source carries that source's terms
even though the file itself is generated, not hand-authored.

| Artifact | Source license(s) | Notes |
|---|---|---|
| `canon.json` | Public domain (KJV) | Book/chapter structure derived from parsing the KJV text |
| `verses-kjv.json` | Public domain (KJV) | |
| `places.json` | CC-BY 4.0 (OpenBible geocoding) + CC BY-SA 4.0 (Theographic, for places synthesized only from Theographic's `places.json`) | Merged; the Theographic-derived subset carries share-alike |
| `events.json` | CC BY-SA 4.0 (Theographic `events.json`) + CC0 (`data/curated/events-extra.toml`, ours) | Merged |
| `narratives.json` | CC0 (ours) | `data/curated/narratives/*.toml` |
| `eras.json` | CC0 (ours) | `data/curated/eras.toml` |
| `books-meta.json` | CC0 (ours) | `data/curated/books.toml` |
| `cross-refs.json` | Free to use with credit (OpenBible cross-references) | |
| `landmarks.json` | CC0 (ours) | `data/curated/landmarks.toml` |
| `polities.json` | CC0 (ours) | Compiled from `data/curated/polities/*.toml`; see "Historical polity borders" above |
| `land-mask.json` | CC0 (ours) | Compiled from `data/curated/land-mask.toml`; see "Land mask" above |
| `report.txt` | Not a licensed dataset | Generated ETL build report (counts/warnings), not app content |

## Everything not listed here

Application code (Rust, C#/Razor, JS/CSS, tests, tooling) is this project's
own and not covered by this file.
