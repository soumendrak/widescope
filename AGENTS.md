# Working in WideScope

Instructions for coding agents and people. [CONTRIBUTING.md](CONTRIBUTING.md) covers the architecture and how to add parsers, views and conventions. This file is what you need before your first commit.

## Setup

You need Rust (rustup) with the `wasm32-unknown-unknown` target, `wasm-pack`, `cargo-llvm-cov`, Node.js 20.19+, 22.13+ or 24+, [just](https://just.systems), and Playwright Chromium. CI pins `just@1.59.0`, `cargo-llvm-cov@0.9.1` and `wasm-pack@0.13.1` (`taiki-e/install-action` in `.github/workflows/ci.yml`) and uses Node 24 and stable Rust. Install the same versions.

```bash
rustup target add wasm32-unknown-unknown
rustup component add clippy rustfmt llvm-tools-preview
cargo install just@1.59.0 wasm-pack@0.13.1 cargo-llvm-cov@0.9.1 --locked
just ui-install
(cd ui && npx playwright install --with-deps chromium)
just doctor
```

`just doctor` ends with `DOCTOR OK` or names the missing piece and how to install it.

## Commands

| Task | Command |
| --- | --- |
| Every standard, the same gate CI runs | `just check` |
| Rust and UI unit tests | `just test` |
| Playwright suite, the same steps as CI (`CI=1`, so it never reuses a server already on port 5173) | `just e2e` |
| Toolchain check | `just doctor` |
| Build WASM, then the UI into `ui/dist/` | `just build` |
| Dev server on `http://localhost:5173/editor/` | `just build-wasm && just dev` |
| CLI summary of a trace file | `cargo run -q -p widescope-cli -- analyze <file>` |

`just check` runs `scripts/standards.mjs` against `git merge-base origin/main HEAD`. CI passes the PR's base SHA. The gate runs rustfmt, clippy with `-D warnings`, the no-new-comments check, Rust tests under `cargo llvm-cov` with a 98% line floor, the WASM build, `svelte-check`, the vitest suite with the thresholds in `ui/vite.config.ts`, and 100% coverage of changed executable lines. It ends with `STANDARDS OK` or `STANDARDS FAIL: <step>`.

Before you open or update a PR, prove the change with [`.agents/skills/verify-widescope`](.agents/skills/verify-widescope/SKILL.md). It serves the built viewer and loads trace files through the real "Open file" button:

- `launch.sh <port>` holds a per-port `flock` while it builds, then serves `ui/dist` with `vite preview` on `127.0.0.1:<port>`. It refuses a port that is taken or that another launch is still starting. Before building, it creates `Cargo.lock` if it is missing and snapshots the build inputs (`inputs.sh`: every tracked or untracked file except `.agents/`, `.github/`, Markdown and tests, plus the gitignored `Cargo.lock`).
- `doctor.sh <port>` checks four things: the port belongs to the process group launch.sh started, `/editor/` answers within 5s, the WASM in `ui/dist` matches `crates/widescope-core/pkg`, and no build input was added, deleted or changed since the snapshot. Drive only after `DOCTOR OK`.
- `drive.mjs` fails on an unmet expectation, a console error or uncaught exception, or a file the picker's `accept` list would reject. It writes evidence to `/tmp/verify-widescope/<port>/`, named after the file's path plus a hash.
- `stop.sh <port>` kills the preview only if its process group still owns the port.

## Layout

- `crates/widescope-core/`: the Rust core, compiled to WASM. `src/lib.rs` is the API the UI calls. `src/parsers/` detects and reads OTLP, Jaeger and OpenInference JSON. `src/trace_builder.rs` assembles spans into a trace. `src/share.rs` encodes share links.
- `crates/widescope-cli/`: the `widescope` CLI (`analyze`, `compare`, `check`) on top of the same core.
- `ui/`: the Svelte 5 + Vite app. `/` is the landing page (`ui/index.html`) and `/editor/` is the viewer (`ui/editor/index.html`). `ui/src/lib/wasm.ts` imports `crates/widescope-core/pkg/`. `ui/e2e/` holds the Playwright specs.
- `conventions/`: attribute conventions and pricing, bundled into the build.
- `test-fixtures/`: sample traces per format. The OTLP, Jaeger and OpenInference samples each have 7 spans.
- `vscode-extension/`: opens a trace in the hosted viewer. `.github/actions/trace-compare/`: the trace-compare GitHub Action.

## Rules and what enforces them

| Rule | Enforced by |
| --- | --- |
| rustfmt formatting, no clippy warnings in any target (`--all-targets`) | `just check` |
| No new comments in `.rs`, `.ts`, `.js`, `.mjs`, `.svelte` or `.css` files. Say it with names, types and tests. | `just check`. `scripts/standards/comments.mjs` finds comment ranges over the whole file. `.ts`, `.js` and `.mjs` use the TypeScript parser. `.rs` uses a lexer that handles lifetimes, char literals, nested blocks and raw strings, and `.css` uses a lexer that skips strings. `.svelte` uses Svelte's own parser (`svelte/compiler`), which gives every JS/TS comment in `<script>` and in markup expressions and attributes, plus HTML comments and every `<style>` element, including nested ones. Every line inside a comment counts, including lines added to an existing block. `scripts/standards/comments.test.mjs` runs first in the gate. |
| Rust line coverage stays at or above `RUST_COVERAGE_MIN` (98) in `scripts/standards.mjs`, and the UI meets the thresholds in `ui/vite.config.ts` | `just check` |
| Those thresholds only go up. Raise them when coverage rises and never lower one. | review, until [#120](https://github.com/soumendrak/widescope/issues/120) adds a check |
| Every changed executable line in `.rs` files under `crates/` and in `.ts` and `.svelte` files under `ui/src/` is covered. A `.js` file under `ui/src/` isn't instrumented or checked, so write UI code in TypeScript. Every `ui/src` `.ts` and `.svelte` file, changed or not, must have a coverage record, so a new entry in the coverage exclusions of `ui/vite.config.ts` fails. The exceptions are tests and the files in `UI_UNINSTRUMENTED` in `scripts/standards.mjs` (`main.ts`, `lib/types.ts`, `FlameGraph.svelte`), which the gate prints for a browser proof. Rust code compiled only for `wasm32` (`#[cfg(target_arch = "wasm32")]`, as in `errors.rs`) has no coverage record and is skipped. `ui/public/`, `.github/actions/`, `vscode-extension/` and `scripts/` are not instrumented, and e2e only opens `/editor/` ([#120](https://github.com/soumendrak/widescope/issues/120)). | `just check`, then the browser proof for listed files |
| No type errors in `ui/src` (`ui/tsconfig.json` includes only `src/`, so `ui/public/*.js` is unchecked) | `svelte-check` in `just check` |
| The viewer loads and every e2e journey passes | `just e2e` in CI |
| The UI uses npm. Change dependencies through `ui/package-lock.json` only. | `.gitignore` ignores `ui/pnpm-lock.yaml` and `ui/yarn.lock` |
| Keep a PR under 400 changed lines, not counting tests, generated files and lockfiles | review |
| Don't deploy. Cloudflare's Git build deploys `main` with a build command set in the Cloudflare dashboard, not the one in `wrangler.jsonc`. If you change either one, change the other too and say so in the PR (README, "Deployment on Cloudflare"). | review |

## Known gaps

- [#120](https://github.com/soumendrak/widescope/issues/120): `ui/public/*.js` has no tests or type checks, `FlameGraph.svelte` is excluded from coverage, and nothing yet enforces that the thresholds only go up.
- [#123](https://github.com/soumendrak/widescope/issues/123): `npm audit` reports `braces` through `svelte-check@3`. The fix is a major upgrade to svelte-check 4.

When a reviewer corrects the same mistake twice, add a check to `scripts/standards.mjs` and a row here.

## Traps

- The UI imports the WASM package from `crates/widescope-core/pkg/`, which is not committed. Run `just build-wasm` after every Rust change, or the UI tests, dev server and e2e run the old core.
- `just e2e` sets `CI=1`, so a server already on port 5173 makes it fail instead of testing that server's code. A bare `npx playwright test` without `CI` reuses it (`reuseExistingServer` in `ui/playwright.config.ts`).
- `vite preview` binds to `localhost`, which on some Linux hosts resolves to `::1` only. Pass `--host 127.0.0.1` when a script calls `127.0.0.1`.
- Regenerating `crates/widescope-core/share-dict.bin` (`just train-share-dict`) breaks every existing share link. `decompress_share` in `share.rs` accepts one tag and uses the one current dictionary, so a new dictionary needs a new tag, and the old dictionary must be kept and chosen by tag when decoding.
- CI runs only on PRs into `main`. A PR stacked on another branch gets no CI until it is retargeted.
