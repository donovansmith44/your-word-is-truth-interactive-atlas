import { test, expect } from '@playwright/test';
import { openVerse } from './lib/verse';

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
  await openVerse(page, 1);
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
  await expect(item).toContainText('GEN.1.1 → Genesis'); // the served labels: the verse, then the book container the "About this book" hop resolves to
  await expect(item).toContainText('2 nodes');
  const itemTestId = await item.getAttribute('data-testid');
  const id = itemTestId!.replace('exploration-item-', '');

  // Expand -> two trail rows, the served label of each node and nothing else
  // (FOCUS-1 R15/F1-5: a node's label is served; kind names are not user words).
  await item.locator('.hamburger-exploration-summary').click();
  await expect(page.getByTestId('exploration-node-0')).toHaveText('GEN.1.1');
  await expect(page.getByTestId('exploration-node-1')).toHaveText('Genesis');

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

// PERI-1 (owner: "NUN is not an event"), re-expressed under FOCUS-1 R15/F1-5:
// the trail shows each node's SERVED label and no client-derived kind badge
// at all, so a general-kind pericope can never be labelled "Event" -- the
// category the original fix round patched per node is closed by there being
// no badge to get wrong. The reopen half stands: Continue resolves the saved
// trail from served ids, and the re-saved trail carries the same labels.
test('PERI-1: saving an exploration through a general-kind pericope (NUN) lists it by its served label, never by a kind word', async ({ page }) => {
  await page.goto('/read/PSA/119');
  await openVerse(page, 105);
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

  // Two trail nodes: PSA.119.105, then NUN -- each row is the served label
  // whole; no "Event", no "Passage", no kind word of any sort.
  await expect(page.getByTestId('exploration-node-0')).toHaveText('PSA.119.105');
  const nunNode = page.getByTestId('exploration-node-1');
  await expect(nunNode).toHaveText('Psalm 119: NUN');

  // "Continue" reopens the trail from its served ids (Resolve + Follow per
  // step) and a fresh save of it carries the same served labels.
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
  // Same shape as the original save above -- index 0 is PSA.119.105,
  // index 1 is NUN (the node "Continue" was clicked from).
  await expect(fresh.getByTestId('exploration-node-0')).toHaveText('PSA.119.105');
  await expect(fresh.getByTestId('exploration-node-1')).toHaveText('Psalm 119: NUN');
});

// Decision 1's "consecutive duplicates collapsed" rule is gone with the
// FocusStack (FOCUS-1 spec §3.5, R4, R15): a trail is a start and its hops,
// every Follow is a hop, and a saved trail is reopened WHOLE -- its start
// resolved and every step followed from the served ids -- with Back walking
// each hop in turn. The old v1 store is read once and translated step for
// step (R4), keeping its item id, so the hand-seeded v1 trail below
// (GEN.1.1, GEN.1.1, GEN.1.2) reopens as exactly the three nodes it lists,
// and saving it again writes those same three.
test('EXPLORE-TRAIL-1: a seeded v1 trail is translated step for step and Continue reopens it whole, Back walking every hop', async ({ page }) => {
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
  await expect(page.getByTestId('toast')).toHaveCount(0); // every v1 step mapped; nothing was dropped
  await page.getByTestId('exploration-item-seed1').locator('.hamburger-exploration-summary').click();
  await page.getByTestId('exploration-node-2').click(); // continue from the last (GEN.1.2) node
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.2');

  // The whole trail is open: Back retraces the two hops and then has nothing left to retrace.
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await expect(page.getByTestId('popover-breadcrumb-back')).toHaveCount(0);

  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.getByTestId('hamburger-menu').click();
  const items = page.locator('[data-testid^="exploration-item-"]');
  await expect(items).toHaveCount(2); // the original seed + the new save
  const fresh = page.getByTestId(/^exploration-item-(?!seed1)/);
  await expect(fresh).toContainText('5 nodes'); // the three seeded nodes, then the two Back landings (a Back is a hop too)
  await expect(fresh).toContainText('GEN.1.1 → GEN.1.1');
});

// COLD-1: a fresh page load (no client-side state at all) still sees the
// saved exploration -- proves this rides real localStorage, not merely the
// in-memory singleton's own app-lifetime persistence (SelectionTrayService's
// own doc comment on why THAT persists across navigation; this test proves
// the same for SavedExplorationsService across an actual RELOAD).
test('EXPLORE-TRAIL-1 (COLD-1): a saved exploration survives a fresh page load', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await openVerse(page, 1);
  await page.getByTestId('popover-save-exploration').click();
  await page.getByTestId('popover-close').click();

  await page.reload();
  await page.getByTestId('hamburger-menu').click();
  await expect(page.locator('[data-testid^="exploration-item-"]')).toHaveCount(1);
  await expect(page.getByTestId('hamburger-panel')).toContainText('GEN.1.1');
});
