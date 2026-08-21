import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Batch E3 (batch-e3-brief.md, "KJV display-name alias layer"): owner bug
// report 2026-08-20, verbatim: "there are two locations, cush and gihon,
// that are both lit up on genesis 2 even though cush isn't mentioned in gen
// 2:13 why is that happening." Root cause: the place WAS right (cush-2 IS
// what GEN.2.13 describes -- "the whole land of Ethiopia"), the LABEL was
// wrong (Theographic's own canonical name "Cush", never the word the KJV
// text itself uses there). ALIAS-1 (this file's own CONTRACT rule) is the
// fix: a curated, translation-keyed alias layer that decisively wins
// whenever no curated period-history name is active.
//
// Map-label assertions below target the LABEL TEXT directly
// (`marker-{id} .atlas-label`, via `toHaveText` -- attached-DOM text match,
// not `toBeVisible`) rather than requiring on-screen visibility: Eden's
// four rivers cluster tightly at GEN.2's own scripture-scene fitScene zoom,
// and design-direction's zoom-tiered label-collision damping (a genuine,
// unrelated, pre-existing feature -- "never everything at once") can hide
// one label at a given zoom while its neighbor's shows. `toHaveText`'s own
// exact-match semantics already prove the OLD (Cush/Tigris/Pishon) text is
// absent -- no separate negative assertion needed.

test('requirement 4: GEN.2 lights Eden\'s rivers/lands with their KJV names, not Cush/Tigris/Pishon', async ({ page }) => {
  await page.goto('/world?ref=GEN.2');

  // Pishon (GEN.2.11): "The name of the first is Pison..."
  await expect(page.getByTestId('marker-pishon').locator('.atlas-label')).toHaveText('Pison');

  // Gihon (GEN.2.13): the KJV already calls it "Gihon" -- untouched, proves
  // this fix is scoped to the genuine mismatch, not a blanket relabel.
  await expect(page.getByTestId('marker-gihon-1').locator('.atlas-label')).toHaveText('Gihon');

  // Cush (GEN.2.13, same verse as Gihon): "...compasseth the whole land of
  // Ethiopia." The owner's own named bug.
  await expect(page.getByTestId('marker-cush-2').locator('.atlas-label')).toHaveText('Ethiopia');

  // Tigris (GEN.2.14): "the name of the third river is Hiddekel..."
  await expect(page.getByTestId('marker-tigris').locator('.atlas-label')).toHaveText('Hiddekel');

  // Euphrates (GEN.2.14): the KJV already calls it "Euphrates" -- no alias
  // needed, and this fix must not touch it.
  await expect(page.getByTestId('marker-euphrates').locator('.atlas-label')).toHaveText('Euphrates');
});

test('requirement 5 (name consistency): marker label, hover card title, and popover title all agree on the aliased name', async ({ page }) => {
  await page.goto('/world?ref=GEN.2');

  await expect(page.getByTestId('marker-cush-2').locator('.atlas-label')).toHaveText('Ethiopia');

  await page.getByTestId('marker-cush-2').hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(page.getByTestId('place-card-title')).toHaveText('Ethiopia');

  await page.getByTestId('place-card-title').click();
  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('Ethiopia');
});

test('requirement 2 (quiet provenance): the place popover shows the canonical name ONCE, quietly', async ({ page }) => {
  await page.goto('/world?ref=GEN.2');
  await page.getByTestId('marker-cush-2').hover({ force: true });
  await page.getByTestId('place-card-title').click();

  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('Ethiopia'); // decisive: never "Cush" as the title
  const provenance = page.getByTestId('popover-place-canonical-name');
  await expect(provenance).toBeVisible();
  await expect(provenance).toHaveText('Known in modern atlases as Cush.');
});

test('requirement 2 (conditional presence): a place with no curated KJV alias shows no canonical-name provenance line', async ({ page }) => {
  // Gihon (gihon-1): correctly named already, no alias curated -- the
  // API-level negative control (server: PlaceDetailOut.canonical_name is
  // omitted whenever the displayed name already IS the canonical name).
  const detail = await api.place('gihon-1');
  expect(detail.canonical_name).toBeUndefined();

  await page.goto('/world?ref=GEN.2');
  await page.getByTestId('marker-gihon-1').hover({ force: true });
  await page.getByTestId('place-card-title').click();
  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('Gihon');
  await expect(popover.getByTestId('popover-place-canonical-name')).toHaveCount(0);
});

test('API: /api/scene/scripture?ref=GEN.2 resolves display_name to the curated KJV alias for cush-2/tigris/pishon', async () => {
  const scene = await api.sceneScripture('GEN.2');
  const byId = (id: string) => scene.places.find((p: any) => p.id === id);

  expect(byId('cush-2').display_name).toBe('Ethiopia');
  expect(byId('cush-2').name).toBe('Cush 2'); // the plain, un-resolved default is untouched on the wire
  expect(byId('tigris').display_name).toBe('Hiddekel');
  expect(byId('pishon').display_name).toBe('Pison');
  expect(byId('gihon-1').display_name).toBe('Gihon'); // untouched -- already KJV-accurate
});

test('API: /api/place/{id} resolves canonical_name only when an alias is the reason the name differs', async () => {
  const cush = await api.place('cush-2');
  expect(cush.canonical_name).toBe('Cush');

  const jerusalem = await api.place('jerusalem');
  expect(jerusalem.canonical_name).toBeUndefined();
});

test('API: GET /api/chapter/GEN.2 resolves place mentions to the KJV alias, so "Ethiopia" is a findable mention', async () => {
  // Reader place mentions (PlaceMentions.cs): plain-text substring scan
  // against the resolved name -- pre-fix, "Cush" never appears in GEN.2.13's
  // own text at all, so the mention was silently undetectable.
  const chapter = await api.chapter('GEN.2');
  const v13 = chapter.verses.find((v: any) => v.verse === 13);
  const cushPlace = v13.places.find((p: any) => p.id === 'cush-2');
  expect(cushPlace.name).toBe('Ethiopia');
  expect(v13.text).toContain('Ethiopia');
});

// Fix round 1 (batch-e3-review.md I-2/I-3/I-4): 4 aliases added on a second
// citation pass -- pin each one's resolved name via /api/place/{id}, the
// same "canonical_name only when it differs" contract the tests above
// already exercise.
// NOTE on surface choice: /api/place/{id}'s own top-level `name` is always
// the RAW, unresolved default (`place.name`, e.g. "Jokmeam 1", un-stripped)
// -- NOT alias-aware; only `canonical_name` (the bare canonical name,
// present when it differs) exposes anything alias-related on THIS endpoint.
// The resolved, alias-aware name lives on the scene API's `display_name`
// field instead (same surface the requirement-4/5 tests above already use
// for cush-2/tigris/pishon) -- so these tests check display_name via
// scripture-mode scenes scoped to each place's own citing chapter, and
// canonical_name via /api/place/{id}, matching each field's real contract.
test('fix round 1 (I-4): gerasa/jokmeam-1 resolve their KJV citation, gadara stays unaliased', async () => {
  const mrk5 = await api.sceneScripture('MRK.5');
  const gerasa = mrk5.places.find((p: any) => p.id === 'gerasa');
  expect(gerasa.display_name).toBe('Gadarenes'); // MRK.5.1/LUK.8.26/LUK.8.37, 3-for-3 unanimous
  expect(gerasa.name).toBe('Gerasa'); // wire default, untouched
  expect((await api.place('gerasa')).canonical_name).toBe('Gerasa');

  const ki4 = await api.sceneScripture('1KI.4');
  const jokmeam = ki4.places.find((p: any) => p.id === 'jokmeam-1');
  expect(jokmeam.display_name).toBe('Jokneam'); // 1KI.4.12
  expect((await api.place('jokmeam-1')).canonical_name).toBe('Jokmeam'); // disambiguation numeral stripped

  // gadara's own single citation (MAT.8.28) says a THIRD word, "Gergesenes"
  // -- neither "Gadara" nor "Gadarenes" -- so it stays dismissed, unaliased.
  const mat8 = await api.sceneScripture('MAT.8');
  const gadara = mat8.places.find((p: any) => p.id === 'gadara');
  expect(gadara.display_name).toBe('Gadara');
  expect((await api.place('gadara')).canonical_name).toBeUndefined();
});

test('fix round 1 (I-2/I-3): heliopolis/thebes resolve their KJV citation once collision-checked safe', async () => {
  const gen41 = await api.sceneScripture('GEN.41');
  const heliopolis = gen41.places.find((p: any) => p.id === 'heliopolis');
  expect(heliopolis.display_name).toBe('On'); // GEN.41.45
  expect((await api.place('heliopolis')).canonical_name).toBe('Heliopolis');

  const nam3 = await api.sceneScripture('NAM.3');
  const thebes = nam3.places.find((p: any) => p.id === 'thebes');
  expect(thebes.display_name).toBe('No'); // NAM.3.8 (this app's own book code for Nahum -- NOT NAH)
  expect((await api.place('thebes')).canonical_name).toBe('Thebes');
});

test('fix round 1 (I-2/I-3): pelusium stays unaliased -- "Sin" collides with the real, unrelated, already-live wilderness-of-Sin place', async () => {
  // Held per the review's own safety check: pelusium (Nile delta) and `sin`
  // (Sinai peninsula, ~231km away, already event-bearing) are NOT the same
  // real-world referent -- unlike the accepted-collision pairs elsewhere in
  // this file (cush-1/cush-2, babylonia/babylon-1, nile/shihor-2), aliasing
  // pelusium to the bare word "Sin" would give two unrelated real places an
  // identical, undifferentiated label. Both must keep rendering their OWN
  // distinct plain name -- proving no accidental collision was introduced.
  const pelusium = await api.place('pelusium');
  expect(pelusium.name).toBe('Pelusium');
  expect(pelusium.canonical_name).toBeUndefined();

  const sin = await api.place('sin');
  expect(sin.name).toBe('Sin');
  expect(sin.canonical_name).toBeUndefined();

  expect(pelusium.id).not.toBe(sin.id);
  expect(pelusium.lat).not.toBeCloseTo(sin.lat, 0); // distinct real locations, not a same-place pair
});
