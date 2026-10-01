import { test, expect, type Page } from '@playwright/test';
import { api } from './lib/api';
import { independentlyHoverableIds } from './lib/hoverSafety';

const EXODUS = { from: -1446, to: -1406 };

async function aLabelledPlace(page: Page): Promise<{ id: string; node: { id: string; label: string } }> {
  await page.goto(`/world?from=${EXODUS.from}&to=${EXODUS.to}`);
  const scene = await api.sceneTime(EXODUS.from, EXODUS.to);
  await expect(page.getByTestId(`marker-${scene.places[0].id}`)).toBeAttached();
  const safeIds = await independentlyHoverableIds(page, scene.places.map((p: { id: string }) => p.id));
  const place = scene.places.find((p: { id: string }) => safeIds.has(p.id));
  expect(place, 'expected at least one independently-hoverable lit place').toBeTruthy();
  return place;
}

test('LABEL-1: hovering a LIT place\'s label opens no popover, exactly as hovering its dot does not', async ({ page }) => {
  // Arrange
  const place = await aLabelledPlace(page);
  const label = page.getByTestId(`marker-${place.id}`).locator('.atlas-label');
  await expect(label).toBeVisible();

  // Act
  await label.hover({ force: true });
  await page.waitForTimeout(800);

  // Assert
  await expect(page.getByTestId('popover')).toHaveCount(0);
});

test('LABEL-1: clicking a LIT place\'s label opens its popover exactly like clicking its dot (PIN-1)', async ({ page }) => {
  // Arrange
  const place = await aLabelledPlace(page);
  const label = page.getByTestId(`marker-${place.id}`).locator('.atlas-label');

  // Act
  await label.click({ force: true });
  await page.mouse.move(5, 5);
  await page.waitForTimeout(600);

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(place.node.label);
  await expect(page.getByTestId('popover-close')).toBeVisible();
});

test('LABEL-1: est/dest dates are reachable in one click from a label (regression: "i used to be able to")', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:jerusalem');
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();

  // Act
  await marker.locator('.atlas-label, .quiet-label').click({ force: true });

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-field-Established')).toContainText(record.place.established.label);
  await expect(page.getByTestId('popover-field-Destroyed')).toContainText(record.place.destroyed.label);
});

test('LABEL-1: a real pointer transit from a marker\'s dot into its own label, then a click, opens that same place', async ({ page }) => {
  // Arrange
  const place = await aLabelledPlace(page);
  const marker = page.getByTestId(`marker-${place.id}`);
  const dotBox = (await marker.boundingBox())!;
  const labelBox = (await marker.locator('.atlas-label').boundingBox())!;
  const from = { x: dotBox.x + dotBox.width / 2, y: dotBox.y + dotBox.height / 2 };
  const to = { x: labelBox.x + labelBox.width / 2, y: labelBox.y + labelBox.height / 2 };
  await page.mouse.move(from.x, from.y);

  // Act
  const steps = 8;
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(from.x + (to.x - from.x) * i / steps, from.y + (to.y - from.y) * i / steps);
    await expect(page.getByTestId('popover')).toHaveCount(0);
  }
  await page.mouse.down();
  await page.mouse.up();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(place.node.label);
});

test('LABEL-1: polity and landmark labels stay non-interactive', async ({ page }) => {
  // Arrange
  await page.goto('/world?from=-1446&to=-1400');
  const polityLabel = page.getByTestId(/^polity-label-/).first();
  const landmarkLabel = page.getByTestId(/^landmark-/).first();
  await expect(polityLabel).toBeAttached();
  await expect(landmarkLabel).toBeAttached();

  // Act
  const polityPointerEvents = await polityLabel.evaluate(el => getComputedStyle(el).pointerEvents);
  const landmarkPointerEvents = await landmarkLabel.evaluate(el => getComputedStyle(el).pointerEvents);

  // Assert
  expect(polityPointerEvents).toBe('none');
  expect(landmarkPointerEvents).toBe('none');
});
