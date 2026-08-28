import type { ViewName } from './types';

export interface ViewTab {
  id: ViewName;
  label: string;
  glyph: string;
  /** Only meaningful once more than one trace is loaded. */
  multiTrace?: boolean;
}

/**
 * The five lenses, left to right. This list is the single source of truth for
 * the tab bar, the 1-5 shortcuts, and the slide direction between views — they
 * drifted apart once before (tabs read Flame-first while `1` selected the
 * waterfall) and that is what this module exists to prevent.
 *
 * Slot 3 swaps: a trace carrying LLM spans gets the Conversation transcript,
 * anything else keeps the Timeline. Five tabs either way — the plan's
 * guardrails forbid a sixth view, so everything else lives in SECONDARY_VIEWS.
 *
 * Args:
 *   hasLlmSpans: Whether the loaded trace has any LLM spans.
 *
 * Returns:
 *   Tab descriptors in display and shortcut order.
 */
export function viewTabs(hasLlmSpans: boolean): ViewTab[] {
  return [
    { id: 'waterfall', label: 'Waterfall', glyph: '≋' },
    { id: 'flame', label: 'Flame', glyph: '▲' },
    hasLlmSpans
      ? { id: 'conversation', label: 'Conversation', glyph: '◍' }
      : { id: 'timeline', label: 'Timeline', glyph: '≡' },
    { id: 'graph', label: 'Graph', glyph: '◉' },
    { id: 'diff', label: 'Diff', glyph: '⇆' },
  ];
}

/**
 * The lenses that did not make the tab bar. Reachable from the overflow menu
 * beside the tabs, from the 6-9 shortcuts, and from the command palette — a
 * long tab strip was the thing the tab bar was capped to avoid, not these
 * views themselves.
 */
export const SECONDARY_VIEWS: ViewTab[] = [
  { id: 'agent', label: 'Agent flow', glyph: '⌥' },
  { id: 'analytics', label: 'Trends', glyph: '▤' },
  { id: 'matrix', label: 'Matrix', glyph: '⊞', multiTrace: true },
  { id: 'dashboard', label: 'Dashboard', glyph: '▦', multiTrace: true },
];

/** Secondary lenses worth showing for the traces currently loaded. */
export function secondaryViews(traceCount: number): ViewTab[] {
  return SECONDARY_VIEWS.filter((v) => !v.multiTrace || traceCount > 1);
}

/** Just the ids, for shortcut indexing and slide-direction maths. */
export function viewOrder(hasLlmSpans: boolean): ViewName[] {
  return viewTabs(hasLlmSpans).map((t) => t.id);
}

/** Tabs first, then the overflow lenses — the 1-9 shortcut order. */
export function shortcutOrder(hasLlmSpans: boolean, traceCount = 1): ViewName[] {
  return [...viewOrder(hasLlmSpans), ...secondaryViews(traceCount).map((v) => v.id)];
}

/**
 * Map a view to the slot it occupies, tolerating the slot-3 swap: if a stored
 * or shared view is not available for this trace (Conversation on a non-LLM
 * trace, or vice versa), fall back to whatever holds that slot.
 *
 * Args:
 *   view: The requested view.
 *   hasLlmSpans: Whether the loaded trace has any LLM spans.
 *
 * Returns:
 *   A view that exists for this trace.
 */
export function resolveView(view: ViewName, hasLlmSpans: boolean): ViewName {
  const order = viewOrder(hasLlmSpans);
  if (order.includes(view)) return view;
  if (SECONDARY_VIEWS.some((v) => v.id === view)) return view;
  if (view === 'conversation' || view === 'timeline') return order[2];
  return order[0];
}

/** Every view id that exists, regardless of which occupies slot 3 right now. */
const ALL_VIEWS: ReadonlyArray<ViewName> = [
  'waterfall', 'flame', 'timeline', 'conversation', 'graph', 'diff',
  'agent', 'analytics', 'matrix', 'dashboard',
];

/** Narrow an untrusted string (localStorage, share link) to a ViewName. */
export function isViewName(value: string | null): value is ViewName {
  return value !== null && ALL_VIEWS.includes(value as ViewName);
}
