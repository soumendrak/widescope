import { createRequire } from 'node:module';
import { resolve } from 'node:path';

const SVELTE_BLOCK = /(<(script|style)[^>]*>)([\s\S]*?)<\/\2>/g;
const HTML_COMMENT_OPEN = '<!-' + '-';
const HTML_COMMENT_CLOSE = '-' + '->';

export function commentLines(file, text, repoRoot) {
  if (/\.(ts|js|mjs)$/.test(file)) return linesOf(text, scriptCommentRanges(text, repoRoot));
  if (file.endsWith('.rs')) return linesOf(text, rustCommentRanges(text));
  if (file.endsWith('.css')) return linesOf(text, cssCommentRanges(text));
  if (file.endsWith('.svelte')) return linesOf(text, svelteCommentRanges(text, repoRoot));
  return new Set();
}

function linesOf(text, ranges) {
  const lines = new Set();
  for (const [start, end] of ranges) {
    const first = lineAt(text, start);
    const last = lineAt(text, Math.max(start, end - 1));
    for (let line = first; line <= last; line++) lines.add(line);
  }
  return lines;
}

function lineAt(text, offset) {
  let line = 1;
  for (let index = text.indexOf('\n'); index !== -1 && index < offset; index = text.indexOf('\n', index + 1)) line++;
  return line;
}

function typescript(repoRoot) {
  return createRequire(resolve(repoRoot, 'ui/package.json'))('typescript');
}

function markupExpressionEnd(ts, text, start, ranges) {
  const { SyntaxKind: K } = ts;
  const operandEnd = new Set([K.Identifier, K.PrivateIdentifier, K.NumericLiteral, K.BigIntLiteral, K.StringLiteral, K.RegularExpressionLiteral, K.NoSubstitutionTemplateLiteral, K.TemplateTail, K.CloseParenToken, K.CloseBracketToken, K.CloseBraceToken, K.ThisKeyword, K.SuperKeyword, K.TrueKeyword, K.FalseKeyword, K.NullKeyword, K.PlusPlusToken, K.MinusMinusToken]);
  const scanner = ts.createScanner(ts.ScriptTarget.Latest, false, ts.LanguageVariant.Standard, text);
  scanner.resetTokenState(start);
  const open = [];
  let previous;
  for (let kind = scanner.scan(); kind !== K.EndOfFileToken; kind = scanner.scan()) {
    if (kind === K.SingleLineCommentTrivia || kind === K.MultiLineCommentTrivia) {
      ranges.push([scanner.getTokenStart(), scanner.getTokenEnd()]);
      continue;
    }
    if (kind === K.NewLineTrivia || kind === K.WhitespaceTrivia) continue;
    if ((kind === K.SlashToken || kind === K.SlashEqualsToken) && !operandEnd.has(previous)) kind = scanner.reScanSlashToken();
    else if (kind === K.OpenBraceToken || kind === K.TemplateHead) open.push(kind);
    else if (kind === K.CloseBraceToken) {
      if (open.length === 0) return scanner.getTokenEnd();
      if (open.pop() === K.TemplateHead && scanner.reScanTemplateToken(false) === K.TemplateMiddle) open.push(K.TemplateHead);
    }
    previous = kind;
  }
  return text.length;
}

function scriptCommentRanges(text, repoRoot) {
  const ts = typescript(repoRoot);
  const source = ts.createSourceFile('file.ts', text, ts.ScriptTarget.Latest, true);
  const ranges = [];
  const add = (found) => {
    for (const range of found ?? []) ranges.push([range.pos, range.end]);
  };
  const visit = (node) => {
    add(ts.getLeadingCommentRanges(text, node.pos));
    add(ts.getTrailingCommentRanges(text, node.end));
    for (const child of node.getChildren(source)) visit(child);
  };
  visit(source);
  return ranges;
}

export function rustCommentRanges(text) {
  const ranges = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    const next = text[i + 1];
    if (c === '/' && next === '/') {
      const end = text.indexOf('\n', i);
      ranges.push([i, end === -1 ? text.length : end]);
      i = end === -1 ? text.length : end;
    } else if (c === '/' && next === '*') {
      const start = i;
      let depth = 0;
      while (i < text.length) {
        if (text[i] === '/' && text[i + 1] === '*') {
          depth++;
          i += 2;
        } else if (text[i] === '*' && text[i + 1] === '/') {
          depth--;
          i += 2;
          if (depth === 0) break;
        } else i++;
      }
      ranges.push([start, i]);
    } else if (rawStringAt(text, i)) {
      i = rawStringAt(text, i);
    } else if (c === '"' || (c === 'b' && next === '"' && !identifierBefore(text, i))) {
      i = text.indexOf('"', i) + 1;
      while (i < text.length && text[i] !== '"') i += text[i] === '\\' ? 2 : 1;
      i++;
    } else if (c === "'") {
      i = charLiteralEnd(text, i) ?? i + 1;
    } else i++;
  }
  return ranges;
}

function identifierBefore(text, i) {
  return i > 0 && /[\w]/.test(text[i - 1]);
}

function rawStringAt(text, i) {
  const match = /^b?r(#*)"/.exec(text.slice(i, i + 260));
  if (!match || identifierBefore(text, i)) return 0;
  const close = `"${match[1]}`;
  const end = text.indexOf(close, i + match[0].length);
  return end === -1 ? text.length : end + close.length;
}

function charLiteralEnd(text, i) {
  if (text[i + 1] === '\\') {
    const end = text.indexOf("'", i + 3);
    return end === -1 ? undefined : end + 1;
  }
  const codePoint = text.codePointAt(i + 1);
  if (codePoint === undefined || text[i + 1] === '\n') return undefined;
  const width = codePoint > 0xffff ? 2 : 1;
  return text[i + 1 + width] === "'" ? i + 2 + width : undefined;
}

export function cssCommentRanges(text) {
  const ranges = [];
  let i = 0;
  while (i < text.length) {
    const c = text[i];
    if (c === '/' && text[i + 1] === '*') {
      const end = text.indexOf('*/', i + 2);
      const stop = end === -1 ? text.length : end + 2;
      ranges.push([i, stop]);
      i = stop;
    } else if (c === '"' || c === "'") {
      i++;
      while (i < text.length && text[i] !== c && text[i] !== '\n') i += text[i] === '\\' ? 2 : 1;
      i++;
    } else i++;
  }
  return ranges;
}

function svelteCommentRanges(text, repoRoot) {
  const ranges = [];
  const blocks = [];
  for (const match of text.matchAll(SVELTE_BLOCK)) {
    const start = match.index + match[1].length;
    const inner = match[3];
    const found = match[2] === 'script' ? scriptCommentRanges(inner, repoRoot) : cssCommentRanges(inner);
    for (const [from, to] of found) ranges.push([start + from, start + to]);
    blocks.push([match.index, match.index + match[0].length]);
  }
  const ts = typescript(repoRoot);
  let i = 0;
  while (i < text.length) {
    const block = blocks.find(([from, to]) => i >= from && i < to);
    if (block) i = block[1];
    else if (text.startsWith(HTML_COMMENT_OPEN, i)) {
      const end = text.indexOf(HTML_COMMENT_CLOSE, i + 4);
      const stop = end === -1 ? text.length : end + 3;
      ranges.push([i, stop]);
      i = stop;
    } else if (text[i] === '{') i = markupExpressionEnd(ts, text, i + 1, ranges);
    else i++;
  }
  return ranges;
}
