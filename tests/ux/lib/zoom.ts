import type { Page } from '@playwright/test';
import { expect } from '@playwright/test';

// Batch C3 (dense-marker disambiguation + clustering): zooms the map in
// `steps` wheel-notches, centered on `testid`'s own current screen
// position -- read from the marker's own Leaflet-set wrapper transform
// (works even while the marker is HIDDEN inside a cluster this pass,
// unlike `boundingBox()`, which returns null for a display:none-ancestor
// element -- applyMarkerClusters' own comment). Used to walk a scene from
// its own far/mid-tier default fitScene view down into NEAR tier, where
// decision 3 guarantees clustering stops (nudges alone, unchanged pre-C3
// behavior) -- the shared technique world-cluster-chooser.spec.ts's own
// tests and several pre-existing close-marker specs (world-same-place.spec.ts's
// UI-1/UI-3, world-map.spec.ts's WORLD-3) all now rely on.
async function markerScreenPos(page: Page, testid: string): Promise<{ x: number; y: number } | null> {
  return page.evaluate((tid) => {
    const el = document.querySelector(`[data-testid="${tid}"]`);
    const wrapper = el?.closest('.leaflet-marker-icon') as HTMLElement | null;
    const m = wrapper?.style.transform.match(/translate3d\(([-\d.]+)px,\s*([-\d.]+)px/);
    const mapEl = document.querySelector('.leaflet-container');
    if (!m || !mapEl) return null;
    const r = mapEl.getBoundingClientRect();
    return { x: r.x + parseFloat(m[1]), y: r.y + parseFloat(m[2]) };
  }, testid);
}

export async function zoomInOnMarker(page: Page, testid: string, steps: number): Promise<void> {
  // Self-review finding, fix round 2: RE-MEASURES the marker's own screen
  // position before EVERY wheel notch and re-centers the mouse there, not
  // just once before the whole loop. A single upfront measurement drifts
  // across multiple steps -- confirmed live: Leaflet's own "zoom toward
  // cursor" keeps the CURSOR's geographic point anchored at that SAME
  // screen pixel, but rounding/projection effects compound step over step
  // for a marker that isn't dead-center, eventually pushing it hundreds to
  // thousands of px off-screen (observed for both GEN.2's Eden rivers and
  // the exodus window's own "marah", at 2-3 notches). Re-centering after
  // every single notch keeps the target anchored at the SAME on-screen
  // point throughout, regardless of how many notches this call takes.
  for (let i = 0; i < steps; i++) {
    const pos = await markerScreenPos(page, testid);
    expect(pos, `expected ${testid} to be attached with a Leaflet-positioned wrapper`).toBeTruthy();
    await page.mouse.move(pos!.x, pos!.y);
    await page.mouse.wheel(0, -240);
    await page.waitForTimeout(500);
  }
  await page.waitForTimeout(500);
}

// Sets the LIVE map's zoom directly to an exact (possibly fractional)
// value via map.js's own debugSetZoom (test-support-only export, same
// dynamic-import-of-the-already-loaded-module technique world-same-place.spec.ts's
// own trueScreenPoint and world-land-mask.spec.ts's debugIsPointOnLand
// already establish -- never a client-internals import). A real
// scroll-wheel gesture only steps by a whole zoom level at a time
// (zoomInOnMarker above), which can overshoot a scene whose own rendered
// pixel gap needs a PRECISE intermediate zoom to land in -- see
// debugSetZoom's own header comment for the concrete case this exists for.
//
// `center` (optional, {lat, lon}): also recenters the map there --
// self-review finding, fix round 1: a bare zoom change zooms around the
// map's CURRENT center, which drifts a target that isn't already near
// center increasingly far off-screen at each higher zoom level (confirmed
// live: GEN.2's own tiny Eden-rivers scene). Pass the target's own TRUE
// lat/lon (its own wire `lat`/`lon`, e.g. from `GET /api/scene...`) to
// keep it anchored at the container center through the zoom instead.
export async function setZoomExact(page: Page, zoom: number, center?: { lat: number; lon: number }): Promise<void> {
  await page.evaluate(async ({ z, lat, lon }) => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    m.debugSetZoom(ids[ids.length - 1], z, lat, lon);
  }, { z: zoom, lat: center?.lat ?? null, lon: center?.lon ?? null });
  await page.waitForTimeout(400);
}
