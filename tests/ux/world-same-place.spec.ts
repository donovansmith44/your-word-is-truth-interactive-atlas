import { test, expect, type Page } from '@playwright/test';
import { api } from './lib/api';

// Batch HOTFIX-2 -- user report 2026-08-20: "in judges 4, zaananim,
// kedesh-naphtali, hazor are all in the ocean." Root cause (batch-hotfix2-
// report.md has the full chain): kedesh-4/kedesh-naphtali and hazor-1/
// hazor_545 are each two compiled records of the SAME real place (see
// atlas_core::merge's own doc comment) -- a scene rendered BOTH as separate
// markers, and the close-marker nudge then displaced one of each pair by a
// fixed GEOGRAPHIC delta far enough to cross the coastline. Two independent
// fixes, tested here against the user's own exact scene: (1) same-place
// dedupe, server-side (atlas_core::merge, applied before any scene is
// built) -- WIRE-1 below; (2) the anti-overlap nudge itself, now bounded
// screen pixels recomputed per zoom (map.js's applyMarkerNudges) -- UI-1
// below.

// WIRE-1: the wire assertion the brief asks for verbatim -- JDG.4 serves
// exactly ONE Hazor node and ONE Kedesh-naphtali node, never the two raw
// upstream records (hazor-1 + hazor_545, kedesh-4 + kedesh-naphtali) a
// pre-fix scene carried. Checked directly against the HTTP API (CONTRACT.md:
// "The UX property suite couples ONLY to this contract... plus the HTTP
// API"), independent of anything client-side.
test('WIRE-1: JDG.4 serves exactly one Hazor and one Kedesh-naphtali node, with merge traceability', async () => {
  const scene = await api.sceneScripture('JDG.4');
  const ids: string[] = scene.places.map((p: any) => p.id);

  // Exactly one Hazor-family id survives; the Theographic-synthesized
  // duplicate is gone entirely, not merely hidden.
  const hazorIds = ids.filter(id => id === 'hazor-1' || id === 'hazor_545');
  expect(hazorIds, `expected exactly one Hazor node, got ${JSON.stringify(hazorIds)}`).toEqual(['hazor-1']);

  // Exactly one Kedesh-naphtali-family id survives.
  const kedeshIds = ids.filter(id => id === 'kedesh-4' || id === 'kedesh-naphtali');
  expect(kedeshIds, `expected exactly one Kedesh-naphtali node, got ${JSON.stringify(kedeshIds)}`).toEqual(['kedesh-4']);

  const hazor = scene.places.find((p: any) => p.id === 'hazor-1');
  expect(hazor.merged_ids, 'hazor-1 must carry hazor_545 in merged_ids for wire traceability').toEqual(['hazor_545']);
  // The merged marker carries the UNION of both records' EVENTS (brief
  // requirement 1) -- confirmed here by a real, live example, not merely a
  // unit test: `theo-138` ("Subjugation by Jabin") was curated ONLY against
  // `hazor_545` (data/compiled/events.json: places ["canaan","hazor_545"]),
  // never against hazor-1 directly. It shows up on hazor-1 here (the
  // curated `cq_hazor` event carries no JDG.4-matching verses of its own,
  // so it does not itself appear in this scripture-mode scene) only because
  // apply_place_merges rewrote theo-138's own `places` entry from
  // hazor_545 to hazor-1 -- proof the event-union is real, not just that
  // the place ROW itself merged.
  expect(hazor.events.map((e: any) => e.id)).toEqual(['theo-138']);
  const hazorVerses = hazor.events.flatMap((e: any) => e.verse_groups.flatMap((g: any) => g.verses));
  expect(hazorVerses).toEqual(['JDG.4.1', 'JDG.4.2', 'JDG.4.3']);

  const kedesh = scene.places.find((p: any) => p.id === 'kedesh-4');
  expect(kedesh.merged_ids, 'kedesh-4 must carry kedesh-naphtali in merged_ids for wire traceability').toEqual(['kedesh-naphtali']);
  // Union of verse groups: kedesh-4's own JDG.4.9-11 mentions PLUS
  // kedesh-naphtali's own JDG.4.6 mention, all on the one surviving node.
  const kedeshVerses = kedesh.events.flatMap((e: any) => e.verse_groups.flatMap((g: any) => g.verses));
  expect(kedeshVerses).toEqual(expect.arrayContaining(['JDG.4.6', 'JDG.4.9', 'JDG.4.10', 'JDG.4.11']));

  // Zaanannim is NOT a same-place pair (it is a genuinely distinct place,
  // ~4.2km from Mount Tabor -- batch-hotfix2-report.md) -- present, un-merged.
  const zaanannim = scene.places.find((p: any) => p.id === 'zaanannim');
  expect(zaanannim, 'zaanannim should be a real, un-merged place in this scene').toBeTruthy();
  expect(zaanannim.merged_ids ?? []).toEqual([]);
});

// Reads a place's TRUE (wire) lat/lon projected to a screen point at the
// live page's CURRENT zoom/pan -- map.js's own debugTrueScreenPoint,
// test-support-only export (never called from production Blazor code),
// via a dynamic import of the SAME live module instance the page's own
// World.razor already loaded (ES modules are cached per resolved URL/
// document -- world-land-mask.spec.ts/split-view.spec.ts already establish
// this exact technique for debugIsPointOnLand/getCamera).
async function trueScreenPoint(page: Page, lat: number, lon: number): Promise<{ x: number; y: number } | null> {
  return page.evaluate(async ({ lat, lon }) => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m.debugTrueScreenPoint(ids[ids.length - 1], lat, lon);
  }, { lat, lon });
}

// UI-1: the brief's own explicit sanity property for the nudge redesign --
// "a nudge must never move a marker more than ~20px from its true position
// at any zoom." Checked against every place in the user's own exact scene,
// at the scene's default fit AND one zoom step in (brief requirement 3's
// own two screenshot points) -- deterministic (no forced hover/randomness),
// comparing each marker's REAL rendered bounding-box center against where
// its own wire lat/lon truly projects, rather than an indirect proxy like
// "screen x inside the land bounding box" (equally valid per the brief,
// but this direct comparison is the one that maps 1:1 onto what
// applyMarkerNudges actually guarantees, and catches a regression to a
// fixed-geographic-delta nudge immediately, at any zoom, not just when a
// coastline happens to be crossed).
test('UI-1: every JDG.4 marker renders within 20px of its true wire position, at fit and one zoom in', async ({ page }) => {
  const scene = await api.sceneScripture('JDG.4');
  await page.goto('/world?ref=JDG.4');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);

  async function assertEveryPlaceWithinBound(label: string) {
    for (const p of scene.places) {
      const box = await page.getByTestId(`marker-${p.id}`).boundingBox();
      expect(box, `${label}: marker-${p.id} has no bounding box`).not.toBeNull();
      const cx = box!.x + box!.width / 2, cy = box!.y + box!.height / 2;
      const truePt = await trueScreenPoint(page, p.lat, p.lon);
      expect(truePt, `${label}: debugTrueScreenPoint returned null for ${p.id}`).not.toBeNull();
      const gap = Math.hypot(cx - truePt!.x, cy - truePt!.y);
      expect(gap, `${label}: marker-${p.id} rendered ${gap.toFixed(1)}px from its true wire position (limit ~20px)`).toBeLessThanOrEqual(20);
    }
  }

  await assertEveryPlaceWithinBound('default fit');

  // One zoom step in, centered on the viewport (world-map.spec.ts's own
  // WORLD-10b test already establishes this technique -- the top-left zoom
  // control sits partly under the fixed header and is unreliable to click).
  await page.mouse.move(720, 450);
  await page.mouse.wheel(0, -300);
  await page.waitForTimeout(300);

  await assertEveryPlaceWithinBound('one zoom in');
});

// UI-2: the user's own literal report, made concrete -- Hazor, Kedesh-
// naphtali (now carried by kedesh-4), and Zaanannim each render as exactly
// one marker, on the correct (real, curated) side of the scene, never
// duplicated. Complements WIRE-1 (server payload) by proving the CLIENT
// renders exactly what the merged wire says -- no separate hazor_545/
// kedesh-naphtali marker ever appears, at either zoom point.
test('UI-2: Hazor, Kedesh-naphtali, and Zaanannim each render as exactly one marker', async ({ page }) => {
  await page.goto('/world?ref=JDG.4');
  await expect(page.getByTestId('marker-hazor-1')).toHaveCount(1);
  await expect(page.getByTestId('marker-hazor_545')).toHaveCount(0);
  await expect(page.getByTestId('marker-kedesh-4')).toHaveCount(1);
  await expect(page.getByTestId('marker-kedesh-naphtali')).toHaveCount(0);
  await expect(page.getByTestId('marker-zaanannim')).toHaveCount(1);
});

// UI-3 (requirement 2's own directional property, isolated from JDG.4's
// particular geometry): "spread apart along the axis between the two, not
// a fixed NW shove." Rigged (WORLD-3's own real-`/api/scene`-response,
// two-coordinates-overwritten technique, not a client-internals import) so
// the pair's TRUE relative bearing is known exactly -- due north/south,
// ~5km apart (comfortably inside NUDGE_TRIGGER_PX's own real-world reach
// at this tiny two-point scene's own fitScene zoom, which -- both points
// being so close together -- always lands on fitScene's own maxZoom:8
// ceiling) -- so the nudged marker's own screen-space displacement must be
// PREDOMINANTLY VERTICAL (toward/away along that same north-south axis),
// never a diagonal northwest shove unrelated to the actual neighbor's
// position (the pre-fix algorithm's own n=1 golden-angle case always
// pointed ~137.5 degrees, i.e. northwest, regardless of bearing -- see
// applyMarkerNudges' own doc comment).
test('UI-3: nudge direction follows the true bearing between two close places, not a fixed compass direction', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: real, richly-populated scene (WORLD-2/WORLD-3's own choice)
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.places.length).toBeGreaterThanOrEqual(2);

  const [a, b] = scene.places.slice(0, 2);
  const midLat = 33.0, midLon = 36.0; // BIBLICAL_WORLD_BOUNDS-safe, same anchor WORLD-3 already uses
  a.lat = midLat - 0.0225; b.lat = midLat + 0.0225; // ~5km apart, due north/south (identical lon)
  a.lon = midLon; b.lon = midLon;

  const rigged = { ...scene, places: [a, b], arrows: [], narratives: [] };
  await page.route(
    url => url.pathname === '/api/scene' && url.searchParams.get('from') === String(w.from) && url.searchParams.get('to') === String(w.to),
    route => route.fulfill({ status: 200, contentType: 'application/json', headers: { 'Access-Control-Allow-Origin': '*' }, body: JSON.stringify(rigged) }));

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await expect(page.getByTestId(/^marker-/)).toHaveCount(2);

  // b sorts after a in scene order (server-sorted by id -- WORLD-3's own
  // established assumption too) whenever a.id < b.id; swap the pair if not,
  // so `later` below is always the one candidate-side of the pairwise
  // check (the earlier-placed member of a pair is never itself nudged --
  // applyMarkerNudges' own doc comment).
  const [earlier, later] = a.id < b.id ? [a, b] : [b, a];

  const earlierBox = await page.getByTestId(`marker-${earlier.id}`).boundingBox();
  const laterBox = await page.getByTestId(`marker-${later.id}`).boundingBox();
  expect(earlierBox).not.toBeNull();
  expect(laterBox).not.toBeNull();

  const laterTrue = await trueScreenPoint(page, later.lat, later.lon);
  expect(laterTrue).not.toBeNull();
  const laterCx = laterBox!.x + laterBox!.width / 2, laterCy = laterBox!.y + laterBox!.height / 2;
  const dx = laterCx - laterTrue!.x, dy = laterCy - laterTrue!.y;
  const mag = Math.hypot(dx, dy);

  expect(mag, 'expected a real nudge (the two points are well inside NUDGE_TRIGGER_PX at this scene\'s own tight fitScene zoom)').toBeGreaterThan(1);
  expect(mag).toBeLessThanOrEqual(20); // brief's own sanity bound
  // Predominantly vertical: the north/south axis component clearly
  // dominates the (near-zero, same-longitude) east/west one -- a fixed
  // ~137.5-degree northwest shove would instead show a LARGE, comparable
  // dx alongside dy, failing this by a wide margin.
  expect(Math.abs(dy), `nudge (dx=${dx.toFixed(1)}, dy=${dy.toFixed(1)}) is not predominantly vertical`).toBeGreaterThan(Math.abs(dx) * 3);
});
