import { test, expect } from '@playwright/test';
import { api } from './lib/api';

// Black-box mirror of PlaceCard.razor's MergedVerses(): flattens every
// event's verse_groups' verse ids (already book/chapter/verse-ascending per
// atlas-core's verse_groups_for) in event order, deduped by canonical id --
// the same shape the card's hover-verse-{VREF} rows render from. Kept local
// to this spec file (no client imports, per the black-box UX suite rule).
function mergedVerses(place: any): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const e of place.events) {
    for (const g of e.verse_groups) {
      for (const v of g.verses) {
        if (!seen.has(v)) {
          seen.add(v);
          out.push(v);
        }
      }
    }
  }
  return out;
}

// Looks up a single verse's real KJV text via the chapter endpoint (the
// same source PlaceCard.razor's AtlasClient.Chapter fetch reads from).
async function verseText(vref: string): Promise<string> {
  const [book, chapter, verseNum] = vref.split('.');
  const chapterOut = await api.chapter(`${book}.${chapter}`);
  const verse = chapterOut.verses.find((v: any) => v.verse === Number(verseNum));
  if (!verse) {
    throw new Error(`verse ${vref} missing from fetched chapter ${book}.${chapter}`);
  }
  return verse.text;
}

test('hover place card leads with two verses of real text, expand reveals the rest, index rows still render', async ({ page }) => {
  const w = { from: -1446, to: -1406 }; // exodus window: rich scene (world-map.spec.ts's own WORLD-2 comment)
  const scene = await api.sceneTime(w.from, w.to);
  const place = scene.places.find((p: any) => mergedVerses(p).length >= 3);
  if (!place) {
    test.skip(true, 'no place with >=3 merged verses in the exodus window');
    return;
  }
  const verses = mergedVerses(place);

  await page.goto(`/world?from=${w.from}&to=${w.to}`);
  await page.getByTestId(`marker-${place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();

  // Exactly two hover-verse-{VREF} rows shown initially, refs+text matching
  // the first two of the place's merged canonical verse list.
  const rows = card.getByTestId(/^hover-verse-/);
  await expect(rows).toHaveCount(2);
  for (const vref of verses.slice(0, 2)) {
    const text = await verseText(vref);
    await expect(card.getByTestId(`hover-verse-${vref}`)).toContainText(text);
  }

  // The expand control shows the correct remaining count.
  const expandBtn = card.getByTestId('place-card-expand');
  await expect(expandBtn).toBeVisible();
  await expect(expandBtn).toContainText(String(verses.length - 2));

  // Clicking it (a descendant of the card) must not close the card -- the
  // pointer never actually leaves .place-card's own bounds along the way.
  await expandBtn.click();
  await expect(card).toBeVisible();

  // Every merged verse now has its own row, including newly revealed ones.
  await expect(rows).toHaveCount(verses.length);
  const spotVref = verses[2];
  const spotText = await verseText(spotVref);
  await expect(card.getByTestId(`hover-verse-${spotVref}`)).toContainText(spotText);

  // The existing compact (book,chapter) index rows (WORLD-2's own merge)
  // still render underneath, untouched by any of the above.
  const groupCounts = new Map<string, number>();
  for (const e of place.events) {
    for (const g of e.verse_groups) {
      const key = `${g.book}-${g.chapter}`;
      groupCounts.set(key, (groupCounts.get(key) ?? 0) + g.count);
    }
  }
  for (const [key, count] of groupCounts) {
    await expect(card.getByTestId(`verse-group-${key}`)).toContainText(String(count));
  }
});

test('a place with two or fewer merged verses shows no expand control', async ({ page }) => {
  // Search a handful of era windows (same three world-arrows.spec.ts uses,
  // plus the full timeline) for a place at or under the 2-verse threshold --
  // per the brief, fall back to another window if the exodus one has none.
  const candidateWindows = [
    { from: -1446, to: -1406 },
    { from: -2100, to: -2085 },
    { from: 46, to: 48 },
    { from: -4004, to: 100 },
  ];

  let match: { w: { from: number; to: number }; place: any } | undefined;
  for (const w of candidateWindows) {
    const scene = await api.sceneTime(w.from, w.to);
    const place = scene.places.find((p: any) => {
      const n = mergedVerses(p).length;
      return n >= 1 && n <= 2;
    });
    if (place) {
      match = { w, place };
      break;
    }
  }
  if (!match) {
    test.skip(true, 'no place with 1-2 merged verses found in any candidate window');
    return;
  }

  await page.goto(`/world?from=${match.w.from}&to=${match.w.to}`);
  await page.getByTestId(`marker-${match.place.id}`).hover({ force: true });
  const card = page.getByTestId('place-card');
  await expect(card).toBeVisible();
  await expect(card.getByTestId(/^hover-verse-/)).toHaveCount(mergedVerses(match.place).length);
  await expect(card.getByTestId('place-card-expand')).toHaveCount(0);
});
