import { test, expect } from '@playwright/test';

// Batch AQC-1 (design spec §2's versioning law, the house fail-loud law):
// the client's own startup check (App.razor -> AtlasClient.Contract() ->
// AqcContract.Satisfies) -- happy path (the real, matching /api/contract)
// + a mocked-mismatch loud path, per the brief's own requirement.
//
// Fix round 1 (Q-5/§0, controller ruling): a third case -- an UNREACHABLE
// /api/contract must not hang first paint (App.razor's own 2s
// CancellationToken timeout) and must NOT be treated as a mismatch (the
// disclosed scope decision: a network failure is not a deployment-skew
// signal). Playwright-only -- neither Gherkin harness can honestly
// express "the request never resolves" (Rust would just hang its own live
// call; the C# side has no live call to abort at all).

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

test('unreachable /api/contract: the app loads normally rather than failing loud', async ({ page }) => {
  await page.route(
    url => url.pathname === '/api/contract',
    route => route.abort());

  await page.goto('/');
  await expect(page.getByTestId('nav-world')).toBeVisible();
  await expect(page.getByTestId('contract-mismatch')).toHaveCount(0);
});

test('hanging /api/contract: the 2s startup-check timeout fires -- the app loads normally, not blank', async ({ page }) => {
  // No route.fulfill/continue/abort call -- the request never resolves,
  // proving App.razor's own CancellationToken timeout is real (Q-5), not
  // merely present in code.
  await page.route(url => url.pathname === '/api/contract', () => {});

  await page.goto('/');
  // Bounded well past the 2s timeout, well short of HttpClient's own
  // 100s default -- proves the SHORT timeout fired, not the long one.
  await expect(page.getByTestId('nav-world')).toBeVisible({ timeout: 10_000 });
  await expect(page.getByTestId('contract-mismatch')).toHaveCount(0);
});
