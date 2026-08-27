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
  // Batch CORPREAD-1a (SPLIT-PERSIST-1): Kretzmann now keeps its OWN split
  // query in sync too (the route itself never changes, but the query now
  // does -- CompositionSplit's own SyncSplitUrl is generic, not
  // Reader-only; see SplitUrlContract.cs's own header).
  await expect(page).toHaveURL(/\/kretzmann\?split=reader$/); // Kretzmann stays the route; no navigation
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
  // Batch CORPREAD-1a (SPLIT-PERSIST-1): ?split=reader rides along now too.
  await expect(page).toHaveURL(/\/kretzmann\?split=reader$/); // still Kretzmann's own route -- no navigation happened
});

// READER-GUEST-1 (batch-corp1-review.md S-2/Q-4, batch-finalp1-brief.md
// ticket 1): KRETZMANN-6's own MISSING reverse-direction sibling. R2's own
// text asked for a test proving "navigate reader in split ⇒ commentary
// follows" -- KRETZMANN-6 above proves the OPPOSITE direction (Kretzmann's
// own picker writes, guest Reader follows). This test drives the READER
// pane's own navigation controls (guest role) and asserts Kretzmann (host)
// follows -- before this fix, Reader's guest-mode picker/prev-next
// unconditionally Nav.NavigateTo'd, which would have navigated the WHOLE
// APP away from /kretzmann to plain /read/..., destroying the split
// (S-2's own disclosed, then-unverified prediction).
test('KRETZMANN-6b (R2, the reverse direction -- READER-GUEST-1): navigating the GUEST reader pane\'s own picker moves BOTH panes, split stays intact', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();

  // Scoped to reader-root specifically -- the kretzmann pane mounts its OWN
  // ScripturePicker too (KRETZMANN-6's own comment on the identical
  // ambiguity, mirrored here for the opposite pane).
  const readerPane = page.getByTestId('reader-root');
  await readerPane.getByTestId('picker-book').selectOption('EXO');
  await readerPane.getByTestId('picker-chapter').selectOption('3');
  await readerPane.getByTestId('picker-apply').click();

  // The GUEST reader pane itself updates (it dispatched SetLocus).
  await expect(page.getByTestId('chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('chapter-head')).toContainText('3');

  // Kretzmann (host) follows -- it never received a direct call; it simply
  // re-renders/refetches off the SAME shared Locus atom the guest just wrote.
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');

  // Split intact throughout -- still Kretzmann's own route, no navigation
  // (Batch CORPREAD-1a: ?split=reader rides along too).
  await expect(page).toHaveURL(/\/kretzmann\?split=reader$/);
  await expect(page.getByTestId('split-view')).toBeVisible();
});

test('KRETZMANN-6c (READER-GUEST-1): reader-next from the GUEST pane also moves Kretzmann, split stays intact', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('1');

  await page.getByTestId('reader-next').click();

  // Batch CORPREAD-1a: ?split=reader rides along too.
  await expect(page).toHaveURL(/\/kretzmann\?split=reader$/);
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('2');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('2');
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

// KRETZ-SCALE-1 (batch-corp1-review.md Q-1, batch-corp1-report.md §5,
// batch-finalp1-brief.md ticket 2): PSA 119 is the exact pileup this
// ticket names -- 176 verses, the chapter whose OLD per-verse fan-out
// meant 176 concurrent `commented-on-by` edges requests on a single locus
// change. Asserts BOTH halves: the page still loads sanely (real content,
// cross-checked against the new endpoint's own ground truth), AND the
// request count via Playwright's own request-tracking is exactly what the
// new chapter-scoped endpoint promises -- ONE request, not 176, and zero
// requests to the retired per-verse edges pattern.
test('KRETZMANN-8 (KRETZ-SCALE-1): PSA 119 loads via ONE chapter-scoped request, not a 176-request per-verse fan-out', async ({ page }) => {
  const ground = await api.kretzmannChapter('PSA.119');
  expect(ground.verses.length).toBeGreaterThan(0);

  const kretzmannChapterRequests: string[] = [];
  const perVerseEdgesRequests: string[] = [];
  page.on('request', req => {
    const url = new URL(req.url());
    if (url.pathname === '/api/kretzmann/chapter/PSA.119') {
      kretzmannChapterRequests.push(req.url());
    } else if (/^\/api\/node\/text-unit%3APSA\.119\.\d+\/edges$/.test(url.pathname) && url.searchParams.get('kind') === 'commented-on-by') {
      perVerseEdgesRequests.push(req.url());
    }
  });

  await page.goto('/kretzmann');
  await page.getByTestId('picker-book').selectOption('PSA');
  await page.getByTestId('picker-chapter').selectOption('119');
  await page.getByTestId('picker-apply').click();

  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Psalms');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('119');

  // Sane load: every ground-truth verse group renders, carrying real
  // (non-fabricated) headings -- not just "the page didn't crash."
  for (const v of ground.verses) {
    const group = page.getByTestId(`kretzmann-verse-group-${v.verse}`);
    await expect(group).toBeVisible();
    for (const item of v.items) {
      await expect(group).toContainText(item.heading ?? 'Commentary');
    }
  }

  // The scale proof itself: exactly one chapter-scoped request, zero of
  // the retired per-verse ones -- not "fewer," exactly the shape this
  // ticket promises.
  expect(kretzmannChapterRequests.length).toBe(1);
  expect(perVerseEdgesRequests.length).toBe(0);
});
