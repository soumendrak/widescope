import { writable } from 'svelte/store';

const STORAGE_PREFIX = 'widescope:annotation:';

function loadAll(): Record<string, string> {
  try {
    const result: Record<string, string> = {};
    for (let i = 0; i < localStorage.length; i++) {
      const key = localStorage.key(i);
      if (key?.startsWith(STORAGE_PREFIX)) {
        result[key.slice(STORAGE_PREFIX.length)] = localStorage.getItem(key) ?? '';
      }
    }
    return result;
  } catch {
    return {};
  }
}

function createAnnotationStore() {
  const { subscribe, set, update } = writable<Record<string, string>>(loadAll());

  return {
    subscribe,

    get(spanId: string): string {
      let val = '';
      subscribe((s) => { val = s[spanId] ?? ''; })();
      return val;
    },

    setNote(spanId: string, text: string) {
      try {
        if (text.trim()) {
          localStorage.setItem(STORAGE_PREFIX + spanId, text);
        } else {
          localStorage.removeItem(STORAGE_PREFIX + spanId);
        }
      } catch { /* quota exceeded */ }
      update((s) => {
        const next = { ...s };
        if (text.trim()) {
          next[spanId] = text;
        } else {
          delete next[spanId];
        }
        return next;
      });
    },

    removeNote(spanId: string) {
      try {
        localStorage.removeItem(STORAGE_PREFIX + spanId);
      } catch { /* ignore */ }
      update((s) => {
        const next = { ...s };
        delete next[spanId];
        return next;
      });
    },
  };
}

export const annotations = createAnnotationStore();
