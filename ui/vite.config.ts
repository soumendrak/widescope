import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath, URL } from 'node:url';
import type { Connect, Plugin } from 'vite';
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';
// Resolves Svelte's browser condition under vitest so components can mount and
// run their lifecycle in jsdom; a no-op for the production build.
import { svelteTesting } from '@testing-library/svelte/vite';
import wasm from 'vite-plugin-wasm';
import { VitePWA } from 'vite-plugin-pwa';

function directoryIndex(indexRoots: string[]): Connect.NextHandleFunction {
  return (req, _res, next) => {
    if ((req.method !== 'GET' && req.method !== 'HEAD') || !req.url) return next();
    const queryAt = req.url.indexOf('?');
    const pathOnly = queryAt === -1 ? req.url : req.url.slice(0, queryAt);
    const search = queryAt === -1 ? '' : req.url.slice(queryAt);
    let decoded = pathOnly;
    try {
      decoded = decodeURIComponent(pathOnly);
    } catch {
      return next();
    }
    if (decoded.includes('..') || path.posix.basename(decoded).includes('.')) return next();
    const directory = decoded.endsWith('/') ? decoded : `${decoded}/`;
    const rel = directory.replace(/^\/+/, '');
    const found = indexRoots.some((indexRoot) => fs.existsSync(path.join(indexRoot, rel, 'index.html')));
    if (found) req.url = `${directory}index.html${search}`;
    next();
  };
}

function publicDirectoryIndex(): Plugin {
  return {
    name: 'public-directory-index',
    configureServer(server) {
      server.middlewares.use(
        directoryIndex([path.join(server.config.root, 'public'), server.config.root]),
      );
    },
    configurePreviewServer(server) {
      server.middlewares.use(
        directoryIndex([path.resolve(server.config.root, server.config.build.outDir)]),
      );
    },
  };
}

export default defineConfig({
  appType: 'mpa',
  plugins: [
    publicDirectoryIndex(),
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
          'Drop an AI-agent trace (OTLP, OpenInference or Jaeger JSON, or a zip of them) into your browser and debug the run — nothing uploaded, nothing installed, shareable as a link.',
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
        'src/main.ts',
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
        lines: 39,
        functions: 27,
        branches: 30,
        statements: 30,
        // The logic layers carry the real bar. The large visualization
        // components are driven by the browser gate instead — asserting canvas
        // and layout behaviour through jsdom would test the mock, not the view.
        'src/lib/**': { lines: 71, functions: 57, branches: 67, statements: 67 },
        'src/stores/**': { lines: 85, functions: 85, branches: 70, statements: 85 },
        'src/components/ui/**': { lines: 88, functions: 96, branches: 62, statements: 91 },
      },
    },
  },
});
