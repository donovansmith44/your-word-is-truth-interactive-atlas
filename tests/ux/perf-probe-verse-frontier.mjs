// PERF-3 measurement harness: verse-click -> frontier-rendered latency.
//
// Deliberately NOT a *.spec.ts file (Playwright's default testMatch is
// `**/*.@(spec|test).?(c|m)[jt]s`) -- this is a scripted probe, run
// explicitly (`node perf-probe-verse-frontier.mjs`), never picked up by
// `npx playwright test`. Per the PERF-2b ceiling lesson (batch-perf2-brief.md):
// a browser/network-timing measurement is noise-prone by nature and must
// NOT join the green/red suite -- this is a perf-probe, not a test.
//
// 127.0.0.1, never localhost -- perf_smoke.rs's own header comment
// documents a ~200ms hostname-resolution tax on this machine's "localhost".
//
// Methodology (PHASE 0 finding, live-verified): a `page.goto()` to a NEW
// URL is a REAL browser navigation -- it reboots the whole Blazor WASM
// runtime from scratch (fresh module instantiation, fresh interpreter
// state), which is NOT what a real reader does when moving between
// chapters (Reader.razor's own reader-prev/reader-next links use Blazor's
// client-side router -- no reload). Conflating the two badly overstates
// "typical" click cost with a one-time-per-session interpreter/JS-import
// warmup that has nothing to do with this batch's own fix. This probe
// measures BOTH, separately and honestly:
//   - COLD samples: a fresh page (fresh WASM boot) for each one, first
//     verse click after `waitUntil: 'networkidle'` + a settle delay. This
//     is the one-time-per-session cost a reader pays exactly once.
//   - WARM samples: ONE page load, then repeated verse clicks across many
//     DIFFERENT chapters reached via the app's own in-app reader-next
//     link (real SPA navigation, no reload) -- this is what "click on a
//     verse" costs for every click after the first one in a real session,
//     which is the overwhelming majority of real usage.
import { chromium } from '@playwright/test';

const BASE = process.env.PERF3_BASE || 'http://127.0.0.1:8000';
const COLD_TARGETS = [
  ['GEN', 1, 1], ['EXO', 20, 3], ['PSA', 23, 1], ['ISA', 53, 5], ['MAT', 5, 3],
  ['JHN', 3, 16], ['ROM', 8, 28], ['GEN', 1, 5], ['PSA', 23, 2], ['JHN', 3, 17],
];
const WARM_SAMPLE_COUNT = 12;

function median(xs) {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}
function p95(xs) {
  const s = [...xs].sort((a, b) => a - b);
  return s[Math.min(s.length - 1, Math.ceil(s.length * 0.95) - 1)];
}

async function clickFirstVerseAndMeasure(page, verseNum = 1) {
  const clickStart = Date.now();
  await page.getByTestId(`verse-line-${verseNum}`).focus();
  await page.keyboard.press('Enter');
  await page.waitForSelector('[data-testid^="popover-section-"]', { state: 'attached', timeout: 15000 });
  const wall = Date.now() - clickStart;
  await page.keyboard.press('Escape');
  await page.waitForSelector('[data-testid="popover-backdrop"]', { state: 'detached', timeout: 5000 }).catch(() => {});
  return wall;
}

async function measureCold(browser, book, chapter, verse) {
  const page = await browser.newPage();
  await page.goto(`${BASE}/read/${book}/${chapter}`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(300); // let the runtime finish settling before the FIRST click of this fresh session
  const wall = await clickFirstVerseAndMeasure(page, verse);
  await page.close();
  return wall;
}

async function measureWarm(browser) {
  // ONE page load (one WASM boot), then move between chapters via the
  // app's OWN client-side router (reader-next -- a real Blazor-routed
  // anchor, never a fresh document load) -- this is what every click AFTER
  // the first one in a real reading session actually costs. Crosses book
  // boundaries naturally (Genesis 1 -> ... -> Exodus) as reader-next walks
  // the canon TOC forward.
  const page = await browser.newPage();
  await page.goto(`${BASE}/read/GEN/1`, { waitUntil: 'networkidle' });
  await page.waitForTimeout(300);
  // Burn the one-time session warmup on a throwaway click, deliberately
  // excluded from the WARM sample set (that cost is what COLD measures).
  await clickFirstVerseAndMeasure(page, 1);

  const walls = [];
  for (let i = 0; i < WARM_SAMPLE_COUNT; i++) {
    await page.getByTestId('reader-next').click();
    await page.waitForSelector('[data-testid="verse-line-1"]', { state: 'attached' });
    walls.push(await clickFirstVerseAndMeasure(page, 1));
  }
  await page.close();
  return walls;
}

async function main() {
  const browser = await chromium.launch();

  const coldWalls = [];
  for (const [b, c, v] of COLD_TARGETS) {
    coldWalls.push(await measureCold(browser, b, c, v));
  }

  const warmWalls = await measureWarm(browser);

  await browser.close();

  const report = {
    base: BASE,
    cold: { n: coldWalls.length, walls: coldWalls, median: median(coldWalls), p95: p95(coldWalls) },
    warm: { n: warmWalls.length, walls: warmWalls, median: median(warmWalls), p95: p95(warmWalls) },
  };
  console.log(JSON.stringify(report, null, 2));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
