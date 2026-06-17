<script lang="ts">
  import type { LlmDetail } from '../lib/types';
  import { segmentContent } from '../lib/chat';

  export let llm: LlmDetail;

  // Normalise an arbitrary role string to a known bubble style.
  function roleClass(role: string): string {
    const r = role.toLowerCase();
    if (r.includes('system')) return 'system';
    if (r.includes('assistant') || r.includes('ai') || r.includes('model')) return 'assistant';
    if (r.includes('tool') || r.includes('function')) return 'tool';
    if (r.includes('user') || r.includes('human')) return 'user';
    return 'other';
  }

  $: hasMessages =
    llm.input_messages.length > 0 ||
    llm.output_messages.length > 0 ||
    llm.tool_calls.length > 0;
</script>

{#if hasMessages}
  <div class="thread">
    {#if llm.input_messages.length > 0}
      <div class="turn-label">Prompt</div>
      {#each llm.input_messages as m}
        <div class="bubble {roleClass(m.role)}">
          <div class="bubble-role">{m.role}</div>
          <div class="bubble-body">
            {#each segmentContent(m.content) as seg}
              {#if seg.kind === 'code'}
                <pre class="code"><span class="code-lang">{seg.lang ?? 'code'}</span><code>{seg.text}</code></pre>
              {:else}
                <p class="text">{seg.text}</p>
              {/if}
            {/each}
          </div>
        </div>
      {/each}
    {/if}

    {#if llm.output_messages.length > 0 || llm.tool_calls.length > 0}
      <div class="turn-label">Completion</div>
      {#each llm.output_messages as m}
        <div class="bubble {roleClass(m.role)}">
          <div class="bubble-role">{m.role}</div>
          <div class="bubble-body">
            {#each segmentContent(m.content) as seg}
              {#if seg.kind === 'code'}
                <pre class="code"><span class="code-lang">{seg.lang ?? 'code'}</span><code>{seg.text}</code></pre>
              {:else}
                <p class="text">{seg.text}</p>
              {/if}
            {/each}
          </div>
        </div>
      {/each}

      {#each llm.tool_calls as tc}
        <details class="bubble tool tool-call" open>
          <summary>
            <span class="caret" aria-hidden="true">▸</span>
            <span class="tool-tag">tool call</span>
            <span class="tool-name">{tc.name}</span>
          </summary>
          {#if tc.arguments}
            <div class="tool-part">
              <div class="tool-part-label">arguments</div>
              <pre class="code"><code>{tc.arguments}</code></pre>
            </div>
          {/if}
          {#if tc.result}
            <div class="tool-part">
              <div class="tool-part-label">result</div>
              <pre class="code"><code>{tc.result}</code></pre>
            </div>
          {/if}
        </details>
      {/each}
    {/if}
  </div>
{:else}
  <div class="empty">No messages captured for this span.</div>
{/if}

<style>
  .thread {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .turn-label {
    font-family: var(--font-mono);
    font-size: 0.58rem;
    text-transform: uppercase;
    letter-spacing: 0.16em;
    color: var(--color-text-faint, #5b6b84);
    margin-top: 0.4rem;
  }

  .turn-label:first-child { margin-top: 0; }

  .bubble {
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-left-width: 2px;
    border-radius: 8px;
    padding: 0.4rem 0.55rem;
    background: color-mix(in srgb, var(--color-canvas-bg, #070c16) 55%, transparent);
  }

  .bubble.user { border-left-color: var(--color-sky, #7dd3fc); }
  .bubble.assistant { border-left-color: var(--color-amber, #f59e0b); }
  .bubble.system { border-left-color: var(--color-text-muted, #94a3b8); border-left-style: dashed; }
  .bubble.tool { border-left-color: var(--color-violet, #a78bfa); }
  .bubble.other { border-left-color: var(--color-border, rgba(125, 211, 252, 0.13)); }

  .bubble-role {
    font-family: var(--font-mono);
    font-size: 0.6rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    margin-bottom: 0.25rem;
    color: var(--color-text-muted, #9aa8bd);
  }

  .bubble.user .bubble-role { color: var(--color-sky, #7dd3fc); }
  .bubble.assistant .bubble-role { color: var(--color-gold, #fcd34d); }
  .bubble.tool .bubble-role { color: var(--color-violet, #a78bfa); }

  .text {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 0.74rem;
    line-height: 1.6;
    color: var(--color-code-muted, #b9c5d8);
    white-space: pre-wrap;
    word-break: break-word;
  }

  .text + .code,
  .code + .text { margin-top: 0.4rem; }

  .code {
    position: relative;
    margin: 0;
    padding: 0.5rem 0.55rem;
    background: var(--color-canvas-bg, #070c16);
    border: 1px solid var(--color-border-soft, rgba(125, 211, 252, 0.07));
    border-radius: 6px;
    overflow-x: auto;
  }

  .code code {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    line-height: 1.5;
    color: var(--color-code-text, #e9eff8);
    white-space: pre;
  }

  .code-lang {
    position: absolute;
    top: 0;
    right: 0;
    padding: 0.05rem 0.4rem;
    border-bottom-left-radius: 6px;
    font-family: var(--font-mono);
    font-size: 0.56rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--color-text-faint, #5b6b84);
    background: var(--color-panel-subtle, rgba(125, 211, 252, 0.06));
  }

  .tool-call summary {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    cursor: pointer;
    list-style: none;
    user-select: none;
  }

  .tool-call summary::-webkit-details-marker { display: none; }

  .caret {
    font-size: 0.65rem;
    color: var(--color-text-muted, #94a3b8);
    transition: transform 0.12s ease;
  }

  .tool-call[open] .caret { transform: rotate(90deg); }

  .tool-tag {
    font-family: var(--font-mono);
    font-size: 0.56rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--color-violet, #a78bfa);
  }

  .tool-name {
    font-family: var(--font-mono);
    font-size: 0.74rem;
    font-weight: 600;
    color: var(--color-text, #e9eff8);
  }

  .tool-part { margin-top: 0.4rem; }

  .tool-part-label {
    font-family: var(--font-mono);
    font-size: 0.56rem;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    color: var(--color-text-faint, #5b6b84);
    margin-bottom: 0.2rem;
  }

  .empty {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    color: var(--color-text-muted, #9aa8bd);
    padding: 0.4rem 0;
  }
</style>
