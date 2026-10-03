import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const sources = JSON.parse(fs.readFileSync(path.resolve(__dirname, '../../data/compiled/sources.json'), 'utf8'));
const respond = { json: sources, headers: { 'Access-Control-Allow-Origin': '*' } };

test('the independent WebAssembly client renders every served source and shared stylesheet', async ({ page }) => {
    const errors: string[] = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/api/sources', route => route.fulfill(respond));
    await page.goto('/sources');
    await expect(page.locator('.source-card')).toHaveCount(sources.sources.length);
    const actual = await page.locator('.sources-category').evaluateAll(categories => categories.map(category => ({
        id: category.getAttribute('data-testid')?.replace('sources-category-', ''),
        title: category.querySelector('h2')?.textContent,
        sources: Array.from(category.querySelectorAll('.source-card')).map(source => ({
            id: source.getAttribute('data-testid')?.replace('source-', ''),
            title: source.querySelector('h3')?.textContent,
            what: source.querySelector('.source-what')?.textContent,
            built: source.querySelector('.source-built')?.textContent,
            license: source.querySelector('.source-license')?.textContent,
            link: source.querySelector('a')?.getAttribute('href') ?? null,
        })),
    })));
    expect(actual).toEqual(sources.categories.map((category: { id: string, label: string }) => ({
        id: category.id,
        title: category.label,
        sources: sources.sources.filter((source: { category: string }) => source.category === category.id).map((source: Record<string, string>) => ({
            id: source.id, title: source.title, what: source.what_it_is,
            built: `What we built: ${source.what_we_built}`, license: `License: ${source.license}`, link: source.link ?? null,
        })),
    })));
    expect(await page.locator('.app-header').evaluate(element => getComputedStyle(element).display)).toBe('flex');
    const ledger = JSON.parse(fs.readFileSync(path.resolve(__dirname, 'ledger.json'), 'utf8'));
    const actualAssets = [];
    const expectedAssets = [];
    for (const entry of ledger.inventory.filter((entry: { status: string }) => entry.status === 'shared-asset')) {
        const response = await page.request.get('/' + entry.source.replace('client/wwwroot/', ''));
        actualAssets.push({ path: entry.source, status: response.status(), hash: crypto.createHash('sha256').update(await response.body()).digest('hex') });
        expectedAssets.push({ path: entry.source, status: 200, hash: entry.sha256 });
    }
    expect(actualAssets).toEqual(expectedAssets);
    expect(errors).toEqual([]);
});

test('a failed WebAssembly read can retry through the Elmish command interpreter', async ({ page }) => {
    let requests = 0;
    await page.route('**/api/sources', route => ++requests === 1
        ? route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'offline' } }, headers: respond.headers })
        : route.fulfill(respond));
    await page.goto('/sources');
    await page.getByTestId('could-not-load-retry').click();
    await expect(page.locator('.source-card')).toHaveCount(sources.sources.length);
    expect(requests).toBe(2);
});


test('a malformed answer is a contract failure instead of an empty source card', async ({ page }) => {
    await page.route('**/api/sources', route => route.fulfill({ json: { categories: sources.categories, sources: [{ id: 'missing', title: 'Incomplete' }] }, headers: respond.headers }));
    await page.goto('/sources');
    await expect(page.getByTestId('could-not-load')).toBeVisible();
    await expect(page.locator('.source-card')).toHaveCount(0);
});
