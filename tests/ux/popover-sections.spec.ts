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

  // Section order: verse-text first, xrefs second (catechism, if present,
  // third -- this predicate only requires >0 xrefs, so a catechism section
  // may or may not also be showing; check structurally either way).
  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(sectionIds[0]).toBe('popover-section-verse-text');
  expect(sectionIds[1]).toBe('popover-section-xrefs');

  // Batch F2 requirement 6 (XREF-1): capped at 3 (xrefs-only) or 2 (THE
  // SMALL CATECHISM also present) on initial render.
  const hasCatechism = sectionIds.includes('popover-section-catechism');
  const cap = hasCatechism ? 2 : 3;
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
  await page.getByTestId(`verse-line-${v.verse}`).click();
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

  // Compact view first -- no mini-reader yet (requirement 4: "fetch on
  // expand, not before").
  await expect(page.getByTestId('popover-verse-reader')).toHaveCount(0);
  const expandBtn = page.getByTestId('popover-verse-expand');
  await expect(expandBtn).toHaveAttribute('aria-expanded', 'false');

  await expandBtn.click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();
  await expect(expandBtn).toHaveAttribute('aria-expanded', 'true');

  const chapter = await api.chapter('GEN.1');
  await expect(page.getByTestId(/^popover-reader-verse-/)).toHaveCount(chapter.verses.length);

  const focal = page.getByTestId('popover-reader-verse-3');
  await expect(focal).toHaveAttribute('data-focal', 'true');
  await expect(focal).toBeInViewport();
  await expect(page.getByTestId('popover-reader-verse-1')).toHaveAttribute('data-focal', 'false');

  // Collapse restores the exact compact view.
  await expandBtn.click();
  await expect(page.getByTestId('popover-verse-reader')).toHaveCount(0);
  await expect(expandBtn).toHaveAttribute('aria-expanded', 'false');
});

// A passage's own focal range highlights EVERY member verse, not just the
// first -- and the compact/aggregated text still matches READ-5/READ-6's
// own established "aggregate as today" behavior (requirement 3's own
// closing line).
test('READER-1: a passage\'s whole focal range is highlighted when expanded', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await page.getByTestId('verse-num-3').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('verse-num-5').click();
  await page.keyboard.up('Shift');
  await page.getByTestId('passage-chip').click();
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.3-5');

  await page.getByTestId('popover-verse-expand').click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

  for (const n of [3, 4, 5]) {
    await expect(page.getByTestId(`popover-reader-verse-${n}`)).toHaveAttribute('data-focal', 'true');
  }
  await expect(page.getByTestId('popover-reader-verse-2')).toHaveAttribute('data-focal', 'false');
  await expect(page.getByTestId('popover-reader-verse-6')).toHaveAttribute('data-focal', 'false');
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

  const detail = await api.place('jerusalem');
  await expect(page.locator('[data-testid^="place-event-"]')).toHaveCount(detail.events.length);
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
  const found = await findMentionableVerse(['JOS.6', 'JOS.10', 'GEN.13', 'GEN.19', 'EXO.14', 'JDG.7', '2SA.5', '1KI.3', 'GEN.12']);
  test.skip(!found, 'no candidate chapter had a literal, verse-linked place mention');
  if (!found) return;
  const { book, chapter, verse, placeId } = found;

  // Split view: reader + a LIVE atlas pane, both on one page (BLINK-1: "the
  // live map in split view"). Follow mode's own scripture scene for this
  // chapter lights every place whose verse_links intersect it, so the
  // mentioned place is guaranteed present, lit or quiet.
  await page.goto(`/read/${book}/${chapter}?split=1`);
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');

  const marker = page.getByTestId(`marker-${placeId}`).or(page.getByTestId(`quiet-marker-${placeId}`));
  await expect(marker).toBeAttached({ timeout: 15000 });

  await page.getByTestId(`verse-line-${verse}`).click();
  await page.getByTestId('popover-verse-expand').click();
  await expect(page.getByTestId('popover-verse-reader')).toBeVisible();

  const mention = page.getByTestId(`popover-reader-mention-${verse}-${placeId}`);
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

  // Section order: verse-text, then xrefs (if any), then catechism, per
  // REGISTRY-1's own VERSE ordering. batch-t-brief.md appends the "EVENT"
  // membership section AFTER catechism, conditionally (replacing Batch N's
  // own retired narrative PRIOR/FOLLOWING sections in this same slot) --
  // MAT.28.19 (the Great Commission) is within pw_galilee's own verse range
  // (MAT.28.16-20), so it's EVENT-linked in the real curated data, making
  // "catechism precedes the EVENT section" (not "catechism is
  // unconditionally the LAST section") the correct, still-precise check --
  // this stays correct regardless of whether a future data change adds or
  // removes this verse's own event membership.
  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(sectionIds[0]).toBe('popover-section-verse-text');
  const catechismIndex = sectionIds.indexOf('popover-section-catechism');
  expect(catechismIndex).toBeGreaterThan(-1);
  const eventIndex = sectionIds.indexOf('popover-section-event-membership');
  if (eventIndex !== -1) {
    expect(catechismIndex).toBeLessThan(eventIndex);
  } else {
    expect(catechismIndex).toBe(sectionIds.length - 1);
  }
});

test('CATECH-1: verse -> catechism item -> proof verse hop, with Luther\'s own verbatim heading', async ({ page }) => {
  await page.goto('/read/MAT/28');
  await page.getByTestId('verse-line-19').click();
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

  // A CatechismNode offers no chips at all (no geography).
  await expect(page.locator('.popover-chips')).toHaveCount(0);

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

test('XREF-1: a verse with only cross-reference context caps at 3 with a down-arrow reveal', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerseWithCounts(toc, d => d.cross_refs.length > 3 && d.catechism.length === 0);
  test.skip(!found, 'no sampled verse had >3 xrefs and 0 catechism');
  if (!found) return;
  const v = parseVerse(found.vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(found.vref);
  await expect(page.getByTestId('popover-section-catechism')).toHaveCount(0);

  const items = page.getByTestId(/^xref-item-/);
  await expect(items).toHaveCount(3);
  await expect(page.getByTestId('xrefs-more')).toBeVisible();
  await expect(page.getByTestId('xrefs-collapse')).toHaveCount(0);

  await page.getByTestId('xrefs-more').click();
  await expect(items).toHaveCount(found.detail.cross_refs.length);
  await expect(page.getByTestId('xrefs-collapse')).toBeVisible();
  await expect(page.getByTestId('xrefs-more')).toHaveCount(0);

  await page.getByTestId('xrefs-collapse').click();
  await expect(items).toHaveCount(3);
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
  await page.getByTestId(`verse-line-${v.verse}`).click();
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

  // Structurally impossible now: no PRIOR/FOLLOWING section exists on a
  // VERSE popover at all -- the whole family of retired testids matches
  // nothing, not merely "the specific one we used to check is absent."
  await expect(page.getByTestId(/^popover-section-event-prior/)).toHaveCount(0);
  await expect(page.getByTestId(/^popover-section-event-following/)).toHaveCount(0);
});

test('EVENT-1: a verse with no titled event shows no EVENT section at all (conditional presence)', async ({ page }) => {
  const toc = await loadToc();
  const found = await findVerse(toc, d => d.events.length === 0);
  test.skip(!found, 'no sampled verse had zero events');
  if (!found) return;
  const { vref } = found;
  const v = parseVerse(vref);

  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).click();
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

  // Exactly one qualifying narrative -> bare headings, no "-- name" suffix.
  const priorSection = page.getByTestId('popover-section-event-prior');
  await expect(priorSection).toBeVisible();
  await expect(priorSection.getByTestId('event-section-heading')).toHaveText('PRIOR EVENT');
  const priorBtn = priorSection.getByTestId('event-prior-event-exodus');
  await expect(priorBtn).toHaveText('Israel departs Rameses');
  await expect(priorSection.getByTestId('event-prior-verse-exodus-EXO.12.37')).toContainText('EXO.12.37');

  const followingSection = page.getByTestId('popover-section-event-following');
  await expect(followingSection).toBeVisible();
  await expect(followingSection.getByTestId('event-section-heading')).toHaveText('FOLLOWING EVENT');
  const followingBtn = followingSection.getByTestId('event-following-event-exodus');
  await expect(followingBtn).toHaveText('Crossing the Red Sea');
  // The adjacent event's FULL verse text renders via the shared passage-list
  // component -- same house rendering (ref + full KJV text together) every
  // other passage list in this app uses, never a truncated preview.
  const followingVerse = followingSection.getByTestId('event-following-verse-exodus-EXO.14.21-31');
  await expect(followingVerse).toBeVisible();
  await expect(followingVerse).toContainText('EXO.14.21-31');
  const chapter14 = await api.chapter('EXO.14');
  const v21Text = chapter14.verses.find((v: any) => v.verse === 21).text;
  await expect(followingVerse).toContainText(v21Text);
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
  // The narrative's own FIRST leg -- no prior section at all (conditional
  // presence, never a disabled stub), but it DOES have a following (back
  // toward ex_succoth), proving this is the genuine start, not a dead end.
  await expect(page.getByTestId('popover-section-event-prior')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-following')).toBeVisible();

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
  // The narrative's own LAST leg -- no following section, but a prior IS
  // present (this is genuinely the end, not merely a node missing one leg).
  await expect(page.getByTestId('popover-section-event-following')).toHaveCount(0);
  await expect(page.getByTestId('popover-section-event-prior')).toBeVisible();
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
    await expect(page.getByTestId('popover-section-event-prior')).toHaveCount(0); // first event: no PRIOR (conditional presence)

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
    await expect(page.getByTestId('popover-section-event-following')).toHaveCount(0); // last event: no FOLLOWING (conditional presence)

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
    await expect(page.getByTestId('popover-section-event-prior')).toHaveCount(0);
  }
});

// ---------------------------------------------------------------------
// EVENT-1: PARALLEL WITNESSES (requirement 4/7 -- "Crucifixion event shows
// 4 witness passages, each clamped to 2 verses, expandable") and
// chronological-vs-reading-order (requirement 2/7 -- "a JHN event whose
// FOLLOWING is not the next pericope in JHN").
// ---------------------------------------------------------------------

test('EVENT-1/PASSAGE-1: the Crucifixion event shows 4 witness passages under "PARALLEL ACCOUNTS", each clamped to 2 verses with independent expand/collapse', async ({ page }) => {
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

  for (let i = 0; i < 4; i++) {
    const entry = entries.nth(i);
    const entryTestId = await entry.getAttribute('data-testid');
    expect(entryTestId).toBeTruthy();

    const expandBtn = entry.locator('[data-testid^="popover-passage-clamp-expand-"]');
    await expect(expandBtn, `witness ${i}'s own clamp-expand affordance`).toBeVisible();
    const versesBeforeExpand = await entry.locator('.popover-passage-verse-num').count();
    expect(versesBeforeExpand, `witness ${i} must show exactly 2 clamped verses`).toBe(2);

    await expandBtn.click();
    const collapseBtn = entry.locator('[data-testid^="popover-passage-clamp-collapse-"]');
    await expect(collapseBtn).toBeVisible();
    const versesAfterExpand = await entry.locator('.popover-passage-verse-num').count();
    expect(versesAfterExpand, `witness ${i} must show all its own verses once expanded`).toBeGreaterThan(2);

    await collapseBtn.click();
    await expect(entry.locator('.popover-passage-verse-num')).toHaveCount(2); // restored
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

  const followingBtn = page.getByTestId('popover-section-event-following').getByTestId('event-following-event-passion-week');
  await expect(followingBtn).toHaveText('Jesus cleanses the temple a second time');
  await followingBtn.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Jesus cleanses the temple a second time');

  // This destination event's own PARALLEL ACCOUNTS never include John --
  // confirming, live, that the chronological target is NOT anything John's
  // own text narrates at all, let alone "the next pericope in JHN."
  const witnessSection = page.getByTestId(/^popover-section-event-witness/);
  await expect(witnessSection).toBeVisible();
  await expect(page.getByTestId(/^event-witness-JHN\./)).toHaveCount(0);
});
