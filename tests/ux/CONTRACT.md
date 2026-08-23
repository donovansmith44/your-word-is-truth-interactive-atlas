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
- `/world` (no `from`/`to`/`ref` at all) — defaults to a Gospel-era window,
  `[-5, 33]` (`World.razor`'s own `DefaultFrom`/`DefaultTo` — HOTFIX-4 fix
  round 1 CORRECTION: was `[-5, 29]`, matching `data/curated/eras.toml`'s
  own `gospels` era exactly; extended to 33, the real calibrated Gospel-era
  end — see EVENT-MERGE note below — once nt_calibration made 29 exclude
  Passion Week/Resurrection/Ascension from the default view; the `eras.toml`
  preset itself is intentionally left at `[-5, 29]`, unconsumed by any
  client UI today) UNLESS this session already has a saved atlas position
  (batch-h-brief.md, VIEWSTATE-1 below), in which case that position wins
  instead
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
  `polity-ring-*` above, one tag per ring, not per era),
  `polity-delta-{id}-{from}-{ringIndex}` (batch-m-brief.md requirement 4; a
  wide, transparent hit-stroke `<path>` sharing that same ring's own `d`
  geometry -- real, keyboard-reachable (`tabindex="0"`, `role="button"`);
  present ONLY when that ring's own era boundary (its start for a
  transition, its end for a fall, see DELTA-1 below for the exact rule)
  falls INSIDE the currently applied window, REGARDLESS of whether a
  `[era.transition]`/`[era.fall]` block was actually curated for it --
  "an uneventful boundary stays visible but gets the minimal popover," not
  "stays uninteractive"; opens a `PolityDelta` node -- see DELTA-1)
Picker (ScripturePicker, shared by world and reader):
  `picker-book` (select of 66 books), `picker-chapter` (select sized from TOC),
  `picker-verse-from`, `picker-verse-to` (numeric inputs bounded by TOC),
  `picker-apply` (button; composes the canonical ref)
Reader: `reader-root`, `chapter-head` (batch-g1-brief.md; button, wraps the
  book-name/chapter-numeral spans; opens the ExplorerPopover with a
  ChapterNode -- M-D3/U4/B3: that popover's own content is now the
  chapter's metadata-and-context card, `ChapterCardSection` -- see
  B3-CARD below and the `chapter-card-*` testids further down -- NEVER
  the chapter's own verse text, hover OR click both open it (`@onmouseenter`
  added alongside the existing `@onclick`, matching every other superscript/
  heading entry point's own "hover and click are the same open" rule)),
  `verse-line-{n}` (batch-g1-brief.md; THE explorable element
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
  the current shape and exact numbers), `passage-chip`,
  `pericope-heading-{eventId}` (batch-t-brief.md requirement 5; a real
  `<h2>`, explorable (ONE-RULE) -- a quiet small-caps title rendered
  immediately above its own first verse's `verse-line-{n}`, present
  whenever `GET /api/chapter/{cref}`'s own per-verse `heading` field is
  present (server: `atlas_graph::heading::build_heading_index`, the live production path; `AtlasData::heading_for_verse` remains only as the tested lockstep oracle) -- conditional presence,
  absent entirely for an uncovered book/chapter (no verse there is ever a
  heading anchor); clicking opens a fresh `EventNode` for that heading's own
  `event_id` -- see EVENT-1. Shares this SAME code path for the split-view
  reader pane (Reader.razor is reused, not copied, for both -- SPLIT-1)),
  `verse-xref-marker-{n}` (batch-md2-brief.md, the owner's cross-reference
  superscript directive; one `<sup>`, immediately after `verse-line-{n}`'s
  own verse text, present iff `GET /api/chapter/{cref}`'s own per-verse
  `xref_count` (batch-md2-brief.md; the verse's own `cites` edge-summary
  count, THE PORT) is `> 0` -- see XSCRIPT-1 below for the full lettering
  scheme; entry point into the SAME `ExplorerPopover`, xrefs section
  leading -- see XSCRIPT-1/CAP-RECONCILE-1), `verse-mention-{n}-{placeId}`
  / `verse-mention-person-{n}-{personId}` (M-D3/U5; zero or more per verse,
  inside `verse-line-{n}`'s own verse text -- see MENTION-1 below for the
  full scan/click/blink behavior)
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
  `split-divider` (M-D3/B2, Components/SplitDivider.razor; present only
  while split is open -- the drag-resize handle between the two panes; see
  DIVIDER-1 below for the full mechanism),
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
  `catechism-scriptures`, `event-membership`, `event-date-places`,
  `event-witnesses` (or `event-witness`, singular, for a single-witness
  event -- see EVENT-1), `event-prior`, `event-following`,
  `polity-delta-event`, `polity-delta-scriptures`, `polity-delta-grounding`
  today -- see REGISTRY-1/CATECH-1/EVENT-1/DELTA-1; conditional presence,
  absent whenever that section's own provider resolved no content this
  open),
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
  numbered suffix, `catechism-item-{ID}--q2`, `--q3`, ...; M-D3/U2/U6:
  only the first 2 (Default) render initially -- see `catechism-more`/
  `catechism-collapse` immediately below and the XREF-1 note's own
  CATECH-1/U6 paragraph),
  `catechism-more` / `catechism-collapse` (M-D3/U2/U6; the shared
  RevealControls.razor mechanic's own arrows for THE SMALL CATECHISM's own
  citing-item list -- present only when there are more items than the
  Default (2); see the XREF-1 note's own CATECH-1/U6 paragraph for the
  full +2/-2/all/default behavior, identical to `xrefs-more`/
  `xrefs-collapse`),
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
  the compact passage text vs. that entry's own scrollable mini-reader.
  M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  when already reading that chapter -- a chapter-aware policy, not a new
  data path." CONDITIONAL PRESENCE, applying to every instance of this
  button regardless of which section renders it (verse-text, xrefs, THE
  SCRIPTURES, place est/dest, event witnesses, narrative/timeline
  PRIOR/FOLLOWING -- MiniReaderExpand.razor is the ONE shared mechanism,
  so this is one check in one place, not a per-caller flag): absent
  entirely, not merely disabled, whenever the Book+Chapter it would open
  is the SAME Book+Chapter a Reader.razor instance is ACTIVELY showing
  right now (`ViewStateService.MountedReaderChapter` -- deliberately
  separate from the pre-existing, PERSISTENT `ViewState.Reader` "last
  known position" record, which stays set even after Reader.razor
  unmounts and would wrongly suppress the affordance on a plain `/world`
  visit too; this field is null unless a Reader.razor instance -- standalone
  or split view's own embedded pane, SPLIT-1 -- is mounted on that exact
  chapter at this exact moment). A verse popover reached by clicking a
  verse-line in the reader is, by construction, always FROM the chapter on
  screen, so its own `popover-verse-expand` is now unconditionally absent
  every time; exploring onward (a cross-reference, a catechism proof
  verse, a witness, ...) to a DIFFERENT book/chapter keeps that entry's
  own expand button fully present and functional, unaffected),
  `popover-verse-reader{-ENTRY-ID}` (same generalization; the mini-reader's
  own scrollable container; present only while that entry is expanded),
  `popover-reader-verse-{n}{-ENTRY-ID}` (same generalization; one per verse
  of the lazily-fetched chapter, `n` = verse number within it; attr
  `data-focal` = `"true"` for every verse in that entry's own focal
  verse/passage range, `"false"` otherwise -- see READER-1),
  `popover-reader-mention-{n}-{placeId}{-ENTRY-ID}` / `popover-reader-
  mention-person-{n}-{personId}{-ENTRY-ID}` (batch-r-brief.md requirement
  5, same generalization; widened M-D3/U5 to a second entity kind -- one
  per detected place-/person-name mention inside verse `n`'s own text --
  see BLINK-1/MENTION-1 below; hovering or keyboard-focusing a PLACE one
  blinks `placeId`'s own map marker; either kind opens that entity's own
  node on click/Enter),
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
  `popover-place-canonical-name` (batch-e3-brief.md requirement 2; a plain,
  non-interactive quiet line reading "Known in modern atlases as {name}." --
  present only when this place's displayed name differs from its own bare
  canonical name (a curated KJV alias resolved to something else); see
  ALIAS-1 below for the full precedence and conditional-presence rule),
  `place-event-{id}` (button; one per this place's own recorded event, up
  to M-D1's own truncation-audit cap of 10 (`place-events-more`/`-collapse`
  reveal the rest -- see the M-D1 TRUNCATION AUDIT note), pushes a
  `TimeAndPlaceNode`; PRE-EXISTING since Task 15, undocumented before
  Batch R -- see REGISTRY-1),
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
  `event-section-heading` (batch-n-brief.md, retargeted batch-t-brief.md,
  NARROWED M-D3/U1; small-caps eyebrow, the SAME shared testid every
  section-registry heading in this popover platform uses --
  `catechism-section-heading`'s own class, reused directly, not a fourth
  copy under a fourth name (Batch T retires the pixel-identical
  `narrative-section-heading`) -- rendered on FOUR section bodies today
  (M-D3/U1 retires the narrative PRIOR EVENT/FOLLOWING EVENT headings,
  folding that traversal into a headingless nav row -- see EVENT-1's own
  U1 note -- M-D3/U6 adds "PARALLELS" in their place, a net-even count):
  "EVENT" (a VERSE node's own event-membership list, see EVENT-1),
  "PARALLELS" (a VERSE/PASSAGE node's own OTHER-witness preview, NEW this
  batch, conditional -- see EVENT-1's own U6 note and
  `verse-parallel{-slug}-{SPAN}` in the testid inventory above),
  "PARALLEL ACCOUNTS" (an EVENT node's own witness list, conditional --
  absent for a single-witness event, see EVENT-1), and "PRIOR IN TIME"/
  "FOLLOWING IN TIME" (the GLOBAL-timeline adjacency, UNCHANGED by M-D3/U1
  -- see the GLOBAL TIMELINE note under EVENT-1),
  `verse-event-{eventId}` (batch-t-brief.md; button; one per EVENT-kind
  PASSAGE citing the current VERSE, inside `popover-section-event-membership`
  -- REPLACES batch-n-brief.md's own verse-level PRIOR/FOLLOWING (retired,
  see EVENT-1); clicking pushes a fresh `EventNode` for that event, id-keyed),
  `event-date` (batch-t-brief.md; an EVENT node's own quiet date line,
  non-interactive -- carries the event's own curated `ref_note`, if any, as
  a plain hover tooltip, per requirement 4's own "ref_note provenance on
  hover or a quiet note"; batch-t2-brief.md: present only when this
  passage HAS a date, i.e. `kind == "event"` -- absent for a general-kind
  passage, never a fabricated line), `event-places` (wraps one or more `event-place-{placeId}`
  rows, present only when the event has >=1 resolved place), `event-place-{placeId}`
  (button; opens a `PlaceNode` for that place -- "place opens the place
  node," requirement 4 verbatim), `event-witness-{SPAN}` (one per passage
  entry among an EVENT node's own PARALLEL ACCOUNTS/single-passage list, via
  the shared passage-list component, clamped per PASSAGE-1's own
  `ClampVerses` extension -- see EVENT-1),
  `verse-parallel{-slug}-{SPAN}` (M-D3/U6, NEW this batch; a VERSE/PASSAGE
  node's own "PARALLELS" section -- one passage entry per OTHER witness
  (excluding the one the current verse itself belongs to) of an event the
  verse cites, via the SAME shared passage-list component EVENT-1's own
  PARALLEL ACCOUNTS uses (`WitnessUnitsResolver`, `ClampVerses=2`); `-slug`
  is the event's own label, slugified, present ONLY when the verse
  qualifies through more than one event at once (rare) -- a single
  qualifying event needs no slug at all, same "single entry needs no name"
  rule EventDateAndPlacesSection's own narrative nav (U1) already
  establishes. Conditional presence, doubly: absent when the verse cites
  no titled event at all, and absent per-event when that event turns out
  to have no OTHER witness once the current one is excluded -- see
  VerseParallelsSection's own doc comment),
  `event-nav` (M-D3/U1; wraps the WHOLE narrative prior/following nav --
  one or more `event-nav-row-{idSuffix}` rows, each holding one
  `event-nav-arrows` pair; present iff the event belongs to >=1 narrative
  with a Prior and/or Following on either side -- see EVENT-1's own U1
  note), `popover-event-nav-narrative` (a row's own narrative-name label
  -- present ONLY when the event belongs to >1 narrative; a single
  qualifying narrative needs no name, see EVENT-1),
  `event-prior-event-{narrativeId}` / `event-following-event-{narrativeId}`
  (batch-n-brief.md, retargeted batch-t-brief.md, REBUILT M-D3/U1; button;
  the adjacent event's own label; present once per qualifying narrative
  position in that direction -- a SECOND position sharing the same
  `narrativeId` (a theoretical edge case, guarded defensively -- see
  EVENT-1's own U1 note) gets a numbered `--2`/`--3` suffix, same
  disambiguation shape as `catechism-item-{ID}--q2`; clicking (or Enter)
  traverses -- pushes a fresh `EventNode` re-anchoring the popover onto
  that event, recursively -- see EVENT-1). M-D3/U1 REBUILDS this button's
  own surrounding markup (a compact flanking arrow inside `event-nav`, not
  its own headed "PRIOR EVENT"/"FOLLOWING EVENT" section) -- the testid
  itself, its click behavior, and its recursive-traversal semantics are
  UNCHANGED,
  `event-prior-verse-{narrativeId}` / `event-following-verse-{narrativeId}`
  (M-D3/U1, REPLACES the retired `event-prior-verse-{narrativeId}-{SPAN}`/
  `event-following-verse-{narrativeId}-{SPAN}` passage-list-entry shape;
  a plain, non-explorable quiet caption -- "those foci truncated to ONE
  VERSE," the owner's own words -- carrying the adjacent event's own FIRST
  vref's text only (never the full multi-verse-group preview the retired
  shape showed, and never itself a `VerseNode`/`PassageNode` explorable
  target -- the ARROW beside it is the one explorable affordance for that
  side of the row); present only when that side's adjacent event has
  >=1 verse group with >=1 verse; carries the SAME `--2`/`--3` disambiguation
  suffix as its own `event-{prior,following}-event-{narrativeId}` sibling
  when one applies -- see EVENT-1),
  `event-prior-event-timeline` / `event-following-event-timeline`
  (batch-hotfix4-brief.md requirement 1; button; the GLOBAL-timeline
  adjacent event's own label -- present once per direction, independent of
  narrative membership; conditional presence only at the atlas's own true
  first/last dated event -- see the GLOBAL TIMELINE note under EVENT-1),
  `event-prior-verse-timeline-{SPAN}` / `event-following-verse-timeline-{SPAN}`
  (same requirement; one per passage entry among the timeline-adjacent
  event's own verses, via the shared passage-list component, identically
  to the narrative-scoped verse rows immediately above),
  `polity-delta-verse-{SPAN}` (batch-m-brief.md requirement 4; one per
  passage entry among a `PolityDelta` node's own curated verses, same
  shared-component (PASSAGE-1) treatment as every list above -- nested
  `popover-verse-expand{-ENTRY-ID}`/etc. testids apply identically; absent
  entirely for a delta with zero curated verses, or for the minimal-
  popover case -- see DELTA-1),
  `mini-map`, `mini-map-open-world`,
  `popover-head-actions` (M-D3/U3'; wraps the chip row -- see the REGISTRY-1
  note's own chip-relocation paragraph above; present exactly when
  `popover-chip-*` would be, i.e. whenever the current node offers at
  least one chip), `popover-head-action` (the per-chip button class inside
  it; the SAME `popover-chip-{map,book,context,verse-{VREF}}` testid
  lives on this same element, unchanged),
  `chapter-card-position` (M-D3/U4/B3; a CHAPTER node's own "Chapter N of
  M" line, `M` present only when the book's own TOC total chapter count
  is known -- see B3-CARD below), `chapter-card-verse-count` (plain
  "N verses." line), `chapter-card-headings-heading` (small-caps eyebrow,
  "CONTAINERS IN THIS CHAPTER", present only when >=1 heading anchors in
  this chapter), `chapter-card-headings` (wraps one `chapter-card-heading-
  {eventId}` button per DISTINCT heading container anchored in this
  chapter -- a multi-witness container anchoring more than one verse here
  still contributes exactly one row, deduplicated by `EventId`; opens a
  fresh `EventNode`), `chapter-card-places-heading` (small-caps eyebrow,
  "PLACES MENTIONED", present only when >=1 place is mentioned somewhere
  in this chapter), `chapter-card-places` (wraps one `chapter-card-place-
  {placeId}` button per distinct place mentioned in this chapter,
  deduplicated by place id; opens a fresh `PlaceNode`),
  `chapter-card-xref-total` (plain "N cross-references in this chapter."
  line, present only when the chapter's own verses carry at least one
  cross-reference between them -- a non-explorable aggregate total, not a
  list; there is no single node a chapter-wide xref COUNT could push to)
Notes:
- REGISTRY-1 (batch-r-brief.md requirement 3, "the popover becomes a
  content-first section platform"): the popover body is a composable,
  ordered, CLIENT-SIDE registry of section providers
  (`client/Explore/PopoverSections.cs`'s own `PopoverSectionRegistry`), not
  one fixed per-node-kind fragment -- each provider declares which node
  `Kind`s it applies to and independently answers "does THIS node have
  content for me" (conditional presence: no content -> no
  `popover-section-{id}` at all, not an empty placeholder). VERSE node
  sections, in this order (M-D3/U6, owner verbatim: "Header / Verse
  (focus) / Event / Parallels / Small Catechism / cross references LAST"
  -- REPLACES the pre-M-D3 order this note used to describe: cross-refs
  2nd, catechism 3rd, EVENT membership appended at the very end):
  the verse's own text with its own expand affordance
  (`popover-section-verse-text`, "focus," see READER-1), "EVENT"
  (`popover-section-event-membership`, batch-t-brief.md, conditional --
  absent for a verse citing zero EVENT-kind PASSAGEs -- see EVENT-1;
  REPLACES batch-n-brief.md's own verse-level "PRIOR EVENT"/"FOLLOWING
  EVENT" sections, retired by batch-t-brief.md requirement 3: "rather than
  putting the next/previous event on every verse, add titles of events...
  traversal lives on event nodes," the owner verbatim), "PARALLELS"
  (`popover-section-parallels`, M-D3/U6, NEW this batch, conditional --
  see EVENT-1's own U6 note), "PERSONS" (`popover-section-persons`, Batch
  P, conditional -- moved here from its own former position much later in
  this list, U6's own "Persons then Places" reconciliation; no VERSE-scoped
  "Places" section exists anywhere in this codebase to also reposition --
  see the testid-inventory note above), "THE SMALL CATECHISM"
  (`popover-section-catechism`, batch-f-brief.md, conditional -- absent
  for a verse citing zero catechism items, now capped to 2 shown + U2's
  shared reveal mechanic -- see CATECH-1), and cross-references LAST
  (`popover-section-xrefs`, conditional -- absent for a verse with zero
  recorded cross-references, truncated per XREF-1). PASSAGE nodes get the
  same verse-text/PARALLELS/PERSONS/catechism/cross-references sections
  (aggregating as before) -- NOT the EVENT section (unchanged Batch N
  scope: a shift-click passage span's own per-verse narrative/event
  membership is genuinely ambiguous in a way a single verse never is;
  PARALLELS stays available for a PASSAGE regardless, first-verse-anchored,
  the same convention PERSONS's own mentions already establish for one).
  EVENT node
  sections (batch-t-brief.md; an `EventNode`, reached by a verse's own
  "EVENT" row above, a reader heading, or a narrative-nav arrow, below,
  recursively), in this order: date + place(s) + narrative nav
  (`popover-section-event-date-places` -- batch-t2-brief.md: the date/
  places half is conditional, present only with a date, a resolved place,
  or (M-D3/U1) a narrative-nav entry to show -- the date line renders only
  when this passage `kind == "event"` (never a fabricated line for a
  general-kind one), the place row only if >=1 place resolves; M-D3/U1
  folds the narrative PRIOR/FOLLOWING traversal directly into the TOP of
  this SAME section, `event-nav`, a compact flanking-arrow row rather than
  its own two separate headed sections -- see EVENT-1's own U1 note for
  the full shape/conditional-presence rule), then PARALLEL ACCOUNTS
  (`popover-section-event-witnesses`, conditional heading -- present with
  the "PARALLEL ACCOUNTS" eyebrow only when the event has >=2 witnesses;
  exactly one witness renders the SAME section id as `event-witness`
  (singular) with no eyebrow at all, requirement 4's own "no 'parallel'
  framing when n=1" -- see EVENT-1) -- recursion falls out
  of an EventNode's own narrative-nav arrow pushing ANOTHER EventNode, the
  SAME `AppliesTo` clause matching it too, not a second mechanism. PLACE node
  sections, in this order:
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
  grouped/captioned per PASSAGE-1 -- see CATECH-1). POLITYDELTA node
  sections (batch-m-brief.md; a `PolityDelta` node, reached by clicking a
  delta-eligible border ring -- see DELTA-1), in order: the delta's own
  event prose (`popover-section-polity-delta-event`, conditional -- absent
  for the minimal-popover case), "THE SCRIPTURES"
  (`popover-section-polity-delta-scriptures`, conditional -- one
  `polity-delta-verse-{SPAN}` passage entry per curated verse, per
  PASSAGE-1), the grounding note (`popover-section-polity-delta-grounding`,
  conditional, quiet). CHAPTER node sections (M-D3, U4/B3 -- `ChapterCardSection`,
  reached by `chapter-head` or a reader heading's own container link, see
  B3-CARD below) -- a node `Kind` no OTHER provider claims at all
  (Book/Author/TimeAndPlace/Year) keeps its own PRE-BATCH-R rendering,
  byte for byte -- unaffected by this note. The chips row
  (`popover-chip-map`/`-book`/`-context`/`-verse-{VREF}`) is unchanged,
  pre-existing machinery -- "Explore" (the map affordance) is
  `popover-chip-map`, not a registry section; a CATECHISM node offers NO
  chips at all (no geography -- conditional presence extends to
  affordances too, per CATECH-1). M-D3 (U3') RELOCATES this row from
  below the body to inline beside the title in the popover HEAD (icon-
  scale glyphs, `.popover-head-actions` wrapping `.popover-head-action`
  buttons -- ⌖/❧/¶ for map/book/context respectively, the SAME
  `popover-chip-{map,book,context,verse-{VREF}}` testids, condition, and
  click behavior, unchanged -- a visual/DOM-position move only, disclosed
  here since this note's own "rendered below every section" wording
  predates it; the retired below-body `.popover-chips` CSS class is gone).
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

  batch-n2's own requirement 2 (folded into batch-t-brief.md, which needed
  the identical mechanism for EVENT-kind PASSAGE witnesses -- see EVENT-1 --
  the owner, near-verbatim: "if in hover menu we show passage, we don't
  show more than two verses of the passage at a time, and you can click
  button to expand/collapse that passage, and also read the whole chapter
  like we've already got"): `PassageList.razor`'s own `ClampVerses`
  parameter caps a PASSAGE entry's own COMPACT text at N verses --
  independent of, and orthogonal to, `Cap` above (which counts whole
  ENTRIES; `ClampVerses` clamps WITHIN one entry). A per-entry
  `popover-passage-clamp-expand-{ENTRY-ID}` button (text "Show the rest of
  this passage") reveals the entry's own remaining verses in place;
  `popover-passage-clamp-collapse-{ENTRY-ID}` (text "Show fewer verses",
  `aria-expanded="true"` on the expand button throughout) restores the
  clamped view -- `ENTRY-ID` is ALWAYS present here (unlike
  `popover-verse-expand{-ENTRY-ID}`'s own optional suffix, READER-1: this
  control has no bare, single-instance context the way that one's
  VerseTextSection caller does -- it only ever exists inside a
  `PassageList` entry, so the suffix is unconditional, no empty-string
  case). Visually and testid-distinct from `popover-verse-expand` (italic,
  bronze-ink-resting vs. that control's own small-caps lapis-resting) so
  the two independent affordances on one entry
  -- "expand THIS passage" vs. "read the whole CHAPTER" -- never read as
  the same control; both are keyboard-reachable, and expanding/collapsing
  one never disturbs the other's own state on the same entry. Conditional
  presence: a lone-verse entry, or a passage entry whose own verse count is
  already `<=` `ClampVerses`, shows no clamp affordance at all. Currently
  applied only where a caller passes `ClampVerses` (EVENT-1's own PARALLEL
  ACCOUNTS/single-witness section, `ClampVerses="2"`) -- every other
  `PassageList` consumer (xrefs, THE SCRIPTURES, place est/dest,
  PRIOR/FOLLOWING) leaves it unset (no clamp, unchanged behavior), since the
  mechanism is implemented ONCE, generically, and each caller opts in
  independently.
- TRUNC-1 (batch-hotfix4-brief.md requirement 7, W2 review Important-1:
  "the 20-verse wire cap on witness (book,chapter) verse groups silently
  truncates today -- VerseGroup.Count ships on the wire but the client
  never reads it"). DISTINCT from `ClampVerses` above (a CLIENT-side,
  opt-in, 2-verse compact-text clamp on an ALREADY-fully-delivered
  passage) -- this is the SERVER's own `scene::verse_groups_for` cap
  (`take(20)` per (book,chapter) group, unconditional, every caller),
  which the client previously had no way to even know it hit: the verses
  past 20 were never sent at all. FIX (minimal, per the brief's own
  instruction -- "the full span/lazy presentation redesign is HOTFIX-5's,
  do not build it here"): `PassageListVerse.GroupCount` (client,
  `Explore/PassageBlock.cs`) carries the source `VerseGroup.Count` for
  every EVENT-witness and PRIOR/FOLLOWING (narrative AND global-timeline)
  verse -- the only `PassageList` consumers actually `VerseGroup`-sourced;
  cross-references/THE SCRIPTURES/place est/dest are never capped this way
  (their own verse lists aren't `VerseGroup`s at all) and always carry
  `GroupCount = null`, making this whole mechanism a no-op for them, not a
  per-caller flag. `PassageBlockBuilder` (same file) computes each
  resulting block's own `TruncatedBy` (the true count minus what's
  actually delivered, attributed to the block reaching that group's own
  HIGHEST delivered verse -- the cap always keeps the LOWEST-numbered
  verses, so the missing tail always follows it). WIRED TO THE EXISTING
  MINI-READER, NO PARALLEL AFFORDANCE: `MiniReaderExpand`'s OWN
  `popover-verse-expand{-ENTRY-ID}` button (unchanged testid, unchanged
  click handler, unchanged full-chapter-fetch behavior) reads
  `TruncatedBy` and, when collapsed and `>0`, reads "+{N} more — read the
  chapter" instead of "Read the whole chapter" (`data-truncated="true"` on
  the SAME button, for a robust hook independent of exact wording) --
  clicking it opens the identical full chapter the plain wording already
  did. CONDITIONAL PRESENCE: a group at or under the cap (`GroupCount` ==
  the delivered count, or `null`) shows the ordinary "Read the whole
  chapter" wording, no signal at all -- the SAME "no affordance where
  nothing is missing" discipline every other conditional-presence rule in
  this file follows. NEVER assumed reachable via a general redesign of the
  cap itself -- the cap (`scene::verse_groups_for`'s own `take(20)`) is
  UNCHANGED by this fix; only the client's own honesty about hitting it.
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
  as later batches add providers) is ALSO present. Fewer entries than the
  cap -> no arrow at all (conditional presence). Counts exclude the
  verse-text section itself (`popover-section-verse-text` is the subject
  being read, not context pulled in alongside it).
  batch-f2-brief.md requirement 6b extends the SAME cap/reveal MECHANISM
  (via PASSAGE-1's shared component) to the PLACE popover's own
  established/destroyed supporting verses -- but with an UNCONDITIONAL cap
  of 2 passage entries per date (est and dest each), not context-dependent:
  "the place popover always has sibling sections" (blurb/events routinely
  present alongside dates), so there is no "only kind of context" case to
  distinguish there the way there is for xrefs. See
  `popover-place-date-established-verse-{SPAN}`/`-destroyed-verse-{SPAN}`
  and their own `-more`/`-collapse` pair in the testid inventory above.

  M-D3/U2 REBUILDS the reveal MECHANIC itself (`xrefs-more`/`xrefs-collapse`
  keep their own testids, click behavior, and conditional-presence rule
  unchanged -- this is a mechanic change, not a new affordance): owner,
  verbatim, progress.md: "down: +2 refs; up: -2; double-down: all;
  double-up: collapse to original." RETIRES the all-or-nothing toggle
  ("reveals the rest... all remaining entries at once -- not an
  incremental step," this note's own retired wording) in favor of
  `RevealControls.razor` (`client/Components/RevealControls.razor`), the
  ONE shared mechanic -- "one component/behavior, parameterized -- never
  two implementations," the owner's own words -- now used EVERYWHERE
  `PassageList.razor`'s own `Cap` parameter applies (xrefs AND the place
  est/dest supporting-verse lists above; every OTHER `PassageList`
  consumer -- THE SCRIPTURES, event witnesses, PRIOR/FOLLOWING -- passes
  no `Cap` at all and is structurally untouched, per PASSAGE-1's own "no
  cap asked for" note). A single click on the down arrow reveals `Step`
  (2) MORE, never past the true total; a single click on the up arrow
  collapses `Step` fewer, NEVER below the section's own Default (3 xrefs-
  only / 2 mixed-context / 2 est-dest, per the rules above -- "never below
  the default," the owner's own words). BOTH arrows can render together
  now, in a genuine middle state the retired toggle never had (past the
  first reveal, short of the true total). SHIFT-CLICKING either arrow
  jumps straight to its own far end -- ALL on the down arrow, back to
  Default on the up -- realizing the owner's own literal "double-down"/
  "double-up" wording. DISCLOSED DEVIATION: a real native double-click
  gesture was tried first and found genuinely unsafe, reproducibly, not
  merely in theory -- this popover platform re-centers its own outer panel
  as content height changes (every popover here is viewport- or
  pane-centered), so a dblclick's own SECOND sub-click (aimed at the
  first's now-stale screen coordinate, after the first sub-click's own
  Reveal already grew the panel and shifted it) can land on a
  newly-revealed entry instead of the arrow, pushing THAT node and
  navigating the whole popover away from what the user was expanding --
  observed live, repeatedly. A Shift-click needs exactly one physical
  click/dispatch, structurally immune to this (no second, separately-
  targeted coordinate to go stale). Disclosed via each arrow's own `title`
  tooltip, the same disclosure a dblclick shortcut would have needed
  anyway; the SINGLE-click, fully keyboard-reachable step remains the
  PRIMARY affordance either way. See RevealControls.razor's own doc
  comment for the fuller interaction/accessibility reasoning.

  CATECH-1/U6 (owner: "Catechism defaults to 2 shown"): THE SMALL
  CATECHISM (`CatechismSeamSection`) gets a cap for the first time this
  batch -- previously EVERY citing item rendered unconditionally, no
  reveal mechanism at all. Its own citing-item list is NOT
  `PassageList`-shaped (plain buttons, not passage blocks), so it cannot
  route through that component -- `CatechismList.razor` (a new, genuinely
  stateful sibling to `PassageList.razor`, extracted from
  `CatechismSeamSection`'s own former inline rendering) owns its own
  reveal state and renders `RevealControls` directly, the SAME shared
  mechanic xrefs uses (`catechism-more`/`catechism-collapse`, Default 2,
  Step 2, identical +2/-2/all/default behavior) -- see CatechismList.razor's
  own doc comment. `catechism-item-{ID}`'s own testid/disambiguation/click
  behavior (CATECH-1 below) is otherwise UNCHANGED; only which of them are
  currently rendered is new.
- B3-CARD (M-D3, owner decisions U4/B3, "when you're reading a chapter,
  you're in its focus. you can focus further by clicking chapter heading
  and you get metadata and context... container title, position in book,
  edge summary -- what the graph knows ABOUT the chapter" -- NEVER the
  chapter's own verse text): `chapter-head`'s own popover
  (`ChapterCardSection`, a new `IPopoverSectionProvider` claiming
  `Kind == "Chapter"`, registered in `PopoverSectionRegistry.Providers`)
  replaces the standing "first verse of the chapter appears, that's
  completely pointless" bug (M-D1's own live report #3, quoted in the
  M-D1 REQUIREMENT 2 note above for the SIBLING `pericope-heading` click,
  which was already fixed then -- `chapter-head` itself had no such fix
  before this batch, since `ChapterNode.BodyAsync` had no drill-in
  mechanism of its own to build one on; it is now a minimal, unreachable-
  in-practice defensive fallback, superseded by this section provider).
  Content, every fact read straight off the SAME `ChapterOut` the reading
  view itself already fetched when opened on the chapter currently being
  read (`ChapterNode.AlreadyLoaded`/`.Load`, zero extra network cost for
  that common case): position in book (`chapter-card-position`), verse
  count (`chapter-card-verse-count`), every DISTINCT heading container
  anchored in this chapter (`chapter-card-headings`, conditional,
  deduplicated by event id -- see the testid inventory above), every
  DISTINCT place mentioned in this chapter (`chapter-card-places`,
  conditional, deduplicated by place id), and a plain cross-reference
  COUNT for the whole chapter (`chapter-card-xref-total`, conditional,
  summing each verse's own already-on-the-wire `XrefCount` -- never a
  fetch of its own, and never a list, since no single node a chapter-wide
  total could push to). Headings and places are independently explorable
  (`IPopoverSectionContext.PushAsync`, the SAME drill-in mechanism every
  other section-native row in the popover platform uses) -- "outward
  connections," not a dead-end summary. `chapter-head` opens on hover OR
  click, identically (see the testid-inventory note above) -- the SAME
  "hover and click both open the same popover" rule XSCRIPT-1's own ENTRY
  POINT note establishes for the superscript entry point, applied here
  too, INCLUDING `_hoverOnlyOpen`/`ShowBackdrop`/auto-dismiss (fix round,
  corrected from this note's own original claim that `chapter-head` had no
  auto-dismiss of its own): a real, live-caught bug -- the hover trigger
  originally called the SAME no-parameter open method a click did, which
  never set `_hoverOnlyOpen`, so `ShowBackdrop` stayed permanently true
  even for a hover-only open. Every mouse click is PRECEDED by its own
  hover (the pointer moves onto the element before the click fires), so
  clicking `chapter-head` opened the full-viewport backdrop on the hover
  half of that SAME gesture, and the click half then landed on the
  now-covering backdrop instead of the button -- caught by reader.spec.ts's
  own READ-2c property test stalling for minutes, Playwright's own click
  retrying against a target that kept reappearing then vanishing under it.
  `OpenChapter` now takes the identical `persistent` parameter/
  `_hoverOnlyOpen` wiring `OpenVerseXrefEntry` already established: hover
  opens quietly (no backdrop, auto-dismisses via the SAME `ScheduleHoverClose`/
  `DelayedHoverClose` grace-period mechanism, now generalized off
  `_hoverOnlyOpen` alone rather than an xref-specific node-kind check), a
  genuine click (or Enter -- native `<button>` semantics) upgrades the SAME
  popover to persistent in place.
- XSCRIPT-GATE (2026-08-23, owner order, ledgered; RESOLVED M-D3/R3): the
  superscript feature was briefly GATED OFF by
  `FeatureFlags.XrefSuperscripts = false` (`client/FeatureFlags.cs`) --
  owner verbatim: "just disable superscripts until the rework is
  released." M-D3's rework (click AND hover entry, popover anchored OVER
  the verse -- see XSCRIPT-1's own ANCHORING note below, ALWAYS VISIBLE
  and never cut off at a viewport edge -- see XSCRIPT-ANCHOR-1..4, no
  auto-modal requiring an X -- a hover-only open auto-dismisses on its
  own once the pointer leaves both the marker and the panel, see
  XSCRIPT-1's own ENTRY POINT note below and XSCRIPT-DISMISS-1..4) flips
  the flag back to `true` and un-skips the 8 XSCRIPT tests (plus the 8
  new anchoring/dismiss tests, `tests/ux/reader-xref-anchoring.spec.ts`).
  XSCRIPT-1 below is the binding contract for this now-live state; its
  own ANCHORING/ENTRY POINT paragraphs are updated in place for R3 rather
  than superseded by a separate note, since the gate suspended the
  feature, it never rewrote the contract underneath it.
- XSCRIPT-1 (batch-md2-brief.md; the owner's cross-reference superscript
  directive, batch-x-brief.md verbatim, 2026-08-21: "little superscripts
  visible near verses/passages to which cross references apply... `i,j,k`
  to represent multiple cross references to a single element if there are
  > 3 xrefs, the superscript should be `...`... if you hover over it you
  get another explorable, collapsable/expandable hover menu that shows 3
  explorable verses to start"). Closed on the graph platform (M-D2's own
  controller decisions): a verse's superscript STATE is its `cites`
  edge-summary COUNT at its TextUnit locus (`GraphQuery::edge_summary`,
  design spec §5) -- served inline on `GET /api/chapter/{cref}` as
  `VerseOut.xref_count` (server-side reuse of the SAME generic query `GET
  /api/node/{id}` itself answers, batched onto the existing per-chapter
  response rather than one round trip per verse -- see
  `handlers::chapter`'s own doc comment). `0` -> no marker; `1..=3` -> that
  many lettered superscripts; `>3` -> the many-marker. ONLY verse-attached
  (the original brief's own data note, re-affirmed): headings/passages
  carry no superscript of their own, and verse xrefs are never aggregated
  UP to a heading's own container -- the mechanism (a plain per-locus
  count) does not preclude passage-level attachment later, it simply is not
  wired to one today, since no passage-level xref data exists.

  LETTERING SCHEME: the owner's own literal letters, `i`/`j`/`k` (taken as
  the scheme itself, not an illustrative stand-in for "some sequential
  letters" -- a verse only ever needs 1-3 of them, since 4+ is always the
  many-marker instead). RESTART BOUNDARY: PER VERSE -- "to represent
  multiple cross references to a SINGLE ELEMENT," the owner's own words --
  every verse with 1-3 xrefs independently starts its own lettering at
  `i`; there is no chapter-wide or global counter anywhere (a pure function
  of ONE integer, `Reader.razor`'s own `XrefMarkerText(int count)` --
  `"ijk"`'s own first `count` characters -- with no mutable accumulator to
  declare inside or outside a closure in the first place, which is what
  sidesteps the house pattern's own named hazard, "the shape that bit Batch
  N" -- per-render state captured OUTSIDE a re-invoked `RenderFragment`
  closure -- by construction, not by a guard). MANY-MARKER: the owner's own
  suggested `...`, realized as the single Unicode HORIZONTAL ELLIPSIS
  character (U+2026, `…`) rather than three ASCII periods, for typographic
  correctness. ORDERING: letters map 1:1 onto the wire's own order, which
  is ALREADY votes-ranked (`EdgeMeta::Votes`, design types doc §3) --
  proven, not merely assumed, by
  `server/atlas-server/tests/graph_api.rs::generic_cites_edges_are_already_votes_descending_matching_the_bespoke_verse_endpoint`
  (a real multi-xref verse's own generic `cites` edge page compared,
  position for position, against the bespoke, provably-votes-sorted
  `/api/verse/{vref}` endpoint) -- no client-side re-sort exists or is
  needed.

  PLACEMENT: immediately after the verse's own text (inside `.verse-text`,
  trailing `<sup class="verse-xref-marker">`) -- the brief's own explicit
  "after the verse text/number" allowance, chosen over "after the verse
  NUMBER" specifically because this app's own xref granularity is
  whole-verse (no sub-verse/word-level addressing materializes yet, design
  doc §3), so placing the marker at the verse's own END never implies a
  word-level precision this data does not carry.

  ENTRY POINT (batch-x-brief.md requirement 2, owner's composability law,
  verbatim: "the hover menu... should not have parallel interfaces"):
  hovering (`mouseenter`), keyboard-focusing (`focus`), OR clicking the
  marker all open the SAME `ExplorerPopover` for that verse (a fresh
  `VerseNode` with `XrefEntryPoint=true` -- a PARAMETER on the one
  abstraction, never a second node type or a parallel popover, owner
  decree: "a parallel popover implementation is a defect") -- see
  CAP-RECONCILE-1 immediately below for exactly what that parameter
  changes.

  M-D3/R3 (owner order, "no auto-modal that must be X'd out... hover
  brings it up automatically, so it should also go away automatically
  without needing to click an X"): a HOVER-only open (`mouseenter`/
  `Reader.razor`'s own `OpenVerseXrefEntryHover`, tracked by the private
  `_hoverOnlyOpen` field) now auto-dismisses on its own once the pointer
  leaves BOTH the marker and the popover panel, no explicit close needed
  -- see XSCRIPT-DISMISS-1 (`tests/ux/reader-xref-anchoring.spec.ts`). A
  CLICK or keyboard-FOCUS open (`OpenVerseXrefEntryPersistent`) is, and
  remains, PERSISTENT -- the ordinary backdrop-click/Escape/close-button
  dismissal every other popover already uses, unaffected (XSCRIPT-DISMISS-
  3/4). The two are the SAME popover and SAME entry point, not two
  behaviors bolted together: `_hoverOnlyOpen` is cleared the instant a
  click or focus fires on an already-hover-open marker (XSCRIPT-DISMISS's
  own "hover then click" case), and re-hovering an already-open entry
  point is idempotent (below), so there is no scenario where a genuinely
  persistent popover is ever silently auto-closed.
  Mechanism: leaving the marker (`@onmouseleave`) OR leaving the popover
  PANEL itself (`ExplorerPopover`'s own `OnPanelPointerLeave`, wired
  through `ShowBackdrop`'s sibling parameters `OnPanelPointerEnter`/
  `-Leave`) each schedule the SAME short grace-period close
  (`DelayedHoverClose`, 1000ms -- see that method's own doc comment in
  `Reader.razor` for the two, real, live-caught timing bugs that landed on
  this figure: too tight a window closes the popover mid-transit, whether
  the user is moving the mouse from the marker into the panel, or
  Playwright's own synthetic move+actionability-check sequence is simply
  slower than a real cursor), cancelled the instant the pointer re-enters
  EITHER the marker or the panel. `ShowBackdrop` (a NEW `ExplorerPopover`
  parameter) is `false` for exactly as long as `_hoverOnlyOpen` is true --
  a real, live-caught bug: the pre-existing full-viewport
  `.popover-backdrop` used to render unconditionally the instant the
  popover opened, which (combined with `.reader-page`'s own
  `contain:layout` stacking context) physically intercepted every
  mousedown/mouseup/click aimed at the marker that had just opened it,
  silently re-closing the popover on what looked, from the outside, like
  a plain click; the backdrop now only grows in once the SAME popover
  becomes persistent, by which point there is no marker-click race left to
  intercept. `Reader.razor`'s own `OpenVerseXrefEntry` still no-ops (an
  idempotent open, not a teardown/rebuild) when the popover is ALREADY
  open for the SAME verse's own entry point -- `ExplorerPopover` is keyed
  by `_activeNode` itself (a plain class, no value equality), so an
  unguarded reassignment to an equivalent-but-distinct `VerseNode`
  instance would otherwise tear down and rebuild the whole popover for no
  actual change.

  TESTING NOTE, self-obscuring target: a plain, un-forced Playwright
  `locator.hover()`/`.click()` on the marker still cannot be trusted to
  resolve cleanly -- pre-R3 this was the backdrop (see above) always
  covering the marker the instant it opened; post-R3 a PERSISTENT
  popover's backdrop still does, and even a hover-only popover's own
  panel sits close enough to the marker that a real click's own
  actionability re-check can still land on it mid-retry. `tests/ux/
  reader-xref-superscripts.spec.ts` and `reader-xref-anchoring.spec.ts`
  both use `{ force: true }` on every marker hover/click for exactly this
  reason, disclosed in each file's own header/inline comments -- real
  production behavior, a Playwright automation artifact, not a defect
  either side. A SEPARATE, genuinely PRODUCT-side race exists right after
  an explicit close (`popover-close` click): `ExplorerPopover.RequestClose`
  performs a JS interop call BEFORE it ever nulls the parent's own
  `_activeNode`, so an immediately-following re-open (no intervening wait)
  can occasionally have its own fresh state clobbered a tick later by that
  still-in-flight close -- live-caught via `reader-xref-superscripts.spec.ts`'s
  own "expansion reveals the rest" test, whose second close-then-re-hover
  now waits for `popover` to reach count 0 first, the same guard this
  file's own "entry-point parameter vs F2's general popover" test already
  used one test earlier. Not a bug in `_activeNode`/`OpenVerseXrefEntry`
  itself (confirmed live: the SAME re-hover reliably reopens a fully
  correct popover once that wait is in place) -- a Blazor render-ordering
  hazard around an async close, not a stale-state defect.

  ANCHORING (M-D3/R3, REPLACES the pre-R3 viewport-centered/pane-centered
  reading below): the popover now anchors OVER THE VERSE that opened it --
  horizontally centered on the marker's own verse LINE (not the viewport
  or the pane), vertically flush against that line's own top or bottom
  edge (below by default, flipping above when there is not enough room
  below), clamped so it is ALWAYS FULLY VISIBLE and never cut off at any
  viewport (or, in split view, PANE -- see PANE-ANCHOR-1) edge -- see
  XSCRIPT-ANCHOR-1 (horizontal centering on the line), XSCRIPT-ANCHOR-2/3
  (top-of-chapter / bottom-of-viewport edge cases), XSCRIPT-ANCHOR-4
  (split-view pane edges). Measured live via a small JS interop helper
  (`reader.js`'s own `getVerseAnchorRect`) against the marker's OWN verse
  line element (`#v{n}`, `ExplorerPopover`'s new `VerseAnchorSelector`
  parameter) once per open, not assumed from static layout math -- a real
  DOM measurement, the same "measure once via JS interop, then position"
  pattern the pre-existing pane-anchored mode (F2 requirement 6d) already
  established, applied to a finer-grained target. `popover-verse-anchored`
  and `popover-pane-anchored` are MUTUALLY EXCLUSIVE on the popover PANEL
  itself (verse-anchoring wins outright when both could apply, e.g. a
  superscript opened inside a split-view reader pane) -- a real, live-
  caught bug when they briefly were not: pane-anchoring's own `width` rule
  silently applied underneath verse-anchoring's own centering math, which
  assumes the wider, non-pane-capped base width, producing a popover
  rendered off-screen to the left. Pane-anchoring remains independently
  applicable to the BACKDROP regardless (SPLIT-1's own pane-confinement,
  unaffected). Every OTHER existing popover trigger in this app
  (verse-line click, chapter-head, pericope-heading) is UNCHANGED --
  still viewport-centered full-page / pane-centered while split, F2
  requirement 6d -- this finer anchoring is scoped to the superscript
  entry point alone, the one case where the brief asks for it explicitly
  ("anchored at the superscript").

  JANK GUARD (the brief's own explicit test: "verse line-height/measure
  unchanged"): `.verse-xref-marker`'s own `vertical-align: super` +
  `line-height: 1` (app.css) keeps the marker in normal inline flow (never
  `position:absolute/relative`) without ever inflating `.verse-text`'s own
  line box -- verified live, not merely asserted in CSS:
  `tests/ux/reader-xref-superscripts.spec.ts`'s own `JANK-1` tests compare
  `.verse-text`'s own COMPUTED `line-height` between a marker-bearing and a
  marker-free verse in the same chapter (content-length-independent,
  mechanism-level) and, for a pair short enough to trust as single-line,
  their own rendered `.verse-line` `boundingBox().height` too (the brief's
  own literal ask, a real-world proxy, skipped gracefully rather than
  asserted against a wrapped multi-line verse where content length, not
  jank, would explain any difference). REDUCED MOTION: no rule declares any
  transition/animation on `.verse-xref-marker` at all -- satisfied by
  construction (`.pericope-heading`'s own established precedent for the
  identical reasoning), confirmed live under `prefers-reduced-motion:
  reduce` emulation by the same spec file's own second `JANK-1` test.
  CONTRAST: the brief's own explicit quiet-accent allowance (`>=7:1`, not
  body text's `>=10:1`) -- reuses `--bronze-ink` (ALREADY this exact role
  one level up, `.verse-num`, same `--parchment` background), computed
  (this file's own established WCAG relative-luminance method) at ~7.27:1
  against `--parchment` specifically (`#654A2A`/`#F6F1E5`) -- clears the
  floor with real, if modest, margin. NOT the same 7.85:1 figure
  `.verse-num`'s own `:root` comment elsewhere in this file cites -- that
  figure is against the LIGHTER `--parchment-raised` the POPOVER sits on,
  a materially different background from this darker `--parchment` the
  READER itself sits on (see NAV-4's own note on why the two are not
  interchangeable for a contrast claim); the ~7.27:1 figure here is this
  batch's own independent computation against the correct background.
- CAP-RECONCILE-1 (batch-x-brief.md requirement 2, "CAP RECONCILIATION" --
  owner decree, verbatim: "do not silently break F2... implement it as
  such [a parameter on the one abstraction, NOT a second interface] -- a
  parallel popover implementation is a defect"): `IPopoverSectionContext`
  gains ONE new property, `XrefEntryPoint` (bool; `true` exactly when
  `Current` is a `VerseNode` with its own `XrefEntryPoint=true` -- see
  XSCRIPT-1's own "ENTRY POINT" note), read the SAME way
  `OtherContextSectionCount` already is (INSIDE `CrossRefsSection`'s own
  returned `RenderFragment`, at RENDER time, never captured during the
  concurrent `ResolveAsync` phase -- both are the ONLY two inputs that
  method's own cap decision reads). `Cap` becomes:
  `ctx.XrefEntryPoint ? 3 : (ctx.OtherContextSectionCount > 0 ? 2 : 3)` --
  F2's own xrefs-only-vs-mixed-context rule (2 vs 3) governs the GENERAL
  (non-entry-point) popover byte-for-byte UNCHANGED; the entry-point
  popover shows 3 UNCONDITIONALLY, regardless of whether catechism or any
  other context section is also present (the owner's own words, "shows 3
  explorable verses to start"). Both branches' "3" read
  `client/Explore/EdgeSectionRegistry.cs`'s own `Cites.InitialClamp` -- one
  constant, not two coincidentally-equal literals. ORDER: the entry-point
  popover ALSO reorders its already-resolved section list so `xrefs` LEADS
  (a stable `OrderByDescending` in `ExplorerPopover.LoadCurrent`, over the
  SAME registry-produced list every other open already produces -- one
  small, disclosed special case living directly in that file, the same
  established shape its own map-focus-sync/ShowMiniMap special cases
  already take for a cross-cutting concern the per-node registry has no
  business modeling) -- the GENERAL popover's own section order (verse-text
  first, xrefs second, ...) is unchanged. Proven directly:
  `tests/ux/reader-xref-superscripts.spec.ts`'s own "entry-point parameter
  vs F2's general popover" test opens the IDENTICAL verse both ways in one
  test and asserts BOTH caps in the SAME run, never two separately-
  plausible-but-unconnected assertions.
- CLIENT-ACCESS-1 (batch-md2-brief.md; P7's "CLIENT ACCESS" seam closure --
  design doc §2's own seam inventory: "GAP: the Blazor client still calls
  bespoke endpoints via a concrete AtlasClient. CLOSES AT M-D: the client
  consumes the generic IExplorableClient contract as surfaces migrate").

  THE CONTRACT AS SHIPPED: `client/IExplorableClient.cs` -- three methods,
  1:1 onto the design spec's own two generic endpoints plus the
  text-window endpoint (types doc §6/§8): `Card(string id)` ->
  `GET /api/node/{id}`; `Edges(string id, string kind, int? cursor, int
  limit)` -> `GET /api/node/{id}/edges?kind=&cursor=&limit=`;
  `Reading(string fromRef, int n, string dir)` -> `GET /api/text?ref=&n=&dir=`.
  DISCLOSED SIMPLIFICATION from the design spec's own literal C# sketch
  (`AnyNodeId`/`EdgeKind`/`Cursor`/`Direction`, typed): every parameter is a
  plain STRING, matching how this client already represents every ref
  everywhere else (`VerseNode.Title` IS the vref; no typed ref wrapper
  exists anywhere in `client/` today) and how the wire itself is already
  string-keyed (`graph_wire::encode_node_id`/`EdgeKind::label()`) -- see
  that file's own doc comment for the full reasoning. ONE concrete
  implementation, `client/GraphExplorableClient.cs`, DI-registered as a
  singleton (`Program.cs`) independent of `AtlasClient` -- no caching, no
  retry, no derived state ("client stays thin: no business logic; the
  server's summaries/pages are the truth," the batch brief's own controller
  decision 1). Proven correct in isolation (URL construction + real-wire-
  shape JSON deserialization, no live server needed) by
  `client.Tests/GraphExplorableClientTests.cs` (5 tests, a stub
  `HttpMessageHandler`); the underlying wire contract itself (both generic
  endpoints, live, real compiled data) is proven by
  `server/atlas-server/tests/graph_api.rs`'s own pre-existing suite plus
  this batch's own 3 new tests (see XSCRIPT-1's own "ORDERING" note).

  THE EDGE-KIND SECTION REGISTRY: `client/Explore/EdgeSectionRegistry.cs`
  -- a small, REAL realization of the design spec's own `SectionSpec` shape
  (types doc §7: `{ kind, renderer, style, initial, order }`), keyed by
  edge-kind label. `Cites` is the one entry this batch populates (style
  Quiet/entry-point-capable, `InitialClamp=3`, order VotesRanked) --
  consulted by `CrossRefsSection` for the ONE "3" both its general
  xrefs-only cap (F2) and its NEW entry-point cap (CAP-RECONCILE-1) must
  agree on. A real, live consumer, not a paper interface -- see that C#
  file's own doc comment for the registry's own design.

  STRANGLER INVENTORY (disclosed, per the batch brief's own "surfaces not
  touched this batch may stay bespoke" allowance -- strangler, not big
  bang):

  | Surface | This batch | Status |
  |---|---|---|
  | Superscript xref COUNT (`VerseOut.xref_count`) | Server-side: computed via `GraphQuery::edge_summary` (THE PORT), inline on the existing `/api/chapter` response | MIGRATED -- genuinely NEW capability, generic-native from day one (no bespoke predecessor existed) |
  | Xrefs section display POLICY (cap/order) | Client-side: `EdgeSectionRegistry.Cites` consulted by `CrossRefsSection` | MIGRATED -- a real registry lookup replaces what were two bare integer literals |
  | `IExplorableClient`/`GraphExplorableClient` | Built, DI-wired, unit-tested | SHIPPED, but ZERO live in-app UI call sites THIS batch (M-D2) -- disclosed, not silently hidden; see this batch's own report/concerns. FIRST live call sites land at Batch P (PERSONS-1 below: `VersePersonsSection`/`PersonCardAndMentionsSection`) |
  | Xrefs section DATA FETCH (`CrossRefsSection`'s own preview text) | Unchanged | STAYS BESPOKE (`VerseDetail.CrossRefs`/`PassageNode.XrefsAsync`) -- the generic `cites` edge stores only a target's FIRST verse (design doc §4: "verse-level today, loci by design"); `to_last`/`target_display` (F2's own same-chapter-range full-text enrichment, ~25% of real targets) live on the AUTHORED `CrossRef` row, never lowered into the generic edge index's `EdgeMeta` (only `Votes` is). Migrating this fetch would either silently truncate that ~25% down to one verse (a real regression against F2's own shipped, tested behavior) or require widening `EdgeMeta` -- a relation-shape change "reviewed like any relation change" (graph-types' own law), correctly bigger than this batch's own scope |
  | Every OTHER popover data path (catechism, place, events, PARALLEL ACCOUNTS, traversal) | Untouched | STAYS BESPOKE -- not touched by the superscript work at all, per the brief's own explicit scope |

  Reading the table plainly: the superscript feature itself is fully
  graph-native (count + display policy, both generic); the CLIENT-ACCESS
  seam's OWN structural closure (interface + one implementation + DI
  wiring + tests) shipped as a real, correct, reusable contract for a
  FUTURE batch's own migration to build on, rather than forcing a
  same-batch data-fetch migration that would have cost real, shipped
  fidelity to gain a checkbox.
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
  reverse of a place's curated `verse_links`) for a mention of each linked
  place's own name (see MENTION-1 below for the exact scan rule, shared with
  every other in-text mention surface this batch); the accepted match per
  place becomes `popover-reader-mention-{n}-{placeId}`, hoverable and
  keyboard-focusable. Hovering/focusing one toggles `.atlas-blink`
  (`app.css`) on `placeId`'s own marker CORE (`.atlas-marker`/
  `.quiet-marker`) across EVERY currently-live, non-mini map instance at
  once (map.js's own `blinkPlace`, looping its module-level `instances`
  registry) -- so this works identically whether the live map is the full
  `/world` page's own (a popover opened over it) or a split view's embedded
  atlas pane's own, with no page-specific wiring. A few beats of an
  ember-glow pulse (~1.7s, 3 cycles), then a steady, amplified glow for as
  long as the hover/focus holds; `prefers-reduced-motion: reduce` skips the
  pulse and shows the steady amplified glow immediately instead, never a
  moving animation. M-D3/U5: the SAME mention span is now ALSO explorable
  (click/Enter opens `PlaceNode`) -- see MENTION-1 -- alongside this
  pre-existing hover-blink, not instead of it.
- MENTION-1 (M-D3/U5, "in-text mentions-attested links"): `Explore/
  PlaceMentions.cs`'s `PlaceMentions.Scan(text, places, persons)` -- ONE
  shared mechanism (widened from BLINK-1's own place-only, hover-only
  original) -- splits a verse's rendered text into plain prose and
  entity-mention spans, now covering BOTH `GET /api/chapter/{cref}`'s
  per-verse `places` array AND its new `persons` array (server:
  `GraphService.persons_by_verse`, a startup-assembled index over the
  graph's own `mentions` table, mirroring `places`' "always present,
  possibly empty" shape). Matching is CASE-SENSITIVE
  (`StringComparison.Ordinal`, corrected from BLINK-1's original
  case-insensitive search -- case-insensitive matching made the place "Sin"
  indistinguishable from the common word "sin") AND WHOLE-WORD (the
  character immediately before/after a candidate match, if any, must not
  itself be a letter -- a real, live-caught bug: a bare substring search,
  even case-sensitive, finds "Sin" not only as its own genuine mention but
  also as the first three letters of an unrelated "Sinai" a few words
  later). Longest-match-wins on overlap; ties between a Place and a Person
  candidate at the identical span (a real, confirmed case: GEN.28.1 attests
  BOTH a Place "Canaan" and an unrelated Person "Canaan" for the same
  "...daughters of Canaan" span) resolve to the Place, deterministically
  (a stable sort, places enqueued before persons -- see that file's own doc
  comment for the full disclosure). Still a plain, best-effort text match,
  not a claim of exhaustive recall -- an entity named only by a pronoun, or
  under a curated name the verse's own KJV wording doesn't literally use
  (e.g. GEN.28.19's own "Beth&ndash;el" vs. the curated "Bethel"), is
  simply not detected.
  Rendered in TWO places, the SAME mechanism both times: `popover-reader-
  mention-{n}-{placeId}{-ENTRY-ID}` inside a mini-reader (BLINK-1, above,
  unchanged) AND, new this batch, Reader.razor's own PRIMARY verse text
  (previously plain, unscanned) -- `verse-mention-{n}-{placeId}` (place) /
  `verse-mention-person-{n}-{personId}` (person; mini-reader's own person
  variant is `popover-reader-mention-person-{n}-{personId}{-ENTRY-ID}`).
  Every mention span (both surfaces, both kinds) is now explorable: click,
  or Enter while keyboard-focused, opens that entity's own node
  (`PlaceNode`/`PersonNode`) via `@onclick:stopPropagation`/
  `@onkeydown:stopPropagation` -- the same "more specific target always
  wins" rule `.verse-num`/the xref superscript already establish, so a
  mention click never ALSO opens the verse/passage popover underneath it.
  `PersonNode`'s id carries the graph's wire prefix (`"Person:{id}"`,
  added at the mention-click site -- `VerseOut.Persons`' own id is bare,
  unlike the generic edges page's own already-prefixed ids); `PlaceNode`'s
  stays bare, matching every other `PlaceNode` construction site in this
  app. Place mentions keep BLINK-1's own hover/focus blink alongside the
  new click (independent, both always available); Person mentions have no
  map affordance to blink (`PersonNode` carries no lat/lon) -- click/Enter
  only.
  A real, live-caught TEST hazard, not a product bug (disclosed here since
  it affects how future tests must be written): a plain coordinate
  `.click()` on a `verse-line-{n}` locator clicks that element's own
  geometric center, which can now coincidentally land on one of ITS OWN
  mention spans (stopPropagation) instead of the line itself, opening the
  mention's own node instead of the verse's -- caught live on DEU.5.26's
  own "God" mention. A test that specifically wants "open THIS verse's own
  popover, not a mention within it" should activate via keyboard
  (`.focus()` + `Enter`, `OnVerseLineKeyDown`) rather than a coordinate
  click, sidestepping the geometry entirely; not every pre-existing
  `verse-line` coordinate-click call site in this suite has been converted
  (only the one this batch's own new tests caused to actually fail),
  disclosed as known follow-up risk rather than silently left
  undocumented.
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
- EVENT-1 (batch-t-brief.md, "events as the narrative nodes" -- SUPERSEDES
  batch-n-brief.md's own NARRATIVE-1 note, retired -- owner direction
  2026-08-21, near-verbatim: "you have names for events like 'The Road to
  Emmaus' and 'The Crucifixion at Golgotha'... rather than putting the
  next/previous event on every verse, add titles of events that correspond
  to passages. These events are explorable, part of the graph, have time
  and place data, and you can traverse in time... the narrative is
  traversed by time, which means that the previous/next event is the one
  that is chronologically NEXT, and not necessarily the next event that
  you read (for instance the Gospel of John doesn't have everything in
  order)... add the event titles throughout the reader, and identify
  parallel accounts... internal representation is that the Bible has a set
  of books and a set of passages... this set of passages with their titles
  maps to a mapping of translation to a set of verses"):

  PASSAGE/EVENT DATA MODEL. `atlas_core::data::Event` IS the owner's own
  PASSAGE abstraction: `id`/`label` (this passage's own title -- kept
  under its pre-existing field name, a disclosed decision, not a rename),
  `kind` (`"event"` | `"general"` -- both REAL curated data as of Batch T2,
  see batch-t2-brief.md's own promotion rule: a section ships `"event"`
  only when traditional dating AND a defensible place mapping both exist;
  otherwise `"general"`, the honest default, never a failure), `when`/
  `places` (unchanged since Task 3 for `kind == "event"`; for `kind ==
  "general"`, `Event::when` still holds a structurally-required
  `TimeRange` internally -- `atlas_core::time::TimeRange::undated()`, the
  atlas's own `[-4004,100]` span bounds, never a curator-typed number,
  see that function's own doc comment -- but is OMITTED from the wire
  entirely, never presented to a reader as a real date; `places` stays
  empty by construction), `acts_section` (Batch T2: Acts's own sibling
  provenance field to `robertson_section` below -- owner's own ambiguity
  ruling, verbatim: "acts sections get their own provenance key, NOT
  robertson_section," since Robertson's own 1922 Harmony is Gospels-only;
  counts as a real layer-1 container in `heading_precedence`, identically
  to `robertson_section`; merged onto the FULL combined event set --
  Theographic + events-extra.toml -- by `data/curated/acts-sections.toml`,
  the SAME flat event_id-keyed merge mechanism `event-witnesses.toml`
  already uses, so it can target a bare Theographic-sourced event
  directly with no duplicate `[[event]]` row needed), `atlas_section`
  (Batch W1, "whole-Bible titled verse containers": the GENERAL,
  whole-Bible sibling of `acts_section` -- req 1's own provenance
  vocabulary, "atlas_section (our own sectioning, the sanctioned
  Acts-precedent fallback)" -- used for every book outside the Gospels/
  Acts once W1's own OT sectioning starts, same shape, same layer-1
  treatment, set TWO ways: inline on a brand-new `[[event]]` row in
  `data/curated/passages/*.toml` (one file per book, reusing
  `events-extra.toml`'s own exact `[[event]]` schema), or via
  `data/curated/atlas-sections.toml` (the SAME flat event_id-keyed merge
  mechanism as `acts-sections.toml`, for promoting a bare pre-existing
  Theographic event with no `[[event]]` row of its own to attach to)),
  `kjv_superscription` (Batch W3, "Job-Song of Solomon, Psalms
  granularity": the KJV's OWN literal-citation sibling of the three fields
  above -- req 1's own provenance vocabulary, "kjv_superscription |
  theographic | atlas_section" -- distinct from `atlas_section` because
  this field's own citation IS literal KJV text (e.g. Psalm 3's own "A
  Psalm of David, when he fled from Absalom his son," quoted verbatim from
  PSA.3.1, or Psalm 119's own acrostic Hebrew-letter stanza headers), never
  our own phrasing; same layer-1 treatment; set ONLY inline on a brand-new
  `[[event]]` row in `data/curated/passages/*.toml` -- no merge-file
  sibling to `atlas-sections.toml` exists for it, since Theographic models
  no per-Psalm "event" to promote),
  and PARALLEL
  WITNESSES
  (`witnesses: Vec<EventWitness>` -- "the set of per-book passages that
  recount the same event... one witness passage per Gospel," the owner
  verbatim). Each witness is `{book, translations, ref_note,
  robertson_section}` -- `translations` a REAL `HashMap<String, Vec<String>>`
  (translation code -> flat canonical verse ids), the "mapping of
  translation to a set of verses" the owner's own words ask for, resolved
  fail-loud (`atlas_core::translation::resolve`, unit-tested: KJV present,
  any other code an `Err`, never a silent fallback) -- KJV is the only
  translation this atlas compiles today; the indirection exists so a future
  translation keeps this SAME witness identity (book + span) without
  restructuring. An event with `witnesses` EMPTY (the overwhelming
  majority -- every event this batch does not explicitly curate parallel
  accounts for) has exactly ONE IMPLICIT witness, synthesized server-side
  from its own `verses` grouped by book (`scene::witnesses_for`) -- never a
  fabricated placeholder, and the SAME function the reader-heading index
  and the EVENT popover's own wire both call, so they can never disagree
  about how many witnesses an event has.

  KIND-AGNOSTIC (Batch W1 requirement 1b, "MODEL GENERALIZATION... parallel
  witnesses attach to PASSAGES of any kind, not only EVENT-kind"): nothing
  in this witness machinery -- `scene::witnesses_for`, `handlers::event`,
  `heading_anchors_for`, the client's own `EventWitnessesSection` (keyed
  only on `node.Kind == "Event"`, the CLIENT's own UI-node-kind
  discriminator, distinct from and never checked against the DATA's own
  `Event::kind`) -- branches on `Event::kind` at all; a `kind == "general"`
  passage carrying `witnesses` renders PARALLEL ACCOUNTS identically to a
  `kind == "event"` one (2-verse clamp per witness, per-entry expand,
  single-witness = no parallel framing, all unchanged). Proven at the wire
  level (`event_endpoint_general_kind_with_multiple_witnesses_shows_
  parallel_accounts`, `server/atlas-server/tests/api.rs`) and, live, by
  W1's own req-1b NAMED CASE: `theo-127` ("Ten Commandments Given",
  EXO.19.1-20.17) carries an EXO witness (its own top-level book) and a
  DEU witness (Deuteronomy 5, Moses's own later recitation to the second
  generation at Moab) -- the SAME verse alignment
  `data/curated/catechism-deut5.toml` already establishes per-commandment.

  ONE GRAPH, THREE SURFACES. `Narrative.legs` (an ORDERED chain of event ids
  -- unchanged since Task 3, ETL-validated non-chronological AND, new this
  batch, non-decreasing by `(when.from_year, order_key)` -- `order_key` is
  an explicit SUB-YEAR tiebreak for events sharing one traditional year,
  e.g. Passion Week's own day-of-week ordering, never a fake year offset)
  is the single source EVERY surface reads: `scene::build_arrows` walks it
  to build a time/scripture-mode scene's own `arrows` (the map's own
  threads); `atlas_core::narrative::positions_for_events` (Batch N, UNCHANGED
  by this batch -- its own leg-array-adjacency walk was already exactly
  "chronologically adjacent given a validated leg order," so nothing about
  the resolver itself needed to change) walks the SAME `legs` to find an
  event's own immediate PRIOR/FOLLOWING neighbors; and the READER HEADING
  index (`AtlasData::heading_for_verse`, new) walks `events` directly to
  anchor a title above each witness's own first verse. All three read
  `scene::to_scene_event`/`scene::witnesses_for` for "an event's own verses
  on the wire" -- PROVABLY the same data (see
  `atlas_core::narrative`'s own
  `adjacent_event_verse_groups_equal_the_map_arrows_own_scene_event` test,
  which `assert_eq!`s two independently-derived values).

  WIRE. `GET /api/verse/{vref}`'s own PRE-EXISTING `events` array (id/label/
  verse_groups/places, unchanged since before Batch N) is what the VERSE
  popover's new "EVENT" section reads -- "which titled EVENT-kind PASSAGE(s)
  does this verse belong to," a DIFFERENT question from Batch N's own
  retired `narrative_positions` field (which answered "which chronological
  POSITION," now irrelevant at the verse level since traversal moved
  entirely to the EVENT node). `GET /api/event/{id}` (new) is an EVENT
  node's own rich fetch: `id`/`title`/`kind` (Batch T2, ALWAYS present) /
  `when`/`places` (id+name pairs) / `witnesses` (ALWAYS >=1, see the
  data-model paragraph above) / `robertson_section`/`acts_section`/
  `atlas_section`/`kjv_superscription` (Batch T2/W1/W3, Acts's, the
  whole-Bible's, and the KJV's own literal-citation sibling fields -- see
  the data-model paragraph above)/`ref_note` (each omitted, not null, when uncurated). Batch T2: `when` is OMITTED (not
  null) when `kind == "general"` -- the fabrication guard extends to the
  wire itself, not just the curated source (see the data-model paragraph
  above); `places` is always present, possibly empty (a general-kind
  passage's own `places` is empty by construction, same "array present,
  conditional presence is a client concern" convention every other list
  on this DTO already follows).
  `GET /api/narrative/event/{id}` (Batch N, route UNCHANGED; WIRE SHAPE
  EXTENDED Batch HOTFIX-4 requirement 1 -- see the GLOBAL TIMELINE note
  below) is still the chronological PRIOR/FOLLOWING source -- only its
  CALLER changed (`EventNode` replaces the retired `NarrativeEventNode`).
  `GET /api/chapter/{cref}`'s own per-verse `heading` field (new,
  `{event_id, title, kind, is_continuation}` -- `is_continuation` M-D1,
  see below -- omitted when this verse is not a heading ANCHOR) is what
  Reader.razor reads to place a `pericope-heading-{eventId}` directly in
  the reading flow.

  M-D1 REQUIREMENT 2, CONFIRMED (owner live report #3, 2026-08-21,
  verbatim: "when you click on a chapter heading, the first verse of the
  chapter appears. that's completely pointless."): a heading click MUST
  open the CONTAINER's own node (its card + frontier), never a bare verse
  popover. LIVE-VERIFIED at this batch's own BASE (b5b3949, pre-dating any
  M-D1 change) already correct: `Reader.razor`'s own heading markup
  constructs `new EventNode(heading.EventId, heading.Title)` directly
  (`OpenEvent`, unconditional for both event-kind and general-kind
  headings) -- the SAME code path `tests/ux/reader-headings.spec.ts`
  already exercised (`popover-title` == the container's own title after a
  heading click, an EventNode popover, never a verse's). No regression
  found or fixed; this batch's own CHAPTER-BOUNDARY CONTINUATION headings
  (above) reuse the IDENTICAL click/keydown wiring -- one mechanism, every
  heading kind, primary or continuation.

  M-D1 SPAN-NOT-ECHO (requirement 3, owner live report #4, 2026-08-21,
  verbatim: "it also is completely redundant to just show the verses
  associated with a container in the container's hover box. we should
  just see the passage span."). A container's OWN popover (the
  PARALLEL-ACCOUNTS-style section every EVENT node's own witnesses render
  through, `EventWitnessesSection`) previously echoed the full verse text
  of a SINGLE-witness container -- the overwhelming majority of events --
  identically to a multi-witness one (clamped-to-2 + expand), redundant
  with the text the reader is already looking at. FIX: `PassageList.razor`
  gains a `SpanOnly` parameter -- when true, an entry renders its own ref-
  label SPAN only (no compact text, no clamp toggle); the ONLY affordance
  left is the shared `MiniReaderExpand` "Read the whole chapter"
  mechanism, reused unchanged, never reimplemented. `EventWitnessesSection`
  sets `SpanOnly = !multi` -- a SINGLE-witness event (n=1, `units.Count ==
  1`) renders its span line only; a MULTI-witness event's own PARALLEL
  ACCOUNTS list is UNCHANGED (every witness -- this book's own included --
  keeps its real clamped, expandable text; the Crucifixion's own 4-Gospel
  case stays exactly as rich as before). SCOPED, disclosed: the
  requirement's own further nuance -- "where a multi-witness event's
  popover currently repeats the CURRENT book's own text, the current
  book's witness collapses to its span line too" -- is NOT implemented
  this batch (no "what book is the reader currently reading" context
  currently threads through the popover-section-provider architecture,
  and building that thread is a bigger architectural change than this
  batch's own "client stays thin, existing wire shapes" scope); the
  concretely-named acceptance (single-witness -> span; multi-witness ->
  unchanged) is what red-then-greens (`tests/ux/popover-sections.spec.ts`'s
  own two M-D1 req 3 tests). `PlaceCard.razor`'s own map-hover MergedVerses
  preview is DELIBERATELY untouched -- a materially different, pre-
  existing, already-disclosed "different truncation model" (PassageList's
  own header comment), not a container node's own popover at all; map-side
  users reach a container's own span (and its inline expand) the identical
  way reader-side users do, by opening that container's own EventNode.

  M-D1 TRUNCATION AUDIT (requirement 4, owner priority statement
  2026-08-21: truncating hover-menu output "we need that to improve user
  experience," ranked ABOVE new features) -- POLICY TABLE, swept across
  every popover/hover section in the app:

  | Section | Cap mechanism | Compliant? |
  |---|---|---|
  | Cross-references (`CrossRefsSection`) | `PassageList.Cap` = 3 (xrefs-only) / 2 (mixed context), F2 req 6 | YES, pre-existing |
  | THE SCRIPTURES (`CatechismScripturesSection`) | uncapped (`Cap=null`), DELIBERATE -- the whole small catechism's own proof-text set per item is small and bounded by curation, "no cap asked for" | YES, disclosed exemption |
  | Place est/dest supporting verses (`PlaceDatesSection`) | `PassageList.Cap` = `SupportingVersesCap` (2) | YES, pre-existing |
  | PARALLEL ACCOUNTS, per-witness text (`EventWitnessesSection`) | `PassageList.ClampVerses` = 2 per entry | YES, pre-existing |
  | PARALLEL ACCOUNTS, witness COUNT | uncapped, DELIBERATE -- bounded by construction (how many KJV books narrate one event; realistically <=4-6, the four Gospels' own ceiling) | YES, disclosed exemption |
  | Container's own span (`EventWitnessesSection`, n=1) | span-only, no text by default (M-D1 req 3, above) | YES, this batch |
  | THE SMALL CATECHISM seam, citing items (`CatechismSeamSection`) | uncapped, DELIBERATE -- the whole catechism is 33 items; a single verse citing all of them is the real ceiling, never large | YES, disclosed exemption |
  | Verse's own EVENT membership (`VerseEventMembershipSection`) | uncapped, DELIBERATE -- bounded by construction (overlapping containers per verse is a small integer; W5's own coverage is largely partition-like, never approaching a wall of text) | YES, disclosed exemption |
  | A place's own EVENTS list (`PlaceEventsSection`) | **was uncapped -- a REAL, live-verified gap** (Jerusalem: 236 located-at events across the whole atlas, unbounded by any time window, zero disclosure) | **FIXED this batch** -- `PlaceEventsList.razor` (new component, mirrors `PassageList`'s own down-arrow reveal exactly), cap 10, `place-events-more`/`-collapse` |
  | Reader chapter fetch, per-verse-group truncation (`scene::verse_groups_for`) | server `take(20)`, `GroupCount`/`TruncatedBy` disclosed to `MiniReaderExpand`'s own "+N more" label | YES, pre-existing (HOTFIX-4 req 7) |
  | Map hover preview (`PlaceCard.razor` `MergedVerses`) | Batch D's own reveal-by-5/2-at-a-time mechanism, pre-existing, deliberately untouched (a different truncation model -- see SPAN-NOT-ECHO above) | YES, pre-existing, out of scope |

  Only ONE real gap found: the place-events list, fixed. Every other
  section either already had a real cap with honest "+N more" disclosure,
  or is correctly EXEMPT because its own real-world ceiling is small and
  bounded by construction (never by hope) -- disclosed here rather than
  silently assumed. `tests/ux/popover-sections.spec.ts`'s own
  `REGISTRY-1` test is the live, real-data red-then-green for the one fix
  (Jerusalem's own 236-event list, capped + revealed).

  HEADING-WORTHY RULE (server: `atlas_graph::heading::build_heading_index`,
  the live production path as of M-C2's decisive-title-law migration --
  kept in lockstep with `AtlasData::finish`'s own original implementation,
  which stays the tested reference oracle for surfaces that haven't
  migrated; fix round 1, M-1). An event anchors a
  reader heading -- one per WITNESS, at that witness's own first verse, in
  THAT witness's own book -- exactly when it is a leg of one of the 13
  curated narratives (OT included; every existing narrative event already
  carries a real title via `Event::label`, so this needed zero new
  authoring for those), OR was explicitly curated with `witnesses`, OR
  carries a `robertson_section`, OR carries an `acts_section` (Batch T2),
  OR carries an `atlas_section` (Batch W1 -- the general, whole-Bible
  sibling of the two -- see the data-model paragraph above; fixed in this
  same commit alongside the code, correcting THIS paragraph's own
  pre-existing omission of `acts_section`, which `AtlasData::finish`'s own
  code already checked since Batch T2 but this prose never named), OR
  carries a `kjv_superscription` (Batch W3 -- the KJV's own
  literal-citation sibling of the three -- see the data-model paragraph
  above).
  Originally realized the owner's own coverage decision ("Gospels-first...
  PLUS every event in the existing 13 narratives... General-passage titles
  outside these come later"); Batch W1 begins that "outside these" work.
  Still no separate curated flag needed: a Theographic event no batch has
  yet touched, and that is a leg of no narrative, correctly anchors NO
  heading anywhere.

  DECISIVE-CONTAINER MODEL, COLLISION PRECEDENCE (fix-round-1,
  batch-t-review.md Important-1, amended by the owner's own 2026-08-21
  ruling -- verbatim: "we don't modify the verses because we lose
  composability... these titled passages are containers for verses...
  identity is empty set. the set may have n verses. there may be sets
  that have overlapping verses as we grow the dag, which is fine, but
  we're decisive about the titles of verse groupings that we display on
  the reader"). `Event` IS a titled CONTAINER over a verse SET, never the
  reverse -- headings are built by iterating containers and emitting
  anchors from their own content, never by writing heading data onto a
  verse record (there is no such record to write onto; verses stay plain,
  immutable atoms). The empty set is a lawful container identity. TWO
  containers legitimately overlapping the same verse is an EXPECTED data
  shape as the graph grows, not an error -- a real, live case:
  `jm_bethany`, a bare `jesus-ministry` leg (heading-worthy only via the
  "existing-title freebie" narrative-leg rule, no witnesses/
  robertson_section of its own), and `pw_bethany`, a REAL curated
  container for this exact grouping (this batch's own flagship 3-witness
  passion-week leg), both anchor JHN.12.1. Overlap lives in the DATA; the
  READER is decisive -- exactly ONE title heads any displayed verse
  grouping, chosen by `AtlasData::heading_precedence`'s own 3-tier rule,
  never by incidental file/vec order:
  1. LAYER -- a REAL container (curated `witnesses` non-empty and/or
     `robertson_section`/`acts_section`/`atlas_section`/`kjv_superscription`
     present -- Batch W1 added the third, Batch W3 the fourth; all four
     count identically, see the data-model paragraph above) beats a bare
     "freebie" container (heading-worthy
     only because it happens to be a narrative leg riding its own
     pre-existing `Event::label`). Decides every real collision in
     today's curated data outright (`pw_bethany` real container beats
     `jm_bethany` freebie).
  2. KIND -- `"event"` beats `"general"`, a tiebreak reached only when both
     colliders are genuinely same-layer containers -- LIVE as of Batch T2
     (general-kind passages are now real curated data), though still rare
     in practice: curated sections are expected to PARTITION (see
     WITHIN-LAYER ANCHOR COLLISIONS below), so two same-layer containers
     colliding at all is uncommon.
  3. CHRONOLOGY -- the earlier `(from_year, order_key)` wins, the SAME
     tuple `Narrative.legs`' own ordering already uses -- reached only
     between two real containers of the same kind, not observed anywhere
     in today's curated data.
  Two containers equal on all three tiers keep plain first-wins (stable
  sort order), unchanged from before this fix -- not expected to ever
  actually occur for two distinct real events. The NON-CHOSEN container is
  never unreachable: the verse's own EVENT membership section
  (`events_for_verse`, below) always lists every container touching that
  verse regardless of which one won the heading, so it stays one click
  away at the same verse -- only the one-per-verse reader-HEADING slot is
  contested, never the underlying graph. Every verse in the Bible
  ultimately belonging to at least one titled container is the owner's own
  stated end-state; at the time this paragraph was first written (Batch T
  fix-round-1), Gospels+Acts+13-narratives coverage was a scoped subset,
  not the destination, and the full migration was real future work, not
  that fix.
  ACHIEVED (Batch W5, the whole-Bible titled-verse-container series' own
  fifth and FINAL run): the full migration this paragraph once deferred is
  now real, shipped data, not future work. `data/curated/coverage-
  manifest.toml` declares all 66 canonical books; every one of the compiled
  KJV's own 31,102 verses belongs to >=1 titled container, independently
  verified against the real compiled `canon.json`/`events.json` (never a
  hand-typed count) by `server/atlas-etl/tests/coverage.rs`'s own
  `every_canonical_book_is_declared_and_the_whole_kjv_is_fully_covered`
  test, which reads the real compiled data fresh on every run rather than
  trusting this sentence. The five W-series runs that reached this state:
  W1 (Genesis-Deuteronomy plus Joshua-Ruth), W2 (1 Samuel-Esther), W3
  (Job-Song of Solomon), W4 (Isaiah-Malachi, completing the whole Old
  Testament), W5 (Romans-Revelation, completing the whole Bible) -- see
  each batch's own report (`batch-w1-report.md` through
  `batch-w5-report.md`) for its own coverage table, kind/provenance split,
  and reconciliation notes. This does not change the DECISIVE-CONTAINER
  MODEL or the 3-tier precedence rule stated above -- both already applied
  correctly to the partial coverage that existed when they were written,
  and apply identically now that coverage is total.

  M-D1 CANONICALLY-FIRST ANCHORING + CHAPTER-BOUNDARY CONTINUATION (owner
  live report #2, 2026-08-21, verbatim: "genesis 6 the first verses have
  no container label... i'm assuming this isn't an isolated case"). ROOT
  CAUSE (controller-verified live, both before and after the graph
  migration -- `atlas_graph::heading::heading_anchors_for` mirrored the
  identical bug): `heading_anchors_for` anchored each container at the
  FIRST verse in CURATED/IMPORTED ARRAY order, never the canonically first
  (reading-spine) verse -- `theo-32` ("God decides to destroy every living
  thing") covers GEN.6.1-7, but its original Theographic verse link was
  GEN.6.7 and W1's own enrichment pass APPENDED 6.1-6 afterward, so the
  heading rendered at verse 7 and GEN.6.1-6 read unlabeled. Systemic for
  any enriched container whose original link was not the passage's own
  first verse -- the witness branch (`.find(first parseable)`) carried the
  identical defect. FIX (both `atlas_graph::heading` -- the live production
  path -- and `atlas_core::data`, kept in lockstep per that module's own
  doc comment): the anchor is now the group's own CANONICALLY FIRST
  covered verse -- minimum by (book, chapter, verse) -- computed by hand
  (VerseId carries no derived `Ord` of its own; widening a shared type's
  derive list for one law is a bigger blast radius than a local comparator
  key), never merely positional. Named case, real data:
  `server/atlas-graph/tests/heading_precedence.rs`'s own
  `named_case_gen_6_1_anchors_canonically_first_not_curated_import_order`
  (RED pre-fix: anchored GEN.6.7; GREEN post-fix: GEN.6.1) +
  `tests/ux/reader-headings.spec.ts`'s own matching UI-facing case.

  CHAPTER-BOUNDARY CONTINUATION (same report, "no covered chapter may open
  with unlabeled verses"): when a container spans multiple chapters (first-
  class, container-algebra-legal), its own coverage crossing into a LATER
  chapter at exactly that chapter's own opening verse (verse 1, always the
  chapter boundary in KJV versification -- no canon lookup needed) now
  renders a CONTINUATION heading there -- same title, same click-through
  (M-D1 requirement 2), styled with a quiet WORDING marker ("continued")
  and a step-smaller type size (app.css's own established "quiet via
  SCALE, distinct via WORDING" precedent, never a color-only cue on
  must-stay-readable content). TWO-PASS, never contested:
  `atlas_graph::heading::build_heading_index` resolves every PRIMARY
  anchor first (unchanged collision rule); a SECOND pass then fills
  CONTINUATION candidates only at verses the first pass left unclaimed --
  a primary anchor is an ABSOLUTE, unconditional win over any continuation,
  so "exactly one label per verse" (the decisive-container law) holds
  identically with continuations in play. Competing continuation
  candidates at one still-open verse resolve by the SAME layer/kind/
  chronology/id precedence tuple primary anchors use. Named case, real
  data (`ezr_temple_completed`, EZR.5.1-17 + EZR.6.1-22 -- two chapters,
  one container): `heading_precedence.rs`'s own
  `named_case_ezr_temple_completed_spans_chapters_5_and_6_with_a_continuation_heading_at_6_1`
  (RED pre-fix: EZR.6.1 rendered no heading at all; GREEN post-fix: a
  continuation, same title as EZR.5.1's own primary) +
  `reader-headings.spec.ts`'s own matching UI-facing case (`.pericope-
  heading-continuation`, `data-continuation="true"`, the `continued`
  marker span). Wire: `HeadingOut.is_continuation` (always present once a
  heading exists at all, `false` for every ordinary primary heading);
  `atlas_core::data::HeadingEntry` carries no continuation field at all --
  this law is graph-side only (that struct's own doc comment discloses
  why: a dead, tested reference oracle for the base anchor law, never a
  live consumer this new law needs to reach).

  EVERY-COVERED-CHAPTER ASSERTION (the owner's own brief-level acceptance,
  "an every-covered-chapter assertion over the graph, all 66 books now"):
  `heading_precedence.rs`'s own
  `m_d1_every_chapter_with_heading_worthy_coverage_opens_with_a_real_heading`
  walks the REAL compiled graph's own `Event` nodes -- never a hardcoded
  book/chapter list -- derives every chapter touched, AT ITS OWN OPENING
  VERSE, by >=1 HEADING-WORTHY container (a narrative leg, or a real
  curated container -- the SAME predicate `build_heading_index` itself
  applies), and asserts each one resolves to a real heading (primary or
  continuation) in the built index. SCOPE, precisely, matching the
  HEADING-WORTHY RULE above: a bare Theographic freebie that is a leg of
  no narrative and carries no curated witnesses/section is, correctly,
  "anchor[ing] NO heading anywhere" (the pre-existing rule, unchanged) --
  its own chapters are a SEPARATE, disclosed curation-coverage question
  (W-series book-by-book authoring), not this requirement's own defect
  class. Real, live-caught during this test's own authoring (not a
  hypothetical): `theo-162`'s TOP-LEVEL `verses` field still carries its
  full raw Theographic import span (1 Kings 5:1-7:51) even though its own
  CURATED `witnesses` were deliberately narrowed to chapter 6 alone (that
  event's own `ref_note` -- avoiding a within-layer anchor collision with
  the finer `1ki_hiram_temple_prep`/`1ki_temple_furnishings` siblings) --
  the production law correctly reads ONLY the witness branch once real
  witnesses exist (never both), and this test's own independent
  derivation had to mirror that exact branching to avoid a false claim.

  WITHIN-LAYER ANCHOR COLLISIONS (Batch T2, owner's own ruling: "Robertson
  sections within one Gospel should partition, not collide with each
  other -- a within-layer anchor collision is a curation error your
  validation must catch"). Distinct from the 3-tier precedence rule just
  above, which decisively RESOLVES a collision for display and is fine
  with a layer-1-vs-layer-0 one (the `pw_bethany`/`jm_bethany` case is
  legal DATA, just decisively rendered). A collision between TWO real
  (layer-1) containers is a DIFFERENT thing: correctly-partitioned curated
  sections should never both claim the identical anchor verse in the first
  place, so `atlas_etl::validate::run` fails loud on every such pair
  (`AtlasData::heading_anchor_collisions`, `server/atlas-core/src/data.rs`)
  rather than silently letting tier 2/3 pick a winner -- a real one
  surfaces a curation mistake in THIS batch's own section boundaries, to
  be fixed in the data, not the rule (see batch-t2-report.md for whether
  any fired during authoring).

  EVENT-MERGE, DUPLICATE-IDENTITY RECTIFICATION (batch-hotfix4-brief.md's
  own coordinator amendment, owner live report 2026-08-21: "the ordering of
  the narratives is wrong. the temptation of Jesus in the wilderness, for
  instance, is labeled as being before Jesus' baptism. this is a straight
  up lie"). DISTINCT from WITHIN-LAYER ANCHOR COLLISIONS above (which
  arbitrates which of two REAL containers wins a HEADING) and from the
  ordinary layer-1-beats-layer-0 case (which arbitrates the SAME thing for
  a real container vs. a freebie) -- this is a THIRD, prior concern: TWO
  ids for the SAME real-world event, one a bare Theographic freebie on its
  own approximate scale, one a real curated container on this atlas's own
  AD-33-anchored scale, BOTH still existing as independent graph nodes.
  `heading_precedence` already made the real container win the HEADING at
  any shared verse -- but HOTFIX-4 requirement 1 (whole-DAG chronological
  traversal) makes every dated event a real, independently-reachable node,
  so the freebie stayed one click away, on the WRONG scale, silently
  reachable from any verse it shares with its own richer twin -- exactly
  how a Theographic-dated "Temptation" (AD 26) could sort before an
  AD-33-anchored "Baptism" (AD 29). FIX: `atlas_core::event_merge`
  (mirrors `atlas_core::merge`'s own same-place pattern) -- a curated
  `EVENT_MERGE_PAIRS` table (`{survivor, absorbed, reason}`, 63 pairs,
  verse-set Jaccard overlap >=0.8 against ANY real container found by an
  automated sweep, plus one pair the owner named by hand below that floor)
  applied by `apply_event_merges` in `AtlasData::finish()`, immediately
  after the place merge and before every derived index -- `absorbed` is
  removed from the compiled graph entirely; `survivor`'s own fields
  (label/when/order_key/verses/witnesses/places/provenance) are NEVER read
  from or written by this pass (IDENTITY-ONLY, never a content union) --
  satisfying BOTH the owner's own container-algebra law (progress.md
  "OWNER DIRECTIVE" -- verses stay immutable, never annotated; only a
  duplicate CONTAINER RECORD disappears) and the amendment's own rule B
  ("the superseded scale is not preserved in shipped data") by
  construction. `EVENT_DISTINCT_PAIRS` documents every pair the automated
  sweep also found (same threshold) that is NOT a clean 1:1 duplicate --
  a Theographic MEGA-SPAN bundling two or more separately-curated
  pericopes (merging into either would misattribute the other's own
  citation) or a real duplicate outside this batch's own Gospel-era scope
  (disclosed, deferred, not silently dropped) -- so the validator below
  never re-flags either class every run. FAIL-LOUD VALIDATOR
  (`atlas_etl::validate::run_event_merges`, called from `main.rs` on the
  RAW pre-finish event set, same timing as `run_place_merges`): sweeps
  EVERY (layer-0, layer-1) event pair in the whole compiled set at
  verse-set jaccard >=0.8 -- any pair found that is in NEITHER
  `EVENT_MERGE_PAIRS` nor `EVENT_DISTINCT_PAIRS` fails the ETL, naming
  both ids/labels/the score, so a future curator's own new near-duplicate
  event is caught immediately rather than silently shipped. Red-then-green
  proven on the Baptism pair's own pre-merge shape (server/atlas-core/src/event_merge.rs's
  own `red_then_green_baptism_pair_collapses_to_one_event_on_the_ad33_scale`
  test). One survivor (`jm_jordan`) was additionally given real, individually
  KJV-verified Mark/Luke witness rows (`data/curated/event-witnesses.toml`)
  so the Baptism keeps the same 3-Gospel PARALLEL ACCOUNTS richness its
  own absorbed freebie evidenced, rather than regressing to Matthew-only.

  ALIASING (HOTFIX-4 fix round 1, review finding M-1, doc-only ruling): an
  `absorbed` id does NOT stay resolvable as its own node -- `GET
  /api/event/{absorbed_id}` 404s, exactly matching `atlas_core::merge`'s
  own same-place precedent (an absorbed PLACE id doesn't resolve either).
  Only `Narrative.legs` entries naming an absorbed id are repointed to
  `survivor`, at load (`apply_event_merges`) -- nothing else in the data
  model holds a stale event id (verified; normal client navigation never
  persists one). Amendment A's own "id aliasing so old ids stay resolvable"
  wording is superseded by this ruling.

  M-D1 REMAINING-DUPLICATES RECTIFICATION (owner live report #6's own
  tail + the two HOTFIX-6-era parked pairs, "rectify those kinds of
  problems" applied to the THREE pairs the original HOTFIX-4 sweep left
  parked in `EVENT_DISTINCT_PAIRS`, deferred rather than merged): all
  three now MERGED, via this SAME `EVENT_MERGE_PAIRS`/`apply_event_merges`
  mechanism -- no separate graph-adapter-level merge pass exists or was
  needed, since the graph is built FROM the already-merged, post-`finish()`
  `AtlasData` (P1: one artifact). `pr_rome`/`theo-384` (CROSS-BOOK, title
  jaccard 1.000, both LAYER-0 -- neither the layer0-vs-layer1 verse-jaccard
  sweep nor a within-layer comparison would catch this shape; `pr_rome`
  survives as this atlas's own narrative-integrated identity, on the
  atlas's own established AD-60 scale). `theo-338`/`theo-337` (a verse-set
  PREFIX, 5 of 79 verses, jaccard 0.063 despite full containment; the
  broader, verse-complete `theo-338` survives, absorbing its own redundant
  lead-in freebie). `ezr_altar_and_foundation`/`ret_jerusalem_altar` (below
  BOTH existing detectors' own floors -- verse jaccard 0.143, title jaccard
  0.286 -- found only by HOTFIX-6's own live chronology audit; the real
  curated, `atlas_section`-provenanced `ezr_altar_and_foundation` survives,
  absorbing the bare `return`-narrative freebie, whose own leg repoints to
  the survivor automatically via ALIASING, above). ORDER-DIVERGENCE POLICY
  (owner directive, corrections are lawful data changes w/ justification):
  the `ezr_altar_and_foundation`/`ret_jerusalem_altar` merge is the one
  pair of the three with a real date shift (-537 -> -536, the survivor's
  own already-correct dating for "the second year of their coming," Ezra
  3:8) -- checked against `server/atlas-graph/tests/timeline_equivalence.rs`
  (the graph-vs-old-resolver total-order equivalence test) and
  `narrative_real_data.rs`'s own `amendment_d_monotonicity_audit`: both
  compare independently-derived orders over the SAME live, already-merged
  data, so removing an id changes nothing about how either order agrees
  with itself -- no typed expected-divergence entry was needed for this
  batch's own three pairs (verified, not assumed: full suites green
  unchanged after the merge). Full real-data red-then-green:
  `server/atlas-graph/tests/narrative_real_data.rs`'s own
  `m_d1_the_three_remaining_duplicate_pairs_are_rectified_on_the_real_graph`.

  NT CALIBRATION (HOTFIX-4 fix round 1, review finding C-1, Critical --
  owner's own "straight up lie" bug, still live for content this rectification
  pass above never touches: the Gospel MEGA-SPANS `EVENT_DISTINCT_PAIRS`
  correctly leaves un-merged, plus the ~33 real, curated `acts_section`
  Acts events (`data/curated/acts-sections.toml`), plus any other surviving
  `theo-*` event with NT-book verses -- ALL still on Theographic's own
  internal NT clock, ~3 years ahead of this atlas's AD-33 Passion anchor,
  e.g. Pentecost sorting before the Crucifixion). RULING: re-date, never
  exclude (excluding would manufacture new dead ends, forbidden by the
  owner's own traversal law). `atlas_core::nt_calibration::apply_nt_calibration`
  -- ETL-ONLY (`atlas_etl::main`, on the RAW pre-`finish()` event set,
  NEVER from `AtlasData::finish()` itself, which runs twice across the real
  pipeline and would double-shift a raw date delta -- see that module's own
  doc comment) -- shifts every `theo-*` event with `from_year > 0` and
  >=1 NT-book verse (`canon::BOOKS`, Matthew..Revelation) by a flat +3
  years (Theographic's own internally-consistent NT clock, verified
  correspondences Baptism 26->29 / Crucifixion 30->33), PLUS, for the
  events landing at year 33 alongside the real, densely-curated
  `pw_*`/`rob_*` Passion-Week `order_key` scheme (`0..11_000`,
  `pw_jerusalem_entry`..`pw_mount_of_olives`), an `order_key` placement:
  every Acts-witnessed (`ACT`) event gets `12_000 + chapter*100 + verse`
  (mechanically above `pw_mount_of_olives`'s own `11_000`, and correctly
  chapter:verse ordered within Acts); every Gospel-witnessed mega-span/
  late-ministry freebie still colliding at year 33 gets a hand-placed entry
  in `GOSPEL_ORDER_KEY_OVERRIDES`, keyed to the REAL curated event marking
  its own first contained/nearest pericope, never placed before it. Two
  new fail-loud tests over the real compiled data prove both properties
  hold (`server/atlas-core/src/narrative.rs`):
  `fix_round_1_era_boundary_gate_passion_cluster_sorts_before_every_act_witnessed_event`
  (every `ACT`-witnessed event, EXCLUDING the Passion cluster's own `pw_*`
  ids -- `pw_mount_of_olives` legitimately cites Acts 1:9-12 itself,
  Robertson's own harmonization, `data/curated/acts-sections.toml`'s own
  header -- sorts strictly after `pw_mount_of_olives`) and
  `fix_round_1_within_acts_section_events_follow_acts_chapter_order` (every
  real, curated `acts_section` event stays in Acts's own chapter:verse
  reading order). Provenance is stated POSITIVELY, as a calibration TO the
  AD-33 anchor -- no scale-debate register anywhere (Amendment B,
  inerrancy doctrine). The AMENDMENT D monotonicity audit below remains
  explicitly SAME-BOOK-SCOPED by design (Acts never shares a book with the
  Gospels, so it structurally cannot see a Pentecost-vs-Crucifixion-class
  inversion) -- the two gates above are the deliberate, narrower, cross-book
  check that scope gap needs, not a modification to that audit's own
  same-book methodology.

  CHRONOLOGY ANCHOR TABLE + ERA-WINDOW VALIDATOR (Batch HOTFIX-6, graph-wide
  chronology audit, owner live reports #8/#9, 2026-08-22: Solomon's dream at
  Gibeon rendered PRIOR/FOLLOWING neighbors from the Saul-persecution era --
  "this is a lie... i'm sure these errors are graph-wide"). ROOT CAUSE
  (controller-verified): the `df_*` ("David's Flight from Saul") narrative
  chain, 9 events (`data/curated/events-extra.toml`), was authored ~48
  years LATE relative to this atlas's own declared Ussher/traditional
  scale -- a wrong-anchor calibration error at original authoring time, the
  SAME defect class the NT CALIBRATION note above already fixed for the
  Theographic import, now found in curated OT data instead. `1ki_solomon_
  gibeon` itself was always correctly dated; the global-timeline WIRING
  (GLOBAL TIMELINE note above) was never at fault, only the data it was
  handed.

  THE CANONICAL CHRONOLOGY ANCHOR TABLE (new curated file, `data/curated/
  chronology-anchors.toml` -- that file's own header has the full schema):
  ~21 authoritative dates on this project's own declared scales (Ussher's
  Annals of the World for the OT, this atlas's own AD-33 Passion anchor for
  the NT), each with real source provenance. Each row MAY bind to a real
  compiled `event_id` (`atlas_core::data::ChronologyAnchor`) -- bound only
  where no DISCLOSED scale tension would be misrepresented as a bug. The
  W1/W2-policy-governed early/late-Exodus adjacency is this table's ONE
  true disclosed adjacency: the Ussher-literal `exodus` reference row
  stays honestly UNBOUND, paired with a separate `exodus-departs`
  STRUCTURAL row bound to the real event this atlas's own graph actually
  uses for that transition (search chronology-anchors.toml for
  "STRUCTURAL"). Four further rows -- `jerusalem-falls`, `cyrus-decree`,
  `temple-finished`, `ezra-returns` -- are NOT disclosed adjacencies and
  must never be read as one (controller ruling, fix round on this same
  batch, 2026-08-22): their shipped values (-586/-538/-516/-458) are
  modern-scholarly drift against this atlas's own declared Ussher scale,
  not an equally-valid alternate convention, so this table's `year` on
  each of those four IS canonical -- full stop. Because a planned
  HOTFIX-7 single-feed migration will delete every inline year literal
  and re-date events FROM this table, the corresponding event data is
  deliberately NOT hand-edited yet: each of the four is bound (`event_id`
  set) but flagged a TYPED DEFERRAL in `atlas_core::chronology::
  ANCHOR_DEFERRALS`, a time-bounded exemption kind (carries the anchor
  id, the event id, the shipped value, and a reason) distinct from the
  permanent RECOUNTING/WINDOW_EXEMPTIONS mechanisms below -- these four
  enumerated deferrals exist pending that single-feed migration, and
  resolve automatically once HOTFIX-7 binds their events to this table.
  `jerusalem-falls`/`cyrus-decree` additionally keep their own live
  `exile-begins`/`return-begins` STRUCTURAL companion rows, unchanged,
  honestly describing today's actually-shipped -586/-538;
  `temple-finished`/`ezra-returns` never had one. A subset of rows
  (`era_boundary = true`, always bound) mark the boundaries between the
  brief's own 8 named eras (patriarchal / exodus-wilderness / conquest-
  judges / united-monarchy / divided-kingdoms / exile / return / NT), for
  the era-boundary property test below. FORWARD-COMPATIBLE by design (the
  controller's own 2026-08-22 "single-feed chronology" end-state note, for
  a future HOTFIX-7 that migrates event-date AUTHORING to resolve from this
  table): every row carries a stable, never-renumbered id, so an
  anchor-relative offset table can reference it arithmetically without this
  file's own shape changing.

  ERA-WINDOW VALIDATOR (fail-loud, permanent, `atlas_core::chronology`,
  `atlas_etl::validate::run_chronology_windows`): curated per-book
  NARRATION windows (new curated file, `data/curated/book-narration-
  windows.toml`, ALL 66 canonical books, including the ones with zero
  dated events today -- forward-compatible, same reasoning as the anchor
  table) -- the widest span each book's own narrative NARRATES, derived
  from the anchor table (GEN -4004..-1635, wide and honest; 1SA
  -1171..-1055, tight -- exactly what catches df_ramah's own pre-fix
  -1014). Every dated event's own year must fall inside the window of
  EVERY witness book it claims. RECOUNTING mechanism (the false-positive
  lesson: a plain median-based sweep flagged ~187 events, mostly FALSE
  positives -- genealogy chapters legitimately RECOUNT events thousands of
  years before their own era): `RECOUNTING_CHAPTERS`, a curated (book,
  chapter-range) list checked per-VERSE (so it applies uniformly whether a
  citation arrived via a curated `[[witness]]` row or a bare top-level
  `Event.verses` entry) -- 1 Chronicles 1-9's and Luke 3's/Matthew 1's
  genealogies, Hebrews 11's "by faith" roll call, Acts 7's Stephen speech.
  `WINDOW_EXEMPTIONS` is the EVENT_DISTINCT_PAIRS-style pressure valve for
  a single non-general citation the chapter mechanism doesn't fit (e.g.
  `theo-74`'s own Exodus-12:40/Galatians-3:17 sojourn-count citations),
  each with its own stated reason -- never a silent weakening. Proven BOTH
  directions (`atlas_core::chronology`'s own test module):
  `red_df_ramah_pre_fix_date_fails_the_1sa_window`/`green_df_ramah_post_
  fix_date_passes_the_1sa_window`, and `green_theo7_passes_with_zero_
  exemption_spam` (asserting `theo-7` never appears in `WINDOW_EXEMPTIONS`
  at all -- it passes via the recounting mechanism alone, exactly the
  brief's own named acceptance).

  `THEO_DATE_OVERRIDES` is a DIFFERENT, narrower mechanism from NT
  CALIBRATION's own systematic +3-year shift: a single, isolated,
  genuinely-corrupt raw Theographic import row (`theo-67` "Judgeship of
  Jair," dated -1992 in the source data -- squarely mid-patriarchal-
  genealogy, its own numeric id-neighbors' own era, not Judges) found by
  this batch's own full audit and re-derived from Judges 10:1-3's own
  stated reign lengths, independently corroborated by an untouched
  neighboring entry's own already-correct date. Applied ETL-side, once, on
  the raw pre-`finish()` event set -- the SAME timing/idempotency reasoning
  as `apply_nt_calibration` (see that function's own doc comment).

  AMENDMENT E PROPERTY TESTS (owner directive, 2026-08-22: "we need way
  more than one acceptance test... assert that the application has the
  property of adhering to a canonical table of dates" -- `server/atlas-
  core/src/narrative.rs`'s own test module, loaded via the SAME `load_real_
  compiled_data()` helper the HOTFIX-4 tests above already use): E1
  ANCHOR-EQUALITY -- every bound anchor row's own `year` equals its
  compiled event's own `from_year`, one table-driven test over every row
  -- EXCEPT the 4 `ANCHOR_DEFERRALS` rows, each checked against its own
  recorded `shipped_value` instead and surfaced in a separate,
  always-visible `deferred` list (never silently, never counted as a
  violation); a STALE deferral (the event re-dated without updating the
  deferral entry) fails loud exactly like a real violation. FIX ROUND 2
  (review finding I-2): this is no longer test-time-only -- `atlas_core::
  chronology::anchor_equality_check` is the SAME shared predicate both E1
  above AND `atlas_etl::validate::run_chronology_anchor_equality` (a new
  fail-loud `cargo run -p atlas-etl` build gate) call, so "the table and
  the data agree" is enforced on every build, not only when tests happen
  to run -- two independent LAYERS, one algorithm.
  E2 WINDOW-ADHERENCE -- every compiled dated event obeys its own witness
  books' windows, asserted against the COMPILED JSON on disk (independent
  of the ETL-time validator's own in-process run -- a regression in either
  layer fails loud; both call the SAME `atlas_core::chronology::
  window_violations` predicate deliberately, since a second, independently
  -written implementation would itself be a defect risk -- the real
  independence this buys is LAYER, not a duplicated algorithm). E3
  CANONICAL-ORDER -- bound, NON-deferred anchors, sorted by the TABLE's
  own year, are monotone on the global timeline (deferred anchors are
  excluded from this set: `jerusalem-falls`/`exile-begins` deliberately
  bind the SAME event at two different declared years, which would break
  strict ordering if both were included). E4 ERA-PARTITION -- the OT-wide
  generalization of the NT CALIBRATION note's own two era-boundary gates
  above, to EVERY `era_boundary` anchor: an event whose witness-book
  windows sit entirely at-or-before a boundary's year must sort at-or-
  before it; entirely after, strictly after; a straddling event
  contributes no assertion for that boundary (the same honest carve-out
  the NT gates already established for `pw_mount_of_olives`'s own Acts
  1:9-12 citation). E5 -- the owner's own named Solomon-Gibeon/df_ramah
  case, red-then-green, ON TOP of the properties (not instead of them):
  `1ki_solomon_gibeon`'s own PRIOR/FOLLOWING are verified, against the real
  compiled data, to be `1ki_davids_charge`/`1ki_hiram_temple_prep`
  (Solomon-era); `df_ramah`'s own are `theo-157`/`df_nob` (Saul-
  persecution-era) -- zero `df_*` ids anywhere near the Gibeon dream. The
  ID-equality assertions are the decisive check (table-agnostic by
  construction); FIX ROUND 2 (review finding I-1) additionally derives E5's
  own numeric-window bounds from `chronology_anchors` at test time
  (`solomon-crowned`/`david-hebron`) rather than a hand-typed copy of the
  table's own current years -- the same "zero hardcoded years" rule E1-E4
  already followed, now applied to E5's own secondary bounds too.

  ZERO scale-debate register anywhere in any of the above (inerrancy
  doctrine, unchanged): every disclosed adjacency is stated as "this
  atlas's own already-curated value is X; [Ussher's Annals / a commonly-
  cited alternative] gives Y instead," never adjudicated.

  PROVIDER (no popover surgery -- registered exactly like every other
  section, Explore/PopoverSections.cs). VERSE nodes gain ONE new section,
  appended after catechism: "EVENT" (`VerseEventMembershipSection`,
  conditional -- absent for a verse touching zero titled events), one
  `verse-event-{eventId}` row per event, explorable, opening a fresh
  `EventNode`. EVENT nodes get their own two-to-three sections (the
  witness passage(s) always present; date+places+nav conditional), in
  order:

  date + place(s) + narrative nav (`EventDateAndPlacesSection` -- M-D3/U1,
  see that note below, REBUILDS the narrative-traversal half of this
  section; the date+places half is Batch T2's own UNCHANGED behavior: the
  WHOLE section is conditional, absent when a general-kind passage has
  neither a date nor a place to show AND belongs to no narrative;
  `event-date` present only when this passage HAS a date (i.e.
  `kind == "event"` -- never the server's own internal undated
  placeholder, see the data-model paragraph above); `event-places`, one
  `event-place-{placeId}` row per resolved place, each explorable, opening
  a `PlaceNode` -- "place opens the place node," requirement 4 verbatim;
  the date line carries the event's own curated `ref_note`, when present,
  as a plain hover tooltip -- "ref_note provenance on hover or a quiet
  note"), PARALLEL ACCOUNTS (`EventWitnessesSection` -- one passage-list
  unit per witness, captioned with that witness's own book's DISPLAY name
  -- "Gospel name + passage ref" falls out of the shared component's
  existing Caption + auto-rendered Span, no bespoke rendering needed --
  each clamped to 2 verses via `PassageList`'s own `ClampVerses` (PASSAGE-1);
  the "PARALLEL ACCOUNTS" eyebrow itself is conditional -- present only for
  >=2 witnesses; exactly one witness renders the single passage directly,
  no eyebrow, no "parallel" framing at all, requirement 4 verbatim).

  U1 -- NARRATIVE NAV (M-D3, owner UI spec, progress.md near-verbatim:
  "immediately below focus, left arrow w/ PRIOR event name, right arrow w/
  FOLLOWING (narrative); explorable; hover = normal focus+frontier; those
  foci truncated to ONE VERSE"): RETIRES Batch N/T's own separate "PRIOR
  EVENT"/"FOLLOWING EVENT" sections (`EventPriorSection`/
  `EventFollowingSection`) -- the identical traversal, folded INSIDE
  `EventDateAndPlacesSection` (`event-nav`, at the TOP of that section's
  own body, above the date line) as a compact flanking-arrow row, one per
  qualifying narrative position (unlike Batch N's own retired
  `NarrativeEventNode`, an `EventNode` is never locked to one narrative --
  it has no single "reached through" narrative the way a Verse-originated
  traversal did, so it always surfaces the FULL, unfiltered position list,
  one row per qualifying narrative). LEFT arrow = that narrative's own
  PRIOR event (`event-prior-event-{narrativeId}`, glyph ◂ leading); RIGHT
  = FOLLOWING (`event-following-event-{narrativeId}`, glyph ▸ trailing);
  either side absent (a placeholder span, not a fabricated arrow) when
  that narrative has no adjacent event in that direction -- the narrative's
  own first/last leg. A SINGLE qualifying narrative renders its own row
  with NO name label at all; >1 renders one NAMED row
  (`popover-event-nav-narrative`) per narrative, stacked -- the rare case
  where two qualifying positions would share one narrative id is guarded
  defensively (a numbered `--2`/`--3` testid suffix, same disambiguation
  shape as `catechism-item-{ID}--q2`) though not currently observed in the
  real compiled data. ONE-VERSE FOCI: each side's own adjacent-event
  caption (`event-prior-verse-{narrativeId}`/`event-following-verse-{narrativeId}`)
  is the adjacent event's own FIRST vref only, a plain non-explorable
  quiet line -- NEVER the shared passage-list component's full
  multi-verse-group rendering the retired PRIOR/FOLLOWING sections used
  (that richer, expandable, multi-verse rendering stays exactly as before
  for PARALLEL ACCOUNTS and the GLOBAL TIMELINE pair below, both
  unaffected by U1's own one-verse scoping). "hover = normal
  focus+frontier": no bespoke hover interaction -- `.explorable`, the SAME
  ink-wash ONE-RULE gives every other explorable element in this popover
  platform, is the whole of it.

  TRAVERSAL. Each adjacent event's own arrow is EXPLORABLE (ONE-RULE):
  clicking (or Enter) pushes a fresh `EventNode`, re-anchoring the popover
  onto that event ("its verses become the subject," now richer --
  date/places/witnesses too, not just verses). The traversed node's OWN
  nav resolves by ITS event id (never a re-derived verse), recursing
  exactly as far as the underlying `Narrative.legs` chain goes -- the
  narrative's own first leg has no PRIOR arrow, last has no FOLLOWING,
  both by plain conditional presence (an empty placeholder span, never a
  disabled stub). Each witness's own passage-list entries
  (`event-witness-{SPAN}`) remain independently explorable too -- clicking
  one opens an ordinary `VerseNode`/`PassageNode` for that specific
  verse/span instead of traversing the event as a whole (PASSAGE-1's own
  default click contract, unmodified) -- a second, independent way into
  the same graph, not a competing mechanism. The one-verse nav CAPTION
  itself (`event-prior-verse-*`/`event-following-verse-*`) is NOT
  independently explorable (unlike its retired passage-list-entry
  predecessor) -- the arrow beside it is the row's one explorable
  affordance, by design (a compact nav row, not a second passage list).

  Batch T2: a general-kind EVENT node's own `popover-chip-map` ("Show on
  the map") chip is ABSENT, not merely inert -- there is no date/place to
  bracket a map window with, same "conditional presence extends to
  affordances too" principle CATECH-1 already establishes for a
  CatechismNode's own geography-less chips.

  U6 -- PARALLELS (M-D3, owner verbatim order, progress.md: "Header /
  Verse (focus) / Event / Parallels / Small Catechism / cross references
  LAST"): a VERSE (or PASSAGE, first-verse-anchored -- the SAME convention
  VersePersonsSection below already establishes) node's own quick peek at
  OTHER witnesses of an event it belongs to, `VerseParallelsSection`,
  registered directly after `VerseEventMembershipSection` (so it renders
  immediately below "EVENT" -- see the REGISTRY-1/U6 note above for the
  full VERSE/PASSAGE section order). Reuses `WitnessUnitsResolver`
  (extracted from `EventWitnessesSection`'s own former inline body, no
  behavior change there) fed a FILTERED witness list -- every witness
  EXCEPT the one the current verse itself belongs to (a real per-event
  `EventDetail` fetch, concurrent across every candidate event, since a
  verse's own slim `.Events` membership list carries no sibling-witness
  information). Doubly conditional: absent when the verse cites no titled
  event at all, and absent PER-EVENT when that event has no OTHER witness
  once the current one is excluded (the overwhelming majority of titled
  events, which are single-witness) -- a verse citing >1 QUALIFYING event
  at once (genuinely rare) renders one block per event, each named by that
  event's own label when there is more than one, the identical "single
  entry needs no name" rule U1's own narrative nav already establishes.
  Each block's own passage entries (`verse-parallel{-slug}-{SPAN}`) render
  via the SAME shared passage-list component every other verse list in
  this app uses -- `ClampVerses=2`, never `SpanOnly` (unlike a
  single-witness EVENT node's own PARALLEL ACCOUNTS case, this section
  only ever renders when there genuinely IS an other witness to preview,
  so the compact text is the whole point, never redundant with anything
  already on screen) -- independently explorable, opening a fresh
  `VerseNode`/`PassageNode` for that specific witness verse/span, same
  PASSAGE-1 default click contract every other passage-list entry follows.

  CHRONOLOGICAL-VS-READING-ORDER (requirement 6/7's own worked example, the
  owner's own "the Gospel of John doesn't have everything in order"): the
  Passion Week narrative's own `pw_jerusalem_entry` (witnessed by all four
  Gospels, including John, `JHN.12.12-19`) is chronologically FOLLOWED by
  `pw_temple_cleansing` (Robertson §129 -- Matthew/Mark/Luke ONLY; John
  narrates a DIFFERENT, EARLIER temple cleansing back in John 2 and never
  repeats one during Passion Week). So `pw_jerusalem_entry`'s own FOLLOWING
  EVENT is an event with NO John witness at all -- reading John's own text
  forward from 12:19 never reaches a second temple-cleansing scene (it
  reaches 12:20-36, the Greeks seeking Jesus, a different scene entirely),
  while the graph's own chronological adjacency (via `Narrative.legs`,
  ETL-validated) correctly walks straight to it regardless. This is a real,
  data-grounded case of "the FOLLOWING event is not the next pericope in
  {book}," not merely a hypothetical the acceptance test asserts against.

  CONSISTENCY WITH G1 (requirement 3's own "reuse, don't fork," carried
  forward unchanged from Batch N): `PlaceCard.razor`'s own narrative
  traversal (TRAVERSAL-1) is UNCHANGED by this batch and remains
  client-side, resolved via the SAME event-id-keyed
  `GET /api/narrative/event/{id}` this note's own EVENT popover traversal
  uses (unified onto that one full-chain resolver by Batch N's own
  fix-round-1, see TRAVERSAL-3) -- so a place card's "next event" and a
  popover's "FOLLOWING EVENT" can never disagree about which event, or
  which verses, come next.

  GLOBAL TIMELINE (batch-hotfix4-brief.md requirement 1, owner's own live
  report 2026-08-21: "previous/next event traversal doesn't work. adjacent
  nodes in the dag are dead ends from where we start" -- generalizes the
  ONE resolver from "chronologically adjacent within one narrative's own
  leg chain" to "chronologically adjacent among ALL dated events in the
  whole atlas," per the owner's own recursive-traversal law, "traversed by
  time... arbitrarily far, until the end of the graph"). Every EVENT-kind
  (`kind == "event"`) container gets this; requirement 2's own explicit
  boundary: a GENERAL-kind container never does -- it has no real date to
  traverse by, and fabricating one is forbidden (the SAME "no defensible
  date -> no fabricated line" discipline `EventDateAndPlacesSection`
  already applies to a general-kind passage's own `event-date` row).
  Realized by simple ABSENCE from the server's own timeline index
  (`AtlasData::timeline_order`, built from `kind == "event"` entries only)
  -- never a special-cased branch -- so a general-kind passage's own
  popover carries NO PRIOR IN TIME/FOLLOWING IN TIME section at all
  (conditional presence, as today, requirement 2 verbatim), the identical
  shape its own narrative PRIOR/FOLLOWING absence already has.
  ORDERING: ascending `(when.from_year, order_key)` -- "the T ordering,"
  the SAME tuple `Narrative.legs`' own ETL-validated chronological check
  and `heading_precedence`'s own chronology tier already use, never a
  fresh rule. SAME-DATE RUNS (requirement 1's own explicit case; common --
  order_key defaults to 0 outside deliberately-curated sub-sequencing):
  resolved by a STABLE sort, so ties keep the original compiled-array
  order -- the IDENTICAL "equal on all explicit tiers keeps first-wins"
  precedent `heading_precedence` already establishes and this file already
  documents (DECISIVE-CONTAINER MODEL, above), not a new invented
  tiebreak. `std::cmp::Reverse`, never arithmetic year inversion, anywhere
  a descending comparison is needed elsewhere in this same tuple's own
  family (fix-round-1's own overflow-bug precedent, years run to -4004).
  WIRE: `GET /api/narrative/event/{id}`'s own response is now an OBJECT,
  `{narrative: [...], timeline: {...}}` -- `narrative` is EXACTLY today's
  pre-HOTFIX-4 array, unchanged shape/rows/order (every consumer of the
  OLD bare-array shape migrated to read `.narrative` in the SAME commit:
  `AtlasClient.NarrativeEventPositions`, `EventNode`/`INarrativeAware`,
  `PlaceCard.LoadNarrativePositions` -- TRAVERSAL-1 logic itself
  unchanged, `ExplorerPopover.SyncNarrativeFocusAsync` -- MAP FOCUS SYNC
  logic itself unchanged, and the Playwright helper call sites in
  `world-pin.spec.ts`/`popover-sections.spec.ts`); `timeline` is `{prior,
  following}` (each independently OMITTED, not null, at the atlas's own
  TRUE first/last dated event only -- conditional presence, no stubs
  anywhere else), and the WHOLE `timeline` key is OMITTED (not an
  empty/null object) for a general-kind or unknown event id. PRESENTATION:
  the narrative nav row (M-D3/U1's own `event-nav`, see that note above --
  named "PRIOR EVENT"/"FOLLOWING EVENT" at the time this requirement
  originally shipped, since REBUILT into a headingless flanking-arrow row,
  the traversal semantics themselves untouched by that later rebuild)
  renders first; alongside it (narrative primacy preserved -- BOTH render
  for a narrative member, the timeline pair registered directly after
  `EventDateAndPlacesSection` so the narrative nav always sits above), a
  quiet, clearly distinct pair --
  "PRIOR IN TIME"/"FOLLOWING IN TIME" (`EventTimelinePriorSection`/
  `EventTimelineFollowingSection`, `event-prior-timeline`/
  `event-following-timeline` sections; `event-prior-event-timeline`/
  `event-following-event-timeline` traversal rows; `event-prior-verse-timeline-{SPAN}`/
  `event-following-verse-timeline-{SPAN}` passage-list entries, via the
  SAME shared `PassageList`/`MiniReaderExpand` mechanism every other
  verse list in this popover platform already uses) -- "quiet" via a size
  step on the shared `catechism-section-heading` eyebrow class
  (`.event-timeline-heading`, app.css: NEVER a dimmer color -- the
  existing eyebrow color is already this popover's own established
  7.85:1-on-parchment floor; a size step is this codebase's own
  established de-emphasis technique instead, see `.quiet-label`'s own
  comment), "clearly distinct" via wording ("IN TIME," never "EVENT," so
  the two families are never visually or textually mistakable). For an
  event with NO narrative membership at all, the timeline pair is the
  ONLY traversal shown -- exactly the owner's own report's own case
  (`gen_binding_isaac`, a real W1 container event, previously a dead end).
  MAP COHERENCE (requirement 3, "the same code path, no special case"):
  traversing via a timeline row pushes a fresh `EventNode` exactly like a
  narrative row does (ONE-RULE, unmodified) -- `EventNode` is
  unconditionally `INarrativeAware`, so `SyncNarrativeFocusAsync` (MAP
  FOCUS SYNC, below) runs identically regardless of HOW the popover
  arrived at its current node; a narrative-less destination correctly
  clears arrow focus to baseline via that mechanism's own pre-existing
  empty-list path -- no new map code, no new interop call, exactly the
  same outcome a navigation to any other non-narrative-aware node already
  produces today.

  AMENDMENT D MONOTONICITY AUDIT (batch-hotfix4-brief.md's own coordinator
  amendment; `narrative.rs`'s own `amendment_d_monotonicity_audit_reading_order_vs_global_timeline`
  test): for every pair of dated events sharing a witness BOOK, is GLOBAL
  TIMELINE order consistent with READING order (first-verse-in-book
  ascending)? SCOPE, explicitly documented (HOTFIX-4 fix round 1, review
  finding C-1/Fix 2 -- the original report characterized this scope only
  in code comments, not here): SAME-BOOK ONLY, by design, not merely by
  omission -- full CROSS-book monotonicity is deliberately NOT the bar (OT
  books legitimately interleave, e.g. Kings/Chronicles/prophets narrating
  overlapping reigns; demanding cross-book reading order there would be
  WRONG, not a gap). The direct, structural consequence: Acts (book `ACT`)
  never shares a witness book with the Gospels (`MAT`/`MRK`/`LUK`/`JHN`),
  so this audit cannot see a Pentecost-vs-Crucifixion-CLASS inversion
  between them, however severe -- NOT a defect in the audit (it is doing
  exactly the narrower job it was built for), but a real blind spot this
  file now documents rather than leaving implicit. The NT CALIBRATION note
  above's own two fail-loud tests (era-boundary gate; within-Acts chapter
  order) are the deliberate, narrower, CROSS-book check this exact blind
  spot needs -- a different, additional gate, never a rewrite of this
  audit's own same-book methodology. Re-run post-calibration: inversions
  found dropped from 6,524 to 2,704 (same-book pairs checked: 56,837,
  unchanged -- the SET of pairs an audit this scoped can even see never
  changes; only how many now sort consistently within it does) -- ZERO
  unexplained either time; see batch-hotfix4-report.md's own "Fix round 1"
  section for the full before/after and the CORRECTION block explaining
  why the original run's own count was accurate but incomplete.

  MAP FOCUS SYNC (mechanism UNCHANGED from Batch N -- only which node kind
  triggers it changed: `EventNode` implements `INarrativeAware` where the
  retired `NarrativeEventNode`/`VerseNode` used to; `VerseNode` no longer
  does, since it no longer carries a narrative POSITION of its own, only
  EVENT membership). While the popover's own CURRENT node has >=1 narrative
  position (mid-traversal on an EventNode): every currently-live, non-mini
  map instance (the
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
  proof-verse rows) -- see CATECH-1. batch-t-brief.md RETIRES batch-n-brief.md's
  own `narrative-prior-event-{narrativeId}`/`narrative-following-event-{narrativeId}`/
  `narrative-prior-verse-*`/`narrative-following-verse-*`/`narrative-event-verse-{SPAN}`
  (verse-level traversal is gone) and adds, in their place: `verse-event-{eventId}`
  (a VERSE node's own "EVENT" membership rows), `event-place-{placeId}` (an
  EVENT node's own explorable places), `event-prior-event-{narrativeId}`/
  `event-following-event-{narrativeId}` (the PRIOR/FOLLOWING sections' own
  event-traversal rows, now EVENT-node-only -- M-D3/U1 REBUILDS their own
  surrounding markup into a compact flanking-arrow nav, see EVENT-1's own
  U1 note; these two testids/their explorable click behavior are
  otherwise unchanged) and `event-witness-{SPAN}` (its own passage-list
  entries, PASSAGE-1's existing "every passage-list entry is explorable"
  rule already covers these generically -- M-D3/U1's own one-verse nav
  caption, `event-prior-verse-{narrativeId}`/`event-following-verse-{narrativeId}`,
  is the ONE exception to that PASSAGE-1 rule: a plain non-explorable
  quiet line, not a passage-list entry at all, precisely because it
  REPLACES what used to be one -- see EVENT-1's own U1 note for why) --
  see EVENT-1. batch-hotfix4-brief.md requirement 1
  adds the GLOBAL-timeline counterparts, same rule: `event-prior-event-timeline`/
  `event-following-event-timeline` and `event-prior-verse-timeline-{SPAN}`/
  `event-following-verse-timeline-{SPAN}` -- see the GLOBAL TIMELINE note
  under EVENT-1. `pericope-heading-{eventId}`
  (Reader.razor's own reader-flow heading, batch-t-brief.md requirement 5)
  is ALSO explorable under this same rule -- see EVENT-1; `verse-event-{eventId}`
  and `pericope-heading-{eventId}` carry `.explorable-quiet` INSTEAD of
  `.explorable` for a general-kind container specifically -- see AFFORDANCE-1
  below. batch-m-brief.md
  adds `polity-delta-{id}-{from}-{ringIndex}` (a border ring's own delta
  hit-stroke, ONE-RULE's language adapted for an SVG shape -- see DELTA-1's
  own comment on why plain `.explorable` itself doesn't reach an SVG path)
  and `polity-delta-verse-{SPAN}` (THE SCRIPTURES section's own passage-list
  entries, PASSAGE-1's existing rule covering these generically too) -- see
  DELTA-1. Two kinds of
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
- AFFORDANCE-1 (batch-hotfix4-brief.md requirement 6, owner's own law,
  2026-08-21, near-verbatim: "if something isn't traversable it shouldn't
  look like other things that are actually traversable"). After
  requirement 1's own fix, every DATED event traverses (global timeline,
  narrative, or both); the non-traversable set is GENERAL-kind containers
  (requirement 2's own explicit boundary) and single-direction CHAIN ENDS
  (already honest via ONE-RULE's own existing conditional-presence
  discipline -- one direction present, the other simply absent, no new
  styling needed there). `.explorable-quiet` (app.css) REPLACES
  `.explorable` (never both together) on a general-kind container's own
  identity, everywhere it renders as an explorable target:
  `pericope-heading-{eventId}` (Reader.razor, conditional on
  `heading.Kind == "general"`) and `verse-event-{eventId}` (a VERSE node's
  own EVENT membership row, `VerseEventMembershipSection`, conditional on
  `e.Kind == "general"`) -- ONE rule, both surfaces, no per-surface
  variant. Deliberately a NON-color distinction (unlike `.quiet-label`'s
  own map-furniture precedent) -- both surfaces carry real content a
  reader needs to read at full contrast, so the difference is the near-
  total ABSENCE of `.explorable`'s own darken-on-hover wash
  (`cursor: pointer` alone; whatever base component styling the element
  already independently carries -- e.g. `.popover-event-row-button`'s own
  hover -- is untouched, so the row stays honestly, visibly clickable,
  just never wash-darkening the way a traversable node does). SWEPT, not
  assumed: hover cards and the map itself are VERIFIED unreachable for a
  general-kind container (every `kind == "general"` event has an empty
  `places` list BY CONSTRUCTION -- see the data-model paragraph under
  EVENT-1 -- confirmed against the real compiled data, 0 of 82 general-kind
  events carry any place), so those two surfaces have nothing to sweep;
  the two surfaces named above are the complete set. WIRE: `HeadingOut`
  (`GET /api/chapter/{cref}`'s own per-verse `heading`) and `VerseEventOut`
  (`GET /api/verse/{vref}`'s own `events` array, now its own dedicated
  `VerseEventDto` client-side rather than a reuse of `SceneEvent` -- a
  drive-by fix, found while adding `kind`: `SceneEvent`'s own non-nullable
  `when` would have silently carried the server's internal undated
  sentinel for a general-kind row, dormant only because neither renderer
  ever read it) both gain `kind` (`"event"` | `"general"`) so the client
  never needs a second fetch just to know whether to darken.
- `marker-{placeId}` elements carry the visible place label -- batch-e-brief.md:
  this is the scene's own `display_name` (the period name resolved for the
  scene's current window when the place has curated history and one of its
  name ranges intersects that window, else the place's own DECISIVE fallback
  name -- see ALIAS-1 below), not always the place's plain default name.
  `place-card-title` and the `arrow-tip` text (`{narrative}: {fromName} ->
  {toName}`) use the SAME `display_name`, so a place's name is never shown
  two different ways at once within one scene.
- NAME-1 (batch-e-brief.md): for a time-mode window fully inside one curated
  name range, `marker-{placeId}`'s label and `place-card-title` both equal
  that name; a window crossing the boundary between two curated ranges
  shows whichever one covers the window's own midpoint (or, failing that,
  the later-starting one it still intersects); a window matching no curated
  range falls back to the place's own decisive fallback name (ALIAS-1 below
  -- batch-e3-brief.md AMENDS this rule's prior wording: scripture mode is
  NOT always the plain default name any more -- no curated PERIOD name is
  ever resolved there (there is still no time window to resolve one
  against), but a curated KJV ALIAS, a translation fact rather than a time
  fact, resolves in scripture mode exactly as it does in time mode). A
  place's plain default name is ALWAYS stripped of a trailing ETL
  slug-disambiguation numeral first (batch-e2-brief.md fold-in: "Beersheba 2"
  displays as "Beersheba", never the raw suffixed source name) -- this only
  ever affects the DEFAULT-name fallback; a curated name (already
  hand-written, never suffixed) is untouched. Two places sharing a stripped
  default name may therefore show identical labels at once (their ids stay
  distinct) -- correct cartography, not a collision bug.
- ALIAS-1 (batch-e3-brief.md -- owner bug report 2026-08-20, verbatim: "there
  are two locations, cush and gihon, that are both lit up on genesis 2 even
  though cush isn't mentioned in gen 2:13 why is that happening"; root
  cause: the place WAS right, the label was Theographic's own canonical
  name, never the word the KJV text itself uses there). Server:
  `atlas_core::history::resolve_display_name`/`resolve_display_name_and_canonical`,
  a curated `data/curated/place-names-kjv.toml` (compiled
  `place-names-kjv.json`, `AtlasData::place_name_alias_for`). DECISIVE
  fallback precedence, same "one decisive name per surface" philosophy the
  owner's own passage-container-algebra directive establishes for reader
  headings (progress.md "OWNER DIRECTIVE -- passage container algebra"):
  1. An ACTIVE curated period-history name (NAME-1 above) wins outright when
     one resolves -- it is ALREADY the KJV-accurate name for its own era
     (Luz/Bethel are both real KJV wording, just for different centuries);
     an alias never overrides one that resolved.
  2. Otherwise, a curated KJV alias (a TRANSLATION fact, not a time fact --
     resolves identically in scripture mode, time mode, a reader chapter's
     own place-mention scan, and an EVENT's own `event-places` row,
     regardless of whether a time window even exists) wins.
  3. Otherwise, the place's own plain default name (Theographic canonical),
     stripped of its ETL disambiguation numeral per NAME-1.
  Every KJV-context surface reads this SAME decisive resolution -- no
  parallel/client-side rename map anywhere (owner decree F2 6-ARCH): map
  labels (`marker-{placeId}`, lit AND quiet -- `quiet-marker-{placeId}`),
  `place-card-title`, a place popover's own title (`PlaceNode.Title`, set
  from whichever of the above the caller already resolved), reader place
  mentions (`GET /api/chapter/{cref}`'s own per-verse `places` list --
  `PlaceMentions.cs`'s plain-text substring scan against THIS resolved name
  is what makes an aliased place's mention actually findable in its own
  verse's rendered text at all, e.g. "Ethiopia" in GEN.2.13), and an EVENT
  node's own `event-places`/`event-place-{placeId}` rows (resolved against
  that event's own `when` ONLY when `kind == "event"`, alongside PARALLEL
  ACCOUNTS in the same popover; fix round 1, I-1: a general-kind passage's
  own `when` is the `undated()` sentinel, the WHOLE atlas span -- passing it
  through as a real window would let a period-history name spuriously win
  this tier, the same fabrication guard T2's own wire `when` omission
  already applies extending to this row too -- so a general-kind passage's
  `event-places` resolves with NO window, landing on tier 2 or 3 exactly
  like scripture mode).
  QUIET PROVENANCE (requirement 2's own "canonical name at most once,
  quietly"): the place POPOVER (never the map label, never the hover card)
  gains one non-interactive line, `popover-place-canonical-name` (class
  `popover-meta`, same quiet instrument-face treatment `event-date`'s own
  line already uses), reading "Known in modern atlases as {name}." --
  present ONLY when this place's displayed name differs from its own bare
  canonical name (`PlaceDetail.CanonicalName`, server-decided --
  `Some` only when a KJV ALIAS is the reason the two differ, never for a
  period-history rename, which has nothing to disclose: its own displayed
  name is already the era-accurate KJV wording, not a stand-in for
  something else). Fail-loud ETL validation (`atlas_etl::validate::
  run_place_names_kjv`): every alias id must resolve to a real compiled
  place; no duplicate alias ids; an alias equal to its own place's canonical
  name (after the SAME disambiguation-numeral strip) is rejected as noise;
  every cited verse must parse as a canonical single-verse ref and exist in
  the compiled KJV text.
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
- TRAVERSAL-1 (batch-g1-brief.md requirement 3; adjacency source REPLACED by
  Batch N fix-round-1 -- see TRAVERSAL-3): while pinned, `place-card-narratives`
  shows one row per narrative in which one of this place's own currently-shown
  events is a leg -- a colored swatch (that narrative's own data color) + its
  name, small caps. Adjacency comes from the ONE full-chain narrative resolver
  (`GET /api/narrative/event/{id}`, one call per shown event, `Task.WhenAll`,
  server-side `positions_for_events` over the FULL unwindowed `Narrative.legs`
  chain) -- the exact same endpoint and resolver the EVENT node popover's
  own PRIOR/FOLLOWING sections (EVENT-1, batch-t-brief.md; UNCHANGED
  endpoint/resolver from Batch N, just retargeted onto EventNode) consume,
  so both surfaces answer from
  one computation BY CONSTRUCTION (the previous client-side windowed-arrows
  derivation, and this note's former claim that the two paths "can never
  disagree," were WRONG -- they split on real data under a window ending
  inside a leg-date gap, e.g. Exodus's ex_kadesh -1444 -> ex_moab -1407;
  TRAVERSAL-3 pins the agreement under exactly that window). `card-prev-event-N`/
  `card-next-event-N` present only when the chain has an event in that
  direction (narrative ends: no button -- conditional presence). Clicking pans
  the map to the adjacent place's own marker (no zoom change) and pins ITS
  card -- repeated clicks walk the narrative leg by leg, prev always reversing
  the most recent next back to the previous place. An adjacent place outside
  the current window resolves via the quiet-places fallback (map pans, the
  quiet card renders -- a real navigation); only a place absent from the wire
  entirely no-ops gracefully rather than erroring. A row can therefore exist
  even when the scene draws NO arrow for that narrative (an isolated
  in-window leg whose chain neighbors are both out-of-window) -- the row
  reflects the graph, arrows reflect the window.
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
- DELTA-1 (batch-m-brief.md requirement 4, "the DAG grows a node type" --
  user direction 2026-08-20, verbatim: "...if we're looking at timeranges
  we clearly see the overlapping polities (i.e. each delta throughout the
  timeframe is visible and is explorable in that a delta corresponds to
  some kind of event which we can map to Scripture)"): `GET /api/polities`'s
  own per-era rows now carry `transition`/`fall` (each, when curated, an
  `{event, verses, ref_note}` object -- `atlas_core::data::PolityDelta`,
  curated as a nested `[era.transition]`/`[era.fall]` TOML table under the
  era it belongs to), OMITTED (never null) when a curator honestly found
  nothing to say about that boundary. `transition` describes the change
  FROM the previous era of the SAME polity (or, on a polity's very first
  era, its own rise); `fall` describes a polity's own end, curatable ONLY
  on its chronologically FINAL era (server-validated -- `[era.fall]`
  authored anywhere else fails ETL loudly).

  EXPLORABILITY. Every era boundary whose own year falls INSIDE the
  currently applied window is a real, keyboard-reachable hit target on its
  own ring (`polity-delta-{id}-{from}-{ringIndex}`, see the testid
  inventory above) -- REGARDLESS of whether a `transition`/`fall` block was
  actually curated for it: "an uneventful boundary stays visible but gets
  the minimal popover," not "stays uninteractive." A ring's own START
  boundary (`era.from`) is TRANSITION-explorable whenever it falls
  in-window; its own END boundary (`era.to`) is FALL-explorable whenever it
  falls in-window AND that era is the polity's own chronologically FINAL
  era across its WHOLE curated history (not merely the currently-visible
  window's own narrower subset -- a window showing only one internal era of
  a longer-lived polity must not treat that era's own end as a "fall" when
  a later era genuinely follows it, just outside the window). Both can be
  true for the same ring at once (a narrow window containing an entire
  short-lived era); FALL wins that tie -- the more climactic moment for a
  polity that's ending, a disclosed rule, not an oversight. ONE-RULE's own
  interaction language: hover darkens the ring's own wash+line (~120ms,
  `data-delta-hover`, the SVG-shape reading of ONE-RULE -- see app.css's
  own comment for why plain `.explorable`, background-color-based, has no
  effect on an SVG path), click or Enter (while keyboard-focused) opens the
  ExplorerPopover.

  THE NODE. A NEW `PolityDelta` node kind (`Explore/PolityDeltaNode.cs`),
  built directly from data map.js's own in-memory roster (fetched once,
  the FULL atlas span -- also the morph engine's own roster, requirement
  3a) already resolved -- no second fetch, no server round trip on click.
  `popover-title` is "{polity name}, {fromYear} -> {toYear}" (a CONTRACT
  amendment: a right arrow, not the general Range format's own spaced en
  dash -- a delta describes a directional CHANGE, not a static span; for a
  transition, `fromYear` is the PREVIOUS era's own `to` year when one
  exists, else this era's own `from`; for a fall, `fromYear`/`toYear` are
  this era's own `from`/`to`), ALWAYS present, minimal or full alike.

  SECTIONS (PopoverSectionRegistry, Kind == "PolityDelta" -- no
  `ExplorerPopover.razor` surgery, the SAME seam REGISTRY-1/EVENT-1
  already prove), in order: `popover-section-polity-delta-event` (the
  delta's own curated `event` prose, conditional -- absent for the minimal
  case), `popover-section-polity-delta-scriptures` ("THE SCRIPTURES",
  conditional -- the delta's own curated verses via the SAME shared
  passage-list component (PASSAGE-1) every other verse list in this app
  renders through, truncation-free, entries `polity-delta-verse-{SPAN}`),
  `popover-section-polity-delta-grounding` (the delta's own curated
  `ref_note`, quiet, conditional -- absent for the minimal case). The ONE
  chip `popover-chip-map` ("Show on the map," `ExplorationTarget.NavigateWorld`
  to this delta's own bracketing window -- PANE-ANCHOR-1/NO-NESTED-POPUP
  already make this split-aware for free) is offered UNCONDITIONALLY,
  minimal or full alike -- the window itself is still worth jumping to even
  when the boundary is honestly uneventful.
- MORPH-1 (batch-m-brief.md requirement 3a, "while dragging the time
  slider, borders should MORPH" -- the persisted border-morph-vector design
  idea, user-mandated 2026-08-20): dragging a `TimeSlider` handle fires
  `OnWindowDrag` on EVERY native pointermove (not itself rAF-throttled --
  `BorderLayer.requestMorphFrame`, map.js, is what coalesces however many
  land within one animation frame into a single evaluate+paint), driving a
  transient, purely-visual scrub that never touches the URL or the
  committed `from`/`to` (those stay "the last COMMITTED window" until
  release). Two parallel sub-group pairs hold the border plate: settled
  (`.atlas-wash-settled-group` / `.atlas-border-settled-group`, the static
  layered-era presentation BORDERS-5 etc. already cover) and morph
  (`.atlas-wash-morph-group` / `.atlas-border-morph-group`, this
  requirement's own), both pairs children of the SAME parent groups the
  land-mask clip-path already applies to -- so the morph wash inherits the
  IDENTICAL land/coastline clip the settled wash uses, with no separate
  wiring, and stays clipped for the full duration of a drag, not just at
  rest. `beginMorph` (first drag tick since the last settle) hides the
  settled pair and shows the morph pair (also hiding labels/year-tags for
  the duration -- "no per-frame DOM churn beyond path `d` updates"); every
  path the morph pair creates or reuses across frames carries
  `data-morph-state="morphing"` (settled paths never carry this attribute
  at all). Each evaluated frame calls the SAME `lookup` `setPolities` itself
  calls (border-morph.js), just fed the drag's own transient
  [anchor-year, probe-year] sweep, then `animate`s each affected polity's
  own era sequence to its current ring geometry at the probe year -- "the
  two modes share everything except the final combinator" (requirement 3),
  true by construction. `prefers-reduced-motion: reduce` SNAPS instead of
  interpolating: the probe year is first rounded to whichever bracketing
  era's own midpoint it is numerically nearer (never a value strictly
  between two eras' own knots), fed into the IDENTICAL `animate` --  no
  separate reduced-motion code path, just a different year. On release,
  `settleMorph` tears the morph pair down and restores the settled one
  INSTANTLY, repainted locally from the already-fetched full roster (zero
  network wait) via the SAME `_paintSettled` the network-driven repaint
  that also lands slightly later (the ordinary `from`/`to` URL change) also
  calls -- the two are guaranteed byte-identical, never a visible flash
  between them.
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
  affordance into the SAME split layout -- reader pane left, atlas pane
  right, a thin bronze `split-divider` between them, both fully functional.
  M-D3/B2: the divider is now a real drag-resize handle, not a fixed
  ~55%/45% split -- see DIVIDER-1 below for the full mechanism; ~55%/45%
  (704px reader on a common 1280px-wide viewport) remains only the
  UNDRAGGED default, not an enforced ratio. The atlas pane IS the existing
  World.razor page
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
- DIVIDER-1 (M-D3/B2, owner morning address verbatim: "map toggles halfway
  into view, reader can't -- parity ('not good')"; brief: "the split-view
  drag affordance works from the reader side too"): `Components/
  SplitDivider.razor` -- the `split-divider` line from SPLIT-1 above,
  unchanged in appearance, now wrapped in a wider (13px) invisible
  `split-divider-hit` that owns real drag/keyboard interaction (`role=
  "separator"`, `aria-orientation="vertical"`, `aria-valuenow`/-min/-max in
  pixels). Mouse: pointerdown seeds a drag-start snapshot (`WidthPx` at
  that instant + the pointer's own `ClientX`) and captures the pointer
  (`reader.js`'s `capturePointer`, `setPointerCapture` -- so the gesture
  keeps tracking even once the cursor travels well past the narrow 13px
  strip, unlike TimeSlider.razor's own track-bounded drag, which has no
  equivalently-narrow surface to begin with); every pointermove recomputes
  the reader pane's own width as `dragStartWidth + (currentClientX -
  dragStartClientX)`, clamped, and fires it straight through to
  Reader.razor's own `_splitReaderWidthPx` (no separate "commit at
  release" step -- the resize is live, not a preview). Keyboard: ArrowLeft/
  ArrowRight while the divider is focused nudge by a fixed 24px step,
  identically clamped. Both panes stay independently floored at 320px (MIN)
  -- the reader pane can never be dragged so wide the atlas pane (or vice
  versa) drops below a still-comfortably-usable width; the ceiling is
  computed from a real, live measurement of `.split-view`'s own current
  content-box width (`reader.js`'s `getPaneRect`, already used by
  ExplorerPopover.razor's own pane-anchoring) MINUS both floors AND the
  divider's own 13px hit-area width (a real, live-caught bug in an earlier
  draft shorted the far pane's own floor by exactly that 13px by omitting
  it). `.split-pane-reader`'s own CSS is now `flex: 0 0
  var(--split-reader-width, 55%)` (an exact pixel basis, grow/shrink both
  0 -- Reader.razor sets the custom property on `.split-view` itself, the
  same "custom property set by a context class, read via var() on the
  actual rule" technique `--reader-col-pad-x` already established one
  section up); `.split-pane-atlas` is `flex: 1 1 auto`, filling whatever
  the reader pane's own explicit width doesn't claim, rather than competing
  for its own separate proportional share. `_splitReaderWidthPx` is a
  plain Reader.razor field (survives ordinary chapter-to-chapter
  navigation for free, same reasoning `_splitOpen` itself already has --
  SPLIT-1 above), not `ViewStateService` -- session-local, resets on a
  fresh page load, matching `_splitOpen`'s own persistence tier rather than
  reaching for cross-session persistence nothing else about split-view has
  either. PARITY (the owner's own word): the divider is wired into
  Reader.razor's own markup, the split's one and only host (SPLIT-1 above)
  -- there is exactly one code path, reached identically whichever
  affordance (`split-open-reader` or `split-open-world`) opened it, so
  "works from the reader side too" is true by construction, not a second,
  separately-tested implementation.
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
- NAV-5 (batch-hotfix3-brief.md, user report 2026-08-21, near-verbatim: "the
  next/previous chapter buttons shouldn't be redrawn on every scroll. they
  should be fixed in place like on bible.com"): `reader-prev`/`reader-next`
  hold an EXACT, unchanging screen position across every rendered frame of
  an active scroll, not just at rest -- root-caused live (rAF-timestamped
  `getBoundingClientRect` traces, both a scripted continuous scroll and
  discrete `mouse.wheel` bursts, standalone and split) to a one-frame
  scheduling gap in reader.js's `watchChapterNavCenter`: its compensating
  write (the `--chapter-nav-top` custom property NAV-2 already documents)
  used to be deferred one EXTRA `requestAnimationFrame` tick beyond the
  `scroll` event itself, so for exactly one painted frame per discrete
  scroll input, the browser had already moved the page but the
  compensation hadn't caught up -- reader-next visibly "teleported" by
  exactly that gesture's own scroll delta (measured: a 300px wheel notch
  -> a clean 300px jump) then snapped back the next frame, on 100% of
  sampled gestures, identically in both panes. Two suspects the batch
  brief raised going in were RULED OUT by this same evidence, not left
  unconfirmed: (a) reader.js's `watchScroll` -> `ViewStateService`
  continuous sync triggering a Blazor re-render -- `Reader.razor`'s own
  `OnScroll` has no `StateHasChanged` (unchanged by this fix) and a
  MutationObserver on the whole `<nav aria-label="Chapter navigation">`
  subtree recorded ZERO mutations across every traced scroll; (b) a
  second, split-specific containing-block ancestor beyond `.reader-page`'s
  own documented `contain: layout` (NAV-2) -- none exists
  (`.split-pane-reader` carries no transform/filter/contain of its own),
  and the glitch measured IDENTICALLY in both panes. Fix: `recompute()`
  now runs synchronously as the `scroll`/`resize` listener itself (no rAF
  deferral, no throttle) -- landing the write inside the same task the
  browser already uses to apply the scroll before that frame paints; the
  underlying `--chapter-nav-top` mechanism, its viewport-centering math,
  and the pane-confined `left`/`right` values NAV-2/NAV-3/NAV-4 already
  cover are ALL otherwise unchanged (no CSS touched by this batch).
  `reader.js`'s own `watchChapterNavCenter` comment carries the full trace
  and the accepted trade-off (recompute can now run more than once inside
  a single frame if the browser fires multiple `scroll` events before
  paint -- accepted, since the work is one `getBoundingClientRect` + one
  style write for a single element, not a cost a throttle is needed to
  protect against). Confirmed holding on MAT.26 (this app's own
  heading-dense jank test bed, batch-t-brief.md's `pericope-heading-*`)
  under the same scripted scroll, with headings still rendering and still
  explorable -- this fix touches only `watchChapterNavCenter`'s own
  scheduling, nothing render-path-adjacent. NAV-6 is the same assertion,
  split view. NAV-7 is a permanent render-churn guard (kept even though
  suspect (a) above proved false) -- reader-prev/reader-next's own DOM
  node identity, stamped before a multi-tick scroll, survives it unchanged
  (never recreated), insurance against a future regression reintroducing a
  Blazor re-render on scroll.
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
- PERSONS-1 (batch-p-brief.md, the extensibility proof: "a new node kind is
  a new fact file plus a compiler rule -- the UI machinery picks it up
  through the same generic contract"). Theographic PERSONS enter as a new
  `Person` node kind + `mentions`/`mentioned-in` rows, served through the
  EXISTING two generic endpoints (`GET /api/node/{id}`, `GET
  /api/node/{id}/edges`) with zero new bespoke endpoints, and consumed
  client-side through `IExplorableClient` -- the FIRST live UI call site for
  that interface (M-D2 shipped it DI-wired with none; CLIENT-ACCESS-1's own
  "zero live in-app UI call sites" concern is what this batch closes).

  VERSE/PASSAGE POPOVER, PERSONS SECTION: `persons-section-heading` (text
  "PERSONS"; present iff the locus's own generic `mentions` edge page
  carries >=1 Person-kind entry -- conditional presence, the same "a
  section renders iff count > 0" law every other section already follows).
  One `verse-person-{slug}` button per mentioned person (slug = the
  person's own display name, lowercased, non-alphanumeric runs collapsed to
  a single `-`, leading/trailing `-` trimmed -- `PopoverSectionProviders.cs`'s
  own `VersePersonsSection.Slug`); text is the person's own name; click
  pushes a `PersonNode`. A verse's own `mentions` frontier carries BOTH
  Place and Person entities under ONE edge kind (`graph_types::edge::
  Mentions`, `PlaceOrPerson`-typed rows) -- this section filters to
  `Kind == "Person"` client-side (design doc §7's own CHAIN HOMOGENEITY
  law: one section renders entries of ONE kind-shape only; a mixed list
  would violate it). Fetched at `EdgeSectionRegistry.Mentions.InitialClamp`
  (50) in one page -- a real verse's own total mentions (places+persons
  combined) never comes remotely close to that in the real compiled data;
  if the true total somehow exceeds it (`page.next` non-null), a quiet,
  non-interactive `persons-section-more` line ("+ more mentions in this
  verse") discloses it honestly rather than silently under-reporting.

  PERSON POPOVER: title is the person's own name (`PersonNode.Title`, the
  label carried on the SAME `EdgeEntryDto` the triggering verse's own
  PERSONS row already had in hand -- no extra fetch to learn it).
  `popover-person-provenance` (present iff `Card.Provenance` is non-empty;
  text "Source: {provenance}") is this batch's own genuinely new use of the
  generic Card wire's `Provenance` field -- no prior consumer in this app
  rendered it anywhere. `person-mentions-heading` -- text
  "MENTIONED IN SCRIPTURE ({N})", N = `Card.EdgeSummary`'s own true
  `mentioned-in` count, shown up front regardless of how many rows are
  actually paged in (design doc §7's own "honest count up front" law). One
  `person-mention-{VREF}` button per mentioned verse, VREF = its own
  canonical dot-ref (e.g. `GEN.14.18`) -- click pushes a fresh `VerseNode`
  for that exact ref, closing the loop: VERSE -> PERSON -> a mentioned
  VERSE. CANON ORDER (the brief's own explicit "mentioned-in: every mention
  in Scripture, canon order"): guaranteed by construction, not by any
  client-side sort -- `atlas_etl::people::parse_people` explicitly
  canon-sorts (book index, then chapter, then verse) each person's own
  resolved verse links BEFORE the graph adapter ever sees them (spot-checked
  against real data: Theographic's own upstream list order is NOT reliably
  canon-order already), and the generic port's own `BiIndex` preserves row-
  insertion order end to end with no re-sort anywhere on the path (server or
  client) -- proven at the wire by `tests/ux/reader-persons.spec.ts`'s own
  PERSONS-3.

  HONEST CLAMPS: initial page = `EdgeSectionRegistry.MentionedIn.InitialClamp`
  (12) mentioned-verse rows. A person whose true total exceeds that shows
  `person-mentions-more` (a down-arrow reveal button, the SAME
  `.popover-passage-reveal` affordance/glyph every other capped list in this
  app already uses) -- clicking it is a GENUINE second network fetch
  (`IExplorableClient.Edges`, following the first page's own cursor),
  appended to the shown list, REPEATABLE (each further click walks one more
  page) until the port itself reports no further page (`next` null) -- never
  an artificial wall this component invents; a real busy person (e.g. real
  committed data's own "David" at 896 mentions) needs genuine server-side
  pagination, unlike every OTHER capped list in this app (which caps an
  ALREADY-fully-fetched array client-side only) -- disclosed as the one
  section in this app whose own reveal step is a real fetch, not a client-
  side unhide.

  PERSON PAYLOAD, kept vs. dropped (requirement 1's own "disclose what you
  kept/dropped"): `NodePayload::Person` carries label/gender/birth_year/
  death_year/also_called (real fields, not a stub -- the SAME "widen the
  payload" precedent M-C's own Place/Polity widening established) -- NONE
  of gender/birth_year/death_year/also_called are projected onto the
  generic Card WIRE this batch (`NodeCardOut.label` stays one plain string,
  matching every OTHER kind's own card-label precedent exactly: Place's
  canonical name carries no lat/lon, Event's label carries no date); a
  future batch's own decision to widen the generic card wire for richer
  rendering is real, disclosed follow-up, not attempted here. Dropped
  entirely: Easton's Bible Dictionary prose (19th-century external
  commentary, out of scope the same way the place adapter never carried
  Theographic's own richer place fields either) and Theographic's own
  `status` field (verified NOT a notability signal -- it is Theographic's
  own "has a prose bio been finished" authoring-workflow flag; filtering to
  its `"publish"` value would have silently dropped Saul, Elijah, Jeremiah,
  Daniel, Job, and dozens of other unmistakably major figures, so every
  person record ships regardless of it). ALSO DROPPED, CORRECTED fix round
  1 (R-P2 -- a real gap the first draft of this note left silent): the
  source also carries `father`/`mother`/`children`/`siblings`/`partners`
  (foreign-key ids resolving to OTHER Person records in the SAME file --
  the identical shape `verses` uses for mentions) and
  `birthPlace`/`deathPlace`/`memberOf`/`timeline`. None of it is read this
  batch -- turning it into edges (genealogy/family; a Person-to-Place
  `birthplace`/`deathplace` edge mirroring `located-at`) is a NEW RELATION
  KIND, a types-first + owner-approved decision (design types doc §3's own
  relation-manifest law), correctly outside this batch's own card+mentions
  scope -- ledgered for the owner, not silently built. DISCLOSED
  DATA-SHAPE NOTE: Theographic's own People table is not exclusively
  individuated humans -- it also carries "God", "Holy Spirit", and eponyms
  doubling as tribe/nation names ("Judah": four separate, genuinely bare
  records, `name == displayTitle == "Judah"`, no disambiguation --
  CORRECTED fix round 1, R-P4: an earlier draft named "Israel" here too,
  but the only Israel-named record, `israel_682`, has `displayTitle:
  "Jacob (Israel)"`, already individuated in the exact label that ships to
  the client, so it was an imprecise example, not a real gap) among the
  3,067 real records; this batch imports the source as-is (doctrine:
  "imported confidence class"), with no editorial re-classification
  attempted or needed for anything this batch actually renders
  (gender/years never reach the UI, per the paragraph above).

  WIRE GRAMMAR (M-B/M-C's own established one-line-per-kind pattern,
  continued unchanged): `graph_wire::decode_node_id` gains exactly one new
  match arm, `"Person" => ...` -- `encode_node_id`'s own pre-existing
  generic fallback already produced `"Person:{raw}"` with zero changes.
  Nothing else in `graph_handlers.rs`/`store.rs`/`explore.rs` needed a
  change for the two generic endpoints to serve Person nodes -- the
  thesis's own most direct evidence.
