#!/usr/bin/env node
import { execFileSync, spawnSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { isAbsolute, relative, resolve } from 'node:path';
import { commentLines } from './standards/comments.mjs';

const RUST_COVERAGE_MIN = '98';
const RUST_LCOV = 'target/llvm-cov.lcov';
const UI_LCOV = 'ui/coverage/lcov.info';
const COMMENT_CHECKED = /\.(rs|ts|js|mjs|svelte|css)$/;
const UI_UNINSTRUMENTED = new Set(['ui/src/main.ts', 'ui/src/lib/types.ts', 'ui/src/components/FlameGraph.svelte']);
const UI_TEST_FILE = /^ui\/src\/(.*\.test\.ts|test-setup\.ts)$/;
const COVERAGE_CHECKED = /^(crates\/.*\.rs|ui\/src\/.*\.(ts|svelte))$/;

const root = git('rev-parse', '--show-toplevel');
process.chdir(root);
const baseIndex = process.argv.indexOf('--base');
const base = baseIndex > 0 ? process.argv[baseIndex + 1] : git('merge-base', 'origin/main', 'HEAD');

const steps = [
  ['standards self-test', () => run('node', ['--test', ...readdirSync('scripts/standards').filter((name) => name.endsWith('.test.mjs')).map((name) => `scripts/standards/${name}`)])],
  ['rustfmt', () => run('cargo', ['fmt', '--all', '--check'])],
  ['clippy', () => run('cargo', ['clippy', '--workspace', '--all-targets', '--', '-D', 'warnings'])],
  ['no new comments', checkNoNewComments],
  ['rust tests and coverage', () => run('cargo', ['llvm-cov', '--workspace', '--lcov', '--output-path', RUST_LCOV, '--fail-under-lines', RUST_COVERAGE_MIN])],
  ['wasm build', () => run('wasm-pack', ['build', 'crates/widescope-core', '--target', 'web', '--out-dir', 'pkg'])],
  ['svelte-check', () => run('npx', ['svelte-check', '--tsconfig', './tsconfig.json', '--threshold', 'error'], 'ui')],
  ['ui tests and coverage', () => run('npx', ['vitest', 'run', '--coverage'], 'ui')],
  ['changed-line coverage', checkChangedLineCoverage],
];

console.log(`standards: diffing against ${base}`);
for (const [name, step] of steps) {
  console.log(`\n== ${name}`);
  if (!step()) {
    console.log(`\nSTANDARDS FAIL: ${name}`);
    process.exit(1);
  }
}
console.log('\nSTANDARDS OK');

function git(...args) {
  return execFileSync('git', args, { encoding: 'utf8', maxBuffer: 1 << 30 }).trim();
}

function run(command, args, cwd = '.') {
  return spawnSync(command, args, { cwd, stdio: 'inherit' }).status === 0;
}

function addedLines() {
  const files = new Map();
  let current = null;
  let lineNumber = 0;
  let inHeader = false;
  for (const line of git('-c', 'core.quotePath=false', 'diff', '-U0', '--no-color', '--no-ext-diff', base).split('\n')) {
    if (line.startsWith('diff --git ')) {
      inHeader = true;
      current = null;
      continue;
    }
    if (inHeader && line.startsWith('+++ ')) {
      current = line === '+++ /dev/null' ? null : unquote(line.slice(4)).slice(2);
      if (current && !files.has(current)) files.set(current, new Map());
      continue;
    }
    const hunk = line.match(/^@@ -\S+ \+(\d+)/);
    if (hunk) {
      inHeader = false;
      lineNumber = Number(hunk[1]);
      continue;
    }
    if (current && line.startsWith('+')) files.get(current).set(lineNumber++, line.slice(1));
  }
  for (const file of git('-c', 'core.quotePath=false', 'ls-files', '--others', '--exclude-standard').split('\n').filter(Boolean)) {
    const lines = readFileSync(file, 'utf8').split('\n');
    files.set(file, new Map(lines.map((text, index) => [index + 1, text])));
  }
  return files;
}

function unquote(path) {
  return path.startsWith('"') ? JSON.parse(path) : path;
}

function checkNoNewComments() {
  const found = [];
  for (const [file, lines] of addedLines()) {
    if (!COMMENT_CHECKED.test(file)) continue;
    const comments = commentLines(file, readFileSync(file, 'utf8'), root);
    for (const [number, text] of lines) {
      if (comments.has(number)) found.push(`${file}:${number}: ${text.trim()}`);
    }
  }
  for (const entry of found) console.log(entry);
  if (found.length) console.log('Remove these comments. Say it with names, types and tests instead.');
  return found.length === 0;
}

function readLcov(path, sourceRoot) {
  const hits = new Map();
  let file = null;
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    if (line.startsWith('SF:')) {
      const source = line.slice(3);
      file = relative(root, isAbsolute(source) ? source : resolve(sourceRoot, source));
      hits.set(file, new Map());
    } else if (line.startsWith('DA:') && file) {
      const [number, count] = line.slice(3).split(',').map(Number);
      hits.get(file).set(number, Math.max(count, hits.get(file).get(number) ?? 0));
    }
  }
  return hits;
}

function checkChangedLineCoverage() {
  const hits = new Map([...readLcov(RUST_LCOV, root), ...readLcov(UI_LCOV, resolve(root, 'ui'))]);
  const uncovered = [];
  const unrecorded = git('-c', 'core.quotePath=false', 'ls-files', '--cached', '--others', '--exclude-standard', 'ui/src')
    .split('\n')
    .filter((file) => /\.(ts|svelte)$/.test(file) && !UI_TEST_FILE.test(file) && !UI_UNINSTRUMENTED.has(file) && !hits.has(file));
  let checked = 0;
  for (const [file, lines] of addedLines()) {
    if (!COVERAGE_CHECKED.test(file)) continue;
    const fileHits = hits.get(file);
    if (UI_UNINSTRUMENTED.has(file)) console.log(`not instrumented, prove it in the browser: ${file}`);
    for (const number of lines.keys()) {
      const count = fileHits?.get(number);
      if (count === undefined) continue;
      checked++;
      if (count === 0) uncovered.push(`${file}:${number}`);
    }
  }
  for (const entry of uncovered) console.log(`not covered: ${entry}`);
  for (const file of unrecorded) console.log(`no coverage record: ${file}. Remove it from the coverage exclusions in ui/vite.config.ts, or add it to UI_UNINSTRUMENTED here and prove it in the browser.`);
  console.log(`${checked - uncovered.length}/${checked} changed executable lines covered`);
  return uncovered.length === 0 && unrecorded.length === 0;
}
