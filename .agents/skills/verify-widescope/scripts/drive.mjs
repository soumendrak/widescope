#!/usr/bin/env node
import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { parseArgs } from 'node:util';

const VIEW_ROOT = {
  Waterfall: '.wf-root',
  Flame: 'canvas.flame-canvas',
  Conversation: '.conversation',
  Timeline: '.timeline-root',
  Graph: '.graph-wrap',
  Diff: '.diff-empty',
};

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: { 'expect-spans': { type: 'string' }, 'expect-error': { type: 'string' }, tabs: { type: 'boolean' } },
});
const [baseUrl, traceFile, outDir] = positionals;
if (!baseUrl || !traceFile || !outDir) {
  console.error('usage: drive.mjs <base-url> <trace-file> <evidence-dir> [--expect-spans N | --expect-error TEXT] [--tabs]');
  process.exit(2);
}

const require = createRequire(resolve('ui/package.json'));
const { chromium } = require('@playwright/test');
const run = relative(process.cwd(), resolve(traceFile)).replace(/\W+/g, '-');
mkdirSync(outDir, { recursive: true });
const shot = (page, step) => page.screenshot({ path: `${outDir}/${run}-${step}.png` });

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
const consoleErrors = [];
page.on('console', (message) => message.type() === 'error' && consoleErrors.push(message.text()));
const summary = { file: traceFile, url: `${baseUrl}/editor/`, failures: [] };

await page.goto(summary.url);
await page.getByRole('button', { name: /Open file/ }).first().waitFor();
await shot(page, '1-before');

const chooser = page.waitForEvent('filechooser');
await page.getByRole('button', { name: /Open file/ }).first().click();
await (await chooser).setFiles(resolve(traceFile));

const loaded = page.locator('.stats-bar');
const banner = page.locator('[role="alert"]');
await loaded.or(banner).first().waitFor({ timeout: 30_000 });
await shot(page, '2-after-open');

if (await banner.isVisible()) summary.error = (await banner.innerText()).trim();
if (await loaded.isVisible()) {
  summary.spans = Number(await page.locator('.stat', { hasText: 'spans' }).first().locator('b').innerText());
  summary.services = Number(await page.locator('.stat', { hasText: 'services' }).first().locator('b').innerText());
}

if (values['expect-spans'] !== undefined && summary.spans !== Number(values['expect-spans'])) {
  summary.failures.push(`expected ${values['expect-spans']} spans, saw ${summary.spans ?? 'no trace'}`);
}
if (values['expect-error'] !== undefined && !summary.error?.includes(values['expect-error'])) {
  summary.failures.push(`expected an error containing "${values['expect-error']}", saw ${summary.error ?? 'none'}`);
}
if (values['expect-error'] === undefined && summary.error) summary.failures.push(`unexpected error: ${summary.error}`);

if (values.tabs) {
  summary.tabs = [];
  const tabs = page.locator('.view-tabs [role="tab"]');
  for (let i = 0; i < (await tabs.count()); i++) {
    const tab = tabs.nth(i);
    const name = ((await tab.locator('.vt-label').textContent()) ?? '').trim();
    await tab.click();
    const selected = (await tab.getAttribute('aria-selected')) === 'true';
    const root = VIEW_ROOT[name];
    const shown = root === undefined ? false : await page.locator(`#view-panel ${root}`).first().waitFor({ timeout: 5_000 }).then(() => true, () => false);
    const rendered = shown && !(await banner.isVisible());
    await shot(page, `3-tab-${name.replace(/\W+/g, '-')}`);
    summary.tabs.push({ name, selected, rendered });
    if (!selected || !rendered) summary.failures.push(`tab ${name}: selected=${selected} rendered=${rendered}`);
  }
}

summary.consoleErrors = consoleErrors;
summary.ok = summary.failures.length === 0;
writeFileSync(`${outDir}/${run}-drive.json`, `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify(summary, null, 2));
await browser.close();
process.exit(summary.ok ? 0 : 1);
