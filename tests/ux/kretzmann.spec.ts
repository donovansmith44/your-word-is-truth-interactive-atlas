import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch CORP-1 (R2/R5): the Kretzmann Popular Commentary browser --
// locus-keyed. Coverage: tab navigation from both existing pages (Reader,
// World); the current-locus commentary listing + the picker's own SetLocus
// dispatch; explore-on-click (ONE-RULE popover); and the split-follow-by-
// construction proof R2 ordered ("the state layer's first free win").
//
// Batch CORP-1b (owner authorization, resolving CORP-1's own disclosed
// NEEDS_CONTEXT gap): a CommentaryItem's own real prose now rides the
// generic node card's additive `description` field
// (server: `atlas_graph::legacy::node_description`'s widened match) --
// KRETZMANN-4 below asserts the popover body carries REAL, non-fabricated
// prose (fetched from `/api/node/{id}` directly, the same call
// CommentaryItemProseSection itself makes), not just the heading.

test('KRETZMANN-1: nav-kretzmann reaches /kretzmann from both Reader and World', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-kretzmann')).toBeVisible();
  await page.getByTestId('nav-kretzmann').click();
  await expect(page).toHaveURL(/\/kretzmann$/);
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();

  await page.goto('/world');
  await expect(page.getByTestId('nav-kretzmann')).toBeVisible();
  await page.getByTestId('nav-kretzmann').click();
  await expect(page).toHaveURL(/\/kretzmann$/);
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();
});

test('KRETZMANN-2: shows the current-locus chapter (GEN 1, the reader\'s own default) and every commentary row is real, non-fabricated content', async ({ page }) => {
  // Ground truth: the SAME generic edge query the page itself issues (R2's
  // own "expressible as edge queries from the nodes that anchor them"),
  // read directly here for a CONTRACT-lockstep comparison.
  const v1 = await api.nodeEdges('text-unit:GEN.1.1', 'commented-on-by', { limit: 20 });
  expect(v1.entries.length).toBeGreaterThan(0);

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Genesis');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1');

  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();
  for (const entry of v1.entries) {
    const heading: string = entry.node.label;
    await expect(page.getByTestId('kretzmann-verse-group-1')).toContainText(heading);
  }
});

test('KRETZMANN-3: the picker dispatches SetLocus -- no navigation, the SAME chapter it applies is what renders', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();

  await page.getByTestId('picker-book').selectOption('EXO');
  await page.getByTestId('picker-chapter').selectOption('3');
  await page.getByTestId('picker-apply').click();

  // No navigation -- /kretzmann has no per-chapter route of its own (R2).
  await expect(page).toHaveURL(/\/kretzmann$/);
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');
});

test('KRETZMANN-4 (ONE-RULE): plain click on a commentary row opens the existing explore/popover, carrying the unit\'s own heading', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();

  const firstItem = page.locator('.kretzmann-item').first();
  const heading = (await firstItem.textContent())?.trim();
  expect(heading).toBeTruthy();

  await firstItem.click();

  await expect(page.getByTestId('popover-title')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toContainText(heading!);
});

test('KRETZMANN-4b (CORP-1b): the popover body carries the unit\'s own REAL prose, not just its heading', async ({ page }) => {
  // Ground truth: GEN.1.1's own real first Kretzmann unit -- the SAME
  // generic node card the popover's own CommentaryItemProseSection reads.
  const card = await api.node('CommentaryItem:kretzmann/0.1.0');
  expect(card.description).toBeTruthy();
  expect(card.description.length).toBeGreaterThan(20); // a real paragraph, not a stub

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();
  await page.locator('.kretzmann-item').first().click();

  await expect(page.getByTestId('popover-body')).toBeVisible();
  await expect(page.getByTestId('popover-body')).toContainText(card.description);
});

test('KRETZMANN-5: declares its own "read-beside" hatch -- split opens with Kretzmann hosting, Reader as a genuine, live guest, showing the SAME chapter', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-kretzmann')).toBeVisible();

  await page.getByTestId('split-open-kretzmann').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();
  await expect(page.getByTestId('kretzmann-page')).toBeVisible(); // both members live, not replaced
  await expect(page.getByTestId('split-open-kretzmann')).toHaveCount(0); // hatch button hides once already split

  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('Genesis');
  await expect(page.getByTestId('chapter-head')).toContainText('1'); // the reader's own default, matching Kretzmann's own default locus
  await expect(page).toHaveURL(/\/kretzmann$/); // Kretzmann stays the route; no navigation
});

test('KRETZMANN-6 (R2, the free win): navigating the reader in split -- wait, navigating via Kretzmann\'s OWN picker while split is open -- moves BOTH panes, by construction, no link wired', async ({ page }) => {
  // Both members bear the Locus atom (Kretzmann: BearsLocus, this batch;
  // Reader: BearsLocus, ST-1) -- this is the proof neither view links to
  // the other; they simply render the SAME shared state.
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('1');

  // Scoped to kretzmann-page specifically -- once split, the guest reader
  // mounts its OWN ScripturePicker too (the SAME shared component), so a
  // bare getByTestId('picker-book') would be ambiguous (strict-mode
  // violation: two matches).
  const kretzmannPane = page.getByTestId('kretzmann-page');
  await kretzmannPane.getByTestId('picker-book').selectOption('EXO');
  await kretzmannPane.getByTestId('picker-chapter').selectOption('3');
  await kretzmannPane.getByTestId('picker-apply').click();

  // Kretzmann's OWN pane updates (it dispatched SetLocus itself).
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');

  // The GUEST reader pane follows -- it never received a direct call; it
  // simply re-renders off the SAME shared Locus atom Kretzmann just wrote.
  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('chapter-head')).toContainText('3');
  await expect(page).toHaveURL(/\/kretzmann$/); // still Kretzmann's own route -- no navigation happened
});

test('KRETZMANN-7: closing the embedded reader (the guest\'s own close button) returns to a full, single Kretzmann page', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-close-reader-guest')).toBeVisible();

  await page.getByTestId('split-close-reader-guest').click();

  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('reader-root')).toHaveCount(0);
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();
  await expect(page.getByTestId('split-open-kretzmann')).toBeVisible();
});
