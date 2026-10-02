import { test, expect, Page, Locator } from '@playwright/test';
import { api } from './lib/api';
import { zoomInOnMarker as zoomInOnMarkerTestId } from './lib/zoom';

const AFTER_SHILOH = { from: -900, to: -800 };
const INSIDE_SHILOH = { from: -1250, to: -1150 };

async function zoomInOnMarker(page: Page, marker: Locator): Promise<void> {
  const box = (await marker.boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  for (let i = 0; i < 6; i++) {
    await page.mouse.move(cx, cy);
    await page.mouse.wheel(0, -300);
  }
}

test('existence gating: a place destroyed before the window shows its dot, no label', async ({ page }) => {
  const scene = await api.sceneTime(AFTER_SHILOH.from, AFTER_SHILOH.to);
  const shiloh = scene.quiet_places.find((p: any) => p.id === 'shiloh');
  expect(shiloh, "shiloh must be quiet (no events) in this window -- see this file's own header comment").toBeTruthy();
  expect(shiloh.existence_from).toEqual({ label: '1399 BC', value: -1399 });
  expect(shiloh.existence_to).toEqual({ label: '1050 BC', value: -1050 });

  await page.goto(`/world?from=${AFTER_SHILOH.from}&to=${AFTER_SHILOH.to}`);
  const dot = page.getByTestId('quiet-marker-shiloh');
  await expect(dot).toBeAttached();
  await expect(dot).toBeVisible();
  await zoomInOnMarker(page, dot);

  const label = dot.locator('.quiet-label');
  await expect(label).toHaveCount(1);
  await expect(label).toBeHidden();

  await dot.dispatchEvent('click');
  await expect(page.getByTestId('popover-title')).toHaveText((await api.node(shiloh.node.id)).label);
});

test('existence gating: a window overlapping the existence range shows the label normally', async ({ page }) => {
  const scene = await api.sceneTime(INSIDE_SHILOH.from, INSIDE_SHILOH.to);
  const lit = scene.places.find((p: any) => p.id === 'shiloh');
  const quiet = scene.quiet_places.find((p: any) => p.id === 'shiloh');
  expect(lit ?? quiet, 'shiloh must appear, lit or quiet, in this window').toBeTruthy();

  await page.goto(`/world?from=${INSIDE_SHILOH.from}&to=${INSIDE_SHILOH.to}`);
  const marker = lit ? page.getByTestId('marker-shiloh') : page.getByTestId('quiet-marker-shiloh');
  await expect(marker).toBeVisible();
  await zoomInOnMarker(page, marker);

  const label = marker.locator(lit ? '.atlas-label' : '.quiet-label');
  await expect(label).toHaveCount(1);
  await expect(label).toBeVisible();
});

test('existence gating: a place with no curated existence bounds always labels', async ({ page }) => {
  const w = { from: -1446, to: -1406 };
  const scene = await api.sceneTime(w.from, w.to);
  const jericho = scene.places.find((p: any) => p.id === 'jericho-1');
  expect(jericho, 'jericho-1 must be lit in the exodus window').toBeTruthy();
  expect(jericho.existence_from).toBeUndefined();
  expect(jericho.existence_to).toBeUndefined();

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  const marker = page.getByTestId('marker-jericho-1');
  await zoomInOnMarkerTestId(page, 'marker-jericho-1', 6);
  await expect(marker).toBeVisible();

  const label = marker.locator('.atlas-label');
  await expect(label).toBeVisible();
});

test('existence gating: scripture mode never gates (no window to test outside-ness against)', async ({ page }) => {
  await page.goto('/world?ref=JOS.18');
  const scene = await api.sceneScripture('JOS.18');
  const shiloh = scene.places.find((p: any) => p.id === 'shiloh');
  expect(shiloh, 'shiloh must be lit via JOS.18 (the tabernacle event)').toBeTruthy();

  const marker = page.getByTestId('marker-shiloh');
  await expect(marker).toBeVisible();
  await zoomInOnMarker(page, marker);

  const label = marker.locator('.atlas-label');
  await expect(label).toBeVisible();
});
