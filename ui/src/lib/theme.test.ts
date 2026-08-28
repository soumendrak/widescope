import { beforeEach, describe, expect, it, vi } from 'vitest';
import { get } from 'svelte/store';
import { resolveTheme, theme } from './theme';

function withOsPreference(prefersLight: boolean): void {
  vi.stubGlobal('matchMedia', (query: string) => ({
    matches: query.includes('light') ? prefersLight : !prefersLight,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  }));
}

describe('resolveTheme', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-theme');
  });

  it('honours a stored choice over anything the OS says', () => {
    withOsPreference(true);
    expect(resolveTheme('dark')).toBe('dark');
    withOsPreference(false);
    expect(resolveTheme('light')).toBe('light');
  });

  it('falls back to the OS preference when nothing is stored', () => {
    withOsPreference(true);
    expect(resolveTheme(null)).toBe('light');
    withOsPreference(false);
    expect(resolveTheme(null)).toBe('dark');
  });

  it('ignores a stored value that is not a theme', () => {
    withOsPreference(false);
    expect(resolveTheme('purple')).toBe('dark');
    expect(resolveTheme('')).toBe('dark');
  });
});

describe('theme store', () => {
  it('writes the applied theme onto the document element', () => {
    theme.apply('light');
    expect(get(theme)).toBe('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');

    theme.apply('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('toggles between the two themes', () => {
    theme.apply('dark');
    theme.toggle();
    expect(get(theme)).toBe('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');

    theme.toggle();
    expect(get(theme)).toBe('dark');
  });
});
