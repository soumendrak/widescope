import { refreshCriticalPath } from '../stores/criticalPath';
import { traceState } from '../stores/trace';
import { focusedSpanId, hoveredSpanId, searchQuery, searchResults, selectedSpanId } from '../stores/selection';
import { traceList } from '../stores/traceList';
import { saveRecent } from './recent';
import { parseTrace, getFlameGraphLayout, getTimelineLayout, getWaterfallLayout, getServiceGraph, getAgentFlow, safeParseWasmError } from './wasm';

const MAX_FILE_SIZE = 20 * 1024 * 1024; // 20 MB
const LARGE_TRACE_BYTES = 5 * 1024 * 1024;

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

async function inflateRaw(data: Uint8Array): Promise<Uint8Array> {
  // Node's typed-array generics don't line up with DOM BlobPart once
  // `types: ["node"]` is on; this is a plain Uint8Array at runtime.
  const stream = new Response(
    new Blob([data as unknown as BlobPart]).stream().pipeThrough(new DecompressionStream('deflate-raw')),
  );
  return new Uint8Array(await stream.arrayBuffer());
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

    if (name.endsWith('.json')) {
      const raw = new Uint8Array(buffer, dataStart, compLen);
      // 0 = stored, 8 = deflate (the two methods real zip tools produce).
      if (compMethod === 0) {
        entries.push(decoder.decode(raw));
      } else if (compMethod === 8) {
        try {
          entries.push(decoder.decode(await inflateRaw(raw)));
        } catch {
          // skip entries we can't inflate (unsupported/corrupt)
        }
      }
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

  // Add each entry to the trace list, return the first one
  for (let i = 0; i < entries.length; i++) {
    try {
      const summary = parseTrace(entries[i]);
      const name = summary.root_operation ?? summary.root_service ?? summary.trace_id;
      traceList.add(name, entries[i]);
    } catch {
      // skip invalid entries
    }
  }

  return entries[0];
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

export function handleRawInput(text: string, isSample: boolean, showLoading = true, persist = false): boolean {
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
    const agentFlow = getAgentFlow();
    traceState.setLoaded(summary, flameLayout, timelineLayout, waterfallLayout, serviceGraph, agentFlow, isSample);
    refreshCriticalPath();

    // Add to trace list for multi-trace switching
    const name = summary.root_operation ?? summary.root_service ?? summary.trace_id;
    traceList.add(name, text);
    if (persist) void saveRecent(name, text);

    return true;
  } catch (err) {
    const wasmError = safeParseWasmError(err);
    traceState.setError(wasmError);
    return false;
  }
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

export async function handleRawInputAsync(text: string, isSample: boolean, showLoading = true, persist = false): Promise<boolean> {
  if (showLoading) {
    const sizeMb = text.length / 1024 / 1024;
    const phase = text.length >= LARGE_TRACE_BYTES
      ? `Preparing ${sizeMb.toFixed(1)} MB trace`
      : 'Preparing trace';
    traceState.setLoading(phase, 5);
    await nextFrame();
  }

  selectedSpanId.set(null);
  hoveredSpanId.set(null);
  focusedSpanId.set(null);
  searchQuery.set('');
  searchResults.set([]);

  try {
    if (showLoading) {
      traceState.setLoading('Parsing trace JSON in WASM', 20);
      await nextFrame();
    }
    const summary = parseTrace(text);

    if (showLoading) {
      traceState.setLoading('Computing flame graph layout', 45);
      await nextFrame();
    }
    const flameLayout = getFlameGraphLayout();

    if (showLoading) {
      traceState.setLoading('Computing timeline layout', 65);
      await nextFrame();
    }
    const timelineLayout = getTimelineLayout();

    if (showLoading) {
      traceState.setLoading('Computing waterfall and service graph', 82);
      await nextFrame();
    }
    const waterfallLayout = getWaterfallLayout();
    const serviceGraph = getServiceGraph();
    const agentFlow = getAgentFlow();

    traceState.setLoaded(summary, flameLayout, timelineLayout, waterfallLayout, serviceGraph, agentFlow, isSample);
    refreshCriticalPath();

    const name = summary.root_operation ?? summary.root_service ?? summary.trace_id;
    traceList.add(name, text);
    if (persist) void saveRecent(name, text);

    return true;
  } catch (err) {
    const wasmError = safeParseWasmError(err);
    traceState.setError(wasmError);
    return false;
  }
}
