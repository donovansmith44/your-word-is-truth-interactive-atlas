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

  // EVT-META-TOP-1 (fix round 2, owner verbatim: "we don't need to have
  // TIME: and PLACE:. just put the actual values; AD 31 and Galilee for
  // The Sermon on the Mount." Amended: "have the time and place next to
  // each other; not stacked."): ONE row (`event-time`), bare values only
  // -- no "Time:"/"Place:" label text anywhere in it -- the time value and
  // every place value as siblings, "next to each other."
  const row = page.getByTestId('event-time');
  await expect(row).toBeVisible();
  await expect(row).not.toContainText('Time:');
  await expect(row).not.toContainText('Place:');

  const timeValue = row.getByTestId('event-time-value');
  await expect(timeValue).toHaveText('AD 31');
  expect(await timeValue.evaluate(el => (el as HTMLElement).matches('.explorable'))).toBe(true);

  const placeValue = row.getByTestId('event-place-nazareth');
  await expect(placeValue).toHaveText('Nazareth');
  expect(await placeValue.evaluate(el => (el as HTMLElement).matches('.explorable'))).toBe(true);

  // "Next to each other; not stacked" -- both values sit on the SAME
  // horizontal line (a real geometry check, not just "same parent
  // element"): their own bounding boxes' vertical centers must coincide.
  const timeBox = await timeValue.boundingBox();
  const placeBox = await placeValue.boundingBox();
  expect(timeBox).not.toBeNull();
  expect(placeBox).not.toBeNull();
  expect(Math.abs((timeBox!.y + timeBox!.height / 2) - (placeBox!.y + placeBox!.height / 2)), 'Time and Place values must render on the SAME line, not stacked').toBeLessThanOrEqual(3);
});

test('EVT-META-TOP-1: The Sermon on the Mount\'s own real Time:/Place: values are "AD 31" and "Galilee" -- the owner\'s own second named example', async ({ page }) => {
  const detail = await api.event('rob_sermon_on_the_mount');
  expect(detail.when.from_year).toBe(31);
  expect(detail.when.to_year).toBe(31);
  expect(detail.places.length).toBe(1);
  expect(detail.places[0].name).toBe('Galilee');

  await openEventPopover(page, 'rob_sermon_on_the_mount');
  const row = page.getByTestId('event-time');
  await expect(row.getByTestId('event-time-value')).toHaveText('AD 31');
  await expect(row.getByTestId(`event-place-${detail.places[0].id}`)).toHaveText('Galilee');
});

test('EVT-META-TOP-1: the time/place row renders FIRST among Event sections, right below the header -- ABOVE Chronology now (supersedes CHRONO-MERGE-1\'s own "Chronology always on top")', async ({ page }) => {
  await openEventPopover(page, 'rob_last_nazareth_visit');
  const sectionOrder = await page.locator('[data-testid^="popover-section-"]').evaluateAll(
    (els) => els.map((el) => el.getAttribute('data-testid')),
  );
  const dateplacesIdx = sectionOrder.indexOf('popover-section-event-date-places');
  const chronologyIdx = sectionOrder.indexOf('popover-section-event-chronology');
  expect(dateplacesIdx).toBeGreaterThanOrEqual(0);
  expect(chronologyIdx).toBeGreaterThanOrEqual(0);
  expect(dateplacesIdx, 'the time/place section must render BEFORE Chronology now').toBeLessThan(chronologyIdx);
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
  await expect(page.getByTestId('event-time-value')).toBeVisible();
  // EVT-META-TOP-1: Time: and Place: are now siblings inside the SAME
  // `event-time` row (no separate `event-place` wrapper testid exists any
  // more) -- deliverability is proven by the ABSENCE of any
  // `event-place-{id}` value chip at all.
  await expect(page.locator('[data-testid^="event-place-"]')).toHaveCount(0);
});
