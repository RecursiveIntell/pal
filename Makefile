.PHONY: help bootstrap dev build test clippy fmt-check web-build web-lint check bundle bundle-appimage

help:
	@echo "Targets:"
	@echo "  make bootstrap      # install frontend dependencies"
	@echo "  make dev            # run Tauri dev app"
	@echo "  make build          # cargo build --workspace"
	@echo "  make test           # cargo test --workspace"
	@echo "  make clippy         # cargo clippy --workspace -- -D warnings"
	@echo "  make fmt-check      # cargo fmt --all -- --check"
	@echo "  make web-build      # pnpm build (gui/src)"
	@echo "  make web-lint       # pnpm lint (gui/src)"
	@echo "  make check          # full project checks"
	@echo "  make bundle         # build deb + rpm"
	@echo "  make bundle-appimage # attempt full bundle including AppImage"

bootstrap:
	cd gui/src && pnpm install

dev:
	cd gui/src-tauri && cargo tauri dev

build:
	cargo build --workspace

test:
	cargo test --workspace

clippy:
	cargo clippy --workspace -- -D warnings

fmt-check:
	cargo fmt --all -- --check

web-build:
	cd gui/src && pnpm build

web-lint:
	cd gui/src && pnpm lint

check: build test clippy fmt-check web-build web-lint

bundle:
	cd gui/src-tauri && cargo tauri build --bundles deb,rpm

bundle-appimage:
	cd gui/src-tauri && cargo tauri build
