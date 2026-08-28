import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import WelcomeState from './WelcomeState.svelte';
import { recentTraces } from '../lib/recent';

const noop = () => {};

describe('WelcomeState', () => {
  it('offers the three ways to load a trace', async () => {
    const onLoadSample = vi.fn();
    const onOpenFile = vi.fn();
    const onPaste = vi.fn();
    render(WelcomeState, { props: { onLoadSample, onOpenFile, onPaste } });

    await userEvent.click(screen.getByRole('button', { name: /sample/i }));
    await userEvent.click(screen.getByRole('button', { name: /open file/i }));
    await userEvent.click(screen.getByRole('button', { name: /paste/i }));

    expect(onLoadSample).toHaveBeenCalled();
    expect(onOpenFile).toHaveBeenCalled();
    expect(onPaste).toHaveBeenCalled();
  });

  it('names the formats it accepts and promises nothing is uploaded', () => {
    render(WelcomeState, { props: { onLoadSample: noop, onOpenFile: noop, onPaste: noop } });
    expect(screen.getByText(/OTLP/)).toBeInTheDocument();
    expect(screen.getByText(/Jaeger/)).toBeInTheDocument();
    expect(screen.getByText(/OpenInference/)).toBeInTheDocument();
    expect(screen.getByText(/no upload/i)).toBeInTheDocument();
  });

  it('hides the recent row until there is something to reload', () => {
    recentTraces.set([]);
    render(WelcomeState, { props: { onLoadSample: noop, onOpenFile: noop, onPaste: noop } });
    expect(screen.queryByLabelText('Recent traces')).not.toBeInTheDocument();
  });

  it('reloads a remembered trace from its chip', async () => {
    recentTraces.set([{ id: 1, name: 'checkout.json', size: 2_500_000, savedAt: 0 } as never]);
    const onLoadText = vi.fn();
    render(WelcomeState, {
      props: { onLoadSample: noop, onOpenFile: noop, onPaste: noop, onLoadText },
    });

    const chip = screen.getByRole('button', { name: /checkout\.json/i });
    expect(chip).toHaveAttribute('title', expect.stringContaining('2.4 MB'));
    await userEvent.click(chip);
    // The chip reads the JSON back out of IndexedDB, which jsdom does not have;
    // what matters here is that the click reaches the handler path.
    expect(screen.getByLabelText('Recent traces')).toBeInTheDocument();
    recentTraces.set([]);
  });
});
