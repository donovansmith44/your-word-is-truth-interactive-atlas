import { test, expect } from '@playwright/test';
import { api } from './lib/api';
import { elementEdge } from './lib/edges';
import { openVerse } from './lib/verse';

// Batch PROV-1 -- THE "?" AFFORDANCE, and the resolution law behind it.
//
// THE OWNER'S TWO ORDERS, verbatim, which these fixtures hold fixed:
//
//   1. "one thing we definitely need for EVERY PIECE OF DATA is the source
//      from which it came. openbible, etc."
//   2. "add a ? button on our frontier interface that gives provenance
//      (i.e., sourced from openbible.com) or whatever"
//
// Every test below drives the REAL affordance in the REAL browser: click the
// "?", read what it says. The wire is asserted alongside the DOM in each
// case, so a green DOM can never rest on a testid that renders an empty
// panel.

function parseVerse(vref: string): { book: string; chapter: number; verse: number } {
  const [book, chapter, verse] = vref.split('.');
  return { book, chapter: Number(chapter), verse: Number(verse) };
}

async function openVersePopover(page: any, vref: string) {
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await openVerse(page, v.verse);
}

async function openLeperEvent(page: any) {
  await openVersePopover(page, 'MAT.8.2');
  await page.getByTestId('popover-link-attests-Event:mat_leper_healed').click();
  await expect(page.getByTestId('popover-title')).toHaveText((await api.event('mat_leper_healed')).title);
}

type Provenance = { id: string; title: string };

async function servedEdgeProvenance(verseId: string, kind: string): Promise<Provenance[]> {
  const edges = await api.nodeEdges(verseId, kind);
  const elements = (await api.elements(edges.entries.map(entry => entry.edge.id))).elements;
  return elements.map(element => elementEdge(element, `${verseId}'s ${kind} edge`).provenance ?? { id: '', title: '' });
}

test('PROV-1 (owner order 2, the headline case): a verse\'s cross reference, stepped onto, names its source: OpenBible', async ({ page }) => {
  // Arrange
  const cites = await api.nodeEdges('text-unit:GEN.1.1', 'cites', { limit: 1 });
  const edge = elementEdge((await api.elements([cites.entries[0].edge.id])).elements[0], cites.entries[0].edge.id);
  expect(edge.provenance).toEqual({ id: 'openbible.info-cross-references', title: 'OpenBible.info Cross-References' });
  await openVersePopover(page, 'GEN.1.1');

  // Act
  await page.getByTestId(`popover-entry-edge-cites-${edge.id}`).click();

  // Assert
  await expect(page.getByTestId('popover-card-title')).toHaveText(edge.label);
  await expect(page.getByTestId('popover-field-Provenance').locator('dd')).toHaveText(edge.provenance!.id);
});

test('PROV-1 (a THIRD source): a verse\'s text names the King James Version as its source', async ({ page }) => {
  // Arrange
  const record = await api.node('text-unit:JHN.3.16');
  expect(record.provenance).toEqual({ id: 'kjv', title: 'The King James Version' });

  // Act
  await openVersePopover(page, 'JHN.3.16');

  // Assert
  await expect(page.getByTestId('popover-field-Provenance').locator('dd')).toHaveText(record.provenance.id);
});

test('PROV-1 (owner order 1, an IMPORTED event): the "?" on a Theographic-sourced event says Theographic', async ({ page }) => {
  // theo-249 (the Espousal of Mary) is Theographic's by id -- the
  // `theo-` prefix IS the provenance rule (event_world::event_provenance).
  const espousal = await api.event('theo-249');
  expect(espousal.provenance).toEqual({ id: 'theographic', title: 'Theographic Bible Metadata' });

  // Reached the only way a mention-only event can be: through its temporal
  // neighbour, the same walk accounts-and-mentions.spec.ts uses.
  const zacharias = await api.event('rob_zacharias_vision');
  await openVersePopover(page, zacharias.witnesses[0].verse_groups[0].verses[0]);
  await page.getByTestId('popover-link-attests-Event:rob_zacharias_vision').click();
  await page.getByTestId('event-chrono-prior-event-global').click();
  await expect(page.getByTestId('popover-title')).toHaveText(espousal.title);

  await page.getByTestId('event-provenance-button').click();
  const panel = page.getByTestId('event-provenance-panel');
  await expect(panel).toContainText('Sourced from Theographic Bible Metadata');
  // Imported is this atlas's default register, so it is deliberately SILENT
  // -- saying it on every popover would train the eye to skip the line where
  // it matters. Its absence here is what makes "Our own curated work"
  // legible below.
  await expect(page.getByTestId('event-provenance-confidence-theographic')).toHaveCount(0);
});

test('PROV-1 (TOTAL-CAPTURE HONESTY, the leper lesson): a CURATED row says so, and cannot pass for an imported one', async ({ page }) => {
  // mat_leper_healed is ATTEST-1's own hand-authored event -- the row that
  // was, before that batch, wearing an imported source's clothes as a false
  // parallel account. This is the affordance that makes that impossible.
  const matthews = await api.event('mat_leper_healed');
  expect(matthews.provenance, 'a hand-authored event must not report as Theographic').toEqual({ id: 'curated', title: 'Our Own Curated Work' });

  await openVersePopover(page, 'MAT.8.2');
  await page.getByTestId('popover-link-attests-Event:mat_leper_healed').click();
  await expect(page.getByTestId('popover-title')).toHaveText(matthews.title);

  await page.getByTestId('event-provenance-button').click();
  const panel = page.getByTestId('event-provenance-panel');
  await expect(panel).toContainText('Sourced from Our Own Curated Work');
  await expect(
    page.getByTestId('event-provenance-confidence-curated'),
    'our own work must ANNOUNCE itself -- that is the whole of total-capture honesty'
  ).toHaveText('Our own curated work');

  // And the SIMILAR ACCOUNTS section beside it is attributed too -- an
  // Analogue row is a curatorial CLAIM about two events, so it carries its
  // OWN provenance, not either event's.
  expect(matthews.analogues[0].provenance).toEqual({ id: 'attestation-corrections', title: 'Our Own Curated Work' });
  await page.getByTestId('event-analogues-provenance-button').click();
  await expect(page.getByTestId('event-analogues-provenance-panel')).toContainText('Sourced from Our Own Curated Work');
});

test('PROV-1 (keyboard reachable): the "?" opens with Enter and closes with Escape, with no pointer at all', async ({ page }) => {
  // Arrange
  await openLeperEvent(page);
  const button = page.getByTestId('event-provenance-button');
  await button.focus();
  await expect(button).toBeFocused();
  await expect(button).toHaveAttribute('aria-expanded', 'false');

  // Act
  await page.keyboard.press('Enter');

  // Assert
  await expect(page.getByTestId('event-provenance-panel')).toBeVisible();
  await expect(button).toHaveAttribute('aria-expanded', 'true');
  await page.keyboard.press('Escape');
  await expect(page.getByTestId('event-provenance-panel')).toHaveCount(0);
  await expect(button).toHaveAttribute('aria-expanded', 'false');
  await expect(button).toHaveAttribute('aria-label', 'Source for this event');
});

test('PROV-1 (NO FETCH WATERFALL): every "?" on a popover shares ONE /api/sources request', async ({ page }) => {
  // Arrange
  const sourceRequests: string[] = [];
  await page.route('**/api/sources', async (route) => {
    sourceRequests.push(route.request().url());
    await route.continue();
  });
  await openLeperEvent(page);

  // Act
  await page.getByTestId('event-provenance-button').click();
  await expect(page.getByTestId('event-provenance-panel')).toBeVisible();
  await page.getByTestId('event-analogues-provenance-button').click();
  await expect(page.getByTestId('event-analogues-provenance-panel')).toBeVisible();
  await page.getByTestId('popover-close').click();
  await openVerse(page, 3);
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.8.3');
  await page.getByTestId('popover-link-attests-Event:mat_leper_healed').click();
  await page.getByTestId('event-provenance-button').click();

  // Assert
  await expect(page.getByTestId('event-provenance-panel')).toContainText('Sourced from Our Own Curated Work');
  expect(sourceRequests.length, `/api/sources must be fetched EXACTLY once per app session (AsyncMemo), was ${sourceRequests.length}`).toBe(1);
});

// =====================================================================
// FIX ROUND 1 -- the review's own findings, as fixtures.
// =====================================================================

test('PROV-1 fix round 1 (H-1, NEVER A SILENT BLANK): a blank or unregistered provenance renders a LOUD notice, never nothing', async ({ page }) => {
  // Arrange
  await page.route('**/api/event/mat_leper_healed', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    body.provenance = { id: '', title: '' };
    body.analogues[0].provenance = { id: 'no-such-source-2026', title: '' };
    await route.fulfill({ response, json: body });
  });
  await openLeperEvent(page);

  // Act
  const blank = page.getByTestId('event-provenance-button');
  await expect(blank, 'a blank provenance must still mount a "?" -- silence is the failure mode').toBeVisible();
  await blank.click();
  await page.getByTestId('event-analogues-provenance-button').click();

  // Assert
  await expect(page.getByTestId('event-provenance-unresolved')).toContainText('no source at all');
  await expect(page.getByTestId('event-analogues-provenance-unresolved')).toContainText('no-such-source-2026');
});

test('PROV-1 fix round 1 (M-4): a registry fetch failure blames the SOURCE LIST, never the data', async ({ page }) => {
  // Arrange
  await page.route('**/api/sources', (route) => route.abort());
  const event = await api.event('mat_leper_healed');
  await openLeperEvent(page);

  // Act
  await page.getByTestId('event-provenance-button').click();

  // Assert
  const panel = page.getByTestId('event-provenance-panel');
  await expect(panel).toContainText('The source list could not be loaded');
  await expect(panel).toContainText(event.provenance.id);
  await expect(
    page.getByTestId('event-provenance-unresolved'),
    'an infrastructure fault must NOT wear the data fault\'s clothes'
  ).toHaveCount(0);
});

test('PROV-1 fix round 1 (M-3): a PASSAGE node\'s cross-references and catechism carry their own "?"', async ({ page }) => {
  // THE DISCLOSED GAP, CLOSED -- and its stated cause was false. The batch
  // said /api/xrefs and /api/catechism "are bare JSON arrays with no
  // envelope to hang an additive field on." The array was never where the
  // field goes: both ELEMENT types are structs, and this batch had already
  // added an element-level `provenance` to two other arrays. A reader
  // landing on a cross-reference target span -- the most common way to
  // arrive somewhere other than a verse -- used to see a full
  // cross-references list with no "?" while the identical list one node
  // earlier had one.
  const xrefs = await api.xrefs('MAT.26.26-28');
  expect(xrefs.length, 'MAT.26.26-28 must carry cross references for this fixture to mean anything').toBeGreaterThan(0);
  expect(xrefs[0].provenance, 'the WIRE must carry it, not merely the client render it').toEqual([
    { id: 'openbible.info-cross-references', title: 'OpenBible.info Cross-References' },
  ]);
  const cat = await api.catechism('MAT.26.26-28');
  expect(cat.length, 'MAT.26.26-28 must cite catechism items for this fixture to mean anything').toBeGreaterThan(0);
  // The genuinely MULTI-sourced family: BOTH sources, never collapsed.
  expect(cat[0].provenance.map((p: Provenance) => p.id)).toEqual(['concord-sc-overlap', 'curated-catechism']);

  await page.goto('/read/MAT/26');
  await page.getByTestId('verse-num-26').click();
  await page.keyboard.down('Shift');
  await page.getByTestId('verse-num-28').click();
  await page.keyboard.up('Shift');
  await page.getByTestId('passage-chip').click();
  await expect(page.getByTestId('popover-title')).toHaveText('MAT.26.26-28');

  await page.getByTestId('xrefs-provenance-button').click();
  await expect(page.getByTestId('xrefs-provenance-panel')).toContainText('Sourced from OpenBible.info Cross-References');

  await page.getByTestId('catechism-provenance-button').click();
  const catPanel = page.getByTestId('catechism-provenance-panel');
  await expect(catPanel).toContainText('Sourced from');
  // Two entries, because the family really is two-sourced.
  await expect(catPanel.locator('[data-testid^="catechism-provenance-entry-"]')).toHaveCount(2);
});

// THE OVERLAY'S COMMITTED HEIGHT, and the click offset that discriminates
// against it. Both are CONSTANTS on purpose (fix round 2, review M-NEW-2):
// deriving the click point from the live CSS would make this fixture move
// with the very rule it exists to pin, which is the vacuity trap this
// project keeps finding. See the test below for the arithmetic and for the
// red/green run that proved it discriminates.
const PROVENANCE_TARGET_MAX_HEIGHT_PX = 24;
const CLICK_ABOVE_CENTRE_PX = 18;

test('PROV-1 fix round 1 (L-6): the invisible touch target does not swallow clicks meant for the content above it', async ({ page }) => {
  // Arrange
  await openLeperEvent(page);
  const button = page.getByTestId('event-provenance-button');
  await expect(button).toBeVisible();
  const box = await button.boundingBox();
  expect(box, 'the affordance must be laid out for this measurement to mean anything').not.toBeNull();
  const overlayHeight = await button.evaluate((el) => parseFloat(getComputedStyle(el, '::before').height));

  // Act
  const centreY = box!.y + box!.height / 2;
  await page.mouse.click(box!.x + box!.width / 2, centreY - CLICK_ABOVE_CENTRE_PX);

  // Assert
  expect(overlayHeight, `the invisible touch target may not grow taller than ${PROVENANCE_TARGET_MAX_HEIGHT_PX}px`).toBeLessThanOrEqual(PROVENANCE_TARGET_MAX_HEIGHT_PX);
  await expect(
    page.getByTestId('event-provenance-panel'),
    `a click ${CLICK_ABOVE_CENTRE_PX}px above the mark's centre belongs to whatever is under it, not to the "?"`
  ).toHaveCount(0);
});

test('PROV-1 (the resolution law, at the wire): every provenance this app serves names a registry source and carries that source\'s title', async () => {
  // Arrange
  const sources = await api.sources();
  const registry = new Map<string, string>((sources.provenances ?? []).map((p: any) => [p.id, p.source]));
  const titles = new Map<string, string>((sources.sources ?? []).map((s: any) => [s.id, s.title]));
  const kindOf = (id: string) => (id.includes('/') ? id.slice(0, id.indexOf('/')) : id);
  const verseId = 'text-unit:GEN.1.1';
  const event = await api.event('mat_leper_healed');

  // Act
  const served: Provenance[] = [
    (await api.node(verseId)).provenance,
    ...(await servedEdgeProvenance(verseId, 'cites')),
    ...(await servedEdgeProvenance(verseId, 'catechism-link')),
    ...(await servedEdgeProvenance(verseId, 'attests')),
    event.provenance,
    ...(event.witnesses_provenance ?? []),
    ...(event.mentions_provenance ?? []),
    ...(event.analogues ?? []).map((a: any) => a.provenance),
  ];

  // Assert
  expect(registry.size, 'GET /api/sources must serve the provenance join table').toBeGreaterThan(0);
  expect(served.filter((p) => p === undefined || p.id === ''), 'no provenance the wire serves may be blank').toEqual([]);
  expect(served.length, 'the wire must actually be carrying provenance for this test to mean anything').toBeGreaterThan(4);
  for (const { id, title } of served) {
    const source = registry.get(kindOf(id));
    expect(source, `provenance id '${id}' resolves to no registry row`).toBeTruthy();
    expect(title, `provenance id '${id}' must carry the title of the source '${source}' it names`).toBe(titles.get(source!));
  }
});
