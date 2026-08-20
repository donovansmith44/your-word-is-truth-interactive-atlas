# Bible Atlas UX Contract

Any implementation of the Bible Atlas UI MUST expose the surfaces below.
The UX property suite couples ONLY to this contract (plus the HTTP API).

## URL patterns
- `/` — reader, defaults to GEN 1
- `/read/{BOOK}/{chapter}` — reader deep link (BOOK = canonical 3-letter code)
- `/read/{BOOK}/{chapter}#v{n}` — verse anchor
- `/read/{BOOK}/{chapter}?split=1` — batch-h-brief.md: lands directly in
  split view (reader left, atlas right, following this chapter) -- the
  ONE-SHOT arrival signal both split entry points funnel through (see
  SPLIT-1 below); consumed exactly once per Reader.razor instance, never
  re-applied by a later, unrelated navigation
- `/world?from={year}&to={year}` — time mode (signed years, no zero)
- `/world?ref={REF}` — scripture mode (canonical ref)
- `/world` (no `from`/`to`/`ref` at all) — defaults to the `gospels` era's
  exact window (`[-5, 29]`, see `data/curated/eras.toml`) UNLESS this
  session already has a saved atlas position (batch-h-brief.md, VIEWSTATE-1
  below), in which case that position wins instead
- Split view has NO route of its own -- `/read/{BOOK}/{chapter}` (with
  split open, local component state) IS the split; there is no `/split/...`
  path. Closing the atlas pane returns to plain `/read/{BOOK}/{chapter}`;
  closing the reader pane navigates to plain `/world` (see SPLIT-1)

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
  in scripture mode and on mini-maps), `place-card` (attr `data-pinned` = "true"|"false" --
  batch-g1-brief.md requirement 3, PIN-1 below), `place-card-title`,
  `place-card-close` (batch-g1-brief.md; button; present ONLY while `data-pinned="true"`;
  closes the pinned card -- see PIN-1), `place-card-narratives` (batch-g1-brief.md; present
  ONLY when the pinned place has >=1 narrative leg in the current scene; wraps one row per
  such narrative -- swatch + name + `card-prev-event-{narrativeId}` / `card-next-event-
  {narrativeId}`, see TRAVERSAL-1 below), `card-prev-event-{narrativeId}` (batch-g1-brief.md;
  button; present only when that narrative has an adjacent PREVIOUS place for this one, per
  TRAVERSAL-1), `card-next-event-{narrativeId}` (batch-g1-brief.md; button; present only when
  that narrative has an adjacent NEXT place),
  `hover-verse-{VREF}` (one element per currently shown verse, VREF = canonical id e.g.
  `EXO.14.21`, whether it renders as its own lone-verse row or as one verse inside a
  passage block; element text contains that verse's own KJV text verbatim -- never
  trimmed, never paraphrased; batch-g1-brief.md requirement 1b -- explorable under
  ONE-RULE below, opens a VerseNode), `hover-passage-{SPAN}` (one per currently shown passage
  block -- a maximal run of >=2 consecutive same-book/chapter verses; SPAN = canonical
  span text of that run's CURRENTLY SHOWN extent, e.g. `GEN.12.1-4`; contains that
  block's own `hover-verse-{VREF}` elements; batch-g1-brief.md requirement 1b -- ALSO
  explorable, opens a PassageNode for the whole span -- clicking a specific nested
  `hover-verse-{VREF}` inside it opens that verse's own node instead, per ONE-RULE),
  `place-card-more` (button; present only
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
  `landmark-{slug}` (always-visible, non-interactive landmark label; slug = lowercase
  kebab-case of the landmark's name, e.g. "Mount Sinai" -> "mount-sinai"),
  `polity-label-{slug}` (batch-b2-brief.md; non-interactive polity-name label
  rendered from the currently-active POLITY ERAS whose own `[from,to]` intersects
  the window (`GET /api/polities`) -- slug = lowercase kebab-case of the era's
  own `name`, same rule as `landmark-{slug}`; ONE label per unique (polity id,
  era name) among the currently-visible eras, so a window spanning a border
  change where the SAME polity carries two DIFFERENT era names shows both labels
  at once (each centered on its own era's own rings) -- see `polity-ring-*`/
  `polity-year-tag-*` below for the ring-level testids that same spanning window
  also produces; visible in time mode only, subject to its own per-era zoom/
  viewport visibility rule -- absent entirely whenever no polities are loaded,
  e.g. scripture mode),
  `polity-ring-{id}-{from}-{ringIndex}` (batch-b2-brief.md; the fine border LINE
  path for one RING of one polity era -- `id` = the polity's own stable id (see
  `data/curated/polities/{id}.toml`), `from` = that era's own signed `from` year,
  `ringIndex` = that era's own 0-based ring index (almost always `0`; only a
  handful of curated eras carry a second, disjoint ring); attr `data-age` =
  `"oldest"` | `"middle"` | `"newest"` among that polity's OWN currently-visible
  eras (a single visible era is always `"newest"`; ">=2 rings, distinct dash
  classes" per the batch brief's own test list means at least two DIFFERENT
  `data-age` values are observable in a spanning window) -- present in time mode
  only, one per ring of every era whose `[from,to]` intersects the window,
  absent entirely in scripture mode,
  `polity-year-tag-{id}-{from}-{ringIndex}` (batch-b2-brief.md; a small mono
  "c. {year}" tag -- e.g. "c. 1500 BC" -- next to one ring; present ONLY when
  that ring's own polity has MORE than one currently-visible era, i.e. never
  on a single-era window; same `id`/`from`/`ringIndex` addressing as
  `polity-ring-*` above, one tag per ring, not per era)
Picker (ScripturePicker, shared by world and reader):
  `picker-book` (select of 66 books), `picker-chapter` (select sized from TOC),
  `picker-verse-from`, `picker-verse-to` (numeric inputs bounded by TOC),
  `picker-apply` (button; composes the canonical ref)
Reader: `reader-root`, `chapter-head` (batch-g1-brief.md; button, wraps the
  book-name/chapter-numeral spans; opens the ExplorerPopover with a
  ChapterNode), `verse-line-{n}` (batch-g1-brief.md; THE explorable element
  for that verse -- see ONE-RULE below; the retired `verse-explore-{n}` ∴
  button's replacement, not an addition alongside it), `verse-num-{n}`,
  `reader-prev`, `reader-next`, `passage-chip`
Split view (batch-h-brief.md, "study without page-turning" -- see SPLIT-1/
  FOLLOW-1/VIEWSTATE-1 below for the full behavior these wire up):
  `split-open-reader` (button, reader only, absent once split is open;
  "Open the map beside the text"; opens split, reader stays the route),
  `split-open-world` (button, /world only, absent once split is open;
  "Read beside the map"; navigates to `/read/{book}/{chapter}?split=1` for
  the reader's own last-known chapter, GEN 1 if none this session),
  `split-view` (the split's own flex-row container; present only while
  split is open -- element and testid both, on Reader.razor's own outer
  wrapper),
  `split-pane-atlas` (present on the atlas pane's own root only while
  embedded in a split -- absent on a standalone /world visit),
  `split-close-reader` (button, closes the READER pane -> full `/world`),
  `split-close-atlas` (button, closes the ATLAS pane -> full reader, same
  route),
  `follow-chip` (button; attr `aria-pressed` = follow state; text "Following
  {REF}" while following with a REF to show, else "Follow the text"; present
  only on the embedded atlas pane, i.e. only while split is open)
Popover (shared): `popover`, `popover-title`, `popover-breadcrumb-back`,
  `popover-chip-xrefs` (batch-g1-brief.md requirement 2: offered by BOTH VerseNode
  -- unconditionally, unchanged -- AND PassageNode -- CONDITIONALLY, present only
  when `GET /api/xrefs/{sref}` returns >=1 target for that passage's own span;
  expands the SAME inline `xref-item-{TARGET}` list either way, backed by
  `VerseDetail.CrossRefs` for a VerseNode or the new span-aggregation endpoint for a
  PassageNode),
  `popover-chip-map`, `popover-chip-book`, `popover-chip-context`,
  `popover-chip-verse-{VREF}` (batch-e-brief.md; one per a `YearNode`'s own curated
  supporting verses, in curated order, ALWAYS rendered before that same node's
  `popover-chip-map` chip -- DATE-1: opening a date's popover lists its supporting
  verses first),
  `xref-item-{TARGET}` (TARGET = canonical ref/span text), `mini-map`, `mini-map-open-world`
Notes:
- ONE-RULE (batch-g1-brief.md, user direction 2026-08-19: "the little trinity button
  isn't clear... explorable elements display slightly darker on hover; click opens the
  pop-up menu" -- REPLACES the retired `verse-explore-{n}` ∴ button, which offered no
  visible affordance of its own until Batch C2's interim `--explore-opacity` stopgap):
  every explorable element in the app signals itself and behaves IDENTICALLY -- on
  hover AND `:focus-visible` it darkens slightly (a translucent ink wash, `.explorable`
  in app.css, ~120ms ease, `prefers-reduced-motion: reduce` covered) with
  `cursor:pointer`; a click, or Enter while keyboard-focused, opens the
  ExplorerPopover on that element's own node. This currently covers: `chapter-head`
  (ChapterNode), `verse-line-{n}` (VerseNode), `hover-verse-{VREF}`/`hover-passage-
  {SPAN}` in the world place card (VerseNode/PassageNode -- requirement 1b), and
  `place-card-title`/`place-card-date-established`/`place-card-date-destroyed`
  (PlaceNode/YearNode, pre-existing testids, RESTYLED onto this same rule -- their own
  prior per-element hover color is gone). Two kinds of element are deliberately
  EXCLUDED, never explorable, and keep whatever hover treatment (if any) they already
  had: SELECTION controls (`verse-num-{n}` -- drives the anchor+extend passage-range
  mechanic, shift-click still forms `passage-chip`; a plain click on it no longer opens
  a popover at all, which it did, redundantly with the ∴ button, before this batch --
  selection and exploration are fully independent gestures on independent targets now)
  and PAGING controls (`place-card-more`/`place-card-collapse` -- "controls, not
  nodes"). A passage block's own per-verse `hover-verse-{VREF}` spans nest inside its
  `hover-passage-{SPAN}` row; clicking a specific verse span opens just that verse and
  never also bubbles into the passage's own click (stopPropagation on the inner
  element) -- the more specific target always wins.
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
  page through). UPDATED, batch-g1-brief.md: clicking a marker -- lit OR quiet, identically
  -- now PINS this exact same card open (`OnPlaceClick` gained real behavior; WORLD-1/2's
  original hover-only design is superseded) -- see PIN-1 below, which this quiet-place note
  no longer needs to duplicate.
- PIN-1 (batch-g1-brief.md requirement 3): clicking a `marker-{placeId}` or
  `quiet-marker-{placeId}` pins that place's `place-card` open (`data-pinned="true"`) --
  the exact same card content hover already renders (title/verse-or-quiet-content/
  controls/blurb/dates), now surviving a pointer leaving both the marker and the card
  (hover-persistence's own close-on-leave, batch-c2-brief.md requirement 0c, is
  suppressed while pinned). Hover on any OTHER marker while pinned does nothing (the
  pinned card owns the display slot exclusively) until the pin itself changes -- clicking
  a DIFFERENT marker re-pins to it, same as the first click. A pinned card closes via
  `place-card-close` (present only while pinned), Escape (page-wide; a no-op while an
  ExplorerPopover is open, so one Escape press closes exactly the topmost layer -- the
  popover first, the pin on a second press), or a click on the map BACKGROUND (never a
  marker/arrow click, which each stop propagation before it could also register as one).
  Opening a popover from INSIDE a pinned card (the title, a date, a verse/passage) closes
  the pin the same way it already closed an unpinned hover card -- promoting into a real
  popover always supersedes the card, pinned or not.
- TRAVERSAL-1 (batch-g1-brief.md requirement 3): while pinned, `place-card-narratives`
  shows one row per narrative with >=1 arrow (in the CURRENT scene) touching this place --
  a colored swatch (that narrative's own data color) + its name, small caps. Adjacency is
  derived CLIENT-SIDE from the scene's own `arrows` (no API change): for narrative N and
  place P, an arrow with `to_place==P` yields `card-prev-event-N`'s target (`from_place`);
  an arrow with `from_place==P` yields `card-next-event-N`'s target (`to_place`) -- each
  button present only when that direction has a candidate. A place appearing in more than
  one leg of the same narrative breaks the tie by preferring the arrow whose own event
  (`to_event` for prev, `from_event` for next) is among the CURRENTLY SHOWN place's events,
  else the lowest `order`. Clicking `card-prev-event-N`/`card-next-event-N` pans the map to
  the adjacent place's own marker (no zoom change) and pins ITS card -- a traversal chain:
  repeated clicks walk the narrative leg by leg, `card-prev-event-N` always reversing the
  most recent `card-next-event-N` (and vice versa) back to the previous place. An adjacent
  place that does not resolve in the current scene (lit or quiet) no-ops gracefully rather
  than erroring -- arrows only ever connect lit places (ARROW-1), so this is not expected
  to occur in practice, but is handled rather than assumed away.
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
  pointer that is legitimately still on the card. This whole mechanism (both the
  persistence itself and the ~350ms/~1s close) is SUPPRESSED entirely while the card is
  pinned (batch-g1-brief.md, PIN-1 above) -- a pinned card ignores pointer-leave forever,
  by design, resuming this exact behavior only once it's unpinned again.
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
- BORDER-1 (batch-b2-brief.md, "borders v2, the cartographer's edition"):
  for every time-mode window, `GET /api/polities?from=&to=` returns one row
  per (polity, era) pair whose era `[from,to]` intersects the window,
  ordered by polity id then era `from` -- a polity with exactly one
  intersecting era contributes one row; a polity with `N>=2` intersecting
  eras (a window spanning a border change) contributes `N` rows, oldest
  `from` first. The client renders every returned row (never picks a
  single "nearest" one the retired snapshot-year model used to) and tags
  each polity's own rows, independently, oldest-to-newest as `"oldest"`
  (dotted line, lightest wash), `"middle"` (dashed, intermediate -- reachable
  with 3+ simultaneously-visible eras of one polity; the curated roster has
  exactly one such case today, egypt's own three eras intersecting the
  1600-900 BC window, `BORDERS-5` -- corrected fix round 1, M4: this note
  previously claimed no such case existed, which the batch's own BORDERS-5
  test already contradicted at the time it shipped), `"newest"` (solid
  line, full wash -- ALSO the single-era case, i.e. `data-age="newest"`
  covers both "the only visible era" and "the newest of several"). Rings
  are painted in that same oldest-to-newest order (both across the whole
  layer and within each polity's own rows), so a polity whose later era
  is geographically LARGER paints its fuller wash last/on top over the
  earlier era's own footprint, while a polity whose later era is SMALLER
  still paints last, visibly inside the earlier, fainter ring -- either
  way each ring's own LINE (never just its wash) stays visible regardless
  of paint order, so growth or contraction is legible without the client
  ever comparing two rings' own geometry. `polity-label-{slug}` follows
  NAME-1's own spirit one layer up: one label per unique (polity id, era
  name) currently visible, so two eras of the same polity with the SAME
  name (a redraw, not a rename) still show only one label, while two eras
  with DIFFERENT names both show theirs. The retired "Borders c. X" tag
  (a single honest-about-its-snapshot readout) has no equivalent in this
  model -- there is no single snapshot year to be honest about anymore,
  only however many real eras intersect the window, each labeled on its
  own ring via `polity-year-tag-*` (present only when its own polity has
  more than one visible era).
- SPLIT-1 (batch-h-brief.md, "study without page-turning"): the reader
  (`split-open-reader`) and `/world` (`split-open-world`) each offer an
  affordance into the SAME split layout -- reader pane left (~55% width),
  atlas pane right (~45%), a thin bronze `split-divider` between them, both
  fully functional. The atlas pane IS the existing World.razor page
  componentry (a real child-component instance, `SplitMode=true`, never a
  copy); the reader pane is the existing Reader.razor content, unchanged,
  just narrowed. Reader.razor is ALWAYS the split's host (the URL always
  stays `/read/{BOOK}/{chapter}`, with or without split open) -- `/world`'s
  own affordance works by navigating TO the reader (`?split=1`, consumed
  once on arrival) rather than embedding a reader guest into `/world`
  itself; "closing" is what actually returns you to a full `/world` (see
  below). Closable both ways, independently: `split-close-atlas` closes the
  atlas pane only, leaving a full reader on the SAME `/read/{BOOK}/{chapter}`
  URL (a local toggle -- no navigation, no refetch, the reader instance and
  its scroll position are completely undisturbed); `split-close-reader`
  closes the reader pane only, navigating to a bare `/world` (the atlas's
  own current position round-trips via VIEWSTATE-1 below, since a bare-URL
  `/world` visit is exactly what that mechanism restores from). Both close
  buttons are real `<button>` elements, keyboard-reachable, with visible
  focus (the app's own global `:focus-visible` rule). Reader navigation
  (`reader-prev`/`reader-next`, a book jump, the reader's own picker) keeps
  the split open across it -- Reader.razor is REUSED, not recreated, for an
  ordinary chapter-to-chapter navigation (same as it always was, pre-Batch
  H), so `split-view`'s own open/closed state simply survives.
  NO-NESTED-POPUP: an ExplorerPopover opened from EITHER pane while split
  is open still renders normally -- full-viewport backdrop/panel, unchanged
  (this includes VerseNode/PassageNode's own `popover-chip-map` ->
  ShowMiniMap, "Explore geo-temporally": still a mini-map revealed in place,
  never affected by split at all) -- but a chip that would otherwise
  `Nav.NavigateTo("/world?...")` (opening what would be a SECOND full atlas)
  instead applies its exact query (a scripture ref or a time window) to the
  atlas pane THAT'S ALREADY SHOWING, in place; no second atlas ever opens
  while a split is up, from either pane's own popover.
- FOLLOW-1 (batch-h-brief.md): follow is ON by default the moment a split
  opens. While following, the atlas pane shows the scripture scene of the
  reader's CURRENT chapter via the exact same mechanism `/world?ref=`
  itself uses (no parallel scene-loading path) -- `follow-chip` reads
  "Following {BOOK.chapter}" and `aria-pressed="true"`. Reader navigation
  (`reader-prev`/`reader-next`, a book jump) re-scenes the atlas pane
  automatically, with no user action beyond navigating the text. Scripture-
  mode rules apply while following, unchanged from time mode's own contract
  elsewhere in this document: `slider`'s own `aria-disabled="true"`, zero
  `quiet-marker-*` elements (scripture-mode scenes never carry quiet
  places). Clicking `follow-chip` toggles it: OFF frees the pane to full
  time-mode (the slider re-enables, `mode-chip`/eras/everything `/world`
  itself has becomes reachable, `aria-pressed="false"`, chip reads "Follow
  the text"); ON re-syncs to the reader's current chapter's scene
  immediately. PRECEDENCE (the brief's own explicit requirement): a
  restored window (VIEWSTATE-1) is a snapshot; following is a live link --
  when a split opens with follow ON (the default, and whatever it was last
  explicitly left at, per VIEWSTATE-1), the reader's current chapter's
  scene ALWAYS wins over any restored time-mode window/camera, even if one
  was saved. A restored window only ever actually shows when the pane is
  NOT following (follow was last explicitly turned off, or this session has
  no saved state at all yet -- see VIEWSTATE-1's own field for the exact
  "which one wins" order).
- VIEWSTATE-1 (batch-h-brief.md): a lightweight, in-memory (NOT
  localStorage-persisted -- explicitly out of scope this batch; a hard
  reload starts fresh), app-lifetime view-state service remembers where the
  reader and the atlas were each left, independent of split. Map state
  (window OR scripture ref, follow on/off, camera center/zoom, captured on
  every World.razor dispose -- standalone page-nav-away OR a split pane
  closing, either one) is ONE shared value: full-page `/world` and the
  split's own atlas pane read and write the SAME saved position -- "it is
  the same atlas." Reader state (book/chapter + a plain scroll-Y pixel
  offset) is tracked continuously while Reader.razor is mounted (a
  throttled scroll listener, not a dispose-time read -- Blazor's own router
  resets window scroll on navigation before a dispose-time read could ever
  see anything but 0) and restored only when landing back on the EXACT
  book+chapter last left (a different chapter always starts at its own
  natural top, same as any ordinary navigation). Round-trip acceptance:
  reader (scroll down) -> `/world` (drag/zoom) -> back to reader (same
  scroll) -> back to `/world` (same window/camera) == exactly where left,
  each leg an ordinary full-page navigation. Split open/close preserves
  both sides the same way, since opening/closing on the READER side is a
  local toggle (nothing to restore, the reader instance never moved) and
  closing the ATLAS side is itself a navigation this same round-trip
  covers. `follow` itself is written back ONLY by a `SplitMode` (embedded
  atlas pane) dispose -- an intervening STANDALONE `/world` visit (which has
  no follow chip to change it from) never resets it, so the split's own
  follow state survives a detour through the full-page atlas untouched.
- EXISTENCE-1 (batch-h-brief.md, existence gating, deferred from E2): a
  place's NAME -- the `.atlas-label`/`.quiet-label` span inside
  `marker-{placeId}`/`quiet-marker-{placeId}`, never the dot/marker itself
  -- is hidden when the CURRENT time-mode window falls ENTIRELY outside
  that place's own curated existence bounds (established/destroyed, from
  `data/curated/place-history.toml` via `GET /api/scene`'s own
  `existence_from`/`existence_to` wire fields on each `places`/
  `quiet_places` entry -- both plain years, absent when uncurated). The dot
  stays fully present, hoverable, and clickable regardless -- "for
  availability," per the brief; existence gating is a LABEL rendering
  decision only, never a filter on which places even reach the wire. A
  place with no curated existence bounds at all (`existence_from` and
  `existence_to` both absent) always labels, at every window, with no
  exception. Inclusive on both ends, matching every other curated range in
  this document's own convention: a window reaching exactly a bound's own
  year does NOT gate. Scripture mode never gates anything (there is no
  window to test outside-ness against) -- `existence_from`/`existence_to`
  may still be present on a scripture-mode scene's own entries (the wire
  shape is unconditional), simply unused by the client there.
