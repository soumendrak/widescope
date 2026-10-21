import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { commentLines } from './comments.mjs';

const repoRoot = resolve(import.meta.dirname, '../..');
const lines = (file, text) => [...commentLines(file, text, repoRoot)].sort((a, b) => a - b);
const SLASHES = '/' + '/';
const OPEN = '/' + '*';
const CLOSE = '*' + '/';
const HTML_OPEN = '<!-' + '-';
const HTML_CLOSE = '-' + '->';

test('rust: an odd number of lifetimes does not hide a trailing comment', () => {
  assert.deepEqual(lines('a.rs', `pub type MutRef<'a> = (&'a str, &'a str); ${SLASHES} it's hidden\n`), [1]);
});

test('rust: char literals, including quotes and escapes, are not comments', () => {
  assert.deepEqual(lines('a.rs', `let a = '"';\nlet b = '\\'';\nlet c = '/';\nlet d = '\\u{1F600}';\nlet e = '😀';\nfn f<'a>(x: &'a str) -> &'a str { x }\n`), []);
  assert.deepEqual(lines('a.rs', `let q = '\\''; ${SLASHES} why\n`), [1]);
});

test('rust: strings and raw strings may contain comment delimiters', () => {
  const text = [
    `let u = "https://example.com";`,
    `let j = r#"{"url":"https://example.com"}"#;`,
    `let k = r##"a "# ${SLASHES} b"##;`,
    `let m = b"${OPEN} not a comment ${CLOSE}";`,
    `let n = br"${SLASHES}x";`,
    '',
  ].join('\n');
  assert.deepEqual(lines('a.rs', text), []);
});

test('rust: every line of a block comment counts, including nested blocks', () => {
  const text = `fn a() {}\n${OPEN} outer\n  ${OPEN} inner ${CLOSE}\n  added prose\n${CLOSE}\nfn b() {}\n`;
  assert.deepEqual(lines('a.rs', text), [2, 3, 4, 5]);
});

test('css: a line added inside an existing block comment is a comment line', () => {
  const text = `p { color: red; }\n${OPEN}\n  existing\n  added prose\n${CLOSE}\na { background: url(${SLASHES}cdn.example/x.png); content: "${OPEN}"; }\n`;
  assert.deepEqual(lines('a.css', text), [2, 3, 4, 5]);
});

test('ts: regexes, URLs and template text are not comments; interpolations and blocks are', () => {
  const text = [
    `const k = name.replace(/\\//g, "_");`,
    `const f = () => /\\//.test(x);`,
    `const u = "https://x";`,
    `const t = \`a ${SLASHES} b \${c}\`;`,
    `const v = \`\${value ${SLASHES} note`,
    `}\`;`,
    `${OPEN}`,
    `  added prose`,
    `${CLOSE}`,
    '',
  ].join('\n');
  assert.deepEqual(lines('a.ts', text), [5, 7, 8, 9]);
});

test('svelte: script, style and markup comments are found; strings are not', () => {
  const text = [
    '<script lang="ts">',
    '  const u = "https://x";',
    `  let n = 1; ${SLASHES} why`,
    '</script>',
    `${HTML_OPEN} markup`,
    `  note ${HTML_CLOSE}`,
    '<p>{u}</p>',
    '<style>',
    `  p { color: red; } ${OPEN} style ${CLOSE}`,
    '</style>',
    '',
  ].join('\n');
  assert.deepEqual(lines('a.svelte', text), [3, 5, 6, 9]);
});

test('svelte: comments inside markup expressions are found; braces in strings and templates are not', () => {
  const text = [
    `<span>{${OPEN} hidden ${CLOSE} "x"}</span>`,
    '<p title={`a ${"}"} b`}>{"{"}</p>',
    `<p>{n ${SLASHES} why`,
    '}</p>',
    `{#if ok}<b>{${OPEN} a ${CLOSE} n}</b>{/if}`,
    `<i>don't ${OPEN} text ${CLOSE}</i>`,
    `<a href="https:${SLASHES}x">{fn({ a: 1 })}</a>`,
    `<p>{/[}]/.test(x) ${OPEN} after regex ${CLOSE}}</p>`,
    '<p>{/https?:\\/\\//.test(u) ? a / b : c}</p>',
    '{#each xs as x}{x}{/each}',
    `<p>see https:${SLASHES}y</p>`,
    '',
  ].join('\n');
  assert.deepEqual(lines('a.svelte', text), [1, 3, 5, 8]);
});
