import './styles/tokens.css';
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');

if (!target) {
  document.body.innerHTML = '<pre style="color:red;padding:2rem">Fatal: #app element not found</pre>';
  throw new Error('#app element not found');
}

let app: ReturnType<typeof mount>;
try {
  app = mount(App, { target });
} catch (e) {
  target.innerHTML = `<pre style="color:red;padding:2rem">Fatal init error:\n${String(e)}</pre>`;
  throw e;
}

// Register the PWA service worker — vite-plugin-pwa handles generation
// Registration is deferred to avoid blocking initial render
if ('serviceWorker' in navigator) {
  // Only register in production (dev server uses HMR, not SW)
  if (import.meta.env.PROD) {
    window.addEventListener('load', async () => {
      try {
        const reg = await navigator.serviceWorker.register('/sw.js');
        console.log('[WideScope PWA] Service worker registered', reg.scope);

        // Check for updates on each page load
        reg.addEventListener('updatefound', () => {
          const installing = reg.installing;
          if (installing) {
            installing.addEventListener('statechange', () => {
              if (installing.state === 'installed' && navigator.serviceWorker.controller) {
                // New version available — auto-update (configured in vite.config.ts)
                console.log('[WideScope PWA] Update installed, will activate on next reload');
              }
            });
          }
        });
      } catch (err) {
        console.warn('[WideScope PWA] Service worker registration failed:', err);
      }
    });
  }
}

export default app;
