import { test, expect } from '@playwright/test';

// Batch AQC-1 (design spec §2's versioning law, the house fail-loud law):
// the client's own startup check (App.razor -> AtlasClient.Contract() ->
// AqcContract.Satisfies) -- happy path (the real, matching /api/contract)
// + a mocked-mismatch loud path, per the brief's own requirement.

test('happy path: the real /api/contract matches this client build, the app loads normally', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-world')).toBeVisible();
  await expect(page.getByTestId('contract-mismatch')).toHaveCount(0);
});

test('mocked mismatch: an advertised range excluding this client build fails loud', async ({ page }) => {
  await page.route(
    url => url.pathname === '/api/contract',
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify({ min_version: '0.2.0', max_version: '0.5.0' }),
    }));

  await page.goto('/');
  await expect(page.getByTestId('contract-mismatch')).toBeVisible();
  await expect(page.getByTestId('contract-mismatch-advertised')).toHaveText('0.2.0-0.5.0');
  await expect(page.getByTestId('contract-mismatch-client')).toHaveText('0.1.0');
  // The visible, honest error state REPLACES the app shell -- never a
  // silent degrade alongside it.
  await expect(page.getByTestId('nav-world')).toHaveCount(0);
});
