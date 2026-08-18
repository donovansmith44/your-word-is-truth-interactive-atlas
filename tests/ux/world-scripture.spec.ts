import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbChapterRef } from './lib/canon';
import { formatRange } from './lib/years';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-6: dropdown override and return-to-time', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    const sref = `${c.book}.${c.chapter}`;
    await page.goto('/world?from=-1446&to=-1406');
    await page.getByTestId('picker-book').selectOption(c.book);
    await page.getByTestId('picker-chapter').selectOption(String(c.chapter));
    await page.getByTestId('picker-apply').click();
    await page.waitForURL(u => u.searchParams.get('ref') === sref);
    await expect(page.getByTestId('mode-chip')).toContainText(sref);
    await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
    const scene = await api.sceneScripture(sref);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
    await page.getByTestId('mode-chip-return').click();
    await page.waitForURL(u => u.searchParams.get('from') === '-1446'
                            && u.searchParams.get('to') === '-1406');
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1446, -1406));
  }), RUNS_UI);
});

test('NAV-1 (world/ref): scripture deep link survives reload', async ({ page }) => {
  await page.goto('/world?ref=EXO.14');
  const scene = await api.sceneScripture('EXO.14');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  await page.reload();
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});
