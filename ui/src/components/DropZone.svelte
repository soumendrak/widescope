<script lang="ts">
  import { fade } from 'svelte/transition';
  import { handleFile } from '../lib/input';
  import { fadeConfig } from '../lib/animation';

  export let onFileDrop: ((file: File) => void) | null = null;

  let isDragging = false;
  let dragCounter = 0;

  function onDragEnter(e: DragEvent) {
    e.preventDefault();
    dragCounter++;
    isDragging = true;
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function onDragLeave(e: DragEvent) {
    e.preventDefault();
    dragCounter--;
    if (dragCounter <= 0) {
      isDragging = false;
      dragCounter = 0;
    }
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragging = false;
    dragCounter = 0;
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    if (onFileDrop) {
      onFileDrop(file);
      return;
    }
    handleFile(file);
  }
</script>

<svelte:document
  on:dragenter={onDragEnter}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
/>

{#if isDragging}
  <div class="overlay" aria-hidden="true" transition:fade={fadeConfig()}>
    <div class="inner">
      <div class="drop-flames">
        <i style="--w:84px"></i>
        <span><i style="--w:30px"></i><i class="hot" style="--w:22px"></i><i style="--w:24px"></i></span>
      </div>
      <div class="label">Drop trace to load</div>
      <div class="sublabel">.json — OTLP · Jaeger · OpenInference</div>
      <div class="privacy">⬢ parsed locally, never uploaded</div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: color-mix(in srgb, var(--color-bg, #05080f) 72%, transparent);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    backdrop-filter: blur(6px);
  }

  .overlay::before {
    content: '';
    position: absolute;
    inset: 14px;
    border: 2px dashed color-mix(in srgb, var(--color-sky, #7dd3fc) 55%, transparent);
    border-radius: 18px;
    animation: drop-breathe 1.6s ease-in-out infinite;
  }

  @keyframes drop-breathe {
    0%, 100% { opacity: 0.9; }
    50% { opacity: 0.45; }
  }

  .inner {
    text-align: center;
    background: linear-gradient(180deg, color-mix(in srgb, var(--color-surface, #0d1626) 96%, var(--color-accent)), var(--color-surface, #0d1626));
    border: 1px solid var(--color-border, rgba(125, 211, 252, 0.13));
    border-radius: 16px;
    padding: 2.2rem 3.2rem;
    box-shadow: var(--shadow-panel), 0 -1px 0 rgba(186, 230, 253, 0.12) inset;
    animation: drop-pop 0.35s var(--ease-bounce, cubic-bezier(0.34, 1.56, 0.64, 1));
  }

  @keyframes drop-pop {
    from { transform: scale(0.92); opacity: 0; }
    to { transform: scale(1); opacity: 1; }
  }

  .drop-flames {
    display: inline-flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 1rem;
  }

  .drop-flames span {
    display: flex;
    gap: 5px;
    justify-content: center;
  }

  .drop-flames i {
    display: block;
    width: var(--w, 30px);
    height: 10px;
    border-radius: 3px;
    background: var(--grad-span, linear-gradient(180deg, #7dd3fc, #3b82f6));
    box-shadow: 0 1px 0 rgba(255, 255, 255, 0.18) inset;
  }

  .drop-flames i.hot { background: var(--grad-hot, linear-gradient(180deg, #fcd34d, #f59e0b)); }

  .label {
    font-family: var(--font-display);
    font-size: 1.35rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--color-text, #e9eff8);
    margin-bottom: 0.3rem;
  }

  .sublabel {
    font-size: 0.74rem;
    color: var(--color-text-muted, #9aa8bd);
    font-family: var(--font-mono);
    letter-spacing: 0.05em;
  }

  .privacy {
    margin-top: 0.8rem;
    font-family: var(--font-mono);
    font-size: 0.62rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--color-success, #34d399);
  }
</style>
