import { test, expect } from '@playwright/test';
import { api } from './lib/api';

const EXODUS_WINDOW = { from: -1446, to: -1406 };

async function clickMarker(page: import('@playwright/test').Page, placeId: string): Promise<void> {
  await page.getByTestId(`marker-${placeId}`).dispatchEvent('click');
}

function arrowLocator(page: import('@playwright/test').Page, narrativeId?: string) {
  return narrativeId
    ? page.locator(`[data-testid^="arrow-${narrativeId}-"]`)
    : page.locator('[data-testid^="arrow-"]');
}

async function focusStates(locator: ReturnType<typeof arrowLocator>): Promise<(string | null)[]> {
  return locator.evaluateAll(els => els.map(el => el.getAttribute('data-narrative-focus')));
}

test('EVENT-1: map focus classes flip on EventNode open from a place\'s site-of neighbour, follow traversal, and clear on close', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const toRedSea = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_succoth' && a.to_event === 'ex_red_sea');
  test.skip(!toRedSea, 'exodus ex_succoth -> ex_red_sea leg not present in this window');
  const succothId = toRedSea.from_place;

  const exodusArrows = arrowLocator(page, 'exodus');
  await expect(exodusArrows.first()).toBeVisible();
  const otherArrows = page.locator('[data-testid^="arrow-"]:not([data-testid^="arrow-exodus-"])');
  const otherArrowCount = await otherArrows.count();

  for (const state of await focusStates(page.locator('[data-testid^="arrow-"]'))) {
    expect(state).toBeNull();
  }

  await clickMarker(page, succothId);
  await expect(page.getByTestId('popover-section-site-of')).toBeVisible();
  for (const state of await focusStates(page.locator('[data-testid^="arrow-"]'))) {
    expect(state).toBeNull();
  }
  await page.getByTestId('popover-link-site-of-Event:ex_succoth').click();
  await expect(page.getByTestId('popover-title')).toHaveText('First camp at Succoth');

  const currentLegs = page.locator('[data-testid^="arrow-exodus-"][data-narrative-focus="current"]');
  const expectedCurrentCount = scene.arrows.filter((a: any) => a.narrative === 'exodus' && (a.from_event === 'ex_succoth' || a.to_event === 'ex_succoth')).length;
  await expect(currentLegs).toHaveCount(expectedCurrentCount);

  for (const state of await focusStates(exodusArrows)) {
    expect(['active', 'current']).toContain(state);
  }

  if (otherArrowCount > 0) {
    for (const state of await focusStates(otherArrows)) {
      expect(state).toBe('receded');
    }
    await expect(otherArrows.first()).toBeVisible();
    await expect(otherArrows.first()).not.toHaveAttribute('data-faded', 'true');
  }

  await page.getByTestId('event-chrono-following-event-global').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Crossing the Red Sea');

  const ramesesToSuccoth = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_rameses' && a.to_event === 'ex_succoth');
  if (ramesesToSuccoth) {
    await expect(page.getByTestId(`arrow-exodus-${ramesesToSuccoth.order}`)).toHaveAttribute('data-narrative-focus', 'active');
  }
  const newCurrentCount = scene.arrows.filter((a: any) => a.narrative === 'exodus' && (a.from_event === 'ex_red_sea' || a.to_event === 'ex_red_sea')).length;
  await expect(currentLegs).toHaveCount(newCurrentCount);

  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);
  for (const state of await focusStates(page.locator('[data-testid^="arrow-"]'))) {
    expect(state).toBeNull();
  }
});

test('EVENT-1: data-narrative-focus state changes carry no CSS transition (reduced-motion is a no-op change, by design)', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const arrow = page.locator('[data-testid^="arrow-exodus-"]').first();
  await expect(arrow).toBeVisible();
  const transitionDuration = await arrow.evaluate(el => getComputedStyle(el).transitionDuration);
  expect(transitionDuration).toBe('0s');
});
