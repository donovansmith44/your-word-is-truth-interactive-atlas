import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('READ-4: mini-map equals scripture scene; open-in-world carries the ref', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-num-${v}`).click();
    await page.getByTestId('popover-chip-map').click();
    await expect(page.getByTestId('mini-map')).toBeVisible();
    const scene = await api.sceneScripture(vref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
    await page.getByTestId('mini-map-open-world').click();
    await page.waitForURL(u => u.pathname === '/world' && u.searchParams.get('ref') === vref);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  }), RUNS_UI);
});

test('WORLD-8: place card title opens place history popover', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1406');
  const scene = await api.sceneTime(-1446, -1406);
  const p = scene.places[0];
  await page.getByTestId(`marker-${p.id}`).hover({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover-title')).toHaveText(p.name);
  const detail = await api.place(p.id);
  await expect(page.getByTestId('popover'))
    .toContainText(String(Math.abs(detail.events[0].when.from_year)));
});

test('READ-5: shift-click passage selection', async ({ page }) => {
  const toc = await loadToc();
  const arb = arbChapterRef(toc).filter(c => c.verses >= 4).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses - 1 }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b).map(([a, b]) => ({ ...c, a, b })));
  await fcAssert(fc.asyncProperty(arb, async s => {
    await page.goto(`/read/${s.book}/${s.chapter}`);
    await page.getByTestId(`verse-num-${s.a}`).click();
    await page.keyboard.down('Shift');
    await page.getByTestId(`verse-num-${s.b}`).click();
    await page.keyboard.up('Shift');
    const pref = `${s.book}.${s.chapter}.${s.a}-${s.b}`;
    await expect(page.getByTestId('passage-chip')).toContainText(pref);
    await page.getByTestId('passage-chip').click();
    await expect(page.getByTestId('popover-title')).toHaveText(pref);
    await page.getByTestId('popover-chip-map').click();
    const scene = await api.sceneScripture(pref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
  }), RUNS_UI);
});
