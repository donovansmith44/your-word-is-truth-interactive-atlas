import { defineConfig } from '@playwright/test';
export default defineConfig({
  // 60s was too tight for SCENE-1/2 + ARROW-1..7 (api-scene.spec.ts) at the
  // default FC_NUM_RUNS=150: measured against a live release-mode server,
  // network time is negligible (~5ms/run avg, ~8ms worst case) but the
  // property's own dense expect() checks (proportional to real scene size —
  // up to 167 places / ~500 place-events / 23 arrows for a full-span window)
  // cost ~500ms/run on average and up to ~1.7s for the largest scene, all in
  // Playwright's own matcher overhead, not server latency. 150 runs of that
  // extrapolates to ~80-100s, so 60s failed the test even though every
  // individual assertion passed. Raised with a comfortable margin rather
  // than tuned to the exact measured worst case.
  //
  // Task 16 investigation: with the full 13-narrative dataset (was 3), that
  // same property started taking 10-25+ minutes -- past even a 900s budget
  // tried mid-investigation -- and was mistakenly first attributed to
  // worker/CPU contention from the other 35 tests. Root cause turned out to
  // be something else entirely, confirmed by elimination: a standalone Node
  // script running the EXACT same fetches/conditions with plain `if (!cond)
  // throw` instead of Playwright's `expect()` completed 500 iterations in
  // 0.8s flat (zero violations, ruling out a data/logic bug), while the
  // property run through Playwright's `expect()` -- called 5-10+ times per
  // arrow, thousands of times per iteration -- was the one still not
  // finished after 25+ minutes. `expect()`'s per-call diagnostics (stack
  // capture etc.) are fine at normal call volumes but not built for a
  // property test replaying a dense assertion loop hundreds of times a run.
  // Fix: api-scene.spec.ts's hot loop now uses a plain `ok(cond, msg)`
  // throw-helper instead of expect() (same pass/fail semantics, thousands
  // fewer tracked assertions) -- confirmed back down to <1s for 500
  // iterations even through Playwright.
  //
  // Separately, WORLD-1 (world-map.spec.ts) is genuinely, linearly slower at
  // a raised FC_NUM_RUNS -- unlike the SCENE properties, its cost is real
  // per-iteration browser work (a full `page.goto` reload of the WASM app
  // every run, not an expect()-count artifact): measured 41.8s at the
  // default 20 runs, so FC_NUM_RUNS=60 (RUNS_UI, tier 2 of the deep run)
  // extrapolates to ~125s and did in fact just clear a 120s budget. 240s
  // covers that with real margin (~2x the extrapolated figure) without
  // reintroducing the earlier 900s blanket padding -- every other test,
  // including the next-slowest browser-driven ones, finishes well under
  // 120s even at FC_NUM_RUNS=60.
  timeout: 240_000,
  retries: 0,
  use: { baseURL: 'http://localhost:5000', trace: 'retain-on-failure' },
  webServer: [
    // 300s (was 120s): start-api.ps1 boots the API via `cargo run --release`,
    // kept for its ~5x per-request latency win across every suite run; a cold
    // release build/link on a first run (or after a Rust change) can take
    // meaningfully longer than a cold debug build, so the boot budget is
    // raised to absorb that rather than risk a spurious webServer timeout.
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-api.ps1',
      url: 'http://localhost:8000/health', reuseExistingServer: true, timeout: 300_000 },
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-client.ps1',
      url: 'http://localhost:5000', reuseExistingServer: true, timeout: 180_000 },
  ],
});
