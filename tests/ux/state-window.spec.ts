import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { formatRange } from './lib/years';
import { LIT_MARKER_TESTID } from './lib/markers';

// Batch ST-2 -- TimeWindow full ownership (controller rulings R1-R3) +
// ViewArrangement atom (R4). state-sync.spec.ts (ST-1) already covers the
// Locus/picker side of split-view state sharing; this file covers the
// TimeWindow/ViewArrangement side deliverable 3 asks for: a slider commit
// reflected wherever the window renders, follow ON/OFF's own contract
// (through the now-real FollowTextLink), split open->close->reopen
// preserving Follow/DividerFraction via the atom, and ?from/&to/?ref deep
// links still resolving identically post-migration. See CONTRACT.md's own
// ST-2 AMENDMENT paragraphs (FOLLOW-1/SPLIT-1/DIVIDER-1) for the mechanism
// each test below locks in.

test('ST-2/R3: a time-mode window committed via the slider survives, through the shared TimeWindow atom, into a freshly opened split pane', async ({ page }) => {
  const eras = await api.eras();
  const era = eras[2]; // any era distinct from the bare-/world Gospels default
  await page.goto('/world');
  await page.getByTestId(`slider-era-${era.id}`).click();
  await page.waitForURL(u => u.searchParams.get('from') === String(era.from_year)
                          && u.searchParams.get('to') === String(era.to_year));
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(era.from_year, era.to_year));

  // Client-side navigation (SPA routing, same running WASM app/atoms --
  // never page.goto, which would tear the app down) into a fresh split.
  await page.getByTestId('split-open-world').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  // Follow defaults ON, which would show the reader's followed ref instead
  // of the committed window -- turn it off to reveal the atlas pane's own
  // window, proving it's the SAME TimeMode(era) the atom already held
  // before this NEW World instance ever mounted (SyncTimeWindowProjection,
  // called before this instance's first render), not a page-scoped value.
  await page.getByTestId('follow-chip').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(era.from_year, era.to_year));
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'false');
});

test('ST-2/R2: follow ON -- reader chapter navigation still re-scenes the atlas pane automatically, through the now-real FollowTextLink', async ({ page }) => {
  // FOLLOW-1's own base case (split-view.spec.ts) already pins the
  // user-visible contract; this re-confirms it holds across TWO consecutive
  // navigations specifically, the shape that would expose a link that only
  // fires once (e.g. a stale Active closure, or a no-echo guard wrongly
  // eating the second hop).
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.12');

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.13');
  let scene = await api.sceneScripture('GEN.13');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/14/);
  await expect(page.getByTestId('follow-chip')).toHaveText('Following GEN.14');
  scene = await api.sceneScripture('GEN.14');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);
});

test('ST-2/R2: follow OFF -- reader chapter navigation does not touch the atlas pane\'s own window at all', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await page.getByTestId('follow-chip').click(); // off
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');

  const atlasPane = page.getByTestId('split-pane-atlas');
  const before = await atlasPane.getByTestId('slider-readout').inputValue();

  await page.getByTestId('reader-next').click();
  await page.waitForURL(/\/read\/GEN\/13/);

  // The reader moved; the atlas pane's own window is untouched -- the
  // follow-text link's own Active gate (Split && Follow) correctly
  // suppresses the Locus -> TimeWindow derivation while OFF.
  await expect(page.getByTestId('chapter-head')).toContainText('13');
  await expect(atlasPane.getByTestId('slider-readout')).toHaveValue(before);
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
});

test('ST-2/R4: split open -> close -> reopen preserves the follow flag and the divider width via the ViewArrangement atom', async ({ page }) => {
  await page.goto('/read/GEN/12');
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  // Two independent, non-default bits of Split-arm state: Follow OFF, and a
  // dragged divider width.
  await page.getByTestId('follow-chip').click();
  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');

  const dividerBefore = await page.getByTestId('split-divider').boundingBox();
  expect(dividerBefore).toBeTruthy();
  if (!dividerBefore) return;
  const startX = dividerBefore.x + dividerBefore.width / 2;
  const startY = dividerBefore.y + Math.min(dividerBefore.height, 300) / 2;
  await page.mouse.move(startX, startY);
  await page.mouse.down();
  // LEFT, not right -- widens the atlas pane rather than narrowing it.
  // split-atlas-controls (left-anchored) and .picker-dusk (right-anchored,
  // max-width 19rem) are both position:absolute inside .split-pane-atlas
  // (app.css) with no responsive reflow between them -- narrowing the pane
  // enough (a rightward drag, tried in an earlier draft of this test) makes
  // them genuinely overlap, with the picker painting on top and physically
  // blocking a click on split-close-atlas underneath. Unrelated to this
  // batch's own state migration (pure layout, pre-existing), so this test
  // avoids the combination rather than the app being changed to fix it.
  await page.mouse.move(startX - 120, startY, { steps: 8 });
  await page.mouse.up();
  await page.mouse.move(startX, startY - 200); // clear of both panes' absolute-positioned controls before the next click

  const readerWidthAfterDrag = (await page.getByTestId('reader-root').boundingBox())!.width;

  // Close via World's own close button -- Reader.razor stays mounted (a
  // local field/atom flip, not a Nav.NavigateTo -- SPLIT-1/CloseSplitAtlas's
  // own doc comment).
  await page.getByTestId('split-close-atlas').click();
  await expect(page.getByTestId('split-view')).toHaveCount(0);

  // Reopen from the SAME reader instance.
  await page.getByTestId('split-open-reader').click();
  await expect(page.getByTestId('split-view')).toBeVisible();

  await expect(page.getByTestId('follow-chip')).toHaveAttribute('aria-pressed', 'false');
  const readerWidthAfterReopen = (await page.getByTestId('reader-root').boundingBox())!.width;
  expect(Math.round(readerWidthAfterReopen)).toBe(Math.round(readerWidthAfterDrag));
});

test('ST-2/R3: ?from=&to= and ?ref= deep links still resolve identically after collapsing SyncFromQuery onto EnterTimeMode/EnterScriptureMode', async ({ page }) => {
  await page.goto('/world?from=-1450&to=-1400');
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1450, -1400));
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'false');
  const timeScene = await api.sceneTime(-1450, -1400);
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(timeScene.places.length);

  await page.goto('/world?ref=EXO.14');
  await expect(page.getByTestId('mode-chip')).toContainText('EXO.14');
  await expect(page.getByTestId('slider')).toHaveAttribute('aria-disabled', 'true');
  const scriptureScene = await api.sceneScripture('EXO.14');
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scriptureScene.places.length);
});
