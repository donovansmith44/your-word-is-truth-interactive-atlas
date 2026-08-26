import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { formatClaim } from './lib/years';
import { zoomInOnMarker } from './lib/zoom';

// Batch E (batch-e-brief.md, "time-accurate places"): NAME-1 (period-true
// display names swap at their curated boundary, visible on the map/card),
// BLURB-1 (exactly one blurb, era-vs-broad per the window), and DATE-1 (a
// curated date's Explore affordance opens the popover listing its curated
// supporting verses first). Windows below are chosen against REAL compiled
// data/curated/place-history.toml content and cross-checked against the
// real compiled events that light each marker -- see batch-e-report.md for
// the full per-window verification (in particular why Jerusalem, not
// Bethel, is this suite's own on-screen "watch it swap while dragging"
// example: bethel-1's only compiled events all postdate its own Luz/Bethel
// boundary, so "Luz" is correctly resolved and API-reachable but never
// lights a marker on its own -- Jerusalem's theo-159 event (dated -1055,
// inside the curated Jebus range) does).

// Fix round 1 (batch-e-review.md MAJOR-2): every click below targets a
// button INSIDE an already-open, already-hovered card -- world-hover-text.
// spec.ts's own moveAndClick helper exists for a DIFFERENT problem (a
// marker-to-card HOVER transition, where a single-jump move can miss
// delivering the pointerenter PlaceCard's own hover-stays-open logic
// needs), which none of these clicks are. Reviewer reproduced a ~60%
// failure rate on one such click (a custom helper that snapshots
// boundingBox() ONCE and dispatches raw mouse events at that stale
// position, racing .place-card's own CSS entrance animation -- the second
// click in a test has no incidental settle time the way a first click
// right after several `await expect(...)` calls does) and confirmed
// Playwright's own `.click()` (which re-reads the target's position and
// waits for it to be visually STABLE, not just attached, immediately
// before dispatching) passes 5/5 in the identical scenario. Plain
// `.click()` throughout this file, no custom helper.

test('NAME-1: Jerusalem/Jebus swaps on the marker label and card title when the window crosses the curated boundary', async ({ page }) => {
  // Before David's conquest (theo-159, "Reign of David", dated -1055 --
  // its own initial card content is 2 Samuel 2's Hebron narrative, which
  // never says "Jerusalem", so this window is citation-safe to screenshot;
  // see the file header).
  await page.goto('/world?from=-1060&to=-1050');
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(page.getByTestId('marker-jerusalem').getByText('Jebus', { exact: true })).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Jebus');

  // After the conquest -- the exile/destruction window (also exercises
  // established/destroyed being present at the very same hover).
  await page.goto('/world?from=-590&to=-580');
  await page.waitForSelector('[data-testid="marker-jerusalem"]', { state: 'attached' });
  // Batch C3: this window's own lit set (siege/destruction-era places,
  // scattered well beyond just Jerusalem's own immediate vicinity) fits to
  // a wide enough zoom that Jerusalem can land in a far/mid-tier cluster
  // glyph -- lib/zoom.ts's own zoomInOnMarker (same technique this file's
  // pre-C3 own header comment already flagged this exact risk for) walks
  // it into NEAR tier first, where decision 3 guarantees clustering stops.
  await zoomInOnMarker(page, 'marker-jerusalem', 3);
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  await expect(page.getByTestId('place-card')).toBeVisible();
  await expect(page.getByTestId('marker-jerusalem').getByText('Jerusalem', { exact: true })).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Jerusalem');
});

test('NAME-1: a window fully inside one curated era never shows the OTHER era\'s name (Hebron/Kirjath-arba resolution is API-consistent with the card)', async () => {
  // Cross-checks the API resolution the card itself calls, for a second
  // curated place -- api-place-history.spec.ts already exhaustively pins
  // the boundary years at the API layer; this just confirms the SAME
  // values are what PlaceCard would render (DateText/display_name come
  // straight from this same endpoint).
  const before = await api.placeHistory('hebron', -3000, -2500);
  expect(before.history.display_name).toBe('Kirjath-arba');
  const after = await api.placeHistory('hebron', -2000, -1900);
  expect(after.history.display_name).toBe('Hebron');
});

test('BLURB-1: exactly one blurb shows, era inside one range, broad summary once the window spans both', async ({ page }) => {
  // Inside Jerusalem's first curated "era" blurb range only.
  await page.goto('/world?from=-1060&to=-1050');
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  const blurb = card.getByTestId('place-card-blurb');
  await expect(blurb).toBeVisible();
  await expect(blurb).toContainText('Jebusite stronghold');

  // The default bare-/world window (Gospels, -5..29) sits inside the
  // SECOND curated era range only. Checked at the API layer (same
  // production resolve_blurb the card itself calls), not via a marker
  // hover, for the same reason the whole-span check below already is:
  // Jerusalem's own true position is, batch-hotfix2-report.md confirms,
  // literally 1px from Bethesda's at this window's own fitScene zoom (both
  // real, distinct places -- Bethesda's pool sits inside Jerusalem) --
  // `debugTrueScreenPoint`-measured, not a guess -- so a force:true hover
  // on `marker-jerusalem` here is exactly the "let them overlap" case the
  // batch's own nudge redesign explicitly accepts (brief: "if two distinct
  // places are so close that 20px cannot separate them at the current
  // zoom, let them overlap"), not a bug in the merge/nudge fix itself.
  const gospels = await api.placeHistory('jerusalem', -5, 29);
  expect(gospels.history.blurb).toContain('Second Temple city');
  expect(gospels.history.blurb).not.toContain('Jebusite stronghold');

  // A window spanning BOTH of Jerusalem's curated "era" ranges resolves to
  // the BROAD summary instead of either era one -- "a broad period -> a
  // broad blurb; don't stack everything for Jerusalem" (user direction,
  // 2026-08-19). Checked at the API layer (api-place-history.spec.ts's own
  // BLURB-1 test, same production resolve_blurb the card itself calls)
  // rather than via a marker hover here: a window this wide lights so many
  // geographically-scattered places (confirmed live) that fitScene zooms
  // out far enough for Jerusalem's own marker to visually overlap its
  // close neighbors (e.g. Bethany, "on the Mount of Olives' eastern
  // slope"), and force:true hovers whatever the browser's real hit-testing
  // finds topmost at that pixel -- not necessarily the intended marker.
  const wholeSpan = await api.placeHistory('jerusalem', -4004, 100);
  expect(wholeSpan.history.blurb).toContain("Jerusalem's story runs");
});

test('DATE-1: the established date affordance opens a popover that lists curated supporting verses first, then "Show this time on the map"', async ({ page }) => {
  await page.goto('/world?from=-590&to=-580');
  await page.waitForSelector('[data-testid="marker-jerusalem"]', { state: 'attached' });
  // Batch C3: see NAME-1's own comment above -- this window's own wide
  // fitScene zoom can cluster Jerusalem at far/mid tier.
  await zoomInOnMarker(page, 'marker-jerusalem', 3);
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  const dates = card.getByTestId('place-card-dates');
  await expect(dates).toBeVisible();
  const established = card.getByTestId('place-card-date-established');
  await expect(established).toBeVisible();
  await expect(established).toContainText(formatClaim(-1003, -1003, 'traditional')); // "c. 1003 BC"
  await expect(card.getByTestId('place-card-date-destroyed')).toContainText(formatClaim(-586, -586, null)); // "586 BC", no "c."

  await established.click();
  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(`Established ${formatClaim(-1003, -1003, 'traditional')}`);

  // Supporting verses lead, in curated order, all BEFORE the map chip.
  const chipTestIds = await popover.locator('.popover-head-actions [data-testid]').evaluateAll(
    els => els.map(el => el.getAttribute('data-testid')));
  expect(chipTestIds).toEqual([
    'popover-chip-verse-2SA.5.6',
    'popover-chip-verse-2SA.5.7',
    'popover-chip-verse-2SA.5.9',
    'popover-chip-map',
  ]);

  // Following the first supporting-verse chip pushes a real VerseNode --
  // popover-title becomes the vref, body is that verse's own KJV text.
  await page.getByTestId('popover-chip-verse-2SA.5.6').click();
  await expect(page.getByTestId('popover-title')).toHaveText('2SA.5.6');
  const detail = await api.verse('2SA.5.6');
  await expect(popover).toContainText(detail.text);

  // Breadcrumb back returns to the YearNode.
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(`Established ${formatClaim(-1003, -1003, 'traditional')}`);

  await page.getByTestId('popover-close').click();
  await expect(popover).toHaveCount(0);

  // Activating a date affordance closes the hover card immediately (same
  // "promote into a popover, never let two parchment artifacts overlap"
  // rule OnOpenPlace/place-card-title already follows) -- re-hover to
  // reopen a fresh card before testing the SECOND date.
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  await expect(card).toBeVisible();
  const destroyed = card.getByTestId('place-card-date-destroyed');

  // The destroyed date opens its OWN popover, with ITS OWN curated verses
  // (no "c." -- no note curated for this claim).
  await destroyed.click();
  await expect(page.getByTestId('popover-title')).toHaveText(`Destroyed ${formatClaim(-586, -586, null)}`);
  const destroyedChips = await page.getByTestId('popover').locator('.popover-head-actions [data-testid]').evaluateAll(
    els => els.map(el => el.getAttribute('data-testid')));
  expect(destroyedChips).toEqual(['popover-chip-verse-2KI.25.9', 'popover-chip-verse-2KI.25.10', 'popover-chip-map']);
});

test('DATE-1: "Show this time on the map" navigates /world to the claim\'s own window', async ({ page }) => {
  await page.goto('/world?from=-590&to=-580');
  await page.waitForSelector('[data-testid="marker-jerusalem"]', { state: 'attached' });
  // Batch C3: see NAME-1's own comment above -- this window's own wide
  // fitScene zoom can cluster Jerusalem at far/mid tier.
  await zoomInOnMarker(page, 'marker-jerusalem', 3);
  await page.getByTestId('marker-jerusalem').hover({ force: true });
  await page.getByTestId('place-card-date-established').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await page.getByTestId('popover-chip-map').click();
  await page.waitForURL(u => u.pathname === '/world' && u.searchParams.get('from') === '-1003' && u.searchParams.get('to') === '-1003');
});

test('place-card-blurb and place-card-dates are absent for a place with no curated history', async ({ page }) => {
  // The full 18 ids data/curated/place-history.toml covers -- everything
  // else lit in this window is a genuinely uncurated place.
  const CURATED = new Set([
    'babylon-1', 'beersheba-2', 'bethany-1', 'bethel-1', 'egypt', 'haran', 'hebron', 'jericho-1',
    'jerusalem', 'shechem', 'nineveh', 'dan', 'rome', 'antioch-1', 'samaria_1022', 'shiloh',
    'capernaum', 'damascus',
  ]);
  const w = { from: -1406, to: -1405 };
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  const plain = scene.places.find((p: any) => !CURATED.has(p.id));
  if (!plain) {
    test.skip(true, 'no uncurated place lit in this window');
    return;
  }
  await page.getByTestId(`marker-${plain.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card.getByTestId('place-card-blurb')).toHaveCount(0);
  await expect(card.getByTestId('place-card-dates')).toHaveCount(0);
});
