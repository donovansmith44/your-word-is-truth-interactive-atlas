import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc } from './lib/canon';

// Batch M-D2 -- the owner's cross-reference superscript directive
// (batch-x-brief.md, 2026-08-21, verbatim: "if particular sections or
// verses have cross references, we need to do the thing where we have
// little superscripts visible near verses/passages to which cross
// references apply. `i,j,k` to represent multiple cross references to a
// single element if there are > 3 xrefs, the superscript should be `...`
// ... and if you hover over it you get another explorable,
// collapsable/expandable hover menu that shows 3 explorable verses to
// start"), closed on the graph platform. See CONTRACT.md's own XSCRIPT-1
// note for the full lettering scheme + entry-point parameter design.
//
// { force: true } on every hover()/click() that opens the popover VIA a
// marker (never on interactions with an ALREADY-open popover's own
// content): a real, live-caught Playwright/production interplay, not a
// workaround for a bug. The marker's own mouseenter/click handler opens a
// full-viewport `.popover-backdrop` as its DIRECT, synchronous effect,
// covering the marker itself. Un-forced hover()/click() perform the
// low-level action correctly on their first attempt (confirmed via
// Playwright's own actionability log -- "performing hover action" precedes
// the backdrop appearing) but then retry for the full test timeout trying
// to re-verify the target is STILL cleanly actionable, which it
// structurally never can be again once its own reaction covers it. A real
// human hovering/clicking behaves exactly like that first attempt (one
// event, done, no re-verification loop) -- force: true matches that real
// semantics rather than an automation artifact of a self-obscuring target.

type ChapterVerse = { verse: number; text: string; xref_count: number };

const PAGE = 20;
const CITES_LINKS = '[data-testid^="popover-link-cites-"]';

// M-D4 fix round 2, P1b (owner, verbatim: "well there might be more than
// 26. if there are then we still need to be in mod26 land but have a
// system for new superscripts beyond a-z"): mirrors
// VerseLine.BijectiveBase26 (client/Components/VerseLine.razor)
// independently -- bijective base-26 numeration, the same scheme
// spreadsheet column names use (A, B, ..., Z, AA, AB, ..., AZ, BA, ...,
// ZZ, AAA, ...). 1-based `n`; NOT plain modulo-26, which fix round 1
// used and which collides ordinal 27's own letter back onto ordinal 1's
// ("a" again) -- bijective numeration never collides, since there is no
// digit for zero.
function bijectiveBase26(n: number): string {
  let letters = '';
  while (n > 0) {
    n -= 1;
    letters = String.fromCharCode('a'.charCodeAt(0) + (n % 26)) + letters;
    n = Math.floor(n / 26);
  }
  return letters;
}

// Scans real chapters (never the demo fixture, never a hardcoded book/
// chapter) for one whose own /api/chapter response carries a verse
// satisfying `predicate` on its own xref_count -- same "read real data,
// don't hardcode" discipline popover-sections.spec.ts's own
// findVerseWithCounts already establishes, specialized to the CHAPTER
// endpoint (xref_count lives on VerseOut, this feature's own actual wire
// source -- not VerseDetail.cross_refs.length, a different endpoint this
// feature does not read).
async function findVerseByXrefCount(
  toc: any,
  predicate: (count: number) => boolean,
  maxChapters = 60,
): Promise<{ book: string; chapter: number; verse: ChapterVerse } | null> {
  const books = fc.sample(fc.constantFrom(...toc), Math.min(maxChapters, toc.length));
  for (const b of books) {
    for (const ch of b.chapters.slice(0, 3)) {
      const chapterOut = await api.chapter(`${b.code}.${ch}`);
      const v = (chapterOut.verses as ChapterVerse[]).find(v => predicate(v.xref_count));
      if (v) {
        return { book: b.code, chapter: ch, verse: v };
      }
    }
  }
  return null;
}

test.describe('M-D2: cross-reference superscripts', () => {
  // UN-GATED (M-D3, R3, this batch): the rework promised when the flag
  // went off has landed -- FeatureFlags.XrefSuperscripts is true again
  // (client/FeatureFlags.cs), and the popover now anchors OVER THE VERSE
  // (ExplorerPopover.razor's own VerseAnchorSelector/VerseAnchorStyle,
  // clamped to stay fully on-screen) instead of pane/viewport-centered, and
  // a HOVER-only open (never a click or keyboard focus) auto-dismisses a
  // short grace period after the pointer leaves both the marker and the
  // popover (Reader.razor's own OpenVerseXrefEntryHover/CancelHoverClose) --
  // "no auto-modal that must be X'd out," the owner's own named bug. See
  // reader-xref-anchoring.spec.ts for the new anchoring/viewport-edge/
  // hover-dismiss coverage; the suite below is unchanged from M-D2 and
  // remains the binding contract for click/keyboard-focus entry, the
  // lettering scheme, expansion, and the cap reconciliation.

  // M-D4 fix round 1, P1 (owner CORRECTION, 2026-08-23, verbatim: "i didnt
  // literally mean i,j,k superscripts. i mean ordered alphabetical letters
  // that modulate around a chapter (i.e., first letter is a)"). RETIRES the
  // old per-verse-COUNT scheme (i/ij/ijk + the "…" many-marker) whole: the
  // marker's own glyph no longer depends on the VERSE's OWN xref_count at
  // all (beyond ">0, so a marker renders"), only on its ORDINAL among the
  // chapter's xref-bearing verses, in reading order -- "a" for the first.
  // No count signal at the marker at all (count lives inside the popover,
  // already served). M-D4 fix round 2, P1b (owner: "well there might be
  // more than 26... have a system for new superscripts beyond a-z"):
  // ordinals beyond 26 now use BIJECTIVE base-26 (bijectiveBase26 above),
  // not a bare modulo-26 wraparound -- see that function's own comment.
  test('XSCRIPT-1: superscript lettering is the verse\'s own ordinal among the chapter\'s xref-bearing verses -- "a" first, bijective base-26 beyond "z" -- a sample sweep, not hardcoded verses', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled chapter had any verse with cross-references');
    if (!found) return;

    const chapterOut = await api.chapter(`${found.book}.${found.chapter}`);
    await page.goto(`/read/${found.book}/${found.chapter}`);

    // Mirrors VerseLine.ComputeXrefLetters exactly (client/Components/
    // VerseLine.razor) -- computed independently here, in reading order
    // over the SAME chapter response, rather than read back off the DOM,
    // so this test actually PROVES the scheme instead of just echoing
    // whatever the client happened to render.
    let ordinal = 0;
    let sawZero = false, sawLettered = false, sawMultiLetter = false;
    for (const v of chapterOut.verses as ChapterVerse[]) {
      const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
      if (v.xref_count === 0) {
        await expect(marker).toHaveCount(0);
        sawZero = true;
        continue;
      }
      ordinal++;
      const expectedLetter = bijectiveBase26(ordinal);
      if (expectedLetter.length > 1) sawMultiLetter = true;
      await expect(marker, `verse ${v.verse} is xref-bearing verse #${ordinal} in ${found.book}.${found.chapter} -- expected letter "${expectedLetter}"`).toHaveText(expectedLetter);
      // P1: no count signal at the marker at all -- the accessible name
      // drops it too (VerseLine.razor's own XrefMarkerAriaLabel), not just
      // the visible glyph.
      await expect(marker).toHaveAttribute('aria-label', `Cross-references for verse ${v.verse}`);
      sawLettered = true;
    }
    // Not a hard requirement (a real chapter might not carry every shape,
    // and most chapters have far fewer than 26 xref-bearing verses) --
    // logged so a genuinely narrow sample is visible in output, never
    // silently declared "covered" when it wasn't.
    test.info().annotations.push({ type: 'coverage', description: `zero=${sawZero} lettered=${sawLettered} totalXrefBearing=${ordinal} multiLetterObserved=${sawMultiLetter} in ${found.book}.${found.chapter}` });
  });

  // A dedicated, best-effort search for a chapter with enough xref-bearing
  // verses to directly WITNESS the single-letter -> two-letter transition
  // (ordinal 26 -> "z", ordinal 27 -> "aa") -- the sweep test above only
  // exercises this when it happens to sample a long enough chapter; this
  // test proves the transition deterministically when the compiled
  // dataset has one, and gracefully skips (never fails) when it doesn't.
  // M-D4 fix round 2, P1b (owner: "well there might be more than 26...
  // have a system for new superscripts beyond a-z"): REPLACES fix round
  // 1's own "wraps from z back to a (modulo 26)" test -- that assertion
  // is now the exact BUG P1b retires (a same-letter collision between
  // ordinal 1 and ordinal 27); the 27th xref-bearing verse gets "aa" now,
  // never "a" again.
  test('XSCRIPT-1: lettering continues past "z" as "aa" (bijective base-26), never collides back onto "a", in a chapter with >26 xref-bearing verses', async ({ page }) => {
    const toc = await loadToc();
    let target: { book: string; chapter: number; zVerse: number; aaVerse: number } | null = null;
    outer: for (const b of toc) {
      for (const ch of b.chapters) {
        const chapterOut = await api.chapter(`${b.code}.${ch}`);
        const xrefBearing = (chapterOut.verses as ChapterVerse[]).filter(v => v.xref_count > 0);
        if (xrefBearing.length > 26) {
          target = { book: b.code, chapter: ch, zVerse: xrefBearing[25].verse, aaVerse: xrefBearing[26].verse };
          break outer;
        }
      }
    }
    test.skip(!target, 'no chapter in this compiled dataset has more than 26 xref-bearing verses');
    if (!target) return;

    await page.goto(`/read/${target.book}/${target.chapter}`);
    // Ordinal 26 -- the last single-letter value.
    await expect(page.getByTestId(`verse-xref-marker-${target.zVerse}`)).toHaveText('z');
    // Ordinal 27 -- the first two-letter value, the EXACT point fix round
    // 1's bare `ordinal % 26` collided back onto "a" instead.
    await expect(page.getByTestId(`verse-xref-marker-${target.aaVerse}`)).toHaveText('aa');
  });

  test('XSCRIPT-1: hover opens the verse\'s popover, whose Cites group counts every served cross reference and shows the first page of them', async ({ page }) => {
    // Arrange
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 3);
    test.skip(!found, 'no sampled verse had >3 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;
    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await expect(marker).toHaveText(/^[a-z]+$/);

    // Act
    await marker.hover({ force: true });

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(page.getByTestId('popover-section-cites-heading')).toHaveText(`Cites (${v.xref_count})`);
    await expect(page.locator(CITES_LINKS)).toHaveCount(Math.min(v.xref_count, PAGE));
  });

  test('XSCRIPT-1: keyboard focus opens the popover identically to hover', async ({ page }) => {
    // Arrange
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0 && c <= 3);
    test.skip(!found, 'no sampled verse had 1-3 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;
    await page.goto(`/read/${book}/${chapter}`);

    // Act
    await page.getByTestId(`verse-xref-marker-${v.verse}`).focus();

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(page.locator(CITES_LINKS)).toHaveCount(v.xref_count);
  });

  test('XSCRIPT-1: click also opens the popover (touch-device fallback, no hover state)', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await expect(marker).toBeVisible();
    // { force: true }: same self-obscuring-target interplay as the hover
    // test above (the click's own effect -- opening the backdrop -- covers
    // the marker it was clicked on) -- see that test's own comment for the
    // full reasoning.
    await marker.click({ force: true });
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    // The click never ALSO opens the plain verse-line's own popover on top
    // of / instead of this one (stopPropagation) -- exactly one popover.
    await expect(page.getByTestId('popover')).toHaveCount(1);
  });

  test('XSCRIPT-1: More reveals the next page, an entry is explorable one hop, Less restores the first page', async ({ page }) => {
    // Arrange
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > PAGE);
    test.skip(!found, `no sampled verse had more than ${PAGE} cross-references`);
    if (!found) return;
    const { book, chapter, verse: v } = found;
    const shown = Math.min(v.xref_count, 2 * PAGE);
    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).click({ force: true });
    const links = page.locator(CITES_LINKS);
    await expect(links).toHaveCount(PAGE);

    // Act
    await page.getByTestId('popover-section-cites-more').click();

    // Assert
    await expect(links).toHaveCount(shown);
    await expect(page.getByTestId('popover-section-cites-position')).toHaveText(`1–${shown} of ${v.xref_count}`);

    // Act
    await page.getByTestId('popover-section-cites-collapse').click();

    // Assert
    await expect(links).toHaveCount(PAGE);

    // Act
    const last = links.nth(PAGE - 1);
    const target = await last.textContent();
    await last.click();

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(target!);
  });

  test('XSCRIPT-1: the marker and the verse line open the SAME popover -- one Cites group, the same first page, in served order', async ({ page }) => {
    // Arrange
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 2);
    test.skip(!found, 'no sampled verse had >2 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;
    await page.goto(`/read/${book}/${chapter}`);
    const links = page.locator(CITES_LINKS);
    const linkIds = () => links.evaluateAll(els => els.map(el => el.getAttribute('data-testid')));

    // Act
    await page.getByTestId(`verse-line-${v.verse}`).focus();
    await page.keyboard.press('Enter');
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(links).toHaveCount(Math.min(v.xref_count, PAGE));
    const fromLine = await linkIds();
    await page.getByTestId('popover-close').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);
    await page.getByTestId(`verse-xref-marker-${v.verse}`).click({ force: true });
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(links).toHaveCount(Math.min(v.xref_count, PAGE));

    // Assert
    expect(await linkIds()).toEqual(fromLine);
  });

  test('JANK-1: a verse with a superscript renders the SAME line-height as a verse with none -- no layout jank', async ({ page }) => {
    const toc = await loadToc();
    // Finds one chapter carrying BOTH a marker-bearing and a marker-free
    // verse (common -- most real chapters mix cited and uncited verses).
    const books = fc.sample(fc.constantFrom(...toc), Math.min(60, toc.length));
    let target: { book: string; chapter: number; withMarker: ChapterVerse; withoutMarker: ChapterVerse } | null = null;
    outer: for (const b of books) {
      for (const ch of b.chapters.slice(0, 3)) {
        const chapterOut = await api.chapter(`${b.code}.${ch}`);
        const verses = chapterOut.verses as ChapterVerse[];
        const withMarker = verses.find(v => v.xref_count > 0);
        const withoutMarker = verses.find(v => v.xref_count === 0);
        if (withMarker && withoutMarker) {
          target = { book: b.code, chapter: ch, withMarker, withoutMarker };
          break outer;
        }
      }
    }
    test.skip(!target, 'no sampled chapter mixed a cross-referenced and a non-cross-referenced verse');
    if (!target) return;

    await page.goto(`/read/${target.book}/${target.chapter}`);

    // Mechanism-level, content-length-independent: the SAME computed
    // line-height applies to .verse-text whether or not its own sibling
    // marker is present -- the marker's own `vertical-align: super` +
    // `line-height: 1` (app.css) never inflates the FLOW line box, by
    // construction (see that rule's own comment), verified live here, not
    // merely asserted in CSS.
    const withMarkerLineHeight = await page.getByTestId(`verse-line-${target.withMarker.verse}`).locator('.verse-text').evaluate(el => getComputedStyle(el).lineHeight);
    const withoutMarkerLineHeight = await page.getByTestId(`verse-line-${target.withoutMarker.verse}`).locator('.verse-text').evaluate(el => getComputedStyle(el).lineHeight);
    expect(withMarkerLineHeight).toBe(withoutMarkerLineHeight);

    // Real-world proxy, per the brief's own explicit ask ("verse
    // boundingBox stability... test it"): for two SHORT verses (likely
    // single-line at this viewport), the rendered .verse-line height
    // itself matches too -- skipped gracefully (not failed) if neither
    // candidate is short enough to trust as single-line, rather than
    // asserting on a wrapped multi-line verse where content length, not
    // jank, would explain a height difference.
    const shortEnough = (v: ChapterVerse) => v.text.length <= 70;
    if (shortEnough(target.withMarker) && shortEnough(target.withoutMarker)) {
      const withMarkerBox = await page.getByTestId(`verse-line-${target.withMarker.verse}`).boundingBox();
      const withoutMarkerBox = await page.getByTestId(`verse-line-${target.withoutMarker.verse}`).boundingBox();
      expect(withMarkerBox && withoutMarkerBox && withMarkerBox.height).toBe(withoutMarkerBox!.height);
    } else {
      test.info().annotations.push({ type: 'skip-reason', description: 'neither candidate verse was short enough to trust as single-line; mechanism-level line-height assertion above still ran' });
    }
  });

  test('JANK-1: reduced motion -- the marker introduces no transition/animation', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled verse had any cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await expect(marker).toBeVisible();
    const transition = await marker.evaluate(el => getComputedStyle(el).transitionDuration);
    // "0s" (or an all-zero list) -- no rule in app.css declares a
    // transition on .verse-xref-marker at all (by construction, per that
    // rule's own comment), so this holds identically with or without the
    // reduced-motion emulation above; asserted under reduced-motion
    // specifically per the brief's own explicit requirement.
    expect(transition.split(',').every(d => parseFloat(d) === 0)).toBeTruthy();
  });
});
