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
