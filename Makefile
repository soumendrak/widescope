.PHONY: all build build-wasm wasm-opt-pass build-ui dev clean check fmt clippy test

WASM_PACK   := $(shell command -v wasm-pack 2>/dev/null || echo $(HOME)/.cargo/bin/wasm-pack)
CARGO       := $(shell command -v cargo 2>/dev/null || echo $(HOME)/.cargo/bin/cargo)
WASM_OPT    := $(shell command -v wasm-opt 2>/dev/null || echo wasm-opt)
WASM_PKG    := crates/widescope-core/pkg
WASM_BIN    := $(WASM_PKG)/widescope_core_bg.wasm

WASM_OPT_FLAGS := \
	--enable-bulk-memory \
	--enable-reference-types \
	--enable-mutable-globals \
	--enable-nontrapping-float-to-int \
	--enable-sign-ext

# ── Top-level targets ───────────────────────────────────────────────

all: build

build: build-wasm build-ui

# ── Rust / WASM ─────────────────────────────────────────────────────

build-wasm:
	RUSTFLAGS="" PATH="$(HOME)/.cargo/bin:$(PATH)" \
		$(WASM_PACK) build crates/widescope-core --target web --out-dir pkg
	$(MAKE) wasm-opt-pass

wasm-opt-pass:
	@if command -v wasm-opt >/dev/null 2>&1; then \
		echo "[wasm-opt] Optimising $(WASM_BIN)..."; \
		wasm-opt $(WASM_OPT_FLAGS) -O4 -o $(WASM_BIN) $(WASM_BIN); \
		echo "[wasm-opt] Done — $$(du -sh $(WASM_BIN) | cut -f1) optimised"; \
	else \
		echo "[wasm-opt] Not found — skipping post-optimisation (brew install binaryen)"; \
	fi

check:
	RUSTFLAGS="" $(CARGO) check --workspace

clippy:
	RUSTFLAGS="" $(CARGO) clippy --workspace -- -D warnings

fmt:
	$(CARGO) fmt --all

test:
	RUSTFLAGS="" $(CARGO) test --workspace

# ── UI ──────────────────────────────────────────────────────────────

build-ui:
	cd ui && npm run build

dev:
	cd ui && npm run dev

ui-install:
	cd ui && npm install

# ── Housekeeping ─────────────────────────────────────────────────────

clean:
	$(CARGO) clean
	rm -rf $(WASM_PKG)
	rm -rf ui/dist
	rm -rf ui/node_modules
