import { expect, test } from '@playwright/test';

test('legacy query share links redirect to the editor', async ({ page }) => {
  const remote = 'https://example.com/traces/sample.json?x=1';
  await page.route('https://example.com/**', (route) => route.abort());
  await page.goto(`/?trace=${encodeURIComponent(remote)}&view=flame&span=s-1`);

  await expect(page).toHaveURL(/\/editor\/\?/);
  const landed = new URL(page.url());
  expect(landed.pathname).toBe('/editor/');
  expect(landed.searchParams.get('trace')).toBe(remote);
  expect(landed.searchParams.get('view')).toBe('flame');
  expect(landed.searchParams.get('span')).toBe('s-1');
});

test('legacy hash share links redirect to the editor', async ({ page }) => {
  await page.goto('/#trace=H4sIABC&view=timeline&span=s-42');

  await expect(page).toHaveURL(/\/editor\/#/);
  const landed = new URL(page.url());
  expect(landed.pathname).toBe('/editor/');
  expect(landed.hash).toBe('#trace=H4sIABC&view=timeline&span=s-42');
});

test('known pages answer and unknown paths do not', async ({ request }) => {
  expect((await request.get('/nonexistent-page')).status()).toBe(404);

  expect((await request.get('/')).status()).toBe(200);
  expect((await request.get('/editor/')).status()).toBe(200);
  expect((await request.get('/docs/')).status()).toBe(200);
  expect((await request.get(`/?trace=${encodeURIComponent('https://example.com/t.json')}`)).status()).toBe(200);
  expect((await request.get(`/editor/?trace=${encodeURIComponent('https://example.com/t.json')}`)).status()).toBe(200);
});
