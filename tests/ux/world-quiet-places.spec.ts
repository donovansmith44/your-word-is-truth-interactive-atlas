import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch E2 (the ever-present graph, batch-e2-brief.md -- user direction
// 2026-08-19: "change it so that all of the cities in our graph are
// available in any timerange rather than just loading those which are
// biblically active at the time"). QUIET-1's own property coverage (API,
// fast-check over windows) lives in api-scene.spec.ts alongside the other
// SCENE-*/ARROW-* properties; this file covers the UI: the quiet-dot
// marker/card behavior CONTRACT.md now documents, and the brief's own named
// stress case (primeval era: 2 lit places, ~204 quiet dots).

// The primeval era (data/curated/eras.toml: [-4004,-2167]) is the brief's
// own named stress case: only "ararat"/"babel" are lit there (their
// compiled events both narrate the flood/dispersion), so every OTHER
// event-bearing place -- 204 of them, including "jerusalem" -- renders as a
// quiet dot. Chosen deliberately over a richer window for this file's own
// tests: it is the sparsest real scene this app can show, i.e. the
// opposite end of the density spectrum from world-hover-text.spec.ts's own
// WINDOWS (picked for the OPPOSITE reason -- richest, not sparsest).
const PRIMEVAL = { from: -4004, to: -2167 };

test('quiet dots: primeval era renders both embers and quiet dots, counts matching the API exactly', async ({ page }) => {
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  // The brief's own stress-case figures, pinned here as a floor rather than
  // an exact count so a future data change (more compiled events/places)
  // can't silently regress this test into a false pass on a near-empty scene.
  expect(scene.places.length).toBeGreaterThanOrEqual(2);
  expect(scene.quiet_places.length).toBeGreaterThan(100);

  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  // WORLD-1's own assertion shape, extended to the quiet side: rendered
  // marker/quiet-marker counts equal the API scene exactly. The two testid
  // prefixes never collide (`quiet-marker-` does not match `/^marker-/`),
  // so this also stands as a live QUIET-1 disjointness check at the DOM level.
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);
  for (const p of scene.places) {
    await expect(page.getByTestId(`marker-${p.id}`)).toBeAttached();
  }
  for (const qp of scene.quiet_places) {
    await expect(page.getByTestId(`quiet-marker-${qp.id}`)).toBeAttached();
  }
});

// Jerusalem's own compiled events all postdate David's conquest (theo-159,
// -1055, is the earliest) -- it is QUIET, never lit, throughout the
// primeval era, and its curated "Jebus" name range (data/curated/
// place-history.toml: -4004..-1004) plus its "Once the Jebusite
// stronghold..." era blurb both fully cover this window, so this same
// hover exercises NAME-1/BLURB-1's resolution rules on the quiet side too,
// not just the quiet-specific card content.
//
// Real, positive reason this uses `dispatchEvent('mouseover')` rather than
// this suite's usual `hover({force:true})`: Jerusalem is geocoded to the
// EXACT same lat/lon as "Beautiful-gate" (a temple gate, Acts 3:2) --
// confirmed live, 0px apart at every zoom level, never separating (quiet
// dots are deliberately never nudged apart the way lit markers are -- see
// map.js's own quiet-places diff comment: "furniture, not a precision-hover
// surface"). A pixel-hit-test-based hover can therefore NEVER reliably
// distinguish the two -- this is CONTRACT.md's own documented "best-effort,
// not a per-marker guarantee" hover-ambiguity trade-off, now also true,
// unsurprisingly, of two exactly-coincident QUIET places. `dispatchEvent`
// targets Jerusalem's own DOM node directly (still exercising the real
// production `mouseover` listener map.js's `wireEvents` wires to it, via
// Leaflet's own event delegation -- nothing about the app itself is
// mocked/bypassed) rather than depending on which element the browser's
// hit-test happens to resolve at a shared pixel.
test('quiet dot hover card: Jerusalem in the primeval era shows the quiet line, its display_name, curated history, and no verse controls', async ({ page }) => {
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  expect(scene.places.some((p: any) => p.id === 'jerusalem'), 'jerusalem must be quiet, not lit, in the primeval era').toBe(false);
  expect(scene.quiet_places.some((p: any) => p.id === 'jerusalem'), 'jerusalem must be present as a quiet place').toBe(true);

  await page.getByTestId('quiet-marker-jerusalem').dispatchEvent('mouseover');
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  // display_name (NAME-1, quiet side): "Jebus", the curated pre-conquest name.
  await expect(page.getByTestId('place-card-title')).toHaveText('Jebus');

  // The quiet line: exact text, CONTRACT-documented.
  await expect(page.getByTestId('place-card-quiet')).toHaveText('No recorded events in this window — drag the timeline.');

  // History content still shows -- curated for jerusalem, this window.
  await expect(page.getByTestId('place-card-blurb')).toContainText('Jebusite stronghold');
  await expect(page.getByTestId('place-card-dates')).toBeVisible();
  await expect(page.getByTestId('place-card-date-established')).toBeVisible();
  await expect(page.getByTestId('place-card-date-destroyed')).toBeVisible();

  // NO verse section, NO down-arrow (conditional presence).
  await expect(card.locator('[data-testid^="hover-verse-"]')).toHaveCount(0);
  await expect(card.locator('[data-testid^="hover-passage-"]')).toHaveCount(0);
  await expect(page.getByTestId('place-card-more')).toHaveCount(0);
  await expect(page.getByTestId('place-card-collapse')).toHaveCount(0);

  // The card's place remains explorable, exactly like a lit place: the
  // title promotes into a real ExplorerPopover showing Jerusalem's FULL
  // (not window-scoped) event history -- /api/place/jerusalem, no window.
  await page.getByTestId('place-card-title').click();
  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('Jebus');
  const detail = await api.place('jerusalem');
  expect(detail.events.length).toBeGreaterThan(0);
  await expect(page.locator('[data-testid^="place-event-"]')).toHaveCount(detail.events.length);
});

test('quiet dots: scripture mode never shows one', async ({ page }) => {
  await page.goto('/world?ref=EXO.14');
  const scene = await api.sceneScripture('EXO.14');
  expect(scene.quiet_places.length).toBe(0);
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(0);
  // The scene's own lit markers are unaffected (scripture scenes unchanged).
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});

// Isolate mode (batch-e2-brief.md Requirement 2: "quiet dots stay visible
// but never highlighted; arrows/legend logic untouched -- quiet places
// never anchor arrows"). Exercised end to end rather than asserted by
// absence of code: isolating a narrative fades every OTHER narrative's
// arrows (WORLD-4/Legend), which has no notion of quiet places at all
// (ArrowLayer.setIsolate only ever touches `.atlas-arrow`/
// `.atlas-arrow-casing` elements) -- so quiet dots, having nothing to do
// with arrows or the legend, must simply stay exactly as visible/unfaded
// as they were before isolating.
test('quiet dots: stay visible, unaffected, while a narrative is isolated', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: has narratives AND quiet places
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.quiet_places.length).toBeGreaterThan(0);
  expect(scene.narratives.length).toBeGreaterThan(0);

  // expect(...).toHaveCount(...) polls/retries until the map has actually
  // finished rendering -- a bare .count() read here raced the app's own
  // async WASM-boot-then-fetch-then-SetScene startup sequence and could
  // read 0 before any marker existed yet, a test-timing bug, not a real
  // app one (confirmed live while writing this fix).
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);

  await page.getByTestId(`legend-item-${scene.narratives[0].id}`).click();
  await expect(page.getByTestId(`legend-item-${scene.narratives[0].id}`)).toHaveAttribute('aria-pressed', 'true');

  // Still exactly the same quiet dots, all attached and visible -- isolate
  // never touches quiet markers at all.
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);
  const sample = scene.quiet_places[0];
  await expect(page.getByTestId(`quiet-marker-${sample.id}`)).toBeVisible();
});

// Density smoke test -- REWRITTEN under this batch's own mid-flight scope
// amendment (user direction: "have all biblically relevant names of places
// showing on the map at all times... zooming in reveals what collision
// dropped"; "the primeval stress case matters even more now: ~204 quiet
// dots with always-on labels will collide heavily -- that's fine and
// expected... the plate must still breathe through pruning, not through
// tiers"). Quiet labels no longer have a zoom-tier gate at all (map.js's
// applyLabelTier -- see its own comment): every quiet label is now OFFERED
// a chance to render at every zoom, and COLLISION DAMPING alone decides how
// many actually survive in a scene this dense. Verified live against the
// real running app while writing this test: 204 quiet labels enter the
// collision pass, ~44 survive at the primeval era's own natural (unzoomed)
// fitScene view -- a real, working density floor/ceiling (never all 204,
// never zero) rather than an exact brittle count, since the precise number
// depends on real screen-pixel geometry (viewport size, this specific
// cluster of curated coordinates) that a future data or layout change could
// shift without the underlying mechanism being wrong.
test('density smoke: primeval era (204 quiet dots, always-on labels) prunes via collision damping alone, never all-or-nothing', async ({ page }) => {
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);

  // Every dot renders regardless of density -- dots themselves are never
  // zoom- or collision-gated, only labels are.
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);

  const quietLabels = page.locator('.quiet-label');
  await expect(quietLabels).toHaveCount(scene.quiet_places.length); // every label element exists in the DOM...
  const visibleCount = await quietLabels.evaluateAll(
    els => els.filter(el => window.getComputedStyle(el).display !== 'none').length);
  // ...but collision damping -- not a tier -- is what prunes most of them:
  // some survive (there IS a working mechanism putting names on the plate),
  // most don't (204 labels cannot all fit a normal viewport without stacking).
  expect(visibleCount, 'at least one quiet label should win an uncontested cell').toBeGreaterThan(0);
  expect(visibleCount, 'collision damping should prune the large majority of 204 labels at this density').toBeLessThan(scene.quiet_places.length / 2);

  // The embers -- furniture's counterpart, the unmistakable foreground --
  // are completely unaffected by any of this: still exactly 2, still glowing,
  // and (PLACE_PRIORITY_BASE) always win any contested cell against a quiet
  // label, so both ararat's and babel's own labels are always among the
  // visible survivors regardless of how the 204 quiet ones collide.
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  for (const p of scene.places) {
    // `marker-{id}` IS the `.atlas-marker` div itself (map.js's makeIcon
    // sets the testid directly on it, not a wrapper) -- checking its own
    // class list for SOME glow-N (not asserting the exact N, which depends
    // on this place's own brightness) proves it's still a lit, glowing
    // marker without duplicating brightness->glow-class arithmetic here.
    const marker = page.getByTestId(`marker-${p.id}`);
    await expect(marker).toHaveClass(/\bglow-\d\b/);
    await expect(marker.locator('.atlas-label')).toBeVisible();
  }
});

// Names stay time-accurate on the MAP LABEL itself, not just the card title
// (batch-e2-brief.md scope amendment point 2: "verify a curated rename
// resolves correctly on a QUIET place too"). Reads the label's own
// textContent directly rather than requiring it to currently be VISIBLE --
// Jerusalem sits inside the primeval era's own densest cluster (the Levant),
// so whether its specific label happens to win its collision cell this run
// is exactly the kind of real-geometry detail the density smoke test above
// already treats as expected variance, not something this rename-accuracy
// check should depend on.
test('quiet dot map label: a curated rename resolves correctly on a quiet place (Jerusalem -> Jebus, primeval era)', async ({ page }) => {
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  const label = page.getByTestId('quiet-marker-jerusalem').locator('.quiet-label');
  await expect(label).toHaveCount(1);
  await expect(label).toHaveText('Jebus');
});
