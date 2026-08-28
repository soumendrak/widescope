/**
 * Global keyboard router.
 *
 * Every app-level shortcut is declared here as data, so the bindings can be
 * read (and audited against the help dialog) in one place instead of being
 * buried in a 50-line if-chain inside the root component.
 */

export interface KeyboardActions {
  toggleHelp: () => void;
  closeHelp: () => void;
  /** True while the help dialog owns the keyboard. */
  isHelpOpen: () => boolean;
  openFile: () => void;
  focusSearch: () => void;
  submitEditor: () => void;
  pasteFromClipboard: () => void;
  /** 0-based index into the view order. */
  selectView: (index: number) => void;
  viewCount: number;
  toggleFullscreen: () => void;
  exitFullscreen: () => void;
  isFullscreen: () => boolean;
}

/**
 * Typing must never trigger a shortcut. Covers the editor textarea, the search
 * box, every `<select>`, and any contenteditable surface.
 */
function isEditingTarget(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    (target instanceof HTMLElement && target.isContentEditable)
  );
}

/**
 * Attach the router to `document`.
 *
 * Args:
 *   actions: Callbacks the shortcuts drive, supplied by the root component.
 *
 * Returns:
 *   A teardown function that removes the listener.
 */
export function installKeyboardRouter(actions: KeyboardActions): () => void {
  const handler = (e: KeyboardEvent): void => {
    if (isEditingTarget(e.target)) return;

    if (e.key === '?' && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      actions.toggleHelp();
      return;
    }

    // While help is open it swallows everything except Escape.
    if (actions.isHelpOpen()) {
      if (e.key === 'Escape') {
        e.preventDefault();
        actions.closeHelp();
      }
      return;
    }

    const mod = e.metaKey || e.ctrlKey;

    if (mod && e.key === 'o') { e.preventDefault(); actions.openFile(); return; }
    if (mod && e.key === 'k') { e.preventDefault(); actions.focusSearch(); return; }
    if (mod && e.key === 'Enter') { e.preventDefault(); actions.submitEditor(); return; }
    if (mod && e.key === 'v') { e.preventDefault(); actions.pasteFromClipboard(); return; }

    // Number keys follow the canonical view order, not a second hardcoded list.
    if (!mod && e.key >= '1' && e.key <= String(actions.viewCount)) {
      e.preventDefault();
      actions.selectView(parseInt(e.key, 10) - 1);
      return;
    }

    if (e.key === 'F' && e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      actions.toggleFullscreen();
      return;
    }

    if (actions.isFullscreen() && e.key === 'Escape') {
      e.preventDefault();
      actions.exitFullscreen();
    }
  };

  document.addEventListener('keydown', handler);
  return () => document.removeEventListener('keydown', handler);
}
