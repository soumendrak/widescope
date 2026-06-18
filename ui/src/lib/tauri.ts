// Desktop (Tauri) integration. No-op in the browser: every entry point is
// guarded by isTauri(), and we touch only the injected `window.__TAURI__`
// global (config: app.withGlobalTauri). That means no @tauri-apps/* npm
// dependency and the web/PWA build is byte-for-byte unchanged.

interface TauriGlobal {
  core: { invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> };
  event: {
    listen<T>(event: string, handler: (e: { payload: T }) => void): Promise<() => void>;
  };
}

function tauri(): TauriGlobal | null {
  if (typeof window === 'undefined') return null;
  return (window as unknown as { __TAURI__?: TauriGlobal }).__TAURI__ ?? null;
}

/** True only inside the Tauri desktop shell. */
export function isTauri(): boolean {
  return tauri() !== null;
}

/**
 * Wire desktop file-open into the existing trace pipeline. Drains any file the
 * app was launched with (double-click / "Open with"), then listens for files
 * opened while running (File → Open menu, Finder). No-op outside Tauri.
 */
export async function setupTauriFileOpen(onText: (text: string) => void): Promise<void> {
  const t = tauri();
  if (!t) return;
  try {
    const pending = await t.core.invoke<string[]>('drain_pending');
    for (const text of pending) onText(text);
    await t.event.listen<string>('open-trace', (e) => onText(e.payload));
  } catch (err) {
    console.warn('WideScope desktop bridge failed:', err);
  }
}
