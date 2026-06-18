/**
 * Phase 1 live trace ingestion via Server-Sent Events.
 *
 * When the page is opened with `?live=<sse-url>`, we subscribe to that
 * endpoint with the browser's `EventSource`. Each message's `data` is one
 * trace JSON payload (any format WideScope already parses) — it's appended to
 * the trace list and auto-selected as the newest. A relay sits between the
 * collector and the browser; see issue #29 for the OTLP→SSE bridge.
 *
 * ponytail: EventSource handles reconnection/backoff natively, so there's no
 * custom retry loop here. WebSocket + direct OTLP (Phase 2) when asked.
 */
import { writable } from 'svelte/store';

export interface LiveState {
  /** SSE endpoint, or null when not in live mode. */
  url: string | null;
  /** True while the EventSource connection is open. */
  connected: boolean;
  /** Number of traces received this session. */
  count: number;
}

export const liveState = writable<LiveState>({ url: null, connected: false, count: 0 });

let source: EventSource | null = null;

/** Subscribe to an SSE endpoint; each event's data is one trace JSON payload. */
export function connectLive(url: string, onTrace: (json: string) => void): void {
  disconnectLive();
  liveState.set({ url, connected: false, count: 0 });

  source = new EventSource(url);
  source.onopen = () => liveState.update((s) => ({ ...s, connected: true }));
  source.onerror = () => liveState.update((s) => ({ ...s, connected: false }));
  source.onmessage = (e) => {
    if (typeof e.data !== 'string' || !e.data.trim()) return;
    onTrace(e.data);
    liveState.update((s) => ({ ...s, count: s.count + 1 }));
  };
}

/** Close the live connection, if any. The badge stays until reset. */
export function disconnectLive(): void {
  source?.close();
  source = null;
  liveState.update((s) => ({ ...s, connected: false }));
}
