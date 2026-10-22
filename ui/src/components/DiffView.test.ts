import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/svelte';
import DiffView from './DiffView.svelte';

describe('DiffView', () => {
  it('opens a comparison picker that accepts newline-delimited traces', async () => {
    vi.spyOn(HTMLInputElement.prototype, 'click').mockImplementation(() => {});
    const created: HTMLInputElement[] = [];
    const original = document.createElement.bind(document);
    vi.spyOn(document, 'createElement').mockImplementation((tag: string) => {
      const el = original(tag);
      if (tag === 'input') created.push(el as HTMLInputElement);
      return el;
    });

    render(DiffView);
    await fireEvent.click(screen.getByRole('button', { name: 'Open comparison file' }));
    expect(created.at(-1)?.accept).toBe('.json,.jsonl,.ndjson');
  });
});
