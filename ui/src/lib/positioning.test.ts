import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const HERO =
  'Drop an AI-agent trace (OTLP, OpenInference or Jaeger JSON, or a zip of them) into your browser and debug the run — nothing uploaded, nothing installed, shareable as a link.';

const LEGACY_PHRASES = [/browser-native/i, /zero-backend/i, /powered by rust\/wasm/i];

const REPO_ROOT = resolve(__dirname, '../../..');

function read(path: string): string {
  return readFileSync(resolve(REPO_ROOT, path), 'utf8');
}

const SURFACES_WITH_HERO = [
  'README.md',
  'ui/index.html',
  'ui/editor/index.html',
  'ui/public/llms.txt',
  'ui/public/llms-full.txt',
  'ui/vite.config.ts',
];

const SURFACES_WITHOUT_LEGACY_PHRASES = [
  ...SURFACES_WITH_HERO,
  'CONTRIBUTING.md',
  'docs/assets/widescope-banner.svg',
];

describe('positioning copy', () => {
  it.each(SURFACES_WITH_HERO)('%s states the job-first hero line', (path) => {
    expect(read(path).includes(HERO)).toBe(true);
  });

  it.each(SURFACES_WITHOUT_LEGACY_PHRASES)('%s has no tech-stack-first phrasing', (path) => {
    const text = read(path);
    expect(LEGACY_PHRASES.filter((phrase) => phrase.test(text)).map(String)).toEqual([]);
  });
});
