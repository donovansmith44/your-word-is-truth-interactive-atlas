import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch CORPREAD-1b (ticket C, owner order verbatim: "I want to be able ot
// read it like the normal BoC. Everything doesn't need to be shown
// visually as belonging to these boxes. It should be a smooth reading
// experiecne like the Bible, except verses and verse fragements and
// verse/passage references are explorable. I should have a menu through
// which I can navigate to different parts in the BoC.") -- REBUILDS
// CORP-1's own bordered-row list into a continuous reading surface.
// Coverage: tab navigation (unchanged); part/article/paragraph browse with
// paragraphs now flowing as prose, part/article structure carried by
// typography; the NEW nav menu; ONE-RULE explore-on-click; the
// explorable-reference law (the shared scanner, ticket K's own mechanism);
// read-beside split with clean degradation (unchanged); the boundary
// no-duplicate fix (unchanged mechanism).

test('CONCORD-1: nav-concord reaches /concord from both Reader and World', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByTestId('nav-concord')).toBeVisible();
  await page.getByTestId('nav-concord').click();
  await expect(page).toHaveURL(/\/concord$/);
  await expect(page.getByTestId('concord-page')).toBeVisible();

  await page.goto('/world');
  await expect(page.getByTestId('nav-concord')).toBeVisible();
  await page.getByTestId('nav-concord').click();
  await expect(page).toHaveURL(/\/concord$/);
  await expect(page.getByTestId('concord-page')).toBeVisible();
});

test('CONCORD-2: opens on the Preface (BoC 1.1.1) and every rendered paragraph is real, non-fabricated text', async ({ page }) => {
  const ground = await api.reading('BoC 1.1.1', 20, { corpus: 'concord' });
  expect(ground.units.length).toBeGreaterThan(0);

  await page.goto('/concord');
  await expect(page.getByTestId('concord-page')).toBeVisible();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 1.1.1');

  for (const unit of ground.units) {
    const row = page.getByTestId(`concord-unit-${unit.ref.replace(/ /g, '-').replace(/\./g, '-')}`);
    await expect(row).toBeVisible();
    await expect(row).toContainText(unit.text);
  }
});

test('CONCORD-2b (ticket C, "boxes die ... part/article structure carried by TYPOGRAPHY"): the Preface\'s own part heading renders, no bordered row anywhere', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-part-heading-1')).toBeVisible();
  await expect(page.getByTestId('concord-part-heading-1')).toContainText('Preface to the Book of Concord');

  // The retired CORP-1 box treatment (border + background on the row
  // itself) is genuinely gone -- a real, live style assertion, not just
  // "the class name changed."
  const firstUnit = page.locator('.concord-unit').first();
  await expect(firstUnit).toHaveCSS('border-style', 'none');
});

test('CONCORD-3: the part/article/paragraph picker navigates the corpus\'s OWN shape -- jumping to 7.2.1 shows the real First Commandment text', async ({ page }) => {
  const ground = await api.reading('BoC 7.2.1', 1, { corpus: 'concord' });
  expect(ground.units).toHaveLength(1);
  const realText = ground.units[0].text as string;

  await page.goto('/concord');
  await page.getByTestId('concord-picker-part').fill('7');
  await page.getByTestId('concord-picker-article').fill('2');
  await page.getByTestId('concord-picker-paragraph').fill('1');
  await page.getByTestId('concord-picker-go').click();

  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.2.1');
  await expect(page.getByTestId('concord-unit-BoC-7-2-1')).toContainText(realText);
});

// Batch UX-1 (BOC-CLICK-1): this test's own fixture moved from the
// Preface's own BoC 1.1.1 to the Small Catechism's own BoC 7.2.1 -- Part 1
// is no longer explorable under this batch's own ruling (ConcordToc.
// IsExplorablePart), and BoC 7.2.1 (the First Commandment) already IS the
// established explorable fixture this same file's own CONCORD-3/CONCORD-9d
// tests use. ONE-RULE itself (plain click opens the existing explore/
// popover, carrying the real full paragraph text) is otherwise UNCHANGED
// -- see the new BOC-CLICK-1 test below for explicit coverage of the
// retired affordance on a non-explorable Part.
test('CONCORD-4 (ONE-RULE): plain click on an explorable paragraph opens the existing explore/popover, carrying the REAL full paragraph text', async ({ page }) => {
  const ground = await api.reading('BoC 7.2.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await page.goto('/concord');
  await page.getByTestId('concord-toc-part-7').click();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.1.1');
  await page.getByTestId('concord-picker-part').fill('7');
  await page.getByTestId('concord-picker-article').fill('2');
  await page.getByTestId('concord-picker-paragraph').fill('1');
  await page.getByTestId('concord-picker-go').click();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.2.1');
  await expect(page.getByTestId('concord-unit-BoC-7-2-1')).toBeVisible();

  await page.getByTestId('concord-unit-BoC-7-2-1').click();

  await expect(page.getByTestId('popover-title')).toContainText('BoC 7.2.1');
  await expect(page.getByTestId('popover-body')).toContainText(realText);
});

// ---------------------------------------------------------------------
// BOC-CLICK-1 (owner order, verbatim: "Articles/blocks of text needn't be
// clickable in the BoC right now because there's no actual explorable
// stuff with them. The exception should be the catechism and the creeds
// where we actually do have explorable stuff. make sure that we retain
// the scripture references."): explorability is a DECLARED property of
// the DOCUMENT (ConcordToc.IsExplorablePart) -- see that method's own doc
// comment for the disclosed "Part 7 only" reasoning.
// ---------------------------------------------------------------------

test('BOC-CLICK-1: a non-catechism unit (the Preface) loses the explorable-row affordance -- no cursor/hover class, no click', async ({ page }) => {
  await page.goto('/concord');
  const row = page.getByTestId('concord-unit-BoC-1-1-1');
  await expect(row).toBeVisible();
  await expect(row).not.toHaveClass(/\bexplorable\b/);
  await expect(row).not.toHaveAttribute('role', 'button');
  await expect(row).not.toHaveAttribute('tabindex');

  await row.click();
  // Nothing opens -- the row itself carries no click handler any more.
  await expect(page.getByTestId('popover-title')).toHaveCount(0);
});

test('BOC-CLICK-1: a catechism unit (BoC 7.2.1, the First Commandment) keeps the explorable-row affordance', async ({ page }) => {
  await page.goto('/concord');
  await page.getByTestId('concord-picker-part').fill('7');
  await page.getByTestId('concord-picker-article').fill('2');
  await page.getByTestId('concord-picker-paragraph').fill('1');
  await page.getByTestId('concord-picker-go').click();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.2.1');

  const row = page.getByTestId('concord-unit-BoC-7-2-1');
  await expect(row).toBeVisible();
  await expect(row).toHaveClass(/\bexplorable\b/);
  await expect(row).toHaveAttribute('role', 'button');
  await expect(row).toHaveAttribute('tabindex', '0');

  await row.click();
  await expect(page.getByTestId('popover-title')).toContainText('BoC 7.2.1');
});

// "make sure that we retain the scripture references" -- CONCORD-10 below
// already proves an in-text scripture reference stays clickable inside
// Part 3's own confession prose; Part 3 is a NON-explorable Part under
// this batch's own predicate (ExplorableParts = {7}), so that coverage now
// doubles as this ruling's own "scripture refs unaffected by BOC-CLICK-1"
// proof -- not duplicated here.

test('CONCORD-5: next/previous page the reading spine', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-next')).toBeVisible();
  await expect(page.getByTestId('concord-prev')).toHaveCount(0); // nothing precedes the Preface's own first paragraph

  const firstPosition = await page.getByTestId('concord-position').textContent();
  await page.getByTestId('concord-next').click();
  await expect(page.getByTestId('concord-position')).not.toHaveText(firstPosition ?? '');
  await expect(page.getByTestId('concord-prev')).toBeVisible();
});

test('CONCORD-6: declares its own "read-beside" hatch -- split opens with Concord hosting, Reader as a genuine, live guest', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-concord')).toBeVisible();

  await page.getByTestId('split-open-concord').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();
  await expect(page.getByTestId('concord-page')).toBeVisible();
  await expect(page.getByTestId('split-open-concord')).toHaveCount(0);

  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page).toHaveURL(/\/concord\?split=reader$/);

  await page.getByTestId('split-close-reader-guest').click();
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('concord-page')).toBeVisible();
});

test('CONCORD-7 (degraded-link law, unchanged): concord+reader has no BearsWindow/BearsLocus member -- no follow chip, no follow link, no world map, ever', async ({ page }) => {
  await page.goto('/concord');
  await page.getByTestId('split-open-concord').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await expect(page.getByTestId('follow-chip')).toHaveCount(0);
  await expect(page.getByTestId('kretzmann-follow-chip')).toHaveCount(0);
  await expect(page.getByTestId('mode-chip')).toHaveCount(0);
  await expect(page.getByTestId('world-map')).toHaveCount(0);
});

test('CONCORD-8 (batch-corp1-report.md §13 boundary-overlap fix, unchanged): Next then Previous never repeats a row', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-next')).toBeVisible();
  const firstPosition = await page.getByTestId('concord-position').textContent();

  await page.getByTestId('concord-next').click();
  await expect(page.getByTestId('concord-position')).not.toHaveText(firstPosition ?? '');
  const page2AnchorRef = await page.getByTestId('concord-position').textContent();
  expect(page2AnchorRef).toBeTruthy();
  const anchorSlug = (page2AnchorRef ?? '').replace(/ /g, '-').replace(/\./g, '-');
  await expect(page.getByTestId(`concord-unit-${anchorSlug}`)).toBeVisible();

  await page.getByTestId('concord-prev').click();
  await expect(page.getByTestId('concord-position')).toBeVisible();
  await expect(page.getByTestId(`concord-unit-${anchorSlug}`)).toHaveCount(0);

  await page.getByTestId('concord-next').click();
  await expect(page.getByTestId('concord-position')).toContainText(page2AnchorRef ?? '');
});

// Ticket C, owner order verbatim: "I should have a menu through which I
// can navigate to different parts in the BoC."
// Batch CORPREAD-2 (C2, owner verdict 2 verbatim: "contents should always
// be visible, not needed to click"): the click-to-open toggle
// (concord-toc-toggle/concord-toc-menu) is RETIRED -- the tree now lives,
// unconditionally, inside concord-sidebar-toc. No toggle click anywhere in
// this test any more.
test('CONCORD-9 (BoC nav menu, C2: always visible): the ten traditional documents are visible with no click; jumping lands in full reading flow, not a bare fragment', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-sidebar')).toBeVisible();
  await expect(page.getByTestId('concord-sidebar-toc')).toBeVisible();

  // The traditional documents, verbatim (Explore/ConcordToc.cs, mirroring
  // atlas-etl/src/concord.rs's own CONCORD_DOC_SPECS) -- visible with NO
  // interaction at all.
  await expect(page.getByTestId('concord-toc-part-3')).toContainText('Augsburg Confession');
  await expect(page.getByTestId('concord-toc-part-7')).toContainText('Small Catechism');
  await expect(page.getByTestId('concord-toc-part-9')).toContainText('Epitome');

  const ground = await api.reading('BoC 7.1.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await page.getByTestId('concord-toc-part-7').click();

  // Landed in FULL READING FLOW -- the part heading, the first paragraph's
  // own real text, both rendered -- never a bare fragment. The sidebar
  // stays visible after the jump (never closes -- there is no open/closed
  // state left to have).
  await expect(page.getByTestId('concord-sidebar-toc')).toBeVisible();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.1.1');
  await expect(page.getByTestId('concord-part-heading-7')).toBeVisible();
  await expect(page.getByTestId('concord-part-heading-7')).toContainText('The Small Catechism');
  await expect(page.getByTestId('concord-unit-BoC-7-1-1')).toContainText(realText);
});

// Batch CORPREAD-2 (C2, owner verdict 2 verbatim: "current-position
// highlighted"). Part 7 (the jump target above) must carry the
// current-marker; part 1 (the default Preface, no longer current) must not.
test('CONCORD-9c (C2, current-position highlighted): the sidebar tree entry for the current document is marked, and only that one', async ({ page }) => {
  await page.goto('/concord');
  const part1 = page.getByTestId('concord-toc-part-1');
  await expect(part1).toHaveClass(/\bconcord-toc-item-current\b/);
  await expect(part1).toHaveAttribute('aria-current', 'true');
  await expect(page.getByTestId('concord-toc-part-7')).not.toHaveClass(/\bconcord-toc-item-current\b/);

  await page.getByTestId('concord-toc-part-7').click();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.1.1');

  const part7 = page.getByTestId('concord-toc-part-7');
  await expect(part7).toHaveClass(/\bconcord-toc-item-current\b/);
  await expect(part7).toHaveAttribute('aria-current', 'true');
  await expect(part1).not.toHaveClass(/\bconcord-toc-item-current\b/);
  await expect(part1).not.toHaveAttribute('aria-current', 'true');
});

// Batch CORPREAD-2 (C2, owner verdict 2 verbatim: "search is also to the
// left of the text rather than always at the top of the page" -- this
// app's own disclosed reading: the pre-existing numeric picker, this
// page's one find-your-way control, moved into the sidebar).
test('CONCORD-9d (C2, search relocated into the sidebar): the numeric picker lives inside concord-sidebar-search, not above the reading column', async ({ page }) => {
  await page.goto('/concord');
  const sidebarSearch = page.getByTestId('concord-sidebar-search');
  await expect(sidebarSearch).toBeVisible();
  await expect(sidebarSearch.getByTestId('concord-picker-part')).toBeVisible();
  await expect(sidebarSearch.getByTestId('concord-picker-article')).toBeVisible();
  await expect(sidebarSearch.getByTestId('concord-picker-paragraph')).toBeVisible();
  await expect(sidebarSearch.getByTestId('concord-picker-go')).toBeVisible();

  const ground = await api.reading('BoC 7.2.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await sidebarSearch.getByTestId('concord-picker-part').fill('7');
  await sidebarSearch.getByTestId('concord-picker-article').fill('2');
  await sidebarSearch.getByTestId('concord-picker-paragraph').fill('1');
  await sidebarSearch.getByTestId('concord-picker-go').click();

  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.2.1');
  await expect(page.getByTestId('concord-unit-BoC-7-2-1')).toContainText(realText);
});

// Batch CORPREAD-2 (C2, "below the app's own established responsive
// floor... always visible binds at desktop widths" -- the SAME breakpoint
// .split-open/.split-close already hide behind, 1023.98px).
test('CONCORD-9e (C2, responsive collapse): below the app\'s existing split-affordance breakpoint, the sidebar stacks above the reading column instead of pinning left', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 900 });
  await page.goto('/concord');

  const sidebarBox = await page.getByTestId('concord-sidebar').boundingBox();
  const readingBox = await page.locator('.concord-reading').boundingBox();
  expect(sidebarBox).toBeTruthy();
  expect(readingBox).toBeTruthy();
  // Stacked: the sidebar's own box sits ABOVE the reading area's box (its
  // own bottom edge at or above the reading area's own top edge), not
  // beside it.
  expect(sidebarBox!.y + sidebarBox!.height).toBeLessThanOrEqual(readingBox!.y + 1);

  // At a desktop width, the SAME two boxes sit side by side instead.
  await page.setViewportSize({ width: 1280, height: 900 });
  const sidebarBoxWide = await page.getByTestId('concord-sidebar').boundingBox();
  const readingBoxWide = await page.locator('.concord-reading').boundingBox();
  // Fix round (Q-6, TRIVIA -- review): a bare "sidebar top is above the
  // reading area's own bottom edge" is satisfied by almost any layout,
  // including the STACKED one this same test's first half already proves --
  // real vertical OVERLAP (the sidebar's own bottom edge reaches at least as
  // far down as the reading area's own top edge) is what actually
  // distinguishes "side by side" from "stacked", combined with the x-adjacency
  // assertion immediately below.
  expect(sidebarBoxWide!.y).toBeLessThan(readingBoxWide!.y + readingBoxWide!.height);
  expect(sidebarBoxWide!.y + sidebarBoxWide!.height).toBeGreaterThan(readingBoxWide!.y);
  expect(sidebarBoxWide!.x + sidebarBoxWide!.width).toBeLessThanOrEqual(readingBoxWide!.x + 1);
});

// Fix round (S-8, TRIVIA -- review): CONCORD-9 above only spot-verified
// StartRef ("BoC {part}.1.1" exists) for parts 1 (the default load) and 7
// -- the other eight were unproven; a document whose first article isn't
// numbered 1 would land on the error toast instead. Cheap to cover, per
// the review -- extended here to every one of the ten menu entries. This
// list mirrors Explore/ConcordToc.cs's own ten-entry table verbatim (a
// Playwright spec has no access to that C# constant directly -- the same
// "read directly, not guessed" discipline that table's own header
// documents against atlas-etl/src/concord.rs's real DOCUMENTS table).
const CONCORD_TOC_DOCUMENTS = [
  { part: 1, title: 'Preface to the Book of Concord' },
  { part: 2, title: 'The Three Ecumenical Creeds' },
  { part: 3, title: 'The Augsburg Confession' },
  { part: 4, title: 'Apology of the Augsburg Confession' },
  { part: 5, title: 'The Smalcald Articles' },
  { part: 6, title: 'Treatise on the Power and Primacy of the Pope' },
  { part: 7, title: 'The Small Catechism' },
  { part: 8, title: 'The Large Catechism' },
  { part: 9, title: 'Formula of Concord: Epitome' },
  { part: 10, title: 'Formula of Concord: Solid Declaration' },
];

test('CONCORD-9b (S-8): every one of the ten TOC entries lands on its own real opening paragraph, not an error toast', async ({ page }) => {
  await page.goto('/concord');

  for (const doc of CONCORD_TOC_DOCUMENTS) {
    const startRef = `BoC ${doc.part}.1.1`;
    const ground = await api.reading(startRef, 1, { corpus: 'concord' });
    expect(ground.units).toHaveLength(1);
    const realText = ground.units[0].text as string;

    // Batch CORPREAD-2 (C2): always visible now -- no toggle click needed.
    await page.getByTestId(`concord-toc-part-${doc.part}`).click();

    await expect(page.getByTestId('toast')).toHaveCount(0);
    await expect(page.getByTestId('concord-position')).toContainText(startRef);
    await expect(page.getByTestId(`concord-part-heading-${doc.part}`)).toBeVisible();
    await expect(page.getByTestId(`concord-part-heading-${doc.part}`)).toContainText(doc.title);
    await expect(page.getByTestId(`concord-unit-${startRef.replace(/ /g, '-').replace(/\./g, '-')}`)).toContainText(realText);
  }
});

// Batch UX-1 (BOC-SCROLL-1, owner order verbatim: "if i click one of the
// items in table of contents and scroll to the bottom, and then click
// another item in the table of contents, i am taken to the bottom of that
// item"): the exact repro, reproduced then proven fixed -- click a ToC
// item, scroll to the bottom, click ANOTHER ToC item, land at the TOP.
test('BOC-SCROLL-1: clicking a second ToC item after scrolling to the bottom of the first lands at the TOP, not the bottom', async ({ page }) => {
  await page.goto('/concord');
  await page.getByTestId('concord-toc-part-3').click(); // The Augsburg Confession
  await expect(page.getByTestId('concord-position')).toContainText('BoC 3.1.1');

  await page.evaluate(() => window.scrollTo(0, document.body.scrollHeight));
  const scrolledY = await page.evaluate(() => window.scrollY);
  expect(scrolledY).toBeGreaterThan(200); // genuinely scrolled down, not a no-op on a short page

  await page.getByTestId('concord-toc-part-5').click(); // The Smalcald Articles -- a DIFFERENT document
  await expect(page.getByTestId('concord-position')).toContainText('BoC 5.1.1');

  await expect(async () => {
    const y = await page.evaluate(() => window.scrollY);
    expect(y).toBeLessThan(10);
  }).toPass({ timeout: 3000 });

  // Landed in reading flow at the TOP -- the new document's own part
  // heading is genuinely visible (in the viewport), not scrolled past.
  await expect(page.getByTestId('concord-part-heading-5')).toBeInViewport();
});

// Ticket C, "same mechanism as ticket K -- do not fork the scanner."
test('CONCORD-10 (explorable-reference law): a scripture reference inside confession prose opens the SAME VerseNode popover', async ({ page }) => {
  await page.goto('/concord');
  // Batch CORPREAD-2 (C2): always visible now -- no toggle click needed.
  // The Augsburg Confession -- real, verified citations present in its own
  // vendored text (data/raw/concord/augsburg-confession.html), e.g. "Luke
  // 17:10", "Eph. 4:5-6", within its first several reading windows.
  await page.getByTestId('concord-toc-part-3').click();
  await expect(page.getByTestId('concord-position')).toContainText('BoC 3.1.1');

  let ref = page.locator('.concord-ref').first();
  for (let i = 0; i < 12 && (await ref.count()) === 0; i++) {
    await page.getByTestId('concord-next').click();
    ref = page.locator('.concord-ref').first();
  }
  await expect(ref).toBeVisible();

  await ref.click();

  await expect(page.getByTestId('popover-title')).toBeVisible();
  // A vref-shaped title (e.g. "LUK.17.10") -- the SAME VerseNode popover
  // every other verse reference in this app opens, not a bespoke citation
  // popover.
  await expect(page.getByTestId('popover-title')).toHaveText(/^[A-Z0-9]{2,3}\.\d+\.\d+/);
});
