import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import { traceState } from './stores/trace';
import { MAX_FILE_SIZE } from './lib/traceUrl';

vi.mock('./lib/wasm', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/wasm')>()),
  loadWasm: vi.fn(async () => {}),
  getInitWarnings: vi.fn(() => []),
}));

vi.mock('./lib/input', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./lib/input')>()),
  handleRawInputAsync: vi.fn(async () => true),
}));

import App from './App.svelte';

beforeEach(() => {
  traceState.reset();
  window.history.replaceState(null, '', '/editor/');
});

describe('App editor Format action', () => {
  it('formats newline-delimited OTLP records instead of rejecting them', async () => {
    const { container } = render(App);
    const textarea = await vi.waitFor(() => {
      const found = container.querySelector<HTMLTextAreaElement>('textarea.editor-input');
      if (!found) throw new Error('editor not mounted yet');
      return found;
    });
    await fireEvent.input(textarea, { target: { value: '{ "resourceSpans": [] }\n{"resourceSpans" : []}\n' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Format' }));
    expect(textarea.value).toBe('{"resourceSpans":[]}\n{"resourceSpans":[]}');
    expect(screen.queryByText(/could not be formatted/)).toBeNull();
  });
});

describe('App ?trace= loading', () => {
  it('shows an https-only banner and does not fetch any other scheme', async () => {
    const fetchImpl = vi.fn();
    vi.stubGlobal('fetch', fetchImpl);
    window.history.replaceState(null, '', `/?trace=${encodeURIComponent('http://example.com/t.json')}`);
    render(App);
    expect(await screen.findByRole('alert')).toHaveTextContent('Only HTTPS trace URLs are supported.');
    expect(fetchImpl).not.toHaveBeenCalled();
  });

  it('shows the file-size banner and does not read past the cap', async () => {
    const getReader = vi.fn(() => {
      throw new Error('body was read');
    });
    vi.stubGlobal('fetch', async () => ({
      ok: true,
      headers: new Headers({ 'content-length': String(21 * 1024 * 1024) }),
      body: { cancel: async () => {}, getReader },
      text: async () => {
        throw new Error('body was buffered');
      },
    }));
    window.history.replaceState(null, '', `/?trace=${encodeURIComponent('https://example.com/huge.json')}`);
    render(App);
    expect(await screen.findByRole('alert')).toHaveTextContent('File too large (21.0 MB). Maximum is 20 MB.');
    expect(getReader).not.toHaveBeenCalled();
    expect(MAX_FILE_SIZE).toBe(20 * 1024 * 1024);
  });

  it('loads an https trace into the editor', async () => {
    vi.stubGlobal('fetch', async () => new Response('{"resourceSpans":[]}', { status: 200 }));
    window.history.replaceState(null, '', `/?trace=${encodeURIComponent('https://example.com/t.json')}`);
    const { container } = render(App);
    const textarea = await vi.waitFor(() => {
      const found = container.querySelector<HTMLTextAreaElement>('textarea.editor-input');
      if (!found || found.value !== '{"resourceSpans":[]}') throw new Error('trace not in the editor yet');
      return found;
    });
    expect(textarea.value).toBe('{"resourceSpans":[]}');
    expect(screen.queryByRole('alert')).toBeNull();
  });

  it('keeps the drawer message when the host rejects the trace', async () => {
    vi.stubGlobal('fetch', async () => new Response('missing', { status: 404 }));
    window.history.replaceState(null, '', `/?trace=${encodeURIComponent('https://example.com/missing.json')}`);
    render(App);
    expect(await screen.findByText('Failed to load trace from URL.')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).toBeNull();
  });
});
