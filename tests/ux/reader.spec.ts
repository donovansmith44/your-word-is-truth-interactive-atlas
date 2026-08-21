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

// Batch R fix round 1, Critical-1 (review 2026-08-20): NAV-2 above proves
// reader-prev/reader-next stay VISIBLE and roughly VIEWPORT-CENTERED while
// scrolled -- it never checked that centering against the reading column's
// own text at a REALISTIC width. In split view at the brief's own
// documented 1024px floor, the pane narrows to ~55% of the viewport, and
// the reading column (max-width:66ch, never actually clamping at that
// width) fills the pane almost edge-to-edge -- the nav buttons, sized for a
// full-width standalone reader, printed directly across mid-chapter verse
// text (reviewer-measured live: review-diag-split-1024-overlap.png, ~59px
// of overlap on each side). This asserts real, geometric non-overlap
// between EVERY currently-rendered verse row and each nav button, in split
// view, at the floor width -- not just "visible"/"centered" (NAV-2's own
// bar), which this exact regression already satisfied while still visually
// overlapping text underneath.
test('NAV-3: chapter-nav never overlaps the verse text column in split view at the documented 1024px floor', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }
  expect(longest.verses, 'expected a real long chapter for real mid-chapter scroll room').toBeGreaterThan(30);

  await page.setViewportSize({ width: 1024, height: 800 });
  await page.goto(`/read/${longest.book}/${longest.chapter}?split=1`);

  const mid = Math.floor(longest.verses / 2);
  await page.getByTestId(`verse-line-${mid}`).scrollIntoViewIfNeeded();

  const prev = page.getByTestId('reader-prev');
  const next = page.getByTestId('reader-next');
  // --chapter-nav-top's own recompute is scroll-driven (reader.js's
  // watchChapterNavCenter, rAF-throttled) -- wait for the REAL settled
  // position (toBeInViewport, not a one-shot read) before trusting either
  // box: a pre-recompute read produces a stale/off-screen Y that is a
  // timing artifact, not a real bug (the exact red herring the review's own
  // diagnostic flagged and warned future runs away from).
  await expect(prev).toBeInViewport();
  await expect(next).toBeInViewport();

  const prevBox = await prev.boundingBox();
  const nextBox = await next.boundingBox();
  expect(prevBox && nextBox).toBeTruthy();

  // Check EVERY currently-rendered verse row (the reader has no
  // virtualization -- a whole chapter's own rows are all in the DOM at
  // once), not just the one scrolled to: a real 2D bounding-box
  // intersection against each button, so this catches the bug regardless
  // of exactly which row ends up sharing the buttons' own vertical band.
  const overlaps = await page.evaluate(({ prevBox, nextBox }) => {
    const rows = Array.from(document.querySelectorAll('[data-testid^="verse-line-"]'));
    const hits: string[] = [];
    for (const row of rows) {
      const r = row.getBoundingClientRect();
      for (const [name, b] of [['reader-prev', prevBox], ['reader-next', nextBox]] as const) {
        if (!b) continue;
        const vOverlap = r.top < b.y + b.height && r.bottom > b.y;
        const hOverlap = r.left < b.x + b.width && r.right > b.x;
        if (vOverlap && hOverlap) hits.push(`${name} over ${row.getAttribute('data-testid')}`);
      }
    }
    return hits;
  }, { prevBox, nextBox });
  expect(overlaps).toEqual([]);
});

// HOTFIX batch (batch-hotfix-brief.md requirement 2, NAV-4, user report
// 2026-08-20: "the buttons to go chapter to chapter on the Bible in split
// screen are too tiny to see" -- measured live before this fix: 14.8x22.8px
// at a 13.6px font, well under any real hit-target floor). Same documented
// 1024px floor NAV-3 already uses (the tightest realistic split width -- a
// wider split only ever has MORE gutter room, never less, so this floor is
// the binding case for both "big enough" and "never overlaps").
test('NAV-4: reader-prev/reader-next measure a real, >=40x40px hit target in split view -- legible label, parchment contrast, quiet-until-hover, NAV-3 stays green', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }

  await page.setViewportSize({ width: 1024, height: 800 });
  await page.goto(`/read/${longest.book}/${longest.chapter}?split=1`);

  const prev = page.getByTestId('reader-prev');
  const next = page.getByTestId('reader-next');
  await expect(prev).toBeInViewport();
  await expect(next).toBeInViewport();

  const prevBox = await prev.boundingBox();
  const nextBox = await next.boundingBox();
  expect(prevBox && nextBox).toBeTruthy();
  for (const [name, box] of [['reader-prev', prevBox], ['reader-next', nextBox]] as const) {
    expect(box!.width, `${name} width`).toBeGreaterThanOrEqual(40);
    expect(box!.height, `${name} height`).toBeGreaterThanOrEqual(40);
  }

  // "Arrow glyph + chapter label legible" (the brief's own phrase, verbatim)
  // -- both present, not just the glyph: fix round 1's own chevron-only
  // `display:none` treatment for .reader-nav-label is gone in split view.
  await expect(prev.locator('.reader-nav-label')).toBeVisible();
  await expect(next.locator('.reader-nav-label')).toBeVisible();

  // Parchment contrast >= 10:1 (this requirement's own floor, up from the
  // pre-batch --ink-soft's 5.66:1) -- computed live against the reader
  // page's own background, not assumed. rgb(43,33,23) is --ink (#2B2117),
  // rgb(246,241,229) is --parchment (#F6F1E5); 13.98:1 by the WCAG relative-
  // luminance formula (see the batch report for the computation).
  const [color, bg] = await prev.evaluate(el => [
    getComputedStyle(el).color,
    getComputedStyle(document.querySelector('.reader-page')!).backgroundColor,
  ]);
  expect(color).toBe('rgb(43, 33, 23)');
  expect(bg).toBe('rgb(246, 241, 229)');

  // Quiet-until-hover still holds (visible always, amplified on hover) --
  // same smoke check NAV-2 already runs for the standalone reader.
  const restColor = await next.evaluate(el => getComputedStyle(el).color);
  await next.hover();
  await expect.poll(() => next.evaluate(el => getComputedStyle(el).color)).not.toBe(restColor);

  // NAV-3 (chapter-nav/verse-column non-overlap) is a SEPARATE, unchanged
  // test above in this same file -- re-run it as part of the same suite
  // invocation this batch verifies with, rather than duplicating its own
  // geometric intersection check here.
});

// HOTFIX-3 (batch-hotfix3-brief.md, user report 2026-08-21, near-verbatim:
// "the next/previous chapter buttons shouldn't be redrawn on every scroll.
// they should be fixed in place like on bible.com"). NAV-2 above only ever
// samples boundingBox() at REST -- once before scrolling, once after a
// single scrollIntoViewIfNeeded settles -- so it already passed on the
// buggy code: the actual bug (root-caused live, not assumed -- see
// reader.js's own watchChapterNavCenter comment for the full
// rAF-timestamped trace and evidence) was a TRANSIENT one-frame position
// "teleport" of the whole nav, sized to that gesture's own scroll delta,
// exactly at the START of every discrete scroll input, gone again one
// frame later -- invisible to any test that only checks two rest points.
// This samples EVERY frame DURING a scripted, continuous scroll instead,
// so that transient can't hide between two rest-state checks the way it
// did before. `toBeCloseTo(_, 0)` matches NAV-2's own precision choice
// (within 0.5px, i.e. sub-pixel rounding only).
test('NAV-5: reader-prev/reader-next stay pixel-IDENTICAL across every sampled frame of a scripted >=3-viewport-height scroll -- no reposition jitter, standalone', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }
  expect(longest.verses, 'expected a real long chapter for real scroll room').toBeGreaterThan(30);

  await page.goto(`/read/${longest.book}/${longest.chapter}`);
  await expect(page.getByTestId('reader-next')).toBeVisible();

  for (const testId of ['reader-prev', 'reader-next']) {
    await page.evaluate(() => window.scrollTo(0, 0));
    const trace = await page.evaluate(({ testId }) => new Promise<{ x: number; y: number }[]>(resolve => {
      const pts: { x: number; y: number }[] = [];
      const frames = 90; // well over the brief's own >=5-sample floor
      const delta = (window.innerHeight * 3) / frames; // >=3 viewport heights total
      let frame = 0;
      const step = () => {
        const el = document.querySelector(`[data-testid="${testId}"]`) as HTMLElement | null;
        const r = el ? el.getBoundingClientRect() : null;
        if (r) pts.push({ x: r.x, y: r.y });
        if (frame < frames) {
          window.scrollBy(0, delta);
          frame++;
          requestAnimationFrame(step);
        } else {
          resolve(pts);
        }
      };
      requestAnimationFrame(step);
    }), { testId });

    expect(trace.length, `${testId}: expected >=5 samples across the scroll`).toBeGreaterThanOrEqual(5);
    for (const p of trace) {
      expect(p.x, `${testId}.x drifted mid-scroll`).toBeCloseTo(trace[0].x, 0);
      expect(p.y, `${testId}.y drifted mid-scroll`).toBeCloseTo(trace[0].y, 0);
    }
  }
});

// Same assertion as NAV-5, split view -- the brief's own root-cause leads
// named a SECOND, split-specific suspect (an undocumented transform/filter
// ancestor demoting position:fixed further inside the pane). Root-causing
// live found no such ancestor (`.reader-page`'s own `contain: layout`,
// Batch H, is the SAME single containing-block-creating element in both
// panes -- .split-pane-reader carries no transform/filter/contain of its
// own, confirmed by reading app.css directly) and the measured glitch was
// IDENTICAL in both panes -- but this asserts the split pane explicitly
// rather than only trusting that reading, since split view is exactly
// where NAV-3's own history shows a standalone-only check can miss a real,
// pane-specific regression.
test('NAV-6: reader-prev/reader-next stay pixel-IDENTICAL across every sampled frame of a scripted >=3-viewport-height scroll -- no reposition jitter, split view', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }

  await page.setViewportSize({ width: 1024, height: 800 });
  await page.goto(`/read/${longest.book}/${longest.chapter}?split=1`);
  await expect(page.getByTestId('reader-next')).toBeVisible();

  for (const testId of ['reader-prev', 'reader-next']) {
    await page.evaluate(() => window.scrollTo(0, 0));
    const trace = await page.evaluate(({ testId }) => new Promise<{ x: number; y: number }[]>(resolve => {
      const pts: { x: number; y: number }[] = [];
      const frames = 90;
      const delta = (window.innerHeight * 3) / frames;
      let frame = 0;
      const step = () => {
        const el = document.querySelector(`[data-testid="${testId}"]`) as HTMLElement | null;
        const r = el ? el.getBoundingClientRect() : null;
        if (r) pts.push({ x: r.x, y: r.y });
        if (frame < frames) {
          window.scrollBy(0, delta);
          frame++;
          requestAnimationFrame(step);
        } else {
          resolve(pts);
        }
      };
      requestAnimationFrame(step);
    }), { testId });

    expect(trace.length, `${testId}: expected >=5 samples across the scroll`).toBeGreaterThanOrEqual(5);
    for (const p of trace) {
      expect(p.x, `${testId}.x drifted mid-scroll`).toBeCloseTo(trace[0].x, 0);
      expect(p.y, `${testId}.y drifted mid-scroll`).toBeCloseTo(trace[0].y, 0);
    }
  }
});

// Render-churn guard (batch-hotfix3-brief.md requirement 4: "if suspect (a)
// proved real, assert the nav element is not re-created during scroll").
// Root-causing this batch found suspect (a) -- reader.js's watchScroll ->
// ViewStateService continuous sync triggering a Blazor re-render -- did
// NOT prove real: Reader.razor's own OnScroll has no StateHasChanged (see
// its own comment), and a MutationObserver on the whole nav subtree
// recorded ZERO mutations across every scripted-scroll trace this batch
// ran. This test is kept anyway as cheap, permanent insurance against a
// FUTURE regression reintroducing exactly that (e.g. a later change adding
// StateHasChanged to OnScroll) -- stamps the real DOM node identity before
// a multi-tick scroll and confirms the SAME node, not a same-testid
// replacement, is still there and still connected afterward.
test('NAV-7: reader-prev/reader-next are never re-created by the continuous scroll-position view-state sync', async ({ page }) => {
  const toc = await loadToc();
  let longest = { book: toc[0].code, chapter: 1, verses: toc[0].chapters[0] };
  for (const b of toc) {
    b.chapters.forEach((v, i) => {
      if (v > longest.verses) longest = { book: b.code, chapter: i + 1, verses: v };
    });
  }

  await page.goto(`/read/${longest.book}/${longest.chapter}`);
  const next = page.getByTestId('reader-next');
  const prev = page.getByTestId('reader-prev');
  await expect(next).toBeVisible();
  const nextHandle = await next.elementHandle();
  const prevHandle = await prev.elementHandle();

  // Several distinct scroll ticks, not one jump -- watchScroll's own
  // OnScroll (view-state sync) fires on each, same as a user scrolling in
  // bursts.
  for (let i = 0; i < 5; i++) {
    await page.mouse.wheel(0, 300);
    await page.waitForTimeout(150);
  }

  const nextStillSame = await page.evaluate(el => document.querySelector('[data-testid="reader-next"]') === el, nextHandle);
  const prevStillSame = await page.evaluate(el => document.querySelector('[data-testid="reader-prev"]') === el, prevHandle);
  expect(nextStillSame, 'reader-next was recreated during scroll').toBe(true);
  expect(prevStillSame, 'reader-prev was recreated during scroll').toBe(true);
  expect(await nextHandle!.evaluate(el => el.isConnected)).toBe(true);
  expect(await prevHandle!.evaluate(el => el.isConnected)).toBe(true);
});
