import { test, expect } from '@playwright/test';

// Batch G2 decisions 1/2/3/4/5 (saved explorations + the hamburger menu),
// EXPLORE-TRAIL-1 in CONTRACT.md. Every test clears localStorage once up
// front via a real navigation + evaluate (NOT page.addInitScript, which
// re-fires on every subsequent navigation/reload within the SAME test --
// that would defeat COLD-1's own reload-survival assertion) so each test
// starts from a genuinely empty saved-explorations list.
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
});

// The one full round-trip: save -> hamburger lists it -> continue reopens
// the clicked node with a working back-stack through the SAVED journey ->
// rename -> delete.
test('EXPLORE-TRAIL-1: save from the popover, list in the hamburger, continue with a working back-stack, rename, delete', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');

  // Grow the trail one hop (decision 1: every Push is recorded) --
  // "About this book" pushes an AuthorNode.
  await page.getByTestId('popover-chip-book').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN');

  // Decision 2: save does NOT close the popover.
  await page.getByTestId('popover-save-exploration').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN');

  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);

  // Decision 5: the hamburger lists the save -- auto-name is
  // "first title -> last title".
  await page.getByTestId('hamburger-menu').click();
  const panel = page.getByTestId('hamburger-panel');
  await expect(panel).toBeVisible();
  const item = page.locator('[data-testid^="exploration-item-"]');
  await expect(item).toHaveCount(1);
  await expect(item).toContainText('GEN.1.1 → GEN');
  await expect(item).toContainText('2 nodes');
  const itemTestId = await item.getAttribute('data-testid');
  const id = itemTestId!.replace('exploration-item-', '');

  // Expand -> two trail rows, kind + title each.
  await item.locator('.hamburger-exploration-summary').click();
  await expect(page.getByTestId('exploration-node-0')).toContainText('Verse');
  await expect(page.getByTestId('exploration-node-0')).toContainText('GEN.1.1');
  await expect(page.getByTestId('exploration-node-1')).toContainText('Author');
  await expect(page.getByTestId('exploration-node-1')).toContainText('GEN');

  // "Continue" from the LAST node -- reopens Author(GEN) as Current, with
  // GEN.1.1 as the back-stack's own preceding node.
  await page.getByTestId('exploration-node-1').click();
  await expect(panel).toHaveCount(0); // continuing closes the hamburger panel
  await expect(page.getByTestId('popover-title')).toHaveText('GEN');
  await expect(page.getByTestId('popover-breadcrumb-back')).toBeVisible();
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await page.getByTestId('popover-close').click();

  // Edit: inline rename.
  await page.getByTestId('hamburger-menu').click();
  await page.getByTestId(`exploration-rename-${id}`).click();
  const renameInput = page.getByTestId(`exploration-rename-input-${id}`);
  await expect(renameInput).toBeVisible();
  await renameInput.fill('My Genesis exploration');
  await renameInput.press('Enter');
  await expect(page.getByTestId(`exploration-item-${id}`)).toContainText('My Genesis exploration');

  // Edit: delete, with an INLINE confirm (never a browser dialog).
  await page.getByTestId(`exploration-delete-${id}`).click();
  await expect(page.getByTestId(`exploration-delete-${id}-confirm`)).toBeVisible();
  await page.getByTestId(`exploration-delete-${id}-confirm`).click();
  await expect(page.getByTestId(`exploration-item-${id}`)).toHaveCount(0);
  await expect(page.getByTestId('hamburger-empty')).toBeVisible();
});

// PERI-1 fix round 1 (review S-1a/Q-1a, CRITICAL): the review proved
// ExplorationDescriptor.Capture runs synchronously BEFORE the fetch that
// used to be the ONLY source EventNode.CachedKind read, so a freshly
// clicked general-kind EventNode's own trail badge was captured as "Event"
// deterministically, not as a rare race -- the owner's own PSA.119.105/
// GAL.1.8 repro shape, saved. Fixed by threading the already-on-the-wire
// VerseEventDto.Kind into EventNode's own constructor at the exact click
// site (PopoverSectionProviders.cs's RenderRows) -- this test proves the
// saved-trail badge itself, live, the surface the review found broken.
test('PERI-1 fix round 1: saving an exploration through a general-kind pericope (NUN) labels its own trail badge "Passage," never "Event"', async ({ page }) => {
  await page.goto('/read/PSA/119');
  await page.getByTestId('verse-line-105').click();
  await expect(page.getByTestId('popover-title')).toHaveText('PSA.119.105');

  // Drill into the general-kind PASSAGE row itself -- a fresh EventNode,
  // never previously fetched, the exact shape the review's own trace
  // named as the deterministic failure case.
  await page.getByTestId('verse-event-psa_119_nun').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Psalm 119: NUN');

  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.getByTestId('hamburger-menu').click();
  const item = page.locator('[data-testid^="exploration-item-"]');
  await expect(item).toHaveCount(1);
  await item.locator('.hamburger-exploration-summary').click();

  // Two trail nodes: PSA.119.105 (Verse), then NUN (Event-kind CLIENT
  // node, but a general-kind PASSAGE by data) -- its own badge must read
  // "Passage," never "Event."
  await expect(page.getByTestId('exploration-node-0')).toContainText('Verse');
  const nunNode = page.getByTestId('exploration-node-1');
  await expect(nunNode).toContainText('Psalm 119: NUN');
  await expect(nunNode.locator('.hamburger-node-kind')).toHaveText('Passage');
  await expect(nunNode.locator('.hamburger-node-kind')).not.toHaveText('Event');

  // "Continue" reopens the trail (SeedFromTrail -> Visit -> a FRESH
  // Capture per node, FocusStack.cs) -- proves the fix survives a
  // re-capture, not just the original save (S-1a's own second-order
  // concern: ExplorationDescriptor.Reconstruct must seed knownKind from
  // the saved descriptor's own IsGeneralKind, or a reopen would silently
  // regress the badge back to "Event").
  await nunNode.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Psalm 119: NUN');
  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.getByTestId('hamburger-menu').click();
  const items = page.locator('[data-testid^="exploration-item-"]');
  await expect(items).toHaveCount(2);
  // SavedExplorationsService.Save always APPENDS (never mutates a prior
  // save) -- the LAST item in the list is this second, freshly re-saved one.
  const fresh = items.last();
  await fresh.locator('.hamburger-exploration-summary').click();
  // Same shape as the original save above -- index 0 is the Verse
  // (PSA.119.105), index 1 is NUN (the node "Continue" was clicked from).
  await expect(fresh.getByTestId('exploration-node-0')).toContainText('Verse');
  const reopenedNunNode = fresh.getByTestId('exploration-node-1');
  await expect(reopenedNunNode).toContainText('Psalm 119: NUN');
  await expect(reopenedNunNode.locator('.hamburger-node-kind')).toHaveText('Passage');
});

// Decision 1's own "consecutive duplicates collapsed" rule. This app's real
// navigation graph never revisits the SAME node twice in a row (every Push
// target differs from Current; every Back lands on whatever the stack
// already held below, which by construction differs from what was just
// popped) -- there is no organically-reachable click sequence that produces
// a genuine consecutive duplicate today. RecordTrailVisit's own collapse
// guard is exercised here instead via a directly-controlled repro: a
// hand-seeded saved exploration whose OWN node list carries an adjacent
// duplicate (GEN.1.1, GEN.1.1, GEN.1.2) -- decision 5's "continue" replays
// every one of those descriptors through the SAME RecordTrailVisit call a
// live Push/Back would use (ExplorerPopover.SeedStack's own doc comment),
// so this exercises the real collapse code, not a mock of it. Saving again
// immediately afterward snapshots the POPOVER's own (now-collapsed) trail,
// proving the duplicate never landed twice in a row.
test('EXPLORE-TRAIL-1: consecutive-duplicate collapse -- a seeded trail with GEN.1.1 twice in a row saves with only one', async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => {
    localStorage.setItem('explorations-v1', JSON.stringify([{
      id: 'seed1',
      name: 'Seed',
      createdUtc: '2026-01-01T00:00:00Z',
      nodes: [
        { kind: 'Verse', key: 'GEN.1.1', title: 'GEN.1.1' },
        { kind: 'Verse', key: 'GEN.1.1', title: 'GEN.1.1' },
        { kind: 'Verse', key: 'GEN.1.2', title: 'GEN.1.2' },
      ],
    }]));
  });
  await page.reload();

  await page.getByTestId('hamburger-menu').click();
  await page.getByTestId('exploration-item-seed1').locator('.hamburger-exploration-summary').click();
  await page.getByTestId('exploration-node-2').click(); // continue from the last (GEN.1.2) node
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.2');

  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.getByTestId('hamburger-menu').click();
  const items = page.locator('[data-testid^="exploration-item-"]');
  await expect(items).toHaveCount(2); // the original seed + the new save
  const fresh = page.getByTestId(/^exploration-item-(?!seed1)/);
  await expect(fresh).toContainText('2 nodes'); // collapsed from 3 raw entries to 2
});

// COLD-1: a fresh page load (no client-side state at all) still sees the
// saved exploration -- proves this rides real localStorage, not merely the
// in-memory singleton's own app-lifetime persistence (SelectionTrayService's
// own doc comment on why THAT persists across navigation; this test proves
// the same for SavedExplorationsService across an actual RELOAD).
test('EXPLORE-TRAIL-1 (COLD-1): a saved exploration survives a fresh page load', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.reload();
  await page.getByTestId('hamburger-menu').click();
  await expect(page.locator('[data-testid^="exploration-item-"]')).toHaveCount(1);
  await expect(page.getByTestId('hamburger-panel')).toContainText('GEN.1.1');
});
