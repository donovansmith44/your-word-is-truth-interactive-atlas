import { test, expect } from '@playwright/test';
import { api } from './lib/api';

test('api health and toc', async () => {
  const toc = await api.books();
  expect(toc).toHaveLength(66);
  expect(toc[0].code).toBe('GEN');
  expect(toc[65].code).toBe('REV');
});
test('app shell renders', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-world')).toBeVisible();
});

// Fix round 1 (M1): the Credits popover's Escape-to-close silently did
// nothing until MainLayout focused the popover panel after open (mirroring
// ExplorerPopover's FocusAsync). Using page.keyboard.press (global, whatever
// the page itself currently has focused) rather than
// locator.press('Escape') is load-bearing here -- locator.press() focuses
// its target before pressing, which would pass even without the real fix.
test('credits popover opens and closes on Escape', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('attribution').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByTestId('popover')).toBeHidden();
});
