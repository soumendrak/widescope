import { defineConfig, devices } from '@playwright/test';

// E2E runs against the Vite dev server (which serves the WASM-backed editor at
// /editor/). Build the WASM package first: `just build-wasm`.
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? 'list' : 'line',
  use: {
    baseURL: 'http://localhost:5173',
    trace: 'on-first-retry',
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:5173/editor/',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
