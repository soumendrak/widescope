import { describe, expect, it } from 'vitest';
import { SERVICE_COLORS } from './palette';

describe('service palette', () => {
  it('offers a stable, non-empty set of distinct colours', () => {
    expect(SERVICE_COLORS.length).toBeGreaterThan(4);
    expect(new Set(SERVICE_COLORS).size).toBe(SERVICE_COLORS.length);
  });

  it('is made of usable CSS colours', () => {
    for (const color of SERVICE_COLORS) {
      expect(color).toMatch(/^(#[0-9a-f]{3,8}|rgb|hsl|var\()/i);
    }
  });
});
