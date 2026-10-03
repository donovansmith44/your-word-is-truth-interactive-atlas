import { test, expect, type Page } from '@playwright/test';
import { api } from './lib/api';
import { neighbourNode } from './lib/edges';

async function openJerusalem(page: Page, from: number, to: number): Promise<void> {
  await page.goto(`/world?from=${from}&to=${to}`);
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.dispatchEvent('click');
}

test('NAME-1: Jerusalem/Jebus swaps on the marker label when the window crosses the curated boundary, while the popover keeps the window-free record name', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:jerusalem');

  // Act
  await openJerusalem(page, -1060, -1050);

  // Assert
  await expect(page.getByTestId('marker-jerusalem').locator('.atlas-label')).toHaveText('Jebus');
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await openJerusalem(page, -590, -580);
  await expect(page.getByTestId('marker-jerusalem').locator('.atlas-label')).toHaveText('Jerusalem');
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
});

test('NAME-1: a window fully inside one curated era never shows the OTHER era\'s name (Hebron/Kirjath-arba in the scene; the record is window-free)', async () => {
  // Arrange
  const nameIn = async (from: number, to: number) => {
    const scene = await api.sceneTime(from, to);
    return [...scene.places, ...scene.quiet_places].find((p: any) => p.id === 'hebron').display_name;
  };

  // Act
  const before = await nameIn(-3000, -2500);
  const after = await nameIn(-2000, -1900);
  const record = await api.node('Place:hebron');

  // Assert
  expect(before).toBe('Kirjath-arba');
  expect(after).toBe('Hebron');
  expect(record.label).toBe('Hebron');
});

test('NOBLURB: a place popover shows no blurb, whatever the window', async ({ page }) => {
  // Arrange
  const record = await api.node('Place:jerusalem');

  // Act
  await openJerusalem(page, -1060, -1050);

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-field-Blurb')).toHaveCount(0);
  await openJerusalem(page, -4004, 100);
  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-field-Blurb')).toHaveCount(0);
});

test('DATE-1: the place\'s established and destroyed dates are fields of its record in the popover', async ({ page }) => {
  // Arrange
  const jerusalem = (await api.node('Place:jerusalem')).place;

  // Act
  await openJerusalem(page, -590, -580);

  // Assert
  await expect(page.getByTestId('popover-field-Established')).toContainText(jerusalem.established.label);
  await expect(page.getByTestId('popover-field-Destroyed')).toContainText(jerusalem.destroyed.label);
});

test('DATE-1: a Map the place is shown on bounds /world to that Map\'s own window', async ({ page }) => {
  // Arrange
  const shownOn = await api.nodeEdges('Place:jerusalem', 'shown-on');
  const map = await api.node(neighbourNode(shownOn.entries[0]).id);
  await openJerusalem(page, -590, -580);

  // Act
  await page.getByTestId(`popover-up-shown-on-${map.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-title')).toHaveText(map.label);
  await page.waitForURL(u => u.pathname === '/world'
    && u.searchParams.get('from') === String(map.map.window.from.value)
    && u.searchParams.get('to') === String(map.map.window.to.value));
});

test('place record fields are absent for a place with no curated history', async ({ page }) => {
  // Arrange
  const w = { from: -1406, to: -1405 };
  const scene = await api.sceneTime(w.from, w.to);
  const records = await Promise.all(scene.places.map((p: any) => api.node(p.node.id)));
  const index = records.findIndex((r: any) => !r.place.established && !r.place.destroyed);
  test.skip(index < 0, 'no uncurated place lit in this window');
  const plain = scene.places[index];
  await page.goto(`/world?from=${w.from}&to=${w.to}`);

  // Act
  await page.getByTestId(`marker-${plain.id}`).dispatchEvent('click');

  // Assert
  await expect(page.getByTestId('popover-card-title')).toHaveText(records[index].label);
  await expect(page.getByTestId('popover-field-Established')).toHaveCount(0);
  await expect(page.getByTestId('popover-field-Destroyed')).toHaveCount(0);
});
