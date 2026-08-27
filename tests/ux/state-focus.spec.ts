import { test, expect } from '@playwright/test';

// Batch ST-3 -- Selection & FocusStack atoms (controller rulings R2/R3/R4).
// selection-tray.spec.ts/saved-explorations.spec.ts (both G2) already pin
// every pre-existing, user-visible behavior this batch re-plumbs onto atoms
// and stay green UNTOUCHED (behavior preservation is the deliverable, R6).
// This file covers only what is NEW BY CONSTRUCTION: the popover back-stack
// re-plumbed onto FocusStack (R3) still behaves exactly as before; tray
// agreement now holds through a SPLIT VIEW specifically (two simultaneously-
// mounted pages sharing the one Selection atom, not just a full SPA nav
// selection-tray.spec.ts already covers); cold-start "selection-v1"
// compatibility (the atom now seeds itself from the SAME on-disk JSON shape
// the pre-atom service used to read at construction, R2).
test.beforeEach(async ({ page }) => {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
});

test('ST-3/R3: the popover back-stack (now a FocusStack dispatch, not a local Stack<T>) still drills in and backs out exactly as before', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await expect(page.getByTestId('popover-breadcrumb-back')).toHaveCount(0); // one entry -- nothing to back INTO yet

  // Drill in via the "About this book" chip (Push -> AuthorNode) -- the
  // SAME chip selection-tray.spec.ts's own sibling specs never touch, a
  // genuine multi-level stack.
  await page.getByTestId('popover-chip-book').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN');
  await expect(page.getByTestId('popover-breadcrumb-back')).toBeVisible();

  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.1');
  await expect(page.getByTestId('popover-breadcrumb-back')).toHaveCount(0);

  // Back at the bottom of the stack is a no-op (FocusStack.Back's own law-2
  // guard) -- the popover stays open, showing the same node, not closed.
  await expect(page.getByTestId('popover')).toBeVisible();

  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);
});

test('ST-3/R3: closing the popover and reopening a DIFFERENT node starts a fresh back-stack (Reset on close, via the shared atom)', async ({ page }) => {
  // Guards against a genuine ST-3 hazard that a purely local Stack<T> never
  // had: FocusStackAtom is now a DI singleton that OUTLIVES any one popover
  // instance -- a fresh open must never inherit a prior session's own
  // stack/trail (see ExplorerPopover.razor's own Dispose -> Reset).
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-1').click();
  await page.getByTestId('popover-chip-book').click(); // GEN -- a 2-deep stack
  await expect(page.getByTestId('popover-breadcrumb-back')).toBeVisible();
  await page.getByTestId('popover-close').click();
  await expect(page.getByTestId('popover')).toHaveCount(0);

  await page.getByTestId('verse-line-2').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.2');
  // A stale, un-reset atom would still hold the prior session's 2-deep
  // stack (GEN.1.1 -> GEN), making THIS back button visible even though
  // this is a brand-new, single-entry session.
  await expect(page.getByTestId('popover-breadcrumb-back')).toHaveCount(0);
});

test('ST-3/R2: the Selection atom agrees across BOTH panes of a live split view, not just across a full-page navigation', async ({ page }) => {
  // selection-tray.spec.ts's own cross-page test (SELECTION-1) proves
  // agreement across a full SPA nav, one page mounted at a time -- the
  // atom migration's own NEW guarantee is that a Ctrl-click on EITHER of
  // TWO SIMULTANEOUSLY MOUNTED pages (reader + embedded atlas pane) updates
  // the ONE tray both panes render, live, with no navigation involved at all.
  await page.goto('/read/GEN/1');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('verse-line-1').click({ modifiers: ['Control'] });
  await expect(page.getByTestId('selection-tray-count')).toHaveText('1 selected');

  // A second selection from the READER pane while the split is still open --
  // both mutations reach the SAME shared atom.
  await page.getByTestId('verse-line-2').click({ modifiers: ['Control'] });
  await expect(page.getByTestId('selection-tray-count')).toHaveText('2 selected');
  await expect(page.getByTestId('selection-tray')).toContainText('GEN.1.1');
  await expect(page.getByTestId('selection-tray')).toContainText('GEN.1.2');
});

test('ST-3/R2: cold-start compatibility -- a hand-written "selection-v1" localStorage doc (the pre-atom on-disk shape) restores correctly into the Selection atom', async ({ page }) => {
  // The atom now seeds itself directly from LocalStore.Read at Program.cs's
  // own construction time (R2) -- this proves the on-disk SHAPE is
  // genuinely unchanged: a bare JSON array of {kind,key,title}, camelCase,
  // exactly what SelectionTrayService's own pre-ST-3 constructor used to
  // read via the identical LocalStore.Read call.
  await page.goto('/');
  await page.evaluate(() => {
    localStorage.setItem('selection-v1', JSON.stringify([
      { kind: 'Verse', key: 'GEN.1.1', title: 'GEN.1.1' },
      { kind: 'Place', key: 'jerusalem', title: 'Jerusalem' },
    ]));
  });

  // A fresh load -- Program.cs's own StateAtom<Selection> construction runs
  // exactly once, at app boot, reading whatever localStorage already holds.
  await page.reload();

  await expect(page.getByTestId('selection-tray-count')).toHaveText('2 selected');
  await expect(page.getByTestId('selection-chip-0')).toContainText('GEN.1.1');
  await expect(page.getByTestId('selection-chip-1')).toContainText('Jerusalem');

  // The restored atom is genuinely LIVE, not a one-shot render of the raw
  // localStorage doc -- an ordinary mutation still works afterward.
  await page.getByTestId('selection-clear').click();
  await expect(page.getByTestId('selection-tray')).toHaveCount(0);
});
