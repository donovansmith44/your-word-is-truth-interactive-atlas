import { test, expect, Page } from '@playwright/test';
import { api } from './lib/api';
import { neighbourNode } from './lib/edges';
import { independentlyHoverableIds } from './lib/hoverSafety';

type Ring = [number, number][];

function contains(ring: Ring, lat: number, lon: number): boolean {
  let inside = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const [latI, lonI] = ring[i];
    const [latJ, lonJ] = ring[j];
    if ((lonI > lon) !== (lonJ > lon) && lat < ((latJ - latI) * (lon - lonI)) / (lonJ - lonI) + latI) {
      inside = !inside;
    }
  }
  return inside;
}

function pointOnlyIn(polity: { rings: Ring[] }, others: { rings: Ring[] }[]): [number, number] {
  const ring = polity.rings[0];
  const lats = ring.map(p => p[0]);
  const lons = ring.map(p => p[1]);
  const [minLat, maxLat, minLon, maxLon] = [Math.min(...lats), Math.max(...lats), Math.min(...lons), Math.max(...lons)];
  for (let i = 1; i < 40; i++) {
    for (let j = 1; j < 40; j++) {
      const lat = minLat + ((maxLat - minLat) * i) / 40;
      const lon = minLon + ((maxLon - minLon) * j) / 40;
      if (contains(ring, lat, lon) && !others.some(o => o.rings.some(r => contains(r, lat, lon)))) {
        return [lat, lon];
      }
    }
  }
  throw new Error('no point lies only inside the polity');
}

async function mapCall<T>(page: Page, name: string, ...args: unknown[]): Promise<T> {
  return page.evaluate(async ({ name, args }) => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m[name](ids[ids.length - 1], ...args);
  }, { name, args });
}

test('clicking a polity\'s territory reports that polity', async ({ page }) => {
  const served = await api.polities(30, 90);
  const polities: { id: string; rings: Ring[] }[] = served.polities;
  const target = polities.find(p => p.id === 'roman-empire')!;
  const [lat, lon] = pointOnlyIn(target, polities.filter(p => p.id !== target.id));
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId(/^polity-ring-roman-empire-/).first()).toBeAttached();
  await mapCall(page, 'debugRecordSink');

  await mapCall(page, 'debugClickMap', lat, lon);

  expect(await mapCall(page, 'debugSinkCalls')).toEqual([['OnPolityClick', 'roman-empire']]);
});

test('clicking outside every territory is still a map click', async ({ page }) => {
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId(/^polity-ring-roman-empire-/).first()).toBeAttached();
  await mapCall(page, 'debugRecordSink');

  await mapCall(page, 'debugClickMap', 8.0, -10.0);

  expect(await mapCall(page, 'debugSinkCalls')).toEqual([['OnMapClick']]);
});

test('world-emphasis-site: emphasising a site rings its marker and clearing removes the ring', async ({ page }) => {
  const scene = await api.sceneTime(30, 90);
  const place = scene.places[0];
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId(`marker-${place.id}`)).toBeAttached();

  await mapCall(page, 'setEmphasis', { site: place.node.id, lat: place.lat, lon: place.lon });
  await expect(page.getByTestId('world-emphasis-site')).toHaveCount(1);
  await expect(page.getByTestId(`marker-${place.id}`).getByTestId('world-emphasis-site')).toBeVisible();

  await mapCall(page, 'setEmphasis', {});
  await expect(page.getByTestId('world-emphasis-site')).toHaveCount(0);
});

test('world-emphasis-territory: emphasising a polity outlines its territory at the drawn year and clearing removes it', async ({ page }) => {
  const served = await api.polities(30, 90);
  const rome = served.polities.find((p: { id: string }) => p.id === 'roman-empire');
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId(/^polity-ring-roman-empire-/).first()).toBeAttached();

  await mapCall(page, 'setEmphasis', { polity: rome.node.id });
  await expect(page.getByTestId('world-emphasis-territory').first()).toBeAttached();
  const rings = await page.getByTestId(/^polity-ring-roman-empire-/).count();
  await expect(page.getByTestId('world-emphasis-territory')).toHaveCount(rings);

  await mapCall(page, 'setEmphasis', {});
  await expect(page.getByTestId('world-emphasis-territory')).toHaveCount(0);
});

test('the world slider is unbounded until a focus bounds it', async ({ page }) => {
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId('slider')).toBeVisible();
  await expect(page.getByTestId('world-slider-bounds')).toHaveCount(0);
  await expect(page.getByTestId('world-cross-next')).toHaveCount(0);
});

const NT = { from: 30, to: 90 };
const CENTRED_DEGREES = 0.05;

async function windowOf(page: Page): Promise<{ from: number; to: number }> {
  const url = new URL(page.url());
  return { from: Number(url.searchParams.get('from')), to: Number(url.searchParams.get('to')) };
}

async function aLitPlace(page: Page): Promise<any> {
  const scene = await api.sceneTime(NT.from, NT.to);
  const safe = await independentlyHoverableIds(page, scene.places.map((p: { id: string }) => p.id));
  return scene.places.find((p: { id: string }) => safe.has(p.id));
}

async function aMapWithANextMap(placeNodeId: string): Promise<{ map: any; next: any }> {
  const shownOn = await api.nodeEdges(placeNodeId, 'shown-on');
  for (const map of shownOn.entries.map(neighbourNode)) {
    const following = (await api.nodeEdges(map.id, 'follows-in')).entries.map(neighbourNode);
    if (following.length > 0) {
      return { map: await api.node(map.id), next: await api.node(following[0].id) };
    }
  }
  throw new Error(`${placeNodeId} is shown on no map that another follows`);
}

test('clicking a place focuses it: the popover shows its record and neighbours, the map marks its site, the slider keeps its window', async ({ page }) => {
  await page.goto(`/world?from=${NT.from}&to=${NT.to}`);
  const place = await aLitPlace(page);
  const record = await api.node(place.node.id);

  await page.getByTestId(`marker-${place.id}`).dispatchEvent('click');

  await expect(page.getByTestId('popover-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-card-title')).toHaveText(record.label);
  await expect(page.getByTestId('popover-section-site-of')).toBeVisible();
  await expect(page.getByTestId(`marker-${place.id}`).getByTestId('world-emphasis-site')).toBeAttached();
  await expect(page.getByTestId('world-slider-bounds')).toHaveCount(0);
  expect(await windowOf(page)).toEqual(NT);
});

test('clicking a polity keeps the era, outlines its territory at the slider year, and bands its reign on the slider', async ({ page }) => {
  const polities: { id: string; rings: Ring[]; node: { id: string; label: string } }[] = (await api.polities(NT.from, NT.to)).polities;
  const target = polities.find(p => p.id === 'roman-empire')!;
  const [lat, lon] = pointOnlyIn(target, polities.filter(p => p.id !== target.id));
  await page.goto(`/world?from=${NT.from}&to=${NT.to}`);
  await expect(page.getByTestId(/^polity-ring-roman-empire-/).first()).toBeAttached();

  await mapCall(page, 'debugClickMap', lat, lon);

  await expect(page.getByTestId('popover-title')).toHaveText(target.node.label);
  await expect(page.getByTestId('world-emphasis-territory').first()).toBeAttached();
  await expect(page.getByTestId('world-slider-band')).toBeVisible();
  await expect(page.getByTestId('world-slider-bounds')).toHaveCount(0);
  expect(await windowOf(page)).toEqual(NT);
});

test('focusing a Map bounds the slider to its window; the next arrow at the bound follows follows-in to the next Map and re-bounds', async ({ page }) => {
  await page.goto(`/world?from=${NT.from}&to=${NT.to}`);
  const place = await aLitPlace(page);
  const { map, next } = await aMapWithANextMap(place.node.id);
  await page.getByTestId(`marker-${place.id}`).dispatchEvent('click');

  await page.getByTestId(`popover-up-shown-on-${map.id}`).click();

  await expect(page.getByTestId('popover-title')).toHaveText(map.label);
  await expect(page.getByTestId('world-slider-bounds')).toBeVisible();
  await expect.poll(() => windowOf(page)).toEqual({ from: map.map.window.from.value, to: map.map.window.to.value });

  await page.getByTestId('world-cross-next').click();

  await expect(page.getByTestId('popover-title')).toHaveText(next.label);
  await expect.poll(() => windowOf(page)).toEqual({ from: next.map.window.from.value, to: next.map.window.to.value });
});

test('Back from the next Map returns to the previous one and its bounds', async ({ page }) => {
  await page.goto(`/world?from=${NT.from}&to=${NT.to}`);
  const place = await aLitPlace(page);
  const { map, next } = await aMapWithANextMap(place.node.id);
  await page.getByTestId(`marker-${place.id}`).dispatchEvent('click');
  await page.getByTestId(`popover-up-shown-on-${map.id}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(map.label);
  await page.getByTestId('world-cross-next').click();
  await expect(page.getByTestId('popover-title')).toHaveText(next.label);

  await page.getByTestId('popover-breadcrumb-back').click();

  await expect(page.getByTestId('popover-title')).toHaveText(map.label);
  await expect(page.getByTestId('world-slider-bounds')).toBeVisible();
  await expect.poll(() => windowOf(page)).toEqual({ from: map.map.window.from.value, to: map.map.window.to.value });
});

test('following a Place link in the reader\'s popover offers popover-chip-map, which opens /world with the exploration carried and the place focused', async ({ page }) => {
  const scene = await api.sceneTime(NT.from, NT.to);
  const { map } = await aMapWithANextMap(scene.places[0].node.id);
  const shown = (await api.nodeEdges(map.id, 'shows')).entries.map(neighbourNode);
  const place = await api.node(shown.find(n => n.kind === 'Place')!.id);
  await page.goto('/read/GEN/1');
  await page.evaluate(save => localStorage.setItem('explorations-v3', JSON.stringify([save])), {
    id: 'map', name: map.label, createdUtc: '2026-10-01T00:00:00+00:00',
    start: { position: 'node', node: { id: map.id, kind: map.kind, label: map.label } },
    steps: [],
  });
  await page.reload();
  await page.getByTestId('hamburger-menu').click();
  await page.locator('[data-testid^="exploration-item-"] .hamburger-exploration-summary').click();
  await page.getByTestId('exploration-node-0').click();
  await page.getByTestId(`popover-child-shows-${place.id}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(place.label);

  await page.getByTestId('popover-chip-map').click();

  await expect(page).toHaveURL(/\/world/);
  await expect(page.getByTestId('popover-title')).toHaveText(place.label);
  await expect.poll(async () => {
    const camera = await mapCall<{ lat: number; lng: number }>(page, 'getCamera');
    return Math.hypot(camera.lat - place.place.lat, camera.lng - place.place.lon);
  }).toBeLessThan(CENTRED_DEGREES);
  await page.getByTestId('popover-breadcrumb-back').click();
  await expect(page.getByTestId('popover-title')).toHaveText(map.label);
});

test('selecting a place with its toggle adds its served node to the tray', async ({ page }) => {
  await page.goto(`/world?from=${NT.from}&to=${NT.to}`);
  const place = await aLitPlace(page);

  await page.getByTestId(`marker-${place.id}`).dispatchEvent('click', { ctrlKey: true });

  await expect(page.getByTestId('selection-tray')).toContainText(place.node.label);
  await expect(page.getByTestId('popover')).toHaveCount(0);
});
