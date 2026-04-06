<script lang="ts">
  import { handleFile } from '../lib/input';

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
  <div class="overlay" aria-hidden="true">
    <div class="inner">
      <div class="icon">📂</div>
      <div class="label">Drop trace file to load</div>
      <div class="sublabel">.json — OTLP, Jaeger, or OpenInference</div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 1000;
    background: rgba(59, 130, 246, 0.15);
    border: 3px dashed var(--color-accent, #3b82f6);
    display: flex;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    backdrop-filter: blur(2px);
  }

  .inner {
    text-align: center;
    background: var(--color-surface, #fff);
    border-radius: 12px;
    padding: 2rem 3rem;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
  }

  .icon {
    font-size: 3rem;
    margin-bottom: 0.5rem;
  }

  .label {
    font-size: 1.25rem;
    font-weight: 600;
    color: var(--color-text, #1e293b);
    margin-bottom: 0.25rem;
  }

  .sublabel {
    font-size: 0.875rem;
    color: var(--color-text-muted, #64748b);
    font-family: monospace;
  }
</style>
