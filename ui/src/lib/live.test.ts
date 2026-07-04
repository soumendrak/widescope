import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { connectLive, disconnectLive, toggleLivePause, liveState } from './live';

// Minimal EventSource stub — node env has none. Captures the last instance so
// tests can drive onopen/onmessage manually.
class FakeEventSource {
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((e: { data: string }) => void) | null = null;
  static last: FakeEventSource;
  constructor(public url: string) {
    FakeEventSource.last = this;
  }
  close() {}
}

beforeEach(() => {
  vi.stubGlobal('EventSource', FakeEventSource as unknown);
  disconnectLive();
});

describe('live pause/resume', () => {
  it('skips onTrace while paused but still counts, and resumes', () => {
    const seen: string[] = [];
    connectLive('http://relay', (json) => seen.push(json));
    const es = FakeEventSource.last;

    expect(get(liveState).status).toBe('connecting');
    es.onopen?.();
    expect(get(liveState).status).toBe('streaming');

    es.onmessage?.({ data: '{"a":1}' });
    expect(seen).toEqual(['{"a":1}']);

    toggleLivePause();
    es.onmessage?.({ data: '{"b":2}' }); // paused → not delivered
    expect(seen).toEqual(['{"a":1}']);
    expect(get(liveState).count).toBe(2); // but still counted as received

    toggleLivePause();
    es.onmessage?.({ data: '{"c":3}' }); // resumed → delivered
    expect(seen).toEqual(['{"a":1}', '{"c":3}']);
  });
});
