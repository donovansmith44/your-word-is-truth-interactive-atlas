import { test, expect } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';

const fixture = (name: string) => JSON.parse(fs.readFileSync(path.resolve(__dirname, `../../contracts/atlas-query-contract/fixtures/${name}.json`), 'utf8')).body;
const headers = { 'Access-Control-Allow-Origin': '*' };

test('the reader composes the served chapter text and requests a clicked anchor once', async ({ page }) => {
    const contents = fixture('contents-bible');
    const window = fixture('text-window-single');
    const requests: { path: string, query: [string, string][] }[] = [];
    const errors: string[] = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.route('**/api/**', async route => {
        const uri = new URL(route.request().url());
        requests.push({ path: uri.pathname, query: [...uri.searchParams] });
        if (uri.pathname === '/api/contents/bible') return route.fulfill({ json: contents, headers });
        if (uri.pathname === '/api/text') return route.fulfill({ json: window, headers });
        const anchor = window.units[0].body.anchors[0].node;
        if (uri.pathname === '/api/elements') return route.fulfill({ json: {
            version: window.version, next: null,
            elements: [{ element: 'node', node: { ...anchor, version: window.version, provenance: 'served', edge_summary: [] } }],
        }, headers });
        throw new Error(`unexpected read: ${uri}`);
    });
    await page.goto('/read/JHN/3');
    await expect(page.locator('.verse-line')).toHaveCount(window.units.length);
    const actual = await page.locator('.verse-line').evaluateAll(rows => rows.map(row => ({
        number: row.querySelector('.verse-num')?.textContent,
        text: row.querySelector('.verse-text')?.textContent,
        anchors: Array.from(row.querySelectorAll('.verse-mention')).map(anchor => ({ text: anchor.textContent, label: anchor.getAttribute('aria-label') })),
        red: Array.from(row.querySelectorAll('.words-of-christ')).map(run => run.textContent),
    })));
    expect(actual).toEqual(window.units.map((unit: Record<string, any>) => ({
        number: String(unit.body.locus.verse), text: unit.body.text,
        anchors: unit.body.anchors.map((anchor: Record<string, any>) => ({ text: [...unit.body.text].slice(anchor.start, anchor.end).join(''), label: `Explore ${anchor.node.label}` })),
        red: unit.body.words_of_christ.map((run: { start: number, end: number }) => [...unit.body.text].slice(run.start, run.end).join('')),
    })));
    const anchorRead = page.waitForRequest(request => new URL(request.url()).pathname === '/api/elements');
    await page.locator('.verse-mention').first().focus();
    await page.keyboard.press('Enter');
    await anchorRead;
    await expect(page.getByTestId('popover-card-title')).toHaveText(window.units[0].body.anchors[0].node.label);
    await page.getByTestId('popover-close').click();
    await expect(page.getByTestId('popover')).toHaveCount(0);
    expect(requests).toEqual([
        { path: '/api/contents/bible', query: [] },
        { path: '/api/text', query: [['ref', 'JHN.3'], ['scope', 'chapter'], ['corpus', 'bible']] },
        { path: '/api/elements', query: [['ids', window.units[0].body.anchors[0].node.id]] },
    ]);
    expect(errors).toEqual([]);
});

test('Concord loads a bounded page and retains its served citation text', async ({ page }) => {
    const contents = fixture('contents-concord');
    const citation = { start: 5, end: 14, kind: 'cites', node: { id: 'TextUnit:served-citation', kind: 'TextUnit', label: 'John 3:16' } };
    const unit = {
        ref: contents.roots[0].ref, node: { id: 'TextUnit:served-concord', kind: 'TextUnit', label: contents.roots[0].ref },
        heading: null, edge_summary: [{ kind: 'cites', count: 1 }],
        body: { text: 'Read John 3:16.', locus: contents.roots[0].locus, anchors: [citation], words_of_christ: [] },
    };
    const requests: { path: string, query: [string, string][] }[] = [];
    await page.route('**/api/**', route => {
        const uri = new URL(route.request().url());
        requests.push({ path: uri.pathname, query: [...uri.searchParams] });
        return uri.pathname === '/api/contents/concord'
            ? route.fulfill({ json: contents, headers })
            : route.fulfill({ json: { units: [unit], next: null, version: contents.version }, headers });
    });
    await page.goto('/concord');
    await expect(page.locator('.concord-unit')).toHaveCount(1);
    expect(await page.locator('.concord-unit').evaluate(row => ({
        reference: row.querySelector('.concord-unit-ref')?.textContent,
        text: row.textContent?.slice(row.querySelector('.concord-unit-ref')?.textContent?.length ?? 0),
        citations: Array.from(row.querySelectorAll('.concord-ref')).map(anchor => ({ text: anchor.textContent, label: anchor.getAttribute('aria-label') })),
        role: row.getAttribute('role'),
    }))).toEqual({ reference: unit.ref, text: unit.body.text, citations: [{ text: 'John 3:16', label: 'Explore John 3:16' }], role: 'button' });
    expect(requests).toEqual([
        { path: '/api/contents/concord', query: [] },
        { path: '/api/text', query: [['ref', unit.ref], ['n', '20'], ['corpus', 'concord']] },
    ]);
});

test('Concord retries the failed next page without reopening the old page or growing the reading', async ({ page }) => {
    const contents = fixture('contents-concord');
    const firstRef = contents.roots[0].ref;
    const nextRef = 'BoC 1.1.21';
    const unit = (reference: string, body: string) => ({
        ref: reference, node: { id: `TextUnit:${reference}`, kind: 'TextUnit', label: reference },
        edge_summary: [], heading: null,
        body: { text: body, locus: contents.roots[0].locus, anchors: [], words_of_christ: [] },
    });
    const first = unit(firstRef, 'First served page.');
    const next = unit(nextRef, 'Next served page.');
    const requests: { path: string, query: [string, string][] }[] = [];
    let nextAttempts = 0;
    await page.route('**/api/**', route => {
        const uri = new URL(route.request().url());
        requests.push({ path: uri.pathname, query: [...uri.searchParams] });
        if (uri.pathname === '/api/contents/concord') return route.fulfill({ json: contents, headers });
        if (uri.searchParams.get('ref') === firstRef) return route.fulfill({ json: { units: [first], next: nextRef, version: contents.version }, headers });
        if (++nextAttempts === 1) return route.fulfill({ status: 503, json: { error: { code: 'unavailable', message: 'offline' } }, headers });
        return route.fulfill({ json: { units: [next], next: null, version: contents.version }, headers });
    });
    await page.goto('/concord');
    await page.getByTestId('concord-next').click();
    await page.getByTestId('could-not-load-retry').click();
    await expect(page.locator('.concord-unit')).toHaveCount(1);
    expect(await page.locator('.concord-unit').allTextContents()).toEqual([nextRef + next.body.text]);
    await expect(page.getByTestId('concord-next')).toHaveCount(0);
    const nextQuery: [string, string][] = [['ref', nextRef], ['n', '20'], ['dir', 'onward'], ['corpus', 'concord']];
    expect(requests).toEqual([
        { path: '/api/contents/concord', query: [] },
        { path: '/api/text', query: [['ref', firstRef], ['n', '20'], ['corpus', 'concord']] },
        { path: '/api/text', query: nextQuery },
        { path: '/api/text', query: nextQuery },
    ]);
});
