<script lang="ts">
  /** Boot states for the WASM core: the splash while it loads, the dead end if it fails. */
  export let error: string | null = null;
  /** A ?trace= URL is being fetched — same splash, honest label. */
  export let fetching = false;
</script>

{#if error}
  <div class="fatal-error">
    <h2>Failed to initialize WideScope</h2>
    <pre>{error}</pre>
    <p>Please try refreshing the page. If the issue persists, check that your browser supports WebAssembly.</p>
  </div>
{:else}
  <div class="splash">
    <div class="splash-inner">
      <span class="splash-ring" aria-hidden="true"></span>
      <img class="splash-logo" src="/widescope-logo.svg" alt="" width="64" height="64" />
      <span class="splash-name">WideScope</span>
      <span class="splash-loading">{fetching ? 'fetching trace…' : 'initializing wasm…'}</span>
    </div>
  </div>
{/if}

<style>
  .splash {
    height: 100vh;
    height: 100dvh;
    display: flex;
    align-items: center;
    justify-content: center;
    background: radial-gradient(ellipse 70% 55% at 50% 38%, #0b1322 0%, #05080f 75%);
  }
  .splash-inner {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.55rem;
    color: #e9eff8;
    position: relative;
  }
  .splash-ring {
    position: absolute;
    top: -28px;
    width: 120px;
    height: 120px;
    border-radius: 50%;
    border: 1px solid rgba(125, 211, 252, 0.35);
    border-top-color: #7dd3fc;
    animation: splash-spin 1.1s linear infinite;
  }
  .splash-logo {
    width: 64px;
    height: 64px;
    border-radius: 16px;
    box-shadow: 0 18px 50px -12px rgba(29, 78, 216, 0.6);
  }
  .splash-name {
    margin-top: 1.6rem;
    font-size: 1.55rem;
    font-weight: 750;
    letter-spacing: -0.02em;
    font-family: var(--font-display);
  }
  .splash-loading {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: #5b6b84;
    animation: splash-pulse 1.8s ease-in-out infinite;
  }
  .fatal-error {
    height: 100vh;
    height: 100dvh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    padding: 2rem;
    background: #0f172a;
    color: #f87171;
    text-align: center;
  }
  .fatal-error pre {
    background: rgba(255, 255, 255, 0.05);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.8rem;
    max-width: 600px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  .fatal-error p { color: #94a3b8; font-size: 0.875rem; }
  @keyframes splash-spin {
    to { transform: rotate(360deg); }
  }
  @keyframes splash-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
</style>
