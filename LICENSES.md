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
| All curated data — historical borders, landmarks, narratives, eras, `events-extra.toml`, and future short blurbs of ours (e.g. era/place descriptions) | **CC0 1.0 Universal** (public domain dedication) | Ours. Everything under `data/curated/` and everything compiled purely from it (`narratives.json`, `eras.json`, `books-meta.json`, `landmarks.json`, `borders/*.json`, `borders-index.json`) |
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
independently-sourced data was considered and rejected: per this batch's
mandate, doing so would cost roughly 450 of the compiled dataset's 476
dated events — the great majority of the app's chronological/narrative
content — for a license condition that public release doesn't actually
require working around.

## Esri / Carto — served live, never redistributed

Both basemap tile services are called directly by the browser at
runtime (`https://server.arcgisonline.com/...` for Esri,
`https://basemaps.cartocdn.com/...` for the Carto fallback) — this repo
never downloads, caches, or ships any tile image. Nothing under
`data/compiled/` or `client/wwwroot/` contains tile imagery. Both are
attributed in the app's Credits popover and in the on-map attribution
control, per each service's terms of use.

## Historical borders — CC0, hand-curated by this project

Earlier in this project's history, historical border snapshots were fetched
from `aourednik/historical-basemaps`, a GPL-3.0-licensed dataset (see this
repo's git history for that earlier determination and the removal below).
**This project's license remediation removed that dependency entirely** so
the repo can be published without carrying copyleft data:

- The download lines in `data/fetch-raw.ps1` and its `data/raw/borders/`
  section in `data/raw/README.md` are gone.
- The aourednik credit in the app's Credits popover is gone.
- `data/raw/borders/` is no longer read by anything (`server/atlas-etl/src/
  borders.rs` now reads `data/curated/borders/` instead — see its module
  doc comment).

In its place: `data/curated/borders/{signed_year}.geojson`, 12 snapshots
(`-4000` .. `100`, matching the app's `[-4004, 100]` era span) of coarse,
original polygons — roughly a handful of named polities per snapshot
(7–29 points per polygon; these render as thin ~1.2px ink strokes on a
small map, so coarse is the right level of detail, not a corner cut).

**Authorship.** Every polygon's coordinates were hand-drawn from scratch by
the Bible Atlas project for this repo. None were copied, traced, digitized,
or algorithmically derived from `aourednik/historical-basemaps` or any
other geographic dataset — that GPL file was not opened at any point while
authoring these replacements. The shapes reflect general, well-established
historical-geography knowledge (the approximate territorial extent of
polities like the Neo-Assyrian, Achaemenid Persian, and Roman empires is
common knowledge repeated consistently across historical references,
including pre-1929 public-domain atlases such as Shepherd's *Historical
Atlas* (1911) and the 1911 *Encyclopaedia Britannica*'s historical maps).
To ground the coarser/larger shapes accurately rather than relying on
recollection alone, the following specific pages were fetched and read
while authoring the corresponding snapshot years (each snapshot's
`.geojson` file also carries this citation in its own `cc0_dedication`
property where applicable):

- [Neo-Assyrian Empire](https://en.wikipedia.org/wiki/Neo-Assyrian_Empire) — `-700.geojson`
- [Achaemenid Empire](https://en.wikipedia.org/wiki/Achaemenid_Empire) — `-500.geojson`
- [Wars of Alexander the Great](https://en.wikipedia.org/wiki/Wars_of_Alexander_the_Great) — `-323.geojson`
- [Seleucid Empire](https://en.wikipedia.org/wiki/Seleucid_Empire) — `-200.geojson`
- [Ptolemaic Kingdom](https://en.wikipedia.org/wiki/Ptolemaic_Kingdom) — `-200.geojson`
- [Parthian Empire](https://en.wikipedia.org/wiki/Parthian_Empire) — `-100.geojson`, `-1.geojson`, `100.geojson`
- [Hasmonean dynasty](https://en.wikipedia.org/wiki/Hasmonean_dynasty) — `-100.geojson`
- [Roman Empire](https://en.wikipedia.org/wiki/Roman_Empire) — `-1.geojson`, `100.geojson`
- [Hittites](https://en.wikipedia.org/wiki/Hittites) — `-1500.geojson`
- [United Monarchy](https://en.wikipedia.org/wiki/United_Monarchy) — `-1000.geojson`

These pages were read only for textual, prose descriptions of territorial
extent (e.g. "reached the Mediterranean," "bounded by the Euphrates") to
inform coordinate placement — no coordinate data, geometry, or map image
was copied from them or from any other source; every vertex was placed by
hand. The remaining, smaller/simpler snapshots (`-4000`, `-3000`, `-2000`
and the small polities within `-1000`/`-700`/`-1`) were authored from
general historical-geography knowledge alone (e.g. Egypt's Nile Delta and
valley up to the First Cataract at Aswan, Mesopotamia between the Tigris
and Euphrates) without an additional fetch, since these extents are not
seriously contested and don't hinge on any one source.

**Dedication.** Every file in `data/curated/borders/` carries a
`cc0_dedication` property at the `FeatureCollection` level with the CC0
text and a pointer back to this section. All of it — the choice of which
polities to depict, the simplification, and every coordinate — is an
original work of this project, dedicated to the public domain under
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
| `borders/*.json`, `borders-index.json` | CC0 (ours) | Compiled from `data/curated/borders/*.geojson`; see "Historical borders" above |
| `report.txt` | Not a licensed dataset | Generated ETL build report (counts/warnings), not app content |

## Everything not listed here

Application code (Rust, C#/Razor, JS/CSS, tests, tooling) is this project's
own and not covered by this file.
