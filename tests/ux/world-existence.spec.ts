import { test, expect, Page, Locator } from '@playwright/test';
import { api } from './lib/api';

// Batch H requirement 4 (existence gating, deferred from E2 -- CONTRACT.md's
// own EXISTENCE-1): a place's NAME -- the label span inside its marker,
// never the dot itself -- hides when the current time-mode window falls
// ENTIRELY outside that place's own curated existence bounds (established/
// destroyed, data/curated/place-history.toml, resolved server-side via
// atlas_core::history::resolve_existence and carried on the wire as
// existence_from/existence_to on every places/quiet_places entry).
//
// Shiloh is the brief's own worked example (also pinned in the golden
// fixture and server-side unit tests): established -1399 (JOS.18.1, the
// tabernacle set up there), destroyed a range collapsing to -1050 on the
// wire (the ark's capture, inferred -- 1 Samuel 4). Verified live against
// the real running API (not assumed) before writing these assertions:
// Shiloh is QUIET (no events) in every time-mode window this file uses.
const AFTER_SHILOH = { from: -900, to: -800 }; // divided-kingdom era -- entirely past destroyed's own -1050
const INSIDE_SHILOH = { from: -1250, to: -1150 }; // entirely inside [-1399,-1050]

// A "should show" assertion has to rule out the OTHER, pre-existing reason
// a label can be display:none: collision damping (map.js's own
// applyLabelTier, COLLISION_CELL_PX -- unrelated to this batch, and
// something the app's own worst-case windows are deliberately dense
// enough to trigger, per map.js's own file-header comments). Zooming in
// tightly CENTERED ON the target marker keeps it anchored at that same
// screen point (Leaflet zooms toward the cursor) while spreading every
// OTHER marker away from it, past COLLISION_CELL_PX -- a real, live
// gesture (the same page.mouse.wheel pattern world-borders.spec.ts/
// world-map.spec.ts already use), not a test-only shortcut.
async function zoomInOnMarker(page: Page, marker: Locator): Promise<void> {
  const box = (await marker.boundingBox())!;
  const cx = box.x + box.width / 2;
  const cy = box.y + box.height / 2;
  for (let i = 0; i < 6; i++) {
    await page.mouse.move(cx, cy);
    await page.mouse.wheel(0, -300);
  }
}

test('existence gating: a place destroyed before the window shows its dot, no label', async ({ page }) => {
  const scene = await api.sceneTime(AFTER_SHILOH.from, AFTER_SHILOH.to);
  const shiloh = scene.quiet_places.find((p: any) => p.id === 'shiloh');
  expect(shiloh, "shiloh must be quiet (no events) in this window -- see this file's own header comment").toBeTruthy();
  // The wire itself carries shiloh's real curated bounds (Batch H's own
  // golden-fixture pin) -- established -1399, destroyed's own upper bound -1050.
  expect(shiloh.existence_from).toBe(-1399);
  expect(shiloh.existence_to).toBe(-1050);

  await page.goto(`/world?from=${AFTER_SHILOH.from}&to=${AFTER_SHILOH.to}`);
  const dot = page.getByTestId('quiet-marker-shiloh');
  await expect(dot).toBeAttached(); // the dot stays, for availability
  await expect(dot).toBeVisible();
  await zoomInOnMarker(page, dot); // rule out collision damping -- see this file's own helper comment

  const label = dot.locator('.quiet-label');
  await expect(label).toHaveCount(1); // the element exists in the DOM...
  await expect(label).toBeHidden(); // ...but display:none -- gated (existenceGatesLabel, map.js)

  // The dot stays fully interactive despite its hidden label -- clicking it
  // still pins the SAME place card any other quiet dot would (PIN-1),
  // proving this is purely a label-rendering decision, never a data filter.
  await dot.dispatchEvent('click');
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card).toHaveAttribute('data-pinned', 'true');
  await expect(page.getByTestId('place-card-title')).toHaveText('Shiloh');
});

test('existence gating: a window overlapping the existence range shows the label normally', async ({ page }) => {
  const scene = await api.sceneTime(INSIDE_SHILOH.from, INSIDE_SHILOH.to);
  const lit = scene.places.find((p: any) => p.id === 'shiloh');
  const quiet = scene.quiet_places.find((p: any) => p.id === 'shiloh');
  expect(lit ?? quiet, 'shiloh must appear, lit or quiet, in this window').toBeTruthy();

  await page.goto(`/world?from=${INSIDE_SHILOH.from}&to=${INSIDE_SHILOH.to}`);
  const marker = lit ? page.getByTestId('marker-shiloh') : page.getByTestId('quiet-marker-shiloh');
  await expect(marker).toBeVisible();
  await zoomInOnMarker(page, marker); // rule out collision damping -- see this file's own helper comment

  const label = marker.locator(lit ? '.atlas-label' : '.quiet-label');
  await expect(label).toHaveCount(1);
  await expect(label).toBeVisible();
});

test('existence gating: a place with no curated existence bounds always labels', async ({ page }) => {
  // jericho-1 carries curated NAME ranges (data/curated/place-history.toml)
  // but no established/destroyed date claim at all -- existence_from/
  // existence_to are both absent on the wire, so the gate can never fire
  // (EXISTENCE-1's own "always labels" case) at any window, including this
  // one, where it's lit (the exodus window -- Jericho's own fall, a
  // deliberately DENSE scene -- world-hover-text.spec.ts's own worst case --
  // so this test needs the zoom-in helper more than any other here).
  const w = { from: -1446, to: -1406 };
  const scene = await api.sceneTime(w.from, w.to);
  const jericho = scene.places.find((p: any) => p.id === 'jericho-1');
  expect(jericho, 'jericho-1 must be lit in the exodus window').toBeTruthy();
  expect(jericho.existence_from).toBeUndefined();
  expect(jericho.existence_to).toBeUndefined();

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const marker = page.getByTestId('marker-jericho-1');
  await expect(marker).toBeVisible();
  await zoomInOnMarker(page, marker);

  const label = marker.locator('.atlas-label');
  await expect(label).toBeVisible();
});

test('existence gating: scripture mode never gates (no window to test outside-ness against)', async ({ page }) => {
  // A scripture-mode scene carries no `window` at all -- existenceGatesLabel
  // (map.js) short-circuits to false unconditionally whenever inst.window is
  // null, regardless of what a place's own existence_from/existence_to say.
  // JOS.18 narrates Shiloh's own tabernacle-founding event directly, so it's
  // lit here (not quiet) -- a different code path (the lit marker loop) from
  // the two quiet-side tests above, exercising the same gate on that side too.
  await page.goto('/world?ref=JOS.18');
  const scene = await api.sceneScripture('JOS.18');
  const shiloh = scene.places.find((p: any) => p.id === 'shiloh');
  expect(shiloh, 'shiloh must be lit via JOS.18 (the tabernacle event)').toBeTruthy();

  const marker = page.getByTestId('marker-shiloh');
  await expect(marker).toBeVisible();
  await zoomInOnMarker(page, marker);

  const label = marker.locator('.atlas-label');
  await expect(label).toBeVisible();
});
