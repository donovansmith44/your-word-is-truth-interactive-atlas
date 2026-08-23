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
  // GATED OFF (2026-08-23, owner order, ledgered): "just disable
  // superscripts until the rework is released." The rendering is gated
  // by FeatureFlags.XrefSuperscripts = false (client/FeatureFlags.cs);
  // these 8 tests skip on the same gate rather than red-fail against a
  // deliberately absent feature. M-D3's rework (click AND hover entry,
  // anchored over the verse, always visible, no auto-modal) flips the
  // flag ON and DELETES this skip -- the suite below remains the binding
  // contract for the re-enabled state (CONTRACT.md XSCRIPT-GATE).
  test.skip(true, 'superscripts gated off by owner order until the M-D3 rework (FeatureFlags.XrefSuperscripts)');

  test('XSCRIPT-1: superscript presence/count is wire-driven, letters for 1-3, many-marker for >3 -- a sample sweep, not hardcoded verses', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0);
    test.skip(!found, 'no sampled chapter had any verse with cross-references');
    if (!found) return;

    const chapterOut = await api.chapter(`${found.book}.${found.chapter}`);
    await page.goto(`/read/${found.book}/${found.chapter}`);

    // Sweep EVERY verse of the discovered chapter (not just the one that
    // matched the predicate) -- a real chapter naturally mixes 0/1-3/>3
    // counts, so one page load exercises every branch of the scheme.
    let sawZero = false, sawLettered = false, sawMany = false;
    for (const v of chapterOut.verses as ChapterVerse[]) {
      const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
      if (v.xref_count === 0) {
        await expect(marker).toHaveCount(0);
        sawZero = true;
        continue;
      }
      await expect(marker).toBeVisible();
      const text = await marker.textContent();
      if (v.xref_count <= 3) {
        expect(text, `verse ${v.verse} has ${v.xref_count} xrefs -- expected the first ${v.xref_count} of i,j,k`).toBe(['i', 'j', 'k'].slice(0, v.xref_count).join(''));
        sawLettered = true;
      } else {
        expect(text, `verse ${v.verse} has ${v.xref_count} (>3) xrefs -- expected the many-marker`).toBe('…');
        sawMany = true;
      }
    }
    // Not a hard requirement (a real chapter might not carry all three
    // shapes) -- logged so a genuinely narrow sample is visible in output,
    // never silently declared "covered" when it wasn't.
    test.info().annotations.push({ type: 'coverage', description: `zero=${sawZero} lettered=${sawLettered} many=${sawMany} in ${found.book}.${found.chapter}` });
  });

  test('XSCRIPT-1: hover opens the SAME composable popover, xrefs section leading, 3 initial entries', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 3);
    test.skip(!found, 'no sampled verse had >3 cross-references (the many-marker case)');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await expect(marker).toHaveText('…');

    // { force: true }: a REAL, live-caught Playwright/production interplay
    // (not a workaround for a real bug -- see this file's own header
    // note): the marker's own mouseenter handler opens a full-viewport
    // `.popover-backdrop` SYNCHRONOUSLY over the marker itself as its
    // direct effect. Playwright's un-forced `.hover()` performs the
    // low-level hover correctly on its FIRST attempt (confirmed live via
    // the actionability log: "performing hover action" precedes the
    // backdrop appearing) but then RETRIES for up to the full test
    // timeout trying to re-verify the target is still cleanly hoverable --
    // which it structurally can never be again once its own reaction
    // covers it. A real human hovering behaves identically to the FIRST
    // attempt (one mouseenter, done) with no such re-verification loop;
    // `force: true` skips Playwright's own post-action interception
    // re-check, matching real hover semantics instead of an automation
    // artifact of an element that (by design) obscures itself.
    await marker.hover({ force: true });
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);

    // Xrefs section leads (BEFORE verse-text, the registry's own normal
    // first slot) -- the entry-point reorder, live. A real, live-caught
    // test-timing bug in an earlier draft: `popover-title` renders
    // SYNCHRONOUSLY (straight off Current.Title) the instant the node is
    // pushed, but the section LIST itself only populates once
    // ExplorerPopover.LoadCurrent's own async section-resolution completes
    // (cleared to empty during the fetch, per that method's own comment) --
    // asserting section order right after the title, with no wait of its
    // own, could snapshot the sections list mid-empty. Waiting for the
    // xrefs section specifically to be VISIBLE (an auto-retrying
    // assertion) guarantees resolution has finished before the order
    // snapshot below.
    await expect(page.getByTestId('popover-section-xrefs')).toBeVisible();
    const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
    expect(sectionIds[0]).toBe('popover-section-xrefs');

    // 3 initial entries, unconditionally (owner's own words) -- NOT F2's
    // context-dependent 2-vs-3 rule, even though this verse may also carry
    // other context sections.
    await expect(page.getByTestId(/^xref-item-/)).toHaveCount(3);
    await expect(page.getByTestId('xrefs-more')).toBeVisible();
  });

  test('XSCRIPT-1: keyboard focus opens the popover identically to hover', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 0 && c <= 3);
    test.skip(!found, 'no sampled verse had 1-3 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    const marker = page.getByTestId(`verse-xref-marker-${v.verse}`);
    await marker.focus();
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(page.getByTestId(/^xref-item-/)).toHaveCount(v.xref_count);
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

  test('XSCRIPT-1: expansion reveals the rest, an entry is explorable one hop, collapse restores', async ({ page }) => {
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 3);
    test.skip(!found, 'no sampled verse had >3 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;

    await page.goto(`/read/${book}/${chapter}`);
    // { force: true }: see "hover opens the SAME composable popover"
    // above for the full reasoning (the marker's own reaction covers it).
    await page.getByTestId(`verse-xref-marker-${v.verse}`).hover({ force: true });
    const items = page.getByTestId(/^xref-item-/);
    await expect(items).toHaveCount(3);

    await page.getByTestId('xrefs-more').click();
    await expect(items).toHaveCount(v.xref_count);
    await expect(page.getByTestId('xrefs-collapse')).toBeVisible();

    // One-hop exploration: clicking a revealed entry PUSHES a fresh
    // VerseNode/PassageNode onto the SAME popover stack (no page
    // navigation -- PassageList.Explore -> IPopoverSectionContext.PushAsync,
    // not a URL change), so the popover's own title changes in place.
    const targetTestId = await items.nth(v.xref_count - 1).getAttribute('data-testid');
    await items.nth(v.xref_count - 1).click();
    await expect(page.getByTestId('popover-title')).not.toHaveText(`${book}.${chapter}.${v.verse}`);
    // The onward node is an ordinary (non-entry-point) popover -- reopening
    // its own xrefs section, if any, would follow F2's cap, not this
    // entry's -- out of this test's own scope, not asserted here.
    test.info().annotations.push({ type: 'note', description: `explored ${targetTestId}` });

    // Close (not browser-back -- there is no navigation to undo) and
    // re-enter via the marker for a fresh entry-point popover, to verify
    // collapse restores the capped view.
    await page.getByTestId('popover-close').click();
    await page.getByTestId(`verse-xref-marker-${v.verse}`).hover({ force: true });
    await page.getByTestId('xrefs-more').click();
    await expect(items).toHaveCount(v.xref_count);
    await page.getByTestId('xrefs-collapse').click();
    await expect(page.getByTestId(/^xref-item-/)).toHaveCount(3);
  });

  test('XSCRIPT-1: entry-point parameter vs F2\'s general popover -- the SAME verse, two different initial caps, one abstraction', async ({ page }) => {
    // The owner's own CAP RECONCILIATION law, proven directly: opening the
    // SAME verse via its ordinary verse-line click still obeys F2's
    // xrefs-only-vs-mixed-context rule (2 when ANY other context section is
    // also present); opening the identical verse via its OWN superscript
    // instead shows 3, unconditionally -- one component, one provider, a
    // parameter read at render time, never two implementations.
    const toc = await loadToc();
    const found = await findVerseByXrefCount(toc, c => c > 2);
    test.skip(!found, 'no sampled verse had >2 cross-references');
    if (!found) return;
    const { book, chapter, verse: v } = found;
    const detail = await api.verse(`${book}.${chapter}.${v.verse}`);
    // OtherContextSectionCount (client, ExplorerPopover.razor) counts EVERY
    // resolved section except verse-text/xrefs -- for a Verse node the ONLY
    // two OTHER providers PopoverSectionRegistry registers are
    // CatechismSeamSection and VerseEventMembershipSection (confirmed by
    // reading that registry's own Providers list), so EITHER catechism OR
    // event-membership presence makes the general popover cap at 2. A real,
    // live-caught test bug in an earlier draft checked catechism alone and
    // failed on a real verse that had zero catechism citations but DID
    // touch a titled event (the far more common case, since most narrative-
    // covered verses have no catechism link at all).
    const hasOtherContext = detail.catechism.length > 0 || detail.events.length > 0;
    const generalExpected = Math.min(v.xref_count, hasOtherContext ? 2 : 3);

    await page.goto(`/read/${book}/${chapter}`);
    await page.getByTestId(`verse-line-${v.verse}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(page.getByTestId(/^xref-item-/)).toHaveCount(generalExpected);
    await page.getByTestId('popover-close').click();
    // Wait for the GENERAL popover to fully close (RequestClose is async --
    // a JS interop call precedes _activeNode=null) before opening the
    // entry-point one -- a real, live-caught race: re-hovering immediately
    // could fire while the prior popover's own close is still in flight.
    await expect(page.getByTestId('popover')).toHaveCount(0);

    // { force: true }: see "hover opens the SAME composable popover" above.
    await page.getByTestId(`verse-xref-marker-${v.verse}`).hover({ force: true });
    await expect(page.getByTestId('popover-title')).toHaveText(`${book}.${chapter}.${v.verse}`);
    await expect(page.getByTestId(/^xref-item-/)).toHaveCount(Math.min(v.xref_count, 3));
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
