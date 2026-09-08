import { test, expect } from '@playwright/test';
import { api } from './lib/api';

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

// The SAME hazard-free keyboard activation accounts-and-mentions.spec.ts
// documents: a verse-line click can land on an attested mention sitting at
// the click's geometric centre and hang.
async function openVersePopover(page: any, vref: string) {
  const v = parseVerse(vref);
  await page.goto(`/read/${v.book}/${v.chapter}`);
  await page.getByTestId(`verse-line-${v.verse}`).focus();
  await page.keyboard.press('Enter');
}

test('PROV-1 (owner order 2, the headline case): the "?" on a verse\'s CROSS REFERENCES says OpenBible', async ({ page }) => {
  // "sourced from openbible.com" -- the owner's own example, at the exact
  // section he named it about.
  const verse = await api.verse('GEN.1.1');
  expect(verse.cross_refs.length, 'GEN.1.1 must carry cross references for this fixture to mean anything').toBeGreaterThan(0);
  expect(
    verse.cross_refs_provenance,
    'the wire must carry the section\'s own attribution, not merely render one client-side'
  ).toEqual(['openbible.info-cross-references']);

  await openVersePopover(page, 'GEN.1.1');

  // UNOBTRUSIVE: the panel is CLOSED until asked. An affordance that
  // shouts its contents unprompted is not the small "?" the owner asked for.
  await expect(page.getByTestId('xrefs-provenance-button')).toBeVisible();
  await expect(page.getByTestId('xrefs-provenance-panel')).toHaveCount(0);

  await page.getByTestId('xrefs-provenance-button').click();
  const panel = page.getByTestId('xrefs-provenance-panel');
  await expect(panel).toBeVisible();
  await expect(panel).toContainText('Sourced from OpenBible.info Cross-References');
  // The license rides along -- the brief's own list of what the panel says.
  await expect(panel).toContainText('License:');

  // ...and it closes again on a second press. A disclosure a reader cannot
  // undo is the hover box HOVER-KILL-1 retired, wearing a button.
  await page.getByTestId('xrefs-provenance-button').click();
  await expect(page.getByTestId('xrefs-provenance-panel')).toHaveCount(0);
});

test('PROV-1 (a THIRD source, and the CanonicalText claim): the "?" on the verse text itself says the King James Version', async ({ page }) => {
  const verse = await api.verse('JHN.3.16');
  expect(verse.provenance, 'the verse\'s own text must name its source on the wire').toBe('kjv');

  await openVersePopover(page, 'JHN.3.16');
  await page.getByTestId('verse-text-provenance-button').click();

  const panel = page.getByTestId('verse-text-provenance-panel');
  await expect(panel).toContainText('Sourced from The King James Version');
  // CONFIDENCE IS SHOWN WHEN IT IS NOT THE OBVIOUS DEFAULT (the brief: "a
  // CanonicalText claim and a Derived claim must not look alike"). Scripture
  // is the one CanonicalText source in this atlas, and it says so.
  await expect(page.getByTestId('verse-text-provenance-confidence-kjv')).toHaveText('Canonical text');
});

test('PROV-1 (owner order 1, an IMPORTED event): the "?" on a Theographic-sourced event says Theographic', async ({ page }) => {
  // theo-249 (the Espousal of Mary) is Theographic's by id -- the
  // `theo-` prefix IS the provenance rule (event_world::event_provenance).
  const espousal = await api.event('theo-249');
  expect(espousal.provenance).toBe('theographic');

  // Reached the only way a mention-only event can be: through its temporal
  // neighbour, the same walk accounts-and-mentions.spec.ts uses.
  const zacharias = await api.event('rob_zacharias_vision');
  await openVersePopover(page, zacharias.witnesses[0].verse_groups[0].verses[0]);
  await page.getByTestId('verse-event-rob_zacharias_vision').click();
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
  expect(matthews.provenance, 'a hand-authored event must not report as Theographic').toBe('curated');

  await openVersePopover(page, 'MAT.8.2');
  await page.getByTestId('verse-event-mat_leper_healed').click();
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
  expect(matthews.analogues[0].provenance).toBe('attestation-corrections');
  await page.getByTestId('event-analogues-provenance-button').click();
  await expect(page.getByTestId('event-analogues-provenance-panel')).toContainText('Sourced from Our Own Curated Work');
});

test('PROV-1 (keyboard reachable): the "?" opens with Enter and closes with Escape, with no pointer at all', async ({ page }) => {
  // Not decoration: HOVER-KILL-1 and the mobile direction both mean this
  // affordance must work with no hover and no mouse. Keyboard is the
  // strictest version of that test.
  await openVersePopover(page, 'GEN.1.1');

  const button = page.getByTestId('xrefs-provenance-button');
  await button.focus();
  await expect(button).toBeFocused();
  await expect(button).toHaveAttribute('aria-expanded', 'false');

  await page.keyboard.press('Enter');
  await expect(page.getByTestId('xrefs-provenance-panel')).toBeVisible();
  await expect(button).toHaveAttribute('aria-expanded', 'true');

  await page.keyboard.press('Escape');
  await expect(page.getByTestId('xrefs-provenance-panel')).toHaveCount(0);
  await expect(button).toHaveAttribute('aria-expanded', 'false');

  // "?" is meaningless to a screen reader, so every one carries a real name.
  await expect(button).toHaveAttribute('aria-label', 'Sources for these cross references');
});

test('PROV-1 (NO FETCH WATERFALL): every "?" on a popover shares ONE /api/sources request', async ({ page }) => {
  // THE SUB-100MS FRONTIER LAW. Resolution reads the already-fetched sources
  // document (AtlasClient.Sources is AsyncMemo-backed); a per-popover -- or
  // worse, per-affordance -- fetch would put a round trip inside the
  // frontier's own budget. Counted, not asserted in prose.
  const sourceRequests: string[] = [];
  await page.route('**/api/sources', async (route) => {
    sourceRequests.push(route.request().url());
    await route.continue();
  });

  await openVersePopover(page, 'GEN.1.1');
  // Open two DIFFERENT affordances on the same popover -- if resolution
  // fetched per affordance, this alone would already be 2.
  await page.getByTestId('verse-text-provenance-button').click();
  await expect(page.getByTestId('verse-text-provenance-panel')).toBeVisible();
  await page.getByTestId('xrefs-provenance-button').click();
  await expect(page.getByTestId('xrefs-provenance-panel')).toBeVisible();

  // Then a SECOND popover, a different verse, WITHOUT a page reload --
  // GEN.1.2 is in the chapter already open, so this is a fresh popover in
  // the SAME app session. (Deliberately not a `page.goto` to another
  // chapter: that tears the WASM app down and builds a new one, so its
  // AsyncMemo is a new object and a second fetch there would be correct
  // behavior, not a waterfall. The claim this test makes is about one
  // session, which is the claim the frontier law actually needs.)
  await page.keyboard.press('Escape');
  await page.getByTestId('verse-line-2').focus();
  await page.keyboard.press('Enter');
  await expect(page.getByTestId('popover-title')).toHaveText('GEN.1.2');
  await page.getByTestId('verse-text-provenance-button').click();
  await expect(page.getByTestId('verse-text-provenance-panel')).toBeVisible();

  expect(
    sourceRequests.length,
    `/api/sources must be fetched at most once per app session (AsyncMemo), was ${sourceRequests.length}`
  ).toBeLessThanOrEqual(1);
});

test('PROV-1 (the resolution law, at the wire): every provenance id this app serves resolves to a registry source', async () => {
  // The Rust law (atlas-graph/tests/provenance_registry_real_data.rs) proves
  // this over the WHOLE artifact at build time. This is the same claim
  // spot-checked over what the HTTP surface actually hands a browser -- so a
  // future wire field carrying an un-curated id fails here even if it never
  // reached the artifact sweep.
  const sources = await api.sources();
  const registry = new Map<string, string>(
    (sources.provenances ?? []).map((p: any) => [p.id, p.source])
  );
  expect(registry.size, 'GET /api/sources must serve the provenance join table').toBeGreaterThan(0);

  const sourceIds = new Set((sources.sources ?? []).map((s: any) => s.id));
  const kindOf = (id: string) => (id.includes('/') ? id.slice(0, id.indexOf('/')) : id);

  const verse = await api.verse('GEN.1.1');
  const event = await api.event('mat_leper_healed');
  const served: string[] = [
    verse.provenance,
    ...(verse.cross_refs_provenance ?? []),
    ...(verse.catechism_provenance ?? []),
    ...(verse.events ?? []).map((e: any) => e.provenance),
    event.provenance,
    ...(event.witnesses_provenance ?? []),
    ...(event.mentions_provenance ?? []),
    ...(event.analogues ?? []).map((a: any) => a.provenance),
  ].filter((id) => id !== undefined && id !== '');

  expect(served.length, 'the wire must actually be carrying provenance for this test to mean anything').toBeGreaterThan(4);
  for (const id of served) {
    const source = registry.get(kindOf(id));
    expect(source, `provenance id '${id}' resolves to no registry row`).toBeTruthy();
    expect(sourceIds.has(source), `provenance id '${id}' names source '${source}', which does not exist`).toBe(true);
  }
});
