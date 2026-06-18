import { expect, test } from '@playwright/test';

// The sample fixture (test-fixtures/otlp/sample_llm_pipeline.json) has 7 spans
// across 3 services. ?sample=1 deep-links the editor to auto-load it.
const SAMPLE_SPAN_COUNT = 7;

test.beforeEach(async ({ page }) => {
  await page.goto('/editor/?sample=1');
  // Wait until WASM has parsed the trace and the stats bar is populated.
  await expect(page.locator('.stats-bar')).toBeVisible({ timeout: 30_000 });
});

test('stats bar shows the sample span count', async ({ page }) => {
  await expect(page.locator('.stat', { hasText: 'spans' })).toContainText(
    String(SAMPLE_SPAN_COUNT),
  );
});

test('every view tab renders without error', async ({ page }) => {
  const tabs = page.locator('.view-tabs [role="tab"]');
  const count = await tabs.count();
  expect(count).toBeGreaterThanOrEqual(5);

  for (let i = 0; i < count; i++) {
    const tab = tabs.nth(i);
    await tab.click();
    await expect(tab).toHaveAttribute('aria-selected', 'true');
    // The main view region stays mounted on every switch.
    await expect(page.locator('.workspace, main').first()).toBeVisible();
  }
});

test('clicking a span opens the inspector sidebar', async ({ page }) => {
  // Flame view is default; the root span fills the full width of the top row.
  await page.locator('.view-tab', { hasText: 'Flame' }).click();
  const canvas = page.locator('canvas.flame-canvas');
  await expect(canvas).toBeVisible();

  const box = await canvas.boundingBox();
  expect(box).not.toBeNull();
  // Top-left of the canvas lands inside the root span's bar (depth 0).
  await canvas.click({ position: { x: Math.min(40, box!.width / 2), y: 10 } });

  await expect(page.locator('aside.sidebar')).toBeVisible();
  await expect(page.locator('.sidebar-toolbar-title')).toContainText('span inspector');
});

test('search narrows the result count', async ({ page }) => {
  const search = page.getByLabel('Search spans');
  await search.fill('chat');
  // A non-empty query updates the search status text with a match count.
  await expect(page.locator('.search-status')).toBeVisible();
  await expect(page.locator('.search-status')).not.toBeEmpty();
});
