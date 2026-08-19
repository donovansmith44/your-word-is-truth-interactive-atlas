import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-1: rendered markers equal the API scene', async ({ page }) => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
    for (const p of scene.places) {
      await expect(page.getByTestId(`marker-${p.id}`)).toBeAttached();
    }
  }), RUNS_UI);
});

test('WORLD-2: hover card matches scene data', async ({ page }) => {
  const w = { from: -1446, to: -1406 };                    // exodus window: rich scene
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  await fcAssert(fc.asyncProperty(
    fc.integer({ min: 0, max: scene.places.length - 1 }), async i => {
      const p = scene.places[i];
      await page.getByTestId(`marker-${p.id}`).hover({ force: true });
      const card = page.getByTestId('place-card');
      await expect(card).toBeVisible();
      await expect(page.getByTestId('place-card-title')).toHaveText(p.name);
      const groups = new Map<string, number>();
      for (const e of p.events) for (const g of e.verse_groups) {
        groups.set(`${g.book}-${g.chapter}`, (groups.get(`${g.book}-${g.chapter}`) ?? 0) + g.count);
      }
      for (const [k, count] of groups) {
        await expect(card.getByTestId(`verse-group-${k}`)).toContainText(String(count));
      }
      await page.mouse.move(0, 0);
      await expect(card).toBeHidden();
    }), RUNS_UI);
});

// Fix round 1 (M3): map.js's nudgeCloseLatLng golden-angle-spreads places
// that land within CLOSE_THRESHOLD_KM of an already-placed one, but its
// pre-fix version compared each new candidate against already-placed
// markers' FINAL (already-nudged) coordinates rather than their ORIGINAL
// ones -- since a nudge always moves a marker further than the threshold,
// a 3rd (or 4th, ...) place exactly coincident with the first two only ever
// counted the still-unmoved first one as "close" and collapsed onto the
// SAME slot as the 2nd place. Non-manifesting with today's curated data
// (the only exact coincidence, Shittim/Moab-2, is a pair, not a triple --
// see map.js's own comment), so proven here via a real /api/scene response
// (guaranteed to already satisfy the server's wire contract) with three of
// its places' coordinates overwritten to one identical point. This mocks
// the HTTP API response, not a client-internals import -- CONTRACT.md:
// "The UX property suite couples ONLY to this contract... plus the HTTP
// API" -- map.js itself is exercised completely unmodified.
test('WORLD-3: three exactly-coincident places each land on a distinct marker slot', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: rich scene (WORLD-2's own choice)
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.places.length).toBeGreaterThanOrEqual(3);

  // Well inside BIBLICAL_WORLD_BOUNDS (map.js: lat 7.6-48.9, lon -10.9-71.4)
  // so the region lock / fitScene bounds-fit can't clip or distort it.
  const rigged = scene.places.slice(0, 3);
  for (const p of rigged) { p.lat = 33.0; p.lon = 36.0; }

  // A URL predicate, not a glob string -- Playwright glob patterns treat
  // `?` as a one-character wildcard, not a literal query-string separator,
  // which would make intent here easy to misread even though it happens to
  // still match.
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === String(w.from) && url.searchParams.get('to') === String(w.to),
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify(scene),
    }));

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const positions: string[] = [];
  for (const p of rigged) {
    const marker = page.getByTestId(`marker-${p.id}`);
    await expect(marker, `marker-${p.id} should still render at the rigged coincident point`).toBeAttached();
    const box = await marker.boundingBox();
    expect(box, `marker-${p.id} has no bounding box`).not.toBeNull();
    positions.push(`${Math.round(box!.x)},${Math.round(box!.y)}`);
  }
  expect(new Set(positions).size, `expected 3 pairwise-distinct marker slots, got positions ${positions.join(' | ')}`).toBe(3);
});
