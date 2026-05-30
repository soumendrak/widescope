<script lang="ts">
  import type { RetrievedDocument } from '../lib/types';

  export let documents: RetrievedDocument[];

  let expanded: Record<number, boolean> = {};

  function toggle(i: number) {
    expanded = { ...expanded, [i]: !expanded[i] };
  }

  function scoreClass(score: number | null): string {
    if (score === null) return 'score score--neutral';
    if (score >= 0.7) return 'score score--high';
    if (score >= 0.4) return 'score score--mid';
    return 'score score--low';
  }

  function fmtScore(score: number | null): string {
    return score === null ? '–' : score.toFixed(3);
  }

  function scoreBarWidth(score: number | null): string {
    if (score === null) return '0%';
    const clamped = Math.max(0, Math.min(1, score));
    return `${(clamped * 100).toFixed(1)}%`;
  }
</script>

{#if documents.length > 0}
  <div class="doc-list">
    {#each documents as doc, i (i)}
      <div class="doc">
        <button
          type="button"
          class="doc-header"
          on:click={() => toggle(i)}
          disabled={!doc.content_snippet}
          aria-expanded={expanded[i] ?? false}
        >
          <span class="caret" class:hidden={!doc.content_snippet}>
            {expanded[i] ? '▾' : '▸'}
          </span>
          <span class="rank">#{i + 1}</span>
          <span class="id">{doc.id ?? 'doc'}</span>
          <span class={scoreClass(doc.score)}>{fmtScore(doc.score)}</span>
        </button>
        {#if doc.score !== null}
          <div class="bar-track" aria-hidden="true">
            <div class={scoreClass(doc.score) + ' bar-fill'} style={`width: ${scoreBarWidth(doc.score)};`}></div>
          </div>
        {/if}
        {#if expanded[i] && doc.content_snippet}
          <div class="snippet">{doc.content_snippet}</div>
        {/if}
      </div>
    {/each}
  </div>
{/if}

<style>
  .doc-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .doc {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border: 1px solid var(--color-border, #334155);
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
  }

  .doc-header {
    width: 100%;
    background: none;
    border: 0;
    padding: 0;
    color: inherit;
    font: inherit;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    cursor: pointer;
    text-align: left;
  }

  .doc-header:disabled {
    cursor: default;
  }

  .doc-header:disabled .caret {
    color: transparent;
  }

  .caret {
    font-size: 0.7rem;
    color: var(--color-text-muted, #94a3b8);
    width: 0.7rem;
  }

  .caret.hidden {
    visibility: hidden;
  }

  .rank {
    font-family: monospace;
    font-size: 0.72rem;
    color: var(--color-text-muted, #94a3b8);
    min-width: 1.6rem;
  }

  .id {
    flex: 1;
    font-family: monospace;
    font-size: 0.78rem;
    color: var(--color-text, #e2e8f0);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .score {
    font-family: monospace;
    font-size: 0.78rem;
    font-weight: 600;
    padding: 0.05rem 0.4rem;
    border-radius: 3px;
  }

  .score--high {
    background: rgba(34, 197, 94, 0.18);
    color: var(--color-success, #4ade80);
  }

  .score--mid {
    background: rgba(251, 191, 36, 0.18);
    color: #fbbf24;
  }

  .score--low {
    background: rgba(248, 113, 113, 0.18);
    color: var(--color-danger, #f87171);
  }

  .score--neutral {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    color: var(--color-text-muted, #94a3b8);
  }

  .bar-track {
    margin-top: 0.3rem;
    height: 4px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 2px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    padding: 0;
  }

  .bar-fill.score--high { background: var(--color-success, #4ade80); }
  .bar-fill.score--mid { background: #fbbf24; }
  .bar-fill.score--low { background: var(--color-danger, #f87171); }
  .bar-fill.score--neutral { background: var(--color-link, #60a5fa); }

  .snippet {
    margin-top: 0.4rem;
    padding: 0.35rem 0.45rem;
    background: var(--color-bg, #0f172a);
    border-radius: 4px;
    font-size: 0.74rem;
    color: var(--color-code-muted, #cbd5e1);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 240px;
    overflow-y: auto;
  }
</style>
