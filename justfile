_wasm_pack := `command -v wasm-pack 2>/dev/null || echo ${CARGO_HOME:-${HOME}/.cargo}/bin/wasm-pack`
_cargo := `command -v cargo 2>/dev/null || echo ${CARGO_HOME:-${HOME}/.cargo}/bin/cargo`
_wasm_pkg := "crates/widescope-core/pkg"
_wasm_bin := _wasm_pkg + "/widescope_core_bg.wasm"
_wasm_opt_flags := "--enable-bulk-memory --enable-reference-types --enable-mutable-globals --enable-nontrapping-float-to-int --enable-sign-ext"

# ═══════════════════════════════════════════════════════════════════════
# Top-level targets
# ═══════════════════════════════════════════════════════════════════════

all: build

build: build-wasm build-ui

# ═══════════════════════════════════════════════════════════════════════
# Rust / WASM
# ═══════════════════════════════════════════════════════════════════════

build-wasm:
    RUSTFLAGS="" PATH="${CARGO_HOME:-${HOME}/.cargo}/bin:${PATH}" \
        {{_wasm_pack}} build crates/widescope-core --target web --out-dir pkg
    just _wasm-opt-pass

_wasm-opt-pass:
    @if command -v wasm-opt >/dev/null 2>&1; then \
        echo "[wasm-opt] Optimising {{_wasm_bin}}..." ; \
        wasm-opt {{_wasm_opt_flags}} -O4 -o {{_wasm_bin}} {{_wasm_bin}} ; \
        echo "[wasm-opt] Done — $(du -sh {{_wasm_bin}} | cut -f1) optimised" ; \
    else \
        echo "[wasm-opt] Not found — skipping post-optimisation (brew install binaryen)" ; \
    fi

# Rebuild the share-link compression dictionary from representative fixtures.
# The dictionary is embedded in the WASM binary and seeds DEFLATE compression
# of share links. Keep it at or below DEFLATE's 32 KiB window — bytes beyond
# that are never referenced. If its contents change, bump the format tag in
# crates/widescope-core/src/share.rs so links made with the old dictionary
# still decode, then re-run `just build-wasm`.
train-share-dict:
    cat \
        test-fixtures/jaeger/sample_llm_pipeline.json \
        test-fixtures/openinference/sample_llm_pipeline.json \
        test-fixtures/otlp/upload-samples/01-simple-request.json \
        test-fixtures/otlp/sample_llm_pipeline.json \
        > crates/widescope-core/share-dict.bin
    @echo "[share-dict] $(wc -c < crates/widescope-core/share-dict.bin | tr -d ' ') bytes (keep <= 32768) — run 'just build-wasm' to embed it"

# Every repository standard, the same command CI's standards job runs. Ends with STANDARDS OK or STANDARDS FAIL.
check base="":
    node scripts/standards.mjs {{ if base == "" { "" } else { "--base " + base } }}

fmt:
    {{_cargo}} fmt --all

# Rust and UI unit tests
test: build-wasm
    RUSTFLAGS="" {{_cargo}} test --workspace
    cd ui && npx vitest run

# Playwright suite against the dev server, the same steps as CI's e2e job
e2e: build-wasm
    cd ui && npm run e2e

coverage-html:
    {{_cargo}} llvm-cov --workspace --html --open

# Check the toolchain that check, test and e2e need
doctor:
    #!/usr/bin/env bash
    set -uo pipefail
    status=0
    report() { if [ "$2" = ok ]; then echo "ok   $1"; else echo "FAIL $1: $2"; status=1; fi; }
    has() { command -v "$1" >/dev/null 2>&1; }
    atleast() { [ "$(printf '%s\n%s\n' "$2" "$1" | sort -V | head -n1)" = "$2" ]; }
    has cargo && report "$(cargo --version)" ok || report cargo "install Rust with rustup"
    rustup target list --installed 2>/dev/null | grep -qx wasm32-unknown-unknown && report "wasm32-unknown-unknown target" ok || report "wasm32-unknown-unknown target" "rustup target add wasm32-unknown-unknown"
    has wasm-pack && report "$(wasm-pack --version)" ok || report wasm-pack "cargo install wasm-pack --locked"
    cargo llvm-cov --version >/dev/null 2>&1 && report "$(cargo llvm-cov --version)" ok || report cargo-llvm-cov "cargo install cargo-llvm-cov --locked"
    node_version=$(node --version 2>/dev/null)
    atleast "${node_version#v}" 20.19 && report "node $node_version" ok || report "node ${node_version:-missing}" "install Node.js 20.19+ (CI uses 24)"
    [ -d ui/node_modules ] && report "ui/node_modules" ok || report "ui/node_modules" "just ui-install"
    chromium=$(cd ui && node -e 'process.stdout.write(require("@playwright/test").chromium.executablePath())' 2>/dev/null)
    [ -n "$chromium" ] && [ -x "$chromium" ] && report "playwright chromium" ok || report "playwright chromium" "cd ui && npx playwright install --with-deps chromium"
    [ "$status" = 0 ] && echo "DOCTOR OK" || echo "DOCTOR FAIL"
    exit $status

bench-fixtures:
    RUSTFLAGS="" {{_cargo}} run -p widescope-core --example bench_fixtures -- test-fixtures

# ═══════════════════════════════════════════════════════════════════════
# UI
# ═══════════════════════════════════════════════════════════════════════

build-ui:
    cd ui && npm run build

dev:
    cd ui && npm run dev

ui-install:
    cd ui && npm ci

# ═══════════════════════════════════════════════════════════════════════
# Housekeeping
# ═══════════════════════════════════════════════════════════════════════

clean:
    {{_cargo}} clean
    rm -rf {{_wasm_pkg}}
    rm -rf ui/dist
    rm -rf ui/node_modules
