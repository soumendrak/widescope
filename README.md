<p align="center">
  <img src="docs/assets/widescope-logo.svg" alt="WideScope logo" width="120" />
</p>

<h1 align="center">WideScope</h1>

<p align="center">
  <strong>Browser-native trace viewer for LLM and AI agent pipelines</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/github/actions/workflow/status/soumendrak/widescope/ci.yml?branch=main&label=CI" alt="CI status" />
  <img src="https://img.shields.io/badge/Rust-WASM-0F172A?logo=rust&logoColor=white" alt="Rust and WASM" />
  <img src="https://img.shields.io/badge/UI-Svelte%205-FF3E00?logo=svelte&logoColor=white" alt="Svelte 5" />
  <img src="https://img.shields.io/badge/hosting-Cloudflare%20Pages-F38020?logo=cloudflare&logoColor=white" alt="Cloudflare Pages" />
  <img src="https://img.shields.io/badge/license-MIT-22C55E" alt="MIT license" />
</p>

<p align="center">
  <code>OTLP JSON</code>
  <code>Jaeger JSON</code>
  <code>Flame graph</code>
  <code>Timeline</code>
  <code>LLM-aware</code>
  <code>Local-first</code>
</p>

A browser-based, zero-backend trace viewer for OpenTelemetry- and Jaeger-style traces, with an LLM-aware inspection UI powered by Rust/WASM and Svelte. Load a JSON trace locally, inspect it instantly, and keep the data entirely in your browser.

## Features

- **Zero backend** — static UI + WASM, deployable to Cloudflare Pages, any CDN, or any static host.
- **Privacy-first** — local parsing and rendering in the browser; no upload flow and no runtime fetches for bundled conventions.
- **Supported inputs today** — OTLP JSON (`resourceSpans`) and Jaeger JSON (`data[].spans`).
- **LLM-aware inspection** — resolves OTel GenAI, OpenInference, and LangChain-style attributes into model, token, prompt/completion, and tool-call detail views.
- **Two trace views** — canvas flame graph plus a service-lane timeline view.
- **Fast navigation** — span search, keyboard traversal, zoom, pan, fit/reset controls, and search-result highlighting.
- **Flexible loading** — drag and drop, file picker, clipboard paste, or the built-in JSON editor with live parsing.
- **Detailed sidebar** — timing, status, attributes, events, child spans, and LLM metadata for the selected span.

## Requirements

| Tool | Version | Install |
|---|---|---|
| Rust (via rustup) | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| wasm32 target | — | `rustup target add wasm32-unknown-unknown` |
| wasm-pack | 0.14+ | `cargo install wasm-pack` |
| Node.js | 18+ | <https://nodejs.org> |
| binaryen (`wasm-opt`) | optional, recommended | `brew install binaryen` / `apt install binaryen` |

> **Homebrew Rust users:** the `wasm32-unknown-unknown` target is not included in the Homebrew Rust package. Install Rust via `rustup` instead (both can coexist; prefix commands with `PATH="$HOME/.cargo/bin:$PATH"`).

## Quick Start

```bash
# 1. Install UI deps (one-time)
make ui-install

# 2. Build the WASM package and the production UI bundle
make build

# 3. Start the Vite dev server
make dev
# → http://localhost:5173
```

## Common Targets

```bash
make ui-install    # install ui/package.json dependencies
make build-wasm    # compile Rust -> WASM and optimize with wasm-opt when available
make build-ui      # vite production build -> ui/dist/
make build         # build-wasm + build-ui
make check         # cargo check --workspace
make fmt           # cargo fmt --all
make clippy        # cargo clippy --workspace -- -D warnings
make test          # cargo test --workspace
make clean         # remove Rust, WASM package, UI dist, and node_modules artifacts
```

## Development Notes

- **`make dev` only starts the UI dev server** — if you change Rust code, rerun `make build-wasm` to regenerate `crates/widescope-core/pkg/`.
- **`make build` produces the deployable static assets** in `ui/dist/`.
- **`wasm-opt` is optional** — the build still succeeds without it, but the generated `.wasm` will be larger.

## Deployment on Cloudflare Pages

This repo is set up to publish the static `ui/dist/` bundle to **Cloudflare Pages** from GitHub Actions.

1. Create a Cloudflare Pages project named `widescope`.
2. Add the GitHub repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`.
3. Push to `main` to trigger the deploy workflow.
4. Set a custom domain in Cloudflare Pages if you want the repo website field to use your own domain.

Recommended repo website value after setup:

```text
https://widescope.pages.dev
```

## Usage

1. Open `http://localhost:5173` in development, or deploy `ui/dist/` to Cloudflare Pages or any static host.
2. Load trace JSON by pasting into the editor, clicking **Open file**, dragging in a `.json` file, or using **Load sample JSON**.
3. Use **Format**, **Paste JSON**, **Submit JSON**, and **Clear JSON** in the editor toolbar as needed.
4. Switch between **Flame** and **Timeline** from the top toolbar.
5. Search spans from the toolbar to highlight matches and jump between them.
6. Click any span to inspect details in the resizable right sidebar.
7. In the flame graph, use **Cmd/Ctrl + scroll** to zoom, drag to pan, double-click to zoom to a span, and use `↑↓←→`, `Enter`, `Esc`, `F`, and `0` for keyboard navigation.

## Project Structure

```
widescope/
├── Makefile                         # build automation
├── Cargo.toml                       # workspace root
├── rust-toolchain.toml              # stable toolchain + wasm target
├── crates/
│   └── widescope-core/              # Rust WASM library
│       ├── src/
│       │   ├── lib.rs               # wasm-bindgen exports and trace lifecycle
│       │   ├── models/              # span, trace, llm, and layout types
│       │   ├── parsers/             # OTLP JSON and Jaeger JSON parsers
│       │   ├── conventions/         # convention registry + attribute resolver
│       │   ├── layout/              # flamegraph and timeline layout algorithms
│       │   ├── trace_builder.rs     # trace assembly, warnings, self-time, cycles
│       │   └── errors.rs            # structured errors returned to JS
│       └── pkg/                     # generated by wasm-pack
├── ui/                              # Svelte 5 + Vite app shell
│   ├── src/
│   │   ├── App.svelte               # root shell and trace editor workspace
│   │   ├── components/              # toolbar, graphs, sidebar, drop zone, banners
│   │   ├── lib/                     # wasm loader, input handling, bundles, TS types
│   │   └── stores/                  # trace and selection state
│   ├── static/
│   └── package.json
├── conventions/                     # OTel, OpenInference, and LangChain mappings
├── test-fixtures/                   # sample traces
└── docs/                            # design docs
```

## Supported Formats

| Format | Status |
|---|---|
| OTLP JSON (`resourceSpans`) | ✅ Supported |
| Jaeger JSON (`data[].spans`) | ✅ Supported |
| OpenInference JSON | 🔜 Planned |

> `OpenInference` mappings are already bundled for LLM attribute normalization, but standalone OpenInference trace JSON is not parsed directly yet.

## Convention Mappings

Attribute-to-LLM mappings live in `conventions/` and are bundled into the UI at build time. Three mapping files are included:

| File | Covers |
|---|---|
| `opentelemetry.json` | OTel GenAI semconv (`gen_ai.*`) |
| `openinference.json` | OpenInference attributes (`llm.*`, `openinference.span.kind`) |
| `langchain.json` | LangChain attributes (`langchain.*`) |

Convention resolution is first-match-wins. See `conventions/README.md` to extend the mapping set.

## License

MIT
