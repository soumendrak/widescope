import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ErrorBanner from './ErrorBanner.svelte';
import ViewSkeleton from './ui/ViewSkeleton.svelte';

describe('ErrorBanner', () => {
  it('shows nothing when the trace parsed cleanly', () => {
    const { container } = render(ErrorBanner, { props: { error: null, warnings: [] } });
    expect(container.textContent?.trim()).toBe('');
  });

  it('reports a parse error', () => {
    render(ErrorBanner, {
      props: {
        error: { code: 'INVALID_JSON', message: 'expected value at line 3' } as never,
        warnings: [],
      },
    });
    expect(screen.getByText(/expected value at line 3/)).toBeInTheDocument();
  });

  it('lists data-quality warnings with their codes', () => {
    render(ErrorBanner, {
      props: {
        error: null,
        warnings: [
          { code: 'ORPHAN_PARENT', message: '1 span references a missing parent', count: 1 },
          { code: 'DUPLICATE_SPAN_ID', message: '1 duplicate discarded', count: 1 },
        ] as never,
      },
    });
    expect(screen.getByText(/ORPHAN_PARENT/)).toBeInTheDocument();
    expect(screen.getByText(/missing parent/)).toBeInTheDocument();
  });
});

describe('ViewSkeleton', () => {
  it('names the phase it is waiting on and reflects progress', () => {
    const { container } = render(ViewSkeleton, {
      props: { label: 'Computing waterfall', progress: 60 },
    });
    expect(screen.getByText(/computing waterfall/i)).toBeInTheDocument();
    expect(container.innerHTML).toContain('60');
  });
});
