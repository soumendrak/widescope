import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';

vi.mock('./wasm', () => ({
  parseTrace: vi.fn(() => ({ span_count: 7, trace_id: 't', root_operation: 'POST /api/chat' })),
  getFlameGraphLayout: vi.fn(() => ({ nodes: [], max_depth: 0 })),
  getTimelineLayout: vi.fn(() => ({ blocks: [], rows: [] })),
  getWaterfallLayout: vi.fn(() => ({ rows: [] })),
  getServiceGraph: vi.fn(() => ({ nodes: [], edges: [] })),
  getAgentFlow: vi.fn(() => ({ nodes: [], edges: [] })),
  safeParseWasmError: vi.fn((e: unknown) => ({
    error_type: 'WideError',
    code: 'INVALID_JSON',
    message: String(e),
    context: null,
  })),
}));
vi.mock('./recent', () => ({ saveRecent: vi.fn(async () => {}) }));

import { handleRawInput, handleRawInputAsync, readFileText, openFilePicker } from './input';
import { parseTrace } from './wasm';
import { saveRecent } from './recent';
import { traceState } from '../stores/trace';
import { traceList } from '../stores/traceList';
import { selectedSpanId } from '../stores/selection';

describe('handleRawInput', () => {
  beforeEach(() => {
    traceState.reset();
    traceList.clear();
    selectedSpanId.set(null);
    vi.mocked(parseTrace).mockClear();
    vi.mocked(saveRecent).mockClear();
    vi.mocked(parseTrace).mockImplementation(
      () => ({ span_count: 7, trace_id: 't', root_operation: 'POST /api/chat' }) as never,
    );
  });

  it('parses a payload and publishes the loaded state', () => {
    expect(handleRawInput('{"resourceSpans":[]}', false)).toBe(true);
    const state = get(traceState);
    expect(state.status).toBe('loaded');
    expect(state.summary?.span_count).toBe(7);
    expect(state.isSampleTrace).toBe(false);
  });

  it('remembers the trace in the switcher under its root operation', () => {
    handleRawInput('{"resourceSpans":[]}', false);
    expect(get(traceList)[0]?.name).toBe('POST /api/chat');
  });

  it('persists to recents only when asked', () => {
    handleRawInput('{"a":1}', false, true, false);
    expect(saveRecent).not.toHaveBeenCalled();
    handleRawInput('{"b":2}', false, true, true);
    expect(saveRecent).toHaveBeenCalled();
  });

  it('marks the bundled sample so the banner can say so', () => {
    handleRawInput('{"resourceSpans":[]}', true);
    expect(get(traceState).isSampleTrace).toBe(true);
  });

  it('reports a parse failure as an error state rather than throwing', () => {
    vi.mocked(parseTrace).mockImplementation(() => {
      throw new Error('bad json');
    });
    expect(handleRawInput('nonsense', false)).toBe(false);
    const state = get(traceState);
    expect(state.status).toBe('error');
    expect(state.error?.message).toContain('bad json');
  });

  it('clears the previous selection on every load', () => {
    selectedSpanId.set('old-span');
    handleRawInput('{"resourceSpans":[]}', false);
    expect(get(selectedSpanId)).toBeNull();
  });

  it('has an async form with the same contract', async () => {
    await expect(handleRawInputAsync('{"resourceSpans":[]}', false)).resolves.toBe(true);
    expect(get(traceState).status).toBe('loaded');
  });
});

describe('readFileText', () => {
  beforeEach(() => traceState.reset());

  it('reads a plain JSON file', async () => {
    const file = new File(['{"resourceSpans":[]}'], 'trace.json', { type: 'application/json' });
    await expect(readFileText(file)).resolves.toBe('{"resourceSpans":[]}');
  });

  it('refuses a file over the size cap and says why', async () => {
    const big = new File(['x'], 'huge.json');
    Object.defineProperty(big, 'size', { value: 21 * 1024 * 1024 });
    await expect(readFileText(big)).resolves.toBeNull();
    const error = get(traceState).error;
    expect(error?.code).toBe('FILE_TOO_LARGE');
    expect(error?.message).toContain('20 MB');
  });
});

describe('openFilePicker', () => {
  it('opens a picker that accepts json and zip', () => {
    const click = vi.spyOn(HTMLInputElement.prototype, 'click').mockImplementation(() => {});
    const created: HTMLInputElement[] = [];
    const original = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      const el = original(tag);
      if (tag === 'input') created.push(el as HTMLInputElement);
      return el;
    });

    openFilePicker();
    expect(created[0].type).toBe('file');
    expect(created[0].accept).toBe('.json,.zip');
    expect(click).toHaveBeenCalled();
  });
});
