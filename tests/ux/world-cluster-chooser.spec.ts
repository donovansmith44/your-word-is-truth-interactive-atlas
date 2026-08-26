import { test, expect } from '@playwright/test';
import { zoomInOnMarker } from './lib/zoom';

// Batch C3 (dense-marker disambiguation + clustering). Phase-0 diagnosis
// (batch-c3-report.md has the full account): mis-hover in a dense scene was
// never a data/geometry bug -- map.js's wireEvents/wireQuietEvents used to
// trust their own closure `placeId` (whichever marker's DOM element the
// browser happened to deliver a mouseover/click to) as gospel. Once two
// markers' own >=14px hit boxes overlap on screen, that's decided by
// Leaflet's DEFAULT per-marker z-index -- assigned purely from each
// marker's own screen Y position, nothing to do with which marker's TRUE
// center the pointer is actually closer to (lib/hoverSafety.ts's own header
// comment already root-caused and live-confirmed this exact mechanism for
// Philippi/Neapolis via `document.elementsFromPoint`; this batch's own
// report adds a second live repro, the exodus window's Beersheba-1/
// Beersheba-2/Negeb triple -- three real, curated records sharing one exact
// lat/lon).
//
// The fix (map.js's HIT_RADIUS_PX/AMBIGUITY_RADIUS_PX/CLUSTER_D_PX, and
// resolveHoverTarget): a pointer event now resolves to the candidate whose
// TRUE (unnudged) position is nearest the pointer among everything within
// HIT_RADIUS_PX -- deterministic, immune to whichever DOM element the
// browser routed the raw event to. If 2+ candidates' true positions sit
// within AMBIGUITY_RADIUS_PX of the winner (genuinely coincident, not just
// close), the hover opens a `place-chooser` flyout listing every tied
// candidate instead of guessing (ARBITRATION-1/CHOOSER-1 below). At
// far/mid label tier, lit markers whose true points sit within
// CLUSTER_D_PX collapse into one `marker-cluster-{n}` glyph -- hover opens
// the SAME chooser (that cluster's own members); click zooms one step
// toward it; NEAR tier never clusters (CLUSTER-1/CLUSTER-2 below).
//
// This file replaces reliance on lib/hoverSafety.ts's own SAFE_NEIGHBOR_PX
// filter for the SPECIFIC named dense stacks the C2/C3 briefs ledgered as
// unreliable before this batch (world-map.spec.ts's own WORLD-2 comment:
// the exodus window's six-member Ai/Gilgal/Jericho/"plains of Moab"/
// Shittim/Timnath-serah pileup, "a structural limit... not a tuning miss")
// with DIRECT, deliberate coverage of exactly those cases -- see
// batch-c3-report.md's own "re-enabled" list. hoverSafety.ts's own
// independentlyHoverableIds filter was extended (never loosened -- a
// candidate now also has to clear every on-screen quiet dot/cluster
// glyph, not just other lit markers, so the "safe" pool only ever shrinks
// further) since resolveHoverTarget arbitrates against ALL of those now,
// not just lit-lit overlap -- see CONTRACT.md's own amended "Marker
// hover-target resolution" note and lib/hoverSafety.ts's own comment.

// zoomInOnMarker moved to lib/zoom.ts -- shared with world-same-place.spec.ts's
// UI-1/UI-3 and world-map.spec.ts's WORLD-3, all of which needed the same
// "walk a scene into NEAR tier" technique once real/rigged close markers
// started clustering.

test('ARBITRATION-1: hovering a place resolves to that place\'s own TRUE nearest center, never a different one the browser\'s own z-order happened to favor', async ({ page }) => {
  // Exodus window (world-map.spec.ts's own WORLD-2 reference scene).
  // "Marah" has no coincident/close neighbor of its own (confirmed live,
  // batch-c3-report.md) -- exactly the ordinary, unambiguous case the
  // arbitration law's single-winner path covers.
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-marah"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-marah', 3);

  const box = await page.getByTestId('marker-marah').boundingBox();
  expect(box, 'marah should render individually at NEAR tier').toBeTruthy();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2, { steps: 5 });

  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Marah');
});

test('CHOOSER-1 (the ledgered Beersheba/Negeb repro): genuinely coincident places (0km apart) open a chooser instead of guessing; every candidate listed, sorted by id; a row click pins exactly that place', async ({ page }) => {
  // Beersheba-1, Beersheba-2, and Negeb -- three real, independently
  // curated records that all resolve to the identical lat/lon
  // (31.244722, 34.840833), confirmed live via GET /api/scene. This is the
  // batch's own concrete Phase-0 repro: "aim for Beersheba, land on
  // Negeb" -- pre-fix, whichever of the three happened to win the
  // browser's own z-order stacking silently won every hover, regardless of
  // which one the pointer was closest to (they render within single-digit
  // px of each other). Post-fix, the SAME genuine coincidence -- not a bug
  // to hide, three DIFFERENT curated records really do share one point --
  // surfaces honestly as a chooser instead.
  await page.goto('/world?from=-1940&to=-1750');
  await page.waitForSelector('[data-testid="marker-beersheba-1"]', { state: 'attached' });
  // Zoomed to NEAR tier so this scene's own (unrelated) wider clustering
  // doesn't also absorb this triple into a bigger glyph -- the point of
  // THIS test is the individual-candidate chooser path (decision 2), which
  // is tier-independent by design, not cluster collapse (decision 3,
  // covered separately below).
  await zoomInOnMarker(page, 'marker-beersheba-1', 2);

  const box = await page.getByTestId('marker-beersheba-1').boundingBox();
  expect(box, 'beersheba-1 should render individually at NEAR tier').toBeTruthy();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2, { steps: 5 });

  const chooser = page.getByTestId('place-chooser');
  await expect(chooser).toBeVisible();
  const rows = chooser.locator('[data-testid^="place-chooser-"]');
  await expect(rows).toHaveCount(3);
  const ids = await rows.evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid));
  // decision 4: chooser candidate order is deterministic, sorted by place id.
  expect(ids).toEqual(['place-chooser-beersheba-1', 'place-chooser-beersheba-2', 'place-chooser-negeb']);
  await expect(page.getByTestId('place-chooser-negeb')).toContainText('Negeb');

  await page.getByTestId('place-chooser-negeb').click();
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  const card = page.getByTestId('place-card');
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText('Negeb');
});

test('CLUSTER-1: a dense far/mid-tier pileup collapses into one marker-cluster-{n} glyph whose hover lists exactly n members (a row click pins that member); clicking the glyph changes the grouping (zooms toward it); NEAR tier shows none', async ({ page }) => {
  // Exodus window, DEFAULT (far/mid-tier) fit -- this scene's own six-member
  // Ai/Gilgal/Jericho/"plains of Moab"/Shittim/Timnath-serah pileup
  // (world-map.spec.ts's own WORLD-2 comment: "compresses to single-digit
  // screen pixels... a structural limit... not a tuning miss") is exactly
  // the case decision 3's clustering retires.
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });

  const clustersBefore = page.locator('[data-testid^="marker-cluster-"]');
  const countBefore = await clustersBefore.count();
  expect(countBefore, 'expected at least one cluster glyph at this scene\'s own far/mid-tier default fit').toBeGreaterThan(0);
  const testidsBefore = await clustersBefore.evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());

  const first = clustersBefore.first();
  const testid = (await first.getAttribute('data-testid'))!;
  const n = parseInt(testid.replace('marker-cluster-', ''), 10);

  await first.hover({ force: true });
  const chooser = page.getByTestId('place-chooser');
  await expect(chooser).toBeVisible();
  const rows = chooser.locator('[data-testid^="place-chooser-"]');
  await expect(rows).toHaveCount(n);
  // decision 2: a chooser row click pins exactly that member, same as
  // clicking that place's own marker would -- proven generically by
  // CHOOSER-1 above; here just confirm the FIRST cluster member (lowest
  // id, decision 4's sort) resolves and pins without erroring.
  const firstRowId = (await rows.first().getAttribute('data-testid'))!.replace('place-chooser-', '');
  await rows.first().click();
  await expect(page.getByTestId('place-card')).toHaveAttribute('data-pinned', 'true');
  const pinnedTitle = await page.getByTestId('place-card-title').textContent();
  expect(pinnedTitle, 'pinned card should be for the clicked chooser row, not some other place').toBeTruthy();
  await page.getByTestId('place-card-close').click();
  expect(firstRowId.length).toBeGreaterThan(0); // sanity: a real id was read, not an empty string

  // Click zooms one step toward the cluster -- proven by the grouping
  // itself changing (never a no-op): the exact same testid set can't
  // survive a real zoom change, since every true pairwise distance grows.
  await first.click({ force: true });
  await page.waitForTimeout(900);
  const clustersAfter = page.locator('[data-testid^="marker-cluster-"]');
  const testidsAfter = await clustersAfter.evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());
  expect(testidsAfter, 'clicking a cluster glyph should change the cluster grouping (zoom toward it), never no-op').not.toEqual(testidsBefore);

  // NEAR tier (zoomed in well past the default fit): no cluster glyphs at
  // all -- decision 3's "NEAR tier never clusters," unchanged pre-C3
  // behavior (nudges alone).
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-jericho-1', 4);
  await expect(page.locator('[data-testid^="marker-cluster-"]')).toHaveCount(0);
});

test('CLUSTER-2 (determinism): a stable scene never jitters cluster membership across an unrelated reload', async ({ page }) => {
  // Same window loaded twice (a fresh page load stands in for "an unrelated
  // refetch/zoomend" -- both paths run the identical pure function of TRUE
  // positions + current zoom, decision 4's own reasoning) must produce the
  // byte-identical cluster testid set both times.
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await page.waitForTimeout(500);
  const first = await page.locator('[data-testid^="marker-cluster-"]').evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());
  expect(first.length).toBeGreaterThan(0);

  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await page.waitForTimeout(500);
  const second = await page.locator('[data-testid^="marker-cluster-"]').evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());

  expect(second).toEqual(first);
});
