# WideScope

A browser-based, zero-backend trace viewer for LLM and AI agent pipelines. Drop an OpenTelemetry JSON file — see a flame graph instantly. No server, no sign-up, no data leaves your browser.

## Features

- **Zero backend** — static HTML + WASM + JS, deployable to any CDN or GitHub Pages.
- **Privacy-first** — strict `connect-src: 'none'` CSP; no telemetry, no network calls.
- **OTLP JSON** support (MVP). Jaeger + OpenInference in v1.
- **Flame graph** — canvas-based, zoomable (Ctrl+scroll), pannable, full keyboard navigation.
- **LLM-aware** — auto-detects OTel GenAI (`gen_ai.*`), OpenInference, and LangChain conventions; shows token counts, model, prompt/completion, tool calls.
- **Span detail sidebar** — attributes, events, children, self-time breakdown.
- **Drop / paste / file-open** — three ways to load a trace without any upload.

## Requirements

| Tool | Version | Install |
|---|---|---|
| Rust (via rustup) | stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| wasm32 target | — | `rustup target add wasm32-unknown-unknown` |
| wasm-pack | 0.14+ | `cargo install wasm-pack` |
| binaryen (wasm-opt) | 108+ | `brew install binaryen` / `apt install binaryen` |
| Node.js | 18+ | <https://nodejs.org> |

> **Homebrew Rust users:** the `wasm32-unknown-unknown` target is not included in the Homebrew Rust package. Install Rust via `rustup` instead (both can coexist; prefix commands with `PATH="$HOME/.cargo/bin:$PATH"`).

## Quick Start

```bash
# 1. Install UI deps (one-time)
make ui-install

# 2. Build WASM + optimise + build UI
make build

# 3. Start dev server
make dev
# → http://localhost:5173
```

### Individual targets

```bash
make build-wasm   # compile Rust → WASM + run wasm-opt -O4
make build-ui     # vite production build → ui/dist/
make check        # cargo check
make clippy       # cargo clippy -D warnings
make test         # cargo test
make clean        # remove all build artefacts
```

## Usage

1. Open `http://localhost:5173` (dev) or deploy `ui/dist/` anywhere.
2. A sample LLM pipeline trace loads automatically on first visit.
3. **Drag & drop** a `.json` trace file, or click **Open file** in the toolbar.
4. Click any flame-graph bar to inspect span details in the right sidebar.
5. **Keyboard**: `↑↓←→` navigate spans, `Enter` selects, `F` fits selection, `0` resets zoom.

## Project Structure

```
widescope/
├── Makefile                         # build automation
├── Cargo.toml                       # workspace root
├── rust-toolchain.toml              # pins stable channel + wasm32 target
├── crates/
│   └── widescope-core/              # Rust WASM library
│       ├── src/
│       │   ├── lib.rs               # #[wasm_bindgen] exports
│       │   ├── models/              # Span, Trace, LlmAttributes, Layout
│       │   ├── parsers/             # OTLP JSON parser
│       │   ├── conventions/         # registry + resolver
│       │   ├── layout/              # flamegraph + timeline algorithms
│       │   ├── trace_builder.rs     # build_trace, self-time, cycle detection
│       │   └── errors.rs            # WideError → JsValue
│       └── pkg/                     # wasm-pack output (git-ignored)
├── ui/                              # Svelte 4 SPA (Vite 5)
│   ├── src/
│   │   ├── App.svelte               # root shell + CSS variables
│   │   ├── components/              # Toolbar, FlameGraph, SpanDetail, DropZone, ErrorBanner
│   │   ├── lib/                     # wasm.ts, types.ts, input.ts, theme.ts, bundles
│   │   └── stores/                  # trace.ts, selection.ts
│   └── index.html
├── conventions/                     # OTel, OpenInference, LangChain JSON mappings
├── test-fixtures/otlp/              # sample_llm_pipeline.json
└── docs/                            # LLD.md
```

## Supported Formats

| Format | Status |
|---|---|
| OTLP JSON (`resourceSpans`) | ✅ MVP |
| Jaeger JSON | 🔜 v1 |
| OpenInference JSON | 🔜 v1 |

## Convention mappings

Attribute-to-LLM mappings live in `conventions/`. Three are bundled:

| File | Covers |
|---|---|
| `opentelemetry.json` | OTel GenAI semconv 1.28 (`gen_ai.*`) |
| `openinference.json` | OpenInference 0.1 (`llm.*`, `openinference.span.kind`) |
| `langchain.json` | LangChain (`langchain.*`) |

First match wins. See `conventions/README.md` to add new frameworks.

## License

MIT
