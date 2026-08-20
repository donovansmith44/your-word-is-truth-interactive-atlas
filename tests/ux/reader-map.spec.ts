import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { loadToc, arbVerseRef, arbChapterRef } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';
import { independentlyHoverableIds } from './lib/hoverSafety';

test('READ-4: mini-map equals scripture scene; open-in-world carries the ref', async ({ page }) => {
  const toc = await loadToc();
  await fcAssert(fc.asyncProperty(arbVerseRef(toc), async vref => {
    const [b, c, v] = vref.split('.');
    await page.goto(`/read/${b}/${c}`);
    // Batch G1: verse-num no longer opens a popover on plain click (that's
    // the verse LINE's own job now, see reader.spec.ts's own header note) --
    // this test's own subject is the mini-map chip inside the popover, not
    // how the popover got opened, so it just opens via the line instead.
    await page.getByTestId(`verse-line-${v}`).click();
    await page.getByTestId('popover-chip-map').click();
    await expect(page.getByTestId('mini-map')).toBeVisible();
    const scene = await api.sceneScripture(vref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
    await page.getByTestId('mini-map-open-world').click();
    await page.waitForURL(u => u.pathname === '/world' && u.searchParams.get('ref') === vref);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  }), RUNS_UI);
});

// Batch C2 (requirement 0b/0c): `scene.places[0]` used to be a safe,
// arbitrary pick regardless of which place it happened to be -- the OLD
// 4x4px marker never risked a forced hover landing on a NEIGHBORING
// marker instead. The ember marker's own bigger hit target (see
// lib/hoverSafety.ts's own header comment, and map.js's NUDGE_STEP_DEG
// comment, for the confirmed root cause and a real example) means that's
// no longer true for an UNCHECKED index-0 pick in a dense scene -- this
// window's own place[0] ("Ai") happens to sit within the exodus scene's
// documented Jericho/Ai/Gilgal/"plains of Moab" cluster. Picking the
// first REAL, independently-hoverable place instead keeps the test's own
// original intent ("some place in this scene, doesn't matter which")
// honest against the live rendered page rather than an arbitrary array
// index.
test('WORLD-8: place card title opens place history popover', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1406');
  const scene = await api.sceneTime(-1446, -1406);
  const safeIds = await independentlyHoverableIds(page, scene.places.map((sp: any) => sp.id));
  const p = scene.places.find((sp: any) => safeIds.has(sp.id));
  expect(p, 'expected at least one independently-hoverable place in the exodus scene').toBeTruthy();
  await page.getByTestId(`marker-${p.id}`).hover({ force: true });
  await page.getByTestId('place-card-title').click();
  // World.razor's OpenPlaceFromCard builds the PlaceNode from
  // hoverPlace.DisplayName, not .Name -- CONTRACT.md: popover-title (like
  // place-card-title/marker labels) shows the scene's own display_name.
  // The two only coincide when this place has no curated period name AND
  // (Batch E2) no raw ETL slug-disambiguation suffix in its default name.
  await expect(page.getByTestId('popover-title')).toHaveText(p.display_name);
  const detail = await api.place(p.id);
  await expect(page.getByTestId('popover'))
    .toContainText(String(Math.abs(detail.events[0].when.from_year)));
});

test('READ-5: shift-click passage selection', async ({ page }) => {
  const toc = await loadToc();
  const arb = arbChapterRef(toc).filter(c => c.verses >= 4).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses - 1 }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b).map(([a, b]) => ({ ...c, a, b })));
  await fcAssert(fc.asyncProperty(arb, async s => {
    await page.goto(`/read/${s.book}/${s.chapter}`);
    await page.getByTestId(`verse-num-${s.a}`).click();
    await page.keyboard.down('Shift');
    await page.getByTestId(`verse-num-${s.b}`).click();
    await page.keyboard.up('Shift');
    const pref = `${s.book}.${s.chapter}.${s.a}-${s.b}`;
    await expect(page.getByTestId('passage-chip')).toContainText(pref);
    await page.getByTestId('passage-chip').click();
    await expect(page.getByTestId('popover-title')).toHaveText(pref);
    await page.getByTestId('popover-chip-map').click();
    const scene = await api.sceneScripture(pref);
    await expect(page.getByTestId('mini-map').locator('[data-testid^="marker-"]'))
      .toHaveCount(scene.places.length);
  }), RUNS_UI);
});

// Batch G1 requirement 2 ("passage context -- passages give xrefs, not just
// geo"): the passage-chip's own popover (a PassageNode) shows cross-refs
// CONDITIONALLY, backed by GET /api/xrefs/{sref}'s span-aggregation. Both
// branches (present/absent) are real, reachable outcomes depending on the
// random passage's own real cross-ref data, so this property asserts
// whichever one actually holds for each generated passage rather than
// assuming either. Batch R (requirement 3(b)): the old popover-chip-xrefs
// TOGGLE is gone -- xref-item-* now renders INLINE, immediately, with no
// button press, via the registry's own CrossRefsSection (conditional
// presence: zero items when the endpoint has none, not an absent chip).
test('READ-6: passage cross-references render inline, conditional on the endpoint actually having targets', async ({ page }) => {
  const toc = await loadToc();
  const arb = arbChapterRef(toc).filter(c => c.verses >= 4).chain(c =>
    fc.tuple(fc.integer({ min: 1, max: c.verses - 1 }), fc.integer({ min: 1, max: c.verses }))
      .filter(([a, b]) => a < b).map(([a, b]) => ({ ...c, a, b })));
  await fcAssert(fc.asyncProperty(arb, async s => {
    const pref = `${s.book}.${s.chapter}.${s.a}-${s.b}`;
    const xrefs = await api.xrefs(pref);

    await page.goto(`/read/${s.book}/${s.chapter}`);
    await page.getByTestId(`verse-num-${s.a}`).click();
    await page.keyboard.down('Shift');
    await page.getByTestId(`verse-num-${s.b}`).click();
    await page.keyboard.up('Shift');
    await page.getByTestId('passage-chip').click();
    await expect(page.getByTestId('popover-title')).toHaveText(pref);

    const items = page.getByTestId(/^xref-item-/);
    await expect(items).toHaveCount(xrefs.length);
    for (let i = 0; i < xrefs.length; i++) {
      await expect(items.nth(i)).toContainText(xrefs[i].target);
    }
  }), RUNS_UI);
});
