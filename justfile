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

check:
    RUSTFLAGS="" {{_cargo}} check --workspace

clippy:
    RUSTFLAGS="" {{_cargo}} clippy --workspace -- -D warnings

fmt:
    {{_cargo}} fmt --all

test:
    RUSTFLAGS="" {{_cargo}} test --workspace

# ═══════════════════════════════════════════════════════════════════════
# UI
# ═══════════════════════════════════════════════════════════════════════

build-ui:
    cd ui && npm run build

dev:
    cd ui && npm run dev

ui-install:
    cd ui && npm install

# ═══════════════════════════════════════════════════════════════════════
# Housekeeping
# ═══════════════════════════════════════════════════════════════════════

clean:
    {{_cargo}} clean
    rm -rf {{_wasm_pkg}}
    rm -rf ui/dist
    rm -rf ui/node_modules
