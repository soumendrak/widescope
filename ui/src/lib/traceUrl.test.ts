import { describe, expect, it, vi } from 'vitest';
import {
  HTTPS_TRACE_URL_MESSAGE,
  MAX_FILE_SIZE,
  TraceFetchError,
  fetchTraceText,
  fileTooLargeMessage,
  readBoundedResponse,
  traceUrlSchemeError,
} from './traceUrl';

const encoder = new TextEncoder();

function scheme(raw: string): string | null {
  return traceUrlSchemeError(raw)?.message ?? null;
}

describe('trace URL scheme', () => {
  it('accepts https URLs', () => {
    expect(scheme('https://example.com/trace.json')).toBeNull();
    expect(scheme('HTTPS://cdn.example.com/dir/trace.json?x=1')).toBeNull();
  });

  it('rejects non-local http URLs', () => {
    expect(scheme('http://example.com/trace.json')).toBe(HTTPS_TRACE_URL_MESSAGE);
    expect(scheme('http://localhost.example.com/trace.json')).toBe(HTTPS_TRACE_URL_MESSAGE);
  });

  it('accepts http loopback URLs for local dev', () => {
    expect(scheme('http://localhost:5173/trace.json')).toBeNull();
    expect(scheme('http://127.0.0.1:8787/trace.json')).toBeNull();
    expect(scheme('http://[::1]/trace.json')).toBeNull();
  });

  it('rejects relative paths', () => {
    expect(scheme('/trace.json')).toBe(HTTPS_TRACE_URL_MESSAGE);
    expect(scheme('trace.json')).toBe(HTTPS_TRACE_URL_MESSAGE);
    expect(scheme('../secret.json')).toBe(HTTPS_TRACE_URL_MESSAGE);
  });

  it('rejects data URLs', () => {
    expect(scheme('data:application/json,{}')).toBe(HTTPS_TRACE_URL_MESSAGE);
    expect(scheme('data:text/plain,hello')).toBe(HTTPS_TRACE_URL_MESSAGE);
  });

  it('does not fetch a rejected URL', async () => {
    const fetchImpl = vi.fn();
    await expect(fetchTraceText('http://example.com/t.json', fetchImpl)).rejects.toBeInstanceOf(TraceFetchError);
    await expect(fetchTraceText('/trace.json', fetchImpl)).rejects.toMatchObject({
      code: 'TRACE_URL',
      message: HTTPS_TRACE_URL_MESSAGE,
    });
    await expect(fetchTraceText('data:application/json,{}', fetchImpl)).rejects.toMatchObject({
      code: 'TRACE_URL',
    });
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('fetches an accepted https URL and returns its text', async () => {
    const fetchImpl = vi.fn(async () => new Response('{"ok":true}', { status: 200 }));
    await expect(fetchTraceText('https://example.com/t.json', fetchImpl)).resolves.toBe('{"ok":true}');
    expect(fetchImpl).toHaveBeenCalledWith('https://example.com/t.json');
  });

  it('fetches http localhost', async () => {
    const fetchImpl = vi.fn(async () => new Response('{}', { status: 200 }));
    await expect(fetchTraceText('http://localhost:5173/t.json', fetchImpl)).resolves.toBe('{}');
  });

  it('surfaces an HTTP error without treating it as a scheme rejection', async () => {
    const fetchImpl = vi.fn(async () => new Response('missing', { status: 404 }));
    await expect(fetchTraceText('https://example.com/missing.json', fetchImpl)).rejects.toThrow('HTTP 404');
    await expect(fetchTraceText('https://example.com/missing.json', fetchImpl)).rejects.not.toBeInstanceOf(
      TraceFetchError,
    );
  });
});

function trackedStream(parts: Uint8Array[]): { stream: ReadableStream<Uint8Array>; pulls: () => number } {
  let index = 0;
  let pulls = 0;
  const stream = new ReadableStream<Uint8Array>({
    pull(controller) {
      pulls += 1;
      if (index >= parts.length) {
        controller.close();
        return;
      }
      controller.enqueue(parts[index]);
      index += 1;
    },
  });
  return { stream, pulls: () => pulls };
}

describe('readBoundedResponse', () => {
  it('uses the same too-large message as a local file', () => {
    expect(fileTooLargeMessage(21 * 1024 * 1024)).toBe('File too large (21.0 MB). Maximum is 20 MB.');
    expect(MAX_FILE_SIZE).toBe(20 * 1024 * 1024);
  });

  it('rejects from Content-Length before reading the body', async () => {
    const cancel = vi.fn(async () => {});
    const response = {
      headers: new Headers({ 'content-length': String(MAX_FILE_SIZE + 1) }),
      body: {
        cancel,
        getReader: () => {
          throw new Error('body was read');
        },
      },
      text: async () => {
        throw new Error('body was buffered');
      },
    } as unknown as Response;

    await expect(readBoundedResponse(response)).rejects.toMatchObject({
      code: 'FILE_TOO_LARGE',
      message: fileTooLargeMessage(MAX_FILE_SIZE + 1),
    });
    expect(cancel).toHaveBeenCalledOnce();
  });

  it('stops a stream once the byte count passes the cap', async () => {
    const tracked = trackedStream([
      encoder.encode('aaaa'),
      encoder.encode('bbbb'),
      encoder.encode('cccc'),
      encoder.encode('dddd'),
    ]);
    const response = new Response(tracked.stream, { headers: { 'content-length': '4' } });

    await expect(readBoundedResponse(response, 8)).rejects.toMatchObject({
      code: 'FILE_TOO_LARGE',
      message: fileTooLargeMessage(12),
    });
    expect(tracked.pulls()).toBe(3);
  });

  it('returns a body that fits and ignores a non-numeric Content-Length', async () => {
    const tracked = trackedStream([encoder.encode('{"a":1'), encoder.encode('}')]);
    const response = new Response(tracked.stream, { headers: { 'content-length': 'unknown' } });
    await expect(readBoundedResponse(response, 8)).resolves.toBe('{"a":1}');
  });

  it('accepts a body of exactly the cap', async () => {
    const response = new Response(encoder.encode('abcdefgh'), {
      headers: { 'content-length': '8' },
    });
    await expect(readBoundedResponse(response, 8)).resolves.toBe('abcdefgh');
  });

  it('caps a body with no stream', async () => {
    const small = { headers: new Headers(), body: null, text: async () => 'ok' } as unknown as Response;
    await expect(readBoundedResponse(small, 8)).resolves.toBe('ok');

    const big = { headers: new Headers(), body: null, text: async () => 'abcdefghi' } as unknown as Response;
    await expect(readBoundedResponse(big, 8)).rejects.toMatchObject({
      code: 'FILE_TOO_LARGE',
      message: fileTooLargeMessage(9),
    });
  });
});
