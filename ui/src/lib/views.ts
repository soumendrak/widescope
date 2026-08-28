import type { ViewName } from './types';

export interface ViewTab {
  id: ViewName;
  label: string;
  glyph: string;
}

/**
 * The five lenses, left to right. This list is the single source of truth for
 * the tab bar, the 1-5 shortcuts, and the slide direction between views — they
 * drifted apart once before (tabs read Flame-first while `1` selected the
 * waterfall) and that is what this module exists to prevent.
 *
 * Slot 3 swaps: a trace carrying LLM spans gets the Conversation transcript,
 * anything else keeps the Timeline. Five tabs either way — the plan's
 * guardrails forbid a sixth view.
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

/** Just the ids, for shortcut indexing and slide-direction maths. */
export function viewOrder(hasLlmSpans: boolean): ViewName[] {
  return viewTabs(hasLlmSpans).map((t) => t.id);
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
  if (view === 'conversation' || view === 'timeline') return order[2];
  return order[0];
}

/** Every view id that exists, regardless of which occupies slot 3 right now. */
const ALL_VIEWS: ReadonlyArray<ViewName> = [
  'waterfall', 'flame', 'timeline', 'conversation', 'graph', 'diff',
];

/** Narrow an untrusted string (localStorage, share link) to a ViewName. */
export function isViewName(value: string | null): value is ViewName {
  return value !== null && ALL_VIEWS.includes(value as ViewName);
}
