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
  options: { 'expect-spans': { type: 'string' }, 'expect-format': { type: 'string' }, 'expect-tabs': { type: 'string' }, 'expect-error': { type: 'string' }, tabs: { type: 'boolean' } },
});
const [baseUrl, traceFile, outDir] = positionals;
if (!baseUrl || !traceFile || !outDir) {
  console.error('usage: drive.mjs <base-url> <trace-file> <evidence-dir> [--expect-spans N] [--expect-format LABEL] [--expect-error TEXT] [--tabs] [--expect-tabs A,B,...]');
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
page.on('pageerror', (error) => consoleErrors.push(`uncaught: ${error.message}`));
const summary = { file: traceFile, url: `${baseUrl}/editor/`, failures: [] };

await page.goto(summary.url);
await page.getByRole('button', { name: /Open file/ }).first().waitFor();
await shot(page, '1-before');

const chooser = page.waitForEvent('filechooser');
await page.getByRole('button', { name: /Open file/ }).first().click();
const fileChooser = await chooser;
const accept = (await fileChooser.element().getAttribute('accept')) ?? '';
if (accept && !accept.split(',').some((type) => traceFile.toLowerCase().endsWith(type.trim()))) summary.failures.push(`the file picker accepts ${accept}, so a user cannot pick ${traceFile}`);
await fileChooser.setFiles(resolve(traceFile));

const loaded = page.locator('.stats-bar');
const banner = page.locator('[role="alert"]');
await loaded.or(banner).first().waitFor({ timeout: 30_000 });
await shot(page, '2-after-open');

if (await banner.isVisible()) summary.error = (await banner.innerText()).trim();
if (await loaded.isVisible()) {
  summary.format = ((await page.locator('.format-badge').textContent()) ?? '').trim();
  summary.spans = Number(await page.locator('.stat', { hasText: 'spans' }).first().locator('b').innerText());
  summary.services = Number(await page.locator('.stat', { hasText: 'services' }).first().locator('b').innerText());
}

if (values['expect-spans'] !== undefined && summary.spans !== Number(values['expect-spans'])) {
  summary.failures.push(`expected ${values['expect-spans']} spans, saw ${summary.spans ?? 'no trace'}`);
}
if (values['expect-format'] !== undefined && summary.format !== values['expect-format']) {
  summary.failures.push(`expected format ${values['expect-format']}, saw ${summary.format ?? 'no trace'}`);
}
if (values['expect-error'] !== undefined && !summary.error?.includes(values['expect-error'])) {
  summary.failures.push(`expected an error containing "${values['expect-error']}", saw ${summary.error ?? 'none'}`);
}
if (values['expect-error'] !== undefined && summary.spans !== undefined) summary.failures.push(`expected no trace with the error, saw ${summary.spans} spans`);
if (values['expect-error'] === undefined && summary.error) summary.failures.push(`unexpected error: ${summary.error}`);

if (values.tabs || values['expect-tabs'] !== undefined) {
  summary.tabs = [];
  const tabs = page.locator('.view-tabs [role="tab"]');
  const names = (await tabs.locator('.vt-label').allTextContents()).map((name) => name.trim());
  if (names.length === 0) summary.failures.push('no view tabs found');
  if (values['expect-tabs'] !== undefined && names.join(',') !== values['expect-tabs']) {
    summary.failures.push(`expected tabs ${values['expect-tabs']}, saw ${names.join(',')}`);
  }
  for (let i = 0; i < (await tabs.count()); i++) {
    const tab = tabs.nth(i);
    const name = names[i];
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
for (const error of consoleErrors) summary.failures.push(`browser error: ${error}`);
summary.ok = summary.failures.length === 0;
writeFileSync(`${outDir}/${run}-drive.json`, `${JSON.stringify(summary, null, 2)}\n`);
console.log(JSON.stringify(summary, null, 2));
await browser.close();
process.exit(summary.ok ? 0 : 1);
