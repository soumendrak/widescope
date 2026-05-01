import { traceState } from '../stores/trace';
import { focusedSpanId, hoveredSpanId, searchQuery, searchResults, selectedSpanId } from '../stores/selection';
import { parseTrace, getFlameGraphLayout, getTimelineLayout, getWaterfallLayout, getServiceGraph, safeParseWasmError } from './wasm';

const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20 MB

export function openFilePicker(onText?: (text: string) => void): void {
  const input = document.createElement('input');
  input.type = 'file';
  input.accept = '.json,.zip';
  input.onchange = async (e) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (file) await handleFile(file, onText);
  };
  input.click();
}

export async function readFileText(file: File): Promise<string | null> {
  if (file.size > MAX_FILE_SIZE) {
    traceState.setError({
      error_type: 'WideError',
      code: 'FILE_TOO_LARGE',
      message: `File too large (${(file.size / 1024 / 1024).toFixed(1)} MB). Maximum is 20 MB.`,
      context: null,
    });
    return null;
  }

  if (file.name.endsWith('.zip')) {
    return await readZipFile(file);
  }

  return await file.text();
}

async function readZipFile(file: File): Promise<string | null> {
  // Minimal ZIP parser for trace files
  const buffer = await file.arrayBuffer();
  const view = new DataView(buffer);
  const decoder = new TextDecoder();

  let offset = 0;
  const entries: string[] = [];

  while (offset < buffer.byteLength - 4) {
    const signature = view.getUint32(offset, true);
    if (signature !== 0x04034b50) break;

    const fileNameLen = view.getUint16(offset + 26, true);
    const extraLen = view.getUint16(offset + 28, true);
    const compLen = view.getUint32(offset + 18, true);
    const compMethod = view.getUint16(offset + 8, true);

    const nameStart = offset + 30;
    const name = decoder.decode(new Uint8Array(buffer, nameStart, fileNameLen));

    const dataStart = nameStart + fileNameLen + extraLen;

    if (compMethod === 0 && name.endsWith('.json')) {
      const content = decoder.decode(new Uint8Array(buffer, dataStart, compLen));
      entries.push(content);
    }

    offset = dataStart + compLen;
  }

  if (entries.length === 0) {
    traceState.setError({
      error_type: 'WideError',
      code: 'ZIP_EMPTY',
      message: 'No .json files found in the zip archive.',
      context: null,
    });
    return null;
  }

  if (entries.length === 1) {
    return entries[0];
  }

  // Merge multiple traces into one
  return entries.join('\n');
}

export async function handleFile(file: File, onText?: (text: string) => void): Promise<void> {
  const text = await readFileText(file);
  if (text === null) return;
  if (onText) {
    onText(text);
    return;
  }
  handleRawInput(text, false);
}

export function handleRawInput(text: string, isSample: boolean, showLoading = true): boolean {
  if (showLoading) {
    traceState.setLoading();
  }
  selectedSpanId.set(null);
  hoveredSpanId.set(null);
  focusedSpanId.set(null);
  searchQuery.set('');
  searchResults.set([]);

  try {
    const summary = parseTrace(text);
    const flameLayout = getFlameGraphLayout();
    const timelineLayout = getTimelineLayout();
    const waterfallLayout = getWaterfallLayout();
    const serviceGraph = getServiceGraph();
    traceState.setLoaded(summary, flameLayout, timelineLayout, waterfallLayout, serviceGraph, isSample);
    return true;
  } catch (err) {
    const wasmError = safeParseWasmError(err);
    traceState.setError(wasmError);
    return false;
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
