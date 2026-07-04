// Persist recently-loaded traces to IndexedDB so returning users can reload
// them without re-importing. Traces run multi-MB, so localStorage is too small
// — IndexedDB is the native store for this. Privacy-first: everything here is
// user-clearable via clearRecent().
//
// Metadata and JSON live in separate object stores: the multi-MB `json` blob is
// only ever read when a user reloads a specific trace, so listing/dedup never
// deserializes it. `hash` lets dedup compare cheaply without the payload.
import { writable } from 'svelte/store';

const DB_NAME = 'widescope';
const META = 'recent-meta';
const BLOB = 'recent-blob';
const MAX_RECENT = 5;

export interface RecentMeta {
  id: number;
  name: string;
  savedAt: number;
  size: number;
  hash: number;
}

/** Reactive metadata list (newest first). JSON stays in IDB, fetched on demand. */
export const recentTraces = writable<RecentMeta[]>([]);

let dbPromise: Promise<IDBDatabase> | null = null;

function db(): Promise<IDBDatabase> {
  if (!dbPromise) {
    dbPromise = new Promise((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, 2);
      req.onupgradeneeded = () => {
        const d = req.result;
        if (!d.objectStoreNames.contains(META)) d.createObjectStore(META, { keyPath: 'id', autoIncrement: true });
        if (!d.objectStoreNames.contains(BLOB)) d.createObjectStore(BLOB, { keyPath: 'id' });
        // Drop the v1 single-store layout; the few dev-build entries are disposable.
        if (d.objectStoreNames.contains('recent-traces')) d.deleteObjectStore('recent-traces');
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }
  return dbPromise;
}

function reqDone<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

function txDone(t: IDBTransaction): Promise<void> {
  return new Promise((resolve, reject) => {
    t.oncomplete = () => resolve();
    t.onerror = () => reject(t.error);
    t.onabort = () => reject(t.error);
  });
}

// djb2 — cheap, collision-tolerant. A rare collision just skips a dedup, harmless.
function hashOf(s: string): number {
  let h = 5381;
  for (let i = 0; i < s.length; i++) h = ((h << 5) + h + s.charCodeAt(i)) | 0;
  return h;
}

async function allMeta(): Promise<RecentMeta[]> {
  const conn = await db();
  return reqDone(conn.transaction(META, 'readonly').objectStore(META).getAll());
}

async function refresh(): Promise<void> {
  const all = await allMeta();
  all.sort((a, b) => b.savedAt - a.savedAt);
  recentTraces.set(all);
}

/** Populate the store from IndexedDB. Call once on startup. */
export async function loadRecent(): Promise<void> {
  try {
    await refresh();
  } catch { /* IndexedDB unavailable (private mode) — list stays empty */ }
}

/** Save a loaded trace, dedup by content hash, keep only the newest MAX_RECENT. */
export async function saveRecent(name: string, json: string): Promise<void> {
  try {
    const conn = await db();
    const hash = hashOf(json);
    const meta = await allMeta();
    const dupes = meta.filter((m) => m.hash === hash).map((m) => m.id);
    const kept = meta.filter((m) => m.hash !== hash).sort((a, b) => b.savedAt - a.savedAt);
    const evict = kept.slice(MAX_RECENT - 1).map((m) => m.id);

    const t = conn.transaction([META, BLOB], 'readwrite');
    const metaStore = t.objectStore(META);
    const blobStore = t.objectStore(BLOB);
    for (const id of [...dupes, ...evict]) {
      metaStore.delete(id);
      blobStore.delete(id);
    }
    const id = await reqDone<IDBValidKey>(
      metaStore.add({ name, savedAt: Date.now(), size: json.length, hash }),
    );
    blobStore.add({ id, json });
    await txDone(t);
    await refresh();
  } catch { /* IndexedDB unavailable — persistence is best-effort */ }
}

/** Fetch the full JSON for a recent entry, or null if gone. */
export async function getRecentJson(id: number): Promise<string | null> {
  try {
    const conn = await db();
    const rec = await reqDone<{ json: string } | undefined>(
      conn.transaction(BLOB, 'readonly').objectStore(BLOB).get(id),
    );
    return rec?.json ?? null;
  } catch {
    return null;
  }
}

/** Forget every recent trace. */
export async function clearRecent(): Promise<void> {
  try {
    const conn = await db();
    const t = conn.transaction([META, BLOB], 'readwrite');
    t.objectStore(META).clear();
    t.objectStore(BLOB).clear();
    await txDone(t);
    recentTraces.set([]);
  } catch { /* ignore */ }
}
