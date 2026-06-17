import { describe, it, expect } from 'vitest';
import { segmentContent } from './chat';

describe('segmentContent', () => {
  it('returns nothing for empty/null content', () => {
    expect(segmentContent(null)).toEqual([]);
    expect(segmentContent('')).toEqual([]);
    expect(segmentContent('   \n  ')).toEqual([]);
  });

  it('returns a single text segment for plain prose', () => {
    expect(segmentContent('hello there')).toEqual([{ kind: 'text', text: 'hello there' }]);
  });

  it('captures a fenced code block with its language', () => {
    expect(segmentContent('```python\nprint(1)\n```')).toEqual([
      { kind: 'code', lang: 'python', text: 'print(1)' },
    ]);
  });

  it('captures a fenced block with no language as null', () => {
    expect(segmentContent('```\nx = 1\n```')).toEqual([
      { kind: 'code', lang: null, text: 'x = 1' },
    ]);
  });

  it('interleaves text and code in order', () => {
    expect(segmentContent('before\n```js\nlet a = 2;\n```\nafter')).toEqual([
      { kind: 'text', text: 'before' },
      { kind: 'code', lang: 'js', text: 'let a = 2;' },
      { kind: 'text', text: 'after' },
    ]);
  });

  it('preserves internal newlines and indentation in code', () => {
    const [seg] = segmentContent('```\nif (x):\n    y = 1\n```');
    expect(seg).toEqual({ kind: 'code', lang: null, text: 'if (x):\n    y = 1' });
  });

  it('leaves an unterminated fence as text', () => {
    const segs = segmentContent('```js\nnever closed');
    expect(segs).toEqual([{ kind: 'text', text: '```js\nnever closed' }]);
  });
});
