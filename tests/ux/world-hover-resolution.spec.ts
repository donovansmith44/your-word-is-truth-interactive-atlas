import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { VISIBLE_LIT_MARKER_SELECTOR } from './lib/markers';
import { zoomInOnMarker } from './lib/zoom';

test('HOVER-RESOLUTION-1: clicking each visible lit marker in the exile window opens that place, never a nearer quiet dot', async ({ page }) => {
  // Arrange
  const scene = await api.sceneTime(-590, -580);
  const label = new Map<string, string>(scene.places.map((p: { id: string; node: { label: string } }) => [p.id, p.node.label]));
  await page.goto('/world?from=-590&to=-580');
  await page.waitForSelector('[data-testid="marker-jerusalem"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-jerusalem', 1);
  const ids = await page.locator(VISIBLE_LIT_MARKER_SELECTOR).evaluateAll(els => els
    .filter(el => { const r = el.getBoundingClientRect(); return r.x > 0 && r.y > 0 && r.right < window.innerWidth && r.bottom < window.innerHeight; })
    .map(el => el.getAttribute('data-testid')!.replace(/^marker-/, '')));
  expect(ids).toContain('jerusalem');
  const opened: Record<string, string | null> = {};

  // Act
  for (const id of ids) {
    await page.getByTestId(`marker-${id}`).click({ force: true });
    await expect(page.getByTestId('popover-title')).toBeVisible();
    opened[id] = await page.getByTestId('popover-title').textContent();
    await page.getByTestId('popover-close').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);
  }

  // Assert
  expect(opened).toEqual(Object.fromEntries(ids.map(id => [id, label.get(id)])));
});
