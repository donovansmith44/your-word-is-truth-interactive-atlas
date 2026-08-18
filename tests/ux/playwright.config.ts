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
  timeout: 300_000,
  retries: 0,
  use: { baseURL: 'http://localhost:5000', trace: 'retain-on-failure' },
  webServer: [
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-api.ps1',
      url: 'http://localhost:8000/health', reuseExistingServer: true, timeout: 120_000 },
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-client.ps1',
      url: 'http://localhost:5000', reuseExistingServer: true, timeout: 180_000 },
  ],
});
