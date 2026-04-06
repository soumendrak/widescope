import { traceState } from '../stores/trace';
import { selectedSpanId } from '../stores/selection';
import { parseTrace, getFlameGraphLayout, getWaterfallLayout, safeParseWasmError } from './wasm';

const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20 MB

export function openFilePicker(): void {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json';
  input.onchange = (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) handleFile(file);
  };
  input.click();
}

export async function handleFile(file: File): Promise<void> {
  if (file.size > MAX_FILE_SIZE) {
    traceState.setError({
      error_type: 'WideError',
      code: 'FILE_TOO_LARGE',
      message: `File too large (${(file.size / 1024 / 1024).toFixed(1)} MB). Maximum is 20 MB.`,
      context: null,
    });
    return;
  }
  const text = await file.text();
  handleRawInput(text, false);
}

export function handleRawInput(text: string, isSample: boolean): void {
  traceState.setLoading();
  selectedSpanId.set(null);

  try {
    const summary = parseTrace(text);
    const flameLayout = getFlameGraphLayout();
    const waterfallLayout = getWaterfallLayout();
    traceState.setLoaded(summary, flameLayout, null, waterfallLayout, isSample);
  } catch (err) {
    const wasmError = safeParseWasmError(err);
    traceState.setError(wasmError);
  }
}

export function setupGlobalPasteListener(
  onPaste: (text: string) => void
): () => void {
  const handler = (e: KeyboardEvent) => {
    const active = document.activeElement;
    const isInput =
      active instanceof HTMLInputElement ||
      active instanceof HTMLTextAreaElement;
    if ((e.ctrlKey || e.metaKey) && e.key === 'v' && !isInput) {
      navigator.clipboard.readText().then((text) => {
        if (text.trim()) onPaste(text);
      }).catch(() => {/* clipboard read rejected */});
    }
  };
  document.addEventListener('keydown', handler);
  return () => document.removeEventListener('keydown', handler);
}
