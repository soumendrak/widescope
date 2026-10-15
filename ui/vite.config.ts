import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
// Resolves Svelte's browser condition under vitest so components can mount and
// run their lifecycle in jsdom; a no-op for the production build.
import { svelteTesting } from '@testing-library/svelte/vite';
import wasm from 'vite-plugin-wasm';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    wasm(),
    svelte(),
    svelteTesting(),
    VitePWA({
      registerType: 'autoUpdate',
      includeAssets: ['icons/*.png'],
      manifest: {
        name: 'WideScope — LLM Trace Viewer',
        short_name: 'WideScope',
        description:
          'Browser-native trace viewer for LLM and AI agent pipelines. Zero-backend, privacy-first, powered by Rust/WASM.',
        theme_color: '#05080f',
        background_color: '#05080f',
        display: 'standalone',
        display_override: ['window-controls-overlay', 'standalone'],
        orientation: 'any',
        // The installable app is the trace viewer, which lives under /editor/;
        // scope stays at the root so the SW also serves the landing page offline.
        start_url: '/editor/',
        scope: '/',
        id: 'com.soumendrak.widescope',
        icons: [
          { src: 'icons/icon-192x192.png', sizes: '192x192', type: 'image/png' },
          { src: 'icons/icon-512x512.png', sizes: '512x512', type: 'image/png' },
          {
            src: 'icons/icon-512x512.png',
            sizes: '512x512',
            type: 'image/png',
            purpose: 'maskable',
          },
        ],
      },
      workbox: {
        globPatterns: ['**/*.{js,css,wasm,html,json,png,svg,ico,woff2}'],
        // Multi-page site — disable the blanket NavigationRoute fallback to
        // index.html so /editor/ loads editor/index.html. Cloudflare Pages
        // handles SPA fallback via not_found_handling.
        navigateFallback: null,
        runtimeCaching: [
          {
            urlPattern: /^https:\/\/.*\/conventions\/.*\.json/,
            handler: 'CacheFirst',
            options: {
              cacheName: 'conventions-cache',
              expiration: { maxEntries: 10, maxAgeSeconds: 7 * 24 * 60 * 60 },
            },
          },
        ],
      },
    }),
  ],
  build: {
    target: 'esnext',
    rollupOptions: {
      // Two-page site: marketing landing at /, the trace viewer at /editor/.
      input: {
        landing: fileURLToPath(new URL('./index.html', import.meta.url)),
        editor: fileURLToPath(new URL('./editor/index.html', import.meta.url)),
      },
    },
  },
  server: {
    fs: {
      allow: ['..', '../crates'],
    },
  },
  test: {
    // jsdom so components can actually be rendered and driven, not just imported.
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test-setup.ts'],
    include: ['src/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      include: ['src/**/*.{ts,svelte}'],
      reporter: ['text-summary', 'html', 'lcov'],
      exclude: [
        'src/**/*.test.ts',
        'src/test-setup.ts',
        // Entry bootstraps: they mount the app against a real document and have
        // no behaviour of their own to assert.
        'src/main.ts',
        'src/landing.js',
        // Mirrors of the Rust structs — type declarations, no runtime code.
        'src/lib/types.ts',
        // Canvas painting: jsdom has no 2D context, so every draw call would be
        // asserted against a stub rather than against pixels. The flame graph's
        // layout maths lives in Rust and is covered there.
        'src/components/FlameGraph.svelte',
      ],
      // Ratchets, not aspirations: each number is what the suite actually
      // reaches today, so any drop fails the build. Raise them as coverage
      // grows; never lower one to make a build pass.
      thresholds: {
        lines: 28,
        functions: 21,
        branches: 22,
        statements: 22,
        // The logic layers carry the real bar. The large visualization
        // components are driven by the browser gate instead — asserting canvas
        // and layout behaviour through jsdom would test the mock, not the view.
        'src/lib/**': { lines: 60, functions: 48, branches: 58, statements: 57 },
        'src/stores/**': { lines: 82, functions: 82, branches: 69, statements: 82 },
        'src/components/ui/**': { lines: 88, functions: 95, branches: 61, statements: 90 },
      },
    },
  },
});
