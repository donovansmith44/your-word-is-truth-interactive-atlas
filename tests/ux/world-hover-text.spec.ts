import { test, expect, type Page, type Locator } from '@playwright/test';
import { api } from './lib/api';
import {
  mergedVerses, groups, isPassage, initialShownCount, visibleGroups,
  revealSequence, spanRef, verseText, type Group,
} from './lib/hovercard';
import { independentlyHoverableIds } from './lib/hoverSafety';

// A card anchored above a marker near the TOP of the viewport (this batch's
// own real, live example: "Philippi" in the apostolic window) can need a
// LARGE on-screen jump from the marker (where hover({force:true}) leaves
// the pointer) to place-card-more/collapse at the card's far side.
// Locator.click()'s default single-jump pointer move -- confirmed live,
// isolated from every other variable (identical geometry, only the move's
// step count changed) -- doesn't reliably deliver the pointerenter that
// PlaceCard's own OnPointerEnter needs to cancel World.razor's marker-
// leave-triggered ScheduleCardClose, so the card can close mid-sequence
// regardless of any pause inserted afterward; a real user's mouse, which
// never teleports, always arrives as the many-small-steps case and never
// hits this. Used for every place-card-more/collapse click below so this
// suite's own synthetic input is at least as realistic as a real pointer,
// not a source of flakiness the app itself doesn't have.
async function moveAndClick(page: Page, locator: Locator): Promise<void> {
  const box = await locator.boundingBox();
  if (!box) {
    throw new Error('moveAndClick: locator has no bounding box (not visible?)');
  }

  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 10 });
  await page.mouse.down();
  await page.mouse.up();
}

// Batch D (batch-d-brief.md): the hover place card was rebuilt around
// direct user feedback -- no more bookkeeping (the old per-(book,chapter)
// verse-group-{BOOK}-{chapter} index and its single-shot place-card-expand
// are GONE), passage-aware verse rendering instead (CONTRACT's 4/2-initial,
// 5/2-step, snap-back rule). Every property below selects ONLY via
// CONTRACT testids and is built to fail against the OLD card: `hover-
// passage-{SPAN}`, `place-card-more` and `place-card-collapse` never
// existed on it at all (it had `place-card-expand` instead, asserted
// absent below), and the old card's single-click "reveal everything, no
// way back" behavior cannot satisfy a repeated-chunk or a round-trip
// property.

// Deliberately NOT the full 4004 BC - AD 100 span: confirmed live (while
// investigating this suite) that its 160+ places pack tightly enough around
// hubs like Jerusalem for a forced hover to land on a geographically
// adjacent marker instead of the intended one -- the exact risk app.css's
// own .atlas-marker comment documents ("Playwright's force:true skips ITS
// OWN obstruction checks, but the resulting CDP mouse event still hits
// whatever the browser's real hit-testing says is topmost at that exact
// pixel"). These three windows are the ones the rest of this suite already
// relies on for hover precision (world-map.spec.ts's WORLD-2 picks the
// exodus window for the very same reason).
const WINDOWS = [
  { from: -1446, to: -1406 }, // exodus: rich scene (world-map.spec.ts's own WORLD-2 comment)
  { from: -2100, to: -2085 },
  { from: 46, to: 48 },
];

// Batch C2 (requirement 0b/0c): the tests below that pick "whichever place
// has the most merged verses" (or similar) originally searched pure scene
// DATA across every WINDOWS entry, with no notion of whether the winner's
// marker is actually independently hoverable on screen -- which is exactly
// how one of them found "Philippi" (apostolic window): 32 merged verses,
// the clear data-only winner, but its marker renders within single-digit
// px of "Neapolis" (its own port city, 13.92km away in reality) at that
// scene's natural zoom, and lib/hoverSafety.ts's own header comment has the
// confirmed root cause (Leaflet's default per-marker z-index-by-screen-Y
// stacking, not DOM order) for why a forced hover there can land on the
// WRONG marker. This helper is the fix every affected search below now
// shares: it still walks WINDOWS in order, but -- since "is this marker
// safe" can only be answered by actually rendering a window, unlike a pure
// data search -- it navigates to each candidate window in turn and scores
// `filter`-eligible places ONLY among that window's own real, currently
// independently-hoverable markers, returning the best (by `score`) match in
// the FIRST window that has any. A window with no safe eligible place is
// skipped, not failed -- the caller's own remaining WINDOWS entries are the
// fallback, same as every one of these searches already had.
async function bestHoverablePlace(
  page: Page,
  filter: (verses: string[]) => boolean,
  score: (verses: string[]) => number,
): Promise<{ w: typeof WINDOWS[number]; place: any; verses: string[] } | undefined> {
  for (const w of WINDOWS) {
    const scene = await api.sceneTime(w.from, w.to);
    const eligible = scene.places.filter((p: any) => filter(mergedVerses(p)));
    if (eligible.length === 0) {
      continue;
    }

    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const safeIds = await independentlyHoverableIds(page, eligible.map((p: any) => p.id));

    let best: { place: any; verses: string[] } | undefined;
    for (const p of eligible) {
      if (!safeIds.has(p.id)) {
        continue;
      }
      const verses = mergedVerses(p);
      if (!best || score(verses) > score(best.verses)) {
        best = { place: p, verses };
      }
    }
    if (best) {
      return { w, place: best.place, verses: best.verses };
    }
  }
  return undefined;
}

test('hover place card: a passage-first place renders one capped hover-passage block, verbatim text, no bookkeeping rows', async ({ page }) => {
  // Search every candidate window for whichever INDEPENDENTLY HOVERABLE
  // place has the LONGEST consecutive-run first group, so the "up to 4"
  // cap is proven against a real run that's actually longer than 4 -- not
  // a coincidental 2-4 verse run where "capped at 4" and "just happens to
  // be everything" look the same on screen.
  //
  // FIX ROUND 1 CORRECTION: this test used to run its own inline search
  // (pure scene DATA, no hover-safety check) -- exactly the older,
  // pre-bestHoverablePlace pattern this file's own header comment already
  // documents as fragile ("Philippi... its marker renders within
  // single-digit px of Neapolis"). It never actually tripped before this
  // fix round widened the bare-/world default window (World.razor's own
  // DefaultTo, 29->33): the wider default shifts this page's OWN initial
  // marker population/settling timing before SyncFromQuery lands on
  // whatever window a test explicitly requested, enough to flip Ai/Bochim
  // (both real, ~9km apart, this window's own data unchanged) from
  // independently-hoverable to marker-adjacent on this suite's fixed
  // viewport. Upgraded to the same safety check the rest of this file
  // already uses for this exact failure class, rather than reverting the
  // default-window fix or chasing render-timing instead.
  const best = await bestHoverablePlace(
    page,
    verses => isPassage(groups(verses)[0]),
    verses => groups(verses)[0].length,
  );
  if (!best) {
    test.skip(true, 'no independently-hoverable place with a consecutive-run first group found in any candidate window');
    return;
  }
  const { place, verses } = best;
  const first = groups(verses)[0];
  const expectedShown = Math.min(4, first.length);
  const expectedSpan = spanRef(verses, { start: 0, length: expectedShown });

  // bestHoverablePlace already navigated to (and rendered) `w` while
  // measuring hover safety -- re-navigating here would just re-fetch the
  // same page for nothing.
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  // One passage block spanning exactly the capped range -- the old card
  // never grouped verses at all, so it can never produce this testid.
  await expect(card.getByTestId(`hover-passage-${expectedSpan}`)).toBeVisible();
  await expect(card.locator('[data-testid^="hover-passage-"]')).toHaveCount(1);

  // Exactly `expectedShown` verse rows total -- proves the cap held even
  // though this place's real first run is longer (never a wall).
  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(expectedShown);

  // Every shown verse's own text is the verbatim KJV text from the chapter
  // endpoint -- containing the FULL untrimmed string rules out any
  // "first N chars + an ellipsis" truncation.
  for (const vref of verses.slice(0, expectedShown)) {
    const text = await verseText(vref);
    await expect(card.getByTestId(`hover-verse-${vref}`)).toContainText(text);
  }

  // The killed bookkeeping never reappears.
  await expect(card.locator('[data-testid^="verse-group-"]')).toHaveCount(0);
  await expect(card.getByTestId('place-card-expand')).toHaveCount(0);

  // Nothing is expanded yet -- collapse only appears once it would do
  // something.
  await expect(card.getByTestId('place-card-collapse')).toHaveCount(0);
});

test('hover place card: a lone-first place shows exactly its first two verses individually, never wrapped as a passage', async ({ page }) => {
  let match: { w: typeof WINDOWS[number]; place: any; verses: string[] } | undefined;
  for (const w of WINDOWS) {
    const scene = await api.sceneTime(w.from, w.to);
    for (const place of scene.places) {
      const verses = mergedVerses(place);
      if (verses.length < 3) continue; // also want place-card-more provably present below
      if (!isPassage(groups(verses)[0])) {
        match = { w, place, verses };
        break;
      }
    }
    if (match) break;
  }
  if (!match) {
    test.skip(true, 'no place with a lone-verse first group and >2 total merged verses found');
    return;
  }
  const { w, place, verses } = match;

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(2);
  await expect(card.locator('[data-testid^="hover-passage-"]')).toHaveCount(0);
  for (const vref of verses.slice(0, 2)) {
    const text = await verseText(vref);
    await expect(card.getByTestId(`hover-verse-${vref}`)).toContainText(text);
  }

  await expect(card.locator('[data-testid^="verse-group-"]')).toHaveCount(0);
  await expect(card.getByTestId('place-card-expand')).toHaveCount(0);
  await expect(card.getByTestId('place-card-collapse')).toHaveCount(0);
  await expect(card.getByTestId('place-card-more')).toBeVisible();
});

test('hover place card: place-card-more repeatedly reveals the next chunk (+5 inside a passage, +2 for lone verses) until the full list is shown, then disappears', async ({ page }) => {
  // Whichever INDEPENDENTLY HOVERABLE place has the most merged verses, in
  // the first candidate window with any (bestHoverablePlace's own header
  // comment) -- to exercise several clicks (and, on real curated data, both
  // step sizes) rather than just one.
  const best = await bestHoverablePlace(
    page,
    verses => verses.length > initialShownCount(verses),
    verses => verses.length,
  );
  if (!best) {
    test.skip(true, 'no independently-hoverable place with more than its own initial verse count found in any candidate window');
    return;
  }
  const { place, verses } = best;
  const seq = revealSequence(verses);
  expect(seq.length).toBeGreaterThanOrEqual(1);
  expect(seq[seq.length - 1]).toBe(verses.length);

  // bestHoverablePlace already navigated to (and rendered) `w` while
  // measuring hover safety -- re-navigating here would just re-fetch the
  // same page for nothing.
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(initialShownCount(verses));

  const moreBtn = card.getByTestId('place-card-more');
  for (const expectedShown of seq) {
    await expect(moreBtn).toBeVisible();
    await moveAndClick(page, moreBtn);
    // A descendant click must not close the card (same pointer-stays-inside
    // contract the old expand button had).
    await expect(card).toBeVisible();
    await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(expectedShown);
  }

  // Exhausted: the arrow the old card never had is now gone, same as the
  // old one's own expand button was once IT had revealed everything -- but
  // reached via `seq.length` repeated +5/+2 chunks, not one single click.
  await expect(moreBtn).toHaveCount(0);

  // The very last verse revealed (the one most likely to be wrong if
  // stepping silently mis-stepped or reordered anything) is real, verbatim
  // KJV text.
  const lastVref = verses[verses.length - 1];
  const lastText = await verseText(lastVref);
  await expect(card.getByTestId(`hover-verse-${lastVref}`)).toContainText(lastText);

  await expect(card.getByTestId('place-card-collapse')).toBeVisible();
});

test('hover place card: place-card-collapse snaps back to the exact initial DOM state after expanding', async ({ page }) => {
  // First independently-hoverable eligible place (bestHoverablePlace's own
  // header comment) -- a constant `score` keeps this a "first match" search,
  // same intent the old plain scene.places.find() had.
  const found = await bestHoverablePlace(
    page,
    verses => verses.length > initialShownCount(verses),
    () => 0,
  );
  if (!found) {
    test.skip(true, 'no independently-hoverable place with more than its own initial verse count found in any candidate window');
    return;
  }
  const { place } = found;

  // bestHoverablePlace already navigated to (and rendered) the winning
  // window while measuring hover safety -- re-navigating here would just
  // re-fetch the same page for nothing.
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  const domState = () => card.locator('[data-testid^="hover-verse-"], [data-testid^="hover-passage-"]')
    .evaluateAll(els => els.map(el => el.getAttribute('data-testid')));

  const initialState = await domState();
  await expect(card.getByTestId('place-card-collapse')).toHaveCount(0);
  const moreBtn = card.getByTestId('place-card-more');
  await expect(moreBtn).toBeVisible();

  // Expand as far as two clicks allow (one if the place's whole list
  // exhausts in a single chunk) -- either way proves the round trip from a
  // genuinely-expanded state back to the untouched initial one.
  await moveAndClick(page, moreBtn);
  if (await moreBtn.count() > 0) {
    await moveAndClick(page, moreBtn);
  }
  const expandedState = await domState();
  expect(expandedState.length).toBeGreaterThan(initialState.length);
  await expect(card.getByTestId('place-card-collapse')).toBeVisible();

  await moveAndClick(page, card.getByTestId('place-card-collapse'));
  await expect(card).toBeVisible(); // a descendant click must not close the card

  const collapsedState = await domState();
  expect(collapsedState).toEqual(initialState);
  await expect(card.getByTestId('place-card-collapse')).toHaveCount(0);
  await expect(card.getByTestId('place-card-more')).toBeVisible();
});

test('hover place card: a place with two or fewer merged verses shows every verse and no expand/collapse controls', async ({ page }) => {
  let match: { w: typeof WINDOWS[number]; place: any } | undefined;
  for (const w of WINDOWS) {
    const scene = await api.sceneTime(w.from, w.to);
    const place = scene.places.find((p: any) => {
      const n = mergedVerses(p).length;
      return n >= 1 && n <= 2;
    });
    if (place) {
      match = { w, place };
      break;
    }
  }
  if (!match) {
    test.skip(true, 'no place with 1-2 merged verses found in any candidate window');
    return;
  }
  const { w, place } = match;

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  const verses = mergedVerses(place);
  const vis = visibleGroups(groups(verses), verses.length); // everything is shown

  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(verses.length);
  for (const g of vis) {
    if (isPassage(g)) {
      await expect(card.getByTestId(`hover-passage-${spanRef(verses, g)}`)).toBeVisible();
    }
  }

  await expect(card.getByTestId('place-card-more')).toHaveCount(0);
  await expect(card.getByTestId('place-card-collapse')).toHaveCount(0);
  await expect(card.locator('[data-testid^="verse-group-"]')).toHaveCount(0);
  await expect(card.getByTestId('place-card-expand')).toHaveCount(0);
});

// Requirement 0c (user 2026-08-19: "hover interactions on both the map and
// the Bible are extremely fickle. FIX IT."; CONTRACT-bound acceptance
// test): the exact realistic journey the brief names -- hover a marker,
// travel across the gap into the card, click place-card-more, then leave
// both entirely and confirm the card actually closes. moveAndClick's own
// many-small-steps pointer move (this file's header comment) is what makes
// this "realistic" rather than a synthetic teleport: a real mouse crossing
// the marker/card gap always arrives the same way, and the source-level
// fix this batch ships (World.razor's `_pointerOverCard`, tracked across
// PlaceCard's own OnPointerEnter/OnPointerLeave) is exactly what keeps the
// card open through that crossing without the old 150ms grace window's
// leave-then-enter race closing it out from under the pointer -- see
// World.razor's own comment on `_pointerOverCard` for the full race.
test('hover robustness: hover -> cross the gap -> click place-card-more succeeds, and the card closes within ~1s once the pointer leaves both', async ({ page }) => {
  // Whichever INDEPENDENTLY HOVERABLE place has the most merged verses, in
  // the first candidate window with any -- see bestHoverablePlace's own
  // header comment for why this can't be a pure data search (a real,
  // confirmed example, "Philippi" in the apostolic window, is the DATA-only
  // winner here but not independently hoverable at all).
  const best = await bestHoverablePlace(
    page,
    verses => verses.length > initialShownCount(verses),
    verses => verses.length,
  );
  if (!best) {
    test.skip(true, 'no independently-hoverable place with more than its own initial verse count found in any candidate window');
    return;
  }
  const { place, verses } = best;
  const initialShown = initialShownCount(verses);

  // bestHoverablePlace already navigated to (and rendered) the winning
  // window while measuring hover safety -- re-navigating here would just
  // re-fetch the same page for nothing.

  // Phase 1: hover the marker.
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(initialShown);

  // Phase 2: travel across the gap (moveAndClick's own multi-step
  // page.mouse.move, steps: 10 -- see this file's header comment) and
  // click place-card-more. Success here -- more verses revealed, card
  // still open -- is only possible if the pointer's journey off the
  // marker and onto the card never triggered a premature close.
  const moreBtn = card.getByTestId('place-card-more');
  await expect(moreBtn).toBeVisible();
  await moveAndClick(page, moreBtn);
  await expect(card).toBeVisible();
  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(revealSequence(verses)[0]);

  // Phase 3: the pointer leaves both the marker and the card entirely --
  // CONTRACT's own "~1s" bar. World.razor's grace is 350ms (up from
  // 150ms, requirement 0c); 1200ms gives real margin above that for the
  // close to actually land without asserting an exact millisecond.
  await page.mouse.move(0, 0, { steps: 10 });
  await expect(card).toBeHidden({ timeout: 1200 });
});

// Batch G1 requirement 1b ("the map's verses are nodes too" -- user
// direction: "this is all one big dag that we can explore from anywhere"):
// hover-verse-{VREF}/hover-passage-{SPAN} content in the (UNPINNED) hover
// card is explorable under the same one rule verse-line/chapter-head use in
// the reader -- click opens the ExplorerPopover on that verse's own node,
// and (per the brief's own acceptance test) its xrefs chip works. The card
// stays alive through this whole gesture (hover-persistence keeps it open
// while the pointer travels onto it) until the popover promotion itself
// supersedes it, same "date-explore" pattern Batch E's dates already use.
test('hover place card: clicking a verse in an UNPINNED card opens its VerseNode popover; the xrefs chip works', async ({ page }) => {
  const best = await bestHoverablePlace(page, verses => verses.length > 0, verses => verses.length);
  if (!best) {
    test.skip(true, 'no independently-hoverable place with any merged verse found in any candidate window');
    return;
  }
  const { place, verses } = best;
  const vref = verses[0];

  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  const verseEl = card.getByTestId(`hover-verse-${vref}`);
  await moveAndClick(page, verseEl);

  // The popover promotion supersedes the (unpinned) hover card, same as
  // every other card-explore affordance (Batch E's dates, WORLD-8's title).
  await expect(card).toHaveCount(0);
  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(vref);
  const text = await verseText(vref);
  await expect(popover).toContainText(text);

  // Batch R requirement 3(b): no popover-chip-xrefs toggle anymore --
  // cross-references render INLINE, immediately (conditional presence: a
  // verse with zero recorded cross-refs shows no xrefs section at all,
  // never a "No cross-references recorded." placeholder). Batch F2
  // requirement 6 (XREF-1): the initial render is CAPPED (3 when xrefs is
  // the only context section, 2 when THE SMALL CATECHISM is also present)
  // -- the full count is only reachable after xrefs-more.
  const detail = await api.verse(vref);
  // Batch T: the real cap rule (ExplorerPopover.razor's own
  // OtherContextSectionCount) is generic -- "any OTHER resolved context
  // section, not just catechism" -- so it now ALSO counts the new
  // `event-membership` section (VerseEventMembershipSection) a verse
  // touching a titled event gets. A catechism-only check here would go
  // stale again the next time a new context-section provider ships;
  // mirror the product's own generalized rule instead (every
  // popover-section-* except verse-text/xrefs themselves) so this stays
  // correct without a matching test edit every time. Safe to read
  // bare (no retry) here: `.toContainText(text)` above already proved
  // _sections has been populated (verse-text is itself one of those
  // section-registry providers, resolved in the SAME Task.WhenAll batch
  // as catechism/event-membership -- see ExplorerPopover.razor's own
  // LoadCurrent comment on the single-intermediate-render quirk).
  const otherContextSectionCount = await page.getByTestId(/^popover-section-/).evaluateAll(
    els => els.filter(el => {
      const id = el.getAttribute('data-testid');
      return id !== 'popover-section-verse-text' && id !== 'popover-section-xrefs';
    }).length
  );
  const cap = otherContextSectionCount > 0 ? 2 : 3;
  const expectedInitial = Math.min(detail.cross_refs.length, cap);
  await expect(page.getByTestId(/^xref-item-/)).toHaveCount(expectedInitial);
  if (detail.cross_refs.length > 0) {
    await expect(page.getByTestId(`xref-item-${detail.cross_refs[0].target}`)).toBeVisible();
  } else {
    await expect(page.getByTestId('popover-section-xrefs')).toHaveCount(0);
  }
});

// Same acceptance test, the PASSAGE side: clicking the passage row itself
// (its own ref label, outside any nested verse span) opens the whole
// PassageNode instead of any one verse. Uses bestHoverablePlace (not a plain
// scan) for the same reason every OTHER click/hover-precision test in this
// file does: an unsafe pick can land on a place from the exodus window's own
// documented dense cluster (WORLD-8's own comment names "Ai" specifically) --
// confirmed live while writing this test, a plain scan's first match landed
// on exactly that place and a forced hover resolved to a NEIGHBORING marker
// instead, so the expected hover-passage-{SPAN} testid never appeared and
// the test hung on boundingBox() until timeout.
test('hover place card: clicking a passage block (outside a specific verse) opens its PassageNode popover', async ({ page }) => {
  const best = await bestHoverablePlace(
    page,
    verses => verses.length > 0 && isPassage(groups(verses)[0]),
    verses => groups(verses)[0].length,
  );
  if (!best) {
    test.skip(true, 'no independently-hoverable place with a consecutive-run first group found in any candidate window');
    return;
  }
  const { place, verses } = best;
  const first = groups(verses)[0];
  const span = spanRef(verses, { start: 0, length: Math.min(4, first.length) });

  // bestHoverablePlace already navigated to (and rendered) the winning
  // window while measuring hover safety -- re-navigating here would just
  // re-fetch the same page for nothing.
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  const passageEl = card.getByTestId(`hover-passage-${span}`);
  const refEl = passageEl.locator('.place-card-passage-ref');
  await moveAndClick(page, refEl);

  await expect(card).toHaveCount(0);
  await expect(page.getByTestId('popover-title')).toHaveText(span);
});

// HOTFIX batch (batch-hotfix-brief.md requirement 1, CARD-FLIP-1, user report
// 2026-08-20: "if there are locations at the top of the screen and you hover
// for your hover menu, the hover menu can be cut off by the top of the
// screen"). Controller-reproduced (WebKit, 1440x900, demo at the batch's own
// base): "Sidon marker at viewport y=82 -> place card boundingBox y=-171
// (171px cut off above the top)" -- the DEFAULT `/world` view (no from/to/ref
// at all, CONTRACT's own gospels-era default) reliably renders Sidon at the
// very top of the plate (its own marker's real, live boundingBox.y measured
// well under 100px, confirmed while writing this test), so this uses that
// SAME real scenario directly -- CONTRACT-named ("the DEFAULT /world view"),
// not a search, and the one place-name the controller's own repro already
// names. Explicit 1440x900 (Playwright's own default, 1280x720, is narrower/
// shorter, which shifts fitScene's own auto-fit enough that Sidon no longer
// reliably lands this close to the top -- confirmed live while writing this
// test) -- the SAME viewport the controller's own reproduction used. A
// `precondition` guard (not a hardcoded pixel expectation) still skips
// gracefully rather than asserting the wrong thing if curated data ever
// repositions Sidon or shrinks its card enough that flipping stops being
// necessary -- same "skip rather than fail on a data-dependent precondition"
// shape bestHoverablePlace's own callers already use above.
test('CARD-FLIP-1: a marker near the viewport top flips its place-card below, staying fully within the viewport, and the corridor survives the pointer traveling DOWN onto it', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/world');
  const marker = page.getByTestId('marker-sidon');
  await expect(marker).toBeVisible();
  const markerBox = await marker.boundingBox();
  if (!markerBox || markerBox.y > 150) {
    test.skip(true, 'Sidon is not currently rendered near the viewport top in the default /world view -- precondition for this exact regression no longer holds');
    return;
  }

  await marker.hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  // The card's own rendered height comfortably exceeds the room actually
  // available above the marker (markerBox.y itself) -- i.e. this specific
  // hover genuinely NEEDS to flip, not just happens to pass. Confirms the
  // precondition rather than assuming it.
  const cardBox = await card.boundingBox();
  expect(cardBox).toBeTruthy();
  const GAP = 18;
  expect(cardBox!.height + GAP, 'expected this card to be tall enough that it would NOT fit above the marker unflipped -- precondition for this regression').toBeGreaterThan(markerBox.y);

  // The actual fix: flipped below, attr says so, and the rendered box never
  // crosses the viewport's own top edge (the exact cut-off the user hit).
  await expect(card).toHaveAttribute('data-flip', 'true');
  expect(cardBox!.y, 'place-card boundingBox.y must stay >= 0 -- never cut off by the top of the screen').toBeGreaterThanOrEqual(0);
  // Mirrored below the marker, not just "somewhere on screen": its own top
  // edge sits at (or past) the marker's own Y, the opposite relationship a
  // non-flipped card has (whose BOTTOM sits at the marker's Y instead).
  expect(cardBox!.y).toBeGreaterThanOrEqual(markerBox.y);

  // The corridor itself: moveAndClick's own many-small-steps pointer move
  // (this file's header comment) travels DOWNWARD now (marker above, card
  // below) onto place-card-more specifically -- the SAME safe, non-closing
  // target the pre-existing "hover robustness" test above uses (clicking
  // place-card-title, by contrast, deliberately PROMOTES into a popover and
  // closes the card -- the wrong target for "does the card survive the
  // pointer landing on it"). Success here (more verses reveal, card stays
  // open) is only possible if the existing hover-persistence/grace
  // machinery (_pointerOverCard et al, World.razor) keeps working in this
  // mirrored geometry exactly as it always has in the unflipped one.
  const moreBtn = card.getByTestId('place-card-more');
  await expect(moreBtn).toBeVisible();
  const shownBefore = await card.getByTestId(/^hover-verse-/).count();
  await moveAndClick(page, moreBtn);
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-flip', 'true');
  await expect(card.getByTestId(/^hover-verse-/)).not.toHaveCount(shownBefore);

  // And it still closes normally once the pointer leaves both entirely --
  // the grace/close half of the SAME mechanism, unaffected by orientation.
  await page.mouse.move(0, 0, { steps: 10 });
  await expect(card).toBeHidden({ timeout: 1200 });
});

// The paired "normal" orientation -- CONTRACT's own pre-existing default
// (card renders above, unflipped) MUST keep working byte-for-byte once the
// flip logic exists at all, for the overwhelming common case (a marker with
// genuine room above it). Picks whichever independently-hoverable place in
// WINDOWS has the most merged verses, same discovery bestHoverablePlace's
// other callers in this file already use -- these candidates sit well clear
// of the fitScene padding's own top edge in practice (unlike Sidon's own
// default-view position), so data-flip="false" here is a real, live-
// verified property of this scenario, not an assumption.
test('CARD-FLIP-1 (paired): a marker with room above keeps rendering its place-card there, unflipped, and the corridor survives the pointer traveling UP onto it', async ({ page }) => {
  const best = await bestHoverablePlace(
    page,
    verses => verses.length > initialShownCount(verses),
    verses => verses.length,
  );
  if (!best) {
    test.skip(true, 'no independently-hoverable place with more than its own initial verse count found in any candidate window');
    return;
  }
  const { place } = best;

  const marker = page.getByTestId(`marker-${place.id}`);
  const markerBox = await marker.boundingBox();
  await marker.hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  await expect(card).toHaveAttribute('data-flip', 'false');
  const cardBox = await card.boundingBox();
  expect(cardBox).toBeTruthy();
  expect(cardBox!.y).toBeGreaterThanOrEqual(0);
  // Above the marker, not below: the card's own BOTTOM edge sits at (or
  // before) the marker's own Y -- the mirror-image relationship
  // CARD-FLIP-1's own flipped assertion checks.
  expect(cardBox!.y + cardBox!.height).toBeLessThanOrEqual(markerBox!.y + 1);

  // Corridor traveling UP (marker below, card above) -- the ORIGINAL
  // direction every pre-existing hover-persistence test already covers
  // elsewhere in this file; asserted again here specifically alongside
  // CARD-FLIP-1's own flipped/downward case so "both orientations" is
  // proven by this SAME pair of tests, not split across unrelated ones.
  const moreBtn = card.getByTestId('place-card-more');
  await expect(moreBtn).toBeVisible();
  await moveAndClick(page, moreBtn);
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-flip', 'false');

  await page.mouse.move(0, 0, { steps: 10 });
  await expect(card).toBeHidden({ timeout: 1200 });
});

// Fix round 1 (review finding, Important, live-reproduced by the reviewer:
// "hover the marker at screen point (648, 390)" in the exodus scene at
// 1280x720 resolved to the place titled "Ai" -- an ordinary card, 4 initial
// verses of a real 20-verse Joshua 8 passage, no blurb, no dates -- and
// measured `data-flip="true"`, `cardBox: {y: 426, height: 416}`, i.e. a
// bottom edge of 842 against a 720px-tall viewport: 122px past the bottom,
// the user's own reported bug class moved to the opposite edge, because the
// original cut of CardPlacement.Compute only ever asked "does it fit
// ABOVE" and never checked a flipped card against the container's own
// bottom edge at all).
//
// This reproduces the SAME real place ("Ai", id ai-1 -- the exact Joshua 8
// capture-and-burn passage, still 20 merged verses, still no blurb/dates)
// via `/world?ref=JOS.8` (scripture mode) rather than the reviewer's own
// raw-pixel hover: JOS.8's own scene has only 5 places (Ai/Bethel/Jericho/
// Mount Ebal/Mount Gerizim), sparse enough that marker-ai-1's own locator
// hovers reliably without the exodus window's dense-cluster overlap risk
// (CONTRACT.md's own "best-effort... once markers overlap" caveat) --
// confirmed live: this card's own measured height (416px) exactly matches
// the review's own reported figure, so this is the SAME geometry, just
// reached a more deterministic way.
test('CARD-FLIP-1 (bottom clamp): a card that fits NEITHER orientation at a short viewport clamps fully on-screen, both edges, instead of overflowing the bottom', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto('/world?ref=JOS.8');

  const marker = page.getByTestId('marker-ai-1');
  await expect(marker).toBeVisible();
  const markerBox = await marker.boundingBox();
  await marker.hover({ force: true });

  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card.getByTestId('place-card-title')).toHaveText('Ai');

  const cardBox = await card.boundingBox();
  expect(cardBox).toBeTruthy();
  const viewport = page.viewportSize()!;
  const GAP = 18;

  // Precondition, proven live rather than assumed: this card genuinely
  // fits NEITHER orientation unclamped -- both a naive "render above" and
  // a naive "render below" placement would themselves have crossed an
  // edge. If curated content ever shrinks this specific passage enough
  // that one orientation starts fitting cleanly, this test should skip
  // rather than silently start asserting something it no longer exercises.
  const wouldFitAbove = cardBox!.height <= markerBox!.y - GAP;
  const wouldFitBelow = cardBox!.height <= viewport.height - markerBox!.y - GAP;
  if (wouldFitAbove || wouldFitBelow) {
    test.skip(true, 'Ai/JOS.8 card no longer needs vertical clamping at 1280x720 -- precondition for this regression no longer holds');
    return;
  }

  // The actual fix: fully on-screen, BOTH edges -- never cut off at the
  // top (requirement 1's original bug) and never overflowing the bottom
  // (this fix round's own finding).
  expect(cardBox!.y, 'place-card boundingBox.y must stay >= 0 -- never cut off by the top').toBeGreaterThanOrEqual(0);
  expect(cardBox!.y + cardBox!.height, 'place-card bottom edge must stay <= viewport height -- never overflow the bottom').toBeLessThanOrEqual(viewport.height);

  // The corridor survives this orientation too -- same non-closing target
  // (place-card-more) the other CARD-FLIP-1 tests use, since this card is
  // ALSO a passage-first place with more than its own initial 4 verses.
  const moreBtn = card.getByTestId('place-card-more');
  await expect(moreBtn).toBeVisible();
  const shownBefore = await card.getByTestId(/^hover-verse-/).count();
  await moveAndClick(page, moreBtn);
  await expect(card).toBeVisible();
  await expect(card.getByTestId(/^hover-verse-/)).not.toHaveCount(shownBefore);

  await page.mouse.move(0, 0, { steps: 10 });
  await expect(card).toBeHidden({ timeout: 1200 });
});
