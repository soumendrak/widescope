import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import Footer from './Footer.svelte';

describe('Footer', () => {
  it('shows the Apache 2.0 license', () => {
    render(Footer);
    expect(screen.getByText('Apache 2.0')).toBeInTheDocument();
    expect(screen.queryByText('MIT')).not.toBeInTheDocument();
  });
});
