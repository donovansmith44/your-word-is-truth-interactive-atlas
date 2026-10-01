import { test, expect } from '@playwright/test';
import fc from 'fast-check';
import { api } from './lib/api';
import { arbWindow } from './lib/canon';
import { fcAssert, RUNS_UI } from './lib/fc';
import { LIT_MARKER_TESTID } from './lib/markers';
import { setZoomExact } from './lib/zoom';

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

test('WORLD-2: clicking a marker opens a popover whose window-free record matches the served node', async ({ page }) => {
  const w = { from: -1446, to: -1406 };
  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  const scene = await api.sceneTime(w.from, w.to);
  await expect(page.getByTestId(LIT_MARKER_TESTID)).toHaveCount(scene.places.length);

  await fcAssert(fc.asyncProperty(
    fc.integer({ min: 0, max: scene.places.length - 1 }), async i => {
      const p = scene.places[i];
      const record = await api.node(p.node.id);
      const siteOf = record.edge_summary.find((s: { kind: string }) => s.kind === 'site-of');
      await page.getByTestId(`marker-${p.id}`).dispatchEvent('click');
      await expect(page.getByTestId('popover-title')).toHaveText(record.label);
      await expect(page.getByTestId('popover-card-title')).toHaveText(record.label);
      if (siteOf) {
        await expect(page.getByTestId('popover-section-site-of-heading')).toHaveText(`Site of (${siteOf.count})`);
      } else {
        await expect(page.getByTestId('popover-section-site-of')).toHaveCount(0);
      }
      await page.getByTestId('popover-close').click();
      await expect(page.getByTestId('popover')).toHaveCount(0);
    }), RUNS_UI);
});

test('WORLD-3: three exactly-coincident places each land on a distinct marker slot', async ({ page }) => {
  const w = { from: -1446, to: -1406 };
  const scene = await api.sceneTime(w.from, w.to);
  expect(scene.places.length).toBeGreaterThanOrEqual(3);

  const rigged = scene.places.slice(0, 3);
  for (const p of rigged) { p.lat = 33.0; p.lon = 36.0; }

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

test('WORLD-10a: place labels show at every zoom, collision damping only (no brightness-vs-tier gate left)', async ({ page }) => {
  await page.goto('/world?from=-4004&to=100');
  const egyptClustered = !(await page.getByTestId('marker-egypt').isVisible());
  if (egyptClustered) {
    await expect(page.locator('[data-testid^="marker-cluster-"]'), 'egypt is clustered this pass -- its own glyph must still be on the plate').not.toHaveCount(0);
  } else {
    await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
      'brightness-5 place ("Egypt") visible -- always was, tier or not').toBeVisible();
  }
  const susaClustered = !(await page.getByTestId('marker-susa').isVisible());
  if (susaClustered) {
    await expect(page.locator('[data-testid^="marker-cluster-"]'), 'susa is clustered this pass -- its own glyph must still be on the plate').not.toHaveCount(0);
  } else {
    await expect(page.getByTestId('marker-susa').locator('.atlas-label'),
      'brightness-1 place ("Susa"), isolated from any collision -- NO tier gate left to hide it now').toBeVisible();
  }

  await page.goto('/world?from=-5&to=29');
  await expect(page.getByTestId('marker-egypt').locator('.atlas-label'),
    'brightness-2 place, visible here too').toBeVisible();
});

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
  await expect(page.getByTestId(/^marker-/)).toHaveCount(0);
  await expect(page.getByTestId('landmark-euphrates'),
    'water-kind landmark visible at every tier, incl. FAR -- landmark tiering unchanged, no place-label competition to lose to here').toBeVisible();
  await expect(page.getByTestId('landmark-mount-sinai'),
    'mountain-kind landmark hidden below the MID tier -- landmark tiering unchanged').toBeHidden();

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

test('WORLD-11: polity labels render from the active polity eras and swap when the window moves to a different one', async ({ page }) => {
  await page.goto('/world?from=-3000&to=-2900');
  await expect(page.getByTestId('polity-label-sumer')).toBeVisible();
  await expect(page.getByTestId('polity-label-sumer')).toHaveText('Sumer');
  await expect(page.getByTestId('polity-label-roman-empire')).toHaveCount(0);

  await page.getByTestId('slider-readout').fill('AD 40 – 60');
  await page.getByTestId('slider-readout').press('Enter');
  await page.waitForURL(u => u.searchParams.get('from') === '40' && u.searchParams.get('to') === '60');

  await expect(page.getByTestId('polity-label-sumer')).toHaveCount(0);
  await expect(page.getByTestId('polity-label-roman-empire')).toBeVisible();
  await expect(page.getByTestId('polity-label-roman-empire')).toHaveText('Roman Empire');
});

test('WORLD-12: a landmark yields to a same-named, same-location lit place (no duplicate label)', async ({ page }) => {
  const w = { from: -5, to: 33 };
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
