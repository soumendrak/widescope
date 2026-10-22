import { describe, expect, it } from 'vitest';
import { formatTraceText } from './formatTrace';

describe('formatTraceText', () => {
  it('pretty-prints a single JSON document', () => {
    expect(formatTraceText('{"a":[1]}')).toBe('{\n  "a": [\n    1\n  ]\n}');
  });

  it('keeps newline-delimited records one per line and drops blank lines', () => {
    expect(formatTraceText('{ "a": 1 }\r\n\n{"b" :2}\n')).toBe('{"a":1}\n{"b":2}');
  });

  it('rejects input where any record is not JSON', () => {
    expect(() => formatTraceText('{"a":1}\n{"b":')).toThrow(SyntaxError);
    expect(() => formatTraceText('{"a":')).toThrow(SyntaxError);
  });
});
