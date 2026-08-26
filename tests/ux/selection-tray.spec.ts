import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { independentlyHoverableIds } from './lib/hoverSafety';

// Batch G2 decision 6 (multi-select v1, RULED per batch-r-report.md §7):
// Ctrl/Cmd-click on an explorable element or a map marker/label toggles that
// node into the Selection Tray -- never opens a popover, never pins/moves a
// marker. SELECTION-1 in CONTRACT.md. Every test here clears localStorage
// once up front (via a real navigation + evaluate, NOT page.addInitScript --
// an init script re-fires on every subsequent navigation/reload within the
// SAME test, which would defeat the persistence assertions these tests
// specifically exercise) so each test starts from a genuinely empty tray.
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
});

test('SELECTION-1: Ctrl-click on a verse line adds it to the tray without opening a popover; a second Ctrl-click removes it; Clear empties the tray', async ({ page }) => {
  await page.goto('/read/GEN/1');
  const line = page.getByTestId('verse-line-1');

  await line.click({ modifiers: ['Control'] });
  await expect(page.getByTestId('popover')).toHaveCount(0);
  const tray = page.getByTestId('selection-tray');
  await expect(tray).toBeVisible();
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');
  await expect(page.getByTestId('selection-chip-0')).toContainText('GEN.1.1');

  // A second Ctrl-click on the SAME line toggles it back off.
  await line.click({ modifiers: ['Control'] });
  await expect(tray).toHaveCount(0);

  // Re-add, then use the tray's own Clear control.
  await line.click({ modifiers: ['Control'] });
  await expect(tray).toBeVisible();
  await page.getByTestId('selection-clear').click();
  await expect(tray).toHaveCount(0);
});

test('SELECTION-1: a chip\'s own remove button toggles that one node off, leaving the rest', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click({ modifiers: ['Control'] });
  await page.getByTestId('verse-line-2').click({ modifiers: ['Control'] });
  await expect(page.getByTestId('selection-tray-count')).toHaveText('2 selected');

  await page.getByTestId('selection-chip-0').getByRole('button').click();
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');
  await expect(page.getByTestId('selection-tray')).toContainText('GEN.1.2');
  await expect(page.getByTestId('selection-tray')).not.toContainText('GEN.1.1');
});

// SELECTION-2 (the gesture split, asserted, not assumed): a PLAIN click on
// the SAME element this test's own sibling above Ctrl-clicks must keep
// opening the popover exactly as before, and must never touch the tray.
test('SELECTION-2: plain click on a verse line still opens its popover, exactly as before Ctrl/Cmd-click existed; the tray stays untouched', async ({ page }) => {
  await page.goto('/read/GEN/1');
  const line = page.getByTestId('verse-line-1');

  await line.click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await expect(page.getByTestId('selection-tray')).toHaveCount(0);

  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);
  await expect(page.getByTestId('selection-tray')).toHaveCount(0);
});

test('SELECTION-1: Ctrl-click on a map marker adds it to the tray without pinning it; a plain click on the same marker still pins (SELECTION-2)', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: rich scene, same pick every other world spec uses
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  const safeIds = await independentlyHoverableIds(page, scene.places.map((p: any) => p.id));
  const p = scene.places.find((pl: any) => safeIds.has(pl.id));
  expect(p, 'expected at least one independently-hoverable lit place').toBeTruthy();

  const marker = page.getByTestId(`marker-${p.id}`);
  // Real hit-testing on a dense scene is a documented risk this suite
  // already works around (world-pin.spec.ts's own clickMarker) --
  // dispatchEvent fires the exact production Leaflet click listener
  // directly on the target element, with a real MouseEvent carrying
  // ctrlKey:true (Playwright's own dispatchEvent(type, eventInit) support),
  // the same technique, extended with the one property map.js's own new
  // isToggleSelectClick actually reads.
  await marker.dispatchEvent('click', { ctrlKey: true });
  await expect(page.getByTestId('place-card')).toHaveCount(0);
  await expect(page.getByTestId('selection-tray')).toBeVisible();
  await expect(page.getByTestId('selection-tray')).toContainText(p.display_name);

  // The SAME marker's own PLAIN click still pins, unchanged (SELECTION-2).
  await marker.dispatchEvent('click');
  const card = page.getByTestId('place-card');
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(p.display_name);
  // The tray's own earlier Ctrl-click selection is untouched by the plain
  // click that followed it -- the two gestures act on independent state.
  await expect(page.getByTestId('selection-tray')).toContainText(p.display_name);
});

test('SELECTION-1: a selection persists across reader<->world navigation (one shared, app-lifetime tray)', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click({ modifiers: ['Control'] });
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');

  // Client-side navigation (a real nav-link click, not page.goto) -- the
  // SAME SPA route change every other cross-page test in this suite drives.
  await page.getByTestId('nav-world').click();
  await expect(page).toHaveURL(/\/world/);
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');
  await expect(page.getByTestId('selection-tray')).toContainText('GEN.1.1');

  await page.getByTestId('nav-reader').click();
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');
});

test('SELECTION-1: a selection survives a fresh page load (localStorage-backed)', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-3').click({ modifiers: ['Control'] });
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');

  await page.reload();
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');
  await expect(page.getByTestId('selection-tray')).toContainText('GEN.1.3');
});
