import { defineConfig } from '@playwright/test';
export default defineConfig({
  timeout: 60_000,
  retries: 0,
  use: { baseURL: 'http://localhost:5000', trace: 'retain-on-failure' },
  webServer: [
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-api.ps1',
      url: 'http://localhost:8000/health', reuseExistingServer: true, timeout: 120_000 },
    { command: 'powershell -NoProfile -ExecutionPolicy Bypass -File start-client.ps1',
      url: 'http://localhost:5000', reuseExistingServer: true, timeout: 180_000 },
  ],
});
