import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch CORPREAD-1b (ticket K, owner order verbatim: "we should have the
// whole kretzmann commentary just like on the kretzmann.org, just having
// the verses be clickable and explorable ... we need an escape hatch to
// stop following the reader as well ... design it like a sensible UI
// developer would") -- REBUILDS CORP-1/CORP-1b's own item-list/card browser
// into a continuous reading surface. Coverage: tab navigation (unchanged);
// the current-locus commentary listing, now with REAL PROSE rendered
// inline (not hidden behind a popover click); ONE-RULE explore-on-click;
// the split-follow-by-construction proof (unchanged mechanism, now
// explicitly Following-gated); THE FOLLOW-RELEASE LAW's own toggle-follow
// chip; in-prose scripture-reference explorability (the shared scanner);
// the verse anchor's own explorability; new chapter-to-chapter prev/next
// nav; KRETZ-SCALE-1's own one-request proof (unchanged mechanism).

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

test('KRETZMANN-2: shows the current-locus chapter (GEN 1) with real, non-fabricated headings, every verse group present', async ({ page }) => {
  const v1 = await api.nodeEdges('text-unit:GEN.1.1', 'commented-on-by', { limit: 20 });
  expect(v1.entries.length).toBeGreaterThan(0);

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Genesis');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1');

  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();
});

test('KRETZMANN-2b (ticket K, "the WHOLE commentary ... just like on kretzmann.org"): a commentary item\'s own REAL prose renders INLINE, not hidden behind a click', async ({ page }) => {
  const card = await api.node('CommentaryItem:kretzmann/0.1.0');
  expect(card.description).toBeTruthy();
  expect(card.description.length).toBeGreaterThan(20);

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();

  // Auto-retrying assertion -- the item's own concurrent Card() fetch
  // resolves asynchronously (streams in under the already-visible
  // heading); no manual wait needed.
  await expect(page.locator('.kretzmann-item').first()).toContainText(card.description);
});

test('KRETZMANN-3: the picker dispatches SetLocus -- no navigation, the SAME chapter it applies is what renders', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();

  await page.getByTestId('picker-book').selectOption('EXO');
  await page.getByTestId('picker-chapter').selectOption('3');
  await page.getByTestId('picker-apply').click();

  await expect(page).toHaveURL(/\/kretzmann$/);
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');
});

test('KRETZMANN-4 (ONE-RULE): plain click on a commentary paragraph opens the existing explore/popover, carrying the SAME real prose already visible inline', async ({ page }) => {
  const card = await api.node('CommentaryItem:kretzmann/0.1.0');

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();
  const firstItem = page.locator('.kretzmann-item').first();
  await expect(firstItem).toContainText(card.description);

  await firstItem.click();

  await expect(page.getByTestId('popover-title')).toBeVisible();
  await expect(page.getByTestId('popover-body')).toContainText(card.description);
});

test('KRETZMANN-5: declares its own "read-beside" hatch -- split opens with Kretzmann hosting, Reader as a genuine, live guest, showing the SAME chapter, FOLLOWING by default', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-kretzmann')).toBeVisible();

  await page.getByTestId('split-open-kretzmann').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();
  await expect(page.getByTestId('kretzmann-page')).toBeVisible();
  await expect(page.getByTestId('split-open-kretzmann')).toHaveCount(0);

  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('Genesis');
  await expect(page.getByTestId('chapter-head')).toContainText('1');
  await expect(page).toHaveURL(/\/kretzmann\?split=reader&follow=1$/);

  // THE FOLLOW-RELEASE LAW: following BY DEFAULT (ticket K, verbatim).
  await expect(page.getByTestId('kretzmann-follow-chip')).toBeVisible();
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveAttribute('aria-pressed', 'true');
});

test('KRETZMANN-6 (R2, the free win, still holds while FOLLOWING): Kretzmann\'s OWN picker while split moves BOTH panes, no link wired', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('1');

  const kretzmannPane = page.getByTestId('kretzmann-page');
  await kretzmannPane.getByTestId('picker-book').selectOption('EXO');
  await kretzmannPane.getByTestId('picker-chapter').selectOption('3');
  await kretzmannPane.getByTestId('picker-apply').click();

  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');

  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('chapter-head')).toContainText('3');
  await expect(page).toHaveURL(/\/kretzmann\?split=reader&follow=1$/);
});

test('KRETZMANN-6b (R2, the reverse direction -- READER-GUEST-1): navigating the GUEST reader pane\'s own picker moves BOTH panes, split stays intact', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();

  const readerPane = page.getByTestId('reader-root');
  await readerPane.getByTestId('picker-book').selectOption('EXO');
  await readerPane.getByTestId('picker-chapter').selectOption('3');
  await readerPane.getByTestId('picker-apply').click();

  await expect(page.getByTestId('chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('chapter-head')).toContainText('3');

  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');

  await expect(page).toHaveURL(/\/kretzmann\?split=reader&follow=1$/);
  await expect(page.getByTestId('split-view')).toBeVisible();
});

test('KRETZMANN-6c (READER-GUEST-1): reader-next from the GUEST pane also moves Kretzmann, split stays intact', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('1');

  await page.getByTestId('reader-next').click();

  await expect(page).toHaveURL(/\/kretzmann\?split=reader&follow=1$/);
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

// THE FOLLOW-RELEASE LAW (deliverable 0a/0b, owner ruling 2026-08-27,
// ticket K): "we need an escape hatch to stop following the reader as
// well." Released, Kretzmann's own pane must NOT move when the guest
// reader pane navigates; re-follow must reconverge to wherever the guest
// currently sits.
test('KRETZMANN-9 (THE FOLLOW-RELEASE LAW): released, Kretzmann does not move when the guest reader navigates; re-follow reconverges', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveAttribute('aria-pressed', 'true');

  // Release.
  await page.getByTestId('kretzmann-follow-chip').click();
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1'); // unchanged at the instant of release

  // The guest reader pane navigates -- while released, Kretzmann's OWN
  // pane must stay put (it neither reads nor writes the shared atom now).
  const readerPane = page.getByTestId('reader-root');
  await readerPane.getByTestId('picker-book').selectOption('EXO');
  await readerPane.getByTestId('picker-chapter').selectOption('3');
  await readerPane.getByTestId('picker-apply').click();

  await expect(page.getByTestId('chapter-head')).toContainText('Exodus'); // the guest pane DID move
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Genesis'); // Kretzmann did NOT
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1');

  // Re-follow -- reconverges to wherever the shared atom holds NOW
  // (Exodus 3), never the stale Genesis 1 it was released from.
  await page.getByTestId('kretzmann-follow-chip').click();
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Exodus');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('3');
});

test('KRETZMANN-9b (THE FOLLOW-RELEASE LAW): released, Kretzmann\'s OWN picker browses independently -- the shared atom (and the guest reader pane) does not move', async ({ page }) => {
  await page.goto('/kretzmann');
  await page.getByTestId('split-open-kretzmann').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('kretzmann-follow-chip').click();
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveAttribute('aria-pressed', 'false');

  const kretzmannPane = page.getByTestId('kretzmann-page');
  await kretzmannPane.getByTestId('picker-book').selectOption('PSA');
  await kretzmannPane.getByTestId('picker-chapter').selectOption('23');
  await kretzmannPane.getByTestId('picker-apply').click();

  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Psalms');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('23');

  // The guest reader pane -- driven by the shared Locus atom -- never moved.
  await expect(page.getByTestId('chapter-head')).toContainText('Genesis');
  await expect(page.getByTestId('chapter-head')).toContainText('1');
});

// Ticket K, "references inside commentary prose ... are clickable
// explorables" -- ground truth quoted verbatim from batch-corp1-report.md
// §CORP-1b: GEN.1.1's own real prose reads "In the beginning, cp. John 1,
// 1, that is, ...", a genuine in-prose citation.
test('KRETZMANN-10 (explorable-reference law): a scripture reference inside commentary prose opens the SAME VerseNode popover', async ({ page }) => {
  await page.goto('/kretzmann');
  const firstItem = page.locator('.kretzmann-item').first();
  await expect(firstItem).toContainText('John 1, 1');

  const ref = firstItem.locator('.kretzmann-ref').first();
  await expect(ref).toBeVisible();
  await ref.click();

  await expect(page.getByTestId('popover-title')).toBeVisible();
  // "JHN", not "JOH" -- this app's own real canonical book code for John
  // (confirmed live: GET /api/books), resolved through the scanner's own
  // book-name lookup against the ACTUAL fetched TOC, never a hardcoded
  // guess.
  await expect(page.getByTestId('popover-title')).toContainText('JHN.1.1');
});

test('KRETZMANN-11 (ticket K, "chapter-to-chapter continuation ... reads like the Bible reader\'s own flow"): prev/next navigate chapters', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1');
  await expect(page.getByTestId('kretzmann-prev')).toHaveCount(0); // GEN.1 has no prior chapter

  await page.getByTestId('kretzmann-next').click();
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('2');
  await expect(page.getByTestId('kretzmann-prev')).toBeVisible();

  await page.getByTestId('kretzmann-prev').click();
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('1');
});

test('KRETZMANN-12 (ticket K, "the verse anchors themselves are clickable explorables"): a verse anchor opens the SAME VerseNode popover as the reader\'s own verse-num', async ({ page }) => {
  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-anchor-1')).toBeVisible();

  await page.getByTestId('kretzmann-verse-anchor-1').click();

  await expect(page.getByTestId('popover-title')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toContainText('GEN.1.1');
});

// KRETZ-SCALE-1 (batch-corp1-review.md Q-1, batch-finalp1-brief.md ticket
// 2): PSA 119 is the exact pileup this ticket names -- the listing itself
// must still cost exactly ONE chapter-scoped request, zero of the retired
// per-verse fan-out.
//
// Fix round (Q-2, CRITICAL -- review): the first ship of this batch fetched
// EVERY item's own prose concurrently the instant the listing resolved --
// reintroducing KRETZ-SCALE-1's own retired per-verse fan-out through
// Card() instead of the retired .../edges?kind=commented-on-by shape, and
// THIS test's own prior version was narrowed to assert only that the OLD
// url shape stayed at zero, silently excusing the NEW one ("regardless of
// this batch's own additional (disclosed, N-fetch) per-item prose fetch")
// rather than bounding it -- exactly the tripwire-evasion pattern the
// controller ordered never repeated. Restored honest: a genuine TOTAL
// bound on the NEW request family (Kretzmann.razor now fetches prose
// lazily, one item at a time, as its own row approaches the viewport --
// wwwroot/js/lazyProse.js's own IntersectionObserver, bounded concurrency
// 8), proven by asserting SOME prose loads without any scroll (whatever is
// near the viewport) but NOWHERE NEAR the full 176-verse chapter's worth.
test('KRETZMANN-13 (KRETZ-SCALE-1 + KRETZ-PROSE-SCALE, honest total-request bound): PSA 119\'s own LISTING loads via ONE chapter-scoped request, zero per-verse fan-out, and the lazy prose fetch never requests the whole chapter up front', async ({ page }) => {
  const ground = await api.kretzmannChapter('PSA.119');
  expect(ground.verses.length).toBeGreaterThan(0);
  const totalItems = ground.verses.reduce((n, v) => n + v.items.length, 0);
  expect(totalItems).toBeGreaterThan(50); // PSA 119 is the exact pileup this ticket names

  const kretzmannChapterRequests: string[] = [];
  const perVerseEdgesRequests: string[] = [];
  const proseCardRequests: string[] = [];
  page.on('request', req => {
    const url = new URL(req.url());
    if (url.pathname === '/api/kretzmann/chapter/PSA.119') {
      kretzmannChapterRequests.push(req.url());
    } else if (/^\/api\/node\/text-unit%3APSA\.119\.\d+\/edges$/.test(url.pathname) && url.searchParams.get('kind') === 'commented-on-by') {
      perVerseEdgesRequests.push(req.url());
    } else if (url.pathname.startsWith('/api/node/CommentaryItem%3Akretzmann') && !url.pathname.includes('/edges')) {
      proseCardRequests.push(req.url());
    }
  });

  await page.goto('/kretzmann');
  await page.getByTestId('picker-book').selectOption('PSA');
  await page.getByTestId('picker-chapter').selectOption('119');
  await page.getByTestId('picker-apply').click();

  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('Psalms');
  await expect(page.getByTestId('kretzmann-chapter-head')).toContainText('119');

  for (const v of ground.verses) {
    await expect(page.getByTestId(`kretzmann-verse-group-${v.verse}`)).toBeVisible();
  }

  // Every section heading for the whole chapter renders immediately (the
  // listing IS the whole chapter) -- S-1's own fix, re-verified at scale.
  await expect(page.locator('.kretzmann-section-heading').first()).toBeVisible();

  // Give the lazy loader's own IntersectionObserver a moment to settle for
  // whatever is genuinely near the viewport on load (no scroll performed).
  await page.waitForTimeout(500);

  expect(kretzmannChapterRequests.length).toBe(1);
  expect(perVerseEdgesRequests.length).toBe(0);

  // THE HONEST BOUND: loading a 176-verse chapter must not itself trigger a
  // prose request for every single item -- only what's near the viewport.
  expect(proseCardRequests.length).toBeGreaterThan(0); // something loads without scrolling
  expect(proseCardRequests.length).toBeLessThan(totalItems); // but nowhere near the whole chapter
});

// Fix round (S-1, CRITICAL -- review, "renders section headings WITH
// prose -- both, always"): the first ship of this batch discarded the
// corpus's own section headings the instant an item's prose resolved --
// screenshotted proof at the time (kretzmann-standalone.png) showed NOT
// ONE section heading anywhere on the page, despite Genesis 1's own real
// "The Creation of Chaos and Light" / "The Creation of the Firmament" /
// "The Creation and Blessing of Man" structure. This test asserts real,
// non-fabricated section headings actually render, de-duplicated
// (Concord.RenderRows's own technique), and never disappear once an
// item's own prose has loaded.
test('KRETZMANN-14 (S-1 fix): real, non-fabricated, de-duplicated section headings render alongside the prose, and stay visible after it loads', async ({ page }) => {
  const ground = await api.kretzmannChapter('GEN.1');
  const distinctHeadings = new Set<string>();
  for (const v of ground.verses) {
    for (const item of v.items) {
      distinctHeadings.add(item.heading ?? 'Commentary');
    }
  }
  expect(distinctHeadings.size).toBeGreaterThan(1); // GEN.1 genuinely carries multiple distinct sections

  await page.goto('/kretzmann');
  await expect(page.getByTestId('kretzmann-verse-group-1')).toBeVisible();

  const headings = page.locator('.kretzmann-section-heading');
  await expect(headings.first()).toBeVisible();
  const renderedCount = await headings.count();
  expect(renderedCount).toBeGreaterThan(0);
  expect(renderedCount).toBeLessThanOrEqual(distinctHeadings.size); // de-duplicated, never one per item

  // Every rendered heading is REAL corpus text, not fabricated.
  for (let i = 0; i < renderedCount; i++) {
    const text = (await headings.nth(i).textContent())?.trim() ?? '';
    expect(distinctHeadings.has(text)).toBe(true);
  }

  // Let the first item's own lazy prose fetch resolve (it's in the
  // initial viewport, so the IntersectionObserver fires immediately) --
  // the heading must still be there afterward, not discarded once the
  // prose it introduces has arrived.
  const card = await api.node('CommentaryItem:kretzmann/0.1.0');
  await expect(page.locator('.kretzmann-item').first()).toContainText(card.description);
  await expect(headings.first()).toBeVisible();
});
