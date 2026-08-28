import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { annotations } from './annotations';

const KEY = (id: string) => `widescope:annotation:${id}`;

describe('span notes', () => {
  beforeEach(() => {
    localStorage.clear();
    // The store was seeded at import time; clear whatever it holds.
    for (const id of Object.keys(get(annotations))) annotations.removeNote(id);
  });

  it('persists a note and reads it back', () => {
    annotations.setNote('span-1', 'look here');
    expect(annotations.get('span-1')).toBe('look here');
    expect(localStorage.getItem(KEY('span-1'))).toBe('look here');
    expect(get(annotations)['span-1']).toBe('look here');
  });

  it('treats a blank note as a deletion', () => {
    annotations.setNote('span-1', 'temporary');
    annotations.setNote('span-1', '   ');
    expect(annotations.get('span-1')).toBe('');
    expect(localStorage.getItem(KEY('span-1'))).toBeNull();
  });

  it('removes a note explicitly', () => {
    annotations.setNote('span-2', 'note');
    annotations.removeNote('span-2');
    expect(get(annotations)['span-2']).toBeUndefined();
    expect(localStorage.getItem(KEY('span-2'))).toBeNull();
  });

  it('imports many notes at once, skipping the blank ones', () => {
    annotations.setMany({ a: 'first', b: '  ', c: 'third' });
    expect(get(annotations)).toEqual({ a: 'first', c: 'third' });
    expect(localStorage.getItem(KEY('b'))).toBeNull();
  });

  it('does nothing when the import carries no usable notes', () => {
    annotations.setMany({ a: '  ' });
    expect(get(annotations)).toEqual({});
  });

  it('survives storage being unavailable', () => {
    const setItem = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('QuotaExceededError');
    });
    // The note still lands in memory even though it cannot be persisted.
    annotations.setNote('span-3', 'unsaved');
    expect(annotations.get('span-3')).toBe('unsaved');
    setItem.mockRestore();
  });

  it('returns an empty string for a span with no note', () => {
    expect(annotations.get('never-seen')).toBe('');
  });
});
