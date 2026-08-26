import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';
import { formatRange } from './lib/years';
import { mergedVerses, groups, isPassage, initialShownCount, visibleGroups, spanRef } from './lib/hovercard';
import { independentlyHoverableIds } from './lib/hoverSafety';
import { LIT_MARKER_TESTID } from './lib/markers';
import { setZoomExact } from './lib/zoom';

// Batch C3: `/^marker-/` also matches `marker-cluster-{n}` (decision 3's
// own glyph testid, deliberately namespaced under the same `marker-`
// prefix as every other marker kind on this plate -- see CONTRACT.md's own
// `marker-cluster-{n}` note) -- lib/markers.ts's own LIT_MARKER_TESTID
// excludes it via a negative lookahead so this property keeps meaning
// exactly what it always did, "one element per PLACE," regardless of
// whether any of this window's own places happen to be clustered together
// this pass. Every place's own `marker-{placeId}` element stays ATTACHED
// (never removed) even while hidden inside a cluster (map.js's own
// applyMarkerClusters comment) -- toBeAttached below is unaffected either
// way.
test('WORLD-1: rendered markers equal the API scene', async ({ page }) => {
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.goto(`/world?from=${w.from}&to=${w.to}`);
    const scene = await api.sceneTime(w.from, w.to);
    await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
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
// (map.js's NUDGE_TRIGGER_PX/NUDGE_STEP_PX comments have the full
// empirical derivation) makes precise sub-14px disambiguation impossible
// BY DESIGN -- a deliberate accessibility floor, not a defect -- which the
// OLD 4x4px marker never needed to clear. This exact window lights a real
// six-member geographic cluster (Ai/Gilgal/Jericho/"plains of Moab"/
// Shittim/Timnath-serah -- genuinely different places, 18-58km apart in
// reality) that its own fitScene zoom compresses to single-digit screen
// pixels -- STILL a real structural fact about this scene's own geometry
// at that zoom (a nudge, being bounded to ~20px, genuinely cannot spread
// six markers piled onto a handful of pixels into six independently
// hoverable dots) -- but Batch C3 (dense-marker disambiguation +
// clustering) retired the CONSEQUENCE this comment used to describe:
// hover no longer silently guesses at this pileup. At this scene's own
// far/mid label tier it collapses into ONE `marker-cluster-{n}` glyph
// (map.js's CLUSTER_D_PX) whose hover opens a `place-chooser` listing
// every member (world-cluster-chooser.spec.ts's own CLUSTER-1); at NEAR
// tier the six markers render individually again and any residual
// close-but-real pair (a Philippi/Neapolis-class case -- see lib/
// hoverSafety.ts's own header comment for that pair's own root cause,
// Leaflet's default per-marker z-index-by-screen-Y stacking, not DOM
// order) resolves DETERMINISTICALLY to whichever candidate's TRUE
// position the pointer is nearest (map.js's resolveHoverTarget), never by
// z-order luck -- world-cluster-chooser.spec.ts's own ARBITRATION-1/
// CHOOSER-1 cover both outcomes directly. This property below is
// unaffected by any of that -- it is about hover -> card-CONTENT
// correctness once a hover has already landed somewhere, not about which
// marker a hover resolves to -- and continues to draw only from places
// independentlyHoverableIds (lib/hoverSafety.ts, shared with
// world-hover-text.spec.ts's own affected searches) confirms against this
// run's own live rendered positions; C3 extended that filter (a
// correctness fix, not a loosening) to also check quiet-dot/cluster-glyph
// proximity, not just other lit markers -- see CONTRACT.md's own amended
// "Marker hover-target resolution" note.

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
      // CONTRACT.md: place-card-title shows the scene's own display_name,
      // not the plain default name -- the two only ever coincided by
      // accident before Batch E2's own display-name suffix cleanup (folded
      // into resolve_display_name): a place like "red-sea-1" carries
      // p.name "Red Sea 1" (a raw ETL slug-disambiguation suffix) but
      // p.display_name (and therefore the card) "Red Sea" -- asserting
      // against p.name here would fail for any such place now that the fix
      // is live, not a hover-targeting bug.
      await expect(page.getByTestId('place-card-title')).toHaveText(p.display_name);

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
  await page.waitForSelector(`[data-testid="marker-${rigged[0].id}"]`, { state: 'attached' });

  // Batch C3: three EXACTLY coincident points (0px true separation, by
  // construction) sit inside decision 3's own CLUSTER_D_PX at ANY far/mid
  // tier zoom -- their true distance can never grow no matter how far in
  // this zooms, only the TIER GATE decides whether they cluster (unlike a
  // merely-close real pair). This property is specifically about golden-
  // angle nudge SLOT distinctness, which only applies once the trio
  // renders individually -- ZOOM_TIER_NEAR (map.js) is 9, so a direct jump
  // there (lib/zoom.ts's own setZoomExact, precise -- a real scroll-wheel's
  // own whole-level steps are unnecessary here, this test doesn't care
  // which zoom, only that it's unambiguously NEAR tier) lands past decision
  // 3's own "NEAR tier never clusters" gate, where nudging alone (unchanged
  // pre-C3 behavior) is exactly what this property has always exercised.
  await setZoomExact(page, 10);
  await expect(page.locator('[data-testid^="marker-cluster-"]')).toHaveCount(0);

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
// Batch E2 (the ever-present graph) scope amendment (user direction, this
// batch, mid-flight: "have all biblically relevant names of places showing
// on the map at all times... zooming in reveals what collision dropped"):
// PLACE labels (lit -- this test's own subject -- and quiet both) no longer
// have ANY zoom-tier gate at all, only collision damping -- "zoom-tiered
// label density" now describes LANDMARKS/POLITY LABELS ONLY (map.js's own
// ZOOM_TIER_MID/NEAR comment has the amended rule). This test is UPDATED,
// not retired: it still proves real, live behavior differences between a
// wide-spread and a tight scene, just landmark-only for the tier half, and
// a NEW assertion for the place half -- a place label that a wide scene's
// OWN tier previously hid outright (Susa, isolated from any collision) is
// now VISIBLE there too, since nothing (no tier, no collision) suppresses it.
//
// A note on HOW WORLD-10 changes "zoom" without a manual zoom gesture (still
// relevant for the landmark half): map.js's applyLabelTier gates landmarks
// purely on `map.getZoom()`, which fitScene (map.js) sets from a plain
// bounds-fit of whichever scene is currently loaded -- a widely-spread scene
// needs a looser zoom to fit every marker than a tightly-clustered one.
// Picking two windows whose NATURAL fitScene zoom already lands on opposite
// sides of ZOOM_TIER_MID exercises the exact same production code path
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
test('WORLD-10a: place labels show at every zoom, collision damping only (no brightness-vs-tier gate left)', async ({ page }) => {
  // Full span (-4004..100): 200+ places across the entire biblical-world
  // lock. "Egypt" (brightness 5) and "Susa" (brightness 1, geographically
  // isolated in Persia/Elam -- no other PLACE label contests its cell; the
  // only kind that could is another place, since place priority always
  // beats landmark/quiet) both stay visible here -- Susa is the load-
  // bearing case: under the OLD tier rule it was hidden outright at this
  // window's FAR-tier zoom purely for being brightness < BRIGHT_LABEL_MIN,
  // with no tier gate left to do that anymore, only collision (which it
  // clears) decides.
  await page.goto('/world?from=-4004&to=100');
  // Batch C3: at this FAR-tier full-span density, "egypt" can now land
  // inside a `marker-cluster-{n}` glyph instead of rendering its own dot +
  // label (decision 3, an entirely separate, ORTHOGONAL layer from label
  // collision -- a clustered place never individually competes for a
  // label cell at all, superseded by the glyph). This property is
  // specifically about collision-damping priority, not clustering, so it
  // only asserts the label directly when egypt ISN'T clustered this pass;
  // when it is, the glyph itself is the correct "still shown" answer.
  const egyptClustered = !(await page.getByTestId('marker-egypt').isVisible());
  if (egyptClustered) {
    await expect(page.locator('[data-testid^="marker-cluster-"]'), 'egypt is clustered this pass -- its own glyph must still be on the plate').not.toHaveCount(0);
  } else {
    await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
      'brightness-5 place ("Egypt") visible -- always was, tier or not').toBeVisible();
  }
  // Batch C3: same clustering-is-orthogonal reasoning as egypt above --
  // "isolated from any COLLISION" (this test's own original claim) no
  // longer implies "never absorbed into a cluster GLYPH," a separate
  // mechanism decision 3 adds; confirmed live, Susa can cluster with
  // another Persia/Elam-area place at this same far-tier whole-span view.
  const susaClustered = !(await page.getByTestId('marker-susa').isVisible());
  if (susaClustered) {
    await expect(page.locator('[data-testid^="marker-cluster-"]'), 'susa is clustered this pass -- its own glyph must still be on the plate').not.toHaveCount(0);
  } else {
    await expect(page.getByTestId('marker-susa').locator('.atlas-label'),
      'brightness-1 place ("Susa"), isolated from any collision -- NO tier gate left to hide it now').toBeVisible();
  }

  // Gospels (-5..29, this batch's own new bare-/world default -- CONTRACT):
  // "Egypt" is a real place in BOTH scenes at different brightness per
  // window (a per-window event count, not a fixed property of the place) --
  // visible in both regardless, now that brightness only ever affects
  // COLLISION PRIORITY, never bare tier visibility.
  await page.goto('/world?from=-5&to=29');
  await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
    'brightness-2 place, visible here too').toBeVisible();
});

// WORLD-10b: landmark/polity tiering is explicitly UNCHANGED by this
// batch's scope amendment ("polity/landmark rules unchanged") -- but
// proving that live against the REAL full-span/gospels scenes (as WORLD-10
// did pre-amendment) is no longer reliable: with place labels now ALWAYS
// competing for a cell too (WORLD-10a above), a landmark that used to have
// an uncontested cell can legitimately lose that SAME cell to a nearby
// place label the tier gate no longer keeps out of the race -- confirmed
// live while writing this test: "landmark-euphrates" (water-kind, always
// visible pre-amendment) is hidden at the real full-span scene's default
// viewport, beaten by a now-always-on nearby place label, which is
// COLLISION DAMPING working exactly as intended (batch-e2-brief.md's own
// second amendment: "the plate must still breathe through pruning") --
// NOT a landmark-tiering regression. Isolating the check from that (real,
// working-as-designed) confound needs a scene with NO place labels in the
// race at all -- `places: []` (route-mocked, same black-box "real /api/
// scene response, edited" technique WORLD-3/WORLD-12 already use, never a
// client-internals import) removes every place-label competitor so the
// landmark's own tier gate is the only thing left deciding its visibility.
// fitScene (map.js) no-ops when `markers.size === 0`, so the map's zoom
// stays wherever a fresh page load leaves it (DEFAULT_ZOOM 5, itself
// already < ZOOM_TIER_MID's 6 -- the FAR tier this test's first half needs).
test('WORLD-10b: landmark labels are still zoom-tiered, isolated from place-label collision competition', async ({ page }) => {
  const full = await api.sceneTime(-4004, 100);
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === '-4004' && url.searchParams.get('to') === '100',
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify({ ...full, places: [], quiet_places: [], arrows: [], narratives: [] }),
    }));
  await page.goto('/world?from=-4004&to=100');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(0); // confirms the mock actually applied
  await expect(page.getByTestId('landmark-euphrates'),
    'water-kind landmark visible at every tier, incl. FAR -- landmark tiering unchanged, no place-label competition to lose to here').toBeVisible();
  await expect(page.getByTestId('landmark-mount-sinai'),
    'mountain-kind landmark hidden below the MID tier -- landmark tiering unchanged').toBeHidden();

  // The positive transition (FAR -> MID reveals mountain-kind landmarks)
  // needs an actual zoom change -- fitScene never provides one here (it
  // no-ops on the empty-places mock above), so this drives Leaflet's own
  // scroll-wheel zoom directly (not the unreliable-to-click top-left
  // control -- see this suite's own established reasoning for avoiding
  // it), centered on the viewport, same real user gesture a person would
  // use. 3 notches from DEFAULT_ZOOM (5) reliably clears ZOOM_TIER_MID (6),
  // confirmed empirically while writing this test.
  const otherWindow = await api.sceneTime(-5, 29);
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === '-5' && url.searchParams.get('to') === '29',
    route => route.fulfill({
      status: 200,
      contentType: 'application/json',
      headers: { 'Access-Control-Allow-Origin': '*' },
      body: JSON.stringify({ ...otherWindow, places: [], quiet_places: [], arrows: [], narratives: [] }),
    }));
  await page.goto('/world?from=-5&to=29');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(0);
  await expect(page.getByTestId('landmark-mount-sinai'), 'still hidden at the untouched default (FAR) zoom').toBeHidden();
  for (let i = 0; i < 3; i++) {
    await page.mouse.move(640, 360);
    await page.mouse.wheel(0, -300);
  }
  await expect(page.getByTestId('landmark-mount-sinai'),
    'mountain-kind landmark now visible after crossing into the MID tier -- landmark tiering unchanged').toBeVisible();
});

// Fix round 1 (M3, review MAJOR 3): polity-label-{slug} (CONTRACT, added by
// this batch's BorderLayer) was previously exercised by no spec at all.
// Windows picked against the real curated polity eras (Batch B2's own
// per-polity timerange model, data/curated/polities/*.toml, replacing the
// original snapshot-year model this test was first written against):
// -3000..-2900 intersects ONLY sumer's own single era (-4004..-2004) among
// these two polities' eras; 40..60 intersects ONLY roman-empire's own
// "Roman Empire" era (-30..100) -- a real, unambiguous swap, not just "a
// label happens to still be there" after the window moves. In-app
// navigation via the readout (not a reload) is the same "window moves"
// path world-borders.spec.ts's own tests already use to prove the polity
// vector layer itself swaps.
test('WORLD-11: polity labels render from the active polity eras and swap when the window moves to a different one', async ({ page }) => {
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
  // FIX ROUND 1 CORRECTION: was `{from: -5, to: 29}` -- the bare-/world
  // default's own end moved to 33 (nt_calibration, CONTRACT's own GLOBAL
  // TIMELINE note) once the Gospel narrative's real end (the Ascension)
  // needed to fit inside it; `sea-of-galilee`'s own lighting event
  // (`theo-394`/`rob_walks_on_water`, both post-calibration) now falls at
  // year 32, inside 33 but outside the old 29.
  const w = { from: -5, to: 33 }; // Gospels default window (CONTRACT)
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
