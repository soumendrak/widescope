<script lang="ts">
  import Icon, { type IconName } from './Icon.svelte';

  /** Icon name, or null for a text-only state. */
  export let icon: IconName | null = null;
  export let title: string;
  /** Optional supporting line; the default slot holds any actions. */
  export let description: string | null = null;
  /** Tints the icon for error/loading states. */
  export let tone: 'neutral' | 'danger' | 'muted' = 'neutral';
</script>

<div class="empty" role="status">
  {#if icon}
    <div class="empty-icon empty-icon--{tone}"><Icon name={icon} size={28} /></div>
  {/if}
  <div class="empty-title">{title}</div>
  {#if description}<div class="empty-sub">{description}</div>{/if}
  <slot />
</div>

<style>
  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    padding: var(--space-6);
    text-align: center;
    color: var(--color-text-muted);
  }
  .empty-icon { color: var(--color-text-faint); line-height: 0; }
  .empty-icon--danger { color: var(--color-danger); }
  .empty-icon--muted { color: var(--color-text-faint); }
  .empty-title {
    font-size: var(--text-md);
    font-weight: 600;
    color: var(--color-text);
  }
  .empty-sub {
    font-size: var(--text-sm);
    line-height: 1.55;
    max-width: 46ch;
  }
</style>
