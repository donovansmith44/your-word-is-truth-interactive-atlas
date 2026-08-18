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
