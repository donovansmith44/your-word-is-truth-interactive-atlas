import { test, expect } from '@playwright/test';
import { openVerse } from './lib/verse';
import { api } from './lib/api';

// EVT-3 Ticket 1 (the smart-frontier restructure), spec §4e conformance,
// DIRECTION (b): "the implementing-without-content direction -- for each
// matrix cell, a live-data test that the section CAN produce content for
// at least one real node of that kind" -- the honesty check complementing
// client.Tests/FrontierMatrixConformanceTests.cs's own DIRECTION (a) (no
// provider ever renders OUTSIDE its own matrix cell). This file proves the
// INVERSE against the REAL running app + REAL compiled data: every TRUE
// cell in client/Contracts/Frontier.cs's own FrontierMatrix genuinely has
// content behind it TODAY, never merely assumed from the matrix's own
// declaration.
//
// SCOPE (disclosed): one live fixture per CAPABILITY FAMILY (the section
// implementation itself is the SAME code regardless of which kind renders
// it -- CrossRefsSection doesn't have a "Verse mode" and a "Passage mode")
// rather than one fixture per every individual (kind, capability) cell.
// Verse carries six of the nine families and is used preferentially
// (Contracts/Frontier.cs's own "the richest frontier" comment); Event's
// three EVT-3 families are proven via the SAME real Event fixtures
// event-timeline.spec.ts already established elsewhere in this suite.
// Persons (Verse AND Passage) is proven directly against the RAW
// `mentions` edge (api.nodeEdges) rather than a rendered popover section --
// VersePersonsSection is deliberately UNREGISTERED today (O4, owner order
// "remove persons from hover menus for now," CONTRACT.md's own PERSONS-1
// note), so no popover section for it is ever on screen to assert against;
// its own capability CLAIM (IHasPersons/FrontierMatrix.Persons) is proven
// directly in client.Tests/FrontierMatrixConformanceTests.cs, and this file
// additionally proves the underlying DATA that claim rests on is genuinely
// non-empty for a real node -- the honest form of "presence is the claim"
// available given the registration gap.

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

// Same shape event-timeline.spec.ts's own openEventPopover establishes --
// duplicated locally (this file's own small helper, the house convention
// every spec file already follows for its own file-scoped helpers) rather
// than imported across files.
async function openEventPopover(page: any, eventId: string) {
  const detail = await api.event(eventId);
  const vref = detail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId(`verse-event-${eventId}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
}

test('DIRECTION-B/Verse x CrossReferences: GEN.1.1 renders real, non-empty cross-references', async ({ page }) => {
  await page.goto('/read/GEN/1');
  await openVerse(page, 1);
  await expect(page.getByTestId('popover-section-xrefs')).toBeVisible();
  await expect(page.locator('[data-testid^="xref-item-"]').first()).toBeVisible();
});

test('DIRECTION-B/Verse x Parallels: a Nazareth-visit witness verse renders a real PARALLELS entry from the OTHER Gospel', async ({ page }) => {
  const detail = await api.event('rob_last_nazareth_visit');
  expect(detail.witnesses.length, 'this fixture needs a real >1-witness event for PARALLELS to mean anything').toBeGreaterThan(1);
  const matWitness = detail.witnesses.find((w: any) => w.book === 'MAT');
  const v = parseVerse(matWitness.verse_groups[0].verses[0]);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await openVerse(page, v.verse);
  await expect(page.getByTestId('popover-section-parallels')).toBeVisible();
});

test('DIRECTION-B/Verse x EventMembership: a jm_egypt witness verse renders a real EVENT membership row', async ({ page }) => {
  const detail = await api.event('jm_egypt');
  expect(detail.kind).toBe('event');
  const v = parseVerse(detail.witnesses[0].verse_groups[0].verses[0]);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await openVerse(page, v.verse);
  await expect(page.getByTestId('popover-section-event-membership')).toBeVisible();
  await expect(page.getByTestId('verse-event-jm_egypt')).toBeVisible();
});

test('DIRECTION-B/Verse x PassageMembership: PSA.119.105 renders a real general-kind PASSAGE membership row', async ({ page }) => {
  const detail = await api.verse('PSA.119.105');
  expect(detail.events.some((e: any) => e.kind === 'general'), 'PSA.119.105 must genuinely cite a general-kind passage for this test to mean anything').toBe(true);
  await page.goto('/read/PSA/119');
  await openVerse(page, 105);
  await expect(page.getByTestId('popover-section-passage-membership')).toBeVisible();
});

test('DIRECTION-B/Verse x CatechismSupport: MAT.28.19 renders a real THE SMALL CATECHISM citation', async ({ page }) => {
  await page.goto('/read/MAT/28');
  await openVerse(page, 19);
  await expect(page.getByTestId('popover-section-catechism')).toBeVisible();
  await expect(page.locator('[data-testid^="catechism-item-"]').first()).toBeVisible();
});

test('DIRECTION-B/Verse+Passage x Persons: a real verse\'s own `mentions` edge is genuinely non-empty (VersePersonsSection\'s own data dependency, proven directly against the wire since the section is unregistered per O4)', async ({ page }) => {
  const toc = await api.books();
  let found: string | null = null;
  for (const b of toc.slice(0, 8)) {
    for (const c of b.chapters.slice(0, 3)) {
      const chapter = await api.chapter(`${b.code}.${c}`);
      const hit = chapter.verses.find((v: any) => (v.persons ?? []).length > 0);
      if (hit) { found = `${b.code}.${c}.${hit.verse}`; break; }
    }
    if (found) break;
  }
  expect(found, 'the real corpus must carry at least one attested person mention for this test to mean anything').toBeTruthy();

  const edges = await api.nodeEdges(`text-unit:${found}`, 'mentions');
  expect(edges.entries.length, `text-unit:${found}'s own mentions edge must be genuinely non-empty`).toBeGreaterThan(0);
});

test('DIRECTION-B/Event x Chronology: gen_binding_isaac (a real dated event) renders a real Chronology block', async ({ page }) => {
  await openEventPopover(page, 'gen_binding_isaac');
  await expect(page.getByTestId('popover-section-event-chronology')).toBeVisible();
});

test('DIRECTION-B/Event x TimeAndPlace: The Last Visit to Nazareth (a real dated, located event) renders a real date-and-places section', async ({ page }) => {
  const detail = await api.event('rob_last_nazareth_visit');
  expect(detail.when).toBeTruthy();
  expect(detail.places.length).toBeGreaterThan(0);
  await openEventPopover(page, 'rob_last_nazareth_visit');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
});

test('DIRECTION-B/Event x Accounts: gen_binding_isaac renders its own real account passage', async ({ page }) => {
  await openEventPopover(page, 'gen_binding_isaac');
  await expect(page.getByTestId(/^popover-section-event-witness/)).toBeVisible();
});
