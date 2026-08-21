import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch N requirement 3 ("map focus sync -- the same graph, seen twice",
// user direction 2026-08-20, verbatim: "the appropriate narrative lines on
// the map side ought to be brought into particular focus relative to the
// other lines in the narrative"). CONTRACT.md's own EVENT-1 note (Batch T
// retargets the mechanism from Verse/NarrativeEventNode onto EventNode
// only -- a plain VERSE popover no longer carries a narrative POSITION of
// its own, only EVENT membership, so map focus now activates once the
// popover actually reaches an EventNode, not merely a narrative-linked
// verse).
//
// Reuses world-pin.spec.ts's own established "known narrative chain"
// window (the curated `exodus` narrative's full route, every leg lit, not
// quiet) rather than inventing a second one -- same reasoning that file's
// own header comment gives.
const EXODUS_WINDOW = { from: -1446, to: -1406 };

// Same dispatchEvent-not-pixel-click technique world-pin.spec.ts's own
// clickMarker already established for this exact scene (a documented dense
// marker cluster -- app.css's own .atlas-marker comment).
async function clickMarker(page: import('@playwright/test').Page, placeId: string): Promise<void> {
  await page.getByTestId(`marker-${placeId}`).dispatchEvent('click');
}

// Every arrow whose OWN narrative is `narrativeId`.
function arrowLocator(page: import('@playwright/test').Page, narrativeId?: string) {
  return narrativeId
    ? page.locator(`[data-testid^="arrow-${narrativeId}-"]`)
    : page.locator('[data-testid^="arrow-"]');
}

async function focusStates(locator: ReturnType<typeof arrowLocator>): Promise<(string | null)[]> {
  return locator.evaluateAll(els => els.map(el => el.getAttribute('data-narrative-focus')));
}

test('EVENT-1: map focus classes flip on EventNode open, follow traversal, and clear on close', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const toRedSea = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_succoth' && a.to_event === 'ex_red_sea');
  test.skip(!toRedSea, 'exodus ex_succoth -> ex_red_sea leg not present in this window');
  const succothId = toRedSea.from_place;

  const exodusArrows = arrowLocator(page, 'exodus');
  await expect(exodusArrows.first()).toBeVisible();
  // Plain CSS :not() -- Locator.filter's own hasNot/hasNotText options only
  // ever test a DESCENDANT relationship, which every arrow (a leaf <path>,
  // no children) would trivially "pass" regardless of its own narrative.
  const otherArrows = page.locator('[data-testid^="arrow-"]:not([data-testid^="arrow-exodus-"])');
  const otherArrowCount = await otherArrows.count();

  // Baseline, before any narrative context is open: no arrow -- exodus's
  // own or any other narrative's -- carries the attribute at all.
  for (const state of await focusStates(page.locator('[data-testid^="arrow-"]'))) {
    expect(state).toBeNull();
  }

  // Pin succoth-2 (ex_succoth's own place) and open EXO.13.20's own verse
  // popover via its place-card's hover-verse row (G1 requirement 1b) --
  // "map-side popovers," per the brief's own "works identically in full
  // reader, split view, and map-side popovers." Then click through the
  // verse's own "EVENT" row to reach the EventNode -- map focus activates
  // there, not on the bare VerseNode (see this file's own header comment).
  await clickMarker(page, succothId);
  await expect(page.getByTestId('place-card')).toHaveAttribute('data-pinned', 'true');
  await page.getByTestId('hover-verse-EXO.13.20').click();
  await expect(page.getByTestId('popover-title')).toHaveText('EXO.13.20');
  // Promoting into the popover closes the pinned card underneath it (PIN-1).
  await expect(page.getByTestId('place-card')).toHaveCount(0);
  await page.getByTestId('verse-event-ex_succoth').click();
  await expect(page.getByTestId('popover-title')).toHaveText('First camp at Succoth');

  // Settle gate BEFORE any raw (non-retrying) DOM read -- ExplorerPopover.razor's
  // own LoadCurrent comment documents that Blazor renders an intermediate
  // frame at the method's FIRST await, where Current.Title (EventNode's own
  // title, known synchronously from its constructor -- no fetch) is already
  // correct, while SyncNarrativeFocusAsync -- the thing that actually calls
  // MapInterop.SetNarrativeFocus and mutates these very attributes -- only
  // runs LATER, after every section provider's own fetch resolves. So
  // popover-title turning correct is not proof the map has finished
  // updating; wait on an auto-retrying assertion of the map's OWN state
  // first (this one), THEN read the rest with a raw, non-retrying loop.
  const currentLegs = page.locator('[data-testid^="arrow-exodus-"][data-narrative-focus="current"]');
  const expectedCurrentCount = scene.arrows.filter((a: any) => a.narrative === 'exodus' && (a.from_event === 'ex_succoth' || a.to_event === 'ex_succoth')).length;
  await expect(currentLegs).toHaveCount(expectedCurrentCount);

  // Every exodus arrow is now amplified (active or current, never absent/
  // receded); the CURRENT leg(s) -- the one(s) touching ex_succoth, the
  // node actually open -- carry the strongest state.
  for (const state of await focusStates(exodusArrows)) {
    expect(['active', 'current']).toContain(state);
  }

  // Every OTHER narrative's own arrow recedes -- dimmed, never removed from
  // the DOM (still present, still findable) and never the same as isolate's
  // own near-invisible data-faded state (a SEPARATE attribute entirely).
  if (otherArrowCount > 0) {
    for (const state of await focusStates(otherArrows)) {
      expect(state).toBe('receded');
    }
    await expect(otherArrows.first()).toBeVisible();
    await expect(otherArrows.first()).not.toHaveAttribute('data-faded', 'true');
  }

  // Traverse: FOLLOWING moves the subject to ex_red_sea -- focus follows
  // live. The leg that was "current" a moment ago (ex_rameses -> ex_succoth)
  // is now merely "active" (still amplified, no longer the strongest); the
  // leg(s) touching ex_red_sea become "current" instead.
  await page.getByTestId('event-following-event-exodus').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Crossing the Red Sea');

  const ramesesToSuccoth = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_rameses' && a.to_event === 'ex_succoth');
  if (ramesesToSuccoth) {
    await expect(page.getByTestId(`arrow-exodus-${ramesesToSuccoth.order}`)).toHaveAttribute('data-narrative-focus', 'active');
  }
  const newCurrentCount = scene.arrows.filter((a: any) => a.narrative === 'exodus' && (a.from_event === 'ex_red_sea' || a.to_event === 'ex_red_sea')).length;
  await expect(currentLegs).toHaveCount(newCurrentCount);

  // Close: every arrow -- exodus's own and every other narrative's --
  // returns fully to baseline (attribute absent, not merely re-faded).
  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);
  for (const state of await focusStates(page.locator('[data-testid^="arrow-"]'))) {
    expect(state).toBeNull();
  }
});

// Requirement 3's own explicit accessibility line: "Reduced-motion: state
// change without animated transition." Realized by never declaring a CSS
// transition on data-narrative-focus at all (app.css's own "an instant
// snap needs no prefers-reduced-motion carve-out" precedent) -- a
// computed-style check, same class/state-assertion approach BLINK-1's own
// reduced-motion test already uses, proving the "no transition" claim is
// true regardless of the media query (not merely true because reduced
// motion additionally disables something).
test('EVENT-1: data-narrative-focus state changes carry no CSS transition (reduced-motion is a no-op change, by design)', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const arrow = page.locator('[data-testid^="arrow-exodus-"]').first();
  await expect(arrow).toBeVisible();
  // transition-property's own CSS-spec INITIAL value is "all" even with
  // zero transition rules declared anywhere -- computing to "all" alone
  // proves nothing. transition-duration's own initial value is "0s",
  // which is the actual, load-bearing proof that no property change
  // (opacity/stroke-width included) ever animates here, matching this
  // file's own app.css comment ("an instant snap needs no
  // prefers-reduced-motion carve-out").
  const transitionDuration = await arrow.evaluate(el => getComputedStyle(el).transitionDuration);
  expect(transitionDuration).toBe('0s');
});
