<script lang="ts">
  import { createEventDispatcher, onDestroy, tick } from 'svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  export let open = false;
  export let title: string;
  /** Width cap; dialogs stay below the viewport height and scroll internally. */
  export let maxWidth = '560px';

  const dispatch = createEventDispatcher<{ close: void }>();

  let dialogEl: HTMLElement | null = null;
  let previouslyFocused: HTMLElement | null = null;

  const FOCUSABLE =
    'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

  function focusable(): HTMLElement[] {
    if (!dialogEl) return [];
    return [...dialogEl.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
      (el) => el.offsetParent !== null || el === document.activeElement
    );
  }

  /**
   * Focus trap: Tab from the last control wraps to the first and vice versa, so
   * keyboard focus can never escape into the page behind the dialog.
   */
  function onKeydown(e: KeyboardEvent): void {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      dispatch('close');
      return;
    }
    if (e.key !== 'Tab') return;

    const items = focusable();
    if (items.length === 0) {
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;

    if (e.shiftKey && (active === first || !dialogEl?.contains(active))) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && active === last) {
      e.preventDefault();
      first.focus();
    }
  }

  // Move focus in on open, and restore it to the trigger on close. The dialog
  // component stays mounted across open/close, so restoration has to hang off
  // the `open` transition rather than onDestroy.
  let wasOpen = false;
  $: {
    if (open && !wasOpen) void enter();
    else if (!open && wasOpen) restoreFocus();
    wasOpen = open;
  }

  async function enter(): Promise<void> {
    previouslyFocused = document.activeElement as HTMLElement | null;
    await tick();
    (focusable()[0] ?? dialogEl)?.focus();
  }

  function restoreFocus(): void {
    previouslyFocused?.focus?.();
    previouslyFocused = null;
  }

  onDestroy(restoreFocus);
</script>

<svelte:window on:keydown={onKeydown} />

{#if open}
  <div class="backdrop">
    <!-- A real button carries the click-outside affordance, so the backdrop
         needs no click handler on a non-interactive div. Escape also closes. -->
    <button
      type="button"
      class="backdrop-dismiss"
      tabindex="-1"
      aria-hidden="true"
      on:click={() => dispatch('close')}
    ></button>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      bind:this={dialogEl}
      style={`max-width: ${maxWidth};`}
    >
      <div class="dialog-header">
        <h2>{title}</h2>
        <Button variant="ghost" size="sm" icon aria-label="Close" on:click={() => dispatch('close')}>
          <Icon name="close" size={15} />
        </Button>
      </div>
      <div class="dialog-body"><slot /></div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-4);
    background: color-mix(in srgb, var(--color-bg) 72%, transparent);
    backdrop-filter: blur(4px);
  }
  .backdrop-dismiss {
    position: absolute;
    inset: 0;
    border: 0;
    padding: 0;
    background: transparent;
    cursor: default;
  }
  .dialog {
    position: relative;
    width: 100%;
    max-height: calc(100dvh - 4rem);
    overflow-y: auto;
    background: var(--color-surface);
    color: var(--color-text);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-panel);
  }
  .dialog:focus { outline: none; }
  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-bottom: 1px solid var(--color-border);
  }
  .dialog-body {
    padding: var(--space-4);
  }
  .dialog-header h2 {
    margin: 0;
    font-size: var(--text-md);
    font-weight: 700;
  }
</style>
