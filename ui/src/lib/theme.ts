import { writable } from 'svelte/store';

export type Theme = 'light' | 'dark';

/**
 * Resolve the startup theme.
 *
 * Dark is the product default (every observability tool is dark-first), but an
 * OS that explicitly asks for light is honoured — that setting is often an
 * accessibility choice, not a taste one. A stored choice beats both.
 *
 * Args:
 *   stored: Previously persisted choice, if any.
 *
 * Returns:
 *   The theme to start in.
 */
export function resolveTheme(stored: string | null): Theme {
  if (stored === 'dark' || stored === 'light') return stored;
  if (typeof window === 'undefined') return 'dark';
  return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
}

const STORAGE_KEY = 'widescope:theme';

/** Read the persisted choice, tolerating storage being unavailable. */
function readStored(): string | null {
  try {
    return typeof localStorage === 'undefined' ? null : localStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
}

function createThemeStore() {
  // Seeded from storage at creation, not in onMount: a component-level
  // `$: localStorage.setItem(key, $theme)` runs during init and would otherwise
  // overwrite the stored choice with the default before it could be restored.
  const { subscribe, set, update } = writable<Theme>(resolveTheme(readStored()));

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
