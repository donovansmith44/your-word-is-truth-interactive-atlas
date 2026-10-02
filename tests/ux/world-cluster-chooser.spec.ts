import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { zoomInOnMarker } from './lib/zoom';

test('ARBITRATION-1: clicking a place resolves to that place\'s own TRUE nearest center, never a different one the browser\'s own z-order happened to favor', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:marah');
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-marah"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-marah', 3);
  const box = await page.getByTestId('marker-marah').boundingBox();
  expect(box, 'marah should render individually at NEAR tier').toBeTruthy();

  // Act
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2, { steps: 5 });
  await page.mouse.down();
  await page.mouse.up();

  // Assert
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('CHOOSER-1 (the ledgered Beersheba/Negeb repro): genuinely coincident places (0km apart) open a chooser instead of guessing; every candidate listed, sorted by id; a row click opens exactly that place', async ({ page }) => {
  // Arrange
  const negeb = await api.node('Place:negeb');
  await page.goto('/world?from=-1940&to=-1750');
  await page.waitForSelector('[data-testid="marker-beersheba-1"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-beersheba-1', 2);
  const box = await page.getByTestId('marker-beersheba-1').boundingBox();
  expect(box, 'beersheba-1 should render individually at NEAR tier').toBeTruthy();
  await page.mouse.move(box!.x + box!.width / 2, box!.y + box!.height / 2, { steps: 5 });
  const chooser = page.getByTestId('place-chooser');
  await expect(chooser).toBeVisible();
  const rows = chooser.locator('[data-testid^="place-chooser-"]');
  await expect(rows).toHaveCount(3);
  const ids = await rows.evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid));
  expect(ids).toEqual(['place-chooser-beersheba-1', 'place-chooser-beersheba-2', 'place-chooser-negeb']);
  await expect(page.getByTestId('place-chooser-negeb')).toContainText('Negeb');

  // Act
  await page.getByTestId('place-chooser-negeb').click();

  // Assert
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  await expect(page.getByTestId('popover-title')).toHaveText(negeb.label);
});

test('CLUSTER-1: a dense far/mid-tier pileup collapses into one marker-cluster-{n} glyph whose hover lists exactly n members (a row click opens that member); clicking the glyph changes the grouping (zooms toward it); NEAR tier shows none', async ({ page }) => {
  // Arrange
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  const clustersBefore = page.locator('[data-testid^="marker-cluster-"]');
  expect(await clustersBefore.count(), 'expected at least one cluster glyph at this scene\'s own far/mid-tier default fit').toBeGreaterThan(0);
  const testidsBefore = await clustersBefore.evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());
  const first = clustersBefore.first();
  const n = parseInt((await first.getAttribute('data-testid'))!.replace('marker-cluster-', ''), 10);

  // Act
  await first.hover({ force: true });
  const rows = page.getByTestId('place-chooser').locator('[data-testid^="place-chooser-"]');
  await expect(rows).toHaveCount(n);
  const firstRowId = (await rows.first().getAttribute('data-testid'))!.replace('place-chooser-', '');
  const member = await api.node(`Place:${firstRowId}`);
  await rows.first().click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(member.label);
  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);

  // Act
  await first.click({ force: true });
  await page.waitForTimeout(900);

  // Assert
  const testidsAfter = await page.locator('[data-testid^="marker-cluster-"]').evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());
  expect(testidsAfter, 'clicking a cluster glyph should change the cluster grouping (zoom toward it), never no-op').not.toEqual(testidsBefore);

  // Act
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-jericho-1', 4);

  // Assert
  await expect(page.locator('[data-testid^="marker-cluster-"]')).toHaveCount(0);
});

test('CLUSTER-2 (determinism): a stable scene never jitters cluster membership across an unrelated reload', async ({ page }) => {
  // Arrange
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await page.waitForTimeout(500);
  const first = await page.locator('[data-testid^="marker-cluster-"]').evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());
  expect(first.length).toBeGreaterThan(0);

  // Act
  await page.goto('/world?from=-1446&to=-1406');
  await page.waitForSelector('[data-testid="marker-jericho-1"]', { state: 'attached' });
  await page.waitForTimeout(500);
  const second = await page.locator('[data-testid^="marker-cluster-"]').evaluateAll(els => els.map(el => (el as HTMLElement).dataset.testid).sort());

  // Assert
  expect(second).toEqual(first);
});

test('C3-M1: Philippi and Neapolis (a real, non-coincident, 13.92km-apart close pair) each resolve to their own true center -- aim each, get each', async ({ page }) => {
  await page.goto('/world?from=49&to=52');
  await page.waitForSelector('[data-testid="marker-philippi"]', { state: 'attached' });
  await page.waitForSelector('[data-testid="marker-neapolis"]', { state: 'attached' });

  await zoomInOnMarker(page, 'marker-philippi', 3);
  const philippiBox = await page.getByTestId('marker-philippi').boundingBox();
  expect(philippiBox, 'philippi should render individually at NEAR tier').toBeTruthy();
  await page.mouse.move(philippiBox!.x + philippiBox!.width / 2, philippiBox!.y + philippiBox!.height / 2, { steps: 5 });
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  await expect(page.getByTestId('place-card')).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Philippi');

  // Fresh navigation: re-aiming at Neapolis after Philippi's own hover/zoom
  // state would conflate "did the FIRST aim actually resolve correctly"
  // with "does the SECOND aim also resolve correctly independent of the
  // first" -- this test wants both proven, not just the pair's own
  // ordering.
  await page.goto('/world?from=49&to=52');
  await page.waitForSelector('[data-testid="marker-neapolis"]', { state: 'attached' });
  await zoomInOnMarker(page, 'marker-neapolis', 3);
  const neapolisBox = await page.getByTestId('marker-neapolis').boundingBox();
  expect(neapolisBox, 'neapolis should render individually at NEAR tier').toBeTruthy();
  await page.mouse.move(neapolisBox!.x + neapolisBox!.width / 2, neapolisBox!.y + neapolisBox!.height / 2, { steps: 5 });
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);
  await expect(page.getByTestId('place-card')).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Neapolis');
});
