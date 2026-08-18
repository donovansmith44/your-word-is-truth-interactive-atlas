import { test, expect } from '@playwright/test';
import { api } from './lib/api';

const WINDOWS = [ { from: -1446, to: -1406 }, { from: -2100, to: -2085 }, { from: 46, to: 48 } ];

test('WORLD-3: rendered arrows equal API arrows with correct stroke and arrowheads', async ({ page }) => {
  for (const w of WINDOWS) {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId('arrows-svg').locator('path[data-testid^="arrow-"]'))
      .toHaveCount(scene.arrows.length);
    for (const a of scene.arrows) {
      const path = page.getByTestId(`arrow-${a.narrative}-${a.order}`);
      await expect(path).toHaveAttribute('stroke', a.color);
      await expect(path).toHaveAttribute('marker-end', /url\(/);
      await expect(path).toHaveAttribute('data-faded', 'false');
    }
  }
});

test('WORLD-4: legend isolate fades exactly the other narratives, toggles back', async ({ page }) => {
  const w = WINDOWS[0];
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  if (scene.narratives.length === 0) test.skip();
  const target = scene.narratives[0].id;
  await page.getByTestId(`legend-item-${target}`).click();
  for (const a of scene.arrows) {
    await expect(page.getByTestId(`arrow-${a.narrative}-${a.order}`))
      .toHaveAttribute('data-faded', a.narrative === target ? 'false' : 'true');
  }
  await page.getByTestId(`legend-item-${target}`).click();
  for (const a of scene.arrows) {
    await expect(page.getByTestId(`arrow-${a.narrative}-${a.order}`)).toHaveAttribute('data-faded', 'false');
  }
});

test('arrow hover shows the narrative tooltip', async ({ page }) => {
  const w = WINDOWS[0];
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  if (scene.arrows.length === 0) test.skip();
  const a = scene.arrows[0];
  await page.getByTestId(`arrow-${a.narrative}-${a.order}`).hover({ force: true });
  const name = scene.narratives.find((n: any) => n.id === a.narrative)!.name;
  await expect(page.getByTestId('arrow-tip')).toContainText(name);
});
