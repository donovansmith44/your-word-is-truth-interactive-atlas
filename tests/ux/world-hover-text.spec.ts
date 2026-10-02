import { test, expect, type Page } from '@playwright/test';
import { api } from './lib/api';
import { neighbourNode } from './lib/edges';
import { zoomInOnMarker } from './lib/zoom';

const SITE_OF_CLAMP = 20;
const SITE_OF_WINDOW = 40;
const EXODUS = { from: -1446, to: -1406 };
const EXILE = { from: -590, to: -580 };

type Window = { from: number; to: number };
type ScenePlace = { id: string; node: { id: string; label: string } };

function firstShown(to: number): number {
  return Math.max(0, Math.ceil(to / SITE_OF_CLAMP) - SITE_OF_WINDOW / SITE_OF_CLAMP) * SITE_OF_CLAMP + 1;
}

function siteOfCount(record: any): number {
  return record.edge_summary.find((s: { kind: string }) => s.kind === 'site-of')?.count ?? 0;
}

async function placeWithSiteOf(window: Window, fits: (count: number) => boolean): Promise<{ place: ScenePlace; record: any }> {
  const scene = await api.sceneTime(window.from, window.to);
  for (const place of scene.places as ScenePlace[]) {
    const record = await api.node(place.node.id);
    if (fits(siteOfCount(record))) {
      return { place, record };
    }
  }
  throw new Error(`no lit place in ${window.from}..${window.to} has a fitting site-of count`);
}

async function openPlace(page: Page, window: Window, placeId: string): Promise<void> {
  await page.goto(`/world?from=${window.from}&to=${window.to}`);
  await expect(page.getByTestId(`marker-${placeId}`)).toBeAttached();
  await page.getByTestId(`marker-${placeId}`).dispatchEvent('click');
}

function siteOfLinks(page: Page) {
  return page.getByTestId('popover-section-site-of').locator('[data-testid^="popover-link-site-of-"]');
}

async function siteOfIds(page: Page): Promise<string[]> {
  return siteOfLinks(page).evaluateAll(els => els.map(el => el.getAttribute('data-testid')!.replace('popover-link-site-of-', '')));
}

async function expectWithinViewport(page: Page): Promise<void> {
  const viewport = page.viewportSize()!;
  await expect.poll(async () => {
    const box = await page.getByTestId('popover').boundingBox();
    return box !== null && box.x >= 0 && box.y >= 0 && box.x + box.width <= viewport.width && box.y + box.height <= viewport.height;
  }).toBe(true);
}

async function visibleMarkers(page: Page): Promise<{ id: string; y: number }[]> {
  await expect(page.locator('[data-testid^="marker-"]:not([data-testid^="marker-cluster-"]):visible').first()).toBeVisible();
  return page.locator('[data-testid^="marker-"]:not([data-testid^="marker-cluster-"]):visible').evaluateAll(els => els
    .map(el => ({ id: el.getAttribute('data-testid')!.replace(/^marker-/, ''), y: el.getBoundingClientRect().y }))
    .filter(m => m.y > 0 && m.y < window.innerHeight));
}

test('hover place card: a place\'s popover is its window-free record, identical in every window it is clicked in', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:jerusalem');
  const firstPage = await api.nodeEdges(record.id, 'site-of', { limit: SITE_OF_CLAMP });
  const expectedIds = firstPage.entries.map(entry => neighbourNode(entry).id);

  for (const window of [{ from: -1060, to: -1050 }, EXILE]) {
    // Act
    await openPlace(page, window, 'jerusalem');

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(record.label);
    await expect(page.getByTestId('popover-card-title')).toHaveText(record.label);
    await expect(page.getByTestId('popover-section-site-of-heading')).toHaveText(`Site of (${siteOfCount(record)})`);
    await expect.poll(() => siteOfIds(page)).toEqual(expectedIds);
  }
});

test('hover place card: the site-of section lists the place\'s served site-of neighbours in served order', async ({ page }) => {
  // Arrange
  const { place, record } = await placeWithSiteOf(EXODUS, count => count > 1);
  const served = (await api.nodeEdges(record.id, 'site-of', { limit: SITE_OF_CLAMP })).entries.map(neighbourNode);

  // Act
  await openPlace(page, EXODUS, place.id);

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect.poll(() => siteOfIds(page)).toEqual(served.map(node => node.id));
  for (const node of served) {
    await expect(page.getByTestId(`popover-link-site-of-${node.id}`)).toHaveText(node.label);
  }
});

test('hover place card: popover-section-site-of-more pages onward 20 at a time within a 40-row window, stating the rows shown, until the end', async ({ page }) => {
  // Arrange
  const { place, record } = await placeWithSiteOf(EXILE, count => count > SITE_OF_CLAMP);
  const total = siteOfCount(record);
  await openPlace(page, EXILE, place.id);
  const more = page.getByTestId('popover-section-site-of-more');
  const position = page.getByTestId('popover-section-site-of-position');

  // Act
  for (let to = SITE_OF_CLAMP; to < total; to = Math.min(to + SITE_OF_CLAMP, total)) {
    await expect(position).toHaveText(`${firstShown(to)}–${to} of ${total}`);
    await expect(siteOfLinks(page)).toHaveCount(to - firstShown(to) + 1);
    await more.click();
  }

  // Assert
  await expect(position).toHaveText(`${firstShown(total)}–${total} of ${total}`);
  await expect(siteOfLinks(page)).toHaveCount(total - firstShown(total) + 1);
  await expect(more).toHaveCount(0);
  await expect(page.getByTestId('popover-section-site-of-collapse')).toBeVisible();
});

test('hover place card: popover-section-site-of-collapse snaps back to the first site-of neighbours after expanding', async ({ page }) => {
  // Arrange
  const { place, record } = await placeWithSiteOf(EXILE, count => count > SITE_OF_CLAMP);
  await openPlace(page, EXILE, place.id);
  await expect(siteOfLinks(page)).toHaveCount(SITE_OF_CLAMP);
  const initial = await siteOfIds(page);
  await page.getByTestId('popover-section-site-of-more').click();
  await expect(siteOfLinks(page)).toHaveCount(Math.min(2 * SITE_OF_CLAMP, siteOfCount(record)));

  // Act
  await page.getByTestId('popover-section-site-of-collapse').click();

  // Assert
  await expect(siteOfLinks(page)).toHaveCount(SITE_OF_CLAMP);
  expect(await siteOfIds(page)).toEqual(initial);
  await expect(page.getByTestId('popover-section-site-of-collapse')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-site-of-more')).toBeVisible();
});

test('hover place card: a place with no more site-of neighbours than the clamp shows them all and no more/collapse controls', async ({ page }) => {
  // Arrange
  const { place, record } = await placeWithSiteOf(EXODUS, count => count > 0 && count <= SITE_OF_CLAMP);

  // Act
  await openPlace(page, EXODUS, place.id);

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(siteOfLinks(page)).toHaveCount(siteOfCount(record));
  await expect(page.getByTestId('popover-section-site-of-more')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-site-of-collapse')).toHaveCount(0);
});

test('hover robustness: hovering a marker opens no popover; a click opens it, and it stays open after the pointer leaves', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:marah');
  await page.goto(`/world?from=${EXODUS.from}&to=${EXODUS.to}`);
  await expect(page.getByTestId('marker-marah')).toBeAttached();
  await zoomInOnMarker(page, 'marker-marah', 3);
  const box = (await page.getByTestId('marker-marah').boundingBox())!;
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 10 });
  await page.waitForTimeout(800);
  await expect(page.getByTestId('popover')).toHaveCount(0);
  await expect(page.getByTestId('place-chooser')).toHaveCount(0);

  // Act
  await page.mouse.down();
  await page.mouse.up();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await page.mouse.move(5, 5, { steps: 10 });
  await page.waitForTimeout(1200);

  // Assert
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('hover place card: clicking a mentioned-in verse in the place popover opens its verse popover; Back returns to the place', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:rameses');
  const verse = neighbourNode((await api.nodeEdges(record.id, 'mentioned-in', { limit: 1 })).entries[0]);
  await openPlace(page, EXODUS, 'rameses');
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);

  // Act
  await page.getByTestId(`popover-link-mentioned-in-${verse.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(verse.label);
  await expect(page.getByTestId('popover-text')).toHaveText((await api.node(verse.id)).text.text);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('hover place card: following a site-of event from the place popover opens that event; Back returns to the place', async ({ page }) => {
  // Arrange
  const { place, record } = await placeWithSiteOf(EXODUS, count => count > 0);
  const event = neighbourNode((await api.nodeEdges(record.id, 'site-of', { limit: 1 })).entries[0]);
  await openPlace(page, EXODUS, place.id);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);

  // Act
  await page.getByTestId(`popover-link-site-of-${event.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(event.label);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('CARD-FLIP-1: the place popover opened from the marker nearest the viewport top lies fully within the viewport', async ({ page }) => {
  // Arrange
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/world?from=${EXODUS.from}&to=${EXODUS.to}`);
  const top = (await visibleMarkers(page)).sort((a, b) => a.y - b.y)[0];

  // Act
  await page.getByTestId(`marker-${top.id}`).dispatchEvent('click');

  // Assert
  await expect(page.getByTestId('popover-title')).toBeVisible();
  await expectWithinViewport(page);
});

test('CARD-FLIP-1 (paired): the place popover opened from the marker nearest the viewport bottom lies fully within the viewport', async ({ page }) => {
  // Arrange
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/world?from=${EXODUS.from}&to=${EXODUS.to}`);
  const bottom = (await visibleMarkers(page)).sort((a, b) => b.y - a.y)[0];

  // Act
  await page.getByTestId(`marker-${bottom.id}`).dispatchEvent('click');

  // Assert
  await expect(page.getByTestId('popover-title')).toBeVisible();
  await expectWithinViewport(page);
});

test('CARD-FLIP-1 (bottom clamp): at a short viewport an expanded place popover still lies fully within the viewport', async ({ page }) => {
  // Arrange
  await page.setViewportSize({ width: 1280, height: 500 });
  await openPlace(page, EXILE, 'jerusalem');
  await expect(siteOfLinks(page)).toHaveCount(SITE_OF_CLAMP);

  // Act
  await page.getByTestId('popover-section-site-of-more').click();
  await expect(siteOfLinks(page)).toHaveCount(2 * SITE_OF_CLAMP);

  // Assert
  await expectWithinViewport(page);
});
