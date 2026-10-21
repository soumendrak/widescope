# Working in WideScope

Instructions for coding agents and people. [CONTRIBUTING.md](CONTRIBUTING.md) covers the architecture and how to add parsers, views and conventions. This file is what you need before your first commit.

## Setup

You need Rust (rustup) with the `wasm32-unknown-unknown` target, `wasm-pack`, `cargo-llvm-cov`, Node.js 20.19+ (CI uses 24), [just](https://just.systems), and Playwright Chromium.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack cargo-llvm-cov --locked
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
| Playwright suite, the same steps as CI | `just e2e` |
| Toolchain check | `just doctor` |
| Build WASM, then the UI into `ui/dist/` | `just build` |
| Dev server on `http://localhost:5173/editor/` | `just build-wasm && just dev` |
| CLI summary of a trace file | `cargo run -q -p widescope-cli -- analyze <file>` |

`just check` runs `scripts/standards.mjs` against `git merge-base origin/main HEAD`. CI passes the PR's base SHA. The gate runs rustfmt, clippy with `-D warnings`, the no-new-comments check, Rust tests under `cargo llvm-cov` with a 98% line floor, the WASM build, `svelte-check`, the vitest suite with the thresholds in `ui/vite.config.ts`, and 100% coverage of changed executable lines. It ends with `STANDARDS OK` or `STANDARDS FAIL: <step>`.

Before you open or update a PR, prove the change with [`.agents/skills/verify-widescope`](.agents/skills/verify-widescope/SKILL.md). It serves the built viewer and loads trace files through the real "Open file" button.

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
| rustfmt formatting, no clippy warnings | `just check` |
| No new comments in `.rs`, `.ts`, `.js`, `.mjs`, `.svelte` or `.css` files. Say it with names, types and tests. | `just check` |
| Rust line coverage stays at or above 98%. Raise `RUST_COVERAGE_MIN` in `scripts/standards.mjs` when coverage rises. Never lower it. | `just check` |
| UI coverage thresholds in `ui/vite.config.ts` only go up | `just check` |
| Every changed executable line is covered | `just check` |
| UI type errors | `svelte-check` in `just check` |
| The viewer loads and every e2e journey passes | `just e2e` in CI |
| The UI uses npm. `ui/package-lock.json` is the only lockfile. | review, until #116 adds `.gitignore` entries for the other lockfiles |
| Keep a PR under 400 changed lines, not counting tests, generated files and lockfiles | review |
| Don't deploy. Cloudflare builds and deploys `main` through `wrangler.jsonc`. | review |

When a reviewer corrects the same mistake twice, add a check to `scripts/standards.mjs` and a row here.

## Traps

- The UI imports the WASM package from `crates/widescope-core/pkg/`, which is not committed. Run `just build-wasm` after every Rust change, or the UI tests, dev server and e2e run the old core.
- Locally, Playwright reuses any server already on port 5173 (`reuseExistingServer` in `ui/playwright.config.ts`). A dev server from another worktree makes `just e2e` test that worktree's code. Stop it first.
- `vite preview` binds to `localhost`, which on this Linux box resolved to `::1` only. Pass `--host 127.0.0.1` when a script calls `127.0.0.1`.
- Changed-line coverage skips files that `ui/vite.config.ts` excludes from coverage, such as `FlameGraph.svelte` and `src/main.ts`. Their changes need a browser proof instead.
- Regenerating `crates/widescope-core/share-dict.bin` (`just train-share-dict`) breaks old share links unless you add a new format tag in `share.rs`.
- CI runs only on PRs into `main`. A PR stacked on another branch gets no CI until it is retargeted.
