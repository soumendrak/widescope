import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import wasm from 'vite-plugin-wasm';
import { VitePWA } from 'vite-plugin-pwa';

export default defineConfig({
  plugins: [
    wasm(),
    svelte(),
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
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
