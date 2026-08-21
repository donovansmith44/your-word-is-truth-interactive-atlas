# Bible Atlas UX Contract

Any implementation of the Bible Atlas UI MUST expose the surfaces below.
The UX property suite couples ONLY to this contract (plus the HTTP API).

## URL patterns
- `/` — reader, defaults to GEN 1
- `/read/{BOOK}/{chapter}` — reader deep link (BOOK = canonical 3-letter code)
- `/read/{BOOK}/{chapter}#v{n}` — verse anchor
- `/read/{BOOK}/{chapter}?split=1` — batch-h-brief.md: lands directly in
  split view (reader left, atlas right, following this chapter) -- the
  ARRIVAL signal both split entry points funnel through (see SPLIT-1
  below). `?split=1` itself is consumed exactly once per Reader.razor
  instance to SEED `_splitOpen` (never re-applied by a later, unrelated
  navigation to the same instance) -- but batch-f2-brief.md requirement 6c
  ("if i am in split screen mode and refresh, the split screen mode shalt
  not be ceased on account of refresh") keeps the QUERY STRING ITSELF
  continuously reflecting `_splitOpen` from then on (added the moment split
  opens by either entry point, carried forward across reader navigation,
  removed the moment split closes) -- so a refresh at ANY point while split
  is open always lands back in split view on the SAME chapter, not just
  immediately after the original arrival
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
  batch-g1-brief.md requirement 3, PIN-1 below; attr `data-flip` = "true"|"false" --
  batch-hotfix-brief.md requirement 1, CARD-FLIP-1 below), `place-card-title`,
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
  `reader-prev`, `reader-next` (batch-r-brief.md requirement 6, "always
  visible... middle-aligned... even as i scroll": REPOSITIONED from quiet
  bottom corners to vertically-centered at the reading column's own left/
  right edges -- `position:fixed`, unchanged testids/quiet-until-hovered
  treatment; `.reader-page`'s own `contain:layout`, Batch H, already
  confines this to the reader PANE specifically in split view, not the
  whole window. Review fix round 1, Critical-1 (2026-08-20): the vertically-
  centered position exposed a real overlap with mid-chapter verse text in
  split view at the documented 1024px floor -- fix round 1's own answer was
  a chevron-only compact form with the "Book NN" label hidden entirely
  (`display:none`) in split view, since SUPERSEDED (batch-hotfix-brief.md
  requirement 2, user report 2026-08-20: "the buttons to go chapter to
  chapter on the Bible in split screen are too tiny to see" -- measured
  live, 14.8x22.8px) by NAV-4 below: a real, legible presence in BOTH
  panes, sharing one set of rules (no split-specific fork) -- see NAV-4 for
  the current shape and exact numbers), `passage-chip`
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
  `popover-body` (batch-r-brief.md; the section-registry's own render target
  -- see REGISTRY-1 below),
  `popover-section-{id}` (batch-r-brief.md; one wrapper per RESOLVED section
  the registry rendered for the current node, `id` one of `verse-text`,
  `xrefs`, `catechism`, `place-dates`, `place-blurb`, `place-events`,
  `catechism-text`, `catechism-explanation`, `catechism-where-written`,
  `catechism-scriptures`, `narrative-event-text`, `narrative-prior`,
  `narrative-following` today -- see REGISTRY-1/CATECH-1/NARRATIVE-1;
  conditional presence, absent whenever that section's own provider
  resolved no content this open),
  `catechism-section-heading` (batch-f-brief.md; small-caps eyebrow rendered
  INSIDE a section's own body by that section's provider -- not a separate
  testid-bearing wrapper of its own; present on `popover-section-catechism`
  ("THE SMALL CATECHISM"), `popover-section-catechism-explanation` (Luther's
  own verbatim heading, e.g. "What does this mean?"),
  `popover-section-catechism-where-written` ("Where is this written?"), and
  `popover-section-catechism-scriptures` ("THE SCRIPTURES") -- see CATECH-1),
  `catechism-item-{ID}` (batch-f-brief.md, extended batch-f2-brief.md
  requirement 4; button; one per (item, question) hit citing the current
  VERSE/PASSAGE, `ID` = the item's own curated id, text = "`<Item>`" for an
  item-level hit (Luther's own embedded citation, Batch F, unchanged) or
  "`<Item> — <Question title>`" for a question-level hit (batch-f2-brief.md's
  own repo-mapping/Deut5-supplement citations, e.g. "The First Commandment —
  God the Holy Trinity"); opens a `CatechismNode` for that item -- see
  CATECH-1. The SAME item can legitimately produce >1 row in one span
  (different questions, or a question plus the bare item-level hit) -- the
  FIRST occurrence of a given `ID` keeps the bare `catechism-item-{ID}`
  testid (every pre-F2 single-occurrence case, e.g. Baptism's own items,
  unaffected); the second and later occurrences of the SAME id get a
  numbered suffix, `catechism-item-{ID}--q2`, `--q3`, ...),
  `catechism-verse-{SPAN}` (batch-f-brief.md, rebuilt batch-f2-brief.md
  6-ARCH; button; one per PASSAGE ENTRY inside
  `popover-section-catechism-scriptures` -- `SPAN` is a bare vref for a lone
  verse or a ref-range (e.g. `EXO.20.5-6`) when >=2 consecutive proof verses
  from the SAME source (Luther's own embedded citation, or one repo/Deut5
  question) group into one passage entry, per the shared passage-list
  component (see PASSAGE-1 below) -- text contains that block's own FULL KJV
  text (never truncated), captioned with its own question title when it has
  one; opens a `VerseNode` (lone verse) or `PassageNode` (passage) for
  `SPAN` -- onward navigation from there is ordinary Verse/PassageNode
  behavior, unchanged -- see CATECH-1),
  `popover-verse-expand{-ENTRY-ID}` (batch-r-brief.md requirement 4,
  generalized batch-f2-brief.md 6-ARCH; button; present on EVERY passage
  entry rendered by the shared passage-list component (see PASSAGE-1 below)
  -- the verse-text section's own SINGLE entry keeps the bare
  `popover-verse-expand` testid, byte for byte (no suffix, unchanged since
  Batch R); every entry in a MULTI-entry list (cross-references, THE
  SCRIPTURES, place est/dest) gets its own uniquely-scoped
  `popover-verse-expand-{ENTRY-ID}` -- fix-round-1 correction: `ENTRY-ID` is
  that entry's own FULL, already-prefixed testid (`{RefTestIdPrefix}-{Span}`,
  e.g. `xref-item-GEN.1.1-3`), the exact same string as that entry's own
  `data-testid` (PassageList.razor's `TestIdSuffix="@($"-{entry.TestId}")"`)
  -- NOT a bare span. This also means a repeated-span entry's own numbered
  `--2`/`--3` disambiguation (see PASSAGE-1) carries straight through into
  these nested testids too, so they stay collision-free for exactly the
  same reason the entries themselves do; so several entries can each be
  expanded independently in the same popover; text "Read the whole chapter"
  collapsed / "Show just this verse" expanded, attr `aria-expanded`; toggles
  the compact passage text vs. that entry's own scrollable mini-reader),
  `popover-verse-reader{-ENTRY-ID}` (same generalization; the mini-reader's
  own scrollable container; present only while that entry is expanded),
  `popover-reader-verse-{n}{-ENTRY-ID}` (same generalization; one per verse
  of the lazily-fetched chapter, `n` = verse number within it; attr
  `data-focal` = `"true"` for every verse in that entry's own focal
  verse/passage range, `"false"` otherwise -- see READER-1),
  `popover-reader-mention-{n}-{placeId}{-ENTRY-ID}` (batch-r-brief.md
  requirement 5, same generalization; one per detected place-name mention
  inside verse `n`'s own text -- see BLINK-1 below; hovering or
  keyboard-focusing it blinks `placeId`'s own map marker),
  `popover-place-date-established` / `popover-place-date-destroyed`
  (batch-r-brief.md requirement 3, REBUILT batch-f2-brief.md requirement 6b;
  no longer a button -- the "click to reveal supporting verses" gate is
  RETIRED; a plain instrument-face label+value row (e.g. "Established c.
  1003 BC"), non-interactive, immediately followed by that date's own
  supporting verses/passages rendered INLINE via the shared passage-list
  component -- see PASSAGE-1/XREF-1 below and this file's own est/dest note
  further down; conditional presence, one or both present exactly when
  PlaceCard's own `place-card-date-established`/`-destroyed` equivalents
  would be),
  `popover-place-date-established-verse-{SPAN}` / `popover-place-date-destroyed-verse-{SPAN}`
  (batch-f2-brief.md requirement 6b; button; one per passage entry among a
  date claim's own supporting verses, capped at 2 -- see this file's own
  est/dest note below; opens a `VerseNode`/`PassageNode`, same as every
  other passage-list entry),
  `popover-place-date-established-more` / `-collapse`,
  `popover-place-date-destroyed-more` / `-collapse` (batch-f2-brief.md
  requirement 6b; the down-arrow reveal / up-arrow snap-back for each
  date's own supporting-verse list -- present only when that date has more
  than 2 passage entries),
  `popover-place-blurb` (batch-r-brief.md requirement 3; the popover-native
  rendering of the SAME BLURB-1-resolved text `place-card-blurb` already
  shows; conditional presence, same BLURB-1 rule),
  `place-event-{id}` (button; one per this place's own recorded event,
  pushes a `TimeAndPlaceNode`; PRE-EXISTING since Task 15, undocumented
  before this batch -- see REGISTRY-1),
  `popover-chip-map`, `popover-chip-book`, `popover-chip-context`,
  `popover-chip-verse-{VREF}` (batch-e-brief.md; one per a `YearNode`'s own curated
  supporting verses, in curated order, ALWAYS rendered before that same node's
  `popover-chip-map` chip -- DATE-1: opening a date's popover lists its supporting
  verses first -- reached today only via `PlaceCard`'s own hover-card
  established/destroyed line, unaffected by batch-f2-brief.md requirement
  6b, which is scoped to the POPOVER's own est/dest section only),
  `xref-item-{SPAN}` (`SPAN` = a cross-reference target's own ref-range or
  bare vref; batch-r-brief.md: rendered INLINE, unconditionally offered
  where present -- see REGISTRY-1; the retired `popover-chip-xrefs` toggle
  is GONE for VerseNode/PassageNode, no button press needed to see it.
  REBUILT batch-f2-brief.md 6-ARCH/requirement 6: a target spanning >=2
  consecutive verses now renders as ONE passage entry with its OWN FULL text
  for every member verse -- not just `CrossRefOut.preview`'s first-verse
  text -- via the shared passage-list component; truncated per XREF-1
  below), `xrefs-more` / `xrefs-collapse` (batch-f2-brief.md requirement 6;
  the down-arrow reveal / up-arrow snap-back for the cross-references list
  -- present only when there are more entries than the current cap; see
  XREF-1),
  `narrative-section-heading` (batch-n-brief.md; small-caps eyebrow rendered
  INSIDE `popover-section-narrative-prior`/`-following`'s own body, same
  "shared testid, many different texts" convention as
  `catechism-section-heading` -- text is "PRIOR EVENT"/"FOLLOWING EVENT"
  bare when the current node touches exactly one qualifying narrative in
  that direction, else one heading per qualifying narrative, each reading
  "PRIOR EVENT — {narrative name}" (`FOLLOWING EVENT` symmetrically) --
  see NARRATIVE-1),
  `narrative-prior-event-{narrativeId}` / `narrative-following-event-{narrativeId}`
  (batch-n-brief.md; button; the adjacent event's own label; present once
  per qualifying narrative position in that direction -- a SECOND position
  sharing the same `narrativeId` (a real case in the compiled data, see
  NARRATIVE-1) gets a numbered `--2`/`--3` suffix, same disambiguation
  shape as `catechism-item-{ID}--q2`; clicking (or Enter) traverses --
  pushes a fresh `NarrativeEventNode` re-anchoring the popover onto that
  event, recursively -- see NARRATIVE-1),
  `narrative-prior-verse-{narrativeId}-{SPAN}` / `narrative-following-verse-{narrativeId}-{SPAN}`
  (batch-n-brief.md; one per passage entry among the adjacent event's own
  verses, via the shared passage-list component -- PASSAGE-1's own
  `popover-verse-expand{-ENTRY-ID}`/etc. nested testids apply here
  identically; opens a `VerseNode`/`PassageNode` for `SPAN`, same as any
  other passage-list entry -- see NARRATIVE-1),
  `narrative-event-narrative-name` (batch-n-brief.md; present on a
  `NarrativeEventNode`'s own popover only -- names which narrative this
  traversed event belongs to), `narrative-event-verse-{SPAN}` (batch-n-brief.md;
  one per passage entry among a `NarrativeEventNode`'s own verses, same
  shared-component treatment as above; absent entirely for an event with
  zero curated verses -- see NARRATIVE-1),
  `mini-map`, `mini-map-open-world`
Notes:
- REGISTRY-1 (batch-r-brief.md requirement 3, "the popover becomes a
  content-first section platform"): the popover body is a composable,
  ordered, CLIENT-SIDE registry of section providers
  (`client/Explore/PopoverSections.cs`'s own `PopoverSectionRegistry`), not
  one fixed per-node-kind fragment -- each provider declares which node
  `Kind`s it applies to and independently answers "does THIS node have
  content for me" (conditional presence: no content -> no
  `popover-section-{id}` at all, not an empty placeholder). VERSE node
  sections, in this order: the verse's own text with its own expand
  affordance (`popover-section-verse-text`, see READER-1), cross-references
  inline (`popover-section-xrefs`, conditional -- absent for a verse with
  zero recorded cross-references, truncated per XREF-1), "THE SMALL
  CATECHISM" (`popover-section-catechism`, batch-f-brief.md, conditional --
  absent for a verse citing zero catechism items -- see CATECH-1),
  "PRIOR EVENT" (`popover-section-narrative-prior`, batch-n-brief.md,
  conditional -- absent for a verse touching no narrative, or touching one
  only at its own FIRST leg -- see NARRATIVE-1) and "FOLLOWING EVENT"
  (`popover-section-narrative-following`, symmetric, absent at a
  narrative's own LAST leg). PASSAGE nodes get the same verse-text/
  cross-references/catechism sections (aggregating as before this batch) --
  NOT the narrative sections (batch-n-brief.md scopes PRIOR/FOLLOWING to
  VERSE only, a disclosed choice -- see NARRATIVE-1). NARRATIVE-EVENT node
  sections (batch-n-brief.md; a `NarrativeEventNode`, reached by traversing
  a PRIOR/FOLLOWING row above), in this order: the event's own subject text
  (`popover-section-narrative-event-text`, always present -- a small meta
  line naming the narrative, then the event's own verses via the shared
  passage-list component, absent entirely when the event has zero curated
  verses), then the SAME "PRIOR EVENT"/"FOLLOWING EVENT" sections again
  (recursion -- see NARRATIVE-1). PLACE node sections, in this order:
  an empty seam reserved for a future place-description provider (renders
  nothing today), established/destroyed dates (`popover-section-place-dates`,
  conditional -- batch-f2-brief.md requirement 6b: each date's own
  supporting verses now render INLINE within this section, truncated per
  the same XREF-1-family rule, see this file's own est/dest note below),
  period blurb (`popover-section-place-blurb`, conditional, BLURB-1), events
  (`popover-section-place-events`, one `place-event-{id}` row per event,
  pushes a `TimeAndPlaceNode` -- the thin, events-only PlaceNode popover
  this batch's own brief calls out is retired). CATECHISM node sections
  (batch-f-brief.md; a `CatechismNode`, reached by pushing a
  `catechism-item-{ID}` row above), in this order: the item's own primary-
  source text (`popover-section-catechism-text`, conditional -- absent for
  Baptism/Confession/Sacrament-of-the-Altar items, which pose their own
  question directly with no separate prompt -- see CATECH-1), the
  explanation under Luther's own verbatim heading
  (`popover-section-catechism-explanation`, always present), "Where is this
  written?" (`popover-section-catechism-where-written`, conditional), "THE
  SCRIPTURES" (`popover-section-catechism-scriptures`, conditional -- one
  `catechism-verse-{SPAN}` passage entry per run of curated proof verses,
  grouped/captioned per PASSAGE-1 -- see CATECH-1). A node `Kind` no
  provider claims at all (Chapter/Book/Author/TimeAndPlace/Year) keeps its
  own PRE-BATCH-R rendering, byte for byte -- unaffected by this note. The
  chips row (`popover-chip-map`/`-book`/`-context`/`-verse-{VREF}`) is
  UNCHANGED, pre-existing machinery, rendered below every section exactly as
  before -- "Explore" (the map affordance) is `popover-chip-map`, not a
  registry section; a CATECHISM node offers NO chips at all (no geography --
  conditional presence extends to affordances too, per CATECH-1).
- PASSAGE-1 (batch-f2-brief.md, 6-ARCH, user direction 2026-08-20,
  near-verbatim: "it should use the same underlying data structure as the
  hover menu everywhere else - showing sequential verses as passages...
  reuse the bits that we have"): ONE shared, composable passage-list
  component (`client/Components/PassageList.razor`) renders every verse
  LIST in the popover platform -- cross-references (XREF-1), THE SCRIPTURES
  (CATECH-1), and place est/dest supporting verses (this file's own est/dest
  note below). Sequential verses from the SAME source (one cross-reference
  target's own span, one catechism question's own citations, one date
  claim's own verses) group into ONE passage entry (ref-range + contiguous
  text, per-verse sup numbers) via the SAME grouping algorithm the map hover
  card introduced (Batch D, `client/Explore/PassageGrouping.cs`, shared --
  `PlaceCard.razor` itself now calls it too, rather than a second copy) --
  never N separate verse rows for what's really one contiguous citation, and
  never merged ACROSS two different sources even if numerically adjacent (a
  question caption, or a distinct xref target, must never silently blur
  into its neighbor's). Every passage entry is independently expandable, in
  place, to read the whole chapter -- REUSES Batch R's own mini-reader
  mechanism (`client/Components/MiniReaderExpand.razor`, extracted from
  `VerseTextSection.razor`, which now wraps it too -- one mechanism, every
  caller, per-entry-scoped testids, see this file's own
  `popover-verse-expand{-ENTRY-ID}` note above). Truncation caps (XREF-1 and
  this file's own est/dest note) count PASSAGE ENTRIES, not raw verses.
  Clicking a multi-verse entry (not its own mini-reader expand button --
  that stays in place) pushes a PassageNode by default (the group's own
  span, aggregate view) EXCEPT for cross-references (XREF-1): an xref-item
  always pushes a VerseNode at the TARGET's own first verse, regardless of
  how many verses its own preview text spans -- restoring
  `CrossRefsSection`'s pre-Batch-F2 contract (a cross-reference's own
  identity is "where it points", not "how many verses its preview covers";
  ~25% of real cross-reference targets span more than one verse, so this
  is common, not an edge case), verified by `reader.spec.ts`'s own READ-3
  property test. `PassageList.razor`'s own `ExploreAsVerse` parameter
  (default false; `CrossRefsSection` sets it true) is the mechanism.
- XREF-1 (batch-f2-brief.md requirement 6, user direction 2026-08-20,
  near-verbatim: "truncate the cross references to show no more than 3 if
  cross references are the only kind of context that we're pulling into the
  hover menu... and no more than two if there are other types of context
  pulled in (small catechism, etc.)"): in the VERSE/PASSAGE popover,
  `popover-section-xrefs` initially shows AT MOST 3 passage entries
  (`xref-item-{SPAN}`) when it is the ONLY context section present for the
  current node, AT MOST 2 when any OTHER context section (THE SMALL
  CATECHISM today; any future provider counts automatically -- the
  determination reads the LIVE, fully-resolved section-registry list, never
  a hardcoded "is catechism present" check, so this keeps working unchanged
  as later batches add providers) is ALSO present. `xrefs-more` reveals the
  rest (all remaining entries at once -- not an incremental step);
  `xrefs-collapse` snaps back to the capped view -- same down-arrow-reveal/
  up-arrow-snap-back interaction language `place-card-more`/`-collapse`
  (Batch D) already established, reused rather than a second one. Fewer
  entries than the cap -> no arrow at all (conditional presence). Counts
  exclude the verse-text section itself (`popover-section-verse-text` is
  the subject being read, not context pulled in alongside it).
  batch-f2-brief.md requirement 6b extends the SAME cap/reveal MECHANISM
  (via PASSAGE-1's shared component) to the PLACE popover's own
  established/destroyed supporting verses -- but with an UNCONDITIONAL cap
  of 2 passage entries per date (est and dest each), not context-dependent:
  "the place popover always has sibling sections" (blurb/events routinely
  present alongside dates), so there is no "only kind of context" case to
  distinguish there the way there is for xrefs. See
  `popover-place-date-established-verse-{SPAN}`/`-destroyed-verse-{SPAN}`
  and their own `-more`/`-collapse` pair in the testid inventory above.
- READER-1 (batch-r-brief.md requirement 4): `popover-verse-expand`
  collapsed shows exactly the compact text the popover always showed (one
  verse, or a passage's own already-known concatenated text); clicking it
  fetches the WHOLE chapter (`GET /api/chapter/{cref}`, lazily -- not before
  expand, cached like every other chapter fetch) and replaces the compact
  text with `popover-verse-reader`, a bounded, independently-scrollable
  region (the popover's own head/chips stay in place; only this region
  scrolls) listing every verse of that chapter, auto-scrolled once so the
  node's own focal verse (or, for a passage, its own first focal verse) is
  immediately visible, with every verse in the focal range carrying
  `data-focal="true"` and a calm, static highlight (no flash/animation).
  Collapsing restores the exact compact view; a later re-expand reuses the
  already-fetched chapter (no second fetch).
- BLINK-1 (batch-r-brief.md requirement 5, user 2026-08-19: "if i hover
  over a place within that verse, then the glowy dot associated with that
  location... should blink and be noticeable"): every verse rendered inside
  `popover-verse-reader` is scanned (`GET /api/chapter/{cref}`'s own,
  per-verse `places` array -- server: `AtlasData.places_for_verse`, the
  reverse of a place's curated `verse_links`) for a plain, case-insensitive
  substring match of each linked place's own name; the FIRST (longest-name-
  wins on overlap) match per place becomes `popover-reader-mention-{n}-
  {placeId}`, hoverable and keyboard-focusable. Hovering/focusing one
  toggles `.atlas-blink` (`app.css`) on `placeId`'s own marker CORE
  (`.atlas-marker`/`.quiet-marker`) across EVERY currently-live, non-mini
  map instance at once (map.js's own `blinkPlace`, looping its
  module-level `instances` registry) -- so this works identically whether
  the live map is the full `/world` page's own (a popover opened over it)
  or a split view's embedded atlas pane's own, with no page-specific
  wiring. A few beats of an ember-glow pulse (~1.7s, 3 cycles), then a
  steady, amplified glow for as long as the hover/focus holds;
  `prefers-reduced-motion: reduce` skips the pulse and shows the steady
  amplified glow immediately instead, never a moving animation. A mention
  is a plain, best-effort text match, not a claim of exhaustive recall --
  a place named only by a pronoun, or under a curated name the verse's own
  KJV wording doesn't literally use, is simply not detected. Reader-wide
  (outside a popover's own mini-reader) place-name hovers are explicitly
  OUT of this batch's scope (Batch P).
- CATECH-1 (batch-f-brief.md, "the small catechism" -- user direction, asked
  three separate times: verses should surface catechism refs/relevance
  alongside cross-references): Luther's Small Catechism (the 1921
  Bente-Dau translation, Concordia Triglotta -- public domain, provenance in
  LICENSES.md) is curated data (`data/curated/catechism.toml`, six chief
  parts item by item) wired both directions. VERSE/PASSAGE -> ITEM: `GET
  /api/verse/{vref}`'s own `catechism` field (Verse) / `GET
  /api/catechism/{sref}` (Passage span aggregation, mirrors `GET
  /api/xrefs/{sref}` exactly -- union of member verses' own citations, no
  "votes") populate `popover-section-catechism` ("THE SMALL CATECHISM",
  conditional -- absent for a verse/passage citing nothing), listing
  `catechism-item-{ID}` rows named by the item's own curated display name
  (e.g. "The First Commandment", "Baptism — Part Four"). ITEM -> its own
  content: clicking one pushes a `CatechismNode` (`GET
  /api/catechism/item/{id}`), whose OWN popover renders, in order:
  `popover-section-catechism-text` (the item's own primary-source wording --
  conditional, absent for Baptism/Confession/Sacrament-of-the-Altar items,
  which pose their own question directly with no separate prompt to quote
  first), `popover-section-catechism-explanation` (Luther's OWN verbatim
  heading as its `catechism-section-heading` -- "What does this mean?" for the
  overwhelming majority of items, a distinct real question for
  Baptism/Confession/Sacrament-of-the-Altar items, e.g. "What does Baptism
  give or profit?" -- never a generic placeholder), `popover-section-catechism-where-written`
  ("Where is this written?", conditional -- present only for the items where
  Luther's own text poses that exact question), `popover-section-catechism-scriptures`
  ("THE SCRIPTURES", conditional -- one `catechism-verse-{SPAN}` passage
  entry per run of curated proof verses sharing the same source, per
  PASSAGE-1, each captioned with its own question title when it has one).
  ITEM -> PROOF VERSE -> onward: clicking a `catechism-verse-{SPAN}` row
  pushes an ordinary `VerseNode`/`PassageNode` -- no bespoke code, so its own
  cross-references and (if the SAME verse also happens to cite a DIFFERENT
  catechism item) its own "THE SMALL CATECHISM" section work identically to
  any other verse reached any other way (verse -> catechism -> proof verse ->
  its own cross-references -> ..., the batch brief's own onward-navigation
  requirement, verbatim). A `CatechismNode` offers NO chips
  (`popover-chip-map`/`-book`/`-context`) at all -- catechism items have no
  geography, so "Explore geo-temporally"/"Read in context" have nothing to
  target; conditional presence extends to affordances, not just sections.

  batch-f2-brief.md requirement 3/4 ("the user's own catechism verse
  mapping" -- user direction 2026-08-20: "I gave you the mapping very
  explicitly in the catechism repo"): Luther's own item-level embedded
  citations (above) are no longer the only verse-link source. Each
  catechism item ALSO carries QUESTION-level citations
  (`CatechismItem.questions`, curated from the user's own
  brain-fuel/catechism repo -- see LICENSES.md's own "Catechism verse
  mapping" section -- plus this project's own Deuteronomy 5 parallel
  supplement for the Ten Commandments, `data/curated/catechism-deut5.toml`,
  requirement 5b, source-tagged separately from the repo-derived mapping).
  A `catechism-item-{ID}` row's own text reads "`<Item>`" for an item-level
  hit (unchanged) or "`<Item> — <Question title>`" for a question-level hit
  (e.g. "The First Commandment — God the Holy Trinity") -- the wire's own
  `question` field (`VerseDetail.catechism[].question` /
  `GET /api/catechism/{sref}`'s own array entries), omitted (not null) when
  the hit is item-level. The SAME item can legitimately produce more than
  one row for one span (different questions, or a question plus the bare
  item-level hit) -- deduplication is by the (item, question) PAIR, never
  item id alone, so no real distinction is silently dropped; see this
  file's own `catechism-item-{ID}` testid note above for the numbered-
  suffix disambiguation this requires. "If cheap, highlight/deep-link the
  question context" (requirement 4) is realized as a caption: each proof
  verse in THE SCRIPTURES shows its own question's title next to it
  (`.popover-passage-caption`), so opening the item from a question-titled
  row visibly shows WHICH verses that question itself cited.

  Verse-link sparsity for Luther's OWN embedded citations specifically is
  still a real, disclosed property of the primary source (unchanged since
  Batch F: explicit chapter-and-verse citations appear in only a handful of
  places in the 1921 text itself) -- but this is no longer the ONLY
  reachability path: batch-f2-brief.md's own coverage report
  (batch-f2-report.md) shows all 33 items reachable from >=1 verse once the
  repo mapping and Deut5 supplement are both counted.
- NARRATIVE-1 (batch-n-brief.md, "narratives as first-class graph
  structure" -- user direction 2026-08-20, verbatim: "narratives need to be
  represented as internal structures - not merely dots on a graph that you
  draw a line between... i expect to see, if i'm exploring a verse that is
  part of a narrative, the ability to traverse the narrative graph on the
  side of the reader... so, we have one graph representing narratives, and
  i can traverse arbitrarily far... and the appropriate narrative lines on
  the map side ought to be brought into particular focus"):

  ONE GRAPH, TWO SURFACES. `Narrative.legs` (an ORDERED chain of event ids
  -- unchanged since Task 3) is the single source both surfaces read:
  `scene::build_arrows` walks it to build a time/scripture-mode scene's own
  `arrows` (`SceneArrow`, the map's own threads); `atlas_core::narrative::positions_for_events`
  (new) walks the SAME `legs`, for a given event id, to find its own
  immediate PRIOR (`legs[idx-1]`) and FOLLOWING (`legs[idx+1]`) neighbors --
  each neighbor's own `verse_groups` built by the exact same
  `scene::to_scene_event` call `SceneEvent`/`VerseEventOut` already use
  everywhere else on the wire, so a PRIOR/FOLLOWING event's own verses are
  PROVABLY the same data an arrow endpoint's own place-card would show for
  that identical event id (not merely styled the same -- see
  `atlas_core::narrative`'s own
  `adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event` test,
  which `assert_eq!`s the two independently-derived values).

  WIRE. `GET /api/verse/{vref}`'s own `narrative_positions` array (folded
  into the already-shared verse-detail fetch, same "one fetch, not N"
  precedent `catechism` already set) answers "which narrative position(s)
  does this VERSE occupy" -- one entry per (narrative, event) pair the
  verse's own event(s) touch (a verse cited by >1 event, or an event that
  is itself a leg of >1 narrative -- BOTH real in the compiled data, not
  hypothetical: `EXO.12.37` is cited by both the exodus narrative's
  `ex_rameses` AND `ex_succoth` legs -- each yields its OWN entry, never
  silently collapsed). `GET /api/narrative/event/{id}` (new) answers the
  SAME question keyed by EVENT id instead -- requirement 1's own "traversal
  steps resolve by event, not by re-searching verses": some events carry
  zero curated verses at all, so a verse-based re-lookup would have nothing
  to click, but the event-id lookup always works. Each entry: `narrative_id`/
  `narrative_name`/`event_id`/`event_label` (the CURRENT position) plus
  `prior`/`following` (each, when present, an adjacent event's own
  `id`/`label`/`places`/`verse_groups`) -- `prior`/`following` OMITTED (not
  null) exactly at a narrative's own first/last leg.

  PROVIDER (no popover surgery -- registered exactly like every other
  section, Explore/PopoverSections.cs). VERSE nodes gain two MORE sections,
  appended after catechism (`NarrativePriorEventSection`/
  `NarrativeFollowingEventSection`, "PRIOR EVENT"/"FOLLOWING EVENT"),
  each conditional (absent when the verse has no qualifying narrative
  position in that direction). A verse touching >1 narrative renders one
  block per qualifying narrative inside the SAME section, each named "PRIOR
  EVENT — {narrative name}" (bare "PRIOR EVENT" when there is only one);
  the rare case where TWO qualifying positions share one narrative name
  (the `EXO.12.37` case above) additionally names the current event, "PRIOR
  EVENT — {narrative name} ({event label})", so the two stay
  distinguishable. Each block's own adjacent event renders via the SAME
  shared passage-list component (PASSAGE-1) every other verse list in this
  app uses -- grouped passages, truncation-free (no cap asked for),
  expand-to-chapter all inherited, zero parallel implementation.

  TRAVERSAL. Each adjacent event is EXPLORABLE (ONE-RULE): its own row
  (`narrative-prior-event-{narrativeId}`/`narrative-following-event-{narrativeId}`,
  the event's own label) is the traversal target -- clicking pushes a
  `NarrativeEventNode`, re-anchoring the popover onto that event ("its
  verses become the subject" -- a new NARRATIVE-EVENT node kind, see the
  testid-inventory's own REGISTRY-1 addendum above), locked to the ONE
  narrative it was reached through (a disclosed scope choice: an event that
  also happens to be a leg of a DIFFERENT narrative does not silently
  surface that OTHER narrative's own chain here -- opening one of the
  event's own VERSES instead, an ordinary passage-list click, resolves
  EVERY narrative it belongs to, same as any other verse). The traversed
  node's OWN PRIOR/FOLLOWING sections resolve by ITS event id (never a
  re-derived verse), recursing exactly as far as the underlying
  `Narrative.legs` chain goes -- first leg has no PRIOR section, last has
  no FOLLOWING, both by plain conditional presence, never a disabled stub.
  Also explorable, independently: each adjacent event's own passage-list
  entries (`narrative-prior-verse-*`/`narrative-following-verse-*`/
  `narrative-event-verse-*`) -- clicking one of THOSE opens an ordinary
  `VerseNode`/`PassageNode` for that specific verse/span instead of
  traversing the event as a whole (PASSAGE-1's own default click contract,
  unmodified) -- a second, independent way into the same graph, not a
  competing mechanism.

  CONSISTENCY WITH G1 (requirement 3's own "reuse, don't fork"):
  `PlaceCard.razor`'s own `NarrativeRows`/`PickAdjacent` (TRAVERSAL-1) is
  UNCHANGED by this batch and remains client-side, place-centric adjacency
  derived from a scene's own `arrows` -- a DIFFERENT code path from this
  note's own event/verse-centric one, but never a DIFFERENT ANSWER: both
  ultimately walk the SAME `Narrative.legs` chain (G1's own arrows are
  built from it via `scene::build_arrows`; this batch's own positions read
  it directly) and both resolve an event's own verses via the SAME
  `scene::to_scene_event`, so a place card's "next event" and a popover's
  "FOLLOWING EVENT" can never disagree about which event, or which verses,
  come next.

  MAP FOCUS SYNC. While the popover's own CURRENT node has >=1 narrative
  position (open on a narrative verse, or mid-traversal on a
  NarrativeEventNode): every currently-live, non-mini map instance (the
  split-view atlas pane AND/OR the full `/world` page -- map.js's own
  `instances` registry, same mechanism BLINK-1 already established for
  "reach whichever map is actually showing, with no page-specific wiring")
  has its narrative arrows (`arrow-{narrativeId}-{order}`) marked with a
  new `data-narrative-focus` attribute: `"receded"` for every OTHER
  narrative's own arrows (dimmed, NEVER removed/hidden -- "recede," the
  brief's own word, deliberately gentler than the PRE-EXISTING legend
  isolate's own near-invisible `data-faded="true"` .12 opacity, a SEPARATE,
  coexisting mechanism, not reused for this), `"active"` for the CURRENT
  node's own narrative(s)' other arrows (amplified stroke-width/opacity),
  `"current"` for the specific leg(s) touching the CURRENTLY open event
  (strongest emphasis -- "prior→current or current→following as
  traversed"). Absent (no attribute at all) is the baseline, both for an
  arrow that was never in any focus state and for every arrow once the
  popover closes or Current stops being narrative-aware ("Popover closes /
  context ends -> arrows return to normal") -- `ExplorerPopover`'s own
  `RequestClose` clears focus SYNCHRONOUSLY at the close action itself
  (HOUSE PATTERN: no dispose-time capture; this component has no
  `DisposeAsync` and gains none for this). No CSS transition on this
  attribute at all (app.css's own "an instant snap needs no
  prefers-reduced-motion carve-out" precedent, already established for the
  zoom/pan-driven arrow-path recompute) -- satisfies "state change without
  animated transition" under reduced motion trivially, by never animating
  either way. A scene change (the time slider dragged while a narrative
  popover happens to be open) resets every arrow's own
  `data-narrative-focus` to baseline, the SAME "starts fresh every scene"
  treatment `setArrows` already gives the legend-isolate `data-faded`
  attribute -- the popover's own next navigation restores it.
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
  prior per-element hover color is gone). Batch R adds, all opening a real
  ExplorerPopover node exactly like every element above: `xref-item-{SPAN}`
  (inline cross-reference rows, REGISTRY-1/XREF-1), and `place-event-{id}`
  (REGISTRY-1) -- `.atlas-label`/`.quiet-label` (LABEL-1) are DELIBERATELY NOT
  added to this list; a label is equivalent to its own dot (hover/click ->
  place-card, per PIN-1), never a popover-opening target itself.
  batch-f2-brief.md requirement 6b RETIRES `popover-place-date-established`/
  `popover-place-date-destroyed` FROM this list -- they are no longer
  buttons at all (a plain instrument-face label row now, see this file's
  own testid-inventory note); the explorable entries in that section are
  now `popover-place-date-established-verse-{SPAN}`/`-destroyed-verse-{SPAN}`
  instead (PASSAGE-1/XREF-1), same as every other passage-list entry.
  Batch F adds
  `catechism-item-{ID}` (the "THE SMALL CATECHISM" section's own citing-item
  rows) and `catechism-verse-{SPAN}` ("THE SCRIPTURES" section's own
  proof-verse rows) -- see CATECH-1. batch-n-brief.md adds
  `narrative-prior-event-{narrativeId}`/`narrative-following-event-{narrativeId}`
  (the PRIOR/FOLLOWING sections' own event-traversal rows) and
  `narrative-prior-verse-{narrativeId}-{SPAN}`/`narrative-following-verse-{narrativeId}-{SPAN}`/
  `narrative-event-verse-{SPAN}` (their own passage-list entries,
  PASSAGE-1's existing "every passage-list entry is explorable" rule
  already covers these generically) -- see NARRATIVE-1. Two kinds of
  element are deliberately
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
  data, 205 places as of batch-hotfix2-brief.md (was 206 -- SAMEPLACE-1's own hazor-1/
  hazor_545 merge below removed one event-bearing duplicate), derived from the data
  rather than hardcoded by either side) -- a place is always exactly one of lit or quiet, never both, never
  neither. A quiet place's own displayed name resolves against the SAME window using
  the SAME rules `marker-{placeId}`'s own label does (NAME-1), so a place's name never
  contradicts itself as it crosses from quiet to lit (or back) while a window is
  dragged. Scripture mode has no quiet places at all (`quiet-marker-{placeId}` is
  entirely absent there, same as on a mini-map) -- period relevance without a time
  window has nothing for GLOW to mean.
- SAMEPLACE-1 (batch-hotfix2-brief.md, same-place dedupe -- user report 2026-08-20:
  "in judges 4, zaananim, kedesh-naphtali, hazor are all in the ocean"): two compiled
  place records that are really ONE real-world place (a duplicate OpenBible/Theographic
  lineage, or two independent OpenBible identifications of the same site under
  different text-forms -- e.g. JDG.4.6's fully-qualified "Kedeshnaphtali" vs JDG.4.9-11's
  bare "Kedesh") are merged into ONE `marker-{placeId}`/`quiet-marker-{placeId}` before
  any scene is ever built (`atlas_core::merge`, applied once at data-load time, upstream
  of every consumer -- `/api/place/{id}`, arrow endpoints, and QUIET-1's own event-bearing
  set all already agree they are one node, not just the map). The merged place's `events`/
  `verse_groups` are the UNION of both records' own; its display name is the surviving
  (curated/OpenBible) record's. Wire traceability: `ScenePlace`/`QuietPlace` both carry
  `merged_ids` -- ids of every OTHER record folded into this one, e.g. `["hazor_545"]` on
  the place carrying id `hazor-1` -- omitted (not an empty array) when nothing was merged
  into that place, the overwhelming majority. CURATED, NOT AUTOMATIC: merging is NOT a
  blanket "any two places within 1.0km" rule -- a dataset-wide sweep at that same
  threshold found thousands of coincidentally-close place PAIRS that are genuinely
  DISTINCT real (or traditionally/scholarly disputed) locations sharing an imprecise
  upstream geocode (this file's own "Marker hover-target resolution" note, below, and
  map.js's own `setScene` comment already document a load-bearing example: Shittim and
  the "plains of Moab" camp, 0km apart, "both real, distinct places" that must NOT
  merge) -- so only a small, individually-verified, curated table of confirmed pairs
  (`atlas_core::merge::MERGE_PAIRS`) ever merges; see batch-hotfix2-report.md for the
  full sweep and reasoning. For a remaining pair that is close but NOT the same place
  (Zaanannim/Mount Tabor, ~4.2km apart in JDG.4 -- genuinely distinct), the anti-overlap
  nudge (nudgeCloseLatLng's replacement, `map.js`'s `applyMarkerNudges`) is computed in
  SCREEN PIXELS at the CURRENT zoom (never a fixed geographic delta -- the pre-fix bug's
  own root cause: a 0.6-degree/~65km shove, tuned for a wide-zoomed-out scene, crossing
  the coastline at a much closer-zoomed one), recomputed fresh on every zoom change, and
  never moves a marker more than ~20px from its true position.
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
  to occur in practice, but is handled rather than assumed away. batch-n-brief.md's own
  reader-popover traversal (NARRATIVE-1) is a DIFFERENT code path -- reuses
  the SAME `Narrative.legs` chain and the SAME `scene::to_scene_event` this
  note's own adjacency is built from, so the two can never disagree; see
  NARRATIVE-1's own "consistency with G1" paragraph.
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
- LABEL-1 (batch-r-brief.md requirement 2, regression + enhancement, user
  2026-08-19: "i can no longer click on a location's name, where i used to
  be able to and i would get the est/dest dates. we need that functionality
  back"): a place's name -- `.atlas-label` inside `marker-{placeId}`,
  `.quiet-label` inside `quiet-marker-{placeId}` -- is a full hover/click
  target EQUIVALENT to its own dot: hovering the label opens the exact SAME
  `place-card` a dot-hover would (same content, same
  `QUIET_HOVER_INTENT_MS` debounce on a quiet label), clicking it pins the
  SAME card a dot-click would (PIN-1) -- both by simple DOM event bubbling
  (the label is a descendant of the marker's own icon root map.js's
  `wireEvents`/`wireQuietEvents` already listen on; no map.js change was
  needed for the wiring itself, only restoring `pointer-events` on the label
  from the earlier `none`), never a `.explorable`/ExplorerPopover-opening
  target itself (a label is equivalent to its DOT, not to a verse line --
  the resulting `place-card` IS the hover feedback, same as a bare dot
  always had none of its own). `place-card-title`'s own click still promotes
  the card into a real `PlaceNode` popover (est/dest dates, reachable via
  `popover-place-date-established`/`-destroyed` inside it -- REGISTRY-1) --
  so a date is reachable in exactly 2 clicks from a label: the label pins
  the card, the title opens the popover. Polity-label/landmark-label stay
  entirely non-interactive, unchanged.
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
- LAND-1 (batch-r-brief.md requirement 1, "borders become part of the
  plate", user 2026-08-19: "the borders still suck and are overlays on the
  actual map, when they need to be PART of the actual map"): every polity
  wash (`.atlas-border-wash`) is clipped to a hand-drawn land/coastline mask
  (`GET /api/land-mask`, `data/curated/land-mask.toml`) -- an SVG
  `<clipPath id="atlas-land-clip">` map.js's `BorderLayer` builds once,
  applied to the `<g class="atlas-wash-clip-group">` every wash `<path>`
  lives in (never to the band/line strokes, which stay in the unclipped,
  unblended `overlayPane` exactly as before -- "ink border strokes stay
  crisp"), so a wash never paints over open sea regardless of how a
  hand-drawn polity ring's own edge happens to fall relative to the real
  coastline. The wash's own fill is separately feathered (a single, subtle
  `feGaussianBlur`, `filter: url(#atlas-wash-feather)` on
  `.atlas-border-wash` itself -- "printed tint soaking into paper, not neon
  glow") -- filter resolves before clip-path (the standard order), so a
  blurred edge bleeding toward open water is still cut off hard exactly at
  the coastline; only a wash's OWN inland edges (against another era, or
  simply its own boundary well inside the mask) read as soft. Both the clip
  geometry and the filter live inside the SAME `washPane` SVG the wash
  paths themselves do, so both track zoomanim identically to every other
  layer in this app (no separate wiring). The mask itself is never rendered
  as its own visible layer -- it exists only as clip geometry, mix-blend
  multiply on the pane (Batch B2/B2-fix-round-1) stays completely
  unchanged.
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

  batch-f2-brief.md requirement 6c (user direction 2026-08-20, verbatim:
  "if i am in split screen mode and refresh, the split screen mode shalt
  not be ceased on account of refresh"): the reader's OWN `split-open-reader`
  affordance now ALSO reflects `?split=1` into the URL the moment it opens
  (previously a local field flip only, with no URL change at all --
  `/world`'s own `split-open-world` already built `?split=1` into its
  target URL, so only the reader-side entry point needed this fix), and
  `split-close-atlas` removes it again ("closing the split cleans the
  param") -- both via a query-string-only `NavigateTo(..., replace: true)`
  that never triggers a refetch or disturbs scroll/popover state (book/
  chapter route params are unchanged, so `OnParametersSetAsync`'s own
  redundant-navigation guard short-circuits before touching anything else).
  `reader-prev`/`reader-next`'s own hrefs, the reader's `ScripturePicker`
  navigation, and a popover's "Read in context" chip (`ExplorationTarget.NavigateReader`)
  all carry `?split=1` forward too whenever a split is currently open, so
  the URL never silently drops it partway through an in-session reading
  session. A browser refresh while the URL carries `?split=1` therefore
  always lands back in split view, on the SAME book/chapter (the route
  params, unaffected by any of this). Follow state (FOLLOW-1) is NOT
  separately persisted across a refresh -- `ViewStateService` is a plain
  in-memory singleton, and a hard reload restarts the whole WASM app, so
  its own default (follow ON) simply applies again, same as any other
  fresh session; this is the correct, intended behavior for "follow default
  ON applies unless persisted" (VIEWSTATE-1's own explicitly-scoped
  in-memory-only design), not a gap.
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
  (window OR scripture ref, follow on/off, camera center/zoom) is ONE
  shared value: full-page `/world` and the split's own atlas pane read and
  write the SAME saved position -- "it is the same atlas." Reader state
  (book/chapter + a plain scroll-Y pixel offset) is tracked continuously
  while Reader.razor is mounted (a throttled scroll listener, not a
  dispose-time read -- Blazor's own router resets window scroll on
  navigation before a dispose-time read could ever see anything but 0) and
  restored only when landing back on the EXACT book+chapter last left (a
  different chapter always starts at its own natural top, same as any
  ordinary navigation). FIX ROUND 2: map state is ALSO tracked
  continuously, not captured at dispose time -- window/ref/follow the
  instant any of them changes (every direct scene-entry path on
  World.razor), camera center/zoom on every Leaflet moveend/zoomend (map.js).
  This replaced an earlier dispose-time capture that shipped with a real,
  live-reproduced bug: on a navigation that disposes one World instance
  while mounting a brand-new one in the SAME step (`split-close-reader`,
  and symmetrically "Read beside the map" opening a split), Blazor gives no
  ordering guarantee between the outgoing instance's own DisposeAsync (even
  its fully synchronous portion) and the incoming instance's own
  OnParametersSetAsync reading the shared state -- confirmed live: even a
  purely synchronous dispose-time write, with no `await` before it, still
  lost the race, because the losing side was DisposeAsync not yet having
  been CALLED at all, not merely an in-flight continuation. Continuous
  tracking sidesteps the question of when (or whether) DisposeAsync runs
  relative to a new instance's mount entirely, mirroring the reader-scroll
  mechanism above exactly. Round-trip acceptance: reader (scroll down) ->
  `/world` (drag/zoom) -> back to reader (same scroll) -> back to `/world`
  (same window/camera) == exactly where left, each leg an ordinary
  full-page navigation. Split open/close preserves both sides the same
  way, since opening/closing on the READER side is a local toggle (nothing
  to restore, the reader instance never moved) and closing the ATLAS side
  is itself a navigation this same round-trip covers -- including the
  concurrent dispose+mount shape above, now that the write no longer
  depends on DisposeAsync running in time. `follow` itself is written back
  ONLY by a `SplitMode` (embedded atlas pane) instance -- an intervening
  STANDALONE `/world` visit (which has no follow chip to change it from)
  never resets it, so the split's own follow state survives a detour
  through the full-page atlas untouched.
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
  WIDENED BY CURATED NAMES: `established` documents when a place became
  established AS ITS CURATED IDENTITY, not necessarily when a settlement
  first stood on the ground -- Jerusalem's own curated `established` claim
  is David's conquest (-1003, "traditional"), but the same place separately
  carries the curated name "Jebus" from -4004 to -1004 (see NAME-1 above).
  Gating on `established` alone would hide Jerusalem's label during its own
  curated "Jebus" period, defeating the point of NAME-1's period-name
  resolution existing at all. So `existence_from`/`existence_to` are each
  WIDENED (never narrowed, and never introduced from nothing -- a place
  with no `established` claim still has no lower gate even if it has
  curated names) to also cover every curated `[[place.name]]` entry's own
  range: the lower bound is `min(established, earliest name.from)`, the
  upper is `max(destroyed, latest name.to)`. A place with no curated name
  entries (e.g. Shiloh) is unaffected -- the widening is a no-op. This
  widening happens server-side, inside `resolve_existence`
  (`atlas-core/src/history.rs`) -- the wire already carries the final,
  widened bounds, so the client's own `existenceGatesLabel` (map.js) needs
  no knowledge of name ranges at all, just the plain inclusive-bounds
  comparison.
- CARD-FLIP-1 (batch-hotfix-brief.md requirement 1, user report 2026-08-20:
  "if there are locations at the top of the screen and you hover for your
  hover menu, the hover menu can be cut off by the top of the screen"):
  `place-card` renders ABOVE its marker (the pre-existing default, unchanged)
  UNLESS there is no room above it within the map container it is currently
  rendered inside (`.world-page` standalone, `.split-pane-atlas` embedded --
  the SAME box that box's own `overflow:hidden` clips to, in either
  context), in which case it renders BELOW instead (`data-flip="true"`,
  mirrored across the marker -- same 18px gap on the opposite side); if
  NEITHER orientation fully fits (fix round 1, review finding: a card near
  the vertical middle of a short viewport could previously flip below and
  still overflow the container's own bottom edge, since the original cut
  only ever checked "does it fit above" -- live-reproduced, 122px past the
  bottom at 1280x720, a real marker), whichever orientation shows MORE of
  the card is chosen and the result is clamped fully inside the container
  (top and bottom both) rather than left to overflow either edge.
  Independently, horizontally: the card is clamped the same way so it never
  crosses either side of that same container, nudged inward from its normal
  centered-on-the-marker position only exactly as far as needed (most cards
  need no nudge, on either axis, at all). All of these decisions are made
  ONCE, the first time this place's own card has fully loaded (verse text
  AND, if curated, blurb/dates -- measuring any earlier would size against
  not-yet-loaded content) after opening -- never reconsidered afterward for
  that same open (not on ShowMore/Collapse growing the card, not on the map
  panning underneath an already-open card) -- a fresh decision is only ever
  made on a genuine re-open (hovering/clicking/traversing to a DIFFERENT
  place, or re-hovering the same one after it fully closed). The
  pre-existing hover-persistence/grace mechanism ("Place-card hover
  persistence" above) is completely unaffected by orientation or clamping
  -- it tracks pointer entry/exit of the marker and the card as plain DOM
  elements, never their relative screen position, so the pointer can travel
  from a marker down into a flipped (or clamped) card exactly as reliably
  as it travels up into a non-flipped one. Applies identically everywhere
  this card renders (full `/world`, split view's embedded atlas pane) --
  one shared positioning rule, no per-page fork. The reader's own mini-map
  (`mini-map`, `MiniWorld.razor`) never renders this card at all (its own
  marker hover callbacks are deliberate no-ops, per that component's own
  comment, unaffected by and unrelated to this note) -- there is nothing
  there for CARD-FLIP-1 to apply to.
- NAV-4 (batch-hotfix-brief.md requirement 2, user report 2026-08-20: "the
  buttons to go chapter to chapter on the Bible in split screen are too
  tiny to see" -- measured live before this fix, split view: 14.8x22.8px at
  a 13.6px font): `reader-prev`/`reader-next` share ONE set of rules across
  BOTH standalone and split (no per-pane fork) -- a vertical arrow-glyph-
  over-chapter-label stack, `position:fixed`, vertically centered exactly as
  NAV-2 already established, unchanged. The rendered hit target measures
  comfortably over 40x40px in both dimensions in EITHER pane (a real
  margin, not a last-pixel fit) -- split view's own box is a fixed 44x72px
  (min-width/max-width both 2.75rem), standalone's grows to fit its own
  larger content, never below that same 44/72px floor either. The label
  text is the SAME "Book NN" content `.reader-nav-label` always carried
  (present now in split view too, no longer `display:none` there) --
  unabbreviated, just laid out compactly, wrapping across more lines
  (never growing wider than its fixed box, even for an outlier long
  single-word book name like "1 Thessalonians", which wraps mid-word if it
  must) rather than ever risking a re-overlap with verse text; the `<a>`'s
  own `aria-label` still separately carries the complete "Previous/Next
  chapter: Book NN" phrase regardless, unchanged. Rest-state color is
  `--ink` (13.98:1 against `--parchment`, computed) -- clears this
  requirement's own >=10:1 parchment floor with real margin, up from the
  pre-batch `--ink-soft` (5.66:1, short of it); hover amplifies to
  `--lapis`, the same quiet-until-hover color swap this file already uses
  identically elsewhere (`.verse-num`, etc.), unchanged by this batch.
  `.reader-column`'s own left/right padding widens in split view
  specifically (a CSS custom property inherited down from
  `.split-pane-reader`, never a two-class descendant selector -- see
  app.css's own comment on both rules for the CSS-discipline fix this folds
  in, deferred from Batch R's re-review) so the nav's own footprint still
  clears verse text with real margin at the documented 1024px floor
  (NAV-3, unaffected, stays green) -- standalone is unaffected (that
  padding's own fallback matches the pre-batch value exactly). No layout
  shift, no animation, keyboard focus visible (this file's own global
  `a:focus-visible` rule, unchanged) -- all unaffected by this batch, same
  as before it.
- PANE-ANCHOR-1 (batch-f2-brief.md requirement 6d, user direction
  2026-08-20, near-verbatim: "if i am exploring anything on either side of
  the split screen, the hover windows ought not be smack dab in the center
  of the screen, but on the side of the screen where the hover exploration
  originated... particularly i am referring to when i click on a verse and
  a hover box appears"): while a split is open, the ExplorerPopover
  (`popover`/`popover-backdrop`) anchors to the ORIGINATING PANE's own
  currently-visible region instead of the full viewport -- a verse clicked
  in the reader pane opens centered within the reader pane; an exploration
  started on the atlas pane (a marker/place card promoted into a popover)
  opens centered within the atlas pane. The OTHER pane stays completely
  undimmed and clickable (the backdrop itself is pane-scoped, not just the
  panel) -- "explore on both sides of the screen independently, while still
  following text," per the brief verbatim. Full-page (non-split) popovers
  are byte-for-byte unchanged (still viewport-centered) -- this is purely a
  split-mode behavior, gated on each host page's own split state
  (`Reader.razor`'s `_splitOpen`, `World.razor`'s `SplitMode`), passed down
  as a new `PaneAnchor` parameter (`"reader"` / `"atlas"` / `null`).
  Measured ONCE, at popover-open time (`reader.js`'s `getPaneRect`,
  viewport-clamped so a reader pane taller than one screen still centers
  within the currently VISIBLE slice of it, never somewhere off-screen in
  scrolled-past content) -- the SAME "snapshot at open, never re-measure on
  scroll/pan" discipline `PlaceCard`/`CardPlacement` already established
  for the map hover card's own flip/clamp decision, not a continuous
  tracker. This batch keeps the existing ONE-popover-instance rule
  unchanged (each pane's OWN ExplorerPopover instance -- Reader.razor and
  World.razor each already render their own, per SPLIT-1 -- is what's being
  anchored; this is a POSITION change, not concurrent popovers on both
  panes at once, which stays a possible future extension, not built here).
  Bounded within the pane on every axis (never overflows into the other
  pane) -- the popover's own `max-width`/`max-height` are computed against
  the pane's own measured size (the same margins the full-viewport rule
  already subtracts, just from the pane instead), so an expanded mini-reader
  (READER-1/PASSAGE-1) inside it is automatically confined the same way,
  with no separate rule needed.
