import { test, expect } from '@playwright/test';

// D2 (owner, 2026-09-15, verbatim: "a clean way to window ANY two things
// side by side: multiple maps, multiple readers ... the cartesian product of
// corpora/views -- with a toggle for whether the two panes are 'following'
// each other ... FOR NOW: map + reader only (generalize the abstraction,
// ship one pair)"). The reader and the map each grow a guest MENU beside
// their one-click split button; reader‖reader shows two chapters,
// world‖world two maps; the follow toggle on a same-view guest mirrors the
// host (chapter / time window). Everything below runs against the live app.

test('SPLIT-PAIRS-1: the guest menu offers exactly the map and the reader, default first, on both hosts', async ({ page }) => {
  for (const [url, expected] of [['/read/GEN/1', ['world', 'reader']], ['/world', ['reader', 'world']]] as const) {
    await page.goto(url);
    await page.getByTestId('enter-split-menu-toggle').click();
    const items = page.getByTestId('enter-split-menu').locator('[data-testid^="enter-split-guest-"]');
    expect(await items.evaluateAll(els => els.map(e => e.getAttribute('data-testid')!.replace('enter-split-guest-', '')))).toEqual([...expected]);
  }
});

test('SPLIT-PAIRS-2: reader‖reader -- two chapters side by side, independent until follow, the guest chapter in the URL', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('enter-split-menu-toggle').click();
  await page.getByTestId('enter-split-guest-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page).toHaveURL(/split=reader/);

  const guest = page.locator('.split-pane-guest');
  const host = page.getByTestId('reader-frame').first();
  await expect(guest.getByTestId('chapter-head')).toContainText('Genesis 1'); // seeded from the host
  await expect(guest.getByTestId('reader-follow-chip')).toHaveAttribute('aria-pressed', 'false'); // same-view pairs start independent

  // The guest navigates on its own: the host stays put, the URL carries the guest chapter.
  await guest.getByTestId('reader-next').click();
  await expect(guest.getByTestId('chapter-head')).toContainText('Genesis 2');
  await expect(host.getByTestId('chapter-head')).toContainText('Genesis 1');
  await expect(page).toHaveURL(/guest=GEN\.2/);

  // Follow: the guest mirrors the host's chapter from now on; the URL drops guest=.
  await guest.getByTestId('reader-follow-chip').click();
  await expect(guest.getByTestId('chapter-head')).toContainText('Genesis 1');
  await expect(page).toHaveURL(/follow=1/);
  await expect(page).not.toHaveURL(/guest=/);
  await host.getByTestId('reader-next').click();
  await expect(host.getByTestId('chapter-head')).toContainText('Genesis 2');
  await expect(guest.getByTestId('chapter-head')).toContainText('Genesis 2');
});

test('SPLIT-PAIRS-3: a reader‖reader URL restores both chapters', async ({ page }) => {
  await page.goto('/read/GEN/1?split=reader&guest=EXO.20');
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.locator('.split-pane-guest').getByTestId('chapter-head')).toContainText('Exodus 20');
  await expect(page.getByTestId('reader-frame').first().getByTestId('chapter-head')).toContainText('Genesis 1');
});

test('SPLIT-PAIRS-4: world‖world -- two maps; the guest chip shares the time window', async ({ page }) => {
  await page.goto('/world');
  await page.getByTestId('enter-split-menu-toggle').click();
  await page.getByTestId('enter-split-guest-world').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page).toHaveURL(/split=world/);
  await expect(page.getByTestId('world-map')).toHaveCount(2);
  const chip = page.locator('.split-pane-guest').getByTestId('follow-chip');
  await expect(chip).toHaveText('Share the time window');
  await expect(chip).toHaveAttribute('aria-pressed', 'false');
  await chip.click();
  await expect(chip).toHaveText('Sharing the time window');
  await expect(page).toHaveURL(/follow=1/);
  // ONE-SELECTOR LAW per pane: a guest map sharing the window offers no scripture picker of its own.
  await expect(page.locator('.split-pane-guest [data-testid="picker"]')).toHaveCount(0);
});

test('SPLIT-PAIRS-5: the one-click buttons keep their meaning -- the reader opens the map, the map opens the reader', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();
  await page.goto('/world');
  await page.getByTestId('split-open-world').click();
  await expect(page).toHaveURL(/\/read\/GEN\/1\?split=world/);
});
