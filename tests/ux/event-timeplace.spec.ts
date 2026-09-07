import { test, expect, Page } from '@playwright/test';
import { api } from './lib/api';

// EVT-3 Ticket 3 (owner verbatim, EVENT-TIMEPLACE-1): "At the top, right
// below the header of the 'event' frontier, I want Time: and Place: ...
// They're both explorable elements. If i click on AD 31 i should see, laid
// out, chronologically, the events that occurred in that year. If I click
// on the location, the half screen map thing should pop up and be focused
// on the location I selected at the time of the event we're looking at. so
// exploration of places belonging to events is a function of that location
// and the event's time, and it yields a side effect of the map opening
// with the appropriate state."
//
// NAMED FIXTURE: "The Last Visit To Nazareth" (rob_last_nazareth_visit,
// data/curated/events-extra.toml) -- real compiled data, AD 31, Nazareth,
// TWO real witnesses (MAT.13.54-58, MRK.6.1-6, data/curated/event-witnesses.toml)
// -- the owner's own example, verified against the real corpus rather than
// assumed.

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

async function openEventPopover(page: Page, eventId: string, query = '') {
  const detail = await api.event(eventId);
  const vref = detail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}${query}`);
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId(`verse-event-${eventId}`).click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);
  return detail;
}

// Reads the app's OWN live map.js module's real camera (lat/lng/zoom) --
// the SAME technique split-view.spec.ts's own readCamera establishes (see
// that file's own header comment for why a rendered marker's own
// boundingBox is unsafe here: animated zoom transitions, a perpetually
// pulsing CSS glow on lit markers).
async function readCamera(page: Page): Promise<{ lat: number; lng: number; zoom: number }> {
  return page.evaluate(async () => {
    const m: any = await import('/js/map.js');
    const ids: number[] = m.debugLiveInstanceIds();
    return m.getCamera(ids[ids.length - 1]);
  });
}

test('EVT-3/NAZARETH FIXTURE: the named fixture\'s own real values are AD 31 / Nazareth, with real MAT + MRK witnesses', async ({ page }) => {
  const detail = await api.event('rob_last_nazareth_visit');
  expect(detail.title).toBe('The last visit to Nazareth');
  expect(detail.when.from_year).toBe(31);
  expect(detail.when.to_year).toBe(31);
  expect(detail.places.length).toBe(1);
  expect(detail.places[0].name).toBe('Nazareth');
  expect(detail.witnesses.map((w: any) => w.book).sort()).toEqual(['MAT', 'MRK']);

  await openEventPopover(page, 'rob_last_nazareth_visit');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();

  const timeRow = page.getByTestId('event-time');
  await expect(timeRow).toBeVisible();
  await expect(timeRow.getByTestId('event-time-value')).toHaveText('AD 31');
  expect(await timeRow.getByTestId('event-time-value').evaluate(el => (el as HTMLElement).matches('.explorable'))).toBe(true);

  const placeRow = page.getByTestId('event-place');
  await expect(placeRow).toBeVisible();
  await expect(placeRow.getByTestId('event-place-nazareth')).toHaveText('Nazareth');
  expect(await placeRow.getByTestId('event-place-nazareth').evaluate(el => (el as HTMLElement).matches('.explorable'))).toBe(true);
});

test('EVT-3/Time: clicking "AD 31" lays out that year\'s events chronologically, each explorable', async ({ page }) => {
  await openEventPopover(page, 'rob_last_nazareth_visit');
  await page.getByTestId('event-time-value').click();
  await expect(page.getByTestId('popover-title')).toHaveText('AD 31');

  const chronology = page.getByTestId('year-chronology');
  await expect(chronology).toBeVisible();

  // The very event whose own Time: row was clicked must itself appear in
  // its own year's chronological layout (it is, by construction, a
  // located, dated event within this exact window).
  const ownRow = page.getByTestId('year-chronology-event-rob_last_nazareth_visit');
  await expect(ownRow).toBeVisible();
  await expect(ownRow).toHaveText('The last visit to Nazareth');
  expect(await ownRow.evaluate(el => (el as HTMLElement).matches('.explorable'))).toBe(true);

  // Explorable, for real: clicking a row lands on THAT event's own popover.
  await ownRow.click();
  await expect(page.getByTestId('popover-title')).toHaveText('The last visit to Nazareth');
  await expect(page.getByTestId('popover-section-event-date-places')).toBeVisible();
});

test('EVT-3/Place: clicking "Nazareth" opens the map focused on Nazareth AT the event\'s own window -- state asserted (window + focus)', async ({ page }) => {
  const detail = await api.event('rob_last_nazareth_visit');
  const scene = await api.sceneTime(31, 31);
  const nazareth = scene.places.find((p: any) => p.id === 'nazareth');
  expect(nazareth, 'Nazareth must be a real, lit place in its own event\'s window for this test to mean anything').toBeTruthy();

  // Split view -- "the half screen map thing" -- reader and atlas together,
  // the SAME layout HOTFIX-4 req 3's own "map coherence" test
  // (event-timeline.spec.ts) already uses for an analogous in-place
  // map-focus-sync assertion.
  const vref = detail.witnesses[0].verse_groups[0].verses[0];
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}?split=world`);
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
  await page.getByTestId('verse-event-rob_last_nazareth_visit').click();
  await expect(page.getByTestId('popover-title')).toHaveText(detail.title);

  await page.getByTestId('event-place-nazareth').click();

  // WINDOW: the split pane's own slider readout reflects the event's own
  // time window (the map-focus-at-time hatch's own `from=/to=` half,
  // applied via ApplyExternalQuery -- no page navigation, the split stays
  // open the whole time).
  await expect(page.getByTestId('slider-readout')).toHaveValue('AD 31', { timeout: 5000 });

  // FOCUS: the map's own real camera lands ON Nazareth's own real
  // coordinates -- not merely "some window changed," the hatch's own
  // SECOND half (place-specific pan, MapInterop.PanToPlace), read directly
  // off the live map.js module (split-view.spec.ts's own established
  // technique).
  await expect.poll(async () => {
    const camera = await readCamera(page);
    return Math.abs(camera.lat - nazareth.lat) < 0.01 && Math.abs(camera.lng - nazareth.lon) < 0.01;
  }, { timeout: 5000 }).toBe(true);
});

test('EVT-3/NAV-2 deliverability: an event with no located place (Creation) shows a Time: row but NO Place: row at all', async ({ page }) => {
  const detail = await api.event('theo-1');
  expect(detail.title).toBe('Creation of all things');
  expect(detail.places.length, 'theo-1 must genuinely have zero located places for this test to mean anything (HATCH-DELIVERABLE-1\'s own named fixture)').toBe(0);
  expect(detail.when).toBeTruthy();

  await openEventPopover(page, 'theo-1');
  await expect(page.getByTestId('event-time')).toBeVisible();
  await expect(page.getByTestId('event-place')).toHaveCount(0);
});
