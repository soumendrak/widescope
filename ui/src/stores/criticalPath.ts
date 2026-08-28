import { writable, derived } from 'svelte/store';
import { getCriticalPath } from '../lib/wasm';
import type { CriticalPath } from '../lib/types';

/**
 * Critical path highlighting, shared by the flame graph and the waterfall.
 *
 * On by default, following Jaeger (PR #1582): the slow chain is what someone
 * opens a trace to find, so it should not be behind a toggle they have to
 * discover first.
 */
export const showCriticalPath = writable(true);

/** The current trace's critical path, or null before one is loaded. */
export const criticalPath = writable<CriticalPath | null>(null);

/** Membership set for O(1) per-span lookups in render loops. */
export const criticalPathIds = derived(
  criticalPath,
  ($cp) => new Set($cp?.span_ids ?? [])
);

/** Recompute for the freshly parsed trace; safe to call when none is loaded. */
export function refreshCriticalPath(): void {
  try {
    criticalPath.set(getCriticalPath());
  } catch {
    criticalPath.set(null);
  }
}

export function clearCriticalPath(): void {
  criticalPath.set(null);
}
