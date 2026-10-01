import { test, expect, Page } from '@playwright/test';
import { api } from './lib/api';

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

  await mapCall(page, 'setEmphasis', { site: place.id });
  await expect(page.getByTestId('world-emphasis-site')).toHaveCount(1);
  await expect(page.getByTestId(`marker-${place.id}`).getByTestId('world-emphasis-site')).toBeVisible();

  await mapCall(page, 'setEmphasis', {});
  await expect(page.getByTestId('world-emphasis-site')).toHaveCount(0);
});

test('world-emphasis-territory: emphasising a polity outlines its territory at the drawn year and clearing removes it', async ({ page }) => {
  await page.goto('/world?from=30&to=90');
  await expect(page.getByTestId(/^polity-ring-roman-empire-/).first()).toBeAttached();

  await mapCall(page, 'setEmphasis', { polity: 'roman-empire' });
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
