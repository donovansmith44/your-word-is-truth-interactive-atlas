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

  // FIX ROUND 1 (review L-5): EXACTLY one, not "at most one". The old bound
  // (`toBeLessThanOrEqual(1)`) also passed on ZERO -- a regression in which
  // the client stopped requesting /api/sources entirely would have been
  // green here, while every panel rendered the fail-soft notice instead of a
  // source. The test asserted panel VISIBILITY but not panel CONTENTS, so
  // nothing else caught it either. Both halves are tightened.
  expect(
    sourceRequests.length,
    `/api/sources must be fetched EXACTLY once per app session (AsyncMemo), was ${sourceRequests.length}`
  ).toBe(1);
  // ...and the panels must actually have been RESOLVED by that one fetch.
  await expect(
    page.getByTestId('verse-text-provenance-panel'),
    'one fetch is only the right number if it actually resolved the ids'
  ).toContainText('Sourced from The King James Version');
});

// =====================================================================
// FIX ROUND 1 -- the review's own findings, as fixtures.
// =====================================================================

test('PROV-1 fix round 1 (H-1, NEVER A SILENT BLANK): a blank or unregistered provenance renders a LOUD notice, never nothing', async ({ page }) => {
  // THE TEST THAT WOULD HAVE CAUGHT H-1. Three source comments and the
  // batch report asserted that an unresolvable provenance renders a LOUD
  // notice. It did not: four client sites filtered whitespace ids out
  // BEFORE resolution, so a missing provenance rendered as NO "?" AT ALL --
  // a curatorial claim with no attribution and no sign that attribution was
  // missing, which is the silent blank the brief forbids and exactly how
  // the ATTEST-1 leper row hid.
  //
  // The wire can no longer produce a blank (both `unwrap_or_default()`
  // sites are `ApiError::internal` now), so this fixture INJECTS one --
  // which is the only way to drive a path the real corpus cannot reach, and
  // the reason the dead code went unnoticed for a whole batch.
  await page.route('**/api/verse/GEN.1.1', async (route) => {
    const response = await route.fetch();
    const body = await response.json();
    body.provenance = ''; // A BLANK: the H-1 case exactly.
    body.cross_refs_provenance = ['no-such-source-2026']; // An id no registry row claims.
    await route.fulfill({ response, json: body });
  });

  await openVersePopover(page, 'GEN.1.1');

  // THE BLANK. Before this fix, this button did not exist at all.
  const blank = page.getByTestId('verse-text-provenance-button');
  await expect(blank, 'a blank provenance must still mount a "?" -- silence is the failure mode').toBeVisible();
  await blank.click();
  await expect(page.getByTestId('verse-text-provenance-unresolved')).toContainText('no source at all');

  // THE UNREGISTERED ID, named out loud so it can actually be fixed.
  await page.getByTestId('xrefs-provenance-button').click();
  await expect(page.getByTestId('xrefs-provenance-unresolved')).toContainText('no-such-source-2026');
});

test('PROV-1 fix round 1 (M-4): a registry fetch failure blames the SOURCE LIST, never the data', async ({ page }) => {
  // Before this fix, one failed /api/sources rendered
  //   Unrecognized source "kjv". Please report it.
  // in the loudest register in the panel, on EVERY affordance on the
  // popover, for data whose provenance is perfectly well-formed and
  // perfectly well registered -- asking the reader to report a bug that
  // does not exist. The two failures are genuinely different and now say so.
  await page.route('**/api/sources', (route) => route.abort());

  await openVersePopover(page, 'GEN.1.1');
  await page.getByTestId('verse-text-provenance-button').click();

  const panel = page.getByTestId('verse-text-provenance-panel');
  await expect(panel).toContainText('The source list could not be loaded');
  // The id is still named -- it is the one true thing left to say.
  await expect(panel).toContainText('kjv');
  await expect(
    page.getByTestId('verse-text-provenance-unresolved'),
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
    'openbible.info-cross-references',
  ]);
  const cat = await api.catechism('MAT.26.26-28');
  expect(cat.length, 'MAT.26.26-28 must cite catechism items for this fixture to mean anything').toBeGreaterThan(0);
  // The genuinely MULTI-sourced family: BOTH sources, never collapsed.
  expect(cat[0].provenance).toEqual(['concord-sc-overlap', 'curated-catechism']);

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

test('PROV-1 fix round 1 (L-6): the 44x44 touch target does not swallow clicks meant for the text above it', async ({ page }) => {
  // The pressable area is a transparent 44x44 ::before centred on a mark
  // one-third that size, and the button carries
  // @onclick:stopPropagation="true". On the verse focus card the overlay
  // therefore extends well above and below the glyph, over the verse text.
  // Nothing tested that a click landing in that band reaches the text
  // instead of toggling the panel; the four report screenshots cannot show
  // it. This is the assertion, driven by geometry rather than by eye.
  await openVersePopover(page, 'GEN.1.1');
  const button = page.getByTestId('verse-text-provenance-button');
  await expect(button).toBeVisible();
  const box = await button.boundingBox();
  expect(box, 'the affordance must be laid out for this measurement to mean anything').not.toBeNull();

  // 15px above the top of the visible mark -- inside the 44px overlay's own
  // upward reach (44 vs a ~16px mark leaves ~14px of overhang each way),
  // and over the verse text the reader is trying to click.
  await page.mouse.click(box!.x + box!.width / 2, box!.y - 15);
  await expect(
    page.getByTestId('verse-text-provenance-panel'),
    'a click above the mark belongs to whatever is under it, not to the "?"'
  ).toHaveCount(0);
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
