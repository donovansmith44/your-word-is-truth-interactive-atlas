import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { VISIBLE_LIT_MARKER_SELECTOR } from './lib/markers';
import { zoomInOnMarker } from './lib/zoom';

test('HOVER-RESOLUTION-1: hovering each visible lit marker in the exile window opens that place, never a nearer quiet dot', async ({ page }) => {
  const scene = await api.sceneTime(-590, -580);
  const displayName = new Map<string, string>(scene.places.map((p: any) => [p.id, p.display_name]));
  await page.goto('/world?from=-590&to=-580');
  await page.waitForSelector('[data-testid="marker-jerusalem"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-jerusalem', 1);

  const ids = await page.locator(VISIBLE_LIT_MARKER_SELECTOR).evaluateAll(els => els
    .filter(el => { const r = el.getBoundingClientRect(); return r.x > 0 && r.y > 0 && r.right < window.innerWidth && r.bottom < window.innerHeight; })
    .map(el => el.getAttribute('data-testid')!.replace(/^marker-/, '')));
  expect(ids).toContain('jerusalem');
  const opened: Record<string, string | null> = {};
  for (const id of ids) {
    await page.mouse.move(2, 2);
    await expect(page.getByTestId('place-card')).toHaveCount(0);
    await page.getByTestId(`marker-${id}`).hover({ force: true });
    await expect(page.getByTestId('place-card')).toBeVisible();
    opened[id] = await page.getByTestId('place-card-title').textContent();
  }

  expect(opened).toEqual(Object.fromEntries(ids.map(id => [id, displayName.get(id)])));
});
