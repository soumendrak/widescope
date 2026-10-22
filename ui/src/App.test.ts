import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';

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
