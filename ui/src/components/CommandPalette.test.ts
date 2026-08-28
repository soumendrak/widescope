import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { get } from 'svelte/store';
import CommandPalette from './CommandPalette.svelte';
import { traceState } from '../stores/trace';
import { activeView, selectedSpanId } from '../stores/selection';
import { showCriticalPath } from '../stores/criticalPath';
import { theme } from '../lib/theme';
import type { AgentFlow, FlameGraphLayout, ServiceGraph, TraceSummary } from '../lib/types';

vi.mock('../lib/wasm', () => ({
  searchSpans: vi.fn(() => ['span-a']),
  getSpanDetail: vi.fn(() => ({ span_id: 'span-a', operation_name: 'POST /api/chat' })),
}));

const summary = { span_count: 7, llm_span_count: 4, trace_id: 't' } as unknown as TraceSummary;

function loaded(): void {
  traceState.setLoaded(
    summary,
    { nodes: [], max_depth: 0 } as unknown as FlameGraphLayout,
    null,
    null,
    { nodes: [], edges: [] } as unknown as ServiceGraph,
    { nodes: [], edges: [] } as unknown as AgentFlow,
    false,
  );
}

function open(overrides = {}) {
  return render(CommandPalette, {
    props: {
      open: true,
      onLoadSample: vi.fn(),
      onOpenFile: vi.fn(),
      onPaste: vi.fn(),
      onExport: vi.fn(),
      ...overrides,
    },
  });
}

describe('CommandPalette', () => {
  beforeEach(() => {
    traceState.reset();
    activeView.set('waterfall');
    selectedSpanId.set(null);
  });

  it('stays out of the way until it is opened', () => {
    render(CommandPalette, {
      props: {
        open: false,
        onLoadSample: vi.fn(),
        onOpenFile: vi.fn(),
        onPaste: vi.fn(),
        onExport: vi.fn(),
      },
    });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('opens with the search input focused', async () => {
    loaded();
    open();
    const input = screen.getByRole('combobox') ?? screen.getByRole('textbox');
    await vi.waitFor(() => expect(document.activeElement).toBe(input));
  });

  it('filters to a lens and switches to it', async () => {
    loaded();
    open();
    await userEvent.keyboard('flame');
    const option = await screen.findByRole('option', { name: /flame/i });
    await userEvent.click(option);
    expect(get(activeView)).toBe('flame');
  });

  it('toggles the theme and the critical path as commands', async () => {
    loaded();
    theme.apply('dark');
    showCriticalPath.set(true);
    open();

    await userEvent.keyboard('theme');
    await userEvent.click(await screen.findByRole('option', { name: /theme/i }));
    expect(get(theme)).toBe('light');

    // Reopen for the critical-path command; running one closes the palette.
    open();
    await userEvent.keyboard('critical');
    await userEvent.click(await screen.findByRole('option', { name: /critical/i }));
    expect(get(showCriticalPath)).toBe(false);
  });

  it('runs the trace actions it is handed', async () => {
    const onLoadSample = vi.fn();
    open({ onLoadSample });
    await userEvent.keyboard('sample');
    await userEvent.click(await screen.findByRole('option', { name: /sample/i }));
    expect(onLoadSample).toHaveBeenCalled();
  });

  it('closes on Escape', async () => {
    const onClose = vi.fn();
    render(CommandPalette, {
      props: {
        open: true,
        onLoadSample: vi.fn(),
        onOpenFile: vi.fn(),
        onPaste: vi.fn(),
        onExport: vi.fn(),
      },
      events: { close: onClose },
    });
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalled();
  });
});
