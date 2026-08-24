import { test, expect, Page } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef } from './lib/canon';

// Batch R requirement 3 ("the popover becomes a content-first section
// platform") + requirement 4 (expandable popover / in-context chapter
// reading) + requirement 5 (place-in-verse hover -> marker blink), all
// exercised through the real, live popover -- see CONTRACT.md's own
// REGISTRY-1/READER-1/BLINK-1 notes for the exact behavior each test below
// pins.

// Searches up to `maxTries` random real verses for one whose own
// GET /api/verse/{vref} response satisfies `predicate` -- deterministic
// discovery against the real compiled dataset (not the demo fixture),
// same "read real data, don't hardcode" spirit lib/hoverSafety.ts's own
// independentlyHoverableIds already follows. Returns null (caller skips)
// if none of the samples match -- honest about a low-probability miss
// rather than flaking.
async function findVerse(toc: any, predicate: (detail: any) => boolean, maxTries = 60): Promise<{ vref: string; detail: any } | null> {
  const samples = fc.sample(arbVerseRef(toc), maxTries);
  for (const vref of samples) {
    const detail = await api.verse(vref);
    if (predicate(detail)) {
      return { vref, detail };
    }
  }
  return null;
}

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

// ---------------------------------------------------------------------
// REGISTRY-1: VERSE node sections, in order, conditional presence
// ---------------------------------------------------------------------

test('REGISTRY-1: a verse with real cross-references shows them inline, no button press', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerse(toc, d => d.cross_refs.length > 0);
  test.skip(!found, 'no sampled verse had cross-references');
  if (!found) return;
  const { vref, detail } = found;
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(vref);
  // Settle-wait: `popover-title` binds to Current.Title, set SYNCHRONOUSLY
  // (the click handler itself) the instant a node is pushed -- BEFORE
  // LoadCurrent's own async section-provider fetches even start
  // (ExplorerPopover.razor's own LoadCurrent doc comment: every section
  // provider resolves together, one Task.WhenAll batch, ONE _sections
  // assignment at the end). A real, live-caught race (found by this exact
  // fix's own first draft failing on JDG.2.8-9's real cross-references,
  // reader-map.spec.ts's READ-6): querying popover-section-* right after
  // popover-title can read the DOM before that batch has landed.
  // popover-section-verse-text is UNCONDITIONALLY present for any
  // Verse/Passage node (the "two firm anchors" reasoning right below) and
  // renders in the SAME batch -- waiting for it is a direct, retrying
  // proxy for "the whole batch landed," not a fixed sleep.
  await expect(page.getByTestId('popover-section-verse-text')).toBeVisible();

  // M-D3/U6, owner verbatim order: "Header / Verse (focus) / Event /
  // Parallels / Small Catechism / cross references LAST." Verse-text
  // FIRST and xrefs LAST are the only two positions this predicate (only
  // requires >0 xrefs) can pin unconditionally -- Event/Parallels/Persons/
  // Catechism in between are each independently conditional, so this
  // checks the two firm anchors structurally rather than assuming which
  // (if any) of the middle four also showed for this sampled verse.
  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(sectionIds[0]).toBe('popover-section-verse-text');
  expect(sectionIds[sectionIds.length - 1]).toBe('popover-section-xrefs');

  // Batch F2 requirement 6 (XREF-1): capped at 3 (xrefs-only) or 2 (ANY
  // other context section also present -- ExplorerPopover.razor's own
  // OtherContextSectionCount, `_sections.Count(s => s.Testid is not
  // ("verse-text" or "xrefs"))`, generically counts every OTHER resolved
  // section, catechism/persons/event/parallels alike, "any future provider
  // automatically" per CrossRefsSection's own doc comment).
  // M-D3/U5 fix (real, consistently-reproducing failure, not the rare
  // sampling-luck flake this comment used to describe): this test's own
  // cap formula only ever checked for catechism specifically, hand-
  // enumerated -- going stale the moment U6 added a sibling PERSONS
  // section that resolves for a large, not rare, fraction of real verses.
  // Rewritten to mirror OtherContextSectionCount's own generic rule
  // exactly (any section besides the two firm anchors), so a FUTURE new
  // section type can never repeat this exact drift again.
  const hasOtherContext = sectionIds.some(id => id !== 'popover-section-verse-text' && id !== 'popover-section-xrefs');
  const cap = hasOtherContext ? 2 : 3;
  const expectedInitial = Math.min(detail.cross_refs.length, cap);
  await expect(page.getByTestId(/^xref-item-/)).toHaveCount(expectedInitial);
  await expect(page.getByTestId(`xref-item-${detail.cross_refs[0].target}`)).toBeVisible();
  // No leftover toggle chip -- retired by this batch.
  await expect(page.getByTestId('popover-chip-xrefs')).toHaveCount(0);
});

test('REGISTRY-1: a verse with zero cross-references shows no xrefs section at all (conditional presence)', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerse(toc, d => d.cross_refs.length === 0);
  test.skip(!found, 'no sampled verse had zero cross-references');
  if (!found) return;
  const { vref } = found;
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(vref);

  await expect(page.getByTestId('popover-section-verse-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-xrefs')).toHaveCount(0);
  await expect(page.getByTestId(/^xref-item-/)).toHaveCount(0);
});

// CATECH-1 (batch-f-brief.md, "the small catechism"): a verse with zero
// catechism citations shows no THIRD section -- the general conditional-
// presence case; see CONTRACT.md's own CATECH-1 note. Batch F2 grew
// coverage substantially (repo mapping + Deut5 supplement -- ~4800
// distinct verses now link in, batch-f2-report.md's own coverage figure),
// so a hardcoded "known-uncited" verse (Batch F's own GEN.1.1) is no
// longer a safe assumption -- discovered dynamically instead, same
// approach findVerse already uses for the sibling xrefs test above.
test('CATECH-1: a verse with zero catechism citations shows no catechism section', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerse(toc, d => d.catechism.length === 0);
  test.skip(!found, 'no sampled verse had zero catechism citations');
  if (!found) return;
  const { vref } = found;
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  // M-D3/U5, a real, live-caught regression: a plain coordinate .click() on
  // the verse-line now risks landing on one of ITS OWN in-text mentions
  // (Reader.razor's new @onclick:stopPropagation spans, PlaceMentions.Scan)
  // instead of the line itself -- Playwright clicks an element's own
  // geometric center, and a sampled verse's attested mention can happen to
  // sit right there (caught live: DEU.5.26's own "God" mention opened a
  // PersonNode instead of this test's own expected VerseNode). Keyboard
  // activation (.focus() + Enter, OnVerseLineKeyDown) sidesteps coordinates
  // entirely -- already this codebase's own established alternative to a
  // coordinate click (world-border-morph.spec.ts's own precedent).
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText(vref);
  await expect(page.getByTestId('popover-section-catechism')).toHaveCount(0);
  await expect(page.getByTestId(/^catechism-item-/)).toHaveCount(0);
});

// ---------------------------------------------------------------------
// READER-1: expand -> lazy chapter fetch -> scrollable mini-reader ->
// focal verse visible + highlighted; collapse restores the compact view.
// ---------------------------------------------------------------------

test('READER-1: expanding a verse popover fetches the whole chapter and highlights the focal verse', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-line-3').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.3');

  // M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  // when already reading that chapter." A verse reached by clicking a
  // verse-line in the reader is, by construction, always FROM the chapter
  // currently on screen -- popover-verse-expand is correctly ABSENT here
  // now, every time, structurally (there is no way to click a verse-line
  // for a chapter the reader isn't already showing). Exercising the
  // underlying mechanism (auto-fetch, focal highlight) now requires a
  // node the reader genuinely is NOT already displaying -- explore a real,
  // stable, heavily-cited curated cross-reference (GEN.1.3 -> 2CO.4.6,
  // the top-voted entry, votes=81) to reach one, the same "push a fresh
  // VerseNode for a different book onto the SAME popover stack" every
  // other one-hop-exploration test in this file already exercises.
  await expect(page.getByTestId('popover-verse-expand')).toHaveCount(0);
  await expect(page.getByTestId('xref-item-2CO.4.6')).toBeVisible();
  await page.getByTestId('xref-item-2CO.4.6').click();
  await expect(page.getByTestId('popover-title')).toHaveText('2CO.4.6');

  // Compact view first -- no mini-reader yet (requirement 4: "fetch on
  // expand, not before"). O2 (owner live-preview correction, 2026-08-23)
  // retired the old single-button's own `aria-expanded` toggle along with
  // the button itself -- MiniReaderExpand's own trigger is now a
  // RevealControls-driven arrow pair (down expands, up collapses; see that
  // component's own O2 comment), which communicates state via its own
  // changing label ("Read the whole chapter" vs "Show just this verse",
  // MoreLabel/CollapseLabel) rather than an aria-expanded flag -- verified
  // here the same way every OTHER expand/collapse test in this file already
  // is, by `popover-verse-reader`'s own presence/absence directly, never a
  // separate ARIA proxy for it.
  await expect(page.getByTestId('popover-verse-reader')).toHaveCount(0);
  const expandBtn = page.getByTestId('popover-verse-expand');
  const collapseBtn = page.getByTestId('popover-verse-collapse');
  await expect(collapseBtn).toHaveCount(0);

  await expandBtn.click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();
  await expect(page.getByTestId('popover-verse-expand')).toHaveCount(0);

  const chapter = await api.chapter('2CO.4');
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(chapter.verses.length);

  const focal = page.getByTestId('popover-reader-verse-6');
  await expect(focal).toHaveAttribute('data-focal', 'true');
  await expect(focal).toBeInViewport();
  await expect(page.getByTestId('popover-reader-verse-1')).toHaveAttribute('data-focal', 'false');

  // Collapse restores the exact compact view.
  await collapseBtn.click();
  await expect(page.getByTestId('popover-verse-reader')).toHaveCount(0);
  await expect(page.getByTestId('popover-verse-collapse')).toHaveCount(0);
});

// A passage's own focal range highlights EVERY member verse, not just the
// first -- and the compact/aggregated text still matches READ-5/READ-6's
// own established "aggregate as today" behavior (requirement 3's own
// closing line). M-D3/U6: relocated to a PLACE popover's own destroyed-date
// supporting verses (/world, real curated data -- Jerusalem's own
// destruction, 2KI.25.9-10, two CONSECUTIVE curated verses that group into
// one real passage block) -- a shift-click passage-chip in the reader hits
// the SAME chapter-aware-suppression READER-1 immediately above now hits
// (always the chapter on screen, structurally); /world carries no "current
// reader chapter" concept at all, so this multi-verse focal-range case
// stays fully exercisable there, unaffected.
test('READER-1: a passage\'s whole focal range is highlighted when expanded', async ({ page }) => {
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.hover({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover-place-date-destroyed')).toBeVisible();

  const entry = page.getByTestId('popover-place-date-destroyed-verse-2KI.25.9-10');
  await expect(entry).toBeVisible();
  await entry.click();
  await expect(page.getByTestId('popover-title')).toHaveText('2KI.25.9-10');

  await page.getByTestId('popover-verse-expand').click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

  for (const n of [9, 10]) {
    await expect(page.getByTestId(`popover-reader-verse-${n}`)).toHaveAttribute('data-focal', 'true');
  }
  await expect(page.getByTestId('popover-reader-verse-8')).toHaveAttribute('data-focal', 'false');
  await expect(page.getByTestId('popover-reader-verse-11')).toHaveAttribute('data-focal', 'false');
});

// ---------------------------------------------------------------------
// REGISTRY-1: PLACE node sections -- description seam, dates, blurb,
// events, in order, conditional presence. Jerusalem is heavily curated
// (established AND destroyed, event-bearing at every historical window).
// ---------------------------------------------------------------------

test('REGISTRY-1: a PLACE popover shows dates and events, in order, no thin event-only shell', async ({ page }) => {
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.hover({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover')).toBeVisible();

  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  // description seam (Batch P, not yet registered) never contributes a
  // section; dates then events is the observable order for a place with no
  // window-scoped blurb curated for THIS particular window.
  expect(sectionIds[0]).toBe('popover-section-place-dates');
  expect(sectionIds).toContain('popover-section-place-events');
  expect(sectionIds.indexOf('popover-section-place-dates')).toBeLessThan(sectionIds.indexOf('popover-section-place-events'));

  await expect(page.getByTestId('popover-place-date-established')).toBeVisible();
  await expect(page.getByTestId('popover-place-date-destroyed')).toBeVisible();

  // M-D1 requirement 4 (TRUNCATION AUDIT): this list is capped now
  // (PlaceEventsList.razor, cap 10) -- Jerusalem alone real-carries 236
  // located-at events across the whole atlas, previously rendered with NO
  // cap at all. Capped count visible by default; the down-arrow reveals
  // every remaining row, honest disclosure per the standard pattern.
  const detail = await api.place('jerusalem');
  const cap = 10;
  expect(detail.events.length, 'jerusalem must still real-carry MORE than the cap for this assertion to exercise it').toBeGreaterThan(cap);
  await expect(page.locator('[data-testid^="place-event-"]')).toHaveCount(cap);
  await expect(page.getByTestId('place-events-more')).toBeVisible();
  await page.getByTestId('place-events-more').click();
  await expect(page.locator('[data-testid^="place-event-"]')).toHaveCount(detail.events.length);
  await expect(page.getByTestId('place-events-collapse')).toBeVisible();
});

// Batch F2 requirement 6b (user direction 2026-08-20, verbatim: "on the
// established/destroyed buttons just display verses/passages how we do on
// every other hover menu... rather than the stupid buttons i have to click
// to see"): the established/destroyed date's own supporting verses render
// INLINE, immediately -- no click needed at all. The date row itself is no
// longer a button (the "click to reveal" gate is retired); it stays a
// plain, non-interactive instrument-face label.
test('REGISTRY-1/XREF-1: a PLACE popover\'s established/destroyed verses render inline with no click, capped at 2', async ({ page }) => {
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await marker.hover({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover-section-place-dates')).toBeVisible();

  // The date row itself is a plain label now, not a button -- no onclick,
  // no explorable affordance.
  const establishedRow = page.getByTestId('popover-place-date-established');
  await expect(establishedRow).toBeVisible();
  await expect(establishedRow).not.toHaveJSProperty('tagName', 'BUTTON');
  await expect(establishedRow).toContainText('Established');

  const detail = await api.placeHistory('jerusalem', -1000, -900);
  const establishedVerseCount = detail.history.established.verses.length;
  const cap = 2;

  // Full verse TEXT is already visible -- no extra click needed at all.
  // Note: the cap counts PASSAGE ENTRIES (blocks), not raw verses (XREF-1)
  // -- consecutive curated verses group into one block, so the rendered
  // entry count can be LESS than min(rawVerseCount, cap) whenever grouping
  // applies (Jerusalem's own established claim, 2SA.5.6/5.7/5.9, groups
  // into exactly 2 blocks: 5.6-7 together, 5.9 alone). This test reads the
  // ACTUAL rendered state rather than predicting the block count
  // independently (that would just re-implement PassageGrouping.Groups a
  // second time here).
  const estEntries = page.locator('[data-testid^="popover-place-date-established-verse-"]');
  const initialCount = await estEntries.count();
  expect(initialCount).toBeGreaterThan(0);
  expect(initialCount).toBeLessThanOrEqual(cap);
  const firstEntryText = await estEntries.first().textContent();
  expect((firstEntryText ?? '').trim().length).toBeGreaterThan(15);

  const moreButton = page.getByTestId('popover-place-date-established-more');
  const hasMore = await moreButton.count() > 0;
  if (hasMore) {
    // Only reachable if the real curated data ever grows past 2 blocks for
    // this place/window -- exercised for real whenever it does; asserted
    // either way so this test stays meaningful if the data changes.
    expect(initialCount).toBe(cap);
    await moreButton.click();
    const revealedCount = await estEntries.count();
    expect(revealedCount).toBeGreaterThan(cap);
    await expect(page.getByTestId('popover-place-date-established-collapse')).toBeVisible();
    await page.getByTestId('popover-place-date-established-collapse').click();
    await expect(estEntries).toHaveCount(initialCount);
  } else {
    // Fewer entries than the cap (or exactly at it) -- no arrow at all,
    // per XREF-1's own "conditional presence" rule. Jerusalem's real
    // curated data (2 blocks, at the cap) exercises this branch today.
    expect(initialCount).toBeLessThanOrEqual(cap);
  }
});

// ---------------------------------------------------------------------
// CHAPTER-CARD-1 (M-D3 fix round, R-D2/review I1): ChapterCardSection --
// U4/B3's own metadata-and-context card, owner verbatim (progress.md):
// "when you're reading a chapter, you're in its focus. you can focus
// further by clicking chapter heading and you get metadata and context...
// container title, position in book, edge summary -- what the graph knows
// ABOUT the chapter" -- NEVER the chapter's own verse text (B3, the
// standing "first verse" bug). This section had zero direct Playwright
// content assertions before this fix round -- every `chapter-card-*`
// testid lived only in CONTRACT.md prose. JOS.6 is this file's own real,
// curated exemplar: exactly one heading container ("The walls of Jericho
// fall", cq_jericho) and exactly one attested place (Jericho) for the
// whole chapter -- small enough to assert precisely, real enough to prove
// the card's own data plumbing, not a tautology.
// ---------------------------------------------------------------------

test('CHAPTER-CARD-1: hovering chapter-head opens the metadata-and-context card -- real position/verse-count/headings/places, never the chapter\'s own first verse or its text', async ({ page }) => {
  const toc = await loadToc();
  const jos = toc.find((b: any) => b.code === 'JOS');
  const totalChapters = jos.chapters.length;
  const chapterOut = await api.chapter('JOS.6');
  const verse1Text = chapterOut.verses.find((v: any) => v.verse === 1).text;

  await page.goto('/read/JOS/6');
  await page.getByTestId('chapter-head').hover();
  await expect(page.getByTestId('popover')).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('JOS.6');

  // Real content, read straight off the wire -- not just "a popover opened."
  await expect(page.getByTestId('chapter-card-position')).toHaveText(`Chapter 6 of ${totalChapters}`);
  await expect(page.getByTestId('chapter-card-verse-count')).toHaveText(`${chapterOut.verses.length} verses.`);
  await expect(page.getByTestId('chapter-card-headings-heading')).toHaveText('CONTAINERS IN THIS CHAPTER');
  await expect(page.getByTestId('chapter-card-heading-cq_jericho')).toHaveText('The walls of Jericho fall');
  await expect(page.getByTestId('chapter-card-places-heading')).toHaveText('PLACES MENTIONED');
  await expect(page.getByTestId('chapter-card-place-jericho-1')).toHaveText('Jericho');

  // B3's own standing bug, concretely disproven: the chapter's own first
  // verse (or any verse text at all) never appears -- neither as a
  // dedicated reader testid nor as literal text anywhere in the popover.
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(0);
  await expect(page.getByTestId('popover-verse-text')).toHaveCount(0);
  const popoverText = (await page.getByTestId('popover').textContent()) ?? '';
  expect(popoverText).not.toContain(verse1Text);

  // Both entry points are conditionally explorable -- proves this card is
  // real outward-connection content, not a dead-end summary.
  await page.getByTestId('chapter-card-heading-cq_jericho').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The walls of Jericho fall');
});

test('CHAPTER-CARD-1: clicking chapter-head opens the identical card (hover and click are the same open, XSCRIPT-1\'s own entry-point rule applied here too)', async ({ page }) => {
  await page.goto('/read/JOS/6');
  await page.getByTestId('chapter-head').click();
  await expect(page.getByTestId('popover-title')).toHaveText('JOS.6');
  await expect(page.getByTestId('chapter-card-position')).toBeVisible();
  await expect(page.getByTestId('chapter-card-heading-cq_jericho')).toHaveText('The walls of Jericho fall');
  await expect(page.getByTestId('chapter-card-place-jericho-1')).toHaveText('Jericho');
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(0);

  // A genuine click persists (does not auto-dismiss the way a hover-only
  // open would) -- confirms OpenChapter(persistent: true) actually ran,
  // not merely that SOME popover happened to be on screen at click time.
  await page.mouse.move(2, 2);
  await page.waitForTimeout(1200);
  await expect(page.getByTestId('popover')).toBeVisible();
});

// The fix round's own live-caught bug, now covered directly: PSA.119's real
// 22 acrostic-stanza heading containers (confirmed against the compiled
// data) made this card tall enough to self-overlap chapter-head before the
// cap existed. No places are attested anywhere in PSA.119 (real, confirmed
// data) -- this test exercises the headings cap specifically, the one the
// live bug actually turned on; the places cap shares the identical
// ListCap/ChapterCardSection code path, not independently re-proven here.
test('CHAPTER-CARD-1: the 8-row containers cap renders an honest "+N more" line on a many-container chapter (PSA.119, 22 real acrostic sections)', async ({ page }) => {
  const chapterOut = await api.chapter('PSA.119');
  const distinctHeadings = new Set(chapterOut.verses.filter((v: any) => v.heading).map((v: any) => v.heading.event_id));
  const cap = 8;
  expect(distinctHeadings.size, 'PSA.119 must still real-carry MORE than the cap for this assertion to exercise it').toBeGreaterThan(cap);

  await page.goto('/read/PSA/119');
  await page.getByTestId('chapter-head').click();
  await expect(page.getByTestId('popover-title')).toHaveText('PSA.119');

  await expect(page.locator('[data-testid^="chapter-card-heading-"]')).toHaveCount(cap);
  const moreLine = page.getByTestId('chapter-card-headings-more');
  await expect(moreLine).toBeVisible();
  await expect(moreLine).toHaveText(`+ ${distinctHeadings.size - cap} more containers in this chapter.`);

  // The exact bug this cap fixes: chapter-head itself must still be
  // reachable and clickable with the card open -- proving the card no
  // longer covers its own trigger the way the uncapped version did.
  await expect(page.getByTestId('chapter-head')).toBeVisible();
});

// ---------------------------------------------------------------------
// PARALLELS-1 (O5, owner live-preview correction, 2026-08-23, verbatim:
// "parallels has double headers. for instance we have 1Ki.3.1-15 and 1
// kings right below it when focused on 2ch.1.2. Get rid of the second
// header"): VerseParallelsSection's own "PARALLELS" entries show ONE
// header (the ref-label span) per entry, never a second book-name caption
// beneath it -- see PopoverSectionProviders.cs's own O5 comment
// (WitnessUnitsResolver.ResolveAsync's own book-name Caption is stripped
// to null here, unlike EventWitnessesSection's own "PARALLEL ACCOUNTS,"
// which keeps it -- see event-timeline.spec.ts/popover-sections.spec.ts's
// own EVENT-1 tests for that section, untouched by this ruling). No prior
// test in this suite covered VerseParallelsSection's own rendering at all
// -- this is that coverage's first test, not just a regression guard for
// O5's own fix.
// ---------------------------------------------------------------------

test('PARALLELS-1: a verse\'s own PARALLELS entry shows the ref-label ONLY, never a second book-name header beneath it (O5)', async ({ page }) => {
  // Real, live-verified data (curl-confirmed against GET /api/verse/2CH.1.2
  // and GET /api/event/1ki_solomon_gibeon): 2CH.1.2 belongs to exactly one
  // titled event, "1ki_solomon_gibeon" (Solomon's dream at Gibeon; the gift
  // of wisdom), whose ONLY other witness is 1KI.3.1-15 (15 verses) -- the
  // owner's own named example, verbatim, reproduced exactly.
  await page.goto('/read/2CH/1');
  await page.getByTestId('verse-line-2').click();
  await expect(page.getByTestId('popover-title')).toHaveText('2CH.1.2');

  // Single QUALIFYING event (2CH.1.2 cites exactly one titled event) -- the
  // plain "PARALLELS" heading, not the "PARALLELS — {label}" multi-event
  // variant (VerseParallelsSection's own "single entry needs no name" rule).
  const section = page.getByTestId('popover-section-parallels');
  await expect(section).toBeVisible();
  await expect(section.getByTestId('event-section-heading')).toHaveText('PARALLELS');

  const entry = section.locator('[data-testid^="verse-parallel-"]');
  await expect(entry).toHaveCount(1);
  await expect(entry.locator('.popover-passage-ref-label')).toHaveText('1KI.3.1-15');

  // O5's own fix: no second header (the book-name caption, "1 Kings")
  // beneath the ref-label -- one header per parallel entry.
  await expect(entry.locator('.popover-passage-caption')).toHaveCount(0);

  // The entry is still explorable (O5 only removed the caption, nothing
  // else) -- pushes the real witness passage, same as any other PassageList entry.
  await entry.click();
  await expect(page.getByTestId('popover-title')).toHaveText('1KI.3.1-15');
});

// ---------------------------------------------------------------------
// BLINK-1: hovering a place mention inside the mini-reader blinks the
// SAME place's own live marker.
// ---------------------------------------------------------------------

// Discovers a (chapter, verse, place) triple where the place is (a) linked
// to this exact verse (server: AtlasData.places_for_verse) and (b) named
// literally, case-insensitively, inside the verse's own KJV text -- the
// same plain-text match PlaceMentions.Scan (client) performs -- so the
// mini-reader is guaranteed to render a real, hoverable mention for it.
async function findMentionableVerse(candidates: string[]): Promise<{ book: string; chapter: number; verse: number; placeId: string; placeName: string } | null> {
  for (const cref of candidates) {
    const chapter = await api.chapter(cref);
    for (const v of chapter.verses) {
      const place = (v.places || []).find((p: any) => v.text.toLowerCase().includes(p.name.toLowerCase()));
      if (place) {
        const [book, chapterNum] = cref.split('.');
        return { book, chapter: Number(chapterNum), verse: v.verse, placeId: place.id, placeName: place.name };
      }
    }
  }
  return null;
}

test('BLINK-1: hovering a place mention in the mini-reader blinks its map marker; leaving unblinks it', async ({ page }) => {
  // M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  // when already reading that chapter." findMentionableVerse's own
  // candidates are meant to be opened via a plain verse-line click, which
  // (by construction, always FROM the chapter on screen) now always
  // correctly suppresses popover-verse-expand -- there is no verse-line
  // click that reaches a chapter the reader isn't already displaying.
  // Real, live-verified relocation (a diagnostic script confirmed the
  // exact chain, since GEN.28's own "Beth-el" spelling -- the KJV's own
  // typographic en-dash -- does NOT literally contain "Bethel", so that
  // reading was tried and rejected first): explore a real, stable,
  // curated cross-reference (GEN.12.8, mentions Ai/Bethel directly and
  // carries 17 real cross-refs, -> GEN.28.19, votes=3) to reach a VERSE in
  // a DIFFERENT chapter than the one on screen -- popover-verse-expand is
  // NOT suppressed there, and GEN.28's own text separately, cleanly
  // mentions "Canaan" (verses 1/6/8, no punctuation to trip the literal
  // match) -- a place BOTH genuinely mentioned in GEN.28's own prose AND
  // independently lit on GEN.12's own follow-mode scripture scene
  // (GEN.12.5's own real text: "into the land of Canaan"), so the SAME
  // marker this test hovers-to-blink is authentically on screen for a
  // reason that has nothing to do with the mini-reader's own chapter.
  await page.goto('/read/GEN/12?split=1');
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');

  const marker = page.getByTestId('marker-canaan').or(page.getByTestId('quiet-marker-canaan'));
  await expect(marker).toBeAttached({ timeout: 15000 });

  await page.getByTestId('verse-line-8').click();
  await expect(page.getByTestId('popover-verse-expand')).toHaveCount(0); // chapter-aware suppression, verified above
  const more = page.getByTestId('xrefs-more');
  await expect(more).toBeVisible();
  // M-D4 fix round 1/P2: "all" is conditionally omitted whenever a single
  // MORE click already reaches the true total (RevealControls.razor's own
  // ShowAll rule) -- reveal-all-or-fall-back-to-more, same defensive
  // pattern this file's own CATECH-1 tests already use, so this stays
  // correct regardless of GEN.12.8's own exact real xref count.
  const xrefsAll1 = page.getByTestId('xrefs-more-all');
  if (await xrefsAll1.count() > 0) {
    await xrefsAll1.click();
  } else {
    await more.click();
  }
  const xrefItem = page.getByTestId('xref-item-GEN.28.19');
  await expect(xrefItem).toBeVisible();
  await xrefItem.click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.28.19');

  const expandBtn = page.getByTestId('popover-verse-expand');
  await expect(expandBtn).toBeVisible(); // a DIFFERENT chapter than the reader's own -- not suppressed
  await expandBtn.click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

  const mention = page.getByTestId('popover-reader-mention-1-canaan');
  await expect(mention).toBeVisible();

  // marker-{id}/quiet-marker-{id} carries the testid AND the .atlas-marker/
  // .quiet-marker class on the SAME element (map.js's own makeIcon/
  // makeQuietIcon) -- this locator already IS the blink target, not an
  // ancestor of one.
  await expect(marker).not.toHaveClass(/atlas-blink/);

  await mention.hover();
  await expect(marker).toHaveClass(/atlas-blink/);

  await page.mouse.move(2, 2);
  await expect(marker).not.toHaveClass(/atlas-blink/);
});

// prefers-reduced-motion: the pulse animation itself is disabled -- a
// class/state assertion (computed animation-name), not pixel timing, per
// the batch report's own testing approach.
test('BLINK-1: prefers-reduced-motion disables the pulse animation on .atlas-blink', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/world?from=-1000&to=-900');
  const marker = page.getByTestId('marker-jerusalem').or(page.getByTestId('quiet-marker-jerusalem'));
  await expect(marker).toBeAttached();
  await marker.evaluate(el => el.classList.add('atlas-blink'));
  const animationName = await marker.evaluate(el => getComputedStyle(el).animationName);
  expect(animationName).toBe('none');
});

// ---------------------------------------------------------------------
// MENTION (M-D3/U5, "in-text mentions-attested links"): PlaceMentions.Scan
// widened to a second entity kind (Explore/PlaceMentions.cs) and wired into
// Reader.razor's own PRIMARY verse text (previously plain @v.Text, no
// scanning at all -- only the nested mini-reader, BLINK-1 above, ever did
// this) -- see CONTRACT.md's own MENTION note for the full behavior.
// ---------------------------------------------------------------------

test('MENTION-1: clicking a place mention in the reader\'s own verse text opens that place, not the verse underneath it', async ({ page }) => {
  // GEN.28.1's real text ("...a wife of the daughters of Canaan") carries
  // one clean, punctuation-free "Canaan" -- attested as BOTH a Place and a
  // (unrelated, Genesis 9-10) Person for this same verse; PlaceMentions'
  // own stable-sort tie-break resolves the ambiguity to the Place, which is
  // also the linguistically correct reading here (see that class's own doc
  // comment for the full disclosed story).
  await page.goto('/read/GEN/28');
  const mention = page.getByTestId('verse-mention-1-canaan');
  await expect(mention).toBeVisible();
  await expect(mention).toHaveText('Canaan');

  await mention.click();
  // @onclick:stopPropagation on the mention span is what's under test here
  // -- without it, this click would ALSO bubble into the verse-line's own
  // handler and open a VerseNode (popover-title "GEN.28.1") instead.
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
});

test('MENTION-2: clicking a person mention in the reader\'s own verse text opens that person', async ({ page }) => {
  // EXO.4.14's real text names Aaron and Moses both cleanly (this file's
  // own graph_api.rs sibling, chapter_verse_persons_is_always_present_and_
  // matches_the_generic_mentions_frontier, pins the same verse server-side).
  await page.goto('/read/EXO/4');
  const aaron = page.getByTestId('verse-mention-person-14-aaron_1');
  await expect(aaron).toBeVisible();
  await expect(aaron).toHaveText('Aaron');

  await aaron.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Aaron');
});

test('MENTION-3: a common word is never linked just because it collides with an attested place name under a different case', async ({ page }) => {
  // The positive half of the guard: EXO.16.1 genuinely attests the place
  // "Sin" (a real wilderness name, capitalized, standing alone in real KJV
  // prose) -- confirmed linked.
  await page.goto('/read/EXO/16');
  const placeMention = page.getByTestId('verse-mention-1-sin');
  await expect(placeMention).toBeVisible();
  await expect(placeMention).toHaveText('Sin');

  // The negative half: GEN.4.7 ("...sin lieth at the door...") uses the
  // common noun, lowercase, and attests NEITHER the place Sin NOR any
  // person for this verse (VerseOut.Places/Persons both empty, confirmed
  // against the real compiled data) -- were the old case-INSENSITIVE
  // search still in place, "sin" here would have matched "Sin" and wrongly
  // linked. No mention span of any kind renders in this verse's text.
  await page.goto('/read/GEN/4');
  const verseLine = page.getByTestId('verse-line-7');
  await expect(verseLine).toContainText('sin lieth at the door');
  await expect(verseLine.locator('.verse-mention')).toHaveCount(0);
});

test('MENTION-4: clicking a place mention INSIDE the mini-reader pushes a new popover level, alongside its existing hover-blink', async ({ page }) => {
  // Same real, live-verified navigation BLINK-1 above already establishes
  // (GEN.12.8 -> GEN.28.19's own cross-reference, opened, expanded to its
  // whole chapter) -- reaching the identical "Canaan" mention rendered a
  // SECOND time, this time inside MiniReaderExpand rather than Reader.razor's
  // own primary text, proving OnExplore's own new plumbing (MiniReaderExpand
  // -> VerseTextSection/PassageList -> ctx.PushAsync) independently of the
  // Reader.razor-level MENTION-1 test above.
  await page.goto('/read/GEN/12?split=1');
  await page.getByTestId('verse-line-8').click();
  const more = page.getByTestId('xrefs-more');
  await expect(more).toBeVisible();
  // M-D4 fix round 1/P2: same conditional-"all" fallback as this file's
  // own BLINK-1 test just above.
  const xrefsAll2 = page.getByTestId('xrefs-more-all');
  if (await xrefsAll2.count() > 0) {
    await xrefsAll2.click();
  } else {
    await more.click();
  }
  await page.getByTestId('xref-item-GEN.28.19').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.28.19');

  await page.getByTestId('popover-verse-expand').click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

  const mention = page.getByTestId('popover-reader-mention-1-canaan');
  await expect(mention).toBeVisible();
  await mention.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
});

test('R-M1: a preview row\'s mentioned name (PassageList\'s own compact text -- xref/THE SCRIPTURES/place-date/witness previews) links exactly the way the SAME mention links in the main reader', async ({ page }) => {
  // M-D4 fix round 2/F1 (re-review, Important): fix round 1's own report
  // and CONTRACT.md claimed a parity test for this existed; it did not
  // (zero hits for `popover-passage-mention` anywhere under tests/ux
  // before this test). This is that test, written for real against the
  // R-M1 fix itself -- PassageList.razor's own compact preview text now
  // routes through MentionText, the SAME component every other verse-text
  // surface uses.
  //
  // Real, live-verified fixture (curl-confirmed against GET
  // /api/verse/GEN.24.3 and GET /api/chapter/GEN.10): GEN.24.3's own
  // cross-references include the multi-verse target GEN.10.15-19 (Table
  // of Nations); that span's own FIRST verse, GEN.10.15 ("And Canaan
  // begat Sidon his firstborn, and Heth,"), attests a clean, unambiguous
  // PERSON mention -- "Canaan" (id canaan_914), the verse's own first
  // word, no punctuation collision. Because it's the span's own first
  // verse, it always renders in the xref-item's own compact preview text
  // regardless of ClampVerses.
  await page.goto('/read/GEN/24');
  await page.getByTestId('verse-line-3').focus(); // keyboard activation -- MENTION-1's own documented coordinate-click hazard
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.24.3');

  // GEN.24.3 carries far more than XREF-1's own 2/3-entry cap (37 real
  // cross-references) -- reveal everything so the specific target this
  // test needs is guaranteed visible, the SAME defensive reveal-then-
  // fall-back-to-more pattern this file's own fix-round-1 tests already
  // established for RevealControls' own conditional "all".
  const more = page.getByTestId('xrefs-more');
  await expect(more).toBeVisible();
  const all = page.getByTestId('xrefs-more-all');
  if (await all.count() > 0) {
    await all.click();
  } else {
    await more.click();
  }

  const entry = page.getByTestId('xref-item-GEN.10.15-19');
  await expect(entry).toBeVisible();
  // Read the entry's own real testid rather than predicting the span
  // string ourselves (this file's own established discipline, XREF-1/
  // regression's own comment, verbatim reasoning) -- proves the mention
  // testid's own ENTRY-ID suffix construction directly, not merely
  // assumed to match CONTRACT.md's own documented shape.
  const entryTestId = await entry.getAttribute('data-testid');
  expect(entryTestId).toBe('xref-item-GEN.10.15-19');

  const previewMention = entry.getByTestId(`popover-passage-mention-person-15-canaan_914-${entryTestId}`);
  await expect(previewMention).toBeVisible();
  await expect(previewMention).toHaveText('Canaan');

  // The click contract itself -- @onclick:stopPropagation on the mention
  // span (MentionText.razor) must win over the entry's own outer
  // Explore(entry.Block) click, the SAME "more specific target always
  // wins" rule ONE-RULE establishes everywhere else in this popover
  // platform; without it this click would open a PassageNode for the
  // whole GEN.10.15-19 span instead of the Person.
  await previewMention.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
  await page.getByTestId('popover-close').click();

  // Parity, proven directly rather than assumed from the shared
  // component alone: the IDENTICAL mention, read straight in the main
  // reader (GEN.10.15, no popover/preview involved at all), links the
  // SAME way -- same text, same click-opens-Canaan behavior.
  await page.goto('/read/GEN/10');
  const mainMention = page.getByTestId('verse-mention-person-15-canaan_914');
  await expect(mainMention).toBeVisible();
  await expect(mainMention).toHaveText('Canaan');
  await mainMention.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Canaan');
});

// ---------------------------------------------------------------------
// CATECH-1 (batch-f-brief.md, "the small catechism" -- user direction,
// asked three separate times): the verse->item->proof-verse hop, Luther's
// own verbatim explanation heading, and passage aggregation. All against
// REAL curated data/curated/catechism.toml content (MAT.28.19, "Baptism —
// Part One," is Luther's own institution-of-Baptism proof text --
// the exact "Baptism institution verse" example the batch brief itself
// names) -- not the demo fixture (that's atlas-server's own Rust-level
// integration test; this suite runs against the real compiled dataset).
// ---------------------------------------------------------------------

test('CATECH-1: the Baptism institution verse keeps its item-level citation, now alongside Batch F2\'s own question-level ones', async ({ page }) => {
  await page.goto('/read/MAT/28');
  await page.getByTestId('verse-line-19').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.28.19');

  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  await expect(page.getByTestId('popover-section-catechism').getByTestId('catechism-section-heading')).toHaveText('THE SMALL CATECHISM');
  // M-D3/U2/U6: THE SMALL CATECHISM now defaults to 2 shown -- reveal
  // everything first, since baptism-1's own position among MAT.28.19's
  // many citing items isn't guaranteed (this test's own concern is content,
  // not the reveal mechanic itself).
  const catechismMoreBaptism = page.getByTestId('catechism-more');
  if (await catechismMoreBaptism.count() > 0) {
    // M-D4 fix round 1/P2: "all" is conditionally omitted whenever a
    // single MORE click already reaches the true total (RevealControls.
    // razor's own ShowAll rule) -- fall back to MORE in that case, which
    // by the SAME rule already reveals everything when "all" is absent.
    const catechismAllBaptism = page.getByTestId('catechism-more-all');
    if (await catechismAllBaptism.count() > 0) {
      await catechismAllBaptism.click();
    } else {
      await catechismMoreBaptism.click();
    }
  }

  // Luther's OWN item-level embedded citation (Batch F, unchanged) -- the
  // bare, unsuffixed "Baptism — Part One" row must still be exactly this
  // text (batch-f2-brief.md's own acceptance spot-check: "MAT.28.19 keeps
  // its Baptism links").
  await expect(page.getByTestId('catechism-item-baptism-1')).toHaveText('Baptism — Part One');

  // Batch F2: the repo mapping now ALSO cites many items via this same
  // verse (a real, disclosed richness -- MAT.28.19 is the Great Commission,
  // cited from several different angles across the ~37 ingested files) --
  // this section is no longer a single-item case.
  const items = page.getByTestId(/^catechism-item-/);
  const count = await items.count();
  expect(count).toBeGreaterThan(1);
  const texts = await items.allTextContents();
  expect(texts.some(t => t.includes(' — '))).toBeTruthy(); // at least one question-titled row present

  // M-D3/U6, owner verbatim order: "Header / Verse (focus) / Event /
  // Parallels / Small Catechism / cross references LAST" -- REPLACES the
  // pre-M-D3 order this test's own comment used to pin (catechism BEFORE
  // event membership; catechism unconditionally last when event membership
  // was absent). MAT.28.19 (the Great Commission) is within pw_galilee's
  // own verse range (MAT.28.16-20), so it's EVENT-linked in the real
  // curated data -- event membership is present, and now comes BEFORE
  // catechism, not after.
  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(sectionIds[0]).toBe('popover-section-verse-text');
  const catechismIndex = sectionIds.indexOf('popover-section-catechism');
  expect(catechismIndex).toBeGreaterThan(-1);
  const eventIndex = sectionIds.indexOf('popover-section-event-membership');
  expect(eventIndex).toBeGreaterThan(-1); // MAT.28.19 is a real pw_galilee member -- always present for this verse
  expect(eventIndex).toBeLessThan(catechismIndex);
  // Cross-references LAST when present, else catechism is the tail itself.
  const xrefsIndex = sectionIds.indexOf('popover-section-xrefs');
  if (xrefsIndex !== -1) {
    expect(catechismIndex).toBeLessThan(xrefsIndex);
    expect(sectionIds[sectionIds.length - 1]).toBe('popover-section-xrefs');
  } else {
    expect(catechismIndex).toBe(sectionIds.length - 1);
  }
});

test('CATECH-1: verse -> catechism item -> proof verse hop, with Luther\'s own verbatim heading', async ({ page }) => {
  await page.goto('/read/MAT/28');
  await page.getByTestId('verse-line-19').click();
  // M-D3/U2/U6: THE SMALL CATECHISM now defaults to 2 shown -- MAT.28.19
  // cites many items (the prior test's own subject), so baptism-1 is not
  // guaranteed to be among the initial 2; reveal everything first (this
  // test's own concern is the item -> proof-verse hop, not the reveal
  // mechanic itself). A real, live-caught race: `.count()` has no
  // auto-retry (unlike `expect()`), so checking it immediately after the
  // click can catch LoadCurrent's own documented "cleared, then filled"
  // intermediate frame and wrongly skip the reveal step entirely --
  // waiting for the section to actually settle first (the SAME "wait for
  // a real settled signal" discipline this file's own EVENT-1 traversal
  // tests already apply) is the fix.
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  const catechismMore = page.getByTestId('catechism-more');
  if (await catechismMore.count() > 0) {
    // M-D4 fix round 1/P2: see the identical fallback comment earlier in
    // this file (CATECH-1's own Baptism-institution test, just above) --
    // "all" is conditionally omitted, MORE already reveals everything then.
    const catechismAll = page.getByTestId('catechism-more-all');
    if (await catechismAll.count() > 0) {
      await catechismAll.click();
    } else {
      await catechismMore.click();
    }
  }
  await page.getByTestId('catechism-item-baptism-1').click();

  // The CatechismNode popover: title is the item's own display name.
  await expect(page.getByTestId('popover-title')).toHaveText('Baptism — Part One');

  // "What is Baptism?" is Luther's OWN bespoke heading for this specific
  // item (NOT the generic "What does this mean?") -- rendered verbatim as
  // this section's own title, proving the heading genuinely varies per
  // item rather than being a hardcoded placeholder string.
  const explanationSection = page.getByTestId('popover-section-catechism-explanation');
  await expect(explanationSection).toBeVisible();
  await expect(explanationSection.getByTestId('catechism-section-heading')).toHaveText('What is Baptism?');
  await expect(explanationSection).toContainText('Baptism is not simple water only');

  // Baptism Part One has no separate prompt text of its own (text is
  // absent -- see CatechismItem's doc comment) -- no catechism-text section.
  await expect(page.getByTestId('popover-section-catechism-text')).toHaveCount(0);

  // "Where is this written?" -- Luther's own proof citation for this item.
  const whereWritten = page.getByTestId('popover-section-catechism-where-written');
  await expect(whereWritten).toBeVisible();
  await expect(whereWritten.getByTestId('catechism-section-heading')).toHaveText('Where is this written?');
  await expect(whereWritten).toContainText('Go ye into all the world');

  // "THE SCRIPTURES" -- the proof verse itself, explorable, full text.
  const scriptures = page.getByTestId('popover-section-catechism-scriptures');
  await expect(scriptures).toBeVisible();
  await expect(scriptures.getByTestId('catechism-section-heading')).toHaveText('THE SCRIPTURES');
  const proofVerse = page.getByTestId('catechism-verse-MAT.28.19');
  await expect(proofVerse).toBeVisible();
  await expect(proofVerse).toContainText('MAT.28.19');
  await expect(proofVerse).toContainText('baptizing them in the name of the Father');

  // A CatechismNode offers no chips at all (no geography). M-D3 (U3'):
  // the chip row moved from .popover-chips (below the body) into
  // .popover-head-actions (inline beside the title) -- same "zero
  // explorations, no container renders at all" conditional presence,
  // new class name.
  await expect(page.locator('.popover-head-actions')).toHaveCount(0);

  // The hop: clicking the proof verse pushes an ordinary VerseNode for the
  // SAME verse the item was originally reached from -- onward navigation
  // works, no bespoke code (that verse's own sections render normally,
  // INCLUDING its own "THE SMALL CATECHISM" section again, proving the loop
  // is a real graph edge, not a dead end).
  await proofVerse.click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.28.19');
  await expect(page.getByTestId('popover-section-verse-text')).toBeVisible();
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
});

test('CATECH-1: a same-chapter passage selection aggregates catechism citations across member verses (item-level dedup)', async ({ page }) => {
  // MAT.26.26-28: all three verses cite the SAME item-level citation
  // (altar-1's own `verses`, the Sacrament of the Altar's institution
  // words, Luther's own embedded citation) -- the passage's own section
  // must list that BARE hit exactly ONCE (union+dedup), not three times,
  // even though (Batch F2) the repo mapping now ALSO cites altar-1 (and
  // several other items) via multiple separate QUESTIONS across this same
  // span -- dedup is by (item, question) pair, so those are each their own
  // row, additional to (not replacing) the bare dedup this test pins.
  await page.goto('/read/MAT/26');
  await page.getByTestId('verse-num-26').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('verse-num-28').click();
  await page.keyboard.up('Shift');
  await page.getByTestId('passage-chip').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.26.26-28');

  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  // M-D3/U2/U6: THE SMALL CATECHISM now defaults to 2 shown -- MAT.26.26-28
  // cites well over that (this test's own dedup subject below proves it),
  // so reveal everything first; this test's own concern is dedup, not the
  // reveal mechanic itself (XREF-1/U2's own dedicated test covers that).
  const catechismMore = page.getByTestId('catechism-more');
  if (await catechismMore.count() > 0) {
    // M-D4 fix round 1/P2: same conditional-"all" fallback as above.
    const catechismAll = page.getByTestId('catechism-more-all');
    if (await catechismAll.count() > 0) {
      await catechismAll.click();
    } else {
      await catechismMore.click();
    }
  }
  // The bare (item-level, no question) altar-1 row -- first occurrence,
  // unsuffixed testid -- appears exactly once despite being cited by all
  // three member verses.
  await expect(page.getByTestId('catechism-item-altar-1')).toHaveText('What Is the Sacrament of the Altar?');
  await expect(page.getByTestId('catechism-item-altar-1')).toHaveCount(1);

  // Batch F2: the repo mapping also cites altar-1 via several DIFFERENT
  // questions across this same span -- each is its own, separately
  // numbered-suffix row (id, question) pair, not folded into the bare hit.
  const altarRows = page.getByTestId(/^catechism-item-altar-1/);
  const altarCount = await altarRows.count();
  expect(altarCount).toBeGreaterThan(1);
  const altarTexts = await altarRows.allTextContents();
  expect(altarTexts.some(t => t.includes('What Is the Sacrament of the Altar? — '))).toBeTruthy();
});

// ---------------------------------------------------------------------
// Batch F2 requirement 3/4 (user direction 2026-08-20, verbatim: "I gave
// you the mapping very explicitly in the catechism repo"): a verse
// reachable ONLY via the brain-fuel/catechism mapping (never one of
// Luther's own embedded citations) shows THE SMALL CATECHISM section with
// a QUESTION-TITLED entry. Luke 12:13-14 -> "The First Commandment" (via
// its own repo question "God Alone as Judge") is the brief's own named
// example.
// ---------------------------------------------------------------------

test('CATECH-1/6-ARCH: a verse reachable only via the repo mapping shows a question-titled entry (Luke 12:13-14)', async ({ page }) => {
  await page.goto('/read/LUK/12');
  await page.getByTestId('verse-line-13').click();
  await expect(page.getByTestId('popover-title')).toHaveText('LUK.12.13');
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  // M-D3/U2/U6: reveal everything first -- this test's own concern is the
  // question-titled row's own content/hop, not the reveal mechanic
  // (position among the citing items isn't guaranteed).
  const catechismMore = page.getByTestId('catechism-more');
  if (await catechismMore.count() > 0) {
    // M-D4 fix round 1/P2: same conditional-"all" fallback as above.
    const catechismAll = page.getByTestId('catechism-more-all');
    if (await catechismAll.count() > 0) {
      await catechismAll.click();
    } else {
      await catechismMore.click();
    }
  }

  const items = page.getByTestId(/^catechism-item-/);
  const texts = await items.allTextContents();
  expect(texts.some(t => t === 'The First Commandment — God Alone as Judge')).toBeTruthy();

  // Onward: clicking it opens the item's own popover -- THE SCRIPTURES
  // groups this question's OWN two verses (Luke 12:13-14, consecutive)
  // into ONE passage entry, captioned with the question title (6-ARCH:
  // "sequential verses display as one passage entry").
  const row = items.filter({ hasText: 'God Alone as Judge' });
  await row.click();
  await expect(page.getByTestId('popover-title')).toContainText('First Commandment');
  const scriptures = page.getByTestId('popover-section-catechism-scriptures');
  await expect(scriptures).toBeVisible();
  // The testid lives on the WHOLE entry (PassageList.razor: ref + caption +
  // text together, one explorable target -- see that component's own
  // header comment), not on a nested descendant, so the testid lookup and
  // the caption-text check both target the SAME element.
  const godAloneEntry = scriptures.getByTestId('catechism-verse-LUK.12.13-14');
  await expect(godAloneEntry).toBeVisible();
  await expect(godAloneEntry).toContainText('God Alone as Judge');
});

// M-D4 fix round 1, P2 (owner order, verbatim: "the down/double down thing
// is really ugly and needs work... the ability to show a little more with
// one click; the ability to show everything with another click, and the
// ability to undo either of those operations with a click"). The
// four-glyph arrow-button cluster (more/more-all/collapse/collapse-all)
// retires whole -- replaced by RevealControls.razor's own quiet text row
// (more (n) / all (N) / less), state-adaptive, with LESS a ONE-OP UNDO
// (after ALL: exact pre-ALL view; after MORE: steps back one Step) rather
// than a separate always-paired "collapse-all" button. See that
// component's own header comment for the full design.
test('CATECH-1/U2/U6: THE SMALL CATECHISM defaults to 2 shown, "more"/"all"/"less" are state-adaptive, LESS undoes ALL in one op', async ({ page }) => {
  const toc = await loadToc();
  // >4 (not merely >2): guarantees Total-Default > Step, so "all" is a
  // genuinely different action from "more" (RevealControls.razor's own
  // ShowAll rule) and actually renders for this test to exercise.
  const found = await findVerseWithCounts(toc, d => d.catechism.length > 4);
  test.skip(!found, 'no sampled verse had >4 catechism citations');
  if (!found) return;
  const v = parseVerse(found.vref);
  const total = found.detail.catechism.length;

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(found.vref);

  const items = page.getByTestId(/^catechism-item-/);
  const more = page.getByTestId('catechism-more');
  const all = page.getByTestId('catechism-more-all');
  const less = page.getByTestId('catechism-collapse');

  // U6, owner verbatim: "Catechism defaults to 2 shown." Collapsed state:
  // more+all visible, less absent (nothing to undo AT the default).
  await expect(items).toHaveCount(2);
  await expect(more).toHaveText('more (2)');
  await expect(all).toHaveText(`all (${total})`);
  await expect(less).toHaveCount(0);

  // MORE steps by exactly +2 -- now a PARTIAL state: more/all/less all
  // three visible together (the owner's own middle row).
  await more.click();
  await expect(items).toHaveCount(4);
  await expect(more).toBeVisible();
  await expect(all).toBeVisible();
  await expect(less).toBeVisible();

  // LESS after MORE steps back exactly one Step (not a jump to default).
  await less.click();
  await expect(items).toHaveCount(2);
  await expect(less).toHaveCount(0);

  // M-D4 fix round 2/F2 (re-review): ALL clicked from a NON-DEFAULT
  // partial state now, not straight from the default -- the earlier
  // MORE/LESS round-trip above already returned to exactly 2 (== Default
  // here), so an ALL click there could not discriminate "restore the
  // EXACT pre-ALL view" from "floor to the default" -- both produce the
  // identical count (2) when ALL is clicked FROM the default. Stepping
  // to a genuinely partial, non-default count first (4) and recording it
  // makes the two behaviors diverge, so the assertion below actually
  // proves which one this component does.
  await more.click();
  await expect(items).toHaveCount(4);
  const partialCount = await items.count();
  expect(partialCount, 'this fixture only discriminates restore-vs-floor when the recorded partial differs from Default (2)').not.toBe(2);

  // ALL jumps straight to the true total -- fully expanded: less ONLY
  // (more/all both gone, nothing further to reveal).
  await all.click();
  await expect(items).toHaveCount(total);
  await expect(more).toHaveCount(0);
  await expect(all).toHaveCount(0);
  await expect(less).toBeVisible();

  // LESS's own ONE-OP UNDO: the immediately-preceding action was ALL, so
  // this returns to the EXACT pre-ALL view -- the recorded PARTIAL count
  // (4), not the default (2) -- "undo... with a click," the owner's own
  // words, taken literally: undo means restore what was there before,
  // not reset to some other fixed state. A component that instead
  // floored to Default here would show 2, failing this assertion.
  await less.click();
  await expect(items).toHaveCount(partialCount);
  await expect(less).toBeVisible(); // still above Default -- more undo-able
  await expect(more).toBeVisible();
  await expect(all).toBeVisible();

  // The one-deep memory is CONSUMED, not a full history stack (this
  // file's own RevealControls doc comment, verbatim) -- a SECOND LESS
  // click now steps back by Step (not a second "restore"), landing
  // exactly on Default, "repeated LESS walks home."
  await less.click();
  await expect(items).toHaveCount(2);
  await expect(less).toHaveCount(0);
});

test('CATECH-1: a verse with 1-2 catechism citations shows no reveal arrow at all (at-or-under the default)', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerseWithCounts(toc, d => d.catechism.length >= 1 && d.catechism.length <= 2);
  test.skip(!found, 'no sampled verse had 1-2 catechism citations');
  if (!found) return;
  const v = parseVerse(found.vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();

  await expect(page.getByTestId(/^catechism-item-/)).toHaveCount(found.detail.catechism.length);
  await expect(page.getByTestId('catechism-more')).toHaveCount(0);
  await expect(page.getByTestId('catechism-collapse')).toHaveCount(0);
});

// ---------------------------------------------------------------------
// XREF-1 (batch-f2-brief.md requirement 6, user direction 2026-08-20,
// near-verbatim: "truncate the cross references to show no more than 3 if
// cross references are the only kind of context... and no more than two
// if there are other types of context pulled in (small catechism, etc.)").
// ---------------------------------------------------------------------

// Finds a real verse (scanning real chapters, not the demo fixture) whose
// own /api/verse response satisfies `predicate` -- same discovery approach
// this file's own findVerse already uses.
async function findVerseWithCounts(toc: any, predicate: (d: any) => boolean, maxChapters = 40): Promise<{ vref: string; detail: any } | null> {
  const books = fc.sample(fc.constantFrom(...toc), Math.min(maxChapters, toc.length));
  for (const b of books) {
    for (const ch of b.chapters.slice(0, 2)) {
      const chapter = await api.chapter(`${b.code}.${ch}`);
      for (const v of chapter.verses) {
        const vref = `${b.code}.${ch}.${v.verse}`;
        const detail = await api.verse(vref);
        if (predicate(detail)) {
          return { vref, detail };
        }
      }
    }
  }
  return null;
}

// M-D4 fix round 1, P2 -- see CATECH-1/U2/U6's own updated header comment
// immediately above (this file) for the full "why" behind the redesign.
test('XREF-1/U2: "more"/"all" are state-adaptive text links, "less" is a one-op undo (steps back after MORE, exact-restore after ALL)', async ({ page }) => {
  const toc = await loadToc();
  // >5 (not merely >3): guarantees at least one genuine PARTIAL state where
  // more/all/less show together, regardless of whether this sampled verse's
  // own initial cap turns out to be F2's 3 (xrefs-only) or 2 (mixed
  // context, e.g. a real Persons/Places mention alongside -- a live
  // possibility this predicate doesn't control for, deliberately: this
  // test's own subject is the more/all/less MECHANIC, not the cap VALUE,
  // which REGISTRY-1/XREF-1's own dedicated tests already pin -- reading
  // the actual initial count off the page rather than assuming 3 keeps
  // this test meaningful either way).
  const found = await findVerseWithCounts(toc, d => d.cross_refs.length > 5);
  test.skip(!found, 'no sampled verse had >5 xrefs');
  if (!found) return;
  const v = parseVerse(found.vref);
  const total = found.detail.cross_refs.length;

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(found.vref);

  const items = page.getByTestId(/^xref-item-/);
  const more = page.getByTestId('xrefs-more');
  const all = page.getByTestId('xrefs-more-all');
  const less = page.getByTestId('xrefs-collapse');

  await expect(more).toBeVisible();
  const defaultShown = await items.count();
  expect(defaultShown, 'the default cap must be F2\'s own 2 (mixed context) or 3 (xrefs-only)').toBeGreaterThanOrEqual(2);
  expect(defaultShown).toBeLessThanOrEqual(3);
  await expect(less).toHaveCount(0); // "never below the default" -- nothing to undo AT the default

  // Step up by exactly +2 per MORE click until the true total is reached --
  // hop count read from the wire (never hardcoded), so this test keeps
  // proving itself regardless of which real verse it happens to sample.
  // Each MORE click also invalidates any earlier ALL-undo memory (see
  // RevealControls.razor's own header comment), so LESS from here on
  // always means "step back one Step," verified below.
  let shown = defaultShown;
  let hops = 0;
  while (shown < total) {
    await expect(more, `expected a MORE link with ${total - shown} left to reveal`).toBeVisible();
    await expect(more).toHaveText(`more (${Math.min(2, total - shown)})`);
    await more.click();
    shown = Math.min(shown + 2, total);
    await expect(items).toHaveCount(shown);
    await expect(less, 'once past the default, LESS must also be available').toBeVisible();
    hops++;
    expect(hops, 'XREF-1 +2 reveal walk did not terminate within a sane number of hops').toBeLessThan(total);
  }
  await expect(more).toHaveCount(0); // fully revealed -- no more to show

  // LESS after a run of MORE clicks steps back exactly -2 per click, never
  // below the default (the "after MORE it steps back the increment...
  // repeated LESS walks home" half of the owner's own one-op-undo spec).
  while (shown > defaultShown) {
    await less.click();
    shown = Math.max(shown - 2, defaultShown);
    await expect(items).toHaveCount(shown);
  }
  await expect(less).toHaveCount(0); // back at the default -- nothing left to undo
  await expect(more).toBeVisible();

  // ALL jumps straight to the true total, skipping every intermediate step.
  await expect(all).toBeVisible();
  await expect(all).toHaveText(`all (${total})`);
  await all.click();
  await expect(items).toHaveCount(total);
  await expect(more).toHaveCount(0);
  await expect(all).toHaveCount(0);
  await expect(less).toBeVisible();

  // LESS's own ONE-OP UNDO ("undo... with a click," the owner's own
  // words): the immediately-preceding action was ALL, so THIS click
  // returns to the EXACT pre-ALL view (the default), not a slow -2 walk.
  await less.click();
  await expect(items).toHaveCount(defaultShown);
  await expect(less).toHaveCount(0);
  await expect(more).toBeVisible();
});

test('XREF-1: a verse with catechism context ALSO present caps cross-references at 2', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerseWithCounts(toc, d => d.cross_refs.length > 2 && d.catechism.length > 0);
  test.skip(!found, 'no sampled verse had >2 xrefs and >0 catechism');
  if (!found) return;
  const v = parseVerse(found.vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(found.vref);
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();

  const items = page.getByTestId(/^xref-item-/);
  await expect(items).toHaveCount(2);
  await expect(page.getByTestId('xrefs-more')).toBeVisible();
});

test('XREF-1: a verse with at-or-under-cap cross-references shows no reveal arrow at all', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerseWithCounts(toc, d => d.cross_refs.length >= 1 && d.cross_refs.length <= 2 && d.catechism.length === 0);
  test.skip(!found, 'no sampled verse had 1-2 xrefs and 0 catechism');
  if (!found) return;
  const v = parseVerse(found.vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  // Keyboard activation, not a coordinate click -- MENTION-1's own
  // documented hazard (CONTRACT.md): a plain .click() targets this
  // element's geometric center, which can land on one of its OWN nested
  // in-text mention spans instead (a real, live-caught case here: this
  // predicate's own unique match in a real sampled chapter, 1CO.16.23,
  // is a short verse whose own "Jesus Christ" mention spans much of the
  // line -- opening that PersonNode instead of this test's own intended
  // VerseNode). Sidesteps the coordinate geometry entirely.
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('popover-section-xrefs')).toBeVisible();

  const items = page.getByTestId(/^xref-item-/);
  await expect(items).toHaveCount(found.detail.cross_refs.length);
  await expect(page.getByTestId('xrefs-more')).toHaveCount(0);
  await expect(page.getByTestId('xrefs-collapse')).toHaveCount(0);
});

// fix-round-1, Important-2 (batch-f2-review.md): the ExploreAsVerse fix
// itself (PassageList.razor/CrossRefsSection, batch-f2-report.md's own
// "third real bug") was previously guarded ONLY by reader.spec.ts's own
// READ-3 property test -- a fast-check run sampling real data, which
// happened to catch the regression once but is not GUARANTEED to sample a
// multi-verse xref target on any given run (it could pass clean on a
// future reintroduction of this exact bug, purely by chance of sampling).
// This is a deterministic, targeted regression test for that exact class:
// DISCOVERS (not hardcodes) a real verse whose own FIRST cross-reference
// target spans more than one verse in the same chapter (e.g. "REV.8.3-5")
// -- ~25% of real cross-reference targets do (report §7), so this is
// common, not a contrived case -- and asserts clicking that xref entry
// opens a VerseNode at the target's own FIRST verse, never a PassageNode
// titled with the full range.
//
// `cross_refs[0]` is always exactly `items.first()` in the DOM regardless
// of the xrefs-only/xrefs+catechism cap (2 vs 3): CrossRefsSection builds
// exactly one PassageSourceUnit per xref entry, in the API's own order,
// via a single non-conditional `units.Add(...)` per entry (never zero,
// never merged with a neighbor -- see that function's own comment), and
// PassageBlockBuilder never drops or reorders units -- so the FIRST xref
// entry is always the FIRST rendered block, capped or not, at index 0.
async function findVerseWithFirstXrefMultiVerse(toc: any): Promise<{ vref: string; detail: any; targetHead: string } | null> {
  const isMultiVerseTarget = (target: string) => /^[A-Z0-9]{3}\.\d+\.\d+-\d+$/.test(target); // same-chapter span only (a cross-chapter target like "MAT.5.3-MAT.6.2" does not match: \d+$ can't match a second BOOK.CH prefix)
  const found = await findVerseWithCounts(toc, d => d.cross_refs.length > 0 && isMultiVerseTarget(d.cross_refs[0].target));
  if (!found) {
    return null;
  }
  const targetHead = found.detail.cross_refs[0].target.match(/^[A-Z0-9]{3}\.\d+\.\d+/)![0]; // same extraction READ-3 already uses
  return { ...found, targetHead };
}

test('XREF-1/regression: a cross-reference to a same-chapter multi-verse target opens a VerseNode at its own first verse, never a PassageNode for the whole range', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerseWithFirstXrefMultiVerse(toc);
  test.skip(!found, 'no sampled verse had a same-chapter multi-verse target as its own first cross-reference');
  if (!found) return;
  const v = parseVerse(found.vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  const items = page.getByTestId(/^xref-item-/);
  await expect(items).not.toHaveCount(0);
  const first = items.first();

  // Important-1's own correction, exercised for real (previously untested
  // by anything): the nested mini-reader controls are scoped by this
  // entry's own FULL testid (CONTRACT.md's `ENTRY-ID`), not a bare span --
  // read the entry's real testid rather than predicting the span string
  // ourselves, so this stays robust even if PassageGrouping's own
  // span-format ever changes.
  const entryTestId = await first.getAttribute('data-testid');
  await expect(first.getByTestId(`popover-verse-expand-${entryTestId}`)).toBeVisible();

  // The regression itself: PassageList's generic Explore() pushes a
  // PassageNode (title = the full multi-verse range) for any block with
  // >=2 verses by default; CrossRefsSection's own ExploreAsVerse=true
  // restores its pre-existing contract -- a VerseNode at the target's own
  // first verse. A PassageNode's own title would be the full range (e.g.
  // "REV.8.3-5"), which this exact-text assertion would reject.
  await first.click();
  await expect(page.getByTestId('popover-title')).toHaveText(found.targetHead);
});

// ---------------------------------------------------------------------
// EVENT-1 (batch-t-brief.md, "events as the narrative nodes" -- SUPERSEDES
// batch-n-brief.md's own NARRATIVE-1 verse-level tests, retired: "rather
// than putting the next/previous event on every verse, add titles of
// events that correspond to passages... traversal lives on event nodes,"
// the owner verbatim). EXO.13.20 (the exodus narrative's own "First camp
// at Succoth" leg, `ex_succoth`) is the SAME known narrative verse Batch N
// used, still picked (not discovered) because its exact adjacency (prior:
// ex_rameses/EXO.12.37; following: ex_red_sea/EXO.14.21-31) is
// independently readable straight off data/curated/narratives/exodus.toml
// + events-extra.toml, letting these tests assert EXACT text.
// ---------------------------------------------------------------------

test('EVENT-1: a verse popover shows EVENT membership (not PRIOR/FOLLOWING) -- traversal lives on the EVENT node', async ({ page }) => {
  // requirement 3/7's own explicit acceptance: "verse popover shows event
  // membership and NOT prev/next." VerseDetailOut.narrative_positions is
  // GONE (structurally -- fetching it directly proves the field no longer
  // exists on the wire, not merely that the UI doesn't render it).
  const detail = await api.verse('EXO.13.20');
  expect(detail.narrative_positions).toBeUndefined();
  expect(detail.events.some((e: any) => e.id === 'ex_succoth')).toBeTruthy();

  await page.goto('/read/EXO/13');
  await page.getByTestId('verse-line-20').click();
  await expect(page.getByTestId('popover-title')).toHaveText('EXO.13.20');

  // The "EVENT" section: present, names ex_succoth, explorable.
  const eventSection = page.getByTestId('popover-section-event-membership');
  await expect(eventSection).toBeVisible();
  await expect(eventSection.getByTestId('event-section-heading')).toHaveText('EVENT');
  const eventRow = eventSection.getByTestId('verse-event-ex_succoth');
  await expect(eventRow).toHaveText('First camp at Succoth');

  // Structurally impossible now: no event-traversal nav exists on a VERSE
  // popover at all (EventDateAndPlacesSection.AppliesTo is Event-only,
  // same as its M-D3/U1-retired PRIOR/FOLLOWING predecessors were).
  await expect(page.getByTestId('event-nav')).toHaveCount(0);
});

test('EVENT-1: a verse with no titled event shows no EVENT section at all (conditional presence)', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerse(toc, d => d.events.length === 0);
  test.skip(!found, 'no sampled verse had zero events');
  if (!found) return;
  const { vref } = found;
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  // Keyboard activation, not a coordinate .click() -- CONTRACT.md's own
  // documented MENTION-1 test hazard, live-caught here for real: a
  // zero-titled-event verse is no guarantee of a mention-sparse one, and
  // this sampler landed on RUT.4.21 ("Now Boaz begat Obed, and Obed begat
  // Jesse, and Jesse begat David"), whose text is almost entirely person
  // mentions -- a coordinate click there reliably lands on "Boaz" (or
  // another mentioned name) instead of the plain verse line, opening that
  // PERSON's own popover instead of this VERSE's.
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText(vref);
  await expect(page.getByTestId('popover-section-event-membership')).toHaveCount(0);
});

test('EVENT-1: clicking a verse\'s EVENT row opens the EventNode, whose PRIOR/FOLLOWING verses equal the map arrows\' own endpoint events (one-graph property)', async ({ page }) => {
  // The one-graph proof, at the wire level, independent of the popover's
  // own rendering: ex_succoth's own event-id-keyed narrative position
  // (UNCHANGED endpoint/resolver from Batch N) must report the SAME
  // following-event verse_groups as the live map's own scene (via the SAME
  // event id an arrow's own to_event names) -- byte-for-byte.
  const positions = (await api.narrativeEventPositions('ex_succoth')).narrative;
  const position = positions.find((p: any) => p.narrative_id === 'exodus');
  expect(position.prior.id).toBe('ex_rameses');
  expect(position.prior.label).toBe('Israel departs Rameses');
  expect(position.following.id).toBe('ex_red_sea');
  expect(position.following.label).toBe('Crossing the Red Sea');

  const scene = await api.sceneTime(-1446, -1406); // EXODUS_WINDOW (world-pin.spec.ts's own established exodus window)
  const arrow = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_succoth' && a.to_event === 'ex_red_sea');
  expect(arrow, 'the ex_succoth -> ex_red_sea leg must be a real rendered arrow in this window').toBeTruthy();
  const redSeaPlace = scene.places.find((p: any) => p.events.some((e: any) => e.id === 'ex_red_sea'));
  const redSeaSceneEvent = redSeaPlace.events.find((e: any) => e.id === 'ex_red_sea');
  expect(position.following.verse_groups).toEqual(redSeaSceneEvent.verse_groups); // <- the one-graph assertion itself

  // Also confirm GET /api/event/ex_succoth (the EVENT node's own richer
  // fetch) agrees on the title/date -- a DIFFERENT wire source from the
  // narrative-position lookup above, both describing the same event.
  const eventDetail = await api.event('ex_succoth');
  expect(eventDetail.title).toBe('First camp at Succoth');

  // Now the SAME thing, live: verse -> EVENT row -> EventNode -> PRIOR/FOLLOWING.
  await page.goto('/read/EXO/13');
  await page.getByTestId('verse-line-20').click();
  await page.getByTestId('verse-event-ex_succoth').click();
  await expect(page.getByTestId('popover-title')).toHaveText('First camp at Succoth');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();

  // M-D3/U1: the narrative nav is now MERGED into event-date-places (no
  // more separate PRIOR EVENT/FOLLOWING EVENT sections) -- exactly one
  // qualifying narrative -> no narrative-name label at all (that only
  // renders for a multi-narrative event, see the MULTI-NARRATIVE test
  // below), just the two flanking arrows.
  const eventSection = page.getByTestId('popover-section-event-date-places');
  await expect(eventSection).toBeVisible();
  await expect(eventSection.getByTestId('popover-event-nav-narrative')).toHaveCount(0);

  const priorBtn = eventSection.getByTestId('event-prior-event-exodus');
  // .popover-event-nav-label -- the button's own full text also includes
  // its decorative directional glyph (a sibling span).
  await expect(priorBtn.locator('.popover-event-nav-label')).toHaveText('Israel departs Rameses');
  // M-D4 fix round 1/P4 (owner, verbatim: "we straight up should not have
  // [the verse text]. you get that when you traverse."): the one-verse
  // caption this arrow used to show (EXO.12.37's own text) is RETIRED
  // outright -- no verse content, no attestation text in the affordance at
  // all, the click is what YIELDS the event, not what the button previews.
  // In its place: a static small-caps role caption naming the DIRECTION
  // only, plus a `title` on the (possibly ellipsis-truncated) name itself
  // carrying the untruncated event name for a native hover tooltip.
  await expect(eventSection.getByTestId('event-prior-verse-exodus')).toHaveCount(0);
  await expect(eventSection.getByTestId('event-prior-label-exodus')).toHaveText('PRIOR EVENT');
  await expect(priorBtn.locator('.popover-event-nav-label')).toHaveAttribute('title', 'Israel departs Rameses');

  const followingBtn = eventSection.getByTestId('event-following-event-exodus');
  await expect(followingBtn.locator('.popover-event-nav-label')).toHaveText('Crossing the Red Sea');
  // The FOLLOWING event's own real verse groups span EXO.14.21-31 -- already
  // proven at the wire level above (position.following.verse_groups ==
  // redSeaSceneEvent.verse_groups, the test's own "one-graph" assertion);
  // P4 means that span is never echoed into the UI arrow itself anymore.
  await expect(eventSection.getByTestId('event-following-verse-exodus')).toHaveCount(0);
  await expect(eventSection.getByTestId('event-following-label-exodus')).toHaveText('FOLLOWING EVENT');
  await expect(followingBtn.locator('.popover-event-nav-label')).toHaveAttribute('title', 'Crossing the Red Sea');
});

test('EVENT-1: MULTI-NARRATIVE nav -- an event touching >1 narrative shows one flanking nav row per narrative, each named', async ({ page }) => {
  // Find a real compiled event whose own narrative_positions span >1
  // narrative (most events belong to exactly one -- the single-narrative
  // case is already the norm asserted above; this isolates the real,
  // rarer multi-membership case the code's own occurrences/idSuffix
  // disambiguation exists for).
  const narratives = await api.narratives();
  let target: { eventId: string; positions: any[] } | null = null;
  outer: for (const n of narratives) {
    for (const legId of n.legs) {
      const positions = (await api.narrativeEventPositions(legId)).narrative;
      if (positions.length > 1) {
        target = { eventId: legId, positions };
        break outer;
      }
    }
  }
  test.skip(!target, 'no sampled narrative leg belongs to >1 narrative');
  if (!target) return;

  const detail = await api.event(target.eventId);
  await page.goto('/'); // any page -- reached directly by event id below, no reader navigation needed
  // Open the EventNode directly the same way every recursive traversal
  // test in this file does: via its own first witness verse.
  const wv = detail.witnesses[0].verse_groups[0].verses[0];
  const [book, chapter, verse] = wv.split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId(`verse-event-${target.eventId}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const eventSection = page.getByTestId('popover-section-event-date-places');
  await expect(eventSection).toBeVisible();
  const narrativeLabels = eventSection.getByTestId('popover-event-nav-narrative');
  await expect(narrativeLabels).toHaveCount(target.positions.length);
  for (const position of target.positions) {
    await expect(narrativeLabels.filter({ hasText: position.narrative_name })).toHaveCount(1);
    if (position.prior) {
      await expect(eventSection.getByTestId(`event-prior-event-${position.narrative_id}`).locator('.popover-event-nav-label')).toHaveText(position.prior.label);
      // P4: the role caption's own idSuffix disambiguation (the SAME
      // `--N` suffix the button testid itself carries on a same-name
      // multi-narrative collision) survives the retired-verse-text
      // rebuild -- one caption per row, not one shared across all of them.
      await expect(eventSection.getByTestId(`event-prior-label-${position.narrative_id}`)).toHaveText('PRIOR EVENT');
    }
    if (position.following) {
      await expect(eventSection.getByTestId(`event-following-event-${position.narrative_id}`).locator('.popover-event-nav-label')).toHaveText(position.following.label);
      await expect(eventSection.getByTestId(`event-following-label-${position.narrative_id}`)).toHaveText('FOLLOWING EVENT');
    }
  }
});

test('EVENT-1: recursive traversal reaches both narrative ends -- no PRIOR at the first event, no FOLLOWING at the last', async ({ page }) => {
  const narratives = await api.narratives();
  const exodus = narratives.find((n: any) => n.id === 'exodus');
  test.skip(!exodus || exodus.legs.length < 2, 'exodus narrative not present or too short to walk');
  const firstLegId = exodus.legs[0];
  const lastLegId = exodus.legs[exodus.legs.length - 1];
  const firstLegLabel = (await api.narrativeEventPositions(firstLegId)).narrative.find((p: any) => p.narrative_id === 'exodus').event_label;
  const lastLegLabel = (await api.narrativeEventPositions(lastLegId)).narrative.find((p: any) => p.narrative_id === 'exodus').event_label;

  await page.goto('/read/EXO/13');
  await page.getByTestId('verse-line-20').click(); // ex_succoth (index 1)
  await page.getByTestId('verse-event-ex_succoth').click(); // land on the EventNode -- one prior-hop reaches the first leg (index 0)

  // Walk PRIOR once: ex_succoth -> ex_rameses, exodus.legs[0] itself.
  await page.getByTestId('event-prior-event-exodus').click();
  await expect(page.getByTestId('popover-title')).toHaveText(firstLegLabel);
  // The narrative's own FIRST leg -- no PRIOR arrow at all (conditional
  // presence, never a disabled stub), but it DOES have a FOLLOWING one
  // (back toward ex_succoth), proving this is the genuine start, not a
  // dead end. M-D3/U1: no more separate section to check -- the arrows
  // themselves (unchanged testids) are the presence signal now.
  await expect(page.getByTestId('event-prior-event-exodus')).toHaveCount(0);
  await expect(page.getByTestId('event-following-event-exodus')).toBeVisible();

  // Walk FOLLOWING back to ex_succoth, then keep walking forward
  // (discovering the chain's own real length rather than hardcoding a hop
  // count) until the narrative's own LAST leg -- following disappears.
  //
  // A bare `.count()` read has NO auto-retry (unlike `expect(...)`), so
  // checking it immediately after a click can catch LoadCurrent's own
  // documented "cleared, then filled" intermediate frame (ExplorerPopover.razor's
  // own comment: sections are wiped SYNCHRONOUSLY before the new node's
  // async fetch resolves) and misread a genuinely-present following button
  // as absent, exiting the loop early -- the exact class of bug
  // batch-r-report.md's own "third real bug" writeup already documents for
  // a different section. Waiting for `popover-section-event-date-places`
  // (ALWAYS present, unconditionally, once ANY EventNode's own LoadCurrent
  // has actually settled) after each click before re-checking the count is
  // the fix -- the same "wait for a real settled signal, never a raw
  // one-shot read" discipline READER-1/BLINK-1's own tests already apply
  // elsewhere in this file.
  let guard = 0;
  while (await page.getByTestId('event-following-event-exodus').count() > 0) {
    await page.getByTestId('event-following-event-exodus').click();
    await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
    guard++;
    expect(guard, 'exodus narrative traversal did not terminate within a sane number of hops').toBeLessThan(exodus.legs.length + 2);
  }

  await expect(page.getByTestId('popover-title')).toHaveText(lastLegLabel);
  // The narrative's own LAST leg -- no FOLLOWING arrow, but a PRIOR one IS
  // present (this is genuinely the end, not merely a node missing one leg).
  await expect(page.getByTestId('event-following-event-exodus')).toHaveCount(0);
  await expect(page.getByTestId('event-prior-event-exodus')).toBeVisible();
});

test('EVENT-1: the three-narrative full walk -- FOLLOWING hop by hop to the last event, then PRIOR back to the first (requirement 6, verbatim acceptance)', async ({ page }) => {
  // "we can fully walk the graph for three independent narratives" -- three
  // narratives with >=3 legs each, chosen from the 13 compiled (named in
  // the report): exodus, jesus-ministry, passion-week. Hop count is read
  // from the wire (narratives[].legs.length), never hardcoded, so this test
  // keeps walking the FULL graph if a narrative grows.
  const narratives = await api.narratives();
  const chosen = ['exodus', 'jesus-ministry', 'passion-week'];
  for (const narrativeId of chosen) {
    const narrative = narratives.find((n: any) => n.id === narrativeId);
    expect(narrative, `${narrativeId} must be a real compiled narrative`).toBeTruthy();
    expect(narrative.legs.length, `${narrativeId} must have >=3 legs`).toBeGreaterThanOrEqual(3);

    const firstId = narrative.legs[0];
    const lastId = narrative.legs[narrative.legs.length - 1];
    const firstLabel = (await api.narrativeEventPositions(firstId)).narrative.find((p: any) => p.narrative_id === narrativeId).event_label;

    // Open the FIRST event's own popover via its OWN first witness verse
    // (Reader.razor's own reader-heading click path is exercised separately
    // by the reader-headings spec; this test isolates the traversal-walk
    // acceptance itself, independent of where the popover was opened from).
    const firstDetail = await api.event(firstId);
    const firstWitnessVref = firstDetail.witnesses[0].verse_groups[0].verses[0];
    const fv = parseVerse(firstWitnessVref);
    await page.goto(`/read/${fv.book}/${fv.chapter}`);
    await page.getByTestId(`verse-line-${fv.verse}`).click();
    await page.getByTestId(`verse-event-${firstId}`).click();
    await expect(page.getByTestId('popover-title')).toHaveText(firstLabel);
    await expect(page.getByTestId(`event-prior-event-${narrativeId}`)).toHaveCount(0); // first event: no PRIOR (conditional presence)

    // Walk FOLLOWING hop by hop; hop count must equal legs.length - 1.
    let hops = 0;
    while (await page.getByTestId(`event-following-event-${narrativeId}`).count() > 0) {
      await page.getByTestId(`event-following-event-${narrativeId}`).click();
      await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible(); // destination popover renders
      hops++;
      expect(hops, `${narrativeId} FOLLOWING walk exceeded its own leg count`).toBeLessThanOrEqual(narrative.legs.length - 1);
    }
    expect(hops, `${narrativeId} FOLLOWING walk must visit every leg exactly once`).toBe(narrative.legs.length - 1);
    const lastLabel = (await api.narrativeEventPositions(lastId)).narrative.find((p: any) => p.narrative_id === narrativeId).event_label;
    await expect(page.getByTestId('popover-title')).toHaveText(lastLabel);
    await expect(page.getByTestId(`event-following-event-${narrativeId}`)).toHaveCount(0); // last event: no FOLLOWING (conditional presence)

    // Walk PRIOR all the way back -- same hop count, ending at the first event again.
    let backHops = 0;
    while (await page.getByTestId(`event-prior-event-${narrativeId}`).count() > 0) {
      await page.getByTestId(`event-prior-event-${narrativeId}`).click();
      await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
      backHops++;
      expect(backHops).toBeLessThanOrEqual(narrative.legs.length - 1);
    }
    expect(backHops, `${narrativeId} PRIOR walk must retrace every leg exactly once`).toBe(narrative.legs.length - 1);
    await expect(page.getByTestId('popover-title')).toHaveText(firstLabel); // back at the start, same event
    await expect(page.getByTestId(`event-prior-event-${narrativeId}`)).toHaveCount(0);
  }
});

// ---------------------------------------------------------------------
// EVENT-1: PARALLEL WITNESSES (requirement 4/7 -- "Crucifixion event shows
// 4 witness passages, each clamped to 2 verses, expandable") and
// chronological-vs-reading-order (requirement 2/7 -- "a JHN event whose
// FOLLOWING is not the next pericope in JHN").
// ---------------------------------------------------------------------

test('EVENT-1/PASSAGE-1: the Crucifixion event shows 4 witness passages under "PARALLEL ACCOUNTS", each clamped to 2 verses, each independently expandable to its own whole chapter', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);
  const books = detail.witnesses.map((w: any) => w.book).sort();
  expect(books).toEqual(['JHN', 'LUK', 'MAT', 'MRK']);
  // Every real witness here spans well over 2 verses -- the clamp
  // affordance must be genuinely exercised, not vacuously absent.
  for (const w of detail.witnesses) {
    const total = w.verse_groups.reduce((n: number, g: any) => n + g.verses.length, 0);
    expect(total, `${w.book}'s own witness must span >2 verses for this test to exercise the clamp`).toBeGreaterThan(2);
  }

  // Open the event via one of its own witness verses (a real navigation
  // path, not a synthetic direct-open) -- Matthew's own first verse.
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const firstVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter, verse] = firstVref.split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-pw_golgotha').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');

  // Query generically by testid PREFIX (never reconstruct the exact
  // "{book}.{chapter}.{from}-{to}" span string by hand in the test itself
  // -- that duplicates PassageGrouping.SpanRef's own formatting logic and
  // would make this test fragile to a harmless future change there; the
  // real assertion is "4 witness entries, each independently clamps and
  // expands," not "the span text matches this exact reconstruction").
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(4);

  // M-D4 fix round 1/P5 (owner, verbatim: "we're wasting real estate...
  // it's obvious where they're coming from already"): NO standalone
  // book-name caption renders under any of these four entries' own
  // reference headers anymore -- exactly the 4-Gospel, book-disambiguation
  // scenario WitnessUnitsResolver's own retired doc comment used to call
  // "genuinely load-bearing," now proven unnecessary: PassageList's own
  // ref-label (e.g. "MRK...") already names the book. One header per
  // account entry, the reference -- never a second line duplicating it.
  await expect(witnessesSection.locator('.popover-passage-caption')).toHaveCount(0);

  // O2 (owner live-preview correction, 2026-08-23) retired the per-entry
  // popover-passage-clamp-expand/-collapse toggle this test used to
  // exercise (see PassageList.razor's own O2 comment) -- "each clamped to 2
  // verses with independent expand/collapse" now means MiniReaderExpand's
  // own control (a RevealControls-driven arrow pair, popover-verse-expand/
  // -collapse{-ENTRY-ID}) is the SOLE remaining affordance, and it always
  // jumps straight to that witness's own WHOLE chapter -- never a partial
  // reveal of just this entry's own remaining clamped verses (a disclosed
  // simplification; see PassageList.razor's own header comment). The
  // compact `.popover-passage-verse-num` list and the expanded mini-reader's
  // own `popover-reader-verse-*` are mutually exclusive (MiniReaderExpand.razor's
  // own `@if (!_expanded)`), so "expanded" is verified the same
  // structural-swap way READER-1 already verifies it, not by counting the
  // SAME clamped list grow.
  //
  // Real, live-caught (not guessed): this event was opened via MATTHEW's
  // own first verse, so the reader is now ACTIVELY showing Matthew's own
  // chapter -- M-D3/U6's chapter-aware suppression (READER-1's own
  // established rule, `ViewStateService.MountedReaderChapter`) correctly
  // makes the MATTHEW witness entry's own popover-verse-expand UNCONDITIONALLY
  // ABSENT, the exact same way a verse-line-opened verse popover's own
  // affordance always is -- there is no book/chapter this event could be
  // opened FROM that doesn't land the reader on one of its own four
  // witnesses' books, so exactly one of the four is always structurally
  // suppressed this way; asserted explicitly below rather than avoided.
  for (let i = 0; i < 4; i++) {
    const entry = entries.nth(i);
    const entryTestId = await entry.getAttribute('data-testid');
    expect(entryTestId).toBeTruthy();

    await expect(entry.locator('[data-testid^="popover-passage-clamp-"]'), `witness ${i} carries no retired clamp-toggle testid`).toHaveCount(0);
    const versesBeforeExpand = await entry.locator('.popover-passage-verse-num').count();
    expect(versesBeforeExpand, `witness ${i} must show exactly 2 clamped verses`).toBe(2);

    if (entryTestId!.startsWith(`event-witness-${book}.`)) {
      // The reader's own current book -- the expand control is correctly,
      // structurally absent (M-D3/U6); nothing further to exercise here.
      await expect(entry.getByTestId(`popover-verse-expand-${entryTestId}`), `witness ${i} (${book}, the reader's own chapter) suppresses its expand affordance`).toHaveCount(0);
      await expect(entry.getByTestId(`popover-verse-collapse-${entryTestId}`)).toHaveCount(0);
      continue;
    }

    // Exact getByTestId, not a prefix locator -- popover-verse-expand-{id}
    // and its own always-paired popover-verse-expand-{id}-all sibling both
    // start with this same prefix (R-D3's own double-arrow button), which
    // would make a prefix locator ambiguous (strict-mode violation).
    const expandBtn = entry.getByTestId(`popover-verse-expand-${entryTestId}`);
    await expect(expandBtn, `witness ${i}'s own expand affordance`).toBeVisible();
    await expect(entry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(0);

    await expandBtn.click();
    const collapseBtn = entry.getByTestId(`popover-verse-collapse-${entryTestId}`);
    await expect(collapseBtn).toBeVisible();
    await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(0); // compact clamp view torn down
    const readerVerses = await entry.locator('[data-testid^="popover-reader-verse-"]').count();
    expect(readerVerses, `witness ${i} must show its own WHOLE chapter once expanded`).toBeGreaterThan(2);

    await collapseBtn.click();
    await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(2); // restored, compact clamp
    await expect(entry.locator('[data-testid^="popover-reader-verse-"]')).toHaveCount(0);
  }
});

test('EVENT-1: a single-witness event shows the one passage with no "PARALLEL ACCOUNTS" framing (requirement 4, n=1)', async ({ page }) => {
  // Batch T2 (owner's own live-review ruling, 2026-08-21): pw_emmaus,
  // this test's own original n=1 example, is no longer single-witness --
  // Mark 16:12-13 (also listed by Robertson for that section) is now
  // correctly curated as a genuine 2nd witness (the compiled KJV text is
  // the canon of witnesses; textual-critical dispute is never grounds for
  // omission -- see batch-t2-report.md). jm_temple_cleansing (John's own
  // FIRST cleansing, John 2:13-22) is genuinely, deliberately
  // single-witness -- Robertson's own table lists no Matthew/Mark/Luke
  // parallel for it -- and was Batch T's own original n=1 precedent
  // (batch-t-report.md's own words: "single witness (John alone)...
  // requirement 4's own 'no parallel framing when n=1' is exactly the
  // case this event demonstrates"), so it replaces pw_emmaus here as the
  // still-accurate n=1 acceptance case; the assertions themselves are
  // unchanged in shape.
  const detail = await api.event('jm_temple_cleansing');
  expect(detail.witnesses.length).toBe(1);
  expect(detail.witnesses[0].book).toBe('JHN');

  await page.goto('/read/JHN/2');
  await page.getByTestId('verse-line-13').click();
  await page.getByTestId('verse-event-jm_temple_cleansing').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus cleanses the temple for the first time');

  // Singular section id (event-witness, not event-witnesses) -- no eyebrow at all.
  await expect(page.getByTestId('popover-section-event-witness')).toBeVisible();
  await expect(page.getByTestId('popover-section-event-witnesses')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-witness').getByTestId('event-section-heading')).toHaveCount(0);
});

// ---------------------------------------------------------------------
// M-D1 requirement 3 (SPAN-NOT-ECHO, owner live report #4, verbatim: "it
// also is completely redundant to just show the verses associated with a
// container in the container's hover box. we should just see the passage
// span."): RED before this batch -- a single-witness container's own
// popover echoed its full verse-list text (clamped-to-2 + expand), the
// SAME rendering a multi-witness PARALLEL ACCOUNTS entry gets; GREEN after
// -- a compact span line only, click-to-expand-inline, no default text.
// ---------------------------------------------------------------------

test('M-D1 req 3: a single-witness event\'s popover shows its SPAN, never an enumerated own-verse-list echo', async ({ page }) => {
  const detail = await api.event('jj_bethel_dream');
  expect(detail.witnesses.length).toBe(1);

  // M-D3/U6, owner verbatim: "'read the whole chapter' affordance REMOVED
  // when already reading that chapter" -- jm_temple_cleansing's own single
  // witness (JHN.2, the event's own former subject here) is, structurally,
  // reachable ONLY via a verse WITHIN that same chapter (a single-witness
  // event's own membership can never be cited from any OTHER chapter), so
  // popover-verse-expand there is now unconditionally suppressed -- this
  // test's own concern (span-not-echo) needs the button PRESENT and
  // clickable, so it now uses jj_bethel_dream instead (ALSO single-witness,
  // GEN.28.11-19), reached the same real, live-verified way READER-1/
  // BLINK-1 immediately above reach a different-chapter node: explore a
  // real, stable cross-reference (GEN.12.8 -> GEN.28.19, votes=3) so the
  // event's own witness chapter is never the one the reader is already
  // showing.
  await page.goto('/read/GEN/12');
  await page.getByTestId('verse-line-8').click();
  // A real, live-caught race (not guessed): `.count()` is a plain snapshot,
  // it does NOT auto-retry the way `expect().toBeVisible()` does -- reading
  // it immediately after the click above can land BEFORE ExplorerPopover's
  // own async section-resolution has populated the DOM at all yet, wrongly
  // conclude "all" is absent (this file's own established "settle-wait
  // first" discipline, e.g. the entry-point-vs-general test below), and
  // fall back to a single MORE click that reveals only +2 -- not enough to
  // reach GEN.28.19 (this verse's own 6th-ranked xref). Waiting for `more`
  // to be VISIBLE first (an auto-retrying assertion) guarantees the WHOLE
  // reveal-controls row -- including "all," rendered in the SAME pass --
  // has actually landed before the count check below reads it. M-D4 fix
  // round 1/P2: same conditional-"all" fallback as popover-sections' own
  // BLINK-1/OnExplore tests above, now with the SAME settle-wait those two
  // already had (this was the one call site missing it).
  const more3 = page.getByTestId('xrefs-more');
  await expect(more3).toBeVisible();
  const xrefsAll3 = page.getByTestId('xrefs-more-all');
  if (await xrefsAll3.count() > 0) {
    await xrefsAll3.click();
  } else {
    await more3.click();
  }
  await page.getByTestId('xref-item-GEN.28.19').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.28.19');
  await page.getByTestId('verse-event-jj_bethel_dream').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  const section = page.getByTestId('popover-section-event-witness');
  await expect(section).toBeVisible();

  // The compact SPAN reference renders (the ref label PassageList.razor
  // always shows) --
  const entry = section.locator('[data-testid^="event-witness-"]');
  await expect(entry).toHaveCount(1);
  await expect(entry.locator('.popover-passage-ref-label')).toBeVisible();
  const entryTestId = await entry.getAttribute('data-testid');

  // -- but NOT an enumerated verse-list echo: no compact passage text, no
  // per-verse superscript numbers, no clamp toggle (nothing to clamp when
  // nothing renders by default).
  await expect(entry.locator('.popover-passage-text')).toHaveCount(0);
  await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(0);
  await expect(entry.locator('[data-testid^="popover-passage-clamp-"]')).toHaveCount(0);

  // The span click STILL reads the passage inline -- the existing
  // MiniReaderExpand control, reused, not reimplemented (O2: now a
  // RevealControls-driven arrow pair -- see that component's own O2
  // comment). Exact getByTestId, not a prefix locator: this entry's own
  // popover-verse-expand-{id} and its always-paired popover-verse-expand-
  // {id}-all sibling (R-D3's own double-arrow button) share this prefix,
  // which would otherwise make a prefix locator ambiguous.
  const expandBtn = entry.getByTestId(`popover-verse-expand-${entryTestId}`);
  await expect(expandBtn).toBeVisible();
  await expandBtn.click();
  await expect(entry.locator('[data-testid^="popover-verse-reader"]')).toBeVisible();
  await expect(entry.locator('.popover-reader-verse')).not.toHaveCount(0);
});

test('M-D1 req 3: a multi-witness event (Crucifixion) still shows other-book clamped text -- span-not-echo does not touch PARALLEL ACCOUNTS', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);

  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const firstVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter, verse] = firstVref.split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-pw_golgotha').click();

  const witnessesSection = page.getByTestId('popover-section-event-witnesses');
  await expect(witnessesSection).toBeVisible();
  await expect(witnessesSection.getByTestId('event-section-heading')).toHaveText('PARALLEL ACCOUNTS');
  // UNCHANGED by this batch -- every one of the 4 witnesses (this book's
  // own included) still shows real clamped text, not a span-only line.
  const entries = witnessesSection.locator('[data-testid^="event-witness-"]');
  await expect(entries).toHaveCount(4);
  for (let i = 0; i < 4; i++) {
    await expect(entries.nth(i).locator('.popover-passage-text')).toBeVisible();
    await expect(entries.nth(i).locator('.popover-passage-verse-num').first()).toBeVisible();
  }
});

test('EVENT-1: chronological-vs-reading-order -- a JHN-witnessed event\'s FOLLOWING is not the next pericope in John (requirement 2/6/7, the owner\'s own "John doesn\'t have everything in order")', async ({ page }) => {
  // pw_jerusalem_entry is witnessed by John (JHN.12.12-19); its own
  // chronological FOLLOWING (pw_temple_cleansing, Robertson section 129)
  // is witnessed ONLY by Matthew/Mark/Luke -- John never repeats a
  // Passion-week temple cleansing, having already told a distinct, earlier
  // one in John 2 (jm_temple_cleansing). Ground truth, at the wire level:
  const entryDetail = await api.event('pw_jerusalem_entry');
  const johnWitness = entryDetail.witnesses.find((w: any) => w.book === 'JHN');
  expect(johnWitness, 'pw_jerusalem_entry must have a real John witness').toBeTruthy();

  const positions = (await api.narrativeEventPositions('pw_jerusalem_entry')).narrative;
  const passionWeek = positions.find((p: any) => p.narrative_id === 'passion-week');
  expect(passionWeek.following.id).toBe('pw_temple_cleansing');

  const cleansingDetail = await api.event('pw_temple_cleansing');
  expect(cleansingDetail.witnesses.some((w: any) => w.book === 'JHN'), 'pw_temple_cleansing must have NO John witness -- the whole point of this test').toBeFalsy();
  expect(cleansingDetail.witnesses.map((w: any) => w.book).sort()).toEqual(['LUK', 'MAT', 'MRK']);

  // Live, through the popover: open the John witness verse, traverse to
  // the event, then FOLLOWING -- lands on the temple-cleansing event, whose
  // OWN witness list (asserted above) proves this was never reachable by
  // "just keep reading John's text forward."
  const [book, chapter, verse] = johnWitness.verse_groups[0].verses[0].split('.');
  await page.goto(`/read/${book}/${chapter}`);
  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('verse-event-pw_jerusalem_entry').click();
  await expect(page.getByTestId('popover-title')).toHaveText('The triumphal entry into Jerusalem');

  const followingBtn = page.getByTestId('popover-section-event-date-places').getByTestId('event-following-event-passion-week');
  await expect(followingBtn.locator('.popover-event-nav-label')).toHaveText('Jesus cleanses the temple a second time');
  await followingBtn.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus cleanses the temple a second time');

  // This destination event's own PARALLEL ACCOUNTS never include John --
  // confirming, live, that the chronological target is NOT anything John's
  // own text narrates at all, let alone "the next pericope in JHN."
  const witnessSection = page.getByTestId(/^popover-section-event-witness/);
  await expect(witnessSection).toBeVisible();
  await expect(page.getByTestId(/^event-witness-JHN\./)).toHaveCount(0);
});
