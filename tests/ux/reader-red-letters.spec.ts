import { test, expect } from '@playwright/test';
import { openVerse } from './lib/verse';
import { api } from './lib/api';
import { neighbourNode } from './lib/edges';

// Batch RED-1 (owner order 2026-08-25, verbatim: "Red letters on Jesus'
// words in every translation"; "SpokenAt is another edge"): CONTRACT.md's
// own RED-1 law -- a `.words-of-christ` CSS span (KJV, sub-verse
// precision) rendered wherever verse text renders, through the SAME
// shared `MentionText.razor` component every in-text mention already
// renders through. This file proves the THREE named spot-checks the batch
// brief requires verbatim (decision 6): MAT.4.19 ("Follow me" -- the
// narration prefix is NOT red, the speech is), a fully-red verse (MAT.5.4
// class), and a no-red verse (GEN.1.1) -- each a real, wire-confirmed
// case (the server's own `words_of_christ` field is read first, never
// assumed), then a real Reader-page render assertion.

test.describe('Batch RED-1: red letters (words of Christ)', () => {
  test('RED-1: MAT.4.19 "Follow me" -- the narration prefix is NOT red, the speech is', async ({ page }) => {
    const chapterOut = await api.chapter('MAT.4');
    const v19 = (chapterOut.verses as { verse: number; text: string; words_of_christ?: { start: number; end: number }[] }[]).find(v => v.verse === 19);
    expect(v19, 'MAT.4.19 must exist in the real compiled chapter').toBeTruthy();
    expect(v19!.words_of_christ, 'MAT.4.19 must carry exactly one aligned red-letter span on the wire').toHaveLength(1);
    const span = v19!.words_of_christ![0];
    const redText = v19!.text.slice(span.start, span.end);
    expect(redText, 'the wire span itself must be the speech, not the narration').toBe('Follow me, and I will make you fishers of men.');
    expect(v19!.text.slice(0, span.start), 'the narration prefix must precede the red span').toBe('And he saith unto them, ');

    await page.goto('/read/MAT/4');
    const line = page.getByTestId('verse-line-19');
    await expect(line).toBeVisible();
    const redSpan = line.locator('.words-of-christ');
    await expect(redSpan).toHaveCount(1);
    await expect(redSpan).toHaveText('Follow me, and I will make you fishers of men.');
    // The narration prefix renders in the line but OUTSIDE the red span --
    // confirmed by reading the verse line's own full text and subtracting
    // the red span's text, rather than asserting on color (a genuine,
    // load-bearing structural check, not merely a presence check).
    const fullLineText = await line.innerText();
    expect(fullLineText.startsWith('19'), 'sanity: the verse-num renders first').toBeTruthy();
    expect(fullLineText).toContain('And he saith unto them, Follow me, and I will make you fishers of men.');
    // Report contract (batch-red1-brief.md, "quote the rendered MAT 4:19
    // span HTML in the report"): capture the red span's own outerHTML.
    const outerHtml = await redSpan.evaluate(el => el.outerHTML);
    console.log('RED-1 MAT.4.19 rendered span outerHTML:', outerHtml);
  });

  test('RED-1: a fully-red verse (MAT.5.4 class) -- the whole verse text is inside one red span', async ({ page }) => {
    const chapterOut = await api.chapter('MAT.5');
    const v4 = (chapterOut.verses as { verse: number; text: string; words_of_christ?: { start: number; end: number }[] }[]).find(v => v.verse === 4);
    expect(v4, 'MAT.5.4 must exist in the real compiled chapter').toBeTruthy();
    expect(v4!.words_of_christ).toHaveLength(1);
    const span = v4!.words_of_christ![0];
    expect(span.start, 'a fully-red verse spot starts at char 0').toBe(0);
    expect(span.end, 'and runs to the verse own full length').toBe(v4!.text.length);

    await page.goto('/read/MAT/5');
    const line = page.getByTestId('verse-line-4');
    await expect(line).toBeVisible();
    const redSpan = line.locator('.words-of-christ');
    await expect(redSpan).toHaveCount(1);
    await expect(redSpan).toHaveText(v4!.text);
  });

  test('RED-1: a no-red verse (GEN.1.1) -- no .words-of-christ element renders at all', async ({ page }) => {
    const chapterOut = await api.chapter('GEN.1');
    const v1 = (chapterOut.verses as { verse: number; words_of_christ?: { start: number; end: number }[] }[]).find(v => v.verse === 1);
    expect(v1, 'GEN.1.1 must exist in the real compiled chapter').toBeTruthy();
    expect(v1!.words_of_christ ?? [], 'GEN.1.1 must carry zero red-letter spans on the wire').toHaveLength(0);

    await page.goto('/read/GEN/1');
    const line = page.getByTestId('verse-line-1');
    await expect(line).toBeVisible();
    await expect(line.locator('.words-of-christ')).toHaveCount(0);
  });

  test('RED-1: red letters also render in the verse popover\'s own text, not just the primary reader column', async ({ page }) => {
    // Arrange
    const record = await api.node('text-unit:MAT.4.19');
    const spans = record.text.words_of_christ.map((span: { start: number; end: number }) => record.text.text.slice(span.start, span.end));
    await page.goto('/read/MAT/4');

    // Act
    await openVerse(page, 19);

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(record.label);
    await expect(page.getByTestId('popover-text').locator('.words-of-christ')).toHaveText(spans);
  });

  test('RED-1: a cross reference followed from the popover renders its own red letters', async ({ page }) => {
    // Arrange
    const cites = await api.nodeEdges('text-unit:MAT.4.19', 'cites');
    const target = neighbourNode(cites.entries[0]);
    const record = await api.node(target.id);
    const spans = record.text.words_of_christ.map((span: { start: number; end: number }) => record.text.text.slice(span.start, span.end));
    expect(spans.length, `${target.id} must carry words of Christ on the wire`).toBeGreaterThan(0);
    await page.goto('/read/MAT/4');
    await openVerse(page, 19);

    // Act
    await page.getByTestId(`popover-link-cites-${target.id}`).click();

    // Assert
    await expect(page.getByTestId('popover-title')).toHaveText(target.label);
    await expect(page.getByTestId('popover-text').locator('.words-of-christ')).toHaveText(spans);
  });
});
