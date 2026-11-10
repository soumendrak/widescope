export const MAX_FILE_SIZE = 20 * 1024 * 1024;

export const HTTPS_TRACE_URL_MESSAGE = 'Only HTTPS trace URLs are supported.';

export function fileTooLargeMessage(bytes: number): string {
  return `File too large (${(bytes / 1024 / 1024).toFixed(1)} MB). Maximum is 20 MB.`;
}

export class TraceFetchError extends Error {
  readonly code: 'TRACE_URL' | 'FILE_TOO_LARGE';

  constructor(code: 'TRACE_URL' | 'FILE_TOO_LARGE', message: string) {
    super(message);
    this.name = 'TraceFetchError';
    this.code = code;
  }
}

function isLoopbackHost(hostname: string): boolean {
  return hostname === 'localhost' || hostname === '127.0.0.1' || hostname === '[::1]';
}

export function traceUrlSchemeError(raw: string): TraceFetchError | null {
  let url: URL;
  try {
    url = new URL(raw);
  } catch {
    return new TraceFetchError('TRACE_URL', HTTPS_TRACE_URL_MESSAGE);
  }
  if (url.protocol === 'https:') return null;
  if (url.protocol === 'http:' && isLoopbackHost(url.hostname)) return null;
  return new TraceFetchError('TRACE_URL', HTTPS_TRACE_URL_MESSAGE);
}

function declaredLength(header: string | null): number | null {
  if (header === null || !/^\d+$/.test(header.trim())) return null;
  return Number(header);
}

async function readStream(body: ReadableStream<Uint8Array>, maxBytes: number): Promise<string> {
  const reader = body.getReader();
  const chunks: Uint8Array[] = [];
  let received = 0;
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    received += value.byteLength;
    if (received > maxBytes) {
      await reader.cancel();
      throw new TraceFetchError('FILE_TOO_LARGE', fileTooLargeMessage(received));
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(received);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

function boundedText(text: string, maxBytes: number): string {
  const bytes = new TextEncoder().encode(text).byteLength;
  if (bytes > maxBytes) throw new TraceFetchError('FILE_TOO_LARGE', fileTooLargeMessage(bytes));
  return text;
}

export async function readBoundedResponse(response: Response, maxBytes = MAX_FILE_SIZE): Promise<string> {
  const declared = declaredLength(response.headers.get('content-length'));
  if (declared !== null && declared > maxBytes) {
    await response.body?.cancel();
    throw new TraceFetchError('FILE_TOO_LARGE', fileTooLargeMessage(declared));
  }
  if (response.body) return readStream(response.body, maxBytes);
  return boundedText(await response.text(), maxBytes);
}

export async function fetchTraceText(url: string, fetchImpl: typeof fetch = fetch): Promise<string> {
  const schemeError = traceUrlSchemeError(url);
  if (schemeError) throw schemeError;
  const response = await fetchImpl(url);
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return readBoundedResponse(response);
}
