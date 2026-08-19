import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';
import { formatRange } from './lib/years';
import { mergedVerses, groups, isPassage, initialShownCount, visibleGroups, spanRef } from './lib/hovercard';
import { independentlyHoverableIds } from './lib/hoverSafety';

test('WORLD-1: rendered markers equal the API scene', async ({ page }) => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
    for (const p of scene.places) {
      await expect(page.getByTestId(`marker-${p.id}`)).toBeAttached();
    }
  }), RUNS_UI);
});

// Batch D (batch-d-brief.md) killed the old per-(book,chapter) count-row
// index this property used to check (verse-group-{BOOK}-{chapter}, summed
// via g.count) -- CONTRACT.md's "Hover place card content" note is the
// single authority now. This property is the broad, whole-scene-random
// counterpart to world-hover-text.spec.ts's own few, deliberately-chosen
// scenarios: it re-derives the expected initial shown-verse shape (4/2-
// initial rule) from the SAME scene JSON for every randomly drawn place in
// a rich window, via tests/ux/lib/hovercard.ts's black-box mirror of
// PlaceCard.razor's own grouping logic, and confirms neither the killed
// index nor its old expand button ever reappears. Guaranteed to fail
// against the old card: it never had a hover-passage-{SPAN} testid (no
// grouping existed at all) and always rendered verse-group-*/place-card-
// expand, the opposite of what's asserted below.
// Batch C2 (requirement 0b/0c): the ember marker's own >=14px hit target
// (map.js's NUDGE_STEP_DEG comment has the full empirical derivation) makes
// precise sub-14px disambiguation impossible BY DESIGN -- a deliberate
// accessibility floor, not a defect -- which the OLD 4x4px marker never
// needed to clear. nudgeCloseLatLng's own re-tuning fixes the two pairs it
// safely CAN (the exact Shittim/"plains of Moab" coincidence and Gilgal/
// Jericho, both separated to 29px+ by that fix alone) -- but NOT every
// close pair in this scene: map.js's own comment explains why
// CLOSE_THRESHOLD_KM deliberately stays narrow (raising it far enough to
// also reach Marah/Elim and Rephidim/Mount Sinai caused a confirmed
// regression in a DIFFERENT scene, Philippi/Neapolis in the apostolic
// window -- see lib/hoverSafety.ts's own header comment for the full root
// cause, Leaflet's own default per-marker z-index-by-screen-Y stacking, not
// DOM order) rather than growing to cover every close-but-real pair
// everywhere. This exact window also lights a real six-member geographic
// cluster (Ai/Gilgal/Jericho/"plains of Moab"/Shittim/Timnath-serah --
// genuinely different places, 18-58km apart in reality) that its own
// fitScene zoom compresses to single-digit screen pixels regardless of any
// nudge tuning (see map.js's own comment for why this is a structural
// limit, exhausted by a real grid search, not a tuning miss). So this
// property is about hover -> card-content correctness, not marker layout,
// and draws only from places independentlyHoverableIds (lib/hoverSafety.ts,
// shared with world-hover-text.spec.ts's own affected searches) confirms
// against this run's own live rendered positions.

test('WORLD-2: hover card matches scene data', async ({ page }) => {
  const w = { from: -1446, to: -1406 };                    // exodus window: rich scene
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  const safeIds = await independentlyHoverableIds(page, scene.places.map((p: any) => p.id));
  const safeIndices = scene.places.map((_p: any, i: number) => i).filter((i: number) => safeIds.has(scene.places[i].id));
  expect(safeIndices.length, 'expected at least one independently-hoverable place in the exodus scene').toBeGreaterThan(0);

  await fcAssert(fc.asyncProperty(
    fc.integer({ min: 0, max: safeIndices.length - 1 }), async idx => {
      const i = safeIndices[idx];
      const p = scene.places[i];
      await page.getByTestId(`marker-${p.id}`).hover({ force: true });
      const card = page.getByTestId('place-card');
      await expect(card).toBeVisible();
      await expect(page.getByTestId('place-card-title')).toHaveText(p.name);

      const verses = mergedVerses(p);
      const shown = initialShownCount(verses);
      const visible = visibleGroups(groups(verses), shown);

      await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(shown);
      for (const vref of verses.slice(0, shown)) {
        await expect(card.getByTestId(`hover-verse-${vref}`)).toBeVisible();
      }
      const passageGroups = visible.filter(isPassage);
      await expect(card.locator('[data-testid^="hover-passage-"]')).toHaveCount(passageGroups.length);
      for (const g of passageGroups) {
        await expect(card.getByTestId(`hover-passage-${spanRef(verses, g)}`)).toBeVisible();
      }

      // The killed bookkeeping never reappears.
      await expect(card.locator('[data-testid^="verse-group-"]')).toHaveCount(0);
      await expect(card.getByTestId('place-card-expand')).toHaveCount(0);

      await page.mouse.move(0, 0);
      await expect(card).toBeHidden();
    }), RUNS_UI);
});

// Fix round 1 (M3): map.js's nudgeCloseLatLng golden-angle-spreads places
// that land within CLOSE_THRESHOLD_KM of an already-placed one, but its
// pre-fix version compared each new candidate against already-placed
// markers' FINAL (already-nudged) coordinates rather than their ORIGINAL
// ones -- since a nudge always moves a marker further than the threshold,
// a 3rd (or 4th, ...) place exactly coincident with the first two only ever
// counted the still-unmoved first one as "close" and collapsed onto the
// SAME slot as the 2nd place. Non-manifesting with today's curated data
// (the only exact coincidence, Shittim/Moab-2, is a pair, not a triple --
// see map.js's own comment), so proven here via a real /api/scene response
// (guaranteed to already satisfy the server's wire contract) with three of
// its places' coordinates overwritten to one identical point. This mocks
// the HTTP API response, not a client-internals import -- CONTRACT.md:
// "The UX property suite couples ONLY to this contract... plus the HTTP
// API" -- map.js itself is exercised completely unmodified.
test('WORLD-3: three exactly-coincident places each land on a distinct marker slot', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: rich scene (WORLD-2's own choice)
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.places.length).toBeGreaterThanOrEqual(3);

  // Well inside BIBLICAL_WORLD_BOUNDS (map.js: lat 7.6-48.9, lon -10.9-71.4)
  // so the region lock / fitScene bounds-fit can't clip or distort it.
  const rigged = scene.places.slice(0, 3);
  for (const p of rigged) { p.lat = 33.0; p.lon = 36.0; }

  // A URL predicate, not a glob string -- Playwright glob patterns treat
  // `?` as a one-character wildcard, not a literal query-string separator,
  // which would make intent here easy to misread even though it happens to
  // still match.
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === String(w.from) && url.searchParams.get('to') === String(w.to),
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify(scene),
    }));

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const positions: string[] = [];
  for (const p of rigged) {
    const marker = page.getByTestId(`marker-${p.id}`);
    await expect(marker, `marker-${p.id} should still render at the rigged coincident point`).toBeAttached();
    const box = await marker.boundingBox();
    expect(box, `marker-${p.id} has no bounding box`).not.toBeNull();
    positions.push(`${Math.round(box!.x)},${Math.round(box!.y)}`);
  }
  expect(new Set(positions).size, `expected 3 pairwise-distinct marker slots, got positions ${positions.join(' | ')}`).toBe(3);
});

// Fix round 1 (M3, review MAJOR 3): the batch's two central new map.js
// mechanisms -- zoom-tiered label density and polity labels -- had zero
// spec coverage. WORLD-10/11 close that; WORLD-12 covers this fix round's
// OWN new dedupe mechanism (M1) on top, since it ships in the same round.
//
// A note on HOW WORLD-10 changes "zoom" without a manual zoom gesture:
// map.js's applyLabelTier gates purely on `map.getZoom()`, which fitScene
// (map.js) sets from a plain bounds-fit of whichever scene is currently
// loaded -- a widely-spread scene needs a looser zoom to fit every marker
// than a tightly-clustered one (applyLabelTier's own comment: "a big,
// spread-out scene... naturally LANDS in the FAR tier on its own"). Picking
// two windows whose NATURAL fitScene zoom already lands on opposite sides
// of ZOOM_TIER_MID exercises the exact same production code path
// (map.getZoom() read at render time) a scripted zoom would, without
// depending on one: Leaflet's own top-left zoom control sits partly under
// this page's fixed dusk header at typical viewports and is unreliable to
// click (confirmed while writing this test -- Playwright's own
// actionability check timed out, retrying against "header intercepts
// pointer events"), and a keyboard-driven +/- alternative proved timing-
// sensitive across rapid presses in the same investigation. Two natural
// scenes sidesteps both, and is arguably more representative of how a real
// user actually reaches each density (visiting a wide-span window vs a
// narrow one), not less.
test('WORLD-10: zoom-tiered label density -- a wide-spread scene shows only the brightest place/water labels; a tight scene shows the fuller set', async ({ page }) => {
  // Full span (-4004..100): 200+ places across the entire biblical-world
  // lock (Table-of-Nations entries included, e.g. Punt/Tarshish/India --
  // see map.js's own BIBLICAL_WORLD_BOUNDS comment) -- fitScene's own
  // bounds-fit zoom for a scene this spread out lands well under
  // ZOOM_TIER_MID (6), the FAR tier, confirmed empirically while writing
  // this test (not assumed) against the real running app.
  await page.goto('/world?from=-4004&to=100');
  await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
    'brightness-5 place ("Egypt") stays visible at the FAR tier').toBeVisible();
  await expect(page.getByTestId('marker-susa').locator('.atlas-label'),
    'brightness-1 place ("Susa"), isolated from any collision, hidden at the FAR tier').toBeHidden();
  await expect(page.getByTestId('landmark-euphrates'),
    'water-kind landmark visible at every tier, incl. FAR').toBeVisible();
  await expect(page.getByTestId('landmark-mount-sinai'),
    'mountain-kind landmark hidden below the MID tier').toBeHidden();

  // Gospels (-5..29, this batch's own new bare-/world default -- CONTRACT):
  // a small, tightly-clustered scene (Galilee/Judea) whose fitScene zoom
  // lands in the MID/NEAR tier -- this is exactly the density MAJOR 1
  // itself reported on this same window. "Egypt" is a real place in BOTH
  // scenes but at a different brightness per window (brightness is a
  // per-window event count, not a fixed property of the place) -- here
  // it's 2, so this is the SAME place's label toggling from hidden (FAR,
  // full span) to shown (MID/NEAR, gospels) purely from the window/zoom
  // change. "Mount Sinai" (not narrated anywhere in the Gospels, so no
  // dedupe interaction with a lit place -- see WORLD-12) goes from hidden
  // to shown the same way.
  await page.goto('/world?from=-5&to=29');
  await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
    'brightness-2 place now visible -- the fuller MID/NEAR set').toBeVisible();
  await expect(page.getByTestId('landmark-mount-sinai'),
    'mountain-kind landmark now visible -- the fuller MID/NEAR set').toBeVisible();
});

// Fix round 1 (M3, review MAJOR 3): polity-label-{slug} (CONTRACT, added by
// this batch's BorderLayer) was previously exercised by no spec at all.
// Windows picked by probing every curated border snapshot's own rendered
// polity labels against the real running app (see the batch report): the
// snapshot nearest -3000..-2900 ("3000 BC") is the only one whose features
// include "Sumer"; the snapshot nearest 40..60 ("1 BC") is the only one
// with "Roman Empire" -- a real, unambiguous swap, not just "a label
// happens to still be there" after the window moves. In-app navigation via
// the readout (not a reload) is the same "window moves" path BORDERS-2
// already uses to prove the border vector layer itself swaps.
test('WORLD-11: polity labels render from the active border snapshot and swap when the window moves to a different one', async ({ page }) => {
  await page.goto('/world?from=-3000&to=-2900');
  await expect(page.getByTestId('polity-label-sumer')).toBeVisible();
  await expect(page.getByTestId('polity-label-sumer')).toHaveText('Sumer');
  await expect(page.getByTestId('polity-label-roman-empire')).toHaveCount(0);

  await page.getByTestId('slider-readout').fill(formatRange(40, 60));
  await page.getByTestId('slider-readout').press('Enter');
  await page.waitForURL(u => u.searchParams.get('from') === '40' && u.searchParams.get('to') === '60');

  await expect(page.getByTestId('polity-label-sumer')).toHaveCount(0);
  await expect(page.getByTestId('polity-label-roman-empire')).toBeVisible();
  await expect(page.getByTestId('polity-label-roman-empire')).toHaveText('Roman Empire');
});

// Fix round 1 (M1's own new mechanism, tested here alongside its sibling
// declutter coverage above): a landmark whose name coincides with a
// currently-lit place at the same location yields to the place -- see
// map.js's LANDMARK_DEDUPE_KM comment for the full rule. "Sea of Galilee"
// is both a curated landmark (water kind, data/curated/landmarks.toml) and
// a real lit place in the Gospels default window (data/compiled/
// places.json id "sea-of-galilee", identical lat/lon) -- a second, still-
// live instance of exactly MAJOR 1's reported "Mount Hermon renders twice"
// bug, in the same default scene "Mount Hermon" itself was reported on.
//
// Isolated to just this one place (WORLD-3's own mocked-response
// technique -- a real /api/scene response with its `places` trimmed, not a
// client-internals import, so this stays black-box per CONTRACT's own
// rule) rather than asserted against the full, real Gospels scene: the
// real scene also lights "Capernaum" ~7km away (brightness 5, so higher
// collision priority -- see WORLD-10's own collision-damping mechanism),
// close enough that Sea of Galilee's OWN place label can lose that
// SEPARATE, unrelated collision battle at the page's natural zoom --
// confirmed empirically while writing this test (both the place's own
// label AND the landmark were hidden together in the real scene, which
// would make a naive version of this test pass for the wrong reason: not
// because dedupe was proven, but because collision independently hid both
// candidates). Isolating the scene to just this one place removes every
// OTHER label that could contest its grid cell, so "the place's own label
// is visible" here can only be explained by clearing its own tier check --
// leaving "the landmark is hidden" attributable to dedupe alone, not a
// confound with collision damping.
test('WORLD-12: a landmark yields to a same-named, same-location lit place (no duplicate label)', async ({ page }) => {
  const w = { from: -5, to: 29 }; // Gospels default window (CONTRACT)
  const scene = await api.sceneTime(w.from, w.to);
  const sog = scene.places.find((p: any) => p.id === 'sea-of-galilee');
  expect(sog, 'expected "sea-of-galilee" to be a real lit place in the Gospels window').toBeTruthy();

  const rigged = { ...scene, places: [sog], arrows: [], narratives: [] };
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === String(w.from) && url.searchParams.get('to') === String(w.to),
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify(rigged),
    }));

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await expect(page.getByTestId('marker-sea-of-galilee').locator('.atlas-label'),
    'the lit place itself renders its own label, uncontested').toBeVisible();
  await expect(page.getByTestId('landmark-sea-of-galilee'),
    'the coincident water landmark yields to it').toBeHidden();
});
