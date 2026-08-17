# Bible Atlas M1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the full M1 Bible Atlas: KJV reader + `/world` time-slider map with glowing places, narrative arrows, scripture override, mini-maps, cross-reference exploration — with an exhaustive, property-based, language-agnostic UX test suite.

**Architecture:** Rust (axum) backend loads ETL-compiled open datasets into memory and owns all query semantics, serving composed "scenes" per time-window or scripture ref. A C#/Blazor WebAssembly client renders reader and map UIs, driving Leaflet through one JS interop module. A black-box `tests/ux/` suite (TypeScript + Playwright + fast-check) tests only HTTP and DOM against a documented testid/URL contract.

**Tech Stack:** Rust stable 1.97 (windows-gnu host), axum 0.8, serde, proptest · .NET 10 Blazor WASM, xunit · Leaflet 1.9.4 (vendored) · Node 24, @playwright/test, fast-check 4, TypeScript 5.

**Spec:** `docs/superpowers/specs/2026-08-17-bible-atlas-design.md` — read it first; this plan argues from it.

## Global Constraints

- **Toolchain paths (this machine):** shells opened by the harness predate a PATH update. Start EVERY shell session with:
  `$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet;$env:USERPROFILE\.cargo\bin"; $env:DOTNET_ROOT = "$env:LOCALAPPDATA\Microsoft\dotnet"`
- **Toolchain:** Rust is `x86_64-pc-windows-gnu` — no C compiler exists on this machine. Never add a crate that needs `cc`/`cmake`/`nasm` (no `ring`, no `openssl-sys`, no `aws-lc`). ETL does no networking; `data/fetch-raw.ps1` does all downloads.
- **Ports:** API `http://localhost:8000`, Blazor dev server `http://localhost:5000`. `/health` on the API returns 200 `ok`.
- **Wire format:** JSON, `snake_case` keys. Rust structs use snake_case fields with plain serde derive; C# uses `JsonNamingPolicy.SnakeCaseLower` globally. The shared golden fixture `tests/fixtures/golden-scene.json` is the cross-language pin — both sides test against it.
- **Years:** signed `i32`, `-n` = n BC, `n` = AD n, **zero forbidden**; `-1` and `1` are adjacent (`next_year(-1) == 1`). Span: `-4004..=100`. Display `1447 BC` / `AD 30`; ranges `1447 BC – 1400 BC` (spaced en dash, U+2013).
- **Scene rules:** `brightness = min(activating_event_count, 5)`. Verse groups cap at 20 verse ids + total `count`. Scene arrows per spec §7.4.
- **Canonical scripture refs:** `GEN` | `GEN.1` | `GEN.1.1` | `GEN.1.1-5` (USFM 3-letter codes, uppercase).
- **UX contract:** `tests/ux/CONTRACT.md` (authored in Task 8) is the single authority for `data-testid`s, URL patterns, and displayed-text formats. UI tasks MUST emit exactly those testids; UX tests MUST select only via that contract. The suite never imports from `server/` or `client/`.
- **Property IDs** (SCENE-n, ARROW-n, …) are stable; each maps to a spec section in the Property Catalog below. Run counts: env `FC_NUM_RUNS` (default 150 for API properties, 20 for browser properties). Finite domains (66 books, 10 eras, era boundaries) are enumerated exhaustively, never sampled.
- **Commits:** every task ends in a commit; append `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>` to each message. TDD throughout: failing test → implement → pass → commit.
- **ETL validation:** hard-fail (non-zero exit) on: unknown place id in events/narratives, dangling narrative leg, verse id outside canon, era gaps/overlaps/zero-year, duplicate event ids, non-chronological narrative legs. `books.toml.write_place` that doesn't resolve is a WARN + field dropped (spec refinement: metadata, not geometry).
- **Blazor:** standalone WASM (`Microsoft.AspNetCore.Components.WebAssembly`), no server-side prerender. API base read from `wwwroot/appsettings.Development.json` key `ApiBase` (`http://localhost:8000` in dev, `""` in production = same origin). axum sets `CorsLayer::permissive()`.

---

## UX Property Catalog (traceability: property ↔ spec)

| ID | Property (informal) | Spec | Layer |
|---|---|---|---|
| SCENE-1 | any valid window → 200, `mode:"time"`, window echoed, `from<=to` | §5 | API |
| SCENE-2 | every scene place has ≥1 event; every event's `when` intersects the window; `brightness == min(count,5)`; every verse group ≤20 ids and `count >= len(verses)` | §4.4, §5 | API |
| SCENE-3 | window monotonicity: `w1 ⊆ w2` ⇒ place-id set of `w1` ⊆ that of `w2`, and per shared place, event-id set ⊆ | §4.4 | API |
| SCENE-4 | `?from=y&to=y` scene equals the same call again (determinism) and every event intersects `[y,y]` | §4.4 | API |
| SCENE-5 | zero/invalid years and `from>to` → 4xx typed error `{error:{code,message}}` | §5, §9 | API |
| ARROW-1 | every arrow's `from_place`/`to_place` are ids of lit scene places | §7.4 | API |
| ARROW-2 | arrow colors are functional per narrative and equal both the scene legend color and `/api/narratives` color | §7.4 | API |
| ARROW-3 | per narrative, arrows sorted by `order`: `to_place[k] == from_place[k+1]` (place-chain) | §7.4 | API |
| ARROW-4 | per narrative: first arrow's `from_event` appears in no arrow's `to_event`; last arrow's `to_event` in no `from_event` (pure source / pure sink) | §7.4 | API |
| ARROW-5 | no self-arrows: `from_place != to_place` | §7.4 | API |
| ARROW-6 | chronology: `to_event.when.from_year >= from_event.when.from_year` | §7.4 | API |
| ARROW-7 | `from_event`/`to_event` exist in the scene's respective places' event lists | §5 | API |
| SCRIP-1 | scripture scene: `mode:"scripture"`, ref echoed; every place has ≥1 verse inside the requested ref | §4.4 | API |
| SCRIP-2 | ref monotonicity: verse ⊆ passage ⊆ chapter ⊆ book place-id sets | §4.4 | API |
| SCRIP-3 | scripture arrows: both endpoint events have ≥1 verse in the ref | §4.4 | API |
| XREF-1 | verse detail: 200 for every canon verse sampled; `cross_refs` votes descending; every target parses as a canon ref/span; no self-target; non-empty `preview` | §5, §8 | API |
| CHAP-1 | chapter verse count == TOC count; verse numbers contiguous 1..n; every verse non-empty text | §5 | API |
| ERA-1 | eras ascending, contiguous under `next_year`, cover exactly `[-4004,100]`, no zero year (exhaustive, not sampled) | §4.5, §7.1 | API |
| BOOKS-1 | all 66 books in TOC order Genesis→Revelation; codes unique; chapter counts ≥1 (exhaustive) | §5 | API |
| BOOKS-2 | scripture-book scene succeeds for EVERY book (exhaustive 66) | §4.4 | API |
| WORLD-1 | for a generated window via URL: rendered `marker-*` testid set == API scene place ids | §7.2, §7.3 | UI |
| WORLD-2 | hover a random marker → `place-card` shows the place name and per-book/chapter verse-group counts matching the API scene | §7.3 | UI |
| WORLD-3 | rendered arrow paths (testid `arrow-{narrative}-{order}`) == API scene arrows; each path `stroke` == narrative color; each has an arrowhead marker-end | §7.3, §7.4 | UI |
| WORLD-4 | legend isolate: clicking narrative A sets `data-faded="true"` on exactly the non-A arrows; second click clears all fading | §7.3 | UI |
| WORLD-5 | typing a year/range into the editable readout re-renders markers to equal the API scene for that window, and the URL query reflects it | §7.1 | UI |
| WORLD-6 | picking a random book/chapter enters scripture mode: chip shows the ref, slider `aria-disabled`, markers == scripture-scene; "return to time" restores the exact prior window (URL + markers) | §7.2 | UI |
| WORLD-7 | era segments: every era name from `/api/eras` appears on the slider; clicking one selects exactly that era's range in the readout (exhaustive over eras) | §7.1 | UI |
| WORLD-8 | clicking a place card's title opens a popover titled with the place name, listing its events across time | §6.1, §7.3 | UI |
| READ-1 | random chapter: rendered `verse-line-*` count == TOC verse count; numbers 1..n in order | §8 | UI |
| READ-2 | open a random verse's popover: title is the canonical ref; body text == API verse text | §8 | UI |
| READ-3 | cross-ref chain of generated depth ≤3: list order matches API; each hop pushes breadcrumb; walking back restores each prior title exactly | §8 | UI |
| READ-4 | "Explore geo-temporally" shows a mini-map whose markers == scripture scene of the verse; "Open in /world" lands on `/world?ref=<verse>` with the same marker set | §8 | UI |
| READ-5 | shift-click passage selection over a random verse range shows chip `BOOK.C.V1-V2`; its popover mini-map == passage scene | §8 | UI |
| NAV-1 | deep links: `/read/{book}/{ch}` renders that chapter; `/world?from&to` and `/world?ref` restore their scenes after full page reload | §7.2, §8 | UI |

"Exhaustive" means: every row above is implemented; rows marked exhaustive enumerate their whole domain; the rest generate with shrinking and a deep-run knob (`FC_NUM_RUNS`).

---

### Task 1: Repo scaffolding, workspaces, fetch script

**Files:**
- Create: `server/Cargo.toml` (workspace), `server/atlas-core/Cargo.toml`, `server/atlas-core/src/lib.rs` (stub), `server/atlas-etl/Cargo.toml`, `server/atlas-etl/src/main.rs` (stub), `server/atlas-server/Cargo.toml`, `server/atlas-server/src/main.rs` (stub)
- Create: `client/` via `dotnet new blazorwasm`, `client/wwwroot/appsettings.Development.json`
- Create: `data/fetch-raw.ps1`, `data/raw/README.md`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: compiling empty workspace; `client/BibleAtlas.Client.csproj`; raw datasets on disk under `data/raw/`; vendored `client/wwwroot/vendor/leaflet/leaflet.{js,css}`.

- [ ] **Step 1: Rust workspace skeleton**

`server/Cargo.toml`:

```toml
[workspace]
members = ["atlas-core", "atlas-etl", "atlas-server"]
resolver = "2"
```

`server/atlas-core/Cargo.toml`:

```toml
[package]
name = "atlas-core"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
proptest = "1"
```

`server/atlas-etl/Cargo.toml`:

```toml
[package]
name = "atlas-etl"
version = "0.1.0"
edition = "2021"

[dependencies]
atlas-core = { path = "../atlas-core" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
csv = "1"
toml = "0.8"
anyhow = "1"
```

`server/atlas-server/Cargo.toml`:

```toml
[package]
name = "atlas-server"
version = "0.1.0"
edition = "2021"

[dependencies]
atlas-core = { path = "../atlas-core" }
axum = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
tower-http = { version = "0.6", features = ["cors", "fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

Stub `lib.rs`/`main.rs` files: `pub fn placeholder() {}` / `fn main() {}`.

- [ ] **Step 2: Verify workspace compiles**

Run (from `server/`): `cargo build`
Expected: success, three crates compile.

- [ ] **Step 3: Blazor app**

Run (from repo root): `dotnet new blazorwasm --empty -o client -n BibleAtlas.Client`
(If this SDK's template has no `--empty` flag, create without it and delete the demo pages/css it scaffolds — keep only `Program.cs`, `App.razor`, `_Imports.razor`, `wwwroot/index.html`.)
Then create `client/wwwroot/appsettings.Development.json`:

```json
{ "ApiBase": "http://localhost:8000" }
```

Edit `client/Properties/launchSettings.json`: set the http profile's `applicationUrl` to `http://localhost:5000`.
Run: `dotnet build client` → success.

- [ ] **Step 4: Fetch script**

`data/fetch-raw.ps1` (idempotent — skips files that exist):

```powershell
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$raw = Join-Path $PSScriptRoot 'raw'
New-Item -ItemType Directory -Force $raw | Out-Null

function Fetch($url, $out) {
  $path = Join-Path $raw $out
  if (Test-Path $path) { Write-Output "have $out"; return }
  Write-Output "fetch $url"
  Invoke-WebRequest -Uri $url -OutFile $path -UseBasicParsing
}

# KJV text (single-file JSON; fallback repo noted in raw/README.md)
Fetch 'https://raw.githubusercontent.com/scrollmapper/bible_databases/master/formats/json/kjv.json' 'kjv.json'
# OpenBible geocoding bundle
Fetch 'https://a.openbible.info/geo/data.zip' 'openbible-geo.zip'
if (-not (Test-Path (Join-Path $raw 'geo'))) { Expand-Archive (Join-Path $raw 'openbible-geo.zip') (Join-Path $raw 'geo') }
# Theographic metadata (whole repo)
Fetch 'https://github.com/robertrouse/theographic-bible-metadata/archive/refs/heads/master.zip' 'theographic.zip'
if (-not (Test-Path (Join-Path $raw 'theographic'))) { Expand-Archive (Join-Path $raw 'theographic.zip') (Join-Path $raw 'theographic') }
# Cross references (TSV with votes)
Fetch 'https://a.openbible.info/data/cross-references.zip' 'cross-references.zip'
if (-not (Test-Path (Join-Path $raw 'xrefs'))) { Expand-Archive (Join-Path $raw 'cross-references.zip') (Join-Path $raw 'xrefs') }
# Vendor Leaflet 1.9.4 into the client (deterministic, offline-friendly)
$vendor = Join-Path $PSScriptRoot '..\client\wwwroot\vendor\leaflet'
New-Item -ItemType Directory -Force $vendor | Out-Null
foreach ($f in 'leaflet.js','leaflet.css') {
  $p = Join-Path $vendor $f
  if (-not (Test-Path $p)) { Invoke-WebRequest "https://unpkg.com/leaflet@1.9.4/dist/$f" -OutFile $p -UseBasicParsing }
}
Write-Output 'fetch-raw complete'
```

- [ ] **Step 5: Run the fetch script; document actual formats**

Run: `powershell -NoProfile -ExecutionPolicy Bypass -File data/fetch-raw.ps1`
Expected: all five artifacts present. **If any URL 404s** (upstream moved): find the dataset's current URL on the project's site (openbible.info, the scrollmapper and robertrouse GitHub repos), update the script, and note the change in `data/raw/README.md`. Then open each downloaded artifact, and write `data/raw/README.md` describing the ACTUAL file names/columns found (e.g., which file inside `geo/` holds place→lat/lon→verses; the Theographic `json/*.json` field names; the cross-ref TSV header). Task 4's parsers are written against what you record here — the compiled schema, not the raw one, is the contract (spec §4.3).

- [ ] **Step 6: gitignore + commit**

Append to `.gitignore`: `node_modules/`, `tests/ux/test-results/`, `tests/ux/playwright-report/`, `client/wwwroot/vendor/`.

```powershell
git add -A; git commit -m "feat: scaffold rust workspace, blazor client, data fetch script"
```

---

### Task 2: atlas-core — years, refs, canon, wire types, golden fixture

**Files:**
- Create: `server/atlas-core/src/time.rs`, `src/refs.rs`, `src/canon.rs`, `src/wire.rs`; replace `src/lib.rs`
- Test: inline `#[cfg(test)]` modules + `server/atlas-core/tests/golden.rs`
- Create: `tests/fixtures/golden-scene.json` (generated by the golden test, committed)

**Interfaces:**
- Consumes: nothing.
- Produces (used by every later Rust/C# task):
  - `Year = i32`; `next_year(y: Year) -> Year`; `TimeRange { from_year, to_year }` with `TimeRange::new(from,to) -> Result<TimeRange, CoreError>`, `intersects(&self, &TimeRange) -> bool`, `contains_year(&self, Year) -> bool`
  - `BookId(u8)`; `canon::BOOKS: [BookInfo; 66]` where `BookInfo { code: &'static str, osis: &'static str, name: &'static str }`; `canon::resolve_alias(&str) -> Option<BookId>`; `BookId::code(&self) -> &'static str`
  - `VerseId { book: BookId, chapter: u16, verse: u16 }` (serde as `"GEN.1.1"`), `ScriptureRef` enum `{ Book(BookId), Chapter { book, chapter }, Passage { book, chapter, from_verse, to_verse }, Verse(VerseId) }` with `parse(&str) -> Result<ScriptureRef, CoreError>` and `Display` producing canonical form
  - wire structs (all `Serialize + Deserialize + Clone + PartialEq + Debug`): `Scene { mode: String, window: Option<TimeRange>, sref: Option<String>, places: Vec<ScenePlace>, arrows: Vec<SceneArrow>, narratives: Vec<SceneNarrative> }` (field `sref` serialized as `"ref"` via `#[serde(rename = "ref")]`), `ScenePlace { id, name, lat, lon, brightness, events: Vec<SceneEvent> }`, `SceneEvent { id, label, when: TimeRange, verse_groups: Vec<VerseGroup> }`, `VerseGroup { book: String, chapter: u16, verses: Vec<String>, count: u32 }`, `SceneArrow { narrative, color, from_place, to_place, from_event, to_event, order: u32 }`, `SceneNarrative { id, name, color, legs_in_scene: u32 }`

- [ ] **Step 1: Write failing tests for time + refs**

`src/time.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn zero_year_rejected() {
        assert!(TimeRange::new(0, 5).is_err());
        assert!(TimeRange::new(-5, 0).is_err());
        assert!(TimeRange::new(5, -5).is_err()); // from > to
    }
    #[test]
    fn bc_ad_adjacency() {
        assert_eq!(next_year(-1), 1);
        assert_eq!(next_year(-2), -1);
        assert_eq!(next_year(1), 2);
    }
    #[test]
    fn intersect_examples() {
        let a = TimeRange::new(-1450, -1400).unwrap();
        assert!(a.intersects(&TimeRange::new(-1400, -1300).unwrap())); // touching
        assert!(!a.intersects(&TimeRange::new(-1399, -1300).unwrap()));
    }
    proptest! {
        #[test]
        fn intersects_symmetric(a in range_strategy(), b in range_strategy()) {
            prop_assert_eq!(a.intersects(&b), b.intersects(&a));
        }
        #[test]
        fn contains_implies_intersects(a in range_strategy(), b in range_strategy()) {
            if b.from_year >= a.from_year && b.to_year <= a.to_year {
                prop_assert!(a.intersects(&b));
            }
        }
    }
    fn range_strategy() -> impl Strategy<Value = TimeRange> {
        (-4004i32..=100, -4004i32..=100)
            .prop_filter("no zero", |(a, b)| *a != 0 && *b != 0)
            .prop_map(|(a, b)| TimeRange::new(a.min(b), a.max(b)).unwrap())
    }
}
```

`src/refs.rs` test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn parse_forms() {
        assert_eq!(ScriptureRef::parse("GEN").unwrap().to_string(), "GEN");
        assert_eq!(ScriptureRef::parse("EXO.14").unwrap().to_string(), "EXO.14");
        assert_eq!(ScriptureRef::parse("EXO.14.21").unwrap().to_string(), "EXO.14.21");
        assert_eq!(ScriptureRef::parse("EXO.14.21-31").unwrap().to_string(), "EXO.14.21-31");
        assert!(ScriptureRef::parse("NOPE.1").is_err());
        assert!(ScriptureRef::parse("GEN.0.1").is_err());
        assert!(ScriptureRef::parse("EXO.14.31-21").is_err()); // inverted range
    }
    #[test]
    fn aliases_resolve() {
        for (alias, code) in [("Genesis","GEN"),("Gen","GEN"),("1Sam","1SA"),("1Kgs","1KI"),
                              ("Song","SNG"),("Ps","PSA"),("Phlm","PHM"),("Rev","REV")] {
            assert_eq!(crate::canon::resolve_alias(alias).unwrap().code(), code);
        }
    }
    proptest! {
        #[test]
        fn display_parse_roundtrip(book in 0u8..66, ch in 1u16..150, v in 1u16..176, v2 in 0u16..176) {
            let b = BookId(book);
            let refs = vec![
                ScriptureRef::Book(b),
                ScriptureRef::Chapter { book: b, chapter: ch },
                ScriptureRef::Verse(VerseId { book: b, chapter: ch, verse: v }),
            ];
            for r in refs {
                prop_assert_eq!(ScriptureRef::parse(&r.to_string()).unwrap(), r);
            }
            if v2 > v {
                let p = ScriptureRef::Passage { book: b, chapter: ch, from_verse: v, to_verse: v2 };
                prop_assert_eq!(ScriptureRef::parse(&p.to_string()).unwrap(), p);
            }
        }
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run (from `server/`): `cargo test -p atlas-core`
Expected: compile errors (types not defined) — that is the failing state.

- [ ] **Step 3: Implement time, canon, refs**

`src/time.rs`:

```rust
use serde::{Deserialize, Serialize};

pub type Year = i32;

pub fn next_year(y: Year) -> Year { if y == -1 { 1 } else { y + 1 } }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange { pub from_year: Year, pub to_year: Year }

impl TimeRange {
    pub fn new(from_year: Year, to_year: Year) -> Result<Self, crate::CoreError> {
        if from_year == 0 || to_year == 0 { return Err(crate::CoreError::ZeroYear); }
        if from_year > to_year { return Err(crate::CoreError::InvertedRange); }
        Ok(Self { from_year, to_year })
    }
    pub fn intersects(&self, o: &TimeRange) -> bool {
        self.from_year <= o.to_year && o.from_year <= self.to_year
    }
    pub fn contains_year(&self, y: Year) -> bool {
        self.from_year <= y && y <= self.to_year
    }
}
```

`src/canon.rs` — the full 66-book table. `BookInfo { code, osis, name }`; `resolve_alias` lowercases and strips non-alphanumerics, then matches against code, osis, and name treated the same way:

```rust
pub struct BookInfo { pub code: &'static str, pub osis: &'static str, pub name: &'static str }

pub const BOOKS: [BookInfo; 66] = [
    BookInfo{code:"GEN",osis:"Gen",name:"Genesis"}, BookInfo{code:"EXO",osis:"Exod",name:"Exodus"},
    BookInfo{code:"LEV",osis:"Lev",name:"Leviticus"}, BookInfo{code:"NUM",osis:"Num",name:"Numbers"},
    BookInfo{code:"DEU",osis:"Deut",name:"Deuteronomy"}, BookInfo{code:"JOS",osis:"Josh",name:"Joshua"},
    BookInfo{code:"JDG",osis:"Judg",name:"Judges"}, BookInfo{code:"RUT",osis:"Ruth",name:"Ruth"},
    BookInfo{code:"1SA",osis:"1Sam",name:"1 Samuel"}, BookInfo{code:"2SA",osis:"2Sam",name:"2 Samuel"},
    BookInfo{code:"1KI",osis:"1Kgs",name:"1 Kings"}, BookInfo{code:"2KI",osis:"2Kgs",name:"2 Kings"},
    BookInfo{code:"1CH",osis:"1Chr",name:"1 Chronicles"}, BookInfo{code:"2CH",osis:"2Chr",name:"2 Chronicles"},
    BookInfo{code:"EZR",osis:"Ezra",name:"Ezra"}, BookInfo{code:"NEH",osis:"Neh",name:"Nehemiah"},
    BookInfo{code:"EST",osis:"Esth",name:"Esther"}, BookInfo{code:"JOB",osis:"Job",name:"Job"},
    BookInfo{code:"PSA",osis:"Ps",name:"Psalms"}, BookInfo{code:"PRO",osis:"Prov",name:"Proverbs"},
    BookInfo{code:"ECC",osis:"Eccl",name:"Ecclesiastes"}, BookInfo{code:"SNG",osis:"Song",name:"Song of Solomon"},
    BookInfo{code:"ISA",osis:"Isa",name:"Isaiah"}, BookInfo{code:"JER",osis:"Jer",name:"Jeremiah"},
    BookInfo{code:"LAM",osis:"Lam",name:"Lamentations"}, BookInfo{code:"EZK",osis:"Ezek",name:"Ezekiel"},
    BookInfo{code:"DAN",osis:"Dan",name:"Daniel"}, BookInfo{code:"HOS",osis:"Hos",name:"Hosea"},
    BookInfo{code:"JOL",osis:"Joel",name:"Joel"}, BookInfo{code:"AMO",osis:"Amos",name:"Amos"},
    BookInfo{code:"OBA",osis:"Obad",name:"Obadiah"}, BookInfo{code:"JON",osis:"Jonah",name:"Jonah"},
    BookInfo{code:"MIC",osis:"Mic",name:"Micah"}, BookInfo{code:"NAM",osis:"Nah",name:"Nahum"},
    BookInfo{code:"HAB",osis:"Hab",name:"Habakkuk"}, BookInfo{code:"ZEP",osis:"Zeph",name:"Zephaniah"},
    BookInfo{code:"HAG",osis:"Hag",name:"Haggai"}, BookInfo{code:"ZEC",osis:"Zech",name:"Zechariah"},
    BookInfo{code:"MAL",osis:"Mal",name:"Malachi"}, BookInfo{code:"MAT",osis:"Matt",name:"Matthew"},
    BookInfo{code:"MRK",osis:"Mark",name:"Mark"}, BookInfo{code:"LUK",osis:"Luke",name:"Luke"},
    BookInfo{code:"JHN",osis:"John",name:"John"}, BookInfo{code:"ACT",osis:"Acts",name:"Acts"},
    BookInfo{code:"ROM",osis:"Rom",name:"Romans"}, BookInfo{code:"1CO",osis:"1Cor",name:"1 Corinthians"},
    BookInfo{code:"2CO",osis:"2Cor",name:"2 Corinthians"}, BookInfo{code:"GAL",osis:"Gal",name:"Galatians"},
    BookInfo{code:"EPH",osis:"Eph",name:"Ephesians"}, BookInfo{code:"PHP",osis:"Phil",name:"Philippians"},
    BookInfo{code:"COL",osis:"Col",name:"Colossians"}, BookInfo{code:"1TH",osis:"1Thess",name:"1 Thessalonians"},
    BookInfo{code:"2TH",osis:"2Thess",name:"2 Thessalonians"}, BookInfo{code:"1TI",osis:"1Tim",name:"1 Timothy"},
    BookInfo{code:"2TI",osis:"2Tim",name:"2 Timothy"}, BookInfo{code:"TIT",osis:"Titus",name:"Titus"},
    BookInfo{code:"PHM",osis:"Phlm",name:"Philemon"}, BookInfo{code:"HEB",osis:"Heb",name:"Hebrews"},
    BookInfo{code:"JAS",osis:"Jas",name:"James"}, BookInfo{code:"1PE",osis:"1Pet",name:"1 Peter"},
    BookInfo{code:"2PE",osis:"2Pet",name:"2 Peter"}, BookInfo{code:"1JN",osis:"1John",name:"1 John"},
    BookInfo{code:"2JN",osis:"2John",name:"2 John"}, BookInfo{code:"3JN",osis:"3John",name:"3 John"},
    BookInfo{code:"JUD",osis:"Jude",name:"Jude"}, BookInfo{code:"REV",osis:"Rev",name:"Revelation"},
];

fn norm(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase()
}

pub fn resolve_alias(s: &str) -> Option<crate::refs::BookId> {
    let n = norm(s);
    BOOKS.iter().position(|b| norm(b.code) == n || norm(b.osis) == n || norm(b.name) == n)
        .map(|i| crate::refs::BookId(i as u8))
}
```

`src/refs.rs`: `BookId(pub u8)` with `code()`, `name()`; `VerseId` with custom serde (`Serialize` as `format!("{}.{}.{}", book.code(), chapter, verse)`, `Deserialize` via parse); `ScriptureRef` with `parse` (strict canonical: split on `.`, book must equal a `code` exactly (case-insensitive OK), chapter/verse ≥ 1, passage `V1-V2` requires `V1 < V2`) and `Display`. `CoreError` in `lib.rs` via thiserror: variants `ZeroYear`, `InvertedRange`, `BadRef(String)`.

`src/wire.rs`: the structs exactly as the Interfaces block above (plain derive; only `sref` carries `#[serde(rename = "ref")]`, and `window`/`sref` carry `#[serde(skip_serializing_if = "Option::is_none")]`).

`src/lib.rs`: `pub mod time; pub mod canon; pub mod refs; pub mod wire;` + `CoreError`.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p atlas-core`
Expected: all PASS (including proptest suites).

- [ ] **Step 5: Golden fixture test**

`server/atlas-core/tests/golden.rs` — writes the fixture if absent, verifies byte-identical round-trip if present (the C# side deserializes the same file in Task 7):

```rust
use atlas_core::{time::TimeRange, wire::*};
use std::{fs, path::Path};

fn sample_scene() -> Scene {
    Scene {
        mode: "time".into(),
        window: Some(TimeRange::new(-1450, -1400).unwrap()),
        sref: None,
        places: vec![ScenePlace {
            id: "jericho".into(), name: "Jericho".into(), lat: 31.8703, lon: 35.4436,
            brightness: 2,
            events: vec![SceneEvent {
                id: "ev_jericho_falls".into(), label: "The walls of Jericho fall".into(),
                when: TimeRange::new(-1406, -1406).unwrap(),
                verse_groups: vec![VerseGroup { book: "JOS".into(), chapter: 6,
                    verses: vec!["JOS.6.1".into(), "JOS.6.20".into()], count: 27 }],
            }],
        }],
        arrows: vec![SceneArrow { narrative: "conquest".into(), color: "#7C3AED".into(),
            from_place: "gilgal".into(), to_place: "jericho".into(),
            from_event: "ev_camp_gilgal".into(), to_event: "ev_jericho_falls".into(), order: 2 }],
        narratives: vec![SceneNarrative { id: "conquest".into(), name: "The Conquest".into(),
            color: "#7C3AED".into(), legs_in_scene: 4 }],
    }
}

#[test]
fn golden_scene_fixture() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/golden-scene.json");
    let json = serde_json::to_string_pretty(&sample_scene()).unwrap();
    if !path.exists() {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &json).unwrap();
    }
    let on_disk = fs::read_to_string(&path).unwrap().replace("\r\n", "\n");
    assert_eq!(on_disk.trim(), json.trim(), "wire format drifted from committed golden fixture");
    let back: Scene = serde_json::from_str(&on_disk).unwrap();
    assert_eq!(back, sample_scene());
}
```

Run: `cargo test -p atlas-core --test golden` → PASS, fixture file created. Open `tests/fixtures/golden-scene.json` and verify keys are snake_case and the ref key is literally `"ref"`.

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "feat(core): years, canon, scripture refs, wire types, golden fixture"
```

---

### Task 3: atlas-core — AtlasData + scene composition + arrow invariants (the semantic heart)

**Files:**
- Create: `server/atlas-core/src/data.rs`, `src/scene.rs`
- Test: inline modules with a shared fixture builder + proptest invariants

**Interfaces:**
- Consumes: Task 2 types.
- Produces:
  - `data::AtlasData { pub canon: Canon, pub places: Vec<Place>, pub events: Vec<Event>, pub narratives: Vec<Narrative>, pub eras: Vec<Era>, pub books_meta: Vec<BookMeta>, pub verses: std::collections::HashMap<String, String>, pub cross_refs: std::collections::HashMap<String, Vec<CrossRef>> }` plus derived indexes built by `AtlasData::finish(self) -> AtlasData` (place index by id, events sorted by `when.from_year`, verse→event ids)
  - `Canon { books: Vec<CanonBook> }`, `CanonBook { code: String, name: String, chapters: Vec<u16> }` (verse counts per chapter)
  - `Place { id: String, name: String, lat: f64, lon: f64, verse_links: Vec<String> }` (verse_links = canonical verse ids from geocoding, used by scripture mode)
  - `Event { id: String, label: String, when: TimeRange, places: Vec<String>, verses: Vec<String> }` — `places[0]` is the anchor
  - `Narrative { id: String, name: String, color: String, legs: Vec<String> }`, `Era { id, name, from_year, to_year }`, `BookMeta { book: String, author: String, write_place: Option<String>, write_from: Option<i32>, write_to: Option<i32> }`, `CrossRef { target: String, votes: i32 }`
  - `scene::compose_time_scene(&AtlasData, TimeRange) -> Scene`
  - `scene::compose_scripture_scene(&AtlasData, &ScriptureRef) -> Scene`
  - `scene::ref_contains(r: &ScriptureRef, v: &VerseId) -> bool`
  - All `AtlasData` records derive `Serialize + Deserialize` (they ARE the compiled-file schema; ETL writes them, server reads them)

- [ ] **Step 1: Write the fixture builder and failing example tests**

In `src/scene.rs` tests — a builder producing a small world: 4 places (`gilgal`, `jericho`, `ai`, `hebron`), one narrative `conquest` (#7C3AED) with legs `e1@gilgal(-1406)`, `e2@jericho(-1406)`, `e3@jericho(-1405)` (same place as e2 — skip case), `e4@ai(-1405)`; a second narrative `patriarchs-demo` (#D97706) sharing event `e2`; an unrelated event `e5@hebron(-2000)`; verses wired so each event holds 2–3 canonical verse ids, plus `hebron` geocoding-linked to `GEN.13.18`:

```rust
#[cfg(test)]
pub(crate) fn fixture() -> AtlasData { /* literal construction of the above, then .finish() */ }

#[test]
fn time_scene_lights_only_intersecting() {
    let d = fixture();
    let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
    let ids: Vec<&str> = s.places.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"gilgal") && ids.contains(&"jericho") && ids.contains(&"ai"));
    assert!(!ids.contains(&"hebron")); // -2000 outside window
}

#[test]
fn arrows_skip_same_place_and_chain() {
    let d = fixture();
    let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
    let conquest: Vec<_> = s.arrows.iter().filter(|a| a.narrative == "conquest").collect();
    // e1->e2 (gilgal->jericho), e2->e3 skipped (same place), e3->e4 (jericho->ai)
    assert_eq!(conquest.len(), 2);
    assert_eq!((conquest[0].from_place.as_str(), conquest[0].to_place.as_str()), ("gilgal", "jericho"));
    assert_eq!((conquest[1].from_place.as_str(), conquest[1].to_place.as_str()), ("jericho", "ai"));
    assert_eq!(conquest[0].to_place, conquest[1].from_place); // ARROW-3 place-chain
}

#[test]
fn scripture_scene_uses_links_not_dates() {
    let d = fixture();
    let s = compose_scripture_scene(&d, &ScriptureRef::parse("GEN.13.18").unwrap());
    assert_eq!(s.mode, "scripture");
    assert!(s.places.iter().any(|p| p.id == "hebron"));
}

#[test]
fn brightness_and_caps() {
    let d = fixture();
    let s = compose_time_scene(&d, TimeRange::new(-1406, -1405).unwrap());
    let jericho = s.places.iter().find(|p| p.id == "jericho").unwrap();
    assert_eq!(jericho.brightness, 2u8.min(5)); // e2 and e3
    for pl in &s.places { for ev in &pl.events { for g in &ev.verse_groups {
        assert!(g.verses.len() <= 20);
        assert!(g.count as usize >= g.verses.len());
    }}}
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p atlas-core scene`
Expected: FAIL (functions/types missing).

- [ ] **Step 3: Implement data.rs and scene.rs**

`compose_time_scene` algorithm (implement exactly):

```rust
pub fn compose_time_scene(d: &AtlasData, w: TimeRange) -> Scene {
    let kept: Vec<&Event> = d.events.iter().filter(|e| e.when.intersects(&w)).collect();
    let places = lit_places(d, &kept);          // group kept events by ALL their places
    let arrows = build_arrows(d, &w, None);      // see below
    let narratives = legend(d, &arrows);
    Scene { mode: "time".into(), window: Some(w), sref: None, places, arrows, narratives }
}
```

`lit_places`: for each place with ≥1 kept event: `brightness = min(count, 5) as u8`; events → `SceneEvent` with `verse_groups` grouped by `(book, chapter)`, verses sorted canonically, capped at 20 ids with true `count`.

`build_arrows(d, window, ref_filter)` — the §7.4 algorithm, shared by both modes:

```rust
fn build_arrows(d: &AtlasData, w: &TimeRange, r: Option<&ScriptureRef>) -> Vec<SceneArrow> {
    let mut out = Vec::new();
    for n in &d.narratives {
        let kept: Vec<&Event> = n.legs.iter()
            .filter_map(|id| d.event_by_id(id))
            .filter(|e| match r {
                None => e.when.intersects(w),
                Some(sr) => e.verses.iter().any(|v| ref_contains(sr,
                    &VerseId::parse_canonical(v).expect("etl-validated"))),
            })
            .collect();
        let mut order = 0u32;
        for pair in kept.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            order += 1;
            if a.places[0] == b.places[0] { continue; } // same-place: chain passes through
            out.push(SceneArrow {
                narrative: n.id.clone(), color: n.color.clone(),
                from_place: a.places[0].clone(), to_place: b.places[0].clone(),
                from_event: a.id.clone(), to_event: b.id.clone(), order,
            });
        }
    }
    out
}
```

Wait — the place-chain invariant (ARROW-3) requires that after a same-place skip, the next arrow's `from_place` equals the previous arrow's `to_place`. With `windows(2)` and a skip of `(e2,e3)` at jericho, the next pair `(e3,e4)` has `from_place = jericho` = previous `to_place` ✓ because e2 and e3 share the anchor place. `windows(2)` is correct; keep `order` incrementing over ALL consecutive pairs so orders stay stable, arrows sorted by order per narrative naturally.

For scripture mode, lit places = union of (a) geocoding `verse_links` hits (`ref_contains`), and (b) places of events kept by the ref filter — both rendered as `ScenePlace`s; events attach only where they exist, geocoding-only places get an empty `events` list EXCEPT they must still show their linked verses: represent those as one synthetic `SceneEvent { id: "links", label: "Mentioned in text", when: <the ref's book write-independent placeholder…> }` — NO. Keep the wire honest: give `ScenePlace` for geocoding-only hits a single `SceneEvent` with `id: format!("mention-{place}"), label: "Mentioned"` and `when` spanning the whole atlas span `TimeRange::new(-4004, 100)`, verse_groups = the matched verses. Document this in code and CONTRACT.md: `mention-*` event ids are synthetic text-mention pseudo-events, excluded from ARROW-7 checks (arrows never reference them).

`ref_contains`: Book matches book; Chapter matches book+chapter; Passage matches book+chapter and `from_verse <= v.verse <= to_verse`; Verse matches exactly.

- [ ] **Step 4: Run example tests to verify pass**

Run: `cargo test -p atlas-core scene` → PASS.

- [ ] **Step 5: Write the proptest invariant suite (core-level mirror of ARROW-1..7)**

Add to `src/scene.rs` tests — strategy generates random windows over the fixture (and a second, larger generated dataset: 12 places, 3 narratives with 5–9 legs each, random years in span, built deterministically from a proptest-generated seed vector):

```rust
proptest! {
    #[test]
    fn arrow_invariants(w in window_strategy()) {
        let d = big_fixture();
        let s = compose_time_scene(&d, w);
        let place_ids: std::collections::HashSet<_> = s.places.iter().map(|p| &p.id).collect();
        for n in s.narratives.iter() {
            let mut arrows: Vec<_> = s.arrows.iter().filter(|a| a.narrative == n.id).collect();
            arrows.sort_by_key(|a| a.order);
            for a in &arrows {
                prop_assert!(place_ids.contains(&a.from_place) && place_ids.contains(&a.to_place)); // ARROW-1
                prop_assert_eq!(&a.color, &n.color);                                                // ARROW-2
                prop_assert_ne!(&a.from_place, &a.to_place);                                        // ARROW-5
                let (fe, te) = (d.event_by_id(&a.from_event).unwrap(), d.event_by_id(&a.to_event).unwrap());
                prop_assert!(te.when.from_year >= fe.when.from_year);                               // ARROW-6
            }
            for pair in arrows.windows(2) {
                prop_assert_eq!(&pair[0].to_place, &pair[1].from_place);                            // ARROW-3
            }
            if let (Some(first), Some(last)) = (arrows.first(), arrows.last()) {
                prop_assert!(!arrows.iter().any(|a| a.to_event == first.from_event));               // ARROW-4
                prop_assert!(!arrows.iter().any(|a| a.from_event == last.to_event));                // ARROW-4
            }
        }
        // SCENE-2 mirror
        for p in &s.places {
            prop_assert!(!p.events.is_empty());
            prop_assert_eq!(p.brightness, (p.events.len() as u8).min(5));
            for e in &p.events { prop_assert!(e.when.intersects(&w)); }
        }
    }
    #[test]
    fn window_monotonicity(w in window_strategy()) { // SCENE-3 mirror
        let d = big_fixture();
        let grow = TimeRange::new(
            if w.from_year == 1 { -1 } else { w.from_year - 1 },
            next_year(w.to_year).min(100).max(w.to_year)).unwrap();
        let (s1, s2) = (compose_time_scene(&d, w), compose_time_scene(&d, grow));
        let ids2: std::collections::HashSet<_> = s2.places.iter().map(|p| p.id.clone()).collect();
        for p in &s1.places { prop_assert!(ids2.contains(&p.id)); }
    }
}
```

(`window_strategy` = the Task 2 `range_strategy` clamped to span; `big_fixture` built with plain deterministic code — no randomness inside, so shrinking stays meaningful.)

- [ ] **Step 6: Run full core suite; commit**

Run: `cargo test -p atlas-core` → all PASS.

```powershell
git add -A; git commit -m "feat(core): atlas data model and scene composition with proptest arrow invariants"
```

---

### Task 4: atlas-etl — parsers, validation, report, compile

**Files:**
- Create: `server/atlas-etl/src/main.rs` (replace stub), `src/kjv.rs`, `src/geo.rs`, `src/theographic.rs`, `src/xrefs.rs`, `src/curated.rs`, `src/validate.rs`, `src/report.rs`
- Test: `server/atlas-etl/tests/fixtures/` (tiny handwritten raw-format samples) + `server/atlas-etl/tests/etl.rs`

**Interfaces:**
- Consumes: Task 3 `AtlasData` record types (ETL writes files that deserialize into them), `canon::resolve_alias`, `ScriptureRef`/`VerseId` parsing.
- Produces: `data/compiled/{canon,places,events,narratives,eras,books-meta,verses-kjv,cross-refs}.json` + `data/compiled/report.txt`. File contents = serde JSON of the Task 3 types (`Vec<Place>`, `Vec<Event>`, …; `verses-kjv.json` = `HashMap<String,String>`; `cross-refs.json` = `HashMap<String, Vec<CrossRef>>`).

**Parser contracts** (each `pub fn parse_x(input: &str) -> anyhow::Result<...>`, pure, no I/O — I/O lives in `main.rs`):

1. `kjv::parse(&str) -> (Canon, HashMap<String,String>)` — adapt to the actual `kjv.json` layout recorded in `data/raw/README.md` (scrollmapper layout: array of books with nested chapters/verses; resolve book names via `resolve_alias`, hard-error on unresolved). Output verse keys are canonical (`GEN.1.1`).
2. `geo::parse(&str) -> Vec<Place>` — from the geo bundle's place file: name, lat, lon, linked KJV verse refs (normalize via `resolve_alias` + chapter/verse). Slug: kebab-case of name (`Antioch of Pisidia` → `antioch-of-pisidia`); on collision append `-2`, `-3` in encounter order and note in report.
3. `theographic::parse_events(places_json: &str, verses_json: &str, events_json: &str, place_slug_by_name: &HashMap<String,String>) -> (Vec<Event>, TheoStats)` — join Airtable-style linked records: event → place record ids → place names → our slugs (unmatched name ⇒ create a new `Place` from Theographic lat/lon — return those too); event → verse record ids → osisRef strings → canonical ids. Dates: accept `-1446`, `1446 BC`, ISO-ish `-1446-04-01`; year 0 or unparseable ⇒ event dropped into `TheoStats.undated` (reported, not fatal).
4. `xrefs::parse(&str) -> HashMap<String, Vec<CrossRef>>` — TSV `From<TAB>To<TAB>Votes`; `To` may be a span (`Gen.1.1-Gen.1.5`) — keep span text canonicalized (`GEN.1.1-5` when same chapter, else `GEN.1.1-GEN.1.5`); sort each vec by votes desc; drop self-references.
5. `curated::parse_eras/parse_books/parse_narratives/parse_events_extra` — TOML per §4.5 schemas.

- [ ] **Step 1: Write fixture files + failing tests**

Create tiny literal fixtures under `tests/fixtures/` for each raw format (5–10 lines each, copied/trimmed from the real downloads per `data/raw/README.md`), plus invalid ones: `narrative-dangling-leg.toml`, `eras-gap.toml`, `event-bad-place.toml`, `event-year-zero.toml`. `tests/etl.rs`:

```rust
#[test] fn kjv_parses_and_keys_canonically() {
    let (canon, verses) = atlas_etl::kjv::parse(include_str!("fixtures/kjv-sample.json")).unwrap();
    assert_eq!(canon.books[0].code, "GEN");
    assert!(verses.contains_key("GEN.1.1"));
}
#[test] fn geo_slug_collisions_get_suffixes() { /* two "Antioch" rows -> antioch, antioch-2 */ }
#[test] fn theographic_undated_events_dropped_not_fatal() { /* stats.undated == 1 */ }
#[test] fn xrefs_sorted_desc_and_no_self() { /* votes 3,20,7 -> 20,7,3; self row dropped */ }
#[test] fn validate_dangling_narrative_leg_fails() {
    let err = atlas_etl::validate::run(&bad_data_with_dangling_leg()).unwrap_err();
    assert!(err.to_string().contains("dangling leg"));
}
#[test] fn validate_era_gap_fails() { /* eras [-10..-5],[-3..3(!zero)] -> two errors: gap and zero-year */ }
#[test] fn validate_nonchronological_legs_fail() { /* leg years -5 then -9 -> error */ }
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p atlas-etl` → FAIL (modules missing).

- [ ] **Step 3: Implement parsers, validate, report, main**

`validate::run(&AtlasData) -> anyhow::Result<()>` collects ALL violations into one error message (never stop at first): unknown place ids in events/narratives (via legs→events), dangling legs, non-canon verse ids, era gaps/overlaps/zero-years/coverage of `[-4004,100]`, duplicate event ids, non-chronological legs (`legs[k+1].when.from_year >= legs[k].when.from_year` required), `books.toml` unknown `write_place` ⇒ WARN into report + drop field. `report::write` emits: counts per compiled file; % events dated (Theo stats); % KJV verses with ≥1 geocoded place; per-narrative leg counts; slug collisions; warnings.

`main.rs`: read `data/raw/…` + `data/curated/…` → parse → merge (curated `events-extra` appended to Theographic events; id collision = hard error) → `validate::run` → write `data/compiled/*.json` + `report.txt`; print report to stdout; exit non-zero on validation error.

- [ ] **Step 4: Run tests to verify pass** — `cargo test -p atlas-etl` → PASS.

- [ ] **Step 5: Run ETL against real data**

Run: `cargo run -p atlas-etl` (from `server/`, after Task 1's fetch).
Expected: FAILS on validation — `data/curated/` doesn't exist yet. Confirm the error message lists missing curated inputs clearly (add a friendly "run Task 5 curation first" error if absent). That failure is correct TDD state for Task 5.

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "feat(etl): dataset parsers, hard validation, coverage report"
```

---

### Task 5: Curated data — eras, book metadata, first three narratives

**Files:**
- Create: `data/curated/eras.toml`, `data/curated/books.toml`, `data/curated/events-extra.toml`, `data/curated/narratives/{abraham-migration,exodus,paul-first-journey}.toml`

**Interfaces:**
- Consumes: ETL from Task 4; place slugs from the FIRST successful geo parse (`data/compiled/places.json` names/slugs — run `cargo run -p atlas-etl` and read the error/report to discover exact slugs; adjust slugs below to the real ones where they differ).
- Produces: a compiling `data/compiled/` set consumed by every later task.

- [ ] **Step 1: Write `eras.toml`** (verbatim; contiguous under `next_year`, covering `[-4004,100]`):

```toml
[[era]]
id = "primeval"
name = "Primeval"
from_year = -4004
to_year = -2167

[[era]]
id = "patriarchs"
name = "Patriarchs"
from_year = -2166
to_year = -1877

[[era]]
id = "egypt-exodus"
name = "Egypt & Exodus"
from_year = -1876
to_year = -1407

[[era]]
id = "conquest-judges"
name = "Conquest & Judges"
from_year = -1406
to_year = -1051

[[era]]
id = "united-kingdom"
name = "United Kingdom"
from_year = -1050
to_year = -932

[[era]]
id = "divided-kingdom"
name = "Divided Kingdom"
from_year = -931
to_year = -587

[[era]]
id = "exile"
name = "Exile"
from_year = -586
to_year = -539

[[era]]
id = "return"
name = "Return & Second Temple"
from_year = -538
to_year = -6

[[era]]
id = "gospels"
name = "Gospels"
from_year = -5
to_year = 29

[[era]]
id = "early-church"
name = "Early Church"
from_year = 30
to_year = 100
```

- [ ] **Step 2: Write `books.toml`** — all 66, traditional attributions. Format per row: `[[book]]` with `code`, `author`, optional `write_place` (slug), optional `write_from`/`write_to`. Full content (write_place omitted where tradition is vague; adjust slugs to real compiled ones):

```toml
[[book]]
code = "GEN"
author = "Moses"
write_from = -1445
write_to = -1405
[[book]]
code = "EXO"
author = "Moses"
write_from = -1445
write_to = -1405
[[book]]
code = "LEV"
author = "Moses"
write_from = -1445
write_to = -1405
[[book]]
code = "NUM"
author = "Moses"
write_from = -1445
write_to = -1405
[[book]]
code = "DEU"
author = "Moses"
write_from = -1407
write_to = -1406
[[book]]
code = "JOS"
author = "Joshua"
write_from = -1400
write_to = -1370
[[book]]
code = "JDG"
author = "Samuel (traditional)"
write_from = -1050
write_to = -1000
[[book]]
code = "RUT"
author = "Samuel (traditional)"
write_from = -1050
write_to = -1000
[[book]]
code = "1SA"
author = "Samuel, Nathan, Gad (traditional)"
write_from = -1000
write_to = -960
[[book]]
code = "2SA"
author = "Nathan and Gad (traditional)"
write_from = -960
write_to = -930
[[book]]
code = "1KI"
author = "Jeremiah (traditional)"
write_from = -590
write_to = -560
[[book]]
code = "2KI"
author = "Jeremiah (traditional)"
write_from = -590
write_to = -560
[[book]]
code = "1CH"
author = "Ezra (traditional)"
write_place = "jerusalem"
write_from = -450
write_to = -430
[[book]]
code = "2CH"
author = "Ezra (traditional)"
write_place = "jerusalem"
write_from = -450
write_to = -430
[[book]]
code = "EZR"
author = "Ezra"
write_place = "jerusalem"
write_from = -450
write_to = -430
[[book]]
code = "NEH"
author = "Nehemiah"
write_place = "jerusalem"
write_from = -430
write_to = -400
[[book]]
code = "EST"
author = "Mordecai (traditional)"
write_place = "susa"
write_from = -460
write_to = -440
[[book]]
code = "JOB"
author = "Unknown (Moses, traditional)"
write_from = -1450
write_to = -1400
[[book]]
code = "PSA"
author = "David and others"
write_place = "jerusalem"
write_from = -1010
write_to = -430
[[book]]
code = "PRO"
author = "Solomon and others"
write_place = "jerusalem"
write_from = -950
write_to = -700
[[book]]
code = "ECC"
author = "Solomon"
write_place = "jerusalem"
write_from = -935
write_to = -931
[[book]]
code = "SNG"
author = "Solomon"
write_place = "jerusalem"
write_from = -960
write_to = -950
[[book]]
code = "ISA"
author = "Isaiah"
write_place = "jerusalem"
write_from = -700
write_to = -680
[[book]]
code = "JER"
author = "Jeremiah"
write_from = -585
write_to = -580
[[book]]
code = "LAM"
author = "Jeremiah"
write_place = "jerusalem"
write_from = -586
write_to = -585
[[book]]
code = "EZK"
author = "Ezekiel"
write_place = "babylon"
write_from = -593
write_to = -570
[[book]]
code = "DAN"
author = "Daniel"
write_place = "babylon"
write_from = -605
write_to = -530
[[book]]
code = "HOS"
author = "Hosea"
write_from = -750
write_to = -710
[[book]]
code = "JOL"
author = "Joel"
write_from = -835
write_to = -800
[[book]]
code = "AMO"
author = "Amos"
write_from = -760
write_to = -750
[[book]]
code = "OBA"
author = "Obadiah"
write_from = -586
write_to = -580
[[book]]
code = "JON"
author = "Jonah"
write_from = -780
write_to = -750
[[book]]
code = "MIC"
author = "Micah"
write_from = -735
write_to = -700
[[book]]
code = "NAM"
author = "Nahum"
write_from = -650
write_to = -630
[[book]]
code = "HAB"
author = "Habakkuk"
write_from = -609
write_to = -605
[[book]]
code = "ZEP"
author = "Zephaniah"
write_from = -640
write_to = -620
[[book]]
code = "HAG"
author = "Haggai"
write_place = "jerusalem"
write_from = -520
write_to = -520
[[book]]
code = "ZEC"
author = "Zechariah"
write_place = "jerusalem"
write_from = -520
write_to = -480
[[book]]
code = "MAL"
author = "Malachi"
write_place = "jerusalem"
write_from = -430
write_to = -420
[[book]]
code = "MAT"
author = "Matthew"
write_from = 50
write_to = 60
[[book]]
code = "MRK"
author = "Mark"
write_place = "rome"
write_from = 55
write_to = 65
[[book]]
code = "LUK"
author = "Luke"
write_from = 60
write_to = 62
[[book]]
code = "JHN"
author = "John"
write_place = "ephesus"
write_from = 85
write_to = 95
[[book]]
code = "ACT"
author = "Luke"
write_place = "rome"
write_from = 62
write_to = 64
[[book]]
code = "ROM"
author = "Paul"
write_place = "corinth"
write_from = 57
write_to = 57
[[book]]
code = "1CO"
author = "Paul"
write_place = "ephesus"
write_from = 55
write_to = 55
[[book]]
code = "2CO"
author = "Paul"
write_from = 56
write_to = 56
[[book]]
code = "GAL"
author = "Paul"
write_place = "antioch"
write_from = 48
write_to = 49
[[book]]
code = "EPH"
author = "Paul"
write_place = "rome"
write_from = 60
write_to = 62
[[book]]
code = "PHP"
author = "Paul"
write_place = "rome"
write_from = 61
write_to = 62
[[book]]
code = "COL"
author = "Paul"
write_place = "rome"
write_from = 60
write_to = 62
[[book]]
code = "1TH"
author = "Paul"
write_place = "corinth"
write_from = 51
write_to = 51
[[book]]
code = "2TH"
author = "Paul"
write_place = "corinth"
write_from = 51
write_to = 52
[[book]]
code = "1TI"
author = "Paul"
write_from = 62
write_to = 64
[[book]]
code = "2TI"
author = "Paul"
write_place = "rome"
write_from = 66
write_to = 67
[[book]]
code = "TIT"
author = "Paul"
write_from = 63
write_to = 65
[[book]]
code = "PHM"
author = "Paul"
write_place = "rome"
write_from = 60
write_to = 62
[[book]]
code = "HEB"
author = "Unknown"
write_from = 64
write_to = 68
[[book]]
code = "JAS"
author = "James"
write_place = "jerusalem"
write_from = 45
write_to = 49
[[book]]
code = "1PE"
author = "Peter"
write_place = "rome"
write_from = 63
write_to = 65
[[book]]
code = "2PE"
author = "Peter"
write_place = "rome"
write_from = 65
write_to = 68
[[book]]
code = "1JN"
author = "John"
write_place = "ephesus"
write_from = 85
write_to = 95
[[book]]
code = "2JN"
author = "John"
write_place = "ephesus"
write_from = 85
write_to = 95
[[book]]
code = "3JN"
author = "John"
write_place = "ephesus"
write_from = 85
write_to = 95
[[book]]
code = "JUD"
author = "Jude"
write_from = 65
write_to = 80
[[book]]
code = "REV"
author = "John"
write_place = "patmos"
write_from = 95
write_to = 96
```

- [ ] **Step 3: Write `events-extra.toml` + three narrative files** (verbatim below; fix any slug that differs from `places.json` after the first ETL attempt — the validator will list every unknown slug):

`events-extra.toml` (schema: `[[event]]` with `id`, `label`, `from_year`, `to_year`, `places` (first = anchor), `verses`):

```toml
# --- Abraham's migration ---
[[event]]
id = "ab_ur"
label = "Terah's family leaves Ur"
from_year = -2100
to_year = -2092
places = ["ur"]
verses = ["GEN.11.28", "GEN.11.31"]
[[event]]
id = "ab_haran"
label = "Sojourn in Haran; the call of Abram"
from_year = -2092
to_year = -2091
places = ["haran"]
verses = ["GEN.11.31", "GEN.12.1", "GEN.12.4"]
[[event]]
id = "ab_shechem"
label = "Abram at the oak of Moreh"
from_year = -2091
to_year = -2091
places = ["shechem"]
verses = ["GEN.12.6", "GEN.12.7"]
[[event]]
id = "ab_bethel"
label = "Altar between Bethel and Ai"
from_year = -2091
to_year = -2090
places = ["bethel"]
verses = ["GEN.12.8"]
[[event]]
id = "ab_egypt"
label = "Famine drives Abram to Egypt"
from_year = -2090
to_year = -2089
places = ["egypt"]
verses = ["GEN.12.10", "GEN.12.14-20"]
[[event]]
id = "ab_hebron"
label = "Abram settles at Mamre in Hebron"
from_year = -2088
to_year = -2085
places = ["hebron"]
verses = ["GEN.13.1", "GEN.13.18"]

# --- The Exodus ---
[[event]]
id = "ex_rameses"
label = "Israel departs Rameses"
from_year = -1446
to_year = -1446
places = ["rameses"]
verses = ["EXO.12.37"]
[[event]]
id = "ex_succoth"
label = "First camp at Succoth"
from_year = -1446
to_year = -1446
places = ["succoth"]
verses = ["EXO.12.37", "EXO.13.20"]
[[event]]
id = "ex_red_sea"
label = "Crossing the Red Sea"
from_year = -1446
to_year = -1446
places = ["red-sea"]
verses = ["EXO.14.21-31"]
[[event]]
id = "ex_marah"
label = "Bitter water at Marah"
from_year = -1446
to_year = -1446
places = ["marah"]
verses = ["EXO.15.23-25"]
[[event]]
id = "ex_elim"
label = "Twelve springs at Elim"
from_year = -1446
to_year = -1446
places = ["elim"]
verses = ["EXO.15.27"]
[[event]]
id = "ex_rephidim"
label = "Water from the rock; war with Amalek"
from_year = -1446
to_year = -1446
places = ["rephidim"]
verses = ["EXO.17.1", "EXO.17.8-13"]
[[event]]
id = "ex_sinai"
label = "Covenant at Sinai"
from_year = -1446
to_year = -1445
places = ["sinai"]
verses = ["EXO.19.1-2", "EXO.19.18-20"]
[[event]]
id = "ex_kadesh"
label = "Spies return to Kadesh-barnea"
from_year = -1445
to_year = -1444
places = ["kadesh-barnea"]
verses = ["NUM.13.26", "NUM.14.1-4"]
[[event]]
id = "ex_moab"
label = "Camp on the plains of Moab"
from_year = -1407
to_year = -1406
places = ["moab"]
verses = ["NUM.22.1", "DEU.34.1-5"]
[[event]]
id = "ex_jordan"
label = "Israel crosses the Jordan"
from_year = -1406
to_year = -1406
places = ["jericho"]
verses = ["JOS.3.14-17"]

# --- Paul's first missionary journey ---
[[event]]
id = "p1_antioch"
label = "Barnabas and Saul sent from Antioch"
from_year = 46
to_year = 46
places = ["antioch"]
verses = ["ACT.13.1-3"]
[[event]]
id = "p1_seleucia"
label = "Down to the port of Seleucia"
from_year = 46
to_year = 46
places = ["seleucia"]
verses = ["ACT.13.4"]
[[event]]
id = "p1_salamis"
label = "Preaching in the synagogues of Salamis"
from_year = 46
to_year = 46
places = ["salamis"]
verses = ["ACT.13.5"]
[[event]]
id = "p1_paphos"
label = "Elymas blinded; the proconsul believes"
from_year = 46
to_year = 47
places = ["paphos"]
verses = ["ACT.13.6-12"]
[[event]]
id = "p1_perga"
label = "John Mark departs at Perga"
from_year = 47
to_year = 47
places = ["perga"]
verses = ["ACT.13.13"]
[[event]]
id = "p1_pisidian_antioch"
label = "Sermon in Pisidian Antioch"
from_year = 47
to_year = 47
places = ["antioch-2"]
verses = ["ACT.13.14-52"]
[[event]]
id = "p1_iconium"
label = "The city divided at Iconium"
from_year = 47
to_year = 48
places = ["iconium"]
verses = ["ACT.14.1-7"]
[[event]]
id = "p1_lystra"
label = "Stoned at Lystra"
from_year = 48
to_year = 48
places = ["lystra"]
verses = ["ACT.14.8-20"]
[[event]]
id = "p1_derbe"
label = "Many disciples at Derbe"
from_year = 48
to_year = 48
places = ["derbe"]
verses = ["ACT.14.20-21"]
[[event]]
id = "p1_return"
label = "Sailing home to Antioch"
from_year = 48
to_year = 48
places = ["attalia"]
verses = ["ACT.14.24-26"]
```

`narratives/abraham-migration.toml`:

```toml
id = "abraham-migration"
name = "Abraham's Migration"
color = "#D97706"
legs = ["ab_ur", "ab_haran", "ab_shechem", "ab_bethel", "ab_egypt", "ab_hebron"]
```

`narratives/exodus.toml`:

```toml
id = "exodus"
name = "The Exodus"
color = "#DC2626"
legs = ["ex_rameses", "ex_succoth", "ex_red_sea", "ex_marah", "ex_elim", "ex_rephidim", "ex_sinai", "ex_kadesh", "ex_moab", "ex_jordan"]
```

`narratives/paul-first-journey.toml`:

```toml
id = "paul-first-journey"
name = "Paul's First Journey"
color = "#16A34A"
legs = ["p1_antioch", "p1_seleucia", "p1_salamis", "p1_paphos", "p1_perga", "p1_pisidian_antioch", "p1_iconium", "p1_lystra", "p1_derbe", "p1_return"]
```

- [ ] **Step 4: Run ETL; fix slugs until green**

Run: `cargo run -p atlas-etl`
Expected first run: validation errors listing every curated slug that doesn't exist in the geo-derived places (e.g., `antioch-2`, `sinai`, `moab`, `red-sea` may differ). For each: check `data/compiled` attempt output / the validator's "did you mean" list (implement nearest-name suggestion via case-insensitive substring match if not present), correct the TOML, re-run. Loop until exit 0 and `report.txt` shows: 10 eras, 66 book rows, 3 narratives (6+10+10 legs), 0 errors.

- [ ] **Step 5: Commit**

```powershell
git add -A; git commit -m "feat(data): eras, book metadata, first three curated narratives"
```

---

### Task 6: atlas-server — endpoints over compiled data

**Files:**
- Create: `server/atlas-server/src/main.rs` (replace stub), `src/app.rs` (router construction, testable), `src/handlers.rs`, `src/error.rs`
- Test: `server/atlas-server/tests/api.rs` (tower `oneshot` against the fixture AtlasData from Task 3 — export `atlas_core::scene::tests::fixture` as `atlas_core::data::demo_fixture()` public helper behind `#[doc(hidden)]`)

**Interfaces:**
- Consumes: `AtlasData::load(dir)` (implement here or in core `data.rs`: reads the eight compiled JSON files), `compose_time_scene`, `compose_scripture_scene`, wire types.
- Produces (the HTTP contract — UX suite and client depend on it):
  - `GET /health` → 200 `ok`
  - `GET /api/scene?from=&to=` → `Scene`; zero/inverted/missing years → 400 `{error:{code:"bad_window",message}}`
  - `GET /api/scene/scripture?ref=` → `Scene`; bad ref → 400 `{error:{code:"bad_ref",…}}`
  - `GET /api/books` → `[{code,name,chapters:[verse counts]}]`
  - `GET /api/chapter/{cref}` → `{ref, book, chapter, verses:[{verse, text}]}` (cref form `EXO.14`)
  - `GET /api/verse/{vref}` → `{ref, text, book_meta:{author,write_place,write_from,write_to}, events:[SceneEvent-shaped with place ids], cross_refs:[{target, votes, preview}]}` (preview = text of span's first verse)
  - `GET /api/place/{id}` → `{id,name,lat,lon,events:[…all, sorted by when.from_year…]}`; unknown → 404 `{error:{code:"not_found",…}}`
  - `GET /api/narratives` → `[{id,name,color,legs:[event ids]}]`
  - `GET /api/eras` → `[{id,name,from_year,to_year}]`
  - `app::build(data: std::sync::Arc<atlas_core::data::AtlasData>, static_dir: Option<std::path::PathBuf>) -> axum::Router` — CORS permissive; ServeDir fallback when static_dir given
  - CLI: `atlas-server --data-dir ../data/compiled [--static-dir <path>] [--port 8000]` (hand-parse `std::env::args`, no clap)

- [ ] **Step 1: Write failing endpoint tests** (`tests/api.rs`, using `tower::ServiceExt::oneshot` — one test per endpoint + error cases):

```rust
async fn call(app: &axum::Router, uri: &str) -> (axum::http::StatusCode, serde_json::Value) { /* oneshot boilerplate */ }

#[tokio::test]
async fn scene_time_ok_and_errors() {
    let app = atlas_server::app::build(std::sync::Arc::new(atlas_core::data::demo_fixture()), None);
    let (st, body) = call(&app, "/api/scene?from=-1406&to=-1405").await;
    assert_eq!(st, 200);
    assert_eq!(body["mode"], "time");
    assert!(body["places"].as_array().unwrap().iter().any(|p| p["id"] == "jericho"));
    for bad in ["/api/scene?from=0&to=5", "/api/scene?from=5&to=-5", "/api/scene?from=1"] {
        let (st, body) = call(&app, bad).await;
        assert_eq!(st, 400, "{bad}");
        assert_eq!(body["error"]["code"], "bad_window");
    }
}
#[tokio::test] async fn scene_scripture_ok_and_bad_ref() { /* GEN.13.18 -> hebron; ref=NOPE -> 400 bad_ref */ }
#[tokio::test] async fn health_books_eras_narratives_shapes() { /* status + array lengths + key presence */ }
#[tokio::test] async fn verse_chapter_place_and_404() { /* fixture verse text, unknown place 404 */ }
```

- [ ] **Step 2: Run to verify failure** — `cargo test -p atlas-server` → FAIL.

- [ ] **Step 3: Implement** `error.rs` (an `ApiError { status, code, message }` implementing `IntoResponse` with the `{error:{code,message}}` body), `handlers.rs` (thin: parse params → core calls → Json), `app.rs` (router + `CorsLayer::permissive()` + optional `ServeDir` with `fallback_service`), `main.rs` (args, `AtlasData::load`, `finish()`, bind `0.0.0.0:{port}`, print listening line). Implement `AtlasData::load` + `demo_fixture()` in core if not present (`demo_fixture` = the Task 3 fixture builder moved to `data.rs` as `#[doc(hidden)] pub fn`). Add to `atlas-server` `[dev-dependencies]`: `tower = { version = "0.5", features = ["util"] }`, `http-body-util = "0.1"`.

- [ ] **Step 4: Run tests to verify pass** — `cargo test -p atlas-server` → PASS.

- [ ] **Step 5: Smoke against real data**

Run: `cargo run -p atlas-server -- --data-dir ../data/compiled` then `Invoke-WebRequest http://localhost:8000/api/scene?from=-1450`&`to=-1400` (quote the URL in PowerShell) and `/api/books`. Expected: JSON with lit places incl. exodus-era ones; 66 books. Stop the server.

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "feat(server): axum api serving scenes, scripture, reader and metadata endpoints"
```

---

### Task 7: Blazor skeleton — DTOs, golden-fixture pin, AtlasClient, routing shell

**Files:**
- Create: `client/Dtos.cs`, `client/Wire.cs`, `client/LruCache.cs`, `client/AtlasClient.cs`, `client/Layout/MainLayout.razor`, `client/Pages/Reader.razor` (stub), `client/Pages/World.razor` (stub), modify `client/Program.cs`, `client/wwwroot/index.html`, `client/wwwroot/css/app.css`
- Create: `client.Tests/BibleAtlas.Client.Tests.csproj`, `client.Tests/GoldenSceneTests.cs`, `client.Tests/LruCacheTests.cs`

**Interfaces:**
- Consumes: `tests/fixtures/golden-scene.json` (Task 2), API contract (Task 6).
- Produces (every later client task uses these exact names):
  - `Dtos.cs` records: `Scene(string Mode, TimeRangeDto? Window, string? Ref, List<ScenePlace> Places, List<SceneArrow> Arrows, List<SceneNarrative> Narratives)`, `TimeRangeDto(int FromYear, int ToYear)`, `ScenePlace(string Id, string Name, double Lat, double Lon, int Brightness, List<SceneEvent> Events)`, `SceneEvent(string Id, string Label, TimeRangeDto When, List<VerseGroup> VerseGroups)`, `VerseGroup(string Book, int Chapter, List<string> Verses, int Count)`, `SceneArrow(string Narrative, string Color, string FromPlace, string ToPlace, string FromEvent, string ToEvent, int Order)`, `SceneNarrative(string Id, string Name, string Color, int LegsInScene)`, `BookTocEntry(string Code, string Name, List<int> Chapters)`, `EraDto(string Id, string Name, int FromYear, int ToYear)`, `ChapterOut(string Ref, string Book, int Chapter, List<VerseOut> Verses)`, `VerseOut(int Verse, string Text)`, `VerseDetail(string Ref, string Text, BookMetaDto BookMeta, List<SceneEvent> Events, List<CrossRefOut> CrossRefs)`, `BookMetaDto(string Author, string? WritePlace, int? WriteFrom, int? WriteTo)`, `CrossRefOut(string Target, int Votes, string Preview)`, `PlaceDetail(string Id, string Name, double Lat, double Lon, List<SceneEvent> Events)`, `NarrativeOut(string Id, string Name, string Color, List<string> Legs)`
  - `AtlasClient` (DI singleton): `Task<Scene> SceneTime(int from, int to)`, `Task<Scene> SceneScripture(string sref)`, `Task<List<BookTocEntry>> Books()`, `Task<List<EraDto>> Eras()`, `Task<ChapterOut> Chapter(string book, int chapter)`, `Task<VerseDetail> Verse(string vref)`, `Task<PlaceDetail> Place(string id)`, `Task<List<NarrativeOut>> Narratives()` — scene calls cached in `LruCache<string, Scene>` (capacity 48); `Books()`/`Eras()` cached forever
  - `LruCache<TKey, TValue>` with `bool TryGet(TKey, out TValue)`, `void Put(TKey, TValue)`
  - `JsonSerializerOptions` singleton `Wire.Options` with `PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower` — note `Ref` property + snake_case policy yields `ref` ✓
  - Routes: `/` and `/read/{Book}/{Chapter:int}` → `Reader`; `/world` → `World`. Header per CONTRACT (`nav-reader`, `nav-world`, `translation-select`, `attribution`).

- [ ] **Step 1: Write failing golden + LRU tests**

`client.Tests` csproj: `dotnet new xunit -o client.Tests -n BibleAtlas.Client.Tests` then `dotnet add client.Tests reference client`. `GoldenSceneTests.cs`:

```csharp
[Fact]
public void GoldenFixtureRoundTrips()
{
    var path = Path.Combine(AppContext.BaseDirectory, "../../../../tests/fixtures/golden-scene.json");
    var json = File.ReadAllText(path);
    var scene = JsonSerializer.Deserialize<Scene>(json, Wire.Options)!;
    Assert.Equal("time", scene.Mode);
    Assert.Equal(-1450, scene.Window!.FromYear);
    Assert.Equal("jericho", scene.Places[0].Id);
    Assert.Equal(27, scene.Places[0].Events[0].VerseGroups[0].Count);
    Assert.Equal("conquest", scene.Arrows[0].Narrative);
    Assert.Equal(2, scene.Arrows[0].Order);
    var back = JsonSerializer.Serialize(scene, Wire.Options);
    var reparsed = JsonSerializer.Deserialize<Scene>(back, Wire.Options)!;
    Assert.Equal(scene, reparsed with { }); // record value equality after round-trip
}
```

`LruCacheTests.cs`: put 49 entries into capacity-48 cache → first evicted; `TryGet` refreshes recency.

- [ ] **Step 2: Run to verify failure** — `dotnet test client.Tests` → FAIL (types missing).

- [ ] **Step 3: Implement** `Dtos.cs`, `Wire.cs` (options), `LruCache.cs`, `AtlasClient.cs` (BaseAddress from `ApiBase` config falling back to host base), `Program.cs` DI registration; `index.html` adds `<link href="vendor/leaflet/leaflet.css">`, `<script src="vendor/leaflet/leaflet.js">`; `MainLayout.razor` header with contract testids; stub pages rendering their testid roots (`reader-root`, `world-map` empty div).

Note on record equality: `List<>` members break default record equality — for the golden test compare via re-serialized JSON strings instead:

```csharp
Assert.Equal(JsonSerializer.Serialize(scene, Wire.Options), JsonSerializer.Serialize(reparsed, Wire.Options));
```

- [ ] **Step 4: Run tests to verify pass** — `dotnet test client.Tests` → PASS. Also `dotnet build client` → clean.

- [ ] **Step 5: Manual smoke** — run API (`cargo run -p atlas-server -- --data-dir ../data/compiled`) and `dotnet run --project client`; browse `http://localhost:5000` → header renders, no console errors. Stop both.

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "feat(client): dtos pinned to golden fixture, atlas client with lru cache, app shell"
```

---

### Task 8: UX harness — Playwright + fast-check bootstrap, CONTRACT.md, canon generators

**Files:**
- Create: `tests/ux/package.json`, `tests/ux/tsconfig.json`, `tests/ux/playwright.config.ts`, `tests/ux/start-api.ps1`, `tests/ux/start-client.ps1`, `tests/ux/lib/api.ts`, `tests/ux/lib/canon.ts`, `tests/ux/lib/years.ts`, `tests/ux/lib/fc.ts`, `tests/ux/smoke.spec.ts`, `tests/ux/CONTRACT.md`

**Interfaces:**
- Consumes: running API (8000) + client (5000); nothing from their source trees — ever.
- Produces: `npx playwright test` runs green from `tests/ux/`; helper modules used by Tasks 9–15; the CONTRACT that Tasks 10–15 implement against.

- [ ] **Step 1: Author `tests/ux/CONTRACT.md`** (verbatim — this is the language-agnostic UX contract):

```markdown
# Bible Atlas UX Contract

Any implementation of the Bible Atlas UI MUST expose the surfaces below.
The UX property suite couples ONLY to this contract (plus the HTTP API).

## URL patterns
- `/` — reader, defaults to GEN 1
- `/read/{BOOK}/{chapter}` — reader deep link (BOOK = canonical 3-letter code)
- `/read/{BOOK}/{chapter}#v{n}` — verse anchor
- `/world?from={year}&to={year}` — time mode (signed years, no zero)
- `/world?ref={REF}` — scripture mode (canonical ref)

## Displayed text formats
- Year: `1447 BC` or `AD 30`
- Range: `1447 BC – 1400 BC` (spaced en dash U+2013); single year shown as the year alone
- Canonical refs: `GEN`, `GEN.1`, `GEN.1.1`, `GEN.1.1-5`

## data-testid inventory
Header: `nav-reader`, `nav-world`, `translation-select`, `attribution`
World: `world-map`, `marker-{placeId}`, `place-card`, `place-card-title`,
  `verse-group-{BOOK}-{chapter}` (one per group, text contains `{count}`),
  `arrows-svg`, `arrow-{narrativeId}-{order}` (SVG path; attr `stroke` = narrative color;
  attr `data-faded` = "true"|"false"; `marker-end` set),
  `legend`, `legend-item-{narrativeId}` (button; `aria-pressed` = isolated),
  `slider`, `slider-readout` (an `<input>`, accepts typed year/range text, Enter applies),
  `slider-era-{eraId}` (clickable era label), `mode-chip` (text contains active ref),
  `mode-chip-return`,
  `arrow-tip` (visible while an arrow is hovered; text contains the narrative name),
  `toast` (non-blocking error notice; last good scene stays rendered beneath it)
Reader: `reader-root`, `verse-line-{n}`, `verse-num-{n}`, `verse-explore-{n}`,
  `reader-prev`, `reader-next`, `passage-chip`
Popover (shared): `popover`, `popover-title`, `popover-breadcrumb-back`,
  `popover-chip-xrefs`, `popover-chip-map`, `popover-chip-book`, `popover-chip-context`,
  `xref-item-{TARGET}` (TARGET = canonical ref/span text), `mini-map`, `mini-map-open-world`
Notes:
- `marker-{placeId}` elements carry the visible place label.
- Scene pseudo-events with ids beginning `mention-` are text-mention markers
  (scripture mode); arrows never reference them.
- The slider is `aria-disabled="true"` while scripture mode is active.
```

- [ ] **Step 2: Node project + config**

`package.json`: `{ "name":"bible-atlas-ux", "private":true, "devDependencies": { "@playwright/test":"^1.50", "fast-check":"^4", "typescript":"^5", "@types/node":"^24" } }` → run `npm install` then `npx playwright install chromium` (from `tests/ux/`).

`tsconfig.json`:

```json
{ "compilerOptions": { "target": "ES2022", "module": "ESNext", "moduleResolution": "Bundler", "strict": true, "types": ["node"], "skipLibCheck": true } }
```

`start-api.ps1`:

```powershell
$env:Path = "$env:Path;$env:USERPROFILE\.cargo\bin"
Set-Location "$PSScriptRoot\..\..\server"
cargo run -p atlas-server -- --data-dir ../data/compiled --port 8000
```

`start-client.ps1`:

```powershell
$env:Path = "$env:Path;$env:LOCALAPPDATA\Microsoft\dotnet"
$env:DOTNET_ROOT = "$env:LOCALAPPDATA\Microsoft\dotnet"
Set-Location "$PSScriptRoot\..\.."
dotnet run --project client --launch-profile http
```

`playwright.config.ts`:

```ts
import { defineConfig } from '@playwright/test';
export default defineConfig({
  timeout: 60_000,
  retries: 0,
  use: { baseURL: 'http://localhost:5000', trace: 'retain-on-failure' },
  webServer: [
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-api.ps1',
      url: 'http://localhost:8000/health', reuseExistingServer: true, timeout: 120_000 },
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-client.ps1',
      url: 'http://localhost:5000', reuseExistingServer: true, timeout: 180_000 },
  ],
});
```

- [ ] **Step 3: Helper libraries** (verbatim):

`lib/fc.ts`:

```ts
import fc from 'fast-check';
export const RUNS_API = Number(process.env.FC_NUM_RUNS ?? 150);
export const RUNS_UI = Number(process.env.FC_NUM_RUNS ?? 20);
export async function fcAssert<T>(prop: fc.IAsyncPropertyWithHooks<T> | fc.IAsyncProperty<T>, runs: number) {
  await fc.assert(prop as fc.IAsyncProperty<T>, { numRuns: runs, verbose: 2 });
}
```

`lib/years.ts`:

```ts
export const SPAN = { from: -4004, to: 100 };
export function formatYear(y: number): string { return y < 0 ? `${-y} BC` : `AD ${y}`; }
export function parseYearText(s: string): number {
  const bc = s.match(/^(\d+) BC$/); if (bc) return -Number(bc[1]);
  const ad = s.match(/^AD (\d+)$/); if (ad) return Number(ad[1]);
  throw new Error(`unparseable year text: ${s}`);
}
export function formatRange(from: number, to: number): string {
  return from === to ? formatYear(from) : `${formatYear(from)} – ${formatYear(to)}`;
}
```

`lib/api.ts`:

```ts
export const API = 'http://localhost:8000';
async function getJson(path: string): Promise<any> {
  const r = await fetch(`${API}${path}`);
  if (!r.ok) { const body = await r.json().catch(() => ({})); return { __status: r.status, ...body }; }
  return r.json();
}
export const api = {
  raw: getJson,
  sceneTime: (from: number, to: number) => getJson(`/api/scene?from=${from}&to=${to}`),
  sceneScripture: (sref: string) => getJson(`/api/scene/scripture?ref=${encodeURIComponent(sref)}`),
  books: () => getJson('/api/books'),
  eras: () => getJson('/api/eras'),
  chapter: (cref: string) => getJson(`/api/chapter/${cref}`),
  verse: (vref: string) => getJson(`/api/verse/${vref}`),
  place: (id: string) => getJson(`/api/place/${id}`),
  narratives: () => getJson('/api/narratives'),
};
```

`lib/canon.ts` (canon-derived generators — the key to meaningful exhaustive refs):

```ts
import fc from 'fast-check';
import { api } from './api';
import { SPAN } from './years';

export type Toc = { code: string; name: string; chapters: number[] }[];
let toc: Toc | null = null;
export async function loadToc(): Promise<Toc> { return (toc ??= await api.books()); }

export const arbYear = fc.integer({ min: SPAN.from, max: SPAN.to }).filter(y => y !== 0);
export const arbWindow = fc.tuple(arbYear, arbYear)
  .map(([a, b]) => (a <= b ? { from: a, to: b } : { from: b, to: a }));

export function arbChapterRef(t: Toc) {
  return fc.integer({ min: 0, max: t.length - 1 }).chain(bi =>
    fc.integer({ min: 1, max: t[bi].chapters.length })
      .map(ch => ({ book: t[bi].code, chapter: ch, verses: t[bi].chapters[ch - 1] })));
}
export function arbVerseRef(t: Toc) {
  return arbChapterRef(t).chain(c =>
    fc.integer({ min: 1, max: c.verses }).map(v => `${c.book}.${c.chapter}.${v}`));
}
export function arbPassageRef(t: Toc) {
  return arbChapterRef(t).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b)
      .map(([a, b]) => `${c.book}.${c.chapter}.${a}-${b}`));
}
export function arbAnyRef(t: Toc) {
  return fc.oneof(
    fc.integer({ min: 0, max: t.length - 1 }).map(i => t[i].code),
    arbChapterRef(t).map(c => `${c.book}.${c.chapter}`),
    arbVerseRef(t),
    arbPassageRef(t));
}
```

- [ ] **Step 4: Smoke spec** `smoke.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import { api } from './lib/api';

test('api health and toc', async () => {
  const toc = await api.books();
  expect(toc).toHaveLength(66);
  expect(toc[0].code).toBe('GEN');
  expect(toc[65].code).toBe('REV');
});
test('app shell renders', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-world')).toBeVisible();
});
```

- [ ] **Step 5: Run to verify pass** — from `tests/ux/`: `npx playwright test smoke.spec.ts` → 2 passed (webServer boots both processes; first boot is slow).

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "test(ux): playwright+fast-check harness, ux contract, canon generators"
```

---

### Task 9: API property suite (SCENE, ARROW, SCRIP, XREF, CHAP, ERA, BOOKS)

**Files:**
- Create: `tests/ux/api-scene.spec.ts`, `tests/ux/api-scripture.spec.ts`, `tests/ux/api-reader.spec.ts`, `tests/ux/api-exhaustive.spec.ts`

**Interfaces:**
- Consumes: Task 8 helpers; running API.
- Produces: the API half of the Property Catalog, green.

- [ ] **Step 1: Write `api-scene.spec.ts`** (verbatim; SCENE-1..5, ARROW-1..7):

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow, arbYear } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

function arrowsByNarrative(scene: any): Map<string, any[]> {
  const m = new Map<string, any[]>();
  for (const a of scene.arrows) { (m.get(a.narrative) ?? m.set(a.narrative, []).get(a.narrative)!).push(a); }
  for (const v of m.values()) v.sort((x, y) => x.order - y.order);
  return m;
}
const intersects = (a: any, w: { from: number; to: number }) =>
  a.from_year <= w.to && w.from <= a.to_year;

test('SCENE-1/2 + ARROW-1..7: window scene invariants', async () => {
  const narrColors = new Map((await api.narratives()).map((n: any) => [n.id, n.color]));
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const s = await api.sceneTime(w.from, w.to);
    expect(s.__status).toBeUndefined();                                   // SCENE-1
    expect(s.mode).toBe('time');
    expect(s.window).toEqual({ from_year: w.from, to_year: w.to });
    const placeIds = new Set(s.places.map((p: any) => p.id));
    for (const p of s.places) {                                           // SCENE-2
      expect(p.events.length).toBeGreaterThan(0);
      expect(p.brightness).toBe(Math.min(p.events.length, 5));
      for (const e of p.events) {
        expect(intersects(e.when, w)).toBe(true);
        for (const g of e.verse_groups) {
          expect(g.verses.length).toBeLessThanOrEqual(20);
          expect(g.count).toBeGreaterThanOrEqual(g.verses.length);
        }
      }
    }
    const eventsOf = (pid: string) => new Set(
      s.places.find((p: any) => p.id === pid)?.events.map((e: any) => e.id) ?? []);
    for (const [nid, arrows] of arrowsByNarrative(s)) {
      const sceneColor = s.narratives.find((n: any) => n.id === nid)?.color;
      for (const a of arrows) {
        expect(placeIds.has(a.from_place) && placeIds.has(a.to_place)).toBe(true); // ARROW-1
        expect(a.color).toBe(sceneColor);                                          // ARROW-2
        expect(a.color).toBe(narrColors.get(nid));                                 // ARROW-2
        expect(a.from_place).not.toBe(a.to_place);                                 // ARROW-5
        expect(eventsOf(a.from_place).has(a.from_event)).toBe(true);               // ARROW-7
        expect(eventsOf(a.to_place).has(a.to_event)).toBe(true);                   // ARROW-7
      }
      for (let k = 0; k + 1 < arrows.length; k++) {
        expect(arrows[k].to_place).toBe(arrows[k + 1].from_place);                 // ARROW-3
      }
      if (arrows.length > 0) {
        const toEvents = new Set(arrows.map(a => a.to_event));
        const fromEvents = new Set(arrows.map(a => a.from_event));
        expect(toEvents.has(arrows[0].from_event)).toBe(false);                    // ARROW-4
        expect(fromEvents.has(arrows[arrows.length - 1].to_event)).toBe(false);    // ARROW-4
      }
    }
    // ARROW-6 needs event years: read them from the scene itself
    const whenOf = new Map(s.places.flatMap((p: any) => p.events.map((e: any) => [e.id, e.when])));
    for (const a of s.arrows) {
      expect((whenOf.get(a.to_event) as any).from_year)
        .toBeGreaterThanOrEqual((whenOf.get(a.from_event) as any).from_year);      // ARROW-6
    }
  }), RUNS_API);
});

test('SCENE-3: window monotonicity', async () => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    const grow = { from: w.from === 1 ? -1 : w.from - 1, to: w.to === -1 ? 1 : Math.min(w.to + 1, 100) };
    const [s1, s2] = [await api.sceneTime(w.from, w.to), await api.sceneTime(grow.from, grow.to)];
    const ids2 = new Map(s2.places.map((p: any) => [p.id, new Set(p.events.map((e: any) => e.id))]));
    for (const p of s1.places) {
      expect(ids2.has(p.id)).toBe(true);
      for (const e of p.events) expect(ids2.get(p.id)!.has(e.id)).toBe(true);
    }
  }), RUNS_API);
});

test('SCENE-4: point windows deterministic', async () => {
  await fcAssert(fc.asyncProperty(arbYear, async y => {
    const [a, b] = [await api.sceneTime(y, y), await api.sceneTime(y, y)];
    expect(a).toEqual(b);
    for (const p of a.places) for (const e of p.events) expect(intersects(e.when, { from: y, to: y })).toBe(true);
  }), RUNS_API);
});

test('SCENE-5: invalid windows are typed 400s', async () => {
  for (const q of ['from=0&to=5', 'from=-5&to=0', 'from=5&to=-5', 'from=1', 'to=1', 'from=x&to=y']) {
    const r = await api.raw(`/api/scene?${q}`);
    expect(r.__status).toBe(400);
    expect(r.error.code).toBe('bad_window');
    expect(typeof r.error.message).toBe('string');
  }
});
```

- [ ] **Step 2: Write `api-scripture.spec.ts`** (SCRIP-1..3):

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbAnyRef, arbVerseRef } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

function refContains(sref: string, verseId: string): boolean {
  const [b, c, v] = verseId.split('.');
  const m = sref.match(/^([A-Z0-9]{3})(?:\.(\d+)(?:\.(\d+)(?:-(\d+))?)?)?$/)!;
  if (m[1] !== b) return false;
  if (m[2] === undefined) return true;
  if (Number(m[2]) !== Number(c)) return false;
  if (m[3] === undefined) return true;
  const [lo, hi] = [Number(m[3]), Number(m[4] ?? m[3])];
  return Number(v) >= lo && Number(v) <= hi;
}

test('SCRIP-1/3: scripture scenes are sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbAnyRef(toc), async sref => {
    const s = await api.sceneScripture(sref);
    expect(s.__status).toBeUndefined();
    expect(s.mode).toBe('scripture');
    expect(s.ref).toBe(sref);                                              // SCRIP-1
    for (const p of s.places) {
      const all = p.events.flatMap((e: any) => e.verse_groups.flatMap((g: any) => g.verses));
      expect(all.some((v: string) => refContains(sref, v))).toBe(true);    // SCRIP-1
    }
    const eventVerses = new Map(s.places.flatMap((p: any) => p.events.map((e: any) =>
      [e.id, e.verse_groups.flatMap((g: any) => g.verses)])));
    for (const a of s.arrows) {                                            // SCRIP-3
      for (const ev of [a.from_event, a.to_event]) {
        expect((eventVerses.get(ev) as string[]).some(v => refContains(sref, v))).toBe(true);
      }
    }
  }), RUNS_API);
});

test('SCRIP-2: ref monotonicity verse ⊆ passage ⊆ chapter ⊆ book', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    const chain = [vref, `${b}.${c}.${v}-${Number(v) + 1}`, `${b}.${c}`, b];
    let prev: Set<string> | null = null;
    for (const sref of chain) {
      const s = await api.sceneScripture(sref);
      if (s.__status) continue; // v+1 may exceed chapter; skip that link
      const ids = new Set<string>(s.places.map((p: any) => p.id));
      if (prev) for (const id of prev) expect(ids.has(id)).toBe(true);
      prev = ids;
    }
  }), RUNS_API);
});
```

Wait — SCRIP-2's chain as written compares verse ⊆ passage ⊆ chapter ⊆ book but when the passage link is skipped, `prev` must carry forward (it does — `continue` leaves `prev` untouched). Also bad-ref check: add to this file:

```ts
test('bad refs are typed 400s', async () => {
  for (const bad of ['NOPE', 'GEN.0', 'GEN.1.0', 'GEN.1.9-2', 'gen..1']) {
    const r = await api.raw(`/api/scene/scripture?ref=${encodeURIComponent(bad)}`);
    expect(r.__status).toBe(400);
    expect(r.error.code).toBe('bad_ref');
  }
});
```

- [ ] **Step 3: Write `api-reader.spec.ts`** (XREF-1, CHAP-1):

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef } from './lib/canon';
import { fcAssert, RUNS_API } from './lib/fc';

const SPAN_RE = /^[A-Z0-9]{3}\.\d+\.\d+(-(\d+|[A-Z0-9]{3}\.\d+\.\d+))?$/;

test('XREF-1: verse details are sound', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const d = await api.verse(vref);
    expect(d.__status).toBeUndefined();
    expect(d.ref).toBe(vref);
    expect(d.text.length).toBeGreaterThan(0);
    expect(typeof d.book_meta.author).toBe('string');
    let last = Infinity;
    for (const x of d.cross_refs) {
      expect(x.votes).toBeLessThanOrEqual(last); last = x.votes;   // votes descending
      expect(SPAN_RE.test(x.target)).toBe(true);                   // canon-parseable target
      expect(x.target).not.toBe(vref);                             // no self (exact match only)
      expect(x.preview.length).toBeGreaterThan(0);
    }
  }), RUNS_API);
});

test('CHAP-1: chapters match the TOC', async () => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    const ch = await api.chapter(`${c.book}.${c.chapter}`);
    expect(ch.verses.length).toBe(c.verses);
    ch.verses.forEach((v: any, i: number) => {
      expect(v.verse).toBe(i + 1);
      expect(v.text.length).toBeGreaterThan(0);
    });
  }), RUNS_API);
});
```

- [ ] **Step 4: Write `api-exhaustive.spec.ts`** (ERA-1, BOOKS-1, BOOKS-2 — full enumeration):

```ts
import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { loadToc } from './lib/canon';

const nextYear = (y: number) => (y === -1 ? 1 : y + 1);

test('ERA-1: eras contiguous, zero-free, covering the span (exhaustive)', async () => {
  const eras = await api.eras();
  expect(eras[0].from_year).toBe(-4004);
  expect(eras[eras.length - 1].to_year).toBe(100);
  for (const e of eras) {
    expect(e.from_year).not.toBe(0); expect(e.to_year).not.toBe(0);
    expect(e.from_year).toBeLessThanOrEqual(e.to_year);
  }
  for (let i = 0; i + 1 < eras.length; i++) {
    expect(nextYear(eras[i].to_year)).toBe(eras[i + 1].from_year);
  }
});

test('BOOKS-1/2: all 66 books, each with a working book scene (exhaustive)', async () => {
  const toc = await loadToc();
  expect(toc.map(b => b.code)).toHaveLength(66);
  expect(new Set(toc.map(b => b.code)).size).toBe(66);
  expect(toc[0].code).toBe('GEN'); expect(toc[39].code).toBe('MAT'); expect(toc[65].code).toBe('REV');
  for (const b of toc) {
    expect(b.chapters.length).toBeGreaterThanOrEqual(1);
    const s = await api.sceneScripture(b.code);           // BOOKS-2: every single book
    expect(s.__status).toBeUndefined();
    expect(s.mode).toBe('scripture');
  }
});
```

- [ ] **Step 5: Run to verify current state** — `npx playwright test api-` → expect failures ONLY where the server is wrong; fix server/core bugs (not tests) until green. Every fix gets its own small commit referencing the property id.

- [ ] **Step 6: Commit**

```powershell
git add -A; git commit -m "test(ux): api property suite - scene, arrow, scripture, reader, exhaustive domains"
```

---

### Task 10: TimeSlider + World page shell (WORLD-5, WORLD-7, part of NAV-1)

**Files:**
- Create: `client/Components/TimeSlider.razor`, `client/SliderScale.cs`, `client/YearText.cs`
- Modify: `client/Pages/World.razor`
- Test: `client.Tests/SliderScaleTests.cs`, `client.Tests/YearTextTests.cs`; UX: `tests/ux/world-slider.spec.ts`

**Interfaces:**
- Consumes: `AtlasClient.Eras()`, CONTRACT testids `slider`, `slider-readout`, `slider-era-{eraId}`, URL `/world?from&to`.
- Produces:
  - `static class SliderScale { public static double YearToX(int year, IReadOnlyList<EraDto> eras, double width); public static int XToYear(double x, IReadOnlyList<EraDto> eras, double width); }` — each era gets width `max(width/(eras.Count*2), width * eraYearSpan/totalYearSpan)` then all normalized to sum to `width`; piecewise-linear inside an era; results snapped to valid (non-zero) years
  - `static class YearText { public static string Format(int year); public static string FormatRange(int from, int to); public static bool TryParse(string text, out int from, out int to); }` — formats per CONTRACT; TryParse accepts `1447 BC`, `AD 30`, `700` (bare digits = BC by convention? NO — bare digits are rejected; contract text formats only), and ranges with the spaced en dash or a spaced hyphen (`-` typed by users) between two year texts
  - `World.razor` state: reads `from`/`to`/`ref` query params on init, exposes `ApplyWindow(int from, int to)` (updates URL via NavigationManager, fetches scene, debounced 150 ms latest-wins), renders slider + readout + era labels; scene rendering arrives in Task 11 (until then World renders `world-map` div + slider only). On scene-fetch failure or invalid/zero-year query params: render `toast` (testid, showing the error message) and keep the last good scene state — never blank the page

- [ ] **Step 1: Failing xunit tests**

```csharp
public class SliderScaleTests {
    static readonly List<EraDto> Eras = new() {
        new("a","A",-4004,-2167), new("b","B",-2166,-1877), new("c","C",-1876,-1407),
        new("d","D",-1406,-1051), new("e","E",-1050,-932), new("f","F",-931,-587),
        new("g","G",-586,-539), new("h","H",-538,-6), new("i","I",-5,29), new("j","J",30,100) };

    [Fact]
    public void RoundTripEveryYearInSpan() {
        for (int y = -4004; y <= 100; y++) {
            if (y == 0) continue;
            var x = SliderScale.YearToX(y, Eras, 1000.0);
            Assert.Equal(y, SliderScale.XToYear(x, Eras, 1000.0));
        }
    }
    [Fact]
    public void EveryEraGetsUsableWidth() {
        for (int i = 0; i < Eras.Count; i++) {
            var w0 = SliderScale.YearToX(Eras[i].FromYear, Eras, 1000.0);
            var w1 = SliderScale.YearToX(Eras[i].ToYear, Eras, 1000.0);
            Assert.True(w1 - w0 >= 1000.0 / (Eras.Count * 2) - 1e-6);
        }
    }
}
public class YearTextTests {
    [Theory]
    [InlineData(-1447, "1447 BC")] [InlineData(30, "AD 30")]
    public void Formats(int y, string s) => Assert.Equal(s, YearText.Format(y));
    [Fact] public void RangeUsesEnDash() => Assert.Equal("1447 BC – 1400 BC", YearText.FormatRange(-1447, -1400));
    [Fact] public void SingleYearRange() => Assert.Equal("1447 BC", YearText.FormatRange(-1447, -1447));
    [Theory]
    [InlineData("1447 BC", -1447, -1447)] [InlineData("1447 BC - 1400 BC", -1447, -1400)]
    [InlineData("AD 30 – AD 70", 30, 70)]
    public void Parses(string t, int f, int to) {
        Assert.True(YearText.TryParse(t, out var a, out var b));
        Assert.Equal((f, to), (a, b));
    }
    [Theory] [InlineData("0 BC")] [InlineData("banana")] [InlineData("1400 BC - 1447 BC")]
    public void Rejects(string t) => Assert.False(YearText.TryParse(t, out _, out _));
}
```

- [ ] **Step 2: Run to verify failure** — `dotnet test client.Tests` → FAIL.
- [ ] **Step 3: Implement** `SliderScale`, `YearText`, `TimeSlider.razor` (SVG-free HTML strip: era segments as flex divs sized by SliderScale widths with `slider-era-{id}` labels; brush = two absolutely-positioned drag handles wired with `@onpointerdown/move/up`; the readout `<input data-testid="slider-readout">` applies on Enter via `YearText.TryParse`; era label click → `ApplyWindow(era.FromYear, era.ToYear)`), `World.razor` query-param handling + debounce (`CancellationTokenSource` latest-wins).
- [ ] **Step 4: Run xunit to verify pass** — `dotnet test client.Tests` → PASS.
- [ ] **Step 5: UX properties** `tests/ux/world-slider.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { formatRange } from './lib/years';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-5: typed readout drives window, URL and readout agree', async ({ page }) => {
  await page.goto('/world?from=-1450&to=-1400');
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.getByTestId('slider-readout').fill(formatRange(w.from, w.to));
    await page.getByTestId('slider-readout').press('Enter');
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(w.from, w.to));
    await page.waitForURL(u => u.searchParams.get('from') === String(w.from)
                            && u.searchParams.get('to') === String(w.to));
  }), RUNS_UI);
});

test('WORLD-7: every era is on the slider and clickable (exhaustive)', async ({ page }) => {
  const eras = await api.eras();
  await page.goto('/world?from=-1450&to=-1400');
  for (const e of eras) {
    const label = page.getByTestId(`slider-era-${e.id}`);
    await expect(label).toContainText(e.name);
    await label.click();
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(e.from_year, e.to_year));
  }
});

test('NAV-1 (world/time): deep link survives reload', async ({ page }) => {
  await page.goto('/world?from=-1406&to=-1405');
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
  await page.reload();
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
});

test('errors surface as toast, app keeps standing', async ({ page }) => {
  await page.goto('/world?from=0&to=5');
  await expect(page.getByTestId('toast')).toBeVisible();
  await expect(page.getByTestId('world-map')).toBeAttached();
});
```

Run: `npx playwright test world-slider` → PASS (fix implementation until green).
- [ ] **Step 6: Commit** — `git add -A; git commit -m "feat(world): era-segmented year-exact slider with editable readout"`

---

### Task 11: map.js + markers + hover cards (WORLD-1, WORLD-2)

**Files:**
- Create: `client/wwwroot/js/map.js`, `client/MapInterop.cs`, `client/Components/PlaceCard.razor`
- Modify: `client/Pages/World.razor`, `client/wwwroot/css/app.css`
- Test: `tests/ux/world-map.spec.ts`

**Interfaces:**
- Consumes: Leaflet vendored files; scene DTOs; CONTRACT testids.
- Produces:
  - `map.js` ES module exports: `init(el, dotnetRef, opts) -> number` (opts: `{ mini: bool }`; creates `L.map`, tiles `https://server.arcgisonline.com/ArcGIS/rest/services/World_Shaded_Relief/MapServer/tile/{z}/{y}/{x}` maxNativeZoom 13 + attribution, fallback constant `TILE_FALLBACK = 'https://basemaps.cartocdn.com/light_nolabels/{z}/{x}/{y}.png'` switched to on tile `tileerror`), `setScene(id, scene)`, `fitScene(id)`, `destroy(id)`
  - Markers: `L.marker` with `L.divIcon` html `<div class="atlas-marker glow-{brightness}" data-testid="marker-{id}"><span class="atlas-label">{name}</span></div>`; events `mouseover`/`mouseout`/`click` → `dotnetRef.invokeMethodAsync('OnPlaceHover', id, x, y)` / `'OnPlaceLeave'` / `'OnPlaceClick', id, x, y` (x,y from `e.containerPoint`)
  - `MapInterop` (C#): `static Task<MapInterop> Create(IJSRuntime js, ElementReference el, bool mini, IMapEvents sink)`; `Task SetScene(Scene s)`, `Task FitScene()`, `ValueTask DisposeAsync()`; `interface IMapEvents { void OnPlaceHover(string id, double x, double y); void OnPlaceLeave(); void OnPlaceClick(string id, double x, double y); void OnArrowHover(string key, double x, double y); void OnArrowClick(string key, double x, double y); }`
  - `PlaceCard.razor` params: `ScenePlace Place`, `double X`, `double Y` — renders per CONTRACT (`place-card`, `place-card-title`, `verse-group-{BOOK}-{chapter}` rows showing `{count}`), stays open while pointer is over marker or card (parent tracks with a 150 ms close delay timer)

- [ ] **Step 1: Write failing UX properties** `tests/ux/world-map.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-1: rendered markers equal the API scene', async ({ page }) => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
    for (const p of scene.places) {
      await expect(page.getByTestId(`marker-${p.id}`)).toBeAttached();
    }
  }), RUNS_UI);
});

test('WORLD-2: hover card matches scene data', async ({ page }) => {
  const w = { from: -1446, to: -1406 };                    // exodus window: rich scene
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  await fcAssert(fc.asyncProperty(
    fc.integer({ min: 0, max: scene.places.length - 1 }), async i => {
      const p = scene.places[i];
      await page.getByTestId(`marker-${p.id}`).hover({ force: true });
      const card = page.getByTestId('place-card');
      await expect(card).toBeVisible();
      await expect(page.getByTestId('place-card-title')).toHaveText(p.name);
      const groups = new Map<string, number>();
      for (const e of p.events) for (const g of e.verse_groups) {
        groups.set(`${g.book}-${g.chapter}`, (groups.get(`${g.book}-${g.chapter}`) ?? 0) + g.count);
      }
      for (const [k, count] of groups) {
        await expect(card.getByTestId(`verse-group-${k}`)).toContainText(String(count));
      }
      await page.mouse.move(0, 0);
      await expect(card).toBeHidden();
    }), RUNS_UI);
});
```

- [ ] **Step 2: Run to verify failure** — `npx playwright test world-map` → FAIL (no markers).
- [ ] **Step 3: Implement** `map.js` (~150 lines: map init, tile fallback, marker diffing keyed by place id, event wiring), `MapInterop.cs` (`IJSObjectReference` import of `./js/map.js`, `DotNetObjectReference` sink with `[JSInvokable]` bridge class), `World.razor` (owns `Scene`, calls `SetScene` after fetch + `FitScene` on first scene, hover state → `PlaceCard` positioned at screen x/y), CSS (`.atlas-marker` glowing dot: radial gradient + `box-shadow` scaled by `.glow-1`…`.glow-5`; `.atlas-label` small text-shadowed label).

Note: per-place `verse-group-{BOOK}-{chapter}` totals in the card must merge counts across the place's events (the test above merges; do the same in `PlaceCard`).
- [ ] **Step 4: Run to verify pass** — `npx playwright test world-map world-slider api-` → all PASS (slider + API suites must stay green).
- [ ] **Step 5: Commit** — `git add -A; git commit -m "feat(world): leaflet interop, glowing markers, hover place cards"`

---

### Task 12: Arrows + legend + isolate (WORLD-3, WORLD-4)

**Files:**
- Modify: `client/wwwroot/js/map.js` (arrow layer), `client/Pages/World.razor` (legend), create `client/Components/Legend.razor`
- Test: `tests/ux/world-arrows.spec.ts`

**Interfaces:**
- Consumes: `SceneArrow`s in `setScene`; CONTRACT arrow/legend testids.
- Produces:
  - In `map.js`: a custom `L.Layer` subclass `ArrowLayer` managing one SVG in `overlayPane`; for each arrow: quadratic bezier `M fx,fy Q cx,cy tx,ty` where control point = midpoint + perpendicular offset `0.18 * dist + 14 * parallelIndex` (parallelIndex = position among arrows sharing the same unordered place pair, centered: 0, +1, −1, +2…); `<defs>` holds one `<marker id="ah-{colorhex}">` triangle per distinct color; each path: `data-testid="arrow-{narrative}-{order}"`, `stroke={color}`, `fill=none`, `stroke-width=2.5`, `marker-end=url(#ah-{colorhex})`, `data-faded="false"`; recomputed on `zoomend`/`moveend`; path `mouseover`/`click` → `OnArrowHover/OnArrowClick` with key `"{narrative}:{order}"`
  - `setIsolate(id, narrativeId | null)` export: sets `data-faded="true"` on every path whose narrative ≠ argument (null clears all to `"false"`)
  - `Legend.razor` params: `List<SceneNarrative> Narratives`, `EventCallback<string?> OnIsolate` — buttons per CONTRACT with `aria-pressed`; World holds `isolated` state, second click on same narrative clears
  - World renders `arrow-tip` (absolutely positioned at the hover x/y): text `"{narrative name}: {from place name} → {to place name}"`, shown on `OnArrowHover`, cleared on the next `OnPlaceLeave`-style mouseout callback (`OnArrowLeave` — add it to `IMapEvents` and `map.js`)

- [ ] **Step 1: Failing UX properties** `tests/ux/world-arrows.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import { api } from './lib/api';

const WINDOWS = [ { from: -1446, to: -1406 }, { from: -2100, to: -2085 }, { from: 46, to: 48 } ];

test('WORLD-3: rendered arrows equal API arrows with correct stroke and arrowheads', async ({ page }) => {
  for (const w of WINDOWS) {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId('arrows-svg').locator('path[data-testid^="arrow-"]'))
      .toHaveCount(scene.arrows.length);
    for (const a of scene.arrows) {
      const path = page.getByTestId(`arrow-${a.narrative}-${a.order}`);
      await expect(path).toHaveAttribute('stroke', a.color);
      await expect(path).toHaveAttribute('marker-end', /url\(/);
      await expect(path).toHaveAttribute('data-faded', 'false');
    }
  }
});

test('WORLD-4: legend isolate fades exactly the other narratives, toggles back', async ({ page }) => {
  const w = WINDOWS[0];
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  if (scene.narratives.length === 0) test.skip();
  const target = scene.narratives[0].id;
  await page.getByTestId(`legend-item-${target}`).click();
  for (const a of scene.arrows) {
    await expect(page.getByTestId(`arrow-${a.narrative}-${a.order}`))
      .toHaveAttribute('data-faded', a.narrative === target ? 'false' : 'true');
  }
  await page.getByTestId(`legend-item-${target}`).click();
  for (const a of scene.arrows) {
    await expect(page.getByTestId(`arrow-${a.narrative}-${a.order}`)).toHaveAttribute('data-faded', 'false');
  }
});

test('arrow hover shows the narrative tooltip', async ({ page }) => {
  const w = WINDOWS[0];
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  if (scene.arrows.length === 0) test.skip();
  const a = scene.arrows[0];
  await page.getByTestId(`arrow-${a.narrative}-${a.order}`).hover({ force: true });
  const name = scene.narratives.find((n: any) => n.id === a.narrative)!.name;
  await expect(page.getByTestId('arrow-tip')).toContainText(name);
});
```

- [ ] **Step 2: Run to verify failure** → FAIL (no arrows-svg).
- [ ] **Step 3: Implement** ArrowLayer + defs + isolate in `map.js`; `Legend.razor`; wire isolate through `MapInterop` (`Task SetIsolate(string? narrativeId)`); CSS: `path[data-faded="true"] { opacity: 0.12; }` inside the injected SVG via attribute selector in `app.css` (SVG lives in page DOM, page CSS applies).
- [ ] **Step 4: Run to verify pass** — `npx playwright test world-` → PASS.
- [ ] **Step 5: Commit** — `git add -A; git commit -m "feat(world): narrative bezier arrows with arrowheads, legend isolate"`

---

### Task 13: ScripturePicker + mode override + world deep links (WORLD-6, NAV-1 rest)

**Files:**
- Create: `client/Components/ScripturePicker.razor`
- Modify: `client/Pages/World.razor`
- Test: `tests/ux/world-scripture.spec.ts`

**Interfaces:**
- Consumes: `AtlasClient.Books()`; CONTRACT `picker-*`, `mode-chip`, `mode-chip-return` testids; `/world?ref=`.
- Produces: `ScripturePicker.razor` params `EventCallback<string> OnApply` — selects book (`picker-book` `<select>` of 66), chapter (`picker-chapter` `<select>` sized from TOC), optional verse range (`picker-verse-from`, `picker-verse-to` numeric inputs bounded by TOC), `picker-apply` button composes the canonical ref (book-only / chapter / verse / passage). World: `ref` query param → scripture mode (fetch `SceneScripture`, slider gets `aria-disabled="true"` and ignores input, `mode-chip` shows ref, `mode-chip-return` restores the previous `from/to` URL — the last time-mode window kept in component state, defaulting to the whole span).

- [ ] **Step 1: Failing UX properties** `tests/ux/world-scripture.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbChapterRef } from './lib/canon';
import { formatRange } from './lib/years';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-6: dropdown override and return-to-time', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    const sref = `${c.book}.${c.chapter}`;
    await page.goto('/world?from=-1446&to=-1406');
    await page.getByTestId('picker-book').selectOption(c.book);
    await page.getByTestId('picker-chapter').selectOption(String(c.chapter));
    await page.getByTestId('picker-apply').click();
    await page.waitForURL(u => u.searchParams.get('ref') === sref);
    await expect(page.getByTestId('mode-chip')).toContainText(sref);
    await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
    const scene = await api.sceneScripture(sref);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
    await page.getByTestId('mode-chip-return').click();
    await page.waitForURL(u => u.searchParams.get('from') === '-1446'
                            && u.searchParams.get('to') === '-1406');
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1446, -1406));
  }), RUNS_UI);
});

test('NAV-1 (world/ref): scripture deep link survives reload', async ({ page }) => {
  await page.goto('/world?ref=EXO.14');
  const scene = await api.sceneScripture('EXO.14');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  await page.reload();
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});
```

- [ ] **Step 2: Run to verify failure** → FAIL.
- [ ] **Step 3: Implement** picker + World mode state machine (time ↔ scripture, previous-window memory).
- [ ] **Step 4: Run to verify pass** — `npx playwright test world-` → PASS.
- [ ] **Step 5: Commit** — `git add -A; git commit -m "feat(world): scripture picker override with return-to-time"`

---

### Task 14: Reader + ExplorerPopover + cross-references (READ-1..3, NAV-1 reader)

**Files:**
- Create: `client/Explore/IExplorable.cs`, `client/Explore/ExplorationTarget.cs`, `client/Explore/VerseNode.cs`, `client/Explore/PassageNode.cs`, `client/Explore/ChapterNode.cs`, `client/Explore/BookNode.cs`, `client/Explore/PlaceNode.cs`, `client/Explore/TimeAndPlaceNode.cs`, `client/Explore/AuthorNode.cs`, `client/Components/ExplorerPopover.razor`
- Modify: `client/Pages/Reader.razor`
- Test: `tests/ux/reader.spec.ts`

**Interfaces:**
- Consumes: `AtlasClient`; CONTRACT reader/popover testids.
- Produces:
  - `interface IExplorable { string Title { get; } string Kind { get; } Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api); Task<RenderFragment> BodyAsync(AtlasClient api); }`
  - `record Exploration(string Label, string ChipTestId, ExplorationTarget Target);` `abstract record ExplorationTarget { record Push(IExplorable Next) : ExplorationTarget; record ShowMiniMap(string SRef) : ExplorationTarget; record NavigateWorld(string Query) : ExplorationTarget; record NavigateReader(string Book, int Chapter, int? Verse) : ExplorationTarget; }`
  - `VerseNode(string vref)`: Title = vref, Kind = "Verse", body = verse text; explorations: `("Cross-references","popover-chip-xrefs", Push(CrossRefsListNode))` — implement the list inline: ExploreAsync returns the four chips, and `ExplorerPopover` renders chip `popover-chip-xrefs` by expanding, on click, a list of `xref-item-{TARGET}` rows from `VerseDetail.CrossRefs` (each row click = `Push(new VerseNode(firstVerseOf(target)))`), `("Explore geo-temporally","popover-chip-map", ShowMiniMap(vref))`, `("About this book","popover-chip-book", Push(new AuthorNode(bookCode)))`, `("Read in context","popover-chip-context", NavigateReader(book, chapter, verse))`
  - `AuthorNode(bookCode)`: body = author, write place/years (`YearText`) from `VerseDetail.BookMeta` (fetch via any verse of the book — use `{book}.1.1`); explorations: `NavigateWorld` when `WritePlace != null`
  - `ExplorerPopover.razor`: parameter `IExplorable Root`; maintains `Stack<IExplorable>`; renders `popover`, `popover-title` (= current.Title), body, chips, `popover-breadcrumb-back` (pops; hidden at root); MiniMap area appears when a `ShowMiniMap` chip is active (Task 15 fills the map; this task renders the container `mini-map` empty)
  - `Reader.razor`: routes `/` (defaults GEN 1) and `/read/{Book}/{Chapter:int}`; renders `verse-line-{n}` rows (`verse-num-{n}` + text + `verse-explore-{n}` affordance shown on row hover via CSS `:hover`); clicking num or affordance opens `ExplorerPopover` with `VerseNode`; `reader-prev`/`reader-next` navigate chapters (crossing book boundaries via TOC); `#v{n}` anchor scrolls; ScripturePicker in header applies `NavigateReader`

- [ ] **Step 1: Failing UX properties** `tests/ux/reader.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbChapterRef, arbVerseRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('READ-1 + NAV-1: chapter deep links render exactly the TOC verses', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    await page.goto(`/read/${c.book}/${c.chapter}`);
    await expect(page.getByTestId(/^verse-line-/)).toHaveCount(c.verses);
    await expect(page.getByTestId('verse-num-1')).toHaveText('1');
    await expect(page.getByTestId(`verse-num-${c.verses}`)).toHaveText(String(c.verses));
  }), RUNS_UI);
});

test('READ-2: verse popover shows the API text', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(vref);
    const detail = await api.verse(vref);
    await expect(page.getByTestId('popover')).toContainText(detail.text.slice(0, 40));
  }), RUNS_UI);
});

test('READ-3: cross-ref chains push and pop breadcrumbs faithfully', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), fc.array(fc.nat(4), { maxLength: 3 }), async (vref, picks) => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    const titles = [vref];
    for (const pick of picks) {
      await page.getByTestId('popover-chip-xrefs').click();
      const items = page.getByTestId(/^xref-item-/);
      const n = await items.count();
      if (n === 0) break;
      const detail = await api.verse(titles[titles.length - 1]);
      for (let i = 0; i < Math.min(n, detail.cross_refs.length); i++) {   // list order == API order
        await expect(items.nth(i)).toContainText(detail.cross_refs[i].target);
      }
      const chosen = Math.min(pick, n - 1);
      const target = detail.cross_refs[chosen].target;
      await items.nth(chosen).click();
      const head = target.match(/^[A-Z0-9]{3}\.\d+\.\d+/)![0];
      await expect(page.getByTestId('popover-title')).toHaveText(head);
      titles.push(head);
    }
    while (titles.length > 1) {                                            // walk back restores each title
      await page.getByTestId('popover-breadcrumb-back').click();
      titles.pop();
      await expect(page.getByTestId('popover-title')).toHaveText(titles[titles.length - 1]);
    }
  }), RUNS_UI);
});
```

- [ ] **Step 2: Run to verify failure** → FAIL.
- [ ] **Step 3: Implement** the Explore hierarchy, `ExplorerPopover`, `Reader.razor` per Interfaces. Keep node body rendering simple `RenderFragment`s (builder lambdas).
- [ ] **Step 4: Run to verify pass** — `npx playwright test reader` → PASS; re-run `world-` and `api-` suites → still green.
- [ ] **Step 5: Commit** — `git add -A; git commit -m "feat(reader): kjv reader with explorable popovers and cross-reference chains"`

---

### Task 15: MiniWorld + passage selection + world↔reader hand-off (READ-4, READ-5)

**Files:**
- Create: `client/Components/MiniWorld.razor`
- Modify: `client/Components/ExplorerPopover.razor`, `client/Pages/Reader.razor`, `client/Pages/World.razor` (place-card verse rows open VerseNode popovers — closing the loop)
- Test: `tests/ux/reader-map.spec.ts`

**Interfaces:**
- Consumes: `MapInterop` (`mini: true`), scenes, CONTRACT `mini-map`, `mini-map-open-world`, `passage-chip`.
- Produces: `MiniWorld.razor` params `string SRef` — fetches `SceneScripture(SRef)`, renders a 320×240 map (`mini-map` testid on container), `FitScene` after set, and `mini-map-open-world` link → `/world?ref={SRef}`. Reader passage selection: click = anchor; shift-click another `verse-num` = range → floating `passage-chip` labeled with the canonical passage ref; clicking the chip opens `ExplorerPopover` with `PassageNode` (Title = passage ref; body = concatenated verses; chips: map/context like VerseNode with `SRef` = the passage). World's `PlaceCard` verse-group rows become clickable → `ExplorerPopover` with `VerseNode` (first verse of the group), pinned over the map. `place-card-title` click → `ExplorerPopover` with `PlaceNode` (Title = place name; body via `/api/place/{id}`: its events across all time, each row a `TimeAndPlaceNode` — Title = `"{place name}, {YearText range}"`, body = that event's verse list, exploration `NavigateWorld` to that window).

- [ ] **Step 1: Failing UX properties** `tests/ux/reader-map.spec.ts`:

```ts
import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('READ-4: mini-map equals scripture scene; open-in-world carries the ref', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    await page.getByTestId('popover-chip-map').click();
    await expect(page.getByTestId('mini-map')).toBeVisible();
    const scene = await api.sceneScripture(vref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
    await page.getByTestId('mini-map-open-world').click();
    await page.waitForURL(u => u.pathname === '/world' && u.searchParams.get('ref') === vref);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  }), RUNS_UI);
});

test('WORLD-8: place card title opens place history popover', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1406');
  const scene = await api.sceneTime(-1446, -1406);
  const p = scene.places[0];
  await page.getByTestId(`marker-${p.id}`).hover({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover-title')).toHaveText(p.name);
  const detail = await api.place(p.id);
  await expect(page.getByTestId('popover'))
    .toContainText(String(Math.abs(detail.events[0].when.from_year)));
});

test('READ-5: shift-click passage selection', async ({ page }) => {
  const toc = await loadToc();
  const arb = arbChapterRef(toc).filter(c => c.verses >= 4).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses - 1 }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b).map(([a, b]) => ({ ...c, a, b })));
  await fcAssert(fc.asyncProperty(arb, async s => {
    await page.goto(`/read/${s.book}/${s.chapter}`);
    await page.getByTestId(`verse-num-${s.a}`).click();
    await page.keyboard.down('Shift');
    await page.getByTestId(`verse-num-${s.b}`).click();
    await page.keyboard.up('Shift');
    const pref = `${s.book}.${s.chapter}.${s.a}-${s.b}`;
    await expect(page.getByTestId('passage-chip')).toContainText(pref);
    await page.getByTestId('passage-chip').click();
    await expect(page.getByTestId('popover-title')).toHaveText(pref);
    await page.getByTestId('popover-chip-map').click();
    const scene = await api.sceneScripture(pref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
  }), RUNS_UI);
});
```

Note READ-2/READ-5 interplay: a bare click on `verse-num` opens the verse popover AND sets the passage anchor; shift-click extends. Implement: plain click closes any popover, opens verse popover, sets anchor; shift-click suppresses popover and shows the chip.

- [ ] **Step 2: Run to verify failure** → FAIL.
- [ ] **Step 3: Implement** `MiniWorld`, passage selection state in Reader, PlaceCard row → popover in World.
- [ ] **Step 4: Run FULL suite to verify pass** — `npx playwright test` → every spec green.
- [ ] **Step 5: Commit** — `git add -A; git commit -m "feat: mini world maps, passage selection, world card exploration"`

---

### Task 16: Remaining narratives, attribution, publish path, full exhaustive run

**Files:**
- Create: `data/curated/narratives/{jacob-journeys,joseph-to-egypt,conquest,david-flight,elijah,exile,return,jesus-ministry,passion-week,paul-rome-voyage}.toml` + additions to `data/curated/events-extra.toml`
- Modify: `client/Layout/MainLayout.razor` (attribution content), `README.md` (create: run instructions)

**Interfaces:**
- Consumes: everything.
- Produces: 13 narratives compiled; a publishable single-binary deployment; the full suite green at deep run counts.

- [ ] **Step 1: Curate the remaining ten narratives** — transcribe these leg tables into `events-extra.toml` events (same schema as Task 5; event id prefix per narrative; label from the episode; anchor place first) and one TOML per narrative (colors below are final):

| Narrative (id, color) | Legs: place @ year — verses |
|---|---|
| `jacob-journeys` #A16207 | beersheba@-1930 GEN.28.10 · bethel@-1930 GEN.28.11-19 · haran@-1929 GEN.29.1-14 · mahanaim@-1910 GEN.32.1-2 · peniel@-1910 GEN.32.22-31 · shechem@-1909 GEN.33.18 · bethel@-1900 GEN.35.1-7 · hebron@-1898 GEN.35.27 |
| `joseph-to-egypt` #65A30D | hebron@-1898 GEN.37.14 · dothan@-1898 GEN.37.17-28 · egypt@-1897 GEN.37.36,GEN.39.1 · egypt@-1885 GEN.41.41-46 · beersheba@-1876 GEN.46.1-5 · goshen@-1876 GEN.46.28-34,GEN.47.27 |
| `conquest` #7C3AED | shittim@-1406 JOS.2.1 · gilgal@-1406 JOS.4.19-24 · jericho@-1406 JOS.6.1-27 · ai@-1406 JOS.8.1-29 · gibeon@-1405 JOS.10.6-14 · hazor@-1400 JOS.11.10-11 · shiloh@-1399 JOS.18.1 |
| `david-flight` #2563EB | ramah@-1014 1SA.19.18 · nob@-1013 1SA.21.1-9 · gath@-1013 1SA.21.10-15 · adullam@-1013 1SA.22.1-2 · keilah@-1012 1SA.23.1-13 · ziph@-1012 1SA.23.14-24 · en-gedi@-1011 1SA.24.1-22 · ziklag@-1010 1SA.27.5-7 · hebron@-1010 2SA.2.1-4 |
| `elijah` #0D9488 | tishbe@-870 1KI.17.1 · cherith@-870 1KI.17.2-7 · zarephath@-869 1KI.17.8-24 · carmel@-866 1KI.18.20-40 · jezreel@-866 1KI.18.45-46 · beersheba@-865 1KI.19.3 · horeb@-865 1KI.19.8-18 · abel-meholah@-864 1KI.19.19-21 |
| `exile` #64748B | jerusalem@-586 2KI.25.1-10 · riblah@-586 2KI.25.6-7,2KI.25.20-21 · babylon@-586 2KI.25.11,PSA.137.1 · mizpah@-586 2KI.25.22-25 · tahpanhes@-585 JER.43.5-7 |
| `return` #0891B2 | babylon@-538 EZR.1.1-4 · jerusalem@-537 EZR.2.1,EZR.3.1-2 · jerusalem@-516 EZR.6.14-15 · susa@-445 NEH.1.1 · jerusalem@-444 NEH.2.11,NEH.6.15 |
| `jesus-ministry` #DB2777 | bethlehem@-5 LUK.2.4-7 · egypt@-4 MAT.2.13-15 · nazareth@-4 MAT.2.19-23 · jordan-river@26 MAT.3.13-17 · cana@27 JHN.2.1-11 · capernaum@27 MAT.4.13 · sychar@27 JHN.4.4-26 · bethsaida@29 LUK.9.10-17 · caesarea-philippi@29 MAT.16.13-20 · jericho@30 LUK.19.1-10 · bethany@30 JHN.12.1 |
| `passion-week` #9F1239 | bethany@30 JHN.12.1-11 · jerusalem@30 MAT.21.1-11 · gethsemane@30 MAT.26.36-56 · golgotha@30 JHN.19.16-30 · jerusalem@30 MAT.28.1-10 · emmaus@30 LUK.24.13-35 · galilee@30 MAT.28.16-20 · mount-of-olives@30 ACT.1.9-12 |
| `paul-rome-voyage` #4338CA | caesarea@59 ACT.27.1-2 · sidon@59 ACT.27.3 · myra@59 ACT.27.5-6 · fair-havens@59 ACT.27.8 · malta@59 ACT.28.1 · syracuse@60 ACT.28.12 · rhegium@60 ACT.28.13 · puteoli@60 ACT.28.13-14 · rome@60 ACT.28.16 |

Same-year legs are ordered by their leg position (the validator only requires non-decreasing `from_year`). Passion-week legs share year 30 — order carries the story. Where a slug doesn't exist in `places.json` (likely: `tishbe`, `cherith`, `fair-havens`, `golgotha`), the ETL error lists it — substitute the dataset's nearest real slug (e.g. `golgotha`→`jerusalem` merges a leg: then adjacent same-place legs skip per §7.4, which is correct) and note substitutions in the narrative file as `# comment`.

- [ ] **Step 2: Recompile + full suite** — `cargo run -p atlas-etl` → 0 errors, report shows 13 narratives. `cargo test` (workspace) → green. `npx playwright test` → green.
- [ ] **Step 3: Attribution + README** — `attribution` popup in footer credits: KJV (public domain), OpenBible.info geocoding & cross-references (CC-BY 4.0, openbible.info), Theographic Bible Metadata (CC BY-SA 4.0, Robert Rouse), Leaflet, Esri World Shaded Relief tiles / Carto. `README.md`: prerequisites, `data/fetch-raw.ps1`, `cargo run -p atlas-etl`, dev loop commands, test commands (`cargo test`, `dotnet test client.Tests`, `npx playwright test` in `tests/ux/`), deep-run (`$env:FC_NUM_RUNS=500; npx playwright test`).
- [ ] **Step 4: Publish path proof** — `dotnet publish client -c Release -o publish` then `cargo run -p atlas-server -- --data-dir ../data/compiled --static-dir ../publish/wwwroot`; browse `http://localhost:8000/world?from=-1446&to=-1406` and `http://localhost:8000/read/EXO/14`. Verify manually: markers render, hover cards open, arrows draw, reader shows the chapter — all served same-origin from the one binary (`ApiBase` is empty in production config). Stop the server. (The automated suite stays pointed at the dev pair; the publish path gets this manual gate.)
- [ ] **Step 5: Deep exhaustive run** — `$env:FC_NUM_RUNS = 500` then `npx playwright test` (expect several minutes; all green). Record the run count and duration in README's testing section.
- [ ] **Step 6: Final commit**

```powershell
git add -A; git commit -m "feat: full narrative set, attribution, publish path, deep property run"
```

---

## Plan Self-Review (performed at write time)

- **Spec coverage:** §3 architecture → Tasks 1, 6, 16(publish); §4.1–4.6 → Tasks 2–5; §5 API → Task 6 (+9 as its verifier); §6.1 Explorable → Task 14; §6.2 components → Tasks 10–15; §6.3 interop → Tasks 11–12; §7.1 slider → Task 10; §7.2 modes/deep links → Tasks 10, 13; §7.3 hover/legend → Tasks 11–12; §7.4 arrows → Tasks 3, 9, 12; §8 reader → Tasks 14–15; §9 quality → Tasks 2–4 (Rust/proptest), 7, 10 (xunit), 8–9 + per-feature specs (UX suite), 16 (deep run); §10 exclusions respected (no translations beyond KJV, no people exploration, no MapLibre).
- **Known deliberate simplifications:** scripture-mode geocoding-only places surface as synthetic `mention-*` pseudo-events (documented in CONTRACT.md; excluded from arrow checks). Author map filters stay out of scope per spec §10 — `AuthorNode` links to write-place only.
- **Type consistency:** wire names snake_case end-to-end; `sref`↔`"ref"` rename appears in Task 2 (Rust), Task 7 (C# `Ref` + SnakeCaseLower), Task 9 (`s.ref`); scene shapes in Tasks 2/3/7/9/11 use identical field names; `map.js` export names match `MapInterop` calls (`init`, `setScene`, `fitScene`, `setIsolate`, `destroy`).
