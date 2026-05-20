import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import wasm from 'vite-plugin-wasm';

export default defineConfig({
  plugins: [wasm(), svelte()],
  build: {
    target: 'esnext',
  },
  server: {
    fs: {
      allow: ['..', '../crates'],
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
