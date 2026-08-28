import { describe, expect, it } from 'vitest';
import {
  SECONDARY_VIEWS,
  isViewName,
  resolveView,
  secondaryViews,
  shortcutOrder,
  viewOrder,
  viewTabs,
} from './views';

describe('view tabs', () => {
  it('keeps five lenses whether or not the trace has LLM spans', () => {
    expect(viewTabs(true)).toHaveLength(5);
    expect(viewTabs(false)).toHaveLength(5);
  });

  it('swaps slot 3 between Conversation and Timeline', () => {
    expect(viewTabs(true)[2].id).toBe('conversation');
    expect(viewTabs(false)[2].id).toBe('timeline');
  });

  it('orders the tabs so the 1-5 shortcuts match what is on screen', () => {
    expect(viewOrder(true)).toEqual(['waterfall', 'flame', 'conversation', 'graph', 'diff']);
    expect(viewOrder(false)).toEqual(['waterfall', 'flame', 'timeline', 'graph', 'diff']);
  });

  it('gives every tab a glyph and a label', () => {
    for (const tab of viewTabs(true)) {
      expect(tab.label.length).toBeGreaterThan(0);
      expect(tab.glyph.length).toBeGreaterThan(0);
    }
  });
});

describe('secondary lenses', () => {
  it('hides the multi-trace lenses until a second trace is loaded', () => {
    expect(secondaryViews(1).map((v) => v.id)).toEqual(['agent', 'analytics']);
    expect(secondaryViews(2).map((v) => v.id)).toEqual([
      'agent',
      'analytics',
      'matrix',
      'dashboard',
    ]);
  });

  it('continues the shortcut numbering after the tabs', () => {
    const order = shortcutOrder(true, 2);
    expect(order.slice(0, 5)).toEqual(viewOrder(true));
    expect(order[5]).toBe('agent');
    expect(order).toHaveLength(9);
    expect(shortcutOrder(true, 1)).toHaveLength(7);
  });

  it('marks exactly the lenses that need more than one trace', () => {
    const multi = SECONDARY_VIEWS.filter((v) => v.multiTrace).map((v) => v.id);
    expect(multi).toEqual(['matrix', 'dashboard']);
  });
});

describe('resolveView', () => {
  it('passes through a view the trace actually has', () => {
    expect(resolveView('graph', true)).toBe('graph');
    expect(resolveView('agent', false)).toBe('agent');
  });

  it('falls back to whatever holds slot 3 when the stored view does not fit', () => {
    // A conversation link opened on a non-LLM trace, and the reverse.
    expect(resolveView('conversation', false)).toBe('timeline');
    expect(resolveView('timeline', true)).toBe('conversation');
  });

  it('falls back to the first lens for anything else unknown', () => {
    expect(resolveView('nonsense' as never, true)).toBe('waterfall');
  });
});

describe('isViewName', () => {
  it('accepts every real view and rejects untrusted input', () => {
    for (const id of [...viewOrder(true), ...viewOrder(false), ...SECONDARY_VIEWS.map((v) => v.id)]) {
      expect(isViewName(id)).toBe(true);
    }
    expect(isViewName('dashboard')).toBe(true);
    expect(isViewName('drop table')).toBe(false);
    expect(isViewName(null)).toBe(false);
    expect(isViewName('')).toBe(false);
  });
});
