import { writable, get } from 'svelte/store';
import { traceState } from './trace';
import { handleRawInput } from '../lib/input';

export interface TraceEntry {
  name: string;
  json: string;
}

function createTraceListStore() {
  const { subscribe, set, update } = writable<TraceEntry[]>([]);

  return {
    subscribe,

    add(name: string, json: string) {
      update((list) => {
        const existing = list.findIndex((e) => e.json === json);
        if (existing >= 0) return list;
        return [...list, { name, json }];
      });
    },

    addMultiple(entries: { name: string; json: string }[]) {
      update((list) => {
        const next = [...list];
        for (const e of entries) {
          if (!next.some((x) => x.json === e.json)) {
            next.push(e);
          }
        }
        return next;
      });
    },

    switchTo(index: number) {
      const list = get({ subscribe });
      if (index >= 0 && index < list.length) {
        const entry = list[index];
        handleRawInput(entry.json, false, true);
      }
    },

    clear() {
      set([]);
    },

    count(): number {
      return get({ subscribe }).length;
    },
  };
}

export const traceList = createTraceListStore();
