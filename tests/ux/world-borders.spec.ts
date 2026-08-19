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
