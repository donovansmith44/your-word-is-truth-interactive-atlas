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

  // Section order: verse-text, then xrefs (if any), then catechism -- last,
  // per REGISTRY-1's own VERSE ordering (verse-text, cross-references,
  // catechism).
  const sectionIds = await page.getByTestId(/^popover-section-/).evaluateAll(els => els.map(el => el.getAttribute('data-testid')));
  expect(sectionIds[sectionIds.length - 1]).toBe('popover-section-catechism');
  expect(sectionIds[0]).toBe('popover-section-verse-text');
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
