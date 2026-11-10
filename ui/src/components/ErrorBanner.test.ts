import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ErrorBanner from './ErrorBanner.svelte';
import ViewSkeleton from './ui/ViewSkeleton.svelte';

function bannerText(): string {
  return (screen.getByRole('alert').textContent ?? '').replace(/\s+/g, ' ').trim();
}

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

  it('names a bad NDJSON line once', () => {
    render(ErrorBanner, {
      props: {
        error: {
          error_type: 'WideError',
          code: 'INVALID_JSON',
          message: 'Invalid JSON: line 2: EOF while parsing a list',
          context: { line: 2, column: 1 },
        },
        warnings: [],
      },
    });
    const text = bannerText();
    expect(text).toContain('EOF while parsing a list at line 2, column 1');
    expect(text.match(/line 2/g)).toHaveLength(1);
    expect(text.match(/column 1/g)).toHaveLength(1);
  });

  it('still places a line the message does not name', () => {
    render(ErrorBanner, {
      props: {
        error: {
          error_type: 'WideError',
          code: 'INVALID_JSON',
          message: 'Invalid JSON: line 20: truncated',
          context: { line: 2, column: 8 },
        },
        warnings: [],
      },
    });
    const text = bannerText();
    expect(text).toContain('line 20: truncated');
    expect(text).toContain('at line 2, column 8');
    expect(text.match(/line 2(?!\d)/g)).toHaveLength(1);
  });

  it('does not repeat a serde position already in the message', () => {
    render(ErrorBanner, {
      props: {
        error: {
          error_type: 'WideError',
          code: 'INVALID_JSON',
          message: 'Invalid JSON: expected value at line 1 column 1',
          context: { line: 1, column: 1 },
        },
        warnings: [],
      },
    });
    const text = bannerText();
    expect(text).toContain('expected value at line 1 column 1');
    expect(text.match(/line 1/g)).toHaveLength(1);
    expect(text.match(/column 1/g)).toHaveLength(1);
  });

  it('leaves a position that is already written with a column', () => {
    render(ErrorBanner, {
      props: {
        error: {
          error_type: 'WideError',
          code: 'INVALID_JSON',
          message: 'Invalid JSON: expected value at line 2, column 4',
          context: { line: 2, column: 4 },
        },
        warnings: [],
      },
    });
    const text = bannerText();
    expect(text).toContain('expected value at line 2, column 4');
    expect(text.match(/line 2/g)).toHaveLength(1);
    expect(text.match(/column 4/g)).toHaveLength(1);
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
