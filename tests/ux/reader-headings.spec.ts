import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch T requirement 5 ("event titles throughout the reader" -- owner
// direction 2026-08-21, verbatim: "add the event titles throughout the
// reader... Headings are explorable... clicking opens the event/passage
// node"). See CONTRACT.md's own EVENT-1 note and the reader testid-
// inventory's own `pericope-heading-{eventId}` entry.

test('a covered book renders a pericope heading above its anchor verse, explorable, opens the EventNode', async ({ page }) => {
  // pw_golgotha's own Matthew witness anchors at MAT.27.33 (event-witnesses.toml).
  const detail = await api.event('pw_golgotha');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const anchorVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter] = anchorVref.split('.');

  const chapterOut = await api.chapter(`${book}.${chapter}`);
  const anchorVerse = chapterOut.verses.find((v: any) => `${book}.${chapter}.${v.verse}` === anchorVref);
  expect(anchorVerse.heading, 'the API itself must mark this verse as a heading anchor').toBeTruthy();
  expect(anchorVerse.heading.event_id).toBe('pw_golgotha');
  expect(anchorVerse.heading.title).toBe('The crucifixion at Golgotha');

  await page.goto(`/read/${book}/${chapter}`);
  const heading = page.getByTestId('pericope-heading-pw_golgotha');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The crucifixion at Golgotha');

  // Heading renders immediately ABOVE its own anchor verse's line (DOM
  // order, not just "present somewhere on the page").
  const headingBox = await heading.boundingBox();
  const verseBox = await page.getByTestId(`verse-line-${anchorVerse.verse}`).boundingBox();
  expect(headingBox && verseBox && headingBox.y).toBeLessThan(verseBox!.y);

  // Explorable (ONE-RULE): click opens a fresh EventNode popover.
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
});

test('a pericope heading is keyboard-explorable (Enter opens the popover, same as click)', async ({ page }) => {
  const detail = await api.event('pw_jerusalem_resurrection');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const anchorVref = matWitness.verse_groups[0].verses[0];
  const [book, chapter] = anchorVref.split('.');

  await page.goto(`/read/${book}/${chapter}`);
  const heading = page.getByTestId('pericope-heading-pw_jerusalem_resurrection');
  await expect(heading).toBeVisible();
  await heading.focus();
  await heading.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText('The empty tomb in Jerusalem');
});

test('THE COMPLETION MILESTONE: Romans 1 (the former "uncovered book" exemplar) now renders real pericope headings -- no uncovered book is left anywhere in the Bible', async ({ page }) => {
  // Batch W5 RULING (converts, rather than merely retargets, the test that
  // used to live here -- "an uncovered book/chapter shows no pericope
  // heading at all"). Lineage: W1 retargeted the original exemplar away
  // from Leviticus 5 once that batch covered Leviticus; W4 retargeted it a
  // second time, to Romans 1, once the entire Old Testament's own coverage
  // completed (Romans had ZERO touching events at all then -- verified
  // against the real compiled events.json before that retarget, per
  // batch-w4-report.md). Batch W5 completes the whole Bible's own
  // container coverage -- all 66 canonical books, all 31,102 KJV verses
  // (server/atlas-etl/tests/coverage.rs's own new
  // every_canonical_book_is_declared_and_the_whole_kjv_is_fully_covered
  // pins this fact against the real compiled data) -- so NO book anywhere
  // in the canon can ever again serve as a genuinely "uncovered" exemplar,
  // and the old fixture's own precondition is retired for good, not merely
  // moved to a new temporarily-uncovered book. This test asserts the
  // INVERSE, permanently: the SAME exemplar this whole test lineage has
  // always used, Romans 1, now carries real, heading-worthy containers,
  // reaching all the way to the rendered page -- the direct UI-facing
  // proof that this run's own coverage is real, not just a manifest entry.
  const chapterOut = await api.chapter('ROM.1');
  const headedVerses = chapterOut.verses.filter((v: any) => v.heading);
  expect(headedVerses.length, 'ROM.1 must now carry real heading-anchored verses on the wire (Batch W5 authors 4 containers inside Romans 1 alone)').toBeGreaterThanOrEqual(4);
  expect(headedVerses[0].heading.event_id).toBe('rom_salutation');
  expect(headedVerses[0].heading.title).toBe('Salutation: Paul, a servant of Jesus Christ, called to be an apostle');

  await page.goto('/read/ROM/1');
  const heading = page.getByTestId('pericope-heading-rom_salutation');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('Salutation: Paul, a servant of Jesus Christ, called to be an apostle');
  // Explorable (ONE-RULE), same as every other pericope heading in this
  // whole file -- the completion milestone is a real container, not a
  // decorative label.
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('Salutation: Paul, a servant of Jesus Christ, called to be an apostle');
});

test('a multi-witness event anchors a SEPARATE heading in each of its own witness books, all sharing the same title', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  expect(detail.witnesses.length).toBe(4);

  for (const w of detail.witnesses) {
    const anchorVref = w.verse_groups[0].verses[0];
    const [book, chapter] = anchorVref.split('.');
    await page.goto(`/read/${book}/${chapter}`);
    const heading = page.getByTestId('pericope-heading-pw_golgotha');
    await expect(heading, `${w.book}'s own witness must anchor its own heading`).toBeVisible();
    await expect(heading).toHaveText('The crucifixion at Golgotha');
  }
});

test('split view: the reader pane shows the SAME pericope heading (shared code path, requirement 5)', async ({ page }) => {
  const detail = await api.event('pw_golgotha');
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const [book, chapter] = matWitness.verse_groups[0].verses[0].split('.');

  await page.goto(`/read/${book}/${chapter}?split=1`);
  await expect(page.getByTestId('split-view')).toBeVisible();
  const heading = page.getByTestId('reader-root').getByTestId('pericope-heading-pw_golgotha');
  await expect(heading).toBeVisible();
  await expect(heading).toHaveText('The crucifixion at Golgotha');
  await heading.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The crucifixion at Golgotha');
  // The split's own atlas pane stays present and untouched (SPLIT-1) --
  // opening the heading's popover never disturbs the other pane.
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();
});

test('every curated event\'s reader heading text equals its own title exactly (a sample sweep, not just one event)', async ({ page }) => {
  // Every existing-narrative event already carries a title via Event.label
  // (no new authoring needed) -- spot-check a handful across different
  // narratives/books to confirm the heading-worthy rule reaches them too,
  // not just this batch's own newly-witnessed Gospel events.
  const sample = ['ex_red_sea', 'cq_jericho', 'pw_emmaus'];
  const narratives = await api.narratives();
  const knownIds = new Set(narratives.flatMap((n: any) => n.legs));
  for (const eventId of sample) {
    if (!knownIds.has(eventId)) continue; // skip gracefully if a curated id ever changes
    const detail = await api.event(eventId);
    const anchorVref = detail.witnesses[0].verse_groups[0].verses[0];
    const [book, chapter] = anchorVref.split('.');
    await page.goto(`/read/${book}/${chapter}`);
    const heading = page.getByTestId(`pericope-heading-${eventId}`);
    await expect(heading, `${eventId} must anchor a real reader heading`).toBeVisible();
    await expect(heading).toHaveText(detail.title);
  }
});

// Fix-round-1 (batch-t-review.md Important-1), amended by the owner's own
// "decisive container" ruling (2026-08-21): a real, live-reproduced defect
// -- JHN.12.1 is anchored by TWO titled containers (`jm_bethany`, a bare
// jesus-ministry leg, heading-worthy only via the "existing-title freebie"
// narrative-leg rule, no witnesses/robertson_section of its own,
// pre-existing; `pw_bethany`, a REAL curated container for this exact
// grouping, this batch's own flagship 3-witness passion-week leg) and the
// pre-fix code silently rendered `jm_bethany`'s own heading there, dropping
// `pw_bethany`'s John-witness heading entirely -- not caught by the sweep
// above (or anywhere else in this file), since every other test here
// happens to exercise a collision-free event. Overlap lives in the DATA
// (both containers legitimately cite this verse -- neither is wrong); the
// READER is decisive -- exactly one title renders here, chosen by
// `AtlasData::heading_precedence`'s own layer-then-kind-then-chronology
// rule (data.rs's own doc comment has the full chain). This test MUST fail
// on the pre-fix build (confirmed: red against the pre-fix first-wins
// `or_insert_with`, green after the precedence fix -- see data.rs's own
// `heading_collision_tests` for the Rust-level pin of the same mechanism).
test('fix-round-1/Important-1: a verse anchored by TWO titled containers renders the DECISIVE one\'s heading, and the other stays reachable via the verse\'s own EVENT section', async ({ page }) => {
  const COLLISION_VREF = 'JHN.12.1';
  const RICHER_EVENT_ID = 'pw_bethany'; // the REAL curated container -- wins layer tier 1
  const SHADOWED_EVENT_ID = 'jm_bethany'; // the bare "existing-title freebie" -- stays reachable, never chosen

  // Ground truth, at the wire level, before touching the UI at all.
  const chapterOut = await api.chapter('JHN.12');
  const anchorVerse = chapterOut.verses.find((v: any) => `JHN.12.${v.verse}` === COLLISION_VREF);
  expect(anchorVerse?.heading, 'JHN.12.1 must anchor SOME heading').toBeTruthy();
  expect(anchorVerse.heading.event_id, 'the REAL curated container (has witnesses) must win the collision over the bare freebie').toBe(RICHER_EVENT_ID);

  const verseDetail = await api.verse(COLLISION_VREF);
  const citingIds = verseDetail.events.map((e: any) => e.id).sort();
  expect(citingIds, 'both colliding events must still cite this verse in the EVENT membership list, win or lose the heading').toContain(SHADOWED_EVENT_ID);
  expect(citingIds).toContain(RICHER_EVENT_ID);

  // The reader itself: the winning event's own heading renders, the
  // shadowed one's own heading (same testid convention, different id) does
  // NOT render a SECOND heading at this verse.
  await page.goto('/read/JHN/12');
  await expect(page.getByTestId(`pericope-heading-${RICHER_EVENT_ID}`)).toBeVisible();
  await expect(page.getByTestId(`pericope-heading-${RICHER_EVENT_ID}`)).toHaveText('Mary anoints Jesus at Bethany');
  await expect(page.getByTestId(`pericope-heading-${SHADOWED_EVENT_ID}`)).toHaveCount(0);

  // The shadowed event stays reachable at the SAME verse, one click away,
  // via the verse popover's own EVENT membership section (Important-1's
  // own reason this is Important rather than Critical).
  await page.getByTestId('verse-line-1').click();
  await expect(page.getByTestId(`verse-event-${SHADOWED_EVENT_ID}`)).toBeVisible();
  await page.getByTestId(`verse-event-${SHADOWED_EVENT_ID}`).click();
  const shadowedDetail = await api.event(SHADOWED_EVENT_ID);
  await expect(page.getByTestId('popover-title')).toHaveText(shadowedDetail.title);
});
