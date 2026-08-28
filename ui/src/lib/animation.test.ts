import { describe, expect, it } from 'vitest';
import { viewSlideIn, viewSlideOut } from './animation';

describe('view transitions', () => {
  it('slides the incoming view in from the side it came from', () => {
    expect(viewSlideIn(1).x).toBeGreaterThan(0);
    expect(viewSlideIn(-1).x).toBeLessThan(0);
  });

  it('sends the outgoing view the opposite way', () => {
    expect(Math.sign(viewSlideOut(1).x)).toBe(-Math.sign(viewSlideIn(1).x));
    expect(Math.sign(viewSlideOut(-1).x)).toBe(-Math.sign(viewSlideIn(-1).x));
  });

  it('keeps every transition short enough to feel instant', () => {
    for (const config of [viewSlideIn(1), viewSlideOut(1), viewSlideIn(-1), viewSlideOut(-1)]) {
      expect(config.duration).toBeGreaterThan(0);
      expect(config.duration).toBeLessThanOrEqual(300);
    }
  });
});
