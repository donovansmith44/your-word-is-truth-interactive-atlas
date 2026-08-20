import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbChapterRef, arbVerseRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';

test('READ-1 + NAV-1: chapter deep links render exactly the TOC verses', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    await page.goto(`/read/${c.book}/${c.chapter}`);
    await expect(page.getByTestId(/^verse-line-/)).toHaveCount(c.verses);
    await expect(page.getByTestId('verse-num-1')).toHaveText('1');
    await expect(page.getByTestId(`verse-num-${c.verses}`)).toHaveText(String(c.verses));
  }), RUNS_UI);
});

// Batch G1: the ∴ button is dead -- the verse LINE itself is the explorable
// element (CONTRACT's ONE-RULE note). Every test below that used to click
// `verse-num-{n}` to open the popover now clicks `verse-line-{n}` instead;
// verse-num's own separate role (selection/anchor+extend only, no popover)
// is covered by its own dedicated test further down.
test('READ-2: verse popover shows the API text', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-line-${v}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(vref);
    const detail = await api.verse(vref);
    await expect(page.getByTestId('popover')).toContainText(detail.text.slice(0, 40));
  }), RUNS_UI);
});

test('READ-2b: verse line hover darkens (.explorable) and Enter, while keyboard-focused, opens the same popover', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    const line = page.getByTestId(`verse-line-${v}`);

    const restBg = await line.evaluate(el => getComputedStyle(el).backgroundColor);
    await line.hover();
    await expect.poll(() => line.evaluate(el => getComputedStyle(el).backgroundColor)).not.toBe(restBg);
    await page.mouse.move(0, 0);

    await line.focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText(vref);
  }), RUNS_UI);
});

test('READ-2c: chapter head is explorable -- opens a ChapterNode popover', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbChapterRef(toc), async c => {
    await page.goto(`/read/${c.book}/${c.chapter}`);
    await page.getByTestId('chapter-head').click();
    await expect(page.getByTestId('popover-title')).toHaveText(`${c.book}.${c.chapter}`);
  }), RUNS_UI);
});

// Requirement 1's own split, verbatim: "selection and exploration are
// separate gestures on separate targets." A plain verse-num click must
// NEVER open a popover (it did, redundantly with the dead ∴ button, before
// this batch) -- it only sets the anchor, invisibly, for a LATER shift-click
// to extend into a real passage-chip range.
test('READ-2d: a plain verse-num click never opens a popover, but still sets a usable anchor for shift-click', async ({ page }) => {
  const toc = await loadToc();
  const arb = arbChapterRef(toc).filter(c => c.verses >= 2);
  await fcAssert(fc.asyncProperty(arb, async c => {
    await page.goto(`/read/${c.book}/${c.chapter}`);

    await page.getByTestId('verse-num-1').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);

    await page.keyboard.down('Shift');
    await page.getByTestId('verse-num-2').click();
    await page.keyboard.up('Shift');
    await expect(page.getByTestId('passage-chip')).toContainText(`${c.book}.${c.chapter}.1-2`);
  }), RUNS_UI);
});

// Batch R requirement 3(b): the old popover-chip-xrefs TOGGLE is gone --
// cross-references render INLINE, immediately, no button press -- so each
// loop iteration below reads xref-item-* directly rather than clicking a
// chip to reveal them first.
test('READ-3: cross-ref chains push and pop breadcrumbs faithfully', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), fc.array(fc.nat(4), { maxLength: 3 }), async (vref, picks) => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    await page.getByTestId(`verse-line-${v}`).click();
    const titles = [vref];
    for (const pick of picks) {
      const items = page.getByTestId(/^xref-item-/);
      const n = await items.count();
      if (n === 0) break;
      const detail = await api.verse(titles[titles.length - 1]);
      for (let i = 0; i < Math.min(n, detail.cross_refs.length); i++) {   // list order == API order
        await expect(items.nth(i)).toContainText(detail.cross_refs[i].target);
      }
      const chosen = Math.min(pick, n - 1);
      const target = detail.cross_refs[chosen].target;
      await items.nth(chosen).click();
      const head = target.match(/^[A-Z0-9]{3}\.\d+\.\d+/)![0];
      await expect(page.getByTestId('popover-title')).toHaveText(head);
      titles.push(head);
    }
    while (titles.length > 1) {                                            // walk back restores each title
      await page.getByTestId('popover-breadcrumb-back').click();
      titles.pop();
      await expect(page.getByTestId('popover-title')).toHaveText(titles[titles.length - 1]);
    }
  }), RUNS_UI);
});

// Batch R requirement 6 (user 2026-08-19: "the buttons for going to the next
// chapter should always be visible as being middle-aligned on the page even
// as i scroll"): reader-prev/reader-next are REPOSITIONED from quiet bottom
// corners to vertically-centered at the reading column's own left/right
// edges, position:fixed (unchanged), so they neither move nor disappear as
// the page scrolls. Uses the actual longest chapter in the canon (found from
// the live TOC, not hardcoded) so there is real scroll room to prove the
// "even as i scroll" half of the claim, not just "fixed at page load".
test('NAV-2: chapter-nav stays fixed, vertically centered, and visible while scrolled deep into a long chapter', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }
  expect(longest.verses, 'expected a real long chapter to test scroll persistence against').toBeGreaterThan(30);

  await page.goto(`/read/${longest.book}/${longest.chapter}`);
  const next = page.getByTestId('reader-next');
  const prev = page.getByTestId('reader-prev');
  await expect(next).toBeVisible();
  const boxAtTop = await next.boundingBox();

  // position:fixed -- unaffected by ordinary CSS layout, but confirm the
  // ACTUAL rendered position doesn't drift as real content scrolls past it.
  await page.getByTestId(`verse-line-${Math.floor(longest.verses / 2)}`).scrollIntoViewIfNeeded();
  await expect(next).toBeInViewport();
  await expect(prev).toBeInViewport();
  const boxScrolled = await next.boundingBox();
  expect(boxScrolled!.y).toBeCloseTo(boxAtTop!.y, 0);
  expect(boxScrolled!.x).toBeCloseTo(boxAtTop!.x, 0);

  // Vertically centered in the viewport (requirement 6's own "middle-
  // aligned"), not pinned to a bottom corner anymore.
  const viewport = page.viewportSize()!;
  const center = boxScrolled!.y + boxScrolled!.height / 2;
  expect(Math.abs(center - viewport.height / 2)).toBeLessThan(viewport.height * 0.15);

  // Quiet until hovered -- unchanged treatment, still worth a smoke check.
  const restColor = await next.evaluate(el => getComputedStyle(el).color);
  await next.hover();
  await expect.poll(() => next.evaluate(el => getComputedStyle(el).color)).not.toBe(restColor);
});
