<script lang="ts">
  import type { LlmDetail } from '../lib/types';

  export let llm: LlmDetail;

  type ChatMessage = {
    role: string;
    content: string;
    isFromInput: boolean;
  };

  type RawMessage = { role: string; content: string | null };

  /**
   * OTLP encodes the full chat history as a single JSON string on one event,
   * so a {role:"user", content:"[{...},{...}]"} message frequently expands
   * into N real messages. Try to unpack that here, but never throw — if the
   * content is just plain text, return it unchanged.
   */
  function expandContent(raw: RawMessage, isFromInput: boolean): ChatMessage[] {
    const text = (raw.content ?? '').trim();
    if (!text) return [{ role: raw.role, content: '', isFromInput }];

    if (text.startsWith('[') || text.startsWith('{')) {
      try {
        const parsed = JSON.parse(text);
        if (Array.isArray(parsed)) {
          const out: ChatMessage[] = [];
          for (const item of parsed) {
            if (item && typeof item === 'object' && typeof item.role === 'string') {
              const c = item.content;
              out.push({
                role: item.role,
                content: typeof c === 'string' ? c : JSON.stringify(c, null, 2),
                isFromInput,
              });
            } else if (typeof item === 'string') {
              out.push({ role: raw.role, content: item, isFromInput });
            } else {
              out.push({ role: raw.role, content: JSON.stringify(item, null, 2), isFromInput });
            }
          }
          if (out.length > 0) return out;
        } else if (parsed && typeof parsed === 'object') {
          if (typeof parsed.role === 'string') {
            const c = parsed.content;
            return [{
              role: parsed.role,
              content: typeof c === 'string' ? c : JSON.stringify(c, null, 2),
              isFromInput,
            }];
          }
        }
      } catch {
        // Not JSON — fall through to plain text.
      }
    }

    return [{ role: raw.role, content: text, isFromInput }];
  }

  $: messages = [
    ...llm.input_messages.flatMap((m) => expandContent(m, true)),
    ...llm.output_messages.flatMap((m) => expandContent(m, false)),
  ];

  function bubbleClass(role: string): string {
    const r = role.toLowerCase();
    if (r === 'system') return 'bubble bubble-system';
    if (r === 'user' || r === 'human') return 'bubble bubble-user';
    if (r === 'assistant' || r === 'ai') return 'bubble bubble-assistant';
    if (r === 'tool' || r === 'function') return 'bubble bubble-tool';
    return 'bubble bubble-other';
  }

  function roleLabel(role: string): string {
    const r = role.toLowerCase();
    if (r === 'system') return 'system';
    if (r === 'user' || r === 'human') return 'user';
    if (r === 'assistant' || r === 'ai') return 'assistant';
    if (r === 'tool' || r === 'function') return 'tool';
    return role;
  }

  function tryFormatJson(s: string): string {
    const t = s.trim();
    if (!t.startsWith('{') && !t.startsWith('[')) return s;
    try {
      return JSON.stringify(JSON.parse(t), null, 2);
    } catch {
      return s;
    }
  }
</script>

{#if messages.length === 0 && llm.tool_calls.length === 0}
  <div class="empty">No prompt or completion captured for this span.</div>
{:else}
  <div class="conversation" role="list" aria-label="LLM conversation replay">
    {#each messages as msg, i (i)}
      <div class={bubbleClass(msg.role)} role="listitem">
        <div class="bubble-meta">
          <span class="role-tag">{roleLabel(msg.role)}</span>
          <span class="origin">{msg.isFromInput ? 'prompt' : 'completion'}</span>
        </div>
        <div class="bubble-content">{msg.content}</div>
      </div>
    {/each}

    {#if llm.tool_calls.length > 0}
      <div class="tool-call-group" role="listitem">
        <div class="tool-call-heading">Tool calls ({llm.tool_calls.length})</div>
        {#each llm.tool_calls as tc, i (i)}
          <div class="tc-card">
            <div class="tc-name">→ {tc.name}</div>
            {#if tc.arguments}
              <div class="tc-section-label">arguments</div>
              <pre class="tc-block">{tryFormatJson(tc.arguments)}</pre>
            {/if}
            {#if tc.result}
              <div class="tc-section-label">result</div>
              <pre class="tc-block">{tryFormatJson(tc.result)}</pre>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<style>
  .empty {
    padding: 0.65rem 0;
    color: var(--color-text-muted, #94a3b8);
    font-size: 0.8rem;
    font-style: italic;
  }

  .conversation {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .bubble {
    border-radius: 8px;
    padding: 0.45rem 0.55rem;
    font-size: 0.8rem;
    line-height: 1.45;
    word-break: break-word;
    border: 1px solid transparent;
  }

  .bubble-meta {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .role-tag {
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .origin {
    font-size: 0.65rem;
    color: var(--color-text-muted, #94a3b8);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .bubble-content {
    white-space: pre-wrap;
    font-family: inherit;
    max-height: 320px;
    overflow-y: auto;
  }

  .bubble-system {
    background: var(--color-panel-subtle, rgba(148, 163, 184, 0.08));
    border-color: var(--color-border, #334155);
    color: var(--color-text-muted, #94a3b8);
  }
  .bubble-system .role-tag { color: var(--color-text-muted, #94a3b8); }

  .bubble-user {
    background: var(--color-badge-bg, rgba(59, 130, 246, 0.12));
    border-color: color-mix(in srgb, var(--color-link, #3b82f6) 35%, transparent);
  }
  .bubble-user .role-tag { color: var(--color-link, #93c5fd); }

  .bubble-assistant {
    background: var(--color-llm-panel-bg, rgba(139, 92, 246, 0.10));
    border-color: color-mix(in srgb, var(--color-llm-badge-text, #8b5cf6) 35%, transparent);
  }
  .bubble-assistant .role-tag { color: var(--color-llm-badge-text, #c4b5fd); }

  .bubble-tool {
    background: rgba(34, 197, 94, 0.10);
    border-color: rgba(34, 197, 94, 0.35);
  }
  .bubble-tool .role-tag { color: var(--color-success, #4ade80); }

  .bubble-other {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.04));
    border-color: var(--color-border, #334155);
  }

  .tool-call-group {
    margin-top: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px dashed var(--color-border, #334155);
  }

  .tool-call-heading {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, #94a3b8);
    margin-bottom: 0.3rem;
  }

  .tc-card {
    background: var(--color-panel-subtle, rgba(255, 255, 255, 0.05));
    border-radius: 6px;
    padding: 0.4rem 0.5rem;
    margin-bottom: 0.4rem;
  }

  .tc-name {
    font-weight: 600;
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
    color: var(--color-text, #e2e8f0);
    font-family: monospace;
  }

  .tc-section-label {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-text-muted, #94a3b8);
    margin-top: 0.3rem;
    margin-bottom: 0.15rem;
  }

  .tc-block {
    margin: 0;
    padding: 0.3rem 0.4rem;
    background: var(--color-bg, #0f172a);
    border: 1px solid var(--color-border, #334155);
    border-radius: 4px;
    font-family: monospace;
    font-size: 0.72rem;
    color: var(--color-code-muted, #cbd5e1);
    white-space: pre-wrap;
    max-height: 200px;
    overflow-y: auto;
  }
</style>
