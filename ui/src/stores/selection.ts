import { writable } from 'svelte/store';

export const selectedSpanId = writable<string | null>(null);
export const hoveredSpanId = writable<string | null>(null);
export const focusedSpanId = writable<string | null>(null);
export const activeView = writable<'flame' | 'timeline' | 'waterfall' | 'graph' | 'diff'>('flame');
export const searchQuery = writable<string>('');
export const searchResults = writable<string[]>([]);
export const filteredSpanIds = writable<string[]>([]);
export const filterStatus = writable<string>('');
export const filterService = writable<string>('');
export const filterKind = writable<string>('');
export const filterLlmOnly = writable<boolean>(false);
