# Bible Atlas UX Contract

Any implementation of the Bible Atlas UI MUST expose the surfaces below.
The UX property suite couples ONLY to this contract (plus the HTTP API).

## URL patterns
- `/` — reader, defaults to GEN 1
- `/read/{BOOK}/{chapter}` — reader deep link (BOOK = canonical 3-letter code)
- `/read/{BOOK}/{chapter}#v{n}` — verse anchor
- `/world?from={year}&to={year}` — time mode (signed years, no zero)
- `/world?ref={REF}` — scripture mode (canonical ref)
- `/world` (no `from`/`to`/`ref` at all) — defaults to the `gospels` era's
  exact window (`[-5, 29]`, see `data/curated/eras.toml`)

## Displayed text formats
- Year: `1447 BC` or `AD 30`
- Range: `1447 BC – 1400 BC` (spaced en dash U+2013); single year shown as the year alone
- Canonical refs: `GEN`, `GEN.1`, `GEN.1.1`, `GEN.1.1-5`

## data-testid inventory
Header: `nav-reader`, `nav-world`, `translation-select`, `attribution`
World: `world-map`, `marker-{placeId}`, `quiet-marker-{placeId}` (batch-e2-brief.md,
  "the ever-present graph": one per currently QUIET place -- an event-bearing place not
  lit this window, see QUIET-1 below -- distinct from and never overlapping
  `marker-{placeId}`'s own ids for the same scene, per QUIET-1's disjointness; a small,
  unlit, non-glowing dot, hoverable/clickable exactly like a lit marker; absent entirely
  in scripture mode and on mini-maps), `place-card`, `place-card-title`,
  `hover-verse-{VREF}` (one element per currently shown verse, VREF = canonical id e.g.
  `EXO.14.21`, whether it renders as its own lone-verse row or as one verse inside a
  passage block; element text contains that verse's own KJV text verbatim -- never
  trimmed, never paraphrased), `hover-passage-{SPAN}` (one per currently shown passage
  block -- a maximal run of >=2 consecutive same-book/chapter verses; SPAN = canonical
  span text of that run's CURRENTLY SHOWN extent, e.g. `GEN.12.1-4`; contains that
  block's own `hover-verse-{VREF}` elements), `place-card-more` (button; present only
  while unshown verses remain for this place), `place-card-collapse` (button; present
  only while more than the initial content is shown),
  `place-card-blurb` (batch-e-brief.md; present only when the API's `history.blurb`
  is non-null for the card's place under the scene's own window -- text is that
  blurb's text verbatim), `place-card-dates` (present only when the place has a
  curated `established` and/or `destroyed` date; wraps one or both of
  `place-card-date-established` / `place-card-date-destroyed`, each a button whose
  text contains that date's own formatted year/range and which opens the
  ExplorerPopover, listing the curated supporting verses first, on click),
  `place-card-quiet` (batch-e2-brief.md; present ONLY when the card's place has no
  events in the scene's own window -- i.e. it was opened from a `quiet-marker-{id}`,
  never from a lit `marker-{id}` -- text is exactly "No recorded events in this window
  — drag the timeline."; mutually exclusive with every `hover-verse-{VREF}`/
  `hover-passage-{SPAN}`/`place-card-more`/`place-card-collapse` on the same card, which
  never appear together with it),
  `arrows-svg`, `arrow-{narrativeId}-{order}` (SVG path; attr `stroke` = narrative color;
  attr `data-faded` = "true"|"false"; `marker-end` set),
  `legend`, `legend-item-{narrativeId}` (button; `aria-pressed` = isolated),
  `slider`, `slider-readout` (an `<input>`, accepts typed year/range text, Enter applies),
  `slider-era-{eraId}` (clickable era label; attr `data-active` = "true" when the
  currently-applied window overlaps this era's own `[from_year, to_year]`, "false"
  otherwise -- clicking an era band applies that era's own exact range, which, because
  era ranges are contiguous and non-overlapping by construction, always leaves exactly
  that one era's `data-active` "true" and every other era's "false"),
  `mode-chip` (text contains active ref),
  `mode-chip-return`,
  `arrow-tip` (visible while an arrow is hovered; text contains the narrative name),
  `toast` (non-blocking error notice; last good scene stays rendered beneath it),
  `border-tag` (visible in time mode when a border snapshot is loaded; text contains
  "Borders c."; hidden in scripture mode and whenever no snapshot is loaded),
  `landmark-{slug}` (always-visible, non-interactive landmark label; slug = lowercase
  kebab-case of the landmark's name, e.g. "Mount Sinai" -> "mount-sinai"),
  `polity-label-{slug}` (non-interactive polity-name label rendered from the
  active border snapshot's own features; slug = lowercase kebab-case of the
  feature's `name`, same rule as `landmark-{slug}`; visible in time mode
  only, subject to its own per-feature zoom/viewport visibility rule --
  absent entirely whenever no border snapshot is loaded, e.g. scripture mode)
Picker (ScripturePicker, shared by world and reader):
  `picker-book` (select of 66 books), `picker-chapter` (select sized from TOC),
  `picker-verse-from`, `picker-verse-to` (numeric inputs bounded by TOC),
  `picker-apply` (button; composes the canonical ref)
Reader: `reader-root`, `verse-line-{n}`, `verse-num-{n}`, `verse-explore-{n}`,
  `reader-prev`, `reader-next`, `passage-chip`
Popover (shared): `popover`, `popover-title`, `popover-breadcrumb-back`,
  `popover-chip-xrefs`, `popover-chip-map`, `popover-chip-book`, `popover-chip-context`,
  `popover-chip-verse-{VREF}` (batch-e-brief.md; one per a `YearNode`'s own curated
  supporting verses, in curated order, ALWAYS rendered before that same node's
  `popover-chip-map` chip -- DATE-1: opening a date's popover lists its supporting
  verses first),
  `xref-item-{TARGET}` (TARGET = canonical ref/span text), `mini-map`, `mini-map-open-world`
Notes:
- `marker-{placeId}` elements carry the visible place label -- batch-e-brief.md:
  this is the scene's own `display_name` (the period name resolved for the
  scene's current window when the place has curated history and one of its
  name ranges intersects that window, else its plain default name), not
  always the place's plain default name. `place-card-title` and the
  `arrow-tip` text (`{narrative}: {fromName} -> {toName}`) use the SAME
  `display_name`, so a place's name is never shown two different ways at
  once within one scene.
- NAME-1 (batch-e-brief.md): for a time-mode window fully inside one curated
  name range, `marker-{placeId}`'s label and `place-card-title` both equal
  that name; a window crossing the boundary between two curated ranges
  shows whichever one covers the window's own midpoint (or, failing that,
  the later-starting one it still intersects); a window matching no curated
  range falls back to the place's plain default name. Scripture mode always
  shows the plain default name (no curated period name is ever resolved
  there -- there is no time window to resolve one against). A place's plain
  default name is ALWAYS stripped of a trailing ETL slug-disambiguation
  numeral first (batch-e2-brief.md fold-in: "Beersheba 2" displays as
  "Beersheba", never the raw suffixed source name) -- this only ever affects
  the DEFAULT-name fallback; a curated name (already hand-written, never
  suffixed) is untouched. Two places sharing a stripped default name may
  therefore show identical labels at once (their ids stay distinct) --
  correct cartography, not a collision bug.
- Every curated place-history range (a `[[place.name]]`, a `[[place.blurb]]`,
  or `established`/`destroyed`'s own `when`) is INCLUSIVE on both ends --
  its own `from`/`to` year is itself covered, matching `TimeRange`'s general
  convention throughout this app (`slider-era-{eraId}`'s own `data-active`
  rule already relies on the same inclusive-both-ends reading). A curated
  range whose text names a specific year (e.g. a destruction date) must
  therefore reach that exact year, not stop one short of it (fix round 1,
  M1: Jerusalem's own destruction-year blurb had exactly this off-by-one).
- BLURB-1 (batch-e-brief.md): `place-card-blurb` shows at most one blurb,
  never a stack -- a window inside exactly one of a place's own `"era"`-
  breadth ranges shows that blurb; a window spanning more than one of them
  shows a `"broad"`-breadth blurb instead (falling back to an `"era"` pick
  if no `"broad"` blurb is curated); a window inside NEITHER a place's
  `"era"` ranges NOR any `"broad"` one shows no `place-card-blurb` at all.
  A window that touches ZERO `"era"` ranges (a gap between two curated
  eras) but that a `"broad"` range still intersects is NOT the "matches
  nothing" case -- it shows the `"broad"` blurb (fix round 1, M1: this
  branch existed since the first batch-e commits but was undocumented
  here; a window inside such a gap is still, truthfully, inside the
  place's whole history, so the broad summary is shown rather than nothing).
- QUIET-1 (batch-e2-brief.md, "the ever-present graph" -- user direction 2026-08-19:
  "all of the cities in our graph are available in any timerange rather than just
  loading those which are biblically active at the time"): for every time-mode window,
  the lit place set (`marker-{placeId}`) and the quiet place set (`quiet-marker-{placeId}`)
  are DISJOINT and their union is the full, fixed-cardinality set of event-bearing
  places ("cities in our graph" -- every place with >=1 event anywhere in the compiled
  data, 206 places as of this batch, derived from the data rather than hardcoded by
  either side) -- a place is always exactly one of lit or quiet, never both, never
  neither. A quiet place's own displayed name resolves against the SAME window using
  the SAME rules `marker-{placeId}`'s own label does (NAME-1), so a place's name never
  contradicts itself as it crosses from quiet to lit (or back) while a window is
  dragged. Scripture mode has no quiet places at all (`quiet-marker-{placeId}` is
  entirely absent there, same as on a mini-map) -- period relevance without a time
  window has nothing for GLOW to mean.
- Quiet-place hover card (batch-e2-brief.md): hovering a `quiet-marker-{id}` opens the
  exact SAME `place-card` a lit marker does -- same title (`place-card-title` = the
  place's own `display_name`, per NAME-1), same Batch E history content
  (`place-card-blurb`/`place-card-dates`) when curated for this place, same explorable
  `PlaceNode` behind the title. The one content difference is conditional presence:
  `place-card-quiet` replaces the verse content entirely (no `hover-verse-{VREF}`,
  no `hover-passage-{SPAN}`, no `place-card-more`/`place-card-collapse` -- a quiet
  place has no events THIS window to show, so there is nothing for those controls to
  page through). Clicking a marker -- lit OR quiet -- is a no-op today: `OnPlaceClick`
  is an intentional, documented empty handler, unchanged by this batch (WORLD-1/2's own
  original hover-only design). The brief's own wording for this card was conditional --
  "hover (and click/pin, if Batch G1's pinning has landed by your HEAD -- check)" -- and
  G1 had not landed at this batch's own HEAD, so no click/pin behavior was added; that
  half is deferred to Batch G1, whenever it lands.
- Hover place card content (batch-d-brief.md): the card is place name + verse
  content + controls, nothing else -- no per-(book,chapter) count rows, bare
  canonical-ref rows, or chapter-identifier lines anywhere on it. From the
  place's merged, deduped activating verse list (event order, then each
  event's own already book/chapter/verse-ascending groups -- this list's
  long-standing "canonical order"), maximal runs of consecutive same-book/
  chapter verses (n, n+1, ...) are passages (`hover-passage-{SPAN}`, rendered
  as one flowing block); runs of one are lone verses. Initial state shows up
  to the first 4 verses if the first group is a passage, else the first 2
  verses (necessarily non-consecutive with each other, since the first group
  being a lone verse means the very next verse in the list isn't consecutive
  with it) -- only ever the first group (passage) or the first two lone
  verses, never more, never a wall. Each `place-card-more` click reveals the
  next chunk: +5 verses if the next not-yet-shown verse belongs to a
  passage-sized group, +2 if it belongs to a lone verse's group; repeats
  until the place's full (already server-capped) verse list is exhausted, at
  which point `place-card-more` is absent. `place-card-collapse` restores the
  exact initial state (DOM and card size) in one click.
- Scene pseudo-events with ids beginning `mention-` are text-mention markers
  (scripture mode); arrows never reference them.
- The slider is `aria-disabled="true"` while scripture mode is active.
- Magnetic drag-release snap: releasing a dragged slider handle within ~6px of an
  era boundary (either neighboring era's own `from_year`/`to_year`) snaps the
  released value to that boundary year; releasing anywhere else keeps the exact
  pixel-decoded year (a deliberate mid-era release is never pulled to a boundary).
  Snapping is evaluated only at the release point -- never while the handle is
  still being dragged, so the brush visibly tracks the pointer exactly until the
  pointer is lifted.
- Place-card hover persistence (batch-c2-brief.md requirement 0c): `place-card`
  stays open for as long as the pointer is over its own marker OR the card
  itself -- it never closes merely because the pointer crossed the gap between
  them. Once the pointer has left BOTH the marker and the card, the card closes
  within ~1s (an internal ~350ms grace timer started when the pointer leaves the
  second of the two, plus ordinary event latency); re-entering either one before
  that grace elapses cancels the pending close. This holds regardless of which
  order the marker's own leave and the card's own pointer-enter resolve in -- a
  marker `mouseout` that resolves (its own async map/interop round trip) AFTER
  the card's pointer-enter already ran never schedules a close out from under a
  pointer that is legitimately still on the card.
- Quiet-marker hover-intent debounce (batch-e2-brief.md self-review fix): unlike a lit
  `marker-{id}`, which opens its `place-card` immediately on `mouseover`, a
  `quiet-marker-{id}` only opens it once the pointer has DWELLED on that marker for
  >=150ms. A graze shorter than that -- e.g. a pointer merely transiting toward some
  OTHER nearby target, such as that very place's own already-open card -- fires neither
  `OnPlaceHover` nor `OnPlaceLeave`; from the card's own point of view a sub-150ms graze
  never happened at all. This exists specifically to stop a fast pass-through over an
  unrelated quiet dot from silently hijacking an already-open card mid-transit (a real,
  confirmed regression risk once ~200 more small hoverable dots share the plate with
  every lit marker) -- protection a lit marker, being comparatively sparse, has never
  needed and does not get: `marker-{id}` hover timing is completely unaffected by this,
  every existing hover-persistence guarantee above applies to it exactly as before. A
  genuine, deliberate quiet-marker hover clears 150ms comfortably and opens the SAME
  place-card any other hover does (see "Quiet-place hover card" above), just ~150ms
  later than a lit marker's own immediate open.
- Marker hover-target resolution is best-effort, not a per-marker guarantee,
  once two or more markers' own hit areas overlap on screen -- a disclosed
  trade-off of the >=14px hit target every marker carries (batch-c2-brief.md
  requirement 0c), which two or more genuinely different, merely-close-together
  places can exceed at typical/dense zoom (the exodus scene alone measures a
  majority of its own places mutually within this radius -- 75%, 12 of 16;
  other comparably rich scenes measure lower, e.g. 29% for the apostolic
  window and 33% for the -2100..-2085 window, so "a majority" is a property
  of the exodus scene specifically, not a general claim about every rich
  scene -- batch-c2-rereview.md). Which marker a
  pointer at an overlapping pixel resolves to is decided by the browser's own
  hit-testing (Leaflet's default per-marker z-index, keyed off screen position,
  not DOM or testid order) -- not a rule this app controls or a claim any test
  should assume. A marker is only reliably, individually hoverable when no other
  marker's own hit area overlaps it on the screen as currently rendered.
  Resolving this for real marker density (not just test-side mitigation) is
  deferred to a future marker-clustering/decluttering batch.
