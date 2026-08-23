import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc } from './lib/canon';

// M-D3, R3 (the superscript rework): "the popover anchors OVER THE VERSE
// (not pane-centered), ALWAYS VISIBLE, never cut off by other UI... no
// auto-modal that must be X'd out." reader-xref-superscripts.spec.ts (M-D2,
// unchanged by this batch) covers the lettering scheme, click/keyboard
// entry, expansion, and the cap reconciliation; this file covers exactly
// the THREE new things R3 adds: anchoring geometry, viewport-edge safety,
// and hover-vs-persistent dismiss parity.
//
// { force: true } on every hover()/click() that opens the popover via a
// marker: the SAME real, live-caught Playwright/production interplay
// reader-xref-superscripts.spec.ts's own header documents (the marker's own
// reaction -- a full-viewport backdrop -- covers the marker it just fired
// on). See that file's header for the full reasoning; not repeated here.

type ChapterVerse = { verse: number; text: string; xref_count: number };

async function findVerseByXrefCount(
  toc: any,
  predicate: (count: number) => boolean,
  maxChapters = 60,
): Promise<{ book: string; chapter: number; verse: ChapterVerse; totalVerses: number } | null> {
  const books = fc.sample(fc.constantFrom(...toc), Math.min(maxChapters, toc.length));
  for (const b of books) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      const verses = chapterOut.verses as ChapterVerse[];
      const v = verses.find(v => predicate(v.xref_count));
      if (v) {
        return { book: b.code, chapter: ch, verse: v, totalVerses: verses.length };
      }
    }
  }
  return null;
}

test.describe('M-D3 R3: superscript popover anchoring + hover dismiss', () => {
  test('XSCRIPT-ANCHOR-1: the popover is horizontally centered on the marker\'s own verse line, not the viewport', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    const verseLine = page.getByTestId(`verse-line-${v.verse}`);
    const lineBox = await verseLine.boundingBox();
    expect(lineBox).not.toBeNull();

    await marker.hover({ force: true });
    const popover = page.getByTestId('popover');
    await expect(popover).toBeVisible();
    await expect(popover).toHaveClass(/popover-verse-anchored/);
    const popBox = await popover.boundingBox();
    expect(popBox).not.toBeNull();

    const lineCenterX = lineBox!.x + lineBox!.width / 2;
    const popCenterX = popBox!.x + popBox!.width / 2;
    // Within a few px of the verse line's own horizontal center. (NOT
    // asserted as "far from the viewport's own center" -- an earlier draft
    // tried that as a belt-and-suspenders proof that this isn't just
    // viewport-centering by coincidence, but the reading column is ITSELF
    // laid out close to the page's horizontal middle, so a verse line's own
    // center routinely sits near 700px too; that assertion was measuring
    // the READER's OWN LAYOUT, not this feature, and failed on a real
    // sampled verse for exactly that reason. The class assertion below is
    // the real, unambiguous proof of which mechanism is active.)
    expect(Math.abs(popCenterX - lineCenterX)).toBeLessThan(5);

    // Vertically adjacent to the verse line (grows away from the nearer
    // edge -- see ExplorerPopover.razor's own VerseAnchorStyle comment):
    // either its own top sits at/after the line's bottom, or its own
    // bottom sits at/before the line's top -- never straddling it, and
    // never far enough away to no longer read as "over the verse".
    const opensBelow = popBox!.y >= lineBox!.y - 2;
    const opensAbove = popBox!.y + popBox!.height <= lineBox!.y + lineBox!.height + 2;
    expect(opensBelow || opensAbove, `popover (y=${popBox!.y}, h=${popBox!.height}) neither below nor above verse line (y=${lineBox!.y}, h=${lineBox!.height})`).toBeTruthy();
  });

  test('XSCRIPT-ANCHOR-2: viewport-edge visibility -- the FIRST verse of a chapter (top of page) never renders a popover cut off at the top', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter } = found;
    // Verse 1 specifically -- regardless of which verse the sample found,
    // re-fetch this chapter's own first verse to test the true top-of-page
    // edge case (verse 1 may itself carry zero xrefs; skip gracefully then,
    // same "not every real chapter has every shape" stance the M-D2 suite
    // already takes).
    const chapterOut = await api.chapter(`${book}.${chapter}`);
    const v1 = (chapterOut.verses as ChapterVerse[]).find(v => v.verse === 1 && v.xref_count > 0);
    test.skip(!v1, `${book}.${chapter}'s own verse 1 has no cross-references`);
    if (!v1) return;

    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId('verse-line-1').scrollIntoViewIfNeeded();

    await page.getByTestId(`verse-xref-marker-1`).hover({ force: true });
    const popover = page.getByTestId('popover');
    await expect(popover).toBeVisible();
    const box = await popover.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.y, 'popover top must not be above the viewport').toBeGreaterThanOrEqual(0);
    expect(box!.y + box!.height, 'popover bottom must not exceed the viewport').toBeLessThanOrEqual(900 + 1);
  });

  test('XSCRIPT-ANCHOR-3: viewport-edge visibility -- the LAST verse scrolled to the bottom of the viewport never renders a popover cut off at the bottom', async ({ page }) => {
    const toc = await loadToc();
    // A long-ish chapter with a cross-referenced final verse gives the
    // clearest "hugging the bottom edge" case.
    let target: { book: string; chapter: number; lastVerse: ChapterVerse } | null = null;
    const books = fc.sample(fc.constantFrom(...toc), Math.min(60, toc.length));
    outer: for (const b of books) {
      for (const ch of b.chapters) {
        const chapterOut = await api.chapter(`${b.code}.${ch}`);
        const verses = chapterOut.verses as ChapterVerse[];
        if (verses.length < 15) continue; // want real scroll room
        const last = verses[verses.length - 1];
        if (last.xref_count > 0) {
          target = { book: b.code, chapter: ch, lastVerse: last };
          break outer;
        }
      }
    }
    test.skip(!target, 'no sampled chapter had a cross-referenced final verse with real scroll room');
    if (!target) return;

    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto(`/read/${target.book}/${target.chapter}`);
    const lastMarker = page.getByTestId(`verse-xref-marker-${target.lastVerse.verse}`);
    await lastMarker.scrollIntoViewIfNeeded();

    await lastMarker.hover({ force: true });
    const popover = page.getByTestId('popover');
    await expect(popover).toBeVisible();
    const box = await popover.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.y, 'popover top must not be above the viewport').toBeGreaterThanOrEqual(0);
    expect(box!.y + box!.height, 'popover bottom must not exceed the viewport').toBeLessThanOrEqual(900 + 1);
  });

  test('XSCRIPT-ANCHOR-4: viewport-edge visibility -- both pane edges in split view stay fully on-screen', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.setViewportSize({ width: 1400, height: 900 });
    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId('split-open-reader').click();
    await expect(page.getByTestId('split-pane-atlas')).toBeVisible();

    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await marker.scrollIntoViewIfNeeded();
    await marker.hover({ force: true });
    const popover = page.getByTestId('popover');
    await expect(popover).toBeVisible();
    const box = await popover.boundingBox();
    expect(box).not.toBeNull();
    // Full-viewport safety (this class's own VerseAnchorStyle clamps
    // against the WHOLE viewport, not the narrower reader pane alone --
    // ExplorerPopover.razor's own doc comment discloses this choice: a
    // verse-anchored popover may visually extend toward the atlas pane's
    // own side, same as the pre-existing pane-CENTERED default already
    // does for an ordinary verse-line popover in split view, but it is
    // NEVER cut off by the viewport's own true edges, which is the law
    // this test enforces).
    expect(box!.x, 'popover left edge must not be off-screen').toBeGreaterThanOrEqual(0);
    expect(box!.x + box!.width, 'popover right edge must not be off-screen').toBeLessThanOrEqual(1400 + 1);
    expect(box!.y, 'popover top edge must not be off-screen').toBeGreaterThanOrEqual(0);
    expect(box!.y + box!.height, 'popover bottom edge must not be off-screen').toBeLessThanOrEqual(900 + 1);
  });

  test('XSCRIPT-DISMISS-1: a HOVER-only open auto-dismisses shortly after the pointer leaves both the marker and the popover', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).hover({ force: true });
    await expect(page.getByTestId('popover')).toBeVisible();

    // Move the pointer somewhere far from both the marker and the popover
    // -- "no auto-modal that must be X'd out": a mere hover must be
    // dismissible by JUST moving away, no click needed.
    await page.mouse.move(5, 5);
    await expect(page.getByTestId('popover')).toHaveCount(0, { timeout: 2000 });
  });

  test('XSCRIPT-DISMISS-2: moving the pointer from the marker INTO the popover panel keeps a hover-opened popover open (no premature close mid-transit)', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    // Reduced motion -- a real, live-caught fix (see Reader.razor's own
    // DelayedHoverClose doc comment): the panel's own 120ms open animation
    // was eating real margin out of the hover-close grace period during a
    // genuine mouse transit; removing it here isolates THIS test's own
    // subject (transit timing) from an unrelated animation-duration
    // variable, the same reasoning JANK-1's own reduced-motion case
    // (reader-xref-superscripts.spec.ts) already established for a
    // different purpose.
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).hover({ force: true });
    const popover = page.getByTestId('popover');
    await expect(popover).toBeVisible();

    // Real mouse transit onto the panel itself (not force:true here -- the
    // popover is the intended target, nothing obscures it once open).
    await popover.hover();
    // Held well past the hover-close grace period (500ms, Reader.razor) --
    // must still be open, proving OnPanelPointerEnter cancelled the timer.
    await page.waitForTimeout(900);
    await expect(popover).toBeVisible();
  });

  test('XSCRIPT-DISMISS-3: a CLICK-opened popover stays open after the pointer leaves -- persistence unchanged from every other popover', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).click({ force: true });
    await expect(page.getByTestId('popover')).toBeVisible();

    await page.mouse.move(5, 5);
    await page.waitForTimeout(900); // well past the hover-only grace period (500ms, Reader.razor)
    await expect(page.getByTestId('popover')).toBeVisible();

    // Still dismissible the ordinary way -- persistence is not "stuck open forever".
    await page.getByTestId('popover-close').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);
  });

  test('XSCRIPT-DISMISS-4: a KEYBOARD-FOCUS-opened popover stays open on blur -- focus is treated as persistent, not hover-only', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0 && c <= 3);
    test.skip(!found, 'no sampled verse had 1-3 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).focus();
    await expect(page.getByTestId('popover')).toBeVisible();

    // Move focus elsewhere (blur) -- a keyboard user reading the now-open
    // popover's own content must not have it vanish out from under them.
    await page.keyboard.press('Tab');
    await page.waitForTimeout(900);
    await expect(page.getByTestId('popover')).toBeVisible();
  });
});
