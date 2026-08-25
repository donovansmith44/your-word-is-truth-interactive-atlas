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

## Borders — no longer a `data/raw/` input (Batch L, license remediation)

Historical border snapshots used to be fetched here from
`aourednik/historical-basemaps` (GPL-3.0). Batch L removed that dataset
entirely (the download lines in `fetch-raw.ps1`, this section, and its
attribution in the client) and replaced it with this project's own coarse,
hand-authored, CC0 border snapshots at `data/curated/borders/
{signed_year}.geojson` — curated data, not raw-fetched, so it belongs with
the rest of `data/curated/` (see that directory and `LICENSES.md`, not this
file, for the current borders format and license). `server/atlas-etl/src/
borders.rs` still runs the same clip/simplify pipeline described in its own
module doc comment; only the input source and filename convention changed.

## `catechism-mapping/catechism-{sha}/` (Batch F2 -- the user's own catechism verse mapping)

Extracted GitHub commit-archive zip of `brain-fuel/catechism`, pinned at
commit `0be24fee92e6333f817c4c2a08f99cf7c5274295` (see `fetch-raw.ps1` and
`LICENSES.md`). `resources/*.yaml` (44 files): one file per catechism topic,
each a top-level YAML mapping of question-number -> `{ title: string, refs:
!!set of "Book Chapter:Verse" }` strings -- see
`server/atlas-etl/src/catechism_map.rs`'s own module doc comment for the
full, verified catalog of human ref-string shapes found across every file
(single verse, same-chapter range, bare chapter, bare chapter range,
cross-chapter range, comma-compound) and `data/curated/catechism-mapping.toml`
for exactly which files are ingested vs. deliberately deferred (7 files +
the `svebilius/` directory, a Finnish/English Svebilius Catechism
explanation text, fetched but out of scope this batch).

## `brain-fuel-bible/` (Batch CORP-1a -- the owner's own named source for parallel-edition text)

Selectively vendored from `github.com/brain-fuel/bible`, pinned at commit
`94d44842cb242e8aa840330748e03d2803f2a7c1` (see `data/fetch-raw.ps1` and
`LICENSES.md`). The real, upstream repo is large (~29k files: apocrypha/
Septuagint texts, per-word morphology, a lexicon, a semantic relation
graph, Go/Python tooling) -- only what this app's parser
(`server/atlas-etl/src/brainfuel.rs`) actually reads is vendored:
`data/books.json` (the book-code/`kjv_name` manifest) and every
`bible/ot/*/*.json` + `bible/nt/*/*.json` chapter file (929 + 260 = 1,189
files) -- `bible/apo/`, `bible/lxx/`, and everything else in the upstream
repo is NOT vendored (owner ruling: "no apocrypha for now"; the LXX/
morphology/lexicon/relation-graph material is out of this batch's scope
entirely).

**Layout** (unchanged from upstream): `{ot,nt}/{CODE}/{NNN}.json`, `CODE`
brain-fuel's OWN three-letter book code (verified DIFFERENT from this
app's own `atlas_core::canon::BOOKS` codes in several places -- e.g. `JOH`
not `JHN`, `MAR` not `MRK`, `SOS` not `SNG`, `JDE` not `JUD` -- resolved via
`data/books.json`'s own `kjv_name` field instead, which uses the IDENTICAL
old-style KJV naming convention `data/raw/kjv.json` already does: "I
Samuel", "Revelation of John", etc.; one shared normalizer,
`kjv::normalize_book_name`, reused rather than a second hand-maintained
code table).

**Verse schema** (verified against the real files, not the upstream
README alone -- see `server/atlas-etl/src/brainfuel.rs`'s own module doc
comment for the fully swept catalog): one JSON object per chapter,
`verses: [{ verse, <edition-field>: "text", ..., refs: { <edition-id>: {
src?, absent? } } }]`. `verse` is ALREADY the KJV skeleton position
(pre-aligned by the upstream repo itself). An edition's own field key is
present on EVERY verse of a chapter file or on NONE of them (a
testament-level fact -- confirmed: `hebrew_masoretic`/`douay_rheims` never
appear as keys anywhere in `nt/`; `greek_textus_receptus` never appears
anywhere in `ot/`). A LOUD GOTCHA, verified directly (`bible/ot/1CH/011.json`
verse 47, among 15 real OT + 3 real NT examples found): when
`refs.<edition>.absent` is `true`, that edition's own text KEY IS STILL
PRESENT, holding an EMPTY STRING (`""`) -- never simply omitted; this
app's parser reads `absent` FIRST and imports NO rendering at all in that
case (never the empty string). `refs.<edition>.src` is versification
PROVENANCE only (e.g. Hebrew/Latin Psalm-title numbering, whose own local
"chapter:verse" differs from the KJV position the text is already
correctly placed at) -- disclosed (counted) by this app's parser, never
stored or acted on (CORP-2's own future scope).

**Six editions ingested** (KJV is already this app's own canonical layer,
ingested since M-A): `latin_vulgate` (Clementine Vulgate), `hebrew_masoretic`
(Westminster Leningrad Codex, OT only), `douay_rheims` (Douay-Rheims/
Challoner -- OT only IN THIS DATASET, verified against both
`data/editions.json`'s own manifest row and the real files: brain-fuel's
own NT chapter files never carry a `douay_rheims` key at all), `finnish_biblia`
(Biblia 1776), `swedish_karl_xii` (Karl XII:s Bibel, 1703 -- merged into
the SAME chapter JSONs by the upstream repo's own separate Go tool, per
its own `data/editions.json` note; reads identically to every other
edition from this app's own parser), `greek_textus_receptus` (Greek
Textus Receptus, NT only). `king_james_apocrypha` (KJVA) is SKIPPED
outright (ruled on, not silently dropped): its 66-book canonical coverage
would exactly duplicate this app's own KJV base, and its only unique
content is the apocryphal books this batch already excludes.

**Verified real counts** (full programmatic sweep of all 31,102 aligned
verse positions, cross-checked independently in both Python and this
app's own Rust parser -- see `server/atlas-etl/tests/brainfuel_real_data.rs`):
929 OT + 260 NT chapter files; per-edition imported verse counts
latin_vulgate 31,092, hebrew_masoretic 23,145, douay_rheims 23,132,
finnish_biblia 31,102 (zero absences -- identity-placed), swedish_karl_xii
31,099, greek_textus_receptus 7,957; absent-marker counts latin_vulgate 10
(ten OT verses merged into the preceding verse in the Vulgate tradition,
matching the upstream repo's own README), douay_rheims 13, swedish_karl_xii
3; zero anomalies (an edition ever empty-and-unmarked) anywhere in the
real data.

## `concord/*.html` (Batch CORP-2a — the Book of Concord, 1921 Bente-Dau)

Ten document-root pages fetched from `bookofconcord.org` (see
`data/fetch-raw.ps1`'s own `$concordDocs` list and `LICENSES.md`'s "The
Book of Concord" section for the full provenance/PD reasoning):
`preface.html`, `ecumenical-creeds.html`, `augsburg-confession.html`,
`defense.html` (= the Apology), `smalcald-articles.html`,
`power-and-primacy.html` (= the Treatise), `small-catechism.html`,
`large-catechism.html`, `epitome.html`, `solid-declaration.html`.

**Each document's ROOT page already carries its own FULL text inline** —
verified before writing the fetch script: `/augsburg-confession/`'s own
HTML contains every one of its 28 articles' complete paragraph text, not
just a table-of-contents linking out to 28 separate per-article pages
(those per-article pages exist too, e.g. `/augsburg-confession/
of-justification/`, and carry byte-identical paragraph text — confirmed
by direct comparison — so fetching them as well would be pure
redundancy). This is why only 10 files are vendored here, not the ~150
the document/article grammar might suggest.

**HTML shape** (`server/atlas-etl/src/concord.rs`'s own module doc
comment has the full parser-facing grammar): each article is
`<a href="/{doc}/{slug}/"><h3>TITLE</h3></a>` immediately followed by
`<section>...paragraphs...</section>`. Paragraph boundaries are
`<span id="{prefix}-acontent" class="{prefix2}-content">LABEL</span>`
markers, `LABEL` being the source's own visible paragraph number (plain
digits for every document except the Small Catechism, whose own
Question/Answer format sub-letters a shared base number — "1", "1b",
"1c" — or, for its Introduction/Conclusion units only, uses a literal
"*" with an explicit `-ans`-suffixed id pairing instead of a digit).
`smalcald-articles.html`'s own markup has one confirmed template quirk
(harmless): its `{prefix}`/`{prefix2}` template variables render EMPTY
(`id="0001" class="-content"` rather than `id="ac-iv-0001-acontent"
class="bocanchor-content"`-style ids elsewhere) — the marker STILL
matches the same `-acontent"..."-content"` grammar, so parsing is
unaffected. A handful of Small-Catechism sub-lettered marker ids are
ALSO Go template-bug artifacts (literal text like
`id="sc-lords-prayer-%!d(string=001b)-acontent"`, a `fmt.Sprintf("%d",
...)` misfire on the SITE'S OWN end) — cosmetic (affects only an
internal deep-link anchor id never read by this project's parser, never
the visible paragraph label or the paragraph's own prose).

The Three Ecumenical Creeds (`ecumenical-creeds.html`) carry NO inline
paragraph-number markers at all in the source (plain `<p>` prose only) —
disclosed structure resistance to the `ConcordRef` triple, per the batch
brief; see `batch-corp2a-report.md` for the full per-document structure
disclosures and paragraph-count tally.

## Vendored Leaflet (`client/wwwroot/vendor/leaflet/`)

`leaflet.js` (147,552 bytes) and `leaflet.css` (14,806 bytes), fetched from
`unpkg.com/leaflet@1.9.4/dist/`. Verified real Leaflet 1.9.4 content (checked the
`@preserve` banner comment in `leaflet.js`). This directory is gitignored
(`client/wwwroot/vendor/`).
