# PSM Player Build System
# =======================
#
# Builds all components: Rust crates, WASM, Python bindings, and frontend.
#
# Usage:
#   make all       - Build everything
#   make rust      - Build Rust crates
#   make wasm      - Build WASM package
#   make python    - Build Python bindings
#   make test      - Run all tests
#   make bench     - Run benchmarks
#   make clean     - Clean all artifacts

.PHONY: all rust wasm python test bench clean fmt lint docs

# Default target
all: rust wasm python

# =============================================================================
# Rust Crates
# =============================================================================

rust:
	@echo "Building Rust crates..."
	cargo build --release --workspace

rust-debug:
	@echo "Building Rust crates (debug)..."
	cargo build --workspace

# Build specific crates
frequency:
	@echo "Building psm-player-frequency..."
	cargo build --release -p psm-player-frequency --all-features

cli:
	@echo "Building psm-player-cli..."
	cargo build --release -p psm-player-cli

core:
	@echo "Building psm-player-core..."
	cargo build --release -p psm-player-core

# =============================================================================
# WebAssembly
# =============================================================================

wasm:
	@echo "Building WASM package..."
	@command -v wasm-pack >/dev/null 2>&1 || { echo "Installing wasm-pack..."; cargo install wasm-pack; }
	cd crates/psm-player-wasm && wasm-pack build --target web --release
	@echo "WASM package built at: crates/psm-player-wasm/pkg/"

wasm-debug:
	@echo "Building WASM package (debug)..."
	cd crates/psm-player-wasm && wasm-pack build --target web --dev

wasm-node:
	@echo "Building WASM package for Node.js..."
	cd crates/psm-player-wasm && wasm-pack build --target nodejs --release

# =============================================================================
# Python Bindings
# =============================================================================

python:
	@echo "Building Python bindings..."
	@command -v maturin >/dev/null 2>&1 || { echo "Installing maturin..."; pip install maturin; }
	cd crates/psm-player-python && maturin develop --release
	@echo "Python module installed: psm_frequency"

python-wheel:
	@echo "Building Python wheel..."
	cd crates/psm-player-python && maturin build --release
	@echo "Wheel built at: target/wheels/"

# =============================================================================
# Testing
# =============================================================================

test:
	@echo "Running all tests..."
	cargo test --workspace --all-features

test-frequency:
	@echo "Running frequency crate tests..."
	cargo test -p psm-player-frequency --all-features

test-wasm:
	@echo "Running WASM tests..."
	cd crates/psm-player-wasm && wasm-pack test --headless --chrome

test-python:
	@echo "Running Python tests..."
	cd crates/psm-player-python && python -m pytest tests/ -v || echo "No Python tests found"

# =============================================================================
# Benchmarks
# =============================================================================

bench:
	@echo "Running benchmarks..."
	cargo bench -p psm-player-frequency

bench-fft:
	@echo "Running FFT benchmarks..."
	cargo bench -p psm-player-frequency -- fft

bench-fingerprint:
	@echo "Running fingerprint benchmarks..."
	cargo bench -p psm-player-frequency -- fingerprint

# =============================================================================
# Code Quality
# =============================================================================

fmt:
	@echo "Formatting code..."
	cargo fmt --all

lint:
	@echo "Running clippy..."
	cargo clippy --workspace --all-features -- -D warnings

check:
	@echo "Checking compilation..."
	cargo check --workspace --all-features

# =============================================================================
# Documentation
# =============================================================================

docs:
	@echo "Building documentation..."
	cargo doc --workspace --all-features --no-deps
	@echo "Docs available at: target/doc/psm_player_frequency/index.html"

docs-open: docs
	@echo "Opening documentation..."
	open target/doc/psm_player_frequency/index.html 2>/dev/null || xdg-open target/doc/psm_player_frequency/index.html 2>/dev/null || echo "Open target/doc/psm_player_frequency/index.html manually"

# =============================================================================
# Examples
# =============================================================================

example-basic:
	@echo "Running basic analysis example..."
	cargo run --example basic_analysis -p psm-player-frequency -- test.wav

example-streaming:
	@echo "Running streaming analysis example..."
	cargo run --example streaming_analysis -p psm-player-frequency -- test.wav

example-similarity:
	@echo "Running content similarity example..."
	cargo run --example content_similarity -p psm-player-frequency --features recommend -- file1.wav file2.wav

# =============================================================================
# Cleanup
# =============================================================================

clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf crates/psm-player-wasm/pkg
	rm -rf crates/psm-player-python/target
	find . -name "*.pyc" -delete
	find . -name "__pycache__" -type d -delete
	@echo "Clean complete"

clean-wasm:
	rm -rf crates/psm-player-wasm/pkg

clean-python:
	rm -rf crates/psm-player-python/target
	find . -name "*.so" -name "psm_frequency*" -delete

# =============================================================================
# Installation
# =============================================================================

install-cli:
	@echo "Installing CLI tool..."
	cargo install --path crates/psm-player-cli
	@echo "psm-player CLI installed"

install-deps:
	@echo "Installing build dependencies..."
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack
	pip install maturin

# =============================================================================
# Docker
# =============================================================================

docker-build:
	@echo "Building Docker image..."
	docker build -t psm-player:latest .

docker-run:
	@echo "Running Docker container..."
	docker run -p 8080:8080 psm-player:latest

# =============================================================================
# CI Helpers
# =============================================================================

ci-check: fmt lint check test
	@echo "CI checks complete"

ci-build: rust wasm python
	@echo "CI build complete"

# =============================================================================
# Development Helpers
# =============================================================================

dev-setup: install-deps rust-debug
	@echo "Development environment ready"

watch:
	@echo "Watching for changes..."
	cargo watch -x "check --workspace"

# Print help
help:
	@echo "PSM Player Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all           - Build everything (default)"
	@echo "  rust          - Build Rust crates"
	@echo "  wasm          - Build WASM package"
	@echo "  python        - Build Python bindings"
	@echo "  test          - Run all tests"
	@echo "  bench         - Run benchmarks"
	@echo "  docs          - Build documentation"
	@echo "  fmt           - Format code"
	@echo "  lint          - Run clippy"
	@echo "  clean         - Clean all artifacts"
	@echo "  install-cli   - Install CLI tool"
	@echo "  help          - Show this help"
