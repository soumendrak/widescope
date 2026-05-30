<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    budgets,
    BUDGET_FIELDS,
    fieldUnit,
    displayToRawValue,
    rawToDisplayValue,
    type BudgetField,
    type BudgetOperator,
    type BudgetViolation,
  } from '../stores/budgets';

  export let open = false;
  export let violations: BudgetViolation[] = [];

  const dispatch = createEventDispatcher<{ close: void }>();

  let newField: BudgetField = 'duration_ns';
  let newOperator: BudgetOperator = '<';
  let newValue = 0;

  const OPERATORS: { value: BudgetOperator; label: string }[] = [
    { value: '<', label: '<' },
    { value: '<=', label: '≤' },
    { value: '>', label: '>' },
    { value: '>=', label: '≥' },
    { value: '==', label: '=' },
  ];

  function close() {
    dispatch('close');
  }

  function onBackdropPointerDown(e: PointerEvent) {
    if (e.target === e.currentTarget) close();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') close();
  }

  function addBudget() {
    if (!Number.isFinite(newValue)) return;
    const raw = displayToRawValue(newField, newValue);
    budgets.add(newField, newOperator, raw);
    newValue = 0;
  }

  function isViolated(id: string): boolean {
    return violations.some((v) => v.budget.id === id);
  }
</script>

<svelte:window on:keydown={onKeydown} />

{#if open}
  <div
    class="backdrop"
    role="presentation"
    on:pointerdown={onBackdropPointerDown}
  >
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="budgets-title"
    >
      <header class="dialog-header">
        <h2 id="budgets-title">Performance budgets</h2>
        <button type="button" class="icon-btn" aria-label="Close" on:click={close}>✕</button>
      </header>

      <p class="hint">
        Budgets are stored locally in your browser and applied to whichever trace is loaded.
        Violations are flagged in the toolbar.
      </p>

      <section class="add-row">
        <select bind:value={newField} aria-label="Budget field">
          {#each BUDGET_FIELDS as f}
            <option value={f.value}>{f.label}</option>
          {/each}
        </select>
        <select bind:value={newOperator} aria-label="Budget operator" class="op-select">
          {#each OPERATORS as o}
            <option value={o.value}>{o.label}</option>
          {/each}
        </select>
        <input
          type="number"
          bind:value={newValue}
          step="any"
          aria-label="Budget value"
          class="value-input"
        />
        <span class="unit">{fieldUnit(newField) || '—'}</span>
        <button type="button" class="add-btn" on:click={addBudget}>Add</button>
      </section>

      <section class="list">
        {#if $budgets.length === 0}
          <div class="empty">No budgets defined yet.</div>
        {:else}
          {#each $budgets as b (b.id)}
            <div class="budget-row" class:violated={isViolated(b.id)}>
              <div class="budget-meta">
                <span class="badge">{BUDGET_FIELDS.find(f => f.value === b.field)?.label ?? b.field}</span>
                <span class="op">{OPERATORS.find(o => o.value === b.operator)?.label ?? b.operator}</span>
                <span class="value">{rawToDisplayValue(b.field, b.value)}</span>
                <span class="unit">{fieldUnit(b.field)}</span>
                {#if isViolated(b.id)}
                  <span class="violated-tag">VIOLATED</span>
                {/if}
              </div>
              <button type="button" class="icon-btn" aria-label="Remove budget" on:click={() => budgets.remove(b.id)}>✕</button>
            </div>
          {/each}
        {/if}
      </section>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(15, 23, 42, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    z-index: 100;
  }

  .dialog {
    background: var(--color-bg, #0f172a);
    color: var(--color-text, #e2e8f0);
    border: 1px solid var(--color-border, #334155);
    border-radius: 10px;
    width: 100%;
    max-width: 560px;
    max-height: calc(100vh - 4rem);
    overflow-y: auto;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.35);
  }

  .dialog-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--color-border, #334155);
  }

  .dialog-header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 700;
  }

  .icon-btn {
    background: transparent;
    border: 0;
    color: var(--color-text-muted, #94a3b8);
    cursor: pointer;
    font-size: 1rem;
    line-height: 1;
    padding: 0.25rem 0.4rem;
    border-radius: 4px;
  }

  .icon-btn:hover {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.06));
    color: var(--color-text, #e2e8f0);
  }

  .hint {
    margin: 0.85rem 1rem;
    font-size: 0.8rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .add-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0 1rem 0.75rem;
    border-bottom: 1px solid var(--color-border, #334155);
    flex-wrap: wrap;
  }

  .add-row select,
  .add-row input {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border: 1px solid var(--color-border, #334155);
    color: var(--color-text, #e2e8f0);
    border-radius: 5px;
    padding: 0.3rem 0.5rem;
    font-size: 0.82rem;
    font-family: inherit;
  }

  .op-select {
    width: 4rem;
    font-family: monospace;
  }

  .value-input {
    width: 7rem;
  }

  .unit {
    font-size: 0.75rem;
    color: var(--color-text-muted, #94a3b8);
    min-width: 2.5rem;
  }

  .add-btn {
    background: var(--color-accent, #3b82f6);
    border: 0;
    color: white;
    padding: 0.32rem 0.7rem;
    border-radius: 5px;
    font-size: 0.8rem;
    font-weight: 600;
    cursor: pointer;
  }

  .add-btn:hover {
    background: var(--color-accent-hover, #2563eb);
  }

  .list {
    padding: 0.5rem 1rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .empty {
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.82rem;
    font-style: italic;
    padding: 0.5rem 0;
  }

  .budget-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    padding: 0.4rem 0.55rem;
  }

  .budget-row.violated {
    border-color: var(--color-danger, #f87171);
    background: rgba(248, 113, 113, 0.10);
  }

  .budget-meta {
    display: flex;
    align-items: center;
    gap: 0.45rem;
    flex-wrap: wrap;
    font-size: 0.82rem;
  }

  .badge {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.2));
    color: var(--color-badge-text, #93c5fd);
    border-radius: 3px;
    padding: 0.1rem 0.4rem;
    font-size: 0.76rem;
    font-weight: 600;
  }

  .op {
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--color-text-muted, #94a3b8);
  }

  .value {
    font-family: monospace;
    font-weight: 600;
  }

  .violated-tag {
    background: var(--color-danger, #f87171);
    color: white;
    font-size: 0.64rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
  }
</style>
