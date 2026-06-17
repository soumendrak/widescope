/** Message content split into renderable pieces: prose and fenced code blocks. */
export type Segment =
  | { kind: 'text'; text: string }
  | { kind: 'code'; lang: string | null; text: string };

// ```lang\n ... ``` — language is optional, body is non-greedy.
const FENCE = /```([^\n`]*)\n([\s\S]*?)```/g;

/**
 * Split chat message content into ordered text/code segments so the replay
 * view can render fenced code blocks distinctly. Unterminated fences stay as
 * text (the regex simply won't match them).
 */
export function segmentContent(content: string | null): Segment[] {
  if (!content) return [];
  const segments: Segment[] = [];
  let last = 0;
  for (const m of content.matchAll(FENCE)) {
    const start = m.index ?? 0;
    pushText(segments, content.slice(last, start));
    const lang = m[1].trim();
    segments.push({ kind: 'code', lang: lang || null, text: m[2].replace(/\n$/, '') });
    last = start + m[0].length;
  }
  pushText(segments, content.slice(last));
  return segments;
}

function pushText(segments: Segment[], raw: string): void {
  const text = raw.trim();
  if (text) segments.push({ kind: 'text', text });
}
