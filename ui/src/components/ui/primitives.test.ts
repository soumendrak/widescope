import { describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import Button from './Button.svelte';
import Chip from './Chip.svelte';
import EmptyState from './EmptyState.svelte';
import Icon from './Icon.svelte';

describe('Button', () => {
  it('is a real button that reports clicks', async () => {
    const onClick = vi.fn();
    render(Button, { props: { 'aria-label': 'Open file' }, events: { click: onClick } });
    await userEvent.click(screen.getByRole('button', { name: 'Open file' }));
    expect(onClick).toHaveBeenCalled();
  });

  it('does not fire while disabled', async () => {
    const onClick = vi.fn();
    render(Button, {
      props: { disabled: true, 'aria-label': 'Open file' },
      events: { click: onClick },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Open file' }));
    expect(onClick).not.toHaveBeenCalled();
  });

  it('carries its variant and size onto the class list', () => {
    render(Button, { props: { variant: 'primary', size: 'sm', icon: true, 'aria-label': 'x' } });
    const button = screen.getByRole('button', { name: 'x' });
    expect(button.className).toContain('btn--primary');
    expect(button.className).toContain('btn--sm');
    expect(button.className).toContain('btn--icon');
  });
});

describe('Chip', () => {
  it('is a static badge by default', () => {
    render(Chip, { props: { 'data-testid': 'chip' } });
    expect(screen.getByTestId('chip').tagName).toBe('SPAN');
  });

  it('becomes a real button when interactive', async () => {
    const onClick = vi.fn();
    render(Chip, {
      props: { interactive: true, 'aria-label': 'Clear filter' },
      events: { click: onClick },
    });
    await userEvent.click(screen.getByRole('button', { name: 'Clear filter' }));
    expect(onClick).toHaveBeenCalled();
  });
});

describe('EmptyState', () => {
  it('reads as a status region with its title and description', () => {
    render(EmptyState, {
      props: {
        title: 'Could not parse trace',
        description: 'Update the JSON in the editor',
        icon: 'warning',
        tone: 'danger',
      },
    });
    const status = screen.getByRole('status');
    expect(status).toHaveTextContent('Could not parse trace');
    expect(status).toHaveTextContent('Update the JSON in the editor');
  });

  it('renders without an icon or description', () => {
    render(EmptyState, { props: { title: 'Nothing here' } });
    expect(screen.getByRole('status')).toHaveTextContent('Nothing here');
  });
});

describe('Icon', () => {
  it('renders an inline svg marked decorative', () => {
    const { container } = render(Icon, { props: { name: 'link', size: 13 } });
    const svg = container.querySelector('svg');
    expect(svg).not.toBeNull();
    expect(svg).toHaveAttribute('aria-hidden', 'true');
    expect(svg?.getAttribute('width')).toBe('13');
  });
});
