# Bible Atlas — Design

**Date:** 2026-08-17
**Status:** Approved section-by-section in brainstorming; this document is the written spec.
**Spec source:** `BibleGraph.hs` (repo root) is the type-level sketch this design realizes. It stays in the repo, untouched, as the conceptual reference.

## 1. Vision

An explorable GUI for the Bible.

- **`/` (homepage):** a KJV reader. Other translations selectable later; KJV only in M1.
- **`/world`:** a Google-Maps-style viewer of the Biblical world with a time slider (single year or year range). Locations relevant in the selected window light up. Hovering a lit location shows the verses that activate it; those verses are themselves explorable.
- **Narrative arrows:** within any time window, colored directed arrows connect lit locations that are narratively linked, earlier → later. Each narrative has exactly one color. Events may belong to multiple narratives.
- **Scripture override:** a top-right dropdown (book → chapter → verse range) overrides the time window and lights the locations for that scripture unit.
- **Explorable everywhere:** every entity — verse, passage, chapter, book, place, time-and-place, author — implements one abstraction: a focused thing that can produce further explorable things (the sketch's `Explorable` bind). Cross-references, mini-map popups, and metadata panels all hang off this.

**Standing assumptions (confirmed):** the map is driven by *narrative* time (where/when events happen); write-time/write-place/author are metadata surfaced when exploring. 66-book KJV canon. Timeline span ≈ 4000 BC → AD 100.

## 2. Decisions log

| Decision | Choice | Notes |
|---|---|---|
| Backend | **Rust** (axum) | Rust is backend-only, per user. Owns all query semantics. |
| Frontend | **C# / Blazor WebAssembly** | WASM frontend; Rust explicitly excluded from frontend. Thin JS shim only at the map boundary. |
| Map library | **Leaflet** via a small JS interop module | DOM markers for robust hover; SVG overlay pane for arrows. MapLibre is a possible later swap behind the same interop contract. |
| Topology | **Hybrid scene-per-window** | One `GET /api/scene` per slider/dropdown change returns everything hover needs; hovering never touches the network. Deep dives fetch on demand. |
| Data | **Open datasets, traditional chronology** | OpenBible.info geocoding + cross-references, Theographic Bible metadata, public-domain KJV. |
| Narratives | **Hand-curated starter set** (~10–15 arcs) | Clean schema; extensible file-per-narrative. |
| Time slider | **Era-segmented, year granularity** | Every year addressable; eras give non-uniform density usable width. |
| M1 scope | **Everything in the vision** | Base + arrows + dropdown override + mini-map popups + cross-references. |
| Deployment | **Local, deploy-ready** | Single Rust binary serves API + published client in release. |
| Rust toolchain | **windows-gnu** if no MSVC present | Self-contained (rust-mingw), no VS Build Tools / UAC. Dependency profile is pure Rust. Switch to MSVC later only if a crate demands it. |
| .NET install | **User-local** via `dotnet-install.ps1`, channel 10.0 → 9.0 → 8.0 fallback | No elevation required. |

## 3. Architecture

```
bible-atlas-sketch/
├── BibleGraph.hs          # the spec, untouched
├── server/                # Rust cargo workspace
│   ├── atlas-core/        # domain types (Rust rendering of the sketch) + query semantics
│   ├── atlas-etl/         # binary: data/raw + data/curated → data/compiled (validated)
│   └── atlas-server/      # axum: /api/* + static file serving of published client
├── client/                # Blazor WebAssembly app (C#)
│   └── wwwroot/js/map.js  # the ONE JS interop module (Leaflet glue)
├── data/
│   ├── raw/               # downloaded datasets (gitignored where large; ETL fetches)
│   ├── curated/           # narratives/*.toml, books.toml, eras.toml
│   └── compiled/          # ETL output; atlas-server loads at startup
└── docs/superpowers/specs/
```

**Runtime shape.** `atlas-server` loads `data/compiled/` into memory at startup and builds indexes: an interval index over years → events, maps place→events, verse→events, verse→cross-refs, and the narrative tables. It answers the sketch's `TimeAndPlaceQuery`/`BibleRefQuery` as HTTP endpoints. Scenes are computed server-side; the client renders and caches them.

**Dev loop.** Two processes: `cargo run -p atlas-server` (API, port 8000) and `dotnet watch` (client dev server, proxying `/api` → 8000). **Release:** `dotnet publish` output is served by atlas-server itself — one binary + assets = deployable.

**Wire types.** Serde structs in `atlas-core`; hand-mirrored C# records in `client/` (M1; the surface is small and stable — codegen is future work).

## 4. Data

### 4.1 Time

Signed astronomical year in an `i32` (`-1446` = 1447 BC; no year zero in display, handled at format time). Display: `1447 BC`, `AD 30`. `TimeRange { from_year, to_year }`, inclusive. Year is the finest granularity everywhere (slider, events, queries).

### 4.2 Core types (atlas-core)

- `PlaceId`, `Place { id, name, lat: f64, lon: f64 }` (sketch's `Location`; its `Integer` lat/lon newtypes were placeholders for real decimal degrees).
- `VerseId { book: BookCode, chapter: u16, verse: u16 }` — canonical string form `EXO.14.21`. `PassageRef { book, chapter, from_verse, to_verse }`. `ScriptureRef = Book | Chapter | Passage | Verse` (sketch's `BibleRef`).
- `Event { id, label, when: TimeRange, places: Vec<PlaceId>, verses: Vec<VerseId> }`.
- `Narrative { id, name, color, legs: Vec<EventId> }` — legs are chronologically ordered; an event may appear in many narratives.
- `Era { id, name, range: TimeRange }`.
- `BookMeta { book, author, write_place: Option<PlaceId>, write_range: Option<TimeRange> }` (sketch's `BibleRefMetadata`; traditional attributions, hand-curated).
- `Translation` — M1 ships `kjv` only, but text storage/API are keyed by translation id.

### 4.3 Sources & licenses

| Dataset | Gives us | License |
|---|---|---|
| Public-domain KJV (JSON) | Verse text, canon structure | Public domain (US) |
| OpenBible.info geocoding | Place ↔ coordinates ↔ verse links (~1,200 places) | CC-BY 4.0 |
| Theographic Bible metadata | Dated events with places + verse links; people; periods (traditional chronology) | CC-BY-SA 4.0 |
| OpenBible.info cross-references | ~340k vote-ranked cross-references | Free use (credit) |
| `data/curated/` (ours) | Narratives, eras, book authorship/write-time | Ours |

An attribution footer in the app credits all of the above plus the tile provider. Exact dataset URLs/formats are resolved at implementation time; if a source's actual shape differs from the assumption above, the ETL adapts (the compiled schema, not the raw one, is the contract).

### 4.4 Activation semantics (load-bearing)

- **Time mode** (slider): place `p` is lit in window `w` iff ∃ event `e` with `p ∈ e.places` and `e.when ∩ w ≠ ∅`. Hover verses = union of such events' verses, grouped by book/chapter. Brightness ∝ number of activating events (bucketed).
- **Scripture mode** (dropdown): place `p` is lit for scripture ref `r` iff the geocoding links some verse in `r` to `p` — no dating required, full coverage. Hover verses = the linked verses within `r`. Arrows in scripture mode: **a leg is drawn iff both endpoint events have ≥1 verse inside `r`**.
- Verses without geocoded places simply activate nothing; ETL reports coverage percentages so gaps are known.

### 4.5 Curated file schemas

`data/curated/narratives/exodus.toml`:

```toml
id = "exodus"
name = "The Exodus and Wilderness Journey"
color = "#C0392B"          # one color per narrative, fixed here
legs = ["ev_burning_bush", "ev_plagues_egypt", "ev_red_sea", "ev_sinai_covenant", "..."]
# legs are event ids in chronological order; events may also appear in other narratives
```

Starter set (~10–15): Abraham's migration, Jacob's journeys, Joseph to Egypt, the Exodus + wilderness, the Conquest, David's flight from Saul, Elijah's circuit, the Exile, the Return, Jesus' ministry, Passion week, Paul's three journeys, Paul's voyage to Rome.

`eras.toml`: Primeval, Patriarchs, Egypt & Exodus, Conquest & Judges, United Kingdom, Divided Kingdom, Exile, Return, Gospels, Early Church — each `{ id, name, from_year, to_year }`, contiguous, covering the full span. `books.toml`: 66 rows of `BookMeta`.

Curated narratives may reference Theographic events or define supplementary events in `data/curated/events-extra.toml` (same `Event` schema) when a needed leg has no Theographic event; ETL merges both, rejecting id collisions.

### 4.6 ETL (atlas-etl)

`cargo run -p atlas-etl` : reads `data/raw/` + `data/curated/` → validates → writes `data/compiled/*.json` (gzip-friendly plain JSON). **Hard failures** (exit non-zero): unknown place id, dangling narrative leg, verse id outside KJV canon, era gaps/overlaps, duplicate event ids, non-chronological narrative legs (leg n+1's `when.from_year` must be ≥ leg n's). **Report** (stdout + `data/compiled/report.txt`): % events dated, % verses geocoded, per-narrative leg counts, places without any event. A `fetch` subcommand downloads raw datasets to `data/raw/` (which is gitignored; `data/curated/` and small derived files are committed).

## 5. API

All under `/api`, JSON, served by axum. Errors: `{ error: { code, message } }` with proper status codes.

- `GET /api/scene?from=-1450&to=-1400` → **Scene**
- `GET /api/scene/scripture?ref=EXO.14` → **Scene** (ref = `EXO` | `EXO.14` | `EXO.14.21` | `EXO.14.21-31`)
- `GET /api/verse/EXO.14.21` → verse text (KJV), cross-refs (vote-ranked, each with preview text), its events with time+place, `BookMeta`, and exploration edges
- `GET /api/chapter/EXO.14?translation=kjv` → ordered verses for the reader
- `GET /api/books` → canon TOC (books, chapter counts, verse counts per chapter) for dropdowns
- `GET /api/place/{id}` → place details + all its events across time (a place's full history)
- `GET /api/narratives` → list with colors and leg summaries (legend, future narrative browser)
- `GET /api/eras` → era definitions for the slider

**Scene shape:**

```jsonc
{
  "mode": "time" | "scripture",
  "window": { "from": -1450, "to": -1400 },        // or "ref": "EXO.14"
  "places": [{
    "id": "jericho", "name": "Jericho", "lat": 31.87, "lon": 35.44,
    "brightness": 3,                                // bucketed activation count
    "events": [{ "id": "ev_jericho_falls", "label": "The walls fall",
                 "when": { "from": -1406, "to": -1406 },
                 "verse_groups": [{ "book": "JOS", "chapter": 6,
                                    "verses": ["JOS.6.1", "…"], "count": 27 }] }]
  }],
  "arrows": [{ "narrative": "conquest", "color": "#8E44AD",
               "from_place": "gilgal", "to_place": "jericho",
               "from_event": "ev_camp_gilgal", "to_event": "ev_jericho_falls",
               "order": 2 }],
  "narratives": [{ "id": "conquest", "name": "The Conquest", "color": "#8E44AD",
                   "legs_in_scene": 4 }]
}
```

A scene stays small (tens of KB): verse lists inside `verse_groups` are capped per group (e.g., first 20 + `count`); the full list comes from `/api/place/{id}` or per-verse fetches when actually expanded.

## 6. Frontend (Blazor WebAssembly)

### 6.1 The Explorable abstraction (the sketch's typeclass, in C#)

```csharp
interface IExplorable {
    string Title { get; }
    ExplorableKind Kind { get; }          // Verse, Passage, Chapter, Book, Place, TimeAndPlace, Author
    Task<IReadOnlyList<Exploration>> ExploreAsync(AtlasClient api);
}
record Exploration(string Label, ExplorationTarget Target);
// ExplorationTarget is a closed hierarchy:
//   Push(IExplorable next)                — go deeper in the popover (breadcrumb grows)
//   ShowMiniMap(SceneRequest scene)       — embedded mini /world
//   NavigateWorld(SceneRequest scene)     — full /world with state preset
//   NavigateReader(ScriptureRef target)   — jump the reader
```

`explore : e a → (a → [e b]) → [e b]` becomes: a focused node yields labelled continuations, each yielding further nodes. The existential `SomeExplorable` is just `IExplorable` (non-generic) — C#'s interface erasure does the job.

**Node types → their explorations:**

| Node | Explorations |
|---|---|
| Verse | Cross-references (→ Verse nodes), Explore geo-temporally (mini-map), About this book (→ Author/BookMeta), Read in context (reader nav) |
| Passage/Chapter/Book | Show on map (scripture-mode scene), Read, About this book |
| Place | Its events across time (→ TimeAndPlace nodes), Show on /world |
| TimeAndPlace | The verses there-and-then (→ Verse nodes), Show on /world |
| Author | Books written (→ Book nodes), write-locations on map |

### 6.2 Components

- `ExplorerPopover` — renders ANY `IExplorable`: header (kind icon + title), body (kind-specific content: verse text, place summary, …), exploration chips, breadcrumb stack with back navigation. Used by hover cards, cross-ref chains, everything.
- `WorldMap` — full /world page: owns the Leaflet interop instance, scene state, slider, dropdown, legend.
- `MiniWorld` — the embedded small map (own Leaflet instance, non-interactive slider readout, "Open in /world" button).
- `TimeSlider` — era-segmented brush (see 7.1).
- `ScripturePicker` — book → chapter → verse-range dropdown (shared by reader and /world).
- `Reader` — chapter view, verse-per-line, hover affordances, passage selection.
- `AtlasClient` — typed HTTP client with an in-memory scene cache (keyed by window/ref; LRU of a few dozen scenes).

### 6.3 Map interop contract (`wwwroot/js/map.js`)

Small, explicit surface; C# calls it via `IJSRuntime`, it calls back via `DotNetObjectReference`:

- `atlasMap.init(el, opts)` / `destroy()` — create Leaflet map, label-free terrain basemap (Esri World Physical raster tiles + attribution; Carto light_nolabels is the fallback provider if those tiles misbehave), pan/zoom.
- `atlasMap.setScene(scene)` — diff-render: glowing circle markers (CSS glow, radius/opacity by `brightness`), our own place labels, arrows as SVG bezier paths in the overlay pane with arrowhead markers, per-narrative color, parallel offset for shared pairs, self-edges skipped.
- `atlasMap.fitScene()` — fly to scene bounds.
- Callbacks to C#: `onPlaceHover(placeId, screenXY)`, `onPlaceLeave()`, `onPlaceClick(placeId)`, `onArrowHover(arrowKey, screenXY)`, `onArrowClick(arrowKey)`.
- Hover *cards themselves* are Blazor-rendered (positioned by screenXY) so `ExplorerPopover` works identically on map and reader.

Everything else — state, caching, popovers, slider, routing — is C# in WASM. This module is the only JS we write.

## 7. /world UX

Full-bleed map; slim app header (Reader ⇄ World links, attribution link).

### 7.1 Time slider (bottom, overlaid)

- One horizontal strip divided into **era segments** (from `eras.toml`); each era gets a minimum usable width regardless of year-span; era names are labels on the strip.
- Within a segment, position ↔ year piecewise-linearly: **every year is addressable** (the mapping is exact; drag snaps to integer years).
- A **two-handle brush** selects `[from, to]`; dragging the handles together (or double-clicking a spot) collapses to a **single-year point**. Clicking an era name selects that era's whole range.
- Readout, editable as text: `1447 BC — 1400 BC` (or a single `1447 BC`).
- Changing the selection debounce-fetches `GET /api/scene` (~150 ms) and re-renders.

### 7.2 Scripture mode

`ScripturePicker` top-right. Choosing a unit fetches `GET /api/scene/scripture?ref=…`; the slider dims with a chip — `Showing EXO.14 · return to time` — that restores the last time window. Deep-linking: `/world?from=-1450&to=-1400` and `/world?ref=EXO.14` are both routable/shareable states.

### 7.3 Hover & explore

- Hover a lit marker → Blazor hover card: place name, activating events in the window (label + years), verse groups by book/chapter (capped with counts). The card persists while the pointer is over marker or card.
- Any verse row / event row / "see all N verses" opens `ExplorerPopover` (pinned; multiple may be open; draggable is a nice-to-have, not M1).
- Hover an arrow → tooltip: narrative name + leg (`Gilgal → Jericho`, leg 2 of 7, years). Click → popover for the narrative leg (both events, explorable).
- **Legend** (collapsible, bottom-right): narratives present in scene with color swatches and `legs_in_scene`; click to **isolate** (others fade), click again to restore.

### 7.4 Arrow semantics (invariants, tested)

Within a scene, for each narrative: take its legs (events) in order, keep those whose event intersects the window (time mode) or has ≥1 verse in the ref (scripture mode) **and** whose anchor place is lit; draw an arrow between each *consecutive kept pair* with distinct places. Consequences (test assertions): earliest kept event = pure source; latest = pure sink; an event in k narratives can carry k differently-colored arrows (parallel offset); same-place consecutive events produce no self-arrow (the chain passes through); one color per narrative always. Multi-place events use their first listed place as the arrow anchor.

## 8. Reader UX (/)

- Chapter at a time, verse-per-line with verse numbers, KJV text; `ScripturePicker` top-right for navigation; prev/next chapter buttons; translation selector visible with KJV enabled only.
- Hovering a verse line reveals a quiet explore affordance (∴ icon at line end); clicking it — or the verse number — opens the verse's `ExplorerPopover` (see 6.1 table: cross-references, mini-map, book metadata, context).
- **Cross-references:** vote-ranked list with preview text; each row is a Verse node — chain as deep as you like, breadcrumb walks back.
- **Mini-map:** `MiniWorld` renders the verse's scene — scripture-mode for that verse, plus its events' time window shown in the readout; *Open in /world* navigates with identical state.
- **Passage selection:** shift-click another verse number to select the inclusive range → floating chip `EXO.14.21–31 · explore` opens a Passage popover (same chips, scripture-mode map of the whole passage).
- Deep link: `/read/EXO/14#21` (book/chapter routable, verse anchors).

## 9. Quality

- **Rust unit tests** (the semantics live here): window intersection incl. boundary years and BC math; scripture-ref parsing/resolution; scene composition; all §7.4 arrow invariants; scripture-mode leg rule; brightness bucketing; cross-ref ordering.
- **ETL tests + report:** validation failures actually fail (fixture-driven); coverage report generated every run.
- **Client:** hand-mirrored DTO serialization round-trip tests; component smoke tests only where cheap (bUnit for `TimeSlider` year↔position math — that mapping is worth pinning).
- **E2E:** verified by driving the running app with Playwright during development (hover cards appear, arrows render, mode override works, cross-ref chain navigates). Scripted CI E2E is future work.
- **Perf budgets:** scene response ≤ ~100 KB typical; hover = zero network; slider drag → ≤ 1 in-flight scene fetch (debounced, latest-wins); Leaflet handles the ≤ ~1,200-marker worst case, typical scenes far smaller.
- **Errors:** server returns typed errors; client shows non-blocking toasts and keeps the last good scene on screen.

## 10. Out of scope for M1 (explicitly later)

Other translations (architecture is ready); people/participant exploration and author-based map filters (sketch's `Author` query is exposed only as book metadata in M1); zoomable-axis timeline upgrade; MapLibre swap; auto-derived narrative backfill; scripted E2E in CI; DTO codegen from Rust; deployment hosting setup; uncertainty ranges on dates beyond what `TimeRange` already expresses.

## 11. Implementation phasing (input to the plan)

1. **Toolchains + skeleton** — installs (running), cargo workspace, Blazor app, `map.js` stub, dev proxy, git hygiene (.gitignore: `target/`, `bin/obj/`, `data/raw/`, `data/compiled/`).
2. **ETL + data** — fetch raw datasets, resolve their real formats, compile, curate eras/books/first 3 narratives; coverage report.
3. **atlas-core + server** — types, indexes, scene composition, endpoints, unit tests.
4. **/world base** — map interop, markers/glow, slider, hover cards.
5. **Arrows + legend + scripture mode + dropdown.**
6. **Reader + ExplorerPopover + cross-refs + mini-map.**
7. **Remaining narratives (to ~10–15), polish, attribution, perf pass, Playwright verification.**
