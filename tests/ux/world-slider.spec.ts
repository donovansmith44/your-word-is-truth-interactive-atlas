import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { formatRange } from './lib/years';
import { fcAssert, RUNS_UI } from './lib/fc';

test('WORLD-5: typed readout drives window, URL and readout agree', async ({ page }) => {
  await page.goto('/world?from=-1450&to=-1400');
  await fcAssert(fc.asyncProperty(arbWindow, async w => {
    await page.getByTestId('slider-readout').fill(formatRange(w.from, w.to));
    await page.getByTestId('slider-readout').press('Enter');
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(w.from, w.to));
    await page.waitForURL(u => u.searchParams.get('from') === String(w.from)
                            && u.searchParams.get('to') === String(w.to));
  }), RUNS_UI);
});

test('WORLD-7: every era is on the slider and clickable (exhaustive); clicking pins exactly that era as active', async ({ page }) => {
  const eras = await api.eras();
  await page.goto('/world?from=-1450&to=-1400');
  for (const e of eras) {
    const label = page.getByTestId(`slider-era-${e.id}`);
    await expect(label).toContainText(e.name);
    await label.click();
    await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(e.from_year, e.to_year));

    // CONTRACT/spec amendment (Batch C, user feedback 2026-08-19): "if I
    // select a time period in the slider, it should snap to that
    // timerange exactly without overlapping with other time periods" --
    // pinned here as an exhaustive property: after clicking era `e`, EVERY
    // slider-era-{id} button's own data-active reflects that e is the only
    // active one, not just that e itself became active.
    for (const other of eras) {
      await expect(page.getByTestId(`slider-era-${other.id}`))
        .toHaveAttribute('data-active', other.id === e.id ? 'true' : 'false');
    }
  }
});

// CONTRACT/spec amendment (Batch C, user feedback 2026-08-19): magnetic
// drag-release snap. Boundary pixel positions are read off the real,
// rendered slider-era-{id} elements' own bounding boxes (era bands lay out
// edge-to-edge, SliderScale.cs's own doc comment) rather than reimplementing
// SliderScale's year<->pixel math in TypeScript -- black-box per this
// suite's own rule (CONTRACT.md: couples ONLY to the contract + the HTTP
// API), and avoids a second implementation of that math silently drifting
// from the real one.
test('WORLD-9: magnetic drag-release snaps a handle within ~6px of an era boundary; a mid-era release keeps the exact pixel year', async ({ page }) => {
  const eras = await api.eras();
  // `to=100` (the timeline's own last year) rather than the file's usual
  // `-1400` -- pins the TO handle at the track's far right edge, clear of
  // anywhere this test drags the FROM handle. Caught in dev (fix round):
  // a narrow `-1450..-1400` window puts both handles within a few px of
  // each other, so a `mouse.move` aimed at the FROM handle's own
  // bounding-box center could land on the (overlapping) TO handle instead
  // -- confirmed via a throwaway trace showing elementFromPoint at that
  // exact point resolving to "Window end year", not "Window start year".
  const START = '/world?from=-1450&to=100';
  await page.goto(START);

  const dragFromHandleTo = async (x: number, y: number) => {
    const handle = page.getByRole('slider', { name: 'Window start year' });
    const box = (await handle.boundingBox())!;
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    await page.mouse.move(x, y, { steps: 5 });
    await page.mouse.up();
  };

  // Snap case: release just inside era[1]'s own band, a few px past its
  // shared edge with era[0] -- well within the ~6px magnetic threshold of
  // that boundary. Accepts EITHER of the two adjacent boundary years
  // (era[0].to_year, era[1].from_year -- one year apart, not the same
  // number, see SnapToEraBoundary's own comment) since both sit on
  // essentially the same pixel and either is a correct snap.
  const era1Box = (await page.getByTestId(`slider-era-${eras[1].id}`).boundingBox())!;
  await dragFromHandleTo(era1Box.x + 3, era1Box.y + era1Box.height / 2);
  await page.waitForFunction(
    prev => new URL(location.href).searchParams.get('from') !== prev,
    '-1450');
  await expect(async () => {
    const snapped = Number(new URL(page.url()).searchParams.get('from'));
    expect([eras[0].to_year, eras[1].from_year]).toContain(snapped);
  }).toPass();

  // No-snap case: release at era[1]'s own visual midpoint -- far past the
  // 6px threshold from either of ITS OWN edges -- must land strictly
  // inside era[1]'s own range, never pulled to a boundary ("a mid-era
  // release keeps the exact year -- deliberate drag wins").
  await page.goto(START);
  const era1BoxAgain = (await page.getByTestId(`slider-era-${eras[1].id}`).boundingBox())!;
  const midX = era1BoxAgain.x + era1BoxAgain.width / 2;
  await dragFromHandleTo(midX, era1BoxAgain.y + era1BoxAgain.height / 2);
  await page.waitForFunction(
    prev => new URL(location.href).searchParams.get('from') !== prev,
    '-1450');
  const midYear = Number(new URL(page.url()).searchParams.get('from'));
  expect(midYear).toBeGreaterThan(eras[1].from_year);
  expect(midYear).toBeLessThan(eras[1].to_year);
});

test('NAV-1 (world/time): deep link survives reload', async ({ page }) => {
  await page.goto('/world?from=-1406&to=-1405');
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
  await page.reload();
  await expect(page.getByTestId('slider-readout')).toHaveValue(formatRange(-1406, -1405));
});

test('errors surface as toast, app keeps standing', async ({ page }) => {
  await page.goto('/world?from=0&to=5');
  await expect(page.getByTestId('toast')).toBeVisible();
  await expect(page.getByTestId('world-map')).toBeAttached();
});
