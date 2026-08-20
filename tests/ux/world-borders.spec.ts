import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { formatRange } from './lib/years';

// Black-box mirror of map.js's slugify (kebab-case of the landmark's own
// name) -- must match it exactly for landmark-{slug} testids to resolve.
// Kept local to this spec file (no client imports, per the black-box UX
// suite rule).
function kebab(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
}

test('BORDERS-1: every era resolves to a real, non-empty border snapshot (exhaustive)', async () => {
  const eras = await api.eras();
  expect(eras.length).toBeGreaterThan(0);
  for (const e of eras) {
    const res = await api.borders(e.from_year, e.to_year);
    expect(res.__status, `era ${e.id}: ${JSON.stringify(res)}`).toBeUndefined();
    expect(Number.isInteger(res.snapshot_year), `era ${e.id}: snapshot_year=${res.snapshot_year}`).toBe(true);
    expect(res.geojson?.features?.length, `era ${e.id}`).toBeGreaterThan(0);
  }
});

test('BORDERS-2: border tag reflects the loaded snapshot and updates on in-app window navigation', async ({ page }) => {
  await page.goto('/world?from=-2000&to=-1900');
  const tag = page.getByTestId('border-tag');
  await expect(tag).toBeVisible();
  await expect(tag).toContainText('Borders c.');
  const firstText = (await tag.textContent())?.trim();

  // At least one real vector path is on the map for this snapshot, distinct
  // from narrative-arrow paths (arrows-svg's own .atlas-arrow elements) --
  // an honest, stable signal that the border layer actually rendered
  // geometry rather than just showing the tag text.
  const mapPaths = page.locator('[data-testid="world-map"] path:not(.atlas-arrow)');
  await expect(mapPaths.first()).toBeAttached();
  expect(await mapPaths.count()).toBeGreaterThan(0);

  // Navigate in-app via the readout (not a reload) to a window nowhere near
  // the first one -- the border snapshot must change accordingly.
  await page.getByTestId('slider-readout').fill(formatRange(40, 60));
  await page.getByTestId('slider-readout').press('Enter');
  await page.waitForURL(u => u.searchParams.get('from') === '40' && u.searchParams.get('to') === '60');

  await expect(tag).toBeVisible();
  await expect(async () => {
    const nowText = (await tag.textContent())?.trim();
    expect(nowText).not.toBe(firstText);
  }).toPass();

  expect(await mapPaths.count()).toBeGreaterThan(0);
});

test('BORDERS-3: every curated landmark is attached and styled by kind', async ({ page }) => {
  const landmarks = await api.landmarks();
  expect(landmarks.length).toBeGreaterThan(0);

  await page.goto('/world');

  for (const l of landmarks) {
    const slug = kebab(l.name);
    await expect(page.getByTestId(`landmark-${slug}`), `landmark "${l.name}" (kind ${l.kind})`).toBeAttached();
  }

  const water = landmarks.find((l: any) => l.kind === 'water');
  const mountain = landmarks.find((l: any) => l.kind === 'mountain');
  expect(water, 'expected at least one water landmark in the curated set').toBeTruthy();
  expect(mountain, 'expected at least one mountain landmark in the curated set').toBeTruthy();

  const waterStyle = await page.getByTestId(`landmark-${kebab(water.name)}`).evaluate(el => getComputedStyle(el).fontStyle);
  expect(waterStyle).toBe('italic');

  const mountainStyle = await page.getByTestId(`landmark-${kebab(mountain.name)}`).evaluate(el => getComputedStyle(el).fontStyle);
  expect(mountainStyle).not.toBe('italic');
});

test('BORDERS-4: scripture mode hides the border tag', async ({ page }) => {
  await page.goto('/world?ref=EXO.14');
  await expect(page.getByTestId('border-tag')).toHaveCount(0);
});

// Batch C2, Requirement 1 (hand-tinted polity area washes) + Requirement 3's
// own test list: "every rendered border feature has a fill with opacity in
// the chosen value ±0" and "polity-name -> color determinism... assert
// identical fill color... across two snapshots". Black-box mirror of
// map.js's own POLITY_TINTS/polityFillColor (same "no client-internals
// import" rule this file's own kebab() already follows) -- a pure function
// of the feature's NAME alone, so this doesn't need to guess or hardcode
// which hex a given polity gets; it computes the SAME thing map.js does and
// checks the live app agrees.
const POLITY_TINTS = ['#C98A8A', '#C9B37E', '#93A98B', '#7E99B5', '#A18CB0', '#B59B7E', '#8FA07A', '#C08E7A'];
function polityFillColor(name: string): string {
  const normalized = name.trim().toLowerCase().replace(/\s+/g, ' ');
  let sum = 0;
  for (let i = 0; i < normalized.length; i++) {
    sum += normalized.charCodeAt(i);
  }
  return POLITY_TINTS[sum % POLITY_TINTS.length];
}
function hexToRgbString(hex: string): string {
  const n = parseInt(hex.slice(1), 16);
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
}

// Batch C2 fix round 1 (review M1): the brief's ORIGINAL 0.10-0.14 ceiling
// was itself wrong -- screenshot + pixel-sampling review proved 0.14 (that
// ceiling's own top) reads as indistinguishable from bare terrain, not a
// hand-tinted wash. Re-ruled band: 0.22-0.35 (design-direction.md's own
// Atlas plate detail addendum carries the same corrected number); shipped
// value 0.32 (app.css's own .atlas-border comment has the full
// screenshot-review history). This property still checks a RANGE, not a
// hardcoded single value, for the same reason it always did: fill-opacity
// is a shared, class-wide design value (like the palette hexes), not a
// wire-level contract number CONTRACT.md pins -- see the batch report's own
// "CONTRACT amendments" reasoning.
test('BORDERS-5: every rendered border feature shares one fill-opacity inside the 0.22-0.35 range', async ({ page }) => {
  await page.goto('/world?from=-2000&to=-1900');
  const paths = page.locator('[data-testid="world-map"] .atlas-border');
  await expect(paths.first()).toBeAttached();
  const opacities = await paths.evaluateAll(els => els.map(el => getComputedStyle(el).fillOpacity));
  expect(opacities.length).toBeGreaterThan(0);

  const chosen = parseFloat(opacities[0]);
  expect(chosen).toBeGreaterThanOrEqual(0.22);
  expect(chosen).toBeLessThanOrEqual(0.35);
  for (const raw of opacities) {
    // "±0" -- every feature's own fill-opacity is the exact same value,
    // not merely close to it (app.css sets this once, class-wide, never
    // per-feature -- see .atlas-border's own comment).
    expect(raw).toBe(opacities[0]);
  }
});

test('BORDERS-6: a polity name determines its fill color, identically, across two different border snapshots', async ({ page }) => {
  // Two real, different snapshot years (data/compiled/borders/*.json) that
  // both happen to carry "Roman Empire" at feature index 0 -- confirmed via
  // the live API, not assumed (see this test's own two windows below).
  const windows = [
    { from: -40, to: -20 }, // -> snapshot -1
    { from: 50, to: 90 },   // -> snapshot 100
  ];

  const expectedRgb = hexToRgbString(polityFillColor('Roman Empire'));

  for (const w of windows) {
    const borders = await api.borders(w.from, w.to);
    const idx = borders.geojson.features.findIndex((f: any) => f.properties.name === 'Roman Empire');
    expect(idx, `Roman Empire not found in the snapshot for ${w.from}..${w.to} (got ${borders.geojson.features.map((f: any) => f.properties.name)})`).toBeGreaterThanOrEqual(0);

    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const path = page.locator('[data-testid="world-map"] .atlas-border').nth(idx);
    await expect(path).toBeAttached();
    const fill = await path.evaluate(el => getComputedStyle(el).fill);
    expect(fill, `Roman Empire's own fill at snapshot ${borders.snapshot_year}`).toBe(expectedRgb);
  }
});

// Requirement 3's own far-field smoke test: "at least N curated far-field
// landmark labels visible outside the active border features' bounding box
// ... the point is 'not bereft'". The full 4004 BC - AD 100 span is the
// SAME "default zoom-out" window WORLD-10 (world-map.spec.ts) already uses
// and confirms lands in the FAR zoom tier -- the honest stand-in for "as
// zoomed out as this app's own UI ever naturally goes" (fitScene has no
// separate "show me everything" affordance; a scene this wide is the widest
// real view). N=6 of 7 landmarks confirmed independently, deterministically
// visible here across repeated runs (see the batch report) -- one below
// that count for a small honest margin, matching this suite's own
// "pick honest N from your curation" instruction rather than the exact
// observed count.
test('BORDERS-7: the populated far field -- at least 6 curated landmarks outside the active border snapshot render visible at the widest natural view', async ({ page }) => {
  const w = { from: -4004, to: 100 };
  const borders = await api.borders(w.from, w.to);
  let minLat = Infinity, maxLat = -Infinity, minLon = Infinity, maxLon = -Infinity;
  for (const f of borders.geojson.features) {
    for (const polygon of f.geometry.coordinates) {
      for (const ring of polygon) {
        for (const [lon, lat] of ring) {
          minLat = Math.min(minLat, lat); maxLat = Math.max(maxLat, lat);
          minLon = Math.min(minLon, lon); maxLon = Math.max(maxLon, lon);
        }
      }
    }
  }

  const landmarks = await api.landmarks();
  const outside = landmarks.filter((l: any) => l.lat < minLat || l.lat > maxLat || l.lon < minLon || l.lon > maxLon);
  expect(outside.length, 'expected at least some curated landmarks outside the active snapshot bbox').toBeGreaterThan(0);

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  // This window's own scene is 200+ places (WORLD-10's own comment) --
  // slower to fetch/render/settle (applyLabelTier's own tier+collision
  // passes included) than this suite's usual small windows. A plain
  // isVisible() check (unlike expect().toBeVisible(), which polls) reads
  // the CURRENT state with no retry, so it needs the map to have actually
  // finished settling first -- waiting for the landmark layer's own first
  // element existing at all also guarantees setLandmarks/applyLabelTier
  // have run at least once.
  await page.waitForSelector('.landmark-label', { timeout: 15000 });
  await page.waitForTimeout(500);

  function kebab(name: string): string {
    return name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
  }
  let visibleCount = 0;
  for (const l of outside) {
    if (await page.getByTestId(`landmark-${kebab(l.name)}`).isVisible().catch(() => false)) {
      visibleCount++;
    }
  }
  expect(visibleCount, `expected at least 6 of ${outside.length} outside-bbox landmarks visible; the plate must not read as bereft at its widest view`).toBeGreaterThanOrEqual(6);
});
