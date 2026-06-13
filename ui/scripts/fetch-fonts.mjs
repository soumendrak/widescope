/**
 * One-shot vendoring script: downloads the Bricolage Grotesque and
 * Spline Sans Mono variable fonts (latin + latin-ext) from Google Fonts
 * and rewrites the @font-face CSS to self-hosted /fonts/ URLs.
 *
 * Run: node scripts/fetch-fonts.mjs
 */
import { writeFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const OUT_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'fonts');
mkdirSync(OUT_DIR, { recursive: true });

const UA =
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36';

const FAMILIES = [
  {
    key: 'bricolage',
    url: 'https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:opsz,wght@12..96,200..800&display=swap',
  },
  {
    key: 'spline-mono',
    url: 'https://fonts.googleapis.com/css2?family=Spline+Sans+Mono:wght@300..700&display=swap',
  },
];

const KEEP_SUBSETS = new Set(['latin', 'latin-ext']);

let outCss =
  '/* Self-hosted variable fonts (Google Fonts, SIL OFL) — latin + latin-ext subsets. */\n' +
  '/* Regenerate with: node scripts/fetch-fonts.mjs */\n';

for (const fam of FAMILIES) {
  const css = await (await fetch(fam.url, { headers: { 'User-Agent': UA } })).text();
  const blockRe = /\/\*\s*([a-z-]+)\s*\*\/\s*(@font-face\s*\{[^}]+\})/g;
  for (const match of css.matchAll(blockRe)) {
    const [, subset, rawBlock] = match;
    if (!KEEP_SUBSETS.has(subset)) continue;
    const urlMatch = rawBlock.match(/url\((https:[^)]+\.woff2)\)/);
    if (!urlMatch) continue;
    const fname = `${fam.key}-${subset}.woff2`;
    const bytes = Buffer.from(
      await (await fetch(urlMatch[1], { headers: { 'User-Agent': UA } })).arrayBuffer(),
    );
    writeFileSync(join(OUT_DIR, fname), bytes);
    console.log(`${fname}  ${(bytes.length / 1024).toFixed(0)} KB`);
    outCss += `/* ${subset} */\n${rawBlock.replace(/url\(https:[^)]+\)/, `url(/fonts/${fname})`)}\n`;
  }
}

writeFileSync(join(OUT_DIR, 'fonts.css'), outCss);
console.log(`fonts.css  ${outCss.length} bytes`);
