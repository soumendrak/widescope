export const DUR = { micro: 120, fast: 180, normal: 280, slow: 380 } as const;

export function springGentle(t: number): number {
  if (t === 0 || t === 1) return t;
  return 1 - Math.exp(-7 * t) * Math.cos(5 * t);
}

export function springBouncy(t: number): number {
  if (t === 0 || t === 1) return t;
  return 1 - Math.exp(-5.5 * t) * Math.cos(12 * t);
}

export function springSnappy(t: number): number {
  if (t === 0 || t === 1) return t;
  return 1 - Math.pow(1 - t, 3);
}

export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined') return false;
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function respectMotion<T extends { duration?: number }>(config: T): T {
  if (prefersReducedMotion()) return { ...config, duration: 0 };
  return config;
}

export const fadeConfig = () => respectMotion({ duration: DUR.normal });
export const flyRightConfig = () => respectMotion({ x: 48, duration: DUR.normal, easing: springGentle });
export const flyUpConfig = () => respectMotion({ y: -14, duration: DUR.normal, easing: springGentle });
export const scaleConfig = () => respectMotion({ start: 0.92, duration: DUR.normal, easing: springBouncy });

export const viewSlideIn = (direction: 1 | -1) =>
  respectMotion({ x: direction * 40, duration: DUR.fast, easing: springSnappy });
export const viewSlideOut = (direction: 1 | -1) =>
  respectMotion({ x: direction * -40, duration: DUR.micro + 40, easing: springSnappy });
