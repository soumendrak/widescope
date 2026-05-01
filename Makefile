CARGO_HOME ?= $(HOME)/.cargo
WASM_PACK ?= $(HOME)/.cargo/bin/wasm-pack

.PHONY: build

build:
	RUSTFLAGS="" PATH="$(CARGO_HOME)/bin:$$PATH" \
		$(WASM_PACK) build crates/widescope-core --target web --out-dir pkg
	cd ui && npm run build
