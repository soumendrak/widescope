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

check:
    RUSTFLAGS="" {{_cargo}} check --workspace

clippy:
    RUSTFLAGS="" {{_cargo}} clippy --workspace -- -D warnings

fmt:
    {{_cargo}} fmt --all

test:
    RUSTFLAGS="" {{_cargo}} test --workspace

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
    cd ui && npm install

# ═══════════════════════════════════════════════════════════════════════
# Desktop (Tauri)
# ═══════════════════════════════════════════════════════════════════════
# One-time: `cargo install tauri-cli --version '^2'` (or `cargo binstall`).
# The web UI is unchanged — these wrap it in a native window with local-file
# open and .json/.trace double-click associations.

# build-wasm first so a cold checkout has crates/widescope-core/pkg/ for the
# UI dev server to import (beforeDevCommand only starts Vite).
tauri-dev: build-wasm
    cd src-tauri && cargo tauri dev

# Build the full frontend (WASM + UI dist) first; beforeBuildCommand is empty
# so `cargo tauri build` just bundles the prebuilt ui/dist — no working-dir or
# shell assumptions to break under tauri-action in CI.
tauri-build: build
    cd src-tauri && cargo tauri build

# Cut a desktop release: bump the version in tauri.conf.json + Cargo.toml,
# commit, tag, and push. The release.yml workflow then builds Win/Linux/macOS
# installers and uploads them to a draft GitHub Release to review and publish.
# Run on an up-to-date main. Usage (no "v" prefix): just release 0.2.0
release version:
    sed 's/"version": "[^"]*"/"version": "{{version}}"/' src-tauri/tauri.conf.json > src-tauri/tauri.conf.json.tmp && mv src-tauri/tauri.conf.json.tmp src-tauri/tauri.conf.json
    sed 's/^version = "[^"]*"/version = "{{version}}"/' src-tauri/Cargo.toml > src-tauri/Cargo.toml.tmp && mv src-tauri/Cargo.toml.tmp src-tauri/Cargo.toml
    git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
    git commit -m "release: v{{version}}"
    git tag v{{version}}
    git push origin HEAD
    git push origin v{{version}}
    @echo "Tagged v{{version}} — watch the 'Release Desktop' workflow, then publish the draft release on GitHub."

# ═══════════════════════════════════════════════════════════════════════
# Housekeeping
# ═══════════════════════════════════════════════════════════════════════

clean:
    {{_cargo}} clean
    rm -rf {{_wasm_pkg}}
    rm -rf ui/dist
    rm -rf ui/node_modules
    rm -rf src-tauri/target src-tauri/gen
