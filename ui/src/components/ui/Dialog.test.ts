import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Dialog from './Dialog.svelte';

/**
 * The dialog stays mounted across open/close, so `open` is the switch. Svelte 5
 * removed `$on`, so the close event is subscribed through the props object.
 */
function open(onClose = vi.fn(), title = 'Performance budgets') {
  const result = render(Dialog, {
    props: { open: true, title },
    events: { close: onClose },
  });
  return { ...result, onClose };
}

describe('Dialog', () => {
  it('renders nothing until it is opened', () => {
    render(Dialog, { props: { open: false, title: 'Budgets' } });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('announces itself as a modal dialog with its title', () => {
    open();
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveAttribute('aria-modal', 'true');
    expect(screen.getByText('Performance budgets')).toBeInTheDocument();
  });

  it('emits close on Escape', async () => {
    const { onClose } = open();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });

  it('emits close from the close control', async () => {
    const { onClose } = open();
    const closeButton = screen
      .getAllByRole('button')
      .find((b) => /close/i.test(b.getAttribute('aria-label') ?? b.textContent ?? ''));
    expect(closeButton).toBeDefined();
    await userEvent.click(closeButton!);
    expect(onClose).toHaveBeenCalled();
  });

  it('moves focus inside itself when it opens', async () => {
    open();
    const dialog = screen.getByRole('dialog');
    await vi.waitFor(() => {
      expect(dialog.contains(document.activeElement)).toBe(true);
    });
  });

  it('keeps Tab inside the dialog', async () => {
    open();
    const dialog = screen.getByRole('dialog');
    for (let i = 0; i < 8; i++) {
      await userEvent.tab();
      expect(dialog.contains(document.activeElement)).toBe(true);
    }
    await userEvent.tab({ shift: true });
    expect(dialog.contains(document.activeElement)).toBe(true);
  });
});
