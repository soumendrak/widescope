import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import {
  encodeTrace,
  decodeTrace,
  parsePermalink,
  buildShareUrl,
  MAX_SHARE_DATA_CHARS,
} from './permalink';

const FIXTURES = [
  '../../../test-fixtures/otlp/sample_llm_pipeline.json',
  '../../../test-fixtures/jaeger/sample_llm_pipeline.json',
  '../../../test-fixtures/openinference/sample_llm_pipeline.json',
];

function readFixture(relative: string): string {
  return readFileSync(fileURLToPath(new URL(relative, import.meta.url)), 'utf-8');
}

describe('encodeTrace / decodeTrace', () => {
  it.each(FIXTURES)('round-trips %s without loss', async (path) => {
    const original = readFixture(path);
    const restored = await decodeTrace(await encodeTrace(original));
    expect(restored).toBe(original);
  });

  it('produces a URL-safe blob (base64url alphabet only)', async () => {
    const encoded = await encodeTrace(readFixture(FIXTURES[0]));
    expect(encoded).toMatch(/^[A-Za-z0-9_-]+$/);
  });

  it('rejects a corrupt blob', async () => {
    await expect(decodeTrace('not-valid-gzip-data')).rejects.toThrow();
  });
});

describe('parsePermalink', () => {
  it('reads self-contained data and state from the hash', () => {
    const state = parsePermalink(
      'https://widescope.test/#trace=H4sIABC&view=timeline&span=s-42',
    );
    expect(state.traceData).toBe('H4sIABC');
    expect(state.traceUrl).toBeNull();
    expect(state.view).toBe('timeline');
    expect(state.spanId).toBe('s-42');
  });

  it('reads a remote URL from the query string', () => {
    const remote = 'https://example.com/trace.json';
    const state = parsePermalink(
      `https://widescope.test/?trace=${encodeURIComponent(remote)}&view=flame`,
    );
    expect(state.traceUrl).toBe(remote);
    expect(state.traceData).toBeNull();
    expect(state.view).toBe('flame');
  });

  it('ignores an unknown view value', () => {
    const state = parsePermalink('https://widescope.test/#view=hologram');
    expect(state.view).toBeNull();
  });

  it('returns all-null state for a bare URL', () => {
    const state = parsePermalink('https://widescope.test/');
    expect(state).toEqual({
      traceData: null,
      traceUrl: null,
      view: null,
      spanId: null,
    });
  });
});

describe('buildShareUrl', () => {
  const baseUrl = 'https://widescope.test/';

  it('builds a hash link that parses back to the same trace', async () => {
    const json = readFixture(FIXTURES[0]);
    const result = await buildShareUrl({
      json,
      view: 'waterfall',
      spanId: 'span-1',
      baseUrl,
    });

    expect(result.tooLarge).toBe(false);
    expect(result.url.startsWith(`${baseUrl}#`)).toBe(true);

    const parsed = parsePermalink(result.url);
    expect(parsed.view).toBe('waterfall');
    expect(parsed.spanId).toBe('span-1');
    expect(parsed.traceData).not.toBeNull();
    expect(await decodeTrace(parsed.traceData as string)).toBe(json);
  });

  it('omits the span param when no span is selected', async () => {
    const result = await buildShareUrl({
      json: readFixture(FIXTURES[0]),
      view: 'flame',
      spanId: null,
      baseUrl,
    });
    expect(parsePermalink(result.url).spanId).toBeNull();
  });

  it('flags a trace too large to embed', async () => {
    let blob = '';
    while (blob.length < 200_000) {
      blob += Math.random().toString(36).slice(2);
    }
    const result = await buildShareUrl({
      json: JSON.stringify({ blob }),
      view: 'flame',
      spanId: null,
      baseUrl,
    });
    expect(result.dataChars).toBeGreaterThan(MAX_SHARE_DATA_CHARS);
    expect(result.tooLarge).toBe(true);
  });
});
