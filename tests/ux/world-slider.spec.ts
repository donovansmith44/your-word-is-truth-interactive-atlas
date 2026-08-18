import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { formatRange } from './lib/years';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-5: typed readout drives window, URL and readout agree', async ({ page }) => {
  await page.goto('/world?from=-1450&to=-1400');
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.getByTestId('slider-readout').fill(formatRange(w.from, w.to));
    await page.getByTestId('slider-readout').press('Enter');
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(w.from, w.to));
    await page.waitForURL(u => u.searchParams.get('from') === String(w.from)
                            && u.searchParams.get('to') === String(w.to));
  }), RUNS_UI);
});

test('WORLD-7: every era is on the slider and clickable (exhaustive)', async ({ page }) => {
  const eras = await api.eras();
  await page.goto('/world?from=-1450&to=-1400');
  for (const e of eras) {
    const label = page.getByTestId(`slider-era-${e.id}`);
    await expect(label).toContainText(e.name);
    await label.click();
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(e.from_year, e.to_year));
  }
});

test('NAV-1 (world/time): deep link survives reload', async ({ page }) => {
  await page.goto('/world?from=-1406&to=-1405');
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
  await page.reload();
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
});

test('errors surface as toast, app keeps standing', async ({ page }) => {
  await page.goto('/world?from=0&to=5');
  await expect(page.getByTestId('toast')).toBeVisible();
  await expect(page.getByTestId('world-map')).toBeAttached();
});
