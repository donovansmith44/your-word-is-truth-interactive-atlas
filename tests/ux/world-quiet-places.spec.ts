import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { LIT_MARKER_TESTID } from './lib/markers';

const PRIMEVAL = { from: -4004, to: -2167 };

test('quiet dots: primeval era renders both embers and quiet dots, counts matching the API exactly', async ({ page }) => {
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  expect(scene.places.length).toBeGreaterThanOrEqual(2);
  expect(scene.quiet_places.length).toBeGreaterThan(100);

  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);
  for (const p of scene.places) {
    await expect(page.getByTestId(`marker-${p.id}`)).toBeAttached();
  }
  for (const qp of scene.quiet_places) {
    await expect(page.getByTestId(`quiet-marker-${qp.id}`)).toBeAttached();
  }
});

test('quiet dot hover card: hovering Jerusalem\'s quiet dot in the primeval era opens nothing; clicking it opens its window-free record with its curated fields and site-of events', async ({ page }) => {
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  expect(scene.places.some((p: any) => p.id === 'jerusalem'), 'jerusalem must be quiet, not lit, in the primeval era').toBe(false);
  expect(scene.quiet_places.some((p: any) => p.id === 'jerusalem'), 'jerusalem must be present as a quiet place').toBe(true);
  const record = await api.node('Place:jerusalem');
  const siteOf = record.edge_summary.find((g: any) => g.kind === 'site-of').count;
  const clamp = 20;
  expect(siteOf, 'jerusalem must carry more site-of events than the clamp').toBeGreaterThan(clamp);
  const links = page.getByTestId('popover-section-site-of').locator('[data-testid^="popover-link-site-of-"]');

  await page.getByTestId('quiet-marker-jerusalem').dispatchEvent('mouseover');
  await page.waitForTimeout(600);
  await expect(page.getByTestId('popover')).toHaveCount(0);
  await page.getByTestId('quiet-marker-jerusalem').dispatchEvent('click');

  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-field-Blurb')).toContainText(record.place.blurb);
  await expect(page.getByTestId('popover-field-Established')).toContainText(record.place.established.label);
  await expect(page.getByTestId('popover-field-Destroyed')).toContainText(record.place.destroyed.label);
  await expect(page.getByTestId('popover-section-site-of-heading')).toHaveText(`Site of (${siteOf})`);
  await expect(links).toHaveCount(clamp);
  await page.getByTestId('popover-section-site-of-more').click();
  await expect(links).toHaveCount(Math.min(2 * clamp, siteOf));
  await page.getByTestId('popover-section-site-of-collapse').click();
  await expect(links).toHaveCount(clamp);
});

test('quiet dot click opens the SAME window-free record a lit marker does, survives pointer-leave, closes via popover-close', async ({ page }) => {
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  const record = await api.node('Place:jerusalem');
  const popover = page.getByTestId('popover');

  await page.getByTestId('quiet-marker-jerusalem').dispatchEvent('click');

  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-section-site-of')).toBeVisible();
  await page.mouse.move(5, 5);
  await page.waitForTimeout(1200);
  await expect(popover).toBeVisible();
  await page.getByTestId('popover-close').click();
  await expect(popover).toHaveCount(0);
});

test('quiet dots: scripture mode never shows one', async ({ page }) => {
  await page.goto('/world?ref=EXO.14');
  const scene = await api.sceneScripture('EXO.14');
  expect(scene.quiet_places.length).toBe(0);
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(0);
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
});

test('quiet dots: stay visible, unaffected, while a narrative is isolated', async ({ page }) => {
  const w = { from: -1446, to: -1406 };
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.quiet_places.length).toBeGreaterThan(0);
  expect(scene.narratives.length).toBeGreaterThan(0);

  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);

  await page.getByTestId(`legend-item-${scene.narratives[0].id}`).click();
  await expect(page.getByTestId(`legend-item-${scene.narratives[0].id}`)).toHaveAttribute('aria-pressed', 'true');

  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);
  const sample = scene.quiet_places[0];
  await expect(page.getByTestId(`quiet-marker-${sample.id}`)).toBeVisible();
});

test('density smoke: primeval era (204 quiet dots, always-on labels) prunes via collision damping alone, never all-or-nothing', async ({ page }) => {
  const scene = await api.sceneTime(PRIMEVAL.from, PRIMEVAL.to);
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);

  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(scene.quiet_places.length);

  const quietLabels = page.locator('.quiet-label');
  await expect(quietLabels).toHaveCount(scene.quiet_places.length);
  const visibleCount = await quietLabels.evaluateAll(
    els => els.filter(el => window.getComputedStyle(el).display !== 'none').length);
  expect(visibleCount, 'at least one quiet label should win an uncontested cell').toBeGreaterThan(0);
  expect(visibleCount, 'collision damping should prune the large majority of 204 labels at this density').toBeLessThan(scene.quiet_places.length / 2);

  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
  for (const p of scene.places) {
    const marker = page.getByTestId(`marker-${p.id}`);
    await expect(marker).toHaveClass(/\bglow-\d\b/);
    await expect(marker.locator('.atlas-label')).toBeVisible();
  }
});

test('quiet dot map label: a curated rename resolves correctly on a quiet place (Jerusalem -> Jebus, primeval era)', async ({ page }) => {
  await page.goto(`/world?from=${PRIMEVAL.from}&to=${PRIMEVAL.to}`);
  const label = page.getByTestId('quiet-marker-jerusalem').locator('.quiet-label');
  await expect(label).toHaveCount(1);
  await expect(label).toHaveText('Jebus');
});
