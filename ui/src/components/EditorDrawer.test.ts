import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import EditorDrawer from './EditorDrawer.svelte';

function drawer(props: Record<string, unknown> = {}) {
  return render(EditorDrawer, {
    props: {
      value: '',
      collapsed: false,
      onInput: vi.fn(),
      onSubmit: vi.fn(),
      onClear: vi.fn(),
      onLoadSample: vi.fn(),
      onPaste: vi.fn(),
      onFormat: vi.fn(),
      ...props,
    },
  });
}

describe('EditorDrawer', () => {
  it('shows the JSON input while open', () => {
    const { container } = drawer();
    expect(container.querySelector('.editor-input')).not.toBeNull();
    expect(container.querySelector('.editor-panel--collapsed')).toBeNull();
  });

  it('unmounts the textarea when collapsed, leaving the strip', () => {
    const { container } = drawer({ collapsed: true });
    expect(container.querySelector('.editor-panel--collapsed')).not.toBeNull();
    expect(container.querySelector('.editor-input')).toBeNull();
  });

  it('reports typing so the live parse can be scheduled', async () => {
    const onInput = vi.fn();
    const { container } = drawer({ onInput });
    const textarea = container.querySelector('textarea');
    expect(textarea).not.toBeNull();
    await userEvent.type(textarea!, '{{');
    expect(onInput).toHaveBeenCalled();
  });

  it('exposes the load shortcuts on the strip', async () => {
    const onLoadSample = vi.fn();
    const onPaste = vi.fn();
    drawer({ onLoadSample, onPaste });
    await userEvent.click(screen.getByRole('button', { name: /sample/i }));
    await userEvent.click(screen.getByRole('button', { name: /paste/i }));
    expect(onLoadSample).toHaveBeenCalled();
    expect(onPaste).toHaveBeenCalled();
  });

  it('disables clear and format until there is JSON to act on', () => {
    drawer({ value: '' });
    expect(screen.getByRole('button', { name: /clear/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /format/i })).toBeDisabled();
  });

  it('enables clear and format once the editor holds a payload', async () => {
    const onClear = vi.fn();
    drawer({ value: '{"resourceSpans":[]}', onClear });
    const clear = screen.getByRole('button', { name: /clear/i });
    expect(clear).toBeEnabled();
    await userEvent.click(clear);
    expect(onClear).toHaveBeenCalled();
  });

  it('surfaces a parse message when one is passed in', () => {
    drawer({ message: 'Clipboard access was blocked' });
    expect(screen.getByText(/clipboard access was blocked/i)).toBeInTheDocument();
  });

  it('toggles open and closed from the disclosure caret', async () => {
    const { container } = drawer({ collapsed: true });
    const caret = container.querySelector('button[aria-controls="editor-body"]');
    expect(caret).toHaveAttribute('aria-expanded', 'false');
    await userEvent.click(caret!);
    expect(container.querySelector('.editor-input')).not.toBeNull();
  });
});
