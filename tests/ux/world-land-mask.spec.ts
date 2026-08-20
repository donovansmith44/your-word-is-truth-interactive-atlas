import { test, expect, Page } from '@playwright/test';
import { api } from './lib/api';

// Batch R requirement 1 ("borders become part of the plate", user
// 2026-08-19: "the borders still suck and are overlays on the actual map,
// when they need to be PART of the actual map"): the curated land mask
// clips every polity wash so it never spills into open sea. Verified via
// the REAL SVG geometry (SVGGeometryElement.isPointInFill against
// BorderLayer's own actual clipPath child <path> elements, in the exact
// projected coordinate space the wash paths themselves are drawn in) --
// deterministic and pixel-noise-free, not a screenshot-based color-sample
// heuristic (this suite carries no image-decoding dependency, and a
// geometric point-in-polygon check is strictly more precise anyway; see the
// batch report's own self-review for the tradeoff). Mirrors split-view.spec.ts's
// own `readCamera` pattern for reaching map.js's real, live module state
// via a dynamic import from page.evaluate.
async function isPointOnLand(page: Page, lat: number, lon: number): Promise<boolean | null> {
  return page.evaluate(async ({ lat, lon }) => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m.debugIsPointOnLand(ids[ids.length - 1], lat, lon);
  }, { lat, lon });
}

// A deterministic open-sea point: the Mediterranean, south of Cyprus, well
// clear of the Levant/Sinai/Egypt/Anatolia coastlines the curated mask
// traces (data/curated/land-mask.toml's own region 1) and clear of every
// OTHER curated region too (region 4a/4b's own southern-Europe/N-Africa
// sketch, region 5's Cyprus oval) -- verified offline against the actual
// curated ring data before being picked (see the batch report).
const SEA_POINT = { lat: 33.5, lon: 29.5 };
// A deterministic land point in the SAME core region, for a same-mechanism
// sanity check that this isn't a helper that just always returns false.
const LAND_POINT = { lat: 31.78, lon: 35.22 }; // Jerusalem

test('LAND-1: GET /api/land-mask returns real, simple ring geometry', async () => {
  const mask = await api.landMask();
  expect(Array.isArray(mask.rings)).toBe(true);
  expect(mask.rings.length).toBeGreaterThanOrEqual(1);
  for (const ring of mask.rings) {
    expect(ring.length).toBeGreaterThanOrEqual(4);
    // Curated convention (matches the polity rings): closed, first point
    // repeats as the last.
    expect(ring[0]).toEqual(ring[ring.length - 1]);
  }
});

test('LAND-1: a deterministic open-sea point is clipped out; a deterministic land point is not', async ({ page }) => {
  // Egypt's own wash (a coastal polity, its ring hugging the Nile/Delta/
  // Mediterranean coast) is guaranteed present in this window -- the
  // scenario the clip actually protects.
  await page.goto('/world?from=-1446&to=-1400');
  await expect(page.getByTestId(/^polity-ring-egypt-/).first()).toBeAttached();

  await expect.poll(() => isPointOnLand(page, SEA_POINT.lat, SEA_POINT.lon)).toBe(false);
  await expect.poll(() => isPointOnLand(page, LAND_POINT.lat, LAND_POINT.lon)).toBe(true);
});

// The clip itself: `.atlas-wash-clip-group` (every wash <path> lives inside
// it) carries a `clip-path` computed style referencing the SAME
// `atlas-land-clip` id the geometry above was checked against -- proves the
// mask is actually WIRED to the wash, not merely present and correct in
// isolation.
test('LAND-1: the wash clip group is actually wired to the land-mask clipPath', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400');
  await expect(page.locator('path.atlas-border-wash').first()).toBeAttached();

  const clipPath = await page.locator('.atlas-wash-clip-group').evaluate(el => getComputedStyle(el).clipPath);
  expect(clipPath).toContain('atlas-land-clip');

  // Band/line strokes stay UNCLIPPED and UNFEATHERED -- "ink border strokes
  // stay crisp" -- living in the plain overlayPane SVG, never washPane.
  const bandFilter = await page.locator('path.atlas-border-band').first().evaluate(el => getComputedStyle(el).filter);
  expect(bandFilter === 'none' || bandFilter === '').toBeTruthy();
});

// Feather: the wash fill itself carries a subtle blur filter (requirement
// 1's own "printed tint soaking into paper, not neon glow") -- checked by
// computed style presence, not a pixel-diff (this suite has no
// image-decoding dependency; see the batch report).
test('LAND-1: the wash fill carries the feather filter', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400');
  const wash = page.locator('path.atlas-border-wash').first();
  await expect(wash).toBeAttached();
  const filter = await wash.evaluate(el => getComputedStyle(el).filter);
  expect(filter).toContain('atlas-wash-feather');
});

// Zoom discipline: an animated zoom must never desync the clip from the
// wash it clips -- same acceptance shape BORDERS-9 (world-borders.spec.ts)
// already uses for the border/arrow SVG layers' own zoomanim transform, now
// checked for the wash's own washPane SVG (which the clipPath geometry
// lives inside, per LAND-1's own CONTRACT note) instead.
test('LAND-1 (zoom-sync): the wash SVG resets to an identity zoomanim transform immediately after an animated zoom, and the clip keeps excluding the sea point', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1400');
  await expect(page.getByTestId(/^polity-ring-egypt-/).first()).toBeAttached();

  for (let i = 0; i < 3; i++) {
    await page.mouse.move(640, 360);
    await page.mouse.wheel(0, -300);
  }

  await expect(async () => {
    const t = await page.locator('svg.atlas-borders-wash').evaluate(el => (el as HTMLElement).style.transform);
    expect(t, `expected an identity transform, got "${t}"`).toMatch(/scale\(1\)\s*$/);
  }).toPass();

  await expect.poll(() => isPointOnLand(page, SEA_POINT.lat, SEA_POINT.lon)).toBe(false);
});
