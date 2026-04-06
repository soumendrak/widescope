import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

function createThemeStore() {
  const prefersDark =
    typeof window !== 'undefined' &&
    window.matchMedia('(prefers-color-scheme: dark)').matches;

  const { subscribe, set, update } = writable<Theme>(prefersDark ? 'dark' : 'light');

  return {
    subscribe,
    toggle() {
      update((t) => {
        const next = t === 'light' ? 'dark' : 'light';
        if (typeof document !== 'undefined') {
          document.documentElement.setAttribute('data-theme', next);
        }
        return next;
      });
    },
    apply(theme: Theme) {
      if (typeof document !== 'undefined') {
        document.documentElement.setAttribute('data-theme', theme);
      }
      set(theme);
    },
  };
}

export const theme = createThemeStore();
