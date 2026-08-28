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

export type LiveStatus = 'connecting' | 'streaming' | 'disconnected';

export interface LiveState {
  /** SSE endpoint, or null when not in live mode. */
  url: string | null;
  /** Connection lifecycle: opening, open, or dropped (EventSource auto-retries). */
  status: LiveStatus;
  /** When true, incoming traces are ignored (connection stays open). */
  paused: boolean;
  /** Number of traces received this session. */
  count: number;
}

export const liveState = writable<LiveState>({ url: null, status: 'disconnected', paused: false, count: 0 });

let source: EventSource | null = null;
let paused = false;

/** Subscribe to an SSE endpoint; each event's data is one trace JSON payload. */
export function connectLive(url: string, onTrace: (json: string) => void): void {
  disconnectLive();
  paused = false;
  liveState.set({ url, status: 'connecting', paused: false, count: 0 });

  source = new EventSource(url);
  source.onopen = () => liveState.update((s) => ({ ...s, status: 'streaming' }));
  source.onerror = () => liveState.update((s) => ({ ...s, status: 'disconnected' }));
  source.onmessage = (e) => {
    if (typeof e.data !== 'string' || !e.data.trim()) return;
    liveState.update((s) => ({ ...s, count: s.count + 1 }));
    if (paused) return;
    onTrace(e.data);
  };
}

/** Pause or resume rendering of incoming traces; the SSE connection stays open. */
export function toggleLivePause(): void {
  paused = !paused;
  liveState.update((s) => ({ ...s, paused }));
}

/** Close the live connection, if any. The badge stays until reset. */
export function disconnectLive(): void {
  source?.close();
  source = null;
  paused = false;
  liveState.update((s) => ({ ...s, status: 'disconnected', paused: false }));
}
