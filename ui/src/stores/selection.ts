import { writable } from 'svelte/store';

export const selectedSpanId = writable<string | null>(null);
export const hoveredSpanId = writable<string | null>(null);
export const focusedSpanId = writable<string | null>(null);
export const activeView = writable<'flame' | 'timeline' | 'waterfall'>('flame');
export const searchQuery = writable<string>('');
export const searchResults = writable<string[]>([]);
