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

// Batch S ("document our sources for everything and give it a dedicated
// page on the site"): the Credits control used to open a popover (Fix
// round 1, M1's own Escape-to-close test, retired here) -- it is a plain
// nav link to /sources now, the app's one decisive home for attribution
// (requirement 1). See sources.spec.ts for the Sources page's own
// content/contrast/CONTRACT tests.
test('credits link navigates to the Sources page', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('attribution').click();
  await expect(page).toHaveURL(/\/sources$/);
  await expect(page.getByTestId('sources-page')).toBeVisible();
});
