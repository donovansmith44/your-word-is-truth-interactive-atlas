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

test('CONCORD-4 (ONE-RULE): plain click on a paragraph opens the existing explore/popover, carrying the REAL full paragraph text', async ({ page }) => {
  const ground = await api.reading('BoC 1.1.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await page.goto('/concord');
  await expect(page.getByTestId('concord-unit-BoC-1-1-1')).toBeVisible();

  await page.getByTestId('concord-unit-BoC-1-1-1').click();

  await expect(page.getByTestId('popover-title')).toContainText('BoC 1.1.1');
  await expect(page.getByTestId('popover-body')).toContainText(realText);
});

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
test('CONCORD-9 (BoC nav menu): Contents reveals the ten traditional documents; jumping lands in full reading flow, not a bare fragment', async ({ page }) => {
  await page.goto('/concord');
  await expect(page.getByTestId('concord-toc-menu')).toHaveCount(0); // collapsed by default -- no chrome clutter
  await expect(page.getByTestId('concord-toc-toggle')).toHaveAttribute('aria-expanded', 'false');

  await page.getByTestId('concord-toc-toggle').click();
  await expect(page.getByTestId('concord-toc-menu')).toBeVisible();
  await expect(page.getByTestId('concord-toc-toggle')).toHaveAttribute('aria-expanded', 'true');

  // The traditional documents, verbatim (Explore/ConcordToc.cs, mirroring
  // atlas-etl/src/concord.rs's own CONCORD_DOC_SPECS).
  await expect(page.getByTestId('concord-toc-part-3')).toContainText('Augsburg Confession');
  await expect(page.getByTestId('concord-toc-part-7')).toContainText('Small Catechism');
  await expect(page.getByTestId('concord-toc-part-9')).toContainText('Epitome');

  const ground = await api.reading('BoC 7.1.1', 1, { corpus: 'concord' });
  const realText = ground.units[0].text as string;

  await page.getByTestId('concord-toc-part-7').click();

  // Landed in FULL READING FLOW -- the part heading, the first paragraph's
  // own real text, both rendered -- never a bare fragment.
  await expect(page.getByTestId('concord-toc-menu')).toHaveCount(0); // closes on jump
  await expect(page.getByTestId('concord-position')).toContainText('BoC 7.1.1');
  await expect(page.getByTestId('concord-part-heading-7')).toBeVisible();
  await expect(page.getByTestId('concord-part-heading-7')).toContainText('The Small Catechism');
  await expect(page.getByTestId('concord-unit-BoC-7-1-1')).toContainText(realText);
});

// Ticket C, "same mechanism as ticket K -- do not fork the scanner."
test('CONCORD-10 (explorable-reference law): a scripture reference inside confession prose opens the SAME VerseNode popover', async ({ page }) => {
  await page.goto('/concord');
  await page.getByTestId('concord-toc-toggle').click();
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
