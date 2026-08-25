import { test, expect } from '@playwright/test';
import { api } from './lib/api';

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

  test('RED-1: red letters also render in the verse popover focal preview (a peek), not just the primary reader column', async ({ page }) => {
    await page.goto('/read/MAT/4');
    await page.getByTestId('verse-line-19').click();
    await expect(page.getByTestId('popover-title')).toHaveText('MAT.4.19');
    // VerseTextSection's own compact FOCUS preview -- the SAME MentionText
    // component, the SAME rule, a genuinely different rendering surface
    // (CONTRACT.md's own RED-1 law: "Present on EVERY surface MENTION-1
    // above already lists"). Scoped to popover-section-verse-text
    // specifically (not the whole popover): a real, unplanned discovery
    // this test itself caught live -- MAT.4.19's own cross-references
    // section ALSO now renders red letters in its OWN passage previews
    // (parallel Gospel accounts of the identical "Follow me" saying,
    // e.g. MRK.1.16-20/LUK.5.1-11 -- PassageList.razor's own coverage,
    // CONTRACT.md's RED-1 law's "PassageList.razor's/ArrowNav.razor's own
    // compact passage previews" clause), so a bare `.popover
    // .words-of-christ` locator is genuinely ambiguous now (a correct
    // consequence of broader coverage, not a bug) -- the focal preview's
    // own dedicated testid disambiguates.
    const focal = page.getByTestId('popover-section-verse-text');
    await expect(focal.locator('.words-of-christ')).toHaveText('Follow me, and I will make you fishers of men.');
  });

  test('RED-1: a parallel-account cross-reference preview also renders its own red letters (PassageList.razor coverage)', async ({ page }) => {
    await page.goto('/read/MAT/4');
    await page.getByTestId('verse-line-19').click();
    await expect(page.getByTestId('popover-title')).toHaveText('MAT.4.19');
    const xrefSection = page.getByTestId('xrefs-section-heading');
    await expect(xrefSection).toBeVisible();
    // At least one cross-reference passage preview must carry a red span
    // (the parallel-account "Follow me" sayings in Mark/Luke) -- proves
    // decision 5's "ONE render rule" reaches this fourth surface too, not
    // merely the three the batch brief names by name.
    const anyPassageRed = page.locator('[data-testid^="xref-item-"] .words-of-christ, [data-testid^="verse-parallel-"] .words-of-christ');
    await expect(anyPassageRed.first()).toBeVisible();
  });
});
