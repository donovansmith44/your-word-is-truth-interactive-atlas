import { defineConfig } from '@playwright/test';
import path from 'node:path';

const published = process.env.FSHARP_PUBLISHED === '1';

export default defineConfig({
    testDir: '.',
    testMatch: '*.spec.ts',
    timeout: 30_000,
    workers: 1,
    use: { baseURL: 'http://127.0.0.1:5100', trace: 'retain-on-failure' },
    webServer: {
        command: published ? 'python3 scripts/fsharp-client/serve-published.py' : 'dotnet run --project client-fsharp --no-launch-profile --no-build --urls http://127.0.0.1:5100',
        cwd: path.resolve(__dirname, '../..'),
        url: 'http://127.0.0.1:5100',
        reuseExistingServer: false,
        timeout: 60_000,
        env: { ASPNETCORE_ENVIRONMENT: 'Development' },
    },
});
