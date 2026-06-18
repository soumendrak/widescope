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

// PWA service worker is registered by vite-plugin-pwa's auto-injected
// registerSW.js (registerType: 'autoUpdate' in vite.config.ts).

export default app;
