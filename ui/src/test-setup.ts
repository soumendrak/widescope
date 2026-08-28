import { afterEach, vi } from 'vitest';

// The suites that instantiate the real WASM module run under `node` (see the
// header of permalink.test.ts); everything DOM-shaped below is skipped there.
const hasDom = typeof window !== 'undefined';

if (hasDom) {
  await import('@testing-library/jest-dom/vitest');
  const { cleanup } = await import('@testing-library/svelte');

  // Components read stores that outlive a single test; unmount and reset
  // between cases so one test's selection cannot leak into the next one.
  afterEach(() => {
    cleanup();
    localStorage.clear();
  });

  // jsdom implements neither of these, and several components call them on mount.
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });

  class ResizeObserverStub {
    observe(): void {}
    unobserve(): void {}
    disconnect(): void {}
  }
  window.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}

afterEach(() => {
  vi.restoreAllMocks();
});
