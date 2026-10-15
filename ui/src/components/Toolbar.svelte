<script lang="ts">
  import { traceState } from '../stores/trace';
  import { openFilePicker } from '../lib/input';
  import { getCostBreakdown } from '../lib/wasm';
  import { budgets, checkViolations } from '../stores/budgets';
  import BudgetsDialog from './BudgetsDialog.svelte';
  import TopBar from './TopBar.svelte';
  import FilterBar from './FilterBar.svelte';

  export let onOpenFile: () => void = () => openFilePicker();

  let budgetsOpen = false;

  // Violations are computed once here because both bars display them: TopBar as
  // a count on the budgets button, FilterBar as a clickable warning stat.
  $: summary = $traceState.summary;
  $: status = $traceState.status;
  $: costBreakdown = status === 'loaded' ? getCostBreakdown() : null;
  $: violations = checkViolations($budgets, summary, costBreakdown?.total_cost_usd ?? null);
</script>

<header class="toolbar" class:toolbar--loaded={status === 'loaded'}>
  <TopBar {onOpenFile} {violations} onOpenBudgets={() => (budgetsOpen = true)} />
  {#if status === 'loaded' && summary}
    <FilterBar {violations} onOpenBudgets={() => (budgetsOpen = true)} />
  {/if}
</header>

<BudgetsDialog
  open={budgetsOpen}
  {violations}
  on:close={() => (budgetsOpen = false)}
/>

<style>
  .toolbar {
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    background: color-mix(in srgb, var(--color-toolbar, #070c16) 88%, transparent);
    backdrop-filter: blur(14px);
    color: var(--color-toolbar-text, #e9eff8);
    border-bottom: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    z-index: 10;
  }















  @keyframes loading-pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.35; }
  }







































  /* ── Stats bar ────────────────────────────────────────────────── */






















  /* ── Responsive ───────────────────────────────────────────────── */

</style>
