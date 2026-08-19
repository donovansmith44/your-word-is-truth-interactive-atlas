# data/raw — actual fetched formats (ground truth for atlas-etl)

This directory is gitignored and populated by `data/fetch-raw.ps1` (idempotent — skips
files that already exist; delete a file/folder and re-run to refresh it). Everything
below was recorded by actually opening the downloaded files on 2026-08-17. Per spec
§4.3: **the compiled schema (data/compiled/*.json), not this raw schema, is the
contract** — this file exists so atlas-etl's parsers are written against reality.

## URL changes made from the original script draft

Two of the four original source URLs had moved; `fetch-raw.ps1` was updated in place
and the changes are recorded here:

1. **KJV JSON**: `formats/json/kjv.json` (lowercase) 404s on the scrollmapper repo.
   The file was renamed upstream to `formats/json/KJV.json` (uppercase). Script now
   fetches `.../formats/json/KJV.json` and still saves it locally as `raw/kjv.json`.
2. **OpenBible geocoding bundle**: `https://a.openbible.info/geo/data.zip` now
   returns `403 Forbidden` (AmazonS3/CloudFront — the object is gone, confirmed with
   a browser-UA `curl`, not a User-Agent block). openbible.info's geo data now lives
   only on GitHub as JSON Lines files (no single zip):
   `https://github.com/openbibleinfo/Bible-Geocoding-Data`. This is a genuinely
   different (much richer, nested) schema than the old flat CSV bundle the spec
   assumed — see "geo/" below. The script now fetches 5 files individually from
   `raw.githubusercontent.com/openbibleinfo/Bible-Geocoding-Data/master/data/*.jsonl`
   into `raw/geo/`.

The other two sources (Theographic zip, openbible cross-references zip) and the
vendored Leaflet 1.9.4 files fetched clean on the original URLs — verified with
`curl -I` and by running the script (see below).

## Verified: `powershell -NoProfile -ExecutionPolicy Bypass -File data/fetch-raw.ps1`

First run fetched all artifacts (after the two URL fixes above); second run printed
`have <file>` for every artifact, confirming idempotency. Final directory listing:

```
data/raw/kjv.json                                    8,400,187 bytes
data/raw/geo/ancient.jsonl                           11,550,193 bytes
data/raw/geo/modern.jsonl                              3,224,520 bytes
data/raw/geo/geometry.jsonl                              155,985 bytes
data/raw/geo/image.jsonl                                1,469,114 bytes
data/raw/geo/source.jsonl                                 225,544 bytes
data/raw/theographic.zip                              23,970,583 bytes
data/raw/theographic/theographic-bible-metadata-master/...  (extracted)
data/raw/cross-references.zip                          1,982,837 bytes
data/raw/xrefs/cross_references.txt                        (extracted)
client/wwwroot/vendor/leaflet/leaflet.js                 147,552 bytes
client/wwwroot/vendor/leaflet/leaflet.css                 14,806 bytes
```

---

## `kjv.json` (public-domain KJV text)

Single JSON object, NOT an array. Despite the translation label mentioning "Strongs
Numbers and Morphology and CatchWords", the `text` field is plain unannotated prose
(no embedded Strong's-number tags) — verified across multiple books.

```json
{
  "translation": "KJV: King James Version (1769) with Strongs Numbers and Morphology and CatchWords",
  "books": [
    {
      "name": "Genesis",
      "chapters": [
        { "chapter": 1, "verses": [ { "verse": 1, "text": "In the beginning God created the heaven and the earth." }, ... ] },
        ...
      ]
    },
    ...
  ]
}
```

Path: `books[].chapters[].verses[]`. Fields: `books[].name` (string), `chapters[].chapter`
(1-based int), `verses[].verse` (1-based int), `verses[].text` (string).

All 66 Protestant-canon books present, in canonical order. **Book naming uses
old-style names, not the OSIS/USX abbreviations used by the other datasets**:
`"I Samuel"`, `"II Samuel"`, `"I Kings"`, `"II Kings"`, `"I Chronicles"`, `"II Chronicles"`,
`"I Corinthians"`, `"II Corinthians"`, `"I Thessalonians"`, `"II Thessalonians"`,
`"I Timothy"`, `"II Timothy"`, `"I Peter"`, `"II Peter"`, `"I John"`, `"II John"`,
`"III John"`, and `"Revelation of John"` (not "Revelation"). A name-mapping table from
these forms to canonical/OSIS book codes will be needed to cross-reference the
Theographic (`Gen`, `1Sam`/`osisRef` style) and cross-reference (`Gen.1.1`, `1Sam.1.1`)
datasets, which both use OSIS abbreviations.

---

## `geo/` — OpenBible.info geocoding data (5 JSON Lines files, replaces the old zip)

Each file is JSON Lines (one JSON object per line, not a JSON array). Records are
deeply nested — this is a materially richer schema than the flat "name/lat/lon" CSV
the spec assumed. For M1's point-marker use case, only **`ancient.jsonl`** (places +
verse links) and **`modern.jsonl`** (clean canonical coordinates) are needed;
`geometry.jsonl`, `image.jsonl`, `source.jsonl` are auxiliary (polygon/line shapes,
photo metadata, bibliography) and not required for M1.

### `ancient.jsonl` — the place ↔ verse mapping (per line, key fields)

```
id                 short id, e.g. "aea17b7"
friendly_id        display name, e.g. "Abana"
url_slug           e.g. "abana"
types               ["river"] / ["mountain range"] / ["settlement"] / etc.
identifications[]   candidate resolutions to a modern place, each with:
  .id                modern place id (e.g. "m39ac0b") -- NOTE: this is the SAME id as
                     identifications[].resolutions[].modern_basis_id, and the SAME id
                     used as the key in modern_associations{} and in modern.jsonl's own "id"
  .score             confidence/vote score (int; higher = better identification)
  .resolutions[]      each has:
    .lonlat            STRING "lon,lat" (comma-separated, LONGITUDE FIRST) e.g. "36.305000,33.513542"
    .lonlat_type       "point" | "representative point"
    .land_or_water     "land" | "water"
    .type              "settlement" | "river" | "mountain range" | ...
    .modern_basis_id   id into modern.jsonl
verses[]            array of verse links, each:
  .osis               OSIS ref, e.g. "2Kgs.5.12", "Num.27.12" (also range-free single-verse form)
  .readable           human string, e.g. "2 Kgs 5:12"
  .usx                USX-style ref, e.g. "2KI 5:12"
  .sort               zero-padded sortable string, e.g. "12005012"
  .translations[]      which translations mention the place at this verse, e.g.
                       ["csb","esv","kjv","leb",...] -- FILTER on "kjv" present to
                       restrict to KJV-relevant links
modern_associations{} map keyed by modern id -> {name, score, url_slug}, same data as
                     identifications[].id but flattened/deduped
```

To resolve one coordinate per ancient place: pick the highest-`score` entry in
`identifications[]`, take its first `resolutions[].lonlat`, split on `,` as
`lon,lat` (reversed from the usual lat,lon order). Multiple identifications per
place are common (ambiguous ancient places); most have exactly one.

Real sample line (truncated) — id `aea17b7`, friendly_id `"Abana"`:
`{"friendly_id":"Abana",...,"identifications":[{"id":"m39ac0b","score":1000,"resolutions":[{"lonlat":"36.305000,33.513542","lonlat_type":"representative point","land_or_water":"water","type":"river","modern_basis_id":"m39ac0b",...}],...}],...,"verses":[{"osis":"2Kgs.5.12","readable":"2 Kgs 5:12","translations":["csb","esv","kjv","leb","nasb","net","niv","nkjv","nrsv"],"usx":"2KI 5:12"}]}`

**Corrections found while implementing atlas-etl (Task 4), against the real,
full `ancient.jsonl` (not just the sample line above):**

1. `identifications[].score` is NOT a plain int as described above and shown
   in the truncated sample line — on the real "Abana" record (`aea17b7`) it is
   actually a nested stats object: `{"time_best_fits":[],"time_intercept":1000,
   "time_r_squared":0,"time_slope":0,"time_total":1000,"time_values":[],
   "vote_average":500,"vote_count":1,"vote_total":500}`. The truncated sample
   above was apparently hand-simplified when this doc was written and doesn't
   match the actual file. The plain-int score genuinely usable for ranking is
   `modern_associations[modern_id].score` instead (confirmed int on every
   record checked) — atlas-etl's `geo::parse` ranks `resolutions[]` by looking
   up each one's `modern_basis_id` in `modern_associations` and comparing
   those scores, not `identifications[].score`.
2. Not every `resolutions[]` entry has `lonlat`/`modern_basis_id` at all: a
   resolution can be a dead-end marker shaped
   `{"special":"not_a_place","type":"special","best_path_score":...,
   "best_time_score":...,"class":...,"description":"not a place","paths":[...]}`
   with neither field present (real example: id `aaee94d`, friendly_id
   `"Addar"`, one of whose `identifications[0].resolutions[]` entries is
   exactly this shape). `geo::parse` treats both fields as optional and skips
   any resolution missing either one.

### `modern.jsonl` — canonical modern-place records (per line, key fields)

Flatter than ancient.jsonl. Top-level fields (no nested resolutions):

```
id                 e.g. "m207993" -- joins to ancient.jsonl's identifications[].id / modern_basis_id
friendly_id        e.g. "Abarim"
lonlat             STRING "lon,lat" (comma-separated, LONGITUDE FIRST), top-level (not nested)
type / class       "mountain range" / "settlement" / etc.; class is "natural" | "human"
land_or_water      "land" | "water"
names[]            [{ "name": "...", "type": "modern", "url_slug": "..." }, ...] (name variants)
ancient_associations{}  reverse map back to ancient ids -> {name, score, url_slug}
```

Real sample line (truncated) — id `m207993`:
`{"ancient_associations":{"aa8275b":{"name":"Abarim","score":1000,...},...},"class":"natural","friendly_id":"Abarim","id":"m207993","land_or_water":"land","lonlat":"35.715200,31.753900","names":[{"name":"Abarim","type":"modern","url_slug":"abarim"}],"type":"mountain range",...}`

### `geometry.jsonl`, `image.jsonl`, `source.jsonl` (fetched, not needed for M1)

- `geometry.jsonl`: polygon/line boundary coordinate lists for regions/rivers (keyed
  by ids like `g3d49f6` referenced from `identifications[].resolutions[].geojson_roles`).
  Not needed since M1 renders point markers only, no polygons.
- `image.jsonl`: photo/thumbnail metadata (credits, URLs). M1 has no place-photo feature.
- `source.jsonl`: bibliography/citation records (book/author/year). Not needed.

---

## `theographic/theographic-bible-metadata-master/` (extracted zip; note the nested folder)

GitHub zip archives nest one level: the actual files are under
`data/raw/theographic/theographic-bible-metadata-master/`, not directly in
`data/raw/theographic/`. Relevant subfolder: `json/` (8 files: `books.json`,
`chapters.json`, `easton.json`, `events.json`, `people.json`, `peopleGroups.json`,
`places.json`, `verses.json`). The repo also ships an authoritative field reference
at `docs/json-fields-documentation.md` (cross-checked against real records below;
matched in every case checked).

**Every file is a JSON array `[...]` (not JSON Lines) of records shaped
`{ "id": "recXXXXXXXXXXXXXX", "createdTime": "ISO8601", "fields": { ...entity data... } }`.**
All cross-references between tables are via these 14-char `rec...` record ids.

**Important — no `periods.json` exists.** The spec's assumption of a Theographic
"periods" table is stale: the repo's own docs say the periods table "was removed."
`books.json`'s `yearWritten` field (which referenced it) is documented as "currently
in unusable format" — do not rely on it. `data/curated/eras.toml` and
`books.toml` (§4.5) must be entirely hand-authored, which matches the spec (they were
already specified as hand-curated).

### `events.json` (450 records) — dated events; the spine of narratives/scenes

```
fields.title              string, e.g. "Creation of all things"
fields.startDate          string, astronomical year number, e.g. "-4003" (= 4004 BC;
                           negate and add 1 for BC year). Present on most but not all records.
fields.duration            e.g. "7D" (7 days), "1D"
fields.participants[]      -> people.json record ids
fields.locations[]         -> places.json record ids (present on 305/450 = ~68% of
                           records checked by grep count -- i.e. roughly a third of
                           events have no location, matches "Incomplete" status in docs)
fields.verses[]            -> verses.json record ids
fields.predecessor[]        -> another events.json record id (single-element array)
fields.partOf[]             -> parent event's record id
fields.sortKey             float, year + verseSort/1e8, for chronological sort
fields.rangeFlag           bool, true = year is approximate
fields.eventID             int, stable numeric id
```

Real sample (id `recOIPcFaPCjMpQ6w`): `{"title":"Creation of all things","startDate":"-4003","duration":"7D","participants":["reccZB8SVU5bEMcgo",...],"verses":["rec7mkRLwey2ntUG9",...],"verseSort":"01011001","sortKey":-4002.98...,"eventID":1}` — note this particular record has NO `locations` field at all (field is simply absent, not empty array) — code reading `fields.locations` must treat missing-key the same as empty.

### `places.json` — geographic locations

```
fields.placeLookup        e.g. "egypt_362" (name_id, stable-ish slug-like key)
fields.kjvName             string, name as it appears in KJV text
fields.latitude            string (!) decimal degrees -- "best available" per docs
                           (falls back through Recogito -> OpenBible priority)
fields.longitude           string (!) decimal degrees, SAME order as normal (lon is
                           lon, lat is lat here -- unlike the openbible geo/ files above)
fields.openBibleLat/Long   string, OpenBible.info-sourced lat/long specifically
fields.featureType         e.g. "Region", "Water" (present for some, esp. Acts)
fields.verses[]             -> verses.json record ids (verses mentioning this place)
fields.eventsHere[]          -> events.json record ids
fields.slug                url-friendly, e.g. "egypt_362"
fields.displayTitle        disambiguated display name, e.g. "Egypt"
```

`latitude`/`longitude` are strings, not numbers — must `.parse()` in atlas-etl. Real
sample confirms field names exactly match `docs/json-fields-documentation.md`.

### `people.json`, `peopleGroups.json`, `chapters.json`, `easton.json`

Matched `docs/json-fields-documentation.md` field-for-field on spot-checked real
records (not independently re-transcribed here; see that file for the full field
list if Task 4/5 need people/peopleGroups). Key ids for cross-referencing:
`people.json` records use `fields.personLookup` (e.g. `"israel_682"`) and
`fields.birthYear`/`deathYear` (string, same astronomical-year convention as events).

### `books.json` (66 records)

```
fields.osisName    OSIS abbreviation, e.g. "Rom", "Gen" -- use this to cross-reference
                    verses.json/events.json osisRef and the xrefs TSV (both use this scheme)
fields.bookName     full name, e.g. "Romans" (NOTE: differs from kjv.json's "I Samuel"
                    etc. style for numbered books -- kjv.json has no osisName equivalent)
fields.bookOrder    int, 1-66 canonical order
fields.testament    "Old Testament" | "New Testament"
fields.chapters[]    -> chapters.json record ids
fields.writers[]      -> people.json record ids (traditional authorship)
fields.placeWritten[] -> places.json record ids
fields.yearWritten    BROKEN/unusable, see note above -- ignore
```

### `verses.json` — full KJV text, keyed by record id, cross-linked

```
fields.osisRef      e.g. "Gen.1.1" -- matches xrefs TSV format and events/places refs
fields.verseID      zero-padded sortable string, e.g. "01001001" (BB CCC VVV)
fields.verseText    plain KJV text (unformatted)
fields.yearNum      int, year from Torrey's Treasury of Scripture Knowledge --
                    docs explicitly warn: "Not aligned with the events table" (i.e.
                    don't assume verses.yearNum agrees with events.startDate for the
                    same passage)
fields.book[]        -> single books.json record id
fields.chapter[]      -> single chapters.json record id
fields.people[]        -> people.json record ids mentioned in this verse
fields.places[]        -> places.json record ids mentioned in this verse
fields.event[]         -> events.json record ids
```

---

## `xrefs/cross_references.txt` (extracted from cross-references.zip)

Tab-separated. **344,800 total lines including header.** Header line has a trap:

```
From Verse<TAB>To Verse<TAB>Votes<TAB>#www.openbible.info CC-BY 2026-08-17
```

The header row has **4** tab-separated fields, but every data row has only **3**
(`From Verse`, `To Verse`, `Votes`) — the header's 4th field is an attribution
comment glued on with a tab, not a real column. A parser must special-case line 1
(skip it) rather than infer column count from the header.

Real data rows:
```
Gen.1.1	Ps.124.8	71
Gen.1.1	Col.1.16-Col.1.17	171
Rev.22.21	Rom.16.23	-2
```

Notes:
- Refs use the same OSIS-style `Book.Chapter.Verse` scheme as Theographic's `osisRef`
  (e.g. `Gen.1.1`), so book abbreviations should cross-reference directly.
- Refs can be **ranges within one field**: `Col.1.16-Col.1.17` means "Col 1:16 through
  Col 1:17" as a single cross-reference target — the parser must detect and split on
  the internal `-` (careful: this is unrelated to any minus sign in a year; xrefs
  don't have years, only book/chapter/verse).
- **Votes can be negative** (e.g. `-2`, seen in the real tail of the file) — net
  downvoted cross-references exist and are not filtered out upstream. atlas-etl
  should decide whether to drop non-positive-vote rows (spec doesn't mandate this;
  noting the fact here since a naive "votes are always positive" assumption is false).

---

## Borders — `aourednik/historical-basemaps` (historical border snapshots)

Fetched by the "Historical border snapshots" block in `fetch-raw.ps1`, added
2026-08-19 for the time-period-accurate borders feature. 12 GeoJSON files
into `data/raw/borders/`, verified against the repo's real GitHub tree
(`api.github.com/repos/aourednik/historical-basemaps/git/trees/master?recursive=1`)
before picking them — the repo ships 53 `geojson/world_*.geojson` snapshots
total, spanning `world_bc123000.geojson` (123,000 BC) through
`world_2010.geojson` (AD 2010); filenames are `world_bc{N}.geojson` (year
`-N`) or `world_{N}.geojson` (year `N`, always positive in this repo — there
is no `world_0.geojson`).

**Chosen 12** (covering/nearest atlas-etl's `[-4004,100]` span; the earliest,
`bc4000`, is 4 years short of the exact span start but is the nearest
available snapshot, matching the brief's "the earliest available BC snapshot
covers the deep past"; `world_100` is an exact match for the span's end):

```
world_bc4000.geojson  (-4000)   world_bc500.geojson (-500)
world_bc3000.geojson  (-3000)   world_bc323.geojson (-323, Alexander's death)
world_bc2000.geojson  (-2000)   world_bc200.geojson (-200)
world_bc1500.geojson  (-1500)   world_bc100.geojson (-100)
world_bc1000.geojson  (-1000)   world_bc1.geojson   (-1, 1 BC)
world_bc700.geojson   (-700)    world_100.geojson   (100, AD 100)
```

Sizes range 786,563–1,185,423 bytes each (~10.7 MB total). Verified real
snapshot content includes biblically-relevant regions by name at multiple
years, e.g. `world_bc1000.geojson` contains a feature named `"Kingdom of
David and Solomon"`; `world_bc1.geojson`/`world_100.geojson` contain
`"Judea"`; `world_bc323.geojson` contains `"Empire of Alexander"`.

### License — GNU GPL v3.0 (verified against the repo's actual `LICENSE` file)

`https://github.com/aourednik/historical-basemaps` ships a full `LICENSE`
file containing the standard GPL-3.0 text (verified by fetching it directly,
not inferred from a repo topic tag). GPL-3.0's copyleft/source-availability
obligations (providing a corresponding-source offer, licensing derivative
works under GPL-3.0, etc.) are triggered by **conveying/distributing** the
work to others — the license places no restriction on private use, local
modification, or study. Bible Atlas is (per the project's own stack
decision) a **local, personal, deploy-ready sketch app**, not a distributed
product — this use (fetch once, clip/simplify at build time via atlas-etl,
serve locally from `atlas-server`) stays entirely within GPL-3.0's
unrestricted private-use terms. **Determination: usable, not blocked.**
Required attribution is carried in two places: this README (source +
license) and the app's own `attribution` UI element per the header
CONTRACT — if this project is ever distributed/published beyond local
personal use, the borders feature would need either GPL-3.0 compliance
(offering corresponding source, matching license on redistribution) or the
dataset swapped for a differently-licensed one; out of scope for M1.

**Required attribution**: "Historical border data from [aourednik/historical-basemaps](https://github.com/aourednik/historical-basemaps), GNU GPL v3.0."

### GeoJSON structure (verified against real fetched files, not the repo's prose docs)

Each file is a `FeatureCollection` of `Feature`s; every geometry observed
across all 12 chosen files (and a handful of others spot-checked) is
`MultiPolygon` — atlas-etl's parser also accepts a bare `Polygon` per the
GeoJSON spec (normalizing it to a one-polygon MultiPolygon internally) since
nothing guarantees every one of the repo's 53 files stays MultiPolygon-only,
but none of the 12 chosen files exercise that branch. Coordinates are
`[lon, lat]` pairs (standard GeoJSON order), full `f64` precision (~15
significant digits — far more than this app's world-scale display needs;
atlas-etl rounds to 4 decimal places, ~11m, when writing the compiled
output).

Feature `properties` (the repo's own README documents `NAME`/`SUBJECTO`/
`PARTOF`/`BORDERPRECISION`; verified present on real records, all
UPPERCASE):

```
NAME              country/region name, e.g. "Roman Empire", "Kingdom of David and Solomon";
                  NULL on many features (unnamed/unclassified regions in the
                  source dataset -- real, not a parsing bug; e.g. feature[0]
                  of world_bc323.geojson has NAME: null). atlas-etl keeps
                  these features (geometry is still valid map content) --
                  only the CLIENT's landmark labels need real names, and
                  those come from the separately-curated landmarks.toml, not
                  from border feature names.
ABBREVN           short name; also frequently null; not used by atlas-etl.
CONTROL           null on every record checked; not used by atlas-etl.
SUBJECTO          colonial power / region name per the repo's README; not used by atlas-etl.
BORDERPRECISION   ordinal 1 (approximate) .. 3 (legally determined); always
                  present as an integer (1 on every record checked in the
                  biblical-world bbox); not used by atlas-etl (M1 renders
                  every border the same stroke style regardless).
PARTOF            parent cultural area name if applicable; not used by atlas-etl.
```

Two of the four older/deep-past files spot-checked (`world_bc4000.geojson`)
additionally carry `type` and `weblnks` fields not present on the
later/closer-to-span files (`world_bc323.geojson` etc.) — the schema is not
perfectly uniform across the repo's 53-file history, confirming the
hand-parse-via-`serde_json::Value` approach (no fixed properties struct) was
the right call: atlas-etl's `borders::parse` reads `properties` as an
untyped JSON object and passes it through to the compiled output unchanged,
so it tolerates whatever fields any given snapshot happens to carry.

Biblical-world bbox overlap check (lat 7.6..48.9, lon -10.9..71.4, same
constants `client/wwwroot/js/map.js`'s `BIBLICAL_WORLD_BOUNDS` uses):
every one of the 12 chosen files has 18–37 features overlapping that box out
of 136–442 total features in the file (the biblical world sits in the
Africa-Eurasia landmass all 53 snapshots cover, so this is true across the
whole dataset, not just the chosen 12) — comfortably nonzero, so
atlas-etl's "zero features after clipping = fatal" check is a real
bbox-bug guard, not a check expected to ever legitimately fire on this data.

## Vendored Leaflet (`client/wwwroot/vendor/leaflet/`)

`leaflet.js` (147,552 bytes) and `leaflet.css` (14,806 bytes), fetched from
`unpkg.com/leaflet@1.9.4/dist/`. Verified real Leaflet 1.9.4 content (checked the
`@preserve` banner comment in `leaflet.js`). This directory is gitignored
(`client/wwwroot/vendor/`).
