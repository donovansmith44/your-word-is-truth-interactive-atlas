import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { independentlyHoverableIds } from './lib/hoverSafety';

// Batch G1 requirement 3 (click-to-pin/narrative traversal). PIN-1/TRAVERSAL-1
// in CONTRACT.md. The exodus window (-1446..-1406) is this suite's own
// long-standing "rich scene" pick (world-map.spec.ts's WORLD-2, world-hover-
// text.spec.ts's WINDOWS) -- here it doubles as the brief's own suggested
// "known narrative chain": the curated `exodus` narrative's legs (rameses ->
// succoth -> red sea -> marah -> elim -> rephidim -> sinai -> kadesh-barnea ->
// plains of moab -> jericho) all fall inside this exact window, so every one
// of its places is lit (not quiet) here.
const EXODUS_WINDOW = { from: -1446, to: -1406 };

// Real, confirmed risk in THIS exodus scene specifically (app.css's own
// .atlas-marker comment, lib/hoverSafety.ts's header comment: "the exodus
// scene alone measures a majority of its own places mutually within [14px] --
// 75%, 12 of 16"): a pixel-coordinate click/hover (even with Playwright's own
// force:true, which only skips PLAYWRIGHT's actionability pre-check, not the
// BROWSER's real hit-test at that pixel) can land on a DIFFERENT, overlapping
// marker than the one asked for. dispatchEvent bypasses hit-testing entirely
// -- it fires the DOM event directly ON the target element, still exercising
// the real production Leaflet click listener (nothing about the app is
// mocked), the exact technique world-quiet-places.spec.ts's own Jerusalem/
// Beautiful-gate coincidence test already established for this same class of
// problem (there for an exact coincidence; here for the exodus cluster's own
// documented near-coincidences).
async function clickMarker(page: import('@playwright/test').Page, placeId: string): Promise<void> {
  await page.getByTestId(`marker-${placeId}`).dispatchEvent('click');
}

test('PIN-1: clicking a marker pins its card open, ignores hover elsewhere, and closes via its own X', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const legs = scene.arrows.filter((a: any) => a.narrative === 'exodus').sort((a: any, b: any) => a.order - b.order);
  test.skip(legs.length === 0, 'exodus narrative not present in this window');
  const start = legs[0].from_place;
  const startPlace = scene.places.find((p: any) => p.id === start);

  const card = page.getByTestId('place-card');
  await clickMarker(page, start);
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(startPlace.display_name);
  await expect(page.getByTestId('place-card-close')).toBeVisible();

  // Hovering elsewhere on the map does nothing while pinned -- the card
  // keeps showing the PINNED place, never swaps to whatever else the
  // pointer passes over. (A forced hover here MAY, by the same hit-testing
  // ambiguity, land on a marker other than `other` -- harmless either way:
  // the assertion holds regardless of WHICH marker's hover actually fired,
  // since hover-while-pinned is a no-op for every marker uniformly.)
  const other = scene.places.find((p: any) => p.id !== start);
  if (other) {
    await page.getByTestId(`marker-${other.id}`).hover({ force: true });
    await expect(page.getByTestId('place-card-title')).toHaveText(startPlace.display_name);
  }

  // The card also survives the pointer leaving both the marker and the
  // card entirely (batch-c2-brief.md requirement 0c's own ~1s close is
  // suppressed while pinned) -- move away and wait past that window.
  await page.mouse.move(5, 5);
  await page.waitForTimeout(1200);
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-pinned', 'true');

  await page.getByTestId('place-card-close').click();
  await expect(card).toHaveCount(0);
});

test('PIN-2: Escape closes a pinned card (and only the topmost layer, popover first)', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const p = scene.places[0];

  const card = page.getByTestId('place-card');
  await clickMarker(page, p.id);
  await expect(card).toHaveAttribute('data-pinned', 'true');

  // With a popover open (opened FROM the pinned card's own title), Escape
  // closes the popover first, leaving the pin intact underneath it.
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(page.getByTestId('popover')).toHaveCount(0);

  // NOTE: promoting the title into a popover already supersedes (fully
  // closes, including the pin) the card, per PIN-1 -- re-pin fresh here to
  // test Escape's OWN direct close path in isolation.
  await clickMarker(page, p.id);
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await page.keyboard.press('Escape');
  await expect(card).toHaveCount(0);
});

test('PIN-3: clicking elsewhere on the map background closes a pinned card', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const p = scene.places[0];

  const card = page.getByTestId('place-card');
  await clickMarker(page, p.id);
  await expect(card).toHaveAttribute('data-pinned', 'true');

  // "Elsewhere on the map" via dispatchEvent on the map container itself
  // (the exact element map.js's own map.on('click', ...) is bound to,
  // per its own comment) -- NOT a pixel-coordinate click, which the ever-
  // present graph's own quiet dots (QUIET-1: every one of ~200+ event-
  // bearing places renders a marker somewhere on the plate, in EVERY
  // time-mode window, well beyond just the exodus narrative's own places)
  // make genuinely risky to land cleanly: there is no large corner of this
  // scene's own screen guaranteed marker-free the way there might be in an
  // app with only a handful of markers. Dispatching directly on the
  // container exercises the exact same production listener a real
  // background click would, without gambling on a "probably empty" pixel.
  await page.getByTestId('world-map').dispatchEvent('click');
  await expect(card).toHaveCount(0);
});

// The money shot: pin a place on the exodus route, walk it forward with
// next-event, then back with prev-event. Derives every expectation from the
// LIVE scene's own arrows/places (never hardcoded ids/names) so this stays
// correct if the curated route or its display names ever change.
test('TRAVERSAL-1: pinning a place on the exodus route and walking next/prev traverses the narrative chain', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const legs = scene.arrows.filter((a: any) => a.narrative === 'exodus').sort((a: any, b: any) => a.order - b.order);
  test.skip(legs.length < 2, 'need at least 2 exodus legs to walk next then prev');

  const nameOf = (id: string) => scene.places.find((p: any) => p.id === id).display_name;
  const start = legs[0].from_place;
  const afterNext = legs[0].to_place;

  const card = page.getByTestId('place-card');
  await clickMarker(page, start);
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(start));

  const nextBtn = page.getByTestId('card-next-event-exodus');
  await expect(nextBtn).toBeVisible();
  await nextBtn.click();

  // Traversal re-pins the ADJACENT place -- same card, new place, still pinned.
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(afterNext));

  const prevBtn = page.getByTestId('card-prev-event-exodus');
  await expect(prevBtn).toBeVisible();
  await prevBtn.click();
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(start));

  // The FIRST place in the chain has no previous leg -- its own
  // card-prev-event-exodus must be absent (conditional presence).
  await expect(page.getByTestId('card-prev-event-exodus')).toHaveCount(0);
});

// Fix round 1 regression (batch-g1-review.md M1): place-card-narratives (and
// its card-prev/next-event-N controls) rendered on a plain, UNPINNED hover --
// NarrativeRows() is a pure function of Arrows/Narratives/Place alone, and
// World.razor passes both into PlaceCard unconditionally, so nothing gated
// the section on Pinned. Reviewer verified live: hovering (never clicking)
// marker-rameses in this exact exodus window left data-pinned="false" but
// card-next-event-exodus present and clickable -- silently collapsing this
// batch's own two-gesture design (hover to preview, click to commit) into
// one. Against the pre-fix `@if (narrativeRows.Count > 0)` (no Pinned check),
// this test's first block (asserting place-card-narratives/-next-event/
// -prev-event all ABSENT while merely hovering) fails outright, since
// `start` is guaranteed >=1 outgoing exodus leg (same `legs[0].from_place`
// derivation TRAVERSAL-1 above already relies on) and NarrativeRows() would
// unconditionally render its row.
//
// Uses a REAL, hit-tested pointer gesture rather than clickMarker's own
// dispatchEvent bypass -- this test is specifically about what a genuine,
// un-pinned MOUSE hover renders, so a scripted DOM event dispatch (which
// never touches the browser's real hit-testing) would prove nothing about
// the actual defect. `independentlyHoverableIds` (world-hover-text.spec.ts's
// own shared safety helper) first confirms the route's start marker isn't
// sitting within a neighbor's hit-testing distance in THIS scene (the exact,
// separately-documented dense-cluster risk both that file's and this file's
// own header comments call out for this same exodus window) -- only once
// that's confirmed does `page.mouse.move(..., { steps: 10 })` glide the
// pointer there in many small steps, never a single teleporting jump, the
// same "at least as realistic as a real pointer" technique moveAndClick
// established (world-hover-text.spec.ts's own header comment). The
// pin-click that follows reuses that exact same pointer position (mouse.
// down/up, no further move needed) rather than re-resolving the marker
// locator, so there is no second hit-test to risk landing on a different,
// overlapping marker.
test('TRAVERSAL-2: an unpinned hover shows no narrative section; pinning the same marker reveals it and traversal still works', async ({ page }) => {
  await page.goto(`/world?from=${EXODUS_WINDOW.from}&to=${EXODUS_WINDOW.to}`);
  const scene = await api.sceneTime(EXODUS_WINDOW.from, EXODUS_WINDOW.to);
  const legs = scene.arrows.filter((a: any) => a.narrative === 'exodus').sort((a: any, b: any) => a.order - b.order);
  test.skip(legs.length === 0, 'exodus narrative not present in this window');

  const nameOf = (id: string) => scene.places.find((p: any) => p.id === id).display_name;
  const start = legs[0].from_place;
  const afterNext = legs[0].to_place;

  const safe = await independentlyHoverableIds(page, [start]);
  test.skip(!safe.has(start), 'exodus route start place is not independently hoverable in this window (marker cluster too dense)');

  const marker = page.getByTestId(`marker-${start}`);
  const box = await marker.boundingBox();
  if (!box) {
    throw new Error('exodus route start marker has no bounding box');
  }
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;

  const card = page.getByTestId('place-card');

  // Phase 1: hover only -- real, multi-step pointer arrival, never a click.
  await page.mouse.move(cx, cy, { steps: 10 });
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-pinned', 'false');
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(start));

  // The MAJOR finding itself: none of these may appear while merely hovering.
  await expect(card.getByTestId('place-card-narratives')).toHaveCount(0);
  await expect(card.getByTestId('card-next-event-exodus')).toHaveCount(0);
  await expect(card.getByTestId('card-prev-event-exodus')).toHaveCount(0);

  // Phase 2: click the SAME marker, from the SAME pointer position (already
  // there -- no jump) -- the deliberate, real "click to commit" half of the
  // batch's own two-gesture design.
  await page.mouse.down();
  await page.mouse.up();
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(start));

  // Now pinned: the narrative section (and this route's own first-stop
  // conditional presence -- next only, no previous leg yet) appears.
  await expect(card.getByTestId('place-card-narratives')).toBeVisible();
  const nextBtn = card.getByTestId('card-next-event-exodus');
  await expect(nextBtn).toBeVisible();
  await expect(card.getByTestId('card-prev-event-exodus')).toHaveCount(0);

  // Traversal still walks correctly once actually pinned: next re-pins the
  // adjacent place, which now shows its own previous-event control back.
  await nextBtn.click();
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText(nameOf(afterNext));
  await expect(card.getByTestId('card-prev-event-exodus')).toBeVisible();
});

// TRAVERSAL-3 (fix-round-1, batch-n-review.md Critical-1): the place card's
// own next/prev traversal must agree with the verse popover's own PRIOR/
// FOLLOWING traversal EVEN WHEN THE ACTIVE TIME WINDOW SPLITS THE NARRATIVE
// CHAIN -- the review's own repro: the curated exodus narrative has a real
// 37-year gap between ex_kadesh (-1445..-1444) and ex_moab (-1407..-1406)
// (data/curated/events-extra.toml). An entirely ordinary window ending
// inside that gap makes scene.rs's build_arrows drop the ex_kadesh ->
// ex_moab leg from its own windowed `kept` list (both legs still individually
// intersect nothing past -1444, so the PAIR never survives `kept.windows(2)`)
// -- pre-fix-round-1, PlaceCard's NarrativeRows/PickAdjacent read ONLY that
// windowed Arrows array, so pinning kadesh-barnea in this window showed NO
// "next event" button, while the SAME window's verse popover (server-side
// positions_for_events, batch-n-brief.md, deliberately unwindowed -- see its
// own doc comment) showed a live, clickable "FOLLOWING EVENT -- Camp on the
// plains of Moab" for the identical event. Both surfaces must agree, per the
// brief's own "if the place card and popover both show traversal, they agree
// on order and verses."
const KADESH_WINDOW = { from: -1446, to: -1444 }; // ends exactly at ex_kadesh's own to_year -- before ex_moab's own -1407 from_year

test('TRAVERSAL-3: the place card\'s next-event traversal agrees with the popover\'s FOLLOWING EVENT under a window that splits the narrative chain', async ({ page }) => {
  // Confirm the fixture premise directly against the live data, so this
  // test fails loudly (not silently vacuous) if the curated chain ever
  // changes: ex_kadesh must have a real "following" leg to walk to.
  const kadeshPositions = await api.narrativeEventPositions('ex_kadesh');
  const exodusPosition = kadeshPositions.find((p: any) => p.narrative_id === 'exodus');
  test.skip(!exodusPosition?.following, 'ex_kadesh has no following leg in the curated data');
  const followingLabel = exodusPosition.following.label;
  const followingPlaceId = exodusPosition.following.places[0];

  // Confirm the window genuinely SPLITS the chain server-side: the windowed
  // scene must carry no outgoing ex_kadesh arrow (otherwise this window no
  // longer exercises the gap and the test would prove nothing).
  const scene = await api.sceneTime(KADESH_WINDOW.from, KADESH_WINDOW.to);
  const windowedArrow = scene.arrows.find((a: any) => a.narrative === 'exodus' && a.from_event === 'ex_kadesh');
  expect(windowedArrow, 'KADESH_WINDOW must not keep an outgoing ex_kadesh arrow, or this window no longer splits the chain').toBeFalsy();
  const kadeshPlace = scene.places.find((p: any) => p.events.some((e: any) => e.id === 'ex_kadesh'));
  expect(kadeshPlace, 'ex_kadesh\'s own place must still be lit in this window').toBeTruthy();

  // --- Surface 1: the map-side place card, pinned in the splitting window ---
  await page.goto(`/world?from=${KADESH_WINDOW.from}&to=${KADESH_WINDOW.to}`);
  await page.getByTestId(`marker-${kadeshPlace.id}`).dispatchEvent('click');
  const card = page.getByTestId('place-card');
  await expect(card).toHaveAttribute('data-pinned', 'true');

  const nextBtn = page.getByTestId('card-next-event-exodus');
  await expect(nextBtn, 'the card must find the full-chain next leg even though its own arrow was filtered out of this window').toBeVisible();
  await nextBtn.click();
  await expect(card).toHaveAttribute('data-pinned', 'true');

  // The traversal landed on the SAME place the popover's own FOLLOWING EVENT
  // points to (below) -- off-window (ex_moab's own dates sit well outside
  // KADESH_WINDOW), so the graceful quiet-place fallback this codebase
  // already documents (World.razor's GoToAdjacent comment: "off-window...
  // no-op gracefully") renders instead of verse content -- itself proof the
  // traversal actually crossed the window boundary, not a same-window
  // coincidence.
  const quietTarget = scene.quiet_places.find((p: any) => p.id === followingPlaceId);
  expect(quietTarget, 'ex_moab\'s own place must be a QUIET (off-window) place in this scene').toBeTruthy();
  await expect(page.getByTestId('place-card-title')).toHaveText(quietTarget.display_name);
  await expect(page.getByTestId('place-card-quiet')).toBeVisible();

  // --- Surface 2: the EVENT node popover, independently, reached from an
  // ex_kadesh verse (Batch T requirement 3: verse-level PRIOR/FOLLOWING is
  // retired -- the verse popover itself now shows EVENT membership only;
  // traversal lives on the EVENT node reached from its own "EVENT" row) ---
  await page.goto('/read/NUM/13');
  await page.getByTestId('verse-line-26').click(); // NUM.13.26, one of ex_kadesh's own curated verses
  await expect(page.getByTestId('popover-title')).toHaveText('NUM.13.26');
  await page.getByTestId('verse-event-ex_kadesh').click();
  await expect(page.getByTestId('popover-title')).toHaveText('Spies return to Kadesh-barnea');
  const followingSection = page.getByTestId('popover-section-event-following');
  await expect(followingSection).toBeVisible();
  const followingBtn = followingSection.getByTestId('event-following-event-exodus');
  await expect(followingBtn).toHaveText(followingLabel);
});
