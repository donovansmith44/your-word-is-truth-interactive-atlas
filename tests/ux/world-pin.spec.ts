import { test, expect, type Page } from '@playwright/test';
import { openVerse } from './lib/verse';
import { api } from './lib/api';
import { setZoomExact } from './lib/zoom';

const EXODUS_WINDOW = { from: -1446, to: -1406 };

async function clickMarker(page: Page, placeId: string): Promise<void> {
  await page.getByTestId(`marker-${placeId}`).dispatchEvent('click');
}

async function exodusLegs(window: { from: number; to: number }): Promise<{ scene: any; legs: any[] }> {
  const scene = await api.sceneTime(window.from, window.to);
  const legs = scene.arrows.filter((a: any) => a.narrative === 'exodus').sort((a: any, b: any) => a.order - b.order);
  return { scene, legs };
}

function placeOf(scene: any, id: string): any {
  return scene.places.find((p: any) => p.id === id) ?? scene.quiet_places.find((p: any) => p.id === id);
}

test('PIN-1: clicking a marker opens its place\'s record in the popover, ignores hover elsewhere, and closes via popover-close', async ({ page }) => {
  // Arrange
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const { scene, legs } = await exodusLegs(EXODUS_WINDOW);
  test.skip(legs.length === 0, 'exodus narrative not present in this window');
  const start = placeOf(scene, legs[0].from_place);
  const record = await api.node(start.node.id);
  const popover = page.getByTestId('popover');

  // Act
  await clickMarker(page, start.id);

  // Assert
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-card-title')).toHaveText(record.label);
  const otherId: string | null = await page.evaluate((startId) => {
    for (const el of document.querySelectorAll('[data-testid^="marker-"]:not([data-testid^="marker-cluster-"])')) {
      const testid = (el as HTMLElement).dataset.testid!;
      if (testid !== `marker-${startId}` && (el as HTMLElement).offsetParent !== null) {
        return testid;
      }
    }
    return null;
  }, start.id);
  if (otherId) {
    await page.getByTestId(otherId).hover({ force: true });
    await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  }
  await page.mouse.move(5, 5);
  await page.waitForTimeout(1200);
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await page.getByTestId('popover-close').click();
  await expect(popover).toHaveCount(0);
});

test('PIN-2: Escape closes the place popover', async ({ page }) => {
  // Arrange
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const p = scene.places[0];
  await clickMarker(page, p.id);
  await expect(page.getByTestId('popover')).toBeVisible();

  // Act
  await page.keyboard.press('Escape');

  // Assert
  await expect(page.getByTestId('popover')).toHaveCount(0);
});

test('PIN-3: clicking the map background closes the place chooser and leaves the focused place\'s popover open', async ({ page }) => {
  // Arrange
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const p = scene.places[0];
  const record = await api.node(p.node.id);
  await clickMarker(page, p.id);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  const cluster = page.locator('[data-testid^="marker-cluster-"]').first();
  await expect(cluster).toBeAttached();
  await cluster.dispatchEvent('mouseover');
  const chooser = page.getByTestId('place-chooser');
  await expect(chooser).toBeVisible();

  // Act
  await page.getByTestId('world-map').dispatchEvent('click');

  // Assert
  await expect(chooser).toHaveCount(0);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('TRAVERSAL-1: from a place on the exodus route, its site-of event steps along the narrative to the next event, and Back retraces to the place', async ({ page }) => {
  // Arrange
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const { scene, legs } = await exodusLegs(EXODUS_WINDOW);
  test.skip(legs.length < 2, 'need at least 2 exodus legs');
  const start = placeOf(scene, legs[0].from_place);
  const startEvent = await api.node(`Event:${legs[0].from_event}`);
  const nextEvent = await api.node(`Event:${legs[0].to_event}`);
  const startRecord = await api.node(start.node.id);
  await clickMarker(page, start.id);
  await expect(page.getByTestId('popover-title')).toHaveText(startRecord.label);

  // Act
  await page.getByTestId(`popover-link-site-of-${startEvent.id}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(startEvent.label);
  await page.getByTestId('event-chrono-following-event-global').click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(nextEvent.label);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(startEvent.label);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(startRecord.label);
});

test('TRAVERSAL-2: a real hover opens no popover; clicking the same marker opens the place, whose site-of event carries the narrative onward', async ({ page }) => {
  // Arrange
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const { scene, legs } = await exodusLegs(EXODUS_WINDOW);
  test.skip(legs.length === 0, 'exodus narrative not present in this window');
  const start = placeOf(scene, legs[0].from_place);
  const startRecord = await api.node(start.node.id);
  const startEvent = await api.node(`Event:${legs[0].from_event}`);
  const nextEvent = await api.node(`Event:${legs[0].to_event}`);
  await setZoomExact(page, 13, { lat: start.lat, lon: start.lon });
  const box = (await page.getByTestId(`marker-${start.id}`).boundingBox())!;

  // Act
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2, { steps: 10 });

  // Assert
  await page.waitForTimeout(600);
  await expect(page.getByTestId('popover')).toHaveCount(0);
  const chooserRow = page.getByTestId(`place-chooser-${start.id}`);
  if (await chooserRow.isVisible().catch(() => false)) {
    await chooserRow.click();
  } else {
    await page.mouse.down();
    await page.mouse.up();
  }
  await expect(page.getByTestId('popover-title')).toHaveText(startRecord.label);
  await page.getByTestId(`popover-link-site-of-${startEvent.id}`).click();
  await page.getByTestId('event-chrono-following-event-global').click();
  await expect(page.getByTestId('popover-title')).toHaveText(nextEvent.label);
});

const KADESH_WINDOW = { from: -1446, to: -1444 };

test('TRAVERSAL-3: under a window that splits the narrative chain, the place\'s window-free site-of event still reaches the popover\'s FOLLOWING leg', async ({ page }) => {
  // Arrange
  const kadeshPositions = (await api.narrativeEventPositions('ex_kadesh')).narrative;
  const exodusPosition = kadeshPositions.find((p: any) => p.narrative_id === 'exodus');
  test.skip(!exodusPosition?.following, 'ex_kadesh has no following leg in the curated data');
  const followingLabel = exodusPosition.following.label;
  const scene = await api.sceneTime(KADESH_WINDOW.from, KADESH_WINDOW.to);
  const windowedArrow = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_kadesh');
  expect(windowedArrow, 'KADESH_WINDOW must not keep an outgoing ex_kadesh arrow').toBeFalsy();
  const kadeshPlace = scene.places.find((p: any) => p.events.some((e: any) => e.id === 'ex_kadesh'));
  expect(kadeshPlace, 'ex_kadesh\'s own place must still be lit in this window').toBeTruthy();
  const kadesh = await api.node('Event:ex_kadesh');
  await page.goto(`/world?from=${KADESH_WINDOW.from}&to=${KADESH_WINDOW.to}`);

  // Act
  await clickMarker(page, kadeshPlace.id);
  await page.getByTestId(`popover-link-site-of-${kadesh.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(kadesh.label);
  await expect(page.getByTestId('event-story-thread-following-event-exodus')).toHaveText(`next → ${followingLabel}`);
  await page.goto('/read/NUM/13');
  await openVerse(page, 26);
  await expect(page.getByTestId('popover-title')).toHaveText('NUM.13.26');
  await page.getByTestId('popover-link-attests-Event:ex_kadesh').click();
  await expect(page.getByTestId('popover-title')).toHaveText(kadesh.label);
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
  await expect(page.getByTestId('event-story-thread-following-event-exodus')).toHaveText(`next → ${followingLabel}`);
});
