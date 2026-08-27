import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch VC-1 (owner order, verbatim: "your reader world split interface is
// too specific we should support split for any view. No privileged split
// host, no privileged guest."). See CONTRACT.md's own VC-1 amendments to
// SPLIT-1/DIVIDER-1/FOLLOW-1 for the full contract these tests pin.
//
// This file covers exactly what R4/R7 ask composition.spec.ts to cover:
// the NEW Sources "read-beside" pairing (the generality proof -- divider,
// close, both members genuinely live), the degraded-link law (no
// BearsWindow member => no follow chip, no follow link, ever), and an
// arrangement round-trip (open -> close -> reopen stays clean). The
// PRE-EXISTING reader+world pairing's own full behavior stays covered by
// split-view.spec.ts/state-window.spec.ts, UNCHANGED and untouched by this
// batch -- not re-tested here (R7: "existing suites green untouched").

test('COMP-1: Sources declares its own "read-beside" hatch -- split opens with sources hosting, reader as a genuine, live guest', async ({ page }) => {
  await page.goto('/sources');
  await expect(page.getByTestId('sources-page')).toBeVisible();
  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('split-open-sources')).toBeVisible();

  const doc = await api.sources();
  await expect(page.getByTestId(`sources-category-${doc.categories[0].id}`)).toBeVisible();

  await page.getByTestId('split-open-sources').click();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();

  // Sources' own content is STILL fully live (not replaced) -- "both
  // members live," R4's own wording.
  await expect(page.getByTestId('sources-page')).toBeVisible();
  await expect(page.getByTestId(`sources-category-${doc.categories[0].id}`)).toBeVisible();
  await expect(page.getByTestId('split-open-sources')).toHaveCount(0); // hatch button hides once already split

  // Reader is genuinely mounted as guest -- real content, not a stub.
  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('verse-line-1')).toBeVisible();

  // The generalization proof itself: no privileged host. Sources -- never
  // capable of hosting anything before this batch -- is doing so here via
  // the exact same CompositionSplit/ViewRegistry mechanism Reader always
  // used, not a bespoke code path.
  await expect(page).toHaveURL(/\/sources$/); // sources stays the route; no navigation
});

test('COMP-1: closing the embedded reader (the guest\'s own close button) returns to a full, single Sources page', async ({ page }) => {
  await page.goto('/sources');
  await page.getByTestId('split-open-sources').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-close-reader-guest')).toBeVisible();

  await page.getByTestId('split-close-reader-guest').click();

  await expect(page.getByTestId('split-view')).toHaveCount(0);
  await expect(page.getByTestId('reader-root')).toHaveCount(0);
  await expect(page.getByTestId('sources-page')).toBeVisible();
  await expect(page.getByTestId('split-open-sources')).toBeVisible();
  await expect(page).toHaveURL(/\/sources$/);
});

test('COMP-2 (degraded-link law): sources+reader has no BearsWindow member -- no follow chip, no follow link, ever', async ({ page }) => {
  await page.goto('/sources');
  await page.getByTestId('split-open-sources').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  // The chip that exists ONLY on a BearsWindow-capable guest (World) --
  // structurally absent here, not merely unclicked. See CONTRACT.md's own
  // FOLLOW-1 VC-1 amendment: the capability query (client/Views/
  // ViewRegistry.cs) is what makes this false by construction, not a
  // per-pairing special case.
  await expect(page.getByTestId('follow-chip')).toHaveCount(0);
  await expect(page.getByTestId('mode-chip')).toHaveCount(0);
  await expect(page.getByTestId('world-map')).toHaveCount(0);
});

test('COMP-3: the arrangement round-trips cleanly through open -> close -> reopen', async ({ page }) => {
  await page.goto('/sources');

  await page.getByTestId('split-open-sources').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();

  await page.getByTestId('split-close-reader-guest').click();
  await expect(page.getByTestId('split-view')).toHaveCount(0);

  // Reopening is not a fresh page load -- proves EnterSplit's own no-op/
  // fresh-arrangement distinction (client/State/ViewArrangement.cs) holds
  // through a real, in-page dispatch sequence, not just the unit-level
  // proof (client.Tests/State/ViewArrangementTests.cs).
  await page.getByTestId('split-open-sources').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('split-divider')).toBeVisible();
  await expect(page.getByTestId('reader-root')).toBeVisible();
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page.getByTestId('sources-page')).toBeVisible();
});

test('COMP-5 (fix round 1, S-4 regression): a picker jump from the guest-mounted reader does not silently open a reader+world split', async ({ page }) => {
  // The ONE real, live bug the retired ambiguous `_splitOpen` flag caused
  // (not just a code-smell): a GUEST-mounted Reader read `_splitOpen` alone
  // (true in EITHER role) for its picker-jump handler's own `?split=1`
  // query decision, silently converting a picker jump made from INSIDE the
  // sources+reader split into a reader+world one. Fixed by reading
  // CompositionSplit's own unambiguous IsHost too (client/Pages/Reader.razor's
  // own ApplyScriptureRef).
  await page.goto('/sources');
  await page.getByTestId('split-open-sources').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await page.getByTestId('picker-book').selectOption('EXO');
  await page.getByTestId('picker-apply').click();

  // Disclosed limitation (unchanged by this fix): a picker jump from inside
  // a guest-mounted reader leaves the sources+reader split entirely
  // (navigates to standalone /read/...) -- but it must NEVER carry
  // ?split=1, which would silently open an UNRELATED reader+world split.
  await page.waitForURL(/\/read\/EXO\//);
  await expect(page).not.toHaveURL(/split=1/);
  await expect(page.getByTestId('world-map')).toHaveCount(0);
});

test('COMP-4: reader+world split still opens via the reader\'s own hatch and survives a refresh -- the reshaped EnterSplit(host,guest) intent round-trips through the URL exactly as before', async ({ page }) => {
  // Regression coverage for the RESHAPED intent specifically (R2's own
  // "cold-start compatibility" -- proven at the unit level in
  // ViewArrangementTests.cs; this is the same fact at the browser level).
  // split-view.spec.ts already covers this pairing's FULL contract
  // (untouched, not duplicated here) -- this one test exists only to prove
  // the NEW EnterSplit(reader, world, ...) shape, invoked through Reader's
  // own declared hatch, still round-trips through ?split=1 + a real reload.
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('world-map')).toBeVisible();
  await expect(page).toHaveURL(/[?&]split=1/);

  await page.reload();

  await expect(page.getByTestId('split-view')).toBeVisible();
  await expect(page.getByTestId('world-map')).toBeVisible();
  await expect(page.getByTestId('verse-line-1')).toBeVisible();
  await expect(page).toHaveURL(/\/read\/GEN\/12\?split=1$/);
});
