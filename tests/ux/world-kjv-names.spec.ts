import { test, expect } from '@playwright/test';
import { api } from './lib/api';

test('requirement 4: GEN.2 lights Eden\'s rivers/lands with their KJV names, not Cush/Tigris/Pishon', async ({ page }) => {
  await page.goto('/world?ref=GEN.2');

  await expect(page.getByTestId('marker-pishon').locator('.atlas-label')).toHaveText('Pison');

  await expect(page.getByTestId('marker-gihon-1').locator('.atlas-label')).toHaveText('Gihon');

  await expect(page.getByTestId('marker-cush-2').locator('.atlas-label')).toHaveText('Ethiopia');

  await expect(page.getByTestId('marker-tigris').locator('.atlas-label')).toHaveText('Hiddekel');

  await expect(page.getByTestId('marker-euphrates').locator('.atlas-label')).toHaveText('Euphrates');
});

test('requirement 5 (name consistency): marker label, popover title and record card title all agree on the aliased name', async ({ page }) => {
  const scene = await api.sceneScripture('GEN.2');
  const cush = scene.places.find((p: any) => p.id === 'cush-2');
  expect(cush, 'cush-2 must be lit in GEN.2').toBeTruthy();
  await page.goto('/world?ref=GEN.2');
  await page.waitForSelector('[data-testid="marker-cush-2"]', { state: 'attached' });

  await page.getByTestId('marker-cush-2').dispatchEvent('click');

  await expect(page.getByTestId('marker-cush-2').locator('.atlas-label')).toHaveText('Ethiopia');
  await expect(page.getByTestId('popover-title')).toHaveText('Ethiopia');
  await expect(page.getByTestId('popover-card-title')).toHaveText('Ethiopia');
});

test('requirement 2 (quiet provenance): the place popover shows the canonical name ONCE, as a record field', async ({ page }) => {
  await page.goto('/world?ref=GEN.2');
  await page.waitForSelector('[data-testid="marker-cush-2"]', { state: 'attached' });

  await page.getByTestId('marker-cush-2').dispatchEvent('click');

  await expect(page.getByTestId('popover-title')).toHaveText('Ethiopia');
  const provenance = page.getByTestId('popover-field-Canonical name');
  await expect(provenance).toHaveCount(1);
  await expect(provenance.locator('dd')).toHaveText('Cush');
});

test('requirement 2 (conditional presence): a place with no curated KJV alias shows no canonical-name field', async ({ page }) => {
  const detail = await api.node('Place:gihon-1');
  expect(detail.place.canonical_name).toBeUndefined();
  await page.goto('/world?ref=GEN.2');
  await page.waitForSelector('[data-testid="marker-gihon-1"]', { state: 'attached' });

  await page.getByTestId('marker-gihon-1').dispatchEvent('click');

  const popover = page.getByTestId('popover');
  await expect(popover).toBeVisible();
  await expect(page.getByTestId('popover-title')).toHaveText('Gihon');
  await expect(popover.getByTestId('popover-field-Canonical name')).toHaveCount(0);
});

test('API: /api/scene/scripture?ref=GEN.2 resolves display_name to the curated KJV alias for cush-2/tigris/pishon', async () => {
  const scene = await api.sceneScripture('GEN.2');
  const byId = (id: string) => scene.places.find((p: any) => p.id === id);

  expect(byId('cush-2').display_name).toBe('Ethiopia');
  expect(byId('cush-2').name).toBe('Cush 2');
  expect(byId('tigris').display_name).toBe('Hiddekel');
  expect(byId('pishon').display_name).toBe('Pison');
  expect(byId('gihon-1').display_name).toBe('Gihon');
});

test('API: a place record carries canonical_name only when an alias is the reason the name differs', async () => {
  const cush = await api.node('Place:cush-2');
  expect(cush.place.canonical_name).toBe('Cush');

  const jerusalem = await api.node('Place:jerusalem');
  expect(jerusalem.place.canonical_name).toBeUndefined();
});

test('API: GET /api/chapter/GEN.2 resolves place mentions to the KJV alias, so "Ethiopia" is a findable mention', async () => {
  const chapter = await api.chapter('GEN.2');
  const v13 = chapter.verses.find((v: any) => v.verse === 13);
  const cushPlace = v13.places.find((p: any) => p.id === 'cush-2');
  expect(cushPlace.name).toBe('Ethiopia');
  expect(v13.text).toContain('Ethiopia');
});

test('fix round 1 (I-4): gerasa/jokmeam-1 resolve their KJV citation, gadara stays unaliased', async () => {
  const mrk5 = await api.sceneScripture('MRK.5');
  const gerasa = mrk5.places.find((p: any) => p.id === 'gerasa');
  expect(gerasa.display_name).toBe('Gadarenes');
  expect(gerasa.name).toBe('Gerasa');
  expect((await api.node('Place:gerasa')).place.canonical_name).toBe('Gerasa');

  const ki4 = await api.sceneScripture('1KI.4');
  const jokmeam = ki4.places.find((p: any) => p.id === 'jokmeam-1');
  expect(jokmeam.display_name).toBe('Jokneam');
  expect((await api.node('Place:jokmeam-1')).place.canonical_name).toBe('Jokmeam');

  const mat8 = await api.sceneScripture('MAT.8');
  const gadara = mat8.places.find((p: any) => p.id === 'gadara');
  expect(gadara.display_name).toBe('Gadara');
  expect((await api.node('Place:gadara')).place.canonical_name).toBeUndefined();
});

test('fix round 1 (I-2/I-3): heliopolis/thebes resolve their KJV citation once collision-checked safe', async () => {
  const gen41 = await api.sceneScripture('GEN.41');
  const heliopolis = gen41.places.find((p: any) => p.id === 'heliopolis');
  expect(heliopolis.display_name).toBe('On');
  expect((await api.node('Place:heliopolis')).place.canonical_name).toBe('Heliopolis');

  const nam3 = await api.sceneScripture('NAM.3');
  const thebes = nam3.places.find((p: any) => p.id === 'thebes');
  expect(thebes.display_name).toBe('No');
  expect((await api.node('Place:thebes')).place.canonical_name).toBe('Thebes');
});

test('fix round 1 (I-2/I-3): pelusium stays unaliased -- "Sin" collides with the real, unrelated, already-live wilderness-of-Sin place', async () => {
  const pelusium = await api.node('Place:pelusium');
  expect(pelusium.label).toBe('Pelusium');
  expect(pelusium.place.canonical_name).toBeUndefined();

  const sin = await api.node('Place:sin');
  expect(sin.label).toBe('Sin');
  expect(sin.place.canonical_name).toBeUndefined();

  expect(pelusium.id).not.toBe(sin.id);
  expect(pelusium.place.lat).not.toBeCloseTo(sin.place.lat, 0);
});
