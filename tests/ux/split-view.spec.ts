import { test, expect, Page } from '@playwright/test';
import { api } from './lib/api';

// VIEWSTATE-1's own camera round-trip needs the map's EXACT (lat, lng,
// zoom), not a rendered pixel position -- a rendered marker's own
// boundingBox is fragile here for two independent, both CONFIRMED-live
// reasons: Leaflet's wheel-zoom is a ~250ms ANIMATED transition (a read
// moments after the wheel event can land mid-animation, before the truly
// settled position DisposeAsync's own getCamera reads much later), and
// .atlas-marker's own CSS glow/breathe animation perpetually pulses a lit
// marker's rendered size (never a stable box to read AT ALL, at any zoom).
// Reads the app's OWN map.js module directly -- a dynamic import from
// page.evaluate resolves to the SAME already-loaded module instance
// Blazor's own JS interop imported (ES modules are cached per resolved
// URL/document; `/js/map.js`, an absolute path, is unambiguous regardless
// of Blazor's own routing), so this shares REAL live state (`instances`),
// not a mocked/parallel one. debugLiveInstanceIds is map.js's own
// test-support-only export (never called from production Blazor code) --
// needed because the module's own `nextId` keeps incrementing across an
// in-page navigation away from and back to /world within one test (the
// document, and this module's own state with it, never resets the way a
// fresh page.goto would), so a hardcoded id can't be assumed.
async function readCamera(page: Page): Promise<{ lat: number; lng: number; zoom: number }> {
  return page.evaluate(async () => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m.getCamera(ids[ids.length - 1]);
  });
}

// Batch H ("study without page-turning"): the split-view study layout --
// reader left, atlas right -- plus follow mode and the view-state round
// trip. See CONTRACT.md's own SPLIT-1/FOLLOW-1/VIEWSTATE-1 notes for the
// full behavior these tests pin. Reader.razor is ALWAYS the split's host
// (the URL always stays /read/{BOOK}/{chapter}); World.razor is always the
// embedded guest (SplitMode) -- see World.razor's own OnRequestClose param
// and Reader.razor's own file-header comments for why.

test('SPLIT-1: "Open the map beside the text" lands in split, atlas following the current chapter', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-reader')).toBeVisible();

  await page.getByTestId('split-open-reader').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  // Reader pane still fully functional, unchanged content.
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('12');
  // Atlas pane fully functional, following GEN.12 by default (FOLLOW-1).
  await expect(page.getByTestId('world-map')).toBeVisible();
  const chip = page.getByTestId('follow-chip');
  await expect(chip).toHaveAttribute('aria-pressed', 'true');
  await expect(chip).toHaveText('Following GEN.12');
  const scene = await api.sceneScripture('GEN.12');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});

test('SPLIT-1: closing the atlas pane returns to a full reader on the same URL, undisturbed', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('split-close-atlas').click();

  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('world-map')).toHaveCount(0);
  await expect(page).toHaveURL(/\/read\/GEN\/12$/);
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page.getByTestId('split-open-reader')).toBeVisible();
});

test('SPLIT-1: "Read beside the map" from /world lands in split at the reader\'s last chapter (GEN 1 default)', async ({ page }) => {
  await page.goto('/world?from=-1446&to=-1406');
  await expect(page.getByTestId('split-open-world')).toBeVisible();

  await page.getByTestId('split-open-world').click();
  await page.waitForURL(u => u.pathname.startsWith('/read/') && u.searchParams.get('split') === '1');

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page).toHaveURL(/\/read\/GEN\/1\?split=1/);
  await expect(page.getByTestId('chapter-head')).toContainText('1');
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.1');
});

// Fix round 2 (review Critical-1): the reviewer's own live repro was exactly
// this shape -- open split with follow ON (the untouched default, no extra
// interaction needed to reach it), immediately close-reader. Before the fix,
// World.razor's DisposeAsync wrote ViewState.Map's fields AFTER an awaited
// GetCamera() JS-interop call, which genuinely yields (IJSObjectReference
// calls always resolve via a JS promise microtask, even for a synchronous JS
// function) -- so the brand-new standalone World mounting in the SAME
// navigation could (and, per the reviewer's 3/3 and 8/8 live reproductions,
// reliably did) read ViewState.Map.HasData as still false, silently falling
// back to the hardcoded Gospels-era default instead of restoring GEN.12's
// scripture scene. This is the exact state-content assertion Important-1
// flagged as missing -- the ORIGINAL version of this test only ever checked
// DOM structure (split-view gone, world-map visible), which passes whether
// the restore actually worked OR silently discarded state and fell back to
// a default -- both look identical at the DOM-structure level.
test('SPLIT-1: closing the reader pane returns to a full /world, preserving the atlas pane\'s actual state', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true'); // follow ON, the default, untouched

  await page.getByTestId('split-close-reader').click();
  await page.waitForURL(u => u.pathname === '/world');

  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('verse-line-1')).toHaveCount(0);
  await expect(page.getByTestId('world-map')).toBeVisible();
  await expect(page.getByTestId('split-open-world')).toBeVisible();
  // The actual state-content assertion: the resulting /world shows GEN.12's
  // scripture scene (what the pane was actually following), NOT the
  // hardcoded Gospels-era time-mode default (5 BC - AD 29) a lost race
  // silently falls back to.
  await expect(page.getByTestId('mode-chip')).toContainText('GEN.12');
  const scene = await api.sceneScripture('GEN.12');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});

// Important-2 (same review): the reverse direction shares the identical
// mechanism -- "Read beside the map" (OpenReadBesideMap) disposes a
// standalone /world instance while mounting a brand-new SplitMode World, in
// one navigation -- but exercises SyncFollowRef's OWN restore branch (else
// if ViewState.Map.HasData) rather than SyncFromQuery's, only reachable when
// follow is off (the first branch, "_follow && FollowRef is not null",
// otherwise wins regardless of any race). Seeded non-racily first (follow
// toggled off, then closed via the LOCAL split-close-atlas toggle and an
// ordinary nav-world link click -- neither disposes a SplitMode instance
// concurrently with a mount, so ViewState.Map.Follow/HasData land for real
// before the actual test begins) so the only race under test is the ONE
// this finding is actually about: a ref applied on the live standalone page
// (same-route requery, Blazor reuses the instance, no dispose yet) that has
// NEVER been captured into ViewState before "Read beside the map" disposes
// this exact instance while mounting the new pane.
test('SPLIT-1: "Read beside the map" preserves a ref just applied on the standalone page, not a stale/default state', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('follow-chip').click(); // off
  await page.getByTestId('split-close-atlas').click(); // local toggle, no concurrent mount
  await expect(page.getByTestId('split-view')).toHaveCount(0);

  await page.getByTestId('nav-world').click(); // ordinary link nav -- Reader disposes, never touches ViewState.Map
  await page.waitForURL(u => u.pathname === '/world');
  await expect(page.getByTestId('split-open-world')).toBeVisible();

  // Applies to the SAME live standalone instance (same route, Blazor
  // reuses the component rather than disposing it -- ApplyScriptureRef's
  // own NavigateTo only changes the query string) -- this ref exists ONLY
  // on the live instance's own _scriptureRef field until something disposes it.
  await page.getByTestId('picker-book').selectOption('JOS');
  await page.getByTestId('picker-chapter').selectOption('6');
  await page.getByTestId('picker-apply').click();
  await expect(page).toHaveURL(/ref=JOS\.6/);
  await expect(page.getByTestId('mode-chip')).toContainText('JOS.6');

  // The actual race: THIS instance (JOS.6 live, never yet captured)
  // disposes while a brand-new SplitMode instance mounts, in one navigation.
  await page.getByTestId('split-open-world').click();
  await page.waitForURL(u => u.pathname.startsWith('/read/') && u.searchParams.get('split') === '1');

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('mode-chip')).toContainText('JOS.6');
});

// The money shot: reader navigation re-scenes the atlas pane automatically,
// with no user action on the atlas side at all. Also pins the two
// scripture-mode rules that must already hold in the pane unchanged
// (CONTRACT's own slider aria-disabled / QUIET-1's "scripture mode has no
// quiet places") -- following IS scripture mode under the hood, so both
// fall out of reusing DebouncedLoadScriptureScene, not new code.
test('FOLLOW-1: reader navigation (next chapter) re-scenes the atlas pane automatically', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.12');
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(0);

  const scene12 = await api.sceneScripture('GEN.12');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene12.places.length);

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.13');
  const scene13 = await api.sceneScripture('GEN.13');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene13.places.length);
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
  await expect(page.getByTestId(/^quiet-marker-/)).toHaveCount(0);
  // Split itself survives the navigation -- Reader.razor is reused, not
  // recreated, for an ordinary chapter change (see the file's own header).
  await expect(page.getByTestId('split-view')).toBeVisible();
});

test('FOLLOW-1: toggling follow off frees the pane to full time-mode (slider enabled, mode-chip gone)', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');

  await page.getByTestId('follow-chip').click();

  const chip = page.getByTestId('follow-chip');
  await expect(chip).toHaveAttribute('aria-pressed', 'false');
  await expect(chip).toHaveText('Follow the text');
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'false');
  await expect(page.getByTestId('mode-chip')).toHaveCount(0);
  // Everything else /world has is reachable now: eras, the readout, etc.
  await expect(page.getByTestId('slider-readout')).toBeVisible();
});

test('FOLLOW-1: toggling follow back on re-syncs to the current chapter\'s scene', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('follow-chip').click(); // off
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');

  await page.getByTestId('follow-chip').click(); // back on

  const chip = page.getByTestId('follow-chip');
  await expect(chip).toHaveAttribute('aria-pressed', 'true');
  await expect(chip).toHaveText('Following GEN.12');
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
  const scene = await api.sceneScripture('GEN.12');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});

// SPLIT-1's own "no nested-popup rule": a popover chip that would otherwise
// open a second full /world (ExplorationTarget.NavigateWorld) instead
// applies its query to the ALREADY-OPEN atlas pane in place.
test('no nested-popup rule: a "Show on /world" chip updates the open atlas pane in place, never a second one', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'true');

  await page.getByTestId('chapter-head').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  await page.getByTestId('popover-chip-map').click();

  // Never a second full atlas: still exactly one world-map, still on /read,
  // split-view still up -- ExplorerPopover "still renders normally," but
  // its own NavigateWorld target was redirected, not followed verbatim.
  await expect(page).toHaveURL(/\/read\/GEN\/12/);
  await expect(page.getByTestId('world-map')).toHaveCount(1);
  await expect(page.getByTestId('split-view')).toBeVisible();
  // Follow turned off -- the one observable proof the redirect actually
  // ran (an explicit "go look at this" action supersedes following,
  // FOLLOW-1's own precedence -- nothing else in this flow touches it).
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  const scene = await api.sceneScripture('GEN.12');
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
});

// VIEWSTATE-1: the round-trip the brief itself names verbatim. Uses REAL
// in-page navigation (nav-reader/nav-world link clicks), NEVER page.goto,
// for every hop after the first -- page.goto is a genuine top-level browser
// navigation (a full WASM reload), which would tear down the JS runtime
// (and wipe ViewStateService, an in-memory DI singleton scoped to the
// app's own lifetime) before World.razor/Reader.razor's own async
// DisposeAsync capture could ever run; a client-side-routed link click
// keeps the SAME running app alive across the "navigation," exactly like a
// real user clicking through the app -- the only way this feature works at
// all, and the only way to actually exercise it here. Bare "/" (nav-reader's
// own href) is GEN 1 -- used as this test's own reader chapter throughout,
// rather than a deep link a second page.goto would be needed to reach.
test('VIEWSTATE-1: reader scroll and atlas window/camera round-trip across page-to-page navigation', async ({ page }) => {
  // Leg 1: reader (GEN 1), scroll down.
  await page.goto('/');
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await page.evaluate(() => window.scrollTo(0, 400));
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(100);
  const scrolledY = await page.evaluate(() => window.scrollY);

  // Leg 2: to /world (in-page link) -- drag the map and zoom in, moving the
  // camera away from wherever FitScene auto-centered it.
  await page.getByTestId('nav-world').click();
  await page.waitForURL(u => u.pathname === '/world');
  const scene = await api.sceneTime(-5, 29); // the bare-/world Gospels-era default this lands on
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length); // settled: render matches the API
  // Leg 1 left the document scrolled -- the mouse actions below use plain
  // VIEWPORT coordinates (640,360) that assume that's landing on the map,
  // which only holds once the document's own scroll position has actually
  // settled back to 0 (Blazor's router resets it on navigation, but not
  // necessarily before this point synchronously) -- confirmed as a real gap
  // live (an unreset scroll made the wheel below land somewhere other than
  // the map, silently doing nothing to its zoom). Wait for it explicitly.
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBe(0);
  const before = await readCamera(page);

  await page.mouse.move(640, 360);
  await page.mouse.down();
  await page.mouse.move(500, 260, { steps: 10 });
  await page.mouse.up();
  await page.mouse.move(640, 360);
  await page.mouse.wheel(0, -300); // zoom in one notch, same real-gesture pattern other specs use

  // Leaflet's own wheel-zoom debounces/animates before actually applying --
  // confirmed live (a throwaway diagnostic script, timestamped reads every
  // 150ms): reading getCamera() immediately after dispatching the wheel
  // event still catches the PRE-wheel value; by t+150ms it has always
  // already settled and stays put indefinitely after. A flat, generous wait
  // -- not a "poll for 2 consecutive matching reads" loop -- because THAT
  // shape has its own failure mode confirmed live too: two reads taken back
  // to back (no real elapsed time between them) can BOTH catch the same
  // pre-wheel value and falsely "converge" on attempt #1, before Leaflet's
  // own debounce has even had a chance to fire once.
  await page.waitForTimeout(500);
  const afterDrag = await readCamera(page);
  expect(afterDrag).not.toEqual(before); // genuinely moved

  // Leg 3: back to the reader (in-page link) -- GEN 1, same as leg 1 --
  // scroll should be restored, not reset to the top.
  await page.getByTestId('nav-reader').click();
  await page.waitForURL(u => u.pathname === '/');
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(scrolledY - 50);
  await expect.poll(() => page.evaluate(() => window.scrollY)).toBeLessThan(scrolledY + 50);

  // Leg 4: /world again (in-page link, bare) -- window/camera exactly where
  // leg 2 left them, not a fresh auto-fit. setCamera itself is instant
  // (`{animate:false}`, MapInterop.SetCamera's own doc comment), so no
  // settle-poll is needed on this side -- a single read once the map
  // instance exists is already the final value.
  await page.getByTestId('nav-world').click();
  await page.waitForURL(u => u.pathname === '/world' && !u.search);
  await expect(page.getByTestId(/^marker-/)).toHaveCount(scene.places.length);
  await expect.poll(() => readCamera(page)).toEqual(afterDrag);
});

// ---------------------------------------------------------------------
// Batch F2 requirement 6c (SPLIT-1, user direction 2026-08-20, verbatim:
// "if i am in split screen mode and refresh, the split screen mode shalt
// not be ceased on account of refresh"). Both entry points now keep
// ?split=1 continuously in sync with the split's own open/closed state.
// ---------------------------------------------------------------------

test('SPLIT-1/6c: opening split via the reader reflects ?split=1 in the URL, and a refresh returns to split view on the same chapter', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await expect(page).toHaveURL(/\/read\/GEN\/12$/); // no ?split=1 yet

  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page).toHaveURL(/\/read\/GEN\/12\?split=1$/);

  await page.reload();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('12');
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page).toHaveURL(/\/read\/GEN\/12\?split=1$/);
});

test('SPLIT-1/6c: closing the atlas pane cleans ?split=1 from the URL, and a refresh stays out of split view', async ({ page }) => {
  await page.goto('/read/GEN/12?split=1');
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('split-close-atlas').click();
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page).toHaveURL(/\/read\/GEN\/12$/);

  await page.reload();
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-reader')).toBeVisible();
});

test('SPLIT-1/6c: chapter navigation while split is open keeps ?split=1 in the URL (so a later refresh still restores it)', async ({ page }) => {
  await page.goto('/read/GEN/12?split=1');
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('reader-next').click();
  await expect(page.getByTestId('chapter-head')).toContainText('13');
  await expect(page).toHaveURL(/\/read\/GEN\/13\?split=1$/);
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.reload();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('chapter-head')).toContainText('13');
});

// ---------------------------------------------------------------------
// Batch F2 requirement 6d (PANE-ANCHOR-1, user direction 2026-08-20,
// verbatim: "if i am exploring anything on either side of the split
// screen, the hover windows ought not be smack dab in the center of the
// screen, but on the side of the screen where the hover exploration
// originated"). Both panes' own ExplorerPopover anchors to that pane's own
// currently-visible region, never the full viewport, while split is open.
// ---------------------------------------------------------------------

test('PANE-ANCHOR-1: a verse popover opened from the reader pane stays fully within the reader pane\'s own region', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto('/read/GEN/12?split=1');
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();

  await page.getByTestId('verse-line-1').click();
  await expect(page.getByTestId('popover')).toBeVisible();

  const readerBox = await page.getByTestId('reader-root').boundingBox();
  const popoverBox = await page.getByTestId('popover').boundingBox();
  expect(readerBox).toBeTruthy();
  expect(popoverBox).toBeTruthy();
  if (readerBox && popoverBox) {
    expect(popoverBox.x).toBeGreaterThanOrEqual(readerBox.x - 1);
    expect(popoverBox.x + popoverBox.width).toBeLessThanOrEqual(readerBox.x + readerBox.width + 1);
    // Also genuinely on the LEFT side of the viewport (the reader pane's
    // own side), not coincidentally still within bounds because the pane
    // happens to span the whole window.
    expect(popoverBox.x + popoverBox.width / 2).toBeLessThan(700);
  }

  // The OTHER pane (atlas) stays fully visible and interactive -- "explore
  // on both sides of the screen independently."
  await expect(page.getByTestId('world-map')).toBeVisible();
});

test('PANE-ANCHOR-1: a place popover opened from the atlas pane stays fully within the atlas pane\'s own region', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto('/read/GEN/12?split=1');
  await expect(page.getByTestId('split-pane-atlas')).toBeVisible();
  await expect(page.getByTestId(/^marker-/).first()).toBeVisible({ timeout: 10000 });

  await page.getByTestId(/^marker-/).first().click({ force: true });
  await page.getByTestId('place-card-title').click();
  await expect(page.getByTestId('popover')).toBeVisible();

  const atlasBox = await page.getByTestId('split-pane-atlas').boundingBox();
  const popoverBox = await page.getByTestId('popover').boundingBox();
  expect(atlasBox).toBeTruthy();
  expect(popoverBox).toBeTruthy();
  if (atlasBox && popoverBox) {
    expect(popoverBox.x).toBeGreaterThanOrEqual(atlasBox.x - 1);
    expect(popoverBox.x + popoverBox.width).toBeLessThanOrEqual(atlasBox.x + atlasBox.width + 1);
    // Genuinely on the RIGHT side of the viewport (the atlas pane's own
    // side).
    expect(popoverBox.x).toBeGreaterThan(700);
  }

  // The OTHER pane (reader) stays fully visible.
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
});

test('PANE-ANCHOR-1: full-page (non-split) popovers stay viewport-centered, unaffected', async ({ page }) => {
  await page.setViewportSize({ width: 1400, height: 900 });
  await page.goto('/read/GEN/12');
  await expect(page.getByTestId('split-view')).toHaveCount(0);

  await page.getByTestId('verse-line-1').click();
  await expect(page.getByTestId('popover')).toBeVisible();
  const popoverBox = await page.getByTestId('popover').boundingBox();
  expect(popoverBox).toBeTruthy();
  if (popoverBox) {
    const viewportCenterX = 700;
    const popoverCenterX = popoverBox.x + popoverBox.width / 2;
    expect(Math.abs(popoverCenterX - viewportCenterX)).toBeLessThan(5);
  }
});
